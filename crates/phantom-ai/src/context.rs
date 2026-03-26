//! Context window management — knowledge injection, token budgets, prompt assembly.
//!
//! The ContextManager assembles the full prompt for each agent call:
//!   1. System prompt (role-specific)
//!   2. Injected knowledge chunks (from ChromaDB)
//!   3. Conversation history (pruned to fit)
//!   4. Current task prompt
//!
//! Token counting uses a simple heuristic (chars / 4) since we don't
//! bundle a full tokenizer. This is conservative enough for budget management.

use serde::{Deserialize, Serialize};

use crate::agents::AgentRole;
use crate::client::Message;

/// Approximate tokens from a string (chars / 4, conservative).
pub fn estimate_tokens(text: &str) -> usize {
    // Claude tokenizer averages ~4 chars per token for English
    text.len().div_ceil(4)
}

/// Model context window sizes.
pub fn context_window(model: &str) -> usize {
    match model {
        "claude-opus-4-6" => 200_000,
        "claude-sonnet-4-6" => 200_000,
        "claude-haiku-4-5-20251001" => 200_000,
        m if m.contains("deepseek") => 128_000,
        _ => 100_000, // Conservative default
    }
}

/// A knowledge chunk to inject into the context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChunk {
    /// Source document name
    pub source: String,
    /// Section heading
    pub heading: String,
    /// Chunk content
    pub content: String,
    /// Relevance score (0.0 - 1.0)
    pub score: f64,
}

impl KnowledgeChunk {
    pub fn estimated_tokens(&self) -> usize {
        estimate_tokens(&self.content) + estimate_tokens(&self.heading) + 10 // overhead
    }
}

/// Context window manager for agent conversations.
///
/// Assembles prompts within token budgets, manages knowledge injection,
/// and prunes conversation history to fit.
pub struct ContextManager {
    /// Agent role
    role: AgentRole,
    /// System prompt
    system_prompt: String,
    /// Injected knowledge chunks
    knowledge: Vec<KnowledgeChunk>,
    /// Conversation history
    history: Vec<Message>,
    /// Max tokens reserved for the response
    response_reserve: usize,
    /// Max knowledge tokens to inject
    max_knowledge_tokens: usize,
}

impl ContextManager {
    pub fn new(role: AgentRole, system_prompt: String) -> Self {
        Self {
            role,
            system_prompt,
            knowledge: Vec::new(),
            history: Vec::new(),
            response_reserve: role.max_tokens() as usize,
            max_knowledge_tokens: 10_000,
        }
    }

    /// Set the max tokens reserved for knowledge injection.
    pub fn with_max_knowledge_tokens(mut self, tokens: usize) -> Self {
        self.max_knowledge_tokens = tokens;
        self
    }

