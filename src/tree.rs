//! The tree: nodes inside a [`crate::table`]'s pool. [`crate::tree`] spawns a table with no header
//! whose rows are the *shown* nodes of a [`Tree`] in depth-first order, so the pool, the scroll,
//! the cursor and the selection are the table's ([`crate::update_tables`],
//! [`crate::table_by_input`]); this module adds what only a tree has: the indent and the chevron
//! of a row ([`spawn_lead`], [`bind_lead`]), opening and closing ([`TreeToggled`],
//! [`tree_by_input`]), a removed node's heir ([`update_trees`]) and type-ahead.

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::input::ButtonInput;
use bevy::input::ButtonState;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::events::PointerClick;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::observe;

use crate::clicks::DoublePress;
use crate::model::{tree_ends, tree_heir, tree_parent, tree_visible};
use crate::nav::show_focus;
use crate::skin::Role;
use crate::table::{
    Chosen, Pick, RowChange, RowSource, SelectMode, Table, TableCache, TableCursor, TableRow,
    TableSelection, pick,
};
use crate::widgets::CHEVRON_INK_SHIFT;

/// One node of a [`crate::tree`] and the nodes under it. A node's `key` is its identity: the
/// selection, the cursor and the player's place are kept by key, so they stay with a node while
/// others come and go. Two nodes of one tree never share one (a repeated key is left out, with a
/// warning).
///
/// ```
/// # use bevy_gooey::TreeNode;
/// let world = TreeNode::new(1, "World")
///     .expanded()
///     .child(TreeNode::new(2, "Camera"))
///     .child(TreeNode::new(3, "Lights").children([TreeNode::new(4, "Sun")]));
/// ```
#[derive(Clone, Debug)]
pub struct TreeNode {
    key: u64,
    label: String,
    open: bool,
    children: Vec<TreeNode>,
}

impl TreeNode {
    /// A closed node with no children.
    pub fn new(key: u64, label: impl Into<String>) -> Self {
        Self {
            key,
            label: label.into(),
            open: false,
            children: Vec::new(),
        }
    }

    /// This node, open: its children are shown. Each node keeps its own state, so opening a node
    /// shows its children as the player left them.
    pub fn expanded(mut self) -> Self {
        self.open = true;
        self
    }

    /// This node with one more child, last.
    pub fn child(mut self, node: TreeNode) -> Self {
        self.children.push(node);
        self
    }

    /// This node with `nodes` as more children, last.
    pub fn children(mut self, nodes: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children.extend(nodes);
        self
    }
}

/// What a row of a tree draws before its text: how far in, and whether it has a chevron.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Outline {
    pub(crate) depth: usize,
    /// `Some(open)` for a node with children, `None` for an end node.
    pub(crate) open: Option<bool>,
}

/// A [`crate::tree`]'s nodes: the [`RowSource`] of its [`Table`]. Change the tree through the
/// table (`table.source_mut::<Tree>()`), and the table keeps the row the player was looking at
/// where it was.
///
/// Nodes are stored in one list in depth-first order with each node's own open state, so closing
/// a node never touches the state below it, and 10,000 nodes cost a list, not 10,000 entities.
/// Only the *shown* nodes (every ancestor open) are rows.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn outliner(mut tables: Query<&mut Table>) {
///     for mut table in &mut tables {
///         if let Some(tree) = table.source_mut::<Tree>() {
///             tree.insert(Some(3), TreeNode::new(5, "Moon"));
///             tree.relabel(3, "Lights (1 new)");
///             tree.set_open(3, true);
///             tree.remove(2);
///         }
///     }
/// }
/// ```
#[derive(Clone, Debug, Default)]
pub struct Tree {
    keys: Vec<u64>,
    labels: Vec<String>,
    depths: Vec<u32>,
    open: Vec<bool>,
    /// Where each node's subtree ends ([`tree_ends`]).
    ends: Vec<usize>,
    /// The nodes shown, in order: indexes into the lists above.
    shown: Vec<usize>,
    by_key: HashMap<u64, usize>,
    /// `(gone, heir)` for every node `remove` took out, until [`update_trees`] moves the
    /// selection and cursor.
    heirs: Vec<(u64, u64)>,
    /// A number that is new with every change, for [`RowSource::revision`].
    revision: u64,
    /// What the change that made `revision` did, when it was one a table can patch in.
    last: Option<Delta>,
}

/// What one change did to the rows shown, and the revisions it came between.
#[derive(Clone, Copy, Debug)]
struct Delta {
    from: u64,
    to: u64,
    change: RowChange,
}

