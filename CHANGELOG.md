# Changelog

## [Unreleased]

### Added

- `slot_grid(columns, items)`: an inventory of square slots (a backpack, a hotbar, a chest). The contents are data, `SlotItems(Vec<Option<Slot>>)` on the grid (`Slot::new("Potion").count(3).icon(..).tooltip(..)`), and the grid follows them in the same frame, touching only the slots that changed. Each slot is Bevy's `Button` with the focus ring and a tooltip (an icon slot with none shows its name); the grid is one tab stop (`SlotCursor`, a roving `TabIndex`), the arrows and the D-pad walk it, Home and End go to the ends of a row and with Ctrl of the grid, and Enter, Space, South or a click send one final `ValueChange<usize>` for the slot, empty or not (focus moving onto a slot sends a non-final one). Moving an item is the crate's drag and drop: a filled slot is a `DragPayload<SlotRef>`, every slot a `DropTarget<SlotRef>`, so the pointer, Alt+M and the arrows and a pad all end in one `SlotMoved` on the grid it landed in (across grids too); `SlotMoved::apply` swaps the two slots. `model::slot_edge`. New roles `Role::SlotGrid`, `Slot`, `SlotEmpty`, `SlotName`, `SlotCount` and `SlotIcon`. Guide: `docs/guide/inventory.md`; `examples/inventory`.

### Changed

- A node that loses its `DragPayload` is no longer a drag source (its `DragSource` goes with it), so pressing it lifts nothing.

## [0.26.0] - 2026-11-02

The first publish since 0.24.0 (crates.io refused 0.24.1 to 0.25.1 with a 429): 0.26.0 carries everything in 0.24.1 to 0.25.1 below.

### Added

- `radial(items, selected)`: a weapon wheel. The items stand on a ring round a disc (item 0 at the top, clockwise); a stick (either one, of any pad) pushed past half its travel points at the item it faces, the pointer over a wedge does the same and a click chooses, the arrows and the D-pad step round (wrapping), Home and End jump, and South, Enter or Space send one final `ValueChange<usize>` (`is_final: true`) even when the highlight did not move; a move sends one with `is_final: false`. `RadialValue` and `RadialCount` are its state, `radial_by_pad` and `update_radials` its systems, `Role::{Radial, RadialTrack, RadialSlot, RadialText, RadialCentre}` its look (a quiet circle for the items to stand on, the disc capped at 92% of the window's shorter side so it is whole at 200% text, the lit item is the selected tab's look, its name in the heading size in the middle), a spin button valued with that name for a screen reader, and `navigate_by_input` leaves a focused radial all four arrows and the D-pad. `model::{radial_pick, radial_place, radial_step, RADIAL_STICK_PUSH, RADIAL_DEAD_ZONE}`. `examples/radial`, `docs/guide/radial.md`.

## [0.25.1] - 2026-11-02

Also the first publish since 0.24.0 if 0.25.0 is still unpublished (crates.io refused it with a 429): 0.25.1 carries everything in 0.24.1 to 0.25.0 below.

### Fixed

