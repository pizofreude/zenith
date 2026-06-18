//! Node types for the renderable layer of a `.zen` document.

use std::collections::BTreeMap;

use super::Span;
use super::value::{Dimension, PropertyValue};

/// A raw KDL value retained for an unrecognized property (forward-compat).
///
/// Storing the serialized string keeps the AST lossless for round-trip
/// without pulling the full KDL entry into every node struct.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownProperty {
    /// The serialized representation of the raw KDL value (e.g. `"42"`, `"#true"`, `"\"hello\""`).
    pub raw: String,
}

/// A text content span — a run of text with optional inline style overrides.
///
/// This is deliberately named `TextSpan` to avoid colliding with the source-
/// location type [`Span`].
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    /// The literal text content.
    pub text: String,
    /// Per-span fill override (usually a token ref).
    pub fill: Option<PropertyValue>,
    /// Per-span font-weight override.
    pub font_weight: Option<PropertyValue>,
    /// Italic override.
    pub italic: Option<bool>,
    /// Underline decoration.
    pub underline: Option<bool>,
    /// Strikethrough decoration.
    pub strikethrough: Option<bool>,
}

/// A `rect` node.
#[derive(Debug, Clone, PartialEq)]
pub struct RectNode {
    pub id: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub x: Option<Dimension>,
    pub y: Option<Dimension>,
    pub w: Option<Dimension>,
    pub h: Option<Dimension>,
    pub radius: Option<PropertyValue>,
    pub style: Option<String>,
    pub fill: Option<PropertyValue>,
    pub stroke: Option<PropertyValue>,
    pub stroke_width: Option<PropertyValue>,
    pub stroke_alignment: Option<String>,
    pub opacity: Option<f64>,
    pub visible: Option<bool>,
    pub locked: Option<bool>,
    pub rotate: Option<Dimension>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Unknown properties preserved for forward-compat.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}

/// A `text` node.
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    pub id: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub x: Option<Dimension>,
    pub y: Option<Dimension>,
    pub w: Option<Dimension>,
    pub h: Option<Dimension>,
    pub align: Option<String>,
    pub direction: Option<String>,
    pub overflow: Option<String>,
    pub style: Option<String>,
    pub fill: Option<PropertyValue>,
    pub font_family: Option<PropertyValue>,
    pub font_size: Option<PropertyValue>,
    pub opacity: Option<f64>,
    pub visible: Option<bool>,
    pub locked: Option<bool>,
    pub rotate: Option<Dimension>,
    /// Inline text spans.
    pub spans: Vec<TextSpan>,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
    /// Unknown properties preserved for forward-compat.
    pub unknown_props: BTreeMap<String, UnknownProperty>,
}

/// An unrecognized node kind, preserved for forward-compat.
///
/// When a `.zen` document contains a node kind that this binary does not
/// recognise (e.g. authored with a newer version), the node is wrapped in this
/// variant instead of triggering a hard error.
#[derive(Debug, Clone, PartialEq)]
pub struct UnknownNode {
    /// The KDL node name (e.g. `"sparkle"`, `"table"`, `"chart"`).
    pub kind: String,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
}

/// A renderable content node within a page.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Rect(RectNode),
    Text(TextNode),
    Unknown(UnknownNode),
}
