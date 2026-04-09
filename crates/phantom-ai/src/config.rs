//! Production configuration for Phantom AI providers and routing.
//!
//! Supports multi-provider setups (Ollama local, OpenRouter, Anthropic direct)
//! with per-agent routing rules, fallback chains, performance tuning, and cost control.
//!
//! Zero-config default works with local Ollama for development.
//! Production deployments configure via environment variables.

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::agents::AgentRole;

// ── Top-Level Config ────────────────────────────────────────────────────────

/// Production configuration for Phantom AI providers and routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomAiConfig {
    /// Provider configurations
    pub providers: ProvidersConfig,
    /// Model routing rules per agent
    pub routing: RoutingConfig,
    /// Performance settings
    pub performance: PerformanceConfig,
    /// Cost control
    pub cost: CostConfig,
}

impl PhantomAiConfig {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `PHANTOM_OLLAMA_URL` — Ollama base URL (default: `http://localhost:11434`)
    /// - `PHANTOM_OLLAMA_ENABLED` — Enable Ollama (default: `true`)
    /// - `PHANTOM_OPENROUTER_KEY` — OpenRouter API key
    /// - `PHANTOM_OPENROUTER_ENABLED` — Enable OpenRouter (default: `true` if key set)
    /// - `PHANTOM_ANTHROPIC_KEY` or `ANTHROPIC_API_KEY` — Anthropic API key
    /// - `PHANTOM_ANTHROPIC_ENABLED` — Enable Anthropic (default: `true` if key set)
    /// - `PHANTOM_CACHE_TTL` — Response cache TTL in seconds (default: `300`)
    /// - `PHANTOM_MAX_CONCURRENT` — Max concurrent LLM requests (default: `8`)
    /// - `PHANTOM_BUDGET_LIMIT_USD` — Total budget limit in USD (default: `10.0`)
    pub fn from_env() -> Self {
        let ollama_url = env::var("PHANTOM_OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let ollama_enabled = env::var("PHANTOM_OLLAMA_ENABLED")
            .map(|v| parse_bool(&v, true))
            .unwrap_or(true);

        let openrouter_key = env::var("PHANTOM_OPENROUTER_KEY").ok();
        let openrouter_enabled = env::var("PHANTOM_OPENROUTER_ENABLED")
            .map(|v| parse_bool(&v, openrouter_key.is_some()))
            .unwrap_or_else(|_| openrouter_key.is_some());

        let anthropic_key = env::var("PHANTOM_ANTHROPIC_KEY")
            .or_else(|_| env::var("ANTHROPIC_API_KEY"))
            .ok();
        let anthropic_enabled = env::var("PHANTOM_ANTHROPIC_ENABLED")
            .map(|v| parse_bool(&v, anthropic_key.is_some()))
            .unwrap_or_else(|_| anthropic_key.is_some());

        let cache_ttl_secs: u64 = env::var("PHANTOM_CACHE_TTL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);

        let max_concurrent: usize = env::var("PHANTOM_MAX_CONCURRENT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8);

        let budget_limit: f64 = env::var("PHANTOM_BUDGET_LIMIT_USD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10.0);

        let providers = ProvidersConfig {
            ollama: OllamaConfig {
                enabled: ollama_enabled,
                base_url: ollama_url,
            },
            openrouter: OpenRouterConfig {
                enabled: openrouter_enabled,
                api_key: openrouter_key,
                base_url: "https://openrouter.ai/api/v1".to_string(),
            },
            anthropic: AnthropicConfig {
                enabled: anthropic_enabled,
                api_key: anthropic_key,
                base_url: "https://api.anthropic.com".to_string(),
            },
        };

        let routing = RoutingConfig::default_routing(&providers);

        let performance = PerformanceConfig {
            connection_pool_size: 16,
            request_timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(10),
            max_concurrent_requests: max_concurrent,
            batch_size: 4,
            cache_ttl: Duration::from_secs(cache_ttl_secs),
            retry_max_attempts: 3,
            retry_base_delay: Duration::from_millis(500),
        };

        let cost = CostConfig {
            model_costs: default_model_costs(),
            total_budget_usd: budget_limit,
            per_agent_budgets: default_per_agent_budgets(budget_limit),
            alert_threshold_percent: 80.0,
        };

        Self {
            providers,
            routing,
            performance,
            cost,
        }
    }
}

impl Default for PhantomAiConfig {
    /// Default config for local development — Ollama only, zero external dependencies.
    fn default() -> Self {
        let providers = ProvidersConfig {
            ollama: OllamaConfig {
                enabled: true,
                base_url: "http://localhost:11434".to_string(),
            },
            openrouter: OpenRouterConfig {
                enabled: false,
                api_key: None,
                base_url: "https://openrouter.ai/api/v1".to_string(),
            },
            anthropic: AnthropicConfig {
                enabled: false,
                api_key: None,
                base_url: "https://api.anthropic.com".to_string(),
            },
        };

        let routing = RoutingConfig::default_routing(&providers);

        let performance = PerformanceConfig {
            connection_pool_size: 8,
            request_timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(10),
            max_concurrent_requests: 4,
            batch_size: 2,
            cache_ttl: Duration::from_secs(300),
            retry_max_attempts: 3,
            retry_base_delay: Duration::from_millis(500),
        };

        let cost = CostConfig {
            model_costs: default_model_costs(),
            total_budget_usd: 10.0,
            per_agent_budgets: default_per_agent_budgets(10.0),
            alert_threshold_percent: 80.0,
        };

        Self {
            providers,
            routing,
            performance,
            cost,
        }
    }
}

// ── Provider Configurations ─────────────────────────────────────────────────

/// Provider configurations for all supported LLM backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Local Ollama instance
    pub ollama: OllamaConfig,
    /// OpenRouter cloud provider
    pub openrouter: OpenRouterConfig,
    /// Anthropic direct API
    pub anthropic: AnthropicConfig,
}

/// Ollama local provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Whether Ollama is enabled
    pub enabled: bool,
    /// Base URL for the Ollama API
    pub base_url: String,
}

/// OpenRouter cloud provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    /// Whether OpenRouter is enabled
    pub enabled: bool,
    /// API key (loaded from env, never serialized)
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    /// Base URL for the OpenRouter API
    pub base_url: String,
}

/// Anthropic direct API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Whether Anthropic is enabled
    pub enabled: bool,
    /// API key (loaded from env, never serialized)
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    /// Base URL for the Anthropic API
    pub base_url: String,
}

