//! Integration tests: contrast validation.
//!
//! Test bodies moved verbatim from the former in-`src` `validate/check/tests/`
//! concern files; only import paths changed (`crate::`/`super::common` ->
//! `zenith_core::`/`common`).

use std::collections::BTreeMap;

mod common;

use common::*;
use zenith_core::{GradientKind, GradientLiteral, GradientStopRef};

// ══════════════════════════════════════════════════════════════════════
// WCAG 3 (APCA) contrast advisory tests
// ══════════════════════════════════════════════════════════════════════

/// Build a dimension token in pt.
fn dim_token_pt(id: &str, value: f64) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Dimension,
        value: TokenValue::Literal(TokenLiteral::Dimension(Dimension {
            value,
            unit: Unit::Pt,
        })),
        set: None,
        source_span: None,
    }
}

/// Build a font-weight token.
fn fw_token(id: &str, weight: f64) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::FontWeight,
        value: TokenValue::Literal(TokenLiteral::Number(weight)),
        set: None,
        source_span: None,
    }
}

fn linear_gradient_token(id: &str, stops: Vec<(f64, &str)>) -> Token {
    Token {
        id: id.to_owned(),
        token_type: TokenType::Gradient,
        value: TokenValue::Literal(TokenLiteral::Gradient(GradientLiteral {
            kind: GradientKind::Linear,
            angle_deg: 0.0,
            center_x: None,
            center_y: None,
            radius: None,
            stops: stops
                .into_iter()
                .map(|(offset, color)| GradientStopRef {
                    offset,
                    color_token: color.to_owned(),
                })
                .collect(),
        })),
        set: None,
        source_span: None,
    }
}

/// Helper: build a page with a background color token reference.
fn page_with_bg(id: &str, bg_token_id: &str, children: Vec<Node>) -> Page {
    Page {
        id: id.to_owned(),
        name: None,
        source: None,
        fit: None,
        width: px(1280.0),
        height: px(720.0),
        background: Some(PropertyValue::TokenRef(bg_token_id.to_owned())),
        bleed: None,
        margin_inner: None,
        margin_outer: None,
        margin_top: None,
        margin_bottom: None,
        baseline_grid: None,
        line_jumps: None,
        parity: None,
        master: None,
        safe_zones: Vec::new(),
        folds: Vec::new(),
        construction: zenith_core::ConstructionBlock::default(),
        ports: Vec::new(),
        block_styles: Vec::new(),
        children,
        source_span: None,
    }
}

fn backdrop_image_asset(id: &str) -> AssetDecl {
    AssetDecl {
        id: id.to_owned(),
        kind: AssetKind::Image,
        src: "assets/backdrop.png".to_owned(),
        sha256: None,
        producer_kind: None,
        producer_source: None,
        ai_prompt: None,
        ai_model: None,
        ai_provider: None,
        ai_seed: None,
        ai_generation_date: None,
        ai_license: None,
        ai_source_rights: None,
        ai_safety_status: None,
        ai_reuse_policy: None,
        source_span: None,
        unknown_props: BTreeMap::new(),
    }
}

fn doc_with_backdrop_image(tokens: Vec<Token>, children: Vec<Node>) -> Document {
    let mut doc = doc_with(
        tokens,
        vec![page_with_bg("page.one", "color.page", children)],
    );
    doc.assets = AssetBlock {
        assets: vec![backdrop_image_asset("asset.backdrop")],
        source_span: None,
    };
    doc
}

/// Build a text node with explicit fill and optional font-size / font-weight.
fn text_with_fill_and_size(
    id: &str,
    fill_token: Option<&str>,
    font_size_token: Option<&str>,
    font_weight_token: Option<&str>,
) -> Node {
    Node::Text(Box::new(zenith_core::TextNode {
        shadow: None,
        filter: None,
        mask: None,
        id: id.to_owned(),
        name: None,
        role: None,
        x: Some(pxv(0.0)),
        y: Some(pxv(0.0)),
        w: Some(pxv(200.0)),
        h: Some(pxv(40.0)),
        align: None,
        v_align: None,
        direction: None,
        overflow: None,
        overflow_wrap: None,
        style: None,
        fill: fill_token.map(|t| PropertyValue::TokenRef(t.to_owned())),
        stroke: None,
        stroke_width: None,
        contrast_bg: None,
        font_family: None,
        font_size: font_size_token.map(|t| PropertyValue::TokenRef(t.to_owned())),
        font_size_min: None,
        font_weight: font_weight_token.map(|t| PropertyValue::TokenRef(t.to_owned())),
        font_features: None,
        font_alternates: None,
        letter_spacing: None,
        kerning_pairs: Vec::new(),
        opacity: None,
        visible: None,
        locked: None,
        selectable: None,
        rotate: None,
        blend_mode: None,
        blur: None,
        chain: None,
        drop_cap_lines: None,
        hyphenate: None,
        widow_orphan: None,
        tab_leader: None,
        text_exclusion: None,
        padding_left: None,
        text_indent: None,
        content_format: None,
        src: None,
        bullet: None,
        bullet_gap: None,
        anchor: None,
        anchor_zone: None,
        anchor_sibling: None,
        anchor_edge: None,
        anchor_gap: None,
        anchor_parent: None,
        spans: vec![],
        block_styles: Vec::new(),
        source_span: None,
        unknown_props: BTreeMap::new(),
    }))
}

