use super::*;

// ── connector validation ──────────────────────────────────────────────────────

#[test]
fn connector_unknown_target_warns() {
    // `to="ghost"` names no node id → connector.unknown_target.
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("ghost"),
                    route: None,
                    marker_end: None,
                    from_anchor: None,
                }),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "connector.unknown_target"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_valid_targets_do_not_warn_unknown() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                minimal_rect("b", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("b"),
                    route: None,
                    marker_end: None,
                    from_anchor: None,
                }),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "connector.unknown_target"),
        "valid from/to must not warn; codes: {:?}",
        codes(&report)
    );
    assert!(
        !has_code(&report, "connector.missing_target"),
        "both endpoints present must not warn missing; codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_missing_target_warns() {
    // `to` absent → connector.missing_target.
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: None,
                    route: None,
                    marker_end: None,
                    from_anchor: None,
                }),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "connector.missing_target"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_invalid_route_warns() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                minimal_rect("b", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("b"),
                    route: Some("zigzag"),
                    marker_end: None,
                    from_anchor: None,
                }),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "connector.invalid_route"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_valid_route_does_not_warn() {
    for route in ["straight", "orthogonal", "avoid"] {
        let doc = doc_with(
            vec![],
            vec![minimal_page(
                "page.one",
                vec![
                    minimal_rect("a", None),
                    minimal_rect("b", None),
                    make_connector(ConnectorSpec {
                        id: "c1",
                        from: Some("a"),
                        to: Some("b"),
                        route: Some(route),
                        marker_end: None,
                        from_anchor: None,
                    }),
                ],
            )],
        );
        let report = validate(&doc);
        assert!(
            !has_code(&report, "connector.invalid_route"),
            "route {route:?} must not warn; codes: {:?}",
            codes(&report)
        );
    }
}

#[test]
fn connector_invalid_marker_warns() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                minimal_rect("b", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("b"),
                    route: None,
                    marker_end: Some("diamond"),
                    from_anchor: None,
                }),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "connector.invalid_marker"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_invalid_anchor_warns() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                minimal_rect("b", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("b"),
                    route: None,
                    marker_end: None,
                    from_anchor: Some("sideways"),
                }),
            ],
        )],
    );
    let report = validate(&doc);
    assert!(
        has_code(&report, "connector.invalid_anchor"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_divided_anchors_are_valid() {
    let mut connector = make_connector(ConnectorSpec {
        id: "c1",
        from: Some("a"),
        to: Some("b"),
        route: None,
        marker_end: None,
        from_anchor: Some("35/60"),
    });
    if let Node::Connector(c) = &mut connector {
        c.to_anchor = Some("4/16".to_owned());
    }
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![minimal_rect("a", None), minimal_rect("b", None), connector],
        )],
    );
    let report = validate(&doc);
    assert!(
        !has_code(&report, "connector.invalid_anchor"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_divided_anchor_zero_count_errors() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                minimal_rect("b", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("b"),
                    route: None,
                    marker_end: None,
                    from_anchor: Some("0/0"),
                }),
            ],
        )],
    );
    let report = validate(&doc);
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "connector.anchor_division_zero")
        .unwrap_or_else(|| panic!("diagnostics: {:?}", report.diagnostics));
    assert!(diag.message.contains("count of 0"));
    assert_eq!(diag.severity, Severity::Error);
    // The zero-count failure must NOT be reported under the generic syntax code.
    assert!(!has_code(&report, "connector.invalid_anchor"));
}

#[test]
fn connector_divided_anchor_out_of_range_errors() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                minimal_rect("b", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("b"),
                    route: None,
                    marker_end: None,
                    from_anchor: Some("4/4"),
                }),
            ],
        )],
    );
    let report = validate(&doc);
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "connector.anchor_index_out_of_range")
        .unwrap_or_else(|| panic!("diagnostics: {:?}", report.diagnostics));
    assert!(diag.message.contains("outside divided anchor count"));
    assert_eq!(diag.severity, Severity::Error);
    // An out-of-range index is distinct from a zero division and from syntax.
    assert!(!has_code(&report, "connector.anchor_division_zero"));
    assert!(!has_code(&report, "connector.invalid_anchor"));
}