// ── Routing Configuration ───────────────────────────────────────────────────

/// Model routing rules — determines which model each agent uses and fallback chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Per-agent model preferences. Key is agent role ID (e.g. "cto", "backend").
    pub agent_routes: HashMap<String, AgentRoute>,
    /// Default route for agents without explicit configuration
    pub default_route: AgentRoute,
}

/// Routing configuration for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoute {
    /// Primary model identifier (e.g. "claude-opus-4-6", "llama3.1:70b")
    pub primary_model: String,
    /// Provider to use for the primary model
    pub primary_provider: ProviderKind,
    /// Fallback chain: tried in order if the primary fails
    pub fallback_chain: Vec<FallbackEntry>,
}

/// A fallback entry in the routing chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEntry {
    /// Model identifier
    pub model: String,
    /// Provider
    pub provider: ProviderKind,
}

/// Supported LLM provider kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Ollama,
    OpenRouter,
    Anthropic,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ollama => write!(f, "ollama"),
            Self::OpenRouter => write!(f, "openrouter"),
            Self::Anthropic => write!(f, "anthropic"),
        }
    }
}

impl RoutingConfig {
    /// Build default routing rules based on which providers are enabled.
    ///
    /// Priority: Anthropic > OpenRouter > Ollama.
    /// High-capability agents (CTO, Architect, Security) get the best available model.
    /// Implementation agents (Backend, Frontend, DevOps, QA) get a mid-tier model.
    /// Monitor gets the lightest model available.
    fn default_routing(providers: &ProvidersConfig) -> Self {
        let mut agent_routes = HashMap::new();

        for &role in crate::agents::ALL_ROLES {
            let route = Self::route_for_role(role, providers);
            agent_routes.insert(role.id().to_string(), route);
        }

        let default_route = Self::build_route(
            "llama3.1:8b",
            providers,
            ModelTier::Light,
        );

        Self {
            agent_routes,
            default_route,
        }
    }

    /// Get the route for a specific agent role.
    pub fn route_for(&self, role: AgentRole) -> &AgentRoute {
        self.agent_routes
            .get(role.id())
            .unwrap_or(&self.default_route)
    }

