//! White-box unit tests for the canonical writer's private helpers.
//!
//! Only the tests that exercise a NON-public writer primitive remain here; they
//! cannot move to `tests/` because they reach into `super::` internals. The
//! round-trip / public-API suite lives in `zenith-core/tests/format_writer*`.

#![cfg(test)]

use super::*;

/// **Number formatting**: integral `f64` emits without decimal point.
#[test]
fn test_number_formatting_integral() {
    use crate::ast::{Dimension, Unit};
    let d = Dimension {
        value: 640.0,
        unit: Unit::Px,
    };
    assert_eq!(
        fmt_dimension(&d),
        "(px)640",
        "(px)640.0 must format as (px)640"
    );
}

/// **Number formatting**: non-integral value keeps its decimal.
#[test]
fn test_number_formatting_non_integral() {
    use crate::ast::{Dimension, Unit};
    let d = Dimension {
        value: 10.5,
        unit: Unit::Pt,
    };
    assert_eq!(fmt_dimension(&d), "(pt)10.5");
}
