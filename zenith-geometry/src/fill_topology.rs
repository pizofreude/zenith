use crate::{
    ClosedPolyline, ClosedPolylineRelation, ClosedPolylineWinding, CompoundPathGeometry,
    GeometryError, classify_closed_polyline_relation, validation::validate_tolerance,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundFillRule {
    NonZero,
    EvenOdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilledContourBoundaryRole {
    Paint,
    Hole,
    NoFillChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilledContourTopology {
    pub contour_index: usize,
    pub winding: ClosedPolylineWinding,
    pub depth: usize,
    pub ancestor_winding_number: i32,
    pub interior_winding_number: i32,
    pub exterior_filled: bool,
    pub interior_filled: bool,
    pub role: FilledContourBoundaryRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundFillTopology {
    pub contours: Vec<FilledContourTopology>,
}

pub fn classify_closed_polyline_fill_topology(
    contours: &[ClosedPolyline],
    rule: CompoundFillRule,
    tolerance: f64,
) -> Result<CompoundFillTopology, GeometryError> {
    validate_tolerance(tolerance)?;

    let ancestors = collect_contour_ancestors(contours, tolerance)?;
    let topologies = contours
        .iter()
        .enumerate()
        .map(|(contour_index, contour)| {
            let winding = contour.winding();
            let ancestor_winding_number =
                ancestor_winding_number(contours, &ancestors, contour_index);
            let winding_delta = winding_delta(winding);
            let interior_winding_number = ancestor_winding_number + winding_delta;
            let depth = ancestors.get(contour_index).map_or(0, std::vec::Vec::len);
            let exterior_filled = region_is_filled(rule, depth, ancestor_winding_number);
            let interior_filled =
                region_is_filled(rule, depth.saturating_add(1), interior_winding_number);

            FilledContourTopology {
                contour_index,
                winding,
                depth,
                ancestor_winding_number,
                interior_winding_number,
                exterior_filled,
                interior_filled,
                role: boundary_role(exterior_filled, interior_filled),
            }
        })
        .collect();

    Ok(CompoundFillTopology {
        contours: topologies,
    })
}

pub fn classify_compound_path_fill_topology(
    geometry: &CompoundPathGeometry,
    rule: CompoundFillRule,
    tolerance: f64,
) -> Result<CompoundFillTopology, GeometryError> {
    let flattened_contours = geometry.flatten_contours(tolerance)?;
    let mut contours = Vec::with_capacity(flattened_contours.len());
    let mut source_indices = Vec::with_capacity(flattened_contours.len());

    for flattened in flattened_contours {
        if !flattened.closed {
            return Err(GeometryError::InvalidContour);
        }
        contours.push(ClosedPolyline::new(flattened.points)?);
        source_indices.push(flattened.contour_index);
    }

    let mut topology = classify_closed_polyline_fill_topology(&contours, rule, tolerance)?;
    if topology.contours.len() != source_indices.len() {
        return Err(GeometryError::InvalidContour);
    }
    for (contour, source_index) in topology.contours.iter_mut().zip(source_indices) {
        contour.contour_index = source_index;
    }

    Ok(topology)
}

fn collect_contour_ancestors(
    contours: &[ClosedPolyline],
    tolerance: f64,
) -> Result<Vec<Vec<usize>>, GeometryError> {
    let mut ancestors = vec![Vec::new(); contours.len()];

    for (first_index, first) in contours.iter().enumerate() {
        for (second_index, second) in contours.iter().enumerate().skip(first_index + 1) {
            match classify_closed_polyline_relation(first, second, tolerance)? {
                ClosedPolylineRelation::Disjoint => {}
                ClosedPolylineRelation::Intersecting => return Err(GeometryError::InvalidContour),
                ClosedPolylineRelation::FirstContainsSecond => {
                    let Some(second_ancestors) = ancestors.get_mut(second_index) else {
                        return Err(GeometryError::InvalidContour);
                    };
                    second_ancestors.push(first_index);
                }
                ClosedPolylineRelation::SecondContainsFirst => {
                    let Some(first_ancestors) = ancestors.get_mut(first_index) else {
                        return Err(GeometryError::InvalidContour);
                    };
                    first_ancestors.push(second_index);
                }
            }
        }
    }

    Ok(ancestors)
}

fn ancestor_winding_number(
    contours: &[ClosedPolyline],
    ancestors: &[Vec<usize>],
    contour_index: usize,
) -> i32 {
    let Some(contour_ancestors) = ancestors.get(contour_index) else {
        return 0;
    };

    contour_ancestors
        .iter()
        .filter_map(|ancestor_index| contours.get(*ancestor_index))
        .map(|contour| winding_delta(contour.winding()))
        .sum()
}

const fn winding_delta(winding: ClosedPolylineWinding) -> i32 {
    match winding {
        ClosedPolylineWinding::Clockwise => -1,
        ClosedPolylineWinding::CounterClockwise => 1,
    }
}

const fn region_is_filled(rule: CompoundFillRule, depth: usize, winding_number: i32) -> bool {
    match rule {
        CompoundFillRule::NonZero => winding_number != 0,
        CompoundFillRule::EvenOdd => depth % 2 == 1,
    }
}

const fn boundary_role(exterior_filled: bool, interior_filled: bool) -> FilledContourBoundaryRole {
    match (exterior_filled, interior_filled) {
        (false, true) => FilledContourBoundaryRole::Paint,
        (true, false) => FilledContourBoundaryRole::Hole,
        (false, false) | (true, true) => FilledContourBoundaryRole::NoFillChange,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PathAnchor, PathGeometry, Point2};

    const TOLERANCE: f64 = 0.001;

    #[test]
    fn empty_compound_returns_empty_topology() {
        let polyline_topology =
            classify_closed_polyline_fill_topology(&[], CompoundFillRule::NonZero, TOLERANCE)
                .expect("topology");
        let compound_topology = classify_compound_path_fill_topology(
            &CompoundPathGeometry::new(Vec::new()),
            CompoundFillRule::NonZero,
            TOLERANCE,
        )
        .expect("compound topology");

        assert!(polyline_topology.contours.is_empty());
        assert!(compound_topology.contours.is_empty());
    }

    #[test]
    fn disjoint_closed_contours_are_both_paint() {
        let contours = vec![square(0.0, 0.0, 10.0), square(20.0, 0.0, 10.0)];
        let topology =
            classify_closed_polyline_fill_topology(&contours, CompoundFillRule::NonZero, TOLERANCE)
                .expect("topology");

        assert_eq!(roles(&topology), vec![FilledContourBoundaryRole::Paint; 2]);
        assert_eq!(depths(&topology), vec![0, 0]);
    }

    #[test]
    fn even_odd_nested_contours_alternate_independent_of_winding() {
        let contours = vec![
            square(0.0, 0.0, 30.0),
            reversed_square(5.0, 5.0, 20.0),
            square(10.0, 10.0, 10.0),
        ];
        let topology =
            classify_closed_polyline_fill_topology(&contours, CompoundFillRule::EvenOdd, TOLERANCE)
                .expect("topology");

        assert_eq!(
            roles(&topology),
            vec![
                FilledContourBoundaryRole::Paint,
                FilledContourBoundaryRole::Hole,
                FilledContourBoundaryRole::Paint,
            ]
        );
        assert_eq!(depths(&topology), vec![0, 1, 2]);
    }

    #[test]
    fn nonzero_opposite_winding_child_becomes_hole() {
        let contours = vec![square(0.0, 0.0, 20.0), reversed_square(5.0, 5.0, 10.0)];
        let topology =
            classify_closed_polyline_fill_topology(&contours, CompoundFillRule::NonZero, TOLERANCE)
                .expect("topology");

        assert_eq!(
            roles(&topology),
            vec![
                FilledContourBoundaryRole::Paint,
                FilledContourBoundaryRole::Hole,
            ]
        );
        assert_eq!(winding_numbers(&topology), vec![(0, 1), (1, 0)]);
    }

    #[test]
    fn nonzero_same_winding_child_is_no_fill_change() {
        let contours = vec![square(0.0, 0.0, 20.0), square(5.0, 5.0, 10.0)];
        let topology =
            classify_closed_polyline_fill_topology(&contours, CompoundFillRule::NonZero, TOLERANCE)
                .expect("topology");

        assert_eq!(
            roles(&topology),
            vec![
                FilledContourBoundaryRole::Paint,
                FilledContourBoundaryRole::NoFillChange,
            ]
        );
        assert_eq!(winding_numbers(&topology), vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn inner_before_outer_preserves_source_indices_and_depths() {
        let compound = CompoundPathGeometry::new(vec![
            path(&[(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)], true),
            path(&[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)], true),
        ]);

        let topology =
            classify_compound_path_fill_topology(&compound, CompoundFillRule::EvenOdd, TOLERANCE)
                .expect("topology");

        assert_eq!(indices(&topology), vec![0, 1]);
        assert_eq!(depths(&topology), vec![1, 0]);
    }

    #[test]
    fn intersecting_touching_and_overlapping_contours_return_invalid_contour() {
        let intersecting = vec![square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0)];
        let touching = vec![square(0.0, 0.0, 10.0), square(10.0, 2.0, 4.0)];
        let overlapping = vec![square(0.0, 0.0, 10.0), square(0.0, 0.0, 10.0)];

        for contours in [intersecting, touching, overlapping] {
            assert_eq!(
                classify_closed_polyline_fill_topology(
                    &contours,
                    CompoundFillRule::EvenOdd,
                    TOLERANCE,
                ),
                Err(GeometryError::InvalidContour)
            );
        }
    }

    #[test]
    fn open_compound_contour_is_rejected_by_compound_convenience_api() {
        let compound =
            CompoundPathGeometry::new(vec![path(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false)]);

        assert_eq!(
            classify_compound_path_fill_topology(&compound, CompoundFillRule::NonZero, TOLERANCE,),
            Err(GeometryError::InvalidContour)
        );
    }

    #[test]
    fn invalid_tolerance_returns_an_error() {
        let contours = vec![square(0.0, 0.0, 10.0)];

        assert_eq!(
            classify_closed_polyline_fill_topology(&contours, CompoundFillRule::NonZero, f64::NAN,),
            Err(GeometryError::NonFiniteTolerance)
        );
        assert_eq!(
            classify_closed_polyline_fill_topology(&[], CompoundFillRule::NonZero, 0.0,),
            Err(GeometryError::NonPositiveTolerance)
        );
    }

    fn roles(topology: &CompoundFillTopology) -> Vec<FilledContourBoundaryRole> {
        topology
            .contours
            .iter()
            .map(|contour| contour.role)
            .collect()
    }

    fn depths(topology: &CompoundFillTopology) -> Vec<usize> {
        topology
            .contours
            .iter()
            .map(|contour| contour.depth)
            .collect()
    }

    fn indices(topology: &CompoundFillTopology) -> Vec<usize> {
        topology
            .contours
            .iter()
            .map(|contour| contour.contour_index)
            .collect()
    }

    fn winding_numbers(topology: &CompoundFillTopology) -> Vec<(i32, i32)> {
        topology
            .contours
            .iter()
            .map(|contour| {
                (
                    contour.ancestor_winding_number,
                    contour.interior_winding_number,
                )
            })
            .collect()
    }

    fn square(x: f64, y: f64, size: f64) -> ClosedPolyline {
        contour(&[
            point(x, y),
            point(x + size, y),
            point(x + size, y + size),
            point(x, y + size),
        ])
    }

    fn reversed_square(x: f64, y: f64, size: f64) -> ClosedPolyline {
        contour(&[
            point(x, y),
            point(x, y + size),
            point(x + size, y + size),
            point(x + size, y),
        ])
    }

    fn contour(points: &[Point2]) -> ClosedPolyline {
        ClosedPolyline::new(points.to_vec()).expect("valid contour")
    }

    fn path(points: &[(f64, f64)], closed: bool) -> PathGeometry {
        PathGeometry::new(anchors(points), closed).expect("valid path")
    }

    fn anchors(points: &[(f64, f64)]) -> Vec<PathAnchor> {
        points
            .iter()
            .map(|(x, y)| PathAnchor::new(point(*x, *y), None, None).expect("valid anchor"))
            .collect()
    }

    fn point(x: f64, y: f64) -> Point2 {
        Point2::new(x, y).expect("valid point")
    }
}