    fn route_for_role(role: AgentRole, providers: &ProvidersConfig) -> AgentRoute {
        let tier = match role {
            AgentRole::Cto | AgentRole::Architect | AgentRole::Security => ModelTier::Premium,
            AgentRole::Backend | AgentRole::Frontend | AgentRole::DevOps | AgentRole::Qa => {
                ModelTier::Standard
            }
            AgentRole::Monitor => ModelTier::Light,
        };
        Self::build_route(role.model(), providers, tier)
    }

    fn build_route(
        preferred_model: &str,
        providers: &ProvidersConfig,
        tier: ModelTier,
    ) -> AgentRoute {
        // Determine the best available provider for the primary model
        let (primary_model, primary_provider) = if providers.anthropic.enabled
            && is_anthropic_model(preferred_model)
        {
            (preferred_model.to_string(), ProviderKind::Anthropic)
        } else if providers.openrouter.enabled {
            (preferred_model.to_string(), ProviderKind::OpenRouter)
        } else {
            // Ollama fallback — map Claude models to local equivalents
            let local_model = ollama_equivalent(preferred_model, tier);
            (local_model, ProviderKind::Ollama)
        };

        // Build fallback chain
        let mut fallback_chain = Vec::new();

        // If primary is Anthropic, fall back to OpenRouter then Ollama
        if primary_provider == ProviderKind::Anthropic && providers.openrouter.enabled {
            fallback_chain.push(FallbackEntry {
                model: preferred_model.to_string(),
                provider: ProviderKind::OpenRouter,
            });
        }

        if primary_provider != ProviderKind::Ollama && providers.ollama.enabled {
            let local = ollama_equivalent(preferred_model, tier);
            fallback_chain.push(FallbackEntry {
                model: local,
                provider: ProviderKind::Ollama,
            });
        }

        AgentRoute {
            primary_model,
            primary_provider,
            fallback_chain,
        }
    }
}

/// Model tier for routing decisions.
#[derive(Debug, Clone, Copy)]
enum ModelTier {
    Premium,
    Standard,
    Light,
}

fn is_anthropic_model(model: &str) -> bool {
    model.starts_with("claude-")
}

fn ollama_equivalent(model: &str, tier: ModelTier) -> String {
    if !is_anthropic_model(model) {
        return model.to_string();
    }
    match tier {
        ModelTier::Premium => "llama3.1:70b".to_string(),
        ModelTier::Standard => "llama3.1:8b".to_string(),
        ModelTier::Light => "llama3.2:3b".to_string(),
    }
}

// ── Performance Configuration ───────────────────────────────────────────────

/// Performance tuning for LLM request handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// HTTP connection pool size per provider
    pub connection_pool_size: usize,
    /// Total request timeout (including streaming)
    #[serde(with = "duration_secs")]
    pub request_timeout: Duration,
    /// TCP connection timeout
    #[serde(with = "duration_secs")]
    pub connect_timeout: Duration,
    /// Maximum concurrent LLM requests across all providers
    pub max_concurrent_requests: usize,
    /// Batch size for parallel task submission
    pub batch_size: usize,
    /// Response cache TTL
    #[serde(with = "duration_secs")]
    pub cache_ttl: Duration,
    /// Maximum retry attempts per request
    pub retry_max_attempts: u32,
    /// Base delay for exponential backoff retries
    #[serde(with = "duration_millis")]
    pub retry_base_delay: Duration,
}

// ── Cost Configuration ──────────────────────────────────────────────────────

/// Cost control and budget management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Cost per 1M tokens for each model (input_cost, output_cost)
    pub model_costs: HashMap<String, ModelCost>,
    /// Total budget limit in USD for the entire run
    pub total_budget_usd: f64,
    /// Per-agent budget limits in USD. Key is agent role ID.
    pub per_agent_budgets: HashMap<String, f64>,
    /// Alert when spending exceeds this percentage of budget (0-100)
    pub alert_threshold_percent: f64,
}

/// Cost rates for a specific model (per 1M tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    /// Cost per 1M input tokens in USD
    pub input_per_1m: f64,
    /// Cost per 1M output tokens in USD
    pub output_per_1m: f64,
}

impl ModelCost {
    /// Calculate cost for a given number of input and output tokens.
    pub fn calculate(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 * self.input_per_1m / 1_000_000.0)
            + (output_tokens as f64 * self.output_per_1m / 1_000_000.0)
    }
}

