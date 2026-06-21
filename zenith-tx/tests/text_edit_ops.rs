mod common;
use common::*;
use zenith_tx::{Op, Permissions, Transaction, TxStatus, run_transaction};

// ── Fixtures ──────────────────────────────────────────────────────────────────

/// Doc with:
/// - "txt.ltr"  — text node, two spans (second has italic + font_weight for
///   formatting-preservation checks), contains "V0"
/// - "txt.locked" — locked text node, also contains "V0"
/// - "rect1"    — rect node (for wrong-node-type tests)
const TEXT_EDIT_DOC: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" {
    token id="font.bold" type="fontWeight" value=700
  }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)800 h=(px)600 {
      text id="txt.ltr" x=(px)10 y=(px)10 w=(px)300 h=(px)40 {
        span "Hello V0 World"
        span font-weight=(token)"font.bold" italic=#true "italic V0 span"
      }
      text id="txt.locked" locked=#true x=(px)10 y=(px)60 w=(px)300 h=(px)40 {
        span "Locked V0 text"
      }
      rect id="rect1" x=(px)0 y=(px)100 w=(px)50 h=(px)50
    }
  }
}"##;

/// Doc with a text node whose single span contains "V0 and V0" (two occurrences).
const MULTI_OCCUR_DOC: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      text id="body" x=(px)10 y=(px)10 w=(px)200 h=(px)40 {
        span "V0 and V0"
      }
    }
  }
}"##;

/// Doc with no text containing the search target (for noop tests).
const NO_MATCH_DOC: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      text id="body" x=(px)10 y=(px)10 w=(px)200 h=(px)40 {
        span "Hello World"
      }
    }
  }
}"##;

// ─────────────────────────────────────────────────────────────────────────────
// SetTextDirection tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn set_text_direction_rtl_accepted() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextDirection {
            node: "txt.ltr".to_owned(),
            direction: "rtl".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["txt.ltr".to_owned()]);
    assert!(
        result.source_after.contains("direction=\"rtl\""),
        "source_after must contain direction=\"rtl\"; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_before, result.source_after);
}

#[test]
fn set_text_direction_ltr_accepted() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextDirection {
            node: "txt.ltr".to_owned(),
            direction: "ltr".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["txt.ltr".to_owned()]);
}

#[test]
fn set_text_direction_invalid_value_rejected() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextDirection {
            node: "txt.ltr".to_owned(),
            direction: "sideways".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_value" && d.message.contains("sideways")),
        "expected tx.invalid_value naming \"sideways\"; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn set_text_direction_wrong_node_type_rejected() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextDirection {
            node: "rect1".to_owned(),
            direction: "rtl".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.wrong_node_type" && d.message.contains("rect")),
        "expected tx.wrong_node_type naming \"rect\"; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn set_text_direction_unknown_node_rejected() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextDirection {
            node: "does_not_exist".to_owned(),
            direction: "rtl".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// FindReplaceText tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn find_replace_text_scoped_accepted_and_span_formatting_preserved() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "V0".to_owned(),
            replace: "v0".to_owned(),
            node: Some("txt.ltr".to_owned()),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["txt.ltr".to_owned()]);

    // Both spans of the scoped node have "V0" replaced with "v0".
    assert!(
        result.source_after.contains("Hello v0 World"),
        "scoped node's first span must become \"Hello v0 World\"; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("italic v0 span"),
        "scoped node's bold span must become \"italic v0 span\"; got:\n{}",
        result.source_after
    );
    // The out-of-scope locked node is untouched — it still holds the original "V0".
    assert!(
        result.source_after.contains("Locked V0 text"),
        "out-of-scope node must retain original \"V0\"; got:\n{}",
        result.source_after
    );

    // Span formatting must be preserved: the bold+italic span still has font-weight and italic.
    assert!(
        result
            .source_after
            .contains("font-weight=(token)\"font.bold\""),
        "source_after must preserve font-weight=(token)\"font.bold\"; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("italic=#true"),
        "source_after must preserve italic=#true; got:\n{}",
        result.source_after
    );
}

