//! Table-node compilation: single-page tables with EXPLICIT/PROPORTIONAL
//! column widths and SEPARATE borders only.
//!
//! This unit lays a `table` out as a grid of cells inside its declared
//! `[x, y, w, h]` box, honoring `colspan`/`rowspan` (HTML-table cell flow).
//! Auto columns share the leftover width EQUALLY (content-based auto-sizing is
//! a LATER unit); rows share the table height EQUALLY (content-based row height
//! is a LATER unit); `border-collapse` is carried but only `"separate"` borders
//! are drawn here.
//!
//! Each cell emits, in order: an optional background `FillRect` (cell.fill or
//! table.fill), then an optional border drawn as four independent `StrokeLine`s
//! (cell or table defaults), then its compiled child content clipped to and
//! translated into the cell content box (cell padding inset), with the cell's
//! `h-align`/`v-align` (overriding the table default) shifting the child within
//! the content box. Opacity cascades (table.opacity × ctx.opacity).

use std::collections::{BTreeMap, BTreeSet};

use zenith_core::{
    Diagnostic, FontProvider, PropertyValue, ResolvedToken, Style, TableNode, dim_to_px,
};
use zenith_layout::RustybuzzEngine;

use crate::ir::SceneCommand;

use super::chain::ChainAssignments;
use super::field::FieldCtx;
use super::paint::resolve_property_color;
use super::util::resolve_property_dimension_px;
use super::{ComponentMap, RenderCtx, compile_node};

