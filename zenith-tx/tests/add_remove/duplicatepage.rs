use super::*;

// ── DuplicatePage tests ───────────────────────────────────────────────────

/// Duplicate a 1-page doc with 2 nodes: doc now has 2 pages, the copy has the
/// new page id, the copy's nodes carry the suffix, and the source is unchanged.
#[test]
fn duplicate_page_accepted() {
    let doc = parse(DUP_PAGE_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicatePage {
            page: "pg1".to_owned(),
            new_id: "pg2".to_owned(),
            id_suffix: ".v2".to_owned(),
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
    assert_eq!(result.affected_node_ids, vec!["pg2".to_owned()]);

    // Both page ids present; the new page appears after the original.
    assert!(result.source_after.contains("page id=\"pg1\""));
    assert!(result.source_after.contains("page id=\"pg2\""));
    let pos_pg1 = result
        .source_after
        .find("page id=\"pg1\"")
        .expect("pg1 in source_after");
    let pos_pg2 = result
        .source_after
        .find("page id=\"pg2\"")
        .expect("pg2 in source_after");
    assert!(pos_pg1 < pos_pg2, "new page should follow the source page");

    // The copy's node ids are <orig><suffix>.
    assert!(
        result.source_after.contains("id=\"r1.v2\""),
        "clone node r1.v2 must be present; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("id=\"r2.v2\""),
        "clone node r2.v2 must be present; got:\n{}",
        result.source_after
    );

    // (b) The source page's nodes are NOT renamed — original ids still appear,
    // and they appear exactly once each (only the source carries them).
    assert_eq!(
        result.source_after.matches("id=\"r1\"").count(),
        1,
        "source node r1 must be unchanged and unique; got:\n{}",
        result.source_after
    );
    assert_eq!(
        result.source_after.matches("id=\"r2\"").count(),
        1,
        "source node r2 must be unchanged and unique; got:\n{}",
        result.source_after
    );

    // source_before has only one page.
    assert_eq!(
        result.source_before.matches("page id=").count(),
        1,
        "source_before should have only one page"
    );
}

/// Duplicate with an empty id_suffix → cloned node ids collide with the
/// originals → post-validation rejects via id.duplicate.
#[test]
fn duplicate_page_empty_suffix_rejected() {
    let doc = parse(DUP_PAGE_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicatePage {
            page: "pg1".to_owned(),
            new_id: "pg2".to_owned(),
            id_suffix: String::new(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Rejected,
        "empty suffix must be rejected; diagnostics: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == "id.duplicate"),
        "expected id.duplicate diagnostic; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

/// Duplicate an unknown source page → tx.unknown_node, transaction rejected.
#[test]
fn duplicate_page_unknown_page_rejected() {
    let doc = parse(DUP_PAGE_DOC);
    let tx = Transaction {
        ops: vec![Op::DuplicatePage {
            page: "does_not_exist".to_owned(),
            new_id: "pg2".to_owned(),
            id_suffix: ".v2".to_owned(),
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
