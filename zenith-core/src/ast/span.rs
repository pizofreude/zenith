//! Source-location span type.

/// A source-location span (byte offsets into the original UTF-8 source).
///
/// Named `Span` — distinct from [`super::TextSpan`], which represents a text content run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Byte offset of the first character (inclusive).
    pub start: usize,
    /// Byte offset past the last character (exclusive).
    pub end: usize,
}
