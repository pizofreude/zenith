mod common;
use common::*;
use zenith_tx::{Op, Permissions, Transaction, TxStatus, run_transaction};

/// Two-page doc: one candidate page (selected) with two rects, and one empty
/// export page. Used across all promote_candidate tests.
const PROMOTE_DOC: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#ff0000"
  }
  styles { }
  document id="doc1" title="T" {
    page id="pg.cand" w=(px)400 h=(px)300 candidate-status="selected" {
      rect id="r1" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
      rect id="r2" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
    page id="pg.export" w=(px)400 h=(px)300 {
    }
  }
}"##;

/// Same as PROMOTE_DOC but the candidate page has status "draft" (not selected).
const PROMOTE_DOC_DRAFT: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#ff0000"
  }
  styles { }
  document id="doc1" title="T" {
    page id="pg.cand" w=(px)400 h=(px)300 candidate-status="draft" {
      rect id="r1" x=(px)10 y=(px)20 w=(px)80 h=(px)60 fill=(token)"color.a"
    }
    page id="pg.export" w=(px)400 h=(px)300 {
    }
  }
}"##;

// ── promote_candidate: happy path ─────────────────────────────────────────────

/// Deep-copies children from the candidate page to the target page with
/// suffixed ids; source page is unchanged.
#[test]
fn promote_candidate_deep_copies_children_with_suffixed_ids() {
    let doc = parse(PROMOTE_DOC);
    let tx = Transaction {
        ops: vec![Op::PromoteCandidate {
            source_page: "pg.cand".to_owned(),
            target_page: "pg.export".to_owned(),
            id_suffix: ".p".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );

    // Target page now has children with suffixed ids.
    assert!(
        result.source_after.contains("id=\"r1.p\""),
        "r1.p must be present in source_after:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("id=\"r2.p\""),
        "r2.p must be present in source_after:\n{}",
        result.source_after
    );

    // Source page is unchanged — original ids still present, exactly once each.
    assert_eq!(
        result.source_after.matches("id=\"r1\"").count(),
        1,
        "source node r1 must be unchanged and unique:\n{}",
        result.source_after
    );
    assert_eq!(
        result.source_after.matches("id=\"r2\"").count(),
        1,
        "source node r2 must be unchanged and unique:\n{}",
        result.source_after
    );

    // affected_node_ids contains the target page id and the new child ids.
    assert!(
        result.affected_node_ids.contains(&"pg.export".to_owned()),
        "pg.export must be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
    assert!(
        result.affected_node_ids.contains(&"r1.p".to_owned()),
        "r1.p must be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
    assert!(
        result.affected_node_ids.contains(&"r2.p".to_owned()),
        "r2.p must be in affected_node_ids: {:?}",
        result.affected_node_ids
    );
}

/// After promotion, the target page carries workspace-role="export".
#[test]
fn promote_candidate_sets_target_export_role() {
    let doc = parse(PROMOTE_DOC);
    let tx = Transaction {
        ops: vec![Op::PromoteCandidate {
            source_page: "pg.cand".to_owned(),
            target_page: "pg.export".to_owned(),
            id_suffix: ".p".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );

    // The formatter emits workspace-role="export" on the pg.export page.
    // Find the pg.export page block and confirm workspace-role is present.
    assert!(
        result.source_after.contains("workspace-role=\"export\""),
        "target page must have workspace-role=\"export\" in source_after:\n{}",
        result.source_after
    );
}

// ── promote_candidate: error paths ───────────────────────────────────────────

/// Unknown source page id → tx.unknown_node, transaction rejected.
#[test]
fn promote_candidate_unknown_source_page_errors() {
    let doc = parse(PROMOTE_DOC);
    let tx = Transaction {
        ops: vec![Op::PromoteCandidate {
            source_page: "pg.nope".to_owned(),
            target_page: "pg.export".to_owned(),
            id_suffix: ".p".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "expected Rejected; diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

/// Unknown target page id → tx.unknown_node, transaction rejected.
#[test]
fn promote_candidate_unknown_target_page_errors() {
    let doc = parse(PROMOTE_DOC);
    let tx = Transaction {
        ops: vec![Op::PromoteCandidate {
            source_page: "pg.cand".to_owned(),
            target_page: "pg.nope".to_owned(),
            id_suffix: ".p".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "expected Rejected; diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

/// Source page without candidate-status="selected" → tx.candidate_not_selected,
/// transaction rejected.
#[test]
fn promote_candidate_requires_selected_status() {
    let doc = parse(PROMOTE_DOC_DRAFT);
    let tx = Transaction {
        ops: vec![Op::PromoteCandidate {
            source_page: "pg.cand".to_owned(),
            target_page: "pg.export".to_owned(),
            id_suffix: ".p".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "expected Rejected; diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.candidate_not_selected"),
        "expected tx.candidate_not_selected; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

/// source_page == target_page → tx.invalid_value, transaction rejected.
#[test]
fn promote_candidate_source_equals_target_errors() {
    let doc = parse(PROMOTE_DOC);
    let tx = Transaction {
        ops: vec![Op::PromoteCandidate {
            source_page: "pg.cand".to_owned(),
            target_page: "pg.cand".to_owned(),
            id_suffix: ".p".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "expected Rejected; diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_value"),
        "expected tx.invalid_value; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}
