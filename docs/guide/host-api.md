# What a host touches

This page is the list a game needs: which resource, component and system of the crate it writes or
reads, and which it only sees because `GooeyPlugins` added it. Every item here is exported from the
crate root.

## Settings, as data

| type | what it holds | where it lives |
|---|---|---|
| `GooeySkin` | every colour and size, and the player's `text_scale` | a resource, read by `apply_skin` |
| `GooeyFonts` | the handles of the host's body and heading fonts | a resource |
| `UiScaleSettings` | `reference_short_side` and the player's `scale` | a resource, read by `scale_with_window` |
| `ToastSettings` | `anchor`, `max_visible`, `base_seconds`, `chars_per_second`, `duration_scale`, `fade_seconds` | a resource, read by `update_toasts` |
| `BindingSettings` | `key_prompt`, `pad_prompt`, `pad_style`, `cancel_key`, `cancel_button` | a resource, read by `update_bindings` and `bind_by_input` |
| `Meter` | `value` and `max` | a component on a `meter()` |

All five are serde data under the `serde` feature except `GooeyFonts`, which holds asset handles.

## Components a game spawns or marks

- `Meter { value, max }` — put it on the entity a `meter()` bundle returned to drive that bar.
- `ListView { rows }` — the scrolling viewport of a `list()`, the list's first child. Set its
  `ScrollPosition` to scroll from code, or its `rows` to change how many rows show.
- `ChoiceValue(usize)` and `ChoiceCount(usize)` — a `choice`'s selected index and its number of options.
  Write a `ChoiceValue` to move the choice from code (no `ValueChange` comes back for that); read it to
  know where the choice stands.
- `BindingValue(Binding)` — a `binding`'s input: a key, a mouse button or a pad button. Write one to change
  the row from code (no `ValueChange` comes back for that); read it to know what the player chose.
  `Capturing` is on the row while it waits for the player's next press, for a game that gates raw key
  readers of its own on it. See [rebinding](bindings.md).
- `TabsValue(usize)` and `TabsCount(usize)` — a `tabs`' selected index and its number of labels. Write a
  `TabsValue` to turn the page from code (no `ValueChange` comes back for that); read it to know the page.
- `Tooltip(String)` — the hint on any widget. Insert, remove or edit it and the open box follows.
- `Toast { .. }` — the component `toast()` and `toast_alert()` return; despawn the entity to take one
  back sooner.
- `Dialog` — the component `dialog()` returns, for a game that wants to find the open one.

Bevy's own types come through the crate so a host never adds a second copy: `Checked`, `SliderValue`,
`SliderRange`, `SliderStep`, `ValueChange`, `Activate` and `observe`.

## Messages

`UiFeedback { entity, kind: UiFeedbackKind::Focus | Activate }` is written once for every focus move onto
a widget and every activation, flip or change of option. A game reads it with `MessageReader<UiFeedback>`
and plays a sound; the crate plays none. See [feedback](feedback.md).

`Cancel { entity }` is an event a game observes, sent when the player presses Escape or a pad's East
button (B): it starts at the focused widget, or at the open dialog, and bubbles up to the layer the game
observes. Nothing is closed for it. See [navigation](navigation.md#back).

## Systems, for a host that registers its own

`GooeyStylePlugin` and `GooeyNavPlugin` are independent, and `GooeyPlugins` is both plus
`GooeyScalePlugin`. A host that wants the systems and not the plugins registers them itself, in
`GooeySystems::{Navigate, Style}`, chained Navigate before Style:

| system | set | what it does |
|---|---|---|
| `navigate_by_input` | Navigate | arrows and the D-pad move focus to the nearest widget; takes `FocusScopes` |
| `gamepad_select` | Navigate | South activates, flips a toggle and cycles a choice |
| `cancel_by_input` | Navigate | Escape and a pad's East button send one `Cancel` to the focused widget or the open dialog |
| `gamepad_adjust` | Navigate | the D-pad steps a focused slider or choice, repeating on real time |
| `focus_opened_dialogs` | Navigate | an open dialog takes focus and gives it back on closing |
| `feedback_on_focus` | Navigate | writes `UiFeedback::Focus` when focus moves onto a widget |
| `scroll_to_focus` | Navigate | a list follows the focus on one of its rows |
| `page_lists` | Navigate | PageUp, PageDown and the pad's triggers move focus a viewport |
| `step_tabs` | Navigate | LB, RB, Ctrl+PageUp and Ctrl+PageDown turn a tabs widget's page, and the D-pad steps a focused tab |
| `bind_by_input` | Navigate | a waiting `binding` row takes the player's next press, last in the set |
| `catch_pointer` | Navigate | a transparent node takes the pointer while a `binding` row waits |
| `land_tab_focus` | Navigate | focus never rests on a page that is not shown: it moves to the page's first widget |
| `show_tooltips` | Style | a hovered or visibly focused widget's tooltip opens |
| `update_bindings` | Style | a `binding` row's keycap: the input's name, or the prompt while it waits |
| `update_tooltip_text` | Style | a changed `Tooltip` reaches its box in place |
| `update_sliders` | Style | a slider's thumb, fill and readout from its `SliderValue` |
| `update_choices` | Style | a choice's selected option, and a value past the end pulled back |
| `update_tabs` | Style | a tabs widget's selected page shown and no other, its tab lit, a value past the end pulled back |
| `update_meters` | Style | a meter's fill and frame from its `Meter` |
| `update_lists` | Style | a list's rail, only on a change |
| `update_toasts` | Style | the toast clocks, the stack's order and its cap |
| `apply_skin` | Style | styles every widget carrying a `Role`, and restyles it when the skin changes |
| `scale_with_window` | Style | keeps `UiScale` at the window's share (`GooeyScalePlugin` adds it) |

The tooltip's and the toast's component hooks are registered by the components themselves, so they come
with the spawn. The resources and the `UiFeedback` message are `GooeyNavPlugin`'s
(`init_resource::<InputFocus>()`, `init_resource::<InputFocusVisible>()`,
`add_message::<UiFeedback>()`) and `BindingSettings`'s; a host that registers the systems by hand adds
those four itself, and puts `not_capturing` as a run condition on `navigate_by_input`, `gamepad_select`,
`gamepad_adjust`, `cancel_by_input`, `page_lists` and `step_tabs`, so a waiting binding row hears the
press and they do not.

## The engine-free maths

`model` holds the crate's arithmetic with no `bevy` in it, so a host can run it in its own tests or
use it for its own UI: `relative_luminance`, `contrast_ratio`, `composite`, `meter_fraction`,
`slider_fraction`, `slider_text`, `list_height`, `scroll_to_reveal`, `snap_to_row`, `page_target`,
`snap_line_width`, `window_ui_scale`, `toast_hold`, `toast_life`, `toast_alpha`, `toast_grow`,
`repeat_steps`, `choice_step`, `choice_next`, `tab_step`, `key_name`, `mouse_name`, `pad_name` and
`PadStyle`, and the constants `TEXT_CONTRAST`, `NON_TEXT_CONTRAST`, `MIN_TARGET_PX`,
`COMFORT_TARGET_PX`, `PAD_REPEAT_DELAY` and `PAD_REPEAT_RATE`.
