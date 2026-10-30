# Changelog

## [Unreleased]

### Added

- `GooeySkin`, the look as serde data, with the `dark()` and `parchment()` presets, a `text_scale` player option and `problems()` (WCAG 2.2 contrast and target-size checks, as `SkinProblem`).
- `GooeyFonts`, the host's body and heading fonts.
- `Role` and `apply_skin`, the one place every widget is styled.
- `screen()`, a full-window, centring tab group.
- `panel()`, a framed, padded column.
- `menu()`, a column of buttons.
- `heading()` and `label()`, text in the skin's sizes and fonts.
- `button()`, Bevy's headless button, styled by its rest, hover, press, disabled and focus states.
- `meter()` and `Meter`, a HUD bar whose fill follows `value / max`, never NaN.
- `Tooltip`, a box shown under a widget while the pointer is over it.
- `dialog()`, a modal veil that keeps Tab inside it.
- `navigate_by_input` (arrows and D-pad move focus) and `gamepad_select` (South activates).
- `UiFeedback` and `UiFeedbackKind`, written for every focus move and activation, for the host's sound.
- `GooeyStylePlugin`, `GooeyNavPlugin`, the `GooeyPlugins` group and the `GooeySystems` sets.
- `model`: `relative_luminance`, `contrast_ratio`, `composite`, `meter_fraction` and the WCAG constants.
