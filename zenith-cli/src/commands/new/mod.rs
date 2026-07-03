//! `zenith new` — scaffold a fresh, valid `.zen` document.
//!
//! Wiring only: the scaffolding logic lives in [`scaffold`] and the page-size
//! resolution (named formats, explicit dimensions, orientation, page count) in
//! [`page`].

pub mod page;
mod scaffold;

pub use page::{DEFAULT_PAGE, PageSpec, PaperFormat, resolve_page};
pub use scaffold::{NewErr, NewResult, run, run_in};
