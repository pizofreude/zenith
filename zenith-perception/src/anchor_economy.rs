use crate::diagnostic::{PerceptionDiagnostic, PerceptionSeverity};

const HIGH_EXCESS_ANCHOR_RATIO: f32 = 0.5;
const HIGH_HANDLES_PER_ANCHOR: f32 = 2.0;
const HANDLE_PENALTY_WEIGHT: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorEconomyInput {
    pub anchor_count: usize,
    pub segment_count: usize,
    pub handle_count: usize,
    pub open_subpath_count: usize,
    pub closed_subpath_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnchorEconomyReport {
    pub anchor_count: usize,
    pub segment_count: usize,
    pub handle_count: usize,
    pub open_subpath_count: usize,
    pub closed_subpath_count: usize,
    pub minimum_anchor_count: usize,
    pub excess_anchor_count: usize,
    pub excess_anchor_ratio: f32,
    pub handles_per_anchor: f32,
    /// Normalized count-derived score in `[0, 1]`.
    ///
    /// Invalid count combinations and empty/zero-minimum inputs score `0`.
    /// Otherwise, anchors above the deterministic topology minimum reduce the
    /// score by `excess_anchor_count / minimum_anchor_count`, and handles above
    /// two per anchor reduce it by `0.25 * (handles_per_anchor - 2.0)`.
    pub economy_score: f32,
    pub diagnostics: Vec<PerceptionDiagnostic>,
}

pub fn anchor_economy(input: AnchorEconomyInput) -> AnchorEconomyReport {
    let minimum_anchor_count = minimum_anchor_count(input);
    let excess_anchor_count = input.anchor_count.saturating_sub(minimum_anchor_count);
    let excess_anchor_ratio = ratio(excess_anchor_count, minimum_anchor_count);
    let handles_per_anchor = ratio(input.handle_count, input.anchor_count);
    let diagnostics = diagnostics(
        input,
        minimum_anchor_count,
        excess_anchor_ratio,
        handles_per_anchor,
    );
    let economy_score = economy_score(
        input,
        minimum_anchor_count,
        excess_anchor_ratio,
        handles_per_anchor,
        &diagnostics,
    );

    AnchorEconomyReport {
        anchor_count: input.anchor_count,
        segment_count: input.segment_count,
        handle_count: input.handle_count,
        open_subpath_count: input.open_subpath_count,
        closed_subpath_count: input.closed_subpath_count,
        minimum_anchor_count,
        excess_anchor_count,
        excess_anchor_ratio,
        handles_per_anchor,
        economy_score,
        diagnostics,
    }
}

fn minimum_anchor_count(input: AnchorEconomyInput) -> usize {
    input.segment_count.saturating_add(input.open_subpath_count)
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn economy_score(
    input: AnchorEconomyInput,
    minimum_anchor_count: usize,
    excess_anchor_ratio: f32,
    handles_per_anchor: f32,
    diagnostics: &[PerceptionDiagnostic],
) -> f32 {
    if input.anchor_count == 0 && input.segment_count == 0 && input.handle_count == 0 {
        return 0.0;
    }

    if diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == PerceptionSeverity::Warning
            && diagnostic.code.starts_with("anchor_economy.invalid_")
    }) {
        return 0.0;
    }

    if minimum_anchor_count == 0 {
        return 0.0;
    }

    let anchor_penalty = excess_anchor_ratio;
    let handle_penalty =
        ((handles_per_anchor - HIGH_HANDLES_PER_ANCHOR).max(0.0)) * HANDLE_PENALTY_WEIGHT;
    (1.0 - anchor_penalty - handle_penalty).clamp(0.0, 1.0)
}

