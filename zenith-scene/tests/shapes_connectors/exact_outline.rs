use super::*;

// ── Exact divided anchors: polygon / polyline / path ─────────────────────────

/// Divided anchors on a triangle land on the true outline, not the AABB
/// midpoints. For this triangle the box `1/4` is (400, 100); the exact ring
/// sample is interior to the right edge.
#[test]
fn connector_divided_polygon_lands_on_true_outline_not_bbox() {
    // Triangle: (300,60)-(400,60)-(350,140). AABB = (300,60,100,80).
    // Box 1/4: top-mid + 90 along perimeter → (400, 100).
    // Exact ring 1/4 walks true edges from top-mid → differs from box.
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
  connector id="c1" from="a" to="p" to-anchor="1/4" stroke=(token)"color.line"
}
  }
}
"##;
    let result = compile(&parse(src), &default_provider());
    assert!(
        !has_diag(&result, "connector.unsupported_outline"),
        "exact polygon divided must not warn; got: {:?}",
        result.diagnostics
    );
    let pts = open_stroke_polyline_points(&result.scene.commands);
    let (ex, ey) = (pts[pts.len() - 2], pts[pts.len() - 1]);
    // Must not be the AABB attachment (400, 100).
    assert!(
        (ex - 400.0).abs() > 1.0 || (ey - 100.0).abs() > 1.0,
        "divided polygon must not land on AABB midpoint; got ({ex}, {ey})"
    );
    // Still on/near the right edge of the triangle (x between 350 and 400, y between 60 and 140).
    assert!(
        (350.0..=400.0).contains(&ex) && (60.0..=140.0).contains(&ey),
        "endpoint must lie on the triangle outline; got ({ex}, {ey})"
    );
}

/// Axis-aligned square polygon: `0/4`…`3/4` land on edge midpoints (true
/// outline coincides with AABB midpoints for a square).
#[test]
fn connector_divided_square_polygon_cardinals() {
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
  rect id="a" x=(px)40 y=(px)40 w=(px)20 h=(px)20
  polygon id="p" stroke=(token)"color.line" {{
    point x=(px)100 y=(px)100
    point x=(px)200 y=(px)100
    point x=(px)200 y=(px)200
    point x=(px)100 y=(px)200
  }}
  connector id="c1" from="a" to="p" to-anchor="{anchor}" stroke=(token)"color.line"
}}
  }}
}}
"##
        )
    };

    let expect = |anchor: &str, want: (f64, f64)| {
        let result = compile(&parse(&base(anchor)), &default_provider());
        assert!(
            !has_diag(&result, "connector.unsupported_outline"),
            "{anchor}: unexpected warn: {:?}",
            result.diagnostics
        );
        let pts = open_stroke_polyline_points(&result.scene.commands);
        let got = (pts[pts.len() - 2], pts[pts.len() - 1]);
        assert!(
            (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
            "{anchor}: expected {want:?}, got {got:?}"
        );
    };

    expect("0/4", (150.0, 100.0));
    expect("1/4", (200.0, 150.0));
    expect("2/4", (150.0, 200.0));
    expect("3/4", (100.0, 150.0));
}

/// Open polyline: `0/2` ≈ start, `1/2` ≈ end (inclusive open walk).
#[test]
fn connector_divided_open_polyline_start_and_end() {
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
  rect id="a" x=(px)40 y=(px)40 w=(px)20 h=(px)20
  polyline id="p" stroke=(token)"color.line" {{
    point x=(px)200 y=(px)100
    point x=(px)400 y=(px)100
  }}
  connector id="c1" from="a" to="p" to-anchor="{anchor}" stroke=(token)"color.line"
}}
  }}
}}
"##
        )
    };

    let start = compile(&parse(&base("0/2")), &default_provider());
    assert!(
        !has_diag(&start, "connector.unsupported_outline"),
        "polyline 0/2 must not warn; got: {:?}",
        start.diagnostics
    );
    // Polyline and connector are both open strokes; the connector is emitted last.
    let connector_pts = open_center_strokes(&start.scene.commands)
        .into_iter()
        .last()
        .expect("connector stroke");
    let (ex, ey) = (
        connector_pts[connector_pts.len() - 2],
        connector_pts[connector_pts.len() - 1],
    );
    assert!(
        (ex - 200.0).abs() < 1e-6 && (ey - 100.0).abs() < 1e-6,
        "0/2 must land at polyline start; got ({ex}, {ey})"
    );

    let end = compile(&parse(&base("1/2")), &default_provider());
    assert!(
        !has_diag(&end, "connector.unsupported_outline"),
        "polyline 1/2 must not warn; got: {:?}",
        end.diagnostics
    );
    let connector_pts = open_center_strokes(&end.scene.commands)
        .into_iter()
        .last()
        .expect("connector stroke");
    let (ex, ey) = (
        connector_pts[connector_pts.len() - 2],
        connector_pts[connector_pts.len() - 1],
    );
    assert!(
        (ex - 400.0).abs() < 1e-6 && (ey - 100.0).abs() < 1e-6,
        "1/2 must land at polyline end; got ({ex}, {ey})"
    );
}

/// Closed path square: `0/4`…`3/4` near cardinal midpoints of the true outline.
#[test]
fn connector_divided_closed_path_cardinals() {
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
  rect id="a" x=(px)40 y=(px)40 w=(px)20 h=(px)20
  path id="p" closed=#true stroke=(token)"color.line" {{
    anchor x=(px)100 y=(px)100
    anchor x=(px)200 y=(px)100
    anchor x=(px)200 y=(px)200
    anchor x=(px)100 y=(px)200
  }}
  connector id="c1" from="a" to="p" to-anchor="{anchor}" stroke=(token)"color.line"
}}
  }}
}}
"##
        )
    };

    let expect = |anchor: &str, want: (f64, f64)| {
        let result = compile(&parse(&base(anchor)), &default_provider());
        assert!(
            !has_diag(&result, "connector.unsupported_outline"),
            "{anchor}: unexpected warn: {:?}",
            result.diagnostics
        );
        // Path stroke is StrokePath, not StrokePolyline — connector is the open polyline.
        let pts = open_stroke_polyline_points(&result.scene.commands);
        let got = (pts[pts.len() - 2], pts[pts.len() - 1]);
        assert!(
            (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
            "{anchor}: expected {want:?}, got {got:?}"
        );
    };

    expect("0/4", (150.0, 100.0));
    expect("1/4", (200.0, 150.0));
    expect("2/4", (150.0, 200.0));
    expect("3/4", (100.0, 150.0));
}

/// Named anchors on path / polygon remain bounds-based (no silent change).
#[test]
fn connector_named_anchors_on_path_and_polygon_stay_bounds_based() {
    // Polygon "left" → left-mid of AABB (300, 100) — unchanged from pre-exact.
    let poly = r##"zenith version=1 {
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
    let result = compile(&parse(poly), &default_provider());
    let pts = open_stroke_polyline_points(&result.scene.commands);
    assert_eq!(
        (pts[pts.len() - 2], pts[pts.len() - 1]),
        (300.0, 100.0),
        "named left on polygon must stay AABB left-mid"
    );

    // Path "top" with cubic bulge — extrema-aware bounds top (350, 40), unchanged.
    let path = r##"zenith version=1 {
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
    let result = compile(&parse(path), &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);
    assert_eq!(
        (pts[pts.len() - 2], pts[pts.len() - 1]),
        (350.0, 40.0),
        "named top on path must stay extrema-aware bounds top"
    );
}