/// Build a filled ellipse backdrop large enough for a centered text box.
fn ellipse_backdrop(id: &str, fill_token: &str) -> Node {
    Node::Ellipse(EllipseNode {
        shadow: None,
        filter: None,
        mask: None,
        id: id.to_owned(),
        name: None,
        role: None,
        x: Some(pxv(520.0)),
        y: Some(pxv(300.0)),
        w: Some(pxv(240.0)),
        h: Some(pxv(120.0)),
        rx: None,
        ry: None,
        style: None,
        fill: Some(PropertyValue::TokenRef(fill_token.to_owned())),
        stroke: None,
        stroke_width: None,
        stroke_dash: None,
        stroke_gap: None,
        stroke_linecap: None,
        opacity: None,
        visible: None,
        locked: None,
        rotate: None,
        blend_mode: None,
        blur: None,
        anchor: None,
        anchor_zone: None,
        anchor_sibling: None,
        anchor_edge: None,
        anchor_gap: None,
        anchor_parent: None,
        source_span: None,
        unknown_props: BTreeMap::new(),
    })
}

fn rect_backdrop_at(id: &str, fill_token: &str, x: f64, y: f64, w: f64, h: f64) -> Node {
    rect_backdrop_at_with_opacity(id, fill_token, x, y, w, h, None)
}

fn rect_backdrop_at_with_opacity(
    id: &str,
    fill_token: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    opacity: Option<f64>,
) -> Node {
    let Node::Rect(mut rect) =
        minimal_rect(id, Some(PropertyValue::TokenRef(fill_token.to_owned())))
    else {
        unreachable!("minimal_rect returns Node::Rect");
    };
    rect.x = Some(pxv(x));
    rect.y = Some(pxv(y));
    rect.w = Some(pxv(w));
    rect.h = Some(pxv(h));
    rect.opacity = opacity;
    Node::Rect(rect)
}

fn group_at(id: &str, x: f64, y: f64, children: Vec<Node>) -> Node {
    group_at_with_opacity(id, x, y, None, children)
}

fn group_at_with_opacity(
    id: &str,
    x: f64,
    y: f64,
    opacity: Option<f64>,
    children: Vec<Node>,
) -> Node {
    Node::Group(GroupNode {
        id: id.to_owned(),
        name: None,
        role: None,
        x: Some(pxv(x)),
        y: Some(pxv(y)),
        w: None,
        h: None,
        opacity,
        visible: None,
        locked: None,
        rotate: None,
        blend_mode: None,
        shadow: None,
        filter: None,
        mask: None,
        blur: None,
        style: None,
        semantic_role: None,
        intensity: None,
        layer_priority: None,
        symmetry_count: None,
        symmetry_cx: None,
        symmetry_cy: None,
        symmetry_start_angle: None,
        symmetry_mode: None,
        anchor: None,
        anchor_zone: None,
        anchor_sibling: None,
        anchor_edge: None,
        anchor_gap: None,
        anchor_parent: None,
        children,
        protected_regions: Vec::new(),
        editable_param_ids: Vec::new(),
        source_span: None,
        unknown_props: BTreeMap::new(),
    })
}

fn text_at(id: &str, fill_token: &str, x: f64, y: f64, w: f64, h: f64) -> Node {
    let Node::Text(mut text) =
        minimal_text(id, Some(PropertyValue::TokenRef(fill_token.to_owned())))
    else {
        unreachable!("minimal_text returns Node::Text");
    };
    text.x = Some(pxv(x));
    text.y = Some(pxv(y));
    text.w = Some(pxv(w));
    text.h = Some(pxv(h));
    Node::Text(text)
}

