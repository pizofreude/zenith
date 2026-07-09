//! The import-graph loader: recursive filesystem traversal, per-import parsing,
//! hash verification, and cycle detection. Validation and diagnostic
//! constructors live in sibling submodules.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zenith_core::{Diagnostic, Document, ImportDecl, KdlAdapter, KdlSource};

use super::loaded::LoadedImportGraph;
use super::path::normalize_import_path;

/// Load every reachable `kind="zen"` composition import from `root`.
///
/// `root_dir` is the parent directory of the root `.zen` source. When absent,
/// imports cannot be resolved and each declaration yields `import.missing`.
/// Declared `sha256` values are always verified when present.
pub(crate) fn load_import_graph(root: &Document, root_dir: Option<&Path>) -> LoadedImportGraph {
    let mut loader = ImportGraphLoader {
        diagnostics: Vec::new(),
        documents: BTreeMap::new(),
        document_dirs: BTreeMap::new(),
        documents_by_path: BTreeMap::new(),
        stack: Vec::new(),
    };
    match root_dir {
        Some(dir) => loader.load_document_imports(root, dir),
        None => loader.report_unresolvable_root(root),
    }
    loader.validate_root_targets(root);
    loader.detect_id_collisions(root);
    loader.finish()
}

pub(super) struct ImportGraphLoader {
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) documents: BTreeMap<String, Document>,
    document_dirs: BTreeMap<String, PathBuf>,
    documents_by_path: BTreeMap<PathBuf, CachedImportDocument>,
    pub(super) stack: Vec<PathBuf>,
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
            document_dirs: self.document_dirs,
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
            if let Some(parent) = path.parent() {
                self.document_dirs
                    .insert(import.id.clone(), parent.to_path_buf());
            }
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
        let document_dir = path.parent().map(Path::to_path_buf);
        self.documents_by_path.insert(
            path,
            CachedImportDocument {
                document: doc.clone(),
                sha256: actual_sha256,
            },
        );
        if let Some(dir) = document_dir {
            self.document_dirs.insert(import.id.clone(), dir);
        }
        self.documents.insert(import.id.clone(), doc);
    }
}
