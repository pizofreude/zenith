//! Write the skill to an agent's target location, idempotently.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::agent::{Agent, Scope, SkillFormat};
use super::assets::{COMMAND_FILES, SKILL_FILES};
use super::paths::{SkillTarget, command_dir, skill_target};
use super::render::render_rule;

/// Result of writing (or planning to write) a single file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// File was (or would be) created or overwritten.
    Installed,
    /// File already exists with identical content — nothing to do.
    AlreadyCurrent,
    /// File exists with different content; needs `--force` to overwrite.
    WouldOverwrite,
}

/// Per-file outcome with its absolute path.
#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: PathBuf,
    pub outcome: Result<WriteOutcome, String>,
}

/// Per-agent install report.
#[derive(Debug, Clone)]
pub struct AgentInstall {
    pub agent: Agent,
    /// The root path (skill dir or rule file). `None` when unsupported.
    pub root: Option<PathBuf>,
    pub files: Vec<FileResult>,
    /// Set when the (agent, scope) pair has no automatic target.
    pub unsupported: Option<String>,
}

/// Install (or, when `dry_run`, plan) the skill for one agent.
pub fn install_agent(
    agent: Agent,
    scope: Scope,
    project_root: &Path,
    force: bool,
    dry_run: bool,
) -> AgentInstall {
    let Some(target) = skill_target(agent, scope, project_root) else {
        return AgentInstall {
            agent,
            root: None,
            files: Vec::new(),
            unsupported: Some(format!(
                "{} {} scope has no automatic target",
                agent.display(),
                scope_label(scope)
            )),
        };
    };

    let mut files = Vec::new();
    match (agent.format(), &target) {
        (SkillFormat::Folder, SkillTarget::Folder(dir)) => {
            for (rel, body) in SKILL_FILES {
                let path = dir.join(rel);
                files.push(write(&path, body, force, dry_run));
            }
            if let Some(cmd_dir) = command_dir(agent, scope, project_root) {
                for (name, body) in COMMAND_FILES {
                    let path = cmd_dir.join(name);
                    files.push(write(&path, body, force, dry_run));
                }
            }
        }
        (SkillFormat::Rule, SkillTarget::Rule(path)) => {
            let body = render_rule(agent);
            files.push(write(path, &body, force, dry_run));
        }
        // Format/target mismatch is impossible by construction.
        (SkillFormat::Folder, SkillTarget::Rule(_))
        | (SkillFormat::Rule, SkillTarget::Folder(_)) => {}
    }

    AgentInstall {
        agent,
        root: Some(target.root().to_path_buf()),
        files,
        unsupported: None,
    }
}

/// Plan, then (unless `dry_run`) perform a single idempotent write.
fn write(path: &Path, content: &str, force: bool, dry_run: bool) -> FileResult {
    let outcome = (|| -> io::Result<WriteOutcome> {
        let planned = plan(path, content, force)?;
        if !dry_run && planned == WriteOutcome::Installed {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)?;
        }
        Ok(planned)
    })();
    FileResult {
        path: path.to_path_buf(),
        outcome: outcome.map_err(|e| e.to_string()),
    }
}

/// Decide what writing `content` to `path` would do, without writing.
fn plan(path: &Path, content: &str, force: bool) -> io::Result<WriteOutcome> {
    if path.exists() {
        let existing = fs::read(path)?;
        if existing == content.as_bytes() {
            return Ok(WriteOutcome::AlreadyCurrent);
        }
        if !force {
            return Ok(WriteOutcome::WouldOverwrite);
        }
    }
    Ok(WriteOutcome::Installed)
}

fn scope_label(scope: Scope) -> &'static str {
    match scope {
        Scope::Project => "project",
        Scope::User => "user",
    }
}
