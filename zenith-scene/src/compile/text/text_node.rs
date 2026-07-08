//! The `text` leaf compile path.
//!
//! Wiring only: the submodules carry the logic.
//! - `autofit`: the public `compile_text` entry and the `overflow="autofit"`
//!   shrink-to-fit search.
//! - `sized`: the sized layout engine (`compile_text_sized`) with its fast
//!   single-line path, tab-leader/chain/markdown branches, and
//!   effect/mask/blend/rotation brackets. The multi-sub-path WRAP body lives in
//!   [`super::wrap`].
//! - `overflow`: the post-emit `text.fit_failed` / `text.overflow` diagnostics.

mod autofit;
mod overflow;
mod sized;

pub(in crate::compile) use autofit::compile_text;
pub(in crate::compile) use sized::compile_text_sized;
