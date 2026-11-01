# Tree: a virtualized, expandable tree

Item: bevy_grimoire's request G-11 (#15), `tree()` with `tree_node(..)`: an outliner with thousands of nodes, Left and Right collapse and expand, Up and Down move, typing selects by prefix, Shift- or Cmd-click multi-selects, virtualized, a collapsed node keeps its children's state, deleting a selected node moves the selection to its sibling.

## Sources

- **Craft:** W3C WAI-ARIA Authoring Practices, *Tree View Pattern* (https://www.w3.org/WAI/ARIA/apg/patterns/treeview/): the keyboard model, selection versus focus, and what a tree exposes when not every node is present.
- **Shipping:** Unity UI Toolkit's `TreeView` / `MultiColumnTreeView` (https://docs.unity.com/en-us/engine/6000.6/manual/uitoolkits/uielements/uie-element-ref/uie-uxml-element-tree-view): the engine's shipped tree, a fixed item height, rows recycled and rebound by index, `selectionType`, `itemsChosen` on a double click, expanded state kept by item id.
- **Technical:** this crate's own table (`docs/research/table.md`: a pool of rows rebound as the scroll moves, selection and cursor by key), Bevy 0.20's `ScrollArea`, and the scikit-learn API paper in the corpus (`API design for machine learning software: experiences from the scikit-learn project`: few types, defaults for the rest).
- The corpus has nothing on trees or outliners (`hs distill search`: unrelated hits). Pending index below.

## Craft principle

The APG says what the arrows do, and the rule is the same for every tree: "`Right arrow`: When focus is on a closed node, opens the node; focus does not move. When focus is on a open node, moves focus to the first child node. When focus is on an end node, does nothing." and "`Left arrow`: When focus is on an open node, closes the node. When focus is on a child node that is also either an end node or a closed node, moves focus to its parent node. When focus is on a root node that is also either an end node or a closed node, does nothing." Up and Down move "to the next node that is focusable without opening or closing a node"; `End` moves "to the last node in the tree that is focusable without opening a node". Type-ahead "is recommended for all trees": "Type multiple characters in rapid succession: focus moves to the next node with a name that starts with the string of characters typed." Focus is not selection: "in multi-select trees ... the selected state is always independent of the focus. ... It is important that the visual design distinguish between items that are selected and the item that has focus." And a tree that does not hold every node must say where each is: "each node has aria-level, aria-setsize, and aria-posinset specified", so a row carries its depth.

## How shipping games do it

Unity's `TreeView` is the table's cousin: "`fixed-item-height` ... The height of a single item in the list, in pixels. This property must be set when using the `virtualizationMethod` is set to `FixedHeight`", "As the user scrolls through the list, the TreeView object will recycle elements created by the "makeItem" and invoke the "bindItem" callback to associate the element with the matching data item", every item has an id (`new TreeViewItemData<string>(itemIndex, ..., children)`, "`GetIdForIndex`"), the indent is its own element (`unity-tree-view__item-indents`) beside a toggle (`unity-tree-view__item-toggle`) and the content, `auto-expand` decides whether a new item starts open, `itemsChosen` is "invoked when the user double clicks an item", and `selectionType` is `Single` by default. So: fixed row height, a pool rebound by index, nodes with ids, an indent and a disclosure arrow in front of the content, open state per node.

## What great looks like

A node is one table row: a disclosure chevron one text size and a half wide (never narrower than a 24 px target), the label after it, an indent of one chevron per level, so a child's label sits one chevron's width right of its parent's. A closed node's chevron points right, an open one's down, an end node has none but keeps the room, so labels of siblings align whether or not they have children. Rows are the table's: one pitch high, quiet bands, the accent on the selected, the ring on the cursor only while focus is visible. The chevron is drawn in the row's ink (inverted on a selected row), so it is as legible as the text. Ten thousand nodes cost what ten do, and opening a node under the player's eye does not move the row they are looking at.

## Model adopted

- **A tree is the table's rows, one column, and nothing else.** `tree(nodes)` spawns a `Table` (no header) whose rows are the *visible* nodes in depth-first order; the table's pool, scroll, anchor, cursor, selection (`TableSelection`, `SelectMode`, `TableCursor`, by key) and keys (`table_by_input`) are unchanged, so a tree is virtualized by the code that already virtualizes a table. One path, no second pool.
- **`Tree` is the table's `RowSource`:** the nodes in one flat preorder list (key, label, depth, open), with the visible list kept beside it. `TreeNode::new(key, label)` builds a node (`.child(..)`, `.children(..)`, `.expanded()`), a key is the node's identity (selection and cursor stay with it; two nodes never share one). A game changes the tree through the table: `Table::source_mut::<Tree>()`, then `set_open(key, bool)`, `insert(parent, node)`, `remove(key)`, `set_nodes(..)`. Each open state is the node's own: collapsing a node never touches its descendants' state, so reopening shows them as they were.
- **`RowSource` gains two defaulted methods**, `outline(row) -> Outline` (`depth`, `open: Option<bool>`; a plain table's is flat, so nothing changes for it) and `hidden(key) -> bool` (a row that exists but is not listed, a node under a closed parent: the table keeps its selection when the rows change). The first column of a table whose source has an outline indents by depth and draws the chevron.
- **Keys** (the APG's, the table's already held): Up, Down, Home, End, the Page keys, Space, Ctrl+A, Enter and the pad as in the table. New: Right and Left as above (the D-pad too, Left and Right being the tree's alone, `navigate_by_input` leaves them), `*` opens every sibling of the cursor node, and typing selects by prefix: characters typed within one second of each other make a prefix, and the cursor goes to the next visible row (from the cursor, wrapping; from the one after it when the prefix is one character) whose label starts with it, case-insensitively. The cursor moves as an arrow does (it selects in `Single`).
- **Pointer:** a click on the chevron opens or closes the node and does nothing else (it does not select); a click elsewhere selects as in the table (Ctrl toggles, Shift ranges, in `Multi`); a double click is `RowActivated`. The chevron's box is a row high and 24 px or a text size and a half wide.
- **Events:** `TreeToggled { entity, key, open }` when the *player* opens or closes a node (a pointer, Left, Right, `*`); a game's `set_open` sends none. This is the hook for loading children lazily.
- **A node that hides the cursor:** closing a node while the cursor is on one of its descendants puts the cursor on the node (the APG: focus never rests on a node that is not shown). The selection keeps hidden nodes (state kept).
- **A removed node's heir:** `Tree::remove(key)` records, for every node it takes out, its heir: the next sibling, else the previous one, else the parent. A selected node, or the cursor, that is gone goes to its heir, with no `ValueChange` (a game's own write sends none).

## Parameters

| Constant | Value | Why |
| --- | --- | --- |
| Indent per level and chevron box | `max(24 px, 1.5 text sizes)` | follows `text_scale`; WCAG 2.5.8's 24 px at least |
| Type-ahead window | 1 s of real time between characters | the APG's "rapid succession"; Windows' list views use the same order |
| Open state | each node's own, default closed (`expanded()` opens it) | Unity's `auto-expand` defaults to false |
| Row height, pool, scroll | the table's | one pool |

## Validation targets

1. 10,000 nodes (open, 5 levels) spawn at most visible + 2 row entities; so with every node closed under one root; at 200% text.
2. Left on an open node closes it (one `TreeToggled`, rows under it gone); Right opens it; Right on an open node moves to the first child; Left on a child that is closed or an end node moves to the parent; Left on a closed root and Right on an end node move nothing.
3. Up, Down, Home, End step the visible rows only; End is the last *visible* row.
4. Typing "Ca" selects the next visible row starting "Ca"; a one-character prefix starts after the cursor row; a pause of more than a second starts the prefix over; a Ctrl chord types nothing.
5. A closed node keeps its descendants' open state and selection; closing a node that holds the cursor moves the cursor to it.
6. Removing a selected node moves the selection to its next sibling, else previous, else parent; removing a node with a selected descendant moves the selection to the removed node's heir.
7. A click on a chevron toggles, sends `TreeToggled` and selects nothing; Shift and Ctrl click select ranges and toggle in `Multi`.
8. Contrast: the label 4.5:1 and the chevron 3:1 on the rest, banded, hovered and selected rows in every look; the ring 3:1.
9. Nothing written while idle; opening a node does not move the row at the top of the view.

## Deviations

- Selection follows the cursor in `Single` (the table's rule, one of the APG's two models) and Ctrl moves without selecting; the same as the table. A design default.
- The APG's separate Shift+Space and Ctrl+Shift+Home/End range keys are not added (the table's Shift+arrow and Ctrl+A are); the APG lists them as optional.
- Rows are one height and one line (Unity's `FixedHeight`), as in the table; a label that is too long is cut. No horizontal scroll (a deep tree whose indent passes the width cuts its labels).
- Node data lives in the `Table`'s source: a game with its own scene hierarchy mirrors it into `Tree`. The alternative, a source trait of children callbacks, would not make 10,000 nodes cheaper and would double the API.

## Pending index

Nothing on trees, outliners or hierarchical lists in home-still. The APG tree view pattern and the Unity `TreeView` manual are worth indexing (`paper_search` finds papers, not these pages).
