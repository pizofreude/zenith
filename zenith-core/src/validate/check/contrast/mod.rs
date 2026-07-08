//! WCAG 3 (APCA) contrast advisory check.
//!
//! Compares text-node fills against the colour they visually sit on: the
//! topmost preceding painted backdrop that geometrically covers the text in
//! page coordinates, falling back to the page background. The metric is APCA
//! lightness contrast (`Lc`) with the current WCAG 3 draft thresholds.

mod geometry;

use std::collections::BTreeMap;

use crate::ast::node::{ImageNode, Node, ShapeNode, TableNode, TextNode};
use crate::ast::style::Style;
use crate::ast::value::{PropertyValue, dim_to_px};
use crate::color::{apca_lc, parse_rgb};
use crate::diagnostics::Diagnostic;
use crate::tokens::{ResolvedToken, ResolvedValue};

use geometry::{CoverageShape, RectPx, group_offset, local_box, text_box};

/// Below this APCA magnitude the text is effectively painted into its backdrop,
/// which is a stronger signal than ordinary sub-threshold contrast.
const INVISIBLE_LC_FLOOR: f64 = 15.0;
const MIN_PAINT_ALPHA: f64 = 1.0 / 255.0;

pub(super) fn check_page_text_contrast(
    children: &[Node],
    page_bg_rgb: Option<(u8, u8, u8)>,
    page_size: (f64, f64),
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut candidates = Vec::new();
    let ctx = PaintCtx {
        dx: 0.0,
        dy: 0.0,
        clip: None,
        opacity: 1.0,
        page_bg_rgb,
        page_size,
    };
    let env = ContrastEnv {
        resolved_tokens,
        style_map,
    };
    walk_paint(children, ctx, &mut candidates, env, diagnostics);
}

#[derive(Clone, Copy)]
struct PaintCtx {
    dx: f64,
    dy: f64,
    clip: Option<RectPx>,
    opacity: f64,
    page_bg_rgb: Option<(u8, u8, u8)>,
    page_size: (f64, f64),
}

struct BackdropCandidate {
    paint: BackdropPaint,
    bounds: RectPx,
    shape: CoverageShape,
}

#[derive(Debug)]
enum BackdropPaint {
    Solid(PaintColor),
    Gradient(Vec<PaintColor>),
    Indeterminate,
}

#[derive(Clone, Copy, Debug)]
struct PaintColor {
    rgb: (u8, u8, u8),
    alpha: f64,
}

#[derive(Clone, Copy)]
struct SampledBackdrop {
    rgb: (u8, u8, u8),
    source: &'static str,
}

#[derive(Clone, Copy)]
struct ContrastSample {
    lc: f64,
    source: &'static str,
}

#[derive(Clone, Copy)]
struct ContrastEnv<'a> {
    resolved_tokens: &'a BTreeMap<String, ResolvedToken>,
    style_map: &'a BTreeMap<&'a str, &'a Style>,
}

