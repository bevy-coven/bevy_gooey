# What a host touches

This page is the list a game needs: which resource, component and system of the crate it writes or
reads, and which it only sees because `GooeyPlugins` added it. Every item here is exported from the
crate root.

## Settings, as data

| type | what it holds | where it lives |
|---|---|---|
| `GooeySkin` | every colour and size, `density` (`Density::Compact` for a tool: half the padding, a 24 px least target) and the player's `text_scale` (the `text_size` row edits it); `high_contrast()` and `tool()` are presets, `skin_preset` the row that picks one (`SkinPreset`, `with_colours_of`) | a resource, read by `apply_skin` |
| `GooeyFonts` | the handles of the host's body and heading fonts | a resource |
| `UiScaleSettings` | `reference_short_side` and the player's `scale` (the `interface_size` row edits it) | a resource, read by `scale_with_window` |
| `ToastSettings` | `anchor`, `max_visible`, `base_seconds`, `chars_per_second`, `duration_scale`, `fade_seconds` | a resource, read by `update_toasts` |
| `BindingSettings` | `key_prompt`, `pad_prompt`, `chord_prompt`, `pad_style` (the `pad_style` row edits it), `cancel_key`, `cancel_button` | a resource, read by `update_bindings` and `bind_by_input` |
| `InputDevice` | `Keyboard` or `Pad`: the device the player used last, what a `prompt` shows | a resource, kept by `track_input_device`; write it to start on the pad |
| `Meter` | `value` and `max` | a component on a `meter()` |

`GooeySkin`, `UiScaleSettings`, `ToastSettings` and `BindingSettings` are serde data under the `serde`
feature; `GooeyFonts` holds asset handles and `InputDevice` is runtime state.

## Components a game spawns or marks

- `Meter { value, max }` — put it on the entity a `meter()` bundle returned to drive that bar.
- `SkinScope(GooeySkin)` — on any node: that node and everything below it is styled from this skin and
  not from the `GooeySkin` resource (the nearest scope wins). Insert, change or remove it and `apply_skin`
  restyles exactly that subtree in the same frame. See [skins](skins.md).
- `ListView { rows }` — the scrolling viewport of a `list()`, the list's first child. Set its
  `ScrollPosition` to scroll from code, or its `rows` to change how many rows show.
- `ChoiceValue(usize)` and `ChoiceCount(usize)` — a `choice`'s selected index and its number of options.
  Write a `ChoiceValue` to move the choice from code (no `ValueChange` comes back for that); read it to
  know where the choice stands.
- `OptionRow` — on a `text_size`, `interface_size`, `pad_style` or `skin_preset` row: which of the crate's own settings it
  edits. A host that registers `update_option_rows` itself needs nothing else; a game never builds one.
  See [player options](options.md).
- `BindingValue(Binding)` — a `binding`'s input: a key, a mouse button, a pad button or a chord. Write one to change
  the row from code (no `ValueChange` comes back for that); read it to know what the player chose.
  `Capturing` is on the row while it waits for the player's next press, for a game that gates raw key
  readers of its own on it. See [rebinding](bindings.md).
- `TextInputValue(String)` — a `text_input`'s text. Write it to change the field from code (no
  `ValueChange` comes back for that); read it to know what the player typed. `TextHint(String)` (muted
  words while the field is empty) and `TextLimit(usize)` (most characters the player may type) go on the
  row. `TextEditing` is on the row exactly while its field has focus, for a game that gates its own key
  readers on it (`is_text_editing`, a run condition) or opens the platform's keyboard when it is added;
  `TextInputOf(row)` is on the field and the hint. See [text input](text-input.md).
- `Prompt { keyboard, pad }` — a `prompt`'s two inputs, `Prompt::SELECT` and `Prompt::BACK` for the ones
  the crate acts on. Set it on the prompt's entity to keep the hint true after a rebind (the keycap
  follows in the same frame). See [prompts](prompts.md).
- `TabsValue(usize)` and `TabsCount(usize)` — a `tabs`' selected index and its number of labels. Write a
  `TabsValue` to turn the page from code (no `ValueChange` comes back for that); read it to know the page.
