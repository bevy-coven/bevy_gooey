# Changelog

## [Unreleased]

### Added

- `model::snap_line_width`: a line weight snapped to whole device pixels at a scale, CSS's "snap a length as a border width" (rounded down above one device pixel, up to one below it).

### Fixed

- Every line is one weight at every scale. At a scale that is not a whole number (1920x1080 draws at 1.5x, a 1280x800 Steam Deck at 1.11x, a 150 % display), Bevy's layout rounds each border and padding from the node's sub-pixel position, so a meter's gap was 5 px above its fill and 4 px below at 1080p, and any border could split the same way depending on where its widget landed. `apply_skin` now draws every border (panel, button and its focus ring, meter frame, tooltip) and the meter's gap a whole number of device pixels wide at the node's own scale, and restyles a widget when that scale changes: the 1080p meter is 4 / 4 / 20 / 4 / 4 px, its fill centred.

## [0.2.0] - 2026-10-30

### Added

- `GooeyScalePlugin`, in `GooeyPlugins`: the whole interface keeps its share of the screen at every resolution. `scale_with_window` sets Bevy's `UiScale` to the primary window's shorter side over `UiScaleSettings::reference_short_side` (720), times the player's `UiScaleSettings::scale` (1.0), so a 1920x1080 window draws the skin's sizes at 1.5x and a 2560x1440 one at 2x, and the default text meets the Xbox Accessibility Guidelines' console minimum (26 px at 1080p) on any screen. A minimised window keeps the last scale. `UiScaleSettings` is serde data with the `serde` feature.
- `model::window_ui_scale`, the engine-free maths under it.
- `FocusScopes`, the system parameter `navigate_by_input` takes (which widgets focus may rest on, which open `dialog` is on top, where each scope starts), exported so the function's signature is nameable. The gallery example stages a pointer click with `GOOEY_KEYS=click` and no longer forces `InputFocusVisible`.
- `update_tooltip_text`, the `GooeySystems::Style` system that copies a changed `Tooltip`'s text into its box.

### Changed

- `UiScale` is set by `GooeyScalePlugin` rather than left to the host. A host that drives `UiScale` itself leaves the plugin out (`GooeyPlugins.build().disable::<GooeyScalePlugin>()`); a player's interface size goes in `UiScaleSettings::scale`. At 1280x720 nothing changes.
- `navigate_by_input`, `gamepad_select` and `focus_opened_dialogs` write Bevy's `InputFocusVisible`, so they need that resource. `GooeyNavPlugin` and `GooeyStylePlugin` add it; a host that registers the systems itself calls `init_resource::<InputFocusVisible>()`. `navigate_by_input` takes `FocusScopes` in place of its `TabGroup` and `ChildOf` queries.

### Fixed

- An arrow key, a D-pad press and the pad's South button now show the focus ring and open a focused widget's `Tooltip` (they set Bevy's `InputFocusVisible`, which only Tab set before, so a pad player never saw focus at all and a keyboard player lost it after any click); a pointer press still hides it. A dialog that takes focus shows it too. The flag is written only when it changes, so a press with focus already shown restyles nothing. Fixes #1.
- With nothing focused (a click on empty space, a focused widget despawned, a screen spawned without `AutoFocus`), the first arrow or D-pad press lands where the screen starts: the topmost open `dialog`'s default, or else the screen's `AutoFocus` widget or its first widget in Tab order; the next press moves on. Before, no direction could ever reach a widget again, so one click stranded a pad player. A focused tab stop that is not directionally navigable (a text field) keeps its arrows. Fixes #2.
- A `Tooltip` is a view of its component. Removing it removes its box (the orphaned box kept opening on hover and on focus, with the old text); removing it and adding another leaves one box, not two; inserting a new `Tooltip` over an old one, or editing `Tooltip.0` in place (as a reflection-driven inspector does), changes the open box's line in place, without a blink. A despawned widget leaves no box. Fixes #3.
- `screen()` no longer blocks the pointer. It is a full-window layout root, and in Bevy 0.20 a UI node without `Pickable` hides everything under it from picking, so a HUD built on it kept every sprite, mesh and other UI root beneath it from receiving `Over`, `Press` or `Click`, and any click on the world cleared focus through the HUD instead. It is now `Pickable::IGNORE`; `panel`, `button`, `meter`, the other widgets and `dialog()`'s scrim still take the pointer, and a click on the empty part still blurs the interface (it lands on the world, or the window, and Bevy clears focus there). A screen that must swallow the pointer opts back in with `commands.spawn(screen()).insert(Pickable::default())`. **A `(screen(), Pickable::IGNORE)` workaround now panics on the duplicate component: delete it.** The gallery's `GOOEY_WORLD=1` puts a sprite under the interface for a click to land on. Fixes #4.

## [0.1.0] - 2026-10-30

### Added

- `GooeySkin`, the look as serde data, with the `dark()` and `parchment()` presets, a `text_scale` player option, `outline` (the edge of a widget whose fill does not show its size, such as a meter's frame), `meter_height`, `problems()` (WCAG 2.2 contrast and target-size checks, as `SkinProblem`, including a meter's fill against its track, its frame against the panel, and a meter too thin for its fill) and `solid_surface()`, the opaque fill of a panel in a dialog.
- `GooeyFonts`, the host's body and heading fonts.
- `Role` and `apply_skin`, the one place every widget is styled.
- `screen()`, a full-window, centring tab group.
- `panel()`, a framed, padded column.
- `menu()`, a column of buttons.
- `heading()` and `label()`, text in the skin's sizes and fonts.
- `button()`, Bevy's headless button, styled by its rest, hover, press, disabled and focus states.
- `meter()` and `Meter`, a HUD bar whose fill follows `value / max`, never NaN. The track is framed in `outline`, so its whole length reads against the panel; the fill sits one `border` inside the frame with concentric rounded corners; the bar is `meter_height × text_scale` high, so it grows with the player's size option.
- `Tooltip`, a line of help beside a widget, shown while the pointer is over it or while it has visible focus, so a keyboard or pad player sees it too. It opens across its container's flow (to the right in a column, below in a row) one `gap` outside the widget's panel, flips at the window's edge (Bevy's `Popover`), and never covers a neighbouring widget. It is drawn as a small raised panel with a drop shadow. `TooltipBox` marks its box; `show_tooltips` opens and shuts it.
- `dialog()`, a modal veil that holds focus: on opening, focus moves to its first button (or its `AutoFocus` child); Tab, the arrows and the D-pad stay inside it; on closing, focus returns to the widget that opened it. A panel in it is drawn opaque, so nothing behind shows through its text. `Dialog` is its focus bookkeeping; `focus_opened_dialogs` moves focus in.
- `navigate_by_input` (arrows and D-pad move focus, within the same scope Tab cycles) and `gamepad_select` (South activates).
- `UiFeedback` and `UiFeedbackKind`, written for every focus move and activation, for the host's sound.
- `GooeyStylePlugin`, `GooeyNavPlugin`, the `GooeyPlugins` group and the `GooeySystems` sets.
- `model`: `relative_luminance`, `contrast_ratio`, `composite`, `meter_fraction` and the WCAG constants.
