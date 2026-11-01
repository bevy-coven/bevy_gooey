# Number field: scrub or type a number

Researched 2026-11-01 (iteration 35, #17 G-13). home-still's index has nothing on numeric entry (`hs distill search "numeric input field drag to change value slider alternative precision"` returned a paper on a game of office activities). The sources are the three kinds below, fetched today; the missing ones are under `## Pending index`. This is the first slice of #17: **`number_field`, `vec_field` and `MixedValue`**; the colour field and its picker are the second (see `## Deviations`).

## Sources

1. W3C, *Understanding Success Criterion 2.5.7: Dragging Movements* (WCAG 2.2, Level AA) — https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html — craft (accessibility): a drag is never the only way.
2. Blender Foundation, *Input Fields*, Blender manual — https://docs.blender.org/manual/en/latest/interface/controls/buttons/fields.html — shipping: the number field every 3D tool's inspector copies. Unity, *FloatField* (UI Toolkit, 6000.6) — https://docs.unity.com/en-us/engine/6000.6/manual/uitoolkits/uielements/uie-element-ref/uie-uxml-element-float-field — shipping: the label that drags, the mixed value, expressions as an option.
3. W3C WAI-ARIA Authoring Practices, *Spinbutton Pattern* — https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/ — technical: which keys a number field owns, and what it exposes.
4. This crate's own `docs/research/text-input.md` (the field it is built on), `controls.md` (the slider's keys and D-pad) and `split.md` (a drag that is never a click); Bevy 0.20's `PointerDrag*` events (`distance` from the press).

## Craft principle

WCAG 2.5.7: "All functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging, unless dragging is essential". And, of exactly this widget: "providing a text input can be an acceptable single-pointer alternative to dragging. For example, an input beside a slider could allow any user to enter a precise value for the slider." A scrubbed number is a dragging movement, so the typed field is not an extra, it is the conformance; and the keyboard (SC 2.1.1) and the pad need steps of their own.

## How shipping games do it

Blender: "To change the value with the mouse, hold down LMB and drag to left or right." "Hold Ctrl to snap to the discrete steps while dragging or Shift for precision input." "Press LMB or Return to enter value by typing it with keyboard. When doing this, press Return or LMB outside the field to apply the change, press Esc or RMB to cancel". So: drag scrubs, a click types, Enter applies, Escape reverts, Shift is fine and Ctrl is coarse.

Unity's `FloatField` lists a `labelDraggerVariantUssClassName` ("USS class name of labels in elements of this type, when there is a dragger attached on them"), a `mixedValueLabelUssClassName` ("USS class name of elements that show mixed values") and `support-expressions` ("Indicates whether the field supports expressions that can be evaluated into a value"), off by default. The label is the dragger, a mixed multi-selection is a state of the field, and expressions are opt-in.

## Technical

APG, spinbutton: "The text field is usually the only focusable component because the increase and decrease functions are keyboard accessible via arrow keys. Typically, the text field also allows users to directly edit the value." `Up Arrow` increases, `Down Arrow` decreases, `Home` and `End` set the minimum and the maximum, `Page Up` and `Page Down` take a larger step; "Focus remains on the text field during operation." The element exposes `aria-valuenow`, `aria-valuemin` and `aria-valuemax`.

## Model adopted

