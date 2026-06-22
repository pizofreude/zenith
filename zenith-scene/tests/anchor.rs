//! Integration tests for G-69 units A-1 (page-relative) and A-2 (safe-zone-relative) anchors.
//!
//! An `anchor` attribute on a node derives its missing `x` and/or `y` from the
//! page dimensions. Explicitly-authored `x`/`y` always win over the anchor-
//! derived value. Unrecognized anchor values produce `anchor.unknown_value` from
//! the validator. A node with a recognized anchor and present `w`/`h` must NOT
//! receive `scene.missing_geometry` for its `x`/`y`.

mod common;
use common::*;
use zenith_core::default_provider;
use zenith_scene::compile;

// ── Shared document wrapper ───────────────────────────────────────────────────

/// Wrap a single page child (a raw KDL line) in a minimal document on a
/// 400×300 page.
fn doc_with_node(node_kdl: &str) -> String {
    format!(
        r#"zenith version=1 {{
  project id="proj.anc" name="Anchor"
  tokens format="zenith-token-v1" {{}}
  styles {{}}
  document id="doc.anc" title="Anchor" {{
page id="page.anc" w=(px)400 h=(px)300 {{
  {node_kdl}
}}
  }}
}}"#
    )
}

// ── Test 1: bottom-right anchor places rect at (page_w-w, page_h-h) ─────────

#[test]
fn anchor_bottom_right_rect() {
    // Page 400×300, rect 100×50 with anchor="bottom-right":
    //   x = 400 - 100 = 300,  y = 300 - 50 = 250
    let src = doc_with_node(
        r##"rect id="r.br" anchor="bottom-right" w=(px)100 h=(px)50 fill="#ff0000""##,
    );
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );

    let rects = fill_rects(&result);
    // PushClip for page is index 0; first real node FillRect follows.
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            (x - 300.0).abs() < 0.001
                && (y - 250.0).abs() < 0.001
                && (w - 100.0).abs() < 0.001
                && (h - 50.0).abs() < 0.001
        }),
        "expected FillRect at (300, 250, 100, 50) for bottom-right anchor; got: {rects:?}"
    );
}

// ── Test 2: center anchor places rect at ((pw-w)/2, (ph-h)/2) ───────────────

#[test]
fn anchor_center_rect() {
    // Page 400×300, rect 200×100 with anchor="center":
    //   x = (400-200)/2 = 100,  y = (300-100)/2 = 100
    let src =
        doc_with_node(r##"rect id="r.ctr" anchor="center" w=(px)200 h=(px)100 fill="#00ff00""##);
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );

    let rects = fill_rects(&result);
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            (x - 100.0).abs() < 0.001
                && (y - 100.0).abs() < 0.001
                && (w - 200.0).abs() < 0.001
                && (h - 100.0).abs() < 0.001
        }),
        "expected FillRect at (100, 100, 200, 100) for center anchor; got: {rects:?}"
    );
}

// ── Test 3: explicit y wins over anchor ──────────────────────────────────────

#[test]
fn anchor_explicit_y_wins() {
    // Page 400×300, rect 100×50 with anchor="bottom-right" but y=(px)0:
    //   x is derived: 400-100 = 300
    //   y is explicit: 0
    let src = doc_with_node(
        r##"rect id="r.yw" anchor="bottom-right" w=(px)100 h=(px)50 y=(px)0 fill="#0000ff""##,
    );
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );

    let rects = fill_rects(&result);
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            (x - 300.0).abs() < 0.001
                && (y - 0.0).abs() < 0.001
                && (w - 100.0).abs() < 0.001
                && (h - 50.0).abs() < 0.001
        }),
        "expected FillRect at (300, 0, 100, 50): x from anchor, y explicit; got: {rects:?}"
    );
}

// ── Test 4: explicit x wins over anchor ──────────────────────────────────────

#[test]
fn anchor_explicit_x_wins() {
    // Page 400×300, rect 100×50 with anchor="bottom-right" but x=(px)0:
    //   x is explicit: 0
    //   y is derived: 300-50 = 250
    let src = doc_with_node(
        r##"rect id="r.xw" anchor="bottom-right" w=(px)100 h=(px)50 x=(px)0 fill="#00ffff""##,
    );
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );

    let rects = fill_rects(&result);
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            (x - 0.0).abs() < 0.001
                && (y - 250.0).abs() < 0.001
                && (w - 100.0).abs() < 0.001
                && (h - 50.0).abs() < 0.001
        }),
        "expected FillRect at (0, 250, 100, 50): x explicit, y from anchor; got: {rects:?}"
    );
}

// ── Test 5: no anchor → byte-identical to authored x/y ───────────────────────

