# Cancel: Escape and the pad's B back out of a dialog or a screen

Researched 2026-10-31 (iteration 15). home-still's corpus has nothing on back navigation or cancel buttons (`hs`/MCP `distill_search`, "back button cancel gamepad menu navigation undo exit user control and freedom": a controller usability chapter, a Rust game book, a game-mechanics book, an FDG poster and an agent-platform paper; `catalog_recent` limit 200 held only biomedical, SIGGRAPH and rendering-course embeds), so the three kinds of source below are web copies, Bevy's source and the owner's editor. Every quotation was fetched on 2026-10-31 and matches; the gap is under `## Pending index`.

## Sources

1. Nielsen Norman Group, Nielsen, J., *10 Usability Heuristics for User Interface Design* (1994, reviewed 2024), heuristic 3, *User Control and Freedom* — https://www.nngroup.com/articles/ten-usability-heuristics/ (craft: why every layer needs an exit and what it must feel like).
2. Microsoft, *Xbox Accessibility Guideline 112: UI navigation* and *106: Narration* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112, `/106`; Microsoft, *Gamepad and remote control interactions* — https://learn.microsoft.com/en-us/windows/uwp/ui-input/gamepad-and-remote-interactions (shipping games and the console's convention: Sea of Thieves, Grounded; B is Escape).
3. W3C, *Dialog (Modal) Pattern* (WAI-ARIA Authoring Practices Guide) — https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/; Godot 4's `ProjectSettings` `input/ui_cancel` — https://docs.godotengine.org/en/stable/classes/class_projectsettings.html; and the Bevy 0.20.0 source: `bevy_ui_widgets-0.20.0/src/{dialog.rs,modal.rs,menu.rs,text_input.rs}` and `bevy_input_focus-0.20.0/src/lib.rs` (technical: how a key bubbles from the focused widget, what Bevy already does with Escape, and what an engine's cancel action is).
4. The owner's `bevy_emerge` editor, `crates/bevy_emerge_mapper/src/confirm.rs` and `chooser.rs`, which read `KeyCode::Escape` by hand for a confirm dialog and a chooser (what a game rebuilds today).

## Craft principle

Nielsen, heuristic 3: "Users often perform actions by mistake. They need a clearly marked "emergency exit" to leave the unwanted action without having to go through an extended process." And: "When it's easy for people to back out of a process or undo an action, it fosters a sense of freedom and confidence. Exits allow users to remain in control of the system and avoid getting stuck and feeling frustrated." His tips: "Show a clear way to exit the current interaction, like a *Cancel* button." and "Make sure the exit is clearly labeled and discoverable." A confirm dialog the player opened by a slip ("Leave the game?") is the case: the exit is the least effort the input device has, one press that needs no aim.

XAG 112 states the rule for a game menu: "Players should always have a mechanism of easily returning to the main menu or previous screen." And on consistency: "Interaction mechanisms are consistent throughout (like “A” to select, “B” for back, ...). If interaction methods were to change from screen to screen, this would be very disorienting and confusing for a player."

## How shipping games do it

XAG 112, Sea of Thieves: "regardless of where the player is in the UI navigation sequence, they're always provided the ability (and the important visual interaction prompt on the bottom-left corner of the screen) to press “B” to go back to the previous menu screen." The same page, on its dialog boxes: "When dialog boxes appear, interaction prompts also appear on the bottom of the window." XAG 106 on how fixed the mapping is: "On Xbox, the B button is almost always equivalent to Escape", and its narration examples read the prompts aloud: "“A button Accept,” “B button Back,” “Y button Reset to Default”".

Microsoft's gamepad guide builds the same equivalence into the platform: its built-in mapping table lists "Escape | B/Back button", and says what happens when nothing in the app takes the press: "When neither the KeyDown nor KeyUp events for the B button are handled by the app, the SystemNavigationManager.BackRequested event will be fired, which should result in back navigation within the app." Its handler checks `if (!e.Handled)` first: the innermost handler wins and the one above it does not also act. And: "If the B button is used to go back, then don't show a back button in the UI."

The owner's editor reads the key by hand: `confirm.rs:250`, `else if keyboard.just_pressed(KeyCode::KeyN) || keyboard.just_pressed(KeyCode::Escape)` for a confirm dialog's No, `chooser.rs:6399`, `if keyboard.just_pressed(KeyCode::Escape)` for a chooser, `snip.rs:131` for a selection; none of them reads a pad, and none asks which layer has the focus.

