//! Integration tests for `zenith migrate`.
//!
//! Uses the `run_in` variant with an explicit `StorePaths` backed by a
//! tempdir — no real data directory is touched. Mirrors the setup pattern
//! from `workspace.rs`.

use tempfile::TempDir;
use zenith_cli::commands::migrate::run_in;
use zenith_session::adapter::OsFs;
use zenith_session::{StorePaths, list_scratch, read_previews, read_runs};

// ── Fixture ───────────────────────────────────────────────────────────────────

/// A `.zen` document that has:
/// - a `doc-id`
/// - an `agent-runs` block with 1 run / 2 steps / 1 param / 1 diagnostic
/// - a `previews` block with 1 preview / 1 critique
/// - a page with all 5 candidate metadata fields set
const MIGRATE_FIXTURE: &str = r##"zenith version=1 doc-id="01HWMIGRATE000000000000001" {
  project id="proj.mig" name="Migrate Test"
  tokens format="zenith-token-v1" {
    token id="color.bg" type="color" value="#ffffff"
  }
  styles {
  }
  agent-runs {
    run id="run.1" brief="Layout pass" {
      constraints "Stay within safe zone."
      plan "1. Place header. 2. Place body."
      step id="step.1" action="read_file" {
        affected-node "node.header"
        param name="path" value="layout.zen"
        diagnostic severity="warn" code="agent.overlap" message="2px overlap detected"
        source-hash "abc123"
      }
      step id="step.2" action="write_node" parent="step.1" action-version="write_node@2" {
        affected-node "node.header"
      }
    }
  }
  previews {
    preview candidate="page.main" source-hash="src456" output="out/main.png" output-hash="out789" parent-revision="rev.1" {
      critique severity="warning" code="preview.contrast" message="Contrast ratio too low"
    }
  }
  document id="doc.mig" title="Migrate Test" {
    page id="page.main" w=(px)800 h=(px)600 workspace-role="exploration" candidate-status="draft" notes="First pass" promotion-target="page.final" cleanup-policy="on_select" {
      rect id="node.header" x=(px)0 y=(px)0 w=(px)800 h=(px)80 fill=(token)"color.bg"
    }
  }
}
"##;

const DOC_ID: &str = "01HWMIGRATE000000000000001";

fn setup() -> (TempDir, StorePaths) {
    let tmp = TempDir::new().unwrap();
    let paths = StorePaths::new(tmp.path());
    (tmp, paths)
}

// ── Happy path: full migration ─────────────────────────────────────────────────

#[test]
fn migrate_writes_runs_to_store() {
    let (_tmp, paths) = setup();

    let out = run_in(&paths, MIGRATE_FIXTURE, false, false).unwrap();

    let fs = OsFs;
    let runs = read_runs(&fs, &paths, DOC_ID).unwrap();
    assert_eq!(runs.len(), 1, "must have 1 run after migration");
    assert_eq!(runs[0].id, "run.1");
    assert_eq!(runs[0].brief.as_deref(), Some("Layout pass"));
    assert_eq!(runs[0].seq, 0);
    assert_eq!(runs[0].steps.len(), 2);

    // step.1 params should be present
    let s1 = &runs[0].steps[0];
    assert_eq!(s1.id, "step.1");
    assert_eq!(s1.action, "read_file");
    assert!(s1.params.contains_key("path"), "params must contain 'path'");
    assert_eq!(s1.params["path"], "\"layout.zen\"");
    assert_eq!(s1.affected_nodes, vec!["node.header"]);
    assert_eq!(s1.diagnostics.len(), 1);
    assert_eq!(s1.diagnostics[0].code, "agent.overlap");

    // step.2
    let s2 = &runs[0].steps[1];
    assert_eq!(s2.id, "step.2");
    assert_eq!(s2.parent.as_deref(), Some("step.1"));
    assert_eq!(s2.action_version.as_deref(), Some("write_node@2"));

    // stripped bytes must not contain agent-runs
    let stripped = std::str::from_utf8(&out.stripped_bytes).unwrap();
    assert!(
        !stripped.contains("agent-runs"),
        "stripped doc must not contain agent-runs; got:\n{stripped}"
    );
}

