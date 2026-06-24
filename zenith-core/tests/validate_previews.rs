//! Integration tests: previews block validation.
//!
//! Covers both previews-check diagnostics:
//!   - `preview.unknown_candidate`
//!   - `preview.invalid_critique_severity`
//!
//! Plus a clean-doc regression guard (fully-valid block → no preview.* codes).

mod common;

use common::*;

fn parse_and_validate(src: &str) -> ValidationReport {
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    validate(&doc)
}

// ── Clean previews block → no preview.* diagnostics ──────────────────────────

/// A fully-valid `previews` block must produce no `preview.*` diagnostics.
/// Candidate references a real page; critiques use valid severities.
#[test]
fn valid_previews_block_is_clean() {
    let src = r##"zenith version=1 {
  project id="proj.pv.clean" name="Clean"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  previews {
    preview candidate="page.main" source-hash="abc123" {
      critique severity="error" code="preview.bleed" message="Content bleeds"
      critique severity="warning" code="preview.contrast" message="Low contrast"
      critique severity="advisory" code="preview.spacing" message="Tight spacing"
    }
  }
  document id="doc.pv.clean" title="Clean" {
    page id="page.main" w=(px)1280 h=(px)720 {
    }
  }
}
"##;
    let report = parse_and_validate(src);
    let preview_codes: Vec<&str> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("preview."))
        .map(|d| d.code.as_str())
        .collect();
    assert!(
        preview_codes.is_empty(),
        "clean previews block must produce no preview.* diagnostics; got {:?}",
        preview_codes
    );
}

// ── preview.unknown_candidate ─────────────────────────────────────────────────

/// A preview whose `candidate` names a page id not present in the document →
/// `preview.unknown_candidate` (advisory).
#[test]
fn unknown_candidate_is_advisory() {
    let src = r##"zenith version=1 {
  project id="proj.pv.unk" name="UNK"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  previews {
    preview candidate="page.deleted"
  }
  document id="doc.pv.unk" title="UNK" {
    page id="page.main" w=(px)1280 h=(px)720 {
    }
  }
}
"##;
    let report = parse_and_validate(src);
    assert!(
        has_code(&report, "preview.unknown_candidate"),
        "unknown candidate must produce preview.unknown_candidate; got {:?}",
        codes(&report)
    );
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "preview.unknown_candidate")
        .unwrap();
    assert_eq!(
        diag.severity,
        Severity::Advisory,
        "unknown_candidate must be Advisory severity"
    );
}

// ── preview.invalid_critique_severity ────────────────────────────────────────

/// A critique with an unrecognized severity →
/// `preview.invalid_critique_severity` (warning).
#[test]
fn invalid_critique_severity_is_warning() {
    let src = r##"zenith version=1 {
  project id="proj.pv.sev" name="SEV"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  previews {
    preview candidate="page.main" {
      critique severity="warn" code="preview.x" message="bad severity"
    }
  }
  document id="doc.pv.sev" title="SEV" {
    page id="page.main" w=(px)1280 h=(px)720 {
    }
  }
}
"##;
    let report = parse_and_validate(src);
    assert!(
        has_code(&report, "preview.invalid_critique_severity"),
        "invalid critique severity must produce preview.invalid_critique_severity; got {:?}",
        codes(&report)
    );
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "preview.invalid_critique_severity")
        .unwrap();
    assert_eq!(
        diag.severity,
        Severity::Warning,
        "invalid_critique_severity must be Warning severity"
    );
}
