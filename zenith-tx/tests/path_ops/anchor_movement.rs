use super::*;

#[test]
fn move_path_anchor_moves_point_and_handles_preserving_kind_and_adjacent_anchor() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" closed=#true {
        anchor x=(px)0 y=(px)0
        anchor x=(px)20 y=(px)10 kind="smooth" in-x=(px)15 in-y=(px)10 out-x=(px)25 out-y=(px)10
        anchor x=(px)40 y=(px)0 kind="corner"
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "path1".to_owned(),
            subpath_index: None,
            anchor_index: 1,
            dx: 3.0,
            dy: -4.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["path1".to_owned()]);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "y"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "x"), 23.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "y"), 6.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "in-x"), 18.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "in-y"), 6.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "out-x"), 28.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "out-y"), 6.0);
    assert_px_close(anchor_px_attr(&result.source_after, 2, "x"), 40.0);
    assert_px_close(anchor_px_attr(&result.source_after, 2, "y"), 0.0);
    assert!(
        anchor_line(&result.source_after, 1).contains("kind=\"smooth\""),
        "anchor kind should be preserved; got:\n{}",
        result.source_after
    );
    assert!(
        result.source_after.contains("closed=#true"),
        "closed flag should be preserved; got:\n{}",
        result.source_after
    );
}

#[test]
fn move_path_anchor_moves_anchor_with_no_handles() {
    let doc = parse(PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "path1".to_owned(),
            subpath_index: None,
            anchor_index: 1,
            dx: -5.0,
            dy: 7.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "y"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "x"), 95.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "y"), 7.0);
    assert!(
        !anchor_line(&result.source_after, 1).contains("in-x"),
        "move should not create handles; got:\n{}",
        result.source_after
    );
}

#[test]
fn move_path_anchor_targets_compound_subpath() {
    let doc = parse(COMPOUND_PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "compound".to_owned(),
            subpath_index: Some(1),
            anchor_index: 0,
            dx: 5.0,
            dy: -10.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["compound".to_owned()]);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 3, "x"), 30.0);
    assert_px_close(anchor_px_attr(&result.source_after, 3, "y"), 15.0);
}

#[test]
fn move_path_anchor_non_finite_delta_rejected_without_source_change() {
    let doc = parse(PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "path1".to_owned(),
            subpath_index: None,
            anchor_index: 0,
            dx: f64::INFINITY,
            dy: 0.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_geometry" && d.message.contains("finite")),
        "expected tx.invalid_geometry for non-finite delta; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn move_path_anchor_out_of_range_rejected_without_source_change() {
    let doc = parse(PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "path1".to_owned(),
            subpath_index: None,
            anchor_index: 2,
            dx: 1.0,
            dy: 2.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.out_of_range" && d.message.contains("anchor_index")),
        "expected tx.out_of_range mentioning anchor_index; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn move_path_anchor_non_px_target_anchor_rejected() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)0 y=(px)0
        anchor x=(pt)20 y=(px)0
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "path1".to_owned(),
            subpath_index: None,
            anchor_index: 1,
            dx: 1.0,
            dy: 2.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result.diagnostics.iter().any(|d| {
            d.code == "tx.invalid_path_anchor" && d.message.contains("must be a px value")
        }),
        "expected tx.invalid_path_anchor for non-px target anchor; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn move_path_anchor_incomplete_target_handle_rejected() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)0 y=(px)0 out-x=(px)10
        anchor x=(px)20 y=(px)0
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "path1".to_owned(),
            subpath_index: None,
            anchor_index: 0,
            dx: 1.0,
            dy: 2.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.invalid_path_anchor"),
        "expected tx.invalid_path_anchor; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn move_path_anchor_locked_path_rejected() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" locked=#true {
        anchor x=(px)0 y=(px)0
        anchor x=(px)20 y=(px)0
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "path1".to_owned(),
            subpath_index: None,
            anchor_index: 0,
            dx: 1.0,
            dy: 2.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "node.locked"),
        "expected node.locked; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn move_path_anchor_unsupported_on_rect() {
    let doc = parse(RECT_GEOM_DOC);
    let tx = Transaction {
        ops: vec![Op::MovePathAnchor {
            node: "rect".to_owned(),
            subpath_index: None,
            anchor_index: 0,
            dx: 1.0,
            dy: 2.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unsupported_property" && d.message.contains("rect")),
        "expected tx.unsupported_property mentioning rect; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}
