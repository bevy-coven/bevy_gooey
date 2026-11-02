# `docs/guide/` — the detail behind the README

The README is the page most people read, so it carries the pitch, the quick start and one bullet per
thing you can do. Everything that used to sit in it and needs more than a sentence lives here, one page
per subject. The README links each bullet to its page.

- [The widgets](widgets.md) — every widget, a meter, a tooltip, a dialog, and how a HUD lets the pointer
  through to the game under it.
- [Navigation](navigation.md) — what moves focus, what shows the ring, and what an open dialog holds.
- [Settings controls](controls.md) — `toggle`, `choice` and `slider`: one bundle each, Bevy's own widget
  at the root, and `ValueChange` once per change.
- [Player options](options.md) — `text_size`, `interface_size`, `pad_style` and `skin_preset`: the rows for the four
  settings the crate itself reads (the theme row is the high-contrast switch), with nothing to wire, and the sizes every widget is checked at.
- [Rebinding controls](bindings.md) — `binding(label, value)`: a row that waits for the player's next key,
  mouse button or pad button, what it hears and holds back while it waits, and `BindingSettings`.
- [Shortcuts](shortcuts.md) — `Shortcut`, `Modifiers` and `Platform`: a key chord in the platform's own words
  ("Ctrl+Shift+P", "⇧⌘P"), `just_pressed`, `model::shortcut_conflicts`, a `binding` row that captures a
  chord and a tooltip that shows the command's chord on a keycap.
- [Text input](text-input.md) — `text_input(label, value)`: a row with a field the player types in, the
  edit and its end in `ValueChange<String>`, `TextEditing` for a game's hotkeys and a pad's keyboard, and
  why a field never traps focus.
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
- [Select](select.md) — `select(label, options, selected)`: a choice up to eight options, a searchable list
  above, one `ValueChange<usize>` and "No option matches".
- [Sections](sections.md) — `section(title, open, children)`: a header that folds a group of widgets out of layout
  and focus, `ValueChange<bool>`, a state kept by `SectionId` across a respawn, and "Empty" for a section with nothing in it.
- [Inspector](inspector.md) — `inspector(InspectTarget)` (feature `reflect`): the rows of a reflected component or
  resource, one widget per field, doc comments as tooltips, ranges from attributes, edits written back through
  `ReflectMut` as `InspectorChange`.
- [Prompts](prompts.md) — `prompts()` and `prompt(label, Prompt::BACK)`: the Select and Back hints in the
  bottom-left corner, the keyboard's or the pad's by `InputDevice`, and what they do not draw.
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
- [Toasts](toasts.md) — one spawn, the reading-time maths, `ToastSettings`, and the cap on the stack.
- [Skins](skins.md) — `GooeySkin`, its fields, the two presets, and every rule `problems()` checks.
- [Screen size](scale.md) — `GooeyScalePlugin`, `UiScaleSettings`, and whole-device-pixel lines.
- [The pointer](pointer.md) — which parts of the interface take the pointer and which let it through.
- [Feedback](feedback.md) — `UiFeedback`, the message a game turns into sound.
- [What a host touches](host-api.md) — the settings resources, the components, `UiFeedback`, every
  system and the engine-free maths, in one table.
- [What this crate does not own](scope.md) — the boundary with the game, `bevy_speech_bubbles` and the
  host's audio, strings and fonts.

The design rules and the published sources behind them are in
[`docs/research/`](../research/gooey.md), which is a reading list, not a manual.