/// The one counter every tree takes its revisions from, so a table that is handed another tree
/// never mistakes it for the one it drew.
static REVISIONS: AtomicU64 = AtomicU64::new(1);

/// One node on its way into a [`Tree`].
struct Flat {
    key: u64,
    label: String,
    depth: u32,
    open: bool,
}

/// `nodes` and everything under them, depth-first, `base` levels in. A key already in `taken` (or
/// seen earlier here) leaves its node and its subtree out.
fn flatten(
    nodes: impl IntoIterator<Item = TreeNode>,
    base: u32,
    taken: &HashMap<u64, usize>,
) -> Vec<Flat> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut stack: Vec<(TreeNode, u32)> = nodes.into_iter().map(|n| (n, base)).collect();
    stack.reverse();
    while let Some((mut node, depth)) = stack.pop() {
        if taken.contains_key(&node.key) || !seen.insert(node.key) {
            warn!(
                "tree: key {} is used twice; the second node is left out",
                node.key
            );
            continue;
        }
        let children = std::mem::take(&mut node.children);
        out.push(Flat {
            key: node.key,
            label: node.label,
            depth,
            open: node.open,
        });
        stack.extend(children.into_iter().rev().map(|c| (c, depth + 1)));
    }
    out
}

impl Tree {
    /// A tree of `nodes` (its roots).
    pub fn new(nodes: impl IntoIterator<Item = TreeNode>) -> Self {
        let mut tree = Self::default();
        tree.set_nodes(nodes);
        tree
    }

    /// Replaces every node. The selection keeps the keys that are still there.
    pub fn set_nodes(&mut self, nodes: impl IntoIterator<Item = TreeNode>) {
        let flat = flatten(nodes, 0, &HashMap::new());
        self.keys = flat.iter().map(|f| f.key).collect();
        self.depths = flat.iter().map(|f| f.depth).collect();
        self.open = flat.iter().map(|f| f.open).collect();
        self.labels = flat.into_iter().map(|f| f.label).collect();
        self.relink();
        self.touch(None);
    }

    /// Adds `node` and its children as the last child of the node `parent` (a root when `None`).
    /// `false` when there is no such parent or the node's key is taken.
    ///
    /// This is the append, and it touches what the new nodes touch: the nodes' own lists grow, the
    /// ends of the parent and its ancestors and of every node after the new ones move up by their
    /// count, and the rows shown after them shift. Appending a last root, as a history list does,
    /// moves nothing but the new rows; nothing is rebuilt, and no key is hashed again but the new
    /// ones and those of the nodes after them.
    pub fn insert(&mut self, parent: Option<u64>, node: TreeNode) -> bool {
        let (parent_at, at, depth) = match parent {
            None => (None, self.keys.len(), 0),
            Some(key) => match self.by_key.get(&key) {
                Some(&i) => (Some(i), self.ends[i], self.depths[i] + 1),
                None => return false,
            },
        };
        let flat = flatten([node], depth, &self.by_key);
        if flat.is_empty() {
            return false;
        }
        let count = flat.len();
        let depths: Vec<u32> = flat.iter().map(|f| f.depth).collect();
        let open: Vec<bool> = flat.iter().map(|f| f.open).collect();
        let ends: Vec<usize> = tree_ends(&depths).into_iter().map(|e| e + at).collect();
        // The new nodes are shown when the parent is shown and open, and then as far as their own
        // open states go.
        let visible =
            parent_at.is_none_or(|p| self.open[p] && self.shown.binary_search(&p).is_ok());
        let new_shown: Vec<usize> = if visible {
            tree_visible(&ends, &open)
                .into_iter()
                .map(|i| i + at)
                .collect()
        } else {
            Vec::new()
        };

        // What was there moves: every node from `at` on lies `count` further, and so do its end
        // and its row; the parent and its ancestors (the only nodes before `at` whose subtree
        // takes the new ones) end `count` later.
        for end in &mut self.ends[at..] {
            *end += count;
        }
        if let Some(p) = parent_at {
            let mut above = self.depths[p] + 1;
            for j in (0..=p).rev() {
                if self.depths[j] < above {
                    self.ends[j] += count;
                    above = self.depths[j];
                    if above == 0 {
                        break;
                    }
                }
            }
        }
        for key in &self.keys[at..] {
            if let Some(index) = self.by_key.get_mut(key) {
                *index += count;
            }
        }
        let row = self.shown.partition_point(|&s| s < at);
        for shown in &mut self.shown[row..] {
            *shown += count;
        }

        let shown_count = new_shown.len();
        self.shown.splice(row..row, new_shown);
        self.ends.splice(at..at, ends);
        for (i, f) in flat.iter().enumerate() {
            self.by_key.insert(f.key, at + i);
        }
        self.keys.splice(at..at, flat.iter().map(|f| f.key));
        self.depths.splice(at..at, depths);
        self.open.splice(at..at, open);
        self.labels
            .splice(at..at, flat.into_iter().map(|f| f.label));
        self.touch(Some(RowChange::Inserted {
            row,
            count: shown_count,
        }));
        true
    }

