//! Smart model routing and provider management.
//!
//! Routes agent requests to the best available provider/model based on agent role,
//! with automatic fallback chains, response caching, health checking, and rate limiting.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::agents::AgentRole;
use crate::errors::AiError;
use crate::provider::{LlmProvider, UnifiedRequest, UnifiedResponse};

// ── Routing Rule ────────────────────────────────────────────────────────

/// A provider+model preference within a routing rule.
#[derive(Debug, Clone)]
pub struct ProviderPreference {
    pub provider_name: String,
    pub model_id: String,
    pub priority: u8,
}

/// Rule that determines how to route requests for a given agent role.
#[derive(Debug, Clone)]
pub struct RoutingRule {
    pub agent_role: String,
    pub preferred_providers: Vec<ProviderPreference>,
    pub max_latency_ms: u64,
    pub require_tool_use: bool,
    pub min_context_window: u32,
}

// ── Response Cache ──────────────────────────────────────────────────────

/// A cached response entry with TTL.
#[derive(Debug, Clone)]
struct CacheEntry {
    response: UnifiedResponse,
    inserted_at: Instant,
}

/// LRU-style cache for deduplicating identical prompts.
pub struct ResponseCache {
    entries: HashMap<u64, CacheEntry>,
    order: Vec<u64>,
    max_entries: usize,
    ttl: Duration,
    hits: AtomicU64,
    misses: AtomicU64,
}

/// Snapshot of cache metrics.
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub max_entries: usize,
    pub hit_rate: f64,
}