fn walk_paint(
    children: &[Node],
    ctx: PaintCtx,
    candidates: &mut Vec<BackdropCandidate>,
    env: ContrastEnv<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for node in children {
        if !node_visible(node) {
            continue;
        }
        match node {
            Node::Rect(r) => push_backdrop(
                node,
                &r.fill,
                &r.style,
                CoverageShape::Rect,
                ctx,
                candidates,
                env,
            ),
            Node::Ellipse(e) => push_backdrop(
                node,
                &e.fill,
                &e.style,
                CoverageShape::Ellipse,
                ctx,
                candidates,
                env,
            ),
            Node::Shape(s) => push_shape_backdrop(node, s, ctx, candidates, env),
            Node::Image(img) => push_image_backdrop(node, img, ctx, candidates, env),
            Node::Frame(f) => {
                let frame_box = absolute_box(node, ctx, env.resolved_tokens);
                let frame_clip = frame_box.and_then(|b| clip_bounds(ctx.clip, b));
                let no_fill: Option<PropertyValue> = None;
                push_backdrop(
                    node,
                    &no_fill,
                    &f.style,
                    CoverageShape::Rect,
                    ctx,
                    candidates,
                    env,
                );
                let child_ctx = PaintCtx {
                    clip: frame_clip,
                    opacity: cascaded_opacity(ctx.opacity, f.opacity),
                    ..ctx
                };
                walk_paint(&f.children, child_ctx, candidates, env, diagnostics);
            }
            Node::Group(g) => {
                let (gx, gy) = group_offset(
                    g.x.as_ref(),
                    g.y.as_ref(),
                    ctx.page_size,
                    env.resolved_tokens,
                );
                let child_ctx = PaintCtx {
                    dx: ctx.dx + gx,
                    dy: ctx.dy + gy,
                    opacity: cascaded_opacity(ctx.opacity, g.opacity),
                    ..ctx
                };
                walk_paint(&g.children, child_ctx, candidates, env, diagnostics);
            }
            Node::Text(t) => check_text_node(t, ctx, candidates, env, diagnostics),
            Node::Table(t) => {
                check_table_text_contrast(t, ctx.page_bg_rgb, ctx.page_size, env, diagnostics)
            }
            Node::Line(_)
            | Node::Code(_)
            | Node::Polygon(_)
            | Node::Polyline(_)
            | Node::Path(_)
            | Node::Instance(_)
            | Node::Field(_)
            | Node::Footnote(_)
            | Node::Toc(_)
            | Node::Connector(_)
            | Node::Pattern(_)
            | Node::Chart(_)
            | Node::Light(_)
            | Node::Mesh(_)
            | Node::Unknown(_) => {}
        }
    }
}

fn push_shape_backdrop(
    node: &Node,
    shape: &ShapeNode,
    ctx: PaintCtx,
    candidates: &mut Vec<BackdropCandidate>,
    env: ContrastEnv<'_>,
) {
    let coverage = match shape.kind.as_deref() {
        Some("decision") => CoverageShape::Diamond,
        Some("terminator") => CoverageShape::Capsule,
        Some("ellipse") => CoverageShape::Ellipse,
        Some("process") | None => CoverageShape::Rect,
        _ => CoverageShape::Rect,
    };
    push_backdrop(
        node,
        &shape.fill,
        &shape.style,
        coverage,
        ctx,
        candidates,
        env,
    );
}

fn push_backdrop(
    node: &Node,
    fill: &Option<PropertyValue>,
    style: &Option<String>,
    shape: CoverageShape,
    ctx: PaintCtx,
    candidates: &mut Vec<BackdropCandidate>,
    env: ContrastEnv<'_>,
) {
    let opacity = ctx.opacity * node_opacity(node).unwrap_or(1.0);
    let Some(paint) = resolve_fill_paint(
        fill,
        style.as_deref(),
        env.style_map,
        env.resolved_tokens,
        opacity,
    ) else {
        return;
    };
    let Some(bounds) = absolute_box(node, ctx, env.resolved_tokens) else {
        return;
    };
    if let Some(bounds) = clip_bounds(ctx.clip, bounds) {
        candidates.push(BackdropCandidate {
            paint,
            bounds,
            shape,
        });
    }
}

fn push_image_backdrop(
    node: &Node,
    _image: &ImageNode,
    ctx: PaintCtx,
    candidates: &mut Vec<BackdropCandidate>,
    env: ContrastEnv<'_>,
) {
    if ctx.opacity * node_opacity(node).unwrap_or(1.0) < MIN_PAINT_ALPHA {
        return;
    }
    let Some(bounds) = absolute_box(node, ctx, env.resolved_tokens) else {
        return;
    };
    if let Some(bounds) = clip_bounds(ctx.clip, bounds) {
        candidates.push(BackdropCandidate {
            paint: BackdropPaint::Indeterminate,
            bounds,
            shape: CoverageShape::Rect,
        });
    }
}

