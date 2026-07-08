//! The [`Transaction`] envelope: an ordered batch of [`super::Op`]s plus
//! per-transaction [`super::Permissions`].

use crate::TxError;

use super::ops::Op;
use super::types::Permissions;

/// A batch of operations to apply to a document in order.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction {
    pub ops: Vec<Op>,
    /// Permission flags relaxing per-op guards. Defaults to all-`false`
    /// (every guard active) when the `permissions` key is absent from JSON.
    #[serde(default)]
    pub permissions: Permissions,
}

impl Transaction {
    /// Parse a `Transaction` from a JSON string.
    pub fn from_json(s: &str) -> Result<Transaction, TxError> {
        serde_json::from_str(s).map_err(|e| TxError {
            message: format!("failed to parse transaction JSON: {e}"),
        })
    }
}
