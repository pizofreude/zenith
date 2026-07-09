//! Filesystem-backed `.zen` composition import graph loading.
//!
//! Core owns syntax and local validation. This module owns CLI-time file I/O:
//! resolving import paths relative to the importing document, parsing imported
//! documents, checking declared source hashes, and detecting graph cycles.
//!
//! Wiring only; the concerns live in submodules:
//! - `loaded` — the [`LoadedImportGraph`] result type and import edge records.
//! - `loader` — recursive traversal, parsing, hash verification, cycle detection.
//! - `list` — read-only `zenith imports list` formatting over the loaded graph.
//! - `validate` — root-target validation and expanded-id collision detection.
//! - `diagnostics` — the `import.*` diagnostic constructors.
//! - `source` — import-source string parsing.
//! - `walk` — node-tree walks and page-size comparison.
//! - `path` — import-path normalization.

mod diagnostics;
mod list;
mod loaded;
mod loader;
mod path;
mod source;
mod validate;
mod walk;

#[cfg(test)]
mod tests;

pub(crate) use list::run as list_imports;
pub(crate) use loaded::LoadedImportGraph;
pub(crate) use loader::load_import_graph;
