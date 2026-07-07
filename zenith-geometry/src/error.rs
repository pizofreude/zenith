use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    NonFinitePoint,
    NonFiniteTolerance,
    NonPositiveTolerance,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryError::NonFinitePoint => f.write_str("point coordinates must be finite"),
            GeometryError::NonFiniteTolerance => f.write_str("tolerance must be finite"),
            GeometryError::NonPositiveTolerance => f.write_str("tolerance must be positive"),
        }
    }
}

impl std::error::Error for GeometryError {}
