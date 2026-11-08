# A dialog's default button

Researched 2026-11-07 for issue #66 (G-59), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune `crates/rune/README.md`, "Key routing" step 2 and its Return table: Return presses the window's or open modal's default button unless the focused element takes Return; a field with `on_submit` takes it, one without passes it to the default button; Space presses the focused control.
2. WAI-ARIA Authoring Practices, "Dialog (Modal)": the default action of a dialog is the one Enter performs from a field.

## Model adopted

- **`DefaultButton`** is a marker on one button inside a `dialog()` (or a `Confirm` built with `Confirm::default(index)`). Added without a `ButtonVariant` or `PrimaryButton`, it inserts `ButtonVariant::Submit`; one the host gave a variant keeps it.
- **Return in a single-line field** ends its edit (the final `ValueChange<String>`, as before) and then sends one `Activate` on its dialog's default button. It is the same observer, so the order is fixed: value, then press. Ctrl, Cmd, Shift and Alt Return do not press it. A text area keeps its newline (and its Ctrl+Return submit), and an IME in composition is left alone.
- **Return with nothing focused** presses the default button of the dialog on top. The key reaches the window when no entity has focus; a key that began at a focused widget and bubbled up to the window is not this case, and the observer ignores it.
- **A focused button keeps its own Return** (Bevy's button presses it). South keeps pressing the focused widget.
- **Disabled:** a default button with `InteractionDisabled` presses nothing.
- **Two defaults in one dialog:** a warning once per dialog, and the first in depth-first tree order wins.

## Where this differs from the issue and from Rune

- **`Confirm::default`** is a method on the builder, as the issue says. A `Confirm` opens with its safe choice focused, and a focused button keeps its own Return, so a Return typed at once presses the safe choice; the default answers only when nothing is focused. Give both the same index when the safe choice is also the usual one.
- **Which fields press it.** A plain single-line text row. A number or colour row whose text does not parse is refused and the old value put back, which the dialog must not then submit, so those rows keep their Return.
- **Which dialog.** With nothing focused, only the dialog the player is looking at (the visible modal one highest in the UI stack, the rule focus and Cancel use) answers; one under it, or a hidden one, is out of reach.
- **Gamepad:** the issue asks nothing of a pad's South on a field, and none is added; South presses the focused control.

## Proof

`tests/default_button.rs`, 6 tests: typing a name and Return gives the final change and then one `Activate` on Create, and a focused Cancel keeps its Return; Return with nothing focused presses it; a text area keeps its newline and a disabled default presses nothing; two defaults press the first; a default draws as Submit unless the host chose another variant; a `Confirm` with a default answers with it.
