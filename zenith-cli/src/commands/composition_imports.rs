//! Filesystem-backed `.zen` composition import graph loading.
//!
//! Core owns syntax and local validation. This module owns CLI-time file I/O:
//! resolving import paths relative to the importing document, parsing imported
//! documents, checking declared source hashes, and detecting graph cycles.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use zenith_core::{
    Diagnostic, Dimension, Document, ImportDecl, KdlAdapter, KdlSource, Node, Page, dim_to_px,
};
use zenith_scene::ImportGraph as SceneImportGraph;

/// Parsed import graph plus diagnostics collected while traversing it.
#[derive(Debug)]
pub(crate) struct LoadedImportGraph {
    diagnostics: Vec<Diagnostic>,
    documents: BTreeMap<String, Document>,
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
}

/// Load every reachable `kind="zen"` composition import from `root`.
///
/// `root_dir` is the parent directory of the root `.zen` source. When absent,
/// imports cannot be resolved and each declaration yields `import.missing`.
/// Declared `sha256` values are always verified when present.
pub(crate) fn load_import_graph(root: &Document, root_dir: Option<&Path>) -> LoadedImportGraph {
    let mut loader = ImportGraphLoader {
        diagnostics: Vec::new(),
        documents: BTreeMap::new(),
        documents_by_path: BTreeMap::new(),
        stack: Vec::new(),
    };
    match root_dir {
        Some(dir) => loader.load_document_imports(root, dir),
        None => loader.report_unresolvable_root(root),
    }
    loader.validate_root_targets(root);
    loader.finish()
}

struct ImportGraphLoader {
    diagnostics: Vec<Diagnostic>,
    documents: BTreeMap<String, Document>,
    documents_by_path: BTreeMap<PathBuf, CachedImportDocument>,
    stack: Vec<PathBuf>,
}

#[derive(Debug)]
struct CachedImportDocument {
    document: Document,
    sha256: String,
}

impl ImportGraphLoader {
    fn finish(self) -> LoadedImportGraph {
        LoadedImportGraph {
            diagnostics: self.diagnostics,
            documents: self.documents,
        }
    }

    fn report_unresolvable_root(&mut self, doc: &Document) {
        for import in &doc.imports {
            if import.kind == "zen" {
                self.push_missing(
                    import,
                    format!(
                        "import '{}' cannot be resolved without a project directory",
                        import.id
                    ),
                );
            }
        }
    }

    fn load_document_imports(&mut self, doc: &Document, base_dir: &Path) {
        for import in &doc.imports {
            if import.kind != "zen" {
                continue;
            }
            self.load_one_import(import, base_dir);
        }
    }

