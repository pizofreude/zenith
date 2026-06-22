//! Integration tests for `table` cell content geometry.
//!
//! Covers: a cell providing its children's geometry when the cell text omits
//! w/h/align (auto-box into the content box), `h-align` and `v-align` shifting
//! text within the cell, text wrapping to a narrow column, and the regression
//! that explicit author geometry on the text wins over cell alignment.

mod common;

use common::{SceneCommand, compile, default_provider, parse};

/// Build a single-cell table document whose cell text omits w/h/align, with the
/// given table-level `attrs` appended to the cell open (e.g. `h-align="center"`).
fn auto_cell_src(cell_attrs: &str, text: &str) -> String {
    format!(
        r##"zenith version=1 {{
  project id="proj.ac" name="AC"
  tokens format="zenith-token-v1" {{
    token id="color.ink" type="color" value="#000000"
  }}
  styles {{}}
  document id="doc.ac" title="AC" {{
    page id="page.ac" w=(px)640 h=(px)400 {{
      table id="t.ac" x=(px)40 y=(px)40 w=(px)400 h=(px)200 cell-padding=(px)0 gap=(px)0 {{
        column width=(px)400
        row {{
          cell {cell_attrs} {{ text id="cx" fill=(token)"color.ink" {{ span "{text}" }} }}
        }}
      }}
    }}
  }}
}}
"##
    )
}

fn glyph_runs(result: &zenith_scene::CompileResult) -> Vec<(f64, f64)> {
    result
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::DrawGlyphRun { x, y, .. } => Some((*x, *y)),
            _ => None,
        })
        .collect()
}

#[test]
fn cell_text_without_geometry_compiles_into_content_box() {
    let result = compile(&parse(&auto_cell_src("", "Hello")), &default_provider());
    let runs = glyph_runs(&result);
    assert!(!runs.is_empty(), "cell text without w/h must still render");
    // Cell content x = table origin x (40) + pad (0). Glyph run starts at/after it.
    assert!(
        runs[0].0 >= 40.0 - 0.01,
        "glyph run x must be inside cell content box; got {runs:?}"
    );
}

#[test]
fn cell_h_align_shifts_text_horizontally() {
    let start = compile(&parse(&auto_cell_src("", "Hi")), &default_provider());
    let center = compile(
        &parse(&auto_cell_src("h-align=\"center\"", "Hi")),
        &default_provider(),
    );
    let end = compile(
        &parse(&auto_cell_src("h-align=\"end\"", "Hi")),
        &default_provider(),
    );
    let sx = glyph_runs(&start)[0].0;
    let cx = glyph_runs(&center)[0].0;
    let ex = glyph_runs(&end)[0].0;
    assert!(cx > sx, "center start ({cx}) must be right of start ({sx})");
    assert!(ex > cx, "end start ({ex}) must be right of center ({cx})");
}

/// A row with a SHORT cell (col 0) and a TALL multi-line cell (col 1). Rows are
/// content-sized, so the tall cell sets the row height and the short cell gets
/// vertical slack for `v-align` to act within. (A lone short cell shrink-wraps
/// its row and has no slack — the standard table v-align case needs a taller
/// sibling.)
fn v_align_src(cell_attrs: &str) -> String {
    format!(
        r##"zenith version=1 {{
  project id="proj.va" name="VA"
  tokens format="zenith-token-v1" {{
    token id="color.ink" type="color" value="#000000"
  }}
  styles {{}}
  document id="doc.va" title="VA" {{
    page id="page.va" w=(px)640 h=(px)400 {{
      table id="t.va" x=(px)40 y=(px)40 w=(px)400 h=(px)200 cell-padding=(px)0 gap=(px)0 {{
        column width=(px)120
        column width=(px)120
        row {{
          cell {cell_attrs} {{ text id="short" fill=(token)"color.ink" {{ span "Hi" }} }}
          cell {{ text id="tall" fill=(token)"color.ink" {{ span "L1\nL2\nL3\nL4" }} }}
        }}
      }}
    }}
  }}
}}
"##
    )
}

#[test]
fn cell_v_align_shifts_text_vertically() {
    // glyph_runs[0] is the short cell's "Hi" (row-major: col 0 emits first).
    let top = compile(&parse(&v_align_src("")), &default_provider());
    let middle = compile(
        &parse(&v_align_src("v-align=\"middle\"")),
        &default_provider(),
    );
    let bottom = compile(
        &parse(&v_align_src("v-align=\"bottom\"")),
        &default_provider(),
    );
    let ty = glyph_runs(&top)[0].1;
    let my = glyph_runs(&middle)[0].1;
    let by = glyph_runs(&bottom)[0].1;
    assert!(my > ty, "middle baseline ({my}) must be below top ({ty})");
    assert!(
        by > my,
        "bottom baseline ({by}) must be below middle ({my})"
    );
}

#[test]
fn cell_text_wraps_to_narrow_column() {
    // A long string in a narrow (80px) column must wrap into multiple lines.
    let src = r##"zenith version=1 {
  project id="proj.wr" name="WR"
  tokens format="zenith-token-v1" {
    token id="color.ink" type="color" value="#000000"
  }
  styles {}
  document id="doc.wr" title="WR" {
    page id="page.wr" w=(px)640 h=(px)400 {
      table id="t.wr" x=(px)40 y=(px)40 w=(px)80 h=(px)300 cell-padding=(px)0 gap=(px)0 {
        column width=(px)80
        row {
          cell { text id="cw" fill=(token)"color.ink" { span "one two three four five six seven eight" } }
        }
      }
    }
  }
}
"##;
    let result = compile(&parse(src), &default_provider());
    let runs = glyph_runs(&result);
    assert!(
        runs.len() >= 2,
        "long text in a narrow column must wrap to multiple lines; got {} run(s)",
        runs.len()
    );
    // Wrapped lines descend within the cell: later runs have larger y.
    assert!(
        runs.windows(2).all(|w| w[1].1 >= w[0].1 - 0.01),
        "wrapped lines must descend; got {runs:?}"
    );
}

#[test]
fn cell_text_with_explicit_geometry_unchanged() {
    // Author-specified w/x/align must win — render byte-identically regardless of
    // the cell's h-align (which would otherwise re-place auto-box text).
    let build = |cell_attrs: &str| {
        format!(
            r##"zenith version=1 {{
  project id="proj.ex" name="EX"
  tokens format="zenith-token-v1" {{ token id="color.ink" type="color" value="#000000" }}
  styles {{}}
  document id="doc.ex" title="EX" {{
    page id="page.ex" w=(px)640 h=(px)400 {{
      table id="t.ex" x=(px)40 y=(px)40 w=(px)400 h=(px)200 cell-padding=(px)0 gap=(px)0 {{
        column width=(px)400
        row {{
          cell {cell_attrs} {{ text id="ce" x=(px)0 y=(px)0 w=(px)400 align="start" fill=(token)"color.ink" {{ span "Fixed" }} }}
        }}
      }}
    }}
  }}
}}
"##
        )
    };
    let start = compile(&parse(&build("")), &default_provider());
    let centered = compile(&parse(&build("h-align=\"center\"")), &default_provider());
    assert_eq!(
        glyph_runs(&start),
        glyph_runs(&centered),
        "explicit-geometry cell text must ignore cell h-align (author override wins)"
    );
}
