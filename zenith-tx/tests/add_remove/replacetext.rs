use super::*;

// ── ReplaceText tests ─────────────────────────────────────────────────────

#[test]
fn replace_text_updates_spans() {
    let doc = parse(TEXT_DOC);
    let tx = Transaction {
        ops: vec![Op::ReplaceText {
            node: "label".to_owned(),
            spans: vec![OpSpan {
                text: "Goodbye".to_owned(),
                fill: None,
                font_weight: None,
                italic: None,
                underline: None,
                strikethrough: None,
                vertical_align: None,
                footnote_ref: None,
            }],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc, &tx).expect("run_transaction should not error");

    assert_eq!(result.status, TxStatus::Accepted);
    assert_eq!(result.affected_node_ids, vec!["label".to_owned()]);
    assert!(
        result.source_after.contains("Goodbye"),
        "source_after must contain new text; got:\n{}",
        result.source_after
    );
    assert!(
        !result.source_after.contains("Hello"),
        "old text must not appear in source_after"
    );
    assert_ne!(result.source_before, result.source_after);
}

#[test]
fn replace_text_on_rect_unsupported() {
    let doc = parse(MIXED_DOC);
    let tx = Transaction {
        ops: vec![Op::ReplaceText {
            node: "box1".to_owned(),
            spans: vec![OpSpan {
                text: "hi".to_owned(),
                fill: None,
                font_weight: None,
                italic: None,
                underline: None,
                strikethrough: None,
                vertical_align: None,
                footnote_ref: None,
            }],
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
        "expected tx.unsupported_property naming rect; got: {:?}",
        result.diagnostics
    );
    assert_eq!(result.source_after, result.source_before);
}

#[test]
fn replace_text_span_with_fill_token() {
    // A doc that has both color tokens and a text node.
    const TEXT_WITH_TOKEN_DOC: &str = r##"zenith version=1 {
  project id="proj" name="Test"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#ff0000"
    token id="color.b" type="color" value="#0000ff"
  }
  styles { }
  document id="doc1" title="T" {
    page id="pg1" w=(px)400 h=(px)300 {
      text id="lbl" x=(px)10 y=(px)10 w=(px)200 h=(px)40 {
        span "Original"
      }
    }
  }
}"##;
    let doc2 = parse(TEXT_WITH_TOKEN_DOC);
    let tx = Transaction {
        ops: vec![Op::ReplaceText {
            node: "lbl".to_owned(),
            spans: vec![OpSpan {
                text: "Branded".to_owned(),
                fill: Some("color.a".to_owned()),
                font_weight: None,
                italic: None,
                underline: None,
                strikethrough: None,
                vertical_align: None,
                footnote_ref: None,
            }],
        }],
        permissions: Permissions::default(),
    };
    let result = run_transaction(&doc2, &tx).expect("run_transaction should not error");

    assert_eq!(
        result.status,
        TxStatus::Accepted,
        "expected Accepted; diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(result.affected_node_ids, vec!["lbl".to_owned()]);
    // The formatter should emit the span's fill token ref in source_after.
    assert!(
        result.source_after.contains("Branded"),
        "new text must appear in source_after; got:\n{}",
        result.source_after
    );
}