#[test]
fn no_anchor_byte_identical() {
    // Without anchor, the node must still compile normally when x/y are explicit.
    let with_anchor =
        doc_with_node(r##"rect id="r.na" anchor="top-left" w=(px)80 h=(px)60 fill="#123456""##);
    let without_anchor =
        doc_with_node(r##"rect id="r.na" x=(px)0 y=(px)0 w=(px)80 h=(px)60 fill="#123456""##);

    let doc_a = parse(&with_anchor);
    let doc_b = parse(&without_anchor);
    let res_a = compile(&doc_a, &default_provider());
    let res_b = compile(&doc_b, &default_provider());

    assert!(
        res_a.diagnostics.is_empty(),
        "anchor=top-left should not produce diagnostics: {:?}",
        res_a.diagnostics
    );
    assert!(
        res_b.diagnostics.is_empty(),
        "explicit (0,0) should not produce diagnostics: {:?}",
        res_b.diagnostics
    );

    // Both should produce the same FillRect.
    let rects_a = fill_rects(&res_a);
    let rects_b = fill_rects(&res_b);
    assert_eq!(
        rects_a, rects_b,
        "anchor=top-left and explicit (0,0) must yield identical FillRect geometry"
    );
}

// ── Test 6: unrecognized anchor → anchor.unknown_value error ─────────────────

#[test]
fn anchor_unknown_value_error() {
    // The validator (not the compiler) produces anchor.unknown_value for unknown
    // anchor strings. Use the validate path via zenith_core.
    use zenith_core::{KdlAdapter, KdlSource};

    let src =
        doc_with_node(r##"rect id="r.bad" anchor="bogus" w=(px)100 h=(px)50 fill="#ff0000""##);
    let doc = KdlAdapter.parse(src.as_bytes()).expect("must parse");
    let report = zenith_core::validate(&doc);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "anchor.unknown_value"),
        "expected anchor.unknown_value diagnostic for anchor=\"bogus\"; got: {:?}",
        report.diagnostics
    );
}

// ── Test 7: recognized anchor suppresses missing_geometry for x/y ────────────

#[test]
fn anchor_recognized_suppresses_missing_geometry() {
    // anchor="top-left" with w/h but no x/y: compile must NOT emit
    // scene.missing_geometry (the anchor derives x=0, y=0).
    let src =
        doc_with_node(r##"rect id="r.tl" anchor="top-left" w=(px)80 h=(px)60 fill="#abcdef""##);
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == "scene.missing_geometry"),
        "anchor=top-left with w/h must not produce scene.missing_geometry; got: {:?}",
        result.diagnostics
    );

    // The rect must actually be rendered.
    let rects = fill_rects(&result);
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            x.abs() < 0.001
                && y.abs() < 0.001
                && (w - 80.0).abs() < 0.001
                && (h - 60.0).abs() < 0.001
        }),
        "expected FillRect at (0, 0, 80, 60) for top-left anchor; got: {rects:?}"
    );
}

// ── Test 8: all nine anchors on a 400×300 page, rect 40×30 ─────────────────

#[test]
fn all_nine_anchors_geometry() {
    // For a 400×300 page with a 40×30 rect:
    //   dx = (400-40)/2 = 180,  dy = (300-30)/2 = 135
    let cases: &[(&str, f64, f64)] = &[
        ("top-left", 0.0, 0.0),
        ("top-center", 180.0, 0.0),
        ("top-right", 360.0, 0.0),
        ("center-left", 0.0, 135.0),
        ("center", 180.0, 135.0),
        ("center-right", 360.0, 135.0),
        ("bottom-left", 0.0, 270.0),
        ("bottom-center", 180.0, 270.0),
        ("bottom-right", 360.0, 270.0),
    ];

    for &(anchor_name, exp_x, exp_y) in cases {
        let node_kdl = format!(
            r##"rect id="r.nine" anchor="{anchor_name}" w=(px)40 h=(px)30 fill="#ffffff""##
        );
        let src = doc_with_node(&node_kdl);
        let doc = parse(&src);
        let result = compile(&doc, &default_provider());
        assert!(
            result.diagnostics.is_empty(),
            "anchor=\"{anchor_name}\" produced diagnostics: {:?}",
            result.diagnostics
        );

        let rects = fill_rects(&result);
        assert!(
            rects.iter().any(|&(x, y, w, h)| {
                (x - exp_x).abs() < 0.001
                    && (y - exp_y).abs() < 0.001
                    && (w - 40.0).abs() < 0.001
                    && (h - 30.0).abs() < 0.001
            }),
            "anchor=\"{anchor_name}\": expected ({exp_x}, {exp_y}, 40, 30); got: {rects:?}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// G-69 unit A-2: safe-zone-relative anchors
// ═════════════════════════════════════════════════════════════════════════════

/// Wrap a single page child in a document that also declares one safe-zone.
///
/// Page is 400×300.  Safe-zone "sz.panel" is at (100, 50, 200, 100) —
/// i.e. x=100 y=50 w=200 h=100.
fn doc_with_zone_and_node(node_kdl: &str) -> String {
    format!(
        r#"zenith version=1 {{
  project id="proj.az" name="AnchorZone"
  tokens format="zenith-token-v1" {{}}
  styles {{}}
  document id="doc.az" title="AnchorZone" {{
page id="page.az" w=(px)400 h=(px)300 {{
  safe-zone id="sz.panel" type="required" x=(px)100 y=(px)50 w=(px)200 h=(px)100
  {node_kdl}
}}
  }}
}}"#
    )
}

// ── A-2 Test 1: bottom-right relative to zone ─────────────────────────────

#[test]
fn anchor_zone_bottom_right() {
    // Zone: x=100 y=50 w=200 h=100. Node: w=40 h=30. anchor="bottom-right".
    // Zone-relative:  ox = 200-40 = 160, oy = 100-30 = 70.
    // Absolute:       x  = 100+160 = 260, y = 50+70 = 120.
    let src = doc_with_zone_and_node(
        r##"rect id="r.zbr" anchor="bottom-right" anchor-zone="sz.panel" w=(px)40 h=(px)30 fill="#ff0000""##,
    );
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );

    let rects = fill_rects(&result);
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            (x - 260.0).abs() < 0.001
                && (y - 120.0).abs() < 0.001
                && (w - 40.0).abs() < 0.001
                && (h - 30.0).abs() < 0.001
        }),
        "expected FillRect at (260, 120, 40, 30) for zone bottom-right; got: {rects:?}"
    );
}