fn absolute_box(
    node: &Node,
    ctx: PaintCtx,
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
) -> Option<RectPx> {
    local_box(node, ctx.page_size, resolved_tokens).map(|b| b.translated(ctx.dx, ctx.dy))
}

fn check_text_node(
    text: &TextNode,
    ctx: PaintCtx,
    candidates: &[BackdropCandidate],
    env: ContrastEnv<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(fg_rgb) = resolve_color_property(
        text.fill
            .as_ref()
            .or_else(|| style_property(text.style.as_deref(), "fill", env.style_map)),
        env.resolved_tokens,
    ) else {
        return;
    };

    let size_px = resolve_font_size(text, env.style_map, env.resolved_tokens);
    let weight = resolve_font_weight(text, env.style_map, env.resolved_tokens);
    let is_large = size_px >= 24.0 || (size_px >= 18.66 && weight >= 700);
    let threshold = if is_large { 45.0_f64 } else { 60.0_f64 };

    let hint_rgb = resolve_color_property(text.contrast_bg.as_ref(), env.resolved_tokens);
    let (backdrop_samples, indeterminate_backdrop) = if hint_rgb.is_some() {
        (Vec::new(), false)
    } else {
        text_box(text, ctx.page_size, env.resolved_tokens)
            .map(|bbox| collect_backdrop_samples(bbox, ctx.clip, candidates, ctx.page_bg_rgb))
            .unwrap_or_default()
    };
    let best = select_contrast_sample(fg_rgb, hint_rgb, &backdrop_samples, ctx.page_bg_rgb);

    if hint_rgb.is_none() && indeterminate_backdrop {
        diagnostics.push(Diagnostic::advisory(
            "contrast.indeterminate_backdrop",
            format!(
                "text '{}': backdrop includes an image and cannot be sampled during validation; add a contrast-bg hint",
                text.id
            ),
            text.source_span,
            Some(text.id.clone()),
        ));
    }

    if let Some(sample) = best
        && sample.lc < threshold
    {
        let (code, detail) = if sample.lc < INVISIBLE_LC_FLOOR {
            (
                "contrast.invisible",
                format!(
                    "is effectively invisible against {} (Lc below {:.0})",
                    sample.source, INVISIBLE_LC_FLOOR
                ),
            )
        } else {
            (
                "contrast.low",
                format!(
                    "against {} is below the WCAG 3 draft threshold (Lc {:.0})",
                    sample.source, threshold
                ),
            )
        };
        diagnostics.push(Diagnostic::warning(
            code,
            format!(
                "text '{}': APCA contrast Lc {:.1} {}",
                text.id, sample.lc, detail
            ),
            text.source_span,
            Some(text.id.clone()),
        ));
    }
}

fn collect_backdrop_samples(
    text_box: RectPx,
    clip: Option<RectPx>,
    candidates: &[BackdropCandidate],
    page_bg_rgb: Option<(u8, u8, u8)>,
) -> (Vec<SampledBackdrop>, bool) {
    let mut backdrops = Vec::with_capacity(5);
    let mut indeterminate = false;
    if let Some(clip) = clip
        && !clip.contains_rect(text_box)
    {
        return (backdrops, indeterminate);
    }
    for (x, y) in text_box.sample_points() {
        let mut point_indeterminate = false;
        let mut samples: Vec<SampledBackdrop> = page_bg_rgb
            .into_iter()
            .map(|rgb| SampledBackdrop {
                rgb,
                source: "page background",
            })
            .collect();
        for candidate in candidates {
            if candidate.shape.contains_point(candidate.bounds, x, y) {
                match &candidate.paint {
                    BackdropPaint::Solid(color) => {
                        samples = composite_solid_samples(&samples, *color);
                        if color.alpha >= 1.0 {
                            point_indeterminate = false;
                        }
                    }
                    BackdropPaint::Gradient(stops) => {
                        samples = composite_gradient_samples(&samples, stops);
                        if stops.iter().all(|stop| stop.alpha >= 1.0) {
                            point_indeterminate = false;
                        }
                    }
                    BackdropPaint::Indeterminate => {
                        point_indeterminate = true;
                    }
                }
            }
        }
        if point_indeterminate {
            indeterminate = true;
        }
        for sample in samples {
            push_unique_sample(&mut backdrops, sample);
        }
    }
    (backdrops, indeterminate)
}

