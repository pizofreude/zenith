//! `rustybuzz`-backed shaping engine for Zenith.
//!
//! This is the ONLY module in the crate that imports `rustybuzz` or
//! `rustybuzz::ttf_parser`. No third-party type escapes to a public signature.

use zenith_core::FontProvider;

use crate::engine::{PositionedGlyph, ShapeRequest, TextLayoutEngine, ZenithGlyphRun};
use crate::error::LayoutError;

/// HarfBuzz-port shaping engine backed by `rustybuzz` and `rustybuzz::ttf_parser`.
///
/// Construct once and reuse across many `shape` calls; the engine is stateless.
#[derive(Debug, Clone)]
pub struct RustybuzzEngine;

impl RustybuzzEngine {
    /// Create a new `RustybuzzEngine`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustybuzzEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TextLayoutEngine for RustybuzzEngine {
    fn shape(
        &self,
        req: &ShapeRequest<'_>,
        provider: &dyn FontProvider,
    ) -> Result<ZenithGlyphRun, LayoutError> {
        // ── 1. Resolve font bytes ─────────────────────────────────────────────
        let font_data = provider
            .resolve(req.families, req.weight, req.style)
            .ok_or_else(|| {
                LayoutError::new(format!("no font resolved for families {:?}", req.families))
            })?;

        // ── 2. Parse the font face ────────────────────────────────────────────
        let face =
            rustybuzz::Face::from_slice(&font_data.bytes, font_data.index).ok_or_else(|| {
                LayoutError::new(format!(
                    "failed to parse font face for '{}' (index {})",
                    font_data.id, font_data.index
                ))
            })?;

        // ── 3. Compute pixel scale ────────────────────────────────────────────
        // `units_per_em` comes from the `ttf_parser::Face` trait exposed by
        // `rustybuzz::Face` via Deref.
        let units_per_em = face.units_per_em();
        if units_per_em <= 0 {
            return Err(LayoutError::new(format!(
                "font '{}' reports units_per_em = {}",
                font_data.id, units_per_em
            )));
        }
        // `units_per_em` is a positive `i32` (guarded above); the OTF spec
        // range (16–16384) is exactly representable as `f32`.
        let scale = req.font_size / units_per_em as f32;

        // ── 4. Derive line metrics ────────────────────────────────────────────
        // `ascender` and `descender` are in font units; descender is negative.
        let ascent = f32::from(face.ascender()) * scale;
        let descent = -(f32::from(face.descender()) * scale); // store positive magnitude
        let line_gap = f32::from(face.line_gap()) * scale;
        let line_height = ascent + descent + line_gap;

        // ── 5. Shape the text ─────────────────────────────────────────────────
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(req.text);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);

        // Shape with no extra features; deterministic across machines.
        let glyph_buffer = rustybuzz::shape(&face, &[], buffer);

        let infos = glyph_buffer.glyph_infos();
        let positions = glyph_buffer.glyph_positions();

        // ── 6. Build glyph list ───────────────────────────────────────────────
        let mut glyphs: Vec<PositionedGlyph> = Vec::with_capacity(infos.len());
        let mut pen_x: f32 = 0.0;
        let mut pen_y: f32 = 0.0;

        for (info, pos) in infos.iter().zip(positions.iter()) {
            // glyph_id is u32 in rustybuzz; OTF glyph IDs fit in u16 (max 65535).
            // A value above u16::MAX indicates a malformed font — map it to the
            // .notdef glyph (0) rather than silently truncating.
            let glyph_id = u16::try_from(info.glyph_id).unwrap_or(0);

            let x = pen_x + pos.x_offset as f32 * scale;
            // y_offset is in font units; positive = up in font coords → negative screen y.
            let y = pen_y - pos.y_offset as f32 * scale;

            glyphs.push(PositionedGlyph { glyph_id, x, y });

            pen_x += pos.x_advance as f32 * scale;
            pen_y += pos.y_advance as f32 * scale;
        }

        let advance_width = pen_x;

        Ok(ZenithGlyphRun {
            font_id: font_data.id,
            font_size: req.font_size,
            ascent,
            descent,
            line_height,
            advance_width,
            glyphs,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use zenith_core::{FontStyle, default_provider};

    use super::*;

    fn shape_at(font_size: f32) -> Result<ZenithGlyphRun, LayoutError> {
        let families = vec!["Noto Sans".to_string()];
        let req = ShapeRequest {
            text: "Hello Zenith",
            families: &families,
            weight: 400,
            style: FontStyle::Normal,
            font_size,
        };
        let provider = default_provider();
        RustybuzzEngine::new().shape(&req, &provider)
    }

    #[test]
    fn shape_hello_zenith_at_24px() {
        let run = shape_at(24.0).expect("shaping should succeed");

        // font_id matches the registered stable id.
        assert_eq!(run.font_id, "noto-sans-400-normal");

        // Glyph count: "Hello Zenith" = 12 characters including the space.
        assert!(
            run.glyphs.len() >= 10,
            "expected >= 10 glyphs, got {}",
            run.glyphs.len()
        );

        // Metrics sanity.
        assert!(
            run.ascent > 0.0,
            "ascent must be positive, got {}",
            run.ascent
        );
        assert!(
            run.advance_width > 0.0,
            "advance_width must be positive, got {}",
            run.advance_width
        );

        // Glyph x positions must be non-decreasing (monotonic pen advance).
        let mut prev_x = f32::NEG_INFINITY;
        for g in &run.glyphs {
            assert!(
                g.x >= prev_x - 1e-4,
                "x positions must be non-decreasing: {} < {}",
                g.x,
                prev_x
            );
            prev_x = g.x;
        }
    }

    #[test]
    fn shaping_is_deterministic() {
        let run1 = shape_at(24.0).expect("first shape");
        let run2 = shape_at(24.0).expect("second shape");
        assert_eq!(run1, run2, "shaping must be deterministic");
    }

    #[test]
    fn unknown_family_returns_error() {
        let families = vec!["Nonexistent".to_string()];
        let req = ShapeRequest {
            text: "test",
            families: &families,
            weight: 400,
            style: FontStyle::Normal,
            font_size: 16.0,
        };
        let provider = default_provider();
        let result = RustybuzzEngine::new().shape(&req, &provider);
        assert!(result.is_err(), "unknown family must return Err");
        let msg = result.unwrap_err().message;
        assert!(
            msg.contains("no font resolved"),
            "error message should mention unresolved font, got: {msg}"
        );
    }

    #[test]
    fn font_size_scaling_proportional() {
        let run24 = shape_at(24.0).expect("24px");
        let run48 = shape_at(48.0).expect("48px");

        // Ascent should be ~2× when font_size doubles.
        let ratio_ascent = run48.ascent / run24.ascent;
        assert!(
            (ratio_ascent - 2.0).abs() < 0.01,
            "ascent ratio should be ~2.0, got {ratio_ascent}"
        );

        // advance_width should also be ~2×.
        let ratio_adv = run48.advance_width / run24.advance_width;
        assert!(
            (ratio_adv - 2.0).abs() < 0.01,
            "advance_width ratio should be ~2.0, got {ratio_adv}"
        );
    }
}
