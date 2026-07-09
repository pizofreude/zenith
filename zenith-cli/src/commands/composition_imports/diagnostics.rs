//! Diagnostic constructors for the import-graph loader: hash verification and
//! the `import.*` error emitters.

use std::path::Path;

use zenith_core::{Diagnostic, ImportDecl};

use super::loader::ImportGraphLoader;

impl ImportGraphLoader {
    pub(super) fn verify_hash(&mut self, import: &ImportDecl, actual: &str) {
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

    pub(super) fn push_missing(&mut self, import: &ImportDecl, message: String) {
        self.diagnostics.push(Diagnostic::error(
            "import.missing",
            message,
            import.source_span,
            Some(import.id.clone()),
        ));
    }

    pub(super) fn push_cycle(&mut self, import: &ImportDecl, repeated: &Path) {
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

    pub(super) fn push_unknown_reference(
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

    pub(super) fn push_unsupported_target(
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
