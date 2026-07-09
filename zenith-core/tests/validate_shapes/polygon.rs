use super::*;

// ── polygon: clean doc with token fill → no errors ────────────────────

#[test]
fn polygon_clean_no_errors() {
    let doc = doc_with(
        vec![
            color_token("color.fill"),
            color_token("color.stroke"),
            dim_token("size.stroke"),
        ],
        vec![minimal_page(
            "page.one",
            vec![Node::Polygon(PolygonNode {
                id: "poly.tri".to_owned(),
                name: None,
                role: None,
                fill: Some(token_ref("color.fill")),
                stroke: Some(token_ref("color.stroke")),
                stroke_width: Some(token_ref("size.stroke")),
                stroke_alignment: None,
                fill_rule: None,
                opacity: None,
                visible: None,
                locked: None,
                rotate: None,
                style: None,
                points: tri_points(),
                source_span: None,
                unknown_props: BTreeMap::new(),
            })],
        )],
    );
    let report = validate(&doc);
    assert!(
        report.diagnostics.is_empty(),
        "expected no diagnostics for clean polygon, got: {:?}",
        codes(&report)
    );
    assert!(!report.has_errors());
}

// ── polygon: only 2 points → shape.insufficient_points (Error) ───────

#[test]
fn polygon_too_few_points_insufficient() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![Node::Polygon(PolygonNode {
                id: "poly.bad".to_owned(),
                name: None,
                role: None,
                fill: None,
                stroke: None,
                stroke_width: None,
                stroke_alignment: None,
                fill_rule: None,
                opacity: None,
                visible: None,
                locked: None,
                rotate: None,
                style: None,
                points: vec![
                    Point {
                        x: Some(px(0.0)),
                        y: Some(px(0.0)),
                    },
                    Point {
                        x: Some(px(100.0)),
                        y: Some(px(0.0)),
                    },
                ],
                source_span: None,
                unknown_props: BTreeMap::new(),
            })],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "shape.insufficient_points"),
        "codes: {:?}",
        codes(&report)
    );
    assert!(report.has_errors());
}

// ── polyline: only 1 point → shape.insufficient_points (Error) ───────

#[test]
fn polyline_too_few_points_insufficient() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![Node::Polyline(PolylineNode {
                id: "line.bad".to_owned(),
                name: None,
                role: None,
                fill: None,
                stroke: None,
                stroke_width: None,
                fill_rule: None,
                opacity: None,
                visible: None,
                locked: None,
                rotate: None,
                style: None,
                points: vec![Point {
                    x: Some(px(0.0)),
                    y: Some(px(0.0)),
                }],
                source_span: None,
                unknown_props: BTreeMap::new(),
            })],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "shape.insufficient_points"),
        "codes: {:?}",
        codes(&report)
    );
    assert!(report.has_errors());
}

// ── shape: unknown kind → shape.unknown_kind (Warning) ────────────────

#[test]
fn shape_invalid_kind_warns() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![minimal_shape("s.bad", Some("bogus"), None)],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "shape.unknown_kind"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn shape_valid_kind_does_not_warn() {
    for kind in ["process", "decision", "terminator", "ellipse"] {
        let doc = doc_with(
            vec![],
            vec![minimal_page(
                "page.one",
                vec![minimal_shape("s.ok", Some(kind), None)],
            )],
        );
        let report = validate(&doc);
        assert!(
            !has_code(&report, "shape.unknown_kind"),
            "kind {kind:?} must not warn; codes: {:?}",
            codes(&report)
        );
    }
}

// ── shape: invalid h-align → shape.invalid_h_align (Warning) ───────────

#[test]
fn shape_invalid_h_align_warns() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![minimal_shape("s.bad", Some("process"), Some("sideways"))],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "shape.invalid_h_align"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn shape_valid_h_align_does_not_warn() {
    for h in ["start", "center", "end"] {
        let doc = doc_with(
            vec![],
            vec![minimal_page(
                "page.one",
                vec![minimal_shape("s.ok", Some("process"), Some(h))],
            )],
        );
        let report = validate(&doc);
        assert!(
            !has_code(&report, "shape.invalid_h_align"),
            "h-align {h:?} must not warn; codes: {:?}",
            codes(&report)
        );
    }
}
