use super::*;

// ── connector owned label ─────────────────────────────────────────────────────

/// A `connector` with a `span "Yes"` label must survive parse → format → parse.
/// After roundtrip the connector node carries the original span text and `text-style`.
#[test]
fn connector_label_roundtrip_preserves_spans_and_text_style() {
    let src = r##"zenith version=1 {
  project id="proj.clrt" name="CLRT"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.clrt" title="CLRT" {
    page id="p.clrt" w=(px)640 h=(px)360 {
      rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
      rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80
      connector id="c1" from="a" to="b" text-style="s.branch" {
        span "Yes"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");

    // Verify that after parse the connector carries the span and text_style.
    let connector = match &doc.body.pages[0].children[2] {
        Node::Connector(c) => c,
        other => panic!("expected Connector node, got {other:?}"),
    };
    assert_eq!(connector.spans.len(), 1, "connector must have 1 span");
    assert_eq!(connector.spans[0].text, "Yes", "span text must be \"Yes\"");
    assert_eq!(
        connector.text_style.as_deref(),
        Some("s.branch"),
        "text-style must parse as \"s.branch\""
    );

    // Format then re-parse and verify the span survives.
    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted).expect("formatted must be utf8");

    // The formatted text must contain `span "Yes"` inside a connector block.
    assert!(
        formatted_str.contains("span \"Yes\""),
        "formatted output must contain span \"Yes\"; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("text-style=\"s.branch\""),
        "formatted output must contain text-style=\"s.branch\"; got:\n{formatted_str}"
    );

    let doc2 = adapter
        .parse(formatted_str.as_bytes())
        .expect("re-parse after format must succeed");
    let connector2 = match &doc2.body.pages[0].children[2] {
        Node::Connector(c) => c,
        other => panic!("expected Connector on re-parse, got {other:?}"),
    };
    assert_eq!(
        connector2.spans.len(),
        1,
        "connector must still have 1 span after roundtrip"
    );
    assert_eq!(
        connector2.spans[0].text, "Yes",
        "span text must survive parse → format → parse"
    );
    assert_eq!(
        connector2.text_style.as_deref(),
        Some("s.branch"),
        "text-style must survive parse → format → parse"
    );
}

/// A connector WITHOUT spans must emit NO `{ }` block — the formatted line must
/// end with a plain `\n` (byte-identical to the pre-label behaviour).
#[test]
fn connector_without_label_emits_no_brace_block() {
    let src = r##"zenith version=1 {
  project id="proj.cnbl" name="CNBL"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.cnbl" title="CNBL" {
    page id="p.cnbl" w=(px)640 h=(px)360 {
      rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
      rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80
      connector id="c1" from="a" to="b"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted).expect("utf8");

    // The connector line must NOT contain a `{` (no brace block emitted).
    let connector_line = formatted_str
        .lines()
        .find(|l| l.trim_start().starts_with("connector "))
        .expect("formatted output must contain a connector line");
    assert!(
        !connector_line.contains('{'),
        "label-less connector must not emit a brace block; line: {connector_line:?}"
    );
}
