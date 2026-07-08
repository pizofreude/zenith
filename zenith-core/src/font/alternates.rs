//! OpenType alternate-selection authoring aliases.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontAlternateFeature {
    pub tag: String,
    pub value: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontAlternateParseError {
    pub spec: String,
    pub message: String,
}

pub fn parse_font_alternate_spec(
    spec: &str,
) -> Result<FontAlternateFeature, FontAlternateParseError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(FontAlternateParseError {
            spec: spec.to_owned(),
            message: "empty alternate selection".to_owned(),
        });
    }

    let (selector, value) = match spec.split_once('=') {
        Some((selector, raw_value)) => (selector.trim(), parse_value(spec, raw_value.trim())?),
        None => (spec, 1),
    };

    let selector = selector.to_ascii_lowercase();
    let tag = match selector.as_str() {
        "stylistic" => "salt".to_owned(),
        "swash" => "swsh".to_owned(),
        "contextual-swash" => "cswh".to_owned(),
        "historical" => "hist".to_owned(),
        "ornaments" => "ornm".to_owned(),
        "annotation" => "nalt".to_owned(),
        "randomize" => "rand".to_owned(),
        "titling" => "titl".to_owned(),
        _ => indexed_tag(spec, &selector)?,
    };

    Ok(FontAlternateFeature { tag, value })
}

fn parse_value(spec: &str, raw_value: &str) -> Result<u32, FontAlternateParseError> {
    raw_value
        .parse::<u32>()
        .map_err(|_| FontAlternateParseError {
            spec: spec.to_owned(),
            message: "alternate value must be a non-negative integer".to_owned(),
        })
}

fn indexed_tag(spec: &str, selector: &str) -> Result<String, FontAlternateParseError> {
    if let Some(index) = selector
        .strip_prefix("styleset(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let index = parse_index(spec, index, 20, "styleset")?;
        return Ok(format!("ss{index:02}"));
    }

    if let Some(index) = selector
        .strip_prefix("character-variant(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let index = parse_index(spec, index, 99, "character-variant")?;
        return Ok(format!("cv{index:02}"));
    }

    Err(FontAlternateParseError {
        spec: spec.to_owned(),
        message: "unknown alternate selector".to_owned(),
    })
}

fn parse_index(
    spec: &str,
    raw_index: &str,
    max: u32,
    selector: &str,
) -> Result<u32, FontAlternateParseError> {
    let index = raw_index
        .trim()
        .parse::<u32>()
        .map_err(|_| FontAlternateParseError {
            spec: spec.to_owned(),
            message: format!("{selector} index must be an integer"),
        })?;
    if index == 0 || index > max {
        return Err(FontAlternateParseError {
            spec: spec.to_owned(),
            message: format!("{selector} index must be in 1..={max}"),
        });
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_alternates_lower_to_opentype_tags() {
        let feature = parse_font_alternate_spec("stylistic").expect("valid alternate");
        assert_eq!(feature.tag, "salt");
        assert_eq!(feature.value, 1);

        let feature = parse_font_alternate_spec("swash=2").expect("valid alternate");
        assert_eq!(feature.tag, "swsh");
        assert_eq!(feature.value, 2);
    }

    #[test]
    fn indexed_alternates_lower_to_numbered_tags() {
        let feature = parse_font_alternate_spec("styleset(7)").expect("valid styleset");
        assert_eq!(feature.tag, "ss07");
        assert_eq!(feature.value, 1);

        let feature =
            parse_font_alternate_spec("character-variant(12)=3").expect("valid character variant");
        assert_eq!(feature.tag, "cv12");
        assert_eq!(feature.value, 3);
    }

    #[test]
    fn invalid_alternates_report_parse_errors() {
        assert!(parse_font_alternate_spec("styleset(0)").is_err());
        assert!(parse_font_alternate_spec("character-variant(100)").is_err());
        assert!(parse_font_alternate_spec("stylistic=on").is_err());
        assert!(parse_font_alternate_spec("unknown").is_err());
    }
}
