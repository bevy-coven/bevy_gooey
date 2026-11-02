//! The slot grid: what keeps a [`crate::slot_grid`]'s slots true to its [`SlotItems`], its cursor
//! (the one tab stop), and the events a game hears when a slot is chosen or an item is moved.
//! Its look is [`crate::apply_skin`]'s; the move is [`crate::DragPayload`] and
//! [`crate::DropTarget`], the crate's own drag and drop.

use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{InteractionDisabled, ZIndex};
use bevy::ui_widgets::{Activate, ValueChange, observe};

use crate::drag::{DragPayload, DropRefused, DropTarget, Dropped};
use crate::icon::{Icon, IconSource};
use crate::model::slot_edge;
use crate::nav::show_focus;
use crate::skin::Role;
use crate::tooltip::Tooltip;

/// What a slot of a [`crate::slot_grid`] holds: an item's name, its picture, how many there are
/// and a line for its tooltip.
///
/// ```
/// # use bevy_gooey::*;
/// let potion = Slot::new("Health potion").count(3).tooltip("Restores 40 health");
/// assert_eq!(potion.count, 3);
/// ```
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
pub struct Slot {
    /// The item's name: drawn in the slot while it has no icon, read by a screen reader, and shown
    /// in the tooltip of an icon slot that was given no other text.
    pub name: String,
    /// The picture, tinted by the skin like every [`crate::icon`]; draw it white on transparent.
    pub icon: Option<IconSource>,
    /// How many: a number is drawn in the corner above one.
    pub count: u32,
    /// The tooltip's line, shown on hover and focus. An icon slot with none shows its name; a slot
    /// with no icon, which already shows its name, has no tooltip.
    pub tooltip: Option<String>,
    /// What sort of thing the item is (`"helmet"`, `"potion"`), for a [`SlotRules`] test to read.
    /// The crate gives it no meaning.
    pub kind: Option<String>,
}

impl Slot {
    /// An item called `name`: one of it, no picture, no tooltip.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            icon: None,
            count: 1,
            tooltip: None,
            kind: None,
        }
    }

    /// The same item drawn as `icon`.
    pub fn icon(mut self, icon: IconSource) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The same item, `count` of it.
    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// The same item with `text` for its tooltip.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// The same item of `kind`, for a [`SlotRules`] test.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// The tooltip the slot wears: the given line, else the name of an item the slot only draws
    /// as a picture.
    fn tip(&self) -> Option<&str> {
        self.tooltip
            .as_deref()
            .or_else(|| self.icon.is_some().then_some(self.name.as_str()))
    }
}

/// The contents of a [`crate::slot_grid`], in reading order: `Some` for an item, `None` for an
/// empty slot. It is the grid's data and the one place to change it: push, remove or write a slot
/// and the grid follows in the same frame (slots are added and removed to match its length, and a
/// slot whose content did not change is not touched). A game whose inventory lives elsewhere
/// writes it from there when that changes.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn pick_up(mut grids: Query<&mut SlotItems>) {
///     for mut items in &mut grids {
///         if let Some(free) = items.0.iter().position(Option::is_none) {
///             items.0[free] = Some(Slot::new("Torch"));
///         }
///     }
/// }
/// ```
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct SlotItems(pub Vec<Option<Slot>>);

impl SlotItems {
    /// `len` empty slots.
    pub fn empty(len: usize) -> Self {
        Self(vec![None; len])
    }
}

