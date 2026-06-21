//! Scene-command → PDF content-operator translation.
//!
//! [`translate`] walks the scene display list once and emits a single page
//! content stream, accumulating the page resources it references (alpha
//! ExtGStates, axial-gradient shadings, image XObjects) into [`PageResources`]
//! for the document writer to materialize.
//!
//! Every [`SceneCommand`] variant is handled explicitly — no wildcard arm
//! silently drops a primitive. The two honest v0 limitations are matched
//! explicitly and documented at their arms: blurred drop-shadows (no vector PDF
//! equivalent — the *content* is still drawn, only the blur is skipped) and
//! color-bitmap (emoji) glyphs (omitted; the print scenarios use none).
//!
//! Per-pixel color `filter` brackets ARE honored: PDF has no per-pixel filter
//! primitive, so [`translate`] captures the bracketed commands, rasterizes them
//! to straight-alpha RGBA, applies the filters, and embeds the cropped result as
//! an image XObject (see [`emit_filtered_region`]). This is no longer a no-op.

use pdf_writer::Content;
use zenith_core::{AssetProvider, FontProvider};
use zenith_scene::{Color, FilterSpec, FitMode, ImageClip, Scene, SceneCommand, StrokeAlign};

use super::color;
use super::geometry::{GlyphPen, ellipse_path, poly_path, rounded_rect_path};
use super::gradient::{AxialGradient, resolve as resolve_gradient};
use super::image::{DecodedImage, decode_for_pdf, decoded_image_from_straight_rgba};

/// Page-level resources accumulated during [`translate`], keyed for
/// deduplication and emitted in a deterministic order by the document writer.
#[derive(Default)]
pub(super) struct PageResources {
    /// Distinct fill/stroke alpha values (< 255) seen, each becoming one
    /// `/ExtGState` with `ca` + `CA`. Sorted, deduped → stable resource names.
    pub(super) alphas: Vec<u8>,
    /// Axial gradient shadings, in first-seen (draw) order. Index = resource id.
    pub(super) gradients: Vec<AxialGradient>,
    /// Decoded image XObjects, in first-seen order. Index = resource id.
    pub(super) images: Vec<DecodedImage>,
}

impl PageResources {
    /// Intern an alpha byte, returning its stable `ExtGState` resource index.
    fn intern_alpha(&mut self, a: u8) -> usize {
        match self.alphas.binary_search(&a) {
            Ok(i) => i,
            Err(i) => {
                self.alphas.insert(i, a);
                i
            }
        }
    }
}

/// The resource-name prefixes. Names are `<prefix><index>`, e.g. `ga2`, `sh0`,
/// `im1` — ASCII only, deterministic.
pub(super) const ALPHA_PREFIX: &str = "ga";
pub(super) const SHADING_PREFIX: &str = "sh";
pub(super) const IMAGE_PREFIX: &str = "im";

/// Translate `scene` into a single content stream plus the [`PageResources`] it
/// references. `fonts` resolves glyph outlines; `assets` resolves image bytes.
pub(super) fn translate(
    scene: &Scene,
    fonts: &dyn FontProvider,
    assets: &dyn AssetProvider,
) -> (Content, PageResources) {
    let mut content = Content::new();
    let mut res = PageResources::default();

    // Initial CTM: flip the y axis so scene (top-left, y-down) maps to PDF
    // user space (bottom-left, y-up). 1 scene px == 1 PDF unit.
    content.transform([1.0, 0.0, 0.0, -1.0, 0.0, scene.height as f32]);

    let page = (scene.width, scene.height);

    // Filter-capture buffer. A color-`filter` bracket has no per-pixel vector PDF
    // primitive, so while one is active we BUFFER (clone) the bracketed commands
    // instead of emitting them, then rasterize+filter+embed the whole region at
    // EndFilter. `None` means draws emit directly. Filters are leaf-only and do
    // not nest, so a single active capture suffices.
    let mut capture: Option<(Vec<FilterSpec>, Vec<SceneCommand>)> = None;

    for cmd in &scene.commands {
        match cmd {
            // Open a capture (non-empty filters, none already active). Empty
            // filters or a nested begin are no-ops: nothing is captured, so the
            // inner commands emit normally.
            SceneCommand::BeginFilter { filters } => {
                if capture.is_none() && !filters.is_empty() {
                    capture = Some((filters.clone(), Vec::new()));
                }
                continue;
            }
            // Close the active capture: rasterize the buffered region, apply the
            // filters, and embed it. With no active capture this is a no-op.
            SceneCommand::EndFilter => {
                if let Some((filters, buffered)) = capture.take() {
                    emit_filtered_region(
                        &mut content,
                        &mut res,
                        &buffered,
                        &filters,
                        page,
                        fonts,
                        assets,
                    );
                }
                continue;
            }
            // Any other command: buffer a clone while capturing, else emit now.
            _ => {
                if let Some((_, buffered)) = capture.as_mut() {
                    buffered.push(cmd.clone());
                } else {
                    emit_command(&mut content, &mut res, cmd, page, fonts, assets);
                }
            }
        }
    }

    (content, res)
}

