//! Coverage-mask attenuation for masked layers.
//!
//! A mask is a per-pixel coverage field (a shape — rect / rounded-rect / ellipse
//! — optionally feathered with a Gaussian blur and optionally inverted) that
//! attenuates a captured layer's ink. At `EndMask` the captured ink buffer is
//! multiplied, per pixel, by the mask's coverage: opaque coverage (255) leaves
//! the ink untouched, zero coverage makes it fully transparent.
//!
//! All arithmetic is pure integer / `f64` with fixed evaluation order and the
//! same deterministic rounding used elsewhere in the backend (`(v * c + 127) /
//! 255`), so output is byte-identical across runs on the same machine. The
//! coverage rasterization uses anti-aliased path fill (curved shapes need
//! sub-pixel coverage) which is pure-software and deterministic — matching
//! `build_align_mask` and the ellipse fills.

use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, Rect, Transform};
use zenith_scene::{MaskShape, MaskSpec};

use super::paths::build_rounded_rect_path;
use super::shadow::gaussian_blur_premul;

/// Attenuate `pm` (premultiplied RGBA8) in place by the coverage field described
/// by `spec`.
///
/// Builds the coverage buffer (one byte per pixel, `0..=255`) and multiplies all
/// four premultiplied channels of each pixel uniformly by `coverage / 255` with
/// deterministic rounding. Multiplying premultiplied RGBA uniformly by a coverage
/// factor is exactly alpha attenuation (the premultiplied invariant `c <= a` is
/// preserved). If the coverage buffer cannot be built (allocation failure), `pm`
/// is left untouched — the ink then composites unmasked (a safe degrade, never a
/// panic).
pub(super) fn attenuate_by_mask(pm: &mut Pixmap, spec: &MaskSpec) {
    let (width, height) = (pm.width(), pm.height());
    let Some(coverage) = build_mask_coverage(spec, width, height) else {
        return; // alloc failure → degrade: leave ink unmasked
    };

    // Iterate pixels in lockstep with the coverage slice by index. `chunks_exact`
    // guarantees exactly 4 bytes per chunk; `coverage.get(i)` is bounds-checked.
    for (i, px) in pm.data_mut().chunks_exact_mut(4).enumerate() {
        let cov = u32::from(coverage.get(i).copied().unwrap_or(0));
        // (channel * cov + 127) / 255 — same rounding as shadow.rs premultiply.
        for ch in px.iter_mut() {
            let v = u32::from(*ch);
            *ch = (((v * cov) + 127) / 255).min(255) as u8;
        }
    }
}