- **N1. A number field is a `text_input` row plus a number.** `number_field(label, value, step, min..=max)` is the text row (the same field, caret, selection, paste, focus and `TextEditing`), with `NumberValue(f32)` as the widget's state, `NumberStep(f32)` and `NumberRange { min, max }` beside it. Typing is never a mode (text-input T3).
- **N2. The label is the dragger** (Unity), and so is the row's padding: a press and drag that starts on the row itself scrubs; one that starts in the field is the field's. The label is `Pickable::IGNORE` already, so a drag from it lands on the row. `value = start + distance_px / UiScale × step` (so 50 px at step 0.1 is 5.0), Shift one tenth of that, Ctrl ten times it (Blender), the result rounded to the digits of the rate (no `5.000001`) and held to the range. One non-final `ValueChange<f32>` per change and one final at the end.
- **N3. Typed values apply on Enter or on leaving, and Escape reverts.** The row's final `ValueChange<String>` is parsed (`model::parse_number`): a number is clamped to the range and sent as a final `ValueChange<f32>`; anything else is **refused with its reason** — `NumberRefused { reason }` on the row, the text goes back to the value. An expression (`1.5*2`) is refused as `NumberError::Expression` (Unity: off by default; the issue says unsupported). Escape puts the text back to the value before the edit (Blender) and the blur it causes sends nothing.
- **N4. The keys (APG).** Up and Down on the focused field step by `step` and Shift+Up, Shift+Down by ten (one final change each, held keys repeat); Left, Right, Home and End are the caret's, as in a text field; Tab leaves. The D-pad's Left and Right step it, as a slider's, and its Up and Down move focus. So a pad player has a way that is not typing, and a keyboard player never needs the mouse (WCAG 2.1.1, 2.5.7). Up and Down no longer leave a number field (the spinbutton says they step): Tab does.
- **N5. `MixedValue`** (a marker on the row) is a multi-selection whose members differ: the field is empty and shows a muted `MIXED_HINT` (`--`) as its hint, and any change the player makes (a scrub from the representative `NumberValue`, a typed number, a step) removes it and sets every member through the one `ValueChange<f32>`; Enter on the untouched field changes nothing. On a vec row it is `VecMixed(Vec<bool>)`, one flag a component, because the usual multi-selection differs in one axis.
- **N6. `vec_field(label, [f32; N])`** is a label and N number rows (axis letters X, Y, Z, W for up to 4; numbers past that), each a scrub handle, all keeping the one `VecValue`; a change in any is one `VecChange { index, value, values, is_final }` on the vec row, so a game applying a change to a mixed selection knows the axis (`NumberStep` and `NumberRange` as the row's, applied to every component).
- **N8. A scrub is not typing** (found by capturing it). The press on the label focuses the field, as a form's label does, and the first capture showed a caret and a blue ring while the number rolled, which reads as "edit mode". A scrub lets go of focus when the drag begins (`TextEditing` goes, so a game's hotkeys are back); a click without a drag still focuses. Focusing a number field selects its number (`SelectAllOnFocus`, Bevy's: a pointer press selects on release), so typing replaces it, as in Blender.
- **N7. One path, no new colour.** Styled only in `apply_skin` through the text row's roles; no `GooeySkin` field. A number is shown with the digits of its step, and more when the value needs them, so the text never lies (`model::format_number`).

## Parameters

|part|value|
|---|---|
|widget|`number_field(label, value, step, range) -> impl Bundle`, `vec_field(label, [f32; N], step, range) -> impl Bundle`|
|state|`NumberValue(f32)`, `NumberStep(f32)`, `NumberRange { min, max }`, marker `MixedValue`; `VecValue(Vec<f32>)` on a vec row|
|events|`ValueChange<f32>` (`is_final` false while scrubbing, true at the end, on Enter, on leaving, on a step); `VecChange` on a vec row; `NumberRefused { entity, text, reason }`|
|scrub rate|`step` per logical px; Shift ×0.1; Ctrl ×10; start from the value at the press|
|keys|Up, Down ±step; Shift+Up, Shift+Down ±10 steps; D-pad Left, Right ±step (repeat as the slider's); Enter applies; Escape reverts|
|width|the field is 12 characters of "0"|
|set|`update_numbers` in `GooeySystems::Style` before `update_text_inputs`; the key and pad steps are observers and `gamepad_adjust`|

## Validation targets

- Dragging 50 px with step 0.1 from 0 gives exactly 5.0 within a range of 0..=100 (the issue's acceptance), also at a UI scale of 1.5; a drag past the end holds at the range; Shift and Ctrl scale the rate; the drag's changes are non-final and the last is one final.
- Typing `2.5` and Enter sets 2.5 (one final); typing `150` in a range to 100 sets 100; `1.5*2`, `abc` and an empty field are refused with a `NumberError` and leave the value; Escape restores the text and sends nothing; leaving after a good edit applies it once.
- Up and Down step and move no focus; the D-pad steps and Up/Down on it moves focus; a held Up repeats; Tab leaves.
- `MixedValue` shows the `MIXED_HINT` over an empty field; a scrub, a typed number and a step remove it and send one change; Enter on the untouched field sends nothing.
- The row is a text row's size (a target high) and does not change size while scrubbing a value of 0 to 99999.9; the hint's contrast is the text hint's (4.5:1, all four looks).
- A game's `NumberValue` shows at once with no event; thirty idle frames write nothing; a disabled row does not scrub, step or take text.
- Frame time of the gallery within 0.5 ms of before.

## What great looks like

A Transform block of a properties panel: a "Position" row with three framed fields, each led by a letter, a "Scale" row beneath, and a "Health" row with `100.0`. The pointer over a letter turns to the resize cursor; dragging it rolls the number smoothly, left and right, with the digits steady in their width and no caret; letting go leaves it. Click a field and its number is selected: type `2.5`, Enter, and the number is `2.5`. A multi-selection shows a muted `--` in the field that differing values leave empty.

## Deviations

- **Slice one of #17.** `color_field` and the `color_picker` popover (HSV, hex, alpha) are the second slice; the acceptance line about `#ff8800` is theirs, so this pass does not close #17.
- **Scrub from the row's label and padding, not from the field.** Blender drags the field; Bevy's field takes the press for the caret, and a text field that also drags would give a press two meanings. Unity drags the label. A design default.
- **Up and Down step instead of leaving** (a text field's rule) because the spinbutton pattern says so; the leave is Tab and the pad's Up and Down. A design default.
- **Expressions are refused**, not evaluated, as the issue says. The refusal carries `NumberError::Expression`; a later pass can evaluate behind a setting.
- **No unit suffix, no wrap, no log scale.** Open gaps.
- **A typed value is held to the range, not snapped to the step** (the digits are the player's). A design default.
- **The mixed hint is two hyphens, not an em dash** (found by capturing it): the em dash drew as a missing-glyph box in the default font, and the host supplies the font. A design default; `MIXED_HINT` is public so a host with a fuller font can match it.
- **`selected_text_color` was drawn without focus** (found by capturing it). Bevy applies `TextCursorStyle::selected_text_color` whether or not the field has focus, and `apply_skin` set it to the accent's dark ink: a field left with its text selected (a number field selects on focus and a scrub leaves it) drew its number dark on no highlight. It is set only on the field that has focus now, which also fixes a `text_input` a player Tabs away from with a word selected.
- Assumed decisions (owner may override; on #17): the API is `number_field(label, value, step, range)` as asked, with `NumberValue` as state and `ValueChange<f32>` as the event; `vec_field` carries `VecValue(Vec<f32>)` and sends `VecChange` (an array's length is not a type a component can have without a generic, and the axis matters to a mixed selection); a mixed value is the muted `--` hint over an empty field; the typed `1.5*2` is refused as `NumberError::Expression`, with the reason in words.

## Pending index

- Absent from home-still on 2026-11-01: W3C, *Understanding SC 2.5.7 Dragging Movements*; W3C APG, *Spinbutton Pattern*; the Blender manual, *Input Fields*; Unity, *FloatField*.
