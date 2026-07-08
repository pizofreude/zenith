//! Paint and effect specs: `Paint`, shadow/filter/mask specs, image fit/clip,
//! and SVG style, plus the private serde helpers shared with the command enum.

use serde::Serialize;

use super::{Color, FillRule, GradientPaint, StrokeAlign};

// ── Paint ───────────────────────────────────────────────────────────────────

/// How a filled region is painted.
///
/// Every fill command carries a `Paint`, so any geometry (rectangle, rounded
/// rectangle, ellipse, polygon, …) can be filled with a flat color or a
/// gradient through one uniform model — there is no per-geometry gradient
/// command. New fill kinds (e.g. patterns) are added here as one more variant,
/// and the exhaustive matches over `Paint` force every backend to handle them.
///
/// Serialized internally-tagged on `kind` so the JSON is self-describing:
/// `{ "kind": "solid", "color": {…} }` or
/// `{ "kind": "gradient", "angle_deg": …, "stops": [...] }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Paint {
    /// A flat fill color.
    Solid {
        /// The fill color (straight / un-pre-multiplied alpha).
        color: Color,
    },
    /// A linear or radial gradient.
    Gradient(GradientPaint),
}

impl Paint {
    /// Construct a solid paint from a color.
    pub fn solid(color: Color) -> Self {
        Paint::Solid { color }
    }
}

// ── Shadow ────────────────────────────────────────────────────────────────────

/// A single drop-shadow / outer-glow layer.
///
/// `dx`/`dy` are the offset (pixels) of the shadow relative to the ink; `blur`
/// is the Gaussian blur sigma (pixels, `>= 0`); `color` is the shadow color
/// (straight / un-pre-multiplied alpha). A node may carry several layers, all
/// painted behind the ink.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShadowSpec {
    /// Horizontal offset in pixels (positive = rightward).
    pub dx: f64,
    /// Vertical offset in pixels (positive = downward).
    pub dy: f64,
    /// Gaussian blur sigma in pixels (`>= 0`).
    pub blur: f64,
    /// Shadow color (straight / un-pre-multiplied alpha).
    pub color: Color,
}

// ── Filter ──────────────────────────────────────────────────────────────────

/// A single color-filter operation applied to captured ink (straight-alpha math).
///
/// Each variant carries its already-resolved scalar payload (the per-kind
/// `amount`, defaults substituted at compile time). `Duotone` additionally
/// carries its two resolved colors — the scene IR stays decoupled from the core
/// AST, exactly as [`ShadowSpec`] carries a scene-local [`Color`] rather than a
/// color-token id. The compile step maps core → scene.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum FilterSpec {
    Grayscale(f64),
    Invert(f64),
    Sepia(f64),
    Saturate(f64),
    Brightness(f64),
    Contrast(f64),
    HueRotate(f64),
    /// Maps luma to a blend between `shadow` (dark) and `highlight` (light),
    /// then mixes with the original by `amount`.
    Duotone {
        amount: f64,
        shadow: Color,
        highlight: Color,
    },
    /// Deterministic monochrome additive film grain: adds the same per-pixel
    /// delta to R, G, and B, derived from an integer hash of the page-absolute
    /// pixel cell and `seed`. `amount` scales the grain magnitude; `scale` is the
    /// grain cell size in pixels. Same inputs → same grain on any machine.
    Noise {
        amount: f64,
        seed: i64,
        scale: f64,
    },
}

// ── Mask ──────────────────────────────────────────────────────────────────────

/// The spatial coverage shape of a node mask.
///
/// Mirrors `zenith_core::MaskShape`; the compile step maps core → scene so the
/// scene IR stays decoupled from the core AST (exactly as [`FilterSpec`] carries
/// scene-local payloads rather than core token ids).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum MaskShape {
    Rect,
    RoundedRect,
    Ellipse,
}

