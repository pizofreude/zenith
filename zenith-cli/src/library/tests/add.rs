//! `sanitize_pkg` / `target_component_id` / `parse_spec` tests.

use crate::library::add::{sanitize_pkg, target_component_id};
use crate::library::parse_spec;

#[test]
fn sanitize_pkg_strips_at_and_slash() {
    assert_eq!(sanitize_pkg("@zenith/flowchart"), "zenith.flowchart");
    assert_eq!(
        target_component_id("@zenith/flowchart", "decision"),
        "lib.zenith.flowchart.decision"
    );
}

#[test]
fn parse_spec_splits_pkg_and_item() {
    assert_eq!(
        parse_spec("@zenith/flowchart#decision").expect("ok"),
        ("@zenith/flowchart".to_owned(), "decision".to_owned())
    );
}

#[test]
fn parse_spec_rejects_malformed() {
    assert!(parse_spec("no-hash").is_err());
    assert!(parse_spec("#item").is_err());
    assert!(parse_spec("pkg#").is_err());
}
