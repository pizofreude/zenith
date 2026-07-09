use super::*;

/// A `.zen` document with an image node exercising the string and `(pct)`
/// object-position forms.
const WITH_IMAGE: &str = r##"zenith version=1 {
  project id="proj.img" name="Image Test"
  assets {
    asset id="asset.logo" kind="image" src="assets/logo.png"
  }
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.img" title="Image Test" {
    page id="page.one" w=(px)320 h=(px)200 {
      image id="img.logo" asset="asset.logo" x=(px)80 y=(px)60 w=(px)160 h=(px)48 fit="contain" object-position-x="center" object-position-y=(pct)25
    }
  }
}
"##;

/// Image node parses all fields including both object-position forms.
#[test]
fn image_parses_fields() {
    use zenith_core::{Node, ObjectPosition, Unit};
    let adapter = KdlAdapter;
    let doc = adapter.parse(WITH_IMAGE.as_bytes()).expect("parse");
    let node = &doc.body.pages[0].children[0];
    let img = match node {
        Node::Image(i) => i,
        other => panic!("expected Image, got {other:?}"),
    };
    assert_eq!(img.id, "img.logo");
    assert_eq!(img.asset, "asset.logo");
    let geom_value = |pv: Option<&PropertyValue>| match pv {
        Some(PropertyValue::Dimension(d)) => Some(d.value),
        _ => None,
    };
    assert_eq!(geom_value(img.x.as_ref()), Some(80.0));
    assert_eq!(geom_value(img.y.as_ref()), Some(60.0));
    assert_eq!(geom_value(img.w.as_ref()), Some(160.0));
    assert_eq!(geom_value(img.h.as_ref()), Some(48.0));
    assert!(matches!(
        img.x.as_ref(),
        Some(PropertyValue::Dimension(d)) if d.unit == Unit::Px
    ));
    assert_eq!(img.fit.as_deref(), Some("contain"));
    assert_eq!(img.object_position_x, Some(ObjectPosition::Center));
    assert_eq!(img.object_position_y, Some(ObjectPosition::Pct(25.0)));
}

/// Image node round-trips through format → parse with fields intact, and
/// the formatter is idempotent (incl. an object-position `(pct)25`).
#[test]
fn image_format_round_trip_and_idempotency() {
    use zenith_core::{Node, ObjectPosition};
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(WITH_IMAGE.as_bytes()).expect("parse 1");
    let s1 = format_document(&doc1).expect("format 1");

    // The (pct)25 must survive as an annotated number, not a string.
    let text = String::from_utf8(s1.clone()).unwrap();
    assert!(
        text.contains("object-position-y=(pct)25"),
        "object-position (pct) must format as annotated number; got:\n{text}"
    );
    assert!(
        text.contains("object-position-x=\"center\""),
        "object-position anchor must format as string; got:\n{text}"
    );

    let doc2 = adapter.parse(&s1).expect("parse 2");
    let img2 = match &doc2.body.pages[0].children[0] {
        Node::Image(i) => i,
        other => panic!("expected Image, got {other:?}"),
    };
    assert_eq!(img2.asset, "asset.logo");
    assert_eq!(img2.fit.as_deref(), Some("contain"));
    assert_eq!(img2.object_position_x, Some(ObjectPosition::Center));
    assert_eq!(img2.object_position_y, Some(ObjectPosition::Pct(25.0)));

    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        String::from_utf8(s1).unwrap(),
        String::from_utf8(s2).unwrap(),
        "image format must be idempotent"
    );
}

// ── Style block parse + format tests ──────────────────────────────────

/// **src-rect round-trip**: an image node with `src-x`/`src-y`/`src-w`/`src-h`
/// must parse → format → re-parse byte-identically (all four src-* fields
/// survive the round-trip).
#[test]
fn test_image_src_rect_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.srcrt" name="SrcRt"
  assets {
    asset id="asset.photo" kind="image" src="assets/photo.png"
  }
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.srcrt" title="SrcRt" {
    page id="page.srcrt" w=(px)400 h=(px)300 {
      image id="img.srcrt" asset="asset.photo" x=(px)0 y=(px)0 w=(px)200 h=(px)100 src-x=(px)10 src-y=(px)20 src-w=(px)50 src-h=(px)60 fit="stretch"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let image_node = match &doc.body.pages[0].children[0] {
        Node::Image(i) => i,
        other => panic!("expected Image node, got {other:?}"),
    };

    use zenith_core::{Dimension, Unit};
    assert_eq!(
        image_node.src_x,
        Some(Dimension {
            value: 10.0,
            unit: Unit::Px
        }),
        "src-x must parse to (px)10"
    );
    assert_eq!(
        image_node.src_y,
        Some(Dimension {
            value: 20.0,
            unit: Unit::Px
        }),
        "src-y must parse to (px)20"
    );
    assert_eq!(
        image_node.src_w,
        Some(Dimension {
            value: 50.0,
            unit: Unit::Px
        }),
        "src-w must parse to (px)50"
    );
    assert_eq!(
        image_node.src_h,
        Some(Dimension {
            value: 60.0,
            unit: Unit::Px
        }),
        "src-h must parse to (px)60"
    );

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted.clone()).expect("formatted must be utf8");
    assert!(
        formatted_str.contains("src-x=(px)10"),
        "formatter must emit src-x=(px)10; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("src-y=(px)20"),
        "formatter must emit src-y=(px)20; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("src-w=(px)50"),
        "formatter must emit src-w=(px)50; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("src-h=(px)60"),
        "formatter must emit src-h=(px)60; got:\n{formatted_str}"
    );

    let reparsed = adapter.parse(&formatted).expect("re-parse after format");
    assert_eq!(
        strip_spans(doc),
        strip_spans(reparsed),
        "src-rect image must round-trip identically"
    );
}
