//! Filesystem-backed `.zen` composition import graph loading.
//!
//! Core owns syntax and local validation. This module owns CLI-time file I/O:
//! resolving import paths relative to the importing document, parsing imported
//! documents, checking declared source hashes, and detecting graph cycles.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use zenith_core::{Diagnostic, Document, ImportDecl, KdlAdapter, KdlSource};

/// Parsed import graph plus diagnostics collected while traversing it.
#[derive(Debug)]
pub(crate) struct ImportGraph {
    diagnostics: Vec<Diagnostic>,
}

impl ImportGraph {
    /// Consume the graph and return diagnostics in deterministic traversal order.
    pub(crate) fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

/// Load every reachable `kind="zen"` composition import from `root`.
///
/// `root_dir` is the parent directory of the root `.zen` source. When absent,
/// imports cannot be resolved and each declaration yields `import.missing`.
/// Declared `sha256` values are always verified when present.
pub(crate) fn load_import_graph(root: &Document, root_dir: Option<&Path>) -> ImportGraph {
    let mut loader = ImportGraphLoader {
        diagnostics: Vec::new(),
        loaded_paths: BTreeSet::new(),
        stack: Vec::new(),
    };
    match root_dir {
        Some(dir) => loader.load_document_imports(root, dir),
        None => loader.report_unresolvable_root(root),
    }
    loader.finish()
}

struct ImportGraphLoader {
    diagnostics: Vec<Diagnostic>,
    loaded_paths: BTreeSet<PathBuf>,
    stack: Vec<PathBuf>,
}

impl ImportGraphLoader {
    fn finish(self) -> ImportGraph {
        ImportGraph {
            diagnostics: self.diagnostics,
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
        if self.loaded_paths.contains(&path) {
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

        self.verify_hash(import, &bytes);

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

        self.loaded_paths.insert(path.clone());
        self.stack.push(path.clone());
        if let Some(next_base) = path.parent() {
            self.load_document_imports(&doc, next_base);
        }
        self.stack.pop();
    }

    fn verify_hash(&mut self, import: &ImportDecl, bytes: &[u8]) {
        let Some(declared) = import.sha256.as_deref() else {
            return;
        };
        let actual = format!("{:x}", Sha256::digest(bytes));
        if !declared.trim().eq_ignore_ascii_case(&actual) {
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
        let mut chain: Vec<String> = self
            .stack
            .iter()
            .map(|path| path.display().to_string())
            .collect();
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
}
