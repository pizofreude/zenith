//! Token-type schema: canonical list, summaries, and full descriptors.

// ── Token type list ───────────────────────────────────────────────────────────

/// All authorable token types in their canonical `type=` string form.
///
/// `Unknown` is excluded: it is a forward-compat placeholder, not an authorable
/// type. The list is sorted for deterministic output.
///
/// Exhaustive correspondence is enforced by the `token_type_variant_count_exhaustive`
/// helper in the `#[cfg(test)]` drift-guard below: adding a new `TokenType` variant
/// without updating that match causes a compile error in the tests module.
pub fn token_types() -> &'static [&'static str] {
    &[
        "color",
        "dimension",
        "filter",
        "fontFamily",
        "fontWeight",
        "gradient",
        "mask",
        "number",
        "shadow",
    ]
}

// ── Token type summaries ──────────────────────────────────────────────────────

/// Return a one-line description of the named token type, or `None` if the type
/// is not recognised.
///
/// The `match` arm set here must stay exhaustive over `token_types()`. The
/// drift-guard test `token_type_summary_covers_every_token_type` enforces that.
pub fn token_type_summary(ty: &str) -> Option<&'static str> {
    match ty {
        "color" => Some("sRGB hex, alpha-hex, or CMYK color constant."),
        "dimension" => Some("Typed measurement with unit: px, pt, pct, or deg."),
        "filter" => Some("Ordered stack of image filter ops (grayscale, duotone, noise, …)."),
        "fontFamily" => Some("Named font-family string used for typography."),
        "fontWeight" => Some("Integer font weight in 100–900 (e.g. 400 = regular, 700 = bold)."),
        "gradient" => Some("Linear or radial gradient built from ≥2 color-stop child nodes."),
        "mask" => Some("Spatial coverage mask: a single rect, ellipse, or rounded-rect shape."),
        "number" => Some("Unitless finite number (e.g. opacity ratio, scale factor)."),
        "shadow" => Some("Ordered stack of drop-shadow layers, each referencing a color token."),
        _ => None,
    }
}

// ── Token type descriptors ────────────────────────────────────────────────────

/// Full schema descriptor for one authorable token type.
///
/// Returned by [`token_type_descriptor`].
pub struct TokenTypeDescriptor {
    /// Canonical `type=` string (matches the entry in [`token_types()`]).
    pub type_name: &'static str,
    /// One-line summary (same text as [`token_type_summary`]).
    pub summary: &'static str,
    /// Human-readable description of the value form. Empty for types that carry
    /// no inline value (gradient, shadow, filter, mask — those use child nodes).
    pub value_form: &'static str,
    /// Human-readable description of the expected child nodes. Empty for scalar
    /// types (color, dimension, number, fontFamily, fontWeight).
    pub child_nodes: &'static str,
    /// A minimal, syntactically correct example embedded as a standalone token
    /// node (without the surrounding `tokens { }` block wrapper).
    pub example: &'static str,
}