#[test]
fn find_replace_text_doc_wide_skips_locked_node() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "V0".to_owned(),
            replace: "v0".to_owned(),
            node: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    // AcceptedWithWarnings because txt.locked is skipped with a warning.
    assert_eq!(
        result.status,
        TxStatus::AcceptedWithWarnings,
        "expected AcceptedWithWarnings; got: {:?} with diagnostics: {:?}",
        result.status,
        result.diagnostics
    );

    // tx.locked_skipped warning must name the locked node.
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| { d.code == "tx.locked_skipped" && d.message.contains("txt.locked") }),
        "expected tx.locked_skipped naming \"txt.locked\"; got: {:?}",
        result.diagnostics
    );

    // txt.ltr was modified; txt.locked was not.
    assert!(
        result.affected_node_ids.contains(&"txt.ltr".to_owned()),
        "txt.ltr must be in affected; got: {:?}",
        result.affected_node_ids
    );
    assert!(
        !result.affected_node_ids.contains(&"txt.locked".to_owned()),
        "txt.locked must NOT be in affected; got: {:?}",
        result.affected_node_ids
    );

    // The locked node's text must be unchanged.
    assert!(
        result.source_after.contains("Locked V0 text"),
        "locked node text must stay unchanged; got:\n{}",
        result.source_after
    );
}

#[test]
fn find_replace_text_doc_wide_with_allow_locked_still_skips_locked() {
    // Doc-wide mode always skips locked nodes (self-managed), regardless of
    // allow_locked, because op_lock_targets returns empty for doc-wide.
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "V0".to_owned(),
            replace: "v0".to_owned(),
            node: None,
        }],
        permissions: Permissions {
            allow_locked: true,
            allow_raw_visual_literals: false,
        },
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    // The locked node must still be unchanged (the doc-wide path skips it).
    assert!(
        result.source_after.contains("Locked V0 text"),
        "locked node text must remain \"Locked V0 text\" even with allow_locked; got:\n{}",
        result.source_after
    );
}

#[test]
fn find_replace_text_empty_find_rejected() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: String::new(),
            replace: "x".to_owned(),
            node: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_value" && d.message.contains("non-empty")),
        "expected tx.invalid_value about non-empty; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn find_replace_text_no_match_emits_noop() {
    let doc = parse(NO_MATCH_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "NOTPRESENT".to_owned(),
            replace: "x".to_owned(),
            node: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "noop advisory does not reject; got: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == "tx.noop"),
        "expected tx.noop advisory; got: {:?}",
        result.diagnostics
    );
    assert!(
        result.affected_node_ids.is_empty(),
        "no nodes should be affected; got: {:?}",
        result.affected_node_ids
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn find_replace_text_scoped_no_match_emits_noop() {
    let doc = parse(NO_MATCH_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "NOTPRESENT".to_owned(),
            replace: "x".to_owned(),
            node: Some("body".to_owned()),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "tx.noop"),
        "expected tx.noop advisory; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn find_replace_text_multi_occurrence_in_span() {
    let doc = parse(MULTI_OCCUR_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "V0".to_owned(),
            replace: "v0".to_owned(),
            node: Some("body".to_owned()),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["body".to_owned()]);

    // Both occurrences of "V0" should be replaced.
    assert!(
        result.source_after.contains("v0 and v0"),
        "both occurrences must be replaced; got:\n{}",
        result.source_after
    );
    assert!(
        !result.source_after.contains("V0"),
        "no \"V0\" should remain; got:\n{}",
        result.source_after
    );
}

#[test]
fn find_replace_text_scoped_unknown_node_rejected() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "V0".to_owned(),
            replace: "v0".to_owned(),
            node: Some("nope".to_owned()),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unknown_node"),
        "expected tx.unknown_node; got: {:?}",
        result.diagnostics
    );
}

#[test]
fn find_replace_text_scoped_wrong_node_type_rejected() {
    let doc = parse(TEXT_EDIT_DOC);
    let tx = Transaction {
        ops: vec![Op::FindReplaceText {
            find: "V0".to_owned(),
            replace: "v0".to_owned(),
            node: Some("rect1".to_owned()),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.wrong_node_type" && d.message.contains("rect")),
        "expected tx.wrong_node_type naming \"rect\"; got: {:?}",
        result.diagnostics
    );
}
