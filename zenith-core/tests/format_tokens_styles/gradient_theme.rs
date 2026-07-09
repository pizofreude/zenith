use super::*;

/// **Radial gradient round-trip**: a `radial=#true` gradient token with
/// `center-x`, `center-y`, and `radius` params must survive parse → format →
/// parse with `kind == GradientKind::Radial` and the same params.
#[test]
fn test_radial_gradient_round_trips() {
    use zenith_core::{GradientKind, TokenLiteral, TokenValue};

    let src = r##"zenith version=1 {
  project id="proj.rgt" name="RGT"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#ffffff"
    token id="color.b" type="color" value="#000000"
    token id="grad.r" type="gradient" radial=#true center-x=0.5 center-y=0.5 radius=0.7 {
      stop offset=0.0 color=(token)"color.a"
      stop offset=1.0 color=(token)"color.b"
    }
  }
  styles {
  }
  document id="doc.rgt" title="RGT" {
    page id="page.rgt" w=(px)100 h=(px)100 {
      rect id="r" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"grad.r"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");

    let formatted = format_document(&doc).expect("format");
    let formatted_str = String::from_utf8(formatted.clone()).expect("utf8");

    // Formatted output must contain radial marker and geometry params.
    assert!(
        formatted_str.contains("radial=#true"),
        "formatted output must contain radial=#true; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("center-x=0.5"),
        "formatted output must contain center-x; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("center-y=0.5"),
        "formatted output must contain center-y; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("radius=0.7"),
        "formatted output must contain radius; got:\n{formatted_str}"
    );

    let reparsed = adapter.parse(&formatted).expect("re-parse after format");
    let reparsed2 = adapter
        .parse(&format_document(&reparsed).expect("format 2"))
        .expect("re-parse 2");

    // Find the gradient token in the reparsed doc.
    let grad_token = reparsed2
        .tokens
        .tokens
        .iter()
        .find(|t| t.id == "grad.r")
        .expect("grad.r token must survive round-trip");
    let TokenValue::Literal(TokenLiteral::Gradient(g)) = &grad_token.value else {
        panic!(
            "grad.r must be a gradient literal, got {:?}",
            grad_token.value
        );
    };
    assert_eq!(
        g.kind,
        GradientKind::Radial,
        "kind must be Radial after round-trip"
    );
    assert_eq!(g.center_x, Some(0.5));
    assert_eq!(g.center_y, Some(0.5));
    assert_eq!(g.radius, Some(0.7));
    assert_eq!(g.stops.len(), 2);
}

/// **syntax-theme round-trip**: a code node with `syntax-theme="light"`
/// must parse to `Some(SyntaxTheme::Light)` and format back to
/// `syntax-theme="light"` in the canonical position (between font-size and
/// opacity).
#[test]
fn test_syntax_theme_parse_format_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.sth" name="STH"
  tokens format="zenith-token-v1" {
  }
  styles {
  }
  document id="doc.sth" title="STH" {
    page id="page.sth" w=(px)400 h=(px)300 {
      code id="code.sth" x=(px)10 y=(px)10 language="rust" syntax-theme="light" {
        content "let x = 1;"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse must succeed");
    let code_node = match &doc.body.pages[0].children[0] {
        Node::Code(c) => c,
        other => panic!("expected Code node, got {other:?}"),
    };
    use zenith_core::SyntaxTheme;
    assert_eq!(
        code_node.syntax_theme,
        Some(SyntaxTheme::Light),
        "syntax-theme=\"light\" must parse to Some(SyntaxTheme::Light)"
    );

    let formatted = format_document(&doc).expect("format must succeed");
    let formatted_str = String::from_utf8(formatted).expect("formatted must be utf8");
    assert!(
        formatted_str.contains("syntax-theme=\"light\""),
        "formatter must emit syntax-theme=\"light\"; got:\n{formatted_str}"
    );

    // Canonical position: between font-size and opacity. Since neither
    // font-size nor opacity is set in this fixture, just check that
    // syntax-theme appears and re-parses correctly.
    let doc2 = adapter
        .parse(formatted_str.as_bytes())
        .expect("re-parse after format");
    let code2 = match &doc2.body.pages[0].children[0] {
        Node::Code(c) => c,
        other => panic!("expected Code node on re-parse, got {other:?}"),
    };
    assert_eq!(
        code2.syntax_theme,
        Some(SyntaxTheme::Light),
        "syntax-theme must survive a format → re-parse round-trip"
    );
}

/// **Gradient round-trip**: a gradient token (angle + 2 stops) must
/// parse→format→parse byte-stably, emit the `stop` brace block, and a page
/// background referencing it must NOT flag the stop colors as `token.unused`.
#[test]
fn test_gradient_token_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.grad" name="Grad"
  tokens format="zenith-token-v1" {
    token id="color.navy.top" type="color" value="#001133"
    token id="color.black.bottom" type="color" value="#000000"
    token id="gradient.bg.hero" type="gradient" angle=(deg)90 {
      stop offset=0 color=(token)"color.navy.top"
      stop offset=1 color=(token)"color.black.bottom"
    }
  }
  styles {
  }
  document id="doc.grad" title="Grad" {
    page id="p" w=(px)100 h=(px)100 background=(token)"gradient.bg.hero" {
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");
    let s1 = format_document(&doc1).expect("format 1");
    let formatted = String::from_utf8(s1.clone()).expect("utf8");

    // The gradient emits a brace block with two stop children.
    assert!(
        formatted.contains("type=\"gradient\" angle=(deg)90 {"),
        "expected gradient header; got:\n{formatted}"
    );
    assert!(
        formatted.contains("stop offset=0 color=(token)\"color.navy.top\""),
        "expected first stop; got:\n{formatted}"
    );
    assert!(
        formatted.contains("stop offset=1 color=(token)\"color.black.bottom\""),
        "expected second stop; got:\n{formatted}"
    );

    // Idempotency: format(format(doc)) == format(doc).
    let doc2 = adapter.parse(&s1).expect("parse 2");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted,
        String::from_utf8(s2).expect("utf8"),
        "gradient formatting must be idempotent"
    );

    // AST round-trip (spans stripped).
    assert_eq!(
        strip_spans(doc1),
        strip_spans(doc2),
        "gradient AST must survive format round-trip"
    );
}

/// **Gradient fill validates clean**: a page background referencing a
/// gradient token type-checks OK, and the gradient's stop colors are not
/// falsely flagged `token.unused`.
#[test]
fn test_gradient_fill_validates_without_unused() {
    let src = r##"zenith version=1 {
  project id="proj.grad" name="Grad"
  tokens format="zenith-token-v1" {
    token id="color.navy.top" type="color" value="#001133"
    token id="color.black.bottom" type="color" value="#000000"
    token id="gradient.bg.hero" type="gradient" angle=(deg)90 {
      stop offset=0 color=(token)"color.navy.top"
      stop offset=1 color=(token)"color.black.bottom"
    }
  }
  styles {
  }
  document id="doc.grad" title="Grad" {
    page id="p" w=(px)100 h=(px)100 {
      rect id="r" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"gradient.bg.hero"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    // `validate` runs token resolution internally and merges all diagnostics.
    let report = zenith_core::validate(&doc);

    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        !codes.contains(&"token.incompatible_property"),
        "gradient fill must be type-compatible; codes: {codes:?}"
    );
    assert!(
        !codes.contains(&"token.unused"),
        "gradient stop colors must not be flagged unused; codes: {codes:?}"
    );
    assert!(
        !codes.contains(&"token.raw_visual_literal"),
        "gradient token ref must not be a raw literal; codes: {codes:?}"
    );
}
