//! Scene compilation: `Document` → `CompileResult`.
//!
//! Entry point: [`compile`].
//!
//! Only `rect` nodes and the page are compiled in Unit 6; `text` and unknown
//! nodes produce an advisory diagnostic and are skipped.

use std::collections::BTreeMap;

use zenith_core::{
    Diagnostic, Document, Node, PropertyValue, ResolvedToken, ResolvedValue, Span, Unit,
    resolve_tokens,
};

use crate::color::parse_srgb_hex;
use crate::ir::{Color, Scene, SceneCommand};

// ── Public result type ────────────────────────────────────────────────────────

/// The result of compiling a [`Document`] into a [`Scene`].
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// The compiled display list.
    pub scene: Scene,
    /// All diagnostics collected during compilation (may include token-resolution
    /// diagnostics, unit advisories, and unsupported-node advisories).
    pub diagnostics: Vec<Diagnostic>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Compile `doc` into a [`CompileResult`].
///
/// Only the first page is compiled.  If the document has no pages an empty
/// scene is returned with an advisory diagnostic.
///
/// # No-panic guarantee
///
/// This function never calls `unwrap`, `expect`, `panic!`, `todo!`,
/// `unimplemented!`, or performs unchecked indexing.  All failure paths push a
/// diagnostic and continue.
pub fn compile(doc: &Document) -> CompileResult {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // ── Step 1: resolve tokens ────────────────────────────────────────────
    let token_resolution = resolve_tokens(&doc.tokens);
    diagnostics.extend(token_resolution.diagnostics);
    let resolved = &token_resolution.resolved;

    // ── Step 2: select the first page ────────────────────────────────────
    let Some(page) = doc.body.pages.first() else {
        diagnostics.push(Diagnostic::advisory(
            "scene.no_pages",
            "document has no pages; an empty scene is returned",
            None,
            Some(doc.body.id.clone()),
        ));
        return CompileResult {
            scene: Scene::new(0.0, 0.0),
            diagnostics,
        };
    };

    // ── Step 3: page dimensions → pixels ─────────────────────────────────
    let page_w = match dim_to_px(page.width.value, &page.width.unit) {
        Some(v) => v,
        None => {
            diagnostics.push(Diagnostic::advisory(
                "scene.unsupported_unit",
                format!(
                    "page '{}' width uses an unsupported unit; cannot compile scene",
                    page.id
                ),
                page.source_span,
                Some(page.id.clone()),
            ));
            return CompileResult {
                scene: Scene::new(0.0, 0.0),
                diagnostics,
            };
        }
    };
    let page_h = match dim_to_px(page.height.value, &page.height.unit) {
        Some(v) => v,
        None => {
            diagnostics.push(Diagnostic::advisory(
                "scene.unsupported_unit",
                format!(
                    "page '{}' height uses an unsupported unit; cannot compile scene",
                    page.id
                ),
                page.source_span,
                Some(page.id.clone()),
            ));
            return CompileResult {
                scene: Scene::new(0.0, 0.0),
                diagnostics,
            };
        }
    };

    let mut scene = Scene::new(page_w, page_h);

    // ── Step 4: outermost page-edge clip (doc 09 normative rule) ─────────
    scene.commands.push(SceneCommand::PushClip {
        x: 0.0,
        y: 0.0,
        w: page_w,
        h: page_h,
    });

    // ── Step 5: optional page background ─────────────────────────────────
    if let Some(bg_prop) = &page.background
        && let Some(color) = resolve_property_color(bg_prop, resolved, &mut diagnostics, &page.id)
    {
        scene.commands.push(SceneCommand::FillRect {
            x: 0.0,
            y: 0.0,
            w: page_w,
            h: page_h,
            color,
        });
    }

    // ── Step 6: children in source order (z-order: first = bottom) ───────
    for node in &page.children {
        compile_node(node, resolved, &mut scene.commands, &mut diagnostics);
    }

    // ── Step 7: close the outermost clip ─────────────────────────────────
    scene.commands.push(SceneCommand::PopClip);

    CompileResult { scene, diagnostics }
}

