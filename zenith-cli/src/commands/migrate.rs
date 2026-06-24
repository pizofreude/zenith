//! Pure logic for `zenith migrate <doc.zen>`.
//!
//! Relocates process/provenance state out of a `.zen` deliverable into the
//! app-managed doc-id store (crate `zenith-session`), then re-emits the `.zen`
//! stripped of those blocks.
//!
//! The public entry points [`run`] (resolves the real store) and [`run_in`]
//! (takes an explicit [`StorePaths`] — used by integration tests) operate
//! entirely on in-memory source bytes/strings. File I/O is the responsibility
//! of the dispatcher in `dispatch.rs`.

use std::collections::BTreeMap;

use zenith_core::format::format_document;
use zenith_core::{AgentRun, KdlAdapter, KdlSource as _, Page, PreviewArtifact, PropertyValue};
use zenith_session::adapter::{OsClock, OsFs};
use zenith_session::{
    CandidateMeta, CandidateStatus, NewCandidate, PreviewCritique, PreviewRecord, RunDiagnostic,
    RunRecord, RunStep, StorePaths, append_preview, append_run, put_scratch, read_previews,
};

use crate::commands::serialize_pretty;
use crate::commands::workspace::scratch::open_store;

// ── Error / output types ──────────────────────────────────────────────────────

/// Error type for `migrate`.
#[derive(Debug)]
pub struct MigrateError {
    /// Human-readable error message.
    pub message: String,
    /// Exit code: 2 for hard errors; 1 for nothing-to-do when caller treats it
    /// that way (currently unused — `migrate` always uses 2 on error).
    pub exit_code: u8,
}

impl MigrateError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 2,
        }
    }
}

/// The outcome of a successful migrate run.
#[derive(Debug)]
pub struct MigrateOut {
    /// Formatted `.zen` bytes with `agent-runs`, `previews`, and candidate
    /// metadata stripped from every page.
    pub stripped_bytes: Vec<u8>,
    /// Human-readable (or JSON) report to print to stdout.
    pub report: String,
}

// ── Public entry points ───────────────────────────────────────────────────────

/// Resolve the real store via [`open_store`] and run the migration.
///
/// Reads the source, writes to the OS-level store, and returns the stripped
/// document bytes plus a report.
pub fn run(src: &str, dry_run: bool, json: bool) -> Result<MigrateOut, MigrateError> {
    let paths = open_store().map_err(MigrateError::new)?;
    run_in(&paths, src, dry_run, json)
}

