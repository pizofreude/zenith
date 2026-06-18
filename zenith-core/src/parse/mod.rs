//! Parse layer re-exports and the `KdlSource` trait.

pub mod kdl_adapter;
pub mod transform;

pub use kdl_adapter::KdlAdapter;

use crate::ast::Document;
use crate::error::ParseError;

/// Contract for a type that can parse `.zen` source bytes into a `Document`.
///
/// The only method in scope for Unit 2 is `parse`. A `format` method will be
/// added in Unit 3 once the canonical formatter is designed.
pub trait KdlSource {
    /// Parse raw `.zen` bytes into a `Document` AST.
    ///
    /// # Errors
    ///
    /// Returns a `ParseError` if the bytes are not valid UTF-8, not valid KDL,
    /// or do not conform to the minimal Zenith schema.
    fn parse(&self, source: &[u8]) -> Result<Document, ParseError>;
}