fn composite_solid_samples(samples: &[SampledBackdrop], paint: PaintColor) -> Vec<SampledBackdrop> {
    if samples.is_empty() {
        return vec![SampledBackdrop {
            rgb: paint.rgb,
            source: "backdrop",
        }];
    }
    samples
        .iter()
        .map(|sample| SampledBackdrop {
            rgb: composite_rgb(paint.rgb, paint.alpha, sample.rgb),
            source: "backdrop",
        })
        .collect()
}

fn composite_gradient_samples(
    samples: &[SampledBackdrop],
    stops: &[PaintColor],
) -> Vec<SampledBackdrop> {
    if stops.is_empty() {
        return samples.to_vec();
    }
    let mut composited = Vec::with_capacity(stops.len() * samples.len().max(1));
    for stop in stops {
        if samples.is_empty() {
            composited.push(SampledBackdrop {
                rgb: stop.rgb,
                source: "backdrop",
            });
        } else {
            composited.extend(samples.iter().map(|sample| SampledBackdrop {
                rgb: composite_rgb(stop.rgb, stop.alpha, sample.rgb),
                source: "backdrop",
            }));
        }
    }
    composited
}

fn composite_rgb(src: (u8, u8, u8), alpha: f64, dst: (u8, u8, u8)) -> (u8, u8, u8) {
    let alpha = alpha.clamp(0.0, 1.0);
    (
        composite_channel(src.0, alpha, dst.0),
        composite_channel(src.1, alpha, dst.1),
        composite_channel(src.2, alpha, dst.2),
    )
}

fn composite_channel(src: u8, alpha: f64, dst: u8) -> u8 {
    ((src as f64 * alpha) + (dst as f64 * (1.0 - alpha))).round() as u8
}

fn push_unique_sample(backdrops: &mut Vec<SampledBackdrop>, sample: SampledBackdrop) {
    if !backdrops
        .iter()
        .any(|backdrop| backdrop.rgb == sample.rgb && backdrop.source == sample.source)
    {
        backdrops.push(sample);
    }
}

fn select_contrast_sample(
    fg_rgb: (u8, u8, u8),
    hint_rgb: Option<(u8, u8, u8)>,
    sampled_backdrops: &[SampledBackdrop],
    page_bg_rgb: Option<(u8, u8, u8)>,
) -> Option<ContrastSample> {
    if let Some(rgb) = hint_rgb {
        return Some(ContrastSample {
            lc: apca_lc(fg_rgb, rgb).abs(),
            source: "contrast-bg hint",
        });
    }
    if !sampled_backdrops.is_empty() {
        let mut worst: Option<ContrastSample> = None;
        for backdrop in sampled_backdrops {
            let sample = ContrastSample {
                lc: apca_lc(fg_rgb, backdrop.rgb).abs(),
                source: backdrop.source,
            };
            if worst.is_none_or(|w| sample.lc < w.lc) {
                worst = Some(sample);
            }
        }
        return worst;
    }
    page_bg_rgb.map(|rgb| ContrastSample {
        lc: apca_lc(fg_rgb, rgb).abs(),
        source: "page background",
    })
}

