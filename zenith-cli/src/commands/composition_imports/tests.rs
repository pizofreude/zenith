use std::fs;

use zenith_core::{Document, KdlAdapter, KdlSource};

use super::load_import_graph;

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

fn root_with_imports(imports: &str) -> Document {
    parse(&format!(
        r#"zenith version=1 {{
  project id="proj.root" name="Root"
  imports {{
{imports}
  }}
  document id="doc.root" title="Root" {{
    page id="page.root" w=(px)100 h=(px)100
  }}
}}
"#
    ))
}

fn root_with_import_and_body(src: &str, body: &str) -> Document {
    parse(&format!(
        r#"zenith version=1 {{
  project id="proj.root" name="Root"
  imports {{
    import id="child" kind="zen" src="{src}"
  }}
  document id="doc.root" title="Root" {{
{body}
  }}
}}
"#
    ))
}

fn imported_with_component_and_page(component_id: &str, page_id: &str, w: f64, h: f64) -> String {
    format!(
        r#"zenith version=1 {{
  project id="proj.child" name="Child"
  document id="doc.child" title="Child" {{
    page id="{page_id}" w=(px){w} h=(px){h}
  }}
  components {{
    component id="{component_id}" {{
      rect id="mark" x=(px)0 y=(px)0 w=(px)10 h=(px)10
    }}
  }}
}}
"#
    )
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
fn load_import_graph_keeps_same_file_import_aliases() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("shared.zen"), EMPTY_DOC).expect("write shared");
    let root = root_with_imports(
        r#"    import id="first" kind="zen" src="shared.zen"
    import id="second" kind="zen" src="./shared.zen""#,
    );

    let graph = load_import_graph(&root, Some(dir.path()));

    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert!(graph.documents.contains_key("first"));
    assert!(graph.documents.contains_key("second"));
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
fn load_import_graph_reports_hash_mismatch_for_cached_alias() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("shared.zen"), EMPTY_DOC).expect("write shared");
    let root = root_with_imports(
        r#"    import id="first" kind="zen" src="shared.zen"
    import id="second" kind="zen" src="./shared.zen" sha256="0000""#,
    );

    let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "import.hash_mismatch");
    assert_eq!(diagnostics[0].subject_id.as_deref(), Some("second"));
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

#[test]
fn load_import_graph_reports_unknown_component_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("child.zen"),
        imported_with_component_and_page("component.card", "cover", 100.0, 100.0),
    )
    .expect("write child");
    let root = root_with_import_and_body(
        "child.zen",
        r#"    page id="page.root" w=(px)100 h=(px)100 {
      instance id="inst.missing" source="child#component.missing" x=(px)0 y=(px)0
    }"#,
    );

    let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "import.unknown_reference");
    assert_eq!(diagnostics[0].subject_id.as_deref(), Some("inst.missing"));
}

#[test]
fn load_import_graph_reports_unsupported_instance_page_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("child.zen"),
        imported_with_component_and_page("component.card", "cover", 100.0, 100.0),
    )
    .expect("write child");
    let root = root_with_import_and_body(
        "child.zen",
        r#"    page id="page.root" w=(px)100 h=(px)100 {
      instance id="inst.page" source="child#page.cover" x=(px)0 y=(px)0
    }"#,
    );

    let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "import.unsupported_target");
    assert_eq!(diagnostics[0].subject_id.as_deref(), Some("inst.page"));
}

#[test]
fn load_import_graph_reports_unknown_page_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("child.zen"),
        imported_with_component_and_page("component.card", "cover", 100.0, 100.0),
    )
    .expect("write child");
    let root = root_with_import_and_body(
        "child.zen",
        r#"    page id="page.root" source="child#page.missing" w=(px)100 h=(px)100"#,
    );

    let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "import.unknown_reference");
    assert_eq!(diagnostics[0].subject_id.as_deref(), Some("page.root"));
}

#[test]
fn load_import_graph_reports_expanded_id_collision() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("child.zen"),
        imported_with_component_and_page("component.card", "cover", 100.0, 100.0),
    )
    .expect("write child");
    // The host authors a node whose id equals what the instance expansion
    // (`<instance-id>/<local-id>`) would produce: `card/mark`.
    let root = root_with_import_and_body(
        "child.zen",
        r#"    page id="page.root" w=(px)100 h=(px)100 {
      rect id="card/mark" x=(px)0 y=(px)0 w=(px)10 h=(px)10
      instance id="card" source="child#component.component.card" x=(px)0 y=(px)0
    }"#,
    );

    let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "import.id_collision");
    assert_eq!(diagnostics[0].subject_id.as_deref(), Some("card"));
}

#[test]
fn load_import_graph_reports_page_size_mismatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("child.zen"),
        imported_with_component_and_page("component.card", "cover", 200.0, 100.0),
    )
    .expect("write child");
    let root = root_with_import_and_body(
        "child.zen",
        r#"    page id="page.root" source="child#page.cover" w=(px)100 h=(px)100"#,
    );

    let diagnostics = load_import_graph(&root, Some(dir.path())).into_diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "import.page_size_mismatch");
    assert_eq!(diagnostics[0].subject_id.as_deref(), Some("page.root"));
}
