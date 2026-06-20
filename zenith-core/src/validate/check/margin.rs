//! Page-level book-margin (live-area) validation.
//!
//! Authors declare book live-area margins on a page (`margin-inner`,
//! `margin-outer`, `margin-top`, `margin-bottom`). Combined with the document
//! `mirror-margins` toggle and the page's 1-based index, these define the
//! parity-correct LIVE AREA rectangle for the page. This check compares each
//! direct page child node's authored bounding box against that rectangle and
//! emits a `margin.violation` ADVISORY when the node protrudes past any edge.
//!
//! Page parity (1-based index, in document source order):
//! - **recto** = ODD page (1, 3, 5 …) → binding on the LEFT.
//! - **verso** = EVEN page (2, 4, 6 …) → binding on the RIGHT.
//!
//! Live-area left edge:
//! - `mirror_margins == Some(true)`:
//!   - recto → `live_x = margin_inner` (inner/binding margin on the left);
//!   - verso → `live_x = margin_outer` (inner/binding margin on the right, so
//!     the OUTER margin is the left inset).
//! - otherwise → `live_x = margin_inner` on every page (uniform).
//!
//! In all cases:
//! - `live_y = margin_top`
//! - `live_w = page_w - margin_inner - margin_outer`
//! - `live_h = page_h - margin_top - margin_bottom`
//!
//! This validates the margin grid. It is purely advisory: margins are v0
//! metadata and do NOT auto-reposition nodes (that is master-page / flow-frame
//! work). Nodes with `role="guide"` are exempt (guides intentionally live in the
//! margins). The check is skipped entirely when any margin is absent.

use crate::ast::document::Page;
use crate::ast::value::dim_to_px;
use crate::diagnostics::Diagnostic;

use super::nodes::{node_bbox, node_id_and_span, node_role};

/// Fully-resolved book live-area rectangle in pixels: `(x, y, w, h)`.
struct LiveArea {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Resolve the parity-correct live area for `page`, given the page pixel
/// dimensions, the 1-based page index, and the document mirror toggle.
///
/// Returns `None` when any of the four margins is absent or resolves to a
/// non-pixel (pct/deg/unknown) unit — the caller then skips the check (no
/// panic, no diagnostic): margins are advisory and an unresolvable margin
/// simply yields no live area to validate against.
fn live_area(
    page: &Page,
    page_w: f64,
    page_h: f64,
    page_index_1based: usize,
    mirror_margins: bool,
) -> Option<LiveArea> {
    let inner_dim = page.margin_inner.as_ref()?;
    let outer_dim = page.margin_outer.as_ref()?;
    let top_dim = page.margin_top.as_ref()?;
    let bottom_dim = page.margin_bottom.as_ref()?;

    let inner = dim_to_px(inner_dim.value, &inner_dim.unit)?;
    let outer = dim_to_px(outer_dim.value, &outer_dim.unit)?;
    let top = dim_to_px(top_dim.value, &top_dim.unit)?;
    let bottom = dim_to_px(bottom_dim.value, &bottom_dim.unit)?;

    // recto = odd (1-based); verso = even.
    let is_recto = page_index_1based % 2 == 1;
    let left_inset = if mirror_margins && !is_recto {
        // verso: binding (inner) is on the RIGHT, so the OUTER margin insets the
        // left edge.
        outer
    } else {
        // recto, or non-mirrored: inner margin insets the left edge.
        inner
    };

    Some(LiveArea {
        x: left_inset,
        y: top,
        w: page_w - inner - outer,
        h: page_h - top - bottom,
    })
}

/// Validate every direct page child against the page's parity-correct live area.
///
/// `page_index_1based` is the page's position in `doc.body.pages` (1-based).
/// Deterministic: nodes are iterated in child order. Skipped when any margin is
/// absent/unresolvable, and skipped per-node for `role="guide"` nodes.
pub(super) fn check_margins(
    page: &Page,
    page_w: f64,
    page_h: f64,
    page_index_1based: usize,
    mirror_margins: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(area) = live_area(page, page_w, page_h, page_index_1based, mirror_margins) else {
        // Some margin is absent or unresolvable — nothing to validate against.
        return;
    };

    const EPSILON: f64 = 0.5;
    let parity = if page_index_1based % 2 == 1 {
        "recto"
    } else {
        "verso"
    };

    for node in &page.children {
        // Guides intentionally live in the margins; exempt them.
        if node_role(node) == Some("guide") {
            continue;
        }
        let Some((nx, ny, nw, nh)) = node_bbox(node, page_w, page_h) else {
            continue;
        };

        // Collect every violated edge so the advisory names which side(s).
        let mut edges: Vec<&str> = Vec::new();
        if nx < area.x - EPSILON {
            edges.push("left");
        }
        if ny < area.y - EPSILON {
            edges.push("top");
        }
        if nx + nw > area.x + area.w + EPSILON {
            edges.push("right");
        }
        if ny + nh > area.y + area.h + EPSILON {
            edges.push("bottom");
        }

        if edges.is_empty() {
            continue;
        }

        let (node_id, node_span) = node_id_and_span(node);
        diagnostics.push(Diagnostic::advisory(
            "margin.violation",
            format!(
                "node '{}' falls outside the {} page live area \
                 (x {:.0}, y {:.0}, w {:.0}, h {:.0}); crosses the {} margin edge(s)",
                node_id,
                parity,
                area.x,
                area.y,
                area.w,
                area.h,
                edges.join(", ")
            ),
            node_span,
            Some(node_id.to_owned()),
        ));
    }
}
