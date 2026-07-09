use super::*;

// ── connector.unsupported_outline (Warning) ───────────────────────────────────
/// A DIVIDED anchor on a rounded rect walks the true rounded perimeter — no
/// `connector.unsupported_outline` warning — and the attachment point differs
/// from a sharp box of the same bounds when the walk lands on a corner arc.
#[test]
fn connector_divided_anchor_on_rounded_rect_uses_true_perimeter() {
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

    assert!(
        !has_diag(&plain, "connector.unsupported_outline"),
        "a plain rect target must not warn"
    );
    assert!(
        !has_diag(&rounded, "connector.unsupported_outline"),
        "a divided anchor on a rounded rect must not warn; got: {:?}",
        rounded.diagnostics
    );

    // to-anchor 1/6 on a 100×80 box: plain lands on the right edge; with r=8 the
    // true perimeter is shorter on the corners so the point moves.
    let plain_pts = first_stroke_polyline_points(&plain.scene.commands);
    let rounded_pts = first_stroke_polyline_points(&rounded.scene.commands);
    assert_eq!(plain_pts, vec![90.0, 40.0, 400.0, 70.0]);
    // Endpoint on rounded rect must still sit on/near the box exterior, and
    // differ from the sharp-box attachment for this divided index.
    assert_eq!(rounded_pts.len(), 4);
    assert_ne!(
        plain_pts[2..],
        rounded_pts[2..],
        "rounded-rect divided anchor must differ from sharp box for 1/6"
    );
    // Still attaches on the right side of b (x ≈ 400).
    assert!(
        (rounded_pts[2] - 400.0).abs() < 8.0,
        "expected attachment near right edge; got {:?}",
        rounded_pts
    );
}

/// A DIVIDED anchor on a sharp `process` shape resolves on the bounds perimeter
/// without warning (BoxLike).
#[test]
fn connector_divided_anchor_on_process_shape_does_not_warn() {
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
        !has_diag(&result, "connector.unsupported_outline"),
        "a divided anchor on a process shape must not warn; got: {:?}",
        result.diagnostics
    );
}

/// A rounded `process` shape walks the true rounded perimeter (no warning) and
/// differs from a sharp process of the same bounds for the same divided index.
#[test]
fn connector_divided_anchor_on_rounded_process_uses_true_perimeter() {
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
  shape id="b" kind="process" x=(px)300 y=(px)60 w=(px)100 h=(px)80 {extra}
  connector id="c1" from="a" to="b" to-anchor="1/6" stroke=(token)"color.line"
}}
  }}
}}
"##
        )
    };
    let sharp = compile(&parse(&base("")), &default_provider());
    let rounded = compile(&parse(&base("radius=(px)8")), &default_provider());
    assert!(
        !has_diag(&rounded, "connector.unsupported_outline"),
        "rounded process must not warn; got: {:?}",
        rounded.diagnostics
    );
    let sharp_pts = first_stroke_polyline_points(&sharp.scene.commands);
    let rounded_pts = first_stroke_polyline_points(&rounded.scene.commands);
    assert_ne!(
        sharp_pts[2..],
        rounded_pts[2..],
        "rounded process divided anchor must differ from sharp process"
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

    // Divided anchor on a plain rect + an ellipse: exact modeled outlines.
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
