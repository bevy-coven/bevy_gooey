# Text input: a row with a field the player types in

Researched 2026-11-01 (iteration 19). home-still's index has nothing on game text entry (`hs distill search`: "text input field design usability labels placeholder", "gamepad text entry techniques comparison typing speed", "keyboard trap focus text field accessibility" returned a vision-language benchmark, a Rust book and VR typing papers; the first query, on-screen keyboards, answered no JSON). `paper_search` found the controller text-entry papers below, whose abstracts are quoted; none is indexed yet (`## Pending index`). The pick came from the owner's own editor: bevy_grimoire's `chrome.rs` hand-rolls `text_field` and a `NameBox`, and that module's own note says the `{raw}_` caret idiom "was written five times before it had one home".

## Sources

1. Wobbrock, J. O., Myers, B. A. & Aung, H. H., *Writing With A Joystick: A Comparison of Date Stamp, Selection Keyboard, and EdgeWrite* (CMU, 2004; doi:10.1184/r1/6470696), found by `paper_search` — craft: what a pad player can and cannot do with a text field. With Wilson & Agrawala, *Text entry using a dual joystick game controller* (CHI 2006, doi:10.1145/1124772.1124844) and Költringer et al., *Game controller text entry with alphabetic and multi-tap selection keyboards* (CHI 2007, doi:10.1145/1240866.1241033).
2. Nielsen Norman Group, *Placeholders in Form Fields Are Harmful* — https://www.nngroup.com/articles/form-design-placeholders/ — craft: where the label goes and what a hint is for.
3. Valve, *Getting your game ready for Steam Deck* (Steamworks Documentation) — https://partner.steamgames.com/doc/steamdeck/recommendations, and the *Steamworks SDK 1.52* announcement (`ShowFloatingGamepadTextInput`) — shipping: what the platform makes a game with a name field do, and how the keys reach the field.
4. W3C, *Understanding Success Criterion 2.1.2: No Keyboard Trap* — https://www.w3.org/WAI/WCAG22/Understanding/no-keyboard-trap.html — technical; with WCAG 1.4.3 and 1.4.11 (4.5:1 text, 3:1 non-text), already this crate's contrast rules (`gooey.md`).
5. Bevy 0.20.0's source — `bevy_text-0.20.0/src/editing.rs` (`EditableText`, `TextEditChange`, `TextCursorStyle`), `bevy_ui_widgets-0.20.0/src/text_input.rs` (`TextInput`, `on_focused_keyboard_input`, `SelectAllOnFocus`, IME) and `bevy_input_focus-0.20.0` (`AcquireFocus`, `FocusGained`, `AutoFocus`) — technical: which keys the field consumes and which it lets bubble.
6. The owner's `bevy_grimoire_mapper/src/chrome.rs` (`text_field`, `field_text`, `NameBox`, `paint_name_box`) and Sanctuary's `src/ui/mod.rs` (`UiHolds`, `mask_input`) — what the owner's projects hand-roll, and what it costs.

## Craft principle

Wobbrock et al., on the player with a pad: "A joystick text entry method for game controllers and mobile phones would be valuable, since these devices often have joysticks but no conventional keyboards. But prevalent joystick text entry methods are slow because they are selection-based." A pad is not a text device: a field must never need one, and the player who has a pad types through the platform's keyboard or not at all.

NN/g on where the words go: "Fields need labels and (sometimes) instructions, but placing this text inside the field lowers usability and accessibility and should be avoided." and "Labels outside the form fields make the essential information visible at all times, while placeholder text inside form fields is reserved for supplementary information." The label is outside the field and always shown; a hint is optional and supplementary, and as text it still meets 4.5:1.

## How shipping games do it

Valve, on the platform every pad-and-keyboard game now ships to: "We strongly recommend (and require for Deck/Machine Verified) that games automatically display an on-screen keyboard when requiring the user to input text. We recommend using one of the two Steamworks SDK on-screen keyboard APIs, depending on your use case: ShowFloatingGamepadTextInput (sends direct key inputs) and ShowGamepadTextInput (callback-based)." SDK 1.52: `ShowFloatingGamepadTextInput` "activates the modal gamepad input keyboard which pops up over game content and sends OS keyboard keys directly to the game."

