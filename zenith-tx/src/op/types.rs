//! Supporting value types referenced by [`super::Op`] variants: path geometry
//! payloads, span/asset metadata, insertion position, and per-transaction
//! permission flags.

/// A 2-D vertex used by [`super::Op::SetPoints`], expressed in pixels.
///
/// JSON shape: `{"x": 50.0, "y": 80.0}`
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct OpPoint {
    /// X coordinate in document pixels.
    pub x: f64,
    /// Y coordinate in document pixels.
    pub y: f64,
}

/// A path anchor used by [`super::Op::SetPathAnchors`], expressed in pixels.
///
/// JSON shape: `{"x": 50.0, "y": 80.0, "kind": "smooth", "in_x": 40.0, "in_y": 80.0, "out_x": 60.0, "out_y": 80.0}`.
/// Handle coordinates and authoring kind are optional and default to absent.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct OpPathAnchor {
    /// Anchor X coordinate in document pixels.
    pub x: f64,
    /// Anchor Y coordinate in document pixels.
    pub y: f64,
    /// Optional authoring intent: `corner`, `smooth`, `symmetric`, or a preserved future value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Optional incoming handle X coordinate in document pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_x: Option<f64>,
    /// Optional incoming handle Y coordinate in document pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_y: Option<f64>,
    /// Optional outgoing handle X coordinate in document pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_x: Option<f64>,
    /// Optional outgoing handle Y coordinate in document pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out_y: Option<f64>,
}

/// A contour payload used by [`super::Op::AddPath`] for compound paths.
///
/// JSON shape: `{"closed": true, "anchors": [{"x": 0.0, "y": 0.0}, ...]}`.
/// Anchor coordinates are expressed in document pixels.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct OpPathSubpath {
    /// Optional per-contour closure. `None` preserves the default open contour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<bool>,
    /// Ordered anchor list for this contour.
    #[serde(default)]
    pub anchors: Vec<OpPathAnchor>,
}

/// Which Bezier handle on a path anchor to move.
///
/// JSON values are `"in"` and `"out"`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpPathHandle {
    /// The incoming handle (`in_x`, `in_y`).
    In,
    /// The outgoing handle (`out_x`, `out_y`).
    Out,
}

/// Transform to apply to every editable anchor point in a [`super::Op::TransformPathAnchors`] op.
///
/// JSON shapes:
/// - `{"mode":"translate","dx":10,"dy":-4}`
/// - `{"mode":"rotate","angle_degrees":90,"cx":50,"cy":50}`
/// - `{"mode":"reflect","x1":0,"y1":0,"x2":100,"y2":0}`
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum OpPathTransform {
    /// Translate all anchor and complete handle points by `dx`,`dy` pixels.
    Translate { dx: f64, dy: f64 },
    /// Rotate all anchor and complete handle points around `cx`,`cy` by degrees.
    Rotate {
        angle_degrees: f64,
        cx: f64,
        cy: f64,
    },
    /// Reflect all anchor and complete handle points across the line from `x1`,`y1` to `x2`,`y2`.
    Reflect { x1: f64, y1: f64, x2: f64, y2: f64 },
}

/// Boolean operation to materialize between two simple closed path contours.
///
/// JSON values are `"union"`, `"intersect"`, `"subtract"`, and `"exclude"`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OpPathBooleanOperation {
    /// Boundary union of source and target contours.
    Union,
    /// Boundary intersection of source and target contours.
    Intersect,
    /// Source contour with the target contour subtracted.
    Subtract,
    /// Symmetric difference of source and target contours.
    Exclude,
}

