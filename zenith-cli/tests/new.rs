//! Integration tests for `zenith new` (document scaffolding).
//!
//! Calls `zenith_cli::commands::new::run_in` directly with a tempdir-rooted
//! `StorePaths` so no real data directory is touched. Verifies that the
//! scaffolded file parses, validates clean, and carries a 26-char doc-id; that
//! an existing path is never overwritten; and that the slug/name derivation
//! works from both `--name` and the file stem.

use std::path::PathBuf;

use tempfile::TempDir;
use zenith_cli::commands::new::{DEFAULT_PAGE, PaperFormat, resolve_page};
use zenith_cli::commands::{new, validate};
use zenith_core::{KdlAdapter, KdlSource as _};
use zenith_session::StorePaths;

fn store_in(tmp: &TempDir) -> StorePaths {
    StorePaths::new(tmp.path())
}

fn doc_path(tmp: &TempDir, name: &str) -> PathBuf {
    tmp.path().join(name)
}

/// `zenith new` must create a file that parses, validates clean, and carries a
/// 26-character ULID doc-id.
#[test]
fn creates_valid_document_with_doc_id() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = doc_path(&tmp, "poster.zen");

    let result =
        new::run_in(&paths, &path, Some("Launch Poster"), DEFAULT_PAGE).expect("new must succeed");
    assert_eq!(
        result.doc_id.len(),
        26,
        "doc-id must be a 26-char ULID; got {:?}",
        result.doc_id
    );

    let bytes = std::fs::read(&path).expect("created file must exist");
    let doc = KdlAdapter.parse(&bytes).expect("created file must parse");
    assert_eq!(doc.doc_id.as_deref(), Some(result.doc_id.as_str()));

    let src = String::from_utf8(bytes).expect("utf8");
    let out = validate::run(
        &src,
        path.parent(),
        false,
        &zenith_cli::config::CliPolicyFlags::default(),
    );
    assert_eq!(
        out.exit_code, 0,
        "scaffolded document must validate clean; got:\n{}",
        out.stdout
    );
}

/// `zenith new` must refuse to overwrite an existing path without modifying it.
#[test]
fn refuses_to_overwrite_existing_file() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = doc_path(&tmp, "exists.zen");
    std::fs::write(&path, b"ORIGINAL").unwrap();

    let err = new::run_in(&paths, &path, None, DEFAULT_PAGE).expect_err("must refuse to overwrite");
    assert_ne!(err.exit_code, 0, "overwrite refusal must be non-zero exit");
    assert!(
        err.message.contains("refusing to overwrite"),
        "message must explain the refusal; got: {}",
        err.message
    );

    let on_disk = std::fs::read(&path).unwrap();
    assert_eq!(on_disk, b"ORIGINAL", "existing file must be left untouched");
}

/// The id slug must come from `--name` when provided.
#[test]
fn slug_derives_from_name() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = doc_path(&tmp, "whatever.zen");

    new::run_in(&paths, &path, Some("Acme Brand"), DEFAULT_PAGE).expect("new must succeed");
    let src = std::fs::read_to_string(&path).unwrap();
    assert!(
        src.contains("doc.acme-brand"),
        "doc id from name; got:\n{src}"
    );
    assert!(
        src.contains("proj.acme-brand"),
        "proj id from name; got:\n{src}"
    );
}

/// With no `--name`, the slug must come from the path's file stem and the title
/// defaults to "Untitled".
#[test]
fn slug_derives_from_file_stem() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = doc_path(&tmp, "my-flyer.zen");

    new::run_in(&paths, &path, None, DEFAULT_PAGE).expect("new must succeed");
    let src = std::fs::read_to_string(&path).unwrap();
    assert!(
        src.contains("doc.my-flyer"),
        "doc id from stem; got:\n{src}"
    );
    assert!(src.contains("Untitled"), "default title; got:\n{src}");
}

/// A path with no extension gets a default `.zen` appended; the returned path
/// reflects the file actually written.
#[test]
fn appends_zen_extension_when_absent() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = doc_path(&tmp, "poster"); // no extension

    let result =
        new::run_in(&paths, &path, Some("Poster"), DEFAULT_PAGE).expect("new must succeed");
    let expected = tmp.path().join("poster.zen");
    assert_eq!(result.path, expected, "must append .zen");
    assert!(expected.exists(), "poster.zen must be created");
    assert!(
        !path.exists(),
        "the extension-less path must not be created"
    );
}

