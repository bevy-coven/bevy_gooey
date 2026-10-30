# bevy_gooey loop log

## Iteration 0 — 2026-10-30 — repository setup

By the owner's session, not the loop. The crate's first slice: `GooeySkin` (dark and parchment presets, `text_scale`, `problems()`), `GooeyFonts`, `Role` and `apply_skin`; the widgets `screen`, `panel`, `menu`, `heading`, `label`, `button`, `meter`, `Tooltip`, `dialog`; `navigate_by_input` and `gamepad_select`; `UiFeedback`; `GooeyStylePlugin`, `GooeyNavPlugin`, `GooeyPlugins`; the gallery example with its capture mode. Design rules R1–R7 are in `docs/research/gooey.md`.

### Ranked backlog

Take these in order, after any bug and any filed issue:

1. `Shortcut { key: KeyCode, pad: Option<GamepadButton> }`, a component on a button. It triggers `Activate` when pressed, and the button shows its key glyph beside the label. Source: R5, novice-to-expert rehearsal.
2. `HudDetail { Full, Minimal, Hidden }`, a resource HUD widgets read. Source: R1, *Removing the HUD* (experts want less).
3. Slider, checkbox and toggle wrappers over `bevy_ui_widgets`.
4. A settings-screen kit for the accessibility options vocabulary: text size, contrast preset, remap row. Source: R2.
5. Scroll list, tabs and toast.
6. A moderate press and focus animation. Source: R4, "medium" juice.
7. A bevy_emerge consumer of `GooeySkin`, filed on bevy_emerge as "Family F13: Game UI skins (`bevy_gooey`)".

### Known gap, seen while writing slice 1

- An open `dialog()` keeps Tab inside it (its modal `TabGroup`), but the arrow keys and the D-pad go through `AutoDirectionalNavigator`, which considers every `AutoDirectionalNavigation` entity on screen, so they can move focus to a button behind the scrim. Not yet reproduced in a test; it is a "dialog that leaks input" defect and ranks above the backlog once confirmed.
- A tooltip opens flush against its widget's bottom edge and covers the widget below it (seen on the gallery's Options tooltip over Quit, macOS 2026-10-30): it needs a gap from the skin and a placement that keeps the next control readable.
- A meter's fill has square corners inside the track's rounded ones (`Overflow::clip` clips to the rectangle, not the radius).

### Verified at setup (laptop, macOS, Apple M5, release build)

- `GOOEY_FRAMES=120` stills: dark, parchment and `GOOEY_TEXT_SCALE=1.5`; `mean_ms` 16.66 (vsync-bound) for each.
- Driven with real key events: Down then Enter on Options switches the skin (`feedback Focus`, `feedback Activate`); Shift+T enlarges every text; Enter on Quit opens the dialog with Stay focused; Tab cycles Leave and Stay only; Stay closes it and focus returns to Quit; Leave exits. Hovering Options shows its tooltip and the hovered fill.
