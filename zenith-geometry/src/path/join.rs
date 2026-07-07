use crate::Point2;

use super::{PathJoinVectors, ZERO_LENGTH_EPSILON};

impl PathJoinVectors {
    #[must_use]
    pub fn opposing_tangent_alignment(self) -> f64 {
        if self.in_length <= ZERO_LENGTH_EPSILON || self.out_length <= ZERO_LENGTH_EPSILON {
            return 0.0;
        }

        let normalized_in = Point2::new_unchecked(
            self.in_vector.x / self.in_length,
            self.in_vector.y / self.in_length,
        );
        let normalized_out = Point2::new_unchecked(
            self.out_vector.x / self.out_length,
            self.out_vector.y / self.out_length,
        );
        let dot = normalized_in
            .x
            .mul_add(normalized_out.x, normalized_in.y * normalized_out.y);

        (-dot).max(0.0).clamp(0.0, 1.0)
    }

    #[must_use]
    pub fn handle_length_balance(self) -> f64 {
        let shorter = self.in_length.min(self.out_length);
        let longer = self.in_length.max(self.out_length);
        if longer <= ZERO_LENGTH_EPSILON {
            0.0
        } else {
            (shorter / longer).clamp(0.0, 1.0)
        }
    }
}
