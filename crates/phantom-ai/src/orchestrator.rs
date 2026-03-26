//! Agent Orchestrator — Spawns, coordinates, and routes 8 Claude agents.
//!
//! Architecture Framework §8: Agent Spawning & Orchestration
//!
//! This module provides:
//!   • `AgentOrchestrator`  — spawns agents in parallel via `tokio::spawn`, manages lifecycle
//!   • `AgentWorker`        — per-agent loop: receive task → build context → call Claude → return output
//!   • `DelegationRouter`   — inter-agent delegation: CTO/Architect can request subtasks from others
//!   • `AgentOutput`        — structured response parsed from Claude's output
//!   • `OrchestratorHandle` — returned to the pipeline executor for driving tasks
//!
//! Wiring:
//!   • AGENT_ROLE and AGENT_KB_SCOPE injected into each agent's system prompt via ContextManager
//!   • Knowledge Brain chunks injected per-task from knowledge_query hints
//!   • Agent output routed back through `AgentOutput` to the pipeline executor
//!   • Token tracking aggregated across all agents via AnthropicClient

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tracing::{debug, info, instrument, warn};

use crate::agents::AgentRole;
use crate::backend::AiBackend;
use crate::client::{CompletionRequest, CompletionResponse, Message, TokenUsage};
use crate::context::{ContextManager, KnowledgeChunk};
use crate::errors::AiError;
use crate::prompts::{agent_system_prompt, task_prompt};

// ── Agent Output ────────────────────────────────────────────────────────────

/// Structured output from an agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// The agent that produced this output
    pub agent_id: String,
    /// The agent's role
    pub role: AgentRole,
    /// The task ID this output is for
    pub task_id: String,
    /// Whether the task was completed successfully
    pub success: bool,
    /// Raw text response from Claude
    pub raw_response: String,
    /// Parsed structured output (code, decisions, artifacts)
    pub structured: Option<serde_json::Value>,
    /// Error message if the task failed
    pub error: Option<String>,
    /// Input tokens consumed
    pub input_tokens: u64,
    /// Output tokens consumed
    pub output_tokens: u64,
    /// Total tokens consumed
    pub total_tokens: u64,
    /// Wall-clock execution time in seconds
    pub duration_seconds: f64,
    /// Delegation requests emitted by this agent (if any)
    pub delegations: Vec<DelegationRequest>,
    /// Knowledge chunks that were injected into context
    pub knowledge_sources: Vec<String>,
    /// The stop reason from the API
    pub stop_reason: Option<String>,
}

impl AgentOutput {
    /// Total tokens (convenience).
    pub fn tokens(&self) -> u64 {
        self.total_tokens
    }

    /// Whether this output contains delegation requests for other agents.
    pub fn has_delegations(&self) -> bool {
        !self.delegations.is_empty()
    }
}

// ── Delegation ──────────────────────────────────────────────────────────────

/// A delegation request from one agent to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRequest {
    /// Unique ID for this delegation
    pub id: String,
    /// The agent requesting the delegation
    pub from_agent: String,
    /// The target agent role to delegate to
    pub to_role: AgentRole,
    /// The parent task this delegation is part of
    pub parent_task_id: String,
    /// Description of the subtask
    pub subtask_description: String,
    /// Context from the delegating agent
    pub context: String,
    /// Priority (0 = highest)
    pub priority: u32,
    /// Whether this delegation is blocking (parent waits for result)
    pub blocking: bool,
}

/// Result of a delegation execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResult {
    /// The delegation request ID
    pub delegation_id: String,
    /// The agent output from executing the subtask
    pub output: AgentOutput,
}

// ── Task Request ────────────────────────────────────────────────────────────

/// A task submitted to the orchestrator for execution by an agent.
#[derive(Debug, Clone)]
pub struct TaskRequest {
    /// Unique task ID
    pub task_id: String,
    /// Which agent role should execute this
    pub agent_role: AgentRole,
    /// Task description
    pub description: String,
    /// Optional additional context (e.g., from previous phases)
    pub context: Option<String>,
    /// Knowledge query hint for the Knowledge Brain
    pub knowledge_query: Option<String>,
    /// Pre-fetched knowledge chunks (if already queried)
    pub knowledge_chunks: Vec<KnowledgeChunk>,
    /// Previous conversation history for multi-turn tasks
    pub history: Vec<Message>,
    /// Max tokens to allow for the response
    pub max_tokens: Option<u32>,
    /// Temperature override
    pub temperature: Option<f32>,
}

