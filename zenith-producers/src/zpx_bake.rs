use std::sync::Arc;

use zenith_core::ast::AssetKind;

use crate::error::ProduceError;
use crate::model::{AssetProducer, ProduceRequest, ProducedAsset, Provenance, ZpxBakeProvenance};

#[derive(Debug, Clone, Copy, Default)]
pub struct ZpxBakeProducer;

impl AssetProducer for ZpxBakeProducer {
    fn produce(&self, req: ProduceRequest) -> Result<ProducedAsset, ProduceError> {
        match req {
            ProduceRequest::ZpxBake { doc } => {
                let output = zenith_zpx::bake(&doc)?;
                Ok(ProducedAsset {
                    kind: AssetKind::Image,
                    bytes: Arc::from(output.png),
                    sha256: output.png_sha256.as_str().to_owned(),
                    provenance: Provenance::ZpxBake(ZpxBakeProvenance {
                        source_sha256: output.provenance.source_sha256.as_str().to_owned(),
                    }),
                })
            }
            ProduceRequest::FileImport { .. } => Err(ProduceError::unsupported_request(
                "ZpxBakeProducer",
                "FileImport",
            )),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use zenith_core::{BlendMode, Color};
    use zenith_zpx::{
        AlphaMode, Brush, Canvas, ColorSpace, DabSample, Layer, LayerSource, Stroke, StrokeProgram,
        ZpxDoc,
    };

    use super::*;
    use crate::model::FileImportProvenance;

    pub(crate) fn solid_doc() -> ZpxDoc {
        ZpxDoc {
            canvas: Canvas {
                width_px: 4,
                height_px: 4,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Premultiplied,
            },
            layers: vec![Layer {
                id: "paint".to_owned(),
                blend_mode: BlendMode::Normal,
                opacity: 1.0,
                visible: true,
                clipping: false,
                mask: None,
                source: LayerSource::Program(StrokeProgram {
                    strokes: vec![Stroke {
                        brush: Brush::Round {
                            radius_px: 3.0,
                            hardness: 1.0,
                            spacing: 1.0,
                        },
                        path: vec![DabSample {
                            x: 2.0,
                            y: 2.0,
                            pressure: 1.0,
                        }],
                        color: Color::srgb(255, 0, 0, 255),
                        opacity: 1.0,
                        blend_mode: BlendMode::Normal,
                        seed: 7,
                    }],
                }),
            }],
        }
    }

    #[test]
    fn zpx_bake_hash_and_provenance_match_direct_bake_output() {
        let doc = solid_doc();
        let direct = zenith_zpx::bake(&doc).expect("direct bake succeeds");
        let req = ProduceRequest::ZpxBake { doc };

        let produced = ZpxBakeProducer
            .produce(req)
            .expect("producer bake succeeds");

        assert_eq!(produced.kind, AssetKind::Image);
        assert_eq!(produced.bytes.as_ref(), direct.png.as_slice());
        assert_eq!(produced.sha256, direct.png_sha256.as_str());
        assert_eq!(
            produced.provenance,
            Provenance::ZpxBake(ZpxBakeProvenance {
                source_sha256: direct.provenance.source_sha256.as_str().to_owned(),
            })
        );
    }

    #[test]
    fn zpx_bake_rejects_file_import_request() {
        let req = ProduceRequest::FileImport {
            kind: AssetKind::Image,
            bytes: Arc::<[u8]>::from(&b"not zpx"[..]),
            provenance: FileImportProvenance::new("import:image.png"),
        };

        let err = ZpxBakeProducer
            .produce(req)
            .expect_err("wrong request must fail");

        assert_eq!(
            err,
            ProduceError::UnsupportedRequest {
                producer: "ZpxBakeProducer",
                request: "FileImport",
            }
        );
    }
}
