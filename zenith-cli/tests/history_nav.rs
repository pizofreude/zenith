//! Integration tests for the history navigation functions in `zenith-cli`.
//!
//! Calls `zenith_cli::history::{record_edit_in, history_view_in, undo_edit_in,
//! redo_edit_in}` directly with a tempdir-rooted `StorePaths` so no real data
//! directory is touched.

use std::path::PathBuf;

use tempfile::TempDir;
use zenith_cli::history::{
    NavOutcome, history_view_in, record_edit_in, redo_edit_in, undo_edit_in,
};
use zenith_session::StorePaths;

// ── Fixture ───────────────────────────────────────────────────────────────────

/// A minimal valid `.zen` document (no `doc-id` attribute yet).
const MINIMAL_NO_ID: &str = r##"zenith version=1 {
  project id="proj.hist" name="History Test"
  tokens format="zenith-token-v1" {
    token id="color.bg" type="color" value="#f8fafc"
  }
  styles {
  }
  document id="doc.hist" title="History Test" {
    page id="page.one" w=(px)480 h=(px)160 {
      rect id="rect.bg" x=(px)0 y=(px)0 w=(px)480 h=(px)160 fill=(token)"color.bg"
    }
  }
}
"##;

fn store_in(tmp: &TempDir) -> StorePaths {
    StorePaths::new(tmp.path())
}

fn doc_path_in(tmp: &TempDir) -> PathBuf {
    tmp.path().join("nav-test.zen")
}

/// Record content A into the store, then produce a distinct content B by
/// substituting one numeric value. Returns `(bytes_a_stamped, bytes_b)`.
fn two_distinct_edits(paths: &StorePaths, doc_path: &PathBuf) -> (Vec<u8>, Vec<u8>) {
    // First edit: mints the doc-id and stamps it.
    let first = record_edit_in(paths, MINIMAL_NO_ID.as_bytes(), doc_path, "tx.apply");
    assert!(first.warning.is_none(), "first edit must have no warning");

    // Write the stamped bytes to disk so doc_id_at can read them.
    std::fs::write(doc_path, &first.bytes).unwrap();

    // Build a distinct second version by replacing a width value.
    let second_src = String::from_utf8(first.bytes.clone())
        .unwrap()
        .replace("w=(px)480", "w=(px)500");
    let second = record_edit_in(paths, second_src.as_bytes(), doc_path, "tx.apply");
    assert!(second.warning.is_none(), "second edit must have no warning");

    // Write the second stamped bytes to disk.
    std::fs::write(doc_path, &second.bytes).unwrap();

    (first.bytes, second.bytes)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// After two distinct edits, `history_view_in` must return at least 2 versions
/// and a 26-character doc-id.
#[test]
fn history_view_lists_versions() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    two_distinct_edits(&paths, &doc_path);

    let view = history_view_in(&paths, &doc_path).expect("history_view_in must succeed");

    assert_eq!(
        view.doc_id.len(),
        26,
        "doc-id must be a 26-character ULID; got: {:?}",
        view.doc_id
    );
    assert!(
        view.versions.len() >= 2,
        "expected at least 2 versions after two distinct edits; got {}",
        view.versions.len()
    );
}

/// `undo_edit_in` followed by `redo_edit_in` must restore the original file
/// content at each step.
#[test]
fn undo_then_redo_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    let (bytes_a, bytes_b) = two_distinct_edits(&paths, &doc_path);

    // Disk currently holds bytes_b (the second edit).
    let on_disk_before = std::fs::read(&doc_path).unwrap();
    assert_eq!(
        on_disk_before, bytes_b,
        "disk must hold state B before undo"
    );

    // Undo: should restore state A.
    let outcome = undo_edit_in(&paths, &doc_path).expect("undo_edit_in must succeed");
    assert!(
        matches!(outcome, NavOutcome::Moved),
        "undo must return Moved when a previous state exists"
    );
    let on_disk_after_undo = std::fs::read(&doc_path).unwrap();
    assert_eq!(
        on_disk_after_undo, bytes_a,
        "disk must hold state A after undo"
    );

    // Redo: should restore state B again.
    let outcome = redo_edit_in(&paths, &doc_path).expect("redo_edit_in must succeed");
    assert!(
        matches!(outcome, NavOutcome::Moved),
        "redo must return Moved after an undo"
    );
    let on_disk_after_redo = std::fs::read(&doc_path).unwrap();
    assert_eq!(
        on_disk_after_redo, bytes_b,
        "disk must hold state B after redo"
    );
}

/// After a single edit (one session state, no parent), `undo_edit_in` must
/// return `NothingToDo` and leave the file unchanged.
#[test]
fn undo_at_root_is_nothing_to_do() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // Single edit: establishes the root state with no parent.
    let first = record_edit_in(&paths, MINIMAL_NO_ID.as_bytes(), &doc_path, "tx.apply");
    assert!(first.warning.is_none(), "first edit must have no warning");
    std::fs::write(&doc_path, &first.bytes).unwrap();

    let before = std::fs::read(&doc_path).unwrap();

    let outcome = undo_edit_in(&paths, &doc_path).expect("undo_edit_in must succeed");
    assert!(
        matches!(outcome, NavOutcome::NothingToDo),
        "undo at root must return NothingToDo"
    );

    let after = std::fs::read(&doc_path).unwrap();
    assert_eq!(
        before, after,
        "file must be unchanged after NothingToDo undo"
    );
}

/// A fresh `.zen` file that was never recorded through the history pipeline has
/// no `doc-id` attribute; `history_view_in` must return an `Err` whose message
/// contains "no history".
#[test]
fn history_view_no_doc_id_errors() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // Write the fixture directly without going through record_edit_in (no doc-id).
    std::fs::write(&doc_path, MINIMAL_NO_ID.as_bytes()).unwrap();

    let result = history_view_in(&paths, &doc_path);
    assert!(
        result.is_err(),
        "history_view_in must error for a doc with no doc-id"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("no history"),
        "error message must mention 'no history'; got: {msg:?}"
    );
}
