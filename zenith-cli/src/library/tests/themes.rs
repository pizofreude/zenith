//! Embedded theme-pack tests.

use super::support::hard_errors;
use crate::library::{EMBEDDED_PACKS, PackSource, load_embedded_packs, parse_pack};
use zenith_core::{KdlAdapter, KdlSource};

const THEME_ID_PREFIX: &str = "@zenith/theme.";

const THEME_NAMES: [&str; 10] = [
    "cobalt", "ember", "harbor", "lagoon", "pine", "poppy", "prism", "sorbet", "sunset", "volt",
];

#[test]
fn embedded_theme_packs_parse_and_validate_clean() {
    for (id, src) in EMBEDDED_PACKS
        .iter()
        .filter(|(id, _)| id.starts_with(THEME_ID_PREFIX))
    {
        let doc = KdlAdapter
            .parse(src.as_bytes())
            .unwrap_or_else(|e| panic!("theme pack '{}' must parse: {}", id, e));
        let errors = hard_errors(&doc);
        assert!(
            errors.is_empty(),
            "theme pack '{}' must validate with no errors; got: {:?}",
            id,
            errors
        );

        let pack = parse_pack(src, PackSource::Preset)
            .unwrap_or_else(|e| panic!("theme pack '{}' must parse as a pack: {}", id, e));
        assert_eq!(pack.id, *id, "parsed pack id must match registry id");
        assert_eq!(
            pack.version.as_deref(),
            Some("1.0.0"),
            "theme pack '{}' must declare version 1.0.0",
            id
        );
        assert!(
            pack.items.is_empty(),
            "theme pack '{}' must export no items; got: {:?}",
            id,
            pack.items
        );
    }
}

#[test]
fn load_embedded_packs_includes_all_themes() {
    let packs = load_embedded_packs();
    for name in THEME_NAMES {
        let id = format!("{}{}", THEME_ID_PREFIX, name);
        assert!(
            packs.iter().any(|p| p.id == id),
            "embedded packs must include {}",
            id
        );
    }
}
