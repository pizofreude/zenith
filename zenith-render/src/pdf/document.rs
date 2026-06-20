//! Top-level PDF document assembly: page boxes, object-id allocation, resource
//! materialization, and the deterministic trailer.

use pdf_writer::types::FunctionShadingType;
use pdf_writer::{Filter, Finish, Pdf, Rect as PdfRect, Ref};
use zenith_core::{AssetProvider, FontProvider};
use zenith_scene::Scene;

use super::content::{ALPHA_PREFIX, IMAGE_PREFIX, PageResources, SHADING_PREFIX, name, translate};
use super::gradient::AxialGradient;

/// Render `scene` to a deterministic vector PDF (a single page).
///
/// `fonts` resolves glyph outlines for any `DrawGlyphRun`; `assets` resolves
/// raster bytes for any `DrawImage`. The output carries print box metadata
/// (MediaBox / TrimBox / BleedBox / CropBox) and native DeviceCMYK colors for
/// CMYK-origin tokens. Identical input yields byte-identical output: no
/// timestamps, no document id, ordered iteration throughout.
///
/// Mirrors the shape of [`crate::render_png`] (`scene`, `fonts`, `assets`).
#[must_use]
pub fn render_pdf(scene: &Scene, fonts: &dyn FontProvider, assets: &dyn AssetProvider) -> Vec<u8> {
    let mut pdf = Pdf::new();

    let catalog_id = Ref::new(1);
    let page_tree_id = Ref::new(2);
    let page_id = Ref::new(3);
    let content_id = Ref::new(4);
    // Subsequent objects (resources) start here and are allocated in order.
    let mut next: i32 = 5;
    let mut alloc = || {
        let r = Ref::new(next);
        next += 1;
        r
    };

    // Translate the scene to a content stream + the resources it references.
    let (content, res) = translate(scene, fonts, assets);

    // Allocate refs for every resource up front so the page's resource dict can
    // reference them. Order is fixed: ExtGStates, then gradient shadings (each
    // with its function), then images (each with optional SMask).
    let alpha_ids: Vec<Ref> = res.alphas.iter().map(|_| alloc()).collect();
    let gradient_refs: Vec<GradientRefs> = res
        .gradients
        .iter()
        .map(|g| {
            let shading = alloc();
            let function = alloc();
            // A multi-stop gradient (> 2 stops) needs one exponential
            // subfunction per segment, stitched together. Allocate those refs
            // here so the whole document uses one clean sequential id space.
            let seg_count = g.stops.len().saturating_sub(1);
            let sub_functions = if g.stops.len() > 2 {
                (0..seg_count).map(|_| alloc()).collect()
            } else {
                Vec::new()
            };
            GradientRefs {
                shading,
                function,
                sub_functions,
            }
        })
        .collect();
    let image_refs: Vec<ImageRefs> = res
        .images
        .iter()
        .map(|img| ImageRefs {
            image: alloc(),
            smask: if img.alpha_flate.is_some() {
                Some(alloc())
            } else {
                None
            },
        })
        .collect();

    // ── Catalog + page tree ──────────────────────────────────────────────
    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id).kids([page_id]).count(1);

    // ── Page dict + boxes + resource dict ────────────────────────────────
    write_page(
        &mut pdf,
        page_id,
        page_tree_id,
        content_id,
        scene,
        &res,
        &alpha_ids,
        &gradient_refs,
        &image_refs,
    );

    // ── Content stream ───────────────────────────────────────────────────
    pdf.stream(content_id, &content.finish());

    // ── Resource objects ─────────────────────────────────────────────────
    write_alpha_states(&mut pdf, &res, &alpha_ids);
    write_gradients(&mut pdf, &res, &gradient_refs);
    write_images(&mut pdf, &res, &image_refs);

    pdf.finish()
}

