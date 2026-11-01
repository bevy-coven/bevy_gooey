# Table: a virtualized, sortable table

Item: bevy_grimoire's request G-12 (#16), `table(columns, rows)`: thousands of rows, only the visible ones spawned, a header that sorts, column edges that resize, single or multiple selection, and rows that change while the player is scrolled.

## Sources

- **Craft:** W3C WAI-ARIA Authoring Practices, *Grid (Interactive Tabular Data and Layout Containers) Pattern* (https://www.w3.org/WAI/ARIA/apg/patterns/grid/), the keyboard model of a data grid; and W3C *Understanding SC 2.5.7 Dragging Movements* for the column edge.
- **Shipping:** Unity UI Toolkit's `MultiColumnListView` (the engine's shipped table; `virtualizationMethod` `FixedHeight` / `DynamicHeight`, `fixedItemHeight`, sortable columns, `Column.resizable`); Godot 4's `Tree` column titles with `set_column_title`/`column_title_clicked` is the same shape.
- **Technical:** the scikit-learn API paper in the corpus (`API design for machine learning software: experiences from the scikit-learn project`: one small trait a user implements, defaults for the rest), and Bevy 0.20's `ScrollArea`, `Scrollbar` and `ScrollPosition` (a scroll position is in logical pixels, unclamped, and the layout clamps it).
- The corpus has nothing on tables or virtualized lists (`hs distill search` on virtualization, data tables, grid keyboard navigation: unrelated hits). Pending index below.

## Craft principle

The APG says a data grid is a composite widget that keeps the tab sequence short: "Only one of the focusable elements contained by the grid is included in the page tab sequence." Its keys: "`Down Arrow`: Moves focus one cell down", "`Page Down`: Moves focus down an author-determined number of rows, typically scrolling so the bottom row in the currently visible set of rows becomes one of the first visible rows", "`Control` + `Home`: moves focus to the first cell in the first row", and for selection "`Shift` + `Space`: Selects the row that contains the focus", "`Control` + `A`: Selects all cells", "`Shift` + `Down Arrow`: Extends selection one cell down". It also says what to expose when rows are not all present: "`aria-rowcount` ... is set to the total number of columns or rows", and warns that "key events that move focus to the beginning or end of the grid, such as `control` + `End`, may move focus to the last row in the DOM rather than the last available row in the back-end data": with a virtualized table the end is the data's, never the pool's.

A drag must have a way without a drag: WCAG 2.5.7, "All functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging". A column edge is dragged; the keyboard's way is a key on the focused header.

## How shipping games do it

Unity's collection views virtualize on a fixed row height: "Collection view won't wait for the layout to update items, as the all have the same height" (`CollectionVirtualizationMethod.FixedHeight`), with `fixedItemHeight` "the height of a single item in the list, in pixels", while `DynamicHeight` "will use the actual height of every item when geometry changes. More flexible but less performant"; and "The MultiColumnListView control doesn't support horizontal layout and virtualization. Applying horizontal layout to a MultiColumnListView breaks virtualization." So: fixed row height, a pool of row elements that is rebound as the scroll moves, columns that are widths, a header that sorts and raises an event the game answers.

## What great looks like

A header bar a shade above the rows with the column names in the heading weight, the sorted column marked by a chevron that points the way the order runs; rows one button high with quiet alternate bands (never the only mark of anything), the selected rows in the accent's fill with the text still above 4.5:1, the focused row ringed in the focus ring's colour only while focus is visible; a rail on the right exactly as the list's. Ten thousand rows scroll at the same cost as ten. A column edge is a quiet 1 px line that lights under the pointer and is a pointer-wide target.

## Model adopted

