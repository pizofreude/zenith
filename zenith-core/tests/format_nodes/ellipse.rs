use super::*;

// ── sections: parse, serialize, and round-trip ────────────────────────

/// **Ellipse stroke + stroke-width round-trip**: an ellipse with both
/// `stroke` and `stroke-width` tokens must survive parse→format→parse with
/// those fields preserved in the canonical position (after `fill`).
#[test]
fn ellipse_stroke_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.es" name="ES"
  tokens format="zenith-token-v1" {
    token id="color.border" type="color" value="#334155"
    token id="size.border" type="dimension" value=(px)3
  }
  styles {
  }
  document id="doc.es" title="ES" {
    page id="p" w=(px)200 h=(px)200 {
      ellipse id="e" x=(px)10 y=(px)10 w=(px)80 h=(px)80 stroke=(token)"color.border" stroke-width=(token)"size.border"
    }
  }
}
"##;
    use zenith_core::{Node, PropertyValue};
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");

    // Verify AST fields are set.
    match &doc.body.pages[0].children[0] {
        Node::Ellipse(e) => {
            assert_eq!(
                e.stroke,
                Some(PropertyValue::TokenRef("color.border".to_owned())),
                "stroke must parse to TokenRef(color.border)"
            );
            assert_eq!(
                e.stroke_width,
                Some(PropertyValue::TokenRef("size.border".to_owned())),
                "stroke_width must parse to TokenRef(size.border)"
            );
            assert!(e.fill.is_none(), "fill must be absent");
        }
        other => panic!("expected Ellipse, got {other:?}"),
    }

    // Format and re-parse — the tokens must survive.
    let formatted = format_document(&doc).expect("format");
    let formatted_str = String::from_utf8(formatted.clone()).expect("utf8");
    let doc2 = adapter.parse(&formatted).expect("re-parse");
    match &doc2.body.pages[0].children[0] {
        Node::Ellipse(e) => {
            assert_eq!(
                e.stroke,
                Some(PropertyValue::TokenRef("color.border".to_owned())),
                "stroke must survive format round-trip"
            );
            assert_eq!(
                e.stroke_width,
                Some(PropertyValue::TokenRef("size.border".to_owned())),
                "stroke_width must survive format round-trip"
            );
        }
        other => panic!("expected Ellipse on re-parse, got {other:?}"),
    }

    // Canonical position: stroke comes after fill.
    let ellipse_line = formatted_str
        .lines()
        .find(|l| l.trim_start().starts_with("ellipse"))
        .expect("must find ellipse line");
    assert!(
        ellipse_line.contains("stroke=(token)\"color.border\""),
        "formatted line must contain stroke token; got: {ellipse_line}"
    );
    assert!(
        ellipse_line.contains("stroke-width=(token)\"size.border\""),
        "formatted line must contain stroke-width token; got: {ellipse_line}"
    );
    // stroke must come before stroke-width (canonical order).
    let pos_stroke = ellipse_line.find(" stroke=").expect("must have stroke=");
    let pos_sw = ellipse_line
        .find(" stroke-width=")
        .expect("must have stroke-width=");
    assert!(
        pos_stroke < pos_sw,
        "stroke= must appear before stroke-width= in canonical output"
    );

    // Idempotency: format(format(doc)) == format(doc).
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted_str,
        String::from_utf8(s2).unwrap(),
        "ellipse stroke formatting must be idempotent"
    );
}
