use super::*;

// ── Token `set` provenance: canonical position + round-trip idempotency ──────

const WITH_TOKEN_SET: &str = r##"zenith version=1 {
  project id="proj.set" name="Set"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" set="@zenith/theme.cobalt" value="#ffffff"
    token id="color.b" type="color" value="#000000"
  }
  styles {
  }
  document id="doc.set" title="Set" {
    page id="page.set" w=(px)100 h=(px)100 {
      rect id="r" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"color.a"
    }
  }
}
"##;

/// A token's `set=` attribute is emitted immediately after `type=` (canonical
/// order: id, type, set, value) and only when present; a token with no `set`
/// emits no `set=` at all.
#[test]
fn token_set_emitted_in_canonical_position_and_only_when_present() {
    let adapter = KdlAdapter;
    let doc = adapter.parse(WITH_TOKEN_SET.as_bytes()).expect("parse");
    let formatted = format_document(&doc).expect("format");
    let formatted_str = String::from_utf8(formatted).expect("utf8");

    assert!(
        formatted_str.contains(
            "token id=\"color.a\" type=\"color\" set=\"@zenith/theme.cobalt\" value=\"#ffffff\""
        ),
        "set must be emitted right after type, before value; got:\n{formatted_str}"
    );
    assert!(
        formatted_str.contains("token id=\"color.b\" type=\"color\" value=\"#000000\""),
        "a token with no set must emit no set= at all; got:\n{formatted_str}"
    );
    assert!(
        !formatted_str.contains("color.b\" type=\"color\" set="),
        "color.b must not gain a spurious set=; got:\n{formatted_str}"
    );
}

/// A token's `set` value survives parse → format → parse unchanged.
#[test]
fn token_set_round_trips() {
    let adapter = KdlAdapter;
    let doc = adapter.parse(WITH_TOKEN_SET.as_bytes()).expect("parse");
    let formatted = format_document(&doc).expect("format");
    let reparsed = adapter.parse(&formatted).expect("re-parse after format");

    let a = reparsed
        .tokens
        .tokens
        .iter()
        .find(|t| t.id == "color.a")
        .expect("color.a must survive round-trip");
    assert_eq!(a.set.as_deref(), Some("@zenith/theme.cobalt"));

    let b = reparsed
        .tokens
        .tokens
        .iter()
        .find(|t| t.id == "color.b")
        .expect("color.b must survive round-trip");
    assert_eq!(b.set, None);
}

/// Format twice → identical bytes (idempotency) for a document using `set`.
#[test]
fn token_set_format_idempotent() {
    let adapter = KdlAdapter;
    let doc = adapter.parse(WITH_TOKEN_SET.as_bytes()).expect("parse");
    let s1 = format_document(&doc).expect("format 1");
    let doc2 = adapter.parse(&s1).expect("parse after first format");
    let s2 = format_document(&doc2).expect("format 2");
    assert_eq!(
        String::from_utf8(s1).unwrap(),
        String::from_utf8(s2).unwrap(),
        "set= format must be idempotent"
    );
}

/// A document with no `set` attrs at all validates/formats byte-identically
/// to before `set` existed — the additive invariant.
#[test]
fn no_set_attrs_format_unchanged() {
    let src = r##"zenith version=1 {
  project id="proj.noset" name="NoSet"
  tokens format="zenith-token-v1" {
    token id="color.a" type="color" value="#ffffff"
    token id="color.b" type="color" value="#000000"
  }
  styles {
  }
  document id="doc.noset" title="NoSet" {
    page id="page.noset" w=(px)100 h=(px)100 {
      rect id="r" x=(px)0 y=(px)0 w=(px)100 h=(px)100 fill=(token)"color.a"
    }
  }
}
"##;
    let adapter = KdlAdapter;
    let doc = adapter.parse(src.as_bytes()).expect("parse");
    let formatted = String::from_utf8(format_document(&doc).expect("format")).expect("utf8");
    assert!(
        !formatted.contains(" set="),
        "a document with no set attrs must never gain one; got:\n{formatted}"
    );
}