impl ResponseCache {
    fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries),
            order: Vec::with_capacity(max_entries),
            max_entries,
            ttl,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    fn get(&self, key: u64) -> Option<&UnifiedResponse> {
        if let Some(entry) = self.entries.get(&key) {
            if entry.inserted_at.elapsed() < self.ttl {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(&entry.response);
            }
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn insert(&mut self, key: u64, response: UnifiedResponse) {
        // Evict expired entries first.
        self.evict_expired();

        // If at capacity, remove the oldest entry.
        if self.entries.len() >= self.max_entries {
            if let Some(oldest_key) = self.order.first().copied() {
                self.entries.remove(&oldest_key);
                self.order.remove(0);
            }
        }

        self.entries.insert(
            key,
            CacheEntry {
                response,
                inserted_at: Instant::now(),
            },
        );
        // Remove the key if it already exists in order to move it to the back.
        self.order.retain(|k| *k != key);
        self.order.push(key);
    }

    fn evict_expired(&mut self) {
        let now = Instant::now();
        let ttl = self.ttl;
        let expired_keys: Vec<u64> = self
            .entries
            .iter()
            .filter(|(_, v)| now.duration_since(v.inserted_at) >= ttl)
            .map(|(k, _)| *k)
            .collect();

        for key in &expired_keys {
            self.entries.remove(key);
        }
        self.order.retain(|k| !expired_keys.contains(k));
    }

    fn metrics(&self) -> CacheMetrics {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        CacheMetrics {
            hits,
            misses,
            entries: self.entries.len(),
            max_entries: self.max_entries,
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

// ── Provider Status / Health ────────────────────────────────────────────

/// Health status of a single provider.
#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub name: String,
    pub healthy: bool,
    pub last_check: Option<Instant>,
    pub last_error: Option<String>,
    pub concurrent_requests: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
}

/// Internal mutable state tracked per registered provider.
struct ProviderState {
    provider: Arc<dyn LlmProvider>,
    healthy: bool,
    last_check: Option<Instant>,
    last_error: Option<String>,
    concurrent_requests: Arc<AtomicUsize>,
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    total_latency_ms: AtomicU64,
}

impl ProviderState {
    fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            healthy: true,
            last_check: None,
            last_error: None,
            concurrent_requests: Arc::new(AtomicUsize::new(0)),
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
        }
    }

    fn status(&self) -> ProviderStatus {
        let total = self.total_requests.load(Ordering::Relaxed);
        let latency = self.total_latency_ms.load(Ordering::Relaxed);
        ProviderStatus {
            name: self.provider.name().to_string(),
            healthy: self.healthy,
            last_check: self.last_check,
            last_error: self.last_error.clone(),
            concurrent_requests: self.concurrent_requests.load(Ordering::Relaxed),
            total_requests: total,
            total_errors: self.total_errors.load(Ordering::Relaxed),
            avg_latency_ms: if total > 0 {
                latency as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

// ── Router Config ───────────────────────────────────────────────────────

/// Configuration for the model router, loaded from environment or config file.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub enable_ollama: bool,
    pub ollama_url: String,
    pub enable_openrouter: bool,
    pub enable_anthropic: bool,
    pub cache_ttl_seconds: u64,
    pub cache_max_entries: usize,
    pub health_check_interval_seconds: u64,
    pub max_concurrent_per_provider: usize,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enable_ollama: true,
            ollama_url: "http://localhost:11434".to_string(),
            enable_openrouter: false,
            enable_anthropic: true,
            cache_ttl_seconds: 300,
            cache_max_entries: 512,
            health_check_interval_seconds: 30,
            max_concurrent_per_provider: 16,
        }
    }
}

impl RouterConfig {
    /// Load configuration from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();

        if let Ok(v) = std::env::var("PHANTOM_ENABLE_OLLAMA") {
            cfg.enable_ollama = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Ok(v) = std::env::var("PHANTOM_OLLAMA_URL") {
            cfg.ollama_url = v;
        }
        if let Ok(v) = std::env::var("PHANTOM_ENABLE_OPENROUTER") {
            cfg.enable_openrouter = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Ok(v) = std::env::var("PHANTOM_ENABLE_ANTHROPIC") {
            cfg.enable_anthropic = v == "1" || v.eq_ignore_ascii_case("true");
        }
        if let Ok(v) = std::env::var("PHANTOM_CACHE_TTL") {
            if let Ok(n) = v.parse::<u64>() {
                cfg.cache_ttl_seconds = n;
            }
        }
        if let Ok(v) = std::env::var("PHANTOM_CACHE_MAX_ENTRIES") {
            if let Ok(n) = v.parse::<usize>() {
                cfg.cache_max_entries = n;
            }
        }
        if let Ok(v) = std::env::var("PHANTOM_HEALTH_CHECK_INTERVAL") {
            if let Ok(n) = v.parse::<u64>() {
                cfg.health_check_interval_seconds = n;
            }
        }
        if let Ok(v) = std::env::var("PHANTOM_MAX_CONCURRENT_PER_PROVIDER") {
            if let Ok(n) = v.parse::<usize>() {
                cfg.max_concurrent_per_provider = n;
            }
        }

        cfg
    }
}

// ── Router Metrics ──────────────────────────────────────────────────────

/// Aggregate routing metrics.
#[derive(Debug, Clone)]
pub struct RouterMetrics {
    pub total_requests: u64,
    pub total_fallbacks: u64,
    pub total_failures: u64,
    pub cache: CacheMetrics,
    pub providers: Vec<ProviderStatus>,
}

// ── Provider Manager ────────────────────────────────────────────────────

/// Manages provider lifecycle: health checks, concurrency tracking, rate limiting.
pub struct ProviderManager {
    providers: HashMap<String, ProviderState>,
    config: RouterConfig,
    health_check_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ProviderManager {
    fn new(config: RouterConfig) -> Self {
        Self {
            providers: HashMap::new(),
            config,
            health_check_handle: None,
        }
    }

    fn register(&mut self, provider: Arc<dyn LlmProvider>) {
        let name = provider.name().to_string();
        info!(provider = %name, "registering provider");
        self.providers.insert(name, ProviderState::new(provider));
    }

    fn get(&self, name: &str) -> Option<&ProviderState> {
        self.providers.get(name)
    }

    fn is_healthy(&self, name: &str) -> bool {
        self.providers
            .get(name)
            .map(|p| p.healthy)
            .unwrap_or(false)
    }

    fn is_at_capacity(&self, name: &str) -> bool {
        self.providers
            .get(name)
            .map(|p| {
                p.concurrent_requests.load(Ordering::Relaxed)
                    >= self.config.max_concurrent_per_provider
            })
            .unwrap_or(true)
    }

    fn all_statuses(&self) -> Vec<ProviderStatus> {
        self.providers.values().map(|p| p.status()).collect()
    }

    /// Perform a single round of health checks across all providers.
    async fn run_health_checks(providers: &Arc<RwLock<ProviderManager>>) {
        let names: Vec<String> = {
            let mgr = providers.read().await;
            mgr.providers.keys().cloned().collect()
        };

        for name in names {
            let provider_arc = {
                let mgr = providers.read().await;
                mgr.providers.get(&name).map(|ps| Arc::clone(&ps.provider))
            };

            if let Some(provider) = provider_arc {
                let result = provider.health_check().await;
                let mut mgr = providers.write().await;
                if let Some(state) = mgr.providers.get_mut(&name) {
                    state.last_check = Some(Instant::now());
                    if result {
                        if !state.healthy {
                            info!(provider = %name, "provider recovered");
                        }
                        state.healthy = true;
                        state.last_error = None;
                    } else {
                        if state.healthy {
                            warn!(provider = %name, "provider became unhealthy");
                        }
                        state.healthy = false;
                        state.last_error = Some("health check failed".to_string());
                    }
                }
            }
        }
    }
}

impl Drop for ProviderManager {
    fn drop(&mut self) {
        if let Some(handle) = self.health_check_handle.take() {
            handle.abort();
        }
    }
}

// ── Model Router ────────────────────────────────────────────────────────

/// Intelligently routes agent requests to the best available provider/model.
pub struct ModelRouter {
    providers: Arc<RwLock<ProviderManager>>,
    routing_table: HashMap<String, RoutingRule>,
    fallback_chain: Vec<String>,
    cache: Arc<RwLock<ResponseCache>>,
    config: RouterConfig,
    total_requests: AtomicU64,
    total_fallbacks: AtomicU64,
    total_failures: AtomicU64,
}

impl ModelRouter {
    /// Create a new router with the given configuration.
    pub fn new(config: RouterConfig) -> Self {
        let cache = ResponseCache::new(
            config.cache_max_entries,
            Duration::from_secs(config.cache_ttl_seconds),
        );

        let routing_table = Self::build_default_routing_table(&config);
        let fallback_chain = Self::build_default_fallback_chain(&config);

        Self {
            providers: Arc::new(RwLock::new(ProviderManager::new(config.clone()))),
            routing_table,
            fallback_chain,
            cache: Arc::new(RwLock::new(cache)),
            config,
            total_requests: AtomicU64::new(0),
            total_fallbacks: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
        }
    }

    /// Register a provider and make it available for routing.
    pub async fn register_provider(&self, provider: Arc<dyn LlmProvider>) {
        let mut mgr = self.providers.write().await;
        mgr.register(provider);
    }

    /// Start background health checks. Call once after all providers are registered.
    pub fn start_health_checks(&self) -> tokio::task::JoinHandle<()> {
        let providers = Arc::clone(&self.providers);
        let interval = Duration::from_secs(self.config.health_check_interval_seconds);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                ProviderManager::run_health_checks(&providers).await;
            }
        })
    }

    /// Main routing method: pick the best provider for this agent and send the request.
    pub async fn route(
        &self,
        agent_role: AgentRole,
        request: &UnifiedRequest,
    ) -> Result<UnifiedResponse, AiError> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // Check cache first.
        let cache_key = Self::compute_cache_key(request);
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(cache_key) {
                debug!(agent = %agent_role.id(), "cache hit");
                return Ok(cached.clone());
            }
        }

        // Try routing with fallback.
        let response = self.route_with_fallback(agent_role, request).await?;

        // Store in cache.
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, response.clone());
        }

        Ok(response)
    }

    /// Try providers in priority order, falling back on failure.
    pub async fn route_with_fallback(
        &self,
        agent_role: AgentRole,
        request: &UnifiedRequest,
    ) -> Result<UnifiedResponse, AiError> {
        let role_id = agent_role.id().to_string();
        let preferences = self.resolve_preferences(&role_id);

        let mut last_error: Option<AiError> = None;

        for pref in &preferences {
            let mgr = self.providers.read().await;

            // Skip unhealthy or at-capacity providers.
            if !mgr.is_healthy(&pref.provider_name) {
                debug!(
                    provider = %pref.provider_name,
                    agent = %role_id,
                    "skipping unhealthy provider"
                );
                continue;
            }
            if mgr.is_at_capacity(&pref.provider_name) {
                debug!(
                    provider = %pref.provider_name,
                    agent = %role_id,
                    "skipping provider at capacity"
                );
                continue;
            }

            let state = match mgr.get(&pref.provider_name) {
                Some(s) => s,
                None => continue,
            };

            let provider = Arc::clone(&state.provider);
            let concurrent = Arc::clone(&state.concurrent_requests);

            // Track concurrency.
            concurrent.fetch_add(1, Ordering::Relaxed);
            state.total_requests.fetch_add(1, Ordering::Relaxed);

            // Release the read lock before the async call.
            drop(mgr);

            let start = Instant::now();
            let result = provider.complete(request).await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            concurrent.fetch_sub(1, Ordering::Relaxed);

            // Record latency.
            {
                let mgr = self.providers.read().await;
                if let Some(s) = mgr.get(&pref.provider_name) {
                    s.total_latency_ms.fetch_add(elapsed_ms, Ordering::Relaxed);
                }
            }

            match result {
                Ok(response) => {
                    debug!(
                        provider = %pref.provider_name,
                        model = %pref.model_id,
                        agent = %role_id,
                        latency_ms = elapsed_ms,
                        "request succeeded"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    warn!(
                        provider = %pref.provider_name,
                        model = %pref.model_id,
                        agent = %role_id,
                        error = %e,
                        "provider failed, trying next"
                    );

                    // Record error.
                    {
                        let mgr = self.providers.read().await;
                        if let Some(s) = mgr.get(&pref.provider_name) {
                            s.total_errors.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    self.total_fallbacks.fetch_add(1, Ordering::Relaxed);
                    last_error = Some(e);
                }
            }
        }

        self.total_failures.fetch_add(1, Ordering::Relaxed);
        Err(last_error.unwrap_or_else(|| AiError::ModelNotAvailable {
            model: format!("no available provider for agent {}", role_id),
        }))
    }

    /// Get health and load status for all registered providers.
    pub async fn provider_status(&self) -> Vec<ProviderStatus> {
        let mgr = self.providers.read().await;
        mgr.all_statuses()
    }

    /// Get aggregate routing metrics.
    pub async fn metrics(&self) -> RouterMetrics {
        let cache_metrics = {
            let cache = self.cache.read().await;
            cache.metrics()
        };
        let providers = self.provider_status().await;

        RouterMetrics {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_fallbacks: self.total_fallbacks.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            cache: cache_metrics,
            providers,
        }
    }

    /// Override or add a custom routing rule for an agent role.
    pub fn set_routing_rule(&mut self, rule: RoutingRule) {
        self.routing_table
            .insert(rule.agent_role.clone(), rule);
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Resolve the ordered list of provider preferences for a given agent role.
    fn resolve_preferences(&self, role_id: &str) -> Vec<ProviderPreference> {
        if let Some(rule) = self.routing_table.get(role_id) {
            let mut prefs = rule.preferred_providers.clone();
            prefs.sort_by_key(|p| p.priority);
            return prefs;
        }

        // Fall back to generic chain.
        self.fallback_chain
            .iter()
            .enumerate()
            .map(|(i, name)| ProviderPreference {
                provider_name: name.clone(),
                model_id: String::new(),
                priority: i as u8,
            })
            .collect()
    }

    /// Compute a deterministic cache key from the request content.
    fn compute_cache_key(request: &UnifiedRequest) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        request.model.hash(&mut hasher);
        for msg in &request.messages {
            msg.role.to_string().hash(&mut hasher);
            msg.content.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Build the default routing table based on config and the agent-role routing spec.
    fn build_default_routing_table(config: &RouterConfig) -> HashMap<String, RoutingRule> {
        let mut table = HashMap::new();

        // CTO + Security: critical decisions.
        // Try Claude Opus -> deepseek-coder-v2 on Ollama -> OpenRouter llama-3.1-70b
        for role in &["cto", "security"] {
            let mut prefs = Vec::new();
            if config.enable_anthropic {
                prefs.push(ProviderPreference {
                    provider_name: "anthropic".to_string(),
                    model_id: "claude-opus-4-6".to_string(),
                    priority: 0,
                });
            }
            if config.enable_ollama {
                prefs.push(ProviderPreference {
                    provider_name: "ollama".to_string(),
                    model_id: "deepseek-coder-v2:16b".to_string(),
                    priority: 1,
                });
            }
            if config.enable_openrouter {
                prefs.push(ProviderPreference {
                    provider_name: "openrouter".to_string(),
                    model_id: "meta-llama/llama-3.1-70b-instruct".to_string(),
                    priority: 2,
                });
            }
            table.insert(
                role.to_string(),
                RoutingRule {
                    agent_role: role.to_string(),
                    preferred_providers: prefs,
                    max_latency_ms: 60_000,
                    require_tool_use: true,
                    min_context_window: 128_000,
                },
            );
        }

        // Architect: Claude Sonnet -> Ollama llama-3.1 -> OpenRouter
        {
            let mut prefs = Vec::new();
            if config.enable_anthropic {
                prefs.push(ProviderPreference {
                    provider_name: "anthropic".to_string(),
                    model_id: "claude-sonnet-4-6".to_string(),
                    priority: 0,
                });
            }
            if config.enable_ollama {
                prefs.push(ProviderPreference {
                    provider_name: "ollama".to_string(),
                    model_id: "llama3.1:latest".to_string(),
                    priority: 1,
                });
            }
            if config.enable_openrouter {
                prefs.push(ProviderPreference {
                    provider_name: "openrouter".to_string(),
                    model_id: "meta-llama/llama-3.1-70b-instruct".to_string(),
                    priority: 2,
                });
            }
            table.insert(
                "architect".to_string(),
                RoutingRule {
                    agent_role: "architect".to_string(),
                    preferred_providers: prefs,
                    max_latency_ms: 45_000,
                    require_tool_use: true,
                    min_context_window: 128_000,
                },
            );
        }

        // Backend + Frontend: code generation — prefer local Ollama for speed.
        // Ollama codellama/deepseek-coder -> Claude Sonnet
        for role in &["backend", "frontend"] {
            let mut prefs = Vec::new();
            if config.enable_ollama {
                prefs.push(ProviderPreference {
                    provider_name: "ollama".to_string(),
                    model_id: "deepseek-coder-v2:16b".to_string(),
                    priority: 0,
                });
            }
            if config.enable_anthropic {
                prefs.push(ProviderPreference {
                    provider_name: "anthropic".to_string(),
                    model_id: "claude-sonnet-4-6".to_string(),
                    priority: 1,
                });
            }
            table.insert(
                role.to_string(),
                RoutingRule {
                    agent_role: role.to_string(),
                    preferred_providers: prefs,
                    max_latency_ms: 30_000,
                    require_tool_use: false,
                    min_context_window: 32_000,
                },
            );
        }

        // DevOps + QA: Ollama mistral/llama3.1 -> OpenRouter free -> Claude Haiku
        for role in &["devops", "qa"] {
            let mut prefs = Vec::new();
            if config.enable_ollama {
                prefs.push(ProviderPreference {
                    provider_name: "ollama".to_string(),
                    model_id: "mistral:latest".to_string(),
                    priority: 0,
                });
            }
            if config.enable_openrouter {
                prefs.push(ProviderPreference {
                    provider_name: "openrouter".to_string(),
                    model_id: "meta-llama/llama-3.1-8b-instruct:free".to_string(),
                    priority: 1,
                });
            }
            if config.enable_anthropic {
                prefs.push(ProviderPreference {
                    provider_name: "anthropic".to_string(),
                    model_id: "claude-haiku-4-5-20251001".to_string(),
                    priority: 2,
                });
            }
            table.insert(
                role.to_string(),
                RoutingRule {
                    agent_role: role.to_string(),
                    preferred_providers: prefs,
                    max_latency_ms: 30_000,
                    require_tool_use: false,
                    min_context_window: 32_000,
                },
            );
        }

        // Monitor: lightweight — Ollama phi3-mini -> any small free model
        {
            let mut prefs = Vec::new();
            if config.enable_ollama {
                prefs.push(ProviderPreference {
                    provider_name: "ollama".to_string(),
                    model_id: "phi3:mini".to_string(),
                    priority: 0,
                });
            }
            if config.enable_openrouter {
                prefs.push(ProviderPreference {
                    provider_name: "openrouter".to_string(),
                    model_id: "meta-llama/llama-3.1-8b-instruct:free".to_string(),
                    priority: 1,
                });
            }
            if config.enable_anthropic {
                prefs.push(ProviderPreference {
                    provider_name: "anthropic".to_string(),
                    model_id: "claude-haiku-4-5-20251001".to_string(),
                    priority: 2,
                });
            }
            table.insert(
                "monitor".to_string(),
                RoutingRule {
                    agent_role: "monitor".to_string(),
                    preferred_providers: prefs,
                    max_latency_ms: 15_000,
                    require_tool_use: false,
                    min_context_window: 8_000,
                },
            );
        }

        table
    }

    /// Build the default fallback chain (used when no specific routing rule matches).
    fn build_default_fallback_chain(config: &RouterConfig) -> Vec<String> {
        let mut chain = Vec::new();
        if config.enable_anthropic {
            chain.push("anthropic".to_string());
        }
        if config.enable_ollama {
            chain.push("ollama".to_string());
        }
        if config.enable_openrouter {
            chain.push("openrouter".to_string());
        }
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::UnifiedUsage;

    #[test]
    fn test_router_config_defaults() {
        let config = RouterConfig::default();
        assert!(config.enable_ollama);
        assert!(config.enable_anthropic);
        assert!(!config.enable_openrouter);
        assert_eq!(config.cache_ttl_seconds, 300);
        assert_eq!(config.cache_max_entries, 512);
        assert_eq!(config.health_check_interval_seconds, 30);
        assert_eq!(config.max_concurrent_per_provider, 16);
    }

    fn make_test_response(content: &str) -> UnifiedResponse {
        UnifiedResponse {
            id: "test-id".to_string(),
            model: "test".to_string(),
            content: content.to_string(),
            stop_reason: None,
            usage: UnifiedUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            },
            provider: "test-provider".to_string(),
        }
    }

    #[test]
    fn test_response_cache_insert_and_get() {
        let mut cache = ResponseCache::new(2, Duration::from_secs(60));

        let response = make_test_response("hello");

        cache.insert(42, response.clone());
        assert!(cache.get(42).is_some());
        assert_eq!(cache.get(42).map(|r| r.content.as_str()), Some("hello"));
        assert!(cache.get(99).is_none());
    }

    #[test]
    fn test_response_cache_eviction() {
        let mut cache = ResponseCache::new(2, Duration::from_secs(60));

        cache.insert(1, make_test_response("first"));
        cache.insert(2, make_test_response("second"));
        cache.insert(3, make_test_response("third")); // should evict key=1

        assert!(cache.get(1).is_none());
        assert!(cache.get(2).is_some());
        assert!(cache.get(3).is_some());
    }

    #[test]
    fn test_cache_metrics() {
        let mut cache = ResponseCache::new(10, Duration::from_secs(60));

        let response = make_test_response("test");

        cache.insert(1, response);

        // Hit
        let _ = cache.get(1);
        // Miss
        let _ = cache.get(2);

        let m = cache.metrics();
        assert_eq!(m.hits, 1);
        assert_eq!(m.misses, 1);
        assert_eq!(m.entries, 1);
        assert!((m.hit_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_default_routing_table_has_all_roles() {
        let config = RouterConfig::default();
        let table = ModelRouter::build_default_routing_table(&config);

        assert!(table.contains_key("cto"));
        assert!(table.contains_key("security"));
        assert!(table.contains_key("architect"));
        assert!(table.contains_key("backend"));
        assert!(table.contains_key("frontend"));
        assert!(table.contains_key("devops"));
        assert!(table.contains_key("qa"));
        assert!(table.contains_key("monitor"));
    }

    #[test]
    fn test_cto_prefers_anthropic_first() {
        let config = RouterConfig {
            enable_anthropic: true,
            enable_ollama: true,
            enable_openrouter: true,
            ..RouterConfig::default()
        };
        let table = ModelRouter::build_default_routing_table(&config);
        let rule = table.get("cto").expect("cto rule");
        let mut prefs = rule.preferred_providers.clone();
        prefs.sort_by_key(|p| p.priority);
        assert_eq!(prefs[0].provider_name, "anthropic");
        assert_eq!(prefs[0].model_id, "claude-opus-4-6");
    }

    #[test]
    fn test_backend_prefers_ollama_first() {
        let config = RouterConfig {
            enable_anthropic: true,
            enable_ollama: true,
            ..RouterConfig::default()
        };
        let table = ModelRouter::build_default_routing_table(&config);
        let rule = table.get("backend").expect("backend rule");
        let mut prefs = rule.preferred_providers.clone();
        prefs.sort_by_key(|p| p.priority);
        assert_eq!(prefs[0].provider_name, "ollama");
    }

    #[test]
    fn test_monitor_uses_lightweight_models() {
        let config = RouterConfig {
            enable_ollama: true,
            enable_openrouter: true,
            enable_anthropic: true,
            ..RouterConfig::default()
        };
        let table = ModelRouter::build_default_routing_table(&config);
        let rule = table.get("monitor").expect("monitor rule");
        let mut prefs = rule.preferred_providers.clone();
        prefs.sort_by_key(|p| p.priority);
        assert_eq!(prefs[0].model_id, "phi3:mini");
    }

    #[test]
    fn test_fallback_chain_order() {
        let config = RouterConfig {
            enable_anthropic: true,
            enable_ollama: true,
            enable_openrouter: true,
            ..RouterConfig::default()
        };
        let chain = ModelRouter::build_default_fallback_chain(&config);
        assert_eq!(chain, vec!["anthropic", "ollama", "openrouter"]);
    }

    #[test]
    fn test_disabled_providers_excluded() {
        let config = RouterConfig {
            enable_anthropic: false,
            enable_ollama: true,
            enable_openrouter: false,
            ..RouterConfig::default()
        };
        let table = ModelRouter::build_default_routing_table(&config);
        let rule = table.get("cto").expect("cto rule");
        assert!(rule
            .preferred_providers
            .iter()
            .all(|p| p.provider_name != "anthropic"));
        assert!(rule
            .preferred_providers
            .iter()
            .all(|p| p.provider_name != "openrouter"));
    }

    #[test]
    fn test_routing_rule_override() {
        let config = RouterConfig::default();
        let mut router = ModelRouter::new(config);

        let custom_rule = RoutingRule {
            agent_role: "cto".to_string(),
            preferred_providers: vec![ProviderPreference {
                provider_name: "custom".to_string(),
                model_id: "custom-model".to_string(),
                priority: 0,
            }],
            max_latency_ms: 10_000,
            require_tool_use: false,
            min_context_window: 4_000,
        };

        router.set_routing_rule(custom_rule);

        let prefs = router.resolve_preferences("cto");
        assert_eq!(prefs.len(), 1);
        assert_eq!(prefs[0].provider_name, "custom");
    }

    #[test]
    fn test_provider_status_defaults() {
        let config = RouterConfig::default();
        let mgr = ProviderManager::new(config);
        assert!(mgr.all_statuses().is_empty());
    }
}