So the platform's keyboard arrives as ordinary key events (Bevy 0.20's own test `steam_osk_unidentified_keys_insert_produced_text` feeds `Key::Unidentified` presses with `text` into the field), and the part the game must do is to know **when a field has the player** to ask for it. That is the one thing the crate can give every host: a marker on the row while it is being edited. The call into Steam's API is the host's (the crate has no platform dependency).

The owner's editor, `chrome.rs`: "The `{raw}_` + `ACCENT` idiom was written five times before it had one home; the caret is what makes 'empty because you cleared it' distinguishable from 'empty'"; and of its name prompt: with two conditions for one question, "every keystroke vanished until `Esc`, with nothing on screen to say where they were going". Sanctuary hand-writes `UiHolds` and a key mask so the game does not hear a key the UI took. Both are the same two needs: a real field (caret, selection, paste, IME) and a signal a game's hotkeys can wait on.

## Technical

WCAG 2.1.2 (Level A): "If keyboard focus can be moved to a component of the page using a keyboard interface, then focus can be moved away from that component using only a keyboard interface, and, if it requires more than unmodified arrow or tab keys or other standard exit methods, the user is advised of the method for moving focus away." A text field is the classic trap: it eats the arrows.

Bevy 0.20.0 gives the headless field, and says what is left to build: "Reusable widgets that build on top of this basic text input field … will typically combine this widget with additional UI elements such as borders, backgrounds, and labels"; "this widget is 'headless': it has no built-in styling … and any reactive styling (e.g., focus/hover states) must also be implemented separately"; and "Placeholder text (displayed when the input is empty)" is listed as not implemented. Its handler (`on_focused_keyboard_input`) stops the press for characters, Backspace, Delete, Home, End, Left, Right, **Up and Down**, and the copy, cut and paste shortcuts; it lets Tab, Enter and every other key bubble ("Ignore and propagate to allow for tab navigation and submit actions"), and for Escape: "Escape belongs to the surrounding context (close a dialog, cancel, back out) -- collapse and blur, but do NOT consume: the same press keeps bubbling." The crate's navigation does not read those events: it reads `ButtonInput<KeyCode>` in `Update`, so Bevy stopping an arrow's event does not stop `navigate_by_input` from acting on the same arrow (`nav.rs`). The field notifies with `TextEditChange` (an `EntityEvent` "emitted even if `EditableText::value` is unchanged", since it includes cursor moves), gives no `ValueChange`, and its `ValueChange` doc already says what `is_final` means for typing: "false … user typing … true … input lost focus".

A click on a label should focus its field (the HTML `label` behaviour that makes the whole row a target): `AcquireFocus` bubbles up `ChildOf` from the press target, `acquire_focus_tab_index` focuses the first tab stop it meets, and an observer on the row can take it first.

## Model adopted