## Technical

APG, the modal dialog's keyboard interaction: "`Escape`: Closes the dialog." and "Windows under a modal dialog are inert. That is, users cannot interact with content outside an active dialog window." A Back press in a dialog therefore reaches the dialog and never the screen under it.

Godot's built-in action is the engine-level answer: "input/ui_cancel: Default InputEventAction to discard a modal or pending input. Note: Default ui_* actions cannot be removed as they are necessary for the internal logic of several Controls. The events assigned to the action can however be modified." One named action, Escape and the pad's B on it by default, read by every control that has something to discard.

Bevy 0.20.0 has half of it and names the rest as ours. `bevy_ui_widgets::RequestClose` is "Event used to indicate that the dialog wants to be closed. This can happen because the user clicked on the barrier, hit the escape key, or clicked the close box in the dialog title. This event propagates so that the owner of the dialog can despawn it.": `#[entity_event(propagate, auto_propagate)]`, fired by Bevy's own `ModalDialog` (a different component from this crate's `Dialog`) and only from a click on its barrier or a keypress that bubbles to it. The popup menu closes on Escape and stops the key (`ev.propagate(false)`). `text_input` leaves it alone on purpose: "Escape belongs to the surrounding context (close a dialog, cancel, ...)". Keys bubble from the focused entity through `FocusedInput<KeyboardInput>` with `WindowTraversal`; a pad's buttons do not (`Gamepad` is polled, as `gamepad_select` does), so a Back that is also the pad's cannot be an observer of `FocusedInput` alone.

## Model adopted

