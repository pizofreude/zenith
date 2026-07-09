use super::*;

// ── connector node: obstacle-avoiding routing (route="avoid") ─────────────────
//
// `route="avoid"` routes an orthogonal path that steers around every OTHER box
// in the document (boxes other than the connector's own from/to targets). The
// resulting polyline never passes through an obstacle's interior; when no clear
// path exists it degrades to the plain elbow. A connector with no obstacle in
// the way still produces a valid polyline from the from-anchor to the to-anchor.
/// Two boxes separated horizontally with a THIRD box sitting on the straight
/// line between them; `route="avoid"` must produce a polyline that detours
/// around the middle box's interior while still starting/ending at the anchors.
#[test]
fn connector_avoid_routes_around_obstacle() {
    let src = r##"zenith version=1 {
  project id="proj.av" name="AV"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.av" title="AV" {
page id="page.av" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)80 w=(px)80 h=(px)80 stroke=(token)"color.line"
  rect id="obs" x=(px)260 y=(px)60 w=(px)80 h=(px)120 stroke=(token)"color.line"
  rect id="b" x=(px)480 y=(px)80 w=(px)80 h=(px)80 stroke=(token)"color.line"
  connector id="c1" from="a" to="b" route="avoid" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    // a center (80,120) left of b center (520,120): auto picks a's right edge
    // (120,120) and b's left edge (480,120). The obstacle box spans x∈[260,340],
    // y∈[60,180] — squarely on the y=120 straight line.
    assert_eq!(pts[0], 120.0, "path starts at a's right-edge x");
    assert_eq!(pts[1], 120.0, "path starts at a's right-edge y");
    let n = pts.len();
    assert_eq!(pts[n - 2], 480.0, "path ends at b's left-edge x");
    assert_eq!(pts[n - 1], 120.0, "path ends at b's left-edge y");

    assert_polyline_misses_box(&pts, (260.0, 60.0, 80.0, 120.0));
}

/// `route="avoid"` with no obstacle between the two boxes still yields a valid
/// polyline running from the from-anchor to the to-anchor.
#[test]
fn connector_avoid_without_obstacle_routes_cleanly() {
    let src = r##"zenith version=1 {
  project id="proj.av" name="AV"
  tokens format="zenith-token-v1" {
token id="color.line" type="color" value="#1e3a8a"
  }
  styles {}
  document id="doc.av" title="AV" {
page id="page.av" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)80 w=(px)80 h=(px)80 stroke=(token)"color.line"
  rect id="b" x=(px)480 y=(px)80 w=(px)80 h=(px)80 stroke=(token)"color.line"
  connector id="c1" from="a" to="b" route="avoid" stroke=(token)"color.line"
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let pts = first_stroke_polyline_points(&result.scene.commands);

    assert!(
        pts.len() >= 4,
        "must have at least start and end; got {pts:?}"
    );
    // a right-edge (120,120) → b left-edge (480,120).
    assert_eq!(pts[0], 120.0);
    assert_eq!(pts[1], 120.0);
    let n = pts.len();
    assert_eq!(pts[n - 2], 480.0);
    assert_eq!(pts[n - 1], 120.0);
}
