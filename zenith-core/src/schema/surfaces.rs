//! Non-node authorable surfaces: the `variants`/`override` and `diagnostics` blocks.

use crate::diag_catalog::{DIAGNOSTIC_CODES, DIAGNOSTIC_VERBS, DiagnosticCodeInfo};

// ── Variant / override surface ────────────────────────────────────────────────

/// Full schema descriptor for the `variants` / `override` surface.
pub struct VariantDescriptor {
    /// One-line summary of the surface.
    pub summary: &'static str,
    /// Description of the `variants { … }` block structure.
    pub block_structure: &'static str,
    /// Description of the `variant id=… source=… w=… h=… { … }` node.
    pub variant_node: &'static str,
    /// Description of the `override node="<id>" …` entry and its recognised keys.
    pub override_entry: &'static str,
    /// Recognised properties on an `override` entry, as `(name, type, required)` tuples.
    pub override_props: &'static [(&'static str, &'static str, bool)],
    /// A worked example of a `variants` block containing an override.
    pub example: &'static str,
}

/// Return the descriptor for the `variants` / `override` surface.
///
/// This surface is not a node kind (it is not renderable on its own), so it
/// does not appear in `node_kinds()` or `node_summary()`. It is discoverable
/// via `zenith schema variant`.
pub fn variant_descriptor() -> VariantDescriptor {
    VariantDescriptor {
        summary: "Variant system — named page-level derivatives with per-node property overrides.",
        block_structure: "A `variants { … }` block sits at the document root, as a sibling of \
            `document` (canonical order: after `provenance`, before `document`) — NOT inside a \
            page. It contains one or more `variant` entries, each with its own child block of \
            `override` entries that apply to that variant.",
        variant_node: "variant id=<id> source=<page-id> w=(px)N h=(px)N { … }\n\
            \n\
            • id         — unique identifier for this variant (string, required)\n\
            • source     — the page id to base this variant on (page id string, required)\n\
            • w          — override canvas width in pixels, e.g. (px)1920 (dimension, required)\n\
            • h          — override canvas height in pixels, e.g. (px)1080 (dimension, required)\n\
            \n\
            The child block of `variant { … }` contains `override` entries (see below).",
        override_entry: "override node=\"<id>\" …\n\
            \n\
            Targets the node whose id equals the `node=` value, and applies one or more \
            property overrides. The `node` key is the only required field; all visual/geometry \
            keys are optional and independent (omitted keys retain the source page value).\n\
            \n\
            IMPORTANT: the selector key is `node` (the target node's id string), NOT `id`.\n\
            Wrong:   override id=\"hero\" visible=#false\n\
            Correct: override node=\"hero\" visible=#false",
        override_props: &[
            ("node", "string — target node id selector (required)", true),
            ("visible", "#true or #false", false),
            ("text", "string — replacement text content", false),
            ("fill", "token ref or color string", false),
            ("x", "typed dimension, e.g. (px)100", false),
            ("y", "typed dimension, e.g. (px)50", false),
            ("w", "typed dimension, e.g. (px)800", false),
            ("h", "typed dimension, e.g. (px)600", false),
        ],
        example: concat!(
            "variants {\n",
            "  variant id=\"mobile\" source=\"page.main\" w=(px)390 h=(px)844 {\n",
            "    // hide the desktop-only sidebar\n",
            "    override node=\"sidebar\" visible=#false\n",
            "    // shrink the hero to fit the narrower canvas\n",
            "    override node=\"hero\" x=(px)0 y=(px)0 w=(px)390 h=(px)260\n",
            "    // swap the headline copy\n",
            "    override node=\"headline\" text=\"Mobile headline\"\n",
            "  }\n",
            "}",
        ),
    }
}

// ── Diagnostics surface ────────────────────────────────────────────────────────

/// One-line description of the `diagnostics` surface (the root `diagnostics { … }`
/// lint-policy block).
pub fn diagnostics_summary() -> &'static str {
    "In-file diagnostic policy — allow/deny/warn specific diagnostic codes \
     (integrity Errors cannot be suppressed)."
}

/// The policy verbs accepted inside a `diagnostics { … }` block, in canonical
/// order (`allow`, `deny`, `warn`).
///
/// Single source of truth: re-exposed from [`crate::diag_catalog`].
pub fn diagnostics_verbs() -> &'static [&'static str] {
    DIAGNOSTIC_VERBS
}

/// The full catalog of diagnostic codes the engine can emit, each with its
/// severity and a one-line summary.
///
/// Single source of truth: re-exposed from [`crate::diag_catalog`]. The same
/// table drives the diagnostic-policy validator in [`crate::validate()`], so the
/// `zenith schema diagnostics` surface and the policy checker can never diverge.
pub fn diagnostic_codes() -> &'static [DiagnosticCodeInfo] {
    DIAGNOSTIC_CODES
}
