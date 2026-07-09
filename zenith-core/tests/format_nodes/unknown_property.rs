use super::*;

/// **Unknown-property multi-type round-trip**: unknown properties of every
/// KDL value type survive parse→format→parse with their type intact, and
/// the output is idempotent (format twice → identical bytes).
#[test]
fn test_unknown_property_all_types_round_trip() {
    // Each property exercises one KdlValue variant.
    // Raw string r##"..."## needed because KDL v2 booleans/null use `#`.
    let src = r##"zenith version=1 {
  project id="proj.rt" name="RT"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.rt" title="RT" {
    page id="p" w=(px)100 h=(px)100 {
      rect id="r" x=(px)0 y=(px)0 w=(px)10 h=(px)10 future-flag=#true future-float=1.5 future-int=42 future-null=#null future-str="hi"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");

    // Verify all five types landed correctly after the first parse.
    let rect = match &doc1.body.pages[0].children[0] {
        zenith_core::Node::Rect(r) => r,
        other => panic!("expected Rect, got {other:?}"),
    };
    assert_eq!(
        rect.unknown_props["future-flag"].value,
        zenith_core::UnknownValue::Bool(true),
        "boolean must parse as UnknownValue::Bool(true), not a string"
    );
    assert_eq!(
        rect.unknown_props["future-int"].value,
        zenith_core::UnknownValue::Integer(42),
        "integer must parse as UnknownValue::Integer(42)"
    );
    assert_eq!(
        rect.unknown_props["future-float"].value,
        zenith_core::UnknownValue::Float(1.5),
        "float must parse as UnknownValue::Float(1.5)"
    );
    assert_eq!(
        rect.unknown_props["future-str"].value,
        zenith_core::UnknownValue::String("hi".to_owned()),
        "string must parse as UnknownValue::String"
    );
    assert_eq!(
        rect.unknown_props["future-null"].value,
        zenith_core::UnknownValue::Null,
        "null must parse as UnknownValue::Null"
    );

    // Format once → parse → assert same typed values survive (round-trip).
    let formatted1 = format_document(&doc1).expect("format 1");
    let doc2 = adapter.parse(&formatted1).expect("parse 2 after format");
    let rect2 = match &doc2.body.pages[0].children[0] {
        zenith_core::Node::Rect(r) => r,
        other => panic!("expected Rect in re-parsed doc, got {other:?}"),
    };
    assert_eq!(
        rect2.unknown_props["future-flag"].value,
        zenith_core::UnknownValue::Bool(true),
        "boolean must survive format round-trip as UnknownValue::Bool(true)"
    );
    assert_eq!(
        rect2.unknown_props["future-int"].value,
        zenith_core::UnknownValue::Integer(42),
        "integer must survive format round-trip as UnknownValue::Integer(42)"
    );
    assert_eq!(
        rect2.unknown_props["future-float"].value,
        zenith_core::UnknownValue::Float(1.5),
        "float must survive format round-trip"
    );
    assert_eq!(
        rect2.unknown_props["future-str"].value,
        zenith_core::UnknownValue::String("hi".to_owned()),
        "string must survive format round-trip"
    );
    assert_eq!(
        rect2.unknown_props["future-null"].value,
        zenith_core::UnknownValue::Null,
        "null must survive format round-trip"
    );

    // Idempotence: format a second time → identical bytes.
    let formatted2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted1, formatted2,
        "format must be idempotent for documents with unknown properties of all types"
    );
}

/// **Unknown-property type-annotation round-trip**: KDL type annotations on
/// unrecognized properties (e.g. `(px)42`, `(token)"color.brand"`) must be
/// captured on parse, re-emitted in the value position on format, and survive
/// a full parse→format→parse cycle byte-identically. Non-annotated unknown
/// values must remain unchanged.
#[test]
fn test_unknown_property_type_annotation_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.ann" name="Ann"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.ann" title="Ann" {
    page id="p" w=(px)100 h=(px)100 {
      rect id="r" x=(px)0 y=(px)0 w=(px)10 h=(px)10 mystery=(px)42 magic=(token)"color.brand" plain="hello" flag=#true
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");

    let rect = match &doc1.body.pages[0].children[0] {
        zenith_core::Node::Rect(r) => r,
        other => panic!("expected Rect, got {other:?}"),
    };

    // Annotations captured on parse.
    assert_eq!(
        rect.unknown_props["mystery"].ty.as_deref(),
        Some("px"),
        "`(px)42` must capture ty = Some(\"px\")"
    );
    assert_eq!(
        rect.unknown_props["mystery"].value,
        zenith_core::UnknownValue::Integer(42),
    );
    assert_eq!(
        rect.unknown_props["magic"].ty.as_deref(),
        Some("token"),
        "`(token)\"color.brand\"` must capture ty = Some(\"token\")"
    );
    assert_eq!(
        rect.unknown_props["magic"].value,
        zenith_core::UnknownValue::String("color.brand".to_owned()),
    );
    // Non-annotated unknown props have ty = None.
    assert_eq!(
        rect.unknown_props["plain"].ty, None,
        "non-annotated `plain` must have ty = None"
    );
    assert_eq!(
        rect.unknown_props["flag"].ty, None,
        "non-annotated `flag` must have ty = None"
    );

    // Format → the annotation is emitted in the value position.
    let formatted1 = format_document(&doc1).expect("format 1");
    let text = String::from_utf8_lossy(&formatted1);
    assert!(
        text.contains("mystery=(px)42"),
        "formatted output must contain `mystery=(px)42`, got:\n{text}"
    );
    assert!(
        text.contains(r#"magic=(token)"color.brand""#),
        "formatted output must contain `magic=(token)\"color.brand\"`, got:\n{text}"
    );
    assert!(
        text.contains(r#"plain="hello""#),
        "non-annotated `plain=\"hello\"` must be unchanged, got:\n{text}"
    );
    assert!(
        text.contains("flag=#true"),
        "non-annotated `flag=#true` must be unchanged, got:\n{text}"
    );

    // Re-parse → unknown_props (value + ty) are identical to the first parse.
    let doc2 = adapter.parse(&formatted1).expect("parse 2 after format");
    let rect2 = match &doc2.body.pages[0].children[0] {
        zenith_core::Node::Rect(r) => r,
        other => panic!("expected Rect in re-parsed doc, got {other:?}"),
    };
    assert_eq!(
        rect.unknown_props, rect2.unknown_props,
        "unknown_props (value + ty) must be byte-stable across parse→format→parse"
    );

    // Idempotence: format a second time → identical bytes.
    let formatted2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted1, formatted2,
        "format must be idempotent for annotated unknown properties"
    );
}

/// **Forward-compat preservation**: an unknown property on a rect survives
/// a format round-trip.
#[test]
fn test_unknown_property_preserved() {
    let src = r##"zenith version=1 {
  project id="proj.unk" name="Unk"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.unk" title="Unk" {
    page id="p" w=(px)100 h=(px)100 {
      rect id="r" x=(px)0 y=(px)0 w=(px)10 h=(px)10 future-prop="hello"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    let out = format_document(&doc).expect("format");
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("future-prop="),
        "unknown property `future-prop` must survive format; got:\n{text}"
    );
}
