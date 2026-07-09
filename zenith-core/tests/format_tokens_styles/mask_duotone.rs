use super::*;

/// **Mask round-trip**: a mask token (a `rounded` shape with `radius`,
/// `feather`, and `invert=#true`) must parse→format→parse byte-stably, emit the
/// shape brace block, and a rect node referencing it (via `mask=(token)"..."`)
/// must survive.
#[test]
fn test_mask_token_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.mask" name="Mask"
  tokens format="zenith-token-v1" {
    token id="mask.vignette" type="mask" {
      rounded radius=40 feather=60 invert=#true
    }
  }
  styles {
  }
  document id="doc.mask" title="Mask" {
    page id="p" w=(px)100 h=(px)100 {
      rect id="card" x=(px)0 y=(px)0 w=(px)100 h=(px)40 mask=(token)"mask.vignette"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");
    let s1 = format_document(&doc1).expect("format 1");
    let formatted = String::from_utf8(s1.clone()).expect("utf8");

    // The mask emits a brace block with a single shape child.
    assert!(
        formatted.contains("type=\"mask\" {"),
        "expected mask header; got:\n{formatted}"
    );
    assert!(
        formatted.contains("rounded radius=40 feather=60 invert=#true"),
        "expected rounded shape with radius/feather/invert; got:\n{formatted}"
    );
    assert!(
        formatted.contains(" mask=(token)\"mask.vignette\""),
        "expected node mask prop; got:\n{formatted}"
    );

    // Idempotency.
    let doc2 = adapter.parse(&s1).expect("parse 2");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted,
        String::from_utf8(s2).expect("utf8"),
        "mask formatting must be idempotent"
    );

    // AST round-trip (spans stripped).
    assert_eq!(
        strip_spans(doc1),
        strip_spans(doc2),
        "mask AST must survive format round-trip"
    );
}

/// **Mask prop wrong type**: a node `mask=(token)"x"` where `x` is a color token
/// must produce `token.incompatible_property`.
#[test]
fn test_mask_node_prop_wrong_type() {
    let src = r##"zenith version=1 {
  project id="proj.mask" name="Mask"
  tokens format="zenith-token-v1" {
    token id="color.not-a-mask" type="color" value="#000000"
  }
  styles {
  }
  document id="doc.mask" title="Mask" {
    page id="p" w=(px)100 h=(px)100 {
      rect id="card" x=(px)0 y=(px)0 w=(px)100 h=(px)40 mask=(token)"color.not-a-mask"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    let report = zenith_core::validate(&doc);

    let codes: Vec<&str> = report.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert!(
        codes.contains(&"token.incompatible_property"),
        "a non-mask token in a mask slot must be incompatible; codes: {codes:?}"
    );
}

/// **Duotone filter round-trip**: a duotone op carrying both `shadow` and
/// `highlight` color-token refs (plus `amount`) must parse→format→parse
/// byte-stably and emit `duotone shadow=(token)"…" highlight=(token)"…" amount=…`.
#[test]
fn test_duotone_filter_token_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.duo" name="Duo"
  tokens format="zenith-token-v1" {
    token id="color.sh" type="color" value="#000000"
    token id="color.hi" type="color" value="#ffffff"
    token id="filter.duo" type="filter" {
      duotone shadow=(token)"color.sh" highlight=(token)"color.hi" amount=0.8
    }
  }
  styles {
  }
  document id="doc.duo" title="Duo" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 filter=(token)"filter.duo" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");
    let s1 = format_document(&doc1).expect("format 1");
    let formatted = String::from_utf8(s1.clone()).expect("utf8");

    assert!(
        formatted.contains(
            "duotone shadow=(token)\"color.sh\" highlight=(token)\"color.hi\" amount=0.8"
        ),
        "expected duotone op with both colors and amount; got:\n{formatted}"
    );

    // Idempotency.
    let doc2 = adapter.parse(&s1).expect("parse 2");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted,
        String::from_utf8(s2).expect("utf8"),
        "duotone formatting must be idempotent"
    );

    // AST round-trip (spans stripped).
    assert_eq!(
        strip_spans(doc1),
        strip_spans(doc2),
        "duotone AST must survive format round-trip"
    );
}

/// **Noise filter round-trip**: a noise op carrying `seed`, `scale`, and
/// `amount` must parse→format→parse byte-stably and emit those props by name in
/// the canonical `seed`/`scale`/`amount` order.
#[test]
fn test_noise_filter_token_round_trip() {
    let src = r##"zenith version=1 {
  project id="proj.noise" name="Noise"
  tokens format="zenith-token-v1" {
    token id="filter.grain" type="filter" {
      noise seed=7 scale=2 amount=0.3
    }
  }
  styles {
  }
  document id="doc.noise" title="Noise" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 filter=(token)"filter.grain" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc1 = adapter.parse(src.as_bytes()).expect("parse 1");
    let s1 = format_document(&doc1).expect("format 1");
    let formatted = String::from_utf8(s1.clone()).expect("utf8");

    assert!(
        formatted.contains("noise seed=7 scale=2 amount=0.3"),
        "expected noise op with seed/scale/amount; got:\n{formatted}"
    );

    // Idempotency.
    let doc2 = adapter.parse(&s1).expect("parse 2");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        formatted,
        String::from_utf8(s2).expect("utf8"),
        "noise formatting must be idempotent"
    );

    // AST round-trip (spans stripped).
    assert_eq!(
        strip_spans(doc1),
        strip_spans(doc2),
        "noise AST must survive format round-trip"
    );
}

/// **Duotone color refs are used transitively**: a node referencing a duotone
/// filter token records the duotone's shadow/highlight color tokens as used, so
/// neither is falsely flagged `token.unused`.
#[test]
fn test_duotone_color_refs_not_unused() {
    let src = r##"zenith version=1 {
  project id="proj.duo" name="Duo"
  tokens format="zenith-token-v1" {
    token id="color.sh" type="color" value="#000000"
    token id="color.hi" type="color" value="#ffffff"
    token id="filter.duo" type="filter" {
      duotone shadow=(token)"color.sh" highlight=(token)"color.hi"
    }
  }
  styles {
  }
  document id="doc.duo" title="Duo" {
    page id="p" w=(px)100 h=(px)100 {
      text id="headline" x=(px)0 y=(px)0 w=(px)100 h=(px)40 filter=(token)"filter.duo" {
        span "Hi"
      }
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    let report = zenith_core::validate(&doc);

    let unused: Vec<&str> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "token.unused")
        .filter_map(|d| d.subject_id.as_deref())
        .collect();
    assert!(
        !unused.contains(&"color.sh") && !unused.contains(&"color.hi"),
        "duotone color tokens must be recorded as used; unused: {unused:?}"
    );
}