/// Return the full descriptor for the named token type, or `None` if the type
/// is not recognised.
///
/// The `match` arm set here must stay exhaustive over `token_types()`. The
/// drift-guard tests enforce that and also parse every `example` string.
pub fn token_type_descriptor(ty: &str) -> Option<TokenTypeDescriptor> {
    match ty {
        "color" => Some(TokenTypeDescriptor {
            type_name: "color",
            summary: token_type_summary("color").unwrap_or(""),
            value_form: r##"String literal: "#rrggbb" (6-digit lowercase hex), "#rrggbbaa" (8-digit), or "cmyk(c,m,y,k)" with each channel 0–100."##,
            child_nodes: "",
            example: r##"token id="color.brand.primary" type="color" value="#1a73e8""##,
        }),
        "dimension" => Some(TokenTypeDescriptor {
            type_name: "dimension",
            summary: token_type_summary("dimension").unwrap_or(""),
            value_form: "Dimension literal: (px)N, (pt)N, (pct)N, or (deg)N — annotation then bare number, no space. E.g. (px)16, (pt)12, (pct)100, (deg)45.",
            child_nodes: "",
            example: r#"token id="dim.radius.card" type="dimension" value=(px)8"#,
        }),
        "filter" => Some(TokenTypeDescriptor {
            type_name: "filter",
            summary: token_type_summary("filter").unwrap_or(""),
            value_form: "No inline value. Defined entirely by op child nodes.",
            child_nodes: "≥1 op child node. Valid op names: grayscale, invert, sepia, saturate, brightness, contrast, hue-rotate (each accept optional amount=N); duotone (requires shadow=(token)\"id\" highlight=(token)\"id\", optional amount=N); noise (accepts seed=N scale=N, optional amount=N).",
            example: "token id=\"filter.mono\" type=\"filter\" {\n    grayscale amount=1.0\n}",
        }),
        "fontFamily" => Some(TokenTypeDescriptor {
            type_name: "fontFamily",
            summary: token_type_summary("fontFamily").unwrap_or(""),
            value_form: r#"Non-empty string literal: the font-family name as it appears in the asset block, e.g. "Inter" or "Source Serif 4"."#,
            child_nodes: "",
            example: r#"token id="font.body" type="fontFamily" value="Inter""#,
        }),
        "fontWeight" => Some(TokenTypeDescriptor {
            type_name: "fontWeight",
            summary: token_type_summary("fontWeight").unwrap_or(""),
            value_form: "Bare integer (NOT a string, NOT a dimension): an integer in 100–900 with no unit annotation. E.g. 400, 700.",
            child_nodes: "",
            example: r#"token id="weight.bold" type="fontWeight" value=700"#,
        }),
        "gradient" => Some(TokenTypeDescriptor {
            type_name: "gradient",
            summary: token_type_summary("gradient").unwrap_or(""),
            value_form: "No inline value. Defined entirely by stop child nodes plus optional angle/radial props on the token node itself.",
            child_nodes: "≥2 stop child nodes. Each stop: stop offset=0.0 color=(token)\"color-token-id\". Optional props on the token node: angle=(deg)N (linear, default 90), radial=#true, center-x=0.5 center-y=0.5 radius=1.0.",
            example: "token id=\"gradient.brand\" type=\"gradient\" angle=(deg)90 {\n    stop offset=0.0 color=(token)\"color.brand.primary\"\n    stop offset=1.0 color=(token)\"color.brand.secondary\"\n}",
        }),
        "mask" => Some(TokenTypeDescriptor {
            type_name: "mask",
            summary: token_type_summary("mask").unwrap_or(""),
            value_form: "No inline value. Defined by exactly one shape child node.",
            child_nodes: "Exactly 1 shape child: rect, ellipse, or rounded. Each accepts feather=N (Gaussian sigma px, default 0) and invert=#true/#false. rounded also accepts radius=N (corner radius px).",
            example: "token id=\"mask.card\" type=\"mask\" {\n    rounded radius=8 feather=2\n}",
        }),
        "number" => Some(TokenTypeDescriptor {
            type_name: "number",
            summary: token_type_summary("number").unwrap_or(""),
            value_form: "Bare finite number with no unit annotation. E.g. 1.0, 0.5, 1.05. NaN and ±inf are invalid.",
            child_nodes: "",
            example: r#"token id="number.line-height" type="number" value=1.4"#,
        }),
        "shadow" => Some(TokenTypeDescriptor {
            type_name: "shadow",
            summary: token_type_summary("shadow").unwrap_or(""),
            value_form: "No inline value. Defined entirely by layer child nodes.",
            child_nodes: "≥1 layer child node. Each layer: layer color=(token)\"color-token-id\" dx=(px)N dy=(px)N blur=(px)N. dx/dy can be negative (offsets); blur is clamped to ≥0.",
            example: "token id=\"shadow.card\" type=\"shadow\" {\n    layer color=(token)\"color.shadow\" dx=(px)0 dy=(px)2 blur=(px)8\n}",
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::token::TokenType;
    use crate::parse::KdlSource;
    use crate::parse::kdl_adapter::KdlAdapter;

    /// Exhaustive match over every `TokenType` variant: the compile-time drift guard.
    ///
    /// When a new variant `TokenType::Foo` is added:
    /// 1. The `match` here becomes non-exhaustive → **compile error**.
    /// 2. Developer adds a `TokenType::Foo => 1` arm here.
    /// 3. The developer also updates `TOTAL_TOKEN_TYPE_VARIANTS`.
    /// 4. The `assert_eq` in `token_type_summary_covers_every_token_type` then
    ///    fails, prompting the developer to add `"foo"` to `token_types()`,
    ///    `token_type_summary()`, and `token_type_descriptor()`.
    ///
    /// This function is only ever referenced via a function pointer in the test
    /// body (never actually called); the pointer reference forces the compiler to
    /// type-check the exhaustive match.
    fn token_type_variant_count_exhaustive(ty: &TokenType) -> usize {
        match ty {
            TokenType::Color => 1,
            TokenType::Dimension => 1,
            TokenType::Number => 1,
            TokenType::FontFamily => 1,
            TokenType::FontWeight => 1,
            TokenType::Gradient => 1,
            TokenType::Shadow => 1,
            TokenType::Filter => 1,
            TokenType::Mask => 1,
            // Unknown is intentionally excluded from the authorable type list.
            // This arm is required for exhaustiveness.
            TokenType::Unknown(_) => 1,
        }
    }

    /// Total number of `TokenType` variants as recorded in the exhaustive match above.
    /// Updated by hand when a variant is added (compile error forces it).
    const TOTAL_TOKEN_TYPE_VARIANTS: usize = 10; // 9 authorable + 1 Unknown

    #[test]
    fn token_type_summary_covers_every_token_type() {
        // Cross-check: token_types() must have exactly TOTAL_TOKEN_TYPE_VARIANTS − 1
        // entries (all variants except Unknown).
        let expected_authorable = TOTAL_TOKEN_TYPE_VARIANTS - 1;
        assert_eq!(
            token_types().len(),
            expected_authorable,
            "token_types() has {} entries but the exhaustive TokenType match covers {} authorable \
             variants (plus Unknown). Update token_types(), token_type_summary(), and \
             token_type_descriptor() when adding a variant.",
            token_types().len(),
            expected_authorable,
        );

        // Suppress the "never used" lint on token_type_variant_count_exhaustive by
        // taking a function pointer — this forces the compiler to type-check the
        // fn's exhaustive match without calling it.
        let _guard: fn(&TokenType) -> usize = token_type_variant_count_exhaustive;

        // Every listed type must have a summary.
        for ty in token_types() {
            assert!(
                token_type_summary(ty).is_some(),
                "token_type_summary(\"{ty}\") returned None — add a one-liner to token_type_summary()",
            );
        }
    }

    #[test]
    fn token_type_descriptor_covers_every_token_type() {
        // Every listed type must have a descriptor.
        for ty in token_types() {
            assert!(
                token_type_descriptor(ty).is_some(),
                "token_type_descriptor(\"{ty}\") returned None — add a descriptor to token_type_descriptor()",
            );
        }

        // Every descriptor's type_name must match its key.
        for ty in token_types() {
            let desc = token_type_descriptor(ty).unwrap();
            assert_eq!(
                desc.type_name, *ty,
                "token_type_descriptor(\"{ty}\").type_name is \"{}\", expected \"{ty}\"",
                desc.type_name,
            );
            // summary must be non-empty.
            assert!(
                !desc.summary.is_empty(),
                "token_type_descriptor(\"{ty}\").summary is empty",
            );
            // value_form and child_nodes may not both be empty (every type has one or the other).
            assert!(
                !desc.value_form.is_empty() || !desc.child_nodes.is_empty(),
                "token_type_descriptor(\"{ty}\") has both empty value_form and child_nodes",
            );
            // example must be non-empty.
            assert!(
                !desc.example.is_empty(),
                "token_type_descriptor(\"{ty}\").example is empty",
            );
        }
    }

    /// Example-accuracy guard: each token example must parse as part of a minimal
    /// document without a parse error.
    ///
    /// We do NOT assert that validation is clean — compound examples reference
    /// other token ids that won't resolve standalone, and that is expected. We
    /// only assert syntax correctness: if this fails, the schema is showing agents
    /// syntactically wrong examples.
    #[test]
    fn token_type_examples_parse_without_syntax_errors() {
        for ty in token_types() {
            let desc = token_type_descriptor(ty).unwrap();
            // Wrap the token example in a minimal document.
            // Only `document` is required by the parser; `tokens` is optional
            // but must carry `format="zenith-token-v1"` when present.
            let doc_src = format!(
                "zenith version=1 {{\n\
                 \x20 tokens format=\"zenith-token-v1\" {{\n\
                 \x20   {}\n\
                 \x20 }}\n\
                 \x20 document id=\"doc\" {{\n\
                 \x20   page id=\"pg\" w=(px)1 h=(px)1 {{}}\n\
                 \x20 }}\n\
                 }}\n",
                desc.example,
            );
            let result = KdlAdapter.parse(doc_src.as_bytes());
            assert!(
                result.is_ok(),
                "token_type_descriptor(\"{ty}\").example failed to parse:\n\
                 example:\n  {}\n\
                 wrapped doc:\n{doc_src}\n\
                 parse error: {:?}",
                desc.example,
                result.err(),
            );
        }
    }
}
