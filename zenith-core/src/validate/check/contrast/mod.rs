//! WCAG 3 (APCA) contrast advisory check.
//!
//! Compares text-node fills against the colour they visually sit on: the
//! topmost preceding painted backdrop that geometrically covers the text in
//! page coordinates, falling back to the page background. The metric is APCA
//! lightness contrast (`Lc`) with the current WCAG 3 draft thresholds.

mod geometry;

use std::collections::BTreeMap;

use crate::ast::node::{Node, ShapeNode, TableNode, TextNode};
use crate::ast::style::Style;
use crate::ast::value::{PropertyValue, dim_to_px};
use crate::color::{apca_lc, parse_rgb};
use crate::diagnostics::Diagnostic;
use crate::tokens::{ResolvedToken, ResolvedValue};

use geometry::{CoverageShape, RectPx, group_offset, local_box, text_box};

/// Minimum alpha for a backdrop fill to be treated as opaque enough to act as
/// the effective background. Fills below this (e.g. a translucent scrim) are
/// skipped so they don't override a more solid backdrop or the page colour.
const BACKDROP_OPAQUE_ALPHA: u8 = 128;

/// Below this APCA magnitude the text is effectively painted into its backdrop,
/// which is a stronger signal than ordinary sub-threshold contrast.
const INVISIBLE_LC_FLOOR: f64 = 15.0;

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
    page_bg_rgb: Option<(u8, u8, u8)>,
    page_size: (f64, f64),
}

#[derive(Clone, Copy)]
struct BackdropCandidate {
    rgb: (u8, u8, u8),
    bounds: RectPx,
    shape: CoverageShape,
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
            Node::Frame(f) => {
                let frame_box = absolute_box(node, ctx, env.resolved_tokens);
                let frame_clip = frame_box.and_then(|b| match ctx.clip {
                    Some(clip) => clip.intersect(b),
                    None => Some(b),
                });
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
            | Node::Image(_)
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
    let Some((r, g, b, a)) =
        resolve_fill_rgba(fill, style.as_deref(), env.style_map, env.resolved_tokens)
    else {
        return;
    };
    if a < BACKDROP_OPAQUE_ALPHA {
        return;
    }
    let Some(bounds) = absolute_box(node, ctx, env.resolved_tokens) else {
        return;
    };
    let clipped_bounds = match ctx.clip {
        Some(clip) => clip.intersect(bounds),
        None => Some(bounds),
    };
    if let Some(bounds) = clipped_bounds {
        candidates.push(BackdropCandidate {
            rgb: (r, g, b),
            bounds,
            shape,
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

    let hint_rgb = resolve_color_property(text.contrast_bg.as_ref(), env.resolved_tokens);
    let backdrop = text_box(text, ctx.page_size, env.resolved_tokens)
        .and_then(|bbox| backdrop_rgb(bbox, ctx.clip, candidates));
    let bg_source = if hint_rgb.is_some() {
        "contrast-bg hint"
    } else if backdrop.is_some() {
        "backdrop"
    } else {
        "page background"
    };
    let Some(bg_rgb) = hint_rgb.or(backdrop).or(ctx.page_bg_rgb) else {
        return;
    };

    let size_px = resolve_font_size(text, env.style_map, env.resolved_tokens);
    let weight = resolve_font_weight(text, env.style_map, env.resolved_tokens);
    let is_large = size_px >= 24.0 || (size_px >= 18.66 && weight >= 700);
    let threshold = if is_large { 45.0_f64 } else { 60.0_f64 };
    let lc = apca_lc(fg_rgb, bg_rgb).abs();

    if lc < threshold {
        let (code, detail) = if lc < INVISIBLE_LC_FLOOR {
            (
                "contrast.invisible",
                format!(
                    "is effectively invisible against {} (Lc below {:.0})",
                    bg_source, INVISIBLE_LC_FLOOR
                ),
            )
        } else {
            (
                "contrast.low",
                format!(
                    "of fill on {} is below the WCAG 3 draft threshold (Lc {:.0})",
                    bg_source, threshold
                ),
            )
        };
        diagnostics.push(Diagnostic::warning(
            code,
            format!("text '{}': APCA contrast Lc {:.1} {}", text.id, lc, detail),
            text.source_span,
            Some(text.id.clone()),
        ));
    }
}

fn backdrop_rgb(
    text_bbox: RectPx,
    clip: Option<RectPx>,
    candidates: &[BackdropCandidate],
) -> Option<(u8, u8, u8)> {
    if let Some(clip) = clip
        && !clip.contains_rect(text_bbox)
    {
        return None;
    }
    for candidate in candidates.iter().rev() {
        if candidate.shape.contains_rect(candidate.bounds, text_bbox) {
            return Some(candidate.rgb);
        }
    }
    None
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
        let (r, g, b, a) = resolve_fill_rgba(pv, None, env.style_map, env.resolved_tokens)?;
        if a >= BACKDROP_OPAQUE_ALPHA {
            Some((r, g, b))
        } else {
            None
        }
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
                page_bg_rgb: cell_bg,
                page_size,
            };
            let mut candidates = Vec::new();
            walk_paint(&cell.children, ctx, &mut candidates, env, diagnostics);
        }
    }
}

fn resolve_fill_rgba(
    fill: &Option<PropertyValue>,
    style: Option<&str>,
    style_map: &BTreeMap<&str, &Style>,
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
) -> Option<(u8, u8, u8, u8)> {
    let pv = fill
        .as_ref()
        .or_else(|| style_property(style, "fill", style_map))?;
    let PropertyValue::TokenRef(id) = pv else {
        return None;
    };
    let rt = resolved_tokens.get(id.as_str())?;
    let ResolvedValue::Color(hex) = &rt.value else {
        return None;
    };

    let (r, g, b) = parse_rgb(hex)?;
    let alpha = hex
        .strip_prefix('#')
        .filter(|h| h.len() == 8)
        .and_then(|h| u8::from_str_radix(&h[6..8], 16).ok())
        .unwrap_or(255);
    Some((r, g, b, alpha))
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
