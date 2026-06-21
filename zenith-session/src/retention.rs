//! Time-Machine-style retention thinning for Tier-2 version history.
//!
//! Named versions (those with a label) and the latest version are kept forever;
//! all other versions are thinned by age into coarser buckets the older they get
//! (all recent, then hourly, daily, weekly). Thinning rewrites `versions.jsonl`
//! and re-links each kept version's parent to the previous kept version so that
//! `@head~N` walks remain valid. Object GC and storage caps are separate passes.

use std::collections::{BTreeMap, BTreeSet};
use std::time::UNIX_EPOCH;

use crate::adapter::{Clock, Fs};
use crate::error::SessionError;
use crate::layout::StorePaths;
use crate::manifest::{HistoryRecord, read_records};

// ── Time constants (ms) ───────────────────────────────────────────────────────

const MS_PER_HOUR: u128 = 3_600_000;
const MS_PER_DAY: u128 = 86_400_000;
const MS_PER_WEEK: u128 = 604_800_000;

// ── RetentionPolicy ───────────────────────────────────────────────────────────

/// Controls the age windows used by [`thin_versions`] to decide which versions
/// survive thinning.
///
/// All values are in milliseconds. The default matches the Time-Machine windows
/// documented in this module's top-level description.
#[derive(Debug, Clone, PartialEq)]
pub struct RetentionPolicy {
    /// Below this age, every version is kept (ms).
    pub keep_all_below: u128,
    /// Below this age, keep one per hour (ms).
    pub hourly_below: u128,
    /// Below this age, keep one per day (ms).
    pub daily_below: u128,
    // at/above daily_below → weekly buckets
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_all_below: MS_PER_HOUR,
            hourly_below: MS_PER_DAY,
            daily_below: 30 * MS_PER_DAY,
        }
    }
}

// ── Pure selector ─────────────────────────────────────────────────────────────

/// Return the set of version ids to KEEP under `policy` given the current time.
///
/// Named versions and the latest version are always kept; everything else is
/// thinned to the newest version per age bucket. Versions are assumed to be in
/// append order (oldest first); "newest in bucket" = highest seq.
pub fn thin_versions(
    versions: &[HistoryRecord],
    now_ms: u128,
    policy: &RetentionPolicy,
) -> BTreeSet<String> {
    let mut keep: BTreeSet<String> = BTreeSet::new();
    if versions.is_empty() {
        return keep;
    }

    // Always keep the latest (last in append order).
    if let Some(last) = versions.last() {
        keep.insert(last.id.clone());
    }

    // bucket key -> (highest seq seen, id of that version)
    // Tier tag in the key keeps hour/day/week buckets from ever colliding.
    let mut buckets: BTreeMap<(u8, u128), (u64, String)> = BTreeMap::new();

    for v in versions {
        // Named versions are always kept.
        if v.label.is_some() {
            keep.insert(v.id.clone());
            continue;
        }

        let ts = match v.timestamp_ms {
            Some(t) => t,
            // No timestamp → cannot bucket; always keep.
            None => {
                keep.insert(v.id.clone());
                continue;
            }
        };

        let age = now_ms.saturating_sub(ts);
        let key = if age < policy.keep_all_below {
            // keep-all tier: unique bucket per version (seq used as discriminant)
            (0u8, u128::from(v.seq))
        } else if age < policy.hourly_below {
            (1u8, age / MS_PER_HOUR)
        } else if age < policy.daily_below {
            (2u8, age / MS_PER_DAY)
        } else {
            (3u8, age / MS_PER_WEEK)
        };

        // Keep the highest-seq version in each bucket.
        buckets
            .entry(key)
            .and_modify(|e| {
                if v.seq > e.0 {
                    *e = (v.seq, v.id.clone());
                }
            })
            .or_insert((v.seq, v.id.clone()));
    }

    for (_, (_, id)) in buckets {
        keep.insert(id);
    }
    keep
}

// ── Apply (rewrite manifest, re-link parents) ─────────────────────────────────

/// Outcome of an [`apply_thinning`] pass.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinReport {
    pub kept: usize,
    pub dropped: usize,
}

