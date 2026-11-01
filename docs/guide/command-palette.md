# The command palette

`command_palette()` is one modal field over a searchable list of every command a game or tool has, each
with its chord and, when it cannot run, why. The player types a few letters ("opn prj" finds "File: Open
Project"), moves with Up and Down, runs with Enter or a click and leaves with Escape. It is the way past a
menu hierarchy for someone who does not know the chord yet, and it shows them the chord as they go
(Malacria et al., *ExposeHK*: hotkeys "remov[e] the need to traverse a menu or tab hierarchy" but "few users
employ any form of shortcut interface").

![The palette example: the player opens it, types "opn prj" and "File: Open Project" is the lit first row with Ctrl+O at its end; Down and Enter run it and the line behind reads "Last command: File: Open Project"; opened again it lists that command first, "sav" shows File: Save and File: Save As... dimmed with "No project is open" where their chords would be, Down puts the ring on the second and Escape closes it](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/command-palette.gif)

## Three steps

```rust
use bevy::prelude::*;
use bevy_gooey::*;

fn setup(mut commands: Commands) {
    // 1. The game's commands, once, and again whenever they come and go or switch on and off.
    commands.insert_resource(CommandList(vec![
        PaletteCommand::new("project.open", "Open Project")
            .category("File")
            .shortcut(Shortcut::primary(KeyCode::KeyO)),
        PaletteCommand::new("project.save", "Save")
            .category("File")
            .disabled("No project is open"),
    ]));
}

// 2. The game's own chord opens it. `not(is_text_editing)` keeps it from opening over a field.
fn open_palette(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if Shortcut::new(Modifiers::PRIMARY | Modifiers::SHIFT, KeyCode::KeyP).just_pressed(&keys) {
        commands.spawn(command_palette());
    }
}

fn plug(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, open_palette.run_if(not(is_text_editing)))
        // 3. What a chosen command does. The palette is already gone and focus is back.
        .add_observer(|run: On<RunCommand>| info!("run {}", run.id));
}
```

That is all: the palette takes focus, closes itself on Escape, a click outside or a choice, and gives focus
back to the widget that had it (it is a [dialog](widgets.md#a-dialog) underneath).

## What the player gets

| key | does |
|---|---|
| a letter | filters the list as they type; the first match is lit |
| Down, Up | move the lit row, wrapping; focus stays in the field |
| Page Down, Page Up | move a page of eight, stopping at the ends |
| Enter | run the lit command (not while an input method is composing) |
| a click | on a row, run it; on the card's padding, nothing (focus stays in the field); outside the card, close |
| the pointer | moving over a row lights it, so the pointer and the keys share one lit row |
| Escape | close, focus back where it was; Left, Right, Home, End are the field's caret |
| D-pad Up and Down, South | on a pad, move the lit row (a held direction repeats) and run it |

A command that is off (`PaletteCommand::disabled(reason)`) is listed, can be lit, shows its reason where its
chord would be, and runs nothing; switching it on in `CommandList` makes it run in the open palette in the
same frame. Eight rows show at once and scroll under the lit one; "11 of 32" at the bottom says where.

## What matches, and in what order

The text matched is what the row shows: `"Category: Title"`, so "file open" finds "File: Open Project".
`model::fuzzy_score(query, text)` is the engine-free function: the query is split on whitespace, every term
must occur in the text as a subsequence in any order, ignoring case, and a term scores best where its
letters start words or follow each other (`opn` in "**Op**e**n**" beats `opn` in "Reopen"), with a small
penalty for each letter skipped and for a longer text.

Order: with an empty query, the commands in `PaletteHistory` first (most recent first) and the rest
alphabetically; with a query, the commands that match, recent ones first, then the rest best score first.
`PaletteHistory` holds the last ten ids that ran; it is serde data (feature `serde`) so a game can keep it
between runs, and clearing it forgets them.

## Parts a host reads

- `CommandList`, `PaletteCommand` (`id`, `title`, `category`, `shortcut`, `enabled`), `PaletteHistory` and the
  `RunCommand { id }` event, sent once per choice and never for a disabled command.
- `CommandPalette` is on the open palette: `results()` (indices into the list, in the order shown),
  `selected()` and `selected_command(&list)`. To toggle the palette on its chord, close it if one is open
  (`Query<Entity, With<CommandPalette>>`).
- `PALETTE_ROWS`, the eight reused rows; `update_palette` (Style, before `apply_skin`) and
  `palette_by_input` (Navigate) for a host that registers the systems itself.

## What it does not do

- **It does not draw the matched letters** in bold, and the mouse wheel does not scroll the eight rows
  (Up, Down and the page keys do).
- **Its two sentences are English** ("Type a command", "No command matches"), like a menu's "Nothing here".
- **It does not look in anything but the list**: a game that wants recent files or settings in it lists them as
  commands.
- **A long reason or title does not wrap**: keep a reason to a short phrase.