fn shape_backdrop_at(
    id: &str,
    kind: &str,
    fill_token: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Node {
    Node::Shape(Box::new(ShapeNode {
        id: id.to_owned(),
        name: None,
        role: None,
        x: Some(pxv(x)),
        y: Some(pxv(y)),
        w: Some(pxv(w)),
        h: Some(pxv(h)),
        kind: Some(kind.to_owned()),
        fill: Some(PropertyValue::TokenRef(fill_token.to_owned())),
        stroke: None,
        stroke_width: None,
        radius: None,
        stroke_alignment: None,
        padding: None,
        h_align: None,
        v_align: None,
        text_style: None,
        spans: Vec::new(),
        style: None,
        opacity: None,
        visible: None,
        locked: None,
        rotate: None,
        anchor: None,
        anchor_zone: None,
        anchor_sibling: None,
        anchor_edge: None,
        anchor_gap: None,
        anchor_parent: None,
        source_span: None,
        unknown_props: BTreeMap::new(),
    }))
}

fn image_backdrop_at(id: &str, x: f64, y: f64, w: f64, h: f64) -> Node {
    image_backdrop_at_with_opacity(id, x, y, w, h, None)
}

fn image_backdrop_at_with_opacity(
    id: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    opacity: Option<f64>,
) -> Node {
    Node::Image(ImageNode {
        shadow: None,
        filter: None,
        mask: None,
        id: id.to_owned(),
        name: None,
        role: None,
        asset: "asset.backdrop".to_owned(),
        x: Some(pxv(x)),
        y: Some(pxv(y)),
        w: Some(pxv(w)),
        h: Some(pxv(h)),
        src_x: None,
        src_y: None,
        src_w: None,
        src_h: None,
        fit: None,
        svg_stroke: None,
        svg_fill: None,
        svg_stroke_width: None,
        clip: None,
        clip_radius: None,
        object_position_x: None,
        object_position_y: None,
        opacity,
        visible: None,
        locked: None,
        rotate: None,
        blend_mode: None,
        blur: None,
        style: None,
        anchor: None,
        anchor_zone: None,
        anchor_sibling: None,
        anchor_edge: None,
        anchor_gap: None,
        anchor_parent: None,
        source_span: None,
        unknown_props: BTreeMap::new(),
    })
}

fn polygon_backdrop(id: &str, fill_token: &str, points: Vec<(f64, f64)>) -> Node {
    Node::Polygon(PolygonNode {
        id: id.to_owned(),
        name: None,
        role: None,
        fill: Some(PropertyValue::TokenRef(fill_token.to_owned())),
        stroke: None,
        stroke_width: None,
        stroke_alignment: None,
        fill_rule: None,
        opacity: None,
        visible: None,
        locked: None,
        rotate: None,
        style: None,
        points: points
            .into_iter()
            .map(|(x, y)| Point {
                x: Some(px(x)),
                y: Some(px(y)),
            })
            .collect(),
        source_span: None,
        unknown_props: BTreeMap::new(),
    })
}

fn polyline_backdrop(id: &str, fill_token: &str, points: Vec<(f64, f64)>) -> Node {
    Node::Polyline(PolylineNode {
        id: id.to_owned(),
        name: None,
        role: None,
        fill: Some(PropertyValue::TokenRef(fill_token.to_owned())),
        stroke: None,
        stroke_width: None,
        fill_rule: None,
        opacity: None,
        visible: None,
        locked: None,
        rotate: None,
        style: None,
        points: points
            .into_iter()
            .map(|(x, y)| Point {
                x: Some(px(x)),
                y: Some(px(y)),
            })
            .collect(),
        source_span: None,
        unknown_props: BTreeMap::new(),
    })
}

/// Build a text node with explicit dimensions and page-relative anchor.
fn anchored_text_with_fill_and_size(
    id: &str,
    fill_token: &str,
    font_size_token: &str,
    anchor: &str,
) -> Node {
    let Node::Text(mut text) =
        minimal_text(id, Some(PropertyValue::TokenRef(fill_token.to_owned())))
    else {
        unreachable!("minimal_text returns Node::Text");
    };
    text.font_size = Some(PropertyValue::TokenRef(font_size_token.to_owned()));
    text.x = None;
    text.y = None;
    text.w = Some(pxv(120.0));
    text.h = Some(pxv(40.0));
    text.anchor = Some(anchor.to_owned());
    Node::Text(text)
}

/// Light gray (#aaaaaa) text on white page at 16 px → APCA Lc ~46 < 60
/// → `contrast.low` warning.
#[test]
fn low_contrast_normal_text_warns() {
    let doc = doc_with(
        vec![
            color_token_hex("color.bg", "#ffffff"),
            color_token_hex("color.text", "#aaaaaa"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_size(
                "text.one",
                Some("color.text"),
                None,
                None,
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.low"),
        "light gray on white should warn contrast.low; codes: {:?}",
        codes(&report)
    );
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "contrast.low")
        .expect("must exist");
    assert_eq!(diag.severity, Severity::Warning);
    assert!(!report.has_errors(), "contrast.low must not be an error");
}

/// Same-color text and background should hit `contrast.invisible`, not the
/// ordinary low-contrast bucket.
#[test]
fn same_color_text_warns_invisible() {
    let doc = doc_with(
        vec![
            color_token_hex("color.bg", "#222222"),
            color_token_hex("color.text", "#222222"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_size(
                "text.one",
                Some("color.text"),
                None,
                None,
            )],
        )],
    );
    let report = validate(&doc);
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "contrast.invisible")
        .expect("same-color text should produce contrast.invisible");
    assert_eq!(diag.severity, Severity::Warning);
    assert!(
        !has_code(&report, "contrast.low"),
        "invisible text should not also emit contrast.low; codes: {:?}",
        codes(&report)
    );
}

/// Black (#000000) text on white page → APCA Lc ~106 → NO warning.
#[test]
fn high_contrast_text_no_warning() {
    let doc = doc_with(
        vec![
            color_token_hex("color.bg", "#ffffff"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_size(
                "text.one",
                Some("color.text"),
                None,
                None,
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.low"),
        "black on white must NOT warn contrast.low; codes: {:?}",
        codes(&report)
    );
}

/// Large text (20 pt ≈ 26.67 px, which is >= 24 px) with a mid-contrast
/// color (#777777, APCA Lc ~71 on white) clears the large-text minimum
/// (Lc 45) → NO warning.
///
/// Note: 20 pt × (4/3) = 26.67 px, which exceeds the 24 px large-text cut-off.
#[test]
fn large_text_passes_lower_threshold_no_warning() {
    let doc = doc_with(
        vec![
            color_token_hex("color.bg", "#ffffff"),
            color_token_hex("color.text", "#777777"), // APCA Lc ~71 on white — clears large min (45)
            dim_token_pt("size.large", 20.0),         // 20pt ≈ 26.67px >= 24px → large
        ],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_size(
                "text.one",
                Some("color.text"),
                Some("size.large"),
                None,
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.low"),
        "large text (#777 on white, Lc ~71) should pass the 45 large-text threshold; codes: {:?}",
        codes(&report)
    );
}

/// Small bold text (18 pt ≈ 24 px, which is exactly 24 px → large) with
/// mid-contrast (#777777, APCA Lc ~71 on white) → clears large min (45) → NO warning.
#[test]
fn bold_large_text_passes_lower_threshold() {
    let doc = doc_with(
        vec![
            color_token_hex("color.bg", "#ffffff"),
            color_token_hex("color.text", "#777777"),
            dim_token_pt("size.18pt", 18.0), // 18pt ≈ 24px → exactly at large boundary
            fw_token("weight.bold", 700.0),
        ],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_size(
                "text.one",
                Some("color.text"),
                Some("size.18pt"),
                Some("weight.bold"),
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.low"),
        "18pt bold (large text, Lc ~71) should clear the 45 large-text threshold; codes: {:?}",
        codes(&report)
    );
}

/// Center-anchored text with explicit dimensions should derive its page-relative
/// bbox before contrast backdrop detection, so a preceding filled ellipse wins
/// over the page background.
#[test]
fn centered_anchor_text_uses_preceding_ellipse_backdrop() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
            dim_token_pt("size.small", 9.0),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                ellipse_backdrop("backdrop", "color.backdrop"),
                anchored_text_with_fill_and_size("headline", "color.text", "size.small", "center"),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "black centered-anchor text over the navy ellipse should warn contrast.invisible via the backdrop; codes: {:?}",
        codes(&report)
    );
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "contrast.invisible")
        .expect("must exist");
    assert!(
        diag.message.contains("backdrop"),
        "message must name the ellipse backdrop as the bg source; got: {}",
        diag.message
    );
}

#[test]
fn grouped_text_uses_outer_page_backdrop() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                rect_backdrop_at("backdrop", "color.backdrop", 100.0, 100.0, 220.0, 100.0),
                group_at(
                    "group.label",
                    0.0,
                    0.0,
                    vec![text_at("headline", "color.text", 130.0, 130.0, 80.0, 30.0)],
                ),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "grouped text should use the earlier page-level backdrop; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn page_text_uses_backdrop_inside_translated_group() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                group_at(
                    "group.backdrop",
                    100.0,
                    100.0,
                    vec![rect_backdrop_at(
                        "backdrop",
                        "color.backdrop",
                        0.0,
                        0.0,
                        220.0,
                        100.0,
                    )],
                ),
                text_at("headline", "color.text", 130.0, 130.0, 80.0, 30.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "page text should use the absolute backdrop from the earlier translated group; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn decision_shape_can_be_text_backdrop() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                shape_backdrop_at(
                    "decision",
                    "decision",
                    "color.backdrop",
                    100.0,
                    100.0,
                    220.0,
                    140.0,
                ),
                text_at("headline", "color.text", 190.0, 155.0, 40.0, 20.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "text inside the decision shape interior should use the shape backdrop; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn text_straddling_backdrop_uses_worst_sample() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                rect_backdrop_at("backdrop", "color.backdrop", 100.0, 100.0, 120.0, 80.0),
                text_at("headline", "color.text", 180.0, 120.0, 100.0, 30.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "text partly over the dark backdrop should use the worst sampled backdrop; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn gradient_backdrop_uses_worst_stop() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.light", "#ffffff"),
            color_token_hex("color.dark", "#003087"),
            color_token_hex("color.text", "#000000"),
            linear_gradient_token(
                "gradient.backdrop",
                vec![(0.0, "color.light"), (1.0, "color.dark")],
            ),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                rect_backdrop_at("backdrop", "gradient.backdrop", 100.0, 100.0, 220.0, 100.0),
                text_at("headline", "color.text", 130.0, 130.0, 80.0, 30.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "gradient backdrop should use its worst-contrast stop; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn translucent_backdrop_composites_over_page() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#000000"),
            color_token_hex("color.scrim", "#ffffff80"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                rect_backdrop_at("scrim", "color.scrim", 100.0, 100.0, 220.0, 100.0),
                text_at("headline", "color.text", 130.0, 130.0, 80.0, 30.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.low"),
        "translucent white over black should composite to gray and warn as low contrast; codes: {:?}",
        codes(&report)
    );
    assert!(
        !has_code(&report, "contrast.invisible"),
        "translucent compositing should not fall back to black-on-black invisibility; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn group_opacity_cascades_into_backdrop_compositing() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#000000"),
            color_token_hex("color.scrim", "#ffffff"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                group_at_with_opacity(
                    "group.scrim",
                    0.0,
                    0.0,
                    Some(0.5),
                    vec![rect_backdrop_at(
                        "scrim",
                        "color.scrim",
                        100.0,
                        100.0,
                        220.0,
                        100.0,
                    )],
                ),
                text_at("headline", "color.text", 130.0, 130.0, 80.0, 30.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.low"),
        "group opacity should cascade into the child backdrop before contrast sampling; codes: {:?}",
        codes(&report)
    );
    assert!(
        !has_code(&report, "contrast.invisible"),
        "group opacity compositing should not treat the child backdrop as fully opaque or absent; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn transparent_backdrop_does_not_override_page() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#003087"),
            color_token_hex("color.clear", "#ffffff00"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                rect_backdrop_at("clear", "color.clear", 100.0, 100.0, 220.0, 100.0),
                text_at("headline", "color.text", 130.0, 130.0, 80.0, 30.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "fully transparent paint should leave the navy page as the sampled backdrop; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn polygon_backdrop_uses_true_containment() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                polygon_backdrop(
                    "triangle",
                    "color.backdrop",
                    vec![(100.0, 100.0), (300.0, 100.0), (200.0, 250.0)],
                ),
                text_at("headline", "color.text", 190.0, 140.0, 20.0, 20.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "text inside the triangle fill should use the polygon backdrop; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn polygon_bbox_corner_is_not_a_backdrop() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                polygon_backdrop(
                    "triangle",
                    "color.backdrop",
                    vec![(100.0, 100.0), (300.0, 100.0), (200.0, 250.0)],
                ),
                text_at("headline", "color.text", 105.0, 225.0, 20.0, 20.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.invisible"),
        "text in the triangle bbox but outside the polygon should keep the page backdrop; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn polyline_fill_can_be_text_backdrop() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.backdrop", "#003087"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![
                polyline_backdrop(
                    "polyline.fill",
                    "color.backdrop",
                    vec![(100.0, 100.0), (300.0, 100.0), (200.0, 250.0)],
                ),
                text_at("headline", "color.text", 190.0, 140.0, 20.0, 20.0),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.invisible"),
        "filled polyline should use its renderer-closed fill as a backdrop; codes: {:?}",
        codes(&report)
    );
}

/// Text node with no fill → no contrast check → no warning.
#[test]
fn text_without_fill_skips_contrast_check() {
    let doc = doc_with(
        vec![color_token_hex("color.bg", "#ffffff")],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_size("text.one", None, None, None)],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.low"),
        "text with no fill must not produce contrast.low; codes: {:?}",
        codes(&report)
    );
}

/// Page with no background token → contrast checks are skipped entirely.
#[test]
fn no_page_background_skips_contrast_check() {
    let doc = doc_with(
        vec![color_token_hex("color.text", "#aaaaaa")],
        vec![minimal_page(
            "page.one",
            vec![text_with_fill_and_size(
                "text.one",
                Some("color.text"),
                None,
                None,
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.low"),
        "page with no background must not produce contrast.low; codes: {:?}",
        codes(&report)
    );
}

/// Build a text node with an explicit fill token AND a `contrast-bg` hint token.
fn text_with_fill_and_contrast_bg(id: &str, fill_token: &str, contrast_bg_token: &str) -> Node {
    Node::Text(Box::new(zenith_core::TextNode {
        shadow: None,
        filter: None,
        mask: None,
        id: id.to_owned(),
        name: None,
        role: None,
        x: Some(pxv(0.0)),
        y: Some(pxv(0.0)),
        w: Some(pxv(200.0)),
        h: Some(pxv(40.0)),
        align: None,
        v_align: None,
        direction: None,
        overflow: None,
        overflow_wrap: None,
        style: None,
        fill: Some(PropertyValue::TokenRef(fill_token.to_owned())),
        stroke: None,
        stroke_width: None,
        contrast_bg: Some(PropertyValue::TokenRef(contrast_bg_token.to_owned())),
        font_family: None,
        font_size: None,
        font_size_min: None,
        font_weight: None,
        font_features: None,
        font_alternates: None,
        letter_spacing: None,
        kerning_pairs: Vec::new(),
        opacity: None,
        visible: None,
        locked: None,
        selectable: None,
        rotate: None,
        blend_mode: None,
        blur: None,
        chain: None,
        drop_cap_lines: None,
        hyphenate: None,
        widow_orphan: None,
        tab_leader: None,
        text_exclusion: None,
        padding_left: None,
        text_indent: None,
        content_format: None,
        src: None,
        bullet: None,
        bullet_gap: None,
        anchor: None,
        anchor_zone: None,
        anchor_sibling: None,
        anchor_edge: None,
        anchor_gap: None,
        anchor_parent: None,
        spans: vec![],
        block_styles: Vec::new(),
        source_span: None,
        unknown_props: BTreeMap::new(),
    }))
}

/// A `contrast-bg` hint takes TOP priority over the page background: a dark fill
/// with a near-matching `contrast-bg` on a WHITE page must still warn
/// `contrast.invisible` (judged against the hint, not the page bg), and the
/// message names the hint.
#[test]
fn contrast_bg_hint_used_as_background() {
    // Dark hint + dark fill → effectively invisible despite the white page bg.
    let dark = doc_with(
        vec![
            color_token_hex("color.bg", "#ffffff"),
            color_token_hex("color.text", "#222222"),
            color_token_hex("color.photo.shadow", "#101010"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_contrast_bg(
                "coverline",
                "color.text",
                "color.photo.shadow",
            )],
        )],
    );
    let report = validate(&dark);
    assert!(
        has_code(&report, "contrast.invisible"),
        "dark fill on a near-matching contrast-bg hint must warn contrast.invisible; codes: {:?}",
        codes(&report)
    );
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "contrast.invisible")
        .expect("must exist");
    assert!(
        diag.message.contains("contrast-bg hint"),
        "message must name the contrast-bg hint as the bg source; got: {}",
        diag.message
    );

    // Light hint + dark fill → high contrast → NO warning (hint overrides bg).
    let light = doc_with(
        vec![
            color_token_hex("color.bg", "#000000"),
            color_token_hex("color.text", "#111111"),
            color_token_hex("color.photo.light", "#fafafa"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.bg",
            vec![text_with_fill_and_contrast_bg(
                "coverline",
                "color.text",
                "color.photo.light",
            )],
        )],
    );
    let report = validate(&light);
    assert!(
        !has_code(&report, "contrast.low"),
        "dark fill on a light contrast-bg hint must NOT warn contrast.low; codes: {:?}",
        codes(&report)
    );
    assert!(
        !has_code(&report, "contrast.invisible"),
        "dark fill on a light contrast-bg hint must NOT warn contrast.invisible; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn image_backdrop_without_hint_is_indeterminate() {
    let doc = doc_with_backdrop_image(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![
            image_backdrop_at("image.backdrop", 0.0, 0.0, 220.0, 100.0),
            text_at("headline", "color.text", 40.0, 30.0, 80.0, 30.0),
        ],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.indeterminate_backdrop"),
        "text over image without contrast-bg should request a contrast hint; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn contrast_bg_hint_suppresses_image_indeterminate() {
    let doc = doc_with_backdrop_image(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.text", "#000000"),
            color_token_hex("color.photo.light", "#ffffff"),
        ],
        vec![
            image_backdrop_at("image.backdrop", 0.0, 0.0, 220.0, 100.0),
            text_with_fill_and_contrast_bg("headline", "color.text", "color.photo.light"),
        ],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.indeterminate_backdrop"),
        "contrast-bg hint should suppress image indeterminate advisory; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn opaque_solid_above_image_suppresses_indeterminate_backdrop() {
    let doc = doc_with_backdrop_image(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.cover", "#ffffff"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![
            image_backdrop_at("image.backdrop", 0.0, 0.0, 220.0, 100.0),
            rect_backdrop_at("cover", "color.cover", 0.0, 0.0, 220.0, 100.0),
            text_at("headline", "color.text", 40.0, 30.0, 80.0, 30.0),
        ],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.indeterminate_backdrop"),
        "opaque known paint above an image should make the sampled backdrop determinate; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn translucent_solid_above_image_remains_indeterminate() {
    let doc = doc_with_backdrop_image(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.scrim", "#ffffff80"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![
            image_backdrop_at("image.backdrop", 0.0, 0.0, 220.0, 100.0),
            rect_backdrop_at("scrim", "color.scrim", 0.0, 0.0, 220.0, 100.0),
            text_at("headline", "color.text", 40.0, 30.0, 80.0, 30.0),
        ],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.indeterminate_backdrop"),
        "translucent known paint above an image should still include unknown image pixels; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn transparent_image_backdrop_is_ignored() {
    let doc = doc_with_backdrop_image(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![
            image_backdrop_at_with_opacity("image.backdrop", 0.0, 0.0, 220.0, 100.0, Some(0.0)),
            text_at("headline", "color.text", 40.0, 30.0, 80.0, 30.0),
        ],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.indeterminate_backdrop"),
        "fully transparent image paint should not make the backdrop indeterminate; codes: {:?}",
        codes(&report)
    );
}

// ── Table cell fill regression tests ──────────────────────────────────────────

/// Build a minimal `TableNode` with one body row containing one cell, where the
/// cell holds a single text child.
fn table_with_cell_text(
    cell_fill: Option<PropertyValue>,
    table_fill: Option<PropertyValue>,
    header_fill: Option<PropertyValue>,
    header_rows: Option<u32>,
    text_fill_token: &str,
) -> Node {
    let text = minimal_text(
        "cell.text",
        Some(PropertyValue::TokenRef(text_fill_token.to_owned())),
    );
    let cell = TableCell {
        colspan: 1,
        rowspan: 1,
        children: vec![text],
        fill: cell_fill,
        border: None,
        border_width: None,
        h_align: None,
        v_align: None,
        source_span: None,
        unknown_props: BTreeMap::new(),
    };
    let row = TableRow {
        cells: vec![cell],
        source_span: None,
        unknown_props: BTreeMap::new(),
    };
    Node::Table(Box::new(TableNode {
        id: "table.one".to_owned(),
        name: None,
        role: None,
        x: Some(pxv(0.0)),
        y: Some(pxv(0.0)),
        w: Some(pxv(400.0)),
        h: Some(pxv(200.0)),
        columns: vec![],
        rows: vec![row],
        header_rows,
        flows: None,
        gap: None,
        cell_padding: None,
        border_collapse: None,
        fill: table_fill,
        border: None,
        border_width: None,
        header_fill,
        header_style: None,
        h_align: None,
        v_align: None,
        style: None,
        opacity: None,
        visible: None,
        locked: None,
        rotate: None,
        anchor: None,
        anchor_zone: None,
        anchor_sibling: None,
        anchor_edge: None,
        anchor_gap: None,
        anchor_parent: None,
        source_span: None,
        unknown_props: BTreeMap::new(),
    }))
}

/// White text (`#ffffff`) in a dark-blue-filled cell (`#003087`) on a white
/// page must NOT fire `contrast.low` — the cell fill is the effective bg.
/// APCA Lc of white on #003087 ≈ 83, which clears the Lc 60 threshold.
#[test]
fn white_text_in_dark_cell_no_false_positive() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.cell", r##"#003087"##),
            color_token_hex("color.text", "#ffffff"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![table_with_cell_text(
                Some(PropertyValue::TokenRef("color.cell".to_owned())),
                None,
                None,
                None,
                "color.text",
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.low"),
        "white text in a dark-blue cell should NOT warn contrast.low (cell fill is bg); codes: {:?}",
        codes(&report)
    );
}

/// White text (`#ffffff`) in a light-gray-filled cell (`#dddddd`) on a white
/// page SHOULD still fire `contrast.low` — the cell fill is the bg and it gives
/// insufficient contrast. APCA Lc of white on #dddddd ≈ 21 < 60.
#[test]
fn white_text_in_light_cell_still_warns() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.cell", r##"#dddddd"##),
            color_token_hex("color.text", "#ffffff"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![table_with_cell_text(
                Some(PropertyValue::TokenRef("color.cell".to_owned())),
                None,
                None,
                None,
                "color.text",
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.low"),
        "white text in a light-gray cell should warn contrast.low; codes: {:?}",
        codes(&report)
    );
}

/// When a cell has NO fill and the table has NO fill, the check must fall back
/// to the page background — existing behavior is preserved.
#[test]
fn cell_no_fill_falls_back_to_page_bg() {
    // Light gray text (#aaaaaa) on white page → Lc ~46 < 60 → warns.
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.text", r##"#aaaaaa"##),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![table_with_cell_text(None, None, None, None, "color.text")],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "contrast.low"),
        "light-gray text in an unfilled cell must still warn via page-bg fallback; codes: {:?}",
        codes(&report)
    );
}

/// Table-level `fill` is used as the cell bg when cell has no per-cell fill.
/// White text (`#ffffff`) on a dark table fill (`#003087`) should NOT warn.
#[test]
fn table_fill_used_when_cell_has_no_fill() {
    let doc = doc_with(
        vec![
            color_token_hex("color.page", "#ffffff"),
            color_token_hex("color.table", r##"#003087"##),
            color_token_hex("color.text", "#ffffff"),
        ],
        vec![page_with_bg(
            "page.one",
            "color.page",
            vec![table_with_cell_text(
                None,
                Some(PropertyValue::TokenRef("color.table".to_owned())),
                None,
                None,
                "color.text",
            )],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "contrast.low"),
        "white text on dark table.fill should NOT warn; codes: {:?}",
        codes(&report)
    );
}

/// A raw literal `contrast-bg` value is rejected as `token.raw_visual_literal`,
/// consistent with `fill`/`stroke`.
#[test]
fn contrast_bg_literal_rejected() {
    let mut text = match text_with_fill_and_contrast_bg("t", "color.text", "color.bg") {
        Node::Text(t) => t,
        _ => unreachable!(),
    };
    // Overwrite the hint with a RAW literal.
    text.contrast_bg = Some(PropertyValue::Literal("#000000".to_owned()));
    let doc = doc_with(
        vec![
            color_token_hex("color.bg", "#ffffff"),
            color_token_hex("color.text", "#000000"),
        ],
        vec![page_with_bg("page.one", "color.bg", vec![Node::Text(text)])],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "token.raw_visual_literal"),
        "a raw-literal contrast-bg must flag token.raw_visual_literal; codes: {:?}",
        codes(&report)
    );
}
