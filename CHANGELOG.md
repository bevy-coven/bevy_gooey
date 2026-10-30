# Changelog

## [Unreleased]

### Added

- `GooeySkin`, the look as serde data, with the `dark()` and `parchment()` presets, a `text_scale` player option, `meter_height`, `problems()` (WCAG 2.2 contrast and target-size checks, as `SkinProblem`, including a meter's fill against its track) and `solid_surface()`, the opaque fill of a panel in a dialog.
- `GooeyFonts`, the host's body and heading fonts.
- `Role` and `apply_skin`, the one place every widget is styled.
- `screen()`, a full-window, centring tab group.
- `panel()`, a framed, padded column.
- `menu()`, a column of buttons.
- `heading()` and `label()`, text in the skin's sizes and fonts.
- `button()`, Bevy's headless button, styled by its rest, hover, press, disabled and focus states.
- `meter()` and `Meter`, a HUD bar whose fill follows `value / max`, never NaN. The fill sits one `border` inside its track with concentric rounded corners, and the bar is `meter_height × text_scale` high, so it grows with the player's size option.
- `Tooltip`, a line of help beside a widget, shown while the pointer is over it or while it has visible focus, so a keyboard or pad player sees it too. It opens across its container's flow (to the right in a column, below in a row) one `gap` outside the widget's panel, flips at the window's edge (Bevy's `Popover`), and never covers a neighbouring widget. It is drawn as a small raised panel with a drop shadow. `TooltipBox` marks its box; `show_tooltips` opens and shuts it.
- `dialog()`, a modal veil that holds focus: on opening, focus moves to its first button (or its `AutoFocus` child); Tab, the arrows and the D-pad stay inside it; on closing, focus returns to the widget that opened it. A panel in it is drawn opaque, so nothing behind shows through its text. `Dialog` is its focus bookkeeping; `focus_opened_dialogs` moves focus in.
- `navigate_by_input` (arrows and D-pad move focus, within the same scope Tab cycles) and `gamepad_select` (South activates).
- `UiFeedback` and `UiFeedbackKind`, written for every focus move and activation, for the host's sound.
- `GooeyStylePlugin`, `GooeyNavPlugin`, the `GooeyPlugins` group and the `GooeySystems` sets.
- `model`: `relative_luminance`, `contrast_ratio`, `composite`, `meter_fraction` and the WCAG constants.
