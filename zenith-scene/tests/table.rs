//! Integration tests for single-page `table` compilation (UNIT B).
//!
//! Covers: cell background + border command emission, cell content (text)
//! positioned at the cell content-box origin, a `colspan=2` cell spanning two
//! columns' width, and a `visible=#false` table emitting nothing.

mod common;

use common::{SceneCommand, compile, default_provider, parse};

/// A 2-row × 3-col table: one explicit column (160px) plus two auto columns,
/// with a colspan=2 cell in the first row. Border + fill use color tokens.
fn table_src() -> &'static str {
    r##"zenith version=1 {
  project id="proj.tbl" name="TBL"
  tokens format="zenith-token-v1" {
    token id="color.line" type="color" value="#cccccc"
    token id="color.cellbg" type="color" value="#f0f0f0"
    token id="color.ink" type="color" value="#000000"
  }
  styles {}
  document id="doc.tbl" title="TBL" {
    page id="page.tbl" w=(px)640 h=(px)400 {
      table id="t1" x=(px)40 y=(px)40 w=(px)520 h=(px)240 border=(token)"color.line" border-width=(px)1 fill=(token)"color.cellbg" cell-padding=(px)0 gap=(px)0 {
        column width=(px)160
        column
        column
        row {
          cell { text id="c11" x=(px)0 y=(px)0 fill=(token)"color.ink" { span "Name" } }
          cell colspan=2 { text id="c12" x=(px)0 y=(px)0 fill=(token)"color.ink" { span "Details" } }
        }
        row {
          cell { text id="c21" x=(px)0 y=(px)0 fill=(token)"color.ink" { span "Ada" } }
          cell { text id="c22" x=(px)0 y=(px)0 fill=(token)"color.ink" { span "Lovelace" } }
          cell { text id="c23" x=(px)0 y=(px)0 fill=(token)"color.ink" { span "1815" } }
        }
      }
    }
  }
}
"##
}

#[test]
fn table_emits_cell_backgrounds_and_borders() {
    let doc = parse(table_src());
    let result = compile(&doc, &default_provider());

    let fill_count = result
        .scene
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::FillRect { .. }))
        .count();
    let stroke_count = result
        .scene
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::StrokeLine { .. }))
        .count();

    // 5 placed cells (2 in row 1 due to colspan, 3 in row 2). Each emits one
    // FillRect (cell background) and four StrokeLines (separate border edges).
    // The page has no background, so every FillRect here is a cell background.
    assert_eq!(fill_count, 5, "expected one fill per placed cell");
    assert_eq!(
        stroke_count,
        5 * 4,
        "expected four border edges per placed cell"
    );

    // Cell content: every cell's text must produce a glyph run.
    let glyph_runs = result
        .scene
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::DrawGlyphRun { .. }))
        .count();
    assert_eq!(glyph_runs, 5, "expected one glyph run per cell text");
}

#[test]
fn colspan_cell_spans_two_columns() {
    let doc = parse(table_src());
    let result = compile(&doc, &default_provider());

    // Column widths: explicit 160 + two autos sharing the leftover.
    // table_w=520, gap=0, pad=0 → leftover = 520 - 160 = 360, split → 180 each.
    // Row-1 cells (in emission order): cell0 (col0, w=160), cell1 colspan=2
    // (cols 1+2, w=180+180=360). Find the first two FillRects (row 1, since
    // emission is row-major).
    let fills: Vec<(f64, f64)> = result
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::FillRect { x, w, .. } => Some((*x, *w)),
            _ => None,
        })
        .collect();

    assert!(fills.len() >= 2);
    // First cell: x=40 (table origin), width=160.
    assert!((fills[0].0 - 40.0).abs() < 0.01, "cell0 x; got {fills:?}");
    assert!((fills[0].1 - 160.0).abs() < 0.01, "cell0 w; got {fills:?}");
    // Colspan cell: starts at x=40+160=200, width=360 (two auto cols).
    assert!(
        (fills[1].0 - 200.0).abs() < 0.01,
        "colspan x; got {fills:?}"
    );
    assert!(
        (fills[1].1 - 360.0).abs() < 0.01,
        "colspan w; got {fills:?}"
    );
}

#[test]
fn cell_text_positioned_at_content_origin() {
    let doc = parse(table_src());
    let result = compile(&doc, &default_provider());

    // The first cell's text (authored x=0) is translated to the cell content
    // origin x=40 (table x + 0 padding). With h-align default "start" the run
    // x equals the content-box left edge.
    let first_run_x = result.scene.commands.iter().find_map(|c| match c {
        SceneCommand::DrawGlyphRun { x, .. } => Some(*x),
        _ => None,
    });
    assert_eq!(
        first_run_x,
        Some(40.0),
        "first cell text must sit at the cell content origin x=40"
    );
}

#[test]
fn invisible_table_emits_nothing() {
    let src = table_src().replace("table id=\"t1\"", "table id=\"t1\" visible=#false");
    let doc = parse(&src);
    let result = compile(&doc, &default_provider());

    // No table-derived commands: no FillRect (no page bg), no StrokeLine, no
    // glyph runs. (PushClip for the media box is always present.)
    let drawn = result.scene.commands.iter().any(|c| {
        matches!(
            c,
            SceneCommand::FillRect { .. }
                | SceneCommand::StrokeLine { .. }
                | SceneCommand::DrawGlyphRun { .. }
        )
    });
    assert!(!drawn, "an invisible table must emit no drawing commands");
}
