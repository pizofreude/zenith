//! Token block and token AST types.

use super::Span;
use super::value::Dimension;

/// The five v0 token types, plus an extensibility variant for unknown types.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Color,
    Dimension,
    Number,
    FontFamily,
    FontWeight,
    /// An unrecognized token type (forward-compat; version-relative).
    Unknown(String),
}

impl TokenType {
    /// Parse the token type from the `type` property string. Infallible: an
    /// unrecognized type is preserved as `TokenType::Unknown` (forward-compat).
    pub fn from_type_name(s: &str) -> Self {
        match s {
            "color" => Self::Color,
            "dimension" => Self::Dimension,
            "number" => Self::Number,
            "fontFamily" => Self::FontFamily,
            "fontWeight" => Self::FontWeight,
            other => Self::Unknown(other.to_owned()),
        }
    }
}

/// A literal value held by a token definition.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenLiteral {
    /// A quoted string, e.g. `"#f8fafc"` or `"Inter"`.
    String(String),
    /// A dimensioned number, e.g. `(pt)48` or `(px)28`.
    Dimension(Dimension),
    /// An unannotated finite number, e.g. `1.05` or `700`.
    Number(f64),
}

/// The value of a token — either an inline literal or an alias to another token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    /// A literal token value.
    Literal(TokenLiteral),
    /// An alias to another token, e.g. `(token)"color.text.primary"`.
    Reference { token_id: String },
}

/// A single design token within a `tokens` block.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Globally unique token ID.
    pub id: String,
    /// The token's declared type.
    pub token_type: TokenType,
    /// The token's declared value (literal or reference).
    pub value: TokenValue,
    /// Source declaration span, when available.
    pub source_span: Option<Span>,
}

/// The top-level `tokens` block with its required `format` attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenBlock {
    /// Must be `"zenith-token-v1"` in v0.
    pub format: String,
    /// The ordered list of token definitions.
    pub tokens: Vec<Token>,
}

impl Default for TokenBlock {
    fn default() -> Self {
        Self {
            format: "zenith-token-v1".to_owned(),
            tokens: Vec::new(),
        }
    }
}