impl TaskRequest {
    /// Create a new task request.
    pub fn new(
        task_id: impl Into<String>,
        agent_role: AgentRole,
        description: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            agent_role,
            description: description.into(),
            context: None,
            knowledge_query: None,
            knowledge_chunks: Vec::new(),
            history: Vec::new(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// Set optional context.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set knowledge query hint.
    pub fn with_knowledge_query(mut self, query: impl Into<String>) -> Self {
        self.knowledge_query = Some(query.into());
        self
    }

    /// Inject pre-fetched knowledge chunks.
    pub fn with_knowledge(mut self, chunks: Vec<KnowledgeChunk>) -> Self {
        self.knowledge_chunks = chunks;
        self
    }

    /// Set conversation history for multi-turn.
    pub fn with_history(mut self, history: Vec<Message>) -> Self {
        self.history = history;
        self
    }

    /// Override max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

// ── Agent Worker ────────────────────────────────────────────────────────────

/// Internal message sent to an agent worker.
struct WorkerTask {
    request: TaskRequest,
    response_tx: oneshot::Sender<Result<AgentOutput, AiError>>,
}

/// Per-agent worker state. Each worker owns its ContextManager and receives
/// tasks via an mpsc channel.
struct AgentWorker {
    /// Agent identifier (e.g., "cto-0")
    agent_id: String,
    /// The agent's role
    role: AgentRole,
    /// Context manager for prompt assembly
    context: ContextManager,
    /// Shared Anthropic API client
    client: Arc<Mutex<AiBackend>>,
    /// Task receive channel
    task_rx: mpsc::Receiver<WorkerTask>,
    /// Delegation sender (back to orchestrator)
    delegation_tx: mpsc::Sender<(DelegationRequest, oneshot::Sender<DelegationResult>)>,
    /// Tracks per-agent cumulative tokens
    tokens_consumed: u64,
    /// Budget cap
    token_budget: u64,
    /// Tasks completed counter
    tasks_completed: u32,
}

impl AgentWorker {
    /// Create a new agent worker.
    fn new(
        agent_id: String,
        role: AgentRole,
        client: Arc<Mutex<AiBackend>>,
        task_rx: mpsc::Receiver<WorkerTask>,
        delegation_tx: mpsc::Sender<(DelegationRequest, oneshot::Sender<DelegationResult>)>,
    ) -> Self {
        let system_prompt = agent_system_prompt(role);
        let context = ContextManager::new(role, system_prompt);

        Self {
            agent_id,
            role,
            context,
            client,
            task_rx,
            delegation_tx,
            tokens_consumed: 0,
            token_budget: role.task_token_budget() * 10, // total budget = per-task × 10
            tasks_completed: 0,
        }
    }

    /// Run the worker loop — receives tasks and processes them until the
    /// channel closes.
    async fn run(mut self) {
        info!(agent = %self.agent_id, role = %self.role, "agent worker started");

        while let Some(worker_task) = self.task_rx.recv().await {
            let result = self.execute_task(worker_task.request).await;
            let _ = worker_task.response_tx.send(result);
        }

        info!(
            agent = %self.agent_id,
            tasks_completed = self.tasks_completed,
            tokens = self.tokens_consumed,
            "agent worker stopped"
        );
    }

    /// Execute a single task: build context → call API → parse output.
    #[instrument(skip(self, request), fields(agent = %self.agent_id, task = %request.task_id))]
    async fn execute_task(&mut self, request: TaskRequest) -> Result<AgentOutput, AiError> {
        let start = Instant::now();
        let task_id = request.task_id.clone();

        // Check token budget
        if self.tokens_consumed >= self.token_budget {
            return Err(AiError::TokenBudgetExhausted {
                agent_id: self.agent_id.clone(),
            });
        }

        // ── Step 1: Prepare context ────────────────────────────────────

        // Clear previous task's state
        self.context.clear_knowledge();
        self.context.clear_history();

        // Inject knowledge chunks (AGENT_KB_SCOPE is already baked into system prompt)
        let knowledge_sources: Vec<String> = request
            .knowledge_chunks
            .iter()
            .map(|c| format!("{}/{}", c.source, c.heading))
            .collect();

        self.context
            .inject_knowledge_batch(request.knowledge_chunks);

        // Load conversation history for multi-turn tasks
        for msg in &request.history {
            self.context.add_message(msg.clone());
        }

        // ── Step 2: Build the prompt ───────────────────────────────────

        let system_prompt = self.context.build_system_prompt();
        let task_text = task_prompt(self.role, &request.description, request.context.as_deref());
        let messages = self.context.build_messages(&task_text);

        debug!(
            system_tokens = self.context.system_tokens(),
            knowledge_tokens = self.context.knowledge_tokens(),
            history_tokens = self.context.history_tokens(),
            remaining = self.context.tokens_remaining(),
            "context assembled"
        );

        // ── Step 3: Call Claude API ────────────────────────────────────

        let completion_request = CompletionRequest {
            model: self.role.model().to_string(),
            messages,
            system: Some(system_prompt),
            max_tokens: request.max_tokens.unwrap_or_else(|| self.role.max_tokens()),
            temperature: Some(
                request
                    .temperature
                    .unwrap_or_else(|| self.role.temperature()),
            ),
            stop_sequences: None,
            tools: None,
        };

        let response = {
            let mut client = self.client.lock().await;
            client.complete(&completion_request, &self.agent_id).await?
        };

        // ── Step 4: Parse response ─────────────────────────────────────

        let raw_response = extract_text(&response);
        let input_tokens = response.usage.input_tokens;
        let output_tokens = response.usage.output_tokens;
        let total_tokens = input_tokens + output_tokens;
        let stop_reason = response.stop_reason.clone();

        // Update token tracking
        self.tokens_consumed += total_tokens;
        self.tasks_completed += 1;

        // Parse structured output from the response
        let (structured, delegations) = parse_agent_response(&raw_response, self.role);

        let elapsed = start.elapsed().as_secs_f64();

        info!(
            task = %task_id,
            input_tokens,
            output_tokens,
            elapsed_s = format!("{:.1}", elapsed),
            delegations = delegations.len(),
            "task completed"
        );

        // ── Step 5: Execute blocking delegations ───────────────────────

        let resolved_delegations = delegations.clone();
        let mut delegation_outputs = Vec::new();

        for delegation in &resolved_delegations {
            if delegation.blocking {
                debug!(
                    from = %self.agent_id,
                    to = %delegation.to_role,
                    subtask = %delegation.subtask_description,
                    "executing blocking delegation"
                );

                let (result_tx, result_rx) = oneshot::channel();
                if self
                    .delegation_tx
                    .send((delegation.clone(), result_tx))
                    .await
                    .is_ok()
                {
                    match result_rx.await {
                        Ok(result) => delegation_outputs.push(result),
                        Err(_) => {
                            warn!(delegation_id = %delegation.id, "delegation channel dropped");
                        }
                    }
                }
            }
        }

        // If we received delegation results, we could do a follow-up call to
        // synthesize them. For now, attach them to the output.
        let delegation_context = if !delegation_outputs.is_empty() {
            let summaries: Vec<String> = delegation_outputs
                .iter()
                .map(|d| {
                    format!(
                        "[{} from {}]: {}",
                        d.delegation_id,
                        d.output.agent_id,
                        d.output.raw_response.chars().take(500).collect::<String>()
                    )
                })
                .collect();
            Some(serde_json::json!({
                "delegation_results": summaries,
            }))
        } else {
            None
        };

        // Merge delegation context into structured output
        let final_structured = match (structured, delegation_context) {
            (Some(mut s), Some(d)) => {
                if let Some(obj) = s.as_object_mut() {
                    obj.insert("delegations_received".into(), d);
                }
                Some(s)
            }
            (s, None) => s,
            (None, Some(d)) => Some(d),
        };

        Ok(AgentOutput {
            agent_id: self.agent_id.clone(),
            role: self.role,
            task_id,
            success: true,
            raw_response,
            structured: final_structured,
            error: None,
            input_tokens,
            output_tokens,
            total_tokens,
            duration_seconds: elapsed,
            delegations: resolved_delegations,
            knowledge_sources,
            stop_reason,
        })
    }
}

// ── Delegation Router ───────────────────────────────────────────────────────

/// Routes delegation requests from one agent to another.
///
/// Only CTO and Architect agents are allowed to delegate. The router
/// validates the delegation, finds an appropriate worker, and returns
/// the result.
struct DelegationRouter {
    /// Worker senders indexed by role
    worker_senders: HashMap<AgentRole, mpsc::Sender<WorkerTask>>,
}

impl DelegationRouter {
    fn new(worker_senders: HashMap<AgentRole, mpsc::Sender<WorkerTask>>) -> Self {
        Self { worker_senders }
    }

    /// Handle an incoming delegation request.
    async fn route(&self, request: DelegationRequest) -> Result<DelegationResult, AiError> {
        // Validate: only CTO and Architect can delegate
        let from_role = parse_role_from_agent_id(&request.from_agent);
        if let Some(role) = from_role {
            if !role.can_delegate() {
                return Err(AiError::RequestFailed(format!(
                    "agent {} ({}) is not allowed to delegate tasks",
                    request.from_agent, role
                )));
            }
        }

        // Find the target worker
        let sender = self.worker_senders.get(&request.to_role).ok_or_else(|| {
            AiError::RequestFailed(format!("no worker available for role {}", request.to_role))
        })?;

        // Build a TaskRequest from the delegation
        let task_request = TaskRequest::new(
            format!("delegation-{}", request.id),
            request.to_role,
            &request.subtask_description,
        )
        .with_context(format!(
            "DELEGATION FROM {}: {}\n\nPARENT TASK: {}",
            request.from_agent, request.context, request.parent_task_id,
        ));

        // Send to the target worker
        let (response_tx, response_rx) = oneshot::channel();
        sender
            .send(WorkerTask {
                request: task_request,
                response_tx,
            })
            .await
            .map_err(|_| AiError::RequestFailed("worker channel closed".into()))?;

        // Wait for result
        let output = response_rx
            .await
            .map_err(|_| AiError::RequestFailed("worker dropped response".into()))??;

        Ok(DelegationResult {
            delegation_id: request.id,
            output,
        })
    }
}

// ── Agent Orchestrator ──────────────────────────────────────────────────────

/// Configuration for the orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum concurrent tasks per agent
    pub worker_queue_size: usize,
    /// Maximum delegation depth (prevents infinite loops)
    pub max_delegation_depth: u32,
    /// Timeout per task in seconds
    pub task_timeout_seconds: u64,
    /// Whether to enable delegation routing
    pub enable_delegation: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            worker_queue_size: 16,
            max_delegation_depth: 3,
            task_timeout_seconds: 300,
            enable_delegation: true,
        }
    }
}

