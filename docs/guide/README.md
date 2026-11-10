# `docs/guide/` — the detail behind the README

The README is the page most people read, so it carries the pitch, the quick start and one bullet per
thing you can do. Everything that used to sit in it and needs more than a sentence lives here, one page
per subject. The README links each bullet to its page.

- [The widgets](widgets.md) — every widget, a meter, a tooltip, a dialog, and how a HUD lets the pointer
  through to the game under it.
- [Focus regions](focus-regions.md) — `FocusRegion`, F6 and Shift+F6 between a tool's parts, `viewport()` and a
  region that keeps the arrow keys for the host.
- [Drag and drop](drag-drop.md) — `DragPayload<T>`, `DropTarget<T>`, `Dropped`, `DropRefused` and `DroppedOnWorld`: a
  typed payload, a ghost, a mark that says yes or no before the pointer is let go, and Escape.
- [Accessibility](accessibility.md) — what a screen reader is told of every widget: role, name, value and state from the
  words on screen, `AccessibleLabel` for a bar with no label, and what is hidden.
- [Navigation](navigation.md) — what moves focus, what shows the ring, and what an open dialog holds.
- [Settings controls](controls.md) — `toggle`, `choice` and `slider`: one bundle each, Bevy's own widget
  at the root, and `ValueChange` once per change.
- [Player options](options.md) — `text_size`, `interface_size`, `pad_style`, `skin_preset` and `motion_row`: the rows for the
  settings the crate itself reads (the theme row is the high-contrast switch), with nothing to wire, and the sizes every widget is checked at.
- [Rebinding controls](bindings.md) — `binding(label, value)`: a row that waits for the player's next key,
  mouse button or pad button, what it hears and holds back while it waits, and `BindingSettings`.
- [Shortcuts](shortcuts.md) — `Shortcut`, `Modifiers` and `Platform`: a key chord in the platform's own words
  ("Ctrl+Shift+P", "⇧⌘P"), `just_pressed`, `model::shortcut_conflicts`, a `binding` row that captures a
  chord and a tooltip that shows the command's chord on a keycap.
