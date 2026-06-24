//! Compile-stage `font.local` advisory wiring.
//!
//! These tests are fully deterministic and do NOT depend on the host's installed
//! fonts: they build a `BytesFontProvider` directly, registering a face with
//! `FontSource::Local` (using bundled Noto bytes under a custom family name). A
//! text/code node that resolves to that face must trip the `font.local`
//! advisory; one that resolves to a bundled face must NOT.

use std::sync::Arc;

use zenith_core::font::embedded;
use zenith_core::{
    BytesFontProvider, Diagnostic, FontSource, FontStyle, KdlAdapter, KdlSource, default_provider,
};
use zenith_scene::compile;

/// A provider with the bundled defaults PLUS a single locally-sourced face
/// registered under the family `"Local Test Family"` (real Noto bytes so shaping
/// succeeds, but flagged `FontSource::Local`).
fn provider_with_local_family() -> BytesFontProvider {
    let mut p = default_provider();
    let bytes: Arc<[u8]> = Arc::from(embedded::NOTO_SANS_REGULAR);
    p.register(
        "Local Test Family",
        400,
        FontStyle::Normal,
        bytes,
        0,
        FontSource::Local,
    );
    p
}

fn parse(src: &str) -> zenith_core::Document {
    KdlAdapter
        .parse(src.as_bytes())
        .expect("test document must parse")
}

fn has_code(diags: &[Diagnostic], code: &str) -> bool {
    diags.iter().any(|d| d.code == code)
}

fn text_doc(family: &str) -> String {
    format!(
        r##"zenith version=1 {{
  project id="proj.fl" name="FL"
  tokens format="zenith-token-v1" {{}}
  document id="doc.fl" title="FL" {{
    page id="p1" w=(px)400 h=(px)300 {{
      text id="t1" x=(px)10 y=(px)10 w=(px)300 font-family="{family}" {{
        span "Hello local font"
      }}
    }}
  }}
}}"##
    )
}

#[test]
fn local_source_family_emits_font_local_advisory() {
    let doc = parse(&text_doc("Local Test Family"));
    let result = compile(&doc, &provider_with_local_family());
    assert!(
        has_code(&result.diagnostics, "font.local"),
        "a text node resolving to a Local-source face must emit font.local; got: {:?}",
        result
            .diagnostics
            .iter()
            .map(|d| &d.code)
            .collect::<Vec<_>>()
    );
    // It resolved cleanly (no fallback), so font.unresolved must NOT fire.
    assert!(
        !has_code(&result.diagnostics, "font.unresolved"),
        "a cleanly-resolved local family must not emit font.unresolved"
    );
}

#[test]
fn bundled_family_does_not_emit_font_local() {
    // A bundled family ("Noto Sans") must never trip the local advisory.
    let doc = parse(&text_doc("Noto Sans"));
    let result = compile(&doc, &provider_with_local_family());
    assert!(
        !has_code(&result.diagnostics, "font.local"),
        "a bundled family must not emit font.local"
    );
    assert!(
        !has_code(&result.diagnostics, "font.unresolved"),
        "a bundled family must not emit font.unresolved"
    );
}

#[test]
fn unknown_family_emits_unresolved_not_local() {
    // A family in neither bundled/project nor local falls back to the bundled
    // default → font.unresolved, and never font.local.
    let doc = parse(&text_doc("Totally Unknown Family"));
    let result = compile(&doc, &provider_with_local_family());
    assert!(
        has_code(&result.diagnostics, "font.unresolved"),
        "an unknown family must emit font.unresolved"
    );
    assert!(
        !has_code(&result.diagnostics, "font.local"),
        "falling back to the bundled default must not emit font.local"
    );
}
