use crate::blend::pixel::StraightRgb;

pub(crate) fn blend(
    backdrop: StraightRgb,
    source: StraightRgb,
    channel_blend: fn(f32, f32) -> f32,
) -> StraightRgb {
    StraightRgb {
        r: channel_blend(backdrop.r, source.r),
        g: channel_blend(backdrop.g, source.g),
        b: channel_blend(backdrop.b, source.b),
    }
}

pub(crate) fn multiply(backdrop: f32, source: f32) -> f32 {
    backdrop * source
}

pub(crate) fn screen(backdrop: f32, source: f32) -> f32 {
    backdrop + source - backdrop * source
}

pub(crate) fn overlay(backdrop: f32, source: f32) -> f32 {
    hard_light(source, backdrop)
}

pub(crate) fn darken(backdrop: f32, source: f32) -> f32 {
    backdrop.min(source)
}

pub(crate) fn lighten(backdrop: f32, source: f32) -> f32 {
    backdrop.max(source)
}

pub(crate) fn color_dodge(backdrop: f32, source: f32) -> f32 {
    if backdrop <= 0.0 {
        0.0
    } else if source >= 1.0 {
        1.0
    } else {
        (backdrop / (1.0 - source)).min(1.0)
    }
}

pub(crate) fn color_burn(backdrop: f32, source: f32) -> f32 {
    if backdrop >= 1.0 {
        1.0
    } else if source <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - backdrop) / source).min(1.0)
    }
}

pub(crate) fn hard_light(backdrop: f32, source: f32) -> f32 {
    if source <= 0.5 {
        multiply(backdrop, 2.0 * source)
    } else {
        screen(backdrop, 2.0 * source - 1.0)
    }
}

pub(crate) fn soft_light(backdrop: f32, source: f32) -> f32 {
    if source <= 0.5 {
        backdrop - (1.0 - 2.0 * source) * backdrop * (1.0 - backdrop)
    } else {
        backdrop + (2.0 * source - 1.0) * (soft_light_d(backdrop) - backdrop)
    }
}

pub(crate) fn difference(backdrop: f32, source: f32) -> f32 {
    (backdrop - source).abs()
}

pub(crate) fn exclusion(backdrop: f32, source: f32) -> f32 {
    backdrop + source - 2.0 * backdrop * source
}

fn soft_light_d(channel: f32) -> f32 {
    if channel <= 0.25 {
        ((16.0 * channel - 12.0) * channel + 4.0) * channel
    } else {
        channel.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "actual {actual} expected {expected}"
        );
    }

    #[test]
    fn multiply_preserves_black_and_white_edges() {
        assert_eq!(multiply(0.7, 0.0), 0.0);
        assert_eq!(multiply(0.7, 1.0), 0.7);
        assert_close(multiply(0.25, 0.8), 0.2);
    }

    #[test]
    fn screen_preserves_black_and_white_edges() {
        assert_eq!(screen(0.7, 0.0), 0.7);
        assert_eq!(screen(0.7, 1.0), 1.0);
        assert_close(screen(0.25, 0.8), 0.85);
    }

    #[test]
    fn overlay_multiplies_dark_backdrops_and_screens_light_backdrops() {
        assert_close(overlay(0.25, 0.8), 0.4);
        assert_close(overlay(0.75, 0.2), 0.6);
    }

    #[test]
    fn darken_and_lighten_select_channel_extremes() {
        assert_eq!(darken(0.25, 0.8), 0.25);
        assert_eq!(lighten(0.25, 0.8), 0.8);
    }

    #[test]
    fn color_dodge_handles_edges_and_division() {
        assert_eq!(color_dodge(0.0, 0.7), 0.0);
        assert_eq!(color_dodge(0.4, 1.0), 1.0);
        assert_close(color_dodge(0.25, 0.5), 0.5);
        assert_eq!(color_dodge(0.8, 0.5), 1.0);
    }

    #[test]
    fn color_burn_handles_edges_and_division() {
        assert_eq!(color_burn(1.0, 0.2), 1.0);
        assert_eq!(color_burn(0.4, 0.0), 0.0);
        assert_close(color_burn(0.75, 0.5), 0.5);
        assert_eq!(color_burn(0.2, 0.5), 0.0);
    }

    #[test]
    fn hard_light_multiplies_low_sources_and_screens_high_sources() {
        assert_close(hard_light(0.25, 0.4), 0.2);
        assert_close(hard_light(0.25, 0.8), 0.7);
    }

    #[test]
    fn soft_light_uses_cubic_and_sqrt_branches() {
        assert_close(soft_light(0.2, 0.8), 0.348_8);
        assert_close(soft_light(0.64, 0.8), 0.736);
        assert_close(soft_light(0.4, 0.25), 0.28);
    }

    #[test]
    fn difference_and_exclusion_are_stable() {
        assert_close(difference(0.25, 0.8), 0.55);
        assert_close(exclusion(0.25, 0.8), 0.65);
    }
}