- **K1. One event for "the player backs out": `Cancel`.** Escape, or any gamepad's `East` (B on Xbox, Circle on PlayStation: a position, as `South` is for Enter), is one `Cancel` per press (`just_pressed`: a held key does not repeat, XAG 107's single presses). A game handles one event and gets both devices; a pad's B is never a second code path.
- **K2. It starts at the focused widget and bubbles up its ancestors** (`#[entity_event(propagate, auto_propagate)]`, the way Bevy's own `RequestClose` and `FocusedInput` travel), so the game observes the dialog or the screen it is leaving: `observe(|c: On<Cancel>, mut commands: Commands| commands.entity(c.entity).despawn())`. `Cancel::entity` is the entity the event is at, as the observer sees it (the screen, the dialog); `On::original_event_target()` is the widget that was focused. An observer that handled it calls `cancel.propagate(false)` so a screen above does not also act (the UWP `if (!e.Handled)`).
- **K3. A modal dialog is the end of the road.** The bubble never leaves a `dialog()`: APG's "Windows under a modal dialog are inert", and the same scope rule as the arrows and Tab (`FocusScopes`). A dialog a game spawned under its screen (so the screen is its ancestor) still does not reach the screen's observer. This removes the one way the innermost-handler-wins rule is easy to get wrong.
- **K4. The dialog on top is the target when nothing in it is focused.** No widget focused (a click on empty space cleared it) or focus left outside the open dialog (the host moved it): the event starts at the dialog, so a dialog can always be dismissed (XAG 112: if focus can enter, it can leave by the same input).
- **K5. With no dialog and no focused widget, nothing happens.** Escape is the game's: a game with no menu open reads it for its own pause. Focus on the window (Bevy focuses it at start-up) is not a widget. A game whose pause menu is a `screen()` handles `Cancel` on it to resume, and its own key system only opens the menu when none is open.
- **K6. It shows focus.** A press that has a target sets `InputFocusVisible`, as South and the arrows do ("the ring appears the moment a key is pressed"); a press with no target leaves it alone.
- **K7. This crate closes nothing.** `Cancel` says the player asked to leave; whether a "you must choose" dialog may be left is the game's (`docs/research/dialog.md`). One observer per layer is the whole cost of a Back that works on a keyboard and a pad.
- **K8. Nothing runs at rest.** `cancel_by_input` returns on a frame with neither press. It is in `GooeySystems::Navigate` with the other input systems.

## Parameters

|part|value|
|---|---|
|keys|`KeyCode::Escape`; `GamepadButton::East` on any connected gamepad|
|event|`Cancel { entity }`, `EntityEvent`, propagating up `ChildOf`, never past a `Dialog` (its observers still run)|
|target|the focused widget when it is a live, visible tab stop in the active scope; else the open dialog on top (`FocusScopes::active`); else none|
|set|`GooeySystems::Navigate`, `cancel_by_input`, after `gamepad_select`|

## Validation targets

- Escape with focus on a button inside a screen: one `Cancel` reaches an observer on the screen, its `entity` the screen and `original_event_target()` the button; a pad's East does the same; neither one fires twice for one press, and a key held for ten frames fires once.
- A dialog spawned under a screen, focus on its button: the dialog's observer hears it and the screen's does not; an observer on the dialog's panel that calls `propagate(false)` stops it before the dialog.
- A dialog open and nothing focused: the dialog's observer hears it; a dialog open and focus on a widget outside it: the same.
- No dialog and focus on nothing (or the window): no event, and `InputFocusVisible` is still hidden. No dialog, focus on a widget: the event, and focus is visible.
- A dialog closed by its `Cancel` observer gives focus back to the button that opened it, ring visible (the close path the gallery's Stay already takes).
- A focused slider, choice, tab or toggle is unchanged by Escape and East: no `ValueChange`, no `Activate`.
- Thirty idle frames write nothing (`InputFocusVisible` unchanged, no event).
- Frame time of the gallery within 0.5 ms of before.

## What great looks like

The gallery's Quit dialog: Down, Down, Enter opens "Leave the game?" with the ring on Stay; the player presses B (or Escape) and the dialog is gone in that frame, the ring on Quit where it was, and the screen behind never heard a thing. The game wrote one observer, on the dialog, and the same observer serves the keyboard and the pad. A pause screen of the player's own is the same line, resume, and Escape closes it as it opened it.

## Deviations

- **A new event, not Bevy's `RequestClose`.** `RequestClose` is the headless `ModalDialog`'s, which this crate's `dialog()` is not (it is a `Role::Scrim` with its own scope and focus rules), it is named for dialogs, and Bevy sends it only from a click on its barrier or a keypress that bubbles to it. Back is also a screen's and a pad's. A game that uses Bevy's `ModalDialog` keeps `RequestClose`; gooey's `Cancel` covers its own `dialog()` and everything that is not one. A design default.
- **`East` is positional.** On a pad whose convention confirms with East (some Japanese titles) a game remaps; the crate matches `South` for activate, a position. A design default.
- **The pad's Start and Menu buttons are not Cancel**: they are the game's pause (Microsoft's guide gives the Menu button to context menus).
- **A click on a dialog's scrim is not Cancel.** Bevy's `ModalDialogBarrier` closes on it; in a game a stray click closing a confirm is a slip (XAG 107's up-event rule), and the scrim blocks the pointer so a click behind it is already safe. A design default; a game that wants it observes `PointerClick` on the scrim.
- **No default close and no prompt glyph.** `dialog()` is not despawned by Cancel (K7), and the crate draws no "B Back" prompt (it has no glyph vocabulary; XAG 112 wants the prompt on screen, so the guide tells the game to draw it and not also a Back button, Microsoft's note). A Back button is still right where a mouse player has no key.
- **No `Cancel` from a text field**: the crate has none; when it has, Escape in a field cancels the edit first (Bevy's `text_input` leaves Escape to the context).
- Assumed decisions (owner may override), all above: the event's name, shape and bubbling; the bound at a dialog; the dialog as the target of a Back with nothing focused; East for the pad; showing focus; nothing closed by the crate.

## Pending index

- Nothing relevant in home-still (one query on 2026-10-31 and `catalog_recent` limit 200). Index NN/g's *10 Usability Heuristics* and *User Control and Freedom*, Xbox Accessibility Guidelines 106 and 112, Microsoft's *Gamepad and remote control interactions*, and W3C's *Dialog (Modal) Pattern*. Search later for: "back button gamepad menu hierarchy", "modal dialog dismissal escape key", "cancel action game UI". No studio write-up of a game's own Back handling was found; the shipping-game evidence is Microsoft's guidelines naming Sea of Thieves and Grounded. The Game Accessibility Guidelines and *The Gamer's Brain* remain pending from `docs/research/gooey.md`.
