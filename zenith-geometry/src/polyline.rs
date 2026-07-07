use crate::{GeometryError, Point2, validation::validate_tolerance};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolylineProjection {
    pub point: Point2,
    pub segment_index: usize,
    pub segment_t: f64,
    pub distance_squared: f64,
}

pub fn project_onto_polyline(
    point: Point2,
    polyline: &[Point2],
) -> Result<Option<PolylineProjection>, GeometryError> {
    point.validate()?;
    validate_points(polyline)?;

    if polyline.len() < 2 {
        return Ok(None);
    }

    let mut nearest: Option<PolylineProjection> = None;
    for (segment_index, segment) in polyline.windows(2).enumerate() {
        let Some(segment_start) = segment.first().copied() else {
            continue;
        };
        let Some(segment_end) = segment.get(1).copied() else {
            continue;
        };
        let projection = point.project_onto_segment(segment_start, segment_end);
        let candidate = PolylineProjection {
            point: projection.point,
            segment_index,
            segment_t: projection.t,
            distance_squared: projection.distance_squared,
        };

        match nearest {
            Some(current) if candidate.distance_squared >= current.distance_squared => {
                nearest = Some(current);
            }
            Some(_) | None => {
                nearest = Some(candidate);
            }
        }
    }

    Ok(nearest)
}

pub fn simplify_polyline(points: &[Point2], tolerance: f64) -> Result<Vec<Point2>, GeometryError> {
    validate_tolerance(tolerance)?;
    validate_points(points)?;

    if points.len() <= 2 {
        return Ok(points.to_vec());
    }

    let tolerance_squared = tolerance * tolerance;
    let mut keep = vec![false; points.len()];
    if let Some(first) = keep.first_mut() {
        *first = true;
    }
    if let Some(last) = keep.last_mut() {
        *last = true;
    }

    let mut stack = Vec::new();
    let Some(last_index) = points.len().checked_sub(1) else {
        return Ok(points.to_vec());
    };
    stack.push((0_usize, last_index));

    while let Some((start, end)) = stack.pop() {
        let Some(start_point) = points.get(start).copied() else {
            continue;
        };
        let Some(end_point) = points.get(end).copied() else {
            continue;
        };

        let Some(interior_start) = start.checked_add(1) else {
            continue;
        };
        let Some(interior_count) = end.checked_sub(interior_start) else {
            continue;
        };

        let mut farthest = None;
        let mut farthest_distance_squared = tolerance_squared;

        for (index, point) in points
            .iter()
            .enumerate()
            .skip(interior_start)
            .take(interior_count)
        {
            let distance_squared = point.distance_squared_to_segment(start_point, end_point);
            if distance_squared > farthest_distance_squared {
                farthest = Some(index);
                farthest_distance_squared = distance_squared;
            }
        }

        if let Some(split) = farthest {
            if let Some(slot) = keep.get_mut(split) {
                *slot = true;
            }
            stack.push((split, end));
            stack.push((start, split));
        }
    }

    Ok(points
        .iter()
        .zip(keep.iter())
        .filter_map(|(point, should_keep)| should_keep.then_some(*point))
        .collect())
}