/// Apply `policy` to `doc_id`'s versions: drop thinned-out versions and rewrite
/// `versions.jsonl`, re-linking each kept version's parent to the previous kept
/// version.
///
/// Returns how many were kept/dropped. Orphaned objects are reclaimed by a later
/// GC pass — not here.
pub fn apply_thinning(
    fs: &impl Fs,
    paths: &StorePaths,
    clock: &impl Clock,
    doc_id: &str,
    policy: &RetentionPolicy,
) -> Result<ThinReport, SessionError> {
    let vpath = paths.versions_file(doc_id);
    let versions = read_records(fs, &vpath)?;
    if versions.is_empty() {
        return Ok(ThinReport {
            kept: 0,
            dropped: 0,
        });
    }

    let now_ms = clock
        .now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let keep = thin_versions(&versions, now_ms, policy);

    let mut kept_records: Vec<HistoryRecord> = Vec::new();
    let mut prev_kept_id: Option<String> = None;
    let mut dropped = 0usize;

    for v in &versions {
        if keep.contains(&v.id) {
            let mut r = v.clone();
            r.parent = prev_kept_id.clone();
            prev_kept_id = Some(v.id.clone());
            kept_records.push(r);
        } else {
            dropped += 1;
        }
    }

    // Rewrite versions.jsonl with exactly the kept records (overwrite, not append).
    let mut bytes: Vec<u8> = Vec::new();
    for r in &kept_records {
        let mut line = serde_json::to_vec(r)
            .map_err(|e| SessionError::new(format!("serialize version: {e}")))?;
        line.push(b'\n');
        bytes.extend_from_slice(&line);
    }

    if let Some(parent) = vpath.parent() {
        fs.create_dir_all(parent)?;
    }
    fs.write(&vpath, &bytes)?;

    Ok(ThinReport {
        kept: kept_records.len(),
        dropped,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;
    use crate::adapter::{FakeClock, MemFs};
    use crate::layout::StorePaths;
    use crate::manifest::append_record;

    // ── Helper ────────────────────────────────────────────────────────────────

    fn rec(seq: u64, parent: Option<&str>, ts_ms: u128, label: Option<&str>) -> HistoryRecord {
        let mut r = HistoryRecord::new(
            format!("v{seq}"),
            seq,
            parent.map(str::to_owned),
            format!("hash{seq}"),
        );
        r.timestamp_ms = Some(ts_ms);
        r.label = label.map(str::to_owned);
        r
    }

    fn policy() -> RetentionPolicy {
        RetentionPolicy::default()
    }

    // ── Pure thin_versions tests ──────────────────────────────────────────────

    #[test]
    fn empty_keeps_nothing() {
        let kept = thin_versions(&[], 1_000_000, &policy());
        assert!(kept.is_empty());
    }

    #[test]
    fn all_recent_kept() {
        // now = 10 hours in ms; all three versions within 1h of now → all kept
        let now_ms: u128 = 10 * MS_PER_HOUR;
        let versions = vec![
            rec(0, None, now_ms - 10_000, None),            // 10s ago
            rec(1, Some("v0"), now_ms - 30_000, None),      // 30s ago
            rec(2, Some("v1"), now_ms - 59 * 60_000, None), // 59min ago
        ];
        let kept = thin_versions(&versions, now_ms, &policy());
        assert!(kept.contains("v0"), "v0 should be kept");
        assert!(kept.contains("v1"), "v1 should be kept");
        assert!(kept.contains("v2"), "v2 should be kept (latest + recent)");
    }

    #[test]
    fn latest_always_kept() {
        // A single very old version must be kept because it is the latest.
        let now_ms: u128 = 100 * MS_PER_DAY;
        let versions = vec![rec(0, None, 0, None)];
        let kept = thin_versions(&versions, now_ms, &policy());
        assert!(kept.contains("v0"), "latest must always be kept");
    }

    #[test]
    fn named_always_kept() {
        // Two versions in the same weekly bucket: one named, one not.
        // The named one must be kept; the unnamed one may be thinned.
        let now_ms: u128 = 100 * MS_PER_DAY;
        // Both in week 0 relative to our weekly bucket computation.
        let base_ts = now_ms - 60 * MS_PER_DAY; // 60 days ago → weekly bucket
        let versions = vec![
            rec(0, None, base_ts, Some("release-1.0")), // named
            rec(1, Some("v0"), base_ts + 1_000, None),  // unnamed, same bucket
            rec(2, Some("v1"), now_ms - 1_000, None),   // latest
        ];
        let kept = thin_versions(&versions, now_ms, &policy());
        assert!(kept.contains("v0"), "named version must survive thinning");
        // v2 is the latest and must be kept
        assert!(kept.contains("v2"), "latest must be kept");
        // v1 is an old unnamed in the same weekly bucket as v0; it may be dropped
        // (bucket keeps highest seq, which could be v1, but v0 is named so it's
        // kept independently — v1 may or may not survive depending on buckets)
        // The important invariant is: named v0 is kept regardless.
    }

    #[test]
    fn hourly_bucket_keeps_newest() {
        // Two versions in the same hourly bucket; only the higher-seq one is kept
        // (plus the latest).
        let now_ms: u128 = 10 * MS_PER_HOUR;
        // Age ~2h and ~2h+5min → both fall in hour-bucket 2 (age/MS_PER_HOUR == 2)
        let v0_ts = now_ms - 2 * MS_PER_HOUR - 5 * 60_000; // older (lower seq)
        let v1_ts = now_ms - 2 * MS_PER_HOUR; // newer (higher seq)
        let v2_ts = now_ms - 60_000; // latest (recent, kept)
        let versions = vec![
            rec(0, None, v0_ts, None),
            rec(1, Some("v0"), v1_ts, None),
            rec(2, Some("v1"), v2_ts, None),
        ];
        let kept = thin_versions(&versions, now_ms, &policy());
        // v1 has higher seq in bucket hour-2, so it wins; v0 should be dropped
        assert!(
            !kept.contains("v0"),
            "lower-seq in same hourly bucket should be dropped"
        );
        assert!(
            kept.contains("v1"),
            "higher-seq in hourly bucket must be kept"
        );
        assert!(kept.contains("v2"), "latest must be kept");
    }

    #[test]
    fn daily_and_weekly_buckets() {
        // Construct versions spanning multiple days and weeks; assert one per bucket.
        let now_ms: u128 = 60 * MS_PER_DAY; // "now" = 60 days since epoch

        // Day-1 ago: two versions in the same daily bucket → only higher seq kept.
        let day1_early = now_ms - MS_PER_DAY - 3 * MS_PER_HOUR;
        let day1_late = now_ms - MS_PER_DAY - MS_PER_HOUR;

        // Day-2 ago: one version.
        let day2 = now_ms - 2 * MS_PER_DAY - MS_PER_HOUR;

        // 35 days ago: two versions in the same weekly bucket → only higher seq kept.
        let week5_early = now_ms - 35 * MS_PER_DAY - 2 * MS_PER_HOUR;
        let week5_late = now_ms - 35 * MS_PER_DAY - MS_PER_HOUR;

        let versions = vec![
            rec(0, None, week5_early, None),
            rec(1, Some("v0"), week5_late, None),
            rec(2, Some("v1"), day2, None),
            rec(3, Some("v2"), day1_early, None),
            rec(4, Some("v3"), day1_late, None),
            rec(5, Some("v4"), now_ms - 30_000, None), // latest (recent)
        ];
        let kept = thin_versions(&versions, now_ms, &policy());

        // Latest always kept.
        assert!(kept.contains("v5"));

        // Daily bucket for day-1: v4 (higher seq) kept, v3 dropped.
        assert!(kept.contains("v4"), "v4 is higher-seq in day-1 bucket");
        assert!(!kept.contains("v3"), "v3 is lower-seq in day-1 bucket");

        // Daily bucket for day-2: only v2.
        assert!(kept.contains("v2"), "sole version in day-2 bucket");

        // Weekly bucket for week containing 35d ago: v1 (higher seq) kept, v0 dropped.
        assert!(kept.contains("v1"), "v1 is higher-seq in weekly bucket");
        assert!(!kept.contains("v0"), "v0 is lower-seq in weekly bucket");
    }

    #[test]
    fn missing_timestamp_kept() {
        let now_ms: u128 = 10 * MS_PER_HOUR;
        let mut no_ts = HistoryRecord::new("no_ts", 0, None, "hashX");
        no_ts.timestamp_ms = None; // explicitly absent
        let versions = vec![no_ts];
        let kept = thin_versions(&versions, now_ms, &policy());
        assert!(
            kept.contains("no_ts"),
            "version with no timestamp must always be kept"
        );
    }

    // ── apply_thinning tests ──────────────────────────────────────────────────

    fn seed_versions(fs: &MemFs, paths: &StorePaths, doc_id: &str, records: &[HistoryRecord]) {
        for r in records {
            append_record(fs, &paths.versions_file(doc_id), r).unwrap();
        }
    }

    #[test]
    fn apply_drops_and_relinks() {
        let fs = MemFs::new();
        let paths = StorePaths::new("/store");
        let doc_id = "doc1";

        // now = 5 hours in ms
        let now_ms: u128 = 5 * MS_PER_HOUR;
        let clock = FakeClock(UNIX_EPOCH + Duration::from_millis(now_ms as u64));

        // v0: 3h ago (hourly bucket 3) — will be the only one in that bucket
        // v1: 2h 30min ago (hourly bucket 2) — lower seq in bucket 2
        // v2: 2h ago (hourly bucket 2) — higher seq in bucket 2; v1 should be dropped
        // v3: 30s ago (< 1h — recent, always kept); also the latest
        let v0_ts = now_ms - 3 * MS_PER_HOUR;
        let v1_ts = now_ms - 2 * MS_PER_HOUR - 30 * 60_000;
        let v2_ts = now_ms - 2 * MS_PER_HOUR;
        let v3_ts = now_ms - 30_000;

        let records = vec![
            rec(0, None, v0_ts, None),
            rec(1, Some("v0"), v1_ts, None),
            rec(2, Some("v1"), v2_ts, None),
            rec(3, Some("v2"), v3_ts, None),
        ];
        seed_versions(&fs, &paths, doc_id, &records);

        let report = apply_thinning(&fs, &paths, &clock, doc_id, &policy()).unwrap();

        // v1 (lower seq in bucket 2) is dropped; v0, v2, v3 are kept.
        assert_eq!(report.dropped, 1, "v1 should be dropped");
        assert_eq!(report.kept, 3, "v0, v2, v3 should be kept");

        // Read back and verify parent chain.
        let kept_back = read_records(&fs, &paths.versions_file(doc_id)).unwrap();
        assert_eq!(kept_back.len(), 3);

        // Order must be preserved: v0, v2, v3
        assert_eq!(kept_back[0].id, "v0");
        assert_eq!(kept_back[1].id, "v2");
        assert_eq!(kept_back[2].id, "v3");

        // Parent chain: first kept has None, then each points to previous kept id.
        assert_eq!(
            kept_back[0].parent, None,
            "first kept record must have parent None"
        );
        assert_eq!(
            kept_back[1].parent,
            Some("v0".to_string()),
            "v2 must re-link to v0"
        );
        assert_eq!(
            kept_back[2].parent,
            Some("v2".to_string()),
            "v3 must re-link to v2"
        );

        // Dropped id is absent.
        assert!(
            kept_back.iter().all(|r| r.id != "v1"),
            "v1 must not appear in rewritten manifest"
        );
    }

    #[test]
    fn apply_empty_is_noop() {
        let fs = MemFs::new();
        let paths = StorePaths::new("/store");
        let doc_id = "empty_doc";
        let clock = FakeClock(UNIX_EPOCH + Duration::from_secs(1_000_000));

        let report = apply_thinning(&fs, &paths, &clock, doc_id, &policy()).unwrap();
        assert_eq!(
            report,
            ThinReport {
                kept: 0,
                dropped: 0
            }
        );
    }

    #[test]
    fn apply_preserves_named() {
        let fs = MemFs::new();
        let paths = StorePaths::new("/store");
        let doc_id = "doc2";

        // now = 90 days in ms
        let now_ms: u128 = 90 * MS_PER_DAY;
        let clock = FakeClock(UNIX_EPOCH + Duration::from_millis(now_ms as u64));

        // Named version 60 days ago (weekly bucket).
        // Unnamed neighbour in the same weekly bucket with higher seq.
        // Latest version (recent).
        let named_ts = now_ms - 60 * MS_PER_DAY;
        let unnamed_ts = named_ts + MS_PER_HOUR; // slightly newer, same weekly bucket
        let latest_ts = now_ms - 5_000;

        let records = vec![
            rec(0, None, named_ts, Some("v1.0")), // named — must survive
            rec(1, Some("v0"), unnamed_ts, None), // unnamed, same bucket; may be dropped
            rec(2, Some("v1"), latest_ts, None),  // latest
        ];
        seed_versions(&fs, &paths, doc_id, &records);

        let report = apply_thinning(&fs, &paths, &clock, doc_id, &policy()).unwrap();

        let kept_back = read_records(&fs, &paths.versions_file(doc_id)).unwrap();
        let ids: Vec<&str> = kept_back.iter().map(|r| r.id.as_str()).collect();

        assert!(ids.contains(&"v0"), "named version v0 must be preserved");
        assert!(ids.contains(&"v2"), "latest v2 must be preserved");
        assert_eq!(report.kept + report.dropped, 3);
    }
}
