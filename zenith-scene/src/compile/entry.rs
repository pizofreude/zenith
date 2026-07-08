//! Public scene compilation entry points.

use zenith_core::{DataContext, Document, FontProvider};

use super::imports::ImportGraph;
use super::{CompileResult, compile_page_inner};

/// Compile `doc` into a [`CompileResult`], using `fonts` to shape text nodes.
///
/// [`compile_page`] renders a chosen page; this wrapper renders page 0. If the
/// document has no pages an empty scene is returned with an advisory diagnostic.
///
/// Pass `&zenith_core::default_provider()` to use the bundled Noto Sans font,
/// which is sufficient for basic text rendering.
pub fn compile(doc: &Document, fonts: &dyn FontProvider) -> CompileResult {
    compile_page(doc, fonts, 0, None)
}

/// Compile the page at `page_index` (0-based) of `doc` into a [`CompileResult`],
/// using `fonts` to shape text nodes.
///
/// If the document has no pages an empty scene is returned with a
/// `scene.no_pages` advisory; if `page_index` is out of range (but pages exist)
/// an empty scene is returned with a `scene.page_out_of_range` advisory.
///
/// Pass `Some(&data_ctx)` to resolve `(data)"field.path"` property references at
/// compile time. Pass `None` to skip data binding.
pub fn compile_page(
    doc: &Document,
    fonts: &dyn FontProvider,
    page_index: usize,
    data: Option<&DataContext>,
) -> CompileResult {
    compile_page_inner(doc, fonts, page_index, data, None)
}

/// Compile the page at `page_index` with an explicit in-memory import graph.
///
/// Imported documents must already be parsed by the caller. Scene compilation
/// performs no filesystem or CLI lookup.
pub fn compile_page_with_imports(
    doc: &Document,
    fonts: &dyn FontProvider,
    page_index: usize,
    data: Option<&DataContext>,
    imports: &ImportGraph<'_>,
) -> CompileResult {
    compile_page_inner(doc, fonts, page_index, data, Some(imports))
}