- [Shortcut badges](shortcuts.md#show-every-chord-hold-a-modifier) — `ShortcutBadges` and `Shortcut` on a widget: hold Alt
  and every command's chord shows on its button, inside its frame, gone the frame the key is let go.
- [Text input](text-input.md) — `text_input(label, value)`: a row with a field the player types in, the
  edit and its end in `ValueChange<String>`, `TextEditing` for a game's hotkeys and a pad's keyboard, and
  why a field never traps focus.
- [Text area](text-input.md#a-text-area) — `text_area(label, value, rows)`: the text input several lines high, Enter
  for a line, Ctrl+Enter or Cmd+Enter to end the edit, the arrow keys the caret's.
- [Code editor](code-editor.md) — `code_editor(label, text)`: a gutter, a caret on a spring, selection by word and line,
  undo grouped by word and pause, a clipboard, IME, Tab indentation, `CodeEditorValue`, and 20,000 lines for the price of 20.
- [Numbers](numbers.md) — `number_field(label, value, step, range)` and `vec_field`: a number the player
  scrubs with a drag, types or steps with the keys and the D-pad, held to a range, `MixedValue` for a
  multi-selection, and the reason a typed expression is refused.
- [Colours](colors.md) — `color_field(label, colour)`: a swatch and a hex field the player types (`#ff8800`),
  and a picker the swatch opens (a saturation and value area, hue, brightness and alpha strips) that
  a pointer, the keys and a pad all drive, `MixedValue`, and the reason a typed word is refused.
- [Field problems](field-problems.md) — `FieldProblem::error("A name is needed")` on a text row: the words under
  it, a bar and (for an error) the field's frame in the kind's colour, the rows below making room, gone the
  frame it is removed.
- [Icons](icons.md) — `icon(IconSource)`: the host's own image or atlas cell, a step of the type scale square,
  tinted by the skin (`IconTone`), following its button's state, an empty box of the right size when missing.
- [Progress and spinner](progress.md) — `progress(fraction)` with `ProgressText` and `spinner()`: which to use
  by how long the work takes, "3 of 7: Measuring meshes, about 20 s left", and a ring on real time.
- [Toolbars](toolbars.md) — `toolbar`, `icon_button(icon, tooltip)`, `toggle_group(options, selected)` and
  `toolbar_separator()`: a tool's bar of glyph buttons and modes, and the last items going behind a **More**
  button when the bar is too narrow.
- [Status bar](status-bar.md) — `status_bar(left, right)`, `status_item(text)` with a `StatusKind` mark and a
  `StatusPriority`, `status_progress`: workspace state left, context right, the lowest priority hiding first.
- [Combobox](combobox.md) — `combobox(label, options)`: a field with a list under it that narrows as the player types, the
  lit option moved with Down, Ctrl+N and Ctrl+P, one `ValueChange<String>` with the option's id per choice.
- [Select](select.md) — `select(label, options, selected)`: a choice up to eight options, a searchable list
  above, one `ValueChange<usize>` and "No option matches".
- [Sections](sections.md) — `section(title, open, children)`: a header that folds a group of widgets out of layout
  and focus, `ValueChange<bool>`, a state kept by `SectionId` across a respawn, and "Empty" for a section with nothing in it.
- [Inspector](inspector.md) — `inspector(InspectTarget)` (feature `reflect`): the rows of a reflected component or
  resource, one widget per field, doc comments as tooltips, ranges from attributes, edits written back through
  `ReflectMut` as `InspectorChange`.
- [World anchors](world-anchor.md) — `WorldAnchor::above(enemy, 1.2)` on any root node: a health bar, a prompt or a beacon that
  follows a point in the world, hides or pins to the screen's edge off view (`AnchorPinned`), and is gone with its target.
- [Cooldowns](cooldowns.md) — `Cooldown` on any widget: a dimmed cover that sweeps clear clockwise with the seconds left on a
  chip, counted on game time, `CooldownReady` when it ends, the seconds in the accessible name, a hotbar on a `slot_grid`.
- [Hold buttons](hold-buttons.md) — `hold_button("Hold to delete", 1.2)`: a button that confirms only when held (key, pad or
  pointer), a bar under its label, one `HoldConfirmed`, and `HoldSettings` to shorten the hold or make it a tap.
- [Prompts](prompts.md) — `prompts()` and `prompt(label, Prompt::BACK)`: the Select and Back hints in the
  bottom-left corner, the keyboard's or the pad's by `InputDevice`, and what they do not draw.
- [Inventory](inventory.md) — `slot_grid(columns, items)`: a backpack or a chest of slots a game fills with
  items (`SlotItems`), one tab stop, the arrows and Home and End, a choice, and moving items by drag, keyboard or pad.
- [Cards](cards.md) — `card_grid(cards)`: cards of a picture, a name and quiet lines in a row that wraps to fill its width,
  one tab stop the arrows walk, a chosen card lit and said once, a dimmed card no one chooses.
- [Motion](motion.md) — a dialog's card and a popover's panel spring into place, the skin's motion palette,
  `MotionSettings::reduced()` is the player's "reduce motion" (`off()` no motion at all), `MotionTarget` moves your own panel, and `MotionTrace` tests it.
- [Presence](presence.md) — `Leave` fades a dialog, a menu or any node out and despawns it, `Stay` turns one round
  from where it is drawn, `Opacity` fades a whole subtree; `MotionSettings::off()` makes both instant.
- [Radial menu](radial.md) — `radial(items, selected)`: a weapon wheel chosen by the direction of a stick, the pointer or
  the D-pad, one `ValueChange<usize>` while pointing and a final one for the choice, and release-to-select.
- [Tabs](tabs.md) — `tabs(labels, selected, children![tab_page(..)])`: pages of widgets under a strip, LB
  and RB, and focus that never rests on a page that is not shown.
- [Lists](lists.md) — `list(rows, children![..])`: the row count, the rail, the follow, PageUp and
  PageDown, and what a list cannot do yet.
- [Split panes](splits.md) — `split(SplitAxis::Horizontal, first, second)`: two panes and a divider, `SplitFraction`
  and `SplitMin`, a drag, the arrows, Home, End, Enter, the D-pad and a double click, and what a small window does.
- [Docks](docks.md) — `dock(DockLayout)` and the `PanelRegistry`: a workbench of tab groups and splits kept as
  data, rearranged by dragging a tab or Ctrl+Shift and an arrow, `DockLayoutChanged` for saving, and what a layout
  with an unknown panel does.
- [Tables](tables.md) — `table(columns, rows)` over a `RowSource`: ten thousand rows in a dozen entities, a header
  that sorts (`SortBy`), columns the player sizes, a cursor and a selection by key, and rows that change under a scroll.
- [Trees](trees.md) — `tree(nodes)` over `TreeNode`s: ten thousand nodes in a dozen entities that open and close
  (a chevron, Right and Left, `*`), type-ahead, a selection by key that follows a node and an heir when it is removed.
- [Popovers](popovers.md) — `popover(anchor, Placement::Below)`: a menu or a context menu beside a widget or at
  the pointer, flipped to stay in the window, closed by Escape or a press elsewhere, modal or not.
- [Menus](menus.md) — `menu_bar(children![menu_title(..)])`: a bar of titles with menus of commands, checks, radio
  runs, separators, submenus, right-aligned chords and disabled items that say why; F10, the arrows, Escape and
  the pad.
- [Context menus](context-menus.md) — `context_menu(items)` and `ContextMenu(menu)` on any entity: a menu at a
  right-click or below the focus on Shift+F10, one menu for many things, `ContextTarget` says which.
- [The command palette](command-palette.md) — `command_palette()` over a `CommandList`: a field and a fuzzy-ranked
  list of every command with its chord and why it is off, recents first, Up and Down while focus stays in the
  field, one `RunCommand` on Enter or a click.
- [Toasts](toasts.md) — one spawn, the reading-time maths, `ToastSettings`, and the cap on the stack; `.action(..)`,
  `.sticky()`, merged repeats, the `ToastLog` and `notification_center()`.
- [Skins](skins.md) — `GooeySkin`, its fields, the presets, every rule `problems()` checks, and `tones`: a host's own
  named colours, `Role::Tone` and `tone_problems`.
- [Testing a gooey UI](testing.md) — `bevy_gooey::testing` (feature `testing`): `find_by_label`, `click`, `press`,
  `type_text` and `focused_label` over a headless `App`, by what the player reads.
- [Screen size](scale.md) — `GooeyScalePlugin`, `UiScaleSettings`, and whole-device-pixel lines.
- [The pointer](pointer.md) — which parts of the interface take the pointer and which let it through.
- [Feedback](feedback.md) — `UiFeedback`, the message a game turns into sound.
- [What a host touches](host-api.md) — the settings resources, the components, `UiFeedback`, every
  system and the engine-free maths, in one table.
- [What this crate does not own](scope.md) — the boundary with the game, `bevy_speech_bubbles` and the
  host's audio, strings and fonts.

The design rules and the published sources behind them are in
[`docs/research/`](../research/gooey.md), which is a reading list, not a manual.
