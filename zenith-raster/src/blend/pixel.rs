use zenith_core::BlendMode;

use crate::blend::{nonseparable, separable};
use crate::surface::{LinearRgba, RasterError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StraightRgb {
    pub(crate) r: f32,
    pub(crate) g: f32,
    pub(crate) b: f32,
}

impl StraightRgb {
    pub(crate) const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    pub(crate) fn clamped(self) -> Self {
        Self {
            r: clamp_unit(self.r),
            g: clamp_unit(self.g),
            b: clamp_unit(self.b),
        }
    }
}

/// Blend one premultiplied source pixel over one premultiplied backdrop pixel.
pub fn blend_pixel(
    mode: BlendMode,
    backdrop: LinearRgba,
    source: LinearRgba,
) -> Result<LinearRgba, RasterError> {
    let back_rgb = straight_rgb(backdrop);
    let src_rgb = straight_rgb(source);
    let blended_rgb = blend_rgb(mode, back_rgb, src_rgb).clamped();

    let src_a = source.a();
    let back_a = backdrop.a();
    let out_a = src_a + back_a * (1.0 - src_a);
    let source_weight = src_a * (1.0 - back_a);
    let blend_weight = src_a * back_a;
    let backdrop_weight = back_a * (1.0 - src_a);
    let out_rgb = StraightRgb {
        r: source_weight * src_rgb.r + blend_weight * blended_rgb.r + backdrop_weight * back_rgb.r,
        g: source_weight * src_rgb.g + blend_weight * blended_rgb.g + backdrop_weight * back_rgb.g,
        b: source_weight * src_rgb.b + blend_weight * blended_rgb.b + backdrop_weight * back_rgb.b,
    }
    .clamped();

    LinearRgba::premultiplied(
        clamp_premultiplied(out_rgb.r, out_a),
        clamp_premultiplied(out_rgb.g, out_a),
        clamp_premultiplied(out_rgb.b, out_a),
        clamp_unit(out_a),
    )
}

pub(crate) fn blend_rgb(
    mode: BlendMode,
    backdrop: StraightRgb,
    source: StraightRgb,
) -> StraightRgb {
    match mode {
        BlendMode::Normal => source,
        BlendMode::Multiply => separable::blend(backdrop, source, separable::multiply),
        BlendMode::Screen => separable::blend(backdrop, source, separable::screen),
        BlendMode::Overlay => separable::blend(backdrop, source, separable::overlay),
        BlendMode::Darken => separable::blend(backdrop, source, separable::darken),
        BlendMode::Lighten => separable::blend(backdrop, source, separable::lighten),
        BlendMode::ColorDodge => separable::blend(backdrop, source, separable::color_dodge),
        BlendMode::ColorBurn => separable::blend(backdrop, source, separable::color_burn),
        BlendMode::HardLight => separable::blend(backdrop, source, separable::hard_light),
        BlendMode::SoftLight => separable::blend(backdrop, source, separable::soft_light),
        BlendMode::Difference => separable::blend(backdrop, source, separable::difference),
        BlendMode::Exclusion => separable::blend(backdrop, source, separable::exclusion),
        BlendMode::Hue => nonseparable::hue(backdrop, source),
        BlendMode::Saturation => nonseparable::saturation(backdrop, source),
        BlendMode::Color => nonseparable::color(backdrop, source),
        BlendMode::Luminosity => nonseparable::luminosity(backdrop, source),
    }
}

pub(crate) fn clamp_unit(channel: f32) -> f32 {
    if !channel.is_finite() || channel <= 0.0 {
        0.0
    } else if channel >= 1.0 {
        1.0
    } else {
        channel
    }
}

fn straight_rgb(pixel: LinearRgba) -> StraightRgb {
    let alpha = pixel.a();

    if alpha <= 0.0 {
        StraightRgb::BLACK
    } else {
        StraightRgb {
            r: pixel.r() / alpha,
            g: pixel.g() / alpha,
            b: pixel.b() / alpha,
        }
        .clamped()
    }
}

fn clamp_premultiplied(channel: f32, alpha: f32) -> f32 {
    let alpha = clamp_unit(alpha);
    let channel = clamp_unit(channel);
    if channel > alpha { alpha } else { channel }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_channels_close(actual: LinearRgba, expected: [f32; 4]) {
        let actual = actual.channels();
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 0.000_001,
                "actual {actual} expected {expected}"
            );
        }
    }

    #[test]
    fn normal_uses_source_over_compositing() {
        let backdrop = LinearRgba::straight(0.2, 0.4, 0.6, 0.5).unwrap();
        let source = LinearRgba::straight(0.8, 0.1, 0.3, 0.25).unwrap();

        let blended = blend_pixel(BlendMode::Normal, backdrop, source).unwrap();

        assert_channels_close(blended, [0.275, 0.175, 0.3, 0.625]);
    }

    #[test]
    fn transparent_source_preserves_backdrop() {
        let backdrop = LinearRgba::straight(0.2, 0.4, 0.6, 0.5).unwrap();
        let blended = blend_pixel(BlendMode::Multiply, backdrop, LinearRgba::TRANSPARENT).unwrap();

        assert_eq!(blended, backdrop);
    }

    #[test]
    fn transparent_backdrop_returns_source_for_blend_modes() {
        let source = LinearRgba::straight(0.8, 0.1, 0.3, 0.25).unwrap();
        let blended = blend_pixel(BlendMode::ColorBurn, LinearRgba::TRANSPARENT, source).unwrap();

        assert_eq!(blended, source);
    }
}
