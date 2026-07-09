use super::*;

// ── Image node parse + format tests ───────────────────────────────────

/// A `.zen` document with a `safe-zone` declared as a page child.
const SAFE_ZONE_DOC: &str = r##"zenith version=1 {
  project id="proj.sz" name="Safe Zone Project"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.sz" title="Safe Zone Doc" {
    page id="page.one" w=(px)1500 h=(px)500 {
      safe-zone id="sz.avatar" type="exclusion" x=(px)0 y=(px)358 w=(px)175 h=(px)142 label="X avatar dead zone"
      rect id="logo" x=(px)600 y=(px)40 w=(px)200 h=(px)80 fill="#ffffff"
    }
  }
}
"##;

/// **Parse**: a `safe-zone` page child lands in `page.safe_zones`, NOT in
/// `page.children`.
#[test]
fn test_safe_zone_parses_into_page_not_children() {
    let adapter = KdlAdapter;
    let doc = adapter
        .parse(SAFE_ZONE_DOC.as_bytes())
        .expect("parse must succeed");
    let page = &doc.body.pages[0];

    assert_eq!(page.safe_zones.len(), 1, "exactly one safe-zone parsed");
    let zone = &page.safe_zones[0];
    assert_eq!(zone.id, "sz.avatar");
    assert_eq!(zone.zone_type, zenith_core::SafeZoneType::Exclusion);
    assert_eq!(zone.label.as_deref(), Some("X avatar dead zone"));

    // The renderable rect is the ONLY child; the safe-zone is not a child.
    assert_eq!(page.children.len(), 1, "only the rect is a child node");
    match &page.children[0] {
        Node::Rect(r) => assert_eq!(r.id, "logo"),
        other => panic!("expected Rect, got {other:?}"),
    }
}

/// **Format round-trip**: a safe-zone survives a parse → format → parse pass
/// unchanged (spans excluded).
#[test]
fn test_safe_zone_format_round_trip() {
    let adapter = KdlAdapter;
    let doc_orig = adapter
        .parse(SAFE_ZONE_DOC.as_bytes())
        .expect("original parse");
    let formatted = format_document(&doc_orig).expect("format");

    // The emitted line carries the canonical safe-zone shape.
    let text = String::from_utf8(formatted.clone()).expect("utf8");
    assert!(
        text.contains(
            "safe-zone id=\"sz.avatar\" type=\"exclusion\" \
             x=(px)0 y=(px)358 w=(px)175 h=(px)142 label=\"X avatar dead zone\""
        ),
        "formatted safe-zone line missing/incorrect; output:\n{text}"
    );

    let doc_reparsed = adapter.parse(&formatted).expect("re-parse after format");
    assert_eq!(
        strip_spans(doc_orig),
        strip_spans(doc_reparsed),
        "safe-zone must survive a format round-trip (spans excluded)"
    );
}

/// A safe-zone `label` containing a double-quote and a newline must be escaped on
/// emit so the formatted document re-parses to the identical label.
#[test]
fn test_safe_zone_label_escaping_round_trip() {
    let src = "zenith version=1 {\n  \
         project id=\"proj.szesc\" name=\"SZEsc\"\n  \
         tokens format=\"zenith-token-v1\" {\n  }\n  \
         styles {\n  }\n  \
         document id=\"doc.szesc\" title=\"SZEsc\" {\n    \
           page id=\"page.one\" w=(px)800 h=(px)600 {\n      \
             safe-zone id=\"sz.q\" type=\"exclusion\" x=(px)0 y=(px)0 w=(px)10 h=(px)10 \
                 label=\"a \\\"q\\\" b\\nc\"\n    }\n  }\n}\n";
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let label = doc.body.pages[0].safe_zones[0]
        .label
        .clone()
        .expect("label present");
    assert_eq!(
        label, "a \"q\" b\nc",
        "parsed label has the raw special chars"
    );

    let formatted = format_document(&doc).expect("format must succeed");
    let doc2 = adapter
        .parse(&formatted)
        .expect("re-parse after format must succeed");
    assert_eq!(
        doc2.body.pages[0].safe_zones[0].label.as_deref(),
        Some("a \"q\" b\nc"),
        "safe-zone label with quote/newline must survive parse → format → parse"
    );
}

/// A `.zen` document with a `fold` declared as a page child.
const FOLD_DOC: &str = r##"zenith version=1 {
  project id="proj.fold" name="Fold Project"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.fold" title="Fold Doc" {
    page id="page.one" w=(px)2480 h=(px)1000 {
      fold id="fold.1" orientation="vertical" position=(px)1169
      rect id="logo" x=(px)600 y=(px)40 w=(px)200 h=(px)80 fill="#ffffff"
    }
  }
}
"##;

/// **Parse**: a `fold` page child lands in `page.folds`, NOT in
/// `page.children`.
#[test]
fn test_fold_parses_into_page_not_children() {
    let adapter = KdlAdapter;
    let doc = adapter
        .parse(FOLD_DOC.as_bytes())
        .expect("parse must succeed");
    let page = &doc.body.pages[0];

    assert_eq!(page.folds.len(), 1, "exactly one fold parsed");
    let fold = &page.folds[0];
    assert_eq!(fold.id, "fold.1");
    assert_eq!(fold.orientation, "vertical");
    let pos = fold.position.as_ref().expect("position present");
    assert_eq!(pos.value, 1169.0);

    // The renderable rect is the ONLY child; the fold is not a child.
    assert_eq!(page.children.len(), 1, "only the rect is a child node");
    match &page.children[0] {
        Node::Rect(r) => assert_eq!(r.id, "logo"),
        other => panic!("expected Rect, got {other:?}"),
    }
}

/// **Format round-trip**: a fold survives a parse → format → parse pass
/// unchanged (spans excluded).
#[test]
fn test_fold_format_round_trip() {
    let adapter = KdlAdapter;
    let doc_orig = adapter.parse(FOLD_DOC.as_bytes()).expect("original parse");
    let formatted = format_document(&doc_orig).expect("format");

    let text = String::from_utf8(formatted.clone()).expect("utf8");
    assert!(
        text.contains("fold id=\"fold.1\" orientation=\"vertical\" position=(px)1169"),
        "formatted fold line missing/incorrect; output:\n{text}"
    );

    let doc_reparsed = adapter.parse(&formatted).expect("re-parse after format");
    assert_eq!(
        strip_spans(doc_orig),
        strip_spans(doc_reparsed),
        "fold must survive a format round-trip (spans excluded)"
    );
}