impl FromIterator<Option<Slot>> for SlotItems {
    fn from_iter<T: IntoIterator<Item = Option<Slot>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// The root of a [`crate::slot_grid`]: how many slots are in a row. Its slots are [`SlotItems`]
/// and its state is [`SlotCursor`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[require(SlotItems, SlotCursor, SlotRules)]
pub struct SlotGrid {
    /// Slots in a row; at least one.
    pub columns: usize,
}

/// Which items a [`crate::slot_grid`]'s slots take: `accepts(item, index)` says whether slot `index`
/// of this grid may hold `item`. The default takes everything. It is a plain `fn` (a closure that
/// captures nothing), like [`crate::DropTarget::accepting`], so it is data a game writes once.
///
/// A move is allowed when both ends are: the slot it goes to must accept the carried item, and a
/// filled slot, whose item goes back to where the carried one came from, must have that item
/// accepted there too (by the rule of that slot's grid). A slot that does not take the carried
/// item is a refused drop target while the item is over it: the drop mark and ghost turn to their
/// refused look before the player lets go, a release sends one [`SlotRefused`] and moves nothing,
/// and a keyboard or pad carry skips it. A locked slot is a rule that takes nothing; an item can
/// still be taken out of it into a slot that takes it. Overwrite the component to change the rule
/// (the next drag uses it); never remove it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // An equipment panel: slot 0 holds a helmet, slot 1 a weapon, slot 2 is locked.
/// commands.spawn((
///     slot_grid(3, vec![None, None, None]),
///     SlotRules::new(|item, slot| match slot {
///         0 => item.kind.as_deref() == Some("helmet"),
///         1 => item.kind.as_deref() == Some("weapon"),
///         _ => false,
///     }),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug)]
pub struct SlotRules {
    /// Whether slot `index` may hold `item`.
    pub accepts: fn(&Slot, usize) -> bool,
}

impl SlotRules {
    /// A rule: `accepts(item, index)`.
    pub const fn new(accepts: fn(&Slot, usize) -> bool) -> Self {
        Self { accepts }
    }
}

impl Default for SlotRules {
    fn default() -> Self {
        Self::new(|_, _| true)
    }
}

/// The slot of a grid that Tab enters on: the one the player last had focus on. It is the
/// grid's state and its one tab stop (every other slot is reached by the arrows). Focus moving to
/// a slot writes it and sends one non-final [`ValueChange<usize>`] on the grid; write it yourself
/// to move the stop from code (no event is sent for that). A value past the last slot is pulled
/// back to it.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct SlotCursor(pub usize);

/// Where a slot is: its grid and its index there. It is on every slot the grid draws, and it is
/// what a drag from a slot carries (the payload of [`crate::DragPayload`]), so a drop knows which
/// slot an item came from even when it came from another grid.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
pub struct SlotRef {
    /// The [`SlotGrid`].
    pub grid: Entity,
    /// The slot's index in its [`SlotItems`].
    pub index: usize,
}

/// The player moved an item: dragged it from one slot onto another, or carried it there with the
/// keyboard or a pad ([`crate::DragSettings`]). An event on the grid the item was dropped on. The
/// grid has changed nothing: [`SlotMoved::apply`] swaps the two slots in their [`SlotItems`] (onto
/// an empty slot that is a move), and a game with rules (stacks that merge, a slot that takes only
/// helmets) does its own and writes the result.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(slot_grid(4, vec![Some(Slot::new("Sword")), None])).observe(
///     |moved: On<SlotMoved>, mut grids: Query<&mut SlotItems>| {
///         moved.apply(&mut grids);
///     },
/// );
/// # }
/// ```
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct SlotMoved {
    /// The grid the item was dropped on.
    #[event_target]
    pub entity: Entity,
    /// The slot it came from, in this grid or another.
    pub from: SlotRef,
    /// The slot it was dropped on, in [`SlotMoved::entity`].
    pub to: usize,
}

impl SlotMoved {
    /// Swaps the two slots' contents in their [`SlotItems`], across two grids when the item came
    /// from another. Returns whether anything changed: a slot that is not there, or a drop on the
    /// slot it came from, changes nothing.
    pub fn apply(&self, grids: &mut Query<&mut SlotItems>) -> bool {
        let to = SlotRef {
            grid: self.entity,
            index: self.to,
        };
        if self.from == to {
            return false;
        }
        if self.from.grid == self.entity {
            let Ok(mut items) = grids.get_mut(self.entity) else {
                return false;
            };
            if self.from.index.max(self.to) >= items.0.len() {
                return false;
            }
            items.0.swap(self.from.index, self.to);
            return true;
        }
        let Ok([mut from, mut onto]) = grids.get_many_mut([self.from.grid, self.entity]) else {
            return false;
        };
        let (Some(a), Some(b)) = (from.0.get_mut(self.from.index), onto.0.get_mut(self.to)) else {
            return false;
        };
        std::mem::swap(a, b);
        true
    }
}

/// A slot refused an item: the player let go of it over a slot that its grid's [`SlotRules`] do not
/// let hold it (or whose own item the origin's rules do not take back). An event on the grid of the
/// slot that refused; nothing moved. A game plays its "no" here.
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct SlotRefused {
    /// The grid of the slot that refused.
    #[event_target]
    pub entity: Entity,
    /// The slot the item was carried from.
    pub from: SlotRef,
    /// The slot it was let go over, in [`SlotRefused::entity`].
    pub to: usize,
}

/// What a slot last drew, so a change is found by comparing and nothing else is written.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Shown(Option<Slot>);

/// Enter, Space, the pad's South button and a click (Bevy's `Button` makes each an [`Activate`])
/// choose a slot: one final [`ValueChange`] on its grid, for an empty slot too.
fn slot_on_activate(
    event: On<Activate>,
    slots: Query<&SlotRef, Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    if let Ok(slot) = slots.get(event.entity) {
        commands.trigger(ValueChange {
            source: slot.grid,
            value: slot.index,
            is_final: true,
        });
    }
}

/// An item dropped on a slot is one [`SlotMoved`] on the slot's grid; dropped on its own slot it is
/// nothing.
fn slot_on_dropped(
    dropped: On<Dropped<SlotRef>>,
    slots: Query<&SlotRef>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    let Ok(to) = slots.get(dropped.target) else {
        return;
    };
    if dropped.payload != *to {
        // Focus follows the item: the ring is on the slot it was just put in.
        if focus.get() == Some(dropped.source) {
            focus.set(dropped.target, FocusCause::Navigated);
        }
        commands.trigger(SlotMoved {
            entity: to.grid,
            from: dropped.payload,
            to: to.index,
        });
    }
}

/// A drop a slot's rules refused is one [`SlotRefused`] on its grid.
fn slot_on_refused(
    refused: On<DropRefused<SlotRef>>,
    slots: Query<&SlotRef>,
    mut commands: Commands,
) {
    if let Ok(to) = slots.get(refused.target) {
        commands.trigger(SlotRefused {
            entity: to.grid,
            from: refused.payload,
            to: to.index,
        });
    }
}

/// Whether `zone`, a slot, takes what `source`, another slot, carries: the rule of the zone's grid
/// takes the carried item there, and a filled zone's item goes back to the source's slot under the
/// source's rule. Putting an item back where it was is always allowed.
fn slot_takes(zone: EntityRef, source: EntityRef, from: &SlotRef) -> bool {
    let (Some(to), Some(onto)) = (zone.get::<SlotRef>(), zone.get::<SlotRules>()) else {
        return true;
    };
    if to == from {
        return true;
    }
    let Some(carried) = source.get::<Shown>().and_then(|shown| shown.0.as_ref()) else {
        return true;
    };
    if !(onto.accepts)(carried, to.index) {
        return false;
    }
    match (
        zone.get::<Shown>().and_then(|shown| shown.0.as_ref()),
        source.get::<SlotRules>(),
    ) {
        (Some(there), Some(back)) => (back.accepts)(there, from.index),
        _ => true,
    }
}

/// Home and End go to the first and last slot of the row, and with Ctrl of the grid. The arrows
/// are the crate's spatial navigation, as for any buttons.
fn slot_on_key_input(
    mut input: On<FocusedInput<KeyboardInput>>,
    slots: Query<&SlotRef, Without<InteractionDisabled>>,
    grids: Query<(&SlotGrid, &SlotItems, &Children)>,
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
) {
    let Ok(slot) = slots.get(input.focused_entity) else {
        return;
    };
    if input.input.state != ButtonState::Pressed {
        return;
    }
    let end = match input.input.key_code {
        KeyCode::Home => false,
        KeyCode::End => true,
        _ => return,
    };
    input.propagate(false);
    let Ok((grid, items, children)) = grids.get(slot.grid) else {
        return;
    };
    let whole = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let to = slot_edge(slot.index, grid.columns, items.0.len(), end, whole);
    if let Some(&cell) = children.get(to)
        && cell != input.focused_entity
    {
        show_focus(&mut shown);
        focus.set(cell, FocusCause::Navigated);
    }
}

/// One slot's entity: its look is its role's, and its parts (the name, the count, the picture)
/// are its children.
fn spawn_slot(
    commands: &mut Commands,
    grid: Entity,
    index: usize,
    cursor: usize,
    rules: SlotRules,
    item: &Option<Slot>,
) {
    let at = SlotRef { grid, index };
    let mut slot = commands.spawn((
        Node::default(),
        bevy::ui_widgets::Button,
        slot_role(item),
        Hovered::default(),
        TabIndex(if index == cursor { 0 } else { -1 }),
        AutoDirectionalNavigation::default(),
        at,
        Shown(item.clone()),
        rules,
        DropTarget::<SlotRef>::judged(slot_takes),
        observe(slot_on_activate),
        observe(slot_on_dropped),
        observe(slot_on_refused),
        observe(slot_on_key_input),
        ChildOf(grid),
    ));
    if let Some(item) = item {
        slot.insert(DragPayload(at));
        if let Some(tip) = item.tip() {
            slot.insert(Tooltip::new(tip));
        }
    }
    slot.with_children(|parts| {
        parts.spawn((
            Text::new(item.as_ref().map_or("", |i| i.name.as_str())),
            TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
            name_node(item),
            Role::SlotName,
            Pickable::IGNORE,
        ));
        parts.spawn((
            Text::new(count_text(item)),
            count_node(item),
            ZIndex(1),
            Role::SlotCount,
            Pickable::IGNORE,
        ));
        if let Some(icon) = item.as_ref().and_then(|i| i.icon.clone()) {
            parts.spawn(icon_bundle(icon));
        }
    });
}

fn icon_bundle(icon: IconSource) -> impl Bundle {
    (
        Node::default(),
        Role::SlotIcon,
        Icon(icon),
        Pickable::IGNORE,
    )
}

/// A filled slot and an empty one are two roles, so the skin draws the well and the item apart.
fn slot_role(item: &Option<Slot>) -> Role {
    if item.is_some() {
        Role::Slot
    } else {
        Role::SlotEmpty
    }
}

/// The name shows while there is no picture to show instead.
fn name_node(item: &Option<Slot>) -> Node {
    Node {
        max_width: Val::Percent(100.0),
        display: if item.as_ref().is_some_and(|i| i.icon.is_none()) {
            Display::Flex
        } else {
            Display::None
        },
        ..default()
    }
}

/// The count shows above one.
fn count_node(item: &Option<Slot>) -> Node {
    Node {
        position_type: PositionType::Absolute,
        display: if item.as_ref().is_some_and(|i| i.count > 1) {
            Display::Flex
        } else {
            Display::None
        },
        ..default()
    }
}

fn count_text(item: &Option<Slot>) -> String {
    item.as_ref()
        .filter(|i| i.count > 1)
        .map_or_else(String::new, |i| i.count.to_string())
}

/// What the cursor and the slots of a grid are, read together so the grid's system has one query
/// for them.
type GridItem<'a> = (
    Entity,
    &'a SlotGrid,
    Ref<'a, SlotItems>,
    Ref<'a, SlotRules>,
    &'a mut SlotCursor,
    Option<&'a Children>,
);