fn validate_points(points: &[Point2]) -> Result<(), GeometryError> {
    points.iter().try_for_each(|point| point.validate())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64) -> Point2 {
        Point2::new_unchecked(x, y)
    }

    #[test]
    fn projection_returns_none_for_empty_polyline() {
        assert_eq!(project_onto_polyline(point(1.0, 2.0), &[]), Ok(None));
    }

    #[test]
    fn projection_returns_none_for_one_point_polyline() {
        assert_eq!(
            project_onto_polyline(point(3.0, 4.0), &[point(0.0, 0.0)]),
            Ok(None)
        );
    }

    #[test]
    fn projection_finds_interior_point_on_multi_segment_polyline() {
        let polyline = [
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(2.0, 5.0),
            point(6.0, 5.0),
        ];

        assert_eq!(
            project_onto_polyline(point(4.0, 2.0), &polyline),
            Ok(Some(PolylineProjection {
                point: point(2.0, 2.0),
                segment_index: 1,
                segment_t: 0.4,
                distance_squared: 4.0,
            }))
        );
    }

    #[test]
    fn projection_keeps_earliest_segment_on_tie() {
        let polyline = [point(0.0, 0.0), point(2.0, 0.0), point(2.0, 2.0)];

        assert_eq!(
            project_onto_polyline(point(1.0, 1.0), &polyline),
            Ok(Some(PolylineProjection {
                point: point(1.0, 0.0),
                segment_index: 0,
                segment_t: 0.5,
                distance_squared: 1.0,
            }))
        );
    }

    #[test]
    fn projection_handles_repeated_points_as_degenerate_segments() {
        let polyline = [point(0.0, 0.0), point(0.0, 0.0), point(4.0, 0.0)];

        assert_eq!(
            project_onto_polyline(point(0.0, 2.0), &polyline),
            Ok(Some(PolylineProjection {
                point: point(0.0, 0.0),
                segment_index: 0,
                segment_t: 0.0,
                distance_squared: 4.0,
            }))
        );
    }

    #[test]
    fn projection_rejects_invalid_query_point() {
        let polyline = [point(0.0, 0.0), point(1.0, 0.0)];

        assert_eq!(
            project_onto_polyline(point(f64::NAN, 0.0), &polyline),
            Err(GeometryError::NonFinitePoint)
        );
    }

    #[test]
    fn projection_rejects_invalid_polyline_point() {
        let polyline = [point(0.0, 0.0), point(f64::INFINITY, 0.0)];

        assert_eq!(
            project_onto_polyline(point(0.0, 0.0), &polyline),
            Err(GeometryError::NonFinitePoint)
        );
    }

    #[test]
    fn projection_chooses_nearest_segment_over_farther_segment() {
        let polyline = [
            point(0.0, 0.0),
            point(1.0, 0.0),
            point(10.0, 0.0),
            point(10.0, 5.0),
        ];

        assert_eq!(
            project_onto_polyline(point(10.0, 3.0), &polyline),
            Ok(Some(PolylineProjection {
                point: point(10.0, 3.0),
                segment_index: 2,
                segment_t: 0.6,
                distance_squared: 0.0,
            }))
        );
    }

    #[test]
    fn empty_one_and_two_point_inputs_round_trip() {
        assert_eq!(simplify_polyline(&[], 0.1), Ok(Vec::new()));

        let one = vec![point(1.0, 2.0)];
        assert_eq!(simplify_polyline(&one, 0.1), Ok(one.clone()));

        let two = vec![point(1.0, 2.0), point(3.0, 4.0)];
        assert_eq!(simplify_polyline(&two, 0.1), Ok(two));
    }

    #[test]
    fn removes_near_collinear_middle_point() {
        let points = vec![point(0.0, 0.0), point(5.0, 0.01), point(10.0, 0.0)];

        assert_eq!(
            simplify_polyline(&points, 0.1),
            Ok(vec![point(0.0, 0.0), point(10.0, 0.0)])
        );
    }

    #[test]
    fn preserves_far_middle_point() {
        let points = vec![point(0.0, 0.0), point(5.0, 2.0), point(10.0, 0.0)];

        assert_eq!(simplify_polyline(&points, 0.1), Ok(points));
    }

    #[test]
    fn preserves_reversal_beyond_current_segment() {
        let points = vec![point(0.0, 0.0), point(2.0, 0.0), point(1.0, 0.0)];

        assert_eq!(simplify_polyline(&points, 0.1), Ok(points));
    }

    #[test]
    fn propagates_invalid_point() {
        let points = vec![point(0.0, 0.0), point(f64::NAN, 1.0), point(2.0, 0.0)];

        assert_eq!(
            simplify_polyline(&points, 0.1),
            Err(GeometryError::NonFinitePoint)
        );
    }

    #[test]
    fn propagates_invalid_tolerance() {
        let points = vec![point(0.0, 0.0), point(1.0, 1.0)];

        assert_eq!(
            simplify_polyline(&points, f64::NAN),
            Err(GeometryError::NonFiniteTolerance)
        );
        assert_eq!(
            simplify_polyline(&points, 0.0),
            Err(GeometryError::NonPositiveTolerance)
        );
    }

    #[test]
    fn preserves_endpoints_and_order() {
        let points = vec![
            point(0.0, 0.0),
            point(1.0, 0.01),
            point(2.0, 3.0),
            point(3.0, 0.01),
            point(4.0, 0.0),
        ];
        let simplified = simplify_polyline(&points, 0.1).expect("valid simplification");

        assert_eq!(simplified.first(), points.first());
        assert_eq!(simplified.last(), points.last());

        let mut input = points.iter();
        for simplified_point in &simplified {
            assert!(input.any(|point| point == simplified_point));
        }
    }
}
