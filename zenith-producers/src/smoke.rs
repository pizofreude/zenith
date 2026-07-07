use std::sync::Arc;

use zenith_core::ast::AssetKind;
use zenith_session::object_hash;

use crate::{AssetProducer, FileImportProducer, FileImportProvenance, ProduceRequest, Provenance};

#[test]
fn public_api_smoke_path_imports_asset() {
    let bytes = Arc::<[u8]>::from(&b"public api"[..]);
    let req = ProduceRequest::FileImport {
        kind: AssetKind::Svg,
        bytes: Arc::clone(&bytes),
        provenance: FileImportProvenance::new("import:mark.svg"),
    };

    let produced = FileImportProducer
        .produce(req)
        .expect("public API import succeeds");

    assert_eq!(produced.kind, AssetKind::Svg);
    assert_eq!(produced.bytes, bytes);
    assert_eq!(produced.sha256, object_hash(&bytes));
    assert_eq!(
        produced.provenance,
        Provenance::FileImport(FileImportProvenance::new("import:mark.svg"))
    );
}
