//! Pure logic for `zenith merge`.
//!
//! The public entry point [`run`] operates entirely on in-memory source text
//! plus filesystem paths for outputs.  The source `.zen` file is NEVER
//! mutated; each row's document is produced in-memory via the transaction
//! engine and re-parsed before compilation.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use zenith_core::{AssetKind, BytesAssetProvider, KdlAdapter, KdlSource, Severity};
use zenith_render::render_png;
use zenith_scene::compile_page;
use zenith_tx::{Op, OpSpan, Transaction, TxStatus, run_transaction};

use crate::commands::render::{
    build_asset_provider, build_font_provider, collect_missing_asset_diagnostics,
};

// ── Error type ────────────────────────────────────────────────────────────────

/// A fatal error that prevents the merge from starting.
///
/// Exit code 2 for all setup/template errors (consistent with the other
/// commands whose `RenderCmdErr`/`FmtErr`/`TxCmdErr` all use 2 for this class
/// of failure).
#[derive(Debug)]
pub struct MergeError {
    /// Human-readable message.
    pub message: String,
    /// Recommended exit code (always 2 for template/setup errors).
    pub exit_code: u8,
}

impl MergeError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            exit_code: 2,
        }
    }
}

// ── Report types ──────────────────────────────────────────────────────────────

/// One per-row failure.
#[derive(Debug)]
pub struct RowFailure {
    /// 0-based row index in the CSV (header row not counted).
    pub row: usize,
    /// Human-readable reason.
    pub reason: String,
}

/// Summary of a completed merge run.
#[derive(Debug)]
pub struct MergeReport {
    /// Filenames (not full paths) of PNGs successfully written, in row order.
    pub written: Vec<String>,
    /// Rows that were skipped due to per-row errors.
    pub failed: Vec<RowFailure>,
}

// ── Internal binding types ────────────────────────────────────────────────────

/// Maps a node id to the CSV column that supplies its replacement text.
struct DataBinding {
    node_id: String,
    column: String,
}

/// Maps an image node id to the CSV column that supplies the per-row image path.
struct AssetBinding {
    node_id: String,
    column: String,
}

// ── collect_data_nodes ────────────────────────────────────────────────────────

/// Return an error if `role` starts with `"data."` on a non-text, non-image node.
///
/// The error message and format live here exactly once.
fn reject_data_role_on_non_text(role: Option<&str>, id: &str) -> Result<(), MergeError> {
    if let Some(role) = role
        && role.starts_with("data.")
    {
        return Err(MergeError::new(format!(
            "role=\"{}\" on non-text node {}: replace_text supports text nodes only",
            role, id
        )));
    }
    Ok(())
}

