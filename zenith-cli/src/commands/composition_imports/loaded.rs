//! The [`LoadedImportGraph`] result type: parsed import documents plus the
//! diagnostics collected while traversing the graph.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use zenith_core::{Diagnostic, Document};
use zenith_scene::ImportGraph as SceneImportGraph;

/// Parsed import graph plus diagnostics collected while traversing it.
#[derive(Debug)]
pub(crate) struct LoadedImportGraph {
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) documents: BTreeMap<String, Document>,
    pub(super) document_dirs: BTreeMap<String, PathBuf>,
}

impl LoadedImportGraph {
    /// Consume the graph and return diagnostics in deterministic traversal order.
    pub(crate) fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Diagnostics collected while loading imports.
    pub(crate) fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Build a borrowed scene import graph for compile-time expansion.
    pub(crate) fn to_scene_graph(&self) -> SceneImportGraph<'_> {
        let mut graph = SceneImportGraph::new();
        for (id, doc) in &self.documents {
            graph.insert(id.clone(), doc);
        }
        graph
    }

    pub(crate) fn documents_with_dirs(&self) -> impl Iterator<Item = (&str, &Document, &Path)> {
        self.documents.iter().filter_map(|(id, doc)| {
            self.document_dirs
                .get(id)
                .map(|dir| (id.as_str(), doc, dir.as_path()))
        })
    }
}
