use crate::{
    GeometryError, PathGeometry, PolylineIntersection, collect_open_polyline_intersections,
    validation::validate_tolerance,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PathGeometryIntersections {
    pub first_points: Vec<crate::Point2>,
    pub second_points: Vec<crate::Point2>,
    pub intersections: Vec<PolylineIntersection>,
}

pub fn collect_path_geometry_intersections(
    first: &PathGeometry,
    second: &PathGeometry,
    tolerance: f64,
) -> Result<PathGeometryIntersections, GeometryError> {
    validate_tolerance(tolerance)?;
    let first_points = first.flatten(tolerance)?;
    let second_points = second.flatten(tolerance)?;
    let intersections = collect_open_polyline_intersections(&first_points, &second_points)?;

    Ok(PathGeometryIntersections {
        first_points,
        second_points,
        intersections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CubicBezier, PathAnchor, PathSegment, Point2, SegmentIntersection};

    fn point(x: f64, y: f64) -> Point2 {
        Point2::new_unchecked(x, y)
    }

    fn anchor(x: f64, y: f64) -> PathAnchor {
        PathAnchor::new(point(x, y), None, None).expect("valid anchor")
    }

    #[test]
    fn collects_line_path_intersections() {
        let horizontal =
            PathGeometry::new(vec![anchor(0.0, 0.0), anchor(10.0, 0.0)], false).expect("path");
        let vertical =
            PathGeometry::new(vec![anchor(5.0, -5.0), anchor(5.0, 5.0)], false).expect("path");

        let report = collect_path_geometry_intersections(&horizontal, &vertical, 0.1)
            .expect("intersections");

        assert_eq!(report.first_points, vec![point(0.0, 0.0), point(10.0, 0.0)]);
        assert_eq!(
            report.second_points,
            vec![point(5.0, -5.0), point(5.0, 5.0)]
        );
        assert_eq!(report.intersections.len(), 1);
        assert_eq!(report.intersections[0].first_segment_index, 0);
        assert_eq!(report.intersections[0].second_segment_index, 0);
        assert_eq!(
            report.intersections[0].intersection,
            SegmentIntersection::Point(crate::IntersectionPoint {
                point: point(5.0, 0.0),
                first_t: 0.5,
                second_t: 0.5,
            })
        );
    }

    #[test]
    fn flattens_cubic_paths_before_collecting_intersections() {
        let curve = CubicBezier::new_unchecked(
            point(0.0, 0.0),
            point(0.0, 10.0),
            point(10.0, 10.0),
            point(10.0, 0.0),
        );
        let start = PathAnchor::new(curve.p0, None, Some(curve.p1)).expect("valid anchor");
        let end = PathAnchor::new(curve.p3, Some(curve.p2), None).expect("valid anchor");
        let cubic = PathGeometry::new(vec![start, end], false).expect("path");
        let cutter =
            PathGeometry::new(vec![anchor(5.0, -1.0), anchor(5.0, 9.0)], false).expect("path");

        let report =
            collect_path_geometry_intersections(&cubic, &cutter, 0.5).expect("intersections");

        assert!(report.first_points.len() > 2);
        assert!(
            report.intersections.iter().any(|intersection| matches!(
                intersection.intersection,
                SegmentIntersection::Point(_)
            )),
            "expected at least one point intersection; got {report:?}"
        );
    }

    #[test]
    fn validates_tolerance() {
        let path =
            PathGeometry::new(vec![anchor(0.0, 0.0), anchor(1.0, 0.0)], false).expect("path");

        assert_eq!(
            collect_path_geometry_intersections(&path, &path, 0.0),
            Err(GeometryError::NonPositiveTolerance)
        );
    }

    #[test]
    fn empty_paths_have_no_intersections() {
        let empty = PathGeometry::new(Vec::new(), false).expect("path");
        let line =
            PathGeometry::new(vec![anchor(0.0, 0.0), anchor(1.0, 0.0)], false).expect("path");

        assert_eq!(
            collect_path_geometry_intersections(&empty, &line, 0.1)
                .expect("intersections")
                .intersections,
            Vec::new()
        );
    }

    #[test]
    fn keeps_overlap_intersections() {
        let first =
            PathGeometry::new(vec![anchor(0.0, 0.0), anchor(10.0, 0.0)], false).expect("path");
        let second =
            PathGeometry::new(vec![anchor(4.0, 0.0), anchor(8.0, 0.0)], false).expect("path");

        let report =
            collect_path_geometry_intersections(&first, &second, 0.1).expect("intersections");

        assert!(matches!(
            report.intersections[0].intersection,
            SegmentIntersection::Overlap { .. }
        ));
    }

    #[test]
    fn uses_path_segments_without_mutating_inputs() {
        let path =
            PathGeometry::new(vec![anchor(0.0, 0.0), anchor(1.0, 0.0)], false).expect("path");
        let original_segments = path.segments().expect("segments");

        let _ = collect_path_geometry_intersections(&path, &path, 0.1).expect("intersections");

        assert_eq!(path.segments().expect("segments"), original_segments);
        assert_eq!(
            original_segments,
            vec![PathSegment::Line {
                start: point(0.0, 0.0),
                end: point(1.0, 0.0),
            }]
        );
    }
}