/// Build the coverage buffer (length `width * height`, one byte `0..=255` per
/// pixel) for `spec` at the given device dimensions.
///
/// Returns `None` on any allocation failure or degenerate geometry, in which
/// case the caller leaves the ink unmasked.
fn build_mask_coverage(spec: &MaskSpec, width: u32, height: u32) -> Option<Vec<u8>> {
    let mut mask = Mask::new(width, height)?;

    // Build the shape path in device space at (x, y, w, h).
    let (x, y, w, h) = (spec.x as f32, spec.y as f32, spec.w as f32, spec.h as f32);
    let path = match spec.shape {
        MaskShape::Rect => {
            let rect = Rect::from_xywh(x, y, w, h)?;
            PathBuilder::from_rect(rect)
        }
        MaskShape::Ellipse => {
            let rect = Rect::from_xywh(x, y, w, h)?;
            PathBuilder::from_oval(rect)?
        }
        MaskShape::RoundedRect => {
            // Clamp radius to a non-negative value no larger than half the box.
            let r = (spec.radius as f32).max(0.0).min(w / 2.0).min(h / 2.0);
            build_rounded_rect_path(x, y, w, h, [r; 4])?
        }
    };

    // AA on: curved shapes need sub-pixel coverage; deterministic same-machine
    // (matches build_align_mask). Identity transform — spec coords are already
    // device/page-absolute pixels.
    mask.fill_path(&path, FillRule::Winding, true, Transform::identity());

    // Coverage = the mask's single alpha channel, optionally feathered.
    let mut coverage: Vec<u8> = if spec.feather > 0.0 {
        // Feather by blurring the coverage. `gaussian_blur_premul` only operates
        // on a Pixmap, so splat the single-channel mask alpha into all four
        // channels (AAAA), blur, then read the alpha byte back as coverage.
        let mut temp = Pixmap::new(width, height)?;
        {
            let src = mask.data();
            let dst = temp.data_mut();
            for (out, &a) in dst.chunks_exact_mut(4).zip(src.iter()) {
                out[0] = a;
                out[1] = a;
                out[2] = a;
                out[3] = a;
            }
        }
        gaussian_blur_premul(&mut temp, spec.feather);
        temp.data()
            .chunks_exact(4)
            .map(|px| px.get(3).copied().unwrap_or(0))
            .collect()
    } else {
        mask.data().to_vec()
    };

    // Invert: coverage byte c -> 255 - c.
    if spec.invert {
        for c in coverage.iter_mut() {
            *c = 255 - *c;
        }
    }

    Some(coverage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zenith_scene::{MaskShape, MaskSpec};

    /// Build a fully-opaque red premultiplied pixmap.
    fn red_pixmap(w: u32, h: u32) -> Pixmap {
        let mut pm = Pixmap::new(w, h).expect("alloc");
        for px in pm.data_mut().chunks_exact_mut(4) {
            px[0] = 200; // r (premultiplied, a=255 → unchanged)
            px[1] = 0;
            px[2] = 0;
            px[3] = 255;
        }
        pm
    }

    fn rect_spec(w: f64, h: f64, invert: bool) -> MaskSpec {
        MaskSpec {
            shape: MaskShape::Rect,
            radius: 0.0,
            feather: 0.0,
            invert,
            x: 0.0,
            y: 0.0,
            w,
            h,
        }
    }

    #[test]
    fn full_rect_no_invert_leaves_pixmap_unchanged() {
        let mut pm = red_pixmap(8, 6);
        let before = pm.data().to_vec();
        attenuate_by_mask(&mut pm, &rect_spec(8.0, 6.0, false));
        assert_eq!(pm.data(), &before[..], "coverage 255 → no change");
    }

    #[test]
    fn full_rect_inverted_makes_pixmap_transparent() {
        let mut pm = red_pixmap(8, 6);
        attenuate_by_mask(&mut pm, &rect_spec(8.0, 6.0, true));
        assert!(
            pm.data().iter().all(|&b| b == 0),
            "inverted full coverage → fully transparent",
        );
    }

    #[test]
    fn ellipse_clears_corners_keeps_center() {
        let (w, h) = (16u32, 16u32);
        let mut pm = red_pixmap(w, h);
        let spec = MaskSpec {
            shape: MaskShape::Ellipse,
            radius: 0.0,
            feather: 0.0,
            invert: false,
            x: 0.0,
            y: 0.0,
            w: f64::from(w),
            h: f64::from(h),
        };
        attenuate_by_mask(&mut pm, &spec);
        let data = pm.data();
        // Corner pixel (0,0) is outside the inscribed ellipse → transparent.
        assert_eq!(data.get(3).copied(), Some(0), "corner alpha is 0");
        // Center pixel is inside → fully opaque.
        let cx = (w / 2) as usize;
        let cy = (h / 2) as usize;
        let center_a = (cy * w as usize + cx) * 4 + 3;
        assert_eq!(
            data.get(center_a).copied(),
            Some(255),
            "center alpha is 255"
        );
    }

    #[test]
    fn coverage_is_deterministic() {
        let spec = MaskSpec {
            shape: MaskShape::Ellipse,
            radius: 0.0,
            feather: 2.5,
            invert: true,
            x: 1.0,
            y: 1.0,
            w: 30.0,
            h: 20.0,
        };
        let a = build_mask_coverage(&spec, 32, 24).expect("coverage a");
        let b = build_mask_coverage(&spec, 32, 24).expect("coverage b");
        assert_eq!(a, b, "two identical calls must be byte-identical");
    }
}
