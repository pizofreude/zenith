//! Scratch/candidate store: content-addressed `.zen` snapshots indexed in
//! `scratch/index.jsonl`.
//!
//! Each [`CandidateEntry`] records a `page_id` (the page or source being
//! snapshotted), a `snapshot_hash` that addresses the raw `.zen` bytes in the
//! shared object store, a [`CandidateStatus`], and optional workflow metadata.
//!
//! # Append-only contract
//!
//! This module is **append-only**: [`put_scratch`] adds entries; it never
//! mutates existing ones. Draft → Selected / Rejected transitions are handled
//! by a superseding append in a later unit. Do not implement an update/mutate
//! path here.

use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::adapter::{Clock, Fs};
use crate::error::SessionError;
use crate::layout::StorePaths;
use crate::manifest::{append_jsonl_record, read_jsonl_records};
use crate::store::{get_object, put_object};

// ── CandidateStatus ───────────────────────────────────────────────────────────

/// Lifecycle state of a scratch candidate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    /// The candidate is still being evaluated.
    Draft,
    /// The candidate has been chosen for promotion.
    Selected,
    /// The candidate has been discarded.
    Rejected,
}

// ── CandidateEntry ────────────────────────────────────────────────────────────

/// A single scratch candidate record appended to `scratch/index.jsonl`.
///
/// `id` and `seq` are derived by [`put_scratch`] from the current index
/// length; callers do not supply them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateEntry {
    /// Stable candidate id within this document's scratch index (e.g. `cand0`).
    pub id: String,
    /// Monotonic sequence number (0-based, append order).
    pub seq: u64,
    /// The page or source document this candidate snapshots.
    pub page_id: String,
    /// SHA-256 content hash of the stored `.zen` snapshot bytes (in `objects/`).
    pub snapshot_hash: String,
    /// Lifecycle status at the time this entry was appended.
    pub status: CandidateStatus,
    /// Optional workflow role label (e.g. `"hero"`, `"fallback"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_role: Option<String>,
    /// Optional target to promote this candidate to (e.g. a branch or slot id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promotion_target: Option<String>,
    /// Optional policy controlling when this candidate may be cleaned up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup_policy: Option<String>,
    /// Optional free-text notes about this candidate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Unix timestamp in milliseconds when this entry was created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_ms: Option<u128>,
}

// ── CandidateMeta ─────────────────────────────────────────────────────────────

/// Borrowed optional metadata for a new candidate (mirrors `VersionMeta`).
///
/// All fields default to `None`; supply only what is available at call time.
#[derive(Debug, Clone, Copy, Default)]
pub struct CandidateMeta<'a> {
    /// Optional workflow role label for this candidate.
    pub workspace_role: Option<&'a str>,
    /// Optional promotion target for this candidate.
    pub promotion_target: Option<&'a str>,
    /// Optional cleanup policy tag.
    pub cleanup_policy: Option<&'a str>,
    /// Optional free-text notes.
    pub notes: Option<&'a str>,
}

// ── NewCandidate ──────────────────────────────────────────────────────────────

/// The describing inputs for a new candidate snapshot: which page it captures,
/// the `.zen` snapshot bytes, its lifecycle status, and optional metadata.
#[derive(Debug, Clone, Copy)]
pub struct NewCandidate<'a> {
    /// The page or source document this candidate snapshots.
    pub page_id: &'a str,
    /// Raw `.zen` snapshot bytes to store in the object store.
    pub snapshot: &'a [u8],
    /// Lifecycle status for this candidate at creation time.
    pub status: CandidateStatus,
    /// Optional workflow metadata (role, target, policy, notes).
    pub meta: CandidateMeta<'a>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Store a candidate snapshot and append a [`CandidateEntry`] to the scratch
/// index.
///
/// The `.zen` bytes in `candidate.snapshot` are written to the shared object
/// store (content-addressed, idempotent). `seq` and `id` are derived from the
/// current index length so callers do not need to track them. Returns the
/// created entry.
pub fn put_scratch(
    fs: &impl Fs,
    paths: &StorePaths,
    clock: &impl Clock,
    doc_id: &str,
    candidate: NewCandidate<'_>,
) -> Result<CandidateEntry, SessionError> {
    let snapshot_hash = put_object(fs, paths, doc_id, candidate.snapshot)?;
    let seq = u64::try_from(list_scratch(fs, paths, doc_id)?.len())
        .map_err(|_| SessionError::new("candidate count exceeds u64"))?;
    let id = format!("cand{seq}");
    let timestamp_ms = clock
        .now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis());
    let entry = CandidateEntry {
        id,
        seq,
        page_id: candidate.page_id.to_owned(),
        snapshot_hash,
        status: candidate.status,
        workspace_role: candidate.meta.workspace_role.map(str::to_owned),
        promotion_target: candidate.meta.promotion_target.map(str::to_owned),
        cleanup_policy: candidate.meta.cleanup_policy.map(str::to_owned),
        notes: candidate.meta.notes.map(str::to_owned),
        timestamp_ms,
    };
    append_jsonl_record(fs, &paths.scratch_index(doc_id), &entry)?;
    Ok(entry)
}

