# A text area that fills its host

`text_area_fill(label, value, rows)` is a [`text_input`](text-input.md)'s area as tall as the room its
parent leaves: a project's agent guide, a log, a document. It is a `text_area` in every other way —
`TextInputValue`, one `ValueChange<String>` per edit, `TextLimit`, `TextHint`, `FieldProblem`,
`TextEditing`, Ctrl+Enter to end the edit — with `rows` as a *least* height rather than the height.

```rust
fn guide(mut commands: Commands) {
    commands.spawn((
        text_area_fill("Guide", std::fs::read_to_string("AGENTS.md").unwrap_or_default(), 10),
        observe(|change: On<ValueChange<String>>| {
            if change.is_final {
                info!("{} characters", change.value.chars().count());
            }
        }),
    ));
}
```

- **It fills its host.** Put `flex_grow: 1.0` on a parent of yours, or spawn it in a column that has
  the room: the field takes what is left after its label, and a host that gives it nothing still gets
  `rows` lines.
- **The wheel scrolls it**, wherever the pointer is over the field or its rail, and one line is the
  platform's `MouseScrollPixelsPerLine`.
- **Its rail** shows while there is more text than the field draws: `Role::ListRail` with a
  `Role::ListThumb`, the look a `list` uses. A press or a drag on the rail scrolls to that share of the
  text and does not take focus (a focus would put the view back at the caret).
- **The caret is kept in view** whenever it moves — a keystroke, an arrow, Home and End, Ctrl+End for
  the document's end, a click in the text, or your own write to `TextInputValue`. The wheel may leave
  the view anywhere; the next caret movement brings it back.
- **A very long line wraps** at the field's width (a word with no break opportunity in it still
  breaks), and there is no horizontal scrolling: a document reads as a document.
- **It costs nothing when still.** A settled area writes no rail node and does not move its viewport.

Three lengths meet in a field and they are not the same number: a `Val::Px` is a skin pixel,
`ComputedNode` and `TextViewport` are in physical pixels, and a pointer's position is in the window's
logical pixels. The widget converts at each edge, so a 1080p window (the interface at 1.5x) scrolls a
press a quarter down the rail a quarter into the text. `docs/research/text-area-fill.md` has the
table.
