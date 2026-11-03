# Accessibility of virtualized rows: a table and a tree row know their place, level and state

Researched 2026-11-03 (iteration 57). Backlog #1 of iterations 55 and 56 ("a tree row's `level` and `expanded`, and its Expand/Collapse; a radial's Increment/Decrement"), recorded as "Not done" in `src/access.rs` and as deviations in `accessibility.md` and `access-actions.md`. No filed issue is open that can be worked (#38 waits for the host's debug output). The gap: a table and a tree draw their rows from a pool of about a dozen entities rebound while scrolling, so a screen reader walking a 10,000-node outliner heard "tree item, Rope" with no depth, no "closed", no "row 5 of 10,110", and could not open a node it had focused.

## Sources

1. **Craft:** W3C WAI-ARIA Authoring Practices, *Tree View Pattern*, https://www.w3.org/WAI/ARIA/apg/patterns/treeview/ (read 2026-11-03), with the WAI-ARIA 1.2 definitions it points to.
2. **Shipping game:** Naughty Dog, *The Last of Us Part II: Accessibility Features Detailed*, https://www.naughtydog.com/blog/the_last_of_us_part_ii_accessibility_features_detailed (quoted in `access-actions.md` and `accessibility.md`): every menu is voiced, so a list the voice cannot place ("which one of how many") is a list the player cannot walk blind.
3. **Technical:** `accesskit` 0.24.1 (`Node::{level, position_in_set, size_of_set, row_index, row_count, column_count}`) and the platform adapters that consume it (`accesskit_consumer` 0.38, `accesskit_windows` 0.34), read from the vendored source; Bevy 0.20's `ActionRequest` path as in `access-actions.md`.

## Craft principle

APG, Tree View, *WAI-ARIA Roles, States, and Properties*: "Each element with role treeitem that serves as a parent node has aria-expanded set to false when the node is in a closed state and set to true when the node is in an open state. End nodes do not have the aria-expanded attribute because, if they were to have it, they would be incorrectly described to assistive technologies as parent nodes." And, exactly our case: "If the complete set of available nodes is not present in the DOM due to dynamic loading as the user moves focus in or scrolls the tree, each node has aria-level, aria-setsize, and aria-posinset specified." A virtualized tree is a tree whose complete set is not present, so level and place are required, not decoration, and an end node must carry no expanded state at all (a pooled entity that was a branch a frame ago must lose it).

## How shipping games do it

Naughty Dog: "our menus and other on-screen text can be voiced using the Text-To-Speech option." A voiced list is only usable if each line says where it is; every platform screen reader reads "item 5 of 10" from the node's set size and position, and a tree adds "level 2, collapsed". Where the host keeps the rows in a pool, that information has to come from the data (the `Tree`, the `Table`), never from the entities.

## Model adopted

- **A row's place is the data's, written each frame it differs.** `update_accessibility` already visits every `Role` entity and writes only a difference; a pooled row's `TableRow { table, index, key }` is rebound by `update_tables`, so the node follows it with no second path.
- **Tree row:** `level` = the node's depth (0-based: AccessKit counts from 0 and the adapters add the 1, `accesskit_windows` `level()` is `level + 1`), `position_in_set` = `TableRow::index` (the row's place among the rows shown, 0-based: `accesskit_windows` adds the 1), `expanded` = `Some(open)` for a node with children and **cleared** for an end node (APG), read from `Tree::outline_of(key)`. **The tree's `Role::Table` node carries `size_of_set` = the rows shown**, because `accesskit_consumer`'s `size_of_set_from_container` reads the set's size from the nearest ancestor that has one, never from the item: so the set a reader announces is the flat list of rows shown, "row 5 of 10,110", and `level` and `expanded` give the depth. (A per-parent sibling count cannot be said in this AccessKit: one container holds one size.)
- **Table:** the `Role::Table` node carries `row_count` (every row, not the pool) and `column_count`; each row `row_index` (0-based), so a reader says "row 5 of 10,000".
- **Expand and Collapse on a tree row** are the table's own toggle (`tree::toggle`), the one that Left, Right and the chevron use: a `TreeToggled` for the player's change, nothing for an end node or the state already held. A branch row declares exactly Expand and Collapse (B2 of `access-actions.md`: advertise what is handled); an end node and a row of a disabled tree declare none and the handler ignores a request for them.
- **A radial's Increment and Decrement** are its arrow keys (`radial_step`, wrapping, one non-final `ValueChange<usize>`); it already declared Focus and Click (Click is Bevy's `Activate`: the final choice), so it joins the choice's set.
- **Nothing runs without a screen reader** (the system returns without `AccessibilityRequested`) and a settled tree, table or radial marks no node changed (tested).

## Parameters

None numeric. Which widget declares which: tree branch row: Expand, Collapse; end node, table row: none; radial: Focus, Click, Increment, Decrement. Design defaults mirroring the ARIA roles (treeitem, row, spinbutton).

## Validation targets

- A tree of A { B { D }, C } and E with B closed: A level 0 open position 0, B level 1 closed position 1, C level 1 no expanded state position 2, E level 0 position 3, and the tree says 4 rows.
- A pooled row rebound from a branch to an end node says no `expanded` (mutation: dropping the clear fails).
- Expand on B sends one `TreeToggled` and D appears at level 2, position 2, the set 5; Expand on an open node, Collapse on an end node or a disabled tree send nothing.
- A 40-row table says 40 rows (not the pool's count) and 2 columns, and each row its own index.
- A radial steps and wraps like Left and Right, one report a step (mutation: a fixed direction fails).
- A settled interface holding a tree, a table and a radial marks no node changed.

## Deviations

None from a source. Design defaults: `size_of_set` is the rows shown on the container, not each parent's children (AccessKit's adapters read it from the container); `ReplaceSelectedText`, `Scroll*` and a table cell's `column_index` stay unoffered.

## What great looks like

Not visual. A narrator user on an outliner hears "Scene, collapsed, level 1, row 1 of 4"; presses the platform's expand key and hears "Scene, expanded"; arrows down and hears "Props, collapsed, level 2, row 2 of 9". A weapon wheel answers the platform's Increment with the next item's name.

## Pending index

home-still has no stem on tree accessibility or screen-reader semantics; the APG and WAI-ARIA 1.2 pages and the AccessKit sources were read directly, not from the index. The craft and shipping-game stems of `accessibility.md` and `access-actions.md` are reused.
