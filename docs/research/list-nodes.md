# A scrolling list, in the accessibility tree

Researched 2026-11-09 (iteration 90). No issue is filed. The pass's own pick, and the largest gap the last
three accessibility passes have each recorded: a list of rows had **no nodes at all**.

The problem, measured. `Role::List`, `Role::ListView` and `Role::ListRows` were all `Spec::Group` in
`access.rs` — containers — and a list's rows are the game's own bundles, usually `button`s. So a screen
reader read the rows' names as a flat run of buttons with no list around them and no idea how many
there were, and `docs/guide/lists.md` said so: "A list is not announced to screen readers yet."

## Sources

1. WCAG 2.2 SC 4.1.2, Name, Role, Value (technical): a list is a role and an item count; a row is an item
   and a place in that set. Reading the names without the two roles is the failure the clause names.
2. WCAG 2.2 SC 4.1.3, Status Messages (technical, the same set of clauses): the tree must *say* the state,
   not merely draw it.
3. *Future design of accessibility in games: A design vocabulary* — 2019, IJHCS,
   doi:10.1016/j.ijhcs.2019.06.010 (craft): an interface's structure is part of its design, and a list
   whose order is invisible in the tree has lost the structure a sighted player reads.
4. Rune's list (the owner's reference, read not copied): its scrolling list registers a list node with a
   set size and each row registers its position, the same two facts AccessKit's list adapters read.
5. This crate's own tables, `src/access.rs:538-560`: a `Table` says `Ak::Table` with `row_count` and a row
   says `Ak::Row` with `row_index`. A list should mirror that with `Ak::List` / `ListItem` /
   `position_in_set`.

## Craft principle

An ordered set is a set *and* an order. Drawing one and saying neither leaves a reader with a pile of
names; the crate already knows how to say both, because its tables do.

## How shipping games do it

Rune pairs a list's container role with the item count and each row's index, so a reader hears "Meadow,
3 of 40" rather than a bare label. Nothing visual changes.

## Model adopted

- **The `ListView` is the node.** `Spec::Node`, described `Ak::List` with `size_of_set` = the rows in the
  one `Role::ListRows` column under it (`Look::list_rows`). The viewport rather than the outer wrapper,
  because the viewport is what scrolls and so what holds the rows.
- **A row is a list item.** A `Role::Button` inside a `ListRows` column reads `Ak::ListItem` with
  `position_in_set` = its index among the column's children and `size_of_set` = the column's count
  (`Look::list_place`). The index is read from the column, not stored, so a game that inserts or removes
  rows needs to do nothing.
- **Both are read through `Look`**, which already walks parents and children for the table code; no new
  component, no new system, and a list that never changes writes nothing.

## Parameters

| Name | Value | Where from |
|---|---|---|
| list node | `Ak::List` on the `ListView` | WCAG 4.1.2 |
| set size | the `ListRows` column's child count | the column is the list |
| item role | `Ak::ListItem` on a row button inside that column | the same |
| item place | its index in the column | read, never stored |

## Validation targets

- A list's viewport node is `Ak::List` with `size_of_set` equal to the rows it holds.
- Each row button is `Ak::ListItem` with `position_in_set` its index and the same set size.
- Mutation-checked: removing the row's `list_place` fails on the item role and the place.
- A button outside a list is unchanged (`Ak::Button`, no set).

## Deviations

- **A row keeps its button behaviour but changes its role.** `Ak::ListItem` rather than `Ak::Button`,
  because a row's `Activate` is still how it is chosen and a reader that hears "button, Meadow, 3 of 40"
  loses the order; the press still works through the same event (a design default, mirroring the table's
  rows, which are `Ak::Row` and still activate).
- **The list wraps a viewport that holds a column that holds the rows**, three entity levels; the node
  goes on the middle one because that is the one that scrolls. A reader sees one list, not three (design
  default).

## Pending index

- Nothing new: the corpus's craft stem and WCAG are the bar.
- Checked, no conflict: `Role::ListView` is spawned only by `list()`. A table and a tree are a
  `table_bundle`, which has no `ListView`, so their pooled rows keep `row_index` / `position_in_set` from
  the table's own node and this pass's list code never sees them.
