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

/// Where a [`crate::choice`] lands when it steps `delta` options from `selected` among `len`:
/// clamped to the first and the last, never wrapping, so a player who steps past the end sees that
/// it is the end. With no options it stays at 0, and a `selected` already past the end counts as
/// the last.
///
/// ```
/// assert_eq!(bevy_gooey::choice_step(1, 3, 1), 2);
/// assert_eq!(bevy_gooey::choice_step(2, 3, 1), 2);
/// assert_eq!(bevy_gooey::choice_step(0, 3, -1), 0);
/// ```
pub fn choice_step(selected: usize, len: usize, delta: i32) -> usize {
    let last = len.saturating_sub(1);
    let from = selected.min(last);
    if delta < 0 {
        from.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        from.saturating_add(delta as usize).min(last)
    }
}

/// The option after `selected` among `len`, wrapping from the last to the first: what Enter, Space,
/// the pad's South button and a click on a [`crate::choice`]'s value do, so one repeated press
/// reaches every option. With no options it stays at 0.
///
/// ```
/// assert_eq!(bevy_gooey::choice_next(0, 3), 1);
/// assert_eq!(bevy_gooey::choice_next(2, 3), 0);
/// ```
pub fn choice_next(selected: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (selected.min(len - 1) + 1) % len
    }
}

/// Where a [`crate::tabs`] widget lands when it steps `delta` tabs from `selected` among `len`:
/// round the strip, wrapping from the last to the first and from the first to the last, however
/// large the step (the bumpers, Ctrl+PageUp and Ctrl+PageDown, the D-pad and the arrows all step
/// one). A strip is one direction, so it loops; a [`crate::choice`] is a value with ends and stops
/// ([`choice_step`]). With no tabs it stays at 0, and a `selected` already past the end counts as
/// the last.
///
/// ```
/// assert_eq!(bevy_gooey::tab_step(0, 3, 1), 1);
/// assert_eq!(bevy_gooey::tab_step(2, 3, 1), 0);
/// assert_eq!(bevy_gooey::tab_step(0, 3, -1), 2);
/// ```
pub fn tab_step(selected: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    let from = selected.min(len - 1) as i64;
    (from + i64::from(delta)).rem_euclid(len as i64) as usize
}

/// The family of controller whose button names a [`crate::binding`] row shows: the same physical
/// button is "A" on an Xbox pad, "Cross" on a PlayStation one and "B" on a Nintendo Switch one.
/// The crate does not detect a pad's make; the host sets it in [`crate::BindingSettings`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PadStyle {
    /// A, B, X, Y, LB, RB, LT, RT, View, Menu.
    #[default]
    Xbox,
    /// Cross, Circle, Square, Triangle, L1, R1, L2, R2, Create, Options.
    PlayStation,
    /// B, A, Y, X, L, R, ZL, ZR, -, +: the face buttons by position, so South is "B".
    Switch,
}

/// The words printed on a key's cap, from the name of its Bevy `KeyCode` variant (`format!("{:?}",
/// key)`): the key's *location* named as a US keyboard prints it, so `KeyW` is "W" on every
/// layout (a stored key is a place, not a letter; see `docs/research/binding.md`).
/// Modifiers lead with their side ("Left Shift"), the arrows end in "Arrow", punctuation is the
/// mark itself, and any other name is cut into words ("PageUp" is "Page Up").
///
/// ```
/// assert_eq!(bevy_gooey::key_name("KeyW"), "W");
/// assert_eq!(bevy_gooey::key_name("Digit1"), "1");
/// assert_eq!(bevy_gooey::key_name("ShiftLeft"), "Left Shift");
/// assert_eq!(bevy_gooey::key_name("ArrowUp"), "Up Arrow");
/// assert_eq!(bevy_gooey::key_name("Escape"), "Esc");
/// assert_eq!(bevy_gooey::key_name("PageDown"), "Page Down");
/// ```
pub fn key_name(code: &str) -> String {
    let fixed = match code {
        "Escape" => "Esc",
        "Delete" => "Del",
        "Insert" => "Ins",
        "ContextMenu" => "Menu",
        "Backquote" => "`",
        "Backslash" | "IntlBackslash" => "\\",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Comma" => ",",
        "Equal" => "=",
        "Minus" => "-",
        "Period" => ".",
        "Quote" => "'",
        "Semicolon" => ";",
        "Slash" => "/",
        "NumpadAdd" => "Num +",
        "NumpadSubtract" => "Num -",
        "NumpadMultiply" => "Num *",
        "NumpadDivide" => "Num /",
        "NumpadDecimal" => "Num .",
        "NumpadComma" => "Num ,",
        "NumpadEqual" => "Num =",
        "NumpadEnter" => "Num Enter",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_owned();
    }
    if code.starts_with("Unidentified") {
        return "Unknown key".to_owned();
    }
    for side in ["Left", "Right"] {
        if let Some(modifier) = code.strip_suffix(side) {
            let name = match modifier {
                "Shift" => "Shift",
                "Control" => "Ctrl",
                "Alt" => "Alt",
                "Super" => "Super",
                "Meta" => "Meta",
                "Hyper" => "Hyper",
                _ => continue,
            };
            return format!("{side} {name}");
        }
    }
    if let Some(direction) = code.strip_prefix("Arrow") {
        return format!("{direction} Arrow");
    }
    for prefix in ["Key", "Digit"] {
        if let Some(rest) = code.strip_prefix(prefix)
            && rest.chars().count() == 1
        {
            return rest.to_owned();
        }
    }
    if let Some(rest) = code.strip_prefix("Numpad")
        && rest.chars().all(|c| c.is_ascii_digit())
        && !rest.is_empty()
    {
        return format!("Num {rest}");
    }
    words(code)
}

