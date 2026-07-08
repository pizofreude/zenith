//! The document-wide chain pre-pass entry (`resolve_chains_document`).

use std::collections::BTreeMap;

use zenith_core::{Diagnostic, FontProvider, ResolvedToken, Style, TextNode};
use zenith_layout::RustybuzzEngine;

use crate::compile::markdown_resolve::MdBlockMap;

use super::collect::collect_chains;
use super::distribute::{ChainDocStyles, distribute_chains};
use super::types::{ChainAssignments, Member};

/// Build the DOCUMENT-WIDE chain-assignment map across every page.
///
/// Chains thread across boxes on DIFFERENT pages: members are collected in
/// (page-order, then source-order) over `doc.body.pages`, each carrying its OWN
/// page's box geometry, and a chain's source content is poured greedily across
/// all members in that global order — box 1 fills, the remainder flows into
/// box 2, … across page boundaries. The returned map is keyed by global node id,
/// so `compile_page` for any page looks up the slice assigned to a box on that
/// page.
///
/// Returns an empty map when no `chain` members are present, in which case
/// `compile_text` behaves exactly as before for every node.
pub(in crate::compile) fn resolve_chains_document<'a>(
    doc: &'a zenith_core::Document,
    resolved: &BTreeMap<String, ResolvedToken>,
    style_map: &BTreeMap<&str, &Style>,
    fonts: &dyn FontProvider,
    engine: &RustybuzzEngine,
    md_blocks: &MdBlockMap,
    diagnostics: &mut Vec<Diagnostic>,
) -> ChainAssignments {
    // Collect members + content sources across ALL pages in page-then-source
    // order. A `BTreeMap` per-chain member list preserves the push order, which
    // is exactly the document-wide flow order.
    let mut members: BTreeMap<String, Vec<Member>> = BTreeMap::new();
    let mut source: BTreeMap<String, &'a TextNode> = BTreeMap::new();
    let mut source_page_styles: BTreeMap<String, &'a [zenith_core::BlockStyle]> = BTreeMap::new();
    for page in &doc.body.pages {
        collect_chains(
            &page.children,
            &page.block_styles,
            resolved,
            &mut members,
            &mut source,
            &mut source_page_styles,
        );
    }

    distribute_chains(
        &members,
        &source,
        &source_page_styles,
        ChainDocStyles {
            resolved,
            style_map,
            doc_block_styles: &doc.body.block_styles,
            md_blocks,
        },
        fonts,
        engine,
        diagnostics,
    )
}