/// A pure-syntax anchor error (`sideways`) keeps the `connector.invalid_anchor`
/// code and is now an Error, distinct from the divided-anchor codes.
#[test]
fn connector_invalid_syntax_anchor_is_error() {
    let doc = doc_with(
        vec![],
        vec![minimal_page(
            "page.one",
            vec![
                minimal_rect("a", None),
                minimal_rect("b", None),
                make_connector(ConnectorSpec {
                    id: "c1",
                    from: Some("a"),
                    to: Some("b"),
                    route: None,
                    marker_end: None,
                    from_anchor: Some("sideways"),
                }),
            ],
        )],
    );
    let report = validate(&doc);
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "connector.invalid_anchor")
        .unwrap_or_else(|| panic!("diagnostics: {:?}", report.diagnostics));
    assert_eq!(diag.severity, Severity::Error);
    assert!(!has_code(&report, "connector.anchor_division_zero"));
    assert!(!has_code(&report, "connector.anchor_index_out_of_range"));
}
#[test]
fn connector_port_endpoints_are_valid() {
    let report = validate_source(
        r##"zenith version=1 {
  project id="proj.ports" name="Ports"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.ports" title="Ports" {
page id="page.ports" w=(px)640 h=(px)360 {
  ports {
    port node="agent" id="out" anchor="1/4"
    port node="store" id="in" anchor="3/4"
  }
  rect id="agent" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  rect id="store" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="agent#out" to="store#in"
}
  }
}
"##,
    );
    assert!(
        !has_code(&report, "connector.unknown_port")
            && !has_code(&report, "connector.port_invalid_target")
            && !has_code(&report, "connector.port_duplicate")
            && !has_code(&report, "connector.invalid_anchor"),
        "codes: {:?}",
        codes(&report)
    );
}

#[test]
fn connector_unknown_port_errors() {
    let report = validate_source(
        r##"zenith version=1 {
  project id="proj.ports" name="Ports"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.ports" title="Ports" {
page id="page.ports" w=(px)640 h=(px)360 {
  ports {
    port node="agent" id="out" anchor="1/4"
  }
  rect id="agent" x=(px)40 y=(px)40 w=(px)100 h=(px)80
  rect id="store" x=(px)300 y=(px)60 w=(px)100 h=(px)80
  connector id="c1" from="agent#missing" to="store"
}
  }
}
"##,
    );
    let diag = report
        .diagnostics
        .iter()
        .find(|d| d.code == "connector.unknown_port")
        .unwrap_or_else(|| panic!("codes: {:?}", codes(&report)));
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn duplicate_and_invalid_port_declarations_report() {
    let report = validate_source(
        r##"zenith version=1 {
  project id="proj.ports" name="Ports"
  tokens format="zenith-token-v1" {}
  styles {}
  document id="doc.ports" title="Ports" {
page id="page.ports" w=(px)640 h=(px)360 {
  ports {
    port node="agent" id="out" anchor="1/4"
    port node="agent" id="out" anchor="4/4"
    port node="ghost" id="in" anchor="0/4"
  }
  rect id="agent" x=(px)40 y=(px)40 w=(px)100 h=(px)80
}
  }
}
"##,
    );
    // Port `anchor="4/4"` has index 4 outside its count of 4 → the dedicated
    // `connector.anchor_index_out_of_range` code, NOT the generic syntax code.
    assert!(
        has_code(&report, "connector.port_duplicate")
            && has_code(&report, "connector.port_invalid_target")
            && has_code(&report, "connector.anchor_index_out_of_range"),
        "codes: {:?}",
        codes(&report)
    );
    assert!(!has_code(&report, "connector.invalid_anchor"));
}
