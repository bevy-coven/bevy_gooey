# Lists

`list(rows, children![..])` is a column of rows that scrolls when they do not all fit: save slots, an inventory, a key-binding screen. The rows are any widgets, in `children![]`:

```rust
fn load_game(mut commands: Commands) {
    commands.spawn((
        panel(),
        children![
            heading("Load game"),
            list(
                4.5,
                children![
                    (button("Meadow, 12:40"), observe(|_: On<Activate>| info!("load meadow"))),
                    button("Harbour, 09:15"),
                    button("Mill, 03:02"),
                    button("Cellar, 01:48"),
                    button("Orchard, 00:20"),
                    button("Autosave"),
                ]
            ),
        ],
    ));
}
```

- `rows` is how many rows tall it stands, a row being a button's height and the `gap` after it: `4.5` shows four rows and half of the fifth. A list that ends exactly on a row looks finished; one that cuts the next row does not (NN/g: "Include signifiers (such as cut-off text) to tell people that there is content below the fold"). The count follows the player's `text_scale`, and `ListView::rows` changes it live.
- The wheel and a trackpad scroll it, a drag on the thumb moves it and a click on the rail pages a viewport up or down (Bevy's own `ScrollArea` and `Scrollbar`). The rail, `2 × border` wide, shows only while the rows do not fit; its lane is always reserved, so rows never change width when others come and go.
- **It follows focus.** When a key, the D-pad or Tab moves focus onto a row, the list scrolls by the least that leaves a row and a half beyond it, moved on to the next row's top: the next row whole and the one after it cut, at every step, with a row flush at the top of the list as at rest, so the list visibly goes on, and it ends flush with the last row. A row taller than the list scrolls to its top, and a jump of more than a viewport (PageDown, a focus set from code) centres the row. A focus a pointer press set is not followed: a press that scrolled the content under the pointer would send its release to another row and lose the click.
- **It glides.** A scroll the crate makes (the follow above, and a table's cursor) travels to its place on a third of the skin's `motion.default` spring instead of jumping; a key asked again while it moves keeps the speed, so a held key crosses a long table as one motion with the cursor row never leaving the viewport. The wheel, a trackpad, a drag on the rail and a write to `ScrollPosition` take over and move 1:1, ending the glide (`ScrollGlide::go` asks for a glide from code). Reduced motion and no motion land the scroll in the frame it is asked for.
- **Its edges fade.** A `ScrollEdgeFade` (24 px, on every list and table viewport) draws a fade over the top edge while the content is scrolled down and over the bottom while there is more below, at strength `hidden px / 15` up to full, in the colour behind the list. An edge with nothing hidden under it draws nothing, and a fade never takes a click. `ScrollEdgeFade(0.0)` or removing the component takes the fades down.
- PageUp and PageDown, and the pad's left and right triggers, move focus a viewport up or down the list in one press. Up and Down stay among a list's rows until they run out (rows the list has clipped lie further off than the button under it, so geometry alone would step out past them), and a row the list has clipped away is never the neighbour of a widget outside it.
- To jump from code, set the `ScrollPosition` on the `ListView` entity (the first child of the list's root). A list builds every row it is given, so it is for tens of rows, not thousands; a list at its end does not scroll the panel behind it; a wheel notch is Bevy's 100 px.
- **A list is in the accessibility tree**: its viewport is an `Ak::List` whose set size is the rows it holds, and each row is an `Ak::ListItem` with its place among them, read from the rows' own column so a game that inserts or removes rows does nothing. (Bevy's `ScrollArea` and `Scrollbar` still carry no node of their own; the rail is silent.)

## Typing a name

With a row of a list focused, typing goes to the next shown, enabled row whose words start with what was typed within a second, and the list scrolls it into view: "c" is the first row starting with c after the focused one, "c" again the next, "ce" the first starting "ce", and a pause starts over. Space presses the row, as ever, and is not a name; nothing happens while a text field in the list has focus. A row's words are its own text or the first text under it.