/// The words of a mouse button, from the name of its Bevy `MouseButton` variant: "Left Click",
/// "Right Click", "Middle Click", the side buttons as "Mouse 4" and "Mouse 5" and any other by its
/// number ("Mouse 6").
///
/// ```
/// assert_eq!(bevy_gooey::mouse_name("Left"), "Left Click");
/// assert_eq!(bevy_gooey::mouse_name("Back"), "Mouse 4");
/// assert_eq!(bevy_gooey::mouse_name("Other(6)"), "Mouse 6");
/// ```
pub fn mouse_name(button: &str) -> String {
    match button {
        "Left" => "Left Click".to_owned(),
        "Right" => "Right Click".to_owned(),
        "Middle" => "Middle Click".to_owned(),
        "Back" => "Mouse 4".to_owned(),
        "Forward" => "Mouse 5".to_owned(),
        other => match numbered(other) {
            Some(n) => format!("Mouse {n}"),
            None => words(other),
        },
    }
}

/// The words of a gamepad button, from the name of its Bevy `GamepadButton` variant, in the
/// vocabulary of `style`: `South` is "A", "Cross" or "B"; the shoulders "LB", "L1" or "L"; the D-pad
/// "D-pad Up" on all three. A button with no name of its own is "Button n".
///
/// ```
/// use bevy_gooey::model::PadStyle;
/// assert_eq!(bevy_gooey::pad_name("South", PadStyle::Xbox), "A");
/// assert_eq!(bevy_gooey::pad_name("South", PadStyle::PlayStation), "Cross");
/// assert_eq!(bevy_gooey::pad_name("RightTrigger2", PadStyle::Switch), "ZR");
/// assert_eq!(bevy_gooey::pad_name("DPadUp", PadStyle::Xbox), "D-pad Up");
/// ```
pub fn pad_name(button: &str, style: PadStyle) -> String {
    // Xbox, PlayStation, Switch.
    let (xbox, playstation, switch) = match button {
        "South" => ("A", "Cross", "B"),
        "East" => ("B", "Circle", "A"),
        "North" => ("Y", "Triangle", "X"),
        "West" => ("X", "Square", "Y"),
        "C" => ("C", "C", "C"),
        "Z" => ("Z", "Z", "Z"),
        "LeftTrigger" => ("LB", "L1", "L"),
        "LeftTrigger2" => ("LT", "L2", "ZL"),
        "RightTrigger" => ("RB", "R1", "R"),
        "RightTrigger2" => ("RT", "R2", "ZR"),
        "Select" => ("View", "Create", "-"),
        "Start" => ("Menu", "Options", "+"),
        "Mode" => ("Guide", "PS", "Home"),
        "LeftThumb" => ("LS", "L3", "L3"),
        "RightThumb" => ("RS", "R3", "R3"),
        "DPadUp" => ("D-pad Up", "D-pad Up", "D-pad Up"),
        "DPadDown" => ("D-pad Down", "D-pad Down", "D-pad Down"),
        "DPadLeft" => ("D-pad Left", "D-pad Left", "D-pad Left"),
        "DPadRight" => ("D-pad Right", "D-pad Right", "D-pad Right"),
        other => {
            return match numbered(other) {
                Some(n) => format!("Button {n}"),
                None => words(other),
            };
        }
    };
    match style {
        PadStyle::Xbox => xbox,
        PadStyle::PlayStation => playstation,
        PadStyle::Switch => switch,
    }
    .to_owned()
}

/// `n` of a variant printed as `Other(n)`.
fn numbered(name: &str) -> Option<&str> {
    name.strip_prefix("Other(")?.strip_suffix(')')
}