/// Apply the fill-alpha ExtGState for `color` if it is non-opaque (interning the
/// alpha into `res`). Returns nothing; emits `/ga<i> gs` when needed.
fn apply_alpha(content: &mut Content, res: &mut PageResources, color: &Color) {
    if color.a == 255 {
        return;
    }
    let idx = res.intern_alpha(color.a);
    content.set_parameters(name(ALPHA_PREFIX, idx).as_name());
}

fn emit_command(
    content: &mut Content,
    res: &mut PageResources,
    cmd: &SceneCommand,
    page: (f64, f64),
    fonts: &dyn FontProvider,
    assets: &dyn AssetProvider,
) {
    match cmd {
        // ── Filled shapes ─────────────────────────────────────────────────
        SceneCommand::FillRect { x, y, w, h, color } => {
            if !rect_ok(*x, *y, *w, *h) {
                return;
            }
            content.save_state();
            apply_alpha(content, res, color);
            color::set_fill(content, color);
            content.rect(*x as f32, *y as f32, *w as f32, *h as f32);
            content.fill_nonzero();
            content.restore_state();
        }

        SceneCommand::StrokeRect {
            x,
            y,
            w,
            h,
            color,
            stroke_width,
            // PDF v0 renders solid strokes only; dash params are intentionally ignored here.
            ..
        } => {
            if !rect_ok(*x, *y, *w, *h) || !finite(*stroke_width) {
                return;
            }
            content.save_state();
            apply_alpha(content, res, color);
            color::set_stroke(content, color);
            content.set_line_width(*stroke_width as f32);
            content.rect(*x as f32, *y as f32, *w as f32, *h as f32);
            content.stroke();
            content.restore_state();
        }

        SceneCommand::FillRoundedRect {
            x,
            y,
            w,
            h,
            radius,
            radii,
            color,
        } => {
            if !rect_ok(*x, *y, *w, *h) || !finite(*radius) {
                return;
            }
            let corner_radii = radii.unwrap_or([*radius; 4]);
            content.save_state();
            apply_alpha(content, res, color);
            color::set_fill(content, color);
            rounded_rect_path(content, *x, *y, *w, *h, corner_radii);
            content.fill_nonzero();
            content.restore_state();
        }

        SceneCommand::StrokeRoundedRect {
            x,
            y,
            w,
            h,
            radius,
            radii,
            color,
            stroke_width,
            // PDF v0 renders solid strokes only; dash params are intentionally ignored here.
            ..
        } => {
            if !rect_ok(*x, *y, *w, *h) || !finite(*radius) || !finite(*stroke_width) {
                return;
            }
            let corner_radii = radii.unwrap_or([*radius; 4]);
            content.save_state();
            apply_alpha(content, res, color);
            color::set_stroke(content, color);
            content.set_line_width(*stroke_width as f32);
            rounded_rect_path(content, *x, *y, *w, *h, corner_radii);
            content.stroke();
            content.restore_state();
        }

        SceneCommand::FillEllipse {
            x,
            y,
            w,
            h,
            rx,
            ry,
            color,
        } => {
            if !rect_ok(*x, *y, *w, *h) {
                return;
            }
            content.save_state();
            apply_alpha(content, res, color);
            color::set_fill(content, color);
            ellipse_path(content, *x, *y, *w, *h, *rx, *ry);
            content.fill_nonzero();
            content.restore_state();
        }

        SceneCommand::StrokeEllipse {
            x,
            y,
            w,
            h,
            rx,
            ry,
            color,
            stroke_width,
            // PDF v0 renders solid strokes only; dash params are intentionally ignored here.
            ..
        } => {
            if !rect_ok(*x, *y, *w, *h) || !finite(*stroke_width) {
                return;
            }
            content.save_state();
            apply_alpha(content, res, color);
            color::set_stroke(content, color);
            content.set_line_width(*stroke_width as f32);
            ellipse_path(content, *x, *y, *w, *h, *rx, *ry);
            content.stroke();
            content.restore_state();
        }

        SceneCommand::StrokeLine {
            x1,
            y1,
            x2,
            y2,
            color,
            stroke_width,
            // PDF v0 renders solid strokes only; dash params are intentionally ignored here.
            ..
        } => {
            if !finite(*x1)
                || !finite(*y1)
                || !finite(*x2)
                || !finite(*y2)
                || !finite(*stroke_width)
            {
                return;
            }
            content.save_state();
            apply_alpha(content, res, color);
            color::set_stroke(content, color);
            content.set_line_width(*stroke_width as f32);
            content.move_to(*x1 as f32, *y1 as f32);
            content.line_to(*x2 as f32, *y2 as f32);
            content.stroke();
            content.restore_state();
        }

        SceneCommand::FillPolygon {
            points,
            color,
            even_odd,
        } => {
            if points.len() < 6 || points.iter().any(|v| !v.is_finite()) {
                return;
            }
            content.save_state();
            apply_alpha(content, res, color);
            color::set_fill(content, color);
            if poly_path(content, points, true) {
                if *even_odd {
                    content.fill_even_odd();
                } else {
                    content.fill_nonzero();
                }
            } else {
                content.end_path();
            }
            content.restore_state();
        }

        SceneCommand::StrokePolyline {
            points,
            color,
            stroke_width,
            closed,
            align,
            fill_even_odd,
        } => {
            if points.len() < 4 || points.iter().any(|v| !v.is_finite()) || !finite(*stroke_width) {
                return;
            }

            // Aligned stroke (Inside/Outside on a CLOSED polygon): draw at 2× width
            // and clip to the fill region (Inside) or its complement (Outside) so a
            // full-width stroke sits flush against the boundary. Center / open paths
            // are unchanged.
            let aligned = *closed && !matches!(align, StrokeAlign::Center);

            content.save_state();
            apply_alpha(content, res, color);
            color::set_stroke(content, color);

            if aligned {
                // 1. Install the alignment clip from the polygon fill path.
                match align {
                    StrokeAlign::Inside => {
                        // Clip = polygon interior (per fill rule).
                        if !poly_path(content, points, true) {
                            content.end_path();
                            content.restore_state();
                            return;
                        }
                        if *fill_even_odd {
                            content.clip_even_odd();
                        } else {
                            content.clip_nonzero();
                        }
                        content.end_path();
                    }
                    StrokeAlign::Outside => {
                        // Clip = (generous outer rect) minus polygon interior, via the
                        // even-odd rule on the combined subpaths → the exterior region.
                        let (pw, ph) = page;
                        let m = pw.max(ph).max(1.0); // generous margin past the page
                        content.move_to(-m as f32, -m as f32);
                        content.line_to((pw + m) as f32, -m as f32);
                        content.line_to((pw + m) as f32, (ph + m) as f32);
                        content.line_to(-m as f32, (ph + m) as f32);
                        content.close_path();
                        if !poly_path(content, points, true) {
                            content.end_path();
                            content.restore_state();
                            return;
                        }
                        content.clip_even_odd();
                        content.end_path();
                    }
                    // `aligned` is only true when align != Center, so this arm is dead;
                    // kept (no wildcard) for exhaustiveness. A no-op is the safe fallback
                    // — it simply leaves the clip unchanged.
                    StrokeAlign::Center => {}
                }
                // 2. Stroke the path at 2× width inside the clip.
                content.set_line_width((*stroke_width * 2.0) as f32);
                if poly_path(content, points, true) {
                    content.stroke();
                } else {
                    content.end_path();
                }
            } else {
                content.set_line_width(*stroke_width as f32);
                if poly_path(content, points, *closed) {
                    content.stroke();
                } else {
                    content.end_path();
                }
            }
            content.restore_state();
        }

        // ── Gradient fills ────────────────────────────────────────────────
        //
        // PDF v0 limitation: radial gradients have no axial-shading equivalent
        // and are degraded to a solid fill using the gradient's first stop color,
        // consistent with the v0 shadow-blur and SVG-asset omissions above.
        SceneCommand::FillRectGradient {
            x,
            y,
            w,
            h,
            gradient,
        } => {
            if !rect_ok(*x, *y, *w, *h) {
                return;
            }
            if gradient.radial {
                // Radial PDF degrade: solid fill with first stop color.
                if let Some(first) = gradient.stops.first() {
                    content.save_state();
                    apply_alpha(content, res, &first.color);
                    color::set_fill(content, &first.color);
                    content.rect(*x as f32, *y as f32, *w as f32, *h as f32);
                    content.fill_nonzero();
                    content.restore_state();
                }
            } else if let Some(g) = resolve_gradient(*x, *y, *w, *h, gradient) {
                let id = push_gradient(res, g);
                content.save_state();
                content.rect(*x as f32, *y as f32, *w as f32, *h as f32);
                content.clip_nonzero();
                content.end_path();
                content.shading(name(SHADING_PREFIX, id).as_name());
                content.restore_state();
            }
        }

        SceneCommand::FillRoundedRectGradient {
            x,
            y,
            w,
            h,
            radius,
            radii,
            gradient,
        } => {
            if !rect_ok(*x, *y, *w, *h) || !finite(*radius) {
                return;
            }
            let corner_radii = radii.unwrap_or([*radius; 4]);
            if gradient.radial {
                // Radial PDF degrade: solid fill with first stop color.
                if let Some(first) = gradient.stops.first() {
                    content.save_state();
                    apply_alpha(content, res, &first.color);
                    color::set_fill(content, &first.color);
                    rounded_rect_path(content, *x, *y, *w, *h, corner_radii);
                    content.fill_nonzero();
                    content.restore_state();
                }
            } else if let Some(g) = resolve_gradient(*x, *y, *w, *h, gradient) {
                let id = push_gradient(res, g);
                content.save_state();
                rounded_rect_path(content, *x, *y, *w, *h, corner_radii);
                content.clip_nonzero();
                content.end_path();
                content.shading(name(SHADING_PREFIX, id).as_name());
                content.restore_state();
            }
        }

        SceneCommand::FillEllipseGradient {
            x,
            y,
            w,
            h,
            rx,
            ry,
            gradient,
        } => {
            if !rect_ok(*x, *y, *w, *h) {
                return;
            }
            if gradient.radial {
                // Radial PDF degrade: solid fill with first stop color.
                if let Some(first) = gradient.stops.first() {
                    content.save_state();
                    apply_alpha(content, res, &first.color);
                    color::set_fill(content, &first.color);
                    ellipse_path(content, *x, *y, *w, *h, *rx, *ry);
                    content.fill_nonzero();
                    content.restore_state();
                }
            } else if let Some(g) = resolve_gradient(*x, *y, *w, *h, gradient) {
                let id = push_gradient(res, g);
                content.save_state();
                ellipse_path(content, *x, *y, *w, *h, *rx, *ry);
                content.clip_nonzero();
                content.end_path();
                content.shading(name(SHADING_PREFIX, id).as_name());
                content.restore_state();
            }
        }

        // ── Text ──────────────────────────────────────────────────────────
        SceneCommand::DrawGlyphRun {
            x,
            y,
            font_id,
            font_size,
            color,
            // v0: glyph stroke is fill-only in PDF output; stroke_color/stroke_width
            // are intentionally ignored here.
            stroke_color: _,
            stroke_width: _,
            glyphs,
        } => {
            emit_glyph_run(
                content, res, fonts, *x, *y, font_id, *font_size, color, glyphs,
            );
        }

        // ── Images ────────────────────────────────────────────────────────
        SceneCommand::DrawImage {
            x,
            y,
            w,
            h,
            asset_id,
            fit,
            pos_x,
            pos_y,
            opacity,
            clip_shape,
            src_rect: _,
        } => {
            emit_image(
                content, res, assets, *x, *y, *w, *h, asset_id, *fit, *pos_x, *pos_y, *opacity,
                clip_shape,
            );
        }

        // SVG assets are pre-resolved to a raster in the raster backend; the
        // scene IR for the print scenarios never emits this variant. It is
        // matched explicitly (no silent wildcard) and deferred for PDF v0: a
        // faithful vector embedding would require an SVG→PDF path translator,
        // out of scope here. Documented limitation.
        SceneCommand::DrawSvgAsset { .. } => {}

        // ── Clip stack ────────────────────────────────────────────────────
        // PushClip → save the graphics state, install the rect clip, and clear
        // the path; the matching PopClip restores. This nests one q/Q level per
        // clip exactly like the raster backend's clip stack.
        SceneCommand::PushClip { x, y, w, h } => {
            content.save_state();
            content.rect(*x as f32, *y as f32, *w as f32, *h as f32);
            content.clip_nonzero();
            content.end_path();
        }
        SceneCommand::PopClip => {
            content.restore_state();
        }

        // ── Transform stack ───────────────────────────────────────────────
        // Rotation about a pivot: save, translate to pivot, rotate, translate
        // back; the matching PopTransform restores.
        SceneCommand::PushTransform { angle_deg, cx, cy } => {
            content.save_state();
            let theta = (*angle_deg).to_radians();
            let (s, c) = (theta.sin() as f32, theta.cos() as f32);
            let (cx, cy) = (*cx as f32, *cy as f32);
            // Translate(cx,cy) · Rotate(θ) · Translate(-cx,-cy), as one matrix.
            content.transform([c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy]);
        }
        SceneCommand::PopTransform => {
            content.restore_state();
        }

        // ── Compositing layers ────────────────────────────────────────────
        // Layer opacity is applied per-draw via the color alpha cascade already
        // resolved into each command's color in the scene IR, so a layer
        // bracket needs only a save/restore to scope any state it sets. (No
        // group transparency object in v0; matched explicitly, not dropped.)
        //
        // v0 limitation: the `blend_mode` field is ignored — the PDF backend has
        // no ExtGState soft-mask / blend-mode group, so blended content renders
        // source-over. Documented honest limitation (the PNG backend honors it).
        SceneCommand::PushLayer { .. } => {
            content.save_state();
        }
        SceneCommand::PopLayer => {
            content.restore_state();
        }

        // ── Shadow capture ────────────────────────────────────────────────
        // v0 limitation: a Gaussian blur has no vector PDF equivalent. We do
        // NOT drop the bracketed content — the draws between BeginShadow and
        // EndShadow pass straight through and paint crisp; only the blurred
        // shadow layers are skipped. Documented honest limitation.
        SceneCommand::BeginShadow { .. } => {}
        SceneCommand::EndShadow => {}

        // ── Gaussian blur capture ─────────────────────────────────────────
        // v0 limitation: per-element Gaussian blur has no vector PDF equivalent.
        // The bracketed ink passes straight through and paints crisp; only the
        // blur is skipped. Documented honest limitation.
        SceneCommand::BeginBlur { .. } => {}
        SceneCommand::EndBlur => {}

        // ── Color filter capture ──────────────────────────────────────────
        // `translate` intercepts these before dispatch: a BeginFilter opens a
        // capture buffer and the matching EndFilter rasterizes+filters+embeds the
        // buffered region (see `emit_filtered_region`). These arms are therefore
        // unreachable in normal flow; kept (no wildcard) for exhaustiveness, and a
        // no-op is the safe fallback if one ever reaches here un-bracketed.
        SceneCommand::BeginFilter { .. } => {}
        SceneCommand::EndFilter => {}
    }
}