/// Testable variant with an explicit store root.
///
/// All store writes are performed against `paths`. Construct a
/// tempdir-rooted `StorePaths::new(tmp.path())` in tests.
pub fn run_in(
    paths: &StorePaths,
    src: &str,
    dry_run: bool,
    json: bool,
) -> Result<MigrateOut, MigrateError> {
    // ── Parse ─────────────────────────────────────────────────────────────────
    let doc = KdlAdapter
        .parse(src.as_bytes())
        .map_err(|e| MigrateError::new(format!("parse error: {}", e.message)))?;

    // ── Require doc-id ────────────────────────────────────────────────────────
    let doc_id = doc.doc_id.as_deref().ok_or_else(|| {
        MigrateError::new("document has no doc-id; run `zenith sync` first to mint one")
    })?;

    // ── Collect candidate pages (any of the 5 fields set) ────────────────────
    let candidate_pages: Vec<&Page> = doc
        .body
        .pages
        .iter()
        .filter(|p| {
            p.workspace_role.is_some()
                || p.candidate_status.is_some()
                || p.notes.is_some()
                || p.promotion_target.is_some()
                || p.cleanup_policy.is_some()
        })
        .collect();

    let n_runs = doc.agent_runs.len();
    let n_previews = doc.previews.len();
    let n_candidates = candidate_pages.len();

    // ── Nothing to migrate? ───────────────────────────────────────────────────
    if n_runs == 0 && n_previews == 0 && n_candidates == 0 {
        let stripped_bytes = format_document(&doc)
            .map_err(|e| MigrateError::new(format!("format error: {}", e.message)))?;
        let report = if json {
            serialize_pretty(&serde_json::json!({
                "runs": 0,
                "previews": 0,
                "candidates": 0,
                "warnings": []
            }))
        } else {
            "nothing to migrate".to_owned()
        };
        return Ok(MigrateOut {
            stripped_bytes,
            report,
        });
    }

    let mut warnings: Vec<String> = Vec::new();

    let fs = OsFs;
    let clock = OsClock;

    // ── Migrate to store (skipped in dry-run) ────────────────────────────────
    if !dry_run {
        // Agent runs.
        for (seq, run) in doc.agent_runs.iter().enumerate() {
            let record = agent_run_to_record(run, seq as u64);
            append_run(&fs, paths, doc_id, &record).map_err(|e| {
                MigrateError::new(format!("failed to write run '{}': {}", run.id, e.message))
            })?;
        }

        // Previews — base seq = number of previews already in the store.
        let base_seq = read_previews(&fs, paths, doc_id)
            .map_err(|e| MigrateError::new(format!("failed to read previews: {}", e.message)))?
            .len() as u64;

        for (i, preview) in doc.previews.iter().enumerate() {
            let seq = base_seq + i as u64;
            let record = preview_to_record(preview, seq);
            append_preview(&fs, paths, doc_id, &record).map_err(|e| {
                MigrateError::new(format!(
                    "failed to write preview '{}': {}",
                    preview.candidate, e.message
                ))
            })?;
        }

        // Candidate pages.
        for page in &candidate_pages {
            // Build the single-page snapshot doc.
            let snapshot_bytes = build_page_snapshot(&doc, page).map_err(|e| {
                MigrateError::new(format!(
                    "failed to build snapshot for page '{}': {}",
                    page.id, e
                ))
            })?;

            // Map candidate_status string to CandidateStatus.
            let status = match page.candidate_status.as_deref() {
                Some("draft") | None => CandidateStatus::Draft,
                Some("selected") => CandidateStatus::Selected,
                Some("rejected") => CandidateStatus::Rejected,
                Some(other) => {
                    warnings.push(format!(
                        "page '{}': unknown candidate-status {:?}; stored as Draft",
                        page.id, other
                    ));
                    CandidateStatus::Draft
                }
            };

            let meta = CandidateMeta {
                workspace_role: page.workspace_role.as_deref(),
                promotion_target: page.promotion_target.as_deref(),
                cleanup_policy: page.cleanup_policy.as_deref(),
                notes: page.notes.as_deref(),
            };

            put_scratch(
                &fs,
                paths,
                &clock,
                doc_id,
                NewCandidate {
                    page_id: &page.id,
                    snapshot: &snapshot_bytes,
                    status,
                    meta,
                },
            )
            .map_err(|e| {
                MigrateError::new(format!(
                    "failed to write candidate for page '{}': {}",
                    page.id, e.message
                ))
            })?;
        }
    }

    // ── Build stripped document ───────────────────────────────────────────────
    let mut stripped = doc.clone();
    stripped.agent_runs.clear();
    stripped.previews.clear();
    for page in stripped.body.pages.iter_mut() {
        page.workspace_role = None;
        page.candidate_status = None;
        page.notes = None;
        page.promotion_target = None;
        page.cleanup_policy = None;
    }

    let stripped_bytes = format_document(&stripped)
        .map_err(|e| MigrateError::new(format!("format error: {}", e.message)))?;

    // ── Build report ──────────────────────────────────────────────────────────
    let report = if json {
        serialize_pretty(&serde_json::json!({
            "runs": n_runs,
            "previews": n_previews,
            "candidates": n_candidates,
            "warnings": warnings,
        }))
    } else {
        let mut lines = Vec::new();
        if dry_run {
            lines.push(format!(
                "dry-run: would migrate {} run(s), {} preview(s), {} candidate(s)",
                n_runs, n_previews, n_candidates
            ));
        } else {
            lines.push(format!(
                "migrated {} run(s), {} preview(s), {} candidate(s) into the store",
                n_runs, n_previews, n_candidates
            ));
        }
        for w in &warnings {
            lines.push(format!("warning: {w}"));
        }
        lines.join("\n")
    };

    Ok(MigrateOut {
        stripped_bytes,
        report,
    })
}

// ── Mapping helpers ───────────────────────────────────────────────────────────

/// Map a [`PropertyValue`] to its flat store string representation.
///
/// Must be an exhaustive match — no `_` wildcard arm — because `PropertyValue`
/// is a Zenith enum and a new variant must force a compile error here.
fn property_value_to_string(pv: &PropertyValue) -> String {
    match pv {
        PropertyValue::TokenRef(id) => format!("(token)\"{id}\""),
        PropertyValue::Literal(s) => format!("\"{s}\""),
        PropertyValue::Dimension(d) => d.to_kdl_string(),
    }
}

