//! The set of AI coding agents the Zenith skill can be installed into, plus how
//! each one consumes a skill.

/// An AI coding agent Zenith can install its skill for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Agent {
    ClaudeCode,
    Codex,
    OpenCode,
    Cursor,
    Windsurf,
    Aider,
    Zed,
    Gemini,
    Copilot,
    Continue,
    Kiro,
    Antigravity,
}

/// How an agent loads a skill — this decides what we write to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillFormat {
    /// Folder skill: a `SKILL.md` plus the full `references/`, `templates/`,
    /// `themes/` tree (progressive disclosure works). Optional slash-commands.
    Folder,
    /// A single self-contained rule/markdown file. References cannot be loaded
    /// on demand, so the file points the agent at the self-documenting CLI.
    Rule,
}

/// Installation scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    /// Per-project: write under the project directory (e.g. `.claude/`).
    Project,
    /// Per-user: write under `$HOME` (e.g. `~/.claude/`).
    User,
}

/// Every agent, in a stable order (used by `--all` and auto-detect).
pub const ALL_AGENTS: &[Agent] = &[
    Agent::ClaudeCode,
    Agent::Codex,
    Agent::OpenCode,
    Agent::Cursor,
    Agent::Windsurf,
    Agent::Aider,
    Agent::Zed,
    Agent::Gemini,
    Agent::Copilot,
    Agent::Continue,
    Agent::Kiro,
    Agent::Antigravity,
];

impl Agent {
    /// Parse a CLI agent name. Accepts a few common aliases.
    pub fn parse(s: &str) -> Option<Agent> {
        match s {
            "claude" | "claude-code" | "claudecode" => Some(Agent::ClaudeCode),
            "codex" => Some(Agent::Codex),
            "opencode" | "open-code" => Some(Agent::OpenCode),
            "cursor" => Some(Agent::Cursor),
            "windsurf" => Some(Agent::Windsurf),
            "aider" => Some(Agent::Aider),
            "zed" => Some(Agent::Zed),
            "gemini" | "gemini-cli" => Some(Agent::Gemini),
            "copilot" | "github-copilot" => Some(Agent::Copilot),
            "continue" | "continue-dev" => Some(Agent::Continue),
            "kiro" => Some(Agent::Kiro),
            "antigravity" => Some(Agent::Antigravity),
            _ => None,
        }
    }

    /// Human-readable name for messages.
    pub fn display(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "Claude Code",
            Agent::Codex => "Codex",
            Agent::OpenCode => "OpenCode",
            Agent::Cursor => "Cursor",
            Agent::Windsurf => "Windsurf",
            Agent::Aider => "Aider",
            Agent::Zed => "Zed",
            Agent::Gemini => "Gemini",
            Agent::Copilot => "Copilot",
            Agent::Continue => "Continue",
            Agent::Kiro => "Kiro",
            Agent::Antigravity => "Antigravity",
        }
    }

    /// How this agent consumes a skill.
    pub fn format(self) -> SkillFormat {
        match self {
            Agent::ClaudeCode | Agent::Codex | Agent::OpenCode => SkillFormat::Folder,
            Agent::Cursor
            | Agent::Windsurf
            | Agent::Aider
            | Agent::Zed
            | Agent::Gemini
            | Agent::Copilot
            | Agent::Continue
            | Agent::Kiro
            | Agent::Antigravity => SkillFormat::Rule,
        }
    }
}
