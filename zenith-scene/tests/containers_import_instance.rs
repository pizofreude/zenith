mod common;

use common::{Paint, SceneCommand, default_provider, parse};
use zenith_scene::{ImportGraph, compile_page, compile_page_with_imports};

type FillRectSummary = (f64, f64, f64, f64, (u8, u8, u8));

fn imported_doc() -> common::Document {
    parse(
        r##"zenith version=1 {
  project id="proj.imported" name="Imported"
  tokens format="zenith-token-v1" {
    token id="color.brand" type="color" value="#0000ff"
    token id="color.alt" type="color" value="#00ff00"
  }
  styles {}
  components {
    component id="component.card" {
      rect id="bg" x=(px)0 y=(px)0 w=(px)40 h=(px)20 fill=(token)"color.brand"
    }
    component id="component.alt" {
      rect id="bg" x=(px)0 y=(px)0 w=(px)30 h=(px)10 fill=(token)"color.alt"
    }
  }
  document id="doc.imported" title="Imported" {
    page id="page.imported" w=(px)10 h=(px)10 {}
  }
}
"##,
    )
}

fn host_doc(source: &str) -> common::Document {
    host_doc_with_instance_body(&format!(
        r#"instance id="inst.imported" source="{source}" x=(px)5 y=(px)7"#
    ))
}

fn host_doc_with_instance_body(instance_body: &str) -> common::Document {
    let src = format!(
        r##"zenith version=1 {{
  project id="proj.host" name="Host"
  tokens format="zenith-token-v1" {{
    token id="color.brand" type="color" value="#ff0000"
    token id="color.override" type="color" value="#ffff00"
  }}
  styles {{}}
  components {{}}
  document id="doc.host" title="Host" {{
    page id="page.host" w=(px)100 h=(px)80 {{
      {instance_body}
    }}
  }}
}}
"##
    );
    parse(&src)
}

fn fill_rects(result: &zenith_scene::CompileResult) -> Vec<FillRectSummary> {
    result
        .scene
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::FillRect {
                x,
                y,
                w,
                h,
                paint: Paint::Solid { color },
            } => Some((*x, *y, *w, *h, (color.r, color.g, color.b))),
            SceneCommand::FillRect {
                paint: Paint::Gradient(_),
                ..
            }
            | SceneCommand::StrokeRect { .. }
            | SceneCommand::FillRoundedRect { .. }
            | SceneCommand::StrokeRoundedRect { .. }
            | SceneCommand::FillEllipse { .. }
            | SceneCommand::StrokeEllipse { .. }
            | SceneCommand::StrokeLine { .. }
            | SceneCommand::FillPolygon { .. }
            | SceneCommand::StrokePolyline { .. }
            | SceneCommand::FillPath { .. }
            | SceneCommand::StrokePath { .. }
            | SceneCommand::DrawImage { .. }
            | SceneCommand::DrawSvgAsset { .. }
            | SceneCommand::DrawGlyphRun { .. }
            | SceneCommand::PushClip { .. }
            | SceneCommand::PopClip
            | SceneCommand::PushLayer { .. }
            | SceneCommand::PopLayer
            | SceneCommand::PushTransform { .. }
            | SceneCommand::PushScaleTranslate { .. }
            | SceneCommand::PopTransform
            | SceneCommand::BeginShadow { .. }
            | SceneCommand::EndShadow
            | SceneCommand::BeginBlur { .. }
            | SceneCommand::EndBlur
            | SceneCommand::BeginFilter { .. }
            | SceneCommand::EndFilter
            | SceneCommand::BeginMask { .. }
            | SceneCommand::EndMask => None,
        })
        .collect()
}

fn diagnostic_codes(result: &zenith_scene::CompileResult) -> Vec<&str> {
    result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn imported_instance_expands_component_from_in_memory_graph() {
    let host = host_doc("library#component.component.card");
    let imported = imported_doc();
    let imports = ImportGraph::new().with_document("library", &imported);

    let result = compile_page_with_imports(&host, &default_provider(), 0, None, &imports);

    assert_eq!(diagnostic_codes(&result), Vec::<&str>::new());
    assert_eq!(
        fill_rects(&result),
        vec![(5.0, 7.0, 40.0, 20.0, (0, 0, 255))]
    );
}

#[test]
fn compile_page_without_graph_keeps_unsupported_source_advisory() {
    let host = host_doc("library#component.component.card");

    let result = compile_page(&host, &default_provider(), 0, None);

    assert_eq!(
        diagnostic_codes(&result),
        vec!["scene.unsupported_import_source"]
    );
    assert!(fill_rects(&result).is_empty());
}

#[test]
fn imported_component_uses_imported_tokens_not_host_tokens() {
    let host = host_doc("library#component.component.card");
    let imported = imported_doc();
    let imports = ImportGraph::new().with_document("library", &imported);

    let result = compile_page_with_imports(&host, &default_provider(), 0, None, &imports);

    let fills = fill_rects(&result);
    assert_eq!(fills, vec![(5.0, 7.0, 40.0, 20.0, (0, 0, 255))]);
}

#[test]
fn imported_component_override_fill_uses_host_token_scope() {
    let host = host_doc_with_instance_body(
        r#"instance id="inst.imported" source="library#component.component.card" x=(px)5 y=(px)7 {
        override ref="bg" fill=(token)"color.override"
      }"#,
    );
    let imported = imported_doc();
    let imports = ImportGraph::new().with_document("library", &imported);

    let result = compile_page_with_imports(&host, &default_provider(), 0, None, &imports);

    assert_eq!(diagnostic_codes(&result), Vec::<&str>::new());
    assert_eq!(
        fill_rects(&result),
        vec![(5.0, 7.0, 40.0, 20.0, (255, 255, 0))]
    );
}

#[test]
fn missing_import_emits_unknown_import_and_skips() {
    let host = host_doc("missing#component.component.card");
    let imports = ImportGraph::new();

    let result = compile_page_with_imports(&host, &default_provider(), 0, None, &imports);

    assert_eq!(diagnostic_codes(&result), vec!["scene.unknown_import"]);
    assert!(fill_rects(&result).is_empty());
}

#[test]
fn missing_imported_component_emits_unknown_import_component_and_skips() {
    let host = host_doc("library#component.component.missing");
    let imported = imported_doc();
    let imports = ImportGraph::new().with_document("library", &imported);

    let result = compile_page_with_imports(&host, &default_provider(), 0, None, &imports);

    assert_eq!(
        diagnostic_codes(&result),
        vec!["scene.unknown_import_component"]
    );
    assert!(fill_rects(&result).is_empty());
}

#[test]
fn unsupported_page_target_emits_unsupported_import_target_and_skips() {
    let host = host_doc("library#page.page.imported");
    let imported = imported_doc();
    let imports = ImportGraph::new().with_document("library", &imported);

    let result = compile_page_with_imports(&host, &default_provider(), 0, None, &imports);

    assert_eq!(
        diagnostic_codes(&result),
        vec!["scene.unsupported_import_target"]
    );
    assert!(fill_rects(&result).is_empty());
}

#[test]
fn malformed_source_emits_invalid_import_source_and_skips() {
    let host = host_doc("library/component.component.card");
    let imported = imported_doc();
    let imports = ImportGraph::new().with_document("library", &imported);

    let result = compile_page_with_imports(&host, &default_provider(), 0, None, &imports);

    assert_eq!(
        diagnostic_codes(&result),
        vec!["scene.invalid_import_source"]
    );
    assert!(fill_rects(&result).is_empty());
}
