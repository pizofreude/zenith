//! Parse error types for zenith-core.

use crate::ast::Span;

/// Codes that identify the category of a parse error.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorCode {
    /// The input bytes are not valid UTF-8.
    NotUtf8,
    /// The UTF-8 source is not valid KDL.
    InvalidKdl,
    /// No top-level `zenith` node was found in the document.
    MissingZenithRoot,
    /// A node appeared in a context where it is not expected.
    UnexpectedNode,
    /// A property value could not be parsed into the expected type.
    InvalidPropertyValue,
}

/// A single error emitted by the parse layer.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Source span of the offending token or node, if available.
    pub span: Option<Span>,
    /// Stable error category code.
    pub code: ParseErrorCode,
    /// Human-readable description of the error.
    pub message: String,
}

impl ParseError {
    /// Construct a `ParseError` without a source span.
    pub fn spanless(code: ParseErrorCode, message: impl Into<String>) -> Self {
        Self {
            span: None,
            code,
            message: message.into(),
        }
    }

    /// Construct a `ParseError` with an explicit source span.
    pub fn with_span(code: ParseErrorCode, span: Span, message: impl Into<String>) -> Self {
        Self {
            span: Some(span),
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}
