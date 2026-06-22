//! Remove a previously installed skill for an agent.

use std::fs;
use std::path::{Path, PathBuf};

use super::agent::{Agent, Scope, SkillFormat};
use super::assets::COMMAND_FILES;
use super::paths::{SkillTarget, command_dir, skill_target};

/// Outcome of removing one item (file or directory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOutcome {
    /// The item existed and was removed.
    Removed,
    /// The item was not present — nothing to do.
    Absent,
}

/// Per-item removal result.
#[derive(Debug, Clone)]
pub struct RemoveResult {
    pub path: PathBuf,
    pub outcome: Result<RemoveOutcome, String>,
}

/// Per-agent uninstall report.
#[derive(Debug, Clone)]
pub struct AgentUninstall {
    pub agent: Agent,
    pub items: Vec<RemoveResult>,
    pub unsupported: Option<String>,
}

/// Uninstall (or, when `dry_run`, plan removal of) the skill for one agent.
pub fn uninstall_agent(
    agent: Agent,
    scope: Scope,
    project_root: &Path,
    dry_run: bool,
) -> AgentUninstall {
    let Some(target) = skill_target(agent, scope, project_root) else {
        return AgentUninstall {
            agent,
            items: Vec::new(),
            unsupported: Some(format!("{} has no automatic target", agent.display())),
        };
    };

    let mut items = Vec::new();
    match (agent.format(), &target) {
        (SkillFormat::Folder, SkillTarget::Folder(dir)) => {
            items.push(remove(dir, true, dry_run));
            if let Some(cmd_dir) = command_dir(agent, scope, project_root) {
                for (name, _) in COMMAND_FILES {
                    items.push(remove(&cmd_dir.join(name), false, dry_run));
                }
            }
        }
        (SkillFormat::Rule, SkillTarget::Rule(path)) => {
            items.push(remove(path, false, dry_run));
        }
        (SkillFormat::Folder, SkillTarget::Rule(_))
        | (SkillFormat::Rule, SkillTarget::Folder(_)) => {}
    }

    AgentUninstall {
        agent,
        items,
        unsupported: None,
    }
}

/// Remove a file or directory, reporting whether it existed.
fn remove(path: &Path, is_dir: bool, dry_run: bool) -> RemoveResult {
    let exists = path.exists();
    let outcome = if !exists {
        Ok(RemoveOutcome::Absent)
    } else if dry_run {
        Ok(RemoveOutcome::Removed)
    } else {
        let res = if is_dir {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        res.map(|()| RemoveOutcome::Removed)
            .map_err(|e| e.to_string())
    };
    RemoveResult {
        path: path.to_path_buf(),
        outcome,
    }
}
