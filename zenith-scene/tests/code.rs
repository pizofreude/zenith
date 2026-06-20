mod common;
use common::*;
use zenith_core::default_provider;
use zenith_scene::compile;
use zenith_scene::ir::SceneCommand;

#[test]
fn code_node_multi_line_stacks_by_line_height() {
    // A 3-line code node (no w/h → no clip) emits 3 DrawGlyphRun commands
    // whose baselines increase by a constant delta equal to line_height.
    let src = r##"zenith version=1 {
  project id="proj.cd1" name="CD1"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.cd1" title="CD1" {
page id="page.cd1" w=(px)400 h=(px)200 {
  code id="code.cd1" x=(px)10 y=(px)20 font-size=(px)14 overflow="visible" {
    content "line one\nline two\nline three"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());

    let unshaped: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "scene.text_unshaped")
        .collect();
    assert!(
        unshaped.is_empty(),
        "no text_unshaped diagnostics expected; got: {:?}",
        result.diagnostics
    );

    let runs: Vec<f64> = result
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::DrawGlyphRun { y, .. } => Some(*y),
            _ => None,
        })
        .collect();
    assert_eq!(runs.len(), 3, "expected 3 DrawGlyphRun; got {}", runs.len());

    let d0 = runs[1] - runs[0];
    let d1 = runs[2] - runs[1];
    assert!(d0 > 0.0, "baselines must increase; got {runs:?}");
    assert!(
        (d0 - d1).abs() < 0.001,
        "inter-line delta must be constant (line_height); got {d0} vs {d1}"
    );
}

#[test]
fn code_node_overflow_clip_wraps_runs() {
    // Default overflow + w/h present → PushClip, runs…, PopClip.
    let src = r##"zenith version=1 {
  project id="proj.cd2" name="CD2"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.cd2" title="CD2" {
page id="page.cd2" w=(px)400 h=(px)200 {
  code id="code.cd2" x=(px)10 y=(px)20 w=(px)300 h=(px)80 {
    content "alpha\nbeta"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let cmds = &result.scene.commands;

    // First command after the page background is PushClip; last is PopClip.
    let first_run = cmds
        .iter()
        .position(|c| matches!(c, SceneCommand::DrawGlyphRun { .. }))
        .expect("a DrawGlyphRun must exist");
    assert!(
        matches!(cmds[first_run - 1], SceneCommand::PushClip { .. }),
        "PushClip must immediately precede the first run; got {:?}",
        cmds[first_run - 1]
    );
    assert!(
        matches!(cmds.last(), Some(SceneCommand::PopClip)),
        "PopClip must be the final command; got {:?}",
        cmds.last()
    );

    // overflow="visible" → no clip at all.
    let src_vis = r##"zenith version=1 {
  project id="proj.cd2v" name="CD2V"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.cd2v" title="CD2V" {
page id="page.cd2v" w=(px)400 h=(px)200 {
  code id="code.cd2v" x=(px)10 y=(px)20 w=(px)300 h=(px)80 overflow="visible" {
    content "alpha\nbeta"
  }
}
  }
}
"##;
    let doc_vis = parse(src_vis);
    let result_vis = compile(&doc_vis, &default_provider());
    // The page itself always wraps content in one PushClip/PopClip. With
    // overflow=visible the code node must add NO clip of its own, so exactly
    // one PushClip (the page) should be present — not two.
    let push_clips = result_vis
        .scene
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::PushClip { .. }))
        .count();
    assert_eq!(
        push_clips, 1,
        "overflow=visible must add no clip beyond the page's; got {:?}",
        result_vis.scene.commands
    );
}

#[test]
fn code_node_blank_line_preserves_spacing() {
    // "a\n\nb" → 2 runs (blank skipped), but "b" sits at i=2 spacing:
    // baseline_b == code_y + ascent + 2*line_height.
    let src = r##"zenith version=1 {
  project id="proj.cd3" name="CD3"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.cd3" title="CD3" {
page id="page.cd3" w=(px)400 h=(px)200 {
  code id="code.cd3" x=(px)10 y=(px)20 font-size=(px)14 overflow="visible" {
    content "a\n\nb"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let runs: Vec<f64> = result
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::DrawGlyphRun { y, .. } => Some(*y),
            _ => None,
        })
        .collect();
    assert_eq!(
        runs.len(),
        2,
        "blank middle line must be skipped → 2 runs; got {}",
        runs.len()
    );

    // The delta between "a" (i=0) and "b" (i=2) must equal 2*line_height,
    // i.e. exactly twice a single-step delta. Derive a single step from a
    // separate two-line node sharing the same font/size.
    let src2 = r##"zenith version=1 {
  project id="proj.cd3b" name="CD3B"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.cd3b" title="CD3B" {
page id="page.cd3b" w=(px)400 h=(px)200 {
  code id="code.cd3b" x=(px)10 y=(px)20 font-size=(px)14 overflow="visible" {
    content "a\nb"
  }
}
  }
}
"##;
    let doc2 = parse(src2);
    let result2 = compile(&doc2, &default_provider());
    let runs2: Vec<f64> = result2
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::DrawGlyphRun { y, .. } => Some(*y),
            _ => None,
        })
        .collect();
    assert_eq!(runs2.len(), 2);
    let single_step = runs2[1] - runs2[0];
    let blank_gap = runs[1] - runs[0];
    assert!(
        (blank_gap - 2.0 * single_step).abs() < 0.001,
        "blank line must reserve one line: expected 2*{single_step}, got {blank_gap}"
    );
}