- `testing::click` reaches a UI drawn into an image in an app with no window (G-31, #35): the click goes to the render target of the camera that draws the widget (`ComputedUiTargetCamera`, then that camera's `RenderTarget`, a window's or an image's, divided by the target's scale factor), each of two cameras gets the clicks of what it draws, and with no primary window the click also gives focus to the widget under the pointer, which Bevy does only while a window exists. `press` and `type_text` no longer need a primary window. A widget no camera draws is a panic that names it.

## [0.25.0] - 2026-11-02

The first publish since 0.24.0: it also carries 0.24.1 (the accessibility tree) and 0.24.2 (toast buttons), which crates.io refused with a 429.

### Added

- Drag and drop without a pointer (WCAG 2.2, 2.5.7): a focused `DragPayload` source is lifted by `DragSettings::pick_up` (Alt+M; a pad's West), the arrows or the D-pad mark the nearest target that takes the payload in that direction (`target_toward`), PageUp, PageDown, LB and RB step through the targets in reading order (`target_step`), the pick-up key again (a pad's South too) sends the one `Dropped<T>` on the marked target and Escape or East puts it down. The ghost, the mark, `Dragging` and the events are the pointer drag's; `DragState::carried_by_key()` says how it was lifted. `DragSettings` is a serde resource, `model::Toward` the direction.

### Changed

- **Breaking:** `not_capturing` takes a second system parameter (it is also false while a payload is carried from the keyboard or a pad, so the arrow that steers it never also moves focus); a host that registers it as a run condition is unchanged, one that calls it as a plain function needs the parameters.

## [0.24.2] - 2026-11-02

### Fixed

- A toast's buttons are never squeezed or touching (G-30, #34): `Role::ToastAction` is `flex_shrink: 0`, half a gap of margin on each side (two stand a gap apart) and `align_self: Center`, so a message that wraps over several lines no longer takes the room of "Show" and "Dismiss", which read "ShowDismiss" with no padding. A toast with no button is laid out as before.

## [0.24.1] - 2026-11-02

### Added

- Accessibility tree (G-28, #32): every widget is a node of Bevy's accessibility tree with a role, a name, a value and its state, built from the words it draws, while a screen reader is connected (`bevy::a11y::AccessibilityRequested`; otherwise the new system returns at once). A toggle is a CheckBox with its label and its checked state, a slider a Slider with its label, "70%" and its number, a choice a SpinButton with its selected option, a text field a TextInput named by its row and holding its text, a dialog a modal Dialog named by its heading, a menu a Menu of MenuItem, MenuItemCheckBox and MenuItemRadio rows (open or not, checked), a table, a tab strip, a toast, a meter, a progress bar and a splitter their ARIA roles; a page not shown, a closed menu and an icon are hidden, a disabled control is `disabled`. Bevy's own `AccessibleLabel` on any node replaces the crate's name. `access::spec` is an exhaustive `match` over `Role`, so a new role cannot ship unclassified. `GOOEY_A11Y=1` on the gallery prints the tree. `docs/research/accessibility.md`, `docs/guide/accessibility.md`, `tests/access.rs` (16 tests). New dependency: `accesskit` 0.24 (the version Bevy's nodes are made of).

## [0.24.0] - 2026-11-02

### Added

- Named tones (G-29, #33): `GooeySkin::tones: BTreeMap<String, Color>` (serde `default`, empty in every preset) carries a host's own semantic colours, `GooeySkin::tone(name) -> Option<Color>` reads one (an unknown name is `None`, never a stand-in), `Role::Tone(&'static str)` draws a node's fill with one (restyled in the frame the skin changes; a name the skin lacks draws no fill) and `GooeySkin::tone_problems(&[(label, ink, ground, needed)])` checks the pairs a host's chrome draws with the crate's own arithmetic (a tone or a skin colour by name; one `SkinProblem::LowContrast` named `label` per pair under its ratio, one `SkinProblem::UnknownTone` for a name that is neither). A theme step keeps the host's tones. `docs/research/tones.md`, a section of `docs/guide/skins.md`, `tests/tones.rs` (8 tests).
- Headless test helpers (G-27, #31), cargo feature `testing`: `bevy_gooey::testing::{find_by_label, click, press, type_text, focused, focused_label, label_of, FindError}` drive a headless `App` by what the player reads, through real `PointerInput`, `MouseButtonInput` and `KeyboardInput` (two widgets with one label are a `FindError::Ambiguous` naming both; a hidden one is not found; a click lands at any window scale and a covered node is not clicked). `docs/research/testing.md`, `docs/guide/testing.md`, `tests/testing.rs` (9 tests).
- Shortcut overlay (G-26, #30): `ShortcutBadges { hold, delay_seconds }` (a resource; Alt and 0.3 s by default) shows every visible, enabled widget's chord on a badge while exactly that modifier is held, and removes them the frame it is let go. A widget's chord is a `Shortcut` component (`Shortcut` is now a `Component`) or its `Tooltip`'s. The badge (`Role::ShortcutBadge`, marker `ShortcutBadge`, system `update_shortcut_badges`) sits inside the widget's frame, so the focus ring is never covered, in the inverse of a panel's colours (4.5:1 in every look). `examples/badges`, `docs/media/shortcut-badges.gif`, `docs/research/shortcut-overlay.md`, a section of `docs/guide/shortcuts.md`, `tests/badges.rs` (8 tests).

### Changed

- **Breaking:** `GooeySkin` gains the `tones` field (a struct literal without `..` needs it), `Role` gains `Tone` and `ShortcutBadge`, `SkinProblem` gains `UnknownTone`, and `Shortcut` derives `Component`.
- The README's shortcuts GIF gives its place to `shortcut-badges.gif` (it stays in `docs/guide/shortcuts.md`), keeping the README at 250 lines.

## [0.23.0] - 2026-11-02

### Added

- Focus regions (G-23, #27): `FocusRegion { arrows: ArrowPolicy }` (`FocusRegion::host()` and `::navigate()`) on any node. **F6 and Shift+F6** (`cycle_regions`, Navigate) move focus to the first widget of the next or previous region in UI-tree order, wrapping, skipping a region with nothing focusable; with `ArrowPolicy::Host` the arrow keys move no gooey focus and show no ring while focus is in the region (the D-pad, Tab and F6 still leave: no keyboard trap), and stay readable in `ButtonInput<KeyCode>`. An open dialog holds focus over every region. `viewport()` is a framed host region and a tab stop (`Role::Viewport`: the `outline` frame, the ring while focused). `examples/workspace`, `docs/research/focus-regions.md`, `docs/guide/focus-regions.md`, `docs/media/focus-regions.gif`, `tests/region.rs` (13 tests).
- Drag and drop with payloads (G-22, #26): `DragPayload<T>` on a source (a press that moves 6 px lifts it), `DropTarget<T>` / `DropTarget::accepting(fn(&T) -> bool)` on targets (a node may take several types), `Dropped<T> { target, source, payload, pointer }` and `DropRefused<T>` (entity events on the target), `DroppedOnWorld<T>` (global, over nothing the interface draws, with the pointer position), `DragLabel`, `Dragging` (on the source while carried), `DragState`, `is_dragging`, `cancel_drags` and `DRAG_LIFT_PX`. A ghost with the source's words follows the pointer and the target under it gets a mark, the ring's colour when it takes the payload and `danger` (ghost frame too) when it does not; Escape or East puts it down and is no `Cancel`. `Role::{DragGhost, DragGhostRefused, DropMark, DropMarkRefused}`; `docs/research/drag-drop.md`, `docs/guide/drag-drop.md`, `docs/media/drag-drop.gif`, `tests/drag.rs` (13 tests).
- Multiline text area (G-24, #28): `text_area(label, value, rows)` is a text row with the label over a field `rows` lines high that wraps and scrolls: Enter adds a line, **Ctrl+Enter or Cmd+Enter ends the edit** (one final `ValueChange<String>`, the field kept), Up and Down are the caret's (the D-pad still leaves, Tab and Escape too), `TextLimit` is Bevy's rule (an edit that would pass it is ignored). `Role::TextArea`; `docs/research/text-area.md`, a section of `docs/guide/text-input.md`, `docs/media/text-area.gif`, `tests/text_area.rs` (12 tests).
- `Role::Viewport` and `viewport()`; `examples/workspace` shows the three together (`GOOEY_KEYS` steps `f6`, `sf6`, `center`, `type:<text>`, `drag:<from>><to>`).

### Changed

- **Breaking:** the dock's `DropTarget` (where a dragged tab lands) is now `DockDropTarget`, so `DropTarget<T>` can be the name of drag and drop's target, as the request asked. `Role` gains the variants above: a `match` with no wildcard needs them.
- The README's tree GIF gives its place to `drag-drop.gif` (it stays in `docs/guide/trees.md`), keeping the README at 250 lines.

## [0.22.0] - 2026-11-01

### Added

- Colour fields (G-13 slice two, #17): `color_field(label, Srgba)` is a text row with a swatch. The field shows the colour as hex and takes `#ff8800`, `f80` or `#ff880080` (Enter or leaving applies it as one final `ValueChange<Srgba>`, Escape puts the old colour back, anything else is refused with its reason as a `ColorRefused`); the swatch is a button (Enter, Space, South or a click) that opens a modal popover picker and closes it again. The picker is a saturation and value area (a press and drag, or the arrows, Shift ten times as far; the D-pad's Left and Right step the saturation) over Hue, Brightness and Alpha strips, which are `Slider`s drawn with `BackgroundGradient`s (no shader, so it lays out in a headless window). The colour applies as it is picked. The picker keeps the hue, saturation and value it is on, so dragging to black and back or typing a grey keeps the hue. `MixedValue` works as on a number field. State is `ColorValue`; `ColorPicker` is the open picker's.
- `model::{parse_hex, hex_text, hsv_to_rgb, rgb_to_hsv, area_values, HexError}`: the engine-free maths, unit-tested and doctested.
- `update_colors` (Style), `ColorSwatch`, `SwatchHalf`, `ColorArea`, `AreaMarker`, `ColorStrip`, `ColorRail` (markers on what the widget spawns).
- `examples/colors`, `docs/research/color-field.md`, `docs/guide/colors.md`, `docs/media/colors.gif` and `tests/color.rs` (24 tests).
- Icons (G-21, #25): `icon(IconSource)`, with `IconSource::{Image, Atlas}`, `Icon`, `IconTone::{Text, Muted, Accent, Danger}` (the tint is a colour of the skin; `Text` follows the button it is in like the button's text), `IconSize` (a `type_scale` step, default the body's line) and `IconMissing` (a failed image draws an empty box of the right size and warns once). `update_icons` (Style). `Role::Button` gets a half-gap `column_gap`, so an icon beside its label has air.
- Field validation messages (G-25, #29): `FieldProblem { text, kind }` (`FieldProblem::error`, `::warning`; `FieldProblemKind`) on a text row draws its words under it, a bar in the kind's colour beside them and, for an error, the field's frame in `danger`; removing it removes the words in the same frame and the rows below make room. `update_field_problems` (Style).
- Progress and spinner (G-17, #21): `progress(fraction)` with `Progress`, `ProgressText { step, eta_seconds }` ("3 of 7: Measuring meshes, about 20 s left") and `ProgressClamped`, and `spinner()` with `Spinner`, a ring that turns on real time. `update_progress` and `update_spinners` (Style). `model::{eta_words, progress_text, spinner_angle, SPINNER_PERIOD_SECS}`.
- `Role::{Icon, FieldMessage, FieldMark, Progress, ProgressLine, Spinner}` (a minor bump: `Role` is exhaustive).
- `examples/forms`, `docs/research/{icons, field-problem, progress}.md`, `docs/guide/{icons, field-problems, progress}.md`, `docs/media/forms.gif`, and `tests/{icon, field_problem, progress}.rs` (21 tests).
- Toolbars (G-20, #24): `toolbar(items)`, `icon_button(IconSource, Tooltip)`, `toggle_group(options, selected)` with `GroupOption::{icon, text}`, `ToggleGroupValue` and `ToggleGroupCount` (radio behaviour: one `ValueChange<usize>` on the group, the option on lit in the accent, none for the one already on) and `toolbar_separator()`. When the items do not fit the bar's width the last ones hide (`Overflowed`) and a **More** button lists them; `fit_bars` (Style) does it for toolbars and status bars, `model::bar_fit` is the rule. `update_toggle_groups` (Style).
- Status bar (G-19, #23): `status_bar(left, right)`, `status_item(text)` (Bevy's `Button`: `Activate`, a tooltip), `StatusKind::{Info, Warning, Error}` (a mark at the item's start, the words never change colour), `StatusPriority(u8)` (the lowest hides first, the right group's before the left's) and `status_progress(text, fraction)`.
- Searchable select (G-16, #20): `select(label, options, selected)` is a `choice` with eight options or fewer (`SELECT_FILTER_ABOVE`) and opens a filtering list above that: the command palette's card over the select's own options (type to narrow it, "No option matches", Up and Down, Enter or a click chooses, Escape leaves the value), one `ValueChange<usize>` with the option's index in the list given. `Select`.
- `Role::{Toolbar, ToolbarSeparator, ToggleGroup, GroupOption, StatusBar, StatusGroup, StatusItem, StatusMark}`; `CommandPalette` can hold a select's own list (the palette's own list and history are untouched).
- `examples/tools`, `docs/research/{toolbar, status-bar, select}.md`, `docs/guide/{toolbars, status-bar, select}.md`, `docs/media/tools.gif`, and `tests/{toolbar, status_bar, select}.rs`.
- Collapsible sections (G-15, #19): `section(title, open, children)` is a header (a button's look: target high, ring, hover, press; a chevron right when closed and down when open) over a body of any bundle, so a section inside one is a nested section. Enter, Space, South and a click toggle it and send one `ValueChange<bool>` on the section; `SectionOpen(bool)` is the state (a game's write moves it with no event); a closed body is `Display::None` and `Visibility::Hidden`, so layout and focus skip it, and focus inside a body that closes goes to the header; an open section with nothing in it says "Empty". `SectionId` and the `SectionStates` resource (serde with the `serde` feature) keep the state across a despawn and a respawn. `update_sections` (Style), `SectionHeader`.
- `Role::{Section, SectionBody, SectionNote}`; the tree's chevron boxes carry a margin of half their size either side (the tree's twisty centres them, so no tree row moves).
- `examples/sections`, `docs/research/sections.md`, `docs/guide/sections.md`, `docs/media/sections.gif`, and `tests/section.rs` (11 tests).
- Reflect inspector (G-14, #18, feature `reflect`): `inspector(InspectTarget)` builds the rows of a reflected component (`InspectTarget::component::<T>(entity)`) or resource (`::resource::<T>()`): a `toggle` for a `bool`, a `choice` for a unit enum, a `number_field` for a number (integers stay integers), a `vec_field` for `Vec2`/`Vec3`/`Vec4`, a `color_field` for `Srgba`/`Color`, a `text_input` for a `String`, a `section` for a nested struct and a read-only line for anything else. A field's doc comment is its tooltip, `#[reflect(@RangeInclusive::<f32>::new(..))]` its range and `#[reflect(@InspectStep(..))]` (`InspectStep`) its step. An edit is written through `ReflectMut` and sent as one `InspectorChange { entity, path }`; a value the game writes moves the row with no event. `Inspector`, `InspectRow`, `build_inspectors`, `sync_inspectors`; the `reflect` feature turns on `bevy/reflect_documentation`.
- `Role::Inspector`; `examples/inspector` (`required-features = ["reflect"]`), `docs/research/inspector.md`, `docs/guide/inspector.md`, `docs/media/inspector.gif`, and `tests/inspector.rs` (12 tests).
- Toast actions, sticky errors and a notification log (G-18, #22): `toast(..)` and `toast_alert(..)` return a `ToastBundle` with `.action(label, id)` (a button that sends one `ToastAction { toast, id }` and dismisses the toast; the toast's only interactive part, a tab stop; a toast with a button stands at least `ToastSettings::action_seconds`, 6) and `.sticky()` (stands until dismissed, with a Dismiss button, outside `max_visible`). Identical toasts within `ToastSettings::merge_seconds` (2) merge into one that reads "x3" (`Toast::count`). `ToastLog` keeps the last 200 toasts spawned (`ToastLogEntry`); `notification_center()` lists it newest first as a scrolling list of buttons (`NotificationCenter`, `NotificationRow`, `update_notification_centers`, `model::clock_text`). `ToastButton`.
- `Role::{ToastAction, ToastActionText, NotificationCenter}`; `ToastSettings` gains `action_seconds` and `merge_seconds` (a struct literal without `..Default::default()` needs them); a toast's body is now built when it joins the stack, in the same flush.
- `examples/notifications`, `docs/research/toast-actions.md`, the guide `docs/guide/toasts.md`, `docs/media/notifications.gif`, and `tests/toast_actions.rs` (15 tests).

### Changed

- `Role` gains `ColorSwatch`, `ColorSwatchHalf`, `ColorArea`, `ColorAreaMarker`, `ColorAreaMarkerInner`, `ColorStripBox` and `ColorRail`: a `match` on it that has no wildcard needs them. `navigate_by_input` leaves the picker area's four arrow keys (not the D-pad's Up and Down) to it, `gamepad_adjust` steps its saturation. The README's numbers GIF moved to `docs/guide/numbers.md` (where it already was) to make room.
- The README's colours GIF gives its place to `forms.gif` (the colours GIF is in `docs/guide/colors.md`), keeping the README at 250 lines; `forms.gif` in turn gives it to `tools.gif` (it stays in `docs/guide/progress.md`).
- A `choice` row that is a `select` with a list does not cycle on Enter, Space, South or a click: it opens the list (Left, Right, Home and End still step it).

## [0.21.0] - 2026-11-01

### Added

- Number fields (G-13 slice one, #17): `number_field(label, value, step, range)` is a text row whose number the player scrubs (press on the label or the row's padding and drag: 50 px at a step of 0.1 is 5.0, Shift a tenth of that rate, Ctrl ten times it), types (Enter or leaving applies it, held to the range; Escape puts the old number back; anything else, an expression such as `1.5*2` included, is refused with its reason as a `NumberRefused`) and steps (Up and Down, Shift ten steps; the D-pad's Left and Right), with the number selected when the field gets focus. `vec_field(label, [x, y, z], step, range)` is a row of them with X, Y, Z and W as scrub handles. State is `NumberValue`, `NumberStep`, `NumberRange`, `VecValue`; events are `ValueChange<f32>` (`is_final` false while a drag rolls) and `VecChange { index, value, values, is_final }`; `MixedValue` and `VecMixed` show `MIXED_HINT` (`--`) for a multi-selection that differs, and any change the player makes is the one value. A scrub takes focus off the field, so there is no caret while the number rolls. `color_field` and its picker are the next slice.
- `model::{parse_number, format_number, number_decimals, scrub_value, step_value, NumberError}`: the engine-free maths, unit-tested and doctested.
- `update_numbers` and `update_vecs` (Style), `NumberInput` and `VecPart` (markers on what the widgets spawn).
- `examples/numbers`, `docs/research/number-field.md`, `docs/guide/numbers.md`, `docs/media/numbers.gif` and `tests/number.rs` (32 tests).

### Changed

- `Role` gains `NumberPart` and `NumberParts`: a `match` on it that has no wildcard needs them. `navigate_by_input` leaves a number field's Up and Down keys and its D-pad's Left and Right to it (`KeepsSideways`); `gamepad_adjust` steps it. The README's Back GIF (`cancel.gif`) moved to `docs/guide/navigation.md` to make room.

### Fixed

- A text field that lost focus with its text selected drew that text in the accent's dark ink on no highlight, so it vanished (Bevy applies `TextCursorStyle::selected_text_color` with or without focus). `apply_skin` now sets it only on the field that has focus.

## [0.20.1] - 2026-11-01

### Changed

- The integration tests are one executable, `all` (`autotests = false`, `tests/all.rs`; every other file in `tests/` is a module of it), so Bevy is linked once and not 34 times (650 to 790 MB each). The 647 tests are unchanged (the sorted names match, plus `every_test_file_is_a_module`, which fails when a `tests/*.rs` file is not declared). Nothing a consumer sees changes.

## [0.20.0] - 2026-11-01

### Added

- Trees (G-11, #15): `tree(nodes)` is nodes that open and close, virtualized like the table it is built on (10,110 nodes spawn at most visible + 2 row entities). A `TreeNode::new(key, label)` has `.expanded()`, `.child(..)` and `.children(..)`; its `key` is its identity, so the selection, the cursor and the player's place stay with a node as others come and go. The rows are the nodes shown (every parent open), each indented a level (`model::tree_indent`: a text size and a half, never under 24 px) with a chevron in front of a node that has children. `Tree` is the table's `RowSource`: each node keeps its own open state, so closing a node loses nothing below it, and a game changes it through `table.source_mut::<Tree>()` (`insert`, `remove`, `set_nodes`, `set_open`, `is_open`, `open_keys`). The player opens and closes with a click on the chevron, Right and Left (the D-pad too) and `*` (every sibling), and the game hears `TreeToggled { entity, key, open }`; Right on an open node goes to its first child, Left on a closed node or an end node to its parent; typing goes to the next shown node starting with it (`model::type_ahead`, a one second window, the same letter again goes on). A removed node's selected state and cursor move to its heir, the next sibling, else the one before, else the parent (`update_trees`). Selection, cursor, `RowActivated` and `TableRows` are the table's.
- `model::{tree_indent, tree_ends, tree_visible, tree_parent, tree_heir, type_ahead, TYPE_AHEAD_SECS}`: the engine-free maths, doctested.
- `examples/tree`, `docs/research/tree.md`, `docs/guide/trees.md`, `docs/media/tree.gif` and `tests/tree.rs` (25 tests).

### Changed

- `Role` gains three variants (`TreeTwisty`, `TreeClosed`, `TreeOpen`): a `match` on it that has no wildcard needs them. `navigate_by_input` leaves Left and Right to a focused tree. `update_tables` keeps a tree's selected nodes that are hidden by a closed parent (a table's selected row that is gone still leaves the selection).

## [0.19.0] - 2026-11-01

### Added

- Tables (G-12, #16): `table(columns, rows)` is a header over a scrolling body of rows, virtualized: only the rows in view and one more are entities (at most visible + 2 for ten thousand rows), the rest are the room above and below them, because every row is one height (`GooeySkin::table_pitch`). `RowSource` is the one trait a game implements (`len`, `cell`, and defaults for `key` and `compare`; `Vec<Vec<String>>` is one); `Column { title, width (body text sizes), sortable }`. A click, Enter or Space on a sortable header sorts (`TableSort`, ascending then turned round, a chevron beside the title) and sends one `SortBy`; a column's right edge is dragged, or sized with Ctrl and Left or Right on the focused header, 16 px a press (`TableWidths`). Rows change through `Table::source_mut` or `set_source` and the row at the top of the view stays where it was (by `key`). Selection is by key (`SelectMode` None, Single, Multi; `TableSelection`; Ctrl toggles and Shift ranges), the cursor row (`TableCursor`) is ringed while the table has visible focus, and `ValueChange<TableSelection>`, `RowActivated` (Enter, South, a double click) and `SortBy` are the player's. Keys: Up, Down, Page Up, Page Down, Home, End (the first and last row of the data), Space, Ctrl+A, the D-pad (repeating), the triggers and South; Up from the first row and Down from the last move focus on.
- `model::{natural_cmp, table_window, table_step, TableStep, table_anchor, column_resize, TABLE_LEAST_COLUMN}`: the engine-free maths, unit-tested and doctested.
- `examples/table`, `docs/research/table.md`, `docs/guide/tables.md`, `docs/media/table.gif` and `tests/table.rs` (24 tests).

### Changed

- `Role` gains eleven variants (`Table`, `TableHeader`, `TableHeaderCell`, `TableSortUp`, `TableSortDown`, `TableEdge`, `TableView`, `TableRows`, `TableRow`, `TableCell`, `TableCellText`): a `match` on it that has no wildcard needs them. A table's viewport carries a `ListView` (its `rows` is `TableRows`), so `update_lists` shows its rail and a game's query of `ListView`'s `ScrollPosition` reaches a table's scroll. `navigate_by_input` leaves Up and Down to a focused table that has a row that way and Ctrl and Left or Right to a focused table header.

## [0.18.0] - 2026-11-01

### Added

- Docks (G-10, #14): `dock(DockLayout)` is a workbench of tab groups and splits kept as data. `DockLayout` is `Split { axis, fraction, first, second }` or `Group { panels, selected }` (panel ids as `String`s; serde with the `serde` feature, RON round-trips, a file may name only `panels`), with plain-data edits (`panels`, `contains`, `prune`, `moved`). `PanelRegistry` (a resource) maps an id to a title and a function that fills the panel's page, spawned once and then moved, never rebuilt, when the layout changes; `Dock(DockLayout)` on the dock is the state (write one to load or reset it). An unknown or repeated id is left out, a group left empty collapses, and one `DockPanelSkipped` is sent for each. The player picks a tab up (a card with its title follows the pointer and the part of the dock the panel will take is lit before it is let go) and drops it on a group's tab strip (at that place), its middle (as a tab), an outer quarter (a group of its own on that side) or within 24 px of the dock's edge (a quarter of the whole dock); Escape puts it back. Ctrl+Shift and an arrow on a focused tab moves its panel to the nearest group that way or to the dock's edge (WCAG 2.5.7), focus following. Every change is one `DockLayoutChanged { entity, layout }`: a drop, a picked tab, a divider let go. Panes are at least `DOCK_PANE_MIN_PX` (96 px).
- `model::{DockZone, dock_zone, dock_edge_zone, dock_drop_rect, dock_tab_slot, dock_neighbour}` and the constants `DOCK_PANE_MIN_PX`, `DOCK_EDGE_FRACTION`, `DOCK_EDGE_SHARE`: the drop's engine-free maths.
- `examples/dock`, `docs/research/dock.md`, `docs/guide/docks.md`, `docs/media/dock.gif`, `tests/dock.rs` (19 tests), `tests/serde.rs` (a layout in RON) and unit tests in `src/dock_layout.rs` and `model`.

### Changed

- `Role` gains four variants (`Dock`, `DockBody`, `DockDrop`, `DockGhost`): a `match` on it that has no wildcard needs them. `navigate_by_input` leaves a focused dock tab's arrows to the dock while Ctrl and Shift are held. `model::dock_tab_slot` takes each tab's centre and top and bottom, so a strip of several rows has a place for every tab.
- A `Role::TabList`'s `row_gap` is the skin's gap like its `column_gap` (it only shows when a strip wraps, as a dock's does).

### Fixed

- The command palette's count of rows that fit a short window no longer flips by one as the lit row walks down the list: a row's height is the mean of the rows shown (layout rounds each to whole pixels by where it stands, so the first row alone read 19 where the mean read 18.4) and the last answer is kept while the room is within 1.5 px of changing it. Found when an unrelated resource shifted the timing of `tests/palette.rs`'s 300 px window, which then showed six rows where five were counted.

## [0.17.0] - 2026-11-01

### Added

- Split panes (G-09, #13): `split(SplitAxis, first, second)` is two panes with a divider between them, a flex row or column; the panes' content is any bundle each and a `panel()` fills its pane on both axes. `SplitFraction(f32)` (serde, reflected, 0.5 by default, the first pane's share of the room the divider leaves) is the state and `SplitMin { first, second }` (logical px) each pane's least size; both are required components of `Split`, so a game puts its own in the same tuple. A drag of `d` px changes the fraction by `d` over the room at any UI scale, and neither a drag, a key nor a window too small for both minimums takes a pane below its own (the second pane gives way first, then the first); the stored fraction is not rewritten by a small window. A player's change is one `ValueChange<f32>` on the split (`is_final` false while dragging), a value the game sets sends nothing. The divider is a focusable channel `MIN_TARGET_PX` wide on the list rail's colour with a grip (`text_muted`, 3:1 on the rail in every preset; `text` under the pointer or with focus, `accent` while held) and the focus ring, with a resize cursor; the arrows along its axis step it `SPLIT_KEY_STEP_PX` (16 px, held keys repeat), Home and End take the first pane to its smallest and largest, Enter, South and a double click collapse it to its minimum and restore it (WCAG 2.5.7's way that is not a drag), the D-pad steps it (`split_by_pad`, repeating like a slider's), and `InteractionDisabled` fixes the layout. `model::split_mins`, `split_clamp` and `split_drag` are the engine-free maths.
- `examples/split`, `docs/research/split.md`, `docs/guide/splits.md`, `docs/media/split.gif` and `tests/split.rs` (29 tests).

### Changed

- `Role` gains four variants (`Split`, `SplitPane`, `SplitDivider`, `SplitGrip`): a `match` on it that has no wildcard needs them. `navigate_by_input` leaves a focused divider the arrows along its axis, like a slider's Left and Right.

## [0.16.0] - 2026-11-01

### Added

- Command palette (G-07, #11): `command_palette()` spawns a modal field over the game's `CommandList(Vec<PaletteCommand>)` (`id`, `title`, `category`, `shortcut`, `enabled: Result<(), String>`; build one with `PaletteCommand::new(..).category(..).shortcut(..).disabled(reason)`). The player types ("opn prj" finds "File: Open Project"), Up, Down, PageUp, PageDown and the D-pad move the lit row while focus stays in the field, Enter, South or a click on an enabled row despawns the palette and sends one `RunCommand { id }`, Escape or a click outside closes it with focus given back. Recently run commands (`PaletteHistory`, the last ten ids, serde data) come first, then by score; a disabled command shows its reason where its chord would be and runs nothing; up to eight reused rows scroll under the lit one (fewer at 200% text or in a short window, so the card never leaves it: `CommandPalette::visible_rows()`), "n of m" says where, "No command matches" says there is none. New: `model::fuzzy_score(query, text) -> Option<u32>` (engine-free: every whitespace-separated term a case-insensitive subsequence, word starts and runs score, gaps and length cost), `CommandPalette` (`results()`, `selected()`, `selected_command(..)`, `visible_rows()`), `palette_by_input` (Navigate) and `update_palette` (Style, before `apply_skin`), the `Role`s `Palette`, `PaletteRow`, `PaletteCount` and `PaletteField`, and `PALETTE_ROWS`. Every colour and size is an existing token (the menu rows'), so `problems()` is unchanged.
- `examples/palette`, `docs/research/command-palette.md`, `docs/guide/command-palette.md`, `docs/media/command-palette.gif` and `tests/palette.rs` (19 tests).

### Changed

- `Role` gains four variants (`Palette`, `PaletteRow`, `PaletteCount`, `PaletteField`): a `match` on it that has no wildcard needs them. `text_input` is built by one private row builder, unchanged for its callers.

## [0.15.0] - 2026-11-01

### Added

- Context menus (G-06, #10): `context_menu(items)` spawns a menu of `menu_item`, `check_item`, `radio_item`, `submenu` and `separator` rows that belongs to things, and `ContextMenu(menu)` on any entity (a UI node, a sprite or a mesh with `Pickable`) names it. A secondary click that reaches the entity opens the menu at the pointer (flipping at the window's edge); Shift+F10 or the Menu key with focus on the entity or a widget under it opens the nearest `ContextMenu` below the focused widget (`context_by_key`, in `GooeySystems::Navigate`). A second click moves it; one context menu is open at a time; Escape, B or a press outside close it and focus goes back. While it is open the panel carries `ContextTarget(thing)` (an `On<Add<ContextTarget>>` observer runs before it is first drawn; `ContextTarget::of(item, &parents, &targets)` finds it from an item's `Activate`). One menu may serve many things.
- `examples/context`, `docs/research/context-menu.md`, `docs/guide/context-menus.md`, `docs/media/context-menu.gif` and `tests/context_menu.rs` (13 tests).

### Changed

- `anchor_pointer_popovers` gives a `Placement::AtPointer` popover over an anchor that is not a UI node (a picked sprite or mesh) a pointer point of its own at the root: Bevy never lays out a UI node under a world entity. A UI anchor is unchanged.

### Fixed

- `fit_menus` capped a menu at the room on the side it prefers, so a menu opened near the bottom of the window was cut to three rows and scrolled though it flips above its anchor with room to spare; it is capped at the larger side now.
- Shift+F10 no longer focuses the menu bar (`focus_menu_bar` is F10 with no Shift).

## [0.14.0] - 2026-11-01

### Added

- Menus (G-05, #9): `menu_bar(children![menu_title("File", children![..])])` and the rows `menu_item(label)` (`.with_shortcut(Shortcut)` draws the chord right-aligned in muted text), `check_item(label, checked)`, `radio_item(label, checked)`, `submenu(label, items)` and `separator()`. A menu is spawned once with its title and shown by opening it, as a modal popover under the title (right of the item, for a submenu), so the popover's placement, outside-press close, `Cancel` and focus in and back all apply; the items are live entities, so `observe(|_: On<Activate>| ..)` sits where it is written. F10, or Alt pressed and let go alone, puts focus on the bar; Left and Right walk the titles (wrapping) and, in a menu, open a submenu or move to the next or previous title's menu; Down, Enter, Space, South or a click open a menu with focus on its first item; Home and End go to the first and last item; Escape or B closes one level and gives focus to what opened it, then (from the bar) back to where F10 found it; a pointer on another title switches menus. A check item flips its own `Checked` and a radio item sets its own and clears the run of radio items beside it (to the nearest separator), each sending one `ValueChange<bool>`. `Disabled::new(reason)` makes an item unavailable (`InteractionDisabled`) with a tooltip of the reason; it stays focusable. An empty submenu says "Nothing here"; a menu taller than the window leaves room for is capped and scrolls, following focus.
- `MenuBar`, `MenuTitle`, `MenuPanel`, `MenuItem`, `MenuOpen`, `Disabled`, and the systems `focus_menu_bar`, `menu_by_input`, `switch_menus_on_hover`, `fit_menus` and `scroll_menus_to_focus` (all in `GooeySystems::Navigate`); `Role::{MenuBar, MenuTitle, MenuPanel, MenuItem, MenuMark, MenuCheck, MenuDot, MenuText, MenuShortcut, MenuChevron, MenuSeparator, MenuNote}`.
- `examples/menus`, `docs/research/menu.md`, `docs/guide/menus.md`, `docs/media/menus.gif` and `tests/menu.rs` (16 tests, every preset's contrast and 1x to 2x text included).

### Changed

- `Role` has twelve more variants, so a `match` on it needs arms.
- `navigate_by_input` leaves Left and Right of a menu title or item, and Down of a title, to `menu_by_input`; `FocusScopes` has two more fields.
- `popover::close` hides a menu's panel (a `MenuPanel`) instead of despawning it; every other popover is despawned as before.

## [0.13.0] - 2026-11-01

### Added

- Shortcuts (G-08, #12): `Shortcut { modifiers, key }`, `Modifiers { ctrl, shift, alt, meta }` (the physical modifiers, either side; combine with `|`; `Modifiers::PRIMARY` and `Shortcut::primary(key)` are Command on a mac and Ctrl elsewhere) and `Platform` (`current()`, `Mac`, `Other`). `Shortcut::label()` is the platform's spelling, "Ctrl+Shift+P" in Microsoft's order or "⌃⇧P" in Apple's with glyphs for Return, Delete, Escape, Tab and the arrows (`label_on(Platform)` for another platform); `Shortcut::just_pressed(&ButtonInput<KeyCode>)` is the one rule for a chord that went down this frame with exactly its modifiers; `Shortcut::is_valid()` says the key is not a modifier. `Eq + Hash`, reflected, serde under the `serde` feature.
- `model::shortcut_conflicts(&[(&str, S)])`, generic over any `PartialEq` chord: every pair of commands on one chord, in list order. `model::shortcut_label` and `model::is_modifier_key`, engine-free, re-exported at the crate root.
- `Binding::Chord(Shortcut)`: a `binding` row spawned with a chord captures one. It waits through a bare modifier press, binds the first non-modifier key with the modifiers held at that moment (a bare key is a chord with none), and sends one `ValueChange<Binding>`; Escape exits and is never bound; a mouse button or a pad button exits. A key row is unchanged: a modifier alone still binds there. `BindingSettings::chord_prompt` ("Press a shortcut, {exit} exits") is what a waiting chord row says; a `Prompt` names a chord on its keycap too.
- `Tooltip::new(text)` and `.with_shortcut(Shortcut)`: the chord is drawn on the prompts' keycap to the right of the line, spawned and removed as `Tooltip::shortcut` changes. The box is never narrower than the cap and six text sizes of words beside it, so at 200% text a long chord makes the tooltip take another side of its widget before it leaves the window.
- `examples/shortcuts` (a keymap screen: three chord rows, a conflict line, commands that run on their chords, a Save button whose tooltip follows its row), `docs/research/shortcut.md`, `docs/guide/shortcuts.md`, `docs/media/shortcuts.gif` and `tests/shortcut.rs` (19 tests).

### Changed

- `Tooltip(String)` is now `Tooltip { text, shortcut }`: build one with `Tooltip::new("…")` (a clean cutover; `Tooltip.0` is `Tooltip.text`).
- `show_tooltips` and `update_tooltip_text` take one more system parameter each (the keycap lookup; `Commands`), so a host that registers them by hand adds nothing but may need its own query types to match; the crate's `GooeyStylePlugin` is unchanged.
- `Binding` has one more variant, `Chord`, and `BindingSettings` one more field, `chord_prompt` (a skin or settings file without it reads the default), so a `match` on `Binding` and a struct literal of `BindingSettings` that names every field need the new arm and field.

## [0.12.0] - 2026-11-01

### Added

- `popover(anchor, placement)` and `popover_modal(anchor, placement)` (G-04, #8): a panel that opens beside a widget or at the pointer and is kept inside the window. `Placement { Below, Above, Right, Left, AtPointer }` sets the preferred side; Bevy's own `Popover` takes the candidates (the side aligned to the anchor's start, its end and centred, then the opposite side the same three ways) and every frame picks the first that fits, so a panel flips and slides at the window's edge and is placed again when the window is resized. It is a child of its anchor (despawning the anchor closes it), laid out on the window (`FixedNode`), at most the window's width less two gaps wide, and drawn as `Role::Popover`: an opaque `solid_surface` card framed in `outline`, padded like a panel, with a shadow, above a `dialog` and under a `Tooltip`.
- `PopoverClosed { entity, anchor, reason }` and `CloseReason::{Outside, Cancel}`: a pointer press outside the panel (a press on the anchor of a side placement is not outside, so the anchor's own `Activate` can toggle it; the press that opened it never is), or Escape and the pad's B (one `Cancel`, stopped at the popover), send one event and the crate despawns the popover. A press on the panel never presses or activates its anchor. `popover_modal` is a `Dialog` and a modal `TabGroup` without the scrim: focus moves in, Tab and the arrows stay inside, and focus goes back to the anchor on closing; `popover` leaves focus where it is and Escape from its anchor closes it.
- `PopoverPanel { anchor, placement, modal }` (query it for the popover open on an anchor), `PopoverPoint` (the zero-size node an `AtPointer` popover hangs from) and the systems `close_popovers_on_press`, `anchor_pointer_popovers` and `drop_pointer_anchors`, in `GooeySystems::Navigate`.
- `examples/popover` (a menu bar, an Edit menu at the window's edge, a context menu at the pointer), `docs/research/popover.md`, `docs/guide/popovers.md`, `docs/media/popover.gif` and `tests/popover.rs` (19 tests).

### Changed

- `Role` has one more variant, `Popover`, so a `match` on it needs an arm.

## [0.11.0] - 2026-11-01

### Added

- Design tokens in the skin (G-03): `GooeySkin::spacing` (six steps), `radii` (three), `type_scale` (five) and `surfaces`, a ladder of flat fills, lowest first (five in every preset), read with `space(step)`, `corner(step)`, `type_size(step)` (times `text_scale`) and `surface_at(level)`; a step past the end is the last. Each preset's own `gap`, `padding`, `radius`, `text_size` and `heading_size` are on its scales.
- `Role::Surface(u8)` and the widget `surface(level)`: a column filled with one step of the ladder and nothing else.
- `SkinProblem::ElevationStep { lower, upper, delta_l }` when two adjacent surfaces differ by less than `ELEVATION_STEP`, ΔL\* 2.0 (the method of bevy_grimoire's `the_ladder_is_a_ladder`), and `SkinProblem::BadScale { field }` for an empty ladder and a spacing, corner or type scale that is non-finite, negative (zero, for type) or falling. `model::cie_lightness` and `model::ELEVATION_STEP`, re-exported at the crate root.
- `examples/ladder` (five nested surfaces labelled with their CIE L*, the Theme row in the innermost), `docs/research/tokens.md`, "Design tokens and the surface ladder" in `docs/guide/skins.md`, `docs/media/ladder.gif`.

### Changed

- `GooeySkin` has four more public fields, `spacing`, `radii`, `type_scale` and `surfaces`; `SkinProblem` and `Role` have more variants (`ElevationStep`, `BadScale`, `Surface(u8)`), so a `match` on them needs the arms and `Role`, which holds a `u8` now, is no longer castable with `as`. A struct literal that names every field needs the four, a literal that ends in `..GooeySkin::dark()` (or `default()`) is unchanged, and a skin file without them reads the dark skin's. `with_colours_of`, so a theme-row step, carries the ladder with the colours and keeps the scales. `SkinPreset::of` counts the ladder in a look's colours.

## [0.10.0] - 2026-11-01

### Added

- `Density` (`Comfortable`, the default, and `Compact`) and `GooeySkin::density`: a compact skin draws half the padding and caps the least target at 24 px (`MIN_TARGET_PX`, WCAG 2.2 SC 2.5.8), so a desktop tool's rows stand close enough for a mouse; `GooeySkin::pad()` and `GooeySkin::target()` are the values the widgets read. `problems()` checks `target()`, so a compact skin whose `min_target` is under 24 px still reports `TargetTooSmall`. A skin file without the field reads as comfortable.
- `GooeySkin::tool()` and `SkinPreset::Tool`, the fourth look and the fourth row of `skin_preset`: a cool neutral grey with a blue accent, `Density::Compact`, 14 px text, a 6 px gap, a 2 px border and a 4 px radius. A theme-row step onto it writes its colours only (a player's text size and density stay theirs); `problems()` is empty for it. `GOOEY_SKIN=tool` starts the gallery and the options example in it.
- `examples/density` (an Inspector panel with a Compact switch and the Theme row), `docs/research/density.md`, "Compact density for a tool" in `docs/guide/skins.md`, `docs/media/density.gif` and `tool.png`, and `tests/density.rs` (a compact button at least 24 px and shorter than a comfortable one, the padding halved and live, the floor still refused under 24 px, the Tool look holding every widget inside its box and every control at least 24 px high at 1x to 2x text, the Tool panel well under a game panel's height).

### Changed

- `GooeySkin` has one more public field, `density`: a struct literal that names every field needs `density: Density::Comfortable`, and one that ends in `..GooeySkin::dark()` (or `default()`) is unchanged. `SkinPreset::ALL` has four entries, so the theme row has a fourth option.

### Fixed

- A choice's arrow is at least `1 + 1/√2` chevrons wide, so at 200% text in a skin whose least target is 24 px the turned chevron no longer stands 1.7 px outside its arrow. The default skin is unchanged at every text size the options row offers (its arrow is 44 px).

## [0.9.0] - 2026-11-01

### Added

- `SkinScope(GooeySkin)`, a component on any node: that node and everything below it is styled from this skin and not from the `GooeySkin` resource, so a preview of a theme, a tool's own chrome or a vendor's window can stand next to the rest of the interface in another look. The nearest scope at or above a widget wins (its own included), a nested scope shadows the one above, none is the resource; it is the whole skin, sizes and `text_scale` included, and `GooeyFonts` stays global. `apply_skin` restyles exactly the subtree in the frame a scope is added, changed or removed, or a widget moves under or out of one; a changed resource leaves scoped widgets alone; a scope that fails `problems()` warns and is applied anyway. With no scope anywhere the cost is one empty-query check.
- `examples/scope` (an options panel next to a panel in a parchment `SkinScope`), `docs/research/skin-scope.md`, the "A skin for one subtree" section of `docs/guide/skins.md` and `tests/scope.rs` (two scopes' accents and removal in the same frame, nesting, a scope on the widget itself, a change confined to its subtree, a changed resource, reparenting in and out, a scope's sizes, nothing written idle).

### Changed

- `apply_skin` takes 13 system parameters instead of 14 (the removal readers are one tuple). A host that adds it with `add_systems` is unchanged; one that calls it as a plain function (it is a Bevy system, so none should) needs the new signature.

## [0.8.0] - 2026-11-01

### Added

- `GooeySkin::high_contrast()`, a third preset for the player who needs more contrast than WCAG's minimum: opaque black surfaces, white text and white outlines, an amber accent, a cyan focus ring and a light red for danger (WCAG advises against red on black, for protanopia), on the sizes of the other presets so a layout never moves. Every pair `problems()` checks is at least 7:1 (`ENHANCED_CONTRAST`, WCAG 2.2 SC 1.4.6 and Xbox Accessibility Guideline 102's "7:1 for all UI elements"), and `GooeySkin::high_contrast_problems()` is that check for any skin, so a game that writes its own high-contrast mode has the test in one call (empty for `high_contrast()`, and not for `dark()` or `parchment()`, which meet the minimums and no more).
- `skin_preset(label)`: the options-screen row for the look, a `choice` over Dark, Parchment and High contrast (`SkinPreset::ALL`) that follows the skin like `text_size` does: the preset whose colours are nearest (a skin a game tuned shows the nearest and is left alone until the player steps), a step writing the preset's colours and no size (`GooeySkin::with_colours_of`; a player at 200% text who asks for high contrast gets both), a game's own write moving the row with no `ValueChange`. `SkinPreset` (`#[non_exhaustive]`; `ALL`, `name`, `skin`, `of`; serde under the `serde` feature) and `OptionRow::SkinPreset`.
- `GooeySkin::surface_edge: Option<Color>`, the border of a panel, a tooltip and the prompts legend: `None` (both older presets) is `surface_raised`, the frame they have always drawn, so they are unchanged to the pixel; `high_contrast()` sets white, so a tooltip over a panel is a card of its own. A skin that sets it is checked against the surface at 3:1 (`SkinProblem::LowContrast`, pair `surface_edge on surface`).
- `examples/gallery` and `examples/options` start in high contrast with `GOOEY_SKIN=high_contrast`; the gallery's Display page has the Theme row (in place of Window), and the options example has it as its fourth row.
- `docs/research/high-contrast.md`, `docs/guide/skins.md` and `docs/guide/options.md` (the three looks, the 7:1 check, the row) and `docs/media/theme.gif`.
- `tests/skin.rs` (high contrast holds 7:1 and the other presets do not, so the check can fail; every text a high-contrast screen draws, in every button state, measured against the fill under it at 7:1; the edge of a panel, a tooltip and the legend in each preset) and `tests/options.rs` (the theme row: the three looks in order, a step writing the colours and no size, the skin a game sets moving the row with no event and a tuned skin showing the nearest, a first frame over a loaded high-contrast skin, nothing written idle).

### Changed

- `GooeySkin` has one more public field, `surface_edge`: a struct literal that names every field needs `surface_edge: None`, and one that ends in `..GooeySkin::dark()` (or `default()`) is unchanged. A saved skin file that lacks the field reads as `None`.

## [0.7.1] - 2026-11-01

### Added

- `text_size(label)`, `interface_size(label)` and `pad_style(label)`: the options-screen rows for the three settings this crate itself reads, one line each and nothing to wire. Each is a `choice` (Bevy's `Button`, one tab stop, Left, Right, the D-pad and a click step it, `Tooltip`, `InteractionDisabled` and a game's own `ValueChange<usize>` observer work as on any choice) over a step table: the text size 100%, 125%, 150% and 200% (`GooeySkin::text_scale`; WCAG 2.2 SC 1.4.4 and Xbox Accessibility Guideline 101 ask for 200% and for it to reach the dialog, the toast and the legend, which `text_scale` does), the interface size 75%, 100%, 125% and 150% (`UiScaleSettings::scale`; below 100% for the player who wants less of the game covered, 150% as EVE Online's UI scaling) and the controller Xbox, PlayStation and Switch (`BindingSettings::pad_style`). The setting lives once, in its resource: a player's step writes that one field and no other (a game's own skin, the scale's `reference_short_side` and the binding prompts survive), and the row follows the resource with no `ValueChange`, on the frame it is spawned and whenever the resource changes, so a loaded save shows on it, a second row follows the first, and a value between two steps shows the nearest and is left alone. Labels are the host's; the options are percentages and the makers' names.
- `TEXT_SIZE_STEPS`, `INTERFACE_SIZE_STEPS`, `PAD_STYLES`, `nearest_step` and `PadStyle::name` in `model`; `OptionRow` (a `#[non_exhaustive]` component marking the row) and `update_option_rows` (in `GooeySystems::Style`, before `update_choices`) for a host that registers the systems itself.
- `examples/options`, the three rows and the legend on a screen of their own, with the gallery's capture knobs and `padup`, `paddown`, `padleft` and `padright` for a pad's D-pad; `docs/research/options.md`, `docs/guide/options.md` and `docs/media/options.gif`.
- `tests/options.rs` (the rows: a first frame over a loaded setting, one write a step and no other field, no event for a game's write, the controller renaming a binding's keycap, a plugin left out, nothing written idle) and `tests/large_text.rs` (every widget, a dialog, a toast and the legend inside their boxes at each text size the row offers, and every control's text on one line at every 5% from 100% to 200%).

### Changed

- A choice's value cell is never narrower than 8 body text sizes (160 px at the default size), so choices stacked in a panel have their arrows and values in one column each instead of each at its own widest option's width: the gallery's Display page (Quality, Window) lines up and `tabs.gif` is regenerated.

### Fixed

- A control's label could wrap onto two lines when its row decided its panel's width and layout left the row a hair narrower than the label: at exactly 125% text a slider's label stood on two lines and its row was twice as tall. The label of a toggle, slider, choice, binding or text input, a choice's options, a slider's readout and a binding's keycap parts are one line now (`LineBreak::NoWrap`).

## [0.7.0] - 2026-11-01

### Added

- `text_input(label, value)`: a row for a line of text the player types (a name, a seed, a server address, what to call a save), the first widget a game needs that no other could be: the label on the left, outside the field and always visible, and a framed field on the right, Bevy 0.20's own headless `TextInput` and `EditableText` (a caret, a selection, copy and paste, IME), 17 characters wide at the body size (16, and the one character of room Bevy keeps after the caret, so a name that fills a `TextLimit(16)` shows whole) and a button's height at least, so the row never changes size while the player types. A focused field takes the keys at once (no step to begin typing); Left, Right, Home and End are the caret's, and **Up, Down, Tab, the D-pad and Escape leave it**, so a field never traps focus (WCAG 2.1.2); a click on the label, the field or the row's padding focuses it. The row keeps its state in `TextInputValue(String)` (read it, or write it to change the field from code with no event coming back) and sends Bevy's `ValueChange<String>` for each edit that changes the text (`is_final` false) and once when the edit ends (Enter, a submit even with nothing changed, or focus leaving after a change). `TextHint` (muted words while the field is empty; the label is never the hint, NN/g) and `TextLimit` (a most characters; never cuts a value the game set) go on the row.
- `TextEditing`, on the row exactly while its field has focus, and `is_text_editing`, a run condition: a game gates its own hotkeys on `not(is_text_editing)` so typing a name does not walk the character, and a host opens the platform's on-screen keyboard for a pad player when `TextEditing` is added (Valve requires one on Steam Deck; its keys arrive as ordinary key events the field takes). `track_text_editing` (in `GooeySystems::Navigate`, second in the set) and `update_text_inputs` (in `GooeySystems::Style`, before `apply_skin`), `TextInputOf` (on the field and the hint), `TextEdited`; `Role::{TextInput, TextCell, TextField, TextHint}`, styled in `apply_skin` like every other part: the field is an outlined box in the panel's own colour (the `outline` frame, the muted text colour under the pointer, the `focus_ring` while it holds focus **whether or not focus is shown**: a pointer press hides every other ring, but a caret already says who has the keys), its caret the text colour and its selection the accent with the accent's text.
- `GooeySkin::problems()` checks two more pairs, since the hint and the ring now stand on the panel: `text_muted on surface` at 4.5:1 and `focus_ring on surface` at 3:1 (both presets pass).
- The gallery's right panel holds a **Save as** field (`GOOEY_KEYS` tokens `name` and `type=Aria`); `docs/research/text-input.md`, `docs/guide/text-input.md` and `docs/media/text-input.gif`.

### Changed

- `Role` has four more variants (`TextInput`, `TextCell`, `TextField`, `TextHint`): a `match` over it that is exhaustive needs arms for them. `apply_skin` reads a `TextCursorStyle` more (a field's caret and selection).
- `navigate_by_input` leaves a focused text field's Left and Right keys to its caret and still lets the D-pad move focus off it; a focused field writes `UiFeedback::Focus` like every other widget.
- `show_tooltips` reads one more query: a `text_input` row's `Tooltip` opens while its field has focus (the row is the widget, the field is where focus stands), so the system takes six parameters instead of five.
- The gallery's Load game panel is 402 px wide instead of 302 (the Save as row is label, gap and a 17-character field wide), which moves the settings panel 50 px left; `cancel.gif`, `prompts.gif` and `dialog.png` were regenerated and the crops in `docs/media/README.md` follow.

## [0.6.0] - 2026-10-31

### Added

- `prompts()` and `prompt(label, keys)`: the hints that say which input does what ("Esc Back"), for Xbox Accessibility Guideline 112's "interaction prompts to select or go back are consistently placed on the bottom-left corner of the screen". A prompt is a keycap and the game's words; the keycap names the input of the device the player used last, in the words of a `binding` row ("Enter", "Esc", "A" or "Cross", by `BindingSettings::pad_style`), and a pad button is fully round where a key is a rounded rectangle. `prompts()` is a slim bar on the skin's panel colour, absolute in its parent's bottom-left corner 5% in from the edges (so adding it moves nothing), no wider than 90% of its parent; one in a `dialog()` stands at the same corner over the scrim. Every part is `Pickable::IGNORE` and none is a tab stop.
- `Prompt { keyboard, pad }` (the prompt's state, a component: set it after a rebind and the keycap follows in the same frame), `Prompt::SELECT` (Enter, South), `Prompt::BACK` (Escape, East) and `Prompt::new`; `InputDevice` (`Keyboard` or `Pad`, a resource both plugins initialise) and `track_input_device`, first in `GooeySystems::Navigate`, which keeps it from the last key or mouse button pressed, pad button pressed or stick pushed past `model::STICK_PUSH` (half its travel); a mouse that only moves changes nothing. `Binding::device()` names the device an input is pressed on. A game reads `InputDevice` to swap glyphs of its own.
- `update_prompts` (in `GooeySystems::Style`, before `apply_skin`), `Role::{Prompts, Prompt, PromptKey, PromptButton}`, styled in `apply_skin` like every other part; the gallery's legend (Select and Back, bottom left, and again in the Quit dialog) and `GOOEY_KEYS` tokens `plug` and `pad`; `docs/research/prompts.md`, `docs/guide/prompts.md` and `docs/media/prompts.gif`.

### Changed

- `Role` has four more variants (`Prompts`, `Prompt`, `PromptKey`, `PromptButton`): a `match` over it that is exhaustive needs arms for them.
- A `binding` row's device is now its value's (`Binding::device()`), and the private `BindingDevice` component is gone: a row whose `BindingValue` a game sets to the other device's input listens to that device from then on, where it used to keep the device it was spawned with.
- `docs/media/gallery.png` and `gallery.gif` are regenerated: the gallery now has the legend in its bottom-left corner.

## [0.5.0] - 2026-10-31

### Added

- `binding(label, value)`: a row that rebinds one action, for a controls screen (Xbox Accessibility Guideline 107, Game Accessibility Guidelines: "remappable controls are one of the best value accessibility features"). The label on the left and a keycap on the right with the key, mouse button or pad button it is bound to ("Space", "Right Click", "A"); its root is Bevy's headless `Button` carrying the new `BindingValue(Binding)` (the state: read it, or write one to change the row from code with no event), so Tab, the arrows, the D-pad, Enter, Space, South and a click reach it and start a capture. The keycap lights in the accent and says what it waits for ("Press a key, Esc exits"); the player's next press is the binding, said once as Bevy's `ValueChange<Binding>`. A key row hears any key and any mouse button, a pad row any pad button; Escape (and Start on a pad row) exits without binding and cannot be bound; a press on the other device exits. The press that started a capture is never the binding. The keycap and the row never change size (the name and the prompt share one cell; at least 17 body text sizes wide, so a column of rows shares its edges), and the row is a toggle's height.
- `Binding` (`Key(KeyCode)`, `Mouse(MouseButton)`, `Pad(GamepadButton)`; `Eq + Hash`, serde under the `serde` feature, `label(PadStyle)` for hints and tutorials), `BindingValue`, `Capturing` (on a row while it waits), `BindingSettings` (`key_prompt`, `pad_prompt` with `{exit}` filled in, `pad_style`, `cancel_key`, `cancel_button`; a resource `GooeyStylePlugin` and `GooeyNavPlugin` both initialise) and `PadStyle` (`Xbox`, `PlayStation`, `Switch`).
- While a row waits nothing else hears the player: `not_capturing` (a run condition, now on `navigate_by_input`, `gamepad_select`, `gamepad_adjust`, `cancel_by_input`, `page_lists` and `step_tabs`), the row stopping the key on its way to Bevy's Tab navigation (so Tab is bindable), no `Cancel` for the Escape that exits, and a transparent node over the window (`catch_pointer`) that takes the pointer, so a click is a binding and presses nothing under it. `bind_by_input` (in `GooeySystems::Navigate`, after the held-back systems), `catch_pointer` and `update_bindings` (in `GooeySystems::Style`, before `apply_skin`) join the plugins; nothing runs at rest but one empty query each.
- `model::{key_name, mouse_name, pad_name, PadStyle}`, the engine-free naming under it (tests and doctests); `Role::{Binding, BindingCap, BindingText, BindingPrompt}`, styled in `apply_skin` like every other part; `UiFeedback` for the start and the bind (focus on a binding row writes `Focus`).
- The gallery's Controls tab holds two binding rows (Jump on Space, Use on the pad's South), logs and toasts the new key, and `GOOEY_KEYS` has tokens `q` and `f`; `docs/research/binding.md`, `docs/guide/bindings.md` and `docs/media/binding.gif`.

### Changed

- `Role` has four more variants (`Binding`, `BindingCap`, `BindingText`, `BindingPrompt`): a `match` over it that is exhaustive needs arms for them. `GooeyNavPlugin` runs `navigate_by_input`, `gamepad_select`, `gamepad_adjust`, `cancel_by_input`, `page_lists` and `step_tabs` under `not_capturing`, so a host that registers those systems itself adds the condition; with no `binding` row in the game it is always true. The gallery's Controls tab holds `binding` rows where it held two placeholder buttons.
- `apply_skin` takes one more system parameter (`RemovedComponents<Capturing>`): a host that adds `GooeyStylePlugin` is unaffected, one that calls or registers `apply_skin` by hand rebuilds against the new signature. Breaking for both reasons (`cargo semver-checks`: `enum_variant_added` on `Role`, `function_parameter_count_changed` on `apply_skin`), so the version is 0.5.0.

## [0.4.1] - 2026-10-31

### Added

- `Cancel { entity }`, the player backing out: Escape, or any gamepad's East button (B on Xbox, Circle on PlayStation), one event a press, for a keyboard and a pad in one handler. It starts at the focused widget and bubbles up its parents (Bevy's event propagation, so `observe(|cancel: On<Cancel>, ..| ..)` on the dialog or the screen being left hears it, `cancel.propagate(false)` keeps a layer above from also acting, and `On::original_event_target()` is the widget that had focus), and it never leaves a `dialog()`: a modal dialog is its own scope, so a Back inside it does not reach the screen under it, even one it was spawned beneath. With nothing focused, or focus left behind it, an open dialog still hears it; with no dialog and nothing focused nothing is sent, so Escape stays the game's own pause key. A press that finds a target shows the focus ring. The crate closes nothing: a game observes `Cancel` and despawns its dialog (focus goes back to the button that opened it).
- `cancel_by_input` (in `GooeySystems::Navigate`, after `gamepad_select`; `GooeyNavPlugin` registers it and the `Cancel` type), `CancelScope`, the event's traversal, and `docs/research/cancel.md`.
- The gallery's Quit dialog closes on `Cancel`, and `GOOEY_KEYS` has an `escape` token.

### Changed

- A `panel()` that is a direct child of a `dialog()` is the dialog and looks like one (in `apply_skin`, with no new `GooeySkin` field): 21 to 36 body text sizes wide (420 to 720 px at the default size, growing with `text_scale`), one and a half times a screen panel's padding and gap, and framed in `GooeySkin::outline` (the colour `problems()` already holds at 3:1 against the panel) where a screen's panel keeps `surface_raised`. The scrim pads itself 5% of the window on every side, so a long body wraps at the maximum or squeezes the dialog in and nothing of it reaches the bezel. A game's existing `dialog()` code is better the day it updates, and a `heading` and a `label` in the panel are its title and its body. A panel anywhere else is unchanged, and so is every pixel of a screen with no dialog open. The gallery's Quit dialog has a title and a line of body. `docs/research/dialog-presence.md`.
- `GooeySkin::parchment()`'s `scrim` is `srgba(0.08, 0.05, 0.02, 0.7)`, from 0.55: over the cream menu a parchment dialog was barely lighter than what it stood on. A skin of your own keeps its scrim.

## [0.4.0] - 2026-10-31

### Added

- `choice(label, options, selected)`: the third settings control, for a setting with a few named values (Low, Medium, High). A row with the label on the left and the selected option on the right between two drawn chevrons (two borders of the skin's line weight on a square turned an eighth of a turn, so they are no font glyph and scale with the text). Its root is Bevy's headless `Button` carrying the new `ChoiceValue` (the selected index, the widget's state: read it, or write one to move the choice from code) and `ChoiceCount`, so it takes `observe`, `Tooltip` and `InteractionDisabled`, and says each change once through `ValueChange<usize>`. With it focused, Left and Right (keys, and the D-pad through `gamepad_adjust`, a held direction repeating) step to the previous and next option and stop at the ends, where the chevron that cannot step is dimmed and a press changes and says nothing; Home and End jump to the ends; Up and Down still move focus on. A click on a chevron steps once, on release; Enter, Space, the pad's South button and a click anywhere else on the row cycle to the next option and wrap round. Every option is laid out in one grid cell, so the row, the chevrons and the value never move while stepping. The row is a toggle's height and shares its edges; the chevrons are `min_target` wide.
- `update_choices` (in `GooeySystems::Style`, before `apply_skin`) shows a choice's selected option and pulls a value past the last option back to it; `model::{choice_step, choice_next}`, the engine-free maths under it.
- `Role::{Choice, ChoiceArrow, ChoiceBack, ChoiceForward, ChoiceCell, ChoiceOption}`, styled in `apply_skin` like every other part.
- `UiFeedback`: a choice gaining focus writes `Focus`, and each change of its option writes `Activate`, once.
- The gallery example has a Quality choice (it logs and toasts the option it settles on) and `GOOEY_KEYS` tokens `space`, `home`, `end`, `back` and `forward` (a click on a chevron).
- `list(rows, children![..])`: rows that scroll, for save slots, an inventory, a key-binding screen. A wrapper (`Role::List`) holding a clipped viewport (the new public `ListView { rows }`, on Bevy's `ScrollArea`, so the wheel and a trackpad scroll it) and a rail with a thumb (Bevy's `Scrollbar`, so a drag on the thumb and a click on the rail scroll it). The viewport stands `rows` rows tall (a row is a button's height and the `gap` after it, following `text_scale`), so `4.5` cuts the fifth row, the sign that more is below. The rail, `2 × border` wide, shows only while the rows do not fit; its lane is always reserved so rows never change width. The thumb is `text_muted` on a `surface_raised` track (the pair `problems()` already holds at 3:1), `text` under the pointer and `accent` while dragged.
- `scroll_to_focus` (in `GooeySystems::Navigate`): a list follows the focus a key, the D-pad or Tab gives one of its rows, by the least scroll that leaves a row and a half of list beyond it (the next row whole, the one after cut), moved on to a row's top so the list stands as it does at rest with a row flush at its top and the row it cuts cut the same way at every step (not at the end, where it ends flush with the last row); a row taller than the list scrolls to its top and a jump of more than a viewport centres the row. A focus the pointer set is not followed, so a press never scrolls the content from under its own release. `page_lists` (same set): PageUp, PageDown and the pad's left and right triggers move focus a viewport. `update_lists` (in `GooeySystems::Style`) shows each rail only while there is more to scroll, writing it only on a change.
- `Up` and `Down` stay among a list's rows until they run out, and a row a list has clipped away is not a neighbour from outside the list (`navigate_by_input`): geometry alone stepped out of a list past rows it still held, onto the button below.
- `model::{list_height, scroll_to_reveal, snap_to_row, page_target}`, the engine-free maths under it (tests and doctests); `Role::{List, ListView, ListRows, ListRail, ListThumb}`, styled in `apply_skin` like every other part.
- The gallery example has a Load game list of twelve save slots beside its panel, and `GOOEY_KEYS` tokens `slots`, `pagedown`, `pageup` and `wheel`.
- `tabs(labels, selected, children![tab_page(..), ..])` and `tab_page(children![..])`: a strip of one- or two-word labels over pages of widgets, one shown at a time, for a settings screen too long for one column. The strip is Bevy's headless `TabList` and `Tab` (a click, Left and Right with wrap and Home and End, Enter and Space, a roving Tab stop, `TabActivation::Automatic` so focus selects) in a grid of equal columns, each a button's size and look: the selected tab is lit in the accent with `accent_text` on it, the others are `surface_raised` with `text`. The pages stand in one grid cell, so the widget is as large as its largest page and turning a page moves nothing; the pages not shown are `Visibility::Hidden`, which keeps their room and takes their widgets out of Tab, the arrows and the pointer. The entity carries the new `TabsValue` (the selected index: read it, or set it to turn the page from code, which says nothing) and `TabsCount`, and every change a player makes is one `ValueChange<usize>` on it and one `UiFeedback::Activate`; a tab gaining focus writes `Focus`.
- `step_tabs` (in `GooeySystems::Navigate`): **LB and RB** (the pad's `LeftTrigger` and `RightTrigger`) and **Ctrl+PageUp and Ctrl+PageDown** turn the page from anywhere in the widget, or from beside it when it is the only tabs widget in view and in the focus's scope (an open `dialog` is its own scope), and wrap; the D-pad's Left and Right step a focused tab. `page_lists` ignores PageUp and PageDown while Ctrl is held. `land_tab_focus` (same set, before `navigate_by_input`): focus never rests on a page that is not shown; when the page it was in turns away, it lands on the first widget of the page now shown, or on the selected tab when that page has none. Up and the arrows that reach the strip from a page land on the selected tab, not the nearest. `update_tabs` (in `GooeySystems::Style`, before `apply_skin`) shows the selected page and lights the selected tab in the frame the value changes.
- `model::tab_step`, the engine-free maths under it (wraps; tests and a doctest); `Role::{Tabs, TabList, Tab, TabText, TabPages, TabPage}`, styled in `apply_skin` like every other part.
- The gallery example's settings are three tabs (Audio: Volume and Subtitles, Display: Quality and Window, Controls: two buttons), and `GOOEY_KEYS` tokens `nexttab`, `prevtab` (Ctrl+PageDown and Ctrl+PageUp) and `tab1`, `tab2`, `tab3` (a click on that tab).

### Changed

- `Role` has seventeen more variants (six for `choice`, five for `list`, six for `tabs`): a `match` over it that is exhaustive needs arms for them.
- `FocusScopes` gained a query over lists, and `show_focus` is crate-visible: callers of `navigate_by_input` are unchanged.
- `FocusScopes` gained queries over tabs and their widgets, and `step_tabs` and `land_tab_focus` join `GooeyNavPlugin`'s chain and `update_tabs` `GooeyStylePlugin`'s; a focused tab keeps Left and Right as a slider and a choice do (`navigate_by_input` does not also move focus).
- `gamepad_adjust` steps a focused, enabled choice as well as a slider (its query over sliders became one over both, so its signature changed) and `FocusScopes` keeps Left and Right for a focused, enabled choice as it does for a slider; a host that registers these systems by hand gets them from `GooeyNavPlugin` as before, and calling them directly is unchanged.

### Fixed

- A `Tooltip` on a widget inside a clipped container (a row of a `list`) was clipped by it, so its box, which stands beside the widget and outside the container, never showed: the box now carries Bevy's `OverrideClip`.

## [0.3.0] - 2026-10-31

### Added

- `model::snap_line_width`: a line weight snapped to whole device pixels at a scale, CSS's "snap a length as a border width" (rounded down above one device pixel, up to one below it).
- `toggle(label, on)` and `slider(label, value, range)`: the two controls an options screen is made of. Each returns one bundle whose root is Bevy's own headless widget (`Checkbox`, `Slider`), so the spawned entity carries the state (`Checked`, `SliderValue`), takes `observe`, `Tooltip` and `InteractionDisabled`, and reports `ValueChange<bool>` / `ValueChange<f32>` (re-exported with `Checked`, `SliderValue`, `SliderRange` and `SliderStep`). A toggle is a row with a label and a framed switch whose knob slides to the end and takes the accent when on; a slider is a row with a label, a readout (a percentage for a range of 0 to 1, the number otherwise) and a rail with a thumb, the whole row the click and drag target. Both are at least `min_target` high, wear a button's focus ring, show the pointer with an outline, dim when disabled and grow with `text_scale`.
- A focused slider takes Left and Right (keys through `bevy_ui_widgets`: a twentieth of the range a step, Home and End to the ends) and the D-pad (`gamepad_adjust`, new, in `GooeySystems::Navigate`), which repeats a held direction after `PAD_REPEAT_DELAY` (0.4 s) at `PAD_REPEAT_RATE` (12 a second), so a pad player adjusts a volume with no select step and no held button. Up and Down still move focus on. The pad's South button flips a focused toggle.
- `update_sliders` (in `GooeySystems::Style`) keeps a slider's thumb, fill and readout in step with its value; `model::slider_fraction`, `slider_text` and `repeat_steps`, the engine-free maths under it.
- `Role::{Toggle, ToggleTrack, ToggleKnob, Slider, SliderHeader, SliderTrack, SliderRail, SliderFill, SliderGap, SliderThumb, SliderReadout, ControlText}`, styled in `apply_skin` like every other part.
- `UiFeedback`: a toggle or a slider gaining focus writes `Focus`, a toggle flipping writes `Activate`.
- `GooeySkin::problems()` checks one more pair, the off switch's knob (`text_muted`) against its track (`surface_raised`) at 3:1. Both presets pass; a custom skin whose muted colour sits near its raised surface now warns.
- `toast(text)` and `toast_alert(text)`: a line of news at the edge of the screen, spawned in one call from any system (`commands.spawn(toast("Game saved"))`) with no layout and nothing to set up. The first one creates the one `ToastStack` (a full-window, pointer-transparent column at `GlobalZIndex` 150: above a `dialog`'s scrim, below a tooltip) and every `Toast` becomes its child in the frame it is spawned, through a component hook. It stands as long as its message takes to read, `(base_seconds + characters ÷ chars_per_second) × duration_scale`, then fades out and despawns; a fourth beyond `max_visible` (3) sends the oldest into its fade at once. It fades in as the room it holds opens and out as that room closes (the body is clipped to a height that `apply_skin` opens and closes), so the toasts round it glide to make way and close up and none of them jumps. It runs on `Time<Real>`. A toast takes no input and no focus: every node of it is `Pickable::IGNORE`, it is no tab stop and nothing in it writes `InputFocus`. A mark down its left edge is the accent for `toast` and `danger` for `toast_alert`, twice as wide, so the kind is told by shape as well as colour; the words carry it too. It is not announced to screen readers yet.
- `ToastSettings` (a resource, serde data with the `serde` feature): `anchor` (`ToastAnchor`, six places, bottom centre by default; the newest stands nearest the edge), `max_visible`, `base_seconds` (2.0), `chars_per_second` (20, Netflix's reading speed for adults), `duration_scale` (1.0: the player's option, which Xbox Accessibility Guideline 116 wants adjustable to at least ten times the default; any value above zero works) and `fade_seconds` (0.3; zero removes the fade and the opening of room). `update_toasts` (in `GooeySystems::Style`) runs the clocks and writes a toast only when its opacity or room changes.
- `model::{toast_hold, toast_life, toast_alpha, toast_grow}`, the engine-free maths under it; `Toast`, `ToastKind`, `ToastStack`, `Role::{ToastStack, Toast, ToastBody, ToastMark, ToastText}`.
- `GooeySkin::problems()` checks two more pairs, `accent` and `danger` against the solid surface a toast stands on, at 3:1. Both presets pass; a custom skin whose accent or danger colour sits near its surface now warns.
- The gallery example spawns toasts (Play, the Subtitles switch and the Volume slider; `GOOEY_KEYS` tokens `toast`, `alert` and `long`; `GOOEY_TOAST_ANCHOR`).

### Changed

- `Role` has seventeen more variants (twelve for the settings controls, five for toasts): a `match` over it that is exhaustive needs arms for them.
- `gamepad_select` takes one more query parameter (the toggles it flips) and `FocusScopes` one more, so a host that registers these systems by hand gets them from `GooeyNavPlugin` / `GooeyStylePlugin` as before; calling them directly is unchanged. `navigate_by_input` leaves Left and Right to a focused, enabled slider.

### Fixed

- Every line is one weight at every scale. At a scale that is not a whole number (1920x1080 draws at 1.5x, a 1280x800 Steam Deck at 1.11x, a 150 % display), Bevy's layout rounds each border and padding from the node's sub-pixel position, so a meter's gap was 5 px above its fill and 4 px below at 1080p, and any border could split the same way depending on where its widget landed. `apply_skin` now draws every border (panel, button and its focus ring, meter frame, tooltip) and the meter's gap a whole number of device pixels wide at the node's own scale, and restyles a widget when that scale changes: the 1080p meter is 4 / 4 / 20 / 4 / 4 px, its fill centred.

## [0.2.0] - 2026-10-30

### Added

- `GooeyScalePlugin`, in `GooeyPlugins`: the whole interface keeps its share of the screen at every resolution. `scale_with_window` sets Bevy's `UiScale` to the primary window's shorter side over `UiScaleSettings::reference_short_side` (720), times the player's `UiScaleSettings::scale` (1.0), so a 1920x1080 window draws the skin's sizes at 1.5x and a 2560x1440 one at 2x, and the default text meets the Xbox Accessibility Guidelines' console minimum (26 px at 1080p) on any screen. A minimised window keeps the last scale. `UiScaleSettings` is serde data with the `serde` feature.
- `model::window_ui_scale`, the engine-free maths under it.
- `FocusScopes`, the system parameter `navigate_by_input` takes (which widgets focus may rest on, which open `dialog` is on top, where each scope starts), exported so the function's signature is nameable. The gallery example stages a pointer click with `GOOEY_KEYS=click` and no longer forces `InputFocusVisible`.
- `update_tooltip_text`, the `GooeySystems::Style` system that copies a changed `Tooltip`'s text into its box.

### Changed

- `UiScale` is set by `GooeyScalePlugin` rather than left to the host. A host that drives `UiScale` itself leaves the plugin out (`GooeyPlugins.build().disable::<GooeyScalePlugin>()`); a player's interface size goes in `UiScaleSettings::scale`. At 1280x720 nothing changes.
- `navigate_by_input`, `gamepad_select` and `focus_opened_dialogs` write Bevy's `InputFocusVisible`, so they need that resource. `GooeyNavPlugin` and `GooeyStylePlugin` add it; a host that registers the systems itself calls `init_resource::<InputFocusVisible>()`. `navigate_by_input` takes `FocusScopes` in place of its `TabGroup` and `ChildOf` queries.

### Fixed

- An arrow key, a D-pad press and the pad's South button now show the focus ring and open a focused widget's `Tooltip` (they set Bevy's `InputFocusVisible`, which only Tab set before, so a pad player never saw focus at all and a keyboard player lost it after any click); a pointer press still hides it. A dialog that takes focus shows it too. The flag is written only when it changes, so a press with focus already shown restyles nothing. Fixes #1.
- With nothing focused (a click on empty space, a focused widget despawned, a screen spawned without `AutoFocus`), the first arrow or D-pad press lands where the screen starts: the topmost open `dialog`'s default, or else the screen's `AutoFocus` widget or its first widget in Tab order; the next press moves on. Before, no direction could ever reach a widget again, so one click stranded a pad player. A focused tab stop that is not directionally navigable (a text field) keeps its arrows. Fixes #2.
- A `Tooltip` is a view of its component. Removing it removes its box (the orphaned box kept opening on hover and on focus, with the old text); removing it and adding another leaves one box, not two; inserting a new `Tooltip` over an old one, or editing `Tooltip.0` in place (as a reflection-driven inspector does), changes the open box's line in place, without a blink. A despawned widget leaves no box. Fixes #3.
- `screen()` no longer blocks the pointer. It is a full-window layout root, and in Bevy 0.20 a UI node without `Pickable` hides everything under it from picking, so a HUD built on it kept every sprite, mesh and other UI root beneath it from receiving `Over`, `Press` or `Click`, and any click on the world cleared focus through the HUD instead. It is now `Pickable::IGNORE`; `panel`, `button`, `meter`, the other widgets and `dialog()`'s scrim still take the pointer, and a click on the empty part still blurs the interface (it lands on the world, or the window, and Bevy clears focus there). A screen that must swallow the pointer opts back in with `commands.spawn(screen()).insert(Pickable::default())`. **A `(screen(), Pickable::IGNORE)` workaround now panics on the duplicate component: delete it.** The gallery's `GOOEY_WORLD=1` puts a sprite under the interface for a click to land on. Fixes #4.

## [0.1.0] - 2026-10-30

### Added

- `GooeySkin`, the look as serde data, with the `dark()` and `parchment()` presets, a `text_scale` player option, `outline` (the edge of a widget whose fill does not show its size, such as a meter's frame), `meter_height`, `problems()` (WCAG 2.2 contrast and target-size checks, as `SkinProblem`, including a meter's fill against its track, its frame against the panel, and a meter too thin for its fill) and `solid_surface()`, the opaque fill of a panel in a dialog.
- `GooeyFonts`, the host's body and heading fonts.
- `Role` and `apply_skin`, the one place every widget is styled.
- `screen()`, a full-window, centring tab group.
- `panel()`, a framed, padded column.
- `menu()`, a column of buttons.
- `heading()` and `label()`, text in the skin's sizes and fonts.
- `button()`, Bevy's headless button, styled by its rest, hover, press, disabled and focus states.
- `meter()` and `Meter`, a HUD bar whose fill follows `value / max`, never NaN. The track is framed in `outline`, so its whole length reads against the panel; the fill sits one `border` inside the frame with concentric rounded corners; the bar is `meter_height × text_scale` high, so it grows with the player's size option.
- `Tooltip`, a line of help beside a widget, shown while the pointer is over it or while it has visible focus, so a keyboard or pad player sees it too. It opens across its container's flow (to the right in a column, below in a row) one `gap` outside the widget's panel, flips at the window's edge (Bevy's `Popover`), and never covers a neighbouring widget. It is drawn as a small raised panel with a drop shadow. `TooltipBox` marks its box; `show_tooltips` opens and shuts it.
- `dialog()`, a modal veil that holds focus: on opening, focus moves to its first button (or its `AutoFocus` child); Tab, the arrows and the D-pad stay inside it; on closing, focus returns to the widget that opened it. A panel in it is drawn opaque, so nothing behind shows through its text. `Dialog` is its focus bookkeeping; `focus_opened_dialogs` moves focus in.
- `navigate_by_input` (arrows and D-pad move focus, within the same scope Tab cycles) and `gamepad_select` (South activates).
- `UiFeedback` and `UiFeedbackKind`, written for every focus move and activation, for the host's sound.
- `GooeyStylePlugin`, `GooeyNavPlugin`, the `GooeyPlugins` group and the `GooeySystems` sets.
- `model`: `relative_luminance`, `contrast_ratio`, `composite`, `meter_fraction` and the WCAG constants.