    /// Gives the node `key` another label: one text changes, and with it one row if the node is
    /// shown. `true` when the label changed; `false` when there is no such node or it already
    /// reads so. The place of the node, its open state, the selection and the cursor are as they
    /// were.
    pub fn relabel(&mut self, key: u64, label: impl Into<String>) -> bool {
        let Some(&i) = self.by_key.get(&key) else {
            return false;
        };
        let label = label.into();
        if self.labels[i] == label {
            return false;
        }
        self.labels[i] = label;
        self.touch(Some(RowChange::Texts));
        true
    }

    /// Takes out the node `key` and everything under it. A selected node, or the cursor, that was
    /// in it goes to its heir: the next sibling, else the one before, else the parent. `false`
    /// when there is no such node.
    pub fn remove(&mut self, key: u64) -> bool {
        let Some(&i) = self.by_key.get(&key) else {
            return false;
        };
        let end = self.ends[i];
        if let Some(heir) = tree_heir(&self.depths, &self.ends, i).map(|h| self.keys[h]) {
            self.heirs
                .extend(self.keys[i..end].iter().map(|&gone| (gone, heir)));
        }
        self.keys.drain(i..end);
        self.depths.drain(i..end);
        self.open.drain(i..end);
        self.labels.drain(i..end);
        self.relink();
        self.touch(None);
        true
    }

    /// Opens or closes the node `key`. `true` when its state changed; this sends no
    /// [`TreeToggled`] (that is for the player's own).
    pub fn set_open(&mut self, key: u64, open: bool) -> bool {
        let Some(&i) = self.by_key.get(&key) else {
            return false;
        };
        if self.open[i] == open {
            return false;
        }
        self.open[i] = open;
        self.shown = tree_visible(&self.ends, &self.open);
        self.touch(None);
        true
    }

    /// Takes a new revision for a change, and keeps what the change did when a table can patch it
    /// in (`None` when only a rebuild will do).
    fn touch(&mut self, change: Option<RowChange>) {
        let from = self.revision;
        self.revision = REVISIONS.fetch_add(1, Ordering::Relaxed);
        self.last = change.map(|change| Delta {
            from,
            to: self.revision,
            change,
        });
    }

    /// Whether the node `key` is open; `None` when there is no such node. An end node reports its
    /// own state too (it is shown open if children are added).
    pub fn is_open(&self, key: u64) -> Option<bool> {
        self.by_key.get(&key).map(|&i| self.open[i])
    }

    /// The keys of every open node, in tree order: what a game saves to bring the tree back as
    /// the player left it.
    pub fn open_keys(&self) -> Vec<u64> {
        (0..self.keys.len())
            .filter(|&i| self.open[i])
            .map(|i| self.keys[i])
            .collect()
    }

    /// Whether there is a node `key`, shown or not.
    pub fn contains(&self, key: u64) -> bool {
        self.by_key.contains_key(&key)
    }

    /// The label of the node `key`.
    pub fn label(&self, key: u64) -> Option<&str> {
        self.by_key.get(&key).map(|&i| self.labels[i].as_str())
    }

    /// How many nodes there are, shown or not. ([`RowSource::len`] is how many are shown.)
    pub fn node_count(&self) -> usize {
        self.keys.len()
    }

    fn relink(&mut self) {
        self.ends = tree_ends(&self.depths);
        self.shown = tree_visible(&self.ends, &self.open);
        self.by_key = self.keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
    }

    pub(crate) fn outline(&self, row: usize) -> Outline {
        self.node_outline(self.shown[row])
    }

    /// The outline of the node `key`, shown or not.
    pub(crate) fn outline_of(&self, key: u64) -> Option<Outline> {
        self.by_key.get(&key).map(|&i| self.node_outline(i))
    }

