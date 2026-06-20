//! Color translation for the PDF backend.
//!
//! A scene [`Color`] carries straight-alpha sRGB channels plus an optional
//! original CMYK quad. CMYK-origin colors emit native DeviceCMYK (`k` / `K`)
//! so the press receives the authored separations untouched; sRGB-origin colors
//! emit DeviceRGB (`rg` / `RG`). Alpha below opaque is honored via a named
//! ExtGState carrying `ca` (fill) / `CA` (stroke) — the content translator
//! interns the alpha byte and the document writer materializes the matching
//! `/ExtGState` resource.

use pdf_writer::Content;
use zenith_scene::Color;

/// Set the non-stroking (fill) color from `color`, choosing DeviceCMYK for a
/// CMYK-origin color and DeviceRGB otherwise.
pub(super) fn set_fill(content: &mut Content, color: &Color) {
    match color.cmyk {
        Some([c, m, y, k]) => {
            content.set_fill_cmyk(c / 100.0, m / 100.0, y / 100.0, k / 100.0);
        }
        None => {
            content.set_fill_rgb(
                f32::from(color.r) / 255.0,
                f32::from(color.g) / 255.0,
                f32::from(color.b) / 255.0,
            );
        }
    }
}

/// Set the stroking color from `color` (see [`set_fill`]).
pub(super) fn set_stroke(content: &mut Content, color: &Color) {
    match color.cmyk {
        Some([c, m, y, k]) => {
            content.set_stroke_cmyk(c / 100.0, m / 100.0, y / 100.0, k / 100.0);
        }
        None => {
            content.set_stroke_rgb(
                f32::from(color.r) / 255.0,
                f32::from(color.g) / 255.0,
                f32::from(color.b) / 255.0,
            );
        }
    }
}
