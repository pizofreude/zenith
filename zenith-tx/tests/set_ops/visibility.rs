use super::*;

// ── SetVisible tests ──────────────────────────────────────────────────────

#[test]
fn set_visible_hides_node() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetVisible {
            node: "a".to_owned(),
            visible: false,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["a".to_owned()]);
    assert!(
        result.source_after.contains("visible=#false"),
        "source_after must contain visible=#false; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_before, result.source_after);
}

#[test]
fn set_visible_on_nested_node() {
    let doc = parse(NESTED_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetVisible {
            node: "inner".to_owned(),
            visible: false,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["inner".to_owned()]);
    assert!(
        result.source_after.contains("visible=#false"),
        "source_after must contain visible=#false for nested node; got:\n{}",
        result.source_after
    );
}

// ── SetLocked tests ───────────────────────────────────────────────────────

#[test]
fn set_locked_sets_lock() {
    let doc = parse(TWO_RECT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetLocked {
            node: "b".to_owned(),
            locked: true,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["b".to_owned()]);
    assert!(
        result.source_after.contains("locked=#true"),
        "source_after must contain locked=#true; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_before, result.source_after);
}

// ── Unknown node targeting ────────────────────────────────────────────────

// `UnknownNode` has no `id` field, so `node_id_of` returns `None` for it.
// `subtree_contains` will never match an unknown node by id, and
// `find_node_any_mut` returns `None` → tx.unknown_node.
// We verify this by targeting a non-existent id that would match an unknown
// node if it had an id; since it doesn't, we just get tx.unknown_node.
#[test]
fn set_visible_on_nonexistent_id_is_unknown_node() {
    // Using TEXT_DOC — there is no node with id "does_not_exist".
    // The important thing: we get tx.unknown_node, not a panic or
    // tx.unsupported_property.
    let doc = parse(TEXT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetVisible {
            node: "does_not_exist".to_owned(),
            visible: false,
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
}
