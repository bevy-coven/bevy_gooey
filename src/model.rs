//! The engine-free maths under the skin: contrast, compositing and meter fill.
//!
//! Plain functions over `[r, g, b]` arrays in sRGB, testable without an app. [`crate::GooeySkin::problems`]
//! is built on them, and a host or an editor may call them to check its own colours.

/// The least contrast body text may have against its background: WCAG 2.2, success criterion 1.4.3.
pub const TEXT_CONTRAST: f32 = 4.5;

/// The least contrast a focus indicator or another part of a control may have against what is next
/// to it: WCAG 2.2, success criteria 1.4.11 and 2.4.13.
pub const NON_TEXT_CONTRAST: f32 = 3.0;

/// The smallest a pointer target may be, in logical pixels: WCAG 2.2, success criterion 2.5.8.
pub const MIN_TARGET_PX: f32 = 24.0;

/// The default [`crate::GooeySkin::min_target`], in logical pixels: a target a thumb or a pad's
/// focus jump lands on comfortably (WCAG 2.2, success criterion 2.5.5, is 44 px).
pub const COMFORT_TARGET_PX: f32 = 44.0;

/// The WCAG relative luminance of an sRGB colour, from 0 (black) to 1 (white).
///
/// ```
/// assert_eq!(bevy_gooey::relative_luminance([1.0, 1.0, 1.0]), 1.0);
/// ```
pub fn relative_luminance(srgb: [f32; 3]) -> f32 {
    let linear = srgb.map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
}

/// The WCAG contrast ratio of two sRGB colours, from 1:1 (the same) to 21:1 (black on white).
///
/// ```
/// let ratio = bevy_gooey::contrast_ratio([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
/// assert!((ratio - 21.0).abs() < 0.01);
/// ```
pub fn contrast_ratio(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `fg` at `alpha` laid over an opaque `bg`, per channel: what the eye sees of a translucent panel.
/// `alpha` is clamped to `0..=1`.
///
/// ```
/// assert_eq!(bevy_gooey::composite([1.0, 1.0, 1.0], 0.5, [0.0, 0.0, 0.0]), [0.5, 0.5, 0.5]);
/// ```
pub fn composite(fg: [f32; 3], alpha: f32, bg: [f32; 3]) -> [f32; 3] {
    let a = alpha.clamp(0.0, 1.0);
    [
        fg[0] * a + bg[0] * (1.0 - a),
        fg[1] * a + bg[1] * (1.0 - a),
        fg[2] * a + bg[2] * (1.0 - a),
    ]
}

/// How full a meter is, from 0 to 1. Never NaN: a non-finite value, a non-finite maximum or a
/// maximum at or below zero reads as empty.
///
/// ```
/// assert_eq!(bevy_gooey::meter_fraction(25.0, 100.0), 0.25);
/// ```
pub fn meter_fraction(value: f32, max: f32) -> f32 {
    if max > 0.0 && max.is_finite() && value.is_finite() {
        (value / max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_on_black_is_21_to_1() {
        assert!((contrast_ratio([1., 1., 1.], [0., 0., 0.]) - 21.0).abs() < 0.01);
        let x = [0.3, 0.6, 0.2];
        assert_eq!(contrast_ratio(x, x), 1.0);
    }

    #[test]
    fn the_meter_fraction_is_clamped_and_never_nan() {
        assert_eq!(meter_fraction(25.0, 100.0), 0.25);
        assert_eq!(meter_fraction(150.0, 100.0), 1.0);
        assert_eq!(meter_fraction(-5.0, 100.0), 0.0);
        assert_eq!(meter_fraction(5.0, 0.0), 0.0);
        assert_eq!(meter_fraction(f32::NAN, 100.0), 0.0);
        assert_eq!(meter_fraction(5.0, f32::INFINITY), 0.0);
    }
}
