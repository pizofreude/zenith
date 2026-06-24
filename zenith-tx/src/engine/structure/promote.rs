//! `PromoteCandidate` application: deep-copy a selected candidate page's
//! content into a target (export) page with freshly-suffixed node ids.

use zenith_core::{Diagnostic, Document};

use super::super::{node_id_of, record_affected};
use super::duplicate::{suffix_ids_in_children, suffix_zone_and_fold_ids};

pub(in crate::engine) fn apply_promote_candidate(
    source_page: &str,
    target_page: &str,
    id_suffix: &str,
    doc: &mut Document,
    diagnostics: &mut Vec<Diagnostic>,
    affected: &mut Vec<String>,
) {
    // 1. Find source page index.
    let Some(src_idx) = doc.body.pages.iter().position(|p| p.id == source_page) else {
        diagnostics.push(Diagnostic::error(
            "tx.unknown_node",
            format!("promote_candidate: page {:?} not found", source_page),
            None,
            Some(source_page.to_owned()),
        ));
        return;
    };

    // 2. Find target page index.
    let Some(tgt_idx) = doc.body.pages.iter().position(|p| p.id == target_page) else {
        diagnostics.push(Diagnostic::error(
            "tx.unknown_node",
            format!("promote_candidate: page {:?} not found", target_page),
            None,
            Some(target_page.to_owned()),
        ));
        return;
    };

    // 3. Source and target must be distinct pages.
    if src_idx == tgt_idx {
        diagnostics.push(Diagnostic::error(
            "tx.invalid_value",
            format!(
                "promote_candidate: source_page and target_page are the same page {:?}; \
                 they must be distinct",
                source_page
            ),
            None,
            Some(source_page.to_owned()),
        ));
        return;
    }

    // 4. Source must have candidate_status == "selected".
    {
        let Some(source) = doc.body.pages.get(src_idx) else {
            return; // unreachable: src_idx came from the position scan above
        };
        match source.candidate_status.as_deref() {
            Some("selected") => {}
            other => {
                let actual = other.unwrap_or("<absent>");
                diagnostics.push(Diagnostic::error(
                    "tx.candidate_not_selected",
                    format!(
                        "promote_candidate: source page {:?} must have \
                         candidate-status=\"selected\", but its status is {:?}",
                        source_page, actual
                    ),
                    None,
                    Some(source_page.to_owned()),
                ));
                return;
            }
        }
    }

    // 5. Advisory if id_suffix is empty (mirrors apply_duplicate_page).
    if id_suffix.is_empty() {
        diagnostics.push(Diagnostic::advisory(
            "tx.noop",
            format!(
                "promote_candidate: empty id_suffix will not keep cloned node ids \
                 unique for source page {:?}; the transaction will be rejected",
                source_page
            ),
            None,
            Some(source_page.to_owned()),
        ));
    }

    // 6. Clone children/safe_zones/folds from the source page before we take
    //    a mutable borrow on the target. The source borrow is released here.
    let (mut new_children, mut new_safe_zones, mut new_folds) = {
        let Some(source) = doc.body.pages.get(src_idx) else {
            return; // unreachable: src_idx came from the position scan above
        };
        (
            source.children.clone(),
            source.safe_zones.clone(),
            source.folds.clone(),
        )
    };

    // 7. Suffix all ids in the cloned content (mirrors apply_duplicate_page).
    suffix_ids_in_children(&mut new_children, id_suffix);
    suffix_zone_and_fold_ids(&mut new_safe_zones, &mut new_folds, id_suffix);

    // 8. Advisory if the target already has children (they will be replaced).
    {
        let Some(target) = doc.body.pages.get(tgt_idx) else {
            return; // unreachable: tgt_idx came from the position scan above
        };
        if !target.children.is_empty() {
            diagnostics.push(Diagnostic::advisory(
                "tx.noop",
                format!(
                    "promote_candidate: target page {:?} has existing children \
                     that will be replaced by the promoted content",
                    target_page
                ),
                None,
                Some(target_page.to_owned()),
            ));
        }
    }

    // Replace content and update metadata on the target page.
    let Some(target) = doc.body.pages.get_mut(tgt_idx) else {
        return; // unreachable: tgt_idx came from the position scan above
    };
    target.children = new_children;
    target.safe_zones = new_safe_zones;
    target.folds = new_folds;
    target.workspace_role = Some("export".to_owned());
    target.promotion_target = None;

    // 9. Record target page and all new child ids as affected.
    record_affected(target_page, affected);
    for child in &target.children {
        if let Some(id) = node_id_of(child) {
            record_affected(id, affected);
        }
    }
}