    fn node_outline(&self, i: usize) -> Outline {
        Outline {
            depth: self.depths[i] as usize,
            open: (self.ends[i] > i + 1).then_some(self.open[i]),
        }
    }

    /// The nearest node above `key` that is shown: where the cursor goes when its own node is
    /// hidden by a closed ancestor. `None` when `key` is unknown or has none.
    pub(crate) fn shown_ancestor(&self, key: u64) -> Option<u64> {
        let mut at = *self.by_key.get(&key)?;
        while let Some(parent) = tree_parent(&self.depths, at) {
            if self.shown.binary_search(&parent).is_ok() {
                return Some(self.keys[parent]);
            }
            at = parent;
        }
        None
    }

    /// The key of the parent of `key`.
    fn parent_key(&self, key: u64) -> Option<u64> {
        let i = *self.by_key.get(&key)?;
        tree_parent(&self.depths, i).map(|p| self.keys[p])
    }

    /// Whether `key` has children.
    fn is_branch(&self, key: u64) -> bool {
        self.by_key.get(&key).is_some_and(|&i| self.ends[i] > i + 1)
    }

    /// The keys of the nodes that share `key`'s parent, itself included.
    fn siblings(&self, key: u64) -> Vec<u64> {
        let Some(&i) = self.by_key.get(&key) else {
            return Vec::new();
        };
        let (from, to) = match tree_parent(&self.depths, i) {
            Some(p) => (p + 1, self.ends[p]),
            None => (0, self.keys.len()),
        };
        let mut out = Vec::new();
        let mut at = from;
        while at < to {
            out.push(self.keys[at]);
            at = self.ends[at];
        }
        out
    }

    pub(crate) fn take_heirs(&mut self) -> Vec<(u64, u64)> {
        std::mem::take(&mut self.heirs)
    }
}

impl RowSource for Tree {
    fn len(&self) -> usize {
        self.shown.len()
    }

    fn cell(&self, row: usize, _column: usize) -> Cow<'_, str> {
        Cow::Borrowed(self.labels[self.shown[row]].as_str())
    }

    fn key(&self, row: usize) -> u64 {
        self.keys[self.shown[row]]
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn change_since(&self, seen: u64) -> Option<RowChange> {
        if seen == self.revision {
            return Some(RowChange::Unchanged);
        }
        self.last
            .filter(|delta| delta.from == seen && delta.to == self.revision)
            .map(|delta| delta.change)
    }
}

/// A [`crate::tree`]'s root: its keys are the tree's ([`tree_by_input`]), and Left and Right never
/// move focus off it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TreeView;

/// The player opened or closed a node of a [`crate::tree`]: a click on its chevron, Left, Right
/// or `*`. A change the game makes with [`Tree::set_open`] sends none. The place to load a node's
/// children the first time it opens.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn opened(toggled: On<TreeToggled>) {
///     info!("node {} is now {}", toggled.key, if toggled.open { "open" } else { "closed" });
/// }
/// ```
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct TreeToggled {
    /// The tree.
    #[event_target]
    pub entity: Entity,
    /// The node's key.
    pub key: u64,
    /// Whether it is open now.
    pub open: bool,
}

/// The parts a row draws before its text, in the first cell: the box that is the indent and the
/// click target (a row high) and the two chevrons in it, one shown at most.
pub(crate) struct Lead {
    twisty: Entity,
    closed: Entity,
    open: Entity,
}

