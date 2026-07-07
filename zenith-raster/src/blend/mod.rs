//! Deterministic W3C compositing blend math.

mod nonseparable;
mod pixel;
mod separable;

pub use pixel::blend_pixel;
