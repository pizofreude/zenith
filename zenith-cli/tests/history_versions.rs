//! Integration tests for `zenith version` and `zenith restore` CLI operations.
//!
//! Calls `zenith_cli::history::{record_edit_in, name_version_in, restore_in}`
//! directly with a tempdir-rooted `StorePaths` so no real data directory is
//! touched.

use std::path::PathBuf;

use tempfile::TempDir;
use zenith_cli::history::{name_version_in, record_edit_in, restore_in};
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
    tmp.path().join("version-test.zen")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// After a `record_edit_in` (which stamps the doc-id and creates v0), calling
/// `name_version_in` must succeed and the returned version id must appear in the
/// version list with the expected label.
#[test]
fn name_version_creates_named_version() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // First edit: mints the doc-id, stamps it, and creates the first version.
    let first = record_edit_in(&paths, MINIMAL_NO_ID.as_bytes(), &doc_path, "tx.apply");
    assert!(first.warning.is_none(), "first edit must have no warning");
    std::fs::write(&doc_path, &first.bytes).unwrap();

    // Save a named version.
    let version_id =
        name_version_in(&paths, &doc_path, "release-1").expect("name_version_in must succeed");
    assert!(
        !version_id.is_empty(),
        "returned version id must not be empty"
    );

    // Verify the version appears in the list with the correct label.
    let fs = zenith_session::adapter::OsFs;
    let doc_id = {
        use zenith_core::{KdlAdapter, KdlSource as _};
        let bytes = std::fs::read(&doc_path).unwrap();
        let doc = KdlAdapter.parse(&bytes).unwrap();
        doc.doc_id
            .expect("doc must have a doc-id after record_edit_in")
    };
    let versions =
        zenith_session::list_versions(&fs, &paths, &doc_id).expect("list_versions must succeed");

    let named = versions
        .iter()
        .find(|v| v.id == version_id)
        .expect("the returned version id must appear in the version list");
    assert_eq!(
        named.label.as_deref(),
        Some("release-1"),
        "named version must carry the label 'release-1'"
    );
}

/// After recording two distinct versions (A then B), `restore_in` with
/// `@head~1` must write version A's bytes back to the file on disk and return
/// the resolved version id of A.
#[test]
fn restore_writes_past_content_back() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // First edit: mints doc-id and creates version A.
    let first = record_edit_in(&paths, MINIMAL_NO_ID.as_bytes(), &doc_path, "tx.apply");
    assert!(first.warning.is_none(), "first edit must have no warning");
    std::fs::write(&doc_path, &first.bytes).unwrap();
    let bytes_a = first.bytes;

    // Build a distinct second version by replacing a width value, then record.
    let second_src = String::from_utf8(bytes_a.clone())
        .unwrap()
        .replace("w=(px)480", "w=(px)500");
    let second = record_edit_in(&paths, second_src.as_bytes(), &doc_path, "tx.apply");
    assert!(second.warning.is_none(), "second edit must have no warning");
    std::fs::write(&doc_path, &second.bytes).unwrap();

    // Disk currently holds version B (second edit).
    let on_disk_before = std::fs::read(&doc_path).unwrap();
    assert_ne!(on_disk_before, bytes_a, "versions A and B must differ");

    // Restore to @head~1 (one step before HEAD — that is, version A).
    let outcome = restore_in(&paths, &doc_path, "@head~1").expect("restore_in must succeed");
    assert!(
        !outcome.version_id.is_empty(),
        "restored version id must not be empty"
    );

    // The file on disk must now hold the content of version A.
    let on_disk_after = std::fs::read(&doc_path).unwrap();
    assert_eq!(
        on_disk_after, bytes_a,
        "disk must hold version A's bytes after restore"
    );
}

/// Attempting to restore an unknown revision spec must return an `Err`.
#[test]
fn restore_unknown_rev_errors() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // Establish a document with a doc-id.
    let first = record_edit_in(&paths, MINIMAL_NO_ID.as_bytes(), &doc_path, "tx.apply");
    assert!(first.warning.is_none());
    std::fs::write(&doc_path, &first.bytes).unwrap();

    let result = restore_in(&paths, &doc_path, "v999");
    assert!(
        result.is_err(),
        "restore_in must return Err for an unknown revision spec"
    );
}

/// A file that has never been processed through the history pipeline has no
/// `doc-id`; `name_version_in` must return an `Err` whose message mentions
/// "no history".
#[test]
fn name_version_no_doc_id_errors() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let doc_path = doc_path_in(&tmp);

    // Write the fixture directly without going through record_edit_in (no doc-id).
    std::fs::write(&doc_path, MINIMAL_NO_ID.as_bytes()).unwrap();

    let result = name_version_in(&paths, &doc_path, "should-fail");
    assert!(
        result.is_err(),
        "name_version_in must error for a doc with no doc-id"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("no history"),
        "error message must mention 'no history'; got: {msg:?}"
    );
}
