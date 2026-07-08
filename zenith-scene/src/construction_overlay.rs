//! Opt-in scene overlay for page-scoped construction guides.

use zenith_core::{Color, Page, dim_to_px};

use crate::ir::{LineCap, Scene, SceneCommand};

const OVERLAY_COLOR: Color = Color::srgb(0, 174, 239, 180);
const OVERLAY_STROKE_WIDTH: f64 = 1.0;
const OVERLAY_DASH: f64 = 6.0;
const OVERLAY_GAP: f64 = 4.0;

/// Append non-printing construction guides to an already compiled scene.
///
/// This is deliberately opt-in and post-compile: canonical rendering never
/// reads construction metadata, while overlay callers get the same scene command
/// contract as every backend.
pub fn append_construction_overlay(scene: &mut Scene, page: &Page) {
    for guide in &page.construction.guides {
        match guide.guide_type.as_str() {
            "segment" => {
                let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                    guide.x1.as_ref().and_then(|d| dim_to_px(d.value, &d.unit)),
                    guide.y1.as_ref().and_then(|d| dim_to_px(d.value, &d.unit)),
                    guide.x2.as_ref().and_then(|d| dim_to_px(d.value, &d.unit)),
                    guide.y2.as_ref().and_then(|d| dim_to_px(d.value, &d.unit)),
                ) else {
                    continue;
                };
                if finite_non_degenerate_segment(x1, y1, x2, y2) {
                    scene.commands.push(SceneCommand::StrokeLine {
                        x1,
                        y1,
                        x2,
                        y2,
                        color: OVERLAY_COLOR,
                        stroke_width: OVERLAY_STROKE_WIDTH,
                        stroke_dash: Some(OVERLAY_DASH),
                        stroke_gap: Some(OVERLAY_GAP),
                        stroke_linecap: Some(LineCap::Round),
                    });
                }
            }
            "circle" => {
                let (Some(cx), Some(cy), Some(r)) = (
                    guide.cx.as_ref().and_then(|d| dim_to_px(d.value, &d.unit)),
                    guide.cy.as_ref().and_then(|d| dim_to_px(d.value, &d.unit)),
                    guide.r.as_ref().and_then(|d| dim_to_px(d.value, &d.unit)),
                ) else {
                    continue;
                };
                if cx.is_finite() && cy.is_finite() && r.is_finite() && r > 0.0 {
                    scene.commands.push(SceneCommand::StrokeEllipse {
                        x: cx - r,
                        y: cy - r,
                        w: r * 2.0,
                        h: r * 2.0,
                        color: OVERLAY_COLOR,
                        stroke_width: OVERLAY_STROKE_WIDTH,
                        stroke_dash: Some(OVERLAY_DASH),
                        stroke_gap: Some(OVERLAY_GAP),
                        stroke_linecap: Some(LineCap::Round),
                        rx: None,
                        ry: None,
                    });
                }
            }
            _ => {}
        }
    }
}

fn finite_non_degenerate_segment(x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
    x1.is_finite() && y1.is_finite() && x2.is_finite() && y2.is_finite() && (x1 != x2 || y1 != y2)
}

#[cfg(test)]
mod tests {
    use zenith_core::{KdlAdapter, KdlSource};

    use super::*;
    use crate::Scene;

    #[test]
    fn appends_segment_and_circle_overlay_commands() {
        let doc = KdlAdapter
            .parse(
                br#"zenith version=1 {
  document id="doc.guides" {
    page id="page.guides" w=(px)400 h=(px)300 {
      construction {
        guide id="axis" type="segment" x1=(px)0 y1=(px)150 x2=(px)400 y2=(px)150
        guide id="ring" type="circle" cx=(px)200 cy=(px)150 r=(px)50
      }
    }
  }
}
"#,
            )
            .expect("parse");
        let mut scene = Scene::new(400.0, 300.0);

        append_construction_overlay(&mut scene, &doc.body.pages[0]);

        assert_eq!(scene.commands.len(), 2);
        assert!(matches!(scene.commands[0], SceneCommand::StrokeLine { .. }));
        assert!(matches!(
            scene.commands[1],
            SceneCommand::StrokeEllipse { .. }
        ));
    }
}
