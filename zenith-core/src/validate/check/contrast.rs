//! WCAG 2.2 contrast advisory check.
//!
//! Compares text-node fills against the page background colour and emits a
//! `contrast.low` warning when the ratio is below the WCAG AA threshold.

use std::collections::BTreeMap;

use crate::ast::node::Node;
use crate::ast::style::Style;
use crate::ast::value::{PropertyValue, dim_to_px};
use crate::color::{contrast_ratio, parse_rgb};
use crate::diagnostics::Diagnostic;
use crate::tokens::{ResolvedToken, ResolvedValue};

/// Recursively check text nodes against the page background for WCAG AA contrast.
///
/// # v0 Limitations
/// - Only compares against the PAGE background color; an intervening rect or
///   scrim the text visually sits on is NOT detected or used.
/// - Per-span fills (TextSpan.fill) are NOT individually checked; the node-level
///   `fill` is used as a proxy for all spans.
/// - Fill / font-size / font-weight are resolved from the node's direct
///   property when present, otherwise from the referenced `style` block's
///   matching property (`fill` / `font-size` / `font-weight`). A node with
///   neither a direct nor a style-inherited fill is simply skipped. Per-span
///   fills (TextSpan.fill) are still not individually consulted here.
pub(super) fn check_text_contrast(
    node: &Node,
    page_bg_rgb: Option<(u8, u8, u8)>,
    resolved_tokens: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // If we don't know the page background we cannot compute contrast — bail.
    let Some(bg_rgb) = page_bg_rgb else {
        return;
    };

    match node {
        Node::Text(t) => {
            // Effective property = direct node property, falling back to the
            // referenced style block's matching property when the node omits it.
            let style_prop = |key: &str| -> Option<&PropertyValue> {
                style_map
                    .get(t.style.as_deref()?)
                    .and_then(|s| s.properties.get(key))
            };

            // Resolve the text fill color from a TokenRef → Color token.
            // If no fill is set or it doesn't resolve to a color, skip.
            let text_rgb = match t.fill.as_ref().or_else(|| style_prop("fill")) {
                Some(PropertyValue::TokenRef(id)) => {
                    resolved_tokens.get(id.as_str()).and_then(|rt| {
                        if let ResolvedValue::Color(hex) = &rt.value {
                            parse_rgb(hex)
                        } else {
                            None
                        }
                    })
                }
                // Literal / Dimension fills are caught as raw_visual_literal errors
                // elsewhere; no need to chase them here.
                _ => None,
            };

            let Some(fg_rgb) = text_rgb else {
                return;
            };

            // Resolve font-size in px (default 16.0 px when absent).
            let size_px: f64 = t
                .font_size
                .as_ref()
                .or_else(|| style_prop("font-size"))
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
                .unwrap_or(16.0);

            // Resolve font-weight as u32 (default 400 when absent).
            let weight: u32 = t
                .font_weight
                .as_ref()
                .or_else(|| style_prop("font-weight"))
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
                .unwrap_or(400);

            // WCAG AA large text: >= 24 px OR >= 18.66 px bold (>= 700).
            let is_large = size_px >= 24.0 || (size_px >= 18.66 && weight >= 700);
            let threshold = if is_large { 3.0_f64 } else { 4.5_f64 };

            let ratio = contrast_ratio(fg_rgb, bg_rgb);

            if ratio < threshold {
                diagnostics.push(Diagnostic::warning(
                    "contrast.low",
                    format!(
                        "text '{}': contrast ratio {:.2}:1 of fill on page background \
                         is below WCAG AA ({:.1}:1)",
                        t.id, ratio, threshold
                    ),
                    t.source_span,
                    Some(t.id.clone()),
                ));
            }
        }

        // Recurse into container nodes, passing the same page_bg through.
        // Group and Frame children may contain text nodes.
        Node::Group(g) => {
            for child in &g.children {
                check_text_contrast(child, page_bg_rgb, resolved_tokens, style_map, diagnostics);
            }
        }
        Node::Frame(f) => {
            for child in &f.children {
                check_text_contrast(child, page_bg_rgb, resolved_tokens, style_map, diagnostics);
            }
        }

        // All other leaf node types carry no text — nothing to check.
        _ => {}
    }
}
