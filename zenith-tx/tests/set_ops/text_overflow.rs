use super::*;

// ── SetTextOverflow tests ─────────────────────────────────────────────────

#[test]
fn set_text_overflow_on_text_accepted() {
    let doc = parse(TEXT_CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextOverflow {
            node_id: "body".to_owned(),
            overflow: "visible".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");
    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["body".to_owned()]);
    assert!(
        result.source_after.contains("overflow=\"visible\""),
        "source_after should set overflow=\"visible\": {}",
        result.source_after
    );
}

#[test]
fn set_text_overflow_on_code_accepted() {
    let doc = parse(TEXT_CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextOverflow {
            node_id: "snip".to_owned(),
            overflow: "clip".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");
    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["snip".to_owned()]);
    assert!(
        result.source_after.contains("overflow=\"clip\""),
        "source_after should set overflow=\"clip\": {}",
        result.source_after
    );
}

#[test]
fn set_text_overflow_invalid_value_rejected() {
    let doc = parse(TEXT_CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextOverflow {
            node_id: "body".to_owned(),
            overflow: "wrap".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");
    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_value" && d.message.contains("wrap")),
        "expected tx.invalid_value naming \"wrap\"; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn set_text_overflow_wrong_node_type_rejected() {
    let doc = parse(THREE_RECTS_DOC); // rects, no overflow field
    let tx = Transaction {
        ops: vec![Op::SetTextOverflow {
            node_id: "r1".to_owned(),
            overflow: "visible".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction must not error");
    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.wrong_node_type"),
        "expected tx.wrong_node_type; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn set_text_overflow_missing_node_rejected() {
    let doc = parse(TEXT_CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetTextOverflow {
            node_id: "nope".to_owned(),
            overflow: "fit".to_owned(),
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