/// Indirect references backing one axial gradient: its shading dict and its
/// stitching/exponential color function.
struct GradientRefs {
    shading: Ref,
    function: Ref,
    /// One exponential subfunction ref per gradient segment, used only when the
    /// gradient has more than two stops (stitched via `function`).
    sub_functions: Vec<Ref>,
}

/// Indirect references backing one embedded image: the RGB image XObject and an
/// optional alpha SMask image XObject.
struct ImageRefs {
    image: Ref,
    smask: Option<Ref>,
}

#[allow(clippy::too_many_arguments)]
fn write_page(
    pdf: &mut Pdf,
    page_id: Ref,
    page_tree_id: Ref,
    content_id: Ref,
    scene: &Scene,
    res: &PageResources,
    alpha_ids: &[Ref],
    gradient_refs: &[GradientRefs],
    image_refs: &[ImageRefs],
) {
    let w = scene.width as f32;
    let h = scene.height as f32;
    let media = PdfRect::new(0.0, 0.0, w, h);

    let mut page = pdf.page(page_id);
    page.parent(page_tree_id);
    page.media_box(media);

    // Print boxes. When a trim box is present (bleed active), the trim rect is
    // converted from scene (top-left, y-down) coords to PDF (bottom-left, y-up):
    // a scene rect [tx, ty, tw, th] becomes PDF [tx, H-(ty+th), tx+tw, H-ty].
    // BleedBox / CropBox = MediaBox (the canvas already includes the bleed).
    // With no trim, all four boxes equal the MediaBox.
    match scene.trim {
        Some(t) => {
            let x0 = t.x as f32;
            let x1 = (t.x + t.w) as f32;
            let y0 = (scene.height - (t.y + t.h)) as f32;
            let y1 = (scene.height - t.y) as f32;
            page.trim_box(PdfRect::new(x0, y0, x1, y1));
            page.bleed_box(media);
            page.crop_box(media);
        }
        None => {
            page.trim_box(media);
            page.bleed_box(media);
            page.crop_box(media);
        }
    }

    page.contents(content_id);

    // Resource dictionary referencing every interned resource by its stable
    // `<prefix><index>` name.
    let mut resources = page.resources();
    if !res.alphas.is_empty() {
        let mut gs = resources.ext_g_states();
        for (i, r) in alpha_ids.iter().enumerate() {
            let nm = name(ALPHA_PREFIX, i);
            gs.pair(nm.as_name(), *r);
        }
        gs.finish();
    }
    if !res.gradients.is_empty() {
        let mut sh = resources.shadings();
        for (i, gr) in gradient_refs.iter().enumerate() {
            let nm = name(SHADING_PREFIX, i);
            sh.pair(nm.as_name(), gr.shading);
        }
        sh.finish();
    }
    if !res.images.is_empty() {
        let mut xo = resources.x_objects();
        for (i, ir) in image_refs.iter().enumerate() {
            let nm = name(IMAGE_PREFIX, i);
            xo.pair(nm.as_name(), ir.image);
        }
        xo.finish();
    }
    resources.finish();
    page.finish();
}

/// Write one `/ExtGState` per interned alpha, carrying both `ca` (fill) and
/// `CA` (stroke) so a single state serves filled and stroked draws.
fn write_alpha_states(pdf: &mut Pdf, res: &PageResources, alpha_ids: &[Ref]) {
    for (a, r) in res.alphas.iter().zip(alpha_ids) {
        let factor = f32::from(*a) / 255.0;
        let mut gs = pdf.ext_graphics(*r);
        gs.non_stroking_alpha(factor);
        gs.stroking_alpha(factor);
        gs.finish();
    }
}

