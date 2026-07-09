//! Import-source string parsing: `import#component.id` / `import#page.id`.

pub(super) enum ImportSource<'a> {
    Component {
        import_id: &'a str,
        component_id: &'a str,
    },
    Page {
        import_id: &'a str,
        page_id: &'a str,
    },
    UnsupportedTarget,
    Invalid,
}

pub(super) fn parse_import_source(source: &str) -> ImportSource<'_> {
    let Some((import_id, target)) = source.split_once('#') else {
        return ImportSource::Invalid;
    };
    if import_id.is_empty() || target.is_empty() || target.contains('#') {
        return ImportSource::Invalid;
    }

    if let Some(component_id) = target.strip_prefix("component.") {
        if component_id.is_empty() {
            return ImportSource::Invalid;
        }
        return ImportSource::Component {
            import_id,
            component_id,
        };
    }

    if let Some(page_id) = target.strip_prefix("page.") {
        if page_id.is_empty() {
            return ImportSource::Invalid;
        }
        return ImportSource::Page { import_id, page_id };
    }

    ImportSource::UnsupportedTarget
}