#[test]
fn migrate_writes_previews_to_store() {
    let (_tmp, paths) = setup();

    run_in(&paths, MIGRATE_FIXTURE, false, false).unwrap();

    let fs = OsFs;
    let previews = read_previews(&fs, &paths, DOC_ID).unwrap();
    assert_eq!(previews.len(), 1, "must have 1 preview after migration");
    assert_eq!(previews[0].candidate_page_id, "page.main");
    assert_eq!(previews[0].source_hash.as_deref(), Some("src456"));
    assert_eq!(previews[0].output.as_deref(), Some("out/main.png"));
    assert_eq!(previews[0].output_hash.as_deref(), Some("out789"));
    assert_eq!(previews[0].parent_revision.as_deref(), Some("rev.1"));
    assert_eq!(previews[0].critiques.len(), 1);
    assert_eq!(previews[0].critiques[0].code, "preview.contrast");
    assert_eq!(previews[0].seq, 0);
    assert_eq!(previews[0].id, "prev-0");
}

#[test]
fn migrate_writes_candidate_to_store() {
    let (_tmp, paths) = setup();

    run_in(&paths, MIGRATE_FIXTURE, false, false).unwrap();

    let fs = OsFs;
    let candidates = list_scratch(&fs, &paths, DOC_ID).unwrap();
    assert_eq!(candidates.len(), 1, "must have 1 candidate after migration");
    let cand = &candidates[0];
    assert_eq!(cand.page_id, "page.main");
    assert_eq!(
        cand.status,
        zenith_session::CandidateStatus::Draft,
        "candidate-status='draft' must map to Draft"
    );
    assert_eq!(cand.workspace_role.as_deref(), Some("exploration"));
    assert_eq!(cand.notes.as_deref(), Some("First pass"));
    assert_eq!(cand.promotion_target.as_deref(), Some("page.final"));
    assert_eq!(cand.cleanup_policy.as_deref(), Some("on_select"));
}

#[test]
fn migrate_stripped_bytes_lack_provenance_fields() {
    let (_tmp, paths) = setup();

    let out = run_in(&paths, MIGRATE_FIXTURE, false, false).unwrap();
    let stripped = std::str::from_utf8(&out.stripped_bytes).unwrap();

    assert!(
        !stripped.contains("agent-runs"),
        "stripped doc must not contain 'agent-runs'; got:\n{stripped}"
    );
    assert!(
        !stripped.contains("previews"),
        "stripped doc must not contain 'previews'; got:\n{stripped}"
    );
    assert!(
        !stripped.contains("candidate-status"),
        "stripped doc must not contain 'candidate-status'; got:\n{stripped}"
    );
    assert!(
        !stripped.contains("workspace-role"),
        "stripped doc must not contain 'workspace-role'; got:\n{stripped}"
    );
    assert!(
        !stripped.contains("promotion-target"),
        "stripped doc must not contain 'promotion-target'; got:\n{stripped}"
    );
    assert!(
        !stripped.contains("cleanup-policy"),
        "stripped doc must not contain 'cleanup-policy'; got:\n{stripped}"
    );

    // doc-id and the page itself must still be present
    assert!(
        stripped.contains(DOC_ID),
        "stripped doc must still contain doc-id; got:\n{stripped}"
    );
    assert!(
        stripped.contains("page.main"),
        "stripped doc must still contain the page id; got:\n{stripped}"
    );
}

#[test]
fn migrate_human_report_includes_counts() {
    let (_tmp, paths) = setup();
    let out = run_in(&paths, MIGRATE_FIXTURE, false, false).unwrap();
    assert!(
        out.report.contains("1 run(s)"),
        "report must mention run count; got: {}",
        out.report
    );
    assert!(
        out.report.contains("1 preview(s)"),
        "report must mention preview count; got: {}",
        out.report
    );
    assert!(
        out.report.contains("1 candidate(s)"),
        "report must mention candidate count; got: {}",
        out.report
    );
}