    /// Add a knowledge chunk (sorted by score, highest first).
    pub fn inject_knowledge(&mut self, chunk: KnowledgeChunk) {
        self.knowledge.push(chunk);
        self.knowledge.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Add multiple knowledge chunks.
    pub fn inject_knowledge_batch(&mut self, chunks: Vec<KnowledgeChunk>) {
        for chunk in chunks {
            self.inject_knowledge(chunk);
        }
    }

    /// Clear all knowledge chunks.
    pub fn clear_knowledge(&mut self) {
        self.knowledge.clear();
    }

    /// Add a message to the conversation history.
    pub fn add_message(&mut self, message: Message) {
        self.history.push(message);
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get the context window size for this agent's model.
    pub fn window_size(&self) -> usize {
        context_window(self.role.model())
    }

    /// Calculate tokens used by the system prompt.
    pub fn system_tokens(&self) -> usize {
        estimate_tokens(&self.system_prompt)
    }

    /// Calculate tokens used by knowledge chunks (respecting budget).
    pub fn knowledge_tokens(&self) -> usize {
        let mut total = 0;
        for chunk in &self.knowledge {
            let chunk_tokens = chunk.estimated_tokens();
            if total + chunk_tokens > self.max_knowledge_tokens {
                break;
            }
            total += chunk_tokens;
        }
        total
    }

    /// Calculate tokens used by conversation history.
    pub fn history_tokens(&self) -> usize {
        self.history
            .iter()
            .map(|m| estimate_tokens(&m.content) + 5) // 5 tokens overhead per message
            .sum()
    }

    /// Total tokens currently used.
    pub fn total_tokens_used(&self) -> usize {
        self.system_tokens() + self.knowledge_tokens() + self.history_tokens()
    }

    /// Tokens available for new content.
    pub fn tokens_remaining(&self) -> usize {
        let window = self.window_size();
        let used = self.total_tokens_used() + self.response_reserve;
        window.saturating_sub(used)
    }

    /// Build the knowledge section of the prompt.
    pub fn build_knowledge_section(&self) -> Option<String> {
        let mut sections = Vec::new();
        let mut total_tokens = 0;

        for chunk in &self.knowledge {
            let chunk_tokens = chunk.estimated_tokens();
            if total_tokens + chunk_tokens > self.max_knowledge_tokens {
                break;
            }
            sections.push(format!(
                "--- {} / {} (relevance: {:.2}) ---\n{}",
                chunk.source, chunk.heading, chunk.score, chunk.content
            ));
            total_tokens += chunk_tokens;
        }

        if sections.is_empty() {
            None
        } else {
            Some(format!(
                "KNOWLEDGE BRAIN CONTEXT:\n\n{}\n\n---END KNOWLEDGE---",
                sections.join("\n\n")
            ))
        }
    }

    /// Build the full system prompt with knowledge injected.
    pub fn build_system_prompt(&self) -> String {
        match self.build_knowledge_section() {
            Some(knowledge) => format!("{}\n\n{}", self.system_prompt, knowledge),
            None => self.system_prompt.clone(),
        }
    }

    /// Get conversation history, pruned to fit within budget.
    pub fn build_messages(&self, task_prompt: &str) -> Vec<Message> {
        let task_tokens = estimate_tokens(task_prompt);
        let available = self.tokens_remaining().saturating_sub(task_tokens);

        let mut messages = Vec::new();
        let mut used = 0;

        // Include history from most recent, working backwards
        for msg in self.history.iter().rev() {
            let msg_tokens = estimate_tokens(&msg.content) + 5;
            if used + msg_tokens > available {
                break;
            }
            messages.push(msg.clone());
            used += msg_tokens;
        }

        // Reverse to chronological order
        messages.reverse();

        // Add the current task prompt
        messages.push(Message::user(task_prompt));

        messages
    }

    /// Get a summary of context usage.
    pub fn usage_summary(&self) -> ContextUsage {
        ContextUsage {
            window_size: self.window_size(),
            system_tokens: self.system_tokens(),
            knowledge_tokens: self.knowledge_tokens(),
            history_tokens: self.history_tokens(),
            response_reserve: self.response_reserve,
            total_used: self.total_tokens_used(),
            remaining: self.tokens_remaining(),
            knowledge_chunks: self.knowledge.len(),
            history_messages: self.history.len(),
        }
    }
}

/// Summary of context window usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUsage {
    pub window_size: usize,
    pub system_tokens: usize,
    pub knowledge_tokens: usize,
    pub history_tokens: usize,
    pub response_reserve: usize,
    pub total_used: usize,
    pub remaining: usize,
    pub knowledge_chunks: usize,
    pub history_messages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> ContextManager {
        ContextManager::new(AgentRole::Backend, "You are the Backend Agent.".into())
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("hello world!!"), 4); // 13 chars / 4 ≈ 4
    }

    #[test]
    fn test_context_window_sizes() {
        assert_eq!(context_window("claude-opus-4-6"), 200_000);
        assert_eq!(context_window("claude-sonnet-4-6"), 200_000);
        assert_eq!(context_window("unknown-model"), 100_000);
    }

    #[test]
    fn test_context_manager_creation() {
        let ctx = test_context();
        assert!(ctx.system_tokens() > 0);
        assert_eq!(ctx.knowledge_tokens(), 0);
        assert_eq!(ctx.history_tokens(), 0);
    }