/// Push a gradient and return its resource index.
fn push_gradient(res: &mut PageResources, g: AxialGradient) -> usize {
    let id = res.gradients.len();
    res.gradients.push(g);
    id
}

#[allow(clippy::too_many_arguments)]
fn emit_glyph_run(
    content: &mut Content,
    res: &mut PageResources,
    fonts: &dyn FontProvider,
    x: f64,
    y: f64,
    font_id: &str,
    font_size: f32,
    color: &Color,
    glyphs: &[zenith_scene::SceneGlyph],
) {
    let Some(font_data) = fonts.by_id(font_id) else {
        return;
    };
    let Ok(face) = ttf_parser::Face::parse(&font_data.bytes, font_data.index) else {
        return;
    };
    let units_per_em = face.units_per_em();
    if units_per_em == 0 {
        return;
    }
    let scale = font_size / f32::from(units_per_em);

    content.save_state();
    apply_alpha(content, res, color);
    color::set_fill(content, color);

    // Build one combined path of all glyph outlines, then a single fill. Color
    // bitmap (emoji) glyphs would return Some from `glyph_raster_image`; for PDF
    // v0 they are skipped (documented). Outline fonts never hit that branch.
    let mut any = false;
    for glyph in glyphs {
        if face
            .glyph_raster_image(ttf_parser::GlyphId(glyph.glyph_id), font_size as u16)
            .is_some()
        {
            // Color-bitmap emoji: omitted in PDF v0 (no scenario uses emoji).
            continue;
        }
        let origin_x = x as f32 + glyph.dx;
        let baseline_y = y as f32 + glyph.dy;
        let mut pen = GlyphPen::new(content, origin_x, baseline_y, scale);
        if face
            .outline_glyph(ttf_parser::GlyphId(glyph.glyph_id), &mut pen)
            .is_some()
        {
            any = true;
        }
    }
    if any {
        content.fill_nonzero();
    } else {
        content.end_path();
    }
    content.restore_state();
}

