use super::*;

/// Source document with style properties to test parsing and formatting.
const WITH_STYLES: &str = r##"zenith version=1 {
  project id="proj.styles" name="Styles Test"
  tokens format="zenith-token-v1" {
    token id="color.text.primary" type="color" value="#111827"
    token id="size.text.title" type="dimension" value=(pt)24
    token id="font.family.body" type="fontFamily" value="Noto Sans"
  }
  styles {
    style id="style.text.title" {
      fill (token)"color.text.primary"
      font-family (token)"font.family.body"
      font-size (token)"size.text.title"
    }
  }
  document id="doc.styles" {
    page id="page.one" w=(px)640 h=(px)360 {
    }
  }
}
"##;

/// Style properties are parsed into `Style.properties` with correct canonical keys.
#[test]
fn style_properties_parsed() {
    use zenith_core::PropertyValue;
    let adapter = KdlAdapter;
    let doc = adapter.parse(WITH_STYLES.as_bytes()).expect("parse");

    assert_eq!(doc.styles.styles.len(), 1);
    let style = &doc.styles.styles[0];
    assert_eq!(style.id, "style.text.title");
    assert_eq!(style.properties.len(), 3);

    assert_eq!(
        style.properties.get("fill"),
        Some(&PropertyValue::TokenRef("color.text.primary".to_owned())),
        "fill must be a TokenRef to color.text.primary"
    );
    assert_eq!(
        style.properties.get("font-family"),
        Some(&PropertyValue::TokenRef("font.family.body".to_owned())),
        "font-family must be a TokenRef to font.family.body"
    );
    assert_eq!(
        style.properties.get("font-size"),
        Some(&PropertyValue::TokenRef("size.text.title".to_owned())),
        "font-size must be a TokenRef to size.text.title"
    );
}

/// Underscore variant keys are canonicalized to hyphenated forms.
#[test]
fn style_underscore_keys_canonicalized() {
    use zenith_core::PropertyValue;
    let src = r##"zenith version=1 {
  project id="proj.usk" name="USK"
  tokens format="zenith-token-v1" {
    token id="size.sw" type="dimension" value=(px)2
  }
  styles {
    style id="style.usk" {
      stroke_width (token)"size.sw"
    }
  }
  document id="doc.usk" {
    page id="page.usk" w=(px)100 h=(px)100 {
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");

    let style = &doc.styles.styles[0];
    assert!(
        style.properties.contains_key("stroke-width"),
        "stroke_width must be stored under canonical key stroke-width"
    );
    assert!(
        !style.properties.contains_key("stroke_width"),
        "underscore key must not appear in properties map"
    );
    assert_eq!(
        style.properties.get("stroke-width"),
        Some(&PropertyValue::TokenRef("size.sw".to_owned()))
    );
}

/// `padding` and `gap` are recognized token-only dimension style props:
/// they parse into `Style.properties` under their canonical keys and survive
/// a parse → format → parse round-trip.
#[test]
fn style_padding_gap_round_trip() {
    use zenith_core::PropertyValue;
    let src = r##"zenith version=1 {
  project id="proj.pg" name="PG"
  tokens format="zenith-token-v1" {
    token id="space.pad" type="dimension" value=(px)16
    token id="space.gap" type="dimension" value=(px)8
  }
  styles {
    style id="style.flow" {
      gap (token)"space.gap"
      padding (token)"space.pad"
    }
  }
  document id="doc.pg" {
    page id="page.pg" w=(px)200 h=(px)200 {
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");

    let style = &doc.styles.styles[0];
    assert_eq!(
        style.properties.get("padding"),
        Some(&PropertyValue::TokenRef("space.pad".to_owned())),
        "padding must be a TokenRef to space.pad"
    );
    assert_eq!(
        style.properties.get("gap"),
        Some(&PropertyValue::TokenRef("space.gap".to_owned())),
        "gap must be a TokenRef to space.gap"
    );
    assert!(
        style.unknown_props.is_empty(),
        "padding/gap must be recognized, not captured as unknown props"
    );

    // Round-trip: parse → format → parse preserves both props.
    let formatted = format_document(&doc).expect("format");
    let reparsed = adapter.parse(&formatted).expect("re-parse after format");
    let style2 = &reparsed.styles.styles[0];
    assert_eq!(
        style2.properties.get("padding"),
        Some(&PropertyValue::TokenRef("space.pad".to_owned())),
        "padding must survive round-trip"
    );
    assert_eq!(
        style2.properties.get("gap"),
        Some(&PropertyValue::TokenRef("space.gap".to_owned())),
        "gap must survive round-trip"
    );
}

/// Unknown style child names are captured in `unknown_props`.
#[test]
fn style_unknown_child_captured() {
    let src = r##"zenith version=1 {
  project id="proj.unk" name="UNK"
  tokens format="zenith-token-v1" {
  }
  styles {
    style id="style.unk" {
      bogus "some-value"
    }
  }
  document id="doc.unk" {
    page id="page.unk" w=(px)100 h=(px)100 {
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");

    let style = &doc.styles.styles[0];
    assert!(style.properties.is_empty(), "no recognized props expected");
    assert!(
        style.unknown_props.contains_key("bogus"),
        "unknown prop 'bogus' must be captured in unknown_props"
    );
}

/// Parse → format → parse round-trips correctly (spans stripped for equality).
#[test]
fn styles_round_trip() {
    let adapter = KdlAdapter;
    let doc_orig = adapter
        .parse(WITH_STYLES.as_bytes())
        .expect("original parse");
    let formatted = format_document(&doc_orig).expect("format");
    let doc_reparsed = adapter.parse(&formatted).expect("re-parse after format");

    let orig_stripped = strip_spans(doc_orig);
    let reparsed_stripped = strip_spans(doc_reparsed);
    assert_eq!(
        orig_stripped.styles, reparsed_stripped.styles,
        "styles must survive round-trip (spans excluded)"
    );
}

/// Format twice → identical bytes (idempotency).
#[test]
fn styles_format_idempotent() {
    let adapter = KdlAdapter;
    let doc = adapter.parse(WITH_STYLES.as_bytes()).expect("parse");
    let s1 = format_document(&doc).expect("format 1");
    let doc2 = adapter.parse(&s1).expect("parse after first format");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        String::from_utf8(s1).unwrap(),
        String::from_utf8(s2).unwrap(),
        "styles format must be idempotent"
    );
}
