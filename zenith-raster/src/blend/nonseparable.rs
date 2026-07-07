use crate::blend::pixel::{StraightRgb, clamp_unit};

pub(crate) fn hue(backdrop: StraightRgb, source: StraightRgb) -> StraightRgb {
    set_lum(set_sat(source, sat(backdrop)), lum(backdrop))
}

pub(crate) fn saturation(backdrop: StraightRgb, source: StraightRgb) -> StraightRgb {
    set_lum(set_sat(backdrop, sat(source)), lum(backdrop))
}

pub(crate) fn color(backdrop: StraightRgb, source: StraightRgb) -> StraightRgb {
    set_lum(source, lum(backdrop))
}

pub(crate) fn luminosity(backdrop: StraightRgb, source: StraightRgb) -> StraightRgb {
    set_lum(backdrop, lum(source))
}

fn lum(color: StraightRgb) -> f32 {
    0.3 * color.r + 0.59 * color.g + 0.11 * color.b
}

fn sat(color: StraightRgb) -> f32 {
    color.r.max(color.g).max(color.b) - color.r.min(color.g).min(color.b)
}

fn clip_color(color: StraightRgb) -> StraightRgb {
    let luminosity = lum(color);
    let min = color.r.min(color.g).min(color.b);
    let max = color.r.max(color.g).max(color.b);
    let mut clipped = color;

    if min < 0.0 {
        clipped = scale_from_luminosity(clipped, luminosity, luminosity / (luminosity - min));
    }

    if max > 1.0 {
        clipped =
            scale_from_luminosity(clipped, luminosity, (1.0 - luminosity) / (max - luminosity));
    }

    StraightRgb {
        r: clamp_unit(clipped.r),
        g: clamp_unit(clipped.g),
        b: clamp_unit(clipped.b),
    }
}

fn set_lum(color: StraightRgb, luminosity: f32) -> StraightRgb {
    let delta = luminosity - lum(color);
    clip_color(StraightRgb {
        r: color.r + delta,
        g: color.g + delta,
        b: color.b + delta,
    })
}

fn set_sat(color: StraightRgb, saturation: f32) -> StraightRgb {
    let ordering = ComponentOrdering::from(color);
    let min = ordering.min.value(color);
    let mid = ordering.mid.value(color);
    let max = ordering.max.value(color);

    if max > min {
        ordering.rebuild(0.0, ((mid - min) * saturation) / (max - min), saturation)
    } else {
        ordering.rebuild(0.0, 0.0, 0.0)
    }
}

fn scale_from_luminosity(color: StraightRgb, luminosity: f32, scale: f32) -> StraightRgb {
    StraightRgb {
        r: luminosity + (color.r - luminosity) * scale,
        g: luminosity + (color.g - luminosity) * scale,
        b: luminosity + (color.b - luminosity) * scale,
    }
}

#[derive(Debug, Clone, Copy)]
struct ComponentOrdering {
    min: Component,
    mid: Component,
    max: Component,
}

impl ComponentOrdering {
    fn from(color: StraightRgb) -> Self {
        let mut ordering = Self {
            min: Component::Red,
            mid: Component::Green,
            max: Component::Blue,
        };
        ordering.sort_pair(color, RolePair::MinMid);
        ordering.sort_pair(color, RolePair::MidMax);
        ordering.sort_pair(color, RolePair::MinMid);
        ordering
    }

    fn sort_pair(&mut self, color: StraightRgb, pair: RolePair) {
        let (left, right) = match pair {
            RolePair::MinMid => (self.min, self.mid),
            RolePair::MidMax => (self.mid, self.max),
        };

        if left.value(color) > right.value(color) {
            match pair {
                RolePair::MinMid => {
                    self.min = right;
                    self.mid = left;
                }
                RolePair::MidMax => {
                    self.mid = right;
                    self.max = left;
                }
            }
        }
    }

    fn rebuild(self, min_value: f32, mid_value: f32, max_value: f32) -> StraightRgb {
        let mut color = StraightRgb::BLACK;
        color = self.min.with_value(color, min_value);
        color = self.mid.with_value(color, mid_value);
        self.max.with_value(color, max_value)
    }
}

#[derive(Debug, Clone, Copy)]
enum RolePair {
    MinMid,
    MidMax,
}

#[derive(Debug, Clone, Copy)]
enum Component {
    Red,
    Green,
    Blue,
}

impl Component {
    fn value(self, color: StraightRgb) -> f32 {
        match self {
            Component::Red => color.r,
            Component::Green => color.g,
            Component::Blue => color.b,
        }
    }

    fn with_value(self, color: StraightRgb, value: f32) -> StraightRgb {
        match self {
            Component::Red => StraightRgb { r: value, ..color },
            Component::Green => StraightRgb { g: value, ..color },
            Component::Blue => StraightRgb { b: value, ..color },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(r: f32, g: f32, b: f32) -> StraightRgb {
        StraightRgb { r, g, b }
    }

    fn assert_rgb_close(actual: StraightRgb, expected: StraightRgb) {
        for (actual, expected) in [actual.r, actual.g, actual.b]
            .into_iter()
            .zip([expected.r, expected.g, expected.b])
        {
            assert!(
                (actual - expected).abs() < 0.000_001,
                "actual {actual} expected {expected}"
            );
        }
    }

    #[test]
    fn clip_color_preserves_luminosity_with_channels_in_unit_range() {
        let clipped = clip_color(rgb(1.4, 0.5, -0.2));

        assert_rgb_close(clipped, rgb(0.931_243, 0.627_963_36, 0.392_079_2));
        assert!((lum(clipped) - lum(rgb(1.4, 0.5, -0.2))).abs() < 0.000_001);
    }

    #[test]
    fn set_lum_changes_luminosity_and_clips_overflow() {
        let adjusted = set_lum(rgb(1.0, 0.2, 0.0), 0.8);

        assert_rgb_close(adjusted, rgb(1.0, 0.725_085_9, 0.656_357_4));
        assert!((lum(adjusted) - 0.8).abs() < 0.000_001);
    }

    #[test]
    fn set_sat_handles_component_ordering() {
        assert_rgb_close(set_sat(rgb(0.7, 0.2, 0.4), 0.3), rgb(0.3, 0.0, 0.12));
        assert_rgb_close(set_sat(rgb(0.2, 0.7, 0.4), 0.3), rgb(0.0, 0.3, 0.12));
        assert_rgb_close(set_sat(rgb(0.4, 0.2, 0.7), 0.3), rgb(0.12, 0.0, 0.3));
        assert_rgb_close(set_sat(rgb(0.4, 0.4, 0.4), 0.3), rgb(0.0, 0.0, 0.0));
    }

    #[test]
    fn nonseparable_modes_match_stable_reference_values() {
        let backdrop = rgb(0.25, 0.5, 0.75);
        let source = rgb(0.8, 0.2, 0.4);

        assert_rgb_close(
            hue(backdrop, source),
            rgb(0.784_166_63, 0.284_166_63, 0.450_833_32),
        );
        assert_rgb_close(
            saturation(backdrop, source),
            rgb(0.209_499_98, 0.509_5, 0.809_5),
        );
        assert_rgb_close(
            color(backdrop, source),
            rgb(0.850_5, 0.250_500_02, 0.450_500_04),
        );
        assert_rgb_close(
            luminosity(backdrop, source),
            rgb(0.199_499_96, 0.449_499_96, 0.699_5),
        );
    }
}
