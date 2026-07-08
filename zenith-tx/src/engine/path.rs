//! Path op application: `set_path_anchors`, `insert_path_anchor`,
//! `insert_path_anchor_at_point`, `move_path_anchor`, `simplify_path_anchors`,
//! and `transform_path_anchors`.
//!
//! Submodules: `apply` (the per-op entry points), `geometry` (anchor/geometry
//! resolution and conversion), `diagnostics` (diagnostic constructors).

mod apply;
mod diagnostics;
mod geometry;

pub(crate) use apply::{
    apply_insert_path_anchor, apply_insert_path_anchor_at_point, apply_remove_path_anchor,
    apply_set_path_anchor_kind, apply_set_path_anchors, apply_simplify_path_anchors,
    apply_transform_path_anchors,
};
pub(crate) use diagnostics::{invalid_anchor, reject_compound_path, unknown_node};
pub(crate) use geometry::{
    anchor_coordinate, geometry_anchor_to_core, optional_handle, resolved_path_geometry,
};
