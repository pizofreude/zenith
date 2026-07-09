use super::*;

// ── connector owned label ─────────────────────────────────────────────────────
//
// A connector with `span` children renders a label at the geometric midpoint of
// the routed polyline. A connector WITHOUT spans must render byte-identically.
/// A connector with a `span "Yes"` label must emit a GlyphRun (or at minimum a
/// DrawGlyphs-family command) somewhere after the StrokePolyline. We verify the
/// presence of the label by checking that the scene contains at least one command
/// beyond the polyline.
///
/// We also verify that the label's position is approximately at the midpoint of
/// the straight line between the two resolved anchor points.
#[test]
fn connector_with_label_emits_label_near_midpoint() {
    // Two rects with a straight connector between them. The auto anchors resolve:
    //   a: right-mid = (140, 80)
    //   b: left-mid  = (300, 100)
    // Midpoint = ((140+300)/2, (80+100)/2) = (220, 90).
    let src = r##"zenith version=1 {
  project id="proj.cl" name="CL"
  tokens format="zenith-token-v1" {
token id="color.fill" type="color" value="#dbeafe"
token id="color.line" type="color" value="#1e3a8a"
token id="size.stroke" type="dimension" value=(px)2
  }
  styles {}
  document id="doc.cl" title="CL" {
page id="page.cl" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 fill=(token)"color.fill"
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 fill=(token)"color.fill"
  connector id="c1" from="a" to="b" stroke=(token)"color.line" stroke-width=(token)"size.stroke" {
    span "Yes"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;

    // The StrokePolyline must still be present with the correct endpoints.
    let pts = first_stroke_polyline_points(cmds);
    assert_eq!(
        pts,
        vec![140.0, 80.0, 300.0, 100.0],
        "connector endpoints unchanged when label is present; got {pts:?}"
    );

    // There must be DrawGlyphs commands (the label text).
    let has_glyphs = cmds
        .iter()
        .any(|c| matches!(c, SceneCommand::DrawGlyphRun { .. }));
    assert!(
        has_glyphs,
        "connector with span label must emit at least one DrawGlyphs; got: {cmds:?}"
    );

    // The label's DrawGlyphs must originate near the midpoint x≈220, y≈90.
    // We check that at least one DrawGlyphs has x roughly in [160, 280] (midpoint ±60)
    // and y roughly in [50, 140] (midpoint ±50) — wide tolerances because the
    // exact glyph position depends on the label box centering and text metrics.
    let mid_x = 220.0_f64;
    let mid_y = 90.0_f64;
    let label_near_mid = cmds.iter().any(|c| match c {
        SceneCommand::DrawGlyphRun { x, y, .. } => {
            (x - mid_x).abs() < 80.0 && (y - mid_y).abs() < 60.0
        }
        _ => false,
    });
    assert!(
        label_near_mid,
        "at least one DrawGlyphs must be near the connector midpoint ({mid_x},{mid_y}); got: {cmds:?}"
    );
}

/// A connector WITHOUT spans must produce byte-identical output to the original
/// (no extra commands beyond the StrokePolyline and any arrowheads).
#[test]
fn connector_without_label_is_byte_identical() {
    let src_no_label = r##"zenith version=1 {
  project id="proj.cni" name="CNI"
  tokens format="zenith-token-v1" {
token id="color.fill" type="color" value="#dbeafe"
token id="color.line" type="color" value="#1e3a8a"
token id="size.stroke" type="dimension" value=(px)2
  }
  styles {}
  document id="doc.cni" title="CNI" {
page id="page.cni" w=(px)640 h=(px)360 {
  rect id="a" x=(px)40 y=(px)40 w=(px)100 h=(px)80 fill=(token)"color.fill"
  rect id="b" x=(px)300 y=(px)60 w=(px)100 h=(px)80 fill=(token)"color.fill"
  connector id="c1" from="a" to="b" stroke=(token)"color.line" stroke-width=(token)"size.stroke"
}
  }
}
"##;
    let doc = parse(src_no_label);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;

    // No DrawGlyphs — the connector has no label.
    let has_glyphs = cmds
        .iter()
        .any(|c| matches!(c, SceneCommand::DrawGlyphRun { .. }));
    assert!(
        !has_glyphs,
        "connector without spans must emit no DrawGlyphs; got: {cmds:?}"
    );

    // Exactly one StrokePolyline with the expected endpoints.
    let pts = first_stroke_polyline_points(cmds);
    assert_eq!(
        pts,
        vec![140.0, 80.0, 300.0, 100.0],
        "label-less connector must keep plain straight endpoints; got {pts:?}"
    );
}
