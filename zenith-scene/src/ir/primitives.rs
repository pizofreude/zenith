//! Backend-neutral style enums and path-segment primitives: line caps/joins,
//! stroke alignment, fill rule, and the `PathSegment` list with its finiteness
//! and bounding-box helpers.

use serde::Serialize;
use zenith_geometry::{CubicBezier, Point2, RectBounds};

// ── LineCap ───────────────────────────────────────────────────────────────────

/// Stroke end-cap style.
///
/// Maps directly to the `tiny_skia::LineCap` values; serialized in lowercase
/// JSON so the scene JSON is human-readable and matches the KDL attribute values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

// ── LineJoin ──────────────────────────────────────────────────────────────────

/// Stroke corner join style.
///
/// `None` at command sites means the renderer default (`Miter`) and preserves
/// prior serialized IR. Serialized in lowercase JSON to match the KDL
/// `stroke-linejoin` values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

// ── StrokeAlign ─────────────────────────────────────────────────────────────────

/// Stroke alignment relative to a closed polygon's boundary.
///
/// `Center` (the default) strokes centered on the path — identical to the prior
/// IR and the only alignment valid for open polylines. `Inside`/`Outside` shift
/// the visible stroke fully inside / outside the fill boundary; the renderer
/// implements them via a fill-region clip mask, so self-intersecting shapes
/// (stars) and rotation are handled without geometry offsetting. Serialized in
/// lowercase JSON to match the KDL `stroke-alignment` attribute values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StrokeAlign {
    #[default]
    Center,
    Inside,
    Outside,
}

// ── FillRule ─────────────────────────────────────────────────────────────────

/// Fill rule for closed scene geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

impl FillRule {
    pub(crate) fn from_author_value(value: Option<&str>) -> Self {
        match value {
            Some("evenodd") => Self::EvenOdd,
            Some(_) | None => Self::NonZero,
        }
    }
}

/// Structured scene path segment, preserving cubic Bezier geometry for native
/// raster and PDF backends.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum PathSegment {
    MoveTo {
        x: f64,
        y: f64,
    },
    LineTo {
        x: f64,
        y: f64,
    },
    CubicTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    Close,
}

/// Return `true` when every coordinate in `segments` is finite.
pub fn path_segments_finite(segments: &[PathSegment]) -> bool {
    segments.iter().all(|segment| match segment {
        PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => {
            x.is_finite() && y.is_finite()
        }
        PathSegment::CubicTo {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        } => {
            x1.is_finite()
                && y1.is_finite()
                && x2.is_finite()
                && y2.is_finite()
                && x.is_finite()
                && y.is_finite()
        }
        PathSegment::Close => true,
    })
}

/// Axis-aligned bounding box `(x, y, w, h)` of a structured path segment list.
pub fn path_segments_bbox(segments: &[PathSegment]) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<RectBounds> = None;
    let mut current: Option<Point2> = None;
    for segment in segments {
        match segment {
            PathSegment::MoveTo { x, y } | PathSegment::LineTo { x, y } => {
                let point = Point2::new(*x, *y).ok()?;
                bounds = Some(include_point(bounds, point));
                current = Some(point);
            }
            PathSegment::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let p1 = Point2::new(*x1, *y1).ok()?;
                let p2 = Point2::new(*x2, *y2).ok()?;
                let end = Point2::new(*x, *y).ok()?;
                bounds = Some(match current {
                    Some(start) => {
                        let curve = CubicBezier::new(start, p1, p2, end).ok()?;
                        include_bounds(bounds, curve.bounds().ok()?)
                    }
                    None => include_point(bounds, p1)
                        .include_point(p2)
                        .include_point(end),
                });
                current = Some(end);
            }
            PathSegment::Close => {}
        }
    }
    let bounds = bounds?;
    Some((bounds.min_x, bounds.min_y, bounds.width(), bounds.height()))
}

fn include_point(bounds: Option<RectBounds>, point: Point2) -> RectBounds {
    bounds.map_or_else(|| RectBounds::from_point(point), |b| b.include_point(point))
}

fn include_bounds(bounds: Option<RectBounds>, next: RectBounds) -> RectBounds {
    bounds.map_or(next, |b| b.include_bounds(next))
}
