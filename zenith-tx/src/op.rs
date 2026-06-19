//! Transaction envelope: [`Transaction`] and the [`Op`] enum.
//!
//! Deserializes from JSON like:
//! ```json
//! {"ops":[
//!   {"op":"set_text_align","node":"label","align":"center"},
//!   {"op":"set_fill","node":"box","fill":"color.accent"},
//!   {"op":"set_visible","node":"box","visible":false},
//!   {"op":"set_locked","node":"box","locked":true}
//! ]}
//! ```

use crate::TxError;

/// A batch of operations to apply to a document in order.
#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub ops: Vec<Op>,
}

impl Transaction {
    /// Parse a `Transaction` from a JSON string.
    pub fn from_json(s: &str) -> Result<Transaction, TxError> {
        serde_json::from_str(s).map_err(|e| TxError {
            message: format!("failed to parse transaction JSON: {e}"),
        })
    }
}

/// A single operation within a [`Transaction`].
///
/// The `op` field in JSON is the snake_case tag, e.g. `"set_text_align"`.
#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Set the `align` property on a text node.
    ///
    /// Valid values: `start`, `center`, `end`, `justify`.
    SetTextAlign {
        /// The stable node `id` to target.
        node: String,
        /// The new alignment value.
        align: String,
    },
    /// Move a node one sibling position toward the end (front/top of z-order).
    ///
    /// Has no effect if the node is already last in its parent's children.
    MoveForward {
        /// The stable node `id` to target.
        node: String,
    },
    /// Set the `fill` property on a node that supports fill.
    ///
    /// The `fill` value is a token id (e.g. `"color.accent"`); the engine
    /// wraps it as `PropertyValue::TokenRef(fill)`. Post-validation rejects
    /// unknown token ids automatically.
    ///
    /// Supported nodes: `rect`, `ellipse`, `text`, `polygon`, `polyline`.
    /// Unsupported: `line`, `frame`, `group`, `image` — yields
    /// `tx.unsupported_property`.
    SetFill {
        /// The stable node `id` to target.
        node: String,
        /// Token id to set as the fill (e.g. `"color.brand"`).
        fill: String,
    },
    /// Show or hide a node by setting its `visible` property.
    ///
    /// All known node variants except `Unknown` support this property.
    SetVisible {
        /// The stable node `id` to target.
        node: String,
        /// `false` hides the node; `true` makes it visible.
        visible: bool,
    },
    /// Lock or unlock a node by setting its `locked` property.
    ///
    /// All known node variants except `Unknown` support this property.
    SetLocked {
        /// The stable node `id` to target.
        node: String,
        /// `true` locks the node; `false` unlocks it.
        locked: bool,
    },
}