/// A single text span used by [`super::Op::ReplaceText`].
///
/// JSON shape: `{"text":"Hello","fill":"color.brand","italic":true}`.
/// All fields except `text` are optional and default to `None`/absent.
/// `fill` and `font_weight` are token ids (like [`super::Op::SetFill`]), not raw values.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct OpSpan {
    /// The literal text content of this span.
    pub text: String,
    /// Token id to set as the per-span fill (e.g. `"color.brand"`). `None` = inherit.
    #[serde(default)]
    pub fill: Option<String>,
    /// Token id to set as the per-span font-weight. `None` = inherit.
    #[serde(default)]
    pub font_weight: Option<String>,
    /// Italic override. `None` = inherit.
    #[serde(default)]
    pub italic: Option<bool>,
    /// Underline decoration. `None` = inherit.
    #[serde(default)]
    pub underline: Option<bool>,
    /// Strikethrough decoration. `None` = inherit.
    #[serde(default)]
    pub strikethrough: Option<bool>,
    /// Vertical alignment (`"super"` / `"sub"`). `None` = baseline (inherit).
    #[serde(default)]
    pub vertical_align: Option<String>,
    /// Footnote reference — the id of a page-level footnote. `None` = no ref.
    #[serde(default)]
    pub footnote_ref: Option<String>,
}

/// Optional producer and AI provenance carried by [`super::Op::AddAsset`].
///
/// The struct is flattened in JSON so the public operation shape remains
/// `producer_kind`, `ai_prompt`, and so on at the top level of `add_asset`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AddAssetMetadata {
    /// Which producer froze this asset (e.g. `"file-import"`, `"zpx-bake"`).
    #[serde(default)]
    pub producer_kind: Option<String>,
    /// The producer-specific source reference (imported file path, or source
    /// `.zpx` manifest hash).
    #[serde(default)]
    pub producer_source: Option<String>,
    /// Prompt text used to generate the asset.
    #[serde(default)]
    pub ai_prompt: Option<String>,
    /// Model identifier used to generate the asset.
    #[serde(default)]
    pub ai_model: Option<String>,
    /// Provider that hosted the generation model.
    #[serde(default)]
    pub ai_provider: Option<String>,
    /// Random seed passed to the generation model.
    #[serde(default)]
    pub ai_seed: Option<i64>,
    /// Date on which the asset was generated.
    #[serde(default)]
    pub ai_generation_date: Option<String>,
    /// License under which the generated asset may be used.
    #[serde(default)]
    pub ai_license: Option<String>,
    /// Rights information for source material used during generation.
    #[serde(default)]
    pub ai_source_rights: Option<String>,
    /// Safety review status of the generated asset.
    #[serde(default)]
    pub ai_safety_status: Option<String>,
    /// Policy governing reuse of the generated asset.
    #[serde(default)]
    pub ai_reuse_policy: Option<String>,
}

/// Insertion position for [`super::Op::AddNode`] and [`super::Op::AddPath`] within a container's children.
///
/// JSON shapes: `{"at":"last"}`, `{"at":"first"}`, `{"at":"index","index":2}`,
/// `{"at":"before","id":"sibling"}`, `{"at":"after","id":"sibling"}`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(tag = "at", rename_all = "snake_case")]
pub enum Position {
    /// Insert as the last child (topmost in z-order). Default.
    #[default]
    Last,
    /// Insert as the first child (bottommost in z-order).
    First,
    /// Insert at an explicit index (clamped to the children length).
    Index { index: usize },
    /// Insert immediately before the sibling with this id.
    Before { id: String },
    /// Insert immediately after the sibling with this id.
    After { id: String },
}

/// Per-transaction permission flags that relax otherwise-enforced guards.
///
/// Carried in a transaction's optional `"permissions"` object, e.g.
/// `{"permissions":{"allow_locked":false,"allow_raw_visual_literals":false}}`.
/// Both flags default to `false`, so a transaction JSON that omits the
/// `permissions` key still parses with all guards active.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
pub struct Permissions {
    /// When `true`, mutating ops are allowed to target locked nodes.
    /// When `false` (default), a guarded op against a locked node is rejected
    /// with a `node.locked` diagnostic.
    #[serde(default)]
    pub allow_locked: bool,
    /// When `true`, raw (non-token) visual literal values are permitted.
    #[serde(default)]
    pub allow_raw_visual_literals: bool,
}