#[test]
fn migrate_json_report_has_correct_counts() {
    let (_tmp, paths) = setup();
    let out = run_in(&paths, MIGRATE_FIXTURE, false, true).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out.report).expect("must be valid JSON");
    assert_eq!(v["runs"], 1);
    assert_eq!(v["previews"], 1);
    assert_eq!(v["candidates"], 1);
    let warnings = v["warnings"].as_array().unwrap();
    assert!(
        warnings.is_empty(),
        "no warnings expected; got: {warnings:?}"
    );
}

// ── Dry-run ────────────────────────────────────────────────────────────────────

#[test]
fn dry_run_does_not_write_to_store() {
    let (_tmp, paths) = setup();

    run_in(&paths, MIGRATE_FIXTURE, true, false).unwrap();

    let fs = OsFs;
    let runs = read_runs(&fs, &paths, DOC_ID).unwrap();
    assert!(runs.is_empty(), "dry-run must not write runs to store");

    let previews = read_previews(&fs, &paths, DOC_ID).unwrap();
    assert!(
        previews.is_empty(),
        "dry-run must not write previews to store"
    );

    let candidates = list_scratch(&fs, &paths, DOC_ID).unwrap();
    assert!(
        candidates.is_empty(),
        "dry-run must not write candidates to store"
    );
}

#[test]
fn dry_run_report_mentions_dry_run() {
    let (_tmp, paths) = setup();
    let out = run_in(&paths, MIGRATE_FIXTURE, true, false).unwrap();
    assert!(
        out.report.contains("dry-run"),
        "dry-run report must mention 'dry-run'; got: {}",
        out.report
    );
}

// ── Edge cases ─────────────────────────────────────────────────────────────────

#[test]
fn missing_doc_id_returns_error() {
    let (_tmp, paths) = setup();
    let no_id_doc = r##"zenith version=1 {
  project id="proj.noid" name="No ID"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.noid" title="No ID" {
    page id="page.noid" w=(px)400 h=(px)300 {
    }
  }
}
"##;
    let result = run_in(&paths, no_id_doc, false, false);
    assert!(result.is_err(), "missing doc-id must return an error");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("doc-id"),
        "error must mention 'doc-id'; got: {}",
        err.message
    );
    assert_eq!(err.exit_code, 2);
}

#[test]
fn nothing_to_migrate_succeeds_with_unchanged_bytes() {
    let (_tmp, paths) = setup();
    let empty_doc = r##"zenith version=1 doc-id="01HWMIGRATE000000000000002" {
  project id="proj.empty" name="Empty"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.empty" title="Empty" {
    page id="page.empty" w=(px)400 h=(px)300 {
    }
  }
}
"##;
    let out = run_in(&paths, empty_doc, false, false).unwrap();
    assert!(
        out.report.contains("nothing to migrate"),
        "report must say nothing to migrate; got: {}",
        out.report
    );
    // The stripped bytes must still be valid (parseable) UTF-8.
    let s = std::str::from_utf8(&out.stripped_bytes).unwrap();
    assert!(
        s.contains("page.empty"),
        "returned bytes must still contain the page; got:\n{s}"
    );
}

#[test]
fn preview_seq_accounts_for_existing_previews() {
    let (_tmp, paths) = setup();

    // First migration — writes prev-0.
    run_in(&paths, MIGRATE_FIXTURE, false, false).unwrap();

    // Second migration of the same doc (same doc-id) — should compute base_seq=1
    // and write prev-1.
    run_in(&paths, MIGRATE_FIXTURE, false, false).unwrap();

    let fs = OsFs;
    let previews = read_previews(&fs, &paths, DOC_ID).unwrap();
    assert_eq!(previews.len(), 2, "two migrations must produce 2 previews");
    assert_eq!(previews[0].id, "prev-0");
    assert_eq!(previews[0].seq, 0);
    assert_eq!(previews[1].id, "prev-1");
    assert_eq!(previews[1].seq, 1);
}
