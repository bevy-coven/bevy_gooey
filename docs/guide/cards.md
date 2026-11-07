# Cards: the card grid

`card_grid(cards)` is a project browser, a level select, a save list with thumbnails or a shop's
front page: cards of a picture above a name and a few quiet lines, in a row that wraps to fill its
width.

```rust
fn maps(mut commands: Commands, assets: Res<AssetServer>) {
    commands
        .spawn(card_grid([
            Card::new("Harbour")
                .image(assets.load("thumbs/harbour.png"))
                .line("10 x 10 m, 4 m tall")
                .line("Opened 2 hours ago"),
            Card::new("Keep").line("13 pieces"),
            Card::new("Ruins").line("Missing assets").disabled(),
        ]))
        // Focus moved to a card (not final), or the player chose one (final): Enter, Space,
        // South or a click.
        .observe(|change: On<ValueChange<usize>>| {
            if change.is_final {
                info!("open map {}", change.value);
            }
        });
}
```

- **Layout.** Every card is 14 body text sizes wide (it follows `text_scale`), its picture across
  the card at 16:9 and a placeholder of the same size when the card has none, so a row stays even.
  The grid is as wide as its parent and wraps a card to the next row when the next would cross its
  edge; put it in a `list` to scroll a long one.
- **Focus.** The grid is one tab stop, `CardCursor` (the card the player last had focus on); the
  arrows and the D-pad walk the cards by where they stand, so Down goes to the card under it.
  Focus moving to a card sends one non-final `ValueChange<usize>` on the grid.
- **Choosing.** `CardSelection(Option<usize>)` is the chosen card, drawn in the accent (Bevy's
  `Selected` is on it, and a screen reader hears it selected). The player's choice writes it and
  sends one final `ValueChange<usize>`; a game writes it to choose from code and hears nothing. A
  value past the last card chooses none.
- **Dimmed.** `Card::disabled()` puts `InteractionDisabled` on the card: its words are muted, its
  picture half transparent, and nothing chooses it, but focus still reaches it so the player can
  read why ("Missing assets").
- **More.** Each card is a `CardRef { grid, index }` entity and Bevy's `Button`, so a
  `ContextMenu(menu)` or a `Tooltip` goes on it like on any button (find the cards with
  `Query<(Entity, &CardRef)>`). The cards are spawned once: respawn the grid when the list changes.
- **Look.** A card rests on the opaque surface in the `outline` frame, so its quiet lines are
  `text_muted` at 4.5:1 (`problems()` holds "text_muted on surface"); under the pointer its fill is
  `hovered()` and its lines turn to `text`; chosen or pressed it is the accent with every word in
  `accent_text`. The focus ring replaces the frame.
- **Screen readers.** A card's name is its name, its value the quiet lines joined by commas.