/// Spawns a row's indent and chevrons in `cell`.
pub(crate) fn spawn_lead(
    commands: &mut Commands,
    cell: Entity,
    outline: Outline,
    indent: f32,
) -> Lead {
    let twisty = commands
        .spawn((
            Node {
                margin: UiRect::left(Val::Px(outline.depth as f32 * indent)),
                justify_content: JustifyContent::Center,
                align_self: AlignSelf::Stretch,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Role::TreeTwisty,
            observe(twisty_on_click),
            observe(twisty_on_double_press),
            ChildOf(cell),
        ))
        .id();
    let mut chevron = |role, at: Vec2, shown: bool| {
        commands
            .spawn((
                Node {
                    display: if shown { Display::Flex } else { Display::None },
                    ..default()
                },
                UiTransform {
                    translation: Val2::percent(at.x, at.y),
                    rotation: Rot2::degrees(45.0),
                    ..default()
                },
                role,
                Pickable::IGNORE,
                ChildOf(twisty),
            ))
            .id()
    };
    // A closed node points right, an open one down: the same turned square as a choice's arrow
    // and a header's sort mark, shifted to put its ink in the middle.
    let closed = chevron(
        Role::TreeClosed,
        Vec2::new(-CHEVRON_INK_SHIFT, 0.0),
        outline.open == Some(false),
    );
    let open = chevron(
        Role::TreeOpen,
        Vec2::new(0.0, -CHEVRON_INK_SHIFT),
        outline.open == Some(true),
    );
    Lead {
        twisty,
        closed,
        open,
    }
}

/// Puts a row's indent and chevron where its node says, writing only a difference.
pub(crate) fn bind_lead(lead: &Lead, outline: Outline, indent: f32, nodes: &mut Query<&mut Node>) {
    if let Ok(mut node) = nodes.get_mut(lead.twisty) {
        let left = Val::Px(outline.depth as f32 * indent);
        if node.margin.left != left {
            node.margin.left = left;
        }
    }
    for (entity, shown) in [
        (lead.closed, outline.open == Some(false)),
        (lead.open, outline.open == Some(true)),
    ] {
        if let Ok(mut node) = nodes.get_mut(entity) {
            let want = if shown { Display::Flex } else { Display::None };
            if node.display != want {
                node.display = want;
            }
        }
    }
}

/// Opens or closes `key` for the player and says so; nothing for a node with no children or that
/// is already so.
pub(crate) fn toggle(
    entity: Entity,
    table: &mut Mut<Table>,
    key: u64,
    open: Option<bool>,
    commands: &mut Commands,
) {
    let Some(tree) = table.source::<Tree>() else {
        return;
    };
    let Some(was) = tree.is_open(key).filter(|_| tree.is_branch(key)) else {
        return;
    };
    let now = open.unwrap_or(!was);
    if now == was {
        return;
    }
    if let Some(tree) = table.source_mut::<Tree>() {
        tree.set_open(key, now);
    }
    commands.trigger(TreeToggled {
        entity,
        key,
        open: now,
    });
}

/// Two quick clicks on a chevron open and close its node and do not open the row: the double
/// press stops at the chevron, as each click does.
fn twisty_on_double_press(mut double: On<DoublePress>) {
    double.propagate(false);
}

/// A click on a chevron opens or closes its node, and selects nothing.
fn twisty_on_click(
    mut click: On<PointerClick>,
    parents: Query<&ChildOf>,
    rows: Query<&TableRow>,
    mut trees: Query<&mut Table, (With<TreeView>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    click.propagate(false);
    if click.button != PointerButton::Primary {
        return;
    }
    let Some(row) = parents
        .get(click.entity)
        .ok()
        .and_then(|cell| parents.get(cell.parent()).ok())
        .and_then(|row| rows.get(row.parent()).ok())
    else {
        return;
    };
    let Ok(mut table) = trees.get_mut(row.table) else {
        return;
    };
    toggle(row.table, &mut table, row.key, None, &mut commands);
}

/// Moves what a removed node held to its heir ([`Tree::remove`]): a selected node, the cursor and
/// the range anchor that were in the removed subtree go to the node that takes its place. Sends no
/// `ValueChange` (a game's own write sends none). Runs in [`crate::GooeySystems::Style`], before
/// [`crate::update_tables`].
pub fn update_trees(
    mut trees: Query<
        (
            &mut Table,
            &mut TableSelection,
            &mut TableCursor,
            &mut TableCache,
        ),
        With<TreeView>,
    >,
) {
    for (mut table, mut selection, mut cursor, mut cache) in &mut trees {
        if table.source::<Tree>().is_none_or(|t| t.heirs.is_empty()) {
            continue;
        }
        // The table is already marked changed by whoever removed the node.
        let heirs = table
            .bypass_change_detection()
            .source_mut::<Tree>()
            .map(Tree::take_heirs)
            .unwrap_or_default();
        for (gone, heir) in heirs {
            if selection.0.contains(&gone) {
                let mut next: BTreeSet<u64> = selection.0.clone();
                next.remove(&gone);
                next.insert(heir);
                selection.0 = next;
            }
            if cursor.0 == Some(gone) {
                cursor.0 = Some(heir);
            }
            if cache.anchor == Some(gone) {
                cache.anchor = Some(heir);
            }
        }
    }
}

/// What [`tree_by_input`] moves.
type Typed = (
    &'static mut Table,
    &'static SelectMode,
    &'static mut TableSelection,
    &'static mut TableCursor,
    &'static mut TableCache,
);

/// What a tree key does to the cursor node.
enum Act {
    Open(u64),
    Close(u64),
    Goto(usize),
}

/// The keys a focused tree takes beyond the table's (Up, Down, the Page keys, Home, End, Space,
/// Ctrl+A, Enter and the pad's South are [`crate::table_by_input`]'s): Right opens a closed node,
/// or goes to the first child of an open one; Left closes an open node, or goes to the parent of
/// a closed one or an end node (the D-pad's Left and Right too); `*` opens every sibling of the
/// cursor node (typing a node's name is the table's type-select, `crate::table_by_input`). Shows
/// focus. Runs in [`crate::GooeySystems::Navigate`], after
/// [`crate::navigate_by_input`], which leaves Left and Right to a tree.
#[allow(clippy::too_many_arguments)]
pub fn tree_by_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut presses: MessageReader<KeyboardInput>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    gamepads: Query<&Gamepad>,
    mut trees: Query<Typed, (With<TreeView>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    let typed: Vec<KeyboardInput> = presses
        .read()
        .filter(|p| p.state == ButtonState::Pressed)
        .cloned()
        .collect();
    let Some(entity) = focus.get().filter(|&f| trees.contains(f)) else {
        return;
    };
    let Ok((mut table, mode, mut selection, mut cursor, mut cache)) = trees.get_mut(entity) else {
        return;
    };
    let held = |a, b| keys.pressed(a) || keys.pressed(b);
    let plain = !held(KeyCode::ControlLeft, KeyCode::ControlRight)
        && !held(KeyCode::AltLeft, KeyCode::AltRight)
        && !held(KeyCode::SuperLeft, KeyCode::SuperRight);
    let pad = |button| gamepads.iter().any(|g| g.just_pressed(button));

    let mut sideways: Vec<i8> = Vec::new();
    let mut star = false;
    for press in &typed {
        if !plain {
            continue;
        }
        match (&press.key_code, &press.logical_key) {
            (KeyCode::ArrowLeft, _) => sideways.push(-1),
            (KeyCode::ArrowRight, _) => sideways.push(1),
            (_, Key::Character(c)) if c.as_str() == "*" => star = true,
            _ => {}
        }
    }
    if pad(GamepadButton::DPadLeft) {
        sideways.push(-1);
    }
    if pad(GamepadButton::DPadRight) {
        sideways.push(1);
    }
    if sideways.is_empty() && !star {
        return;
    }
    show_focus(&mut shown);

    for direction in sideways {
        let act = {
            let (Some(tree), Some(key)) = (table.source::<Tree>(), cursor.0) else {
                break;
            };
            let branch = tree.is_branch(key);
            let open = tree.is_open(key).unwrap_or(false);
            let at = cache.index_of(Some(key));
            if direction > 0 {
                match (branch, open) {
                    (true, false) => Some(Act::Open(key)),
                    (true, true) => at
                        .map(|i| i + 1)
                        .filter(|&i| i < cache.keys.len())
                        .map(Act::Goto),
                    _ => None,
                }
            } else if branch && open {
                Some(Act::Close(key))
            } else {
                tree.parent_key(key)
                    .and_then(|parent| cache.index_of(Some(parent)))
                    .map(Act::Goto)
            }
        };
        match act {
            Some(Act::Open(key)) => toggle(entity, &mut table, key, Some(true), &mut commands),
            Some(Act::Close(key)) => toggle(entity, &mut table, key, Some(false), &mut commands),
            Some(Act::Goto(index)) => pick(
                entity,
                index,
                Pick::Only,
                true,
                chosen(mode, &mut selection, &mut cursor, &mut cache),
                &mut commands,
            ),
            None => {}
        }
    }

    if star
        && let Some(key) = cursor.0
        && let Some(tree) = table.source::<Tree>()
    {
        let closed: Vec<u64> = tree
            .siblings(key)
            .into_iter()
            .filter(|&k| tree.is_branch(k) && tree.is_open(k) == Some(false))
            .collect();
        for key in closed {
            toggle(entity, &mut table, key, Some(true), &mut commands);
        }
    }
}

/// The table's own bundle of selection state, for [`pick`].
fn chosen<'a, 'w>(
    mode: &'a SelectMode,
    selection: &'a mut Mut<'w, TableSelection>,
    cursor: &'a mut Mut<'w, TableCursor>,
    cache: &'a mut Mut<'w, TableCache>,
) -> Chosen<'a, 'w> {
    (mode, selection, cursor, cache)
}