/// Convert an [`AgentRun`] AST record to a [`RunRecord`] for the store.
fn agent_run_to_record(run: &AgentRun, seq: u64) -> RunRecord {
    let steps = run
        .steps
        .iter()
        .map(|step| {
            // params: Vec<AgentStepParam> → BTreeMap<String, String> (last write wins)
            let mut params: BTreeMap<String, String> = BTreeMap::new();
            for param in &step.params {
                params.insert(param.name.clone(), property_value_to_string(&param.value));
            }

            // diagnostics: Vec<AgentStepDiagnostic> → Vec<RunDiagnostic>
            let diagnostics: Vec<RunDiagnostic> = step
                .diagnostics
                .iter()
                .map(|d| RunDiagnostic {
                    severity: d.severity.clone(),
                    code: d.code.clone(),
                    message: d.message.clone(),
                })
                .collect();

            RunStep {
                id: step.id.clone(),
                parent: step.parent.clone(),
                action: step.action.clone(),
                action_version: step.action_version.clone(),
                action_hash: step.action_hash.clone(),
                params,
                affected_nodes: step.affected_nodes.clone(),
                diagnostics,
                source_hash: step.source_hash.clone(),
            }
        })
        .collect();

    RunRecord {
        id: run.id.clone(),
        seq,
        brief: run.brief.clone(),
        constraints: run.constraints.clone(),
        plan: run.plan.clone(),
        steps,
        timestamp_ms: None,
        snapshot_hash: None,
    }
}

/// Convert a [`PreviewArtifact`] AST record to a [`PreviewRecord`] for the store.
fn preview_to_record(preview: &PreviewArtifact, seq: u64) -> PreviewRecord {
    let critiques: Vec<PreviewCritique> = preview
        .critiques
        .iter()
        .map(|c| PreviewCritique {
            severity: c.severity.clone(),
            code: c.code.clone(),
            message: c.message.clone(),
        })
        .collect();

    PreviewRecord {
        id: format!("prev-{seq}"),
        seq,
        candidate_page_id: preview.candidate.clone(),
        source_hash: preview.source_hash.clone(),
        output: preview.output.clone(),
        output_hash: preview.output_hash.clone(),
        parent_revision: preview.parent_revision.clone(),
        critiques,
        timestamp_ms: None,
    }
}

/// Build a single-page snapshot document from the full doc and one page.
///
/// Clones the full document, clears `agent_runs` and `previews`, strips all
/// five candidate fields on the target page, and sets `body.pages` to just
/// that single cleaned page. Returns the formatted bytes.
fn build_page_snapshot(doc: &zenith_core::Document, page: &Page) -> Result<Vec<u8>, String> {
    let mut snap_doc = doc.clone();
    snap_doc.agent_runs.clear();
    snap_doc.previews.clear();

    // Find the page, clear its candidate fields, then set body.pages to just it.
    let mut snap_page = page.clone();
    snap_page.workspace_role = None;
    snap_page.candidate_status = None;
    snap_page.notes = None;
    snap_page.promotion_target = None;
    snap_page.cleanup_policy = None;

    snap_doc.body.pages = vec![snap_page];

    format_document(&snap_doc).map_err(|e| e.message)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use zenith_core::{Dimension, Unit};

    #[test]
    fn property_value_token_ref() {
        let pv = PropertyValue::TokenRef("color.bg".to_owned());
        assert_eq!(property_value_to_string(&pv), "(token)\"color.bg\"");
    }

    #[test]
    fn property_value_literal() {
        let pv = PropertyValue::Literal("#ff0000".to_owned());
        assert_eq!(property_value_to_string(&pv), "\"#ff0000\"");
    }

    #[test]
    fn property_value_dimension_integral() {
        let pv = PropertyValue::Dimension(Dimension {
            value: 100.0,
            unit: Unit::Px,
        });
        assert_eq!(property_value_to_string(&pv), "(px)100");
    }

    #[test]
    fn property_value_dimension_fractional() {
        let pv = PropertyValue::Dimension(Dimension {
            value: 13.5,
            unit: Unit::Pt,
        });
        assert_eq!(property_value_to_string(&pv), "(pt)13.5");
    }
}
