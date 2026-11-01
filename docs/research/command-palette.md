# Command palette: one searchable list of every command, with its chord and why it is off

Researched 2026-11-01 (iteration 29). Issue #11 (G-07, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item: a tool with hundreds of commands cannot hang them all in menus, and a game with a console, a debug menu or a mod list wants the same thing, a text field over the list of what can be done, found by typing a few letters of its name.

## Sources

1. Malacria, Bailly, Cockburn and others, *Promoting Hotkey use through rehearsal with ExposeHK*, CHI 2013, home-still `10.1145_2470654.2470735` (craft: why a command list shows each command's chord, and what a chord saves).
2. Godot Engine, *Default editor shortcuts* and `EditorCommandPalette` (docs.godotengine.org 4.3 and stable), the palette of a shipped game engine's editor, opened with Ctrl+Shift+P (shipping).
3. W3C WAI-ARIA Authoring Practices, *Editable Combobox With List Autocomplete Example* (https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-autocomplete-list/), the keyboard contract of a text field over a list, and *Managing Focus in Composites Using aria-activedescendant* (technical).
4. The scikit-learn API paper already in the corpus (`docs/research/corpus.md`): one obvious way to do a thing, so the palette has one spawn and one event.

## Craft principle

ExposeHK: "Hotkeys ... offer a shortcut alternative to pointer-based selection of commands from toolbars and menus. Their efficiency stems from three mechanical advantages over pointing: ... they allow a wide range of commands to be selected with a single key combination, thus removing the need to traverse a menu or tab hierarchy." And: "hotkeys are under-used: several studies have demonstrated that few users employ any form of shortcut interface". A palette is the way past the menu hierarchy for the player who does not yet know the chord, and it is a place to learn it: every row shows its chord at the right edge, as the menus do, so using the palette teaches the shortcut that skips it. A command that is off says why instead of vanishing, so the player learns what it needs (the menus' rule, iteration 27).

## How shipping games do it

Godot 4's editor, a shipped engine and the tool a game is made in: "Action name: Command Palette. Windows, Linux: Ctrl + Shift + P. macOS: Cmd + Shift + P." (*Default editor shortcuts*), "Open the editor's command palette to run any action." The class doc describes it as an "Object that holds all the available Commands and their shortcuts text", so the list carries the shortcut text, and "Command key names use slash delimiters to distinguish sections", so a command has a section (here: a category). Quick Open next to it "fuzzy-search[es] and jump[s] to any file in the project". The same shape (a modal field at the top of the window, a list under it, the chord at each row's right edge, Enter runs, Escape closes, recently used first) is in VS Code, Sublime Text, Lapce and every editor since; the player's hand never leaves the keyboard. This crate follows that shape; the row look is the menu's.

## Technical: the keys

The APG combobox with a list popup: DOM focus stays in the text box while visual focus moves in the list ("`aria-activedescendant`"). Its keys: Down Arrow "Moves visual focus to the next option. If visual focus is on the last option, moves visual focus to the first option", Up Arrow the reverse ("This wrapping behavior is useful when `Home` and `End` move the editing cursor"), "Printable Characters: Moves visual focus to the textbox. Types the character in the textbox. Options in the listbox are filtered based on characters in the textbox", Escape closes it, and "Managing aria-activedescendant visibility is essential to accessibility for people who use a browser's zoom feature": the active row is always scrolled into view. Left, Right, Home and End stay the caret's. A palette is this pattern with the listbox always open and Enter running the active option.

Bevy 0.20 supplies the field (`bevy_ui_widgets::TextInput`, wrapped by `text_input`), the modal scope (`dialog`'s `TabGroup` and `Dialog`, which also give focus in, focus back and `Cancel`), the row (`Button`), and `PointerMove`/`PointerClick`. Its text field consumes Up and Down (single-line: `TextEdit::Up`/`Down`), so the palette reads them from `ButtonInput`/`KeyboardInput` like `navigate_by_input`, which it must be kept from moving focus.

## Model adopted

- **P1. The list is a resource, the palette a bundle.** `CommandList(Vec<PaletteCommand>)` is the one list (`id`, `title`, `category`, `shortcut`, `enabled: Result<(), String>`); `command_palette()` is the modal, a scrim, a card, a text field and a fixed number of rows (8) that are reused, so typing spawns and despawns nothing and 5000 commands cost one scoring pass a keystroke. The game spawns it when its chord is pressed and that is all.
- **P2. One event.** Enter or a click on an enabled row despawns the palette and then sends `RunCommand { id }` (global `Event`, observed with `app.add_observer`). Despawning first gives focus back to where it was, so a command that opens a dialog or moves focus leaves it where it put it. A disabled row runs nothing, shows its reason in place of its chord, and is still selectable (the menus' rule: it is focusable, so its reason can be read).
- **P3. Order.** Empty query: recently run commands (`PaletteHistory`, most recent first), then the rest alphabetically by what the row shows. With a query: the commands that match, recently run ones first (in history order), then the rest by score, best first, ties alphabetical. The row shows "Category: Title", and that is the text matched, so "file open" finds "File: Open Project".
- **P4. Matching is `model::fuzzy_score(query, text) -> Option<u32>`, engine-free.** The query is split on whitespace; every term must occur in the text as a case-insensitive subsequence (so "opn prj" finds "Open Project"); the best alignment of each term scores a point per character, more for a character that starts a word or follows the one before it, less for the gap before it; the terms' scores add; a shorter text beats a longer at the same alignment. No match is `None`.
- **P5. The keys are the combobox's.** Focus stays in the field. Down and Up move the active row (wrapping), PageDown and PageUp a page, the D-pad Up and Down the same, Enter and South run it, Escape closes (the dialog's `Cancel`), Left, Right, Home, End and the rest are the caret's. The active row is the one lit with the menus' look and the focus ring (no fill alone: the ring is the 3:1 indicator); the pointer moving over a row makes it the active one; a click runs it. Whatever the query does, the active row starts at the first match, and it is always inside the eight shown (the window scrolls by the active row). "n of m" shows when there are more matches than rows.
- **P6. Nothing to wire for sound or looks.** `UiFeedback::Focus` on a move, `Activate` on a run; every colour and size is an existing skin token (the menu rows'), so no new check is added to `problems()`.

## Parameters

| value | choice |
| --- | --- |
| rows shown | 8 (`PALETTE_ROWS`), reused |
| history | 10 ids (`PaletteHistory::LIMIT`) |
| card width | 40 text sizes, at most the scrim's inside (window less 5% a side) |
| card top | 5% of the window's height below the scrim's safe edge (5% of its width), about a seventh of the way down at 16:9, so the field never moves while the list grows or shrinks |
| scrim | the dialog's, z 100, closes on a click outside the card |
| key repeat | the keyboard's own (held Down repeats), the D-pad's `repeat_steps` rate |

## Validation targets

- 200 commands, query "opn prj": "Open Project" is the first row; Enter sends exactly one `RunCommand` with its id and the palette is gone (`tests/palette.rs`).
- Empty query lists recents first then alphabetical; a recently run command that matches outranks a better score; a query that matches nothing shows "No command matches" and Enter runs nothing.
- A disabled command shows its reason, Enter and a click run nothing and the palette stays open with focus in the field.
- Up and Down never move focus off the field and never leave the caret at the start of the text (typing after an arrow appends); Down wraps; the active row is always among the rows shown; PageDown moves a page.
- A click on a row runs it once; a click on the card's padding keeps focus in the field; a click on the scrim closes it; Escape closes it and focus goes back to the widget that had it.
- At 100% to 200% text every row, the field and the card stay inside the window and no text leaves its row; four presets keep every text at its contrast (the menu pairs).
- Nothing is written while the palette idles, and nothing is sent while it is closed.

## What great looks like

VS Code's and Godot's: a card near the top of a dimmed screen, the field framed with the ring and its caret, below it rows of one line with the category in the title, the chord muted at the right edge; the active row a bar the width of the card with the ring; a disabled row dimmed with its reason where the chord would be; "No command matches" in muted text. It reads as the crate's menus' sibling, not a web widget.

## Deviations

- The issue's `PaletteCommand.enabled` is kept (`Result<(), String>`); the `category` is `Option<String>` (a command with none shows its title alone).
- Recent commands come first among the matches of a query (the issue's order), though a much better score could argue for the other way; `PaletteHistory` is the game's to clear.
- The palette matches the row's text, "Category: Title", not the id.
- The field's frame is as wide as the rows' lit bars (a `Role::PaletteField` row has no side padding or frame), so its text stands about 5 px left of the rows' text at 100%; an inset field aligned to the row text was the alternative (assumed design default).
- No highlight of the matched letters and no mouse-wheel scroll of the eight rows in this iteration (listed in the log).

## Pending index

- home-still has no document on command palettes or fuzzy string matching for interfaces (`distill_search` "command palette search commands keyboard", "fuzzy matching approximate string matching ranking", 2026-11-01: ExposeHK and *Supporting Novice to Expert Transitions* only; an entity-resolution survey for the second). To index: Sublime Text's and VS Code's UX guidelines on the Command Palette, fzf's scoring notes (junegunn, `src/algo`), and Hick/Bailly's "Menu selection" literature already listed for menus.
