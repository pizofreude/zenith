use zenith_session::object_hash;

use crate::error::ProduceError;
use crate::model::{AssetProducer, ProduceRequest, ProducedAsset, Provenance};

#[derive(Debug, Clone, Copy, Default)]
pub struct FileImportProducer;

impl AssetProducer for FileImportProducer {
    fn produce(&self, req: ProduceRequest) -> Result<ProducedAsset, ProduceError> {
        match req {
            ProduceRequest::FileImport {
                kind,
                bytes,
                provenance,
            } => {
                let sha256 = object_hash(&bytes);
                Ok(ProducedAsset {
                    kind,
                    bytes,
                    sha256,
                    provenance: Provenance::FileImport(provenance),
                })
            }
            ProduceRequest::ZpxBake { .. } => Err(ProduceError::unsupported_request(
                "FileImportProducer",
                "ZpxBake",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use zenith_core::ast::AssetKind;

    use super::*;
    use crate::model::FileImportProvenance;

    #[test]
    fn import_preserves_kind_bytes_and_provenance_with_deterministic_hash() {
        let bytes = Arc::<[u8]>::from(&b"asset bytes"[..]);
        let provenance = FileImportProvenance::new("import:logo.png");
        let req = ProduceRequest::FileImport {
            kind: AssetKind::Image,
            bytes: Arc::clone(&bytes),
            provenance: provenance.clone(),
        };

        let produced = FileImportProducer.produce(req).expect("import succeeds");

        assert_eq!(produced.kind, AssetKind::Image);
        assert_eq!(produced.bytes, bytes);
        assert_eq!(produced.sha256, object_hash(&bytes));
        assert_eq!(produced.provenance, Provenance::FileImport(provenance));
    }

    #[test]
    fn import_rejects_zpx_bake_request() {
        let req = ProduceRequest::ZpxBake {
            doc: crate::zpx_bake::tests::solid_doc(),
        };

        let err = FileImportProducer
            .produce(req)
            .expect_err("wrong request must fail");

        assert_eq!(
            err,
            ProduceError::UnsupportedRequest {
                producer: "FileImportProducer",
                request: "ZpxBake",
            }
        );
    }
}
