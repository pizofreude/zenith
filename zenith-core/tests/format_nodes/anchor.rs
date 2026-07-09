use super::*;

/// **anchor-sibling round-trip**: a rect with `anchor="top-left"` and
/// `anchor-sibling="some-id"` must parse onto the AST with both fields set,
/// survive `format_document`, and still carry `anchor-sibling="some-id"` after
/// a format → re-parse cycle.
#[test]
fn test_anchor_sibling_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.as" name="AS"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.as" title="AS" {
    page id="p" w=(px)200 h=(px)200 {
      rect id="r" anchor="top-left" anchor-sibling="some-id" x=(px)0 y=(px)0 w=(px)50 h=(px)50
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");

    // Verify AST fields are set after parse.
    match &doc.body.pages[0].children[0] {
        Node::Rect(r) => {
            assert_eq!(
                r.anchor.as_deref(),
                Some("top-left"),
                "anchor must parse to \"top-left\""
            );
            assert_eq!(
                r.anchor_sibling.as_deref(),
                Some("some-id"),
                "anchor-sibling must parse to \"some-id\""
            );
        }
        other => panic!("expected Rect, got {other:?}"),
    }

    // Format and assert the KDL text contains anchor-sibling.
    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted).expect("formatted must be utf8");
    assert!(
        formatted_str.contains("anchor-sibling=\"some-id\""),
        "formatter must emit anchor-sibling=\"some-id\"; got:\n{formatted_str}"
    );

    // Re-parse the formatted output and verify anchor-sibling survived.
    let doc2 = adapter
        .parse(formatted_str.as_bytes())
        .expect("re-parse after format must succeed");
    match &doc2.body.pages[0].children[0] {
        Node::Rect(r) => {
            assert_eq!(
                r.anchor_sibling.as_deref(),
                Some("some-id"),
                "anchor-sibling must survive a format → re-parse round-trip"
            );
            assert_eq!(
                r.anchor.as_deref(),
                Some("top-left"),
                "anchor must survive a format → re-parse round-trip"
            );
        }
        other => panic!("expected Rect on re-parse, got {other:?}"),
    }
}
