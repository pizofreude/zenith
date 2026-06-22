//! The MCP tool catalog: names, descriptions, and JSON-Schema input shapes.
//!
//! Each tool maps one-to-one onto a `zenith` CLI command and reuses the exact
//! same `commands::*` logic via [`super::exec`]. Schemas are plain
//! `serde_json` values so no schema-derivation dependency is needed.

use serde_json::{Value, json};

/// A single MCP tool definition.
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: Value,
}

/// The full tool catalog (read + write surface).
pub fn catalog() -> Vec<Tool> {
    vec![
        Tool {
            name: "zenith_validate",
            description: "Validate a .zen document and return diagnostics. Hard (Error) \
diagnostics block rendering — fix them first.",
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the .zen document." },
                    "json": { "type": "boolean", "description": "Return machine-readable JSON." }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "zenith_inspect",
            description: "Print the node tree of a .zen document (read-only). Use it to \
discover node ids before editing with zenith_tx.",
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "node": { "type": "string", "description": "Only the subtree at this id." },
                    "json": { "type": "boolean" }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "zenith_tokens",
            description: "List every design token and its resolved value. Visual properties \
must reference tokens, so this reveals the palette/type/spacing a document exposes.",
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "json": { "type": "boolean" }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "zenith_fmt",
            description: "Canonicalize a .zen document in place (idempotent). Returns whether \
the file changed and its content hash.",
            schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        },
        Tool {
            name: "zenith_render",
            description: "Render a .zen document deterministically to a file. format is one of \
png, pdf, or scene (display-list JSON). Blocked by hard diagnostics — validate first.",
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "format": { "type": "string", "enum": ["png", "pdf", "scene"] },
                    "out": { "type": "string", "description": "Output file path." },
                    "page": { "type": "integer", "minimum": 1, "description": "1-based page (default 1)." },
                    "locked": { "type": "boolean", "description": "Verify asset sha256 and fail on mismatch." }
                },
                "required": ["path", "format", "out"]
            }),
        },
        Tool {
            name: "zenith_tx",
            description: "Apply a typed transaction (a JSON edit script) to a .zen document. \
Dry-run by default (returns a diff); set apply=true to write. Enforces id-uniqueness and \
referential integrity.",
            schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "transaction": {
                        "description": "The transaction as a JSON object or a JSON string.",
                        "type": ["object", "string", "array"]
                    },
                    "apply": { "type": "boolean", "description": "Write the result to disk." },
                    "json": { "type": "boolean" }
                },
                "required": ["path", "transaction"]
            }),
        },
        Tool {
            name: "zenith_merge",
            description: "Mail-merge a .zen template with a CSV, writing one PNG per row. Mark \
variable nodes with role=\"data.<column>\". Use for localized/personalized/batch variants.",
            schema: json!({
                "type": "object",
                "properties": {
                    "doc": { "type": "string", "description": "Template .zen path." },
                    "data": { "type": "string", "description": "CSV data file path." },
                    "out_dir": { "type": "string", "description": "Directory for the output PNGs." },
                    "name_by": { "type": "string", "description": "CSV column to name files by." },
                    "manifest": { "type": "string", "description": "Write a reproducibility manifest here." }
                },
                "required": ["doc", "data", "out_dir"]
            }),
        },
        Tool {
            name: "zenith_theme_new",
            description: "Synthesize a complete token-only theme pack (.zen) from brand \
colours, with APCA-correct content pairings for WCAG 3 contrast. Writes to `out` or returns \
the source.",
            schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "scheme": { "type": "string", "enum": ["light", "dark"] },
                    "primary": { "type": "string", "description": "#rrggbb" },
                    "secondary": { "type": "string" },
                    "accent": { "type": "string" },
                    "neutral": { "type": "string" },
                    "info": { "type": "string" },
                    "success": { "type": "string" },
                    "warning": { "type": "string" },
                    "error": { "type": "string" },
                    "radius_box": { "type": "number" },
                    "radius_field": { "type": "number" },
                    "radius_selector": { "type": "number" },
                    "border": { "type": "number" },
                    "depth": { "type": "boolean" },
                    "noise": { "type": "boolean" },
                    "out": { "type": "string", "description": "Write the theme here instead of returning it." }
                },
                "required": ["name", "scheme", "primary"]
            }),
        },
    ]
}

/// Render the catalog as the `tools/list` result payload.
pub fn list_payload() -> Value {
    let tools: Vec<Value> = catalog()
        .into_iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": t.schema,
            })
        })
        .collect();
    json!({ "tools": tools })
}
