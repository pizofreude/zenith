//! Geometry helpers for the PDF backend: rounded-rect and ellipse bezier path
//! emission into a `pdf_writer::Content`, plus the glyph outline pen.
//!
//! All emitters append path-construction operators (`m`, `l`, `c`, `h`) to the
//! content stream but never paint — the caller chooses the paint operator
//! (`f`, `S`, `W n`, …) afterwards. Coordinates are in scene space; the page's
//! initial flip CTM maps them to PDF user space, so no per-point flip is done
//! here.

use pdf_writer::Content;

/// Circle-approximation constant κ for a 90° cubic arc (matches the raster
/// backend's `build_rounded_rect_path`).
const KAPPA: f64 = 0.552_284_8;

/// Append a rounded-rectangle subpath (uniform corner radius `r`, clamped to
/// `min(w, h) / 2`) to `content`. Does nothing for a degenerate box.
pub(super) fn rounded_rect_path(content: &mut Content, x: f64, y: f64, w: f64, h: f64, r: f64) {
    if !(w > 0.0 && h > 0.0 && r.is_finite()) {
        return;
    }
    let r = r.max(0.0).min(w / 2.0).min(h / 2.0);
    let k = KAPPA * r;
    let (x, y, w, h) = (x as f32, y as f32, w as f32, h as f32);
    let (r, k) = (r as f32, k as f32);
    content.move_to(x + r, y);
    content.line_to(x + w - r, y);
    content.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r); // top-right
    content.line_to(x + w, y + h - r);
    content.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h); // bottom-right
    content.line_to(x + r, y + h);
    content.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r); // bottom-left
    content.line_to(x, y + r);
    content.cubic_to(x, y + r - k, x + r - k, y, x + r, y); // top-left
    content.close_path();
}

/// Append a full ellipse subpath inscribed in the box `[x, y, w, h]` to
/// `content`, as four cubic bezier arcs. Does nothing for a degenerate box.
pub(super) fn ellipse_path(content: &mut Content, x: f64, y: f64, w: f64, h: f64) {
    if !(w > 0.0 && h > 0.0) {
        return;
    }
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rx = w / 2.0;
    let ry = h / 2.0;
    let kx = (KAPPA * rx) as f32;
    let ky = (KAPPA * ry) as f32;
    let (cx, cy, rx, ry) = (cx as f32, cy as f32, rx as f32, ry as f32);
    // Start at the rightmost point, go clockwise (in scene/y-down space).
    content.move_to(cx + rx, cy);
    content.cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry); // → bottom
    content.cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy); // → left
    content.cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry); // → top
    content.cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy); // → right
    content.close_path();
}

/// Append a flat `[x0, y0, x1, y1, …]` polygon/polyline subpath to `content`.
///
/// `closed` closes the subpath (polygon outline / fill). Returns `false` and
/// emits nothing when fewer than two vertices are present.
pub(super) fn poly_path(content: &mut Content, points: &[f64], closed: bool) -> bool {
    let (Some(&x0), Some(&y0)) = (points.first(), points.get(1)) else {
        return false;
    };
    content.move_to(x0 as f32, y0 as f32);
    let mut i = 2;
    while i + 1 < points.len() {
        let (Some(&px), Some(&py)) = (points.get(i), points.get(i + 1)) else {
            break;
        };
        content.line_to(px as f32, py as f32);
        i += 2;
    }
    if closed {
        content.close_path();
    }
    true
}

// ── Glyph outline pen ─────────────────────────────────────────────────────────

/// A `ttf_parser::OutlineBuilder` that emits glyph outline segments as PDF
/// path-construction operators into a `pdf_writer::Content`.
///
/// Mirrors the raster backend's `GlyphOutlinePen`, but targets a PDF content
/// buffer. Font coordinates are y-UP with the origin at the glyph origin; the
/// transform applied per point is `px = origin_x + fx*scale`,
/// `py = baseline_y - fy*scale`, matching the raster pen exactly so PDF text
/// outlines align with the rasterized reference. (The page-level flip CTM then
/// maps these y-down scene coordinates back to y-up PDF space.) Quadratic
/// segments are promoted to cubics because PDF has no quadratic operator.
pub(super) struct GlyphPen<'a> {
    content: &'a mut Content,
    origin_x: f32,
    baseline_y: f32,
    scale: f32,
    /// Current pen position in scene coordinates, needed to elevate a TrueType
    /// quadratic to a cubic.
    cur: (f32, f32),
}

impl<'a> GlyphPen<'a> {
    pub(super) fn new(
        content: &'a mut Content,
        origin_x: f32,
        baseline_y: f32,
        scale: f32,
    ) -> Self {
        Self {
            content,
            origin_x,
            baseline_y,
            scale,
            cur: (0.0, 0.0),
        }
    }

    #[inline]
    fn map(&self, fx: f32, fy: f32) -> (f32, f32) {
        (
            self.origin_x + fx * self.scale,
            self.baseline_y - fy * self.scale,
        )
    }
}

impl ttf_parser::OutlineBuilder for GlyphPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.map(x, y);
        self.content.move_to(px, py);
        self.cur = (px, py);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (px, py) = self.map(x, y);
        self.content.line_to(px, py);
        self.cur = (px, py);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        // Elevate the quadratic (p0, c, p1) to a cubic with control points
        // c1 = p0 + 2/3 (c - p0), c2 = p1 + 2/3 (c - p1).
        let (cx, cy) = self.map(x1, y1);
        let (px, py) = self.map(x, y);
        let (p0x, p0y) = self.cur;
        let c1x = p0x + 2.0 / 3.0 * (cx - p0x);
        let c1y = p0y + 2.0 / 3.0 * (cy - p0y);
        let c2x = px + 2.0 / 3.0 * (cx - px);
        let c2y = py + 2.0 / 3.0 * (cy - py);
        self.content.cubic_to(c1x, c1y, c2x, c2y, px, py);
        self.cur = (px, py);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1x, c1y) = self.map(x1, y1);
        let (c2x, c2y) = self.map(x2, y2);
        let (px, py) = self.map(x, y);
        self.content.cubic_to(c1x, c1y, c2x, c2y, px, py);
        self.cur = (px, py);
    }

    fn close(&mut self) {
        self.content.close_path();
    }
}
