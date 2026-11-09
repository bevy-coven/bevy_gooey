# A sortable column's header, as a node a screen reader can sort with

Researched 2026-11-09 (iteration 88). No issue is filed. The pass's own pick — the follow-up iteration 87's
note found and could not fit.

The problem, measured. `Role::TableHeaderCell` was `Spec::Group` in `access.rs`, which makes it a
container in the accessibility tree: it had **no node of its own**, so a screen reader walked past it
into the rows and a keyboard or pointer player could sort while a screen reader could not. The cell is
pressed to sort (`header_on_activate` on its `Activate`), so every other input path worked.

## Sources

1. *Future design of accessibility in games: A design vocabulary* — 2019, IJHCS,
   doi:10.1016/j.ijhcs.2019.06.010 (craft): its argument is that what a game offers must be operable by
   every input it advertises, which is the crate's own rule from that paper.
2. WCAG 2.2 SC 4.1.2, Name, Role, Value (technical): "states, properties, and values that can be set by
   the user can be programmatically set". A sort order is a value; the header must carry it and offer
   the way to change it.
3. AccessKit 0.24, `lib.rs:517` and `:2142` (technical): `SortDirection { Ascending, Descending, Other }`
   and the `sort_direction` property with `set_`/`clear_`; `Role::ColumnHeader` is the role a platform
   reads a sort from.
4. Rune's table (the owner's reference, read not copied): a sortable header is a button-like cell that
   reports the column's sort direction, and pressing it is the same action as clicking.

## Craft principle

A control that can be pressed must say it can be pressed, and a state a player can set must be a state
the tree reports. A header that is only a container is a sorting control that exists for three inputs
and not the fourth.

## How shipping games do it

Rune registers a sortable header as a header node with its direction and a press action, and its
visible chevron and that state are drawn from one value. The crate already draws the chevron from
`TableSort`; the node now reads the same component.

## Model adopted

- **The cell is a `Spec::Node`**, described as `Ak::ColumnHeader` with its own words (the column's
  title) — the name a host's `AccessibleLabel` can still replace.
- **Its sort is the drawn sort.** `Facts` gains the `TableHeaderCell` (its table and column), and
  `describe` reads that table's `TableSort`: `Ascending` when this column is the one sorted by and not
  descending, `Descending` when it is and descending, and **cleared** otherwise. `Wanted::sort` is
  `Option<Option<..>>` so "no header" and "this column is not the sort" differ, and `says` compares it —
  without that comparison the node was never rewritten when only the sort changed (measured: the test
  failed with `None` against `Some(Ascending)`).
- **No new action arm.** A header cell is already Bevy's `Button`, so `actions_of` gave it Focus and
  Click before this pass and a Click already reached `Activate` — the sort worked. A `COLUMN` arm was
  written and then removed: it was redundant, and the mutation that deleted it left every test green,
  which is what proved it. The defect was the *node*, not the action.

## Parameters

| Name | Value | Where from |
|---|---|---|
| role | `Ak::ColumnHeader` | WCAG 4.1.2, AccessKit |
| actions | Focus, Click | the cell's own `Activate` |
| sort state | the table's `TableSort`, cleared when not this column | `update_tables`'s chevron |
| name | the cell's words, overridable by `AccessibleLabel` | the crate's naming rule |

## Validation targets

- A header cell's node is `Ak::ColumnHeader`, named with its own words, and declares Click.
- A Click sorts by that column, and the node then reads `Descending` after a second.
- A column that is not the sort reads no direction (cleared, not stale).
- Mutation-checked: removing the `says` comparison fails the direction assertion (it read `None`
  against `Some(Ascending)` before that comparison was added); removing the `sort` wiring fails on the
  role and the Click.

## Deviations

- **No `Ak::SortDirection::Other`.** A table sorts by this crate's own `natural_cmp`, which is a normal
  ascending/descending; `Other` would claim a third order there is not (design default).
- **A non-sortable column still declares Click**, unchanged from before: it is a Bevy `Button` and its
  observer refuses. A reader that presses hears nothing happen, as a pointer player does (design
  default).

## Pending index

- Nothing new: the corpus had the accessibility-vocabulary stem; WCAG 4.1.2 and AccessKit are the
  technical bar.
- **Follow-up found, not built:** a list's rows and a tree's nodes have no nodes at all — Bevy's
  `ScrollArea` carries none — so a screen reader cannot read a scrolling list of ten thousand rows.