fn check_table_text_contrast(
    table: &TableNode,
    page_bg_rgb: Option<(u8, u8, u8)>,
    page_size: (f64, f64),
    env: ContrastEnv<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let header_rows = table.header_rows.unwrap_or(0);
    let resolve_fill = |pv: &Option<PropertyValue>| -> Option<(u8, u8, u8)> {
        resolve_fill_paint(pv, None, env.style_map, env.resolved_tokens, 1.0)?.as_solid_rgb()
    };

    for (row_idx, row) in table.rows.iter().enumerate() {
        let is_header = (row_idx as u32) < header_rows;
        for cell in &row.cells {
            let cell_bg = if let Some(rgb) = resolve_fill(&cell.fill) {
                Some(rgb)
            } else if is_header {
                resolve_fill(&table.header_fill)
                    .or_else(|| resolve_fill(&table.fill))
                    .or(page_bg_rgb)
            } else {
                resolve_fill(&table.fill).or(page_bg_rgb)
            };
            let ctx = PaintCtx {
                dx: 0.0,
                dy: 0.0,
                clip: None,
                opacity: 1.0,
                page_bg_rgb: cell_bg,
                page_size,
            };
            let mut candidates = Vec::new();
            walk_paint(&cell.children, ctx, &mut candidates, env, diagnostics);
        }
    }
}

fn resolve_fill_paint(
    fill: &Option<PropertyValue>,
    style: Option<&str>,
    style_map: &BTreeMap<&str, &Style>,
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
    opacity: f64,
) -> Option<BackdropPaint> {
    let pv = fill
        .as_ref()
        .or_else(|| style_property(style, "fill", style_map))?;
    let PropertyValue::TokenRef(id) = pv else {
        return None;
    };
    let token = resolved_tokens.get(id.as_str())?;
    match &token.value {
        ResolvedValue::Color(hex) => solid_paint_from_hex(hex, opacity),
        ResolvedValue::CmykColor { hex, .. } => solid_paint_from_hex(hex, opacity),
        ResolvedValue::Gradient(gradient) => {
            let stops: Vec<PaintColor> = gradient
                .stops
                .iter()
                .filter_map(|(_, color_id)| {
                    resolved_tokens
                        .get(color_id.as_str())
                        .and_then(|token| resolved_color_paint(token, opacity))
                })
                .collect();
            if stops.is_empty() {
                None
            } else {
                Some(BackdropPaint::Gradient(stops))
            }
        }
        ResolvedValue::Dimension(_)
        | ResolvedValue::Number(_)
        | ResolvedValue::FontFamily(_)
        | ResolvedValue::FontWeight(_)
        | ResolvedValue::Shadow(_)
        | ResolvedValue::Filter(_)
        | ResolvedValue::Mask(_) => None,
    }
}

impl BackdropPaint {
    fn as_solid_rgb(&self) -> Option<(u8, u8, u8)> {
        match self {
            BackdropPaint::Solid(color) if color.alpha >= 1.0 => Some(color.rgb),
            BackdropPaint::Solid(_) => None,
            BackdropPaint::Gradient(_) | BackdropPaint::Indeterminate => None,
        }
    }
}

fn solid_paint_from_hex(hex: &str, opacity: f64) -> Option<BackdropPaint> {
    parse_paint_color(hex, opacity).map(BackdropPaint::Solid)
}

fn resolved_color_paint(token: &ResolvedToken, opacity: f64) -> Option<PaintColor> {
    match &token.value {
        ResolvedValue::Color(hex) => parse_paint_color(hex, opacity),
        ResolvedValue::CmykColor { hex, .. } => parse_paint_color(hex, opacity),
        ResolvedValue::Dimension(_)
        | ResolvedValue::Number(_)
        | ResolvedValue::FontFamily(_)
        | ResolvedValue::FontWeight(_)
        | ResolvedValue::Gradient(_)
        | ResolvedValue::Shadow(_)
        | ResolvedValue::Filter(_)
        | ResolvedValue::Mask(_) => None,
    }
}