fn diagnostics(
    input: AnchorEconomyInput,
    minimum_anchor_count: usize,
    excess_anchor_ratio: f32,
    handles_per_anchor: f32,
) -> Vec<PerceptionDiagnostic> {
    let mut diagnostics = Vec::new();
    let subpath_count = input
        .open_subpath_count
        .saturating_add(input.closed_subpath_count);

    if input.segment_count > 0 && input.anchor_count == 0 {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.invalid_segments_without_anchors",
            PerceptionSeverity::Warning,
            "segments are present without anchors",
        ));
    }

    if input.handle_count > 0 && input.anchor_count == 0 {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.invalid_handles_without_anchors",
            PerceptionSeverity::Warning,
            "handles are present without anchors",
        ));
    }

    if input.anchor_count > 0 && input.segment_count == 0 {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.invalid_anchors_without_segments",
            PerceptionSeverity::Warning,
            "anchors are present without path segments",
        ));
    }

    if input.anchor_count > 0 && input.segment_count == 0 && subpath_count > 0 {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.invalid_topology_without_segments",
            PerceptionSeverity::Warning,
            "subpath counts are present without path segments",
        ));
    }

    if input.segment_count > 0 && subpath_count > input.segment_count {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.invalid_topology_overflow",
            PerceptionSeverity::Warning,
            "subpath count exceeds segment count",
        ));
    }

    if input.segment_count > 0 && input.open_subpath_count == 0 && input.closed_subpath_count == 0 {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.invalid_missing_topology",
            PerceptionSeverity::Warning,
            "segments are present without open or closed subpath counts",
        ));
    }

    if input.segment_count > 0
        && minimum_anchor_count > 0
        && input.anchor_count < minimum_anchor_count
    {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.invalid_anchor_deficit",
            PerceptionSeverity::Warning,
            "anchor count is below the deterministic topology minimum",
        ));
    }

    if excess_anchor_ratio > HIGH_EXCESS_ANCHOR_RATIO {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.high_excess_anchor_ratio",
            PerceptionSeverity::Info,
            "anchor count exceeds the deterministic topology minimum",
        ));
    }

    if handles_per_anchor > HIGH_HANDLES_PER_ANCHOR {
        diagnostics.push(PerceptionDiagnostic::new(
            "anchor_economy.high_handles_per_anchor",
            PerceptionSeverity::Info,
            "handle count is high relative to anchors",
        ));
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_input_reports_zero_ratios_and_zero_score() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 0,
            segment_count: 0,
            handle_count: 0,
            open_subpath_count: 0,
            closed_subpath_count: 0,
        });

        assert_eq!(report.anchor_count, 0);
        assert_eq!(report.segment_count, 0);
        assert_eq!(report.handle_count, 0);
        assert_eq!(report.minimum_anchor_count, 0);
        assert_eq!(report.excess_anchor_ratio, 0.0);
        assert_eq!(report.handles_per_anchor, 0.0);
        assert_eq!(report.economy_score, 0.0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn simple_open_path_has_high_economy_without_diagnostics() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 2,
            segment_count: 1,
            handle_count: 0,
            open_subpath_count: 1,
            closed_subpath_count: 0,
        });

        assert_eq!(report.minimum_anchor_count, 2);
        assert_eq!(report.excess_anchor_count, 0);
        assert_eq!(report.excess_anchor_ratio, 0.0);
        assert_eq!(report.handles_per_anchor, 0.0);
        assert_eq!(report.economy_score, 1.0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn simple_closed_path_has_high_economy_without_diagnostics() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 4,
            segment_count: 4,
            handle_count: 0,
            open_subpath_count: 0,
            closed_subpath_count: 1,
        });

        assert_eq!(report.minimum_anchor_count, 4);
        assert_eq!(report.excess_anchor_count, 0);
        assert_eq!(report.economy_score, 1.0);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn excessive_anchor_ratio_lowers_score_and_emits_diagnostic() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 8,
            segment_count: 4,
            handle_count: 0,
            open_subpath_count: 1,
            closed_subpath_count: 0,
        });

        assert_eq!(report.minimum_anchor_count, 5);
        assert_eq!(report.excess_anchor_count, 3);
        assert_eq!(report.excess_anchor_ratio, 0.6);
        assert_eq!(report.handles_per_anchor, 0.0);
        assert_eq!(report.economy_score, 0.39999998);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.high_excess_anchor_ratio"
        );
    }

    #[test]
    fn excessive_handles_per_anchor_lowers_score_and_emits_diagnostic() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 4,
            segment_count: 4,
            handle_count: 12,
            open_subpath_count: 0,
            closed_subpath_count: 1,
        });

        assert_eq!(report.excess_anchor_ratio, 0.0);
        assert_eq!(report.handles_per_anchor, 3.0);
        assert_eq!(report.economy_score, 0.75);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.high_handles_per_anchor"
        );
    }

    #[test]
    fn too_few_open_path_anchors_emit_invalid_diagnostic() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 1,
            segment_count: 1,
            handle_count: 0,
            open_subpath_count: 1,
            closed_subpath_count: 0,
        });

        assert_eq!(report.minimum_anchor_count, 2);
        assert_eq!(report.economy_score, 0.0);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.invalid_anchor_deficit"
        );
    }

    #[test]
    fn too_few_closed_path_anchors_emit_invalid_diagnostic() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 2,
            segment_count: 3,
            handle_count: 0,
            open_subpath_count: 0,
            closed_subpath_count: 1,
        });

        assert_eq!(report.minimum_anchor_count, 3);
        assert_eq!(report.economy_score, 0.0);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.invalid_anchor_deficit"
        );
    }

    #[test]
    fn impossible_subpath_count_emits_invalid_diagnostic() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 3,
            segment_count: 1,
            handle_count: 0,
            open_subpath_count: 2,
            closed_subpath_count: 0,
        });

        assert_eq!(report.economy_score, 0.0);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.invalid_topology_overflow"
        );
    }

    #[test]
    fn invalid_counts_emit_diagnostics_and_zero_score() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 0,
            segment_count: 1,
            handle_count: 2,
            open_subpath_count: 1,
            closed_subpath_count: 0,
        });

        assert_eq!(report.minimum_anchor_count, 2);
        assert_eq!(report.economy_score, 0.0);
        assert_eq!(report.diagnostics.len(), 3);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.invalid_segments_without_anchors"
        );
        assert_eq!(
            report.diagnostics[1].code,
            "anchor_economy.invalid_handles_without_anchors"
        );
        assert_eq!(
            report.diagnostics[2].code,
            "anchor_economy.invalid_anchor_deficit"
        );
    }

    #[test]
    fn topology_without_segments_emits_invalid_diagnostics() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 1,
            segment_count: 0,
            handle_count: 0,
            open_subpath_count: 1,
            closed_subpath_count: 0,
        });

        assert_eq!(report.economy_score, 0.0);
        assert_eq!(report.diagnostics.len(), 2);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.invalid_anchors_without_segments"
        );
        assert_eq!(
            report.diagnostics[1].code,
            "anchor_economy.invalid_topology_without_segments"
        );
    }

    #[test]
    fn anchors_without_segments_emit_invalid_diagnostic() {
        let report = anchor_economy(AnchorEconomyInput {
            anchor_count: 3,
            segment_count: 0,
            handle_count: 0,
            open_subpath_count: 0,
            closed_subpath_count: 0,
        });

        assert_eq!(report.minimum_anchor_count, 0);
        assert_eq!(report.economy_score, 0.0);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "anchor_economy.invalid_anchors_without_segments"
        );
    }
}
