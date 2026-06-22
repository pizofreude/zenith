//! Detect which agents are present on the machine, and whether the Zenith skill
//! is already installed for them.

use std::path::{Path, PathBuf};

use super::agent::{ALL_AGENTS, Agent, Scope};
use super::paths::{home_dir, skill_target};

/// True when the skill is already present for `(agent, scope)`.
pub fn is_installed(agent: Agent, scope: Scope, project_root: &Path) -> bool {
    skill_target(agent, scope, project_root)
        .map(|t| t.root().exists())
        .unwrap_or(false)
}

/// The directory whose existence signals that `agent` is in use, for a scope.
/// Detection markers are intentionally the agent's *config* dir, which may
/// differ from where the skill is written.
pub fn marker_dir(agent: Agent, scope: Scope, project_root: &Path) -> Option<PathBuf> {
    let (proj, user): (&str, fn(&Path) -> PathBuf) = match agent {
        Agent::ClaudeCode => (".claude", |h| h.join(".claude")),
        Agent::Codex => (".codex", |h| h.join(".codex")),
        Agent::OpenCode => (".opencode", |h| h.join(".config").join("opencode")),
        Agent::Cursor => (".cursor", |h| h.join(".cursor")),
        Agent::Windsurf => (".windsurf", |h| h.join(".windsurf")),
        Agent::Aider => (".aider", |h| h.join(".aider")),
        Agent::Zed => (".zed", |h| h.join(".zed")),
        Agent::Gemini => (".gemini", |h| h.join(".gemini")),
        Agent::Copilot => (".copilot", |h| h.join(".copilot")),
        Agent::Continue => (".continue", |h| h.join(".continue")),
        Agent::Kiro => (".kiro", |h| h.join(".kiro")),
        Agent::Antigravity => (".antigravity", |h| h.join(".antigravity")),
    };
    match scope {
        Scope::Project => Some(project_root.join(proj)),
        Scope::User => home_dir().map(|h| user(&h)),
    }
}

/// True when `agent` appears to be in use in either the project or user scope.
pub fn is_present(agent: Agent, project_root: &Path) -> bool {
    [Scope::Project, Scope::User].into_iter().any(|scope| {
        marker_dir(agent, scope, project_root)
            .map(|d| d.exists())
            .unwrap_or(false)
    })
}

/// All agents detected as present, in stable order.
pub fn detect_present(project_root: &Path) -> Vec<Agent> {
    ALL_AGENTS
        .iter()
        .copied()
        .filter(|a| is_present(*a, project_root))
        .collect()
}