// ── Node dispatch ─────────────────────────────────────────────────────────────

fn compile_node(
    node: &Node,
    resolved: &BTreeMap<String, ResolvedToken>,
    commands: &mut Vec<SceneCommand>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match node {
        Node::Rect(rect) => {
            // Skip invisible rects.
            if rect.visible == Some(false) {
                return;
            }

            // Resolve geometry — all four are required; skip if any is absent
            // or uses an unsupported unit.
            let (Some(x_dim), Some(y_dim), Some(w_dim), Some(h_dim)) =
                (&rect.x, &rect.y, &rect.w, &rect.h)
            else {
                diagnostics.push(Diagnostic::advisory(
                    "scene.missing_geometry",
                    format!(
                        "rect '{}' is missing one or more geometry properties (x, y, w, h); \
                         skipped",
                        rect.id
                    ),
                    rect.source_span,
                    Some(rect.id.clone()),
                ));
                return;
            };

            let Some(x) = dim_to_px(x_dim.value, &x_dim.unit) else {
                diagnostics.push(unsupported_unit_diag(&rect.id, "x", rect.source_span));
                return;
            };
            let Some(y) = dim_to_px(y_dim.value, &y_dim.unit) else {
                diagnostics.push(unsupported_unit_diag(&rect.id, "y", rect.source_span));
                return;
            };
            let Some(w) = dim_to_px(w_dim.value, &w_dim.unit) else {
                diagnostics.push(unsupported_unit_diag(&rect.id, "w", rect.source_span));
                return;
            };
            let Some(h) = dim_to_px(h_dim.value, &h_dim.unit) else {
                diagnostics.push(unsupported_unit_diag(&rect.id, "h", rect.source_span));
                return;
            };

            // Resolve fill color.
            let Some(fill_prop) = &rect.fill else {
                // No fill → nothing to draw for a fill-only skeleton.
                return;
            };
            let Some(mut color) =
                resolve_property_color(fill_prop, resolved, diagnostics, &rect.id)
            else {
                return;
            };

            // Apply opacity.
            if let Some(opacity) = rect.opacity {
                let o = opacity.clamp(0.0, 1.0);
                color.a = (color.a as f64 * o).round() as u8;
            }

            commands.push(SceneCommand::FillRect { x, y, w, h, color });
        }

        Node::Text(text) => {
            diagnostics.push(Diagnostic::advisory(
                "scene.unsupported_node",
                format!(
                    "text node '{}' cannot be compiled in this version (text compile is deferred)",
                    text.id
                ),
                text.source_span,
                Some(text.id.clone()),
            ));
        }

        Node::Unknown(unknown) => {
            diagnostics.push(Diagnostic::advisory(
                "scene.unsupported_node",
                format!(
                    "unknown node kind '{}' cannot be compiled; the node is skipped \
                     (forward-compatibility: this kind may be supported in a later version)",
                    unknown.kind
                ),
                unknown.source_span,
                None,
            ));
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Convert a dimension value + unit to pixels.
///
/// Returns `None` for unsupported / unknown units (caller pushes advisory).
fn dim_to_px(value: f64, unit: &Unit) -> Option<f64> {
    match unit {
        Unit::Px => Some(value),
        Unit::Pt => Some(value * 96.0 / 72.0),
        Unit::Pct | Unit::Deg | Unit::Unknown(_) => None,
    }
}

/// Build an `scene.unsupported_unit` advisory for a named geometry field.
fn unsupported_unit_diag(node_id: &str, field: &str, span: Option<Span>) -> Diagnostic {
    Diagnostic::advisory(
        "scene.unsupported_unit",
        format!(
            "rect '{}' field '{}' uses an unsupported unit; the rect is skipped",
            node_id, field
        ),
        span,
        Some(node_id.to_owned()),
    )
}

/// Resolve a `PropertyValue` to a `Color`, or push a diagnostic and return
/// `None`.
///
/// Accepts:
/// - `TokenRef(id)` → looks up in `resolved`, must be a `ResolvedValue::Color`.
/// - `Literal(hex)` → parses as sRGB hex string directly.
fn resolve_property_color(
    prop: &PropertyValue,
    resolved: &BTreeMap<String, ResolvedToken>,
    diagnostics: &mut Vec<Diagnostic>,
    subject_id: &str,
) -> Option<Color> {
    match prop {
        PropertyValue::TokenRef(token_id) => {
            match resolved.get(token_id.as_str()) {
                Some(rt) => match &rt.value {
                    ResolvedValue::Color(hex) => match parse_srgb_hex(hex) {
                        Some(c) => Some(c),
                        None => {
                            // Should not happen — token resolution validates hex —
                            // but be robust.
                            diagnostics.push(Diagnostic::advisory(
                                "scene.invalid_color",
                                format!(
                                    "token '{}' resolved to '{}' which is not a valid \
                                     sRGB hex color; skipped",
                                    token_id, hex
                                ),
                                None,
                                Some(subject_id.to_owned()),
                            ));
                            None
                        }
                    },
                    other => {
                        diagnostics.push(Diagnostic::advisory(
                            "scene.wrong_token_type",
                            format!(
                                "node '{}' references token '{}' which resolved to a \
                                 non-color value ({:?}); skipped",
                                subject_id, token_id, other
                            ),
                            None,
                            Some(subject_id.to_owned()),
                        ));
                        None
                    }
                },
                None => {
                    diagnostics.push(Diagnostic::advisory(
                        "scene.unresolved_token",
                        format!(
                            "node '{}' references token '{}' which did not resolve \
                             (check token diagnostics); skipped",
                            subject_id, token_id
                        ),
                        None,
                        Some(subject_id.to_owned()),
                    ));
                    None
                }
            }
        }
        PropertyValue::Literal(hex) => match parse_srgb_hex(hex) {
            Some(c) => Some(c),
            None => {
                diagnostics.push(Diagnostic::advisory(
                    "scene.invalid_color",
                    format!(
                        "node '{}' has a fill literal '{}' that is not a valid \
                         sRGB hex color; skipped",
                        subject_id, hex
                    ),
                    None,
                    Some(subject_id.to_owned()),
                ));
                None
            }
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use zenith_core::{KdlAdapter, KdlSource};

    // ── Helper to parse a .zen source string ──────────────────────────────

    fn parse(src: &str) -> Document {
        KdlAdapter
            .parse(src.as_bytes())
            .expect("test document must parse")
    }

    // ── Minimal single-rect document ──────────────────────────────────────

    /// A page with a single full-page rect filled via a token color.
    /// Expected scene: PushClip → FillRect (bg from token) → FillRect (rect) → PopClip.
    /// In this test the page has no background, so background FillRect is absent.
    #[test]
    fn single_rect_token_fill_compiles_correctly() {
        let src = r##"zenith version=1 {
  project id="proj.t1" name="T1"
  tokens format="zenith-token-v1" {
    token id="color.fill" type="color" value="#f8fafc"
  }
  styles {}
  document id="doc.t1" title="T1" {
    page id="page.t1" w=(px)640 h=(px)360 {
      rect id="rect.t1" x=(px)0 y=(px)0 w=(px)640 h=(px)360 fill=(token)"color.fill"
    }
  }
}
"##;
        let doc = parse(src);
        let result = compile(&doc);

        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );

        let cmds = &result.scene.commands;
        // PushClip, FillRect, PopClip
        assert_eq!(cmds.len(), 3, "expected 3 commands, got: {:?}", cmds);

        assert!(
            matches!(cmds[0], SceneCommand::PushClip { x, y, w, h } if x == 0.0 && y == 0.0 && w == 640.0 && h == 360.0),
            "first command must be PushClip covering the page"
        );

        match &cmds[1] {
            SceneCommand::FillRect { x, y, w, h, color } => {
                assert_eq!(*x, 0.0);
                assert_eq!(*y, 0.0);
                assert_eq!(*w, 640.0);
                assert_eq!(*h, 360.0);
                // #f8fafc → r=0xf8=248, g=0xfa=250, b=0xfc=252, a=255
                assert_eq!(color.r, 0xf8);
                assert_eq!(color.g, 0xfa);
                assert_eq!(color.b, 0xfc);
                assert_eq!(color.a, 255);
            }
            other => panic!("expected FillRect, got {other:?}"),
        }

        assert!(
            matches!(cmds[2], SceneCommand::PopClip),
            "last command must be PopClip"
        );
    }

    // ── Two rects → two FillRects in source order ─────────────────────────

    #[test]
    fn two_rects_emitted_in_source_order() {
        let src = r##"zenith version=1 {
  project id="proj.t2" name="T2"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#111111"
    token id="color.b" type="color" value="#222222"
  }
  styles {}
  document id="doc.t2" title="T2" {
    page id="page.t2" w=(px)100 h=(px)100 {
      rect id="rect.a" x=(px)0 y=(px)0 w=(px)50 h=(px)50 fill=(token)"color.a"
      rect id="rect.b" x=(px)50 y=(px)50 w=(px)50 h=(px)50 fill=(token)"color.b"
    }
  }
}
"##;
        let doc = parse(src);
        let result = compile(&doc);

        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );

        let cmds = &result.scene.commands;
        // PushClip, FillRect(a), FillRect(b), PopClip
        assert_eq!(cmds.len(), 4, "expected 4 commands, got: {:?}", cmds);

        match &cmds[1] {
            SceneCommand::FillRect { color, .. } => assert_eq!(color.r, 0x11),
            other => panic!("expected FillRect for rect.a, got {other:?}"),
        }
        match &cmds[2] {
            SceneCommand::FillRect { color, .. } => assert_eq!(color.r, 0x22),
            other => panic!("expected FillRect for rect.b, got {other:?}"),
        }
    }

    // ── visible=false rect is not emitted ─────────────────────────────────

    #[test]
    fn invisible_rect_not_emitted() {
        let src = r##"zenith version=1 {
  project id="proj.t3" name="T3"
  tokens format="zenith-token-v1" {
    token id="color.fill" type="color" value="#abcdef"
  }
  styles {}
  document id="doc.t3" title="T3" {
    page id="page.t3" w=(px)100 h=(px)100 {
      rect id="rect.hidden" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"color.fill" visible=#false
    }
  }
}
"##;
        let doc = parse(src);
        let result = compile(&doc);

        // No diagnostics expected (visible=false is a normal skip, not an error).
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );

        let cmds = &result.scene.commands;
        // Only PushClip + PopClip; no FillRect.
        assert_eq!(
            cmds.len(),
            2,
            "expected PushClip + PopClip only; got: {:?}",
            cmds
        );
        assert!(matches!(cmds[0], SceneCommand::PushClip { .. }));
        assert!(matches!(cmds[1], SceneCommand::PopClip));
    }

    // ── text node → advisory, no draw command ────────────────────────────

    #[test]
    fn text_node_produces_advisory_not_draw_command() {
        let src = r##"zenith version=1 {
  project id="proj.t4" name="T4"
  tokens format="zenith-token-v1" {
    token id="color.text" type="color" value="#111827"
  }
  styles {}
  document id="doc.t4" title="T4" {
    page id="page.t4" w=(px)200 h=(px)100 {
      text id="label.t4" x=(px)0 y=(px)0 w=(px)200 h=(px)50 fill=(token)"color.text" {
        span "Hello"
      }
    }
  }
}
"##;
        let doc = parse(src);
        let result = compile(&doc);

        // Must have exactly one advisory with code "scene.unsupported_node".
        let unsupported: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "scene.unsupported_node")
            .collect();
        assert_eq!(
            unsupported.len(),
            1,
            "expected 1 unsupported_node advisory; got: {:?}",
            result.diagnostics
        );

        // No FillRect or DrawGlyphRun — only PushClip + PopClip.
        let draw_cmds: Vec<_> = result
            .scene
            .commands
            .iter()
            .filter(|c| !matches!(c, SceneCommand::PushClip { .. } | SceneCommand::PopClip))
            .collect();
        assert!(
            draw_cmds.is_empty(),
            "no draw commands expected; got: {:?}",
            draw_cmds
        );
    }

    // ── JSON schema field is "zenith-scene-v1" ────────────────────────────

    #[test]
    fn json_schema_field_value() {
        let src = r##"zenith version=1 {
  project id="proj.t5" name="T5"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.t5" title="T5" {
    page id="page.t5" w=(px)100 h=(px)100 {}
  }
}
"##;
        let doc = parse(src);
        let result = compile(&doc);
        let json = result.scene.to_json().expect("serialize must succeed");
        assert!(
            json.contains(r#""schema": "zenith-scene-v1""#),
            "JSON must contain schema field; got snippet: {}",
            &json[..json.len().min(200)]
        );
    }

    // ── JSON determinism ──────────────────────────────────────────────────

    #[test]
    fn json_serialization_is_deterministic() {
        let src = r##"zenith version=1 {
  project id="proj.t6" name="T6"
  tokens format="zenith-token-v1" {
    token id="color.fill" type="color" value="#aabbcc"
  }
  styles {}
  document id="doc.t6" title="T6" {
    page id="page.t6" w=(px)200 h=(px)100 {
      rect id="rect.t6" x=(px)10 y=(px)20 w=(px)100 h=(px)50 fill=(token)"color.fill"
    }
  }
}
"##;
        let doc = parse(src);
        let r1 = compile(&doc);
        let r2 = compile(&doc);
        let j1 = r1.scene.to_json().expect("serialize 1");
        let j2 = r2.scene.to_json().expect("serialize 2");
        assert_eq!(
            j1, j2,
            "two compiles of the same doc must produce identical JSON"
        );
    }

    // ── Page background emitted as first FillRect ─────────────────────────

    #[test]
    fn page_background_emitted_before_children() {
        let src = r##"zenith version=1 {
  project id="proj.t7" name="T7"
  tokens format="zenith-token-v1" {
    token id="color.bg" type="color" value="#ffffff"
    token id="color.fill" type="color" value="#000000"
  }
  styles {}
  document id="doc.t7" title="T7" {
    page id="page.t7" w=(px)100 h=(px)100 background=(token)"color.bg" {
      rect id="rect.t7" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"color.fill"
    }
  }
}
"##;
        let doc = parse(src);
        let result = compile(&doc);

        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );

        let cmds = &result.scene.commands;
        // PushClip, FillRect(bg=white), FillRect(rect=black), PopClip
        assert_eq!(cmds.len(), 4, "expected 4 commands; got: {:?}", cmds);

        // Background fill must be white.
        match &cmds[1] {
            SceneCommand::FillRect { color, .. } => {
                assert_eq!(color.r, 255, "bg must be white");
                assert_eq!(color.g, 255);
                assert_eq!(color.b, 255);
            }
            other => panic!("expected background FillRect, got {other:?}"),
        }

        // Child rect must be black.
        match &cmds[2] {
            SceneCommand::FillRect { color, .. } => {
                assert_eq!(color.r, 0, "child rect must be black");
                assert_eq!(color.g, 0);
                assert_eq!(color.b, 0);
            }
            other => panic!("expected child FillRect, got {other:?}"),
        }
    }

    // ── Opacity multiplied into alpha ─────────────────────────────────────

    #[test]
    fn opacity_applied_to_fill_alpha() {
        // A full-alpha color (#ffffff, a=255) with opacity=0.5 → a≈128.
        let src = r##"zenith version=1 {
  project id="proj.t8" name="T8"
  tokens format="zenith-token-v1" {
    token id="color.fill" type="color" value="#ffffff"
  }
  styles {}
  document id="doc.t8" title="T8" {
    page id="page.t8" w=(px)100 h=(px)100 {
      rect id="rect.t8" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"color.fill" opacity=0.5
    }
  }
}
"##;
        let doc = parse(src);
        let result = compile(&doc);
        assert!(
            result.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            result.diagnostics
        );

        match &result.scene.commands[1] {
            SceneCommand::FillRect { color, .. } => {
                // 255 * 0.5 = 127.5 → rounds to 128.
                assert_eq!(color.a, 128, "opacity 0.5 must give a=128; got {}", color.a);
            }
            other => panic!("expected FillRect, got {other:?}"),
        }
    }
}
