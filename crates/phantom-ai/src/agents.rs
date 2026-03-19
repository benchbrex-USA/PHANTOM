//! Agent type definitions — CTO, Architect, Backend, Frontend, DevOps, QA, Security, Monitor.
//!
//! Each agent maps to a Claude model, temperature, max token budget,
//! and a set of knowledge files it can query from the Knowledge Brain.

use std::fmt;

use serde::{Deserialize, Serialize};

/// All agent roles.
pub const ALL_ROLES: &[AgentRole] = &[
    AgentRole::Cto,
    AgentRole::Architect,
    AgentRole::Backend,
    AgentRole::Frontend,
    AgentRole::DevOps,
    AgentRole::Qa,
    AgentRole::Security,
    AgentRole::Monitor,
];

/// Agent role in the Phantom engineering team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    Cto,
    Architect,
    Backend,
    Frontend,
    DevOps,
    Qa,
    Security,
    Monitor,
}

impl AgentRole {
    /// The Claude model to use for this agent.
    pub fn model(&self) -> &'static str {
        match self {
            Self::Cto | Self::Architect | Self::Security => "claude-opus-4-6",
            Self::Backend | Self::Frontend | Self::DevOps | Self::Qa => "claude-sonnet-4-6",
            Self::Monitor => "claude-haiku-4-5-20251001",
        }
    }

    /// Temperature setting for this agent.
    pub fn temperature(&self) -> f32 {
        match self {
            Self::Cto => 0.3,
            Self::Architect | Self::Security => 0.2,
            Self::Backend | Self::Frontend | Self::DevOps | Self::Qa => 0.1,
            Self::Monitor => 0.0,
        }
    }

    /// Max tokens per response for this agent.
    pub fn max_tokens(&self) -> u32 {
        match self {
            Self::Cto | Self::Architect => 16384,
            Self::Backend | Self::Frontend => 16384,
            Self::DevOps | Self::Security => 8192,
            Self::Qa => 8192,
            Self::Monitor => 4096,
        }
    }

    /// Token budget per task for this agent.
    pub fn task_token_budget(&self) -> u64 {
        match self {
            Self::Cto => 500_000,
            Self::Architect => 300_000,
            Self::Backend | Self::Frontend => 200_000,
            Self::DevOps | Self::Security | Self::Qa => 100_000,
            Self::Monitor => 50_000,
        }
    }

    /// Knowledge files this agent has access to.
    pub fn knowledge_scope(&self) -> &'static [&'static str] {
        match self {
            Self::Cto => &[
                "CTO_Architecture_Framework",
                "CTO_Complete_Technology_Knowledge",
                "Complete_Multi_Agent_System",
                "Build_Once_Launch_Directly",
                "Full_Stack_Blueprint",
                "Every_Technology",
                "Design_Expert",
                "AI_ML_Expert",
                "API_Expert",
                "AI_Code_Errors",
            ],
            Self::Architect => &[
                "CTO_Architecture_Framework",
                "CTO_Complete_Technology_Knowledge",
                "Full_Stack_Blueprint",
                "Every_Technology",
            ],
            Self::Backend => &[
                "API_Expert",
                "CTO_Complete_Technology_Knowledge",
                "Full_Stack_Blueprint",
                "AI_ML_Expert",
            ],
            Self::Frontend => &[
                "Design_Expert",
                "CTO_Complete_Technology_Knowledge",
                "Full_Stack_Blueprint",
            ],
            Self::DevOps => &[
                "Build_Once_Launch_Directly",
                "AI_Code_Errors",
                "CTO_Complete_Technology_Knowledge",
                "Every_Technology",
            ],
            Self::Qa => &[
                "AI_Code_Errors",
                "CTO_Complete_Technology_Knowledge",
                "Full_Stack_Blueprint",
            ],
            Self::Security => &[
                "CTO_Complete_Technology_Knowledge",
                "Full_Stack_Blueprint",
                "CTO_Architecture_Framework",
                "AI_ML_Expert",
            ],
            Self::Monitor => &[
                "Complete_Multi_Agent_System",
                "CTO_Complete_Technology_Knowledge",
                "Build_Once_Launch_Directly",
            ],
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Cto => "CTO Agent",
            Self::Architect => "Architect Agent",
            Self::Backend => "Backend Agent",
            Self::Frontend => "Frontend Agent",
            Self::DevOps => "DevOps Agent",
            Self::Qa => "QA Agent",
            Self::Security => "Security Agent",
            Self::Monitor => "Monitor Agent",
        }
    }

    /// Short identifier for logging / IDs.
    pub fn id(&self) -> &'static str {
        match self {
            Self::Cto => "cto",
            Self::Architect => "architect",
            Self::Backend => "backend",
            Self::Frontend => "frontend",
            Self::DevOps => "devops",
            Self::Qa => "qa",
            Self::Security => "security",
            Self::Monitor => "monitor",
        }
    }

    /// Whether this agent can delegate tasks to other agents.
    pub fn can_delegate(&self) -> bool {
        matches!(self, Self::Cto | Self::Architect)
    }

    /// Whether this agent needs code execution capabilities.
    pub fn needs_code_exec(&self) -> bool {
        matches!(
            self,
            Self::Backend | Self::Frontend | Self::DevOps | Self::Qa
        )
    }
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Configuration for a specific agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub role: AgentRole,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub task_token_budget: u64,
    /// Custom system prompt override (if any)
    pub system_prompt_override: Option<String>,
}

