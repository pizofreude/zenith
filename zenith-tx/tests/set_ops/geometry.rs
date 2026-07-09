use super::*;

// ── SetGeometry tests ─────────────────────────────────────────────────────

#[test]
fn set_geometry_moves_rect() {
    let doc = parse(RECT_GEOM_DOC);
    let tx = Transaction {
        ops: vec![Op::SetGeometry {
            node: "rect".to_owned(),
            x: Some(50.0),
            y: None,
            w: Some(200.0),
            h: None,
            rotate: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["rect".to_owned()]);

    // Changed fields appear in source_after.
    assert!(
        result.source_after.contains("x=(px)50"),
        "source_after must contain x=(px)50; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("w=(px)200"),
        "source_after must contain w=(px)200; got:\n{}",
        result.source_after
    );
    // Untouched fields stay at their original values.
    assert!(
        result.source_after.contains("y=(px)0"),
        "source_after must retain y=(px)0; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("h=(px)100"),
        "source_after must retain h=(px)100; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_before, result.source_after);
}

#[test]
fn set_geometry_unsupported_on_line() {
    let doc = parse(LINE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetGeometry {
            node: "ln1".to_owned(),
            x: Some(10.0),
            y: None,
            w: None,
            h: None,
            rotate: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unsupported_property" && d.message.contains("line")),
        "expected tx.unsupported_property mentioning \"line\"; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn set_geometry_no_fields_is_noop() {
    let doc = parse(RECT_GEOM_DOC);
    let tx = Transaction {
        ops: vec![Op::SetGeometry {
            node: "rect".to_owned(),
            x: None,
            y: None,
            w: None,
            h: None,
            rotate: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    // All-None must produce Accepted (advisory is not an error/warning) with
    // no affected nodes and identical source.
    assert_eq!(result.status, TxStatus::Accepted);
    assert!(
        result.affected_node_ids.is_empty(),
        "affected must be empty for a noop; got: {:?}",
        result.affected_node_ids
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == "tx.noop"),
        "expected tx.noop advisory; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

// ── Code node tx tests ────────────────────────────────────────────────────

#[test]
fn set_visible_on_code_accepted() {
    let doc = parse(CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetVisible {
            node: "snip".to_owned(),
            visible: false,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["snip".to_owned()]);
    assert!(
        result.source_after.contains("visible=#false"),
        "source_after must contain visible=#false; got:\n{}",
        result.source_after
    );
    // Content blob must survive the edit untouched.
    assert!(result.source_after.contains("content \"fn main() {}\""));
}

#[test]
fn set_fill_on_code_accepted() {
    let doc = parse(CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetFill {
            node: "snip".to_owned(),
            fill: "color.b".to_owned(),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["snip".to_owned()]);
    assert!(
        result.source_after.contains("(token)\"color.b\""),
        "source_after must reference color.b; got:\n{}",
        result.source_after
    );
}

#[test]
fn set_geometry_supported_on_code() {
    let doc = parse(CODE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetGeometry {
            node: "snip".to_owned(),
            x: Some(10.0),
            y: None,
            w: None,
            h: None,
            rotate: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["snip".to_owned()]);
    assert!(
        result.source_after.contains("x=(px)10"),
        "source_after must contain x=(px)10; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_after, result.source_before);
}

#[test]
fn set_geometry_supported_on_text() {
    let doc = parse(TEXT_DOC);
    let tx = Transaction {
        ops: vec![Op::SetGeometry {
            node: "label".to_owned(),
            x: Some(-200.0),
            y: None,
            w: None,
            h: None,
            rotate: None,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["label".to_owned()]);
    assert!(
        result.source_after.contains("x=(px)-200"),
        "source_after must contain x=(px)-200; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_after, result.source_before);
}

// ── SetGeometry rotate tests ─────────────────────────────────────────────

#[test]
fn set_geometry_rotate_on_image_accepted() {
    let doc = parse(IMAGE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetGeometry {
            node: "pic".to_owned(),
            x: None,
            y: None,
            w: None,
            h: None,
            rotate: Some(45.0),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["pic".to_owned()]);
    assert!(
        result.source_after.contains("rotate=(deg)45"),
        "source_after must contain rotate=(deg)45; got:\n{}",
        result.source_after
    );
    assert_ne!(result.source_before, result.source_after);
}

#[test]
fn set_geometry_rotate_on_line_rejected() {
    let doc = parse(LINE_DOC);
    let tx = Transaction {
        ops: vec![Op::SetGeometry {
            node: "ln1".to_owned(),
            x: None,
            y: None,
            w: None,
            h: None,
            rotate: Some(30.0),
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unsupported_property" && d.message.contains("line")),
        "expected tx.unsupported_property mentioning \"line\"; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}
