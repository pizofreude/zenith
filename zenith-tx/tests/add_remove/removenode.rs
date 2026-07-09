use super::*;

// ── RemoveNode tests ──────────────────────────────────────────────────────

#[test]
fn remove_node_top_level() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::RemoveNode {
            node: "a".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["a".to_owned()]);
    assert!(
        !result.source_after.contains("id=\"a\""),
        "node a must be gone from source_after; got:\n{}",
        result.source_after
    );
    assert!(result.source_after.contains("id=\"b\""), "b must remain");
}

#[test]
fn remove_node_nested_in_group() {
    let doc = parse(ADD_GROUP_DOC);
    let tx = Transaction {
        ops: vec![Op::RemoveNode {
            node: "g.a".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "{:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["g.a".to_owned()]);
    assert!(
        !result.source_after.contains("id=\"g.a\""),
        "nested node g.a must be gone; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("id=\"g.b\""),
        "g.b must remain"
    );
}

#[test]
fn remove_node_unknown_rejected() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::RemoveNode {
            node: "nope".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
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
