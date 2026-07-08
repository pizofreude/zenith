//! Depth-first collection of chain members and their content source.

use std::collections::BTreeMap;

use zenith_core::{Node, ResolvedToken, TextNode};

use crate::compile::util::resolve_geometry_px;

use super::types::Member;

/// Resolve a text node's explicit box to pixels, or `None` if any of
/// `x`/`y`/`w`/`h` is absent, a non-dimension, an unresolved token, or uses an
/// unsupported unit. Raw `(px)` dims are byte-identical to the prior read;
/// dimension token refs resolve via the token table.
fn member_box(
    text: &TextNode,
    resolved: &BTreeMap<String, ResolvedToken>,
) -> Option<(f64, f64, f64, f64)> {
    Some((
        resolve_geometry_px(text.x.as_ref(), resolved)?,
        resolve_geometry_px(text.y.as_ref(), resolved)?,
        resolve_geometry_px(text.w.as_ref(), resolved)?,
        resolve_geometry_px(text.h.as_ref(), resolved)?,
    ))
}

/// Depth-first walk in source order collecting `(chain_id → ordered members)`
/// plus the first span-bearing member node per chain (the content source) and the
/// block-style cascade scope (the page the source lives on) for that source.
pub(super) fn collect_chains<'a>(
    nodes: &'a [Node],
    page_block_styles: &'a [zenith_core::BlockStyle],
    resolved: &BTreeMap<String, ResolvedToken>,
    members: &mut BTreeMap<String, Vec<Member>>,
    source: &mut BTreeMap<String, &'a TextNode>,
    source_page_styles: &mut BTreeMap<String, &'a [zenith_core::BlockStyle]>,
) {
    for node in nodes {
        match node {
            Node::Text(t) => {
                if let Some(chain_id) = &t.chain {
                    // First span-bearing member becomes the content source. Record
                    // the page-scope block styles for that source's page, so a
                    // markdown chain resolves its block cascade against the page it
                    // is authored on (chains span pages, but the source is on one).
                    let has_spans = t.spans.iter().any(|s| !s.text.is_empty());
                    if has_spans && !source.contains_key(chain_id) {
                        source.insert(chain_id.clone(), t);
                        source_page_styles.insert(chain_id.clone(), page_block_styles);
                    }
                    if let Some((_x, _y, w, h)) = member_box(t, resolved) {
                        members.entry(chain_id.clone()).or_default().push(Member {
                            id: t.id.clone(),
                            w,
                            h,
                        });
                    }
                }
            }
            Node::Frame(f) => collect_chains(
                &f.children,
                page_block_styles,
                resolved,
                members,
                source,
                source_page_styles,
            ),
            Node::Group(g) => collect_chains(
                &g.children,
                page_block_styles,
                resolved,
                members,
                source,
                source_page_styles,
            ),
            Node::Table(t) => {
                for row in &t.rows {
                    for cell in &row.cells {
                        collect_chains(
                            &cell.children,
                            page_block_styles,
                            resolved,
                            members,
                            source,
                            source_page_styles,
                        );
                    }
                }
            }
            Node::Rect(_)
            | Node::Ellipse(_)
            | Node::Line(_)
            | Node::Code(_)
            | Node::Image(_)
            | Node::Polygon(_)
            | Node::Polyline(_)
            | Node::Path(_)
            | Node::Instance(_)
            | Node::Field(_)
            | Node::Footnote(_)
            | Node::Toc(_)
            | Node::Shape(_)
            | Node::Connector(_)
            | Node::Pattern(_)
            | Node::Chart(_)
            | Node::Light(_)
            | Node::Mesh(_)
            | Node::Unknown(_) => {}
        }
    }
}