- **T1. `text_input(label, value)` is a row like a toggle's**: the label on the left, a framed field on the right. The field is Bevy's `TextInput` + `EditableText`, one line, a fixed width in characters so the row never changes size while the player types. The label is outside the field and always visible (NN/g).
- **T2. State on the row.** `TextInputValue(String)` is the widget's state (like `ChoiceValue`, `BindingValue`): read it, or set it from code and the field shows it with the caret at the end and no event coming back. Each change the player makes is one `ValueChange<String>` on the row: `is_final: false` for each edit that changes the text, `true` when the edit ends, which is Enter (always: a submit) or the field losing focus after an edit (Bevy's own `is_final` meaning). Moving the caret or selecting sends nothing.
- **T3. Editing is live, never a mode.** A focused field takes the keys; there is no "press Enter to start typing" step, because a keyboard player already has the keys and a pad player has no better one (Wobbrock: selection-based entry is slow). A pad cannot type; its D-pad always moves focus, so a field never traps it, and the platform's keyboard arrives as key events the field already takes (Valve).
- **T4. A field never traps focus (WCAG 2.1.2).** Left, Right, Home, End and the editing keys are the field's. Up, Down, Tab and Escape leave it: Up and Down move focus to the neighbour (the crate's navigation reads the key itself, and a one-line field has no use for them), Tab is Bevy's, Escape is Bevy's (it ends the edit and bubbles on, so inside a `dialog()` it is also Back). Clicking the label, the padding or the field focuses the field (an `AcquireFocus` observer on the row).
- **T5. The host's signal: `TextEditing`**, a public marker on the row while its field has focus, and `is_text_editing`, a run condition on it. A game's own hotkeys wait on it (Sanctuary's `UiHolds`, the editor's "keystrokes vanished" bug are what it replaces), and a game that opens the platform's keyboard does it on `Add<TextEditing>` (Valve). The crate itself opens nothing.
- **T6. A hint, optional and supplementary.** `TextHint("Enter a name")` on the row shows muted text in the field while it is empty (Bevy has none). Text meeting 4.5:1 on the field in both presets; the label never moves into it. `TextLimit(n)` caps the characters.
- **T7. Looks like the family.** The field is an outlined box in the panel's own colour (an opaque fill, so the hint and the typed text contrast with the same colour the crate already checks): the `outline` frame at rest (3:1 on the panel, the meter's rule), `text_muted` under the pointer, the `focus_ring` while it holds focus, **whether or not focus is "visible"**: a pointer press hides every other ring, but a caret in a field says who has the keys, so its frame stays lit. The caret is the text colour; the selection the accent with the accent's text, the pair the lit button already holds at 9:1.
- **T8. One path.** Styled only in `apply_skin` by `Role`; the field's caret and selection colours are `TextCursorStyle` set there. The row is a `Role::TextInput` with the toggle's frame and padding so its label stands where every row's does.
- **T9. Nothing runs at rest.** `TextEditChange` is an observer, `track_text_editing` runs on a changed `InputFocus`, `update_text_inputs` on a changed `TextInputValue`, `TextHint` or `TextLimit`.

## Parameters

|part|value|
|---|---|
|widget|`text_input(label, value) -> impl Bundle`: a `Role::TextInput` row (`TextInputValue`), holding the label and a cell with the field (`Role::TextField`: Bevy's `TextInput` + `EditableText`, `TabIndex(0)`, `AutoDirectionalNavigation`, `Hovered`) and the hint (`Role::TextHint`, absolute over the field, `Pickable::IGNORE`)|
|state|`TextInputValue(pub String)`; `TextEditing` on the row while its field has focus; optional `TextHint(pub String)`, `TextLimit(pub usize)` on the row|
|event|`ValueChange<String>` on the row; `is_final` false per edit, true on Enter and on leaving after an edit|
|keys|characters, Backspace, Delete, Left, Right, Home, End, selection, copy, cut, paste, IME: Bevy's; Up, Down: focus moves; Tab: Bevy's; Escape: Bevy's blur, then bubbles; Enter: `is_final`|
|look|row: a toggle's frame (a line, transparent), `padding * 1.5` at the sides, `min_height` `min_target`; field: `min_height` `min_target` (44 px), `0.6 padding` at the sides, a line of frame, `radius`, opaque panel fill; width 16 characters of "0" at the body size (scales with `text_scale`); hint `text_muted`; caret `text`, selection `accent`, selected text `accent_text`, an unfocused selection none|
|set|`track_text_editing` in `GooeySystems::Navigate`, first; `update_text_inputs` in `GooeySystems::Style` before `apply_skin`|

## Validation targets

- Typing a character into the focused field changes `TextInputValue` and sends one `ValueChange<String>` with `is_final: false`; Backspace likewise; the caret moving sends nothing. Enter sends one with `is_final: true` and the same text; focus leaving after an edit sends one with `is_final: true`, once; leaving after no edit sends nothing.
- A `TextInputValue` set from code shows in the field with the caret at its end and sends no `ValueChange`; the hint is shown exactly while the value is empty; `TextLimit` stops the field taking more (typed or pasted) and does not cut a value the game set.
- Left and Right move the caret and never focus; Up and Down move focus to the neighbour above or below; the D-pad (any of the four) moves focus; Tab leaves; Escape blurs and sends one `Cancel` to an open dialog only; a click on the label focuses the field.
- `TextEditing` is on the row exactly while its field has focus; an `AutoFocus` on the row focuses the field; a dialog opens with its first field focused.
- Contrast, both presets: typed text on the field 4.5:1; the hint on the field 4.5:1; the resting frame on the panel 3:1; the focused ring on the panel 3:1; the selected text on the selection 4.5:1. `GooeySkin::problems()` empty.
- The row is a toggle's height (52.8 px at the default skin, to within the field's 44 px minimum), the field at least 44 px high, and neither changes by a pixel while the player types a 60-character line (the text scrolls inside), at 1x, 2x and 1.5x text. One whole-pixel line on all four sides.
- A disabled field takes no character and shows muted text. Thirty idle frames write nothing.
- Frame time of the gallery within 0.5 ms of before.

