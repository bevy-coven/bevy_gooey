//! The engine-free maths under the skin: contrast, compositing, meter fill, the interface's scale
//! against the window and line weights on whole device pixels.
//!
//! Plain functions over `[r, g, b]` arrays in sRGB and plain sizes, testable without an app.
//! [`crate::GooeySkin::problems`] is built on them, and a host or an editor may call them to check
//! its own colours; [`crate::scale_with_window`] on [`window_ui_scale`]; [`crate::apply_skin`] on
//! [`snap_line_width`].

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

/// The `UiScale` that keeps an interface's share of a `width` × `height` window (logical px): the
/// window's shorter side over `reference_short_side`, times the player's `scale`. `None` when there
/// is no usable scale: a window with no size (minimised), or a reference or a scale that is not
/// above zero.
///
/// ```
/// assert_eq!(bevy_gooey::window_ui_scale(2560.0, 1440.0, 720.0, 1.0), Some(2.0));
/// ```
pub fn window_ui_scale(
    width: f32,
    height: f32,
    reference_short_side: f32,
    scale: f32,
) -> Option<f32> {
    let ui = width.min(height) / reference_short_side * scale;
    (reference_short_side > 0.0 && scale > 0.0 && ui > 0.0 && ui.is_finite()).then_some(ui)
}

/// How close to a whole device pixel a line already is to count as that pixel: a thousandth.
const SNAP_TOLERANCE: f32 = 1.0e-3;

/// A line weight `length` (logical px) snapped so it draws as a whole number of device pixels at
/// `scale` device pixels per logical px: CSS's "snap a length as a border width". Above one device
/// pixel it rounds down, below one it rounds up to one, and zero stays zero. Returns logical px.
///
/// Bevy lays a node out at `scale × length` and rounds each edge from the node's position, so a
/// 4.5 px border draws 4 px on one side and 5 px on another; a whole-pixel border draws the same on
/// every side. The result is chosen so that `scale * result`, as `f32`, is exactly the whole number
/// wherever an `f32` allows it, and otherwise the least length above it, never below.
///
/// A length or scale that is not finite and above zero is returned unchanged.
///
/// ```
/// // 3 px at 1.5x would be 4.5 device px: it is drawn 4.
/// let line = bevy_gooey::snap_line_width(3.0, 1.5);
/// assert_eq!(line * 1.5, 4.0);
/// ```
pub fn snap_line_width(length: f32, scale: f32) -> f32 {
    let usable = |x: f32| x.is_finite() && x > 0.0;
    if !usable(length) || !usable(scale) {
        return length;
    }
    let device = (length * scale + SNAP_TOLERANCE).floor().max(1.0);
    let mut logical = device / scale;
    // Division and multiplication each round, so step by ulps until the product is exact, or
    // failing that, the least one above: a product an ulp short rounds a pixel short.
    for _ in 0..4 {
        let drawn = logical * scale;
        if drawn < device {
            logical = logical.next_up();
        } else if drawn > device && logical.next_down() * scale >= device {
            logical = logical.next_down();
        } else {
            break;
        }
    }
    logical
}

/// How long a held D-pad direction waits before it repeats on a [`crate::slider`], in seconds. A
/// design default: the delay of a keyboard's own repeat.
pub const PAD_REPEAT_DELAY: f32 = 0.4;

/// How many repeats a second a held D-pad direction makes on a [`crate::slider`], once the delay
/// is over. A design default: with a twentieth of the range a step, a 0 to 1 slider sweeps in about
/// two seconds.
pub const PAD_REPEAT_RATE: f32 = 12.0;

