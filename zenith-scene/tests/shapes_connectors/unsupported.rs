use super::*;

// ── connector.unsupported_outline (Warning) ───────────────────────────────────
/// A DIVIDED anchor on a rounded rect (any corner radius set) falls back to the
/// bounds perimeter and emits `connector.unsupported_outline`. The routed
/// endpoints are byte-identical to the same document with a PLAIN rect target.
#[test]
fn connector_divided_anchor_on_rounded_rect_warns_and_matches_box() {
    let base = |extra: &str| {
        format!(
            r##"zenith version=1 {{
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {{
token id="color.line" type="color" value="#1e3a8a"
  }}
  styles {{}}
  document id="doc.cn" title="CN" {{
page id="page.cn" w=(px)640 h=(px)360 {{
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 {extra}
  connector id="c1" from="a" to="b" from-anchor="0/4" to-anchor="1/6" stroke=(token)"color.line"
}}
  }}
}}
"##
        )
    };

    let plain = compile(&parse(&base("")), &default_provider());
    let rounded = compile(&parse(&base("radius=(px)8")), &default_provider());

    // The rounded rect warns; the plain rect does not.
    assert!(
        !has_diag(&plain, "connector.unsupported_outline"),
        "a plain rect target must not warn"
    );
    assert!(
        has_diag(&rounded, "connector.unsupported_outline"),
        "a divided anchor on a rounded rect must warn; got: {:?}",
        rounded.diagnostics
    );

    // Render is byte-identical: ApproxOutline resolves exactly like BoxLike.
    assert_eq!(
        first_stroke_polyline_points(&plain.scene.commands),
        first_stroke_polyline_points(&rounded.scene.commands),
        "ApproxOutline must resolve on the bounds perimeter, identical to BoxLike"
    );
}

/// A DIVIDED anchor on a `shape kind="process"` (rounded rect outline) warns.
#[test]
fn connector_divided_anchor_on_process_shape_warns() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  shape id="b" kind="process" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="a" to="b" to-anchor="1/6" stroke=(token)"color.line"
}
  }
}
"##;
    let result = compile(&parse(src), &default_provider());
    assert!(
        has_diag(&result, "connector.unsupported_outline"),
        "a divided anchor on a process shape must warn; got: {:?}",
        result.diagnostics
    );
}

/// A NAMED anchor on a rounded rect is the intended bounds semantics — no warning.
/// Likewise a DIVIDED anchor on a plain rect or an ellipse must not warn.
#[test]
fn connector_named_anchor_and_plain_shapes_do_not_warn_unsupported_outline() {
    // Named anchor on a rounded rect: no warning.
    let named = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 radius=(px)8
  connector id="c1" from="a" to="b" to-anchor="top-left" stroke=(token)"color.line"
}
  }
}
"##;
    let result = compile(&parse(named), &default_provider());
    assert!(
        !has_diag(&result, "connector.unsupported_outline"),
        "a named anchor on a rounded rect must not warn; got: {:?}",
        result.diagnostics
    );

    // Divided anchor on a plain rect + an ellipse: neither is ApproxOutline.
    let plain_shapes = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  ellipse id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="a" to="b" from-anchor="0/4" to-anchor="1/4" stroke=(token)"color.line"
}
  }
}
"##;
    let result = compile(&parse(plain_shapes), &default_provider());
    assert!(
        !has_diag(&result, "connector.unsupported_outline"),
        "divided anchors on a plain rect/ellipse must not warn; got: {:?}",
        result.diagnostics
    );
}