- `Tooltip { text, shortcut }` — the hint on any widget, built `Tooltip::new("Save")` and, for a command with a
  keyboard shortcut, `.with_shortcut(Shortcut::primary(KeyCode::KeyS))` (a keycap beside the line). Insert, remove or
  edit it and the open box follows. See [shortcuts](shortcuts.md).
- `Shortcut { modifiers, key }`, `Modifiers` and `Platform` — a key chord, its platform's words
  (`label`, `label_on`) and the one rule for "went down this frame" (`just_pressed`).
- `Toast { .. }` — the component `toast()` and `toast_alert()` return; despawn the entity to take one
  back sooner.
- `Dialog` — the component `dialog()` returns, for a game that wants to find the open one.
- `PopoverPanel { anchor, placement, modal }` — the component `popover()` and `popover_modal()` return: query it
  for the popover open on an anchor. `PopoverClosed { entity, anchor, reason }` is the event the player's
  closing sends first (`CloseReason::Outside` or `Cancel`); `Placement` says where one opens. See
  [popovers](popovers.md).

- `MenuBar`, `MenuTitle`, `MenuPanel`, `MenuItem`, `MenuOpen` — what `menu_bar()`, `menu_title()`, the item
  constructors and `submenu()` return or add; a menu's panel is open exactly while it has a `PopoverPanel`,
  and `MenuOpen` is on its title or submenu item meanwhile. `Disabled(reason)` makes an item unavailable with a
  tooltip. See [menus](menus.md).

- `ContextMenu(menu)`, `ContextTarget(thing)` — on any entity, the menu `context_menu(..)` made that opens for it; the
  menu's panel carries `ContextTarget` exactly while it is open, and `ContextTarget::of(item, ..)` finds it from an
  item. See [context menus](context-menus.md).

- `CommandList(Vec<PaletteCommand>)`, `PaletteHistory`, `RunCommand { id }`, `CommandPalette` — the commands a
  `command_palette()` lists, the ids that ran last, the event a choice sends and what an open palette
  holds (`results()`, `selected()`). See [the command palette](command-palette.md).

