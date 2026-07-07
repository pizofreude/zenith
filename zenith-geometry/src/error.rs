use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    NonFinitePoint,
    NonFiniteParameter,
    ParameterOutOfRange,
    NonFiniteTolerance,
    NonPositiveTolerance,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryError::NonFinitePoint => f.write_str("point coordinates must be finite"),
            GeometryError::NonFiniteParameter => f.write_str("parameter must be finite"),
            GeometryError::ParameterOutOfRange => {
                f.write_str("parameter must be in the inclusive range [0.0, 1.0]")
            }
            GeometryError::NonFiniteTolerance => f.write_str("tolerance must be finite"),
            GeometryError::NonPositiveTolerance => f.write_str("tolerance must be positive"),
        }
    }
}

impl std::error::Error for GeometryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_parameter_errors() {
        assert_eq!(
            GeometryError::NonFiniteParameter.to_string(),
            "parameter must be finite"
        );
        assert_eq!(
            GeometryError::ParameterOutOfRange.to_string(),
            "parameter must be in the inclusive range [0.0, 1.0]"
        );
    }
}
