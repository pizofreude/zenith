//! Canonical formatter for Zenith AST → `.zen` text.
//!
//! The only public surface is [`format_document`], which is delegated to by
//! `KdlAdapter::format`.

mod writer;

pub use writer::format_document;
