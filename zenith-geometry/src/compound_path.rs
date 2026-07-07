use crate::{
    AffineTransform, GeometryError, PathAnchor, PathGeometry, PathProjection, PathTopology, Point2,
    RectBounds,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CompoundPathGeometry {
    contours: Vec<PathGeometry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathContourSpec<'a> {
    pub anchors: &'a [PathAnchor],
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlattenedPathContour {
    pub contour_index: usize,
    pub closed: bool,
    pub points: Vec<Point2>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompoundPathProjection {
    pub contour_index: usize,
    pub projection: PathProjection,
}

impl CompoundPathGeometry {
    pub fn new(contours: Vec<PathGeometry>) -> Self {
        Self { contours }
    }

    pub fn from_specs<'a, I>(specs: I) -> Result<Self, GeometryError>
    where
        I: IntoIterator<Item = PathContourSpec<'a>>,
    {
        let contours = specs
            .into_iter()
            .map(|spec| PathGeometry::new(spec.anchors.to_vec(), spec.closed))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { contours })
    }

    #[must_use]
    pub fn contours(&self) -> &[PathGeometry] {
        &self.contours
    }

    #[must_use]
    pub fn contour_count(&self) -> usize {
        self.contours.len()
    }

    #[must_use]
    pub fn topology(&self) -> PathTopology {
        self.contours.iter().fold(
            PathTopology {
                anchor_count: 0,
                segment_count: 0,
                open_subpath_count: 0,
                closed_subpath_count: 0,
            },
            |mut topology, contour| {
                let contour_topology = contour.topology();
                topology.anchor_count = topology
                    .anchor_count
                    .saturating_add(contour_topology.anchor_count);
                topology.segment_count = topology
                    .segment_count
                    .saturating_add(contour_topology.segment_count);
                topology.open_subpath_count = topology
                    .open_subpath_count
                    .saturating_add(contour_topology.open_subpath_count);
                topology.closed_subpath_count = topology
                    .closed_subpath_count
                    .saturating_add(contour_topology.closed_subpath_count);
                topology
            },
        )
    }

    pub fn bounds(&self) -> Result<Option<RectBounds>, GeometryError> {
        let mut bounds: Option<RectBounds> = None;
        for contour in &self.contours {
            let Some(contour_bounds) = contour.bounds()? else {
                continue;
            };
            bounds = Some(match bounds {
                Some(bounds) => bounds.include_bounds(contour_bounds),
                None => contour_bounds,
            });
        }

        Ok(bounds)
    }

    pub fn transform(&self, transform: AffineTransform) -> Result<Self, GeometryError> {
        let contours = self
            .contours
            .iter()
            .map(|contour| contour.transform(transform))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { contours })
    }

    pub fn flatten_contours(
        &self,
        tolerance: f64,
    ) -> Result<Vec<FlattenedPathContour>, GeometryError> {
        self.contours
            .iter()
            .enumerate()
            .map(|(contour_index, contour)| {
                Ok(FlattenedPathContour {
                    contour_index,
                    closed: contour.closed(),
                    points: contour.flatten(tolerance)?,
                })
            })
            .collect()
    }

    pub fn project(
        &self,
        point: Point2,
        tolerance: f64,
    ) -> Result<Option<CompoundPathProjection>, GeometryError> {
        let mut nearest: Option<CompoundPathProjection> = None;
        for (contour_index, contour) in self.contours.iter().enumerate() {
            let Some(projection) = contour.project(point, tolerance)? else {
                continue;
            };
            let candidate = CompoundPathProjection {
                contour_index,
                projection,
            };
            match nearest {
                Some(current)
                    if candidate.projection.distance_squared
                        >= current.projection.distance_squared =>
                {
                    nearest = Some(current);
                }
                Some(_) | None => {
                    nearest = Some(candidate);
                }
            }
        }

        Ok(nearest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PathAnchor, Point2};

    #[test]
    fn topology_sums_all_contours() {
        let open_anchors = anchors(&[(0.0, 0.0), (10.0, 0.0)]);
        let closed_anchors = anchors(&[(20.0, 0.0), (30.0, 0.0), (30.0, 10.0)]);
        let compound = CompoundPathGeometry::from_specs([
            PathContourSpec {
                anchors: &open_anchors,
                closed: false,
            },
            PathContourSpec {
                anchors: &closed_anchors,
                closed: true,
            },
        ])
        .expect("compound path");

        assert_eq!(compound.contour_count(), 2);
        assert_eq!(
            compound.topology(),
            PathTopology {
                anchor_count: 5,
                segment_count: 4,
                open_subpath_count: 1,
                closed_subpath_count: 1,
            }
        );
        assert_eq!(compound.contours()[0].anchors()[0].point, point(0.0, 0.0));
        assert_eq!(compound.contours()[1].anchors()[0].point, point(20.0, 0.0));
    }

    #[test]
    fn bounds_union_non_empty_contours() {
        let compound = CompoundPathGeometry::new(vec![
            PathGeometry::new(Vec::new(), false).expect("empty path"),
            path(&[(5.0, 10.0), (10.0, 15.0)], false),
            path(&[(-2.0, -4.0), (1.0, 3.0)], false),
        ]);

        let bounds = compound
            .bounds()
            .expect("bounds should resolve")
            .expect("non-empty bounds");

        assert_eq!(bounds.min_x, -2.0);
        assert_eq!(bounds.min_y, -4.0);
        assert_eq!(bounds.max_x, 10.0);
        assert_eq!(bounds.max_y, 15.0);
    }

    #[test]
    fn flatten_contours_preserves_contour_boundaries() {
        let compound = CompoundPathGeometry::new(vec![
            path(&[(0.0, 0.0), (10.0, 0.0)], false),
            path(&[(20.0, 0.0), (30.0, 0.0), (30.0, 10.0)], true),
        ]);

        let flattened = compound.flatten_contours(0.25).expect("flatten");

        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].contour_index, 0);
        assert!(!flattened[0].closed);
        assert_eq!(flattened[0].points, vec![point(0.0, 0.0), point(10.0, 0.0)]);
        assert_eq!(flattened[1].contour_index, 1);
        assert!(flattened[1].closed);
        assert_eq!(flattened[1].points.first().copied(), Some(point(20.0, 0.0)));
    }

    #[test]
    fn project_reports_contour_index_with_earliest_tie() {
        let compound = CompoundPathGeometry::new(vec![
            path(&[(0.0, 0.0), (10.0, 0.0)], false),
            path(&[(0.0, 2.0), (10.0, 2.0)], false),
        ]);

        let projection = compound
            .project(point(5.0, 1.0), 0.25)
            .expect("projection")
            .expect("nearest point");

        assert_eq!(projection.contour_index, 0);
        assert_eq!(projection.projection.segment_index, 0);
        assert_eq!(projection.projection.point, point(5.0, 0.0));
        assert_eq!(projection.projection.distance_squared, 1.0);
    }

    #[test]
    fn empty_compound_path_has_zero_topology_and_no_geometry() {
        let compound = CompoundPathGeometry::new(Vec::new());

        assert_eq!(
            compound.topology(),
            PathTopology {
                anchor_count: 0,
                segment_count: 0,
                open_subpath_count: 0,
                closed_subpath_count: 0,
            }
        );
        assert_eq!(compound.bounds().expect("bounds"), None);
        assert_eq!(
            compound.project(point(0.0, 0.0), 0.25).expect("projection"),
            None
        );
        assert!(compound.flatten_contours(0.25).expect("flatten").is_empty());
    }

    #[test]
    fn transform_applies_to_all_contours() {
        let compound = CompoundPathGeometry::new(vec![path(&[(0.0, 0.0), (10.0, 0.0)], false)]);

        let transformed = compound
            .transform(AffineTransform::translation(3.0, 4.0).expect("translation"))
            .expect("transform");

        assert_eq!(
            transformed.contours()[0].anchors()[0].point,
            point(3.0, 4.0)
        );
        assert_eq!(
            transformed.contours()[0].anchors()[1].point,
            point(13.0, 4.0)
        );
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