#[allow(clippy::too_many_arguments)]
fn emit_image(
    content: &mut Content,
    res: &mut PageResources,
    assets: &dyn AssetProvider,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    asset_id: &str,
    fit: FitMode,
    pos_x: f64,
    pos_y: f64,
    opacity: f64,
    clip_shape: &Option<ImageClip>,
) {
    if !rect_ok(x, y, w, h) {
        return;
    }
    let Some(asset) = assets.by_id(asset_id) else {
        return;
    };
    // Only raster images are embedded; SVG-kind assets are deferred (see
    // DrawSvgAsset). Match the kind explicitly.
    if !matches!(asset.kind, zenith_core::AssetKind::Image) {
        return;
    }
    let Some(decoded) = decode_for_pdf(&asset.bytes) else {
        return;
    };
    let (sw, sh) = (f64::from(decoded.width), f64::from(decoded.height));
    if !(sw > 0.0 && sh > 0.0) {
        return;
    }

    // Fit transform (sx, sy, tx, ty) in scene space — identical math to the
    // raster backend's DrawImage arm.
    let (sx, sy, tx, ty) = match fit {
        FitMode::Stretch => (w / sw, h / sh, x, y),
        FitMode::Contain => {
            let s = (w / sw).min(h / sh);
            let (rw, rh) = (sw * s, sh * s);
            (
                s,
                s,
                x + (w - rw) * pos_x / 100.0,
                y + (h - rh) * pos_y / 100.0,
            )
        }
        FitMode::Cover => {
            let s = (w / sw).max(h / sh);
            let (rw, rh) = (sw * s, sh * s);
            (
                s,
                s,
                x - (rw - w) * pos_x / 100.0,
                y - (rh - h) * pos_y / 100.0,
            )
        }
        FitMode::None => (
            1.0,
            1.0,
            x - (sw - w) * pos_x / 100.0,
            y - (sh - h) * pos_y / 100.0,
        ),
    };
    if !finite(sx) || !finite(sy) || !finite(tx) || !finite(ty) || sx <= 0.0 || sy <= 0.0 {
        return;
    }

    let id = res.images.len();
    res.images.push(decoded);

    content.save_state();

    // Opacity via an ExtGState (image opacity is a separate factor from any
    // color alpha). 1.0 needs no state.
    let op = (opacity as f32).clamp(0.0, 1.0);
    if op < 1.0 {
        let a = (op * 255.0).round().clamp(0.0, 255.0) as u8;
        let aidx = res.intern_alpha(a);
        content.set_parameters(name(ALPHA_PREFIX, aidx).as_name());
    }

    // Box clip (rect or inscribed shape). The compiler also pushes a PushClip
    // box around images, but re-asserting the box here is harmless and makes
    // the non-rectangular shape clip self-contained.
    match clip_shape {
        None => {
            content.rect(x as f32, y as f32, w as f32, h as f32);
            content.clip_nonzero();
            content.end_path();
        }
        Some(ImageClip::Ellipse) => {
            ellipse_path(content, x, y, w, h, None, None);
            content.clip_nonzero();
            content.end_path();
        }
        Some(ImageClip::RoundedRect { radius }) => {
            rounded_rect_path(content, x, y, w, h, [*radius; 4]);
            content.clip_nonzero();
            content.end_path();
        }
    }

    // An image XObject is a 1×1 unit square in its own space; place it by
    // mapping that unit square onto the fitted box. PDF images are y-up, so we
    // flip within the placement matrix: image row 0 (top) must land at the box
    // top (smaller scene-y). The CTM below maps unit (u, v) → scene point
    // (tx + u*sw*sx, ty + (1-v)*sh*sy), i.e. scale_y is negative with a +height
    // translate, all composed with the page's outer flip.
    let iw = (sw * sx) as f32;
    let ih = (sh * sy) as f32;
    content.transform([iw, 0.0, 0.0, -ih, tx as f32, ty as f32 + ih]);
    content.x_object(name(IMAGE_PREFIX, id).as_name());

    content.restore_state();
}