/// Token usage summary across all agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrchestratorUsage {
    pub per_agent: HashMap<String, AgentUsageStats>,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_requests: u64,
    pub total_delegations: u64,
}

/// Per-agent usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentUsageStats {
    pub role: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub requests: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub delegations_sent: u64,
    pub delegations_received: u64,
    pub avg_duration_seconds: f64,
}

/// The Agent Orchestrator — spawns 8 Claude-backed agents, routes tasks,
/// and handles inter-agent delegation.
///
/// # Usage
///
/// ```ignore
/// let client = AiBackend::auto_detect()?;
/// let orch = AgentOrchestrator::new(client, OrchestratorConfig::default());
/// let handle = orch.start().await;
///
/// // Submit a task
/// let output = handle.submit_task(TaskRequest::new(
///     "task-1", AgentRole::Backend, "Implement user auth"
/// )).await?;
///
/// // Shut down
/// handle.shutdown().await;
/// ```
pub struct AgentOrchestrator {
    /// Shared Anthropic client
    client: Arc<Mutex<AiBackend>>,
    /// Configuration
    config: OrchestratorConfig,
}

impl AgentOrchestrator {
    /// Create a new orchestrator.
    pub fn new(client: AiBackend, config: OrchestratorConfig) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            config,
        }
    }

    /// Create with default config.
    pub fn with_client(client: AiBackend) -> Self {
        Self::new(client, OrchestratorConfig::default())
    }

    /// Start the orchestrator — spawns 8 agent workers and the delegation router.
    /// Returns an `OrchestratorHandle` for submitting tasks and controlling lifecycle.
    pub async fn start(self) -> OrchestratorHandle {
        info!("starting agent orchestrator with 8 agents");

        // Delegation channel (workers → router)
        let (delegation_tx, mut delegation_rx) =
            mpsc::channel::<(DelegationRequest, oneshot::Sender<DelegationResult>)>(32);

        // Per-role worker senders
        let mut worker_senders: HashMap<AgentRole, mpsc::Sender<WorkerTask>> = HashMap::new();
        let mut worker_handles = Vec::new();

        // Spawn one worker per role
        for role in crate::agents::ALL_ROLES {
            let (task_tx, task_rx) = mpsc::channel::<WorkerTask>(self.config.worker_queue_size);
            let agent_id = format!("{}-0", role.id());

            let worker = AgentWorker::new(
                agent_id.clone(),
                *role,
                Arc::clone(&self.client),
                task_rx,
                delegation_tx.clone(),
            );

            let handle = tokio::spawn(worker.run());
            worker_handles.push(handle);
            worker_senders.insert(*role, task_tx);

            debug!(agent = %agent_id, role = %role, "spawned agent worker");
        }

        // Spawn the delegation router
        let router_senders = worker_senders.clone();
        let router_handle = tokio::spawn(async move {
            let router = DelegationRouter::new(router_senders);

            while let Some((request, response_tx)) = delegation_rx.recv().await {
                let delegation_id = request.id.clone();
                debug!(
                    delegation = %delegation_id,
                    from = %request.from_agent,
                    to = %request.to_role,
                    "routing delegation"
                );

                match router.route(request).await {
                    Ok(result) => {
                        let _ = response_tx.send(result);
                    }
                    Err(e) => {
                        warn!(delegation = %delegation_id, error = %e, "delegation failed");
                        // Send a failure result
                        let _ = response_tx.send(DelegationResult {
                            delegation_id: delegation_id.clone(),
                            output: AgentOutput {
                                agent_id: "delegation-router".into(),
                                role: AgentRole::Cto,
                                task_id: delegation_id,
                                success: false,
                                raw_response: String::new(),
                                structured: None,
                                error: Some(e.to_string()),
                                input_tokens: 0,
                                output_tokens: 0,
                                total_tokens: 0,
                                duration_seconds: 0.0,
                                delegations: Vec::new(),
                                knowledge_sources: Vec::new(),
                                stop_reason: None,
                            },
                        });
                    }
                }
            }
        });

        // Build the usage tracker
        let usage = Arc::new(RwLock::new(OrchestratorUsage::default()));

        OrchestratorHandle {
            worker_senders,
            usage,
            client: Arc::clone(&self.client),
            config: self.config,
            _worker_handles: worker_handles,
            _router_handle: router_handle,
        }
    }
}

// ── Orchestrator Handle ─────────────────────────────────────────────────────

/// Handle returned by `AgentOrchestrator::start()` — used to submit tasks,
/// query usage, and shut down.
pub struct OrchestratorHandle {
    /// Per-role task senders
    worker_senders: HashMap<AgentRole, mpsc::Sender<WorkerTask>>,
    /// Aggregated usage stats
    usage: Arc<RwLock<OrchestratorUsage>>,
    /// Client ref for usage queries
    client: Arc<Mutex<AiBackend>>,
    /// Config
    config: OrchestratorConfig,
    /// Worker join handles (kept alive)
    _worker_handles: Vec<tokio::task::JoinHandle<()>>,
    /// Router join handle
    _router_handle: tokio::task::JoinHandle<()>,
}