## What great looks like

A "Name" row in the gallery's Options tab, a field a button's height with a hint ("Enter a name") in muted text. The player tabs onto it: the frame lights blue and a caret blinks at the left, and the hint stays. They type "Aria": the hint goes, the caret follows, the panel does not move by a pixel, and the game saw four changes and, on Enter, one final. Down moves to the next row, the frame goes back to the quiet outline, and the typed name stays. On a pad the same row is a frame the D-pad passes over; the platform keyboard, when the host opens it on `TextEditing`, types into it.

## Deviations

- **One line only.** Bevy's `allow_newlines` and `visible_lines` exist, but a multi-line area needs Up and Down for itself and so a different leave-the-field rule; a design default, the open gap.
- **No password mask, no filter, no undo.** Bevy lists masking and undo as not implemented; `EditableTextFilter` exists but is reached only through the field, and a filter is a game's rule (a name, a number); `TextLimit` is the one rule every game needs. A design default.
- **Escape does not revert.** It blurs, as Bevy's does, and the text stays as typed (and the blur sends a final change); inside a dialog the same press is also `Cancel`, so the game decides what a cancelled dialog does with its text. A design default (Bevy's).
- **Up and Down leave the field though Bevy's handler also consumes them.** Bevy moves the caret to the line's start for a press the crate's navigation also acts on; on a one-line field that is harmless and the focus move wins. A design default.
- **A pad has no way to edit and the crate adds none.** No on-screen keyboard (Wobbrock: selection keyboards are slow, and every platform ships its own, Valve requires it); `TextEditing` is the hook. A design default.
- **The frame stays lit under focus that is not "visible".** Every other ring follows `InputFocusVisible`, which a pointer press clears; a field keeps its ring because its caret already shows the focus (browsers do the same for text fields). A design default.
- Assumed decisions (owner may override), all above: `ValueChange<String>` on the row with Bevy's `is_final`; Enter always final and not leaving the field; Escape as Bevy has it; `TextEditing` and `is_text_editing` as the host's hook; the hint and limit as components on the row; the field's fill the panel's colour; a 16-character width.

## Pending index

- Index (all absent from home-still on 2026-11-01): Wobbrock, Myers & Aung, *Writing With A Joystick* (figshare, doi:10.1184/r1/6470696); Wilson & Agrawala, *Text entry using a dual joystick game controller* (CHI 2006); Költringer et al., *Game controller text entry with alphabetic and multi-tap selection keyboards* (CHI 2007); NN/g, *Placeholders in Form Fields Are Harmful*; Valve's Steam Deck recommendations page; W3C WCAG 2.2 *Understanding 2.1.2* and *3.3.2 Labels or Instructions*. `paper_download` needs a DOI the provider resolves; the figshare record is a thesis-style upload.
