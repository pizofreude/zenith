//! `zenith inspect` command — module wiring.
//!
//! - `document`    — error type, tree types, tree builders, node finder,
//!   geometry helpers, human renderers, and the public `run` entry point.
//! - [`path`]        — `zenith inspect path` topology / bounds / craft.
//! - [`recipes`]     — recipe-block JSON builder and human renderer.

mod document;
pub mod path;
pub mod recipes;

pub use document::{
    InspectCmdErr, InspectNodeOutput, InspectOutput, NodeEntry, NodeGeometry, PageEntry,
    build_doc_tree, find_node_tree, run, summary,
};