    fn load_one_import(&mut self, import: &ImportDecl, base_dir: &Path) {
        let path = normalize_import_path(base_dir, &import.src);

        if self.stack.contains(&path) {
            self.push_cycle(import, &path);
            return;
        }
        if let Some(cached) = self.documents_by_path.get(&path) {
            let cached_sha256 = cached.sha256.clone();
            let cached_document = cached.document.clone();
            self.verify_hash(import, &cached_sha256);
            self.documents.insert(import.id.clone(), cached_document);
            return;
        }

        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.push_missing(
                    import,
                    format!(
                        "import '{}' file not found: '{}': {}",
                        import.id,
                        path.display(),
                        err
                    ),
                );
                return;
            }
        };

        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        self.verify_hash(import, &actual_sha256);

        let doc = match KdlAdapter.parse(bytes.as_slice()) {
            Ok(doc) => doc,
            Err(err) => {
                self.diagnostics.push(Diagnostic::error(
                    "import.parse_error",
                    format!(
                        "import '{}' could not be parsed from '{}': {}",
                        import.id,
                        path.display(),
                        err.message
                    ),
                    import.source_span,
                    Some(import.id.clone()),
                ));
                return;
            }
        };

        self.stack.push(path.clone());
        if let Some(next_base) = path.parent() {
            self.load_document_imports(&doc, next_base);
        }
        self.stack.pop();
        self.documents_by_path.insert(
            path,
            CachedImportDocument {
                document: doc.clone(),
                sha256: actual_sha256,
            },
        );
        self.documents.insert(import.id.clone(), doc);
    }

    fn validate_root_targets(&mut self, root: &Document) {
        let declared_imports: BTreeMap<&str, &ImportDecl> = root
            .imports
            .iter()
            .map(|import| (import.id.as_str(), import))
            .collect();
        for page in &root.body.pages {
            self.validate_page_source(page, &declared_imports);
            self.validate_node_sources(&page.children, &declared_imports);
        }
        for component in &root.components {
            self.validate_node_sources(&component.children, &declared_imports);
        }
        for master in &root.masters {
            self.validate_node_sources(&master.children, &declared_imports);
        }
    }

    fn validate_page_source(
        &mut self,
        page: &Page,
        declared_imports: &BTreeMap<&str, &ImportDecl>,
    ) {
        let Some(source) = page.source.as_deref() else {
            return;
        };
        match parse_import_source(source) {
            ImportSource::Page { import_id, page_id } => {
                let Some(imported) = self.imported_document_for_reference(
                    import_id,
                    declared_imports,
                    page.source_span,
                ) else {
                    return;
                };
                let Some(imported_page) = imported
                    .body
                    .pages
                    .iter()
                    .find(|candidate| candidate.id == page_id)
                else {
                    self.push_unknown_reference(
                        format!(
                            "page '{}' source references unknown page '{}' in import '{}'",
                            page.id, page_id, import_id
                        ),
                        page.source_span,
                        Some(page.id.clone()),
                    );
                    return;
                };
                if page.fit.is_none() && !same_page_size(page, imported_page) {
                    self.diagnostics.push(Diagnostic::error(
                        "import.page_size_mismatch",
                        format!(
                            "page '{}' source '{}' has different dimensions and no explicit fit",
                            page.id, source
                        ),
                        page.source_span,
                        Some(page.id.clone()),
                    ));
                }
            }
            ImportSource::Component { .. }
            | ImportSource::UnsupportedTarget
            | ImportSource::Invalid => self.push_unsupported_target(
                format!(
                    "page '{}' source '{}' is not a supported page target",
                    page.id, source
                ),
                page.source_span,
                Some(page.id.clone()),
            ),
        }
    }

    fn validate_node_sources(
        &mut self,
        nodes: &[Node],
        declared_imports: &BTreeMap<&str, &ImportDecl>,
    ) {
        for node in nodes {
            match node {
                Node::Frame(frame) => self.validate_node_sources(&frame.children, declared_imports),
                Node::Group(group) => self.validate_node_sources(&group.children, declared_imports),
                Node::Table(table) => {
                    for row in &table.rows {
                        for cell in &row.cells {
                            self.validate_node_sources(&cell.children, declared_imports);
                        }
                    }
                }
                Node::Instance(instance) => {
                    if let Some(source) = instance.source.as_deref() {
                        match parse_import_source(source) {
                            ImportSource::Component {
                                import_id,
                                component_id,
                            } => {
                                let Some(imported) = self.imported_document_for_reference(
                                    import_id,
                                    declared_imports,
                                    instance.source_span,
                                ) else {
                                    continue;
                                };
                                if !imported
                                    .components
                                    .iter()
                                    .any(|component| component.id == component_id)
                                {
                                    self.push_unknown_reference(
                                        format!(
                                            "instance '{}' source references unknown component '{}' in import '{}'",
                                            instance.id, component_id, import_id
                                        ),
                                        instance.source_span,
                                        Some(instance.id.clone()),
                                    );
                                }
                            }
                            ImportSource::Page { .. }
                            | ImportSource::UnsupportedTarget
                            | ImportSource::Invalid => self.push_unsupported_target(
                                format!(
                                    "instance '{}' source '{}' is not a supported component target",
                                    instance.id, source
                                ),
                                instance.source_span,
                                Some(instance.id.clone()),
                            ),
                        }
                    }
                }
                Node::Unknown(unknown) => {
                    self.validate_node_sources(&unknown.children, declared_imports);
                }
                Node::Rect(_)
                | Node::Ellipse(_)
                | Node::Line(_)
                | Node::Text(_)
                | Node::Code(_)
                | Node::Image(_)
                | Node::Polygon(_)
                | Node::Polyline(_)
                | Node::Path(_)
                | Node::Field(_)
                | Node::Footnote(_)
                | Node::Toc(_)
                | Node::Shape(_)
                | Node::Connector(_)
                | Node::Pattern(_)
                | Node::Chart(_)
                | Node::Light(_)
                | Node::Mesh(_) => {}
            }
        }
    }

    fn imported_document_for_reference(
        &mut self,
        import_id: &str,
        declared_imports: &BTreeMap<&str, &ImportDecl>,
        span: Option<zenith_core::Span>,
    ) -> Option<&Document> {
        if self.documents.contains_key(import_id) {
            return self.documents.get(import_id);
        }
        if declared_imports.contains_key(import_id) {
            return None;
        }
        self.push_unknown_reference(
            format!("source references undeclared import '{}'", import_id),
            span,
            Some(import_id.to_owned()),
        );
        None
    }

    fn verify_hash(&mut self, import: &ImportDecl, actual: &str) {
        let Some(declared) = import.sha256.as_deref() else {
            return;
        };
        if !declared.trim().eq_ignore_ascii_case(actual) {
            self.diagnostics.push(Diagnostic::error(
                "import.hash_mismatch",
                format!(
                    "import '{}' sha256 mismatch (declared {}, actual {})",
                    import.id, declared, actual
                ),
                import.source_span,
                Some(import.id.clone()),
            ));
        }
    }

    fn push_missing(&mut self, import: &ImportDecl, message: String) {
        self.diagnostics.push(Diagnostic::error(
            "import.missing",
            message,
            import.source_span,
            Some(import.id.clone()),
        ));
    }

    fn push_cycle(&mut self, import: &ImportDecl, repeated: &Path) {
        let mut chain = Vec::with_capacity(self.stack.len() + 1);
        chain.extend(self.stack.iter().map(|path| path.display().to_string()));
        chain.push(repeated.display().to_string());
        self.diagnostics.push(Diagnostic::error(
            "import.cycle",
            format!(
                "import '{}' forms a cycle: {}",
                import.id,
                chain.join(" -> ")
            ),
            import.source_span,
            Some(import.id.clone()),
        ));
    }

    fn push_unknown_reference(
        &mut self,
        message: String,
        span: Option<zenith_core::Span>,
        subject_id: Option<String>,
    ) {
        self.diagnostics.push(Diagnostic::error(
            "import.unknown_reference",
            message,
            span,
            subject_id,
        ));
    }

    fn push_unsupported_target(
        &mut self,
        message: String,
        span: Option<zenith_core::Span>,
        subject_id: Option<String>,
    ) {
        self.diagnostics.push(Diagnostic::error(
            "import.unsupported_target",
            message,
            span,
            subject_id,
        ));
    }
}

