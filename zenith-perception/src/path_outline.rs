use crate::{
    PerceptionDiagnostic, PerceptionSeverity, VectorPathContourInput, path_geometry::geometry_path,
};
use zenith_core::PathAnchor;
use zenith_geometry::{
    ClosedPolylineOutlinePolicy, OpenPolylineOutlinePolicy, PathGeometry, PathOutline, Point2,
    RectBounds, outline_path_geometry,
};

#[derive(Debug, Clone, Copy)]
pub struct PathOutlinePerceptionInput<'a> {
    pub anchors: &'a [PathAnchor],
    pub closed: bool,
    pub tolerance: f64,
    pub stroke_width: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct CompoundPathOutlinePerceptionInput<'a> {
    pub contours: &'a [VectorPathContourInput<'a>],
    pub tolerance: f64,
    pub stroke_width: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathOutlineKind {
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathOutlinePerceptionReport {
    pub anchor_count: usize,
    pub segment_count: usize,
    pub closed: bool,
    pub outline_kind: Option<PathOutlineKind>,
    pub outline_point_count: Option<usize>,
    pub left_ring_point_count: Option<usize>,
    pub right_ring_point_count: Option<usize>,
    pub bounds: Option<RectBounds>,
    pub signed_area: Option<f64>,
    pub complexity_score: f32,
    pub diagnostics: Vec<PerceptionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompoundPathOutlinePerceptionReport {
    pub contour_count: usize,
    pub anchor_count: usize,
    pub segment_count: usize,
    pub open_outline_count: usize,
    pub closed_outline_count: usize,
    pub outline_point_count: Option<usize>,
    pub bounds: Option<RectBounds>,
    pub signed_area: Option<f64>,
    pub complexity_score: f32,
    pub diagnostics: Vec<PerceptionDiagnostic>,
}

pub fn path_outline(input: PathOutlinePerceptionInput<'_>) -> PathOutlinePerceptionReport {
    let anchor_count = input.anchors.len();
    let topology = PathGeometry::topology_for(anchor_count, input.closed);
    let mut diagnostics = Vec::new();

    let outline = if !input.stroke_width.is_finite() || input.stroke_width <= 0.0 {
        diagnostics.push(PerceptionDiagnostic::new(
            "path_outline.invalid_stroke_width",
            PerceptionSeverity::Info,
            "path outline perception requires a positive finite stroke width",
        ));
        None
    } else {
        match geometry_path(input.anchors, input.closed) {
            Ok(path) => match outline_path_geometry(
                &path,
                input.tolerance,
                input.stroke_width,
                OpenPolylineOutlinePolicy::default(),
                ClosedPolylineOutlinePolicy::default(),
            ) {
                Ok(outline) => outline,
                Err(_) => {
                    diagnostics.push(PerceptionDiagnostic::new(
                        "path_outline.invalid_outline_input",
                        PerceptionSeverity::Warning,
                        "path outline perception requires valid tolerance and outline geometry",
                    ));
                    None
                }
            },
            Err(()) => {
                diagnostics.push(PerceptionDiagnostic::new(
                    "path_outline.invalid_geometry",
                    PerceptionSeverity::Warning,
                    "path outline perception requires complete finite px anchor and handle coordinates",
                ));
                None
            }
        }
    };

    let summary = outline.as_ref().map(outline_summary);
    PathOutlinePerceptionReport {
        anchor_count,
        segment_count: topology.segment_count,
        closed: input.closed,
        outline_kind: summary.map(|summary| summary.kind),
        outline_point_count: summary.map(|summary| summary.outline_point_count),
        left_ring_point_count: summary.and_then(|summary| summary.left_ring_point_count),
        right_ring_point_count: summary.and_then(|summary| summary.right_ring_point_count),
        bounds: summary.and_then(|summary| summary.bounds),
        signed_area: summary.map(|summary| summary.signed_area),
        complexity_score: complexity_score(summary.map(|summary| summary.outline_point_count)),
        diagnostics,
    }
}

pub fn compound_path_outline(
    input: CompoundPathOutlinePerceptionInput<'_>,
) -> CompoundPathOutlinePerceptionReport {
    let mut diagnostics = Vec::new();
    let mut anchor_count = 0;
    let mut segment_count = 0;
    let mut open_outline_count = 0;
    let mut closed_outline_count = 0;
    let mut outline_point_count = 0;
    let mut bounds: Option<RectBounds> = None;
    let mut signed_area_total = 0.0;
    let mut measured = false;

    for contour in input.contours {
        anchor_count += contour.anchors.len();
        segment_count +=
            PathGeometry::topology_for(contour.anchors.len(), contour.closed).segment_count;
    }

    if !input.stroke_width.is_finite() || input.stroke_width <= 0.0 {
        diagnostics.push(PerceptionDiagnostic::new(
            "path_outline.invalid_stroke_width",
            PerceptionSeverity::Info,
            "path outline perception requires a positive finite stroke width",
        ));
    } else {
        for contour in input.contours {
            match contour_outline(contour, input.tolerance, input.stroke_width) {
                Ok(Some(summary)) => {
                    measured = true;
                    match summary.kind {
                        PathOutlineKind::Open => open_outline_count += 1,
                        PathOutlineKind::Closed => closed_outline_count += 1,
                    }
                    outline_point_count += summary.outline_point_count;
                    signed_area_total += summary.signed_area;
                    if let Some(summary_bounds) = summary.bounds {
                        bounds = Some(match bounds {
                            Some(bounds) => bounds.include_bounds(summary_bounds),
                            None => summary_bounds,
                        });
                    }
                }
                Ok(None) => {}
                Err(diagnostic) => diagnostics.push(diagnostic),
            }
        }
    }

    CompoundPathOutlinePerceptionReport {
        contour_count: input.contours.len(),
        anchor_count,
        segment_count,
        open_outline_count,
        closed_outline_count,
        outline_point_count: measured.then_some(outline_point_count),
        bounds,
        signed_area: measured.then_some(signed_area_total),
        complexity_score: complexity_score(measured.then_some(outline_point_count)),
        diagnostics,
    }
}

fn contour_outline(
    contour: &VectorPathContourInput<'_>,
    tolerance: f64,
    stroke_width: f64,
) -> Result<Option<OutlineSummary>, PerceptionDiagnostic> {
    let path = geometry_path(contour.anchors, contour.closed).map_err(|()| {
        PerceptionDiagnostic::new(
            "path_outline.invalid_geometry",
            PerceptionSeverity::Warning,
            "path outline perception requires complete finite px anchor and handle coordinates",
        )
    })?;
    let outline = outline_path_geometry(
        &path,
        tolerance,
        stroke_width,
        OpenPolylineOutlinePolicy::default(),
        ClosedPolylineOutlinePolicy::default(),
    )
    .map_err(|_| {
        PerceptionDiagnostic::new(
            "path_outline.invalid_outline_input",
            PerceptionSeverity::Warning,
            "path outline perception requires valid tolerance and outline geometry",
        )
    })?;
    Ok(outline.as_ref().map(outline_summary))
}

#[derive(Debug, Clone, Copy)]
struct OutlineSummary {
    kind: PathOutlineKind,
    outline_point_count: usize,
    left_ring_point_count: Option<usize>,
    right_ring_point_count: Option<usize>,
    bounds: Option<RectBounds>,
    signed_area: f64,
}

fn outline_summary(outline: &PathOutline) -> OutlineSummary {
    match outline {
        PathOutline::Open(open) => OutlineSummary {
            kind: PathOutlineKind::Open,
            outline_point_count: open.points.len(),
            left_ring_point_count: None,
            right_ring_point_count: None,
            bounds: bounds_for_points(open.points.iter().copied()),
            signed_area: signed_area(open.points.iter().copied()),
        },
        PathOutline::Closed(closed) => {
            let outline_point_count = closed.left_ring.len() + closed.right_ring.len();
            OutlineSummary {
                kind: PathOutlineKind::Closed,
                outline_point_count,
                left_ring_point_count: Some(closed.left_ring.len()),
                right_ring_point_count: Some(closed.right_ring.len()),
                bounds: bounds_for_points(
                    closed
                        .left_ring
                        .iter()
                        .copied()
                        .chain(closed.right_ring.iter().copied()),
                ),
                signed_area: signed_area(closed.left_ring.iter().copied())
                    + signed_area(closed.right_ring.iter().copied()),
            }
        }
    }
}

fn complexity_score(point_count: Option<usize>) -> f32 {
    let Some(point_count) = point_count else {
        return 0.0;
    };
    if point_count == 0 {
        0.0
    } else {
        (1.0 / (point_count as f32 / 8.0).max(1.0)).clamp(0.0, 1.0)
    }
}

fn bounds_for_points(points: impl IntoIterator<Item = Point2>) -> Option<RectBounds> {
    let mut points = points.into_iter();
    let first = points.next()?;
    let mut bounds = RectBounds::from_point(first);
    for point in points {
        bounds = bounds.include_point(point);
    }
    if bounds.is_valid() {
        Some(bounds)
    } else {
        None
    }
}

fn signed_area(points: impl IntoIterator<Item = Point2>) -> f64 {
    let mut points = points.into_iter();
    let Some(first) = points.next() else {
        return 0.0;
    };
    let Some(second) = points.next() else {
        return 0.0;
    };
    let mut area = 0.0;
    let mut previous = first;
    let mut current = second;
    loop {
        area += previous.x * current.y - current.x * previous.y;
        previous = current;
        match points.next() {
            Some(next) => current = next,
            None => break,
        }
    }
    area += previous.x * first.y - first.x * previous.y;
    area * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use zenith_core::{Dimension, Unit};

    #[test]
    fn open_path_reports_outline_summary() {
        let anchors = [anchor(0.0, 0.0), anchor(10.0, 0.0)];

        let report = path_outline(PathOutlinePerceptionInput {
            anchors: &anchors,
            closed: false,
            tolerance: 0.25,
            stroke_width: 4.0,
        });

        assert_eq!(report.anchor_count, 2);
        assert_eq!(report.segment_count, 1);
        assert_eq!(report.outline_kind, Some(PathOutlineKind::Open));
        assert_eq!(report.outline_point_count, Some(4));
        assert_eq!(report.left_ring_point_count, None);
        assert_eq!(report.right_ring_point_count, None);
        assert_eq!(report.signed_area, Some(-40.0));
        assert_eq!(report.complexity_score, 1.0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn closed_path_reports_ring_counts() {
        let anchors = [
            anchor(0.0, 0.0),
            anchor(10.0, 0.0),
            anchor(10.0, 10.0),
            anchor(0.0, 10.0),
        ];

        let report = path_outline(PathOutlinePerceptionInput {
            anchors: &anchors,
            closed: true,
            tolerance: 0.25,
            stroke_width: 4.0,
        });

        assert_eq!(report.outline_kind, Some(PathOutlineKind::Closed));
        assert_eq!(report.outline_point_count, Some(12));
        assert_eq!(report.left_ring_point_count, Some(4));
        assert_eq!(report.right_ring_point_count, Some(8));
        assert_eq!(
            report.bounds,
            Some(RectBounds {
                min_x: -2.0,
                min_y: -2.0,
                max_x: 12.0,
                max_y: 12.0,
            })
        );
        assert_eq!(report.complexity_score, 2.0 / 3.0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn zero_stroke_reports_no_measurement() {
        let anchors = [anchor(0.0, 0.0), anchor(10.0, 0.0)];

        let report = path_outline(PathOutlinePerceptionInput {
            anchors: &anchors,
            closed: false,
            tolerance: 0.25,
            stroke_width: 0.0,
        });

        assert_eq!(report.outline_kind, None);
        assert_eq!(report.outline_point_count, None);
        assert_eq!(report.complexity_score, 0.0);
        assert_eq!(
            report.diagnostics,
            vec![PerceptionDiagnostic::new(
                "path_outline.invalid_stroke_width",
                PerceptionSeverity::Info,
                "path outline perception requires a positive finite stroke width",
            )]
        );
    }

    #[test]
    fn invalid_anchor_geometry_reports_warning() {
        let anchors = [PathAnchor {
            x: Some(px(0.0)),
            y: None,
            kind: None,
            in_x: None,
            in_y: None,
            out_x: None,
            out_y: None,
        }];

        let report = path_outline(PathOutlinePerceptionInput {
            anchors: &anchors,
            closed: false,
            tolerance: 0.25,
            stroke_width: 4.0,
        });

        assert_eq!(report.outline_kind, None);
        assert_eq!(report.complexity_score, 0.0);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "path_outline.invalid_geometry")
        );
    }

    #[test]
    fn invalid_tolerance_reports_warning() {
        let anchors = [anchor(0.0, 0.0), anchor(10.0, 0.0)];

        let report = path_outline(PathOutlinePerceptionInput {
            anchors: &anchors,
            closed: false,
            tolerance: 0.0,
            stroke_width: 4.0,
        });

        assert_eq!(report.outline_kind, None);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "path_outline.invalid_outline_input")
        );
    }

    #[test]
    fn compound_path_outline_aggregates_contour_summaries() {
        let open = [anchor(0.0, 0.0), anchor(10.0, 0.0)];
        let closed = [
            anchor(20.0, 0.0),
            anchor(30.0, 0.0),
            anchor(30.0, 10.0),
            anchor(20.0, 10.0),
        ];
        let contours = [
            VectorPathContourInput {
                anchors: &open,
                closed: false,
            },
            VectorPathContourInput {
                anchors: &closed,
                closed: true,
            },
        ];

        let report = compound_path_outline(CompoundPathOutlinePerceptionInput {
            contours: &contours,
            tolerance: 0.25,
            stroke_width: 4.0,
        });

        assert_eq!(report.contour_count, 2);
        assert_eq!(report.anchor_count, 6);
        assert_eq!(report.segment_count, 5);
        assert_eq!(report.open_outline_count, 1);
        assert_eq!(report.closed_outline_count, 1);
        assert_eq!(report.outline_point_count, Some(16));
        assert_eq!(
            report.bounds,
            Some(RectBounds {
                min_x: 0.0,
                min_y: -2.0,
                max_x: 32.0,
                max_y: 12.0,
            })
        );
        assert!(report.signed_area.is_some());
        assert!(report.complexity_score > 0.0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn compound_path_outline_reports_invalid_contour_geometry() {
        let valid = [anchor(0.0, 0.0), anchor(10.0, 0.0)];
        let invalid = [PathAnchor {
            x: Some(px(20.0)),
            y: None,
            kind: None,
            in_x: None,
            in_y: None,
            out_x: None,
            out_y: None,
        }];
        let contours = [
            VectorPathContourInput {
                anchors: &valid,
                closed: false,
            },
            VectorPathContourInput {
                anchors: &invalid,
                closed: false,
            },
        ];

        let report = compound_path_outline(CompoundPathOutlinePerceptionInput {
            contours: &contours,
            tolerance: 0.25,
            stroke_width: 4.0,
        });

        assert_eq!(report.contour_count, 2);
        assert_eq!(report.anchor_count, 3);
        assert_eq!(report.open_outline_count, 1);
        assert_eq!(report.outline_point_count, Some(4));
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "path_outline.invalid_geometry")
        );
    }

    fn anchor(x: f64, y: f64) -> PathAnchor {
        PathAnchor {
            x: Some(px(x)),
            y: Some(px(y)),
            kind: None,
            in_x: None,
            in_y: None,
            out_x: None,
            out_y: None,
        }
    }

    fn px(value: f64) -> Dimension {
        Dimension {
            value,
            unit: Unit::Px,
        }
    }
}
