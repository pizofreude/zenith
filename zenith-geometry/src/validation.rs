use crate::GeometryError;

pub(crate) fn validate_parameter(t: f64) -> Result<(), GeometryError> {
    if !t.is_finite() {
        Err(GeometryError::NonFiniteParameter)
    } else if !(0.0..=1.0).contains(&t) {
        Err(GeometryError::ParameterOutOfRange)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_tolerance(tolerance: f64) -> Result<(), GeometryError> {
    if !tolerance.is_finite() {
        Err(GeometryError::NonFiniteTolerance)
    } else if tolerance <= 0.0 {
        Err(GeometryError::NonPositiveTolerance)
    } else {
        Ok(())
    }
}
