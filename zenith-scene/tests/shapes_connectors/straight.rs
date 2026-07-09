use super::*;

// ── connector node (U1): straight line between resolved edge anchors ──────────
//
// A `connector` declares `from`/`to` target ids and, at compile time, resolves
// those nodes' boxes to draw a STRAIGHT 2-point line between anchor points on
// their edges. U1 = straight line, no arrowhead markers, no orthogonal routing.
/// Two rects laid out horizontally with a connector between them, default
/// (auto) anchors. The from-box center is left of the to-box center, so auto
/// picks the right edge of `a` and the left edge of `b`.
#[test]
fn connector_auto_anchors_between_horizontal_boxes() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.fill" type="color" value="#dbeafe"
token id="color.line" type="color" value="#1e3a8a"
token id="size.stroke" type="dimension" value=(px)2
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 fill=(token)"color.fill"
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 fill=(token)"color.fill"
  connector id="c1" from="a" to="b" stroke=(token)"color.line" stroke-width=(token)"size.stroke"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;

    // a: x=40 y=40 w=100 h=80  → center (90,80),  right-mid edge = (140, 80)
    // b: x=300 y=60 w=100 h=80 → center (350,100), left-mid edge = (300, 100)
    let pts = first_stroke_polyline_points(cmds);
    assert_eq!(
        pts,
        vec![140.0, 80.0, 300.0, 100.0],
        "auto anchors must be a's right-mid and b's left-mid; got {pts:?}"
    );

    // U1: straight 2-point open line, centered stroke.
    assert!(
        cmds.iter().any(|c| matches!(
            c,
            SceneCommand::StrokePolyline { points, closed: false, align: StrokeAlign::Center, .. }
                if points.len() == 4
        )),
        "connector must emit a straight open StrokePolyline; got {cmds:?}"
    );
}

/// Explicit `from-anchor="right"` / `to-anchor="left"` anchors are honored
/// verbatim (no auto resolution).
#[test]
fn connector_explicit_anchors_are_honored() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 stroke=(token)"color.line"
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 stroke=(token)"color.line"
  connector id="c1" from="a" to="b" from-anchor="right" to-anchor="left" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;

    // a right-mid = (140, 80); b left-mid = (300, 100).
    let pts = first_stroke_polyline_points(cmds);
    assert_eq!(pts, vec![140.0, 80.0, 300.0, 100.0]);
}

/// Nine-point grid anchors resolve to box corners: `from-anchor="bottom-right"`
/// / `to-anchor="top-left"` attach at those exact corners.
#[test]
fn connector_nine_point_corner_anchors() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 stroke=(token)"color.line"
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 stroke=(token)"color.line"
  connector id="c1" from="a" to="b" from-anchor="bottom-right" to-anchor="top-left" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;
    // a bottom-right = (140, 120); b top-left = (300, 60).
    let pts = first_stroke_polyline_points(cmds);
    assert_eq!(pts, vec![140.0, 120.0, 300.0, 60.0]);
}

/// `mid` is a synonym for `center`, and a bare edge name is that edge's
/// mid-point: `from-anchor="mid-right"` = right-mid, `to-anchor="top"` =
/// top-center.
#[test]
fn connector_anchor_synonyms_and_edge_midpoints() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 stroke=(token)"color.line"
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 stroke=(token)"color.line"
  connector id="c1" from="a" to="b" from-anchor="mid-right" to-anchor="top" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;
    // a mid-right = (140, 80); b top-center = (350, 60).
    let pts = first_stroke_polyline_points(cmds);
    assert_eq!(pts, vec![140.0, 80.0, 350.0, 60.0]);
}

#[test]
fn connector_divided_box_anchors_walk_perimeter() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="a" to="b" from-anchor="0/4" to-anchor="1/6" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    assert_eq!(pts, vec![90.0, 40.0, 400.0, 70.0]);
}

#[test]
fn connector_divided_ellipse_anchor_uses_ellipse_perimeter() {
    let src = r##"zenith version=1 {
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
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    assert_eq!(pts, vec![90.0, 40.0, 400.0, 100.0]);
}

#[test]
fn connector_divided_decision_anchor_uses_diamond_perimeter() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  shape id="b" kind="decision" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="a" to="b" from-anchor="0/4" to-anchor="1/8" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    assert_eq!(pts, vec![90.0, 40.0, 375.0, 80.0]);
}

#[test]
fn connector_divided_terminator_anchor_uses_capsule_perimeter() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  shape id="b" kind="terminator" x=(px)300 y=(px)60 w=(px)140 h=(px)80
  connector id="c1" from="a" to="b" from-anchor="0/4" to-anchor="1/4" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    assert_eq!(pts, vec![90.0, 40.0, 440.0, 100.0]);
}

#[test]
fn connector_port_endpoints_resolve_to_declared_anchors() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  ports {
    port node="agent" id="out" anchor="1/4"
    port node="store" id="in" anchor="3/4"
  }
  rect id="agent" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  rect id="store" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="agent#out" to="store#in" from-anchor="left" to-anchor="right" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    assert_eq!(pts, vec![140.0, 80.0, 300.0, 100.0]);
}

#[test]
fn connector_component_port_projects_through_instance() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  components {
    component id="agent.card" {
      ports {
        port node="body" id="out" anchor="1/4"
      }
      rect id="body" x=(px)0 y=(px)0 w=(px)100 h=(px)80
    }
  }
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  instance id="agent" component="agent.card" x=(px)40 y=(px)40
  rect id="store" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="agent#out" to="store" to-anchor="left" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    assert_eq!(pts, vec![140.0, 80.0, 300.0, 100.0]);
}

/// A connector to a MISSING target emits no StrokePolyline (graceful skip).
#[test]
fn connector_missing_target_emits_nothing() {
    let src = r##"zenith version=1 {
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.cn" title="CN" {
page id="page.cn" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 stroke=(token)"color.line"
  connector id="c1" from="a" to="ghost" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;
    assert!(
        !cmds
            .iter()
            .any(|c| matches!(c, SceneCommand::StrokePolyline { .. })),
        "connector to a missing target must emit no StrokePolyline; got {cmds:?}"
    );
}

/// A connector reroutes when its target box changes: compiling two documents
/// that differ only in the `to` rect's position yields different endpoints.
#[test]
fn connector_reroutes_when_target_moves() {
    let doc_src = |to_x: u32| {
        format!(
            r##"zenith version=1 {{
  project id="proj.cn" name="CN"
  tokens format="zenith-token-v1" {{
token id="color.line" type="color" value="#1e3a8a"
  }}
  styles {{}}
  document id="doc.cn" title="CN" {{
page id="page.cn" w=(px)640 h=(px)360 {{
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 stroke=(token)"color.line"
  rect id="b" x=(px){to_x} y=(px)60 w=(px)100 h=(px)80 stroke=(token)"color.line"
  connector id="c1" from="a" to="b" from-anchor="right" to-anchor="left" stroke=(token)"color.line"
}}
  }}
}}
"##
        )
    };

    let r1 = compile(&parse(&doc_src(300)), &default_provider());
    let r2 = compile(&parse(&doc_src(420)), &default_provider());

    let p1 = first_stroke_polyline_points(&r1.scene.commands);
    let p2 = first_stroke_polyline_points(&r2.scene.commands);

    // The `to` (left-edge) endpoint must move with the target rect.
    assert_eq!(p1[2], 300.0, "first layout: b left-mid x = 300");
    assert_eq!(p2[2], 420.0, "moved layout: b left-mid x = 420");
    assert_ne!(
        p1, p2,
        "connector endpoints must change when the target moves"
    );
}
