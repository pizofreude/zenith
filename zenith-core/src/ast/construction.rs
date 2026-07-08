//! Non-printing construction guide metadata.

use super::Span;
use super::value::Dimension;

/// A page-scoped construction block.
///
/// Construction guides are authoring scaffolds. They are parsed, formatted, and
/// validated by core, but the canonical render path ignores them unless a caller
/// explicitly requests an overlay.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConstructionBlock {
    pub guides: Vec<ConstructionGuideDef>,
}

/// A single construction guide declaration.
///
/// The guide type is intentionally preserved as a string so future guide kinds
/// can round-trip before the current validator understands them.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructionGuideDef {
    pub id: String,
    pub guide_type: String,
    pub x1: Option<Dimension>,
    pub y1: Option<Dimension>,
    pub x2: Option<Dimension>,
    pub y2: Option<Dimension>,
    pub cx: Option<Dimension>,
    pub cy: Option<Dimension>,
    pub r: Option<Dimension>,
    pub label: Option<String>,
    pub source_span: Option<Span>,
}