/// How far along its range a slider's thumb is, from 0 to 1. Never NaN: a value outside the range
/// is clamped to an end, and a range that is not finite or has no length reads as the start.
///
/// ```
/// assert_eq!(bevy_gooey::slider_fraction(25.0, 0.0, 100.0), 0.25);
/// assert_eq!(bevy_gooey::slider_fraction(2.0, 0.0, 1.0), 1.0);
/// ```
pub fn slider_fraction(value: f32, min: f32, max: f32) -> f32 {
    if min.is_finite() && max.is_finite() && value.is_finite() && max > min {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The text a [`crate::slider`] shows for its value: a percentage when the range is 0 to 1 (a
/// volume, a brightness), the plain number otherwise, rounded to two decimals with trailing zeros
/// dropped. A value outside the range reads as the end it is clamped to, and one that is not finite
/// as 0.
///
/// ```
/// assert_eq!(bevy_gooey::slider_text(0.7, 0.0, 1.0), "70%");
/// assert_eq!(bevy_gooey::slider_text(1.25, 0.5, 2.0), "1.25");
/// assert_eq!(bevy_gooey::slider_text(40.0, 0.0, 100.0), "40");
/// ```
pub fn slider_text(value: f32, min: f32, max: f32) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let value = if min.is_finite() && max.is_finite() && min <= max {
        value.clamp(min, max)
    } else {
        value
    };
    if min == 0.0 && max == 1.0 {
        return format!("{}%", (value * 100.0).round());
    }
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    // A value that rounds to nothing is "0", never "-0".
    if text == "-0" || text.is_empty() {
        "0".into()
    } else {
        text.into()
    }
}

/// How many repeats a held button has made between two moments, `held_before` and `held_now`
/// seconds after it went down: none until `delay`, then `per_second` a second, the first at
/// `delay` itself. The press itself is not a repeat. Zero for a delay or rate that is not usable.
///
/// ```
/// // Held 0.35 s then 0.45 s: the first repeat, at 0.4 s, falls in between.
/// assert_eq!(bevy_gooey::repeat_steps(0.35, 0.45, 0.4, 10.0), 1);
/// // Held through a whole second at once, delay 0.5 s: the repeats at 0.5, 0.6 … 1.0 s.
/// assert_eq!(bevy_gooey::repeat_steps(0.0, 1.0, 0.5, 10.0), 6);
/// ```
pub fn repeat_steps(held_before: f32, held_now: f32, delay: f32, per_second: f32) -> u32 {
    let due = |held: f32| {
        if held >= delay && held.is_finite() {
            ((held - delay) * per_second).floor() as u32 + 1
        } else {
            0
        }
    };
    let usable = |x: f32| x.is_finite() && x >= 0.0;
    if usable(delay) && per_second.is_finite() && per_second > 0.0 && held_now > held_before {
        due(held_now).saturating_sub(due(held_before))
    } else {
        0
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

    #[test]
    fn the_shorter_side_over_the_reference_sets_the_scale() {
        assert_eq!(window_ui_scale(1280.0, 720.0, 720.0, 1.0), Some(1.0));
        assert_eq!(window_ui_scale(1920.0, 1080.0, 720.0, 1.0), Some(1.5));
        assert_eq!(window_ui_scale(3840.0, 2160.0, 720.0, 1.0), Some(3.0));
        // Portrait and ultrawide: the shorter side decides.
        assert_eq!(window_ui_scale(720.0, 1280.0, 720.0, 1.0), Some(1.0));
        assert_eq!(window_ui_scale(3440.0, 1440.0, 720.0, 1.0), Some(2.0));
        // The player's size multiplies the window's.
        assert_eq!(window_ui_scale(1920.0, 1080.0, 720.0, 1.25), Some(1.875));
        // A window smaller than the reference draws a smaller interface, in proportion.
        assert_eq!(window_ui_scale(640.0, 360.0, 720.0, 1.0), Some(0.5));
    }

    #[test]
    fn no_window_or_an_invalid_setting_gives_no_scale() {
        assert_eq!(window_ui_scale(0.0, 0.0, 720.0, 1.0), None);
        assert_eq!(window_ui_scale(1280.0, 0.0, 720.0, 1.0), None);
        assert_eq!(window_ui_scale(1280.0, 720.0, 0.0, 1.0), None);
        assert_eq!(window_ui_scale(1280.0, 720.0, -720.0, -1.0), None);
        assert_eq!(window_ui_scale(1280.0, 720.0, 720.0, 0.0), None);
        assert_eq!(window_ui_scale(1280.0, 720.0, f32::NAN, 1.0), None);
        assert_eq!(window_ui_scale(1280.0, 720.0, 720.0, f32::INFINITY), None);
    }

    #[test]
    fn a_line_rounds_down_to_whole_device_pixels_and_never_below_one() {
        // The default 3 px border at the window scales the crate draws.
        let device = |length: f32, scale: f32| snap_line_width(length, scale) * scale;
        assert_eq!(device(3.0, 1.0), 3.0);
        assert_eq!(device(3.0, 800.0 / 720.0), 3.0);
        assert_eq!(device(3.0, 1.25), 3.0);
        assert_eq!(device(3.0, 1.5), 4.0);
        assert_eq!(device(3.0, 2.0), 6.0);
        assert_eq!(device(3.0, 3.0), 9.0);
        // Under one device pixel: one. Zero: none.
        assert_eq!(device(0.25, 1.0), 1.0);
        assert_eq!(device(1.0, 0.5), 1.0);
        assert_eq!(snap_line_width(0.0, 1.5), 0.0);
        // Already whole, within float noise: kept, not rounded a pixel down.
        assert_eq!(device(4.0 / 1.5, 1.5), 4.0);
    }

    #[test]
    fn a_snapped_line_is_never_a_hair_short_of_its_pixel() {
        // An ulp short of `n` is what Bevy's rounding turns into `n − 1` on a half-pixel edge.
        for height in 360..=4320 {
            let scale = height as f32 / 720.0;
            for length in [1.0, 2.0, 3.0, 5.0, 8.0] {
                let drawn = snap_line_width(length, scale) * scale;
                let whole = (length * scale + 1.0e-3).floor().max(1.0);
                assert!(
                    drawn >= whole && drawn - whole < 1.0e-5,
                    "{length} px at {height} px tall draws {drawn}"
                );
            }
        }
        // Exact at the common window heights.
        for height in [720, 768, 800, 900, 1050, 1080, 1200, 1440, 1600, 2160] {
            let scale = height as f32 / 720.0;
            let drawn = snap_line_width(3.0, scale) * scale;
            assert_eq!(
                drawn,
                drawn.floor(),
                "3 px at {height} px tall draws {drawn}"
            );
        }
    }

    #[test]
    fn an_unusable_length_or_scale_is_left_alone() {
        assert_eq!(snap_line_width(-3.0, 1.5), -3.0);
        assert!(snap_line_width(f32::NAN, 1.5).is_nan());
        assert_eq!(snap_line_width(3.0, 0.0), 3.0);
        assert_eq!(snap_line_width(3.0, f32::INFINITY), 3.0);
    }

    #[test]
    fn a_sliders_fraction_is_clamped_and_never_nan() {
        assert_eq!(slider_fraction(25.0, 0.0, 100.0), 0.25);
        assert_eq!(slider_fraction(1.25, 0.5, 2.0), 0.5);
        assert_eq!(slider_fraction(-3.0, 0.0, 1.0), 0.0);
        assert_eq!(slider_fraction(9.0, 0.0, 1.0), 1.0);
        // No range to be along: the start.
        assert_eq!(slider_fraction(5.0, 5.0, 5.0), 0.0);
        assert_eq!(slider_fraction(5.0, 9.0, 1.0), 0.0);
        assert_eq!(slider_fraction(f32::NAN, 0.0, 1.0), 0.0);
        assert_eq!(slider_fraction(0.5, 0.0, f32::INFINITY), 0.0);
    }

    #[test]
    fn a_sliders_text_is_a_percentage_for_zero_to_one_and_the_number_otherwise() {
        assert_eq!(slider_text(0.7, 0.0, 1.0), "70%");
        assert_eq!(slider_text(0.0, 0.0, 1.0), "0%");
        assert_eq!(slider_text(1.0, 0.0, 1.0), "100%");
        // A float that is a hair under 0.7 still reads 70.
        assert_eq!(slider_text(0.1 + 0.6, 0.0, 1.0), "70%");
        assert_eq!(slider_text(40.0, 0.0, 100.0), "40");
        assert_eq!(slider_text(1.25, 0.5, 2.0), "1.25");
        assert_eq!(slider_text(1.5, 0.5, 2.0), "1.5");
        assert_eq!(slider_text(0.333, 0.0, 2.0), "0.33");
        assert_eq!(slider_text(-0.001, -1.0, 1.0), "0");
        assert_eq!(slider_text(-2.5, -5.0, 5.0), "-2.5");
        // Outside the range it reads the end it is clamped to; not finite, 0.
        assert_eq!(slider_text(7.0, 0.0, 1.0), "100%");
        assert_eq!(slider_text(300.0, 0.0, 100.0), "100");
        assert_eq!(slider_text(f32::NAN, 0.0, 100.0), "0");
    }

    #[test]
    fn a_held_direction_repeats_after_the_delay_at_the_same_pace_at_any_frame_rate() {
        // Nothing before the delay, the first repeat at it.
        assert_eq!(repeat_steps(0.0, 0.3, 0.5, 10.0), 0);
        assert_eq!(repeat_steps(0.3, 0.5, 0.5, 10.0), 1);
        // The same total however the time is cut into frames.
        let total = |frame: f32| {
            let (mut held, mut steps) = (0.0_f32, 0);
            while held < 2.0 {
                let next = held + frame;
                steps += repeat_steps(held, next, 0.5, 10.0);
                held = next;
            }
            (steps, held)
        };
        for frame in [1.0 / 144.0, 1.0 / 60.0, 1.0 / 20.0, 0.25, 1.0] {
            let (steps, held) = total(frame);
            let expected = repeat_steps(0.0, held, 0.5, 10.0);
            assert_eq!(steps, expected, "{frame} s frames");
        }
        // A frame that does not advance, or goes back, repeats nothing.
        assert_eq!(repeat_steps(1.0, 1.0, 0.5, 10.0), 0);
        assert_eq!(repeat_steps(1.0, 0.5, 0.5, 10.0), 0);
    }

    #[test]
    fn an_unusable_repeat_setting_repeats_nothing() {
        assert_eq!(repeat_steps(0.0, 5.0, 0.5, 0.0), 0);
        assert_eq!(repeat_steps(0.0, 5.0, 0.5, -10.0), 0);
        assert_eq!(repeat_steps(0.0, 5.0, f32::NAN, 10.0), 0);
        assert_eq!(repeat_steps(0.0, 5.0, 0.5, f32::INFINITY), 0);
        assert_eq!(repeat_steps(0.0, f32::INFINITY, 0.5, 10.0), 0);
    }
}