/// A resolved soft-mask applied to a node's draws.
///
/// The mask coverage is the `shape` inscribed in the node box `[x, y, w, h]`
/// (page-absolute pixels), optionally with a corner `radius` (RoundedRect),
/// a Gaussian `feather` sigma (`>= 0`), and an `invert` flag. The renderer
/// brackets the node's draws with [`SceneCommand::BeginMask`](crate::ir::SceneCommand::BeginMask) /
/// [`SceneCommand::EndMask`](crate::ir::SceneCommand::EndMask) and composites the captured ink through the
/// feathered coverage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MaskSpec {
    pub shape: MaskShape,
    /// Resolved corner radius in pixels (RoundedRect; `0.0` otherwise).
    pub radius: f64,
    /// Gaussian feather sigma in pixels (`>= 0`).
    pub feather: f64,
    pub invert: bool,
    /// Node box, page-absolute pixels.
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

// ── Fit mode ────────────────────────────────────────────────────────────────

/// How a raster image asset scales to fill its declared box.
///
/// - `Contain` — scale to fit entirely inside the box (letterboxed).
/// - `Cover` — scale to cover the whole box (cropped, clipped to the box).
/// - `Stretch` — scale each axis independently to exactly fill the box.
/// - `None` — draw at native pixel size, anchored by object-position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    Contain,
    Cover,
    Stretch,
    None,
}

// ── Image source rect ─────────────────────────────────────────────────────────

/// A sub-rectangle within the source image used as the effective source for a
/// [`SceneCommand::DrawImage`](crate::ir::SceneCommand::DrawImage) command.
///
/// All four coordinates are in source-image pixels (top-left origin). The rect
/// is clamped to the source image bounds at render time; a degenerate rect (zero
/// width or height after clamping) causes the draw to be skipped.
///
/// Applies to raster `kind="image"` assets only; ignored for SVG assets (vector
/// assets are resolution-independent and src-rect is a raster concept). This is
/// a documented v0 limitation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SrcRect {
    /// Left edge of the crop in source pixels.
    pub x: f64,
    /// Top edge of the crop in source pixels.
    pub y: f64,
    /// Width of the crop in source pixels (> 0).
    pub w: f64,
    /// Height of the crop in source pixels (> 0).
    pub h: f64,
}

/// Optional SVG-only styling applied at render time before SVG parsing.
///
/// These fields never mutate source asset bytes; they parameterize rendering of
/// `currentColor` / root stroke-fill attributes after locked asset verification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SvgStyle {
    /// Override for SVG stroke/currentColor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Color>,
    /// Override for SVG fill/currentColor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<Color>,
    /// Override for SVG stroke-width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
}

// ── Image clip shape ──────────────────────────────────────────────────────────

/// A non-rectangular clip shape applied to a [`SceneCommand::DrawImage`](crate::ir::SceneCommand::DrawImage).
///
/// `None` on the `DrawImage` (no `clip_shape`) means the default rectangular
/// box-clip (the raster is clipped to its declared `[x, y, w, h]` box). A
/// `Some` value constrains the blit to a shape INSCRIBED in that box:
///
/// - `Ellipse` — the ellipse inscribed in the box (a circle when the box is
///   square): the circular-avatar case.
/// - `RoundedRect { radius }` — a rounded rectangle with uniform corner radius.
///
/// Tagged in JSON via `#[serde(tag = "shape")]` for a self-describing form,
/// consistent with the `op`-tagged [`SceneCommand`](crate::ir::SceneCommand).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "shape")]
pub enum ImageClip {
    /// Clip to the ellipse inscribed in the image's `[x, y, w, h]` box.
    Ellipse,
    /// Clip to a rounded rectangle with uniform corner `radius` (pixels).
    RoundedRect { radius: f64 },
}

pub(in crate::ir) fn is_center(a: &StrokeAlign) -> bool {
    matches!(a, StrokeAlign::Center)
}

pub(in crate::ir) fn is_nonzero(rule: &FillRule) -> bool {
    matches!(rule, FillRule::NonZero)
}

pub(in crate::ir) fn serialize_fill_rule_as_bool<S>(
    rule: &FillRule,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bool(matches!(rule, FillRule::EvenOdd))
}