/// What of a slot the update changes in place.
type CellParts<'a> = (
    &'a mut Shown,
    &'a mut Role,
    &'a mut TabIndex,
    Option<&'a Children>,
    Has<DragPayload<SlotRef>>,
    Option<&'a Tooltip>,
);

/// Keeps every [`crate::slot_grid`] true to its [`SlotItems`]: slots are spawned and despawned to
/// match the length, a slot whose item changed is updated (the name, the count, the picture, the
/// tooltip, whether it can be lifted), and the grid's [`SlotCursor`] follows focus and is the one
/// slot with a `TabIndex` of 0. Writes nothing for a grid that did not change. Runs in
/// [`crate::GooeySystems::Style`] before [`crate::update_icons`].
pub fn update_slot_grids(
    mut commands: Commands,
    focus: Res<InputFocus>,
    mut grids: Query<GridItem>,
    mut cells: Query<CellParts, With<SlotRef>>,
    refs: Query<&SlotRef>,
    mut texts: Query<(&mut Text, &mut Node, &Role), Without<SlotRef>>,
    icons: Query<(Entity, &Role, &Icon), Without<SlotRef>>,
) {
    for (grid_entity, _, items, rules, mut cursor, children) in &mut grids {
        let len = items.0.len();
        // Focus on one of this grid's slots is the player moving the cursor.
        if focus.is_changed()
            && let Some(slot) = focus.get().and_then(|f| refs.get(f).ok())
            && slot.grid == grid_entity
            && slot.index < len
            && cursor.0 != slot.index
        {
            cursor.0 = slot.index;
            commands.trigger(ValueChange {
                source: grid_entity,
                value: slot.index,
                is_final: false,
            });
        }
        let last = len.saturating_sub(1);
        if cursor.0 > last {
            cursor.0 = last;
        }
        let existing: &[Entity] = children.map_or(&[][..], |c| &c[..]);
        let settled = !items.is_changed()
            && !rules.is_changed()
            && !cursor.is_changed()
            && !focus.is_changed()
            && existing.len() == len;
        if settled {
            continue;
        }
        for (index, item) in items.0.iter().enumerate() {
            let Some(&cell) = existing.get(index) else {
                spawn_slot(&mut commands, grid_entity, index, cursor.0, *rules, item);
                continue;
            };
            let Ok((mut shown, mut role, mut tab, parts, lifts, tip)) = cells.get_mut(cell) else {
                continue;
            };
            if rules.is_changed() {
                commands.entity(cell).insert(*rules);
            }
            let stop = TabIndex(if index == cursor.0 { 0 } else { -1 });
            if *tab != stop {
                *tab = stop;
            }
            if shown.0 == *item {
                continue;
            }
            let wanted = slot_role(item);
            if *role != wanted {
                *role = wanted;
            }
            match (item, lifts) {
                (Some(_), false) => {
                    commands.entity(cell).insert(DragPayload(SlotRef {
                        grid: grid_entity,
                        index,
                    }));
                }
                (None, true) => {
                    commands.entity(cell).remove::<DragPayload<SlotRef>>();
                }
                _ => {}
            }
            let line = item.as_ref().and_then(Slot::tip);
            match (line, tip) {
                (Some(line), Some(now)) if now.text == line => {}
                (Some(line), _) => {
                    commands.entity(cell).insert(Tooltip::new(line));
                }
                (None, Some(_)) => {
                    commands.entity(cell).remove::<Tooltip>();
                }
                (None, None) => {}
            }
            let mut icon_part = None;
            for &part in parts.map_or(&[][..], |p| p) {
                if let Ok((mut text, mut node, role)) = texts.get_mut(part) {
                    match role {
                        Role::SlotName => {
                            let name = item.as_ref().map_or("", |i| i.name.as_str());
                            if text.0 != name {
                                text.0 = name.to_owned();
                            }
                            let wanted = name_node(item);
                            if node.display != wanted.display {
                                node.display = wanted.display;
                            }
                        }
                        Role::SlotCount => {
                            let count = count_text(item);
                            if text.0 != count {
                                text.0 = count;
                            }
                            let wanted = count_node(item);
                            if node.display != wanted.display {
                                node.display = wanted.display;
                            }
                        }
                        _ => {}
                    }
                } else if let Ok((icon, role, _)) = icons.get(part)
                    && *role == Role::SlotIcon
                {
                    icon_part = Some(icon);
                }
            }
            let picture = item.as_ref().and_then(|i| i.icon.clone());
            match (picture, icon_part) {
                (Some(icon), Some(part)) => {
                    if icons.get(part).is_ok_and(|(_, _, now)| now.0 != icon) {
                        commands.entity(part).insert(Icon(icon));
                    }
                }
                (Some(icon), None) => {
                    commands.entity(cell).with_child(icon_bundle(icon));
                }
                (None, Some(part)) => commands.entity(part).despawn(),
                (None, None) => {}
            }
            shown.0 = item.clone();
        }
        for &extra in existing.iter().skip(len) {
            commands.entity(extra).despawn();
        }
    }
}
