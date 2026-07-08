//! The public `compile_text` entry and the `overflow="autofit"` shrink-to-fit
//! search. Both are thin wrappers over [`compile_text_sized`](super::sized::compile_text_sized).

use zenith_core::{Diagnostic, Dimension, PropertyValue, TextNode, Unit};

use crate::compile::RenderCtx;
use crate::compile::text::ctx::TextCompileEnv;
use crate::compile::text::measure::font_size_px;
use crate::compile::util::{resolve_geometry_px, resolve_property_dimension_px};
use crate::ir::SceneCommand;

use super::sized::compile_text_sized;

/// Compile a `text` leaf node.
///
/// This is the public entry point. It is a thin BLACK-BOX wrapper around
/// [`compile_text_sized`](super::sized::compile_text_sized) (which carries every
/// layout path verbatim):
///
/// - For any node whose `overflow` is NOT `"autofit"` it is a pure pass-through
///   — it forwards every argument unchanged to `compile_text_sized`, so the
///   emitted [`SceneCommand`] stream is BYTE-IDENTICAL to before this attribute
///   existed (the determinism gate).
/// - For `overflow="autofit"` it drives `compile_text_sized` at TRIAL font
///   sizes (into throwaway buffers) to find the LARGEST size in
///   `[floor, declared]` whose content fits the box height, then performs the
///   single real emit at that size. See [`compile_text_autofit`].
///
/// Returns the laid-out content height in pixels (`line_count * line_height`).
pub(in crate::compile) fn compile_text(
    text: &TextNode,
    env: TextCompileEnv,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    ctx: RenderCtx,
) -> f64 {
    // Emit, then (only when the node opts out of selectable text) downgrade this
    // node's glyph runs to outlines. `selectable` is purely a PDF render concern,
    // so it never affects layout — it is applied as a post-pass over exactly the
    // commands this node produced. Default (`None`/`Some(true)`) is byte-identical.
    let start = commands.len();
    let height = if text.overflow.as_deref() != Some("autofit") {
        // Pass-through: byte-identical command stream for every non-autofit node.
        compile_text_sized(text, env, commands, diagnostics, ctx)
    } else {
        compile_text_autofit(text, env, commands, diagnostics, ctx)
    };
    if text.selectable == Some(false) {
        crate::compile::text::shape::mark_runs_unselectable(&mut commands[start..]);
    }
    height
}

/// PowerPoint-style shrink-to-fit search for an `overflow="autofit"` text node.
///
/// Drives [`compile_text_sized`](super::sized::compile_text_sized) at trial
/// integer-px font sizes (into throwaway command/diagnostic buffers) to find the
/// LARGEST size in `[floor, declared]` whose content fits the box height, then
/// performs ONE real emit at that size.
///
/// - The declared node font size (px) is the search ceiling; `font-size-min`
///   (token → dimension) is the floor. When `font-size-min` is absent the floor
///   defaults to `(declared * 0.5).max(8.0)`.
/// - Both `box_w` and `box_h` must resolve; if either is missing autofit cannot
///   measure, so it falls back to a single `compile_text_sized` call with the
///   node's `overflow` left as-is (no crash, no silent skip).
/// - A trial at size `fs` FITS iff its throwaway diagnostics contain NO
///   `text.fit_failed` whose subject is this node id (the trial sets
///   `overflow="fit"` so the inner height-overflow check reports exactly that).
/// - The search is a DOWNWARD linear scan from `declared` to `floor` over
///   integer px, breaking on the first fit (deterministic: same inputs → same
///   `fs`).
/// - If some size fits, the real emit uses that size with `overflow="clip"` so
///   the fitted text renders clip-safe and emits NO `fit_failed`. If NONE fits
///   (even at the floor) the real emit uses the floor with `overflow="fit"`, so
///   the genuine `text.fit_failed` is emitted at the floor (PowerPoint gives up
///   too).
///
/// v0 limitation: a span carrying its OWN explicit `font-size` does not scale —
/// only the node-level font size drives inheriting spans (the typical single-
/// span title inherits, so it scales).
fn compile_text_autofit(
    text: &TextNode,
    env: TextCompileEnv,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
    ctx: RenderCtx,
) -> f64 {
    // Require both box dimensions to measure fit; otherwise fall back to a
    // single sized compile with overflow untouched (documented; no crash).
    let box_w = resolve_geometry_px(text.w.as_ref(), env.resolved);
    let box_h = resolve_geometry_px(text.h.as_ref(), env.resolved);
    let (Some(_bw), Some(_bh)) = (box_w, box_h) else {
        return compile_text_sized(text, env, commands, diagnostics, ctx);
    };

    // Resolve the declared node font size (px) — the search ceiling — and the
    // floor from `font-size-min`, defaulting to `(declared * 0.5).max(8.0)`.
    let declared = f64::from(font_size_px(text, env.resolved, env.style_map));
    let floor = resolve_property_dimension_px(
        text.font_size_min.as_ref(),
        env.resolved,
        (declared * 0.5).max(8.0),
    );
    // Integer-px search bounds. Clamp the floor at/below the ceiling.
    let ceil_px = declared.floor().max(1.0) as i64;
    let floor_px = floor.floor().max(1.0).min(declared.floor().max(1.0)) as i64;

    // Build a trial/real clone at size `fs` with the given overflow.
    let clone_sized = |fs: f64, ov: &str| -> TextNode {
        let mut t = text.clone();
        t.font_size = Some(PropertyValue::Dimension(Dimension {
            value: fs,
            unit: Unit::Px,
        }));
        t.overflow = Some(ov.to_owned());
        t
    };

    // Does a trial at `fs` fit? Compile into throwaway buffers under
    // overflow="fit" and check for a `text.fit_failed` naming THIS node.
    let fits = |fs: f64| -> bool {
        let trial = clone_sized(fs, "fit");
        let mut throwaway_cmds: Vec<SceneCommand> = Vec::new();
        let mut throwaway_diags: Vec<Diagnostic> = Vec::new();
        compile_text_sized(&trial, env, &mut throwaway_cmds, &mut throwaway_diags, ctx);
        !throwaway_diags.iter().any(|d| {
            d.code == "text.fit_failed" && d.subject_id.as_deref() == Some(text.id.as_str())
        })
    };

    // Downward linear scan from the ceiling to the floor; break on first fit.
    let mut fitted: Option<i64> = None;
    let mut fs = ceil_px;
    while fs >= floor_px {
        if fits(fs as f64) {
            fitted = Some(fs);
            break;
        }
        fs -= 1;
    }

    // Real emit: the fitted size clipped-safe, or the floor with overflow="fit"
    // so the genuine fit_failed surfaces at the floor.
    let (real_fs, real_ov) = match fitted {
        Some(fs) => (fs as f64, "clip"),
        None => (floor_px as f64, "fit"),
    };
    let real = clone_sized(real_fs, real_ov);
    compile_text_sized(&real, env, commands, diagnostics, ctx)
}