#[test]
fn code_node_tab_expansion_compiles() {
    // A line with a leading tab and tab-width=2 expands to 2 leading spaces.
    // Exact glyph counts are brittle, so assert the node compiles to a run
    // without a shaping error.
    let src = r##"zenith version=1 {
  project id="proj.cd4" name="CD4"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.cd4" title="CD4" {
page id="page.cd4" w=(px)400 h=(px)200 {
  code id="code.cd4" x=(px)10 y=(px)20 tab-width=2 overflow="visible" {
    content "\tindented"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let unshaped: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "scene.text_unshaped")
        .collect();
    assert!(
        unshaped.is_empty(),
        "no shaping error expected: {unshaped:?}"
    );
    let run_count = result
        .scene
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::DrawGlyphRun { .. }))
        .count();
    assert_eq!(run_count, 1, "expected one DrawGlyphRun");
}

#[test]
fn code_node_default_font_is_mono() {
    // No font-family → the run's font_id resolves to the mono face.
    let src = r##"zenith version=1 {
  project id="proj.cd5" name="CD5"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.cd5" title="CD5" {
page id="page.cd5" w=(px)400 h=(px)200 {
  code id="code.cd5" x=(px)10 y=(px)20 overflow="visible" {
    content "fn main() {}"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let font_id = result
        .scene
        .commands
        .iter()
        .find_map(|c| match c {
            SceneCommand::DrawGlyphRun { font_id, .. } => Some(font_id.clone()),
            _ => None,
        })
        .expect("a DrawGlyphRun must exist");
    assert!(
        font_id.contains("noto-sans-mono"),
        "default code font must be mono; got font_id {font_id}"
    );
}

/// A code node with `language="rust"` and a Rust snippet must produce MORE
/// DrawGlyphRun commands than there are lines (per-token splitting) and at
/// least two distinct colors.
#[test]
fn code_node_highlighted_rust_emits_per_token_runs() {
    let src = r##"zenith version=1 {
  project id="proj.hl1" name="HL1"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.hl1" title="HL1" {
page id="page.hl1" w=(px)800 h=(px)400 {
  code id="code.hl1" x=(px)10 y=(px)10 language="rust" overflow="visible" {
    content "let x = 42;"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let runs: Vec<&SceneCommand> = result
        .scene
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::DrawGlyphRun { .. }))
        .collect();
    // "let x = 42;" tokenises into multiple tokens → more than 1 run per line.
    assert!(
        runs.len() > 1,
        "highlighted line must emit multiple runs; got {}",
        runs.len()
    );
    // At least two distinct colors must appear (keyword vs number vs operator…).
    let mut colors: Vec<(u8, u8, u8, u8)> = runs
        .iter()
        .filter_map(|c| match c {
            SceneCommand::DrawGlyphRun { color, .. } => Some((color.r, color.g, color.b, color.a)),
            _ => None,
        })
        .collect();
    colors.dedup();
    assert!(
        colors.len() >= 2,
        "at least two distinct colors expected; got {:?}",
        colors
    );
}

/// A code node with NO language (or an unsupported one) must emit exactly
/// ONE DrawGlyphRun per non-empty line — byte-identical to the pre-highlight
/// behavior.
#[test]
fn code_node_no_language_single_run_per_line() {
    let src = r##"zenith version=1 {
  project id="proj.hl2" name="HL2"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.hl2" title="HL2" {
page id="page.hl2" w=(px)800 h=(px)400 {
  code id="code.hl2" x=(px)10 y=(px)10 language="zzz" overflow="visible" {
    content "let x = 42;\nlet y = 1;"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let run_count = result
        .scene
        .commands
        .iter()
        .filter(|c| matches!(c, SceneCommand::DrawGlyphRun { .. }))
        .count();
    // 2 non-empty lines → exactly 2 runs (single-run plain path).
    assert_eq!(
        run_count, 2,
        "unsupported language must yield 1 run/line (2 total); got {run_count}"
    );
}

/// A code node with `language="rust"` and a doc-declared `syntax.keyword`
/// token (red) must use that color for keyword runs, overriding the builtin.
#[test]
fn code_node_highlighted_doc_token_overrides_builtin_color() {
    // `let` is a Rust keyword. With syntax.keyword=#ff0000 the keyword run
    // must be red (r=255, g=0, b=0).
    let src = r##"zenith version=1 {
  project id="proj.hl3" name="HL3"
  tokens format="zenith-token-v1" {
token id="syntax.keyword" type="color" value="#ff0000"
  }
  styles {}
  document id="doc.hl3" title="HL3" {
page id="page.hl3" w=(px)800 h=(px)400 {
  code id="code.hl3" x=(px)10 y=(px)10 language="rust" overflow="visible" {
    content "let x = 1;"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());
    let keyword_run = result.scene.commands.iter().find_map(|c| match c {
        SceneCommand::DrawGlyphRun { color, .. }
            if color.r == 255 && color.g == 0 && color.b == 0 =>
        {
            Some(*color)
        }
        _ => None,
    });
    assert!(
        keyword_run.is_some(),
        "expected a red (r=255,g=0,b=0) run for the overridden keyword token; \
         commands: {:?}",
        result.scene.commands
    );
}

#[test]
fn code_bold_font_weight_uses_bold_mono_face() {
    let src = r##"zenith version=1 {
  project id="proj.cbw" name="CBW"
  tokens format="zenith-token-v1" {
token id="weight.bold" type="fontWeight" value=700
  }
  styles {}
  document id="doc.cbw" title="CBW" {
page id="page.cbw" w=(px)400 h=(px)200 {
  code id="code.regular" x=(px)10 y=(px)10 {
    content "hello"
  }
  code id="code.bold" x=(px)10 y=(px)50 font-weight=(token)"weight.bold" {
    content "hello"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());

    // Collect DrawGlyphRun font_ids for each code node (by order: regular first,
    // bold second). Both nodes shape the same text, so each emits exactly one run.
    let glyph_runs: Vec<_> = result
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::DrawGlyphRun { font_id, .. } => Some(font_id.clone()),
            _ => None,
        })
        .collect();

    assert!(
        glyph_runs.len() >= 2,
        "expected at least 2 DrawGlyphRun commands (one per code node); got: {:?}",
        glyph_runs
    );

    // The first run (regular weight=400) must use a different font than the
    // second run (bold weight=700).
    let regular_font = &glyph_runs[0];
    let bold_font = &glyph_runs[1];
    assert_ne!(
        regular_font, bold_font,
        "bold code node must use a different font_id than regular code node; \
         regular={regular_font:?}, bold={bold_font:?}"
    );

    // The bold font_id must contain "700" (mirrors the provider id format).
    assert!(
        bold_font.contains("700"),
        "bold code font_id should encode weight 700; got {bold_font:?}"
    );
}

/// A code node WITHOUT `font-weight` defaults to weight 400, and must produce
/// a DrawGlyphRun with the same font_id as a code node with explicit weight=400.
/// This confirms the default-weight path is byte-identical to the original.
#[test]
fn code_default_weight_is_regular_mono_face() {
    let src = r##"zenith version=1 {
  project id="proj.cdw" name="CDW"
  tokens format="zenith-token-v1" {
token id="weight.normal" type="fontWeight" value=400
  }
  styles {}
  document id="doc.cdw" title="CDW" {
page id="page.cdw" w=(px)400 h=(px)200 {
  code id="code.implicit" x=(px)10 y=(px)10 {
    content "hello"
  }
  code id="code.explicit400" x=(px)10 y=(px)50 font-weight=(token)"weight.normal" {
    content "hello"
  }
}
  }
}
"##;
    let doc = parse(src);
    let result = compile(&doc, &default_provider());

    let glyph_runs: Vec<_> = result
        .scene
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::DrawGlyphRun { font_id, .. } => Some(font_id.clone()),
            _ => None,
        })
        .collect();

    assert!(
        glyph_runs.len() >= 2,
        "expected at least 2 DrawGlyphRun commands; got: {:?}",
        glyph_runs
    );

    // Both the implicit-400 and explicit-400 code nodes must resolve to the
    // same (regular) mono font_id — the default path is byte-identical.
    assert_eq!(
        glyph_runs[0], glyph_runs[1],
        "implicit weight=400 and explicit weight=400 must resolve to the same \
         mono font face; implicit={:?}, explicit={:?}",
        glyph_runs[0], glyph_runs[1]
    );

    // The font_id must NOT contain "700".
    assert!(
        !glyph_runs[0].contains("700"),
        "regular code font_id must not encode weight 700; got {:?}",
        glyph_runs[0]
    );
}
