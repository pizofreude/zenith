//! In-memory import graph support for scene compilation.
//!
//! The scene crate owns only borrowed, already-parsed documents here. It does
//! not perform filesystem or CLI lookups.

use std::collections::BTreeMap;

use zenith_core::{ComponentDef, Diagnostic, Document, ResolvedToken, Style, resolve_tokens};

use super::ComponentMap;

/// A parsed document made available to scene compilation as an import.
#[derive(Debug, Clone, Copy)]
pub struct ImportedDocument<'a> {
    /// The imported document AST.
    pub document: &'a Document,
}

impl<'a> ImportedDocument<'a> {
    /// Create an imported-document entry from an already parsed document.
    pub fn new(document: &'a Document) -> Self {
        Self { document }
    }
}

/// Deterministic, filesystem-free graph of imported documents.
#[derive(Debug, Clone, Default)]
pub struct ImportGraph<'a> {
    documents: BTreeMap<String, ImportedDocument<'a>>,
}

impl<'a> ImportGraph<'a> {
    /// Create an empty import graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace an import id with a parsed document.
    pub fn insert(&mut self, id: impl Into<String>, document: &'a Document) {
        self.documents
            .insert(id.into(), ImportedDocument::new(document));
    }

    /// Return a new graph containing `id -> document`.
    pub fn with_document(mut self, id: impl Into<String>, document: &'a Document) -> Self {
        self.insert(id, document);
        self
    }
}

pub(in crate::compile) struct ImportScopes<'a> {
    enabled: bool,
    scopes: BTreeMap<String, ImportedScope<'a>>,
}

impl<'a> ImportScopes<'a> {
    pub(in crate::compile) fn disabled() -> Self {
        Self {
            enabled: false,
            scopes: BTreeMap::new(),
        }
    }

    pub(in crate::compile) fn from_graph(
        graph: &'a ImportGraph<'a>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut scopes = BTreeMap::new();
        for (id, imported) in &graph.documents {
            let token_resolution = resolve_tokens(&imported.document.tokens);
            diagnostics.extend(token_resolution.diagnostics);

            let style_map: BTreeMap<&str, &Style> = imported
                .document
                .styles
                .styles
                .iter()
                .map(|style| (style.id.as_str(), style))
                .collect();

            let mut component_map: BTreeMap<&str, &ComponentDef> = BTreeMap::new();
            for component in &imported.document.components {
                component_map
                    .entry(component.id.as_str())
                    .or_insert(component);
            }

            scopes.insert(
                id.clone(),
                ImportedScope {
                    document: imported.document,
                    resolved: token_resolution.resolved,
                    style_map,
                    components: component_map,
                },
            );
        }

        Self {
            enabled: true,
            scopes,
        }
    }

    pub(in crate::compile) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(in crate::compile) fn get(&self, id: &str) -> Option<&ImportedScope<'a>> {
        self.scopes.get(id)
    }
}

pub(in crate::compile) struct ImportedScope<'a> {
    pub(in crate::compile) document: &'a Document,
    pub(in crate::compile) resolved: BTreeMap<String, ResolvedToken>,
    pub(in crate::compile) style_map: BTreeMap<&'a str, &'a Style>,
    pub(in crate::compile) components: ComponentMap<'a>,
}

pub(in crate::compile) enum ImportSource<'a> {
    Component {
        import_id: &'a str,
        component_id: &'a str,
    },
    UnsupportedTarget {
        import_id: &'a str,
        target: &'a str,
    },
    Invalid,
}

pub(in crate::compile) fn parse_import_source(source: &str) -> ImportSource<'_> {
    let Some((import_id, target)) = source.split_once('#') else {
        return ImportSource::Invalid;
    };
    if import_id.is_empty() || target.is_empty() {
        return ImportSource::Invalid;
    }

    if let Some(component_id) = target.strip_prefix("component.") {
        if component_id.is_empty() {
            return ImportSource::Invalid;
        }
        return ImportSource::Component {
            import_id,
            component_id,
        };
    }

    ImportSource::UnsupportedTarget { import_id, target }
}
