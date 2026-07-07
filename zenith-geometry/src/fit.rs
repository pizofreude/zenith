use crate::{GeometryError, Point2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolylineEndpointTangentDirections {
    pub start: Point2,
    pub end: Point2,
}

pub fn chord_length_parameters(points: &[Point2]) -> Result<Option<Vec<f64>>, GeometryError> {
    validate_points(points)?;
    if points.len() < 2 {
        return Ok(None);
    }

    let mut distances = Vec::with_capacity(points.len());
    distances.push(0.0);
    let mut total = 0.0;

    for segment in points.windows(2) {
        let Some(start) = segment.first().copied() else {
            continue;
        };
        let Some(end) = segment.get(1).copied() else {
            continue;
        };
        let length = segment_length(start, end)?;
        total += length;
        if !total.is_finite() {
            return Err(GeometryError::CountOutOfRange);
        }
        distances.push(total);
    }

    if total == 0.0 {
        return Err(GeometryError::DegenerateLine);
    }

    Ok(Some(
        distances
            .into_iter()
            .map(|distance| distance / total)
            .collect(),
    ))
}

pub fn estimate_endpoint_tangent_directions(
    points: &[Point2],
) -> Result<Option<PolylineEndpointTangentDirections>, GeometryError> {
    validate_points(points)?;
    if points.len() < 2 {
        return Ok(None);
    }

    let start = first_non_zero_direction(points)?.ok_or(GeometryError::DegenerateLine)?;
    let end = last_non_zero_direction(points)?.ok_or(GeometryError::DegenerateLine)?;
    Ok(Some(PolylineEndpointTangentDirections { start, end }))
}

fn first_non_zero_direction(points: &[Point2]) -> Result<Option<Point2>, GeometryError> {
    let Some(origin) = points.first().copied() else {
        return Ok(None);
    };
    for point in points.iter().copied().skip(1) {
        if let Some(direction) = unit_direction(origin, point)? {
            return Ok(Some(direction));
        }
    }
    Ok(None)
}

fn last_non_zero_direction(points: &[Point2]) -> Result<Option<Point2>, GeometryError> {
    let Some(origin) = points.last().copied() else {
        return Ok(None);
    };
    for point in points.iter().copied().rev().skip(1) {
        if let Some(direction) = unit_direction(origin, point)? {
            return Ok(Some(direction));
        }
    }
    Ok(None)
}

fn unit_direction(start: Point2, end: Point2) -> Result<Option<Point2>, GeometryError> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if !dx.is_finite() || !dy.is_finite() {
        return Err(GeometryError::CountOutOfRange);
    }

    let length = dx.hypot(dy);
    if !length.is_finite() {
        return Err(GeometryError::CountOutOfRange);
    }
    if length == 0.0 {
        return Ok(None);
    }

    Point2::new(dx / length, dy / length).map(Some)
}

fn segment_length(start: Point2, end: Point2) -> Result<f64, GeometryError> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if !dx.is_finite() || !dy.is_finite() {
        return Err(GeometryError::CountOutOfRange);
    }

    let length = dx.hypot(dy);
    if length.is_finite() {
        Ok(length)
    } else {
        Err(GeometryError::CountOutOfRange)
    }
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
    fn empty_and_single_point_inputs_return_none() {
        assert_eq!(chord_length_parameters(&[]), Ok(None));
        assert_eq!(chord_length_parameters(&[point(0.0, 0.0)]), Ok(None));
    }

    #[test]
    fn parameterizes_by_cumulative_chord_length() {
        assert_eq!(
            chord_length_parameters(&[
                point(0.0, 0.0),
                point(3.0, 4.0),
                point(6.0, 4.0),
                point(6.0, 8.0),
            ]),
            Ok(Some(vec![0.0, 5.0 / 12.0, 8.0 / 12.0, 1.0]))
        );
    }

    #[test]
    fn repeated_interior_points_preserve_parameter_plateaus() {
        assert_eq!(
            chord_length_parameters(&[
                point(0.0, 0.0),
                point(4.0, 0.0),
                point(4.0, 0.0),
                point(8.0, 0.0),
            ]),
            Ok(Some(vec![0.0, 0.5, 0.5, 1.0]))
        );
    }

    #[test]
    fn all_repeated_points_are_degenerate() {
        assert_eq!(
            chord_length_parameters(&[point(1.0, 1.0), point(1.0, 1.0)]),
            Err(GeometryError::DegenerateLine)
        );
    }

    #[test]
    fn rejects_non_finite_points() {
        assert_eq!(
            chord_length_parameters(&[point(0.0, 0.0), point(f64::NAN, 1.0)]),
            Err(GeometryError::NonFinitePoint)
        );
    }

    #[test]
    fn endpoint_tangent_directions_return_none_for_underdefined_input() {
        assert_eq!(estimate_endpoint_tangent_directions(&[]), Ok(None));
        assert_eq!(
            estimate_endpoint_tangent_directions(&[point(0.0, 0.0)]),
            Ok(None)
        );
    }

    #[test]
    fn endpoint_tangent_directions_use_unit_vectors_away_from_endpoints() {
        assert_eq!(
            estimate_endpoint_tangent_directions(&[
                point(0.0, 0.0),
                point(3.0, 4.0),
                point(6.0, 4.0)
            ]),
            Ok(Some(PolylineEndpointTangentDirections {
                start: point(0.6, 0.8),
                end: point(-1.0, 0.0),
            }))
        );
    }

    #[test]
    fn endpoint_tangent_directions_skip_repeated_endpoint_runs() {
        assert_eq!(
            estimate_endpoint_tangent_directions(&[
                point(0.0, 0.0),
                point(0.0, 0.0),
                point(4.0, 0.0),
                point(8.0, 0.0),
                point(8.0, 0.0),
            ]),
            Ok(Some(PolylineEndpointTangentDirections {
                start: point(1.0, 0.0),
                end: point(-1.0, 0.0),
            }))
        );
    }

    #[test]
    fn endpoint_tangent_directions_reject_zero_total_length_and_non_finite_points() {
        assert_eq!(
            estimate_endpoint_tangent_directions(&[point(1.0, 1.0), point(1.0, 1.0)]),
            Err(GeometryError::DegenerateLine)
        );
        assert_eq!(
            estimate_endpoint_tangent_directions(&[point(0.0, 0.0), point(f64::NAN, 1.0)]),
            Err(GeometryError::NonFinitePoint)
        );
    }
}
