//! Font sourcing layer: provider trait, data types, and the bundled default.

pub mod embedded;
pub mod local;
mod provider;

pub use local::{LocalFontEntry, scan_font_dirs};
pub use provider::{
    BytesFontProvider, FontData, FontProvider, FontSource, FontStyle, default_provider,
};