/// List all candidate entries for `doc_id` in append order.
///
/// Returns an empty vec when no scratch index exists for the document.
pub fn list_scratch(
    fs: &impl Fs,
    paths: &StorePaths,
    doc_id: &str,
) -> Result<Vec<CandidateEntry>, SessionError> {
    read_jsonl_records(fs, &paths.scratch_index(doc_id))
}

/// Recover the stored `.zen` snapshot bytes for a candidate entry.
///
/// Decompresses and verifies the object addressed by `entry.snapshot_hash`.
pub fn get_scratch_snapshot(
    fs: &impl Fs,
    paths: &StorePaths,
    doc_id: &str,
    entry: &CandidateEntry,
) -> Result<Vec<u8>, SessionError> {
    get_object(fs, paths, doc_id, &entry.snapshot_hash)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::adapter::{FakeClock, MemFs};
    use crate::layout::StorePaths;

    fn setup() -> (MemFs, StorePaths) {
        (MemFs::new(), StorePaths::new("/data"))
    }

    fn clock() -> FakeClock {
        FakeClock(UNIX_EPOCH + Duration::from_millis(100))
    }

    #[test]
    fn put_then_list_scratch_roundtrip() {
        let (fs, paths) = setup();
        let clk = clock();

        let meta_full = CandidateMeta {
            workspace_role: Some("hero"),
            promotion_target: Some("slot-a"),
            cleanup_policy: Some("on_select"),
            notes: Some("first pass"),
        };
        let e0 = put_scratch(
            &fs,
            &paths,
            &clk,
            "doc1",
            NewCandidate {
                page_id: "page-a",
                snapshot: b"zen content A",
                status: CandidateStatus::Draft,
                meta: meta_full,
            },
        )
        .unwrap();

        let e1 = put_scratch(
            &fs,
            &paths,
            &clk,
            "doc1",
            NewCandidate {
                page_id: "page-b",
                snapshot: b"zen content B",
                status: CandidateStatus::Selected,
                meta: CandidateMeta::default(),
            },
        )
        .unwrap();

        assert_eq!(e0.seq, 0);
        assert_eq!(e0.id, "cand0");
        assert_eq!(e0.page_id, "page-a");
        assert_eq!(e0.status, CandidateStatus::Draft);
        assert_eq!(e0.workspace_role, Some("hero".to_owned()));
        assert_eq!(e0.promotion_target, Some("slot-a".to_owned()));
        assert_eq!(e0.cleanup_policy, Some("on_select".to_owned()));
        assert_eq!(e0.notes, Some("first pass".to_owned()));

        assert_eq!(e1.seq, 1);
        assert_eq!(e1.id, "cand1");
        assert_eq!(e1.page_id, "page-b");
        assert_eq!(e1.status, CandidateStatus::Selected);
        assert_eq!(e1.workspace_role, None);

        let entries = list_scratch(&fs, &paths, "doc1").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], e0);
        assert_eq!(entries[1], e1);
    }

    #[test]
    fn snapshot_bytes_recovered_intact() {
        let (fs, paths) = setup();
        let clk = clock();
        let zen_bytes = b"node layout { width 100 }";

        let entry = put_scratch(
            &fs,
            &paths,
            &clk,
            "doc1",
            NewCandidate {
                page_id: "page-x",
                snapshot: zen_bytes,
                status: CandidateStatus::Draft,
                meta: CandidateMeta::default(),
            },
        )
        .unwrap();

        let recovered = get_scratch_snapshot(&fs, &paths, "doc1", &entry).unwrap();
        assert_eq!(recovered.as_slice(), zen_bytes.as_slice());
    }

    #[test]
    fn lean_candidate_omits_optionals() {
        let (fs, paths) = setup();
        let clk = clock();

        put_scratch(
            &fs,
            &paths,
            &clk,
            "doc1",
            NewCandidate {
                page_id: "page-lean",
                snapshot: b"lean",
                status: CandidateStatus::Draft,
                meta: CandidateMeta::default(),
            },
        )
        .unwrap();

        let raw = fs.read(&paths.scratch_index("doc1")).unwrap();
        let line = std::str::from_utf8(&raw).unwrap();

        assert!(
            !line.contains("workspace_role"),
            "workspace_role must be absent in lean form"
        );
        assert!(
            !line.contains("promotion_target"),
            "promotion_target must be absent in lean form"
        );
        assert!(
            !line.contains("cleanup_policy"),
            "cleanup_policy must be absent in lean form"
        );
        assert!(
            !line.contains("\"notes\""),
            "notes must be absent in lean form"
        );
    }

    #[test]
    fn status_serializes_snake_case() {
        let (fs, paths) = setup();
        let clk = clock();

        put_scratch(
            &fs,
            &paths,
            &clk,
            "doc1",
            NewCandidate {
                page_id: "page-sel",
                snapshot: b"sel",
                status: CandidateStatus::Selected,
                meta: CandidateMeta::default(),
            },
        )
        .unwrap();

        let raw = fs.read(&paths.scratch_index("doc1")).unwrap();
        let line = std::str::from_utf8(&raw).unwrap();
        assert!(
            line.contains("\"selected\""),
            "Selected status must serialize as \"selected\""
        );
    }

    #[test]
    fn list_scratch_absent_is_empty() {
        let (fs, paths) = setup();
        let entries = list_scratch(&fs, &paths, "no-such-doc").unwrap();
        assert!(entries.is_empty());
    }
}
