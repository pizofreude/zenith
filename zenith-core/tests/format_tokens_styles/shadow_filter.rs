use super::*;

/// **Shadow round-trip**: a shadow token (2 layers) must parse→format→parse
/// byte-stably, emit the `layer` brace block, and a text node referencing it
/// (via `shadow=(token)"..."`) must survive the round-trip.
#[test]
fn test_shadow_token_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.shadow" name="Shadow"
  tokens format="zenith-token-v1" {
    token id="color.shadow.black" type="color" value="#000000"
    token id="color.glow.cyan" type="color" value="#00ffff"
    token id="shadow.headline" type="shadow" {
      layer dx=(px)8 dy=(px)8 blur=(px)24 color=(token)"color.shadow.black"
      layer dx=(px)0 dy=(px)0 blur=(px)20 color=(token)"color.glow.cyan"
    }
  }
  styles {
  }
  document id="doc.shadow" title="Shadow" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 shadow=(token)"shadow.headline" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");
    let s1 = format_document(&doc1).expect("format 1");
    let formatted = String::from_utf8(s1.clone()).expect("utf8");

    // The shadow emits a brace block with two layer children.
    assert!(
        formatted.contains("type=\"shadow\" {"),
        "expected shadow header; got:\n{formatted}"
    );
    assert!(
        formatted
            .contains("layer dx=(px)8 dy=(px)8 blur=(px)24 color=(token)\"color.shadow.black\""),
        "expected first layer; got:\n{formatted}"
    );
    assert!(
        formatted.contains(" shadow=(token)\"shadow.headline\""),
        "expected node shadow prop; got:\n{formatted}"
    );

    // Idempotency.
    let doc2 = adapter.parse(&s1).expect("parse 2");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted,
        String::from_utf8(s2).expect("utf8"),
        "shadow formatting must be idempotent"
    );

    // AST round-trip (spans stripped).
    assert_eq!(
        strip_spans(doc1),
        strip_spans(doc2),
        "shadow AST must survive format round-trip"
    );
}

/// **Shadow on a node validates clean**: a text node referencing a shadow
/// token type-checks OK, and the shadow's layer colors are not falsely
/// flagged `token.unused`.
#[test]
fn test_shadow_node_validates_without_unused() {
    let src = r##"zenith version=1 {
  project id="proj.shadow" name="Shadow"
  tokens format="zenith-token-v1" {
    token id="color.shadow.black" type="color" value="#000000"
    token id="color.glow.cyan" type="color" value="#00ffff"
    token id="shadow.headline" type="shadow" {
      layer dx=(px)8 dy=(px)8 blur=(px)24 color=(token)"color.shadow.black"
      layer dx=(px)0 dy=(px)0 blur=(px)20 color=(token)"color.glow.cyan"
    }
  }
  styles {
  }
  document id="doc.shadow" title="Shadow" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 shadow=(token)"shadow.headline" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    let report = zenith_core::validate(&doc);

    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        !codes.contains(&"token.incompatible_property"),
        "shadow ref must be type-compatible; codes: {codes:?}"
    );
    assert!(
        !codes.contains(&"token.unused"),
        "shadow layer colors must not be flagged unused; codes: {codes:?}"
    );
    assert!(
        !codes.contains(&"token.raw_visual_literal"),
        "shadow token ref must not be a raw literal; codes: {codes:?}"
    );
}

/// **Filter round-trip**: a filter token (2 ops, one with an `amount`, one
/// without) must parse→format→parse byte-stably, emit the op brace block, and
/// a text node referencing it (via `filter=(token)"..."`) must survive.
#[test]
fn test_filter_token_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.filter" name="Filter"
  tokens format="zenith-token-v1" {
    token id="filter.photo" type="filter" {
      grayscale amount=0.5
      hue-rotate
    }
  }
  styles {
  }
  document id="doc.filter" title="Filter" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 filter=(token)"filter.photo" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");
    let s1 = format_document(&doc1).expect("format 1");
    let formatted = String::from_utf8(s1.clone()).expect("utf8");

    // The filter emits a brace block with two op children.
    assert!(
        formatted.contains("type=\"filter\" {"),
        "expected filter header; got:\n{formatted}"
    );
    assert!(
        formatted.contains("grayscale amount=0.5"),
        "expected grayscale op with amount; got:\n{formatted}"
    );
    assert!(
        formatted.contains(" filter=(token)\"filter.photo\""),
        "expected node filter prop; got:\n{formatted}"
    );

    // Idempotency.
    let doc2 = adapter.parse(&s1).expect("parse 2");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted,
        String::from_utf8(s2).expect("utf8"),
        "filter formatting must be idempotent"
    );

    // AST round-trip (spans stripped).
    assert_eq!(
        strip_spans(doc1),
        strip_spans(doc2),
        "filter AST must survive format round-trip"
    );
}

/// **Filter on a node validates clean**: a text node referencing a filter
/// token type-checks OK and is not flagged as a raw literal.
#[test]
fn test_filter_node_validates_clean() {
    let src = r##"zenith version=1 {
  project id="proj.filter" name="Filter"
  tokens format="zenith-token-v1" {
    token id="filter.photo" type="filter" {
      grayscale amount=0.5
      hue-rotate
    }
  }
  styles {
  }
  document id="doc.filter" title="Filter" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 filter=(token)"filter.photo" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    let report = zenith_core::validate(&doc);

    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        !codes.contains(&"token.incompatible_property"),
        "filter ref must be type-compatible; codes: {codes:?}"
    );
    assert!(
        !codes.contains(&"token.raw_visual_literal"),
        "filter token ref must not be a raw literal; codes: {codes:?}"
    );
}

/// **Filter prop wrong type**: a node `filter=(token)"x"` where `x` is a color
/// token must produce `token.incompatible_property`.
#[test]
fn test_filter_node_prop_wrong_type() {
    let src = r##"zenith version=1 {
  project id="proj.filter" name="Filter"
  tokens format="zenith-token-v1" {
    token id="color.not-a-filter" type="color" value="#000000"
  }
  styles {
  }
  document id="doc.filter" title="Filter" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 filter=(token)"color.not-a-filter" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    let report = zenith_core::validate(&doc);

    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        codes.contains(&"token.incompatible_property"),
        "a non-filter token in a filter slot must be incompatible; codes: {codes:?}"
    );
}
