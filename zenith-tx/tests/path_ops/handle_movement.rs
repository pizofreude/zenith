use super::*;
use zenith_tx::op::OpPathHandle;

#[test]
fn move_path_handle_corner_and_none_move_only_selected_handle() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)10 y=(px)10 kind="corner" in-x=(px)0 in-y=(px)10 out-x=(px)20 out-y=(px)10
        anchor x=(px)100 y=(px)100 in-x=(px)90 in-y=(px)100 out-x=(px)110 out-y=(px)100
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![
            Op::MovePathHandle {
                node: "path1".to_owned(),
                anchor_index: 0,
                handle: OpPathHandle::Out,
                dx: 3.0,
                dy: -4.0,
            },
            Op::MovePathHandle {
                node: "path1".to_owned(),
                anchor_index: 1,
                handle: OpPathHandle::In,
                dx: -5.0,
                dy: 6.0,
            },
        ],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["path1".to_owned()]);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "x"), 10.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "y"), 10.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "in-x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "in-y"), 10.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "out-x"), 23.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "out-y"), 6.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "x"), 100.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "y"), 100.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "in-x"), 85.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "in-y"), 106.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "out-x"), 110.0);
    assert_px_close(anchor_px_attr(&result.source_after, 1, "out-y"), 100.0);
}

#[test]
fn move_path_handle_smooth_rotates_opposite_preserving_old_length() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)10 y=(px)10 kind="smooth" in-x=(px)0 in-y=(px)10 out-x=(px)20 out-y=(px)10
        anchor x=(px)40 y=(px)10
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathHandle {
            node: "path1".to_owned(),
            anchor_index: 0,
            handle: OpPathHandle::Out,
            dx: 0.0,
            dy: 10.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "x"), 10.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "y"), 10.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "out-x"), 20.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "out-y"), 20.0);
    assert_px_close(
        anchor_px_attr(&result.source_after, 0, "in-x"),
        10.0 - (10.0_f64 / 2.0_f64.sqrt()),
    );
    assert_px_close(
        anchor_px_attr(&result.source_after, 0, "in-y"),
        10.0 - (10.0_f64 / 2.0_f64.sqrt()),
    );
}

#[test]
fn move_path_handle_symmetric_mirrors_opposite_equal_length() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)10 y=(px)10 kind="symmetric" in-x=(px)0 in-y=(px)10 out-x=(px)20 out-y=(px)10
        anchor x=(px)40 y=(px)10
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathHandle {
            node: "path1".to_owned(),
            anchor_index: 0,
            handle: OpPathHandle::Out,
            dx: 0.0,
            dy: 10.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "out-x"), 20.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "out-y"), 20.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "in-x"), 0.0);
    assert_px_close(anchor_px_attr(&result.source_after, 0, "in-y"), 0.0);
}

