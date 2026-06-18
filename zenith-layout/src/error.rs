//! Error types for zenith-layout.

/// An error produced by the text-layout layer.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutError {
    /// Human-readable description of what went wrong.
    pub message: String,
}

impl LayoutError {
    /// Construct a `LayoutError` with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LayoutError {}
