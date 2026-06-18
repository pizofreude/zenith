//! Style block and style placeholder types (full property parsing is deferred).

/// Placeholder for a named style definition.
///
/// Style property resolution is a later unit; this type is intentionally minimal
/// so that the style block can be parsed round-trip without loss.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// Globally unique style ID.
    pub id: String,
}

/// The top-level `styles` block containing named style definitions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleBlock {
    /// Ordered list of style definitions.
    pub styles: Vec<Style>,
}