/// Missing parent directories are created so `new sub/dir/doc.zen` just works.
#[test]
fn creates_missing_parent_directories() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = tmp.path().join("sub").join("deep").join("doc.zen");

    new::run_in(&paths, &path, Some("Deep"), DEFAULT_PAGE).expect("new must create parent dirs");
    assert!(path.exists(), "doc.zen must be created in the new subtree");
}

/// When the history store is unavailable, `zenith new` must still create a valid
/// document with a freshly minted doc-id (recording is best-effort), surfacing the
/// failure as a non-fatal warning rather than aborting.
#[test]
fn creates_document_when_history_store_unavailable() {
    let tmp = TempDir::new().unwrap();
    // Root the store at a regular FILE, so every attempt to create a doc dir
    // under it fails — simulating an unwritable / unavailable history store.
    let blocker = tmp.path().join("store-is-a-file");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let paths = StorePaths::new(&blocker);

    let path = doc_path(&tmp, "resilient.zen");
    let result = new::run_in(&paths, &path, Some("Resilient"), DEFAULT_PAGE)
        .expect("new must succeed despite no store");

    assert_eq!(
        result.doc_id.len(),
        26,
        "a doc-id must still be minted; got {:?}",
        result.doc_id
    );
    assert!(
        result.warning.is_some(),
        "the store failure must surface as a warning"
    );

    let bytes = std::fs::read(&path).expect("created file must exist");
    let doc = KdlAdapter.parse(&bytes).expect("created file must parse");
    assert_eq!(
        doc.doc_id.as_deref(),
        Some(result.doc_id.as_str()),
        "the minted doc-id must be stamped into the written file"
    );
}

/// A named `--format` sets the page dimensions, and the scaffolded document
/// still validates clean.
#[test]
fn format_sets_page_dimensions() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = doc_path(&tmp, "flyer.zen");

    let page = resolve_page(Some(PaperFormat::A4), None, None, false, 1).unwrap();
    new::run_in(&paths, &path, Some("Flyer"), page).expect("new must succeed");

    let src = std::fs::read_to_string(&path).unwrap();
    assert!(src.contains("(px)794"), "A4 width; got:\n{src}");
    assert!(src.contains("(px)1123"), "A4 height; got:\n{src}");

    let out = validate::run(
        &src,
        path.parent(),
        false,
        &zenith_cli::config::CliPolicyFlags::default(),
    );
    assert_eq!(
        out.exit_code, 0,
        "A4 document must validate; got:\n{}",
        out.stdout
    );
}

/// Explicit dimensions plus a multi-page count produce that many `page.N` nodes
/// at the requested size, and the whole document validates clean.
#[test]
fn explicit_dimensions_and_multiple_pages() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let path = doc_path(&tmp, "deck.zen");

    let page = resolve_page(None, Some(1600), Some(900), false, 3).unwrap();
    new::run_in(&paths, &path, Some("Deck"), page).expect("new must succeed");

    let src = std::fs::read_to_string(&path).unwrap();
    assert!(
        src.contains("(px)1600") && src.contains("(px)900"),
        "dims; got:\n{src}"
    );
    for id in ["page.1", "page.2", "page.3"] {
        assert!(src.contains(id), "missing {id}; got:\n{src}");
    }
    assert!(
        !src.contains("page.4"),
        "must not over-create pages; got:\n{src}"
    );

    let out = validate::run(
        &src,
        path.parent(),
        false,
        &zenith_cli::config::CliPolicyFlags::default(),
    );
    assert_eq!(
        out.exit_code, 0,
        "multi-page document must validate; got:\n{}",
        out.stdout
    );
}

/// A directory target is rejected with a clear message (not "file").
#[test]
fn rejects_directory_path() {
    let tmp = TempDir::new().unwrap();
    let paths = store_in(&tmp);
    let dir = tmp.path().join("adir");
    std::fs::create_dir(&dir).unwrap();

    let err = new::run_in(&paths, &dir, None, DEFAULT_PAGE).expect_err("must reject a directory");
    assert_ne!(err.exit_code, 0);
    assert!(
        err.message.contains("is a directory"),
        "message must identify a directory; got: {}",
        err.message
    );
}
