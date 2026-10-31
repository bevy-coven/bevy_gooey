# Changelog

## [Unreleased]

### Added

- `choice(label, options, selected)`: the third settings control, for a setting with a few named values (Low, Medium, High). A row with the label on the left and the selected option on the right between two drawn chevrons (two borders of the skin's line weight on a square turned an eighth of a turn, so they are no font glyph and scale with the text). Its root is Bevy's headless `Button` carrying the new `ChoiceValue` (the selected index, the widget's state: read it, or write one to move the choice from code) and `ChoiceCount`, so it takes `observe`, `Tooltip` and `InteractionDisabled`, and says each change once through `ValueChange<usize>`. With it focused, Left and Right (keys, and the D-pad through `gamepad_adjust`, a held direction repeating) step to the previous and next option and stop at the ends, where the chevron that cannot step is dimmed and a press changes and says nothing; Home and End jump to the ends; Up and Down still move focus on. A click on a chevron steps once, on release; Enter, Space, the pad's South button and a click anywhere else on the row cycle to the next option and wrap round. Every option is laid out in one grid cell, so the row, the chevrons and the value never move while stepping. The row is a toggle's height and shares its edges; the chevrons are `min_target` wide.
- `update_choices` (in `GooeySystems::Style`, before `apply_skin`) shows a choice's selected option and pulls a value past the last option back to it; `model::{choice_step, choice_next}`, the engine-free maths under it.
- `Role::{Choice, ChoiceArrow, ChoiceBack, ChoiceForward, ChoiceCell, ChoiceOption}`, styled in `apply_skin` like every other part.
- `UiFeedback`: a choice gaining focus writes `Focus`, and each change of its option writes `Activate`, once.
- The gallery example has a Quality choice (it logs and toasts the option it settles on) and `GOOEY_KEYS` tokens `space`, `home`, `end`, `back` and `forward` (a click on a chevron).
- `list(rows, children![..])`: rows that scroll, for save slots, an inventory, a key-binding screen. A wrapper (`Role::List`) holding a clipped viewport (the new public `ListView { rows }`, on Bevy's `ScrollArea`, so the wheel and a trackpad scroll it) and a rail with a thumb (Bevy's `Scrollbar`, so a drag on the thumb and a click on the rail scroll it). The viewport stands `rows` rows tall (a row is a button's height and the `gap` after it, following `text_scale`), so `4.5` cuts the fifth row, the sign that more is below. The rail, `2 × border` wide, shows only while the rows do not fit; its lane is always reserved so rows never change width. The thumb is `text_muted` on a `surface_raised` track (the pair `problems()` already holds at 3:1), `text` under the pointer and `accent` while dragged.
- `scroll_to_focus` (in `GooeySystems::Navigate`): a list follows the focus a key, the D-pad or Tab gives one of its rows, by the least scroll that leaves a row and a half of list beyond it (the next row whole, the one after cut), clamped to the list's ends; a row taller than the list scrolls to its top and a jump of more than a viewport centres the row. A focus the pointer set is not followed, so a press never scrolls the content from under its own release. `page_lists` (same set): PageUp, PageDown and the pad's left and right triggers move focus a viewport. `update_lists` (in `GooeySystems::Style`) shows each rail only while there is more to scroll, writing it only on a change.
- `Up` and `Down` stay among a list's rows until they run out, and a row a list has clipped away is not a neighbour from outside the list (`navigate_by_input`): geometry alone stepped out of a list past rows it still held, onto the button below.
- `model::{list_height, scroll_to_reveal, page_target}`, the engine-free maths under it (tests and doctests); `Role::{List, ListView, ListRows, ListRail, ListThumb}`, styled in `apply_skin` like every other part.
- The gallery example has a Load game list of twelve save slots beside its panel, and `GOOEY_KEYS` tokens `slots`, `pagedown`, `pageup` and `wheel`.

### Changed

- `Role` has six more variants: a `match` over it that is exhaustive needs arms for them.
- `Role` has five more variants for the list: a `match` over it that is exhaustive needs arms for them.
- `FocusScopes` gained a query over lists, and `show_focus` is crate-visible: callers of `navigate_by_input` are unchanged.
- `gamepad_adjust` steps a focused, enabled choice as well as a slider (its query over sliders became one over both, so its signature changed) and `FocusScopes` keeps Left and Right for a focused, enabled choice as it does for a slider; a host that registers these systems by hand gets them from `GooeyNavPlugin` as before, and calling them directly is unchanged.

### Fixed

- A `Tooltip` on a widget inside a clipped container (a row of a `list`) was clipped by it, so its box, which stands beside the widget and outside the container, never showed: the box now carries Bevy's `OverrideClip`.

## [0.3.0] - 2026-10-31

### Added

- `model::snap_line_width`: a line weight snapped to whole device pixels at a scale, CSS's "snap a length as a border width" (rounded down above one device pixel, up to one below it).
- `toggle(label, on)` and `slider(label, value, range)`: the two controls an options screen is made of. Each returns one bundle whose root is Bevy's own headless widget (`Checkbox`, `Slider`), so the spawned entity carries the state (`Checked`, `SliderValue`), takes `observe`, `Tooltip` and `InteractionDisabled`, and reports `ValueChange<bool>` / `ValueChange<f32>` (re-exported with `Checked`, `SliderValue`, `SliderRange` and `SliderStep`). A toggle is a row with a label and a framed switch whose knob slides to the end and takes the accent when on; a slider is a row with a label, a readout (a percentage for a range of 0 to 1, the number otherwise) and a rail with a thumb, the whole row the click and drag target. Both are at least `min_target` high, wear a button's focus ring, show the pointer with an outline, dim when disabled and grow with `text_scale`.
- A focused slider takes Left and Right (keys through `bevy_ui_widgets`: a twentieth of the range a step, Home and End to the ends) and the D-pad (`gamepad_adjust`, new, in `GooeySystems::Navigate`), which repeats a held direction after `PAD_REPEAT_DELAY` (0.4 s) at `PAD_REPEAT_RATE` (12 a second), so a pad player adjusts a volume with no select step and no held button. Up and Down still move focus on. The pad's South button flips a focused toggle.
- `update_sliders` (in `GooeySystems::Style`) keeps a slider's thumb, fill and readout in step with its value; `model::slider_fraction`, `slider_text` and `repeat_steps`, the engine-free maths under it.
- `Role::{Toggle, ToggleTrack, ToggleKnob, Slider, SliderHeader, SliderTrack, SliderRail, SliderFill, SliderGap, SliderThumb, SliderReadout, ControlText}`, styled in `apply_skin` like every other part.
- `UiFeedback`: a toggle or a slider gaining focus writes `Focus`, a toggle flipping writes `Activate`.
- `GooeySkin::problems()` checks one more pair, the off switch's knob (`text_muted`) against its track (`surface_raised`) at 3:1. Both presets pass; a custom skin whose muted colour sits near its raised surface now warns.
- `toast(text)` and `toast_alert(text)`: a line of news at the edge of the screen, spawned in one call from any system (`commands.spawn(toast("Game saved"))`) with no layout and nothing to set up. The first one creates the one `ToastStack` (a full-window, pointer-transparent column at `GlobalZIndex` 150: above a `dialog`'s scrim, below a tooltip) and every `Toast` becomes its child in the frame it is spawned, through a component hook. It stands as long as its message takes to read, `(base_seconds + characters ÷ chars_per_second) × duration_scale`, then fades out and despawns; a fourth beyond `max_visible` (3) sends the oldest into its fade at once. It fades in as the room it holds opens and out as that room closes (the body is clipped to a height that `apply_skin` opens and closes), so the toasts round it glide to make way and close up and none of them jumps. It runs on `Time<Real>`. A toast takes no input and no focus: every node of it is `Pickable::IGNORE`, it is no tab stop and nothing in it writes `InputFocus`. A mark down its left edge is the accent for `toast` and `danger` for `toast_alert`, twice as wide, so the kind is told by shape as well as colour; the words carry it too. It is not announced to screen readers yet.
- `ToastSettings` (a resource, serde data with the `serde` feature): `anchor` (`ToastAnchor`, six places, bottom centre by default; the newest stands nearest the edge), `max_visible`, `base_seconds` (2.0), `chars_per_second` (20, Netflix's reading speed for adults), `duration_scale` (1.0: the player's option, which Xbox Accessibility Guideline 116 wants adjustable to at least ten times the default; any value above zero works) and `fade_seconds` (0.3; zero removes the fade and the opening of room). `update_toasts` (in `GooeySystems::Style`) runs the clocks and writes a toast only when its opacity or room changes.
- `model::{toast_hold, toast_life, toast_alpha, toast_grow}`, the engine-free maths under it; `Toast`, `ToastKind`, `ToastStack`, `Role::{ToastStack, Toast, ToastBody, ToastMark, ToastText}`.
- `GooeySkin::problems()` checks two more pairs, `accent` and `danger` against the solid surface a toast stands on, at 3:1. Both presets pass; a custom skin whose accent or danger colour sits near its surface now warns.
- The gallery example spawns toasts (Play, the Subtitles switch and the Volume slider; `GOOEY_KEYS` tokens `toast`, `alert` and `long`; `GOOEY_TOAST_ANCHOR`).

### Changed

- `Role` has seventeen more variants (twelve for the settings controls, five for toasts): a `match` over it that is exhaustive needs arms for them.
- `gamepad_select` takes one more query parameter (the toggles it flips) and `FocusScopes` one more, so a host that registers these systems by hand gets them from `GooeyNavPlugin` / `GooeyStylePlugin` as before; calling them directly is unchanged. `navigate_by_input` leaves Left and Right to a focused, enabled slider.

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
