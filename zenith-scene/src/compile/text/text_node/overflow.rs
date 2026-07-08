//! Post-emit overflow diagnostics for a sized `text` node: the hard
//! `text.fit_failed` (overflow="fit") and the advisory `text.overflow`
//! (overflow="clip"/default). Lifted verbatim out of `compile_text_sized`.

use zenith_core::{Diagnostic, TextNode};

/// Inputs to the overflow checks — the laid-out geometry and box dimensions.
pub(super) struct OverflowCheck<'a> {
    /// The node being checked (for `overflow`, `id`, and `source_span`).
    pub text: &'a TextNode,
    /// Resolved box width in px, when available.
    pub box_w_opt: Option<f64>,
    /// Resolved box height in px, when available.
    pub box_h_opt: Option<f64>,
    /// Line count after emit (1 on the fast path, wrapped count otherwise).
    pub fit_line_count: usize,
    /// Shared per-line height in px.
    pub first_line_height: f64,
    /// Whether the wrap path was taken.
    pub needs_wrap: bool,
    /// Total single-line advance width in px.
    pub total_advance: f64,
    /// Resolved node font size in px.
    pub font_size: f32,
}

/// Emit the overflow="fit" hard error and the overflow="clip" advisory, exactly
/// as the inline tail of `compile_text_sized` did.
pub(super) fn check_text_overflow(c: OverflowCheck, diagnostics: &mut Vec<Diagnostic>) {
    let OverflowCheck {
        text,
        box_w_opt,
        box_h_opt,
        fit_line_count,
        first_line_height,
        needs_wrap,
        total_advance,
        font_size,
    } = c;

    // ── overflow="fit" check ──────────────────────────────────────────
    // Hard-fail when the text content does not fit the declared box.
    // Only evaluated when BOTH box_w and box_h are present — without a
    // complete box we cannot determine fit and silently skip the check.
    // Glyph runs are STILL emitted above; this diagnostic rides alongside.
    if text.overflow.as_deref() == Some("fit")
        && let (Some(box_w), Some(box_h)) = (box_w_opt, box_h_opt)
    {
        const EPSILON: f64 = 0.5;
        let content_height = fit_line_count as f64 * first_line_height;

        // Height overflow: wrapped text is taller than the box.
        let height_overflow = content_height > box_h + EPSILON;

        // Word-wider-than-box: a single word in a single-word line
        // exceeds box_w (wrapping cannot help). In the wrap path, any
        // line with one word whose content_w > box_w is unwrappable.
        // In the single-line path (needs_wrap=false), total_advance ≤
        // box_w by definition, so no word can be wider.
        let word_overflow = if needs_wrap {
            // Re-check each token's advance against box_w. Any token
            // wider than box_w is unwrappable.  We use total_advance
            // as a fast proxy: if total_advance > box_w AND there is
            // exactly one shaped span whose run.advance_width > box_w
            // the single word is wider than the box. More precisely,
            // we need to check the per-word tokens; those were consumed
            // inside the wrap block, so we detect this via the fact
            // that any line with content_w > box_w must contain a lone
            // word wider than box_w (the greedy packer would have split
            // it if it could). The wrap path set fit_line_count from
            // lines.len(), so checking content_height already catches
            // the height dimension; the word-wider check is an
            // additional width dimension. A simpler heuristic: if
            // total_advance > box_w AND fit_line_count==1 the whole
            // text landed on one line only because no word break was
            // possible — meaning one word >= box_w width.
            fit_line_count == 1 && total_advance > box_w + EPSILON
        } else {
            false // fast path: total_advance ≤ box_w by definition
        };

        if height_overflow || word_overflow {
            diagnostics.push(Diagnostic::error(
                "text.fit_failed",
                format!(
                    "text '{}': content does not fit its box (overflow=\"fit\"): \
                         at {:.0}px font-size it needs ~{:.0}px height in a {:.0}px-tall box \
                         (or a word wider than the {:.0}px box width)",
                    text.id, font_size as f64, content_height, box_h, box_w
                ),
                text.source_span,
                Some(text.id.clone()),
            ));
        }
    }

    // ── overflow="clip" warning ────────────────────────────────────────
    // Clip mode (the default when `overflow` is absent) silently truncates
    // ink at the box edge, which can hide content. Surface a non-fatal
    // warning so the author knows text was clipped — mirrors the fit check
    // but advisory, never a hard fail. `overflow="visible"` opts out (the
    // overflow is intentional) and `overflow="fit"` is handled above.
    if matches!(text.overflow.as_deref(), None | Some("clip"))
        && let (Some(box_w), Some(box_h)) = (box_w_opt, box_h_opt)
    {
        const EPSILON: f64 = 0.5;
        let content_height = fit_line_count as f64 * first_line_height;
        let height_overflow = content_height > box_h + EPSILON;
        let word_overflow = needs_wrap && fit_line_count == 1 && total_advance > box_w + EPSILON;
        if height_overflow || word_overflow {
            diagnostics.push(Diagnostic::warning(
                "text.overflow",
                format!(
                    "text '{}': content is clipped at the box edge \
                     (overflow=\"clip\"): at {:.0}px font-size it needs ~{:.0}px height in a {:.0}px-tall box; \
                     preserve declared type scale first by increasing the box height, moving nearby nodes, \
                     splitting content, or reflowing layout. Shrink type only when intended or geometry is constrained.",
                    text.id, font_size as f64, content_height, box_h
                ),
                text.source_span,
                Some(text.id.clone()),
            ));
        }
    }
}
