//! Raster image → PDF image XObject encoding.
//!
//! The asset bytes (PNG or JPEG) are decoded to straight-alpha RGBA8 via the
//! same decoder the raster backend uses, then split into a FlateDecode RGB
//! image XObject plus, when any pixel is non-opaque, a FlateDecode DeviceGray
//! `/SMask` carrying the alpha channel. Re-encoding to Flate (rather than
//! passing JPEGs through as DCTDecode) keeps a single, uniform, deterministic
//! path for both formats. miniz_oxide deflate is deterministic for fixed input.

use miniz_oxide::deflate::compress_to_vec_zlib;

use crate::tiny_skia::decode_raster_to_pixmap;

/// A decoded image ready to embed: deflated RGB samples, the optional deflated
/// alpha SMask, and the pixel dimensions.
pub(super) struct DecodedImage {
    pub(super) width: u32,
    pub(super) height: u32,
    /// zlib-deflated interleaved RGB8 samples (row-major, top-to-bottom).
    pub(super) rgb_flate: Vec<u8>,
    /// zlib-deflated DeviceGray8 alpha samples; `None` when the image is fully
    /// opaque (no `/SMask` needed).
    pub(super) alpha_flate: Option<Vec<u8>>,
}

/// Decode `bytes` (PNG or JPEG) and prepare it for embedding, or `None` when the
/// format is unsupported / the data is malformed / dimensions are zero.
pub(super) fn decode_for_pdf(bytes: &[u8]) -> Option<DecodedImage> {
    // Reuse the raster decoder (PNG via tiny-skia, JPEG via jpeg-decoder). The
    // returned Pixmap holds premultiplied RGBA; convert back to straight alpha
    // so the RGB plane and the SMask are independent (PDF composites them).
    let pixmap = decode_raster_to_pixmap(bytes)?;
    let width = pixmap.width();
    let height = pixmap.height();
    if width == 0 || height == 0 {
        return None;
    }
    let px = pixmap.pixels();
    let mut rgb = Vec::with_capacity(px.len() * 3);
    let mut alpha = Vec::with_capacity(px.len());
    let mut any_transparent = false;
    for p in px {
        let a = p.alpha();
        // tiny-skia stores premultiplied channels; `demultiply` yields straight
        // (un-premultiplied) RGBA so the RGB plane is independent of alpha.
        let s = p.demultiply();
        rgb.push(s.red());
        rgb.push(s.green());
        rgb.push(s.blue());
        alpha.push(a);
        if a != 255 {
            any_transparent = true;
        }
    }

    let level = 6; // miniz_oxide default; deterministic for fixed input.
    let rgb_flate = compress_to_vec_zlib(&rgb, level);
    let alpha_flate = if any_transparent {
        Some(compress_to_vec_zlib(&alpha, level))
    } else {
        None
    };

    Some(DecodedImage {
        width,
        height,
        rgb_flate,
        alpha_flate,
    })
}