/// Rasterize a color-`filter` bracket and embed it as an image XObject.
///
/// PDF has no per-pixel filter primitive (grayscale, sepia, hue-rotate, …), so a
/// faithful vector translation is impossible. The user-chosen strategy is
/// rasterize-and-embed: the `buffered` commands are rendered to a standalone
/// full-page sub-scene via the crate's own raster backend, the `filters` are
/// applied to the resulting STRAIGHT-alpha RGBA in place (identical math to the
/// raster backend), the opaque region is cropped to its tight bounding box, and
/// that crop is embedded as a FlateDecode image XObject placed back at its scene
/// position. All arithmetic is deterministic (`f64`, fixed rounding, fixed deflate
/// level) so the PDF stays byte-identical across runs.
///
/// If rasterization fails, the buffered commands are emitted UNFILTERED so the
/// bracketed content is never lost.
fn emit_filtered_region(
    content: &mut Content,
    res: &mut PageResources,
    buffered: &[SceneCommand],
    filters: &[FilterSpec],
    page: (f64, f64),
    fonts: &dyn FontProvider,
    assets: &dyn AssetProvider,
) {
    // 1. Build a standalone full-page sub-scene from the buffered commands. The
    //    background is the default fully-transparent canvas, so only the leaf's
    //    ink is opaque — exactly what the alpha-bbox crop below keys on.
    let (pw, ph) = page;
    let mut sub_scene = Scene::new(pw, ph);
    sub_scene.commands = buffered.to_vec();

    // 2. Rasterize. On failure, fall back to emitting the buffered commands
    //    UNFILTERED so content is never lost.
    let img = match crate::render::render_image(&sub_scene, fonts, assets) {
        Ok(i) => i,
        Err(_) => {
            for c in buffered {
                emit_command(content, res, c, page, fonts, assets);
            }
            return;
        }
    };
    let (iw, ih) = (img.width, img.height);
    let mut rgba = img.rgba;

    // 3. Apply the per-pixel filters in place on straight-alpha RGBA.
    crate::tiny_skia::filter::apply_filters_straight(&mut rgba, filters);

    // Defensive: the buffer must be exactly iw*ih*4 bytes for the row math below.
    let expected = match (iw as usize)
        .checked_mul(ih as usize)
        .and_then(|n| n.checked_mul(4))
    {
        Some(n) => n,
        None => return,
    };
    if iw == 0 || ih == 0 || rgba.len() != expected {
        return;
    }
    let stride = iw as usize * 4;

    // 4. Scan for the tight opaque bounding box (alpha byte > 0). All-transparent
    //    ⇒ nothing to draw.
    let mut min_x = iw;
    let mut min_y = ih;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;
    for (y, row) in rgba.chunks_exact(stride).enumerate() {
        for (x, px) in row.chunks_exact(4).enumerate() {
            if px[3] > 0 {
                found = true;
                let (xu, yu) = (x as u32, y as u32);
                if xu < min_x {
                    min_x = xu;
                }
                if yu < min_y {
                    min_y = yu;
                }
                if xu > max_x {
                    max_x = xu;
                }
                if yu > max_y {
                    max_y = yu;
                }
            }
        }
    }
    if !found {
        return;
    }

    // 5. Crop to (cw, ch) at offset (ox, oy) by copying rows.
    let ox = min_x;
    let oy = min_y;
    let cw = max_x - min_x + 1;
    let ch = max_y - min_y + 1;
    let crop_stride = cw as usize * 4;
    let mut cropped = Vec::with_capacity(crop_stride * ch as usize);
    for y in oy..=max_y {
        let row_start = y as usize * stride + ox as usize * 4;
        let row_end = row_start + crop_stride;
        match rgba.get(row_start..row_end) {
            Some(slice) => cropped.extend_from_slice(slice),
            None => return, // bounds guard: never index out of range
        }
    }

    // 6. Encode the crop as an image XObject.
    let Some(decoded) = decoded_image_from_straight_rgba(&cropped, cw, ch) else {
        return;
    };
    let id = res.images.len();
    res.images.push(decoded);

    // 7. Place it: the crop's top-left maps to scene (ox, oy) and its pixel size
    //    is (cw, ch). The outer page CTM already flips y, so an image y-up unit
    //    square maps via [cw 0 0 -ch ox oy+ch] — identical pattern to emit_image.
    content.save_state();
    content.transform([
        cw as f32,
        0.0,
        0.0,
        -(ch as f32),
        ox as f32,
        oy as f32 + ch as f32,
    ]);
    content.x_object(name(IMAGE_PREFIX, id).as_name());
    content.restore_state();
}

