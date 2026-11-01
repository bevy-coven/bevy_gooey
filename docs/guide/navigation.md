# Navigation

- Arrow keys and the D-pad move focus to the nearest button in that direction (`navigate_by_input`); a pad moves focus, it never steers a cursor. The first key or D-pad press shows the focus ring (Bevy's `InputFocusVisible`) and a mouse click hides it again, so a mouse player never sees a stray ring and a pad player always sees theirs. A pad-first game that wants the ring from the first frame inserts `InputFocusVisible(true)`.
- Enter and Space activate the focused button (Bevy's own button handling), flip the focused `toggle` and cycle the focused `choice`; the pad's South button does all three (`gamepad_select`).
- Tab and Shift+Tab cycle through a `screen`'s buttons.
- An open `dialog` holds focus: focus moves to its first button when it opens, Tab, the arrows and the D-pad cannot leave it, and closing it puts focus back on the button that opened it. Put `AutoFocus` on the dialog's safest choice ("Stay", "Cancel") to start there instead. Escape and the pad's B reach it as a `Cancel` (below).
- A `Tooltip` opens when its widget has focus from a key or the pad, as it does under the pointer. It opens beside the widget (to the right in a column, below in a row, flipped at the window's edge), one step outside its panel, so it never covers the next button.
- A `Tooltip` is a view of its component: `commands.entity(button).remove::<Tooltip>()` takes the hint off (a "Locked: reach level 5" line once the player unlocks it), and inserting a new `Tooltip`, or editing `Tooltip.0`, changes the line the open tooltip shows.
- Put Bevy's `AutoFocus` on the button a screen should start on. With nothing focused (a click on empty space, a button despawned under the player), the first arrow or D-pad press lands there, or on the screen's first button in Tab order, and the next press moves on; with a `dialog` open it lands inside the dialog.
- A `list` scrolls to follow the focus a key, the D-pad or Tab gives its rows (with the next row kept in sight), PageUp and PageDown (the pad's triggers) move a viewport at a time, and Up and Down stay among a list's rows until they run out; see [lists](lists.md).
- `tabs` turn their page with a click, Left and Right on a focused tab (they wrap), the D-pad on a tab, and **LB and RB** (or Ctrl+PageUp and Ctrl+PageDown) from anywhere in the widget, so a pad player never goes back up to the strip; focus stays in the page and lands on the new page's first widget, and Up from a page goes to the selected tab. See [tabs](tabs.md).
- A focused `text_input` field takes the keys a caret and a typist need (characters, Backspace, Left, Right, Home, End, selection, paste), so Left and Right never also move focus off it; **Up, Down, Tab, the D-pad and Escape leave it**, so a field never traps a keyboard or a pad. A click on its label focuses it. See [text input](text-input.md).

## Back

Escape and any gamepad's East button (B on Xbox, Circle on PlayStation) are one event, `Cancel`, one per press. Observe it on the dialog or the screen the player is leaving; nothing is closed for you:

```rust
commands.spawn((
    dialog(),
    observe(|cancel: On<Cancel>, mut commands: Commands| {
        commands.entity(cancel.entity).despawn(); // focus goes back to what opened it
    }),
    children![(panel(), children![label("Leave the game?"), button("Stay")])],
));
```

- **It starts at the focused widget and bubbles up its parents**, so the innermost layer hears it first. `cancel.entity` is the entity your observer is on; `On::original_event_target()` is the widget that had focus. An observer that handled it calls `cancel.propagate(false)` so a layer above does not also act.
- **It never leaves a `dialog`.** A modal dialog is its own scope, as for the arrows and Tab: a Back inside it does not reach the screen under it, even one the dialog was spawned beneath.
- **A dialog can always be dismissed**: with nothing focused (a click on empty space) or focus left behind it, the open dialog still hears it.
- **With no dialog open and nothing focused, nothing is sent**: Escape is the game's own, for its pause menu. A pause screen of your own handles `Cancel` on its `screen()` to resume; open it from your own key system only when none is open.
- **A focused slider, choice, toggle, tab or list is not changed by it**, and a press that finds a target shows the focus ring, as every key does.
- Say it on screen: a `prompts()` legend with `prompt("Back", Prompt::BACK)` shows "Esc Back", or "B Back" once the player picks up a pad (the crate has no glyph art, so the keycap is the input's name in a key or a round cap; see [prompts](prompts.md)). With the prompt on screen a Back button is not also needed; a mouse player still wants one.
