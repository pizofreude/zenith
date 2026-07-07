use super::*;

#[test]
fn make_path_symmetric_inserts_transformed_sibling_paths() {
    let doc = parse(TRANSFORM_PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MakePathSymmetric {
            node: "path1".to_owned(),
            id_prefix: "path1.sym.".to_owned(),
            count: 4,
            cx: 0.0,
            cy: 0.0,
            start_angle_degrees: 0.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(
        result.affected_node_ids,
        vec![
            "path1.sym.1".to_owned(),
            "path1.sym.2".to_owned(),
            "path1.sym.3".to_owned()
        ]
    );
    assert!(
        result
            .source_after
            .contains("path id=\"path1\" closed=#true")
    );
    assert!(
        result
            .source_after
            .contains("path id=\"path1.sym.1\" closed=#true")
    );
    assert!(
        result
            .source_after
            .contains("path id=\"path1.sym.2\" closed=#true")
    );
    assert!(
        result
            .source_after
            .contains("path id=\"path1.sym.3\" closed=#true")
    );
    assert_px_close(anchor_px_attr(&result.source_after, 3, "x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 3, "y"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 4, "x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 4, "y"), 20.0);
    assert_px_close(anchor_px_attr(&result.source_after, 4, "in-x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 4, "in-y"), 10.0);
    assert!(anchor_line(&result.source_after, 4).contains("kind=\"smooth\""));
}

#[test]
fn make_path_symmetric_rejects_counts_below_two() {
    let doc = parse(TRANSFORM_PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MakePathSymmetric {
            node: "path1".to_owned(),
            id_prefix: "path1.sym.".to_owned(),
            count: 1,
            cx: 0.0,
            cy: 0.0,
            start_angle_degrees: 0.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "tx.invalid_geometry"),
        "expected tx.invalid_geometry; got {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn make_path_symmetric_rejects_unsupported_source_nodes() {
    let doc = parse(RECT_GEOM_DOC);
    let tx = Transaction {
        ops: vec![Op::MakePathSymmetric {
            node: "rect".to_owned(),
            id_prefix: "rect.sym.".to_owned(),
            count: 4,
            cx: 0.0,
            cy: 0.0,
            start_angle_degrees: 0.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "tx.unsupported_property"),
        "expected tx.unsupported_property; got {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}