/// Geometry of one placed cell in absolute page pixels (already including the
/// table origin but NOT the cell-padding inset).
struct CellRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compile_table(
    table: &TableNode,
    resolved: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    components: &ComponentMap,
    fonts: &dyn FontProvider,
    engine: &RustybuzzEngine,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    chains: &ChainAssignments,
    field_ctx: &FieldCtx,
    ctx: RenderCtx,
) {
    // Entire subtree excluded when visible=false (no commands emitted).
    if table.visible == Some(false) {
        return;
    }

    // ── Resolve table geometry ───────────────────────────────────────────
    let (Some(x_dim), Some(y_dim), Some(w_dim), Some(h_dim)) =
        (&table.x, &table.y, &table.w, &table.h)
    else {
        diagnostics.push(Diagnostic::advisory(
            "scene.missing_geometry",
            format!(
                "table '{}' is missing one or more geometry properties (x, y, w, h); skipped",
                table.id
            ),
            table.source_span,
            Some(table.id.clone()),
        ));
        return;
    };
    let (Some(table_x), Some(table_y), Some(table_w), Some(table_h)) = (
        dim_to_px(x_dim.value, &x_dim.unit),
        dim_to_px(y_dim.value, &y_dim.unit),
        dim_to_px(w_dim.value, &w_dim.unit),
        dim_to_px(h_dim.value, &h_dim.unit),
    ) else {
        diagnostics.push(Diagnostic::advisory(
            "scene.missing_geometry",
            format!(
                "table '{}' has an unresolvable geometry unit (x, y, w, h); skipped",
                table.id
            ),
            table.source_span,
            Some(table.id.clone()),
        ));
        return;
    };

    // Absolute page origin (cascade translation applied).
    let origin_x = ctx.dx + table_x;
    let origin_y = ctx.dy + table_y;

    // ── Resolve gap + cell padding (token or literal), default 0 ─────────
    let gap = resolve_property_dimension_px(&table.gap, resolved, 0.0).max(0.0);
    let pad = resolve_property_dimension_px(&table.cell_padding, resolved, 0.0).max(0.0);

    // Opacity cascade.
    let opacity = (table.opacity.unwrap_or(1.0).clamp(0.0, 1.0)) * ctx.opacity;

    // ── Grid dimensions ──────────────────────────────────────────────────
    let col_count = table.columns.len().max(1);
    let row_count = table.rows.len();
    if row_count == 0 {
        // No rows → nothing to draw (the table box itself has no fill in v0).
        return;
    }

    // ── Column widths ────────────────────────────────────────────────────
    // Explicit `column.width` resolves to its px; AUTO columns (no width)
    // share the LEFTOVER width EQUALLY. Content-based auto-sizing is a LATER
    // unit — equal split is this unit's behavior.
    let mut explicit_w: Vec<Option<f64>> = Vec::with_capacity(col_count);
    for i in 0..col_count {
        let w = table
            .columns
            .get(i)
            .and_then(|c| c.width.as_ref())
            .and_then(|d| dim_to_px(d.value, &d.unit))
            .map(|v| v.max(0.0));
        explicit_w.push(w);
    }
    let sum_explicit: f64 = explicit_w.iter().filter_map(|w| *w).sum();
    let auto_count = explicit_w.iter().filter(|w| w.is_none()).count();
    // Content box width = table width minus the two outer padding insets and
    // the interior gaps (matches the frame content-box convention).
    let total_gap_w = gap * (col_count.saturating_sub(1)) as f64;
    let leftover = (table_w - sum_explicit - total_gap_w - 2.0 * pad).max(0.0);
    let auto_w = if auto_count > 0 {
        leftover / auto_count as f64
    } else {
        0.0
    };
    let col_widths: Vec<f64> = explicit_w
        .iter()
        .map(|w| w.unwrap_or(auto_w).max(0.0))
        .collect();

    // Left edge of each column (content-box left = origin + pad).
    let mut col_left: Vec<f64> = Vec::with_capacity(col_count);
    let mut cursor = origin_x + pad;
    for w in &col_widths {
        col_left.push(cursor);
        cursor += w + gap;
    }

    // ── Row heights (equal split — content-based height is a LATER unit) ──
    let total_gap_h = gap * (row_count.saturating_sub(1)) as f64;
    let row_h = ((table_h - total_gap_h - 2.0 * pad) / row_count as f64).max(0.0);
    let mut row_top: Vec<f64> = Vec::with_capacity(row_count);
    let mut rcursor = origin_y + pad;
    for _ in 0..row_count {
        row_top.push(rcursor);
        rcursor += row_h + gap;
    }

    // ── Cell placement (HTML-table flow with an occupancy grid) ──────────
    // BTreeSet keyed by (row, col) for deterministic occupancy tracking.
    let mut occupied: BTreeSet<(usize, usize)> = BTreeSet::new();

    for (r, row) in table.rows.iter().enumerate() {
        let mut col_cursor = 0usize;
        for cell in &row.cells {
            // Skip slots already covered by a previous cell's span.
            while col_cursor < col_count && occupied.contains(&(r, col_cursor)) {
                col_cursor += 1;
            }
            if col_cursor >= col_count {
                // Overflowing cell (already diagnosed at validate time); skip.
                break;
            }
            let cs = (cell.colspan.max(1) as usize).min(col_count - col_cursor);
            let rs = (cell.rowspan.max(1) as usize).min(row_count - r);

            // Cell rect: from column `col_cursor` left to the right edge of the
            // last spanned column (including interior gaps); similarly for rows.
            let left = col_left.get(col_cursor).copied().unwrap_or(origin_x + pad);
            let mut span_w = 0.0;
            for c in col_cursor..col_cursor + cs {
                span_w += col_widths.get(c).copied().unwrap_or(0.0);
            }
            // Add interior gaps between the spanned columns.
            span_w += gap * (cs.saturating_sub(1)) as f64;

            let top = row_top.get(r).copied().unwrap_or(origin_y + pad);
            let span_h = row_h * rs as f64 + gap * (rs.saturating_sub(1)) as f64;

            let rect = CellRect {
                x: left,
                y: top,
                w: span_w.max(0.0),
                h: span_h.max(0.0),
            };

            emit_cell(
                table,
                cell,
                &rect,
                pad,
                opacity,
                resolved,
                style_map,
                components,
                fonts,
                engine,
                commands,
                diagnostics,
                chains,
                field_ctx,
                ctx,
            );

            // Mark covered slots (clamped to the grid).
            for dr in 0..rs {
                for dc in 0..cs {
                    occupied.insert((r + dr, col_cursor + dc));
                }
            }
            col_cursor += cs;
        }
    }
}