/// A variant name cut into words at each capital that follows a lower-case letter or a digit:
/// "CapsLock" is "Caps Lock".
fn words(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    let mut before: Option<char> = None;
    for c in name.chars() {
        if c.is_uppercase() && before.is_some_and(|b| b.is_lowercase() || b.is_ascii_digit()) {
            out.push(' ');
        }
        out.push(c);
        before = Some(c);
    }
    out
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

/// How long a toast stands fully shown, in seconds: `(base_seconds + characters ÷ chars_per_second)
/// × scale`. The reading time is the message's length at a reading speed (Netflix holds timed text to
/// 20 characters a second for adults), the base is the time to notice a thing at the edge of the eye,
/// and the scale is the player's own (Xbox Accessibility Guideline 116: adjustable up to at least ten
/// times the default).
///
/// Never negative, infinite or NaN: a base or a reading speed that is not finite and above zero
/// counts for nothing (no base, no reading time), and a scale that is not finite and above zero
/// gives no time at all.
///
/// ```
/// // "Game saved" at the defaults: 2 s to notice and half a second to read.
/// assert_eq!(bevy_gooey::toast_hold(10, 2.0, 20.0, 1.0), 2.5);
/// ```
pub fn toast_hold(characters: usize, base_seconds: f32, chars_per_second: f32, scale: f32) -> f32 {
    let usable = |x: f32| x.is_finite() && x > 0.0;
    let base = if usable(base_seconds) {
        base_seconds
    } else {
        0.0
    };
    let reading = if usable(chars_per_second) {
        characters as f32 / chars_per_second
    } else {
        0.0
    };
    if usable(scale) {
        (base + reading) * scale
    } else {
        0.0
    }
}

/// How long a toast lives in all: its fade in, `hold` fully shown and its fade out. A `fade` that is
/// not finite and above zero is no fade.
///
/// ```
/// assert_eq!(bevy_gooey::toast_life(2.5, 0.25), 3.0);
/// ```
pub fn toast_life(hold: f32, fade: f32) -> f32 {
    let fade = if fade.is_finite() && fade > 0.0 {
        fade
    } else {
        0.0
    };
    let hold = if hold.is_finite() && hold > 0.0 {
        hold
    } else {
        0.0
    };
    hold + 2.0 * fade
}

/// How opaque a toast is `age` seconds after it was spawned, from 0 to 1: rising over the first
/// `fade`, whole for `hold`, then falling over the last `fade` (see [`toast_life`]). With no fade
/// it is whole for `hold` and then gone. Never NaN: an age that is not finite or is negative reads
/// as not yet shown.
///
/// ```
/// assert_eq!(bevy_gooey::toast_alpha(0.0, 2.5, 0.25), 0.0);
/// assert_eq!(bevy_gooey::toast_alpha(1.0, 2.5, 0.25), 1.0);
/// assert_eq!(bevy_gooey::toast_alpha(3.0, 2.5, 0.25), 0.0);
/// ```
pub fn toast_alpha(age: f32, hold: f32, fade: f32) -> f32 {
    if !age.is_finite() || age < 0.0 {
        return 0.0;
    }
    let hold = if hold.is_finite() && hold > 0.0 {
        hold
    } else {
        0.0
    };
    if !(fade.is_finite() && fade > 0.0) {
        return if age < hold { 1.0 } else { 0.0 };
    }
    let rising = age / fade;
    let falling = (hold + 2.0 * fade - age) / fade;
    rising.min(falling).clamp(0.0, 1.0)
}

/// How much of its room a toast has opened `age` seconds after it was spawned, from 0 to 1: the
/// fade's envelope ([`toast_alpha`]) eased out (`1 − (1 − alpha)³`), so a new toast opens its room
/// quickly while it is still faint and settles, and a leaving one holds its room while it fades
/// and closes it at the end. The toasts around it glide with it, because the room is layout. With
/// no fade it is whole for `hold` and then gone.
///
/// ```
/// assert_eq!(bevy_gooey::toast_grow(0.0, 2.5, 0.25), 0.0);
/// assert_eq!(bevy_gooey::toast_grow(1.0, 2.5, 0.25), 1.0);
/// assert!(bevy_gooey::toast_grow(0.125, 2.5, 0.25) > 0.8);
/// ```
pub fn toast_grow(age: f32, hold: f32, fade: f32) -> f32 {
    let left = 1.0 - toast_alpha(age, hold, fade);
    1.0 - left * left * left
}

/// How tall a list stands to show `rows` rows, in the units of its inputs: `rows × (row + gap)`,
/// `row` the height of one row and `gap` the space between two, so each row counts with the gap
/// after it. `4.5` is four rows and half a pitch of the fifth, which cuts it, the signifier that
/// more is below (NN/g, *Scrolling and Attention*); `4.0` ends a gap past the fourth row. Zero for
/// a count that is not finite and above zero.
///
/// ```
/// // Four and a half rows of 48 px with 10 px between: four rows and 29 px of the fifth.
/// assert_eq!(bevy_gooey::list_height(4.5, 48.0, 10.0), 4.5 * 58.0);
/// assert_eq!(bevy_gooey::list_height(4.0, 48.0, 10.0), 4.0 * 58.0);
/// assert_eq!(bevy_gooey::list_height(f32::NAN, 48.0, 10.0), 0.0);
/// ```
pub fn list_height(rows: f32, row: f32, gap: f32) -> f32 {
    if rows.is_finite() && rows > 0.0 && row.is_finite() && gap.is_finite() {
        (rows * (row + gap)).max(0.0)
    } else {
        0.0
    }
}

/// The scroll position that brings a span into a list's viewport with context round it, measured
/// from the start of the scrolled content: the span `top..bottom` (a focused widget), a viewport
/// `view` tall at `scroll`, a furthest scroll `max`, and `margin` of list to leave beyond the span.
///
/// The least scroll that leaves `margin` of list on the side the span would leave through, so the
/// player always sees what is next (revealing only to the edge glues the focus to the bottom pixel
/// of the viewport with nothing in sight below it). The margin is cut to what the viewport can hold
/// beside the span. A span taller than the viewport scrolls to its top, the part read first; a move
/// of more than one viewport centres the span instead of creeping to the edge; the answer is
/// always within `0..=max`, and `scroll` itself when the span is already comfortably inside.
/// Nothing usable (a viewport of no height, a value that is not finite) leaves `scroll` alone.
///
/// ```
/// // A 160 px viewport at the top of a 440 px list of 40 px rows, a margin of one row: the fourth
/// // row (120..160) wants the fifth in sight too, 160 + 40 − 160 = 40 px of scroll, and from there
/// // the fifth (160..200) wants 200 + 40 − 160 = 80.
/// assert_eq!(bevy_gooey::scroll_to_reveal(120.0, 160.0, 160.0, 0.0, 280.0, 40.0), 40.0);
/// assert_eq!(bevy_gooey::scroll_to_reveal(160.0, 200.0, 160.0, 40.0, 280.0, 40.0), 80.0);
/// // The third row (80..120) already has a row below it in view: nothing to do.
/// assert_eq!(bevy_gooey::scroll_to_reveal(80.0, 120.0, 160.0, 0.0, 280.0, 40.0), 0.0);
/// // The last row stops at the end of the list, not a margin past it.
/// assert_eq!(bevy_gooey::scroll_to_reveal(400.0, 440.0, 160.0, 200.0, 280.0, 40.0), 280.0);
/// ```
pub fn scroll_to_reveal(
    top: f32,
    bottom: f32,
    view: f32,
    scroll: f32,
    max: f32,
    margin: f32,
) -> f32 {
    let usable = [top, bottom, view, scroll, max, margin]
        .iter()
        .all(|x| x.is_finite());
    let max = max.max(0.0);
    if !usable || view <= 0.0 || bottom < top {
        return if scroll.is_finite() {
            scroll.clamp(0.0, max)
        } else {
            0.0
        };
    }
    let height = bottom - top;
    // A span taller than the viewport is read from its top, whichever way it is reached.
    if height > view {
        return top.clamp(0.0, max);
    }
    // What the viewport can hold beside the span, so a short viewport asks for the most margin it
    // can keep rather than a margin on each side it cannot have both of.
    let margin = margin.max(0.0).min((view - height) * 0.5);
    let want = if top - margin < scroll {
        top - margin
    } else if bottom + margin > scroll + view {
        bottom + margin - view
    } else {
        scroll
    };
    // Further than a viewport is a jump, not a step: put the span in the middle.
    let want = if (want - scroll).abs() > view {
        (top + bottom) * 0.5 - view * 0.5
    } else {
        want
    };
    want.clamp(0.0, max)
}

/// A scroll position moved on to the top of a row, so a list that follows focus always stands with a
/// row flush at the top of its viewport, as it does at rest, and the row it cuts at the bottom is
/// cut the same way at every step: `want` is where [`scroll_to_reveal`] asked to go from `scroll`,
/// and `tops` are the rows' tops in the same units. Moving down it takes the first row top at or
/// after `want`, moving up the last at or before it (half a unit of slack for rounded layout), so
/// it never scrolls less than was asked. `None` when no row top lies that way (the end of the
/// list: the caller keeps `want`, which then ends flush with the last row) or `want` is no move.
///
/// ```
/// let tops = [0.0, 57.0, 114.0, 171.0];
/// // A step of 46.8 down, from the top, goes on to the next row's top.
/// assert_eq!(bevy_gooey::snap_to_row(46.8, 0.0, &tops), Some(57.0));
/// // A step up from 171 that asks for 120 stops at the row top above it.
/// assert_eq!(bevy_gooey::snap_to_row(120.0, 171.0, &tops), Some(114.0));
/// // Past the last row top there is nothing to snap to.
/// assert_eq!(bevy_gooey::snap_to_row(190.0, 114.0, &tops), None);
/// ```
pub fn snap_to_row(want: f32, scroll: f32, tops: &[f32]) -> Option<f32> {
    if !(want.is_finite() && scroll.is_finite()) || (want - scroll).abs() < f32::EPSILON {
        return None;
    }
    let usable = tops.iter().copied().filter(|t| t.is_finite());
    if want > scroll {
        usable
            .filter(|&t| t >= want - 0.5)
            .min_by(|a, b| a.total_cmp(b))
    } else {
        usable
            .filter(|&t| t <= want + 0.5)
            .max_by(|a, b| a.total_cmp(b))
    }
}

/// Which of a list's rows a page away from the focused one is, for PageUp and PageDown: `from` is
/// the focused row's centre along the list, `centres` the rows' centres in the same units, and
/// `page` how far to go, positive for down and negative for up. The row furthest from `from` in
/// that direction that is still within a page of it; if even the next row is further than a page
/// away (tall rows), the next row. `None` when no row lies that way. Rows less than half a unit
/// from `from` count as its own row; of rows at one distance the first is taken.
///
/// ```
/// let rows = [20.0, 70.0, 120.0, 170.0, 220.0, 270.0];
/// // From the first row, a 150-unit page lands on the row at 170.
/// assert_eq!(bevy_gooey::page_target(20.0, &rows, 150.0), Some(3));
/// // Up from the last row, the same page lands on the row at 120.
/// assert_eq!(bevy_gooey::page_target(270.0, &rows, -150.0), Some(2));
/// // Nothing lies below the last row.
/// assert_eq!(bevy_gooey::page_target(270.0, &rows, 150.0), None);
/// // A page shorter than the pitch still takes one step.
/// assert_eq!(bevy_gooey::page_target(20.0, &rows, 10.0), Some(1));
/// ```
pub fn page_target(from: f32, centres: &[f32], page: f32) -> Option<usize> {
    if !(from.is_finite() && page.is_finite() && page != 0.0) {
        return None;
    }
    let ahead = |centre: f32| (centre - from) * page.signum();
    let mut nearest: Option<(usize, f32)> = None;
    let mut farthest: Option<(usize, f32)> = None;
    for (i, &centre) in centres.iter().enumerate() {
        let d = ahead(centre);
        if d.is_nan() || d < 0.5 {
            continue;
        }
        if nearest.is_none_or(|(_, n)| d < n) {
            nearest = Some((i, d));
        }
        if d <= page.abs() && farthest.is_none_or(|(_, f)| d > f) {
            farthest = Some((i, d));
        }
    }
    farthest.or(nearest).map(|(i, _)| i)
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

    #[test]
    fn a_toast_stands_as_long_as_its_message_takes_to_read_times_the_players_scale() {
        // The defaults: 2 s to notice, 20 characters a second.
        assert_eq!(toast_hold(10, 2.0, 20.0, 1.0), 2.5);
        assert_eq!(toast_hold(100, 2.0, 20.0, 1.0), 7.0);
        assert_eq!(toast_hold(0, 2.0, 20.0, 1.0), 2.0);
        // XAG 116: the player may stretch it tenfold, and the message still counts.
        assert_eq!(toast_hold(100, 2.0, 20.0, 10.0), 70.0);
        assert!(toast_hold(40, 2.0, 20.0, 1.0) < toast_hold(41, 2.0, 20.0, 1.0));
    }

    #[test]
    fn an_unusable_hold_setting_never_gives_a_negative_infinite_or_nan_time() {
        for scale in [-1.0, 0.0, f32::NAN, f32::INFINITY] {
            assert_eq!(toast_hold(10, 2.0, 20.0, scale), 0.0);
        }
        // No reading speed means no reading time, not an infinite one.
        for speed in [0.0, -5.0, f32::NAN, f32::INFINITY] {
            assert_eq!(toast_hold(10, 2.0, speed, 1.0), 2.0);
        }
        for base in [-2.0, f32::NAN, f32::NEG_INFINITY] {
            assert_eq!(toast_hold(10, base, 20.0, 1.0), 0.5);
        }
    }

    #[test]
    fn a_toast_fades_in_holds_and_fades_out_over_its_life() {
        let (hold, fade) = (2.0, 0.5);
        assert_eq!(toast_life(hold, fade), 3.0);
        assert_eq!(toast_alpha(0.0, hold, fade), 0.0);
        assert_eq!(toast_alpha(0.25, hold, fade), 0.5);
        assert_eq!(toast_alpha(0.5, hold, fade), 1.0);
        assert_eq!(toast_alpha(2.5, hold, fade), 1.0);
        assert_eq!(toast_alpha(2.75, hold, fade), 0.5);
        assert_eq!(toast_alpha(3.0, hold, fade), 0.0);
        assert_eq!(toast_alpha(10.0, hold, fade), 0.0);
    }

    #[test]
    fn a_toast_with_no_fade_appears_whole_and_goes_at_once() {
        for fade in [0.0, -1.0, f32::NAN] {
            assert_eq!(toast_alpha(0.0, 2.0, fade), 1.0);
            assert_eq!(toast_alpha(1.99, 2.0, fade), 1.0);
            assert_eq!(toast_alpha(2.0, 2.0, fade), 0.0);
            assert_eq!(toast_life(2.0, fade), 2.0);
            assert_eq!(toast_grow(0.0, 2.0, fade), 1.0);
            assert_eq!(toast_grow(2.0, 2.0, fade), 0.0);
        }
    }

    #[test]
    fn a_toast_shortened_while_fading_in_still_ends_and_is_never_out_of_range() {
        // A hold cut to nothing: the fade in meets the fade out, and alpha stays in 0..=1.
        for step in 0..=40 {
            let age = step as f32 * 0.025;
            let alpha = toast_alpha(age, 0.0, 0.5);
            assert!((0.0..=1.0).contains(&alpha), "age {age}: {alpha}");
        }
        assert_eq!(toast_alpha(1.0, 0.0, 0.5), 0.0);
        assert_eq!(toast_alpha(f32::NAN, 2.0, 0.5), 0.0);
        assert_eq!(toast_alpha(-1.0, 2.0, 0.5), 0.0);
    }

    #[test]
    fn a_toast_opens_its_room_early_holds_it_and_closes_it_at_the_end() {
        let (hold, fade) = (2.0, 0.5);
        assert_eq!(toast_grow(0.0, hold, fade), 0.0);
        assert_eq!(toast_grow(1.0, hold, fade), 1.0);
        assert_eq!(toast_grow(3.0, hold, fade), 0.0);
        // Entering: more than half the room is open by the time a quarter of the fade is.
        assert!(toast_grow(0.125, hold, fade) > 0.5);
        // Leaving: most of the room is still there half way through the fade out, and it only
        // closes as the toast is nearly gone.
        assert!(toast_grow(2.75, hold, fade) > 0.8);
        assert!(toast_grow(2.95, hold, fade) < 0.5);
        // Never out of range, and never opens while the toast fades out.
        let mut last = 0.0;
        for step in 0..=20 {
            let grow = toast_grow(step as f32 * 0.025, hold, fade);
            assert!((0.0..=1.0).contains(&grow));
            assert!(
                grow >= last,
                "the room closed while entering at step {step}"
            );
            last = grow;
        }
        for age in [f32::NAN, -1.0, 100.0] {
            assert_eq!(toast_grow(age, hold, fade), 0.0);
        }
    }

    #[test]
    fn a_choice_steps_to_its_ends_and_stops_there() {
        // One step each way inside the list, then held against either end.
        assert_eq!(choice_step(1, 4, 1), 2);
        assert_eq!(choice_step(1, 4, -1), 0);
        assert_eq!(choice_step(3, 4, 1), 3);
        assert_eq!(choice_step(0, 4, -1), 0);
        // A jump of any size lands on an end, and a step of nothing stays put.
        assert_eq!(choice_step(1, 4, i32::MAX), 3);
        assert_eq!(choice_step(2, 4, i32::MIN), 0);
        assert_eq!(choice_step(2, 4, 0), 2);
        // A value past the end (a game set it carelessly) is read as the last option.
        assert_eq!(choice_step(9, 4, 0), 3);
        assert_eq!(choice_step(9, 4, -1), 2);
        // Nothing to choose from: nowhere to go, and no panic.
        assert_eq!(choice_step(0, 0, 1), 0);
        assert_eq!(choice_step(5, 0, -1), 0);
    }

    #[test]
    fn a_choice_cycles_round_when_activated() {
        assert_eq!(choice_next(0, 3), 1);
        assert_eq!(choice_next(2, 3), 0);
        assert_eq!(choice_next(0, 1), 0);
        assert_eq!(
            choice_next(9, 3),
            0,
            "past the end reads as the last, then wraps"
        );
        assert_eq!(choice_next(0, 0), 0);
    }

    #[test]
    fn tabs_step_round_the_strip_in_both_directions() {
        assert_eq!(tab_step(1, 3, 1), 2);
        assert_eq!(tab_step(1, 3, -1), 0);
        // Off either end it wraps to the other.
        assert_eq!(tab_step(2, 3, 1), 0);
        assert_eq!(tab_step(0, 3, -1), 2);
        // A step of nothing stays put, and a whole turn is back where it started.
        assert_eq!(tab_step(1, 3, 0), 1);
        assert_eq!(tab_step(1, 3, 3), 1);
        assert_eq!(tab_step(1, 3, -7), 0);
        assert_eq!(tab_step(0, 3, i32::MIN), 1);
        // One tab is always the one tab.
        assert_eq!(tab_step(0, 1, 1), 0);
        assert_eq!(tab_step(0, 1, -1), 0);
        // A value past the end (a game set it carelessly) is read as the last tab.
        assert_eq!(tab_step(9, 3, 0), 2);
        assert_eq!(tab_step(9, 3, 1), 0);
        // No tabs: nowhere to go, and no panic.
        assert_eq!(tab_step(0, 0, 1), 0);
        assert_eq!(tab_step(5, 0, -1), 0);
    }

    #[test]
    fn a_list_stands_a_pitch_tall_for_each_row() {
        assert_eq!(list_height(1.0, 48.0, 10.0), 58.0);
        assert_eq!(list_height(2.0, 48.0, 10.0), 116.0);
        assert_eq!(list_height(4.5, 48.0, 10.0), 261.0);
        assert_eq!(list_height(0.5, 48.0, 10.0), 29.0);
        for rows in [0.0, -2.0, f32::NAN, f32::INFINITY] {
            assert_eq!(list_height(rows, 48.0, 10.0), 0.0, "rows {rows}");
        }
    }

    #[test]
    fn a_reveal_keeps_the_next_row_in_sight_and_stays_put_when_it_can() {
        // Rows of 40 with a 160 viewport, a margin of one row, in a 440 list (280 of scroll).
        let reveal =
            |top: f32, scroll: f32| scroll_to_reveal(top, top + 40.0, 160.0, scroll, 280.0, 40.0);
        assert_eq!(reveal(0.0, 0.0), 0.0);
        assert_eq!(reveal(40.0, 0.0), 0.0, "a row below it in view: stay");
        assert_eq!(
            reveal(80.0, 0.0),
            0.0,
            "the third row still has the fourth in view"
        );
        assert_eq!(
            reveal(120.0, 0.0),
            40.0,
            "the fourth brings its next row in"
        );
        assert_eq!(reveal(160.0, 40.0), 80.0);
        // Walking back up: the margin is kept above the row too.
        assert_eq!(reveal(120.0, 200.0), 80.0);
        assert_eq!(
            reveal(0.0, 80.0),
            0.0,
            "the first row stops at the top, not a margin above"
        );
        // The ends of the list clamp: nothing scrolls past them.
        assert_eq!(reveal(400.0, 200.0), 280.0);
    }

    #[test]
    fn a_reveal_of_a_far_row_centres_it_and_a_tall_row_shows_its_top() {
        // A jump of more than a viewport (a click, a page, a rebuilt list) centres the row.
        assert_eq!(
            scroll_to_reveal(300.0, 340.0, 100.0, 0.0, 400.0, 0.0),
            270.0
        );
        assert_eq!(
            scroll_to_reveal(100.0, 140.0, 100.0, 400.0, 400.0, 0.0),
            70.0
        );
        // A row taller than the viewport is read from its top, whichever way it is reached.
        assert_eq!(
            scroll_to_reveal(150.0, 300.0, 100.0, 0.0, 300.0, 40.0),
            150.0
        );
        assert_eq!(
            scroll_to_reveal(150.0, 300.0, 100.0, 250.0, 300.0, 40.0),
            150.0
        );
        // A viewport too short for row and margin keeps what margin it can.
        assert_eq!(scroll_to_reveal(80.0, 120.0, 60.0, 0.0, 300.0, 40.0), 70.0);
    }

    #[test]
    fn a_reveal_never_leaves_the_list_and_ignores_what_it_cannot_read() {
        assert_eq!(scroll_to_reveal(0.0, 40.0, 100.0, 50.0, 300.0, 40.0), 0.0);
        assert_eq!(
            scroll_to_reveal(390.0, 430.0, 100.0, 0.0, 300.0, 40.0),
            300.0
        );
        // A list that fits has nowhere to scroll.
        assert_eq!(scroll_to_reveal(60.0, 100.0, 200.0, 30.0, 0.0, 40.0), 0.0);
        assert_eq!(scroll_to_reveal(60.0, 100.0, 200.0, 30.0, -5.0, 40.0), 0.0);
        // No viewport, or a value that is not a number: the scroll stays (clamped), never NaN.
        assert_eq!(scroll_to_reveal(60.0, 100.0, 0.0, 30.0, 300.0, 40.0), 30.0);
        assert_eq!(
            scroll_to_reveal(f32::NAN, 100.0, 100.0, 30.0, 300.0, 40.0),
            30.0
        );
        assert_eq!(
            scroll_to_reveal(60.0, 100.0, 100.0, f32::NAN, 300.0, 40.0),
            0.0
        );
        assert_eq!(
            scroll_to_reveal(100.0, 60.0, 100.0, 30.0, 300.0, 40.0),
            30.0
        );
    }

    #[test]
    fn a_page_goes_a_viewport_and_stops_at_the_ends() {
        let rows = [20.0, 70.0, 120.0, 170.0, 220.0, 270.0];
        assert_eq!(page_target(20.0, &rows, 150.0), Some(3));
        assert_eq!(
            page_target(170.0, &rows, 150.0),
            Some(5),
            "the last row within a page"
        );
        assert_eq!(page_target(220.0, &rows, 150.0), Some(5));
        assert_eq!(
            page_target(270.0, &rows, 150.0),
            None,
            "nothing below the last row"
        );
        assert_eq!(page_target(270.0, &rows, -150.0), Some(2));
        assert_eq!(
            page_target(70.0, &rows, -150.0),
            Some(0),
            "the first row within a page"
        );
        assert_eq!(page_target(20.0, &rows, -150.0), None);
        // Tall rows, a short page: still one row.
        assert_eq!(page_target(20.0, &rows, 10.0), Some(1));
        assert_eq!(page_target(270.0, &rows, -10.0), Some(4));
        // Two columns of rows share their centres: the first at the distance is taken, and the
        // focused row's own neighbours at the same height are not "beyond".
        let grid = [20.0, 20.0, 70.0, 70.0];
        assert_eq!(page_target(20.0, &grid, 100.0), Some(2));
        // Nothing to page over, or a page that is no distance.
        assert_eq!(page_target(20.0, &[], 100.0), None);
        assert_eq!(page_target(20.0, &rows, 0.0), None);
        assert_eq!(page_target(f32::NAN, &rows, 100.0), None);
    }

    #[test]
    fn a_scroll_moves_on_to_a_row_top_and_never_short_of_what_was_asked() {
        let tops = [0.0, 57.0, 114.0, 171.0, 228.0];
        assert_eq!(snap_to_row(1.0, 0.0, &tops), Some(57.0));
        assert_eq!(
            snap_to_row(57.0, 0.0, &tops),
            Some(57.0),
            "already on a top"
        );
        assert_eq!(
            snap_to_row(56.6, 0.0, &tops),
            Some(57.0),
            "rounded layout, half a unit of slack"
        );
        assert_eq!(snap_to_row(58.0, 0.0, &tops), Some(114.0));
        // Up takes the top at or before the ask, so the row asked for stays in view.
        assert_eq!(snap_to_row(120.0, 228.0, &tops), Some(114.0));
        assert_eq!(snap_to_row(114.0, 228.0, &tops), Some(114.0));
        assert_eq!(snap_to_row(1.0, 228.0, &tops), Some(0.0));
        // No row top that way: the end of the list.
        assert_eq!(snap_to_row(229.0, 171.0, &tops), None);
        assert_eq!(snap_to_row(-5.0, 57.0, &tops), None);
        // No move, nothing to snap to, or nothing readable.
        assert_eq!(snap_to_row(57.0, 57.0, &tops), None);
        assert_eq!(snap_to_row(60.0, 0.0, &[]), None);
        assert_eq!(snap_to_row(f32::NAN, 0.0, &tops), None);
        assert_eq!(snap_to_row(60.0, 0.0, &[f32::NAN, 114.0]), Some(114.0));
    }
    #[test]
    fn a_key_is_named_as_its_cap_is_printed() {
        // Letters and digits are the mark, whatever the layout: a stored key is a place.
        assert_eq!(key_name("KeyQ"), "Q");
        assert_eq!(key_name("Digit0"), "0");
        assert_eq!(key_name("F5"), "F5");
        assert_eq!(key_name("F13"), "F13");
        // Modifiers lead with their side, so Left Shift and Right Shift are two caps.
        assert_eq!(key_name("ShiftLeft"), "Left Shift");
        assert_eq!(key_name("ControlRight"), "Right Ctrl");
        assert_eq!(key_name("AltLeft"), "Left Alt");
        assert_eq!(key_name("SuperRight"), "Right Super");
        // The arrows are not the "Left" and "Right" of a side.
        assert_eq!(key_name("ArrowLeft"), "Left Arrow");
        assert_eq!(key_name("ArrowDown"), "Down Arrow");
        // The numpad is told from the row of digits.
        assert_eq!(key_name("Numpad7"), "Num 7");
        assert_eq!(key_name("NumpadAdd"), "Num +");
        assert_eq!(key_name("NumpadEnter"), "Num Enter");
        // Punctuation is the mark, and a few long names are the short form on the cap.
        assert_eq!(key_name("Semicolon"), ";");
        assert_eq!(key_name("Backquote"), "`");
        assert_eq!(key_name("Escape"), "Esc");
        assert_eq!(key_name("Delete"), "Del");
        assert_eq!(key_name("Space"), "Space");
        assert_eq!(key_name("Enter"), "Enter");
        assert_eq!(key_name("Tab"), "Tab");
        // Any other name is cut into words.
        assert_eq!(key_name("PageUp"), "Page Up");
        assert_eq!(key_name("CapsLock"), "Caps Lock");
        assert_eq!(key_name("PrintScreen"), "Print Screen");
        assert_eq!(key_name("MediaPlayPause"), "Media Play Pause");
        assert_eq!(key_name("Unidentified(Windows(255))"), "Unknown key");
    }

    #[test]
    fn a_mouse_button_is_named_by_what_a_player_calls_it() {
        assert_eq!(mouse_name("Left"), "Left Click");
        assert_eq!(mouse_name("Right"), "Right Click");
        assert_eq!(mouse_name("Middle"), "Middle Click");
        assert_eq!(mouse_name("Back"), "Mouse 4");
        assert_eq!(mouse_name("Forward"), "Mouse 5");
        assert_eq!(mouse_name("Other(9)"), "Mouse 9");
    }

    #[test]
    fn a_pad_button_is_named_in_the_vocabulary_of_its_make() {
        // One place, three names: the face buttons by position.
        for (button, xbox, playstation, switch) in [
            ("South", "A", "Cross", "B"),
            ("East", "B", "Circle", "A"),
            ("North", "Y", "Triangle", "X"),
            ("West", "X", "Square", "Y"),
            ("LeftTrigger", "LB", "L1", "L"),
            ("RightTrigger2", "RT", "R2", "ZR"),
            ("Select", "View", "Create", "-"),
            ("Start", "Menu", "Options", "+"),
        ] {
            assert_eq!(pad_name(button, PadStyle::Xbox), xbox, "{button}");
            assert_eq!(
                pad_name(button, PadStyle::PlayStation),
                playstation,
                "{button}"
            );
            assert_eq!(pad_name(button, PadStyle::Switch), switch, "{button}");
        }
        // The D-pad reads the same on every pad, and an unnamed button has a number.
        for style in [PadStyle::Xbox, PadStyle::PlayStation, PadStyle::Switch] {
            assert_eq!(pad_name("DPadLeft", style), "D-pad Left");
            assert_eq!(pad_name("Other(14)", style), "Button 14");
        }
        assert_eq!(PadStyle::default(), PadStyle::Xbox);
    }
}
