# Accessibility actions: a screen reader can press, step and set what it reads

Researched 2026-11-02 (iteration 53). Backlog #4 of iteration 52 ("AccessKit actions: `ActionRequest` Click, Focus, Increment, Decrement, SetValue"), recorded as "Not done" in `src/access.rs` since iteration 42. Without it the tree of iteration 42 is read-only: a screen reader can announce a slider and cannot move it, so the "Click" a platform's narrator offers on a button does nothing, and a switch-access or voice-control user (both drive an app through the same platform actions) is shut out of every widget the keyboard reaches.

## Sources

1. **Craft:** W3C WAI, *Understanding SC 4.1.2 Name, Role, Value* (Level A), https://www.w3.org/WAI/WCAG22/Understanding/name-role-value.html (read 2026-11-02), and Microsoft, *Xbox Accessibility Guideline 112: UI navigation*, https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112.
2. **Shipping game:** Naughty Dog, *The Last of Us Part II: Accessibility Features Detailed*, https://www.naughtydog.com/blog/the_last_of_us_part_ii_accessibility_features_detailed (Text-to-Speech over every menu; quoted in `accessibility.md`), and Xbox platform narration as XAG 112 describes it.
3. **Technical:** `accesskit` 0.24 (`Action`, `ActionRequest`, `ActionData`, `Node::add_action`), and Bevy 0.20's `bevy_winit::accessibility` (forwards every platform request as the message `bevy::a11y::ActionRequest`, whose `NodeId` is `Entity::to_bits`) and `bevy_ui_widgets` (`Activate`, `ToggleChecked`, `SetSliderValue`, `ValueChange<Option<Entity>>` on a `TabList`), read from the vendored source.

## Craft principle

WCAG 2.2 SC 4.1.2: "For all user interface components ... the name and role can be programmatically determined; states, properties, and values that can be set by the user can be programmatically set; and notification of changes to these items is available to user agents, including assistive technologies." Its intent: "to ensure that Assistive Technologies (AT) can gather appropriate information about, activate (or set) and keep up to date on the status of user interface controls". Iteration 42 did the first and last of those; this is "activate (or set)". The same note names the failure: "F15: Failure of Success Criterion 4.1.2 due to implementing custom controls that do not use an accessibility API for the technology, or do so incompletely".

## How shipping games do it

Naughty Dog: "our menus and other on-screen text can be voiced using the Text-To-Speech option." The voice reads where focus is, and the player moves with the game's own input. The platform route (Xbox narration, Windows Narrator, VoiceOver) goes one step further: the screen reader *invokes* the focused item, so the platform asks the application to perform the action. XAG 112: "The movement of focus should always be predictable", which here means an assistive technology's request lands exactly where the same player's key or pad press would: one path per action, and the same `InputFocus` the pad moves.

## Model adopted

- **B1. An action is the widget's own event, never a second implementation.** `act_on_requests` (Navigate) turns each `bevy::a11y::ActionRequest` into the event the keyboard, pad or pointer already sends, so a game's `observe` sees the same `ValueChange`, `Activate` or `SelectionChange` whichever input moved it and nothing acts twice (AGENTS: one path per key):
  - **Click** on a Bevy `Button` (a button, a choice, a binding row, a menu row, a slot, a toast button, a swatch): `Activate`. On a `Checkbox`: `ToggleChecked`. On a `Tab`: `ValueChange<Option<Entity>>` on its strip, which is what Bevy's own click observer sends. A table header cell is a `Button`.
  - **Focus**: `InputFocus::set` and `InputFocusVisible` (a screen-reader cursor lands where the ring shows, exactly like Tab).
  - **Increment, Decrement**: a slider `SetSliderValue::RelativeStep(±1)`, a choice `MoveChoice::Step(±1)`, a number field `NumberStepBy ±1`, a colour area `AreaStepBy ±1`.
  - **SetValue** with `NumericValue`: a slider `SetSliderValue::Absolute`, a number field a final `ValueChange<String>` on its row with the number's text (so `number_on_text` parses, holds to the range, reports and refuses as typing does). With `Value(text)`: a text field a final `ValueChange<String>` on its row. **ReplaceSelectedText** is not offered: a one-line field's text is replaced whole by SetValue, and a caret is a keyboard's.
  - **Expand, Collapse**: a section header and a menu title or submenu row, as `Activate` when the state differs (an already-open section asked to expand does nothing).
- **B2. Advertise what is handled, nothing else.** A node declares an action only when the crate will act on it (`Node::add_action`), because a platform offers every declared action to the user. A disabled widget (`InteractionDisabled`) declares none, and a request for one that is disabled is ignored. The set is written in the same `says`/`write` pass as the name, only on a difference, so a settled interface marks nothing changed.
- **B3. A request for a stale node is dropped.** The target is `Entity::try_from_bits(node_id)`; an entity that is gone, has no widget behind it, or is not what the action needs is ignored without a warning (a screen reader holds a tree one frame old).
- **B4. Nothing runs without a screen reader.** `act_on_requests` reads a message queue that is empty unless an assistive technology is connected; the advertising is inside `update_accessibility`, which already returns at once without `AccessibilityRequested`.

## Parameters

None numeric. Which widget advertises which actions: Button, Checkbox, Tab: Focus, Click; choice: Focus, Click, Increment, Decrement; slider, number field, colour area: Focus, Increment, Decrement (and SetValue for a slider and a number field); text field: Focus, SetValue; section header, menu title: Focus, Click, Expand, Collapse; any other focus stop: Focus. Design defaults chosen to mirror the ARIA roles each widget implements in the APG (button, checkbox, tab, spinbutton, slider, textbox, disclosure).

## Validation targets

- A toggle asked to Click flips once, reports one `ValueChange<bool>`, and its node's `toggled` follows (the SC 4.1.2 "set" for a state).
- A button Click sends one `Activate`; a disabled one sends none and declares no Click.
- A slider SetValue 0.25 reads back 0.25 in `SliderValue`, one `ValueChange<f32>`; Increment and Decrement move one step; a value outside the range is held to it.
- A choice Increment steps one option and stops at the end as Right does, with one `ValueChange<usize>` and none at the end; a text field SetValue sends one final `ValueChange<String>` and shows the text; a number field SetValue 150 on a 0 to 100 field reports 100.
- A tab Click selects its page, once, and none when it is already selected.
- Focus moves `InputFocus` and shows the ring; a request for a despawned entity, the window, or a node of no widget does nothing and does not panic.
- A settled tree with actions marks no node changed; every advertised action is one the system handles (a table check over the gallery: no node declares an action its widget ignores).

## Deviations

None from a source. Design defaults: ReplaceSelectedText, SetTextSelection, Scroll* and ShowContextMenu are not offered (no widget owns a caret or a scroll a platform could drive without the keyboard path). `Collapse`/`Expand` for a tree row was not offered here; `access-rows.md` (iteration 57) adds it, as the table's own toggle, and a radial's Increment and Decrement.

## What great looks like

Not visual. A narrator user hears "Subtitles, check box, not checked" then the platform's invoke key flips it and the next announcement is "checked", a volume slider answers Increment with "71%", a settings screen can be walked and changed with a screen reader and no sight, and nothing a game wrote is different from a game with no screen reader.

## Pending index

home-still has no stem on screen-reader invocation or on AccessKit. The WCAG 4.1.2 Understanding page, XAG 112 and the AccessKit source were read from the web and the vendored crate, not from the index.