#[test]
fn move_path_handle_smooth_landing_on_anchor_rejected_without_source_change() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)10 y=(px)10 kind="smooth" in-x=(px)0 in-y=(px)10 out-x=(px)20 out-y=(px)10
        anchor x=(px)40 y=(px)10
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathHandle {
            node: "path1".to_owned(),
            anchor_index: 0,
            handle: OpPathHandle::Out,
            dx: -10.0,
            dy: 0.0,
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Rejected);
    assert!(
        result.diagnostics.iter().any(|d| {
            d.code == "tx.invalid_geometry" && d.message.contains("selected handle lands")
        }),
        "expected tx.invalid_geometry for smooth zero direction; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn move_path_handle_missing_target_handle_rejected_without_source_change() {
    let doc = parse(PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MovePathHandle {
            node: "path1".to_owned(),
            anchor_index: 0,
            handle: OpPathHandle::Out,
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
fn move_path_handle_out_of_range_rejected_without_source_change() {
    let doc = parse(PATH_DOC);
    let tx = Transaction {
        ops: vec![Op::MovePathHandle {
            node: "path1".to_owned(),
            anchor_index: 2,
            handle: OpPathHandle::Out,
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
fn move_path_handle_non_finite_delta_rejected_without_source_change() {
    let doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)0 y=(px)0 out-x=(px)10 out-y=(px)0
        anchor x=(px)20 y=(px)0
      }
    }
  }
}"##,
    );
    let tx = Transaction {
        ops: vec![Op::MovePathHandle {
            node: "path1".to_owned(),
            anchor_index: 0,
            handle: OpPathHandle::Out,
            dx: f64::NAN,
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
fn move_path_handle_non_px_target_or_opposite_handle_rejected() {
    let target_doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)0 y=(px)0 out-x=(pt)10 out-y=(px)0
        anchor x=(px)20 y=(px)0
      }
    }
  }
}"##,
    );
    let target_result = run_transaction(
        &target_doc,
        &Transaction {
            ops: vec![Op::MovePathHandle {
                node: "path1".to_owned(),
                anchor_index: 0,
                handle: OpPathHandle::Out,
                dx: 1.0,
                dy: 2.0,
            }],
            permissions: Permissions::default(),
        },
    )
    .expect("run_transaction should not error");

    assert_eq!(target_result.status, TxStatus::Rejected);
    assert!(
        target_result.diagnostics.iter().any(|d| {
            d.code == "tx.invalid_path_anchor" && d.message.contains("must be a px value")
        }),
        "expected tx.invalid_path_anchor for non-px target handle; got: {:?}",
        target_result.diagnostics
    );
    assert_eq!(target_result.source_after, target_result.source_before);

    let opposite_doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" {
        anchor x=(px)10 y=(px)10 kind="smooth" in-x=(pt)0 in-y=(px)10 out-x=(px)20 out-y=(px)10
        anchor x=(px)40 y=(px)10
      }
    }
  }
}"##,
    );
    let opposite_result = run_transaction(
        &opposite_doc,
        &Transaction {
            ops: vec![Op::MovePathHandle {
                node: "path1".to_owned(),
                anchor_index: 0,
                handle: OpPathHandle::Out,
                dx: 1.0,
                dy: 2.0,
            }],
            permissions: Permissions::default(),
        },
    )
    .expect("run_transaction should not error");

    assert_eq!(opposite_result.status, TxStatus::Rejected);
    assert!(
        opposite_result.diagnostics.iter().any(|d| {
            d.code == "tx.invalid_path_anchor" && d.message.contains("must be a px value")
        }),
        "expected tx.invalid_path_anchor for non-px opposite handle; got: {:?}",
        opposite_result.diagnostics
    );
    assert_eq!(opposite_result.source_after, opposite_result.source_before);
}

#[test]
fn move_path_handle_locked_path_and_unsupported_rect_reject() {
    let locked_doc = parse(
        r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" { }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      path id="path1" locked=#true {
        anchor x=(px)0 y=(px)0 out-x=(px)10 out-y=(px)0
        anchor x=(px)20 y=(px)0
      }
    }
  }
}"##,
    );
    let locked_result = run_transaction(
        &locked_doc,
        &Transaction {
            ops: vec![Op::MovePathHandle {
                node: "path1".to_owned(),
                anchor_index: 0,
                handle: OpPathHandle::Out,
                dx: 1.0,
                dy: 2.0,
            }],
            permissions: Permissions::default(),
        },
    )
    .expect("run_transaction should not error");

    assert_eq!(locked_result.status, TxStatus::Rejected);
    assert!(
        locked_result
            .diagnostics
            .iter()
            .any(|d| d.code == "node.locked"),
        "expected node.locked; got: {:?}",
        locked_result.diagnostics
    );
    assert_eq!(locked_result.source_after, locked_result.source_before);

    let rect_result = run_transaction(
        &parse(RECT_GEOM_DOC),
        &Transaction {
            ops: vec![Op::MovePathHandle {
                node: "rect".to_owned(),
                anchor_index: 0,
                handle: OpPathHandle::Out,
                dx: 1.0,
                dy: 2.0,
            }],
            permissions: Permissions::default(),
        },
    )
    .expect("run_transaction should not error");

    assert_eq!(rect_result.status, TxStatus::Rejected);
    assert!(
        rect_result
            .diagnostics
            .iter()
            .any(|d| d.code == "tx.unsupported_property" && d.message.contains("rect")),
        "expected tx.unsupported_property mentioning rect; got: {:?}",
        rect_result.diagnostics
    );
    assert_eq!(rect_result.source_after, rect_result.source_before);
}