impl OrchestratorHandle {
    /// Submit a task to the appropriate agent worker.
    /// Returns the agent's output once complete.
    #[instrument(skip(self, request), fields(task = %request.task_id, role = %request.agent_role))]
    pub async fn submit_task(&self, request: TaskRequest) -> Result<AgentOutput, AiError> {
        let role = request.agent_role;
        let task_id = request.task_id.clone();

        let sender = self
            .worker_senders
            .get(&role)
            .ok_or_else(|| AiError::RequestFailed(format!("no worker for role {}", role)))?;

        let (response_tx, response_rx) = oneshot::channel();

        sender
            .send(WorkerTask {
                request,
                response_tx,
            })
            .await
            .map_err(|_| AiError::RequestFailed(format!("worker {} channel closed", role)))?;

        // Wait with timeout
        let timeout = Duration::from_secs(self.config.task_timeout_seconds);
        let result = tokio::time::timeout(timeout, response_rx).await;

        match result {
            Ok(Ok(output)) => {
                // Update usage stats
                if let Ok(ref agent_output) = output {
                    self.record_usage(agent_output).await;
                }
                output
            }
            Ok(Err(_)) => Err(AiError::RequestFailed(format!(
                "worker dropped response for task {}",
                task_id
            ))),
            Err(_) => Err(AiError::AgentTimeout {
                agent_id: format!("{}-0", role.id()),
            }),
        }
    }

    /// Submit multiple tasks in parallel (one per agent role).
    /// Returns results in the same order as inputs.
    pub async fn submit_parallel(
        &self,
        requests: Vec<TaskRequest>,
    ) -> Vec<Result<AgentOutput, AiError>> {
        let mut handles = Vec::with_capacity(requests.len());

        for request in requests {
            let role = request.agent_role;
            let task_id = request.task_id.clone();
            let sender = self.worker_senders.get(&role).cloned();
            let timeout_secs = self.config.task_timeout_seconds;
            let usage = Arc::clone(&self.usage);

            let handle = tokio::spawn(async move {
                let sender = sender.ok_or_else(|| {
                    AiError::RequestFailed(format!("no worker for role {}", role))
                })?;

                let (response_tx, response_rx) = oneshot::channel();
                sender
                    .send(WorkerTask {
                        request,
                        response_tx,
                    })
                    .await
                    .map_err(|_| {
                        AiError::RequestFailed(format!("worker {} channel closed", role))
                    })?;

                let timeout = Duration::from_secs(timeout_secs);
                match tokio::time::timeout(timeout, response_rx).await {
                    Ok(Ok(output)) => {
                        if let Ok(ref agent_output) = output {
                            let mut u = usage.write().await;
                            record_usage_inner(&mut u, agent_output);
                        }
                        output
                    }
                    Ok(Err(_)) => Err(AiError::RequestFailed(format!(
                        "worker dropped response for {}",
                        task_id
                    ))),
                    Err(_) => Err(AiError::AgentTimeout {
                        agent_id: format!("{}-0", role.id()),
                    }),
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(Err(AiError::RequestFailed(format!("task panicked: {}", e))))
                }
            }
        }

        results
    }

    /// Submit a task and handle any delegations recursively.
    /// Delegation depth is bounded by `config.max_delegation_depth`.
    pub async fn submit_with_delegations(
        &self,
        request: TaskRequest,
        depth: u32,
    ) -> Result<AgentOutput, AiError> {
        if depth > self.config.max_delegation_depth {
            return Err(AiError::RequestFailed(format!(
                "delegation depth {} exceeds max {}",
                depth, self.config.max_delegation_depth
            )));
        }

        let mut output = self.submit_task(request).await?;

        // Process non-blocking delegations
        if output.has_delegations() {
            let non_blocking: Vec<_> = output
                .delegations
                .iter()
                .filter(|d| !d.blocking)
                .cloned()
                .collect();

            if !non_blocking.is_empty() {
                let delegation_requests: Vec<TaskRequest> = non_blocking
                    .iter()
                    .map(|d| {
                        TaskRequest::new(
                            format!("delegation-{}", d.id),
                            d.to_role,
                            &d.subtask_description,
                        )
                        .with_context(format!("DELEGATION FROM {}: {}", d.from_agent, d.context))
                    })
                    .collect();

                let delegation_results = self.submit_parallel(delegation_requests).await;

                // Attach delegation outputs
                let delegation_summaries: Vec<serde_json::Value> = delegation_results
                    .into_iter()
                    .enumerate()
                    .map(|(i, r)| match r {
                        Ok(o) => serde_json::json!({
                            "delegation_id": non_blocking[i].id,
                            "agent": o.agent_id,
                            "success": o.success,
                            "response_preview": o.raw_response.chars().take(200).collect::<String>(),
                            "tokens": o.total_tokens,
                        }),
                        Err(e) => serde_json::json!({
                            "delegation_id": non_blocking[i].id,
                            "error": e.to_string(),
                        }),
                    })
                    .collect();

                if !delegation_summaries.is_empty() {
                    let existing = output.structured.take().unwrap_or(serde_json::json!({}));
                    let mut obj = existing.as_object().cloned().unwrap_or_default();
                    obj.insert(
                        "delegation_results".into(),
                        serde_json::Value::Array(delegation_summaries),
                    );
                    output.structured = Some(serde_json::Value::Object(obj));
                }
            }
        }

        Ok(output)
    }

    /// Get current usage stats across all agents.
    pub async fn usage(&self) -> OrchestratorUsage {
        self.usage.read().await.clone()
    }

    /// Get API client usage (from the underlying AiBackend).
    pub async fn client_usage(&self) -> HashMap<String, TokenUsage> {
        let client = self.client.lock().await;
        client.all_usage()
    }

    /// Get total tokens used.
    pub async fn total_tokens(&self) -> u64 {
        let client = self.client.lock().await;
        client.total_tokens_used()
    }

    /// Check if a specific agent role has an active worker.
    pub fn has_worker(&self, role: AgentRole) -> bool {
        self.worker_senders.contains_key(&role)
    }

    /// Get the number of active workers.
    pub fn worker_count(&self) -> usize {
        self.worker_senders.len()
    }

    /// Shut down the orchestrator gracefully.
    ///
    /// Drops all worker senders, causing workers to exit after completing
    /// their current task.
    pub async fn shutdown(self) {
        info!("shutting down agent orchestrator");
        // Drop senders — workers will exit when their rx channels close
        drop(self.worker_senders);
        // Wait for workers to finish
        for handle in self._worker_handles {
            let _ = handle.await;
        }
        let _ = self._router_handle.await;
        info!("agent orchestrator stopped");
    }

    /// Record usage from an agent output.
    async fn record_usage(&self, output: &AgentOutput) {
        let mut usage = self.usage.write().await;
        record_usage_inner(&mut usage, output);
    }
}

/// Inner function to record usage (avoids async borrow issues).
fn record_usage_inner(usage: &mut OrchestratorUsage, output: &AgentOutput) {
    usage.total_input_tokens += output.input_tokens;
    usage.total_output_tokens += output.output_tokens;
    usage.total_tokens += output.total_tokens;
    usage.total_requests += 1;

    if output.has_delegations() {
        usage.total_delegations += output.delegations.len() as u64;
    }

    let agent_stats = usage
        .per_agent
        .entry(output.agent_id.clone())
        .or_insert_with(|| AgentUsageStats {
            role: output.role.id().to_string(),
            ..Default::default()
        });

    agent_stats.input_tokens += output.input_tokens;
    agent_stats.output_tokens += output.output_tokens;
    agent_stats.total_tokens += output.total_tokens;
    agent_stats.requests += 1;

    if output.success {
        agent_stats.tasks_completed += 1;
    } else {
        agent_stats.tasks_failed += 1;
    }

    agent_stats.delegations_sent += output.delegations.len() as u64;

    // Running average of duration
    let n = agent_stats.requests as f64;
    agent_stats.avg_duration_seconds =
        agent_stats.avg_duration_seconds * ((n - 1.0) / n) + output.duration_seconds / n;
}

// ── Response Parsing ────────────────────────────────────────────────────────

/// Extract text content from a completion response.
fn extract_text(response: &CompletionResponse) -> String {
    response
        .content
        .iter()
        .filter_map(|block| block.text.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

/// Parse structured output and delegation requests from an agent's raw response.
///
/// Agents emit structured JSON in fenced code blocks:
/// ```json
/// {"type": "output", "code": "...", "files": [...]}
/// ```
///
/// Delegation requests are emitted as:
/// ```json
/// {"type": "delegate", "to": "backend", "task": "...", "context": "...", "blocking": true}
/// ```
fn parse_agent_response(
    raw: &str,
    role: AgentRole,
) -> (Option<serde_json::Value>, Vec<DelegationRequest>) {
    let mut structured = None;
    let mut delegations = Vec::new();

    // Extract JSON blocks from markdown fences
    let json_blocks = extract_json_blocks(raw);

    for block in json_blocks {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&block) {
            match value.get("type").and_then(|t| t.as_str()) {
                Some("delegate") | Some("delegation") => {
                    // Only CTO and Architect can delegate
                    if role.can_delegate() {
                        if let Some(delegation) = parse_delegation(&value, role) {
                            delegations.push(delegation);
                        }
                    }
                }
                Some("output") | Some("result") | Some("artifact") => {
                    structured = Some(value);
                }
                _ => {
                    // Unknown type — treat as structured output
                    if structured.is_none() {
                        structured = Some(value);
                    }
                }
            }
        }
    }

    (structured, delegations)
}

/// Extract JSON code blocks from markdown-style fenced blocks.
fn extract_json_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_block = String::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```json") || trimmed == "```" && in_block {
            if in_block {
                // End of block
                if !current_block.trim().is_empty() {
                    blocks.push(current_block.trim().to_string());
                }
                current_block.clear();
                in_block = false;
            } else if trimmed.starts_with("```json") {
                in_block = true;
            }
        } else if in_block {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }

    // Also try to find standalone JSON objects in the text
    if blocks.is_empty() {
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                let candidate = &text[start..=end];
                if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                    blocks.push(candidate.to_string());
                }
            }
        }
    }

    blocks
}

/// Parse a delegation JSON object into a DelegationRequest.
fn parse_delegation(value: &serde_json::Value, from_role: AgentRole) -> Option<DelegationRequest> {
    let to_str = value.get("to").and_then(|v| v.as_str())?;
    let task = value
        .get("task")
        .or_else(|| value.get("subtask"))
        .and_then(|v| v.as_str())?;

    let to_role = match to_str {
        "cto" => AgentRole::Cto,
        "architect" => AgentRole::Architect,
        "backend" => AgentRole::Backend,
        "frontend" => AgentRole::Frontend,
        "devops" => AgentRole::DevOps,
        "qa" => AgentRole::Qa,
        "security" => AgentRole::Security,
        "monitor" => AgentRole::Monitor,
        _ => return None,
    };

    let context = value
        .get("context")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let blocking = value
        .get("blocking")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let priority = value.get("priority").and_then(|v| v.as_u64()).unwrap_or(5) as u32;

    Some(DelegationRequest {
        id: uuid::Uuid::new_v4().to_string(),
        from_agent: format!("{}-0", from_role.id()),
        to_role,
        parent_task_id: value
            .get("parent_task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        subtask_description: task.to_string(),
        context,
        priority,
        blocking,
    })
}

/// Parse an agent role from an agent ID string like "cto-0" or "backend-1".
fn parse_role_from_agent_id(agent_id: &str) -> Option<AgentRole> {
    let role_str = agent_id
        .rsplit_once('-')
        .map(|(r, _)| r)
        .unwrap_or(agent_id);
    match role_str {
        "cto" => Some(AgentRole::Cto),
        "architect" => Some(AgentRole::Architect),
        "backend" => Some(AgentRole::Backend),
        "frontend" => Some(AgentRole::Frontend),
        "devops" => Some(AgentRole::DevOps),
        "qa" => Some(AgentRole::Qa),
        "security" => Some(AgentRole::Security),
        "monitor" => Some(AgentRole::Monitor),
        _ => None,
    }
}

// ── Pipeline Integration ────────────────────────────────────────────────────

/// Bridge between the pipeline executor and the orchestrator.
///
/// The executor calls these methods instead of directly calling the Anthropic API.
/// This provides a clean separation between pipeline control flow and agent execution.
pub struct PipelineBridge {
    handle: Arc<OrchestratorHandle>,
}

impl PipelineBridge {
    /// Create a new bridge from an orchestrator handle.
    pub fn new(handle: OrchestratorHandle) -> Self {
        Self {
            handle: Arc::new(handle),
        }
    }

    /// Execute a task through the orchestrator, returning the structured result
    /// that the pipeline executor expects.
    ///
    /// Maps from the pipeline's (task_id, agent_role, description) format
    /// to the orchestrator's TaskRequest format.
    pub async fn execute_task(
        &self,
        task_id: &str,
        agent_role_str: &str,
        description: &str,
        knowledge_query: Option<&str>,
        knowledge_chunks: Vec<KnowledgeChunk>,
        context: Option<&str>,
    ) -> Result<PipelineTaskResult, AiError> {
        let role = parse_role_from_agent_id(agent_role_str)
            .or(match agent_role_str {
                "cto" => Some(AgentRole::Cto),
                "architect" => Some(AgentRole::Architect),
                "backend" => Some(AgentRole::Backend),
                "frontend" => Some(AgentRole::Frontend),
                "devops" => Some(AgentRole::DevOps),
                "qa" => Some(AgentRole::Qa),
                "security" => Some(AgentRole::Security),
                "monitor" => Some(AgentRole::Monitor),
                _ => None,
            })
            .unwrap_or(AgentRole::Cto);

        let mut request =
            TaskRequest::new(task_id, role, description).with_knowledge(knowledge_chunks);

        if let Some(q) = knowledge_query {
            request = request.with_knowledge_query(q);
        }
        if let Some(ctx) = context {
            request = request.with_context(ctx);
        }

        let output = self.handle.submit_with_delegations(request, 0).await?;

        Ok(PipelineTaskResult {
            success: output.success,
            output: output.structured,
            error: output.error,
            tokens_used: output.total_tokens,
            raw_response: output.raw_response,
            agent_id: output.agent_id,
            duration_seconds: output.duration_seconds,
            delegations_executed: output.delegations.len() as u32,
        })
    }

    /// Get total tokens consumed across all agents.
    pub async fn total_tokens(&self) -> u64 {
        self.handle.total_tokens().await
    }

    /// Get detailed usage stats.
    pub async fn usage(&self) -> OrchestratorUsage {
        self.handle.usage().await
    }
}

/// Result type that maps back to what the pipeline executor expects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTaskResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub tokens_used: u64,
    pub raw_response: String,
    pub agent_id: String,
    pub duration_seconds: f64,
    pub delegations_executed: u32,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unit tests (no API calls) ──────────────────────────────────────

    #[test]
    fn test_agent_output_creation() {
        let output = AgentOutput {
            agent_id: "cto-0".into(),
            role: AgentRole::Cto,
            task_id: "task-1".into(),
            success: true,
            raw_response: "done".into(),
            structured: Some(serde_json::json!({"key": "value"})),
            error: None,
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            duration_seconds: 1.5,
            delegations: vec![],
            knowledge_sources: vec!["API_Expert/REST".into()],
            stop_reason: Some("end_turn".into()),
        };

        assert_eq!(output.tokens(), 150);
        assert!(!output.has_delegations());
        assert!(output.success);
    }

    #[test]
    fn test_agent_output_with_delegations() {
        let output = AgentOutput {
            agent_id: "cto-0".into(),
            role: AgentRole::Cto,
            task_id: "task-1".into(),
            success: true,
            raw_response: "delegated".into(),
            structured: None,
            error: None,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            duration_seconds: 0.0,
            delegations: vec![DelegationRequest {
                id: "d-1".into(),
                from_agent: "cto-0".into(),
                to_role: AgentRole::Backend,
                parent_task_id: "task-1".into(),
                subtask_description: "implement auth".into(),
                context: "JWT with Ed25519".into(),
                priority: 1,
                blocking: true,
            }],
            knowledge_sources: vec![],
            stop_reason: None,
        };

        assert!(output.has_delegations());
        assert_eq!(output.delegations.len(), 1);
        assert_eq!(output.delegations[0].to_role, AgentRole::Backend);
    }

    #[test]
    fn test_task_request_builder() {
        let request = TaskRequest::new("t-1", AgentRole::Backend, "build API")
            .with_context("REST API for user management")
            .with_knowledge_query("REST API best practices")
            .with_max_tokens(8192);

        assert_eq!(request.task_id, "t-1");
        assert_eq!(request.agent_role, AgentRole::Backend);
        assert_eq!(
            request.context.as_deref(),
            Some("REST API for user management")
        );
        assert_eq!(
            request.knowledge_query.as_deref(),
            Some("REST API best practices")
        );
        assert_eq!(request.max_tokens, Some(8192));
    }

    #[test]
    fn test_task_request_with_knowledge() {
        let chunks = vec![
            KnowledgeChunk {
                source: "API_Expert".into(),
                heading: "REST".into(),
                content: "Use HTTPS".into(),
                score: 0.9,
            },
            KnowledgeChunk {
                source: "Full_Stack_Blueprint".into(),
                heading: "Auth".into(),
                content: "JWT tokens".into(),
                score: 0.85,
            },
        ];

        let request =
            TaskRequest::new("t-2", AgentRole::Security, "audit auth").with_knowledge(chunks);

        assert_eq!(request.knowledge_chunks.len(), 2);
    }

    #[test]
    fn test_delegation_request_serde() {
        let dr = DelegationRequest {
            id: "d-1".into(),
            from_agent: "cto-0".into(),
            to_role: AgentRole::Backend,
            parent_task_id: "task-1".into(),
            subtask_description: "implement auth".into(),
            context: "use JWT".into(),
            priority: 1,
            blocking: true,
        };

        let json = serde_json::to_string(&dr).unwrap();
        let decoded: DelegationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "d-1");
        assert_eq!(decoded.to_role, AgentRole::Backend);
        assert!(decoded.blocking);
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.worker_queue_size, 16);
        assert_eq!(config.max_delegation_depth, 3);
        assert_eq!(config.task_timeout_seconds, 300);
        assert!(config.enable_delegation);
    }

    #[test]
    fn test_parse_role_from_agent_id() {
        assert_eq!(parse_role_from_agent_id("cto-0"), Some(AgentRole::Cto));
        assert_eq!(
            parse_role_from_agent_id("backend-1"),
            Some(AgentRole::Backend)
        );
        assert_eq!(parse_role_from_agent_id("qa-0"), Some(AgentRole::Qa));
        assert_eq!(
            parse_role_from_agent_id("devops-2"),
            Some(AgentRole::DevOps)
        );
        assert_eq!(parse_role_from_agent_id("unknown-0"), None);
    }

    #[test]
    fn test_extract_json_blocks() {
        let text = r#"
Here is the result:

```json
{"type": "output", "code": "fn main() {}"}
```

And here is a delegation:

```json
{"type": "delegate", "to": "backend", "task": "implement API"}
```
"#;

        let blocks = extract_json_blocks(text);
        assert_eq!(blocks.len(), 2);

        let v1: serde_json::Value = serde_json::from_str(&blocks[0]).unwrap();
        assert_eq!(v1["type"], "output");

        let v2: serde_json::Value = serde_json::from_str(&blocks[1]).unwrap();
        assert_eq!(v2["type"], "delegate");
    }

    #[test]
    fn test_extract_json_blocks_inline() {
        let text = "Some text before {\"key\": \"value\"} and after";
        let blocks = extract_json_blocks(text);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn test_extract_json_blocks_empty() {
        let blocks = extract_json_blocks("no json here");
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_parse_agent_response_output() {
        let raw = r#"I'll implement the user auth system.

```json
{"type": "output", "files": ["src/auth.rs"], "loc": 150}
```

Done!"#;

        let (structured, delegations) = parse_agent_response(raw, AgentRole::Backend);
        assert!(structured.is_some());
        assert!(delegations.is_empty());
        assert_eq!(structured.unwrap()["type"], "output");
    }

    #[test]
    fn test_parse_agent_response_with_delegation() {
        let raw = r#"I need the Backend Agent to implement the API:

```json
{"type": "delegate", "to": "backend", "task": "implement REST API", "context": "Use Express.js", "blocking": true}
```

Waiting for delegation."#;

        let (structured, delegations) = parse_agent_response(raw, AgentRole::Cto);
        assert!(structured.is_none());
        assert_eq!(delegations.len(), 1);
        assert_eq!(delegations[0].to_role, AgentRole::Backend);
        assert!(delegations[0].blocking);
    }

    #[test]
    fn test_parse_delegation_non_delegating_role() {
        let raw = r#"```json
{"type": "delegate", "to": "backend", "task": "implement API"}
```"#;

        // Backend agents cannot delegate
        let (_, delegations) = parse_agent_response(raw, AgentRole::Backend);
        assert!(delegations.is_empty());
    }

    #[test]
    fn test_parse_delegation_details() {
        let value = serde_json::json!({
            "type": "delegate",
            "to": "security",
            "task": "audit authentication",
            "context": "Review JWT implementation",
            "blocking": false,
            "priority": 2,
            "parent_task_id": "arch-task-5"
        });

        let delegation = parse_delegation(&value, AgentRole::Architect).unwrap();
        assert_eq!(delegation.to_role, AgentRole::Security);
        assert_eq!(delegation.subtask_description, "audit authentication");
        assert_eq!(delegation.context, "Review JWT implementation");
        assert!(!delegation.blocking);
        assert_eq!(delegation.priority, 2);
        assert_eq!(delegation.parent_task_id, "arch-task-5");
    }

    #[test]
    fn test_parse_delegation_invalid_role() {
        let value = serde_json::json!({
            "type": "delegate",
            "to": "nonexistent",
            "task": "do something"
        });

        assert!(parse_delegation(&value, AgentRole::Cto).is_none());
    }

    #[test]
    fn test_parse_delegation_missing_fields() {
        let value = serde_json::json!({
            "type": "delegate",
            "to": "backend"
            // missing "task"
        });

        assert!(parse_delegation(&value, AgentRole::Cto).is_none());
    }

    #[test]
    fn test_extract_text_from_response() {
        use crate::client::{CompletionResponse, ContentBlock, UsageInfo};

        let response = CompletionResponse {
            id: "msg-123".into(),
            msg_type: "message".into(),
            role: "assistant".into(),
            content: vec![
                ContentBlock {
                    block_type: "text".into(),
                    text: Some("Hello ".into()),
                    id: None,
                    name: None,
                    input: None,
                },
                ContentBlock {
                    block_type: "text".into(),
                    text: Some("World".into()),
                    id: None,
                    name: None,
                    input: None,
                },
            ],
            model: "claude-sonnet-4-6".into(),
            stop_reason: Some("end_turn".into()),
            usage: UsageInfo {
                input_tokens: 100,
                output_tokens: 50,
            },
        };

        assert_eq!(extract_text(&response), "Hello World");
    }

    #[test]
    fn test_record_usage() {
        let mut usage = OrchestratorUsage::default();
        let output = AgentOutput {
            agent_id: "backend-0".into(),
            role: AgentRole::Backend,
            task_id: "t-1".into(),
            success: true,
            raw_response: "done".into(),
            structured: None,
            error: None,
            input_tokens: 500,
            output_tokens: 200,
            total_tokens: 700,
            duration_seconds: 2.5,
            delegations: vec![],
            knowledge_sources: vec![],
            stop_reason: None,
        };

        record_usage_inner(&mut usage, &output);

        assert_eq!(usage.total_input_tokens, 500);
        assert_eq!(usage.total_output_tokens, 200);
        assert_eq!(usage.total_tokens, 700);
        assert_eq!(usage.total_requests, 1);

        let agent = usage.per_agent.get("backend-0").unwrap();
        assert_eq!(agent.role, "backend");
        assert_eq!(agent.tasks_completed, 1);
        assert_eq!(agent.tasks_failed, 0);
        assert!((agent.avg_duration_seconds - 2.5).abs() < 0.01);

        // Record a second task
        let output2 = AgentOutput {
            agent_id: "backend-0".into(),
            role: AgentRole::Backend,
            task_id: "t-2".into(),
            success: false,
            raw_response: "error".into(),
            structured: None,
            error: Some("timeout".into()),
            input_tokens: 300,
            output_tokens: 100,
            total_tokens: 400,
            duration_seconds: 5.0,
            delegations: vec![],
            knowledge_sources: vec![],
            stop_reason: None,
        };

        record_usage_inner(&mut usage, &output2);

        assert_eq!(usage.total_tokens, 1100);
        assert_eq!(usage.total_requests, 2);

        let agent = usage.per_agent.get("backend-0").unwrap();
        assert_eq!(agent.tasks_completed, 1);
        assert_eq!(agent.tasks_failed, 1);
    }

    #[test]
    fn test_usage_with_delegations() {
        let mut usage = OrchestratorUsage::default();
        let output = AgentOutput {
            agent_id: "cto-0".into(),
            role: AgentRole::Cto,
            task_id: "t-1".into(),
            success: true,
            raw_response: "delegated".into(),
            structured: None,
            error: None,
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            duration_seconds: 1.0,
            delegations: vec![
                DelegationRequest {
                    id: "d-1".into(),
                    from_agent: "cto-0".into(),
                    to_role: AgentRole::Backend,
                    parent_task_id: "t-1".into(),
                    subtask_description: "impl".into(),
                    context: String::new(),
                    priority: 5,
                    blocking: false,
                },
                DelegationRequest {
                    id: "d-2".into(),
                    from_agent: "cto-0".into(),
                    to_role: AgentRole::Frontend,
                    parent_task_id: "t-1".into(),
                    subtask_description: "ui".into(),
                    context: String::new(),
                    priority: 5,
                    blocking: false,
                },
            ],
            knowledge_sources: vec![],
            stop_reason: None,
        };

        record_usage_inner(&mut usage, &output);
        assert_eq!(usage.total_delegations, 2);

        let agent = usage.per_agent.get("cto-0").unwrap();
        assert_eq!(agent.delegations_sent, 2);
    }

    #[test]
    fn test_pipeline_task_result_serde() {
        let result = PipelineTaskResult {
            success: true,
            output: Some(serde_json::json!({"files": ["auth.rs"]})),
            error: None,
            tokens_used: 1500,
            raw_response: "implemented auth".into(),
            agent_id: "backend-0".into(),
            duration_seconds: 3.2,
            delegations_executed: 0,
        };

        let json = serde_json::to_string(&result).unwrap();
        let decoded: PipelineTaskResult = serde_json::from_str(&json).unwrap();
        assert!(decoded.success);
        assert_eq!(decoded.tokens_used, 1500);
        assert_eq!(decoded.agent_id, "backend-0");
    }

    #[tokio::test]
    async fn test_orchestrator_start_and_worker_count() {
        let client = AiBackend::anthropic("test-key");
        let orch = AgentOrchestrator::with_client(client);
        let handle = orch.start().await;

        assert_eq!(handle.worker_count(), 8);
        assert!(handle.has_worker(AgentRole::Cto));
        assert!(handle.has_worker(AgentRole::Backend));
        assert!(handle.has_worker(AgentRole::Monitor));

        // Usage should be empty initially
        let usage = handle.usage().await;
        assert_eq!(usage.total_tokens, 0);
        assert_eq!(usage.total_requests, 0);
    }

    #[tokio::test]
    async fn test_orchestrator_has_all_roles() {
        let client = AiBackend::anthropic("test-key");
        let orch = AgentOrchestrator::with_client(client);
        let handle = orch.start().await;

        for role in crate::agents::ALL_ROLES {
            assert!(handle.has_worker(*role), "missing worker for {:?}", role);
        }
    }

    #[test]
    fn test_orchestrator_config_custom() {
        let config = OrchestratorConfig {
            worker_queue_size: 32,
            max_delegation_depth: 5,
            task_timeout_seconds: 600,
            enable_delegation: false,
        };

        assert_eq!(config.worker_queue_size, 32);
        assert_eq!(config.max_delegation_depth, 5);
        assert!(!config.enable_delegation);
    }

    #[test]
    fn test_multiple_json_blocks_mixed_types() {
        let raw = r#"
First I'll delegate the database work:
```json
{"type": "delegate", "to": "backend", "task": "create DB schema", "blocking": true}
```

Then I'll output the architecture:
```json
{"type": "output", "architecture": {"layers": ["api", "service", "data"]}}
```
"#;

        let (structured, delegations) = parse_agent_response(raw, AgentRole::Architect);
        assert!(structured.is_some());
        assert_eq!(delegations.len(), 1);
        assert_eq!(structured.unwrap()["architecture"]["layers"][0], "api");
    }

    #[test]
    fn test_response_parsing_no_json() {
        let raw = "I've completed the task. The authentication module is ready.";
        let (structured, delegations) = parse_agent_response(raw, AgentRole::Backend);
        assert!(structured.is_none());
        assert!(delegations.is_empty());
    }

    #[test]
    fn test_knowledge_sources_tracking() {
        let chunks = [
            KnowledgeChunk {
                source: "API_Expert".into(),
                heading: "REST Design".into(),
                content: "Use standard HTTP methods".into(),
                score: 0.9,
            },
            KnowledgeChunk {
                source: "Full_Stack_Blueprint".into(),
                heading: "Auth".into(),
                content: "JWT tokens".into(),
                score: 0.85,
            },
        ];

        let sources: Vec<String> = chunks
            .iter()
            .map(|c| format!("{}/{}", c.source, c.heading))
            .collect();

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0], "API_Expert/REST Design");
        assert_eq!(sources[1], "Full_Stack_Blueprint/Auth");
    }

    #[test]
    fn test_pipeline_bridge_role_resolution() {
        // Test the role string → AgentRole mapping
        let roles = [
            ("cto", AgentRole::Cto),
            ("architect", AgentRole::Architect),
            ("backend", AgentRole::Backend),
            ("frontend", AgentRole::Frontend),
            ("devops", AgentRole::DevOps),
            ("qa", AgentRole::Qa),
            ("security", AgentRole::Security),
            ("monitor", AgentRole::Monitor),
        ];

        for (role_str, expected) in &roles {
            let resolved = parse_role_from_agent_id(role_str)
                .or(match *role_str {
                    "cto" => Some(AgentRole::Cto),
                    "architect" => Some(AgentRole::Architect),
                    "backend" => Some(AgentRole::Backend),
                    "frontend" => Some(AgentRole::Frontend),
                    "devops" => Some(AgentRole::DevOps),
                    "qa" => Some(AgentRole::Qa),
                    "security" => Some(AgentRole::Security),
                    "monitor" => Some(AgentRole::Monitor),
                    _ => None,
                })
                .unwrap_or(AgentRole::Cto);
            assert_eq!(resolved, *expected, "failed for {}", role_str);
        }
    }

    #[test]
    fn test_context_manager_wiring() {
        // Verify AGENT_ROLE and AGENT_KB_SCOPE are wired into context
        let role = AgentRole::Backend;
        let system_prompt = agent_system_prompt(role);
        let ctx = ContextManager::new(role, system_prompt.clone());

        // System prompt should contain the role name
        let built = ctx.build_system_prompt();
        assert!(built.contains("Backend Agent"));
        assert!(built.contains("KNOWLEDGE BRAIN PROTOCOL"));

        // Knowledge scope should be referenced
        assert!(built.contains("API_Expert"));
        assert!(built.contains("Full_Stack_Blueprint"));
    }

    #[test]
    fn test_context_with_knowledge_injection() {
        let role = AgentRole::Security;
        let system_prompt = agent_system_prompt(role);
        let mut ctx = ContextManager::new(role, system_prompt);

        ctx.inject_knowledge(KnowledgeChunk {
            source: "CTO_Architecture_Framework".into(),
            heading: "Security Model".into(),
            content: "All auth uses Ed25519 signatures.".into(),
            score: 0.95,
        });

        let built = ctx.build_system_prompt();
        assert!(built.contains("KNOWLEDGE BRAIN CONTEXT"));
        assert!(built.contains("Ed25519"));
        assert!(built.contains("Security Agent"));
    }

    #[test]
    fn test_full_prompt_assembly() {
        let role = AgentRole::Cto;
        let system_prompt = agent_system_prompt(role);
        let mut ctx = ContextManager::new(role, system_prompt);

        // Inject knowledge
        ctx.inject_knowledge(KnowledgeChunk {
            source: "CTO_Architecture_Framework".into(),
            heading: "Build Pipeline".into(),
            content: "8 phases from ingest to deliver.".into(),
            score: 0.98,
        });

        // Add history
        ctx.add_message(Message::user("What's the architecture?"));
        ctx.add_message(Message::assistant("The system uses 8 agents."));

        // Build task prompt
        let task = task_prompt(
            role,
            "Coordinate Phase 3: Code generation",
            Some("4 parallel streams: Backend, Frontend, DevOps, Docs"),
        );

        let messages = ctx.build_messages(&task);
        let system = ctx.build_system_prompt();

        // Verify everything is assembled correctly
        assert!(system.contains("CTO Agent"));
        assert!(system.contains("KNOWLEDGE BRAIN CONTEXT"));
        assert!(system.contains("8 phases"));
        assert!(messages.len() >= 3); // 2 history + 1 task
        assert!(messages.last().unwrap().content.contains("Phase 3"));

        let usage = ctx.usage_summary();
        assert!(usage.knowledge_chunks == 1);
        assert!(usage.history_messages == 2);
        assert!(usage.remaining > 0);
    }
}