enum ImportSource<'a> {
    Component {
        import_id: &'a str,
        component_id: &'a str,
    },
    Page {
        import_id: &'a str,
        page_id: &'a str,
    },
    UnsupportedTarget,
    Invalid,
}

fn parse_import_source(source: &str) -> ImportSource<'_> {
    let Some((import_id, target)) = source.split_once('#') else {
        return ImportSource::Invalid;
    };
    if import_id.is_empty() || target.is_empty() || target.contains('#') {
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

    if let Some(page_id) = target.strip_prefix("page.") {
        if page_id.is_empty() {
            return ImportSource::Invalid;
        }
        return ImportSource::Page { import_id, page_id };
    }

    ImportSource::UnsupportedTarget
}

fn same_page_size(host: &Page, imported: &Page) -> bool {
    same_dimension(&host.width, &imported.width) && same_dimension(&host.height, &imported.height)
}

fn same_dimension(left: &Dimension, right: &Dimension) -> bool {
    match (
        dim_to_px(left.value, &left.unit),
        dim_to_px(right.value, &right.unit),
    ) {
        (Some(left_px), Some(right_px)) => (left_px - right_px).abs() <= f64::EPSILON,
        _ => left == right,
    }
}

fn normalize_import_path(base_dir: &Path, src: &str) -> PathBuf {
    let raw = Path::new(src);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    };
    normalize_lexically(&joined)
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let can_pop = normalized
                    .components()
                    .next_back()
                    .is_some_and(|last| matches!(last, Component::Normal(_)));
                if can_pop {
                    normalized.pop();
                } else {
                    normalized.push("..");
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const EMPTY_DOC: &str = r#"zenith version=1 {
  project id="proj.empty" name="Empty"
  document id="doc.empty" title="Empty" {
    page id="page.empty" w=(px)100 h=(px)100
  }
}
"#;

    fn parse(src: &str) -> Document {
        KdlAdapter
            .parse(src.as_bytes())
            .expect("test document must parse")
    }

    fn root_with_import(src: &str, extra: &str) -> Document {
        parse(&format!(
            r#"zenith version=1 {{
  project id="proj.root" name="Root"
  imports {{
    import id="child" kind="zen" src="{src}"{extra}
  }}
  document id="doc.root" title="Root" {{
    page id="page.root" w=(px)100 h=(px)100
  }}
}}
"#
        ))
    }

    fn root_with_imports(imports: &str) -> Document {
        parse(&format!(
            r#"zenith version=1 {{
  project id="proj.root" name="Root"
  imports {{
{imports}
  }}
  document id="doc.root" title="Root" {{
    page id="page.root" w=(px)100 h=(px)100
  }}
}}
"#
        ))
    }

    fn root_with_import_and_body(src: &str, body: &str) -> Document {
        parse(&format!(
            r#"zenith version=1 {{
  project id="proj.root" name="Root"
  imports {{
    import id="child" kind="zen" src="{src}"
  }}
  document id="doc.root" title="Root" {{
{body}
  }}
}}
"#
        ))
    }

    fn imported_with_component_and_page(
        component_id: &str,
        page_id: &str,
        w: f64,
        h: f64,
    ) -> String {
        format!(
            r#"zenith version=1 {{
  project id="proj.child" name="Child"
  document id="doc.child" title="Child" {{
    page id="{page_id}" w=(px){w} h=(px){h}
  }}
  components {{
    component id="{component_id}" {{
      rect id="mark" x=(px)0 y=(px)0 w=(px)10 h=(px)10
    }}
  }}
}}
"#
        )
    }

    #[test]
    fn load_import_graph_resolves_relative_imports() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::create_dir(dir.path().join("modules")).expect("create modules dir");
        fs::write(dir.path().join("modules/child.zen"), EMPTY_DOC).expect("write child");
        let root = root_with_import("modules/child.zen", "");

        let graph = load_import_graph(&root, Some(dir.path()));

        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    }

    #[test]
    fn load_import_graph_reports_missing_import() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = root_with_import("missing.zen", "");

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.missing");
        assert_eq!(diagnostics[0].subject_id.as_deref(), Some("child"));
    }

    #[test]
    fn load_import_graph_keeps_same_file_import_aliases() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("shared.zen"), EMPTY_DOC).expect("write shared");
        let root = root_with_imports(
            r#"    import id="first" kind="zen" src="shared.zen"
    import id="second" kind="zen" src="./shared.zen""#,
        );

        let graph = load_import_graph(&root, Some(dir.path()));

        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
        assert!(graph.documents.contains_key("first"));
        assert!(graph.documents.contains_key("second"));
    }

    #[test]
    fn load_import_graph_reports_parse_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("bad.zen"), "not zenith").expect("write bad child");
        let root = root_with_import("bad.zen", "");

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.parse_error");
    }

    #[test]
    fn load_import_graph_reports_hash_mismatch() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("child.zen"), EMPTY_DOC).expect("write child");
        let root = root_with_import("child.zen", r#" sha256="0000""#);

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.hash_mismatch");
    }

    #[test]
    fn load_import_graph_reports_hash_mismatch_for_cached_alias() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("shared.zen"), EMPTY_DOC).expect("write shared");
        let root = root_with_imports(
            r#"    import id="first" kind="zen" src="shared.zen"
    import id="second" kind="zen" src="./shared.zen" sha256="0000""#,
        );

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.hash_mismatch");
        assert_eq!(diagnostics[0].subject_id.as_deref(), Some("second"));
    }

    #[test]
    fn load_import_graph_reports_cycles() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join("a.zen"),
            r#"zenith version=1 {
  project id="proj.a" name="A"
  imports {
    import id="b" kind="zen" src="b.zen"
  }
  document id="doc.a" title="A" {
    page id="page.a" w=(px)100 h=(px)100
  }
}
"#,
        )
        .expect("write a");
        fs::write(
            dir.path().join("b.zen"),
            r#"zenith version=1 {
  project id="proj.b" name="B"
  imports {
    import id="a" kind="zen" src="a.zen"
  }
  document id="doc.b" title="B" {
    page id="page.b" w=(px)100 h=(px)100
  }
}
"#,
        )
        .expect("write b");
        let root = root_with_import("a.zen", "");

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.cycle");
    }

    #[test]
    fn load_import_graph_reports_unknown_component_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join("child.zen"),
            imported_with_component_and_page("component.card", "cover", 100.0, 100.0),
        )
        .expect("write child");
        let root = root_with_import_and_body(
            "child.zen",
            r#"    page id="page.root" w=(px)100 h=(px)100 {
      instance id="inst.missing" source="child#component.missing" x=(px)0 y=(px)0
    }"#,
        );

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.unknown_reference");
        assert_eq!(diagnostics[0].subject_id.as_deref(), Some("inst.missing"));
    }

    #[test]
    fn load_import_graph_reports_unsupported_instance_page_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join("child.zen"),
            imported_with_component_and_page("component.card", "cover", 100.0, 100.0),
        )
        .expect("write child");
        let root = root_with_import_and_body(
            "child.zen",
            r#"    page id="page.root" w=(px)100 h=(px)100 {
      instance id="inst.page" source="child#page.cover" x=(px)0 y=(px)0
    }"#,
        );

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.unsupported_target");
        assert_eq!(diagnostics[0].subject_id.as_deref(), Some("inst.page"));
    }

    #[test]
    fn load_import_graph_reports_unknown_page_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join("child.zen"),
            imported_with_component_and_page("component.card", "cover", 100.0, 100.0),
        )
        .expect("write child");
        let root = root_with_import_and_body(
            "child.zen",
            r#"    page id="page.root" source="child#page.missing" w=(px)100 h=(px)100"#,
        );

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.unknown_reference");
        assert_eq!(diagnostics[0].subject_id.as_deref(), Some("page.root"));
    }

    #[test]
    fn load_import_graph_reports_page_size_mismatch() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join("child.zen"),
            imported_with_component_and_page("component.card", "cover", 200.0, 100.0),
        )
        .expect("write child");
        let root = root_with_import_and_body(
            "child.zen",
            r#"    page id="page.root" source="child#page.cover" w=(px)100 h=(px)100"#,
        );

        let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "import.page_size_mismatch");
        assert_eq!(diagnostics[0].subject_id.as_deref(), Some("page.root"));
    }
}
