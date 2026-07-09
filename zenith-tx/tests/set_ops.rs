mod common;
use common::*;
use zenith_tx::op::OpPathAnchor;
use zenith_tx::{Op, OpPoint, Permissions, Transaction, TxStatus, run_transaction};

#[path = "set_ops/fill_stroke.rs"]
mod fill_stroke;
#[path = "set_ops/geometry.rs"]
mod geometry;
#[path = "set_ops/points_paths.rs"]
mod points_paths;
#[path = "set_ops/text_align.rs"]
mod text_align;
#[path = "set_ops/text_overflow.rs"]
mod text_overflow;
#[path = "set_ops/visibility.rs"]
mod visibility;
