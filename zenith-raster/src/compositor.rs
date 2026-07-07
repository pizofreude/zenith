//! Deterministic borrowed layer-tree compositing over raster surfaces.

use zenith_core::BlendMode;

use crate::blend_pixel;
use crate::surface::{LinearRgba, RasterError, Surface};

/// A borrowed raster layer.
#[derive(Debug, Clone, Copy)]
pub struct Layer<'a> {
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub source: LayerSource<'a>,
}

impl<'a> Layer<'a> {
    /// Create a visible normal-blend layer from a surface.
    pub const fn surface(surface: &'a Surface) -> Self {
        Self {
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            source: LayerSource::Surface(surface),
        }
    }

    /// Create a visible normal-blend group layer from child layers.
    pub const fn group(layers: &'a [Layer<'a>]) -> Self {
        Self {
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            source: LayerSource::Group(layers),
        }
    }

    /// Return this layer with a new boundary opacity.
    pub const fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// Return this layer with a new boundary blend mode.
    pub const fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    /// Return this layer hidden.
    pub const fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }
}

/// Borrowed layer content.
#[derive(Debug, Clone, Copy)]
pub enum LayerSource<'a> {
    Surface(&'a Surface),
    Group(&'a [Layer<'a>]),
}

/// Compose layers over a transparent target surface.
pub fn compose(width: u32, height: u32, layers: &[Layer<'_>]) -> Result<Surface, RasterError> {
    let base = Surface::new(width, height)?;
    compose_onto(&base, layers)
}

/// Compose layers over a copy of `base`.
pub fn compose_onto(base: &Surface, layers: &[Layer<'_>]) -> Result<Surface, RasterError> {
    let mut target = base.clone();
    compose_layers_into(&mut target, layers)?;
    Ok(target)
}

fn compose_layers_into(target: &mut Surface, layers: &[Layer<'_>]) -> Result<(), RasterError> {
    for layer in layers {
        if !layer.visible {
            continue;
        }

        validate_opacity(layer.opacity)?;

        if layer.opacity == 0.0 {
            continue;
        }

        match layer.source {
            LayerSource::Surface(surface) => {
                validate_dimensions(target, surface)?;
                composite_surface(target, surface, layer.opacity, layer.blend_mode)?;
            }
            LayerSource::Group(layers) => {
                let group = compose(target.width(), target.height(), layers)?;
                composite_surface(target, &group, layer.opacity, layer.blend_mode)?;
            }
        }
    }

    Ok(())
}

fn validate_opacity(opacity: f32) -> Result<(), RasterError> {
    if opacity.is_finite() && (0.0..=1.0).contains(&opacity) {
        Ok(())
    } else {
        Err(RasterError::InvalidOpacity)
    }
}

fn validate_dimensions(target: &Surface, source: &Surface) -> Result<(), RasterError> {
    if target.width() == source.width() && target.height() == source.height() {
        Ok(())
    } else {
        Err(RasterError::DimensionMismatch)
    }
}

fn composite_surface(
    target: &mut Surface,
    source: &Surface,
    opacity: f32,
    blend_mode: BlendMode,
) -> Result<(), RasterError> {
    let width = target.width();
    let height = target.height();

    for y in 0..height {
        for x in 0..width {
            let backdrop = target.get(x, y).ok_or(RasterError::OutOfBounds)?;
            let source_pixel = source.get(x, y).ok_or(RasterError::OutOfBounds)?;
            let source_pixel = scale_pixel(source_pixel, opacity)?;
            let blended = blend_pixel(blend_mode, backdrop, source_pixel)?;
            target.set(x, y, blended)?;
        }
    }

    Ok(())
}

fn scale_pixel(pixel: LinearRgba, opacity: f32) -> Result<LinearRgba, RasterError> {
    LinearRgba::premultiplied(
        pixel.r() * opacity,
        pixel.g() * opacity,
        pixel.b() * opacity,
        pixel.a() * opacity,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(r: f32, g: f32, b: f32, a: f32) -> LinearRgba {
        LinearRgba::straight(r, g, b, a).unwrap()
    }

    fn one_pixel_surface(pixel: LinearRgba) -> Surface {
        Surface::filled(1, 1, pixel).unwrap()
    }

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
    fn empty_layers_return_transparent_surface() {
        let composed = compose(2, 1, &[]).unwrap();

        assert_eq!(composed.width(), 2);
        assert_eq!(composed.height(), 1);
        assert_eq!(
            composed.pixels(),
            &[LinearRgba::TRANSPARENT, LinearRgba::TRANSPARENT]
        );
    }

    #[test]
    fn invisible_and_zero_opacity_layers_are_skipped() {
        let base_pixel = pixel(0.1, 0.2, 0.3, 0.4);
        let base = one_pixel_surface(base_pixel);
        let red = one_pixel_surface(pixel(1.0, 0.0, 0.0, 1.0));
        let green = one_pixel_surface(pixel(0.0, 1.0, 0.0, 1.0));
        let layers = [
            Layer::surface(&red).hidden(),
            Layer::surface(&green).with_opacity(0.0),
        ];

        let composed = compose_onto(&base, &layers).unwrap();

        assert_eq!(composed.get(0, 0), Some(base_pixel));
        assert_eq!(red.get(0, 0), Some(pixel(1.0, 0.0, 0.0, 1.0)));
        assert_eq!(green.get(0, 0), Some(pixel(0.0, 1.0, 0.0, 1.0)));
    }

    #[test]
    fn layer_opacity_scales_source_at_boundary() {
        let source_pixel = pixel(1.0, 0.0, 0.0, 0.8);
        let source = one_pixel_surface(source_pixel);
        let layers = [Layer::surface(&source).with_opacity(0.25)];

        let composed = compose(1, 1, &layers).unwrap();

        assert_channels_close(composed.get(0, 0).unwrap(), [0.2, 0.0, 0.0, 0.2]);
        assert_eq!(source.get(0, 0), Some(source_pixel));
    }

    #[test]
    fn non_normal_blend_modes_route_through_pixel_blending() {
        let base = one_pixel_surface(pixel(0.5, 0.5, 0.5, 1.0));
        let source = one_pixel_surface(pixel(0.4, 0.8, 0.2, 1.0));
        let layers = [Layer::surface(&source).with_blend_mode(BlendMode::Multiply)];

        let composed = compose_onto(&base, &layers).unwrap();

        assert_channels_close(composed.get(0, 0).unwrap(), [0.2, 0.4, 0.1, 1.0]);
    }

    #[test]
    fn source_dimension_mismatch_is_reported() {
        let source = Surface::new(2, 1).unwrap();
        let layers = [Layer::surface(&source)];

        assert_eq!(compose(1, 1, &layers), Err(RasterError::DimensionMismatch));
    }

    #[test]
    fn invalid_opacity_is_reported() {
        let source = one_pixel_surface(pixel(1.0, 0.0, 0.0, 1.0));
        let layers = [Layer::surface(&source).with_opacity(f32::NAN)];

        assert_eq!(compose(1, 1, &layers), Err(RasterError::InvalidOpacity));
    }

    #[test]
    fn hidden_layers_do_not_validate_opacity() {
        let source = one_pixel_surface(pixel(1.0, 0.0, 0.0, 1.0));
        let layers = [Layer::surface(&source).with_opacity(f32::NAN).hidden()];

        assert_eq!(compose(1, 1, &layers), Ok(Surface::new(1, 1).unwrap()));
    }

    #[test]
    fn nested_groups_compose_children_before_group_boundary() {
        let base = one_pixel_surface(pixel(0.0, 0.0, 1.0, 1.0));
        let red = one_pixel_surface(pixel(1.0, 0.0, 0.0, 0.5));
        let green = one_pixel_surface(pixel(0.0, 1.0, 0.0, 0.5));
        let group_layers = [Layer::surface(&red), Layer::surface(&green)];
        let layers = [Layer::group(&group_layers).with_opacity(0.5)];

        let composed = compose_onto(&base, &layers).unwrap();

        assert_channels_close(composed.get(0, 0).unwrap(), [0.125, 0.25, 0.625, 1.0]);
    }
}