/// Write each axial gradient as a Type 2 shading whose color function is a Type
/// 3 stitching function over Type 2 (linear, exponent 1) exponential
/// subfunctions — one per adjacent stop pair. Stops are DeviceRGB.
fn write_gradients(pdf: &mut Pdf, res: &PageResources, refs: &[GradientRefs]) {
    for (g, gr) in res.gradients.iter().zip(refs) {
        write_gradient_function(pdf, gr, g);

        let mut shading = pdf.function_shading(gr.shading);
        shading.shading_type(FunctionShadingType::Axial);
        shading.color_space().device_rgb();
        shading.coords(g.coords);
        shading.function(gr.function);
        // Clamp (don't extend) beyond the endpoints so the shading fills the
        // clipped shape with the edge colors, matching CSS `Pad` spread.
        shading.extend([true, true]);
        shading.finish();
    }
}

/// Write the color function for `g`. With exactly two stops a single Type 2
/// exponential (linear) function is emitted at `gr.function`; with more stops a
/// Type 3 stitching function at `gr.function` combines one exponential
/// subfunction per segment (refs in `gr.sub_functions`).
fn write_gradient_function(pdf: &mut Pdf, gr: &GradientRefs, g: &AxialGradient) {
    // Two-stop (or defensively fewer): a single linear exponential function.
    if g.stops.len() <= 2 {
        let c0 = g.stops.first().map(|s| s.1).unwrap_or([0.0, 0.0, 0.0]);
        let c1 = g.stops.get(1).map(|s| s.1).unwrap_or(c0);
        write_linear_segment(pdf, gr.function, c0, c1);
        return;
    }

    // > 2 stops: one linear exponential per segment, stitched together.
    for (k, sub) in gr.sub_functions.iter().enumerate() {
        let c0 = g.stops.get(k).map(|s| s.1).unwrap_or([0.0, 0.0, 0.0]);
        let c1 = g.stops.get(k + 1).map(|s| s.1).unwrap_or(c0);
        write_linear_segment(pdf, *sub, c0, c1);
    }

    // Interior stop offsets become the stitching bounds; each subfunction's
    // input is encoded over [0, 1].
    let last = g.stops.len() - 1;
    let bounds: Vec<f32> = g.stops[1..last].iter().map(|s| s.0).collect();
    let mut encode: Vec<f32> = Vec::with_capacity(gr.sub_functions.len() * 2);
    for _ in &gr.sub_functions {
        encode.push(0.0);
        encode.push(1.0);
    }

    let mut stitch = pdf.stitching_function(gr.function);
    stitch.domain([0.0, 1.0]);
    stitch.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    stitch.functions(gr.sub_functions.iter().copied());
    stitch.bounds(bounds);
    stitch.encode(encode);
    stitch.finish();
}

/// Write a single Type 2 (exponential, `N = 1` linear) function in DeviceRGB
/// mapping `[0, 1]` from color `c0` to `c1`.
fn write_linear_segment(pdf: &mut Pdf, id: Ref, c0: [f32; 3], c1: [f32; 3]) {
    let mut f = pdf.exponential_function(id);
    f.domain([0.0, 1.0]);
    f.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    f.c0(c0);
    f.c1(c1);
    f.n(1.0);
    f.finish();
}

/// Write each image as a FlateDecode DeviceRGB XObject, with an optional
/// FlateDecode DeviceGray SMask for transparency.
fn write_images(pdf: &mut Pdf, res: &PageResources, refs: &[ImageRefs]) {
    for (img, ir) in res.images.iter().zip(refs) {
        let w = img.width as i32;
        let h = img.height as i32;

        let mut xobj = pdf.image_xobject(ir.image, &img.rgb_flate);
        xobj.filter(Filter::FlateDecode);
        xobj.width(w);
        xobj.height(h);
        xobj.color_space().device_rgb();
        xobj.bits_per_component(8);
        if let Some(smask) = ir.smask {
            xobj.s_mask(smask);
        }
        xobj.finish();

        if let (Some(smask), Some(alpha)) = (ir.smask, &img.alpha_flate) {
            let mut sm = pdf.image_xobject(smask, alpha);
            sm.filter(Filter::FlateDecode);
            sm.width(w);
            sm.height(h);
            sm.color_space().device_gray();
            sm.bits_per_component(8);
            sm.finish();
        }
    }
}