// ── Small helpers ──────────────────────────────────────────────────────────

#[inline]
fn finite(v: f64) -> bool {
    v.is_finite()
}

#[inline]
fn rect_ok(x: f64, y: f64, w: f64, h: f64) -> bool {
    finite(x) && finite(y) && finite(w) && finite(h) && w > 0.0 && h > 0.0
}

/// A small owned resource-name buffer (`<prefix><index>`), kept on the stack to
/// avoid per-call heap churn while satisfying `pdf_writer::Name`'s borrow.
pub(super) struct ResName {
    buf: [u8; 24],
    len: usize,
}

impl ResName {
    pub(super) fn as_name(&self) -> pdf_writer::Name<'_> {
        pdf_writer::Name(&self.buf[..self.len])
    }
}

/// Build a deterministic ASCII resource name `<prefix><index>`.
pub(super) fn name(prefix: &str, index: usize) -> ResName {
    use std::io::Write;
    let mut buf = [0u8; 24];
    let mut cursor = std::io::Cursor::new(&mut buf[..]);
    // prefix is a short ASCII literal and index is a usize; the 24-byte buffer
    // is always large enough, so the writes cannot fail. If they ever did, the
    // name would be truncated to `cursor.position()` bytes — still valid ASCII.
    let _ = write!(cursor, "{prefix}{index}");
    let len = cursor.position() as usize;
    ResName { buf, len }
}
