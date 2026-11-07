# Toasts

A toast is one spawn from any system, with no layout and nothing to set up:

```rust
fn save(mut commands: Commands) {
    commands.spawn(toast("Game saved"));
}

fn pickup_failed(mut commands: Commands) {
    commands.spawn(toast_alert("Inventory full"));
}
```

- The first toast creates the one `ToastStack` (a full-window, pointer-transparent column above a `dialog`'s scrim and below a tooltip) and every toast becomes its child in the frame it is spawned. Despawn the entity to take a toast back sooner.
- A toast stands as long as its message takes to read: `(base_seconds + characters ÷ chars_per_second) × duration_scale`, 2.5 s for "Game saved" at the defaults, then fades out and despawns. It runs on real time, so a paused game's menu still ages it. It fades in as the room it holds opens and fades out as that room closes, so the toasts round it glide to make way and close up and none of them jumps.
- Offer `ToastSettings::duration_scale` in your settings screen: Xbox Accessibility Guideline 116 asks that a player can stretch the time notifications stay up to at least ten times the default (any value above zero works). `fade_seconds: 0` removes the fade and the opening of room for a player who wants no motion. `max_visible` (3) sends the oldest into its fade when a fourth arrives, and `anchor` picks one of six places (bottom centre by default); the newest stands nearest the edge. `inset` keeps the stack off the window's edges, logical px on each side: `ToastInset::top(34.0)` with `ToastAnchor::TopCenter` stands the toasts under a 34 px menu bar (with the stack's own padding below it), and `ToastInset::bottom(..)` keeps them above a status band; rewrite it when the bar's height changes.
- A toast takes no input and no focus: every node of it is `Pickable::IGNORE`, it is no tab stop, and a press, a key or a D-pad direction goes to what is under it. (A toast you give a button, below, has that button and nothing else that does.) The mark down its left edge is the accent for `toast` and `danger` for `toast_alert`; the words carry the kind too. `GooeySkin::problems()` checks both marks against the toast's surface at 3:1.
- A toast is not announced to screen readers yet: it is visual only.

## Actions, sticky errors, repeats and the log

```rust
fn deleted(mut commands: Commands) {
    commands.spawn(toast("Item deleted").action("Undo", "undo"));
}

fn failed(mut commands: Commands) {
    commands.spawn(toast_alert("Could not write the save").action("Show", "show").sticky());
}

fn heard(action: On<ToastAction>) {
    match action.id.as_str() {
        "undo" => { /* put the item back */ }
        "show" => { /* open the folder */ }
        _ => {}
    }
}
```

![The notifications example: Delete item spawns the toast "Item deleted" with an Undo button, Save game pressed three times shows one "Game saved x3", Break something shows "Could not write the save: the disk is full" wrapped over four lines with Show and Dismiss buttons a gap apart, and the Notifications panel lists every toast newest first as "0:03  ! Could not write the save: the disk is full"; the ring goes onto Undo and Enter makes the game hear "undo"](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/notifications.gif)

- **`.action(label, id)`** adds a button (a button's look, fading with its toast). Pressing it (Enter, Space, South or a click) sends **one `ToastAction { toast, id }`** (an `EntityEvent` on the toast, observable globally) and dismisses the toast: it fades out at once, so a second press does nothing. The button is a tab stop, so the keys and the pad reach it, but it never takes focus by itself: a game's focus does not jump. A toast with a button stands at least `ToastSettings::action_seconds` (6, times `duration_scale`) so it can be reached; offer the same command somewhere that does not time out, as WCAG 2.2.1 asks of anything that is only reachable from a timed message. Prefer one button.
- **`.sticky()`** (on `toast` or `toast_alert`) stands until the player dismisses it, with a **Dismiss** button that sends no `ToastAction`. It is not counted against `max_visible`, so a newer toast never pushes it out.
- **A toast's buttons keep their size.** A long message wraps (the body stops at 24 text sizes) but a button never gives up its padded label to it: each is a full button wide, half a gap of margin each side (so two stand a gap apart and the last is inside the frame), and centred on the toast's height beside a message of several lines. A toast without a button is laid out as before.
- **Repeats merge.** A toast identical (kind, words, buttons) to one that arrived less than `ToastSettings::merge_seconds` (2; 0 turns it off) ago folds into it: it reads "Game saved x3" (ASCII `x`: the host supplies the font), stands again from the start of its hold, and **the entity you spawned is despawned** (the one standing is the one that stays; `Toast::count()` says how many).
- **`ToastLog`** is a resource that keeps the last 200 toasts spawned (`ToastLog::CAPACITY`), merged or not, as `ToastLogEntry { seconds, kind, text }` (seconds on `Time<Real>`). It is plain data: read it, save it, `clear()` it.
- **`notification_center()`** is a scrolling list of the log, newest first, a row for each entry as `m:ss  words` (an alert's words start with `! `: the mark is not only a colour) or "No notifications yet". A row is a button, so the keys and the pad scroll it like any list; put it in a panel or a dialog of your own. `examples/notifications.rs` shows all of it.

Research: [`docs/research/toast-actions.md`](../research/toast-actions.md).