- `table(columns, rows)` spawns the root (`Table`, a focusable composite), a header strip (one `Role::TableHeader` button per column; a sortable one sorts, every one has a resize edge), a clipped viewport that scrolls (Bevy's `ScrollArea`, the list's rail) and in it a column of a top spacer, a **pool** of row entities and a bottom spacer. The spacers make the viewport's content the height of all the rows, so Bevy's wheel, scrollbar and clamp work unchanged; the rows' pitch is fixed (a button's height), the one fact virtualization needs.
- The pool holds `ceil(viewport / pitch) + 1` rows (the last is the one cut) plus at most one more when the scroll sits between two rows: never more than visible + 2 whatever the data. `update_tables` rebinds the pool to the rows `first..first + pool` of the table's **order**: the display order, a permutation of the source's rows.
- `RowSource` is the one trait a game implements: `len`, `cell(row, column) -> String`, and defaults for `key(row) -> u64` (the row's identity, its index) and `compare(column, a, b)` (natural order of the cells' text: numbers by value, text case-insensitively). `Vec<Vec<String>>` implements it. A source is a value in the `Table` component; `Table::source_mut::<T>()` changes it in place and `Table::set_source` replaces it. No change is detected by diffing: the game changes it through the component, so `Changed<Table>` is the signal.
- State, each its own component the game reads and writes: `TableSort { column, descending }` (a sort the game sets sorts in the same frame, with no `SortBy`), `TableSelection` (a set of row **keys**, so a sort or an insert never moves it), `SelectMode` (`None`, `Single`, `Multi`; default `Single`), `TableCursor` (the row focus is on, a key). Columns' widths are `ColumnWidths`.
- Events: `SortBy { entity, column, descending }` when the player picks a header (first click ascending, then descending, then ascending again; the table has already reordered), and `ValueChange<TableSelection>` when the player changes the selection. A game's own write sends neither.
- Anchor: when the source changes while scrolled, the table finds the row that was first in view by its key and keeps it at the same offset; a row that is gone keeps the scroll position. Selection by key means deleting a selected row drops it from the set; the cursor moves to the nearest row at the same display index.
- Keys (APG grid, rows only): the table is one tab stop; Up, Down, PageUp, PageDown, Home, End (Ctrl is not needed: a table's rows are one column of cells), Space selects the cursor row (toggles with Ctrl), Shift and an arrow extends, Ctrl+A selects all in `Multi`, Enter and South send `Activate` to the table (a double click too). The D-pad is Up/Down, South is Enter, the triggers page, the same as the list. A header is a button: Enter and Space sort it; Ctrl+Left and Ctrl+Right on a focused header move its right edge by 16 px (WCAG 2.5.7), a drag on the edge does the same by the pointer, and a double click on the edge resets the column to its starting width.
- Pointer: a click selects (Ctrl toggles in `Multi`, Shift selects the range from the cursor); the cell text is `Pickable::IGNORE` so press and release land on the row.

## Parameters

| Constant | Value | Why |
| --- | --- | --- |
| Row pitch | a button's height (`max(target, 1.5 text sizes + 2 pad + 2 line)`), no gap | the list's row; 44 px at 1x so a thumb hits it |
| Pool | `ceil(viewport / pitch) + 2` capped at the row count | cut row plus the scroll between rows |
| Column width unit | body text sizes (ems) | follows `text_scale`, so 200% text keeps the same characters per cell |
| Key step of an edge | 16 px (`SPLIT_KEY_STEP_PX`) | the split's, one rule |
| Least column | 3 text sizes | a header's chevron and a letter always show |
| Visible rows | `TableRows`/`rows` argument, default 8.5 | the cut row tells the player there is more |

## Validation targets

1. 10,000 rows spawn at most visible + 2 row entities, at 1x and at 200% text, and after a resize and after scrolling to the end.
2. Clicking a sortable header sends one `SortBy` with the column and direction and the rows are in that order; a second click reverses it; a click on an unsortable header sends nothing.
3. Scrolled to row 5,000, 100 rows inserted above: the first visible row is the same row at the same offset.
4. Up, Down, PageUp, PageDown, Home, End move the cursor and scroll it into view; End is row 9,999 of the data, not of the pool.
5. Selection: single replaces, Ctrl toggles, Shift ranges, by key across a re-sort; `ValueChange<TableSelection>` once per change.
6. A column edge dragged 40 px widens the column by 40 px; Ctrl+Right on the header does 16 px; never under the least width.
7. Contrast 4.5:1 for every text in every state (rest, banded, selected, cursor) in all four looks; the ring 3:1; a header a target high.
8. Nothing written while idle; thirty frames at rest change no component.

## Deviations

- The APG's grid moves focus among cells; this table keeps focus on the table and the cursor on a row, because a game's table is rows of text: one tab stop, a row cursor (the "active descendant" the APG describes for a composite widget). Cells are not focusable; a game that needs a button in a row spawns its own over the table. A design default.
- Rows have one height (Unity's `FixedHeight`); a cell that wants two lines is cut. A design default, and the reason 10,000 rows cost nothing.
- No horizontal scroll: columns wider than the table are cut (Unity says horizontal layout breaks virtualization too). Documented.

## Pending index

Nothing on tables, virtualization or data grids in home-still. The APG grid and sortable-table patterns and the Unity `MultiColumnListView` manual are worth indexing (`paper_search` finds papers, not these pages).