// ── A-2 Test 2: center within zone ───────────────────────────────────────────

#[test]
fn anchor_zone_center() {
    // Zone: x=100 y=50 w=200 h=100. Node: w=40 h=20. anchor="center".
    // Zone-relative:  ox = (200-40)/2 = 80, oy = (100-20)/2 = 40.
    // Absolute:       x  = 100+80 = 180, y = 50+40 = 90.
    let src = doc_with_zone_and_node(
        r##"rect id="r.zctr" anchor="center" anchor-zone="sz.panel" w=(px)40 h=(px)20 fill="#00ff00""##,
    );
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );

    let rects = fill_rects(&result);
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            (x - 180.0).abs() < 0.001
                && (y - 90.0).abs() < 0.001
                && (w - 40.0).abs() < 0.001
                && (h - 20.0).abs() < 0.001
        }),
        "expected FillRect at (180, 90, 40, 20) for zone center; got: {rects:?}"
    );
}

// ── A-2 Test 3: unresolved zone id produces anchor.unresolved_zone error ─────

#[test]
fn anchor_zone_unresolved() {
    use zenith_core::{KdlAdapter, KdlSource};

    // "sz.ghost" does not exist on the page.
    let src = doc_with_zone_and_node(
        r##"rect id="r.zghost" anchor="top-left" anchor-zone="sz.ghost" w=(px)40 h=(px)30 fill="#ff0000""##,
    );
    let doc = KdlAdapter.parse(src.as_bytes()).expect("must parse");
    let report = zenith_core::validate(&doc);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "anchor.unresolved_zone"),
        "expected anchor.unresolved_zone diagnostic; got: {:?}",
        report.diagnostics
    );
}

// ── A-2 Test 4: anchor-zone without anchor produces anchor.zone_without_anchor

#[test]
fn anchor_zone_without_anchor() {
    use zenith_core::{KdlAdapter, KdlSource};

    // anchor-zone is set but anchor is absent.
    let src = doc_with_zone_and_node(
        r##"rect id="r.znoanc" anchor-zone="sz.panel" x=(px)0 y=(px)0 w=(px)40 h=(px)30 fill="#ff0000""##,
    );
    let doc = KdlAdapter.parse(src.as_bytes()).expect("must parse");
    let report = zenith_core::validate(&doc);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "anchor.zone_without_anchor"),
        "expected anchor.zone_without_anchor warning; got: {:?}",
        report.diagnostics
    );
}

// ── A-2 Test 5: no zone → page-relative behaviour unchanged (regression) ─────

#[test]
fn anchor_no_zone_regression() {
    // Same page, anchor="bottom-right" without anchor-zone: page-relative.
    // Page 400×300, rect 40×30 → x=360, y=270.
    let src = doc_with_zone_and_node(
        r##"rect id="r.nozone" anchor="bottom-right" w=(px)40 h=(px)30 fill="#0000ff""##,
    );
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        result.diagnostics
    );

    let rects = fill_rects(&result);
    assert!(
        rects.iter().any(|&(x, y, w, h)| {
            (x - 360.0).abs() < 0.001
                && (y - 270.0).abs() < 0.001
                && (w - 40.0).abs() < 0.001
                && (h - 30.0).abs() < 0.001
        }),
        "expected FillRect at (360, 270, 40, 30) for page-relative bottom-right; got: {rects:?}"
    );
}
