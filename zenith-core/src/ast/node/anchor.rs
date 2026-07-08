//! Page-relative 9-point anchor and adjacent-edge placement: the [`Anchor`] and
//! [`AnchorEdge`] enums and their derivation helpers.

/// One of the nine named page-relative placement anchors.
///
/// An anchor supplies BOTH the x and y of a node from the page dimensions;
/// an explicitly-authored `x` or `y` on the node overrides the corresponding
/// anchor-derived coordinate. The node's `w` and `h` must be present and in a
/// px-convertible unit for derivation to succeed; when they are absent or
/// use a non-px unit the anchor is silently skipped (the node remains incomplete
/// and the compile step emits the usual `scene.missing_geometry` advisory).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Parse a string into an [`Anchor`] variant.
///
/// Returns `Some` for the nine recognized names; `None` for any other string.
/// This is the SINGLE source of truth for the anchor name list — both the
/// validator (`anchor.unknown_value`) and the scene pre-pass use this function
/// so the names cannot diverge.
pub fn parse_anchor(s: &str) -> Option<Anchor> {
    match s {
        "top-left" => Some(Anchor::TopLeft),
        "top-center" => Some(Anchor::TopCenter),
        "top-right" => Some(Anchor::TopRight),
        "center-left" => Some(Anchor::CenterLeft),
        "center" => Some(Anchor::Center),
        "center-right" => Some(Anchor::CenterRight),
        "bottom-left" => Some(Anchor::BottomLeft),
        "bottom-center" => Some(Anchor::BottomCenter),
        "bottom-right" => Some(Anchor::BottomRight),
        _ => None,
    }
}

/// One of the four adjacent-edge directions for `anchor-edge` placement.
///
/// When a node carries both `anchor-sibling` and `anchor-edge`, the node is
/// placed flush against the named edge of the sibling rather than within the
/// sibling's box. An optional 9-point `anchor` on the same node controls
/// cross-axis alignment (e.g. centering horizontally when placed below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorEdge {
    Above,
    Below,
    Before,
    After,
}

/// Parse a string into an [`AnchorEdge`] variant.
///
/// Returns `Some` for the four recognized names; `None` for any other string.
/// This is the SINGLE source of truth for the anchor-edge name list — both the
/// validator (`anchor.unknown_edge`) and the scene pre-pass use this function
/// so the names cannot diverge.
pub fn parse_anchor_edge(s: &str) -> Option<AnchorEdge> {
    match s {
        "above" => Some(AnchorEdge::Above),
        "below" => Some(AnchorEdge::Below),
        "before" => Some(AnchorEdge::Before),
        "after" => Some(AnchorEdge::After),
        _ => None,
    }
}

/// Parsed connector endpoint anchor.
///
/// Connector anchor attributes remain authored as strings in the AST. This
/// helper gives validation and scene compilation one shared syntax contract for
/// the accepted forms:
/// - `auto`
/// - existing named/grid anchors (`top`, `bottom-right`, `mid-right`, ...)
/// - divided perimeter anchors (`i/N`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorAnchor {
    Auto,
    Grid,
    Divided { index: usize, count: usize },
}

/// Why a connector anchor string failed to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorAnchorParseError {
    InvalidSyntax,
    ZeroCount,
    IndexOutOfRange { index: usize, count: usize },
}

/// Parse a connector endpoint anchor.
///
/// Divided anchors use `i/N` syntax and require `0 <= i < N`; `N=0` is
/// rejected. Existing grid/named connector anchors are accepted exactly as they
/// were before, including `centre`/`mid`/`middle` center synonyms.
pub fn parse_connector_anchor(s: &str) -> Result<ConnectorAnchor, ConnectorAnchorParseError> {
    if s == "auto" {
        return Ok(ConnectorAnchor::Auto);
    }
    if let Some((index, count)) = s.split_once('/') {
        if index.is_empty() || count.is_empty() || count.contains('/') {
            return Err(ConnectorAnchorParseError::InvalidSyntax);
        }
        let Ok(index) = index.parse::<usize>() else {
            return Err(ConnectorAnchorParseError::InvalidSyntax);
        };
        let Ok(count) = count.parse::<usize>() else {
            return Err(ConnectorAnchorParseError::InvalidSyntax);
        };
        if count == 0 {
            return Err(ConnectorAnchorParseError::ZeroCount);
        }
        if index >= count {
            return Err(ConnectorAnchorParseError::IndexOutOfRange { index, count });
        }
        return Ok(ConnectorAnchor::Divided { index, count });
    }
    if is_connector_grid_anchor(s) {
        return Ok(ConnectorAnchor::Grid);
    }
    Err(ConnectorAnchorParseError::InvalidSyntax)
}

fn is_connector_grid_anchor(s: &str) -> bool {
    let mut recognized = false;
    for part in s.split('-') {
        match part {
            "top" | "bottom" | "left" | "right" | "center" | "centre" | "mid" | "middle" => {
                recognized = true;
            }
            _ => return false,
        }
    }
    recognized
}

/// Derive the `(x, y)` for the given anchor given the page and node dimensions.
///
/// `page_w`/`page_h` and `node_w`/`node_h` are all in pixels.
pub fn anchor_xy(anchor: Anchor, page_w: f64, page_h: f64, node_w: f64, node_h: f64) -> (f64, f64) {
    match anchor {
        Anchor::TopLeft => (0.0, 0.0),
        Anchor::TopCenter => ((page_w - node_w) / 2.0, 0.0),
        Anchor::TopRight => (page_w - node_w, 0.0),
        Anchor::CenterLeft => (0.0, (page_h - node_h) / 2.0),
        Anchor::Center => ((page_w - node_w) / 2.0, (page_h - node_h) / 2.0),
        Anchor::CenterRight => (page_w - node_w, (page_h - node_h) / 2.0),
        Anchor::BottomLeft => (0.0, page_h - node_h),
        Anchor::BottomCenter => ((page_w - node_w) / 2.0, page_h - node_h),
        Anchor::BottomRight => (page_w - node_w, page_h - node_h),
    }
}
