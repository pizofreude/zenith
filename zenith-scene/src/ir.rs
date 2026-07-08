//! Scene IR — the backend-neutral display-list primitives.
//!
//! Every type in this module derives `Debug`, `Clone`, `PartialEq`, and
//! `serde::Serialize`.  No `HashMap` or `HashSet` is used anywhere in this
//! module, so JSON serialization is deterministic (struct field order is
//! stable; `BTreeMap` would be used if maps were ever needed).
//!
//! The `scene` field name is always the first field in `Scene` so the
//! `schema` key appears first in the serialized JSON.
//!
//! Submodules: `primitives` (style enums + path segments), `effects` (paint and
//! effect specs), `command` (the `SceneCommand` enum), `scene` (the top-level
//! `Scene` container).

mod command;
mod effects;
mod primitives;
mod scene;

pub use zenith_core::{BlendMode, Color, GradientPaint, GradientStop};

pub use command::SceneCommand;
pub use effects::{
    FilterSpec, FitMode, ImageClip, MaskShape, MaskSpec, Paint, ShadowSpec, SrcRect, SvgStyle,
};
pub use primitives::{
    FillRule, LineCap, LineJoin, PathSegment, StrokeAlign, path_segments_bbox, path_segments_finite,
};
pub use scene::{Rect, Scene, SceneGlyph};