    #[test]
    fn test_knowledge_injection() {
        let mut ctx = test_context();
        ctx.inject_knowledge(KnowledgeChunk {
            source: "API_Expert".into(),
            heading: "REST endpoints".into(),
            content: "Use standard HTTP methods...".into(),
            score: 0.85,
        });
        ctx.inject_knowledge(KnowledgeChunk {
            source: "Full_Stack_Blueprint".into(),
            heading: "Database design".into(),
            content: "Use PostgreSQL with...".into(),
            score: 0.92,
        });

        // Higher score should be first
        assert_eq!(ctx.knowledge.len(), 2);
        assert_eq!(ctx.knowledge[0].source, "Full_Stack_Blueprint");
        assert!(ctx.knowledge_tokens() > 0);
    }

    #[test]
    fn test_knowledge_budget_limit() {
        let mut ctx = test_context().with_max_knowledge_tokens(10);
        ctx.inject_knowledge(KnowledgeChunk {
            source: "test".into(),
            heading: "test".into(),
            content: "a".repeat(1000), // Way over budget
            score: 0.9,
        });

        // Should be capped by budget
        let section = ctx.build_knowledge_section();
        // With 10 token budget, 1000 chars (250 tokens) won't fit
        assert!(section.is_none());
    }

    #[test]
    fn test_conversation_history() {
        let mut ctx = test_context();
        ctx.add_message(Message::user("What API should I use?"));
        ctx.add_message(Message::assistant("Use REST with JSON."));

        assert_eq!(ctx.history.len(), 2);
        assert!(ctx.history_tokens() > 0);
    }

    #[test]
    fn test_build_system_prompt_no_knowledge() {
        let ctx = test_context();
        let prompt = ctx.build_system_prompt();
        assert_eq!(prompt, "You are the Backend Agent.");
        assert!(!prompt.contains("KNOWLEDGE BRAIN CONTEXT"));
    }

    #[test]
    fn test_build_system_prompt_with_knowledge() {
        let mut ctx = test_context();
        ctx.inject_knowledge(KnowledgeChunk {
            source: "API_Expert".into(),
            heading: "REST".into(),
            content: "Always use HTTPS.".into(),
            score: 0.9,
        });

        let prompt = ctx.build_system_prompt();
        assert!(prompt.contains("KNOWLEDGE BRAIN CONTEXT"));
        assert!(prompt.contains("Always use HTTPS"));
    }

    #[test]
    fn test_build_messages() {
        let mut ctx = test_context();
        ctx.add_message(Message::user("first question"));
        ctx.add_message(Message::assistant("first answer"));

        let messages = ctx.build_messages("current task");
        // Should include history + current task
        assert!(messages.len() >= 2);
        assert_eq!(messages.last().unwrap().content, "current task");
    }

    #[test]
    fn test_tokens_remaining() {
        let ctx = test_context();
        let remaining = ctx.tokens_remaining();
        assert!(remaining > 0);
        assert!(remaining < ctx.window_size());
    }

    #[test]
    fn test_usage_summary() {
        let mut ctx = test_context();
        ctx.add_message(Message::user("hello"));
        ctx.inject_knowledge(KnowledgeChunk {
            source: "test".into(),
            heading: "test".into(),
            content: "test content".into(),
            score: 0.5,
        });

        let summary = ctx.usage_summary();
        assert_eq!(summary.knowledge_chunks, 1);
        assert_eq!(summary.history_messages, 1);
        assert!(summary.system_tokens > 0);
        assert!(summary.remaining > 0);
    }

    #[test]
    fn test_clear_operations() {
        let mut ctx = test_context();
        ctx.add_message(Message::user("hello"));
        ctx.inject_knowledge(KnowledgeChunk {
            source: "test".into(),
            heading: "test".into(),
            content: "test".into(),
            score: 0.5,
        });

        ctx.clear_history();
        assert_eq!(ctx.history.len(), 0);

        ctx.clear_knowledge();
        assert_eq!(ctx.knowledge.len(), 0);
    }
}
