use super::*;

// ── connector.anchor_unresolved (Error) ───────────────────────────────────────
/// A connector whose `to` endpoint names an unknown node no longer drops
/// silently: it emits `connector.anchor_unresolved` (naming the failing
/// endpoint) and still skips the render.
#[test]
fn connector_unresolved_endpoint_emits_diagnostic_and_skips() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  connector id="c1" from="a" to="ghost" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    assert!(
        has_diag(&result, "connector.anchor_unresolved"),
        "unresolvable endpoint must emit connector.anchor_unresolved; got: {:?}",
        result.diagnostics
    );
    let unresolved = result
        .diagnostics
        .iter()
        .find(|d| d.code == "connector.anchor_unresolved")
        .expect("diagnostic present");
    assert!(
        unresolved.message.contains("to") && unresolved.message.contains("ghost"),
        "diagnostic must name the failing endpoint; got: {}",
        unresolved.message
    );
    // Still skipped: no StrokePolyline for the connector.
    assert!(
        !result
            .scene
            .commands
            .iter()
            .any(|c| matches!(c, SceneCommand::StrokePolyline { .. })),
        "an unresolved connector must still be skipped"
    );
}

/// A connector to a polygon now ATTACHES at the polygon's bounds perimeter (via
/// the connector-scoped outline-box fallback) instead of erroring: NO
/// `connector.anchor_unresolved`, a stroke is emitted, and the routed endpoint
/// lands on the polygon's bounding-box perimeter.
#[test]
fn connector_to_polygon_attaches_on_bounds_perimeter() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  polygon id="p" stroke=(token)"color.line" {
    point x=(px)300 y=(px)60
    point x=(px)400 y=(px)60
    point x=(px)350 y=(px)140
  }
  connector id="c1" from="a" to="p" to-anchor="left" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    assert!(
        !has_diag(&result, "connector.anchor_unresolved"),
        "connector to a polygon must NOT be unresolved; got: {:?}",
        result.diagnostics
    );
    // polygon points bound x∈[300,400], y∈[60,140] → box (300, 60, 100, 80).
    // to-anchor "left" = left-mid = (min_x, center_y) = (300, 100), which lies on
    // the polygon's bounding-box perimeter (x == min_x). The polygon emits its
    // own (closed) StrokePolyline first; the connector's is the OPEN one.
    let pts = open_stroke_polyline_points(&result.scene.commands);
    let (ex, ey) = (pts[pts.len() - 2], pts[pts.len() - 1]);
    assert_eq!(
        (ex, ey),
        (300.0, 100.0),
        "connector endpoint must land on the polygon bounds perimeter; got {pts:?}"
    );
}

/// A DIVIDED (`i/N`) anchor on a polygon target no longer warns
/// `connector.unsupported_outline` (exact closed-ring sampling); a NAMED anchor
/// on the same polygon also does not warn (bounds semantics, unchanged).
#[test]
fn connector_divided_anchor_on_polygon_no_longer_warns_unsupported_outline() {
    let base = |anchor: &str| {
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
  polygon id="p" stroke=(token)"color.line" {{
    point x=(px)300 y=(px)60
    point x=(px)400 y=(px)60
    point x=(px)350 y=(px)140
  }}
  connector id="c1" from="a" to="p" to-anchor="{anchor}" stroke=(token)"color.line"
}}
  }}
}}
"##
        )
    };

    let divided = compile(&parse(&base("1/4")), &default_provider());
    assert!(
        !has_diag(&divided, "connector.unsupported_outline"),
        "a divided anchor on a polygon must not warn (exact outline); got: {:?}",
        divided.diagnostics
    );

    let named = compile(&parse(&base("left")), &default_provider());
    assert!(
        !has_diag(&named, "connector.unsupported_outline"),
        "a named anchor on a polygon must not warn; got: {:?}",
        named.diagnostics
    );
}

/// A connector to a `path` attaches using EXTREMA-AWARE bounds: the cubic's
/// control handles bulge the curve above its anchor points, so the bounds
/// reflect the true curve extent, not the anchor hull. The `top` anchor lands at
/// the curve's top extremum, well above the anchors' y.
#[test]
fn connector_to_path_uses_extrema_aware_bounds() {
    // Two anchors at y=100 with symmetric handles at y=20:
    //   P0=(300,100) P1=(300,20) P2=(400,20) P3=(400,100)
    // The cubic's minimum y is at t=0.5 → y=40, so bounds = (300, 40, 100, 60).
    // An anchor-hull box would be (300, 100, 100, 0); "top" would then be at
    // y=100. Extrema-aware "top" is (350, 40).
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  path id="p" stroke=(token)"color.line" {
    anchor x=(px)300 y=(px)100 out-x=(px)300 out-y=(px)20
    anchor x=(px)400 y=(px)100 in-x=(px)400 in-y=(px)20
  }
  connector id="c1" from="a" to="p" to-anchor="top" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    assert!(
        !has_diag(&result, "connector.anchor_unresolved"),
        "connector to a path must NOT be unresolved; got: {:?}",
        result.diagnostics
    );
    let pts = first_stroke_polyline_points(&result.scene.commands);
    let (ex, ey) = (pts[pts.len() - 2], pts[pts.len() - 1]);
    assert_eq!(
        (ex, ey),
        (350.0, 40.0),
        "path bounds must be extrema-aware (top at the curve extremum y=40, not \
         the anchor-hull y=100); got {pts:?}"
    );
}

/// A geometry-less target still errors: a connector to a `light` (no box, no
/// outline) emits `connector.anchor_unresolved` at Error.
#[test]
fn connector_to_light_is_unresolved() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  light id="p" kind="point" x=(px)300 y=(px)100
  connector id="c1" from="a" to="p" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    assert!(
        has_diag(&result, "connector.anchor_unresolved"),
        "connector to a geometry-less light must be unresolved; got: {:?}",
        result.diagnostics
    );
}
