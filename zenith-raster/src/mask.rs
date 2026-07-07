//! Raster-local mask coverage primitives.

use crate::surface::{LinearRgba, Surface};

/// A borrowed raster mask applied at a layer boundary.
#[derive(Debug, Clone, Copy)]
pub struct Mask<'a> {
    pub source: MaskSource<'a>,
    pub invert: bool,
}

impl<'a> Mask<'a> {
    /// Create an alpha-channel mask.
    pub const fn alpha(surface: &'a Surface) -> Self {
        Self {
            source: MaskSource::Alpha(surface),
            invert: false,
        }
    }

    /// Create a premultiplied linear luminance mask.
    pub const fn luminance(surface: &'a Surface) -> Self {
        Self {
            source: MaskSource::Luminance(surface),
            invert: false,
        }
    }

    /// Return this mask with inverted coverage.
    pub const fn inverted(mut self) -> Self {
        self.invert = true;
        self
    }

    pub(crate) const fn surface(&self) -> &'a Surface {
        match self.source {
            MaskSource::Alpha(surface) => surface,
            MaskSource::Luminance(surface) => surface,
        }
    }

    pub(crate) fn coverage(&self, pixel: LinearRgba) -> f32 {
        let coverage = match self.source {
            MaskSource::Alpha(_surface) => pixel.a(),
            MaskSource::Luminance(_surface) => {
                0.3 * pixel.r() + 0.59 * pixel.g() + 0.11 * pixel.b()
            }
        };
        let coverage = clamp_unit(coverage);

        if self.invert {
            1.0 - coverage
        } else {
            coverage
        }
    }
}

/// Borrowed raster mask source data.
#[derive(Debug, Clone, Copy)]
pub enum MaskSource<'a> {
    Alpha(&'a Surface),
    Luminance(&'a Surface),
}

fn clamp_unit(channel: f32) -> f32 {
    if !channel.is_finite() || channel <= 0.0 {
        0.0
    } else if channel >= 1.0 {
        1.0
    } else {
        channel
    }
}