impl AgentConfig {
    /// Create default config for a role.
    pub fn for_role(role: AgentRole) -> Self {
        Self {
            role,
            model: role.model().to_string(),
            temperature: role.temperature(),
            max_tokens: role.max_tokens(),
            task_token_budget: role.task_token_budget(),
            system_prompt_override: None,
        }
    }

    /// Override the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override the temperature.
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }
}

/// Build configs for the full 8-agent team.
pub fn full_team_configs() -> Vec<AgentConfig> {
    ALL_ROLES.iter().map(|r| AgentConfig::for_role(*r)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_roles_count() {
        assert_eq!(ALL_ROLES.len(), 8);
    }

    #[test]
    fn test_agent_models() {
        assert_eq!(AgentRole::Cto.model(), "claude-opus-4-6");
        assert_eq!(AgentRole::Backend.model(), "claude-sonnet-4-6");
        assert_eq!(AgentRole::Monitor.model(), "claude-haiku-4-5-20251001");
    }

    #[test]
    fn test_agent_temperatures() {
        assert!(AgentRole::Cto.temperature() > AgentRole::Monitor.temperature());
        assert_eq!(AgentRole::Monitor.temperature(), 0.0);
    }

    #[test]
    fn test_agent_knowledge_scope() {
        let cto_scope = AgentRole::Cto.knowledge_scope();
        assert_eq!(cto_scope.len(), 10); // CTO has access to everything

        let monitor_scope = AgentRole::Monitor.knowledge_scope();
        assert!(monitor_scope.len() < cto_scope.len());
    }

    #[test]
    fn test_agent_display() {
        assert_eq!(AgentRole::Cto.to_string(), "CTO Agent");
        assert_eq!(AgentRole::Qa.to_string(), "QA Agent");
    }

    #[test]
    fn test_agent_id() {
        assert_eq!(AgentRole::Cto.id(), "cto");
        assert_eq!(AgentRole::DevOps.id(), "devops");
    }

    #[test]
    fn test_agent_delegation() {
        assert!(AgentRole::Cto.can_delegate());
        assert!(AgentRole::Architect.can_delegate());
        assert!(!AgentRole::Backend.can_delegate());
    }

    #[test]
    fn test_agent_code_exec() {
        assert!(AgentRole::Backend.needs_code_exec());
        assert!(!AgentRole::Cto.needs_code_exec());
        assert!(!AgentRole::Monitor.needs_code_exec());
    }

    #[test]
    fn test_agent_config() {
        let config = AgentConfig::for_role(AgentRole::Backend);
        assert_eq!(config.model, "claude-sonnet-4-6");
        assert_eq!(config.temperature, 0.1);
        assert_eq!(config.max_tokens, 16384);
    }

    #[test]
    fn test_agent_config_override() {
        let config = AgentConfig::for_role(AgentRole::Backend)
            .with_model("claude-opus-4-6")
            .with_temperature(0.5);
        assert_eq!(config.model, "claude-opus-4-6");
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_full_team_configs() {
        let team = full_team_configs();
        assert_eq!(team.len(), 8);
        assert!(team.iter().any(|c| c.role == AgentRole::Cto));
        assert!(team.iter().any(|c| c.role == AgentRole::Monitor));
    }

    #[test]
    fn test_agent_serde() {
        let role = AgentRole::DevOps;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"devops\"");
        let decoded: AgentRole = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, AgentRole::DevOps);
    }

    #[test]
    fn test_token_budgets() {
        // CTO has highest budget
        assert!(AgentRole::Cto.task_token_budget() > AgentRole::Backend.task_token_budget());
        // Monitor has lowest
        assert!(AgentRole::Monitor.task_token_budget() < AgentRole::Qa.task_token_budget());
    }
}