/// Walk `nodes` recursively and collect every node that carries a
/// `role="data.<column>"` attribute.
///
/// `Node::Text` bindings are collected into `out`; `Node::Image` bindings are
/// collected into `asset_out`.  Any other variant with a `data.*` role is a
/// hard [`MergeError`].
///
/// Recurses into `Node::Frame`, `Node::Group`, and `Node::Table` cell children.
fn collect_data_nodes(
    nodes: &[zenith_core::Node],
    out: &mut Vec<DataBinding>,
    asset_out: &mut Vec<AssetBinding>,
) -> Result<(), MergeError> {
    for node in nodes {
        match node {
            zenith_core::Node::Text(n) => {
                if let Some(role) = n.role.as_deref()
                    && let Some(col) = role.strip_prefix("data.")
                {
                    out.push(DataBinding {
                        node_id: n.id.clone(),
                        column: col.to_owned(),
                    });
                }
            }
            zenith_core::Node::Image(n) => {
                if let Some(role) = n.role.as_deref()
                    && let Some(col) = role.strip_prefix("data.")
                {
                    asset_out.push(AssetBinding {
                        node_id: n.id.clone(),
                        column: col.to_owned(),
                    });
                }
            }
            zenith_core::Node::Rect(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Ellipse(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Line(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Code(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Frame(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
                collect_data_nodes(&n.children, out, asset_out)?;
            }
            zenith_core::Node::Group(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
                collect_data_nodes(&n.children, out, asset_out)?;
            }
            zenith_core::Node::Polygon(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Polyline(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Instance(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Field(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Toc(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Footnote(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Table(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
                for row in &n.rows {
                    for cell in &row.cells {
                        collect_data_nodes(&cell.children, out, asset_out)?;
                    }
                }
            }
            zenith_core::Node::Shape(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Connector(n) => {
                reject_data_role_on_non_text(n.role.as_deref(), &n.id)?;
            }
            zenith_core::Node::Unknown(_n) => {
                // UnknownNode has no id or role field; data.* roles cannot be
                // placed on unknown nodes (the parser would not parse them).
            }
        }
    }
    Ok(())
}

// ── sanitize_filename ─────────────────────────────────────────────────────────

/// Map filesystem-unsafe characters and NUL to `_`, trim leading/trailing
/// dots and whitespace, and return `"_"` for the empty result.
pub fn sanitize_filename(s: &str) -> String {
    let mapped: String = s
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            other => other,
        })
        .collect();
    let trimmed = mapped.trim_matches(|c: char| c == '.' || c.is_whitespace());
    if trimmed.is_empty() {
        "_".to_owned()
    } else {
        trimmed.to_owned()
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Run a mail-merge: for each CSV row, build a per-row document (in-memory),
/// compile it, render it to PNG, and stream the file to `out_dir`.
///
/// # Parameters
///
/// - `doc_src`     — UTF-8 source of the template `.zen` document.
/// - `csv_src`     — UTF-8 CSV with a header row.
/// - `project_dir` — directory of the `.zen` file (for asset resolution).
/// - `out_dir`     — directory to write one PNG per row into.
/// - `name_by`     — CSV column to derive filenames from; default `row-NNNN.png`.
///
/// # Errors
///
/// Returns [`MergeError`] (exit code 2) for template/setup failures that
/// prevent any row from being processed.  Per-row failures are collected into
/// [`MergeReport::failed`] and do not cause an `Err` return.
pub fn run(
    doc_src: &str,
    csv_src: &str,
    project_dir: Option<&Path>,
    out_dir: &Path,
    name_by: Option<&str>,
) -> Result<MergeReport, MergeError> {
    // ── 1. Parse the template document (once) ─────────────────────────────
    let doc = KdlAdapter
        .parse(doc_src.as_bytes())
        .map_err(|e| MergeError::new(format!("error[parse.error]: {}", e.message)))?;

    // ── 2. Collect data bindings ──────────────────────────────────────────
    let mut bindings: Vec<DataBinding> = Vec::new();
    let mut asset_bindings: Vec<AssetBinding> = Vec::new();
    for page in &doc.body.pages {
        collect_data_nodes(&page.children, &mut bindings, &mut asset_bindings)?;
    }
    if bindings.is_empty() && asset_bindings.is_empty() {
        return Err(MergeError::new("no role=\"data.*\" template nodes found"));
    }

    // ── 3. Validate: asset bindings require a project_dir ────────────────
    if !asset_bindings.is_empty() && project_dir.is_none() {
        return Err(MergeError::new(
            "image data bindings require a project directory (the .zen file must be on disk)",
        ));
    }

    // ── 4. Parse CSV headers and validate bindings ────────────────────────
    let mut reader = csv::Reader::from_reader(csv_src.as_bytes());
    let headers = reader
        .headers()
        .map_err(|e| MergeError::new(format!("CSV header error: {}", e)))?
        .clone();

    // Build a header→index map (BTreeMap for deterministic ordering).
    let header_index: BTreeMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.to_owned(), i))
        .collect();

    // Verify all text-binding columns exist.
    let unknown: Vec<String> = bindings
        .iter()
        .filter(|b| !header_index.contains_key(&b.column))
        .map(|b| b.column.clone())
        .collect();
    if !unknown.is_empty() {
        return Err(MergeError::new(format!(
            "CSV column(s) not found in header: {}",
            unknown.join(", ")
        )));
    }

    // Verify all asset-binding columns exist.
    let unknown_asset: Vec<String> = asset_bindings
        .iter()
        .filter(|b| !header_index.contains_key(&b.column))
        .map(|b| b.column.clone())
        .collect();
    if !unknown_asset.is_empty() {
        return Err(MergeError::new(format!(
            "CSV column(s) not found in header: {}",
            unknown_asset.join(", ")
        )));
    }

    // Verify name_by column exists.
    if let Some(col) = name_by
        && !header_index.contains_key(col)
    {
        return Err(MergeError::new(format!(
            "--name-by column {:?} not found in CSV header",
            col
        )));
    }

    // Pre-resolve column indices (avoids per-cell BTreeMap lookups).
    // All columns were verified to exist above so `get` never returns None.
    let binding_indices: Vec<usize> = bindings
        .iter()
        .map(|b| -> Result<usize, MergeError> {
            header_index
                .get(&b.column)
                .copied()
                .ok_or_else(|| MergeError::new(format!("column {:?} not found", b.column)))
        })
        .collect::<Result<Vec<usize>, MergeError>>()?;

    let asset_binding_indices: Vec<usize> = asset_bindings
        .iter()
        .map(|b| -> Result<usize, MergeError> {
            header_index
                .get(&b.column)
                .copied()
                .ok_or_else(|| MergeError::new(format!("column {:?} not found", b.column)))
        })
        .collect::<Result<Vec<usize>, MergeError>>()?;

    let name_by_index: Option<usize> = match name_by {
        None => None,
        Some(col) => Some(
            header_index
                .get(col)
                .copied()
                .ok_or_else(|| MergeError::new(format!("--name-by column {:?} not found", col)))?,
        ),
    };

    // ── 5. Build font + asset providers ONCE from the original doc ────────
    let fonts =
        build_font_provider(&doc, project_dir, false).map_err(|e| MergeError::new(e.message))?;
    // Template assets are loaded once; per-row image bytes are layered on top.
    let template_assets = match project_dir {
        Some(dir) => {
            build_asset_provider(&doc, dir, false).map_err(|e| MergeError::new(e.message))?
        }
        None => BytesAssetProvider::new(),
    };

    // ── 6. Ensure output directory exists ─────────────────────────────────
    std::fs::create_dir_all(out_dir).map_err(|e| {
        MergeError::new(format!(
            "could not create output directory '{}': {}",
            out_dir.display(),
            e
        ))
    })?;

    // ── 7. Iterate CSV rows ───────────────────────────────────────────────
    let mut written: Vec<String> = Vec::new();
    let mut failed: Vec<RowFailure> = Vec::new();
    let mut used_names: BTreeSet<String> = BTreeSet::new();

    for (row_idx, record_result) in reader.records().enumerate() {
        let record = match record_result {
            Ok(r) => r,
            Err(e) => {
                failed.push(RowFailure {
                    row: row_idx,
                    reason: format!("CSV read error: {}", e),
                });
                continue;
            }
        };

        // Build Transaction ops: ReplaceText ops first, then asset ops.
        let mut ops: Vec<Op> = bindings
            .iter()
            .zip(binding_indices.iter())
            .map(|(binding, &col_idx)| {
                let cell = record.get(col_idx).unwrap_or("");
                Op::ReplaceText {
                    node: binding.node_id.clone(),
                    spans: vec![OpSpan {
                        text: cell.to_owned(),
                        fill: None,
                        font_weight: None,
                        italic: None,
                        underline: None,
                        strikethrough: None,
                        vertical_align: None,
                        footnote_ref: None,
                    }],
                }
            })
            .collect();

        // Append AddAsset + SetAsset ops for non-empty image cells.
        for (binding, &col_idx) in asset_bindings.iter().zip(asset_binding_indices.iter()) {
            let cell = record.get(col_idx).unwrap_or("").trim();
            if cell.is_empty() {
                // Empty cell → leave template image in place; no op needed.
                continue;
            }
            let asset_id = row_asset_id(row_idx, &binding.column);
            ops.push(Op::AddAsset {
                id: asset_id.clone(),
                kind: "image".to_owned(),
                src: cell.to_owned(),
                sha256: None,
            });
            ops.push(Op::SetAsset {
                node_id: binding.node_id.clone(),
                asset_id,
            });
        }

        let tx = Transaction {
            ops,
            permissions: Default::default(),
        };

        // Run transaction.
        let tx_result = match run_transaction(&doc, &tx) {
            Ok(r) => r,
            Err(e) => {
                failed.push(RowFailure {
                    row: row_idx,
                    reason: format!("transaction engine error: {}", e.message),
                });
                continue;
            }
        };

        // A Rejected transaction is a per-row failure.
        if tx_result.status == TxStatus::Rejected {
            let msgs: Vec<String> = tx_result
                .diagnostics
                .iter()
                .map(|d| format!("{}[{}]: {}", severity_label(&d.severity), d.code, d.message))
                .collect();
            failed.push(RowFailure {
                row: row_idx,
                reason: format!("transaction rejected: {}", msgs.join("; ")),
            });
            continue;
        }

        // Re-parse source_after → row document.
        let row_doc = match KdlAdapter.parse(tx_result.source_after.as_bytes()) {
            Ok(d) => d,
            Err(e) => {
                failed.push(RowFailure {
                    row: row_idx,
                    reason: format!("post-transaction parse error: {}", e.message),
                });
                continue;
            }
        };

        // Build per-row asset provider: template assets + row-specific images.
        // BytesAssetProvider is not Clone, so we rebuild from template doc and
        // then layer the row image(s) on top.
        let row_assets = if asset_bindings.is_empty() {
            // No image bindings: the pre-built `template_assets` provider is used
            // directly by the render call below — no per-row provider is needed.
            None
        } else {
            let Some(dir) = project_dir else {
                failed.push(RowFailure {
                    row: row_idx,
                    reason: "internal: project directory unexpectedly missing".to_owned(),
                });
                continue;
            };
            // Start with template assets.
            let mut row_provider =
                build_asset_provider(&doc, dir, false).map_err(|e| MergeError::new(e.message))?;
            // Layer in per-row images.
            let mut row_asset_missing = false;
            for (binding, &col_idx) in asset_bindings.iter().zip(asset_binding_indices.iter()) {
                let cell = record.get(col_idx).unwrap_or("").trim();
                if cell.is_empty() {
                    continue;
                }
                let asset_id = row_asset_id(row_idx, &binding.column);
                let img_path = dir.join(cell);
                match std::fs::read(&img_path) {
                    Ok(bytes) => {
                        row_provider.register(&asset_id, AssetKind::Image, bytes.into());
                    }
                    Err(e) => {
                        failed.push(RowFailure {
                            row: row_idx,
                            reason: format!(
                                "error[asset.missing]: asset '{}' file not found: '{}': {}",
                                asset_id,
                                img_path.display(),
                                e
                            ),
                        });
                        row_asset_missing = true;
                        break;
                    }
                }
            }
            if row_asset_missing {
                continue;
            }
            Some(row_provider)
        };

        // Also gate on collect_missing_asset_diagnostics for any declared-but-missing
        // template assets now embedded in row_doc (includes the AddAsset entries).
        if let Some(dir) = project_dir {
            let missing_diags = collect_missing_asset_diagnostics(&row_doc, dir);
            let hard: Vec<String> = missing_diags
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .map(|d| format!("error[{}]: {}", d.code, d.message))
                .collect();
            if !hard.is_empty() {
                failed.push(RowFailure {
                    row: row_idx,
                    reason: format!("asset error(s): {}", hard.join("; ")),
                });
                continue;
            }
        }

        // Compile page 0.
        let compile_result = compile_page(&row_doc, &fonts, 0);

        // Block on Error-severity compile diagnostics (e.g. text.fit_failed).
        let hard_diags: Vec<String> = compile_result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| format!("error[{}]: {}", d.code, d.message))
            .collect();
        if !hard_diags.is_empty() {
            failed.push(RowFailure {
                row: row_idx,
                reason: format!("compile error(s): {}", hard_diags.join("; ")),
            });
            continue;
        }

        // Determine output filename.
        let filename = match name_by_index {
            Some(col_idx) => {
                let cell = record.get(col_idx).unwrap_or("");
                format!("{}.png", sanitize_filename(cell))
            }
            None => format!("row-{:04}.png", row_idx + 1),
        };

        // Collision check.
        if used_names.contains(&filename) {
            failed.push(RowFailure {
                row: row_idx,
                reason: format!("output filename collision: {}", filename),
            });
            continue;
        }
        used_names.insert(filename.clone());

        // Render to PNG bytes, using row-scoped assets when image bindings exist.
        let png_bytes = match &row_assets {
            Some(ra) => render_png(&compile_result.scene, &fonts, ra),
            None => render_png(&compile_result.scene, &fonts, &template_assets),
        };
        let png_bytes = match png_bytes {
            Ok(b) => b,
            Err(e) => {
                failed.push(RowFailure {
                    row: row_idx,
                    reason: format!("render error: {}", e),
                });
                continue;
            }
        };

        // Write immediately (stream — never accumulate all PNGs in memory).
        let out_path = out_dir.join(&filename);
        if let Err(e) = std::fs::write(&out_path, &png_bytes) {
            failed.push(RowFailure {
                row: row_idx,
                reason: format!("write error '{}': {}", out_path.display(), e),
            });
            continue;
        }

        written.push(filename);
    }

    Ok(MergeReport { written, failed })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Canonical asset-id used for per-row image bindings.
///
/// Must match between the op-building pass (AddAsset/SetAsset) and the
/// asset-loading pass (register into row_provider) — keeping it here
/// ensures they can never diverge.
fn row_asset_id(row_idx: usize, column: &str) -> String {
    format!("merge.row.{}.asset.{}", row_idx, column)
}

fn severity_label(sev: &Severity) -> &'static str {
    match sev {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Advisory => "advisory",
    }
}
