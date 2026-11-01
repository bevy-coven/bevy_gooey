# Trees

`tree(nodes)` is nodes that open and close, and it is **virtualized** like the [table](tables.md) it is built on: ten thousand nodes cost about a dozen entities. Use it for an outliner of a scene, a file browser, a quest log, a skill tree's list view: anything with a parent and children.

```rust
fn outliner(mut commands: Commands) {
    commands.spawn((
        panel(),
        children![(
            tree([
                TreeNode::new(1, "World").expanded().children([
                    TreeNode::new(2, "Camera"),
                    TreeNode::new(3, "Lights").child(TreeNode::new(4, "Sun")),
                ]),
                TreeNode::new(5, "Terrain"),
            ]),
            SelectMode::Multi,
            observe(|toggled: On<TreeToggled>| info!("{} is open: {}", toggled.key, toggled.open)),
            observe(|selected: On<ValueChange<TableSelection>>| {
                info!("{} nodes selected", selected.value.len());
            }),
            observe(|opened: On<RowActivated>| info!("opened node {}", opened.key)),
        )],
    ));
}
```

![The tree example: an outliner of ten thousand nodes; a click on a chevron opens Ashfall Docks, Right and Left open and close nodes, Left on a closed node goes to its parent, typing "ha" goes to the next node that starts with it, a star opens every sibling, End goes to the last node shown and the readout counts the nodes shown](https://raw.githubusercontent.com/bevy-coven/bevy_gooey/main/docs/media/tree.gif)

- **A tree is the table's rows in one column.** The tree spawns a `Table` with no header whose rows are the nodes that are *shown* (every parent open), in depth-first order, each indented one level more than its parent, with a chevron in front of every node that has children (pointing right when closed, down when open). The pool, the scroll and its rail, the cursor, the selection by key, `SelectMode`, `ValueChange<TableSelection>`, `RowActivated` and `TableRows` are the table's: [tables](tables.md) says how they work.
- **A node is a `TreeNode`.** `TreeNode::new(key, label)` is a closed node with no children; `.expanded()` opens it, `.child(node)` and `.children(nodes)` add children. The `key` is the node's identity (a `u64` of yours): the selection, the cursor and the player's place are kept by key, so they stay with a node as others come and go. A key used twice leaves the second node out, with a warning.
- **State lives in the nodes.** Each node has its own open flag, so closing a node never touches the state below it: open it again and its children are as the player left them. A selected node under a closed one stays selected. Read what is open with `Tree::is_open(key)` and `Tree::open_keys()` (save it); write it with `Tree::set_open(key, open)`. Your own `set_open` sends no event; `TreeToggled { key, open }` is the player's: a click on a chevron, Left, Right or `*`, and the place to load a node's children the first time it opens.
- **Changing the tree.** Through the table: `table.source_mut::<Tree>()` gives `insert(parent, node)` (the last child of `parent`, a root for `None`), `remove(key)` (the node and everything under it) and `set_nodes(..)` (everything). The table keeps the row that was at the top of the view where it was, by key, so nodes opened above the player's place do not move what they are reading. A cursor on a node that gets hidden moves to the node that hides it. **Deleting a selected node moves the selection to its heir**: the next sibling, else the one before, else the parent (the same for the cursor, and for a selected node inside a removed subtree); that is your change, so no `ValueChange` is sent.
- **Keys** (the W3C's tree view pattern). The tree is one tab stop with a cursor row. Up and Down move over the nodes shown, Home and End go to the first and last shown, the Page keys a page, Space selects, Enter or a double click is `RowActivated`; all the table's. **Right** opens a closed node, and on an open node goes to its first child; **Left** closes an open node, and on a closed node or one with no children goes to the parent (nothing on a closed root); `*` opens every sibling of the cursor node. **Typing** goes to the next shown node whose label starts with what was typed, without regard to case: characters typed within a second of each other make one prefix ("ca" finds "Camera"), a pause starts over, and the same letter again goes on to the next node starting with it. A Ctrl, Alt or Super chord types nothing. On a pad, the D-pad's Left and Right do what the keys do, and Up, Down and South are the table's. Left and Right never move focus off a tree.
- **Pointer.** A click on the chevron opens or closes its node and selects nothing; a click on the label selects (in `Multi`, Ctrl toggles and Shift takes the range over the nodes shown); a double click is `RowActivated`. The chevron's box is a level wide (a text size and a half, never under 24 px) and a row high.
- **Look.** The table's: no fill at rest, a quiet band on every second row, the hovered fill, the accent with inverted text while selected and the ring on the cursor row. A chevron is drawn in the row's ink, so it is inverted on a selected row and checked at 3:1 in all four looks. Indent and chevron follow the player's text size.
- **Where it stops.** One line a node, cut at the tree's edge; no horizontal scroll (a very deep tree cuts its labels); no drag to reorder yet; no checkbox or icon in a row.
