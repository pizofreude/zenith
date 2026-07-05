//! `materialize_action` tests.

use super::support::hard_errors;
use crate::library::{LibraryPack, PackSource, materialize_action, parse_pack, resolve_packs};
use zenith_core::{KdlAdapter, KdlSource};
use zenith_tx::TxStatus;

/// A target doc that declares the `color.brand` token the action touches.
const ACTION_TARGET_SRC: &str = r##"zenith version=1 {
  project id="proj.x" name="Target"
  tokens format="zenith-token-v1" {
    token id="color.brand" type="color" value="#111111"
  }
  styles {}
  document id="d" title="x" {
    page id="pg" w=(px)800 h=(px)600 {}
  }
}
"##;

/// A minimal action pack that updates a single color token `color.brand`.
const ACTION_PACK_SRC_UPDATE: &str = r##"zenith version=1 {
  project id="@test/brandkit" name="Brand Kit"
  libraries { library id="@test/brandkit" version="2.0.0" }
  actions {
    action id="apply-brand-color" label="Apply Brand Color" {
      tx "{\"ops\":[{\"op\":\"update_token_value\",\"id\":\"color.brand\",\"value\":\"#e11d48\"}]}"
    }
  }
  document id="d" title="x" {
    page id="pg" w=(px)100 h=(px)100 {
    }
  }
}
"##;

/// Build a project-backed [`LibraryPack`] from inline `.zen` source by
/// writing it to a temp file, so [`crate::library::load_pack_document`] can
/// re-read it. The returned [`tempfile::TempDir`] must be kept alive for the
/// duration of the test (dropping it deletes the backing file).
fn pack_from_src(src: &str) -> (tempfile::TempDir, LibraryPack) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pack.zen");
    std::fs::write(&path, src).expect("write pack");
    let pack = parse_pack(src, PackSource::Project(path)).expect("pack parses");
    (dir, pack)
}

#[test]
fn embedded_brand_kit_action_applies_via_materialize_action() {
    const TARGET: &str = r##"zenith version=1 {
  project id="proj.x" name="Target"
  tokens format="zenith-token-v1" {
    token id="color.brand" type="color" value="#000000"
    token id="color.accent" type="color" value="#000000"
    token id="color.ink" type="color" value="#000000"
  }
  styles {}
  document id="d" title="x" {
    page id="pg" w=(px)400 h=(px)300 {}
  }
}
"##;
    let packs = resolve_packs(None);
    let outcome = materialize_action(TARGET, &packs, "@zenith/brand-kit", "apply-2026")
        .expect("materialize_action ok");
    let final_src = outcome.final_source.expect("accepted → final_source");
    // The 2026 palette is applied and the action is recorded with provenance.
    assert!(
        final_src.contains("#e11d48"),
        "brand color applied:\n{}",
        final_src
    );
    assert!(final_src.contains("#3b82f6"), "accent color applied");
    assert!(final_src.contains("apply-2026"), "action recorded in doc");
    assert!(
        outcome.provenance_id.is_some(),
        "provenance recorded for the applied action"
    );
}

#[test]
fn materialize_action_accepted_updates_token_records_action_library_provenance() {
    let (_dir, pack) = pack_from_src(ACTION_PACK_SRC_UPDATE);
    let packs = vec![pack];
    let outcome = materialize_action(
        ACTION_TARGET_SRC,
        &packs,
        "@test/brandkit",
        "apply-brand-color",
    )
    .expect("materialize_action ok");

    // Status is Accepted or AcceptedWithWarnings.
    assert!(
        matches!(
            outcome.tx_result.status,
            TxStatus::Accepted | TxStatus::AcceptedWithWarnings
        ),
        "expected Accepted/AcceptedWithWarnings, got {:?}",
        outcome.tx_result.status
    );

    let final_src = outcome
        .final_source
        .expect("final_source must be Some on Accepted");
    let provenance_id = outcome.provenance_id.expect("provenance_id must be Some");

    // The updated token value is present in the output.
    assert!(
        final_src.contains("#e11d48"),
        "updated value must appear in final_source; got:\n{}",
        final_src
    );

    // An `actions` block with the action id is present.
    assert!(
        final_src.contains("apply-brand-color"),
        "action id must appear in final_source; got:\n{}",
        final_src
    );

    // A libraries import for the pack is present.
    assert!(
        final_src.contains("@test/brandkit"),
        "library import must appear in final_source; got:\n{}",
        final_src
    );

    // A provenance record referencing the action id is present.
    assert!(
        final_src.contains(&provenance_id),
        "provenance id must appear in final_source"
    );

    // Re-parse and validate the final source — must have no hard errors.
    let reparsed = KdlAdapter
        .parse(final_src.as_bytes())
        .expect("final_source must re-parse");
    assert!(
        hard_errors(&reparsed).is_empty(),
        "final_source must validate clean; errors: {:?}",
        hard_errors(&reparsed)
    );

    // Confirm the action + library + provenance appear in the parsed tree.
    assert!(
        reparsed.actions.iter().any(|a| a.id == "apply-brand-color"),
        "action must be in parsed actions"
    );
    assert!(
        reparsed.libraries.iter().any(|l| l.id == "@test/brandkit"),
        "library must be in parsed libraries"
    );
    assert!(
        reparsed
            .provenance
            .iter()
            .any(|p| p.node == "apply-brand-color"),
        "provenance node must be the action id"
    );

    // No warnings on a clean apply.
    assert!(
        outcome.warnings.is_empty(),
        "unexpected warnings: {:?}",
        outcome.warnings
    );
}

