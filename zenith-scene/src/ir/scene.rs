//! Top-level scene container and its members: `SceneGlyph`, `Rect`, `Scene`.

use serde::Serialize;

use super::SceneCommand;

// ── Scene glyph ───────────────────────────────────────────────────────────────

/// A single positioned glyph within a [`SceneCommand::DrawGlyphRun`].
///
/// Offsets `dx` and `dy` are pen offsets from the run origin, baseline-relative.
/// Positive `dx` is rightward; positive `dy` is downward (0 = on the baseline).
/// No font bytes appear here — only the glyph ID within the resolved font face.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SceneGlyph {
    /// Glyph identifier within the resolved font face.
    pub glyph_id: u16,
    /// Horizontal pen offset from the run origin, in pixels.
    pub dx: f32,
    /// Vertical offset from the baseline, in pixels (positive = below baseline).
    pub dy: f32,
    /// Source Unicode text this glyph maps back to, for text extraction
    /// (PDF ToUnicode CMap). Empty for the trailing glyphs of a multi-glyph
    /// cluster and for runs that carry no source mapping. Serialized only when
    /// non-empty, so scenes without it stay byte-identical.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
}

// ── Trim rect ───────────────────────────────────────────────────────────────

/// An axis-aligned rectangle in scene (top-left origin, y-down) coordinates,
/// in pixels.
///
/// Used to carry the print **trim box** on a [`Scene`] when a page declares a
/// positive `bleed` margin. The scene canvas (`width`/`height`) is the full
/// media box *including* the bleed; the trim rect is the inner rectangle the
/// finished piece is cut down to.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    /// Left edge in pixels (scene coordinates).
    pub x: f64,
    /// Top edge in pixels (scene coordinates).
    pub y: f64,
    /// Width in pixels.
    pub w: f64,
    /// Height in pixels.
    pub h: f64,
}

// ── Scene ─────────────────────────────────────────────────────────────────────

/// A fully resolved, backend-neutral display list.
///
/// The `schema` field is always `"zenith-scene-v1"` and is declared first so
/// that it serializes as the first key in the JSON output, satisfying the
/// normative requirement from the format spec.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Scene {
    /// Always `"zenith-scene-v1"`.  Declared first so it appears first in JSON.
    pub schema: &'static str,
    /// Page / canvas width in pixels.
    pub width: f64,
    /// Page / canvas height in pixels.
    pub height: f64,
    /// Ordered display list.  Paint order: index 0 is painted first (bottom).
    pub commands: Vec<SceneCommand>,
    /// Print **trim box** in scene (top-left origin, y-down) pixel coordinates.
    ///
    /// `Some` only when the page declared a positive `bleed` margin: then
    /// `width`/`height` are the full media box (including bleed) and `trim` is
    /// the inner page rectangle `[b, b, page_w, page_h]`. `None` when there is
    /// no bleed (trim == media box). Skipped in JSON when absent so existing
    /// non-bleed scenes serialize byte-identically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim: Option<Rect>,
}

