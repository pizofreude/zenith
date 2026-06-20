//! Linear-gradient → PDF axial (Type 2) shading translation.
//!
//! A scene [`GradientPaint`] is a CSS-style linear gradient: an angle plus
//! ordered color stops. The PDF equivalent is a Type 2 (axial) shading whose
//! `/Coords` give the gradient line's endpoints and whose `/Function` maps the
//! parametric `t in [0, 1]` to a color. We build that function as a Type 3
//! *stitching* function over one Type 2 *exponential* (linear, `N = 1`)
//! subfunction per adjacent stop pair — the exact, standards-compliant
//! representation of a multi-stop linear gradient.
//!
//! Stops are emitted in **DeviceRGB** using each stop's device-sRGB channels,
//! matching the raster backend (which also paints gradients from the `r/g/b`
//! triple). Gradient stop alpha is not representable in an axial shading's color
//! function; like the raster backend's gradient path it is ignored (stops are
//! treated as opaque). Gradients in the leaflet scenarios are opaque.

use zenith_scene::GradientPaint;

/// A gradient resolved to PDF axial-shading geometry plus ordered RGB stops.
///
/// Built once per gradient draw by [`resolve`]; consumed by the document writer
/// which materializes the function + shading indirect objects and clips the
/// shading to the shape via `W n` + `sh`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct AxialGradient {
    /// Gradient line endpoints `[x0, y0, x1, y1]` in scene coordinates.
    pub(super) coords: [f32; 4],
    /// Ordered stops: `(offset 0..=1, [r, g, b] each 0..=1)`. At least two.
    pub(super) stops: Vec<(f32, [f32; 3])>,
}

/// Resolve a [`GradientPaint`] over the box `[x, y, w, h]` into an
/// [`AxialGradient`], or `None` when it has fewer than two stops.
///
/// The gradient line runs through the box center at `angle_deg` (clockwise from
/// +x in screen coordinates), with the CSS gradient-line length
/// `|w·cosθ| + |h·sinθ|`, identical to the raster backend's `gradient_shader`.
pub(super) fn resolve(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    gradient: &GradientPaint,
) -> Option<AxialGradient> {
    if gradient.stops.len() < 2 {
        return None;
    }
    let theta = gradient.angle_deg.to_radians();
    let (dir_x, dir_y) = (theta.cos(), theta.sin());
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let line_len = (w * dir_x).abs() + (h * dir_y).abs();
    let half = line_len / 2.0;
    let coords = [
        (cx - dir_x * half) as f32,
        (cy - dir_y * half) as f32,
        (cx + dir_x * half) as f32,
        (cy + dir_y * half) as f32,
    ];
    let stops: Vec<(f32, [f32; 3])> = gradient
        .stops
        .iter()
        .map(|s| {
            (
                (s.offset as f32).clamp(0.0, 1.0),
                [
                    f32::from(s.color.r) / 255.0,
                    f32::from(s.color.g) / 255.0,
                    f32::from(s.color.b) / 255.0,
                ],
            )
        })
        .collect();
    Some(AxialGradient { coords, stops })
}