#[test]
fn materialize_action_rejected_when_token_not_found() {
    /// A pack that references a non-existent token id.
    const REJECT_PACK_SRC: &str = r##"zenith version=1 {
  project id="@test/reject" name="Reject Test"
  libraries { library id="@test/reject" version="1.0.0" }
  actions {
    action id="no-such-token" {
      tx "{\"ops\":[{\"op\":\"update_token_value\",\"id\":\"does.not.exist\",\"value\":\"#fff\"}]}"
    }
  }
  document id="d" title="x" {
    page id="pg" w=(px)100 h=(px)100 {}
  }
}
"##;
    let (_dir, pack) = pack_from_src(REJECT_PACK_SRC);
    let packs = vec![pack];
    let outcome = materialize_action(ACTION_TARGET_SRC, &packs, "@test/reject", "no-such-token")
        .expect("materialize_action itself must succeed (rejected tx is not an Err)");

    assert_eq!(
        outcome.tx_result.status,
        TxStatus::Rejected,
        "tx must be Rejected"
    );
    assert!(
        outcome.final_source.is_none(),
        "final_source must be None on Rejected"
    );
    assert!(
        outcome.provenance_id.is_none(),
        "provenance_id must be None on Rejected"
    );
}

#[test]
fn materialize_action_unknown_pkg_errors_with_available() {
    let (_dir, pack) = pack_from_src(ACTION_PACK_SRC_UPDATE);
    let packs = vec![pack];
    let err = materialize_action(ACTION_TARGET_SRC, &packs, "@no/such", "apply-brand-color")
        .expect_err("unknown pkg errors");
    assert!(
        err.message.contains("unknown library package"),
        "msg: {}",
        err.message
    );
    assert!(
        err.message.contains("@test/brandkit"),
        "must list available packages; msg: {}",
        err.message
    );
}

#[test]
fn materialize_action_unknown_action_errors_with_available() {
    let (_dir, pack) = pack_from_src(ACTION_PACK_SRC_UPDATE);
    let packs = vec![pack];
    let err = materialize_action(
        ACTION_TARGET_SRC,
        &packs,
        "@test/brandkit",
        "no-such-action",
    )
    .expect_err("unknown action errors");
    assert!(
        err.message.contains("unknown action item"),
        "msg: {}",
        err.message
    );
    assert!(
        err.message.contains("apply-brand-color"),
        "must list available actions; msg: {}",
        err.message
    );
}

#[test]
fn materialize_action_malformed_tx_json_errors() {
    const MALFORMED_PACK_SRC: &str = r#"zenith version=1 {
  project id="@test/malformed" name="Malformed"
  libraries { library id="@test/malformed" version="1.0.0" }
  actions {
    action id="bad-action" {
      tx "not valid json"
    }
  }
  document id="d" title="x" {
    page id="pg" w=(px)100 h=(px)100 {}
  }
}
"#;
    let (_dir, pack) = pack_from_src(MALFORMED_PACK_SRC);
    let packs = vec![pack];
    let err = materialize_action(ACTION_TARGET_SRC, &packs, "@test/malformed", "bad-action")
        .expect_err("malformed tx_json must error");
    assert!(
        err.message.contains("malformed tx-script"),
        "msg: {}",
        err.message
    );
    assert!(
        err.message.contains("bad-action"),
        "error must name the action; msg: {}",
        err.message
    );
}
