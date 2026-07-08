//! Integration tests for page-scoped construction guide validation.

use zenith_core::{KdlAdapter, KdlSource, validate};

fn codes(src: &str) -> Vec<String> {
    let doc = KdlAdapter.parse(src.as_bytes()).expect("parse");
    validate(&doc)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn valid_construction_guides_are_clean() {
    let report_codes = codes(
        r##"zenith version=1 {
  document id="doc.guides" {
    page id="page.guides" w=(px)400 h=(px)300 {
      construction {
        guide id="axis.x" type="segment" x1=(px)0 y1=(px)150 x2=(px)400 y2=(px)150
        guide id="ring.outer" type="circle" cx=(px)200 cy=(px)150 r=(px)120
      }
    }
  }
}
"##,
    );

    assert!(
        report_codes
            .iter()
            .all(|code| !code.starts_with("construction.")),
        "valid guides must not emit construction diagnostics; got {report_codes:?}"
    );
}

#[test]
fn invalid_construction_guides_emit_precise_warnings() {
    let report_codes = codes(
        r##"zenith version=1 {
  document id="doc.guides" {
    page id="page.guides" w=(px)400 h=(px)300 {
      construction {
        guide id="axis.bad" type="segment" x1=(px)10 y1=(px)10 x2=(px)10 y2=(px)10
        guide id="ring.bad" type="circle" cx=(px)200 cy=(px)150 r=(px)0
        guide id="unknown.bad" type="spiral"
        guide id="missing.bad" type="segment" x1=(px)0 y1=(px)0
      }
    }
  }
}
"##,
    );

    assert!(report_codes.contains(&"construction.degenerate_guide".to_owned()));
    assert!(report_codes.contains(&"construction.invalid_radius".to_owned()));
    assert!(report_codes.contains(&"construction.unknown_guide_type".to_owned()));
    assert!(report_codes.contains(&"construction.missing_geometry".to_owned()));
}
