//! Linear/radial gradient fill draws (rect, rounded-rect, ellipse). Each
//! function pulls its fields from the matching [`SceneCommand`] variant and
//! fills `target` with a tiny-skia gradient shader under the shared
//! [`DrawCtx`]; behavior is byte-identical to the prior inline match arms.

use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect};
use zenith_scene::SceneCommand;

use super::super::commands::DrawCtx;
use super::super::gradient::gradient_shader;
use super::super::paths::{build_rounded_rect_path, clip_mask, intersect_rects};

pub(in crate::tiny_skia) fn fill_rect_gradient(
    target: &mut Pixmap,
    ctx: DrawCtx,
    cmd: &SceneCommand,
) {
    let SceneCommand::FillRectGradient {
        x,
        y,
        w,
        h,
        gradient,
    } = cmd
    else {
        return;
    };
    if !x.is_finite()
        || !y.is_finite()
        || !w.is_finite()
        || !h.is_finite()
        || *w <= 0.0
        || *h <= 0.0
    {
        return;
    }

    let effective_clip = ctx.effective_clip;
    if intersect_rects((*x, *y, x + w, y + h), effective_clip).is_none() {
        return;
    }

    let Some(rect) = Rect::from_xywh(*x as f32, *y as f32, *w as f32, *h as f32) else {
        return;
    };
    let path = PathBuilder::from_rect(rect);

    let mask = match clip_mask(effective_clip, ctx.width, ctx.height) {
        None => return,
        Some(m) => m,
    };

    let Some(shader) = gradient_shader(*x, *y, *w, *h, gradient) else {
        return;
    };
    let paint = Paint {
        shader,
        anti_alias: true,
        ..Default::default()
    };

    target.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        ctx.current_ts,
        mask.as_ref(),
    );
}

pub(in crate::tiny_skia) fn fill_rounded_rect_gradient(
    target: &mut Pixmap,
    ctx: DrawCtx,
    cmd: &SceneCommand,
) {
    let SceneCommand::FillRoundedRectGradient {
        x,
        y,
        w,
        h,
        radius,
        radii,
        gradient,
    } = cmd
    else {
        return;
    };
    if !x.is_finite()
        || !y.is_finite()
        || !w.is_finite()
        || !h.is_finite()
        || !radius.is_finite()
        || *w <= 0.0
        || *h <= 0.0
    {
        return;
    }

    let effective_clip = ctx.effective_clip;
    if intersect_rects((*x, *y, x + w, y + h), effective_clip).is_none() {
        return;
    }

    // Per-corner radii override uniform radius when present.
    let corner_radii = radii.map_or([*radius as f32; 4], |a| a.map(|v| v as f32));
    let Some(path) =
        build_rounded_rect_path(*x as f32, *y as f32, *w as f32, *h as f32, corner_radii)
    else {
        return;
    };

    let mask = match clip_mask(effective_clip, ctx.width, ctx.height) {
        None => return,
        Some(m) => m,
    };

    let Some(shader) = gradient_shader(*x, *y, *w, *h, gradient) else {
        return;
    };
    let paint = Paint {
        shader,
        anti_alias: true,
        ..Default::default()
    };

    target.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        ctx.current_ts,
        mask.as_ref(),
    );
}

pub(in crate::tiny_skia) fn fill_ellipse_gradient(
    target: &mut Pixmap,
    ctx: DrawCtx,
    cmd: &SceneCommand,
) {
    let SceneCommand::FillEllipseGradient {
        x,
        y,
        w,
        h,
        rx,
        ry,
        gradient,
    } = cmd
    else {
        return;
    };
    if !x.is_finite()
        || !y.is_finite()
        || !w.is_finite()
        || !h.is_finite()
        || *w <= 0.0
        || *h <= 0.0
    {
        return;
    }

    // Compute oval bounding box from rx/ry semi-axes (or node bbox).
    let ow = rx.map_or(*w, |r| r * 2.0);
    let oh = ry.map_or(*h, |r| r * 2.0);
    let ox = x + (w - ow) / 2.0;
    let oy = y + (h - oh) / 2.0;

    let effective_clip = ctx.effective_clip;

    // Early-out: skip if the ellipse bbox is entirely outside the clip.
    if intersect_rects((ox, oy, ox + ow, oy + oh), effective_clip).is_none() {
        return;
    }

    let Some(rect) = Rect::from_xywh(ox as f32, oy as f32, ow as f32, oh as f32) else {
        return;
    };
    let Some(path) = PathBuilder::from_oval(rect) else {
        return;
    };

    let mask = match clip_mask(effective_clip, ctx.width, ctx.height) {
        None => return,
        Some(m) => m,
    };

    let Some(shader) = gradient_shader(*x, *y, *w, *h, gradient) else {
        return;
    };
    let paint = Paint {
        shader,
        anti_alias: true,
        ..Default::default()
    };

    target.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        ctx.current_ts,
        mask.as_ref(),
    );
}