impl CostConfig {
    /// Calculate the cost for a model usage.
    pub fn cost_for(&self, model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        self.model_costs
            .get(model)
            .map(|c| c.calculate(input_tokens, output_tokens))
            .unwrap_or(0.0)
    }

    /// Check if an agent has exceeded its budget.
    pub fn is_over_budget(&self, agent_role: &str, spent_usd: f64) -> bool {
        self.per_agent_budgets
            .get(agent_role)
            .map(|&limit| spent_usd >= limit)
            .unwrap_or(false)
    }

    /// Check if the total budget has been exceeded.
    pub fn is_total_over_budget(&self, total_spent_usd: f64) -> bool {
        total_spent_usd >= self.total_budget_usd
    }

    /// Check if spending has crossed the alert threshold.
    pub fn should_alert(&self, total_spent_usd: f64) -> bool {
        let threshold = self.total_budget_usd * (self.alert_threshold_percent / 100.0);
        total_spent_usd >= threshold
    }
}

fn default_model_costs() -> HashMap<String, ModelCost> {
    let mut costs = HashMap::new();

    // Anthropic models (April 2025 pricing)
    costs.insert(
        "claude-opus-4-6".to_string(),
        ModelCost {
            input_per_1m: 15.0,
            output_per_1m: 75.0,
        },
    );
    costs.insert(
        "claude-sonnet-4-6".to_string(),
        ModelCost {
            input_per_1m: 3.0,
            output_per_1m: 15.0,
        },
    );
    costs.insert(
        "claude-haiku-4-5-20251001".to_string(),
        ModelCost {
            input_per_1m: 0.80,
            output_per_1m: 4.0,
        },
    );

    // Ollama models (free / local)
    costs.insert(
        "llama3.1:70b".to_string(),
        ModelCost {
            input_per_1m: 0.0,
            output_per_1m: 0.0,
        },
    );
    costs.insert(
        "llama3.1:8b".to_string(),
        ModelCost {
            input_per_1m: 0.0,
            output_per_1m: 0.0,
        },
    );
    costs.insert(
        "llama3.2:3b".to_string(),
        ModelCost {
            input_per_1m: 0.0,
            output_per_1m: 0.0,
        },
    );

    costs
}

fn default_per_agent_budgets(total: f64) -> HashMap<String, f64> {
    let mut budgets = HashMap::new();

    // Allocate budget proportionally to agent token budgets
    budgets.insert("cto".to_string(), total * 0.20);
    budgets.insert("architect".to_string(), total * 0.15);
    budgets.insert("backend".to_string(), total * 0.15);
    budgets.insert("frontend".to_string(), total * 0.15);
    budgets.insert("devops".to_string(), total * 0.10);
    budgets.insert("qa".to_string(), total * 0.10);
    budgets.insert("security".to_string(), total * 0.10);
    budgets.insert("monitor".to_string(), total * 0.05);

    budgets
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_bool(s: &str, default: bool) -> bool {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => true,
        "false" | "0" | "no" | "off" => false,
        _ => default,
    }
}