impl Scene {
    /// Construct an empty scene for the given page dimensions.
    ///
    /// `schema` is always set to `"zenith-scene-v1"`.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            schema: "zenith-scene-v1",
            width,
            height,
            commands: Vec::new(),
            trim: None,
        }
    }

    /// Serialize this scene to a pretty-printed JSON string.
    ///
    /// Uses `serde_json::to_string_pretty` which produces deterministic output
    /// because `Scene` and its fields use only `Vec` (ordered) and `struct`
    /// (stable field order in Rust + serde), never `HashMap`.
    ///
    /// # Errors
    ///
    /// Returns an error only if serialization fails, which cannot happen for
    /// the types used in `Scene` (all fields are plain numerics, strings, and
    /// `u8`s).  The `Result` is kept for API robustness.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::{BlendMode, Color, Paint, PathSegment, path_segments_bbox};
    use super::*;

    #[test]
    fn scene_new_sets_schema() {
        let s = Scene::new(800.0, 600.0);
        assert_eq!(s.schema, "zenith-scene-v1");
        assert_eq!(s.width, 800.0);
        assert_eq!(s.height, 600.0);
        assert!(s.commands.is_empty());
    }

    #[test]
    fn to_json_schema_is_first_key() {
        let s = Scene::new(100.0, 200.0);
        let json = s.to_json().expect("serialization must succeed");
        // The very first `"` after `{` must be `"schema"`.
        let trimmed = json.trim_start_matches('{').trim_start();
        assert!(
            trimmed.starts_with(r#""schema""#),
            "schema must be the first JSON key; got: {trimmed}"
        );
    }

    #[test]
    fn to_json_deterministic() {
        let mut s = Scene::new(640.0, 360.0);
        s.commands.push(SceneCommand::FillRect {
            x: 0.0,
            y: 0.0,
            w: 640.0,
            h: 360.0,
            paint: Paint::solid(Color::srgb(10, 20, 30, 255)),
        });
        let a = s.to_json().expect("first serialize");
        let b = s.to_json().expect("second serialize");
        assert_eq!(a, b, "serialization must be deterministic");
    }

    #[test]
    fn fill_rect_serializes_op_tag() {
        let cmd = SceneCommand::FillRect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
            paint: Paint::solid(Color::srgb(255, 0, 0, 255)),
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        assert!(
            json.contains(r#""op":"FillRect""#),
            "op tag must be FillRect; got: {json}"
        );
    }

    #[test]
    fn glyph_run_source_node_id_does_not_serialize() {
        let cmd = SceneCommand::DrawGlyphRun {
            x: 1.0,
            y: 2.0,
            font_id: "noto-sans-400-normal".to_owned(),
            font_size: 12.0,
            color: Color::srgb(0, 0, 0, 255),
            stroke_color: None,
            stroke_width: None,
            link: None,
            selectable: true,
            source_node_id: Some("text.source".to_owned()),
            glyphs: vec![SceneGlyph {
                glyph_id: 1,
                dx: 0.0,
                dy: 0.0,
                text: "A".to_owned(),
            }],
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        assert!(!json.contains("source_node_id"), "got: {json}");
    }

    #[test]
    fn path_segments_bbox_uses_cubic_extrema() {
        let segments = [
            PathSegment::MoveTo { x: 0.0, y: 0.0 },
            PathSegment::CubicTo {
                x1: 0.0,
                y1: 10.0,
                x2: 10.0,
                y2: 10.0,
                x: 10.0,
                y: 0.0,
            },
        ];

        assert_eq!(path_segments_bbox(&segments), Some((0.0, 0.0, 10.0, 7.5)));
    }

    #[test]
    fn path_segments_bbox_rejects_non_finite_coordinates() {
        let segments = [PathSegment::MoveTo {
            x: f64::NAN,
            y: 0.0,
        }];

        assert_eq!(path_segments_bbox(&segments), None);
    }

    #[test]
    fn srgb_color_omits_cmyk_in_json() {
        let cmd = SceneCommand::FillRect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
            paint: Paint::solid(Color::srgb(1, 2, 3, 255)),
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        assert!(
            !json.contains("cmyk"),
            "sRGB-origin color must not serialize a cmyk key; got: {json}"
        );
    }

    #[test]
    fn cmyk_color_carries_channels_and_serializes() {
        // cmyk(59,85,0,7) → #6124ed (97,36,237).
        let c = Color::cmyk(59.0, 85.0, 0.0, 7.0, 97, 36, 237);
        assert_eq!((c.r, c.g, c.b, c.a), (97, 36, 237, 255));
        assert_eq!(c.cmyk, Some([59.0, 85.0, 0.0, 7.0]));
        let json = serde_json::to_string(&c).expect("serialize");
        assert!(
            json.contains(r#""cmyk":[59.0,85.0,0.0,7.0]"#),
            "got: {json}"
        );
    }

    #[test]
    fn nonseparable_blend_mode_serializes_kebab_case() {
        let cmd = SceneCommand::PushLayer {
            opacity: 1.0,
            blend_mode: Some(BlendMode::Luminosity),
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        assert!(json.contains(r#""blend_mode":"luminosity""#), "got: {json}");
    }
}
