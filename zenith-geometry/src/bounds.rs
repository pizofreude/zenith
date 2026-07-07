use crate::Point2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl RectBounds {
    #[must_use]
    pub fn from_point(point: Point2) -> Self {
        Self {
            min_x: point.x,
            min_y: point.y,
            max_x: point.x,
            max_y: point.y,
        }
    }

    #[must_use]
    pub fn include_point(self, point: Point2) -> Self {
        Self {
            min_x: self.min_x.min(point.x),
            min_y: self.min_y.min(point.y),
            max_x: self.max_x.max(point.x),
            max_y: self.max_y.max(point.y),
        }
    }

    #[must_use]
    pub fn include_bounds(self, bounds: Self) -> Self {
        Self {
            min_x: self.min_x.min(bounds.min_x),
            min_y: self.min_y.min(bounds.min_y),
            max_x: self.max_x.max(bounds.max_x),
            max_y: self.max_y.max(bounds.max_y),
        }
    }

    #[must_use]
    pub fn width(self) -> f64 {
        self.max_x - self.min_x
    }

    #[must_use]
    pub fn height(self) -> f64 {
        self.max_y - self.min_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_to_include_points() {
        let bounds = RectBounds::from_point(Point2::new_unchecked(2.0, 3.0))
            .include_point(Point2::new_unchecked(-1.0, 5.0))
            .include_point(Point2::new_unchecked(4.0, -2.0));

        assert_eq!(
            bounds,
            RectBounds {
                min_x: -1.0,
                min_y: -2.0,
                max_x: 4.0,
                max_y: 5.0
            }
        );
        assert_eq!(bounds.width(), 5.0);
        assert_eq!(bounds.height(), 7.0);
    }

    #[test]
    fn expands_to_include_bounds() {
        let bounds =
            RectBounds::from_point(Point2::new_unchecked(2.0, 3.0)).include_bounds(RectBounds {
                min_x: -4.0,
                min_y: 1.0,
                max_x: 8.0,
                max_y: 9.0,
            });

        assert_eq!(
            bounds,
            RectBounds {
                min_x: -4.0,
                min_y: 1.0,
                max_x: 8.0,
                max_y: 9.0
            }
        );
    }
}
