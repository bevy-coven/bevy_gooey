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
- Offer `ToastSettings::duration_scale` in your settings screen: Xbox Accessibility Guideline 116 asks that a player can stretch the time notifications stay up to at least ten times the default (any value above zero works). `fade_seconds: 0` removes the fade and the opening of room for a player who wants no motion. `max_visible` (3) sends the oldest into its fade when a fourth arrives, and `anchor` picks one of six places (bottom centre by default); the newest stands nearest the edge.
- A toast takes no input and no focus: every node of it is `Pickable::IGNORE`, it is no tab stop, and a press, a key or a D-pad direction goes to what is under it. The mark down its left edge is the accent for `toast` and `danger` for `toast_alert`; the words carry the kind too. `GooeySkin::problems()` checks both marks against the toast's surface at 3:1.
- A toast is not announced to screen readers yet: it is visual only.