- `SplitFraction(f32)`, `SplitMin { first, second }`, `Split`, `SplitDivider` — on the entity `split(axis, first,
  second)` returned: the first pane's share (the state, serde data), each pane's least size in logical px, the
  axis, and the divider part. A player's change is one `ValueChange<f32>` on the split. See [split panes](splits.md).

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
| `close_popovers_on_press` | Navigate | a pointer press outside a popover closes it with one `PopoverClosed` |
| `anchor_pointer_popovers`, `drop_pointer_anchors` | Navigate | a `Placement::AtPointer` popover gets a zero-size node at the pointer to hang from, removed with it |
| `focus_menu_bar` | Navigate | F10, or Alt pressed and let go alone, focuses the first menu title; again, gives focus back |
| `menu_by_input` | Navigate | Left, Right, Down, Home and End (and the D-pad) on a menu title or item open, walk and close menus |
| `context_by_key` | Navigate | Shift+F10 or the Menu key opens the context menu of the focused widget's nearest `ContextMenu`, below it |
| `palette_by_input` | Navigate | with focus in a palette's field: Up, Down, PageUp, PageDown and the D-pad move its lit row, Enter and South run it |
| `switch_menus_on_hover`, `fit_menus`, `scroll_menus_to_focus` | Navigate | the pointer on another title switches menus; an open menu is capped to the window's room and follows focus |
| `feedback_on_focus` | Navigate | writes `UiFeedback::Focus` when focus moves onto a widget |
| `scroll_to_focus` | Navigate | a list follows the focus on one of its rows |
| `page_lists` | Navigate | PageUp, PageDown and the pad's triggers move focus a viewport |
| `step_tabs` | Navigate | LB, RB, Ctrl+PageUp and Ctrl+PageDown turn a tabs widget's page, and the D-pad steps a focused tab |
| `split_by_pad` | Navigate | the D-pad steps a focused split divider along its axis (repeating), South collapses or restores its first pane |
| `track_input_device` | Navigate | a key or a mouse button is `InputDevice::Keyboard`, a pad button or a pushed stick `Pad`; first in the set |
| `track_text_editing` | Navigate | `TextEditing` on the `text_input` row whose field has focus, a final change when it loses it, and focus on the row goes to its field; second in the set |
| `bind_by_input` | Navigate | a waiting `binding` row takes the player's next press, last in the set |
| `catch_pointer` | Navigate | a transparent node takes the pointer while a `binding` row waits |
| `land_tab_focus` | Navigate | focus never rests on a page that is not shown: it moves to the page's first widget |
| `show_tooltips` | Style | a hovered or visibly focused widget's tooltip opens |
| `update_bindings` | Style | a `binding` row's keycap: the input's name, or the prompt while it waits |
| `update_prompts` | Style | a `prompt`'s keycap: the input's name for the device in use, and its shape |
| `update_text_inputs` | Style | a `text_input`'s text the game set shown in its field, the hint while it is empty, the limit, a disabled row's field |
| `update_palette` | Style | an open palette's matches, lit row, eight rows' words, "No command matches" and the count, only when its field, its lit row, `CommandList` or `PaletteHistory` changed |
| `update_tooltip_text` | Style | a changed `Tooltip` reaches its box in place |
| `update_sliders` | Style | a slider's thumb, fill and readout from its `SliderValue` |
| `update_choices` | Style | a choice's selected option, and a value past the end pulled back |
| `update_option_rows` | Style | a `text_size`, `interface_size`, `pad_style` or `skin_preset` row follows its setting with no event, and on the frame it is spawned; before `update_choices` |
| `update_tabs` | Style | a tabs widget's selected page shown and no other, its tab lit, a value past the end pulled back |
| `update_splits` | Style | a split's two panes' `flex_grow` and least sizes from `SplitFraction` and `SplitMin`, before `apply_skin`; writes only when they differ |
| `update_meters` | Style | a meter's fill and frame from its `Meter` |
| `update_lists` | Style | a list's rail, only on a change |
| `update_toasts` | Style | the toast clocks, the stack's order and its cap |
| `apply_skin` | Style | styles every widget carrying a `Role`, and restyles it when the skin changes |
| `scale_with_window` | Style | keeps `UiScale` at the window's share (`GooeyScalePlugin` adds it) |

The tooltip's and the toast's component hooks are registered by the components themselves, so they come
with the spawn. The resources and the `UiFeedback` message are `GooeyNavPlugin`'s
(`init_resource::<InputFocus>()`, `init_resource::<InputFocusVisible>()`,
`add_message::<UiFeedback>()`) and `BindingSettings`'s and `InputDevice`'s; a host that registers the
systems by hand adds those itself, and puts `not_capturing` as a run condition on `navigate_by_input`,
`gamepad_select`,
`gamepad_adjust`, `cancel_by_input`, `page_lists` and `step_tabs`, so a waiting binding row hears the
press and they do not.

## The engine-free maths

`model` holds the crate's arithmetic with no `bevy` in it, so a host can run it in its own tests or
use it for its own UI: `relative_luminance`, `contrast_ratio`, `composite`, `meter_fraction`,
`slider_fraction`, `slider_text`, `list_height`, `scroll_to_reveal`, `snap_to_row`, `page_target`,
`snap_line_width`, `window_ui_scale`, `toast_hold`, `toast_life`, `toast_alpha`, `toast_grow`,
`repeat_steps`, `choice_step`, `choice_next`, `nearest_step`, `tab_step`, `key_name`, `mouse_name`,
`shortcut_label`, `shortcut_conflicts`, `is_modifier_key`, `fuzzy_score`, `split_mins`, `split_clamp`,
`split_drag`,
`pad_name` and `PadStyle` (with `PadStyle::name`), and the constants `TEXT_CONTRAST`,
`NON_TEXT_CONTRAST`, `MIN_TARGET_PX`, `COMFORT_TARGET_PX`, `PAD_REPEAT_DELAY`, `PAD_REPEAT_RATE`,
`STICK_PUSH`, `SPLIT_KEY_STEP_PX`, and the steps the option rows offer: `TEXT_SIZE_STEPS`, `INTERFACE_SIZE_STEPS` and
`PAD_STYLES`.