fn parse_paint_color(hex: &str, opacity: f64) -> Option<PaintColor> {
    let rgb = parse_rgb(hex)?;
    let token_alpha = hex
        .strip_prefix('#')
        .filter(|h| h.len() == 8)
        .and_then(|h| u8::from_str_radix(&h[6..8], 16).ok())
        .unwrap_or(255) as f64
        / 255.0;
    let alpha = (token_alpha * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    if alpha < MIN_PAINT_ALPHA {
        return None;
    }
    Some(PaintColor { rgb, alpha })
}

fn cascaded_opacity(parent: f64, opacity: Option<f64>) -> f64 {
    parent * opacity.unwrap_or(1.0).clamp(0.0, 1.0)
}

macro_rules! node_option_field {
    ($node:expr, $field:ident) => {
        match $node {
            Node::Rect(n) => n.$field,
            Node::Ellipse(n) => n.$field,
            Node::Image(n) => n.$field,
            Node::Shape(n) => n.$field,
            Node::Frame(n) => n.$field,
            Node::Group(n) => n.$field,
            Node::Text(n) => n.$field,
            Node::Line(n) => n.$field,
            Node::Code(n) => n.$field,
            Node::Polygon(n) => n.$field,
            Node::Polyline(n) => n.$field,
            Node::Path(n) => n.$field,
            Node::Instance(n) => n.$field,
            Node::Field(n) => n.$field,
            Node::Footnote(_) => None,
            Node::Toc(n) => n.$field,
            Node::Connector(n) => n.$field,
            Node::Pattern(n) => n.$field,
            Node::Chart(n) => n.$field,
            Node::Light(n) => n.$field,
            Node::Mesh(n) => n.$field,
            Node::Table(n) => n.$field,
            Node::Unknown(_) => None,
        }
    };
}

fn node_opacity(node: &Node) -> Option<f64> {
    node_option_field!(node, opacity)
}

fn node_visible(node: &Node) -> bool {
    node_option_field!(node, visible).unwrap_or(true)
}

fn clip_bounds(clip: Option<RectPx>, bounds: RectPx) -> Option<RectPx> {
    match clip {
        Some(clip) => clip.intersect(bounds),
        None => Some(bounds),
    }
}

fn resolve_color_property(
    value: Option<&PropertyValue>,
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
) -> Option<(u8, u8, u8)> {
    let Some(PropertyValue::TokenRef(id)) = value else {
        return None;
    };
    resolved_tokens.get(id.as_str()).and_then(|rt| {
        if let ResolvedValue::Color(hex) = &rt.value {
            parse_rgb(hex)
        } else {
            None
        }
    })
}

fn style_property<'a>(
    style: Option<&str>,
    key: &str,
    style_map: &'a BTreeMap<&str, &Style>,
) -> Option<&'a PropertyValue> {
    style_map.get(style?).and_then(|s| s.properties.get(key))
}

fn resolve_font_size(
    text: &TextNode,
    style_map: &BTreeMap<&str, &Style>,
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
) -> f64 {
    text.font_size
        .as_ref()
        .or_else(|| style_property(text.style.as_deref(), "font-size", style_map))
        .and_then(|pv| {
            if let PropertyValue::TokenRef(id) = pv {
                resolved_tokens.get(id.as_str()).and_then(|rt| {
                    if let ResolvedValue::Dimension(dim) = &rt.value {
                        dim_to_px(dim.value, &dim.unit)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .unwrap_or(16.0)
}

fn resolve_font_weight(
    text: &TextNode,
    style_map: &BTreeMap<&str, &Style>,
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
) -> u32 {
    text.font_weight
        .as_ref()
        .or_else(|| style_property(text.style.as_deref(), "font-weight", style_map))
        .and_then(|pv| {
            if let PropertyValue::TokenRef(id) = pv {
                resolved_tokens.get(id.as_str()).and_then(|rt| {
                    if let ResolvedValue::FontWeight(w) = &rt.value {
                        Some(*w)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .unwrap_or(400)
}