/// Emit one cell: background fill, separate border, and clipped/aligned content.
#[allow(clippy::too_many_arguments)]
fn emit_cell(
    table: &TableNode,
    cell: &zenith_core::TableCell,
    rect: &CellRect,
    pad: f64,
    opacity: f64,
    resolved: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    components: &ComponentMap,
    fonts: &dyn FontProvider,
    engine: &RustybuzzEngine,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    chains: &ChainAssignments,
    field_ctx: &FieldCtx,
    ctx: RenderCtx,
) {
    // ── Background fill: cell.fill else table.fill (token color) ─────────
    let fill_prop: Option<&PropertyValue> = cell.fill.as_ref().or(table.fill.as_ref());
    if let Some(prop) = fill_prop
        && let Some(mut color) = resolve_property_color(prop, resolved, diagnostics, &table.id)
    {
        color.a = (color.a as f64 * opacity).round() as u8;
        commands.push(SceneCommand::FillRect {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: rect.h,
            color,
        });
    }

    // ── Separate border: each cell draws its own four edges independently ─
    let border_prop: Option<&PropertyValue> = cell.border.as_ref().or(table.border.as_ref());
    if let Some(prop) = border_prop
        && let Some(mut color) = resolve_property_color(prop, resolved, diagnostics, &table.id)
    {
        color.a = (color.a as f64 * opacity).round() as u8;
        // Width: cell.border-width else table.border-width else 1px.
        let bw_prop = cell
            .border_width
            .clone()
            .or_else(|| table.border_width.clone());
        let bw = resolve_property_dimension_px(&bw_prop, resolved, 1.0).max(0.0);
        if bw > 0.0 {
            let x0 = rect.x;
            let y0 = rect.y;
            let x1 = rect.x + rect.w;
            let y1 = rect.y + rect.h;
            // Four edges as independent stroke lines (centered stroke).
            for (ax, ay, bx, by) in [
                (x0, y0, x1, y0), // top
                (x0, y1, x1, y1), // bottom
                (x0, y0, x0, y1), // left
                (x1, y0, x1, y1), // right
            ] {
                commands.push(SceneCommand::StrokeLine {
                    x1: ax,
                    y1: ay,
                    x2: bx,
                    y2: by,
                    color,
                    stroke_width: bw,
                    stroke_dash: None,
                    stroke_gap: None,
                    stroke_linecap: None,
                });
            }
        }
    }

    // ── Content box (cell padding inset) ─────────────────────────────────
    let content_x = rect.x + pad;
    let content_y = rect.y + pad;
    let content_w = (rect.w - 2.0 * pad).max(0.0);
    let content_h = (rect.h - 2.0 * pad).max(0.0);

    // Alignment offsets (cell override else table default). Horizontal shifts
    // the child column within the content width; vertical within its height.
    let h_align = cell
        .h_align
        .as_deref()
        .or(table.h_align.as_deref())
        .unwrap_or("start");
    let v_align = cell
        .v_align
        .as_deref()
        .or(table.v_align.as_deref())
        .unwrap_or("top");

    // Clip cell content to the content box, then compile each child with a
    // RenderCtx translated to the content-box origin (plus the alignment
    // offset) so authored coordinate (0,0) lands at the cell's content corner.
    commands.push(SceneCommand::PushClip {
        x: content_x,
        y: content_y,
        w: content_w,
        h: content_h,
    });

    for child in &cell.children {
        // Per-child alignment: shift by the slack between the content box and
        // the child's declared width/height. A child with no declared box (or
        // align="start"/"top") gets a zero offset.
        let (cw, ch) = child_declared_box(child);
        let dx_align = match h_align {
            "center" => ((content_w - cw.unwrap_or(content_w)) / 2.0).max(0.0),
            "end" => (content_w - cw.unwrap_or(content_w)).max(0.0),
            _ => 0.0,
        };
        let dy_align = match v_align {
            "middle" => ((content_h - ch.unwrap_or(content_h)) / 2.0).max(0.0),
            "bottom" => (content_h - ch.unwrap_or(content_h)).max(0.0),
            _ => 0.0,
        };
        let child_ctx = RenderCtx {
            opacity,
            dx: content_x + dx_align,
            dy: content_y + dy_align,
            baseline_grid: ctx.baseline_grid,
        };
        let _ = compile_node(
            child,
            resolved,
            style_map,
            components,
            fonts,
            engine,
            commands,
            diagnostics,
            chains,
            field_ctx,
            child_ctx,
        );
    }

    commands.push(SceneCommand::PopClip);
}

/// The declared `(w, h)` of a cell child in pixels, when the kind carries a
/// box and the dimensions resolve. Used to compute alignment slack. Kinds
/// without a resolvable box yield `(None, None)`.
fn child_declared_box(node: &zenith_core::Node) -> (Option<f64>, Option<f64>) {
    use zenith_core::Node;
    let px =
        |d: &Option<zenith_core::Dimension>| d.as_ref().and_then(|d| dim_to_px(d.value, &d.unit));
    match node {
        Node::Rect(n) => (px(&n.w), px(&n.h)),
        Node::Ellipse(n) => (px(&n.w), px(&n.h)),
        Node::Text(n) => (px(&n.w), px(&n.h)),
        Node::Code(n) => (px(&n.w), px(&n.h)),
        Node::Image(n) => (px(&n.w), px(&n.h)),
        Node::Frame(n) => (px(&n.w), px(&n.h)),
        Node::Group(n) => (px(&n.w), px(&n.h)),
        Node::Field(n) => (px(&n.w), px(&n.h)),
        Node::Toc(n) => (px(&n.w), px(&n.h)),
        Node::Table(n) => (px(&n.w), px(&n.h)),
        Node::Line(_)
        | Node::Polygon(_)
        | Node::Polyline(_)
        | Node::Instance(_)
        | Node::Footnote(_)
        | Node::Unknown(_) => (None, None),
    }
}
