# Hold buttons

A delete the player cannot take back, a quit without saving, abandoning a run: `hold_button` confirms
only when it is held. A bar under the label fills at an even rate; when it is full the game hears one
`HoldConfirmed`. Letting go early confirms nothing.

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn delete_button(mut commands: Commands) {
    commands.spawn((
        hold_button("Hold to delete save", 1.2),
        observe(|_: On<HoldConfirmed>| info!("deleted")),
    ));
}
```

![The hold example: a Save slot 2 panel with a Hold to confirm row (Normal), a Hold to delete button with an empty bar under its label, a Keep the save button and the line "Nothing deleted yet". Enter held for two thirds of a second fills the bar to about half and letting go empties it; Enter held on again fills it across, the line reads "Deleted: 1" and the bar stays full until Enter is let go. Up and Right twice turn the Hold to confirm row to Short and then Tap; a tap of Enter on the button then reads "Deleted: 2" with no bar filled](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/hold.gif)

*`examples/hold`: the button, the setting row that edits `HoldSettings`, and a key held and tapped.*

## What it does

- **Is a button to the skin, and is not a `Button`.** It has a button's look, size, ring and hover, but it
  never sends `Activate`: Bevy's `Button` activates on a key's down edge and on a click, which would
  confirm a delete on a tap. Observe `HoldConfirmed`.
- **Holds by key, pad or pointer.** Enter or Space, or the pad's South, while it has focus; the primary
  mouse button while the pointer is over it. A hold needs a fresh press: an Enter still down from the press
  that opened the screen does not start one.
- **Says how far it has come.** The bar under the label is the meter's frame and fill, so it is the same
  accent in every preset and reads at the meter's 3:1. `HoldProgress` (0 to 1) is on the button for a game
  that wants a rising sound or a rumble.
- **Backs out for free.** Let go and the bar empties four times as fast as it filled; a new press resumes
  from where it stands. Moving the pointer off the button, or focus moving away, or disabling the button,
  ends the hold without confirming (XAG 107: a way to cancel before the up event).
- **Confirms once.** The frame the bar is full, one `HoldConfirmed`, one `UiFeedbackKind::Activate`; the
  bar stays full until the player lets go, and a hold still down does not confirm again.
- **Runs on real time**, so a paused game's menu holds.
- **Is a button to a screen reader**, named by its label. A screen reader cannot hold, so its Click confirms
  at once.

Put what the button does in its label ("Hold to delete"): the bar is the only other sign, and it is empty
until touched.

## The player's setting

A hold is a motor barrier, so a game that uses `hold_button` also offers the way out (XAG 107: "a single
input press instead of an input hold"; GAG: "offer a setting for players to choose"):

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn accessible_interactions(mut commands: Commands) {
    commands.insert_resource(HoldSettings { duration_scale: 0.5 }); // half as long
    commands.insert_resource(HoldSettings::tap()); // no hold at all
}
```

`HoldSettings::duration_scale` multiplies every hold's seconds (0 to 4). At 0 a hold is a tap: pressing
fills the bar at once and the confirm is sent **on the up event** (the key, South or the primary button
let go while still on the button), so the pointer still has its abort. `HoldSettings` is serde data like
the crate's other settings; `examples/hold` edits it with a plain `choice` row.

Use a hold for what cannot be undone and is rare. A hold costs a second every time: do not put one on
anything the player does often.

The maths (`model::hold_seconds`, `model::hold_step`) is engine-free. Research:
[`docs/research/hold-button.md`](https://github.com/bevy-coven/bevy_gooey/blob/main/docs/research/hold-button.md).