/// Serde helper: serialize/deserialize Duration as integer seconds.
mod duration_secs {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_secs())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = u64::deserialize(d)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Serde helper: serialize/deserialize Duration as integer milliseconds.
mod duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_millis() as u64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_works_without_env() {
        let config = PhantomAiConfig::default();
        assert!(config.providers.ollama.enabled);
        assert!(!config.providers.openrouter.enabled);
        assert!(!config.providers.anthropic.enabled);
        assert_eq!(
            config.providers.ollama.base_url,
            "http://localhost:11434"
        );
    }

    #[test]
    fn test_default_routing_ollama_only() {
        let config = PhantomAiConfig::default();

        // All agents should route to Ollama when it's the only provider
        let cto_route = config.routing.route_for(AgentRole::Cto);
        assert_eq!(cto_route.primary_provider, ProviderKind::Ollama);
        assert_eq!(cto_route.primary_model, "llama3.1:70b");

        let backend_route = config.routing.route_for(AgentRole::Backend);
        assert_eq!(backend_route.primary_provider, ProviderKind::Ollama);
        assert_eq!(backend_route.primary_model, "llama3.1:8b");

        let monitor_route = config.routing.route_for(AgentRole::Monitor);
        assert_eq!(monitor_route.primary_provider, ProviderKind::Ollama);
        assert_eq!(monitor_route.primary_model, "llama3.2:3b");
    }

    #[test]
    fn test_routing_with_anthropic() {
        let providers = ProvidersConfig {
            ollama: OllamaConfig {
                enabled: true,
                base_url: "http://localhost:11434".to_string(),
            },
            openrouter: OpenRouterConfig {
                enabled: false,
                api_key: None,
                base_url: "https://openrouter.ai/api/v1".to_string(),
            },
            anthropic: AnthropicConfig {
                enabled: true,
                api_key: Some("sk-test".to_string()),
                base_url: "https://api.anthropic.com".to_string(),
            },
        };

        let routing = RoutingConfig::default_routing(&providers);

        let cto_route = routing.route_for(AgentRole::Cto);
        assert_eq!(cto_route.primary_provider, ProviderKind::Anthropic);
        assert_eq!(cto_route.primary_model, "claude-opus-4-6");

        // Should have Ollama fallback
        assert!(!cto_route.fallback_chain.is_empty());
        assert_eq!(
            cto_route.fallback_chain.last().map(|f| f.provider),
            Some(ProviderKind::Ollama)
        );
    }

    #[test]
    fn test_model_cost_calculation() {
        let cost = ModelCost {
            input_per_1m: 15.0,
            output_per_1m: 75.0,
        };

        // 1000 input tokens, 500 output tokens
        let total = cost.calculate(1000, 500);
        let expected = (1000.0 * 15.0 / 1_000_000.0) + (500.0 * 75.0 / 1_000_000.0);
        assert!((total - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_cost_config_budget_checks() {
        let config = CostConfig {
            model_costs: default_model_costs(),
            total_budget_usd: 10.0,
            per_agent_budgets: default_per_agent_budgets(10.0),
            alert_threshold_percent: 80.0,
        };

        // CTO budget is 20% of $10 = $2.00
        assert!(!config.is_over_budget("cto", 1.99));
        assert!(config.is_over_budget("cto", 2.00));

        // Total budget
        assert!(!config.is_total_over_budget(9.99));
        assert!(config.is_total_over_budget(10.0));

        // Alert at 80% = $8.00
        assert!(!config.should_alert(7.99));
        assert!(config.should_alert(8.00));
    }

    #[test]
    fn test_performance_defaults() {
        let config = PhantomAiConfig::default();
        assert_eq!(config.performance.max_concurrent_requests, 4);
        assert_eq!(config.performance.request_timeout, Duration::from_secs(120));
        assert_eq!(config.performance.retry_max_attempts, 3);
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true", false));
        assert!(parse_bool("1", false));
        assert!(parse_bool("yes", false));
        assert!(parse_bool("on", false));
        assert!(!parse_bool("false", true));
        assert!(!parse_bool("0", true));
        assert!(!parse_bool("no", true));
        assert!(!parse_bool("off", true));
        assert!(parse_bool("garbage", true));
        assert!(!parse_bool("garbage", false));
    }

    #[test]
    fn test_provider_kind_display() {
        assert_eq!(ProviderKind::Ollama.to_string(), "ollama");
        assert_eq!(ProviderKind::OpenRouter.to_string(), "openrouter");
        assert_eq!(ProviderKind::Anthropic.to_string(), "anthropic");
    }

    #[test]
    fn test_config_serialization_excludes_api_keys() {
        let config = PhantomAiConfig::default();
        let json = serde_json::to_string(&config).expect("serialize");
        // API keys should not appear in serialized output
        assert!(!json.contains("api_key"));
    }

    #[test]
    fn test_all_agents_have_budgets() {
        let budgets = default_per_agent_budgets(10.0);
        for &role in crate::agents::ALL_ROLES {
            assert!(
                budgets.contains_key(role.id()),
                "missing budget for {}",
                role.id()
            );
        }
        // Budgets should sum to approximately the total
        let sum: f64 = budgets.values().sum();
        assert!((sum - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_cost_for_unknown_model() {
        let config = CostConfig {
            model_costs: default_model_costs(),
            total_budget_usd: 10.0,
            per_agent_budgets: HashMap::new(),
            alert_threshold_percent: 80.0,
        };

        // Unknown model should return 0 cost
        assert_eq!(config.cost_for("unknown-model", 1000, 1000), 0.0);
    }
}
