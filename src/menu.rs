//! Menus: a bar of titles, each opening a card of commands, with checks, radio runs, separators,
//! right-aligned shortcuts and submenus, walked by key and pad as well as the pointer.
//!
//! A menu is spawned once and shown by opening: its panel is a persistent child of its title (or
//! its submenu item), `Display::None` and in no focus scope while closed, and while open an
//! ordinary modal [`crate::popover_modal`] (a [`PopoverPanel`], a [`Dialog`] and a modal
//! `TabGroup`), so placement, outside press, `Cancel`, focus in and focus back are the popover's.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{
    Checked, ComputedUiRenderTargetInfo, InteractionDisabled, OverrideClip, UiGlobalTransform,
};
use bevy::ui_widgets::{Activate, ScrollArea, ValueChange};

use crate::cancel::Cancel;
use crate::context::ContextTarget;
use crate::list::span;
use crate::model::scroll_to_reveal;
use crate::nav::{Dialog, show_focus};
use crate::popover::{Placement, PopoverPanel};
use crate::shortcut::{Modifiers, Shortcut};
use crate::skin::{GooeySkin, Role};
use crate::tooltip::Tooltip;
use crate::widgets::{CHEVRON_INK_SHIFT, ONE_LINE};

/// A menu title or a menu item: the rows whose Left, Right, Home and End are [`menu_by_input`]'s.
pub(crate) type MenuRow = Or<(With<MenuTitle>, With<MenuItem>)>;

/// A title the pointer has just moved onto or off.
type HoveredTitle = (With<MenuTitle>, Changed<Hovered>);

/// A title whose menu is open.
type OpenTitle = (With<MenuTitle>, With<MenuOpen>);

/// What a submenu with nothing in it says.
const NOTHING_HERE: &str = "Nothing here";

/// A [`crate::menu_bar`]: the row of [`MenuTitle`]s. It remembers the widget that had focus when
/// F10 or Alt took focus into it, so Escape (a [`Cancel`] reaching the bar) and choosing a command
/// give it back.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct MenuBar {
    entered_from: Option<Entity>,
}

/// A [`crate::menu_title`]: a bar's button that opens a menu. Down, Enter, Space, South and a click
/// open it; Left and Right walk the bar.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct MenuTitle;

/// The card a [`MenuTitle`] or a [`crate::submenu`] opens. It exists from spawn, closed (not drawn,
/// in no scope); it is open exactly while it has a [`PopoverPanel`].
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct MenuPanel;

/// On a [`MenuTitle`] or a submenu item exactly while its menu is open: the anchor stays lit.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct MenuOpen;

/// On every item of an open menu that has a check or a radio item in it: they keep the slot at
/// the start of the row that holds the tick, so labels share one edge. A menu with none leaves
/// labels at the row's own padding.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct MenuMarked;

/// A line of a menu that is not a command: the "Nothing here" of an empty menu.
#[derive(Component, Clone, Copy, Debug)]
struct MenuNothing;

/// What a menu item does when it is activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Command,
    Check,
    Radio,
    Submenu,
}

/// One command of a menu: [`crate::menu_item`], [`crate::check_item`], [`crate::radio_item`] and
/// (through [`crate::submenu`]) a row that opens another menu. It is a component whose insertion
/// builds the row (Bevy's `Button`, [`Role::MenuItem`] and a mark, a label, a shortcut and a
/// chevron as the item has them), so it chains `.with_shortcut(..)` and sits in a `children![]`
/// beside the game's own `observe(..)`. It sets the row's `Node`: put your own layout on a
/// wrapper, not on the item.
///
/// Activating an item sends Bevy's [`Activate`] on it and closes the whole cascade; a check item
/// flips its own [`Checked`] and a radio item sets its own and clears the radio items of its run
/// (the siblings between two separators), each sending a [`ValueChange<bool>`]. See
/// [`Disabled`] for an item that cannot be chosen yet.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(menu_bar(children![menu_title(
///     "File",
///     children![
///         (
///             menu_item("Open...").with_shortcut(Shortcut::primary(KeyCode::KeyO)),
///             observe(|_: On<Activate>| info!("open")),
///         ),
///         separator(),
///         check_item("Autosave", true),
///     ],
/// )]));
/// # }
/// ```
#[derive(Component, Clone, Debug)]
#[component(on_add = build_item)]
pub struct MenuItem {
    label: String,
    shortcut: Option<Shortcut>,
    kind: Kind,
    on: bool,
}

impl MenuItem {
    pub(crate) fn command(label: impl Into<String>) -> Self {
        Self::of(label, Kind::Command, false)
    }

    pub(crate) fn check(label: impl Into<String>, on: bool) -> Self {
        Self::of(label, Kind::Check, on)
    }

    pub(crate) fn radio(label: impl Into<String>, on: bool) -> Self {
        Self::of(label, Kind::Radio, on)
    }

    pub(crate) fn submenu(label: impl Into<String>) -> Self {
        Self::of(label, Kind::Submenu, false)
    }

    fn of(label: impl Into<String>, kind: Kind, on: bool) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            kind,
            on,
        }
    }

    /// The role an assistive technology sees, and whether the row opens a menu.
    pub(crate) fn access(&self) -> (accesskit::Role, bool) {
        match self.kind {
            Kind::Command => (accesskit::Role::MenuItem, false),
            Kind::Check => (accesskit::Role::MenuItemCheckBox, false),
            Kind::Radio => (accesskit::Role::MenuItemRadio, false),
            Kind::Submenu => (accesskit::Role::MenuItem, true),
        }
    }

    /// Shows `shortcut` at the end of the row, in the platform's own words ([`Shortcut::label`]).
    /// The item does not listen for the chord: that is the game's, with
    /// [`Shortcut::just_pressed`].
    #[must_use]
    pub fn with_shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

/// Makes the item it is on unavailable, and says why: Bevy's `InteractionDisabled` (so it never
/// activates, by key, pad or pointer, and is drawn muted) and a [`Tooltip`] with the reason. The
/// item stays focusable, so the player who walks onto it by key or pad reads the reason too.
/// Removing it makes the item available and removes the tooltip; inserting it again with another
/// reason changes the line.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((menu_item("Paste"), Disabled::new("Nothing on the clipboard")));
/// # }
/// ```
#[derive(Component, Clone, Debug, PartialEq, Eq)]
#[component(on_insert = disable, on_remove = enable)]
pub struct Disabled(pub String);

impl Disabled {
    /// A disabled item with `reason` as the line its tooltip shows.
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

fn disable(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(reason) = world.get::<Disabled>(entity).map(|d| d.0.clone()) else {
        return;
    };
    world
        .commands()
        .entity(entity)
        .insert((InteractionDisabled, Tooltip::new(reason)));
}

fn enable(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .entity(entity)
        .try_remove::<(InteractionDisabled, Tooltip)>();
}

/// Builds a menu item's row when [`MenuItem`] is added.
fn build_item(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(item) = world.get::<MenuItem>(entity).cloned() else {
        return;
    };
    let mut commands = world.commands();
    let mut row = commands.entity(entity);
    row.insert((
        Node::default(),
        bevy::ui_widgets::Button,
        Role::MenuItem,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
    ));
    if item.on {
        row.insert(Checked);
    }
    row.with_children(|row| {
        let mark = row
            .spawn((Node::default(), Role::MenuMark, Pickable::IGNORE))
            .id();
        row.commands()
            .entity(mark)
            .with_children(|mark| match item.kind {
                Kind::Check => {
                    mark.spawn((
                        Node::default(),
                        Role::MenuCheck,
                        UiTransform {
                            rotation: Rot2::degrees(45.0),
                            translation: Val2::percent(0.0, -12.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                }
                Kind::Radio => {
                    mark.spawn((Node::default(), Role::MenuDot, Pickable::IGNORE));
                }
                Kind::Command | Kind::Submenu => {}
            });
        row.spawn((
            Text::new(item.label),
            Role::MenuText,
            ONE_LINE,
            Pickable::IGNORE,
        ));
        if let Some(shortcut) = item.shortcut {
            row.spawn((
                Text::new(shortcut.label()),
                Role::MenuShortcut,
                ONE_LINE,
                Pickable::IGNORE,
            ));
        }
        if item.kind == Kind::Submenu {
            row.spawn((
                Node::default(),
                Role::MenuChevron,
                UiTransform {
                    translation: Val2::percent(-CHEVRON_INK_SHIFT, 0.0),
                    rotation: Rot2::degrees(45.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
    });
}

// --- opening and closing -------------------------------------------------------------------

/// The menu panel an anchor opens, if it has one.
fn panel_of(world: &World, anchor: Entity) -> Option<Entity> {
    world
        .get::<Children>(anchor)?
        .iter()
        .find(|&child| world.get::<MenuPanel>(child).is_some())
}

fn is_open(world: &World, panel: Entity) -> bool {
    world.get::<PopoverPanel>(panel).is_some()
}

/// The widget whose menu `panel` is.
fn anchor_of(world: &World, panel: Entity) -> Option<Entity> {
    world.get::<ChildOf>(panel).map(ChildOf::parent)
}

/// Every panel under `entity`, deepest first.
fn panels_below(world: &World, entity: Entity, out: &mut Vec<Entity>) {
    let Some(children) = world.get::<Children>(entity) else {
        return;
    };
    for child in children.iter() {
        panels_below(world, child, out);
        if world.get::<MenuPanel>(child).is_some() {
            out.push(child);
        }
    }
}

/// Opens `panel`: it becomes a modal popover under (or, for a submenu, right of) its anchor, and
/// any sibling menu closes. Focus moves into it when it is drawn ([`crate::focus_opened_dialogs`]).
pub(crate) fn open_menu(world: &mut World, panel: Entity) {
    if is_open(world, panel) {
        return;
    }
    let Some(anchor) = anchor_of(world, panel) else {
        return;
    };
    // One menu of a row or a card open at a time.
    let siblings: Vec<Entity> = world
        .get::<ChildOf>(anchor)
        .and_then(|up| world.get::<Children>(up.parent()))
        .map(|kids| kids.iter().filter(|&k| k != anchor).collect())
        .unwrap_or_default();
    for sibling in siblings {
        if let Some(other) = panel_of(world, sibling)
            && is_open(world, other)
        {
            close_menu(world, other);
        }
    }
    let placement = if world.get::<MenuTitle>(anchor).is_some() {
        Placement::Below
    } else {
        Placement::Right
    };
    show_menu(world, panel, anchor, placement);
    world.entity_mut(anchor).insert(MenuOpen);
}

/// Makes `panel` an open modal popover on `anchor`: the one place a menu panel becomes a
/// [`PopoverPanel`], so the placement, the scope and the focus rules are the popover's for a menu
/// of a bar, a submenu and a context menu alike.
pub(crate) fn show_menu(world: &mut World, panel: Entity, anchor: Entity, placement: Placement) {
    sync_nothing(world, panel);
    sync_marks(world, panel);
    let mut entity = world.entity_mut(panel);
    entity.insert((
        PopoverPanel::menu(anchor, placement),
        Dialog::default(),
        TabGroup {
            order: 0,
            modal: true,
        },
        Visibility::Hidden,
    ));
    if let Some(mut node) = entity.get_mut::<Node>() {
        node.display = Display::Flex;
    }
}

/// Closes `panel` and every menu open under it, deepest first, giving focus back to whoever opened
/// each one ([`Dialog`]'s own rule). The panel stays, hidden, for the next opening.
pub(crate) fn close_menu(world: &mut World, panel: Entity) {
    let mut below = Vec::new();
    panels_below(world, panel, &mut below);
    for inner in below {
        if is_open(world, inner) {
            close_menu(world, inner);
        }
    }
    let anchor = anchor_of(world, panel);
    if let Ok(mut entity) = world.get_entity_mut(panel) {
        entity.remove::<(PopoverPanel, Dialog, TabGroup)>();
        entity.insert(Visibility::Hidden);
        if let Some(mut node) = entity.get_mut::<Node>() {
            node.display = Display::None;
        }
        // A context menu stands apart from what it was opened beside, so it is a root again.
        if entity.contains::<ContextTarget>() {
            entity.remove::<(ContextTarget, ChildOf)>();
        }
    }
    if let Some(anchor) = anchor
        && let Ok(mut entity) = world.get_entity_mut(anchor)
    {
        entity.remove::<MenuOpen>();
    }
}

/// Marks every item of `panel` when any of them is a check or a radio item, and none otherwise.
fn sync_marks(world: &mut World, panel: Entity) {
    let items: Vec<(Entity, Kind)> = world
        .get::<Children>(panel)
        .map(|kids| {
            kids.iter()
                .filter_map(|k| world.get::<MenuItem>(k).map(|i| (k, i.kind)))
                .collect()
        })
        .unwrap_or_default();
    let marked = items
        .iter()
        .any(|&(_, kind)| matches!(kind, Kind::Check | Kind::Radio));
    for (item, _) in items {
        if marked {
            world.entity_mut(item).insert(MenuMarked);
        } else {
            world.entity_mut(item).remove::<MenuMarked>();
        }
    }
}

/// An empty menu says so, and a menu that has got items again stops.
fn sync_nothing(world: &mut World, panel: Entity) {
    let kids: Vec<Entity> = world
        .get::<Children>(panel)
        .map(|k| k.iter().collect())
        .unwrap_or_default();
    let has_items = kids.iter().any(|&k| world.get::<MenuItem>(k).is_some());
    let note = kids
        .iter()
        .copied()
        .find(|&k| world.get::<MenuNothing>(k).is_some());
    match (has_items, note) {
        (false, None) => {
            world.spawn((
                Node::default(),
                Text::new(NOTHING_HERE),
                Role::MenuNote,
                ONE_LINE,
                MenuNothing,
                ChildOf(panel),
            ));
        }
        (true, Some(note)) => {
            world.entity_mut(note).despawn();
        }
        _ => {}
    }
}

/// The outermost open menu above `item`, the root of the cascade it is in.
fn root_of(world: &World, from: Entity) -> Option<Entity> {
    let mut root = None;
    let mut at = Some(from);
    while let Some(entity) = at {
        if world.get::<MenuPanel>(entity).is_some() && is_open(world, entity) {
            root = Some(entity);
        }
        at = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    root
}

/// The title a cascade hangs from: walk up from a menu row through its panels to a [`MenuTitle`].
fn title_of(world: &World, from: Entity) -> Option<Entity> {
    let mut at = Some(from);
    while let Some(entity) = at {
        if world.get::<MenuTitle>(entity).is_some() {
            return Some(entity);
        }
        at = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    None
}

fn focus_on(world: &mut World, entity: Entity) {
    world
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    if let Some(mut shown) = world.get_resource_mut::<InputFocusVisible>() {
        show_focus_world(&mut shown);
    }
}

fn show_focus_world(shown: &mut InputFocusVisible) {
    if !shown.0 {
        shown.0 = true;
    }
}

/// Gives focus back to where F10 or Alt took it from, once the bar has been left.
fn leave_bar(world: &mut World, title: Entity) {
    let Some(bar) = world.get::<ChildOf>(title).map(ChildOf::parent) else {
        return;
    };
    let Some(mut state) = world.get_mut::<MenuBar>(bar) else {
        return;
    };
    let Some(back) = state.entered_from.take() else {
        return;
    };
    if world.get_entity(back).is_ok() {
        world
            .resource_mut::<InputFocus>()
            .set(back, FocusCause::Navigated);
    }
}

/// A title's own `Activate`, or an item's: open or close, or choose.
pub(crate) fn menu_on_activate(
    activate: On<Activate>,
    rows: Query<(Has<MenuTitle>, Has<MenuItem>)>,
    mut commands: Commands,
) {
    let entity = activate.entity;
    match rows.get(entity) {
        Ok((true, _)) => {
            commands.queue(move |world: &mut World| toggle_title(world, entity));
        }
        Ok((_, true)) => {
            commands.queue(move |world: &mut World| activate_item(world, entity));
        }
        _ => {}
    }
}

fn toggle_title(world: &mut World, title: Entity) {
    let Some(panel) = panel_of(world, title) else {
        return;
    };
    if is_open(world, panel) {
        close_menu(world, panel);
    } else {
        open_menu(world, panel);
    }
}

fn activate_item(world: &mut World, item: Entity) {
    if world.get::<InteractionDisabled>(item).is_some() {
        return;
    }
    let Some(kind) = world.get::<MenuItem>(item).map(|i| i.kind) else {
        return;
    };
    match kind {
        Kind::Submenu => {
            if let Some(panel) = panel_of(world, item) {
                open_menu(world, panel);
            }
            return;
        }
        Kind::Check => {
            let on = world.get::<Checked>(item).is_none();
            set_checked(world, item, on);
            world.trigger(ValueChange {
                source: item,
                value: on,
                is_final: true,
            });
        }
        Kind::Radio => {
            if world.get::<Checked>(item).is_none() {
                for other in radio_run(world, item) {
                    set_checked(world, other, other == item);
                }
                world.trigger(ValueChange {
                    source: item,
                    value: true,
                    is_final: true,
                });
            }
        }
        Kind::Command => {}
    }
    if let Some(root) = root_of(world, item) {
        let title = anchor_of(world, root).filter(|&a| world.get::<MenuTitle>(a).is_some());
        close_menu(world, root);
        if let Some(title) = title {
            leave_bar(world, title);
        }
    }
}

fn set_checked(world: &mut World, entity: Entity, on: bool) {
    let mut entity = world.entity_mut(entity);
    if on {
        entity.insert(Checked);
    } else {
        entity.remove::<Checked>();
    }
}

/// The radio items of the run `item` is in: its siblings with no separator or other kind of row
/// between.
fn radio_run(world: &World, item: Entity) -> Vec<Entity> {
    let is_radio = |e: Entity| {
        world
            .get::<MenuItem>(e)
            .is_some_and(|i| i.kind == Kind::Radio)
    };
    let Some(kids) = world
        .get::<ChildOf>(item)
        .and_then(|up| world.get::<Children>(up.parent()))
    else {
        return vec![item];
    };
    let kids: Vec<Entity> = kids.iter().collect();
    let Some(at) = kids.iter().position(|&k| k == item) else {
        return vec![item];
    };
    let start = kids[..at]
        .iter()
        .rposition(|&k| !is_radio(k))
        .map_or(0, |i| i + 1);
    let end = kids[at..]
        .iter()
        .position(|&k| !is_radio(k))
        .map_or(kids.len(), |i| at + i);
    kids[start..end].to_vec()
}

// --- keys ----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    Right,
    Left,
    Down,
    Home,
    End,
}

/// Left, Right, Down, Home and End on a focused title or menu item, from the keyboard and the
/// D-pad: Down on a title opens its menu; Left and Right walk the bar (wrapping) and, inside a
/// menu, Right opens a submenu or moves to the next title's menu, Left closes a submenu or moves
/// to the previous title's; Home and End go to the first and last item. Up and Down among a menu's
/// items are [`crate::navigate_by_input`]'s, which leaves the other directions of a menu row to
/// this system. Runs in [`crate::GooeySystems::Navigate`], before it.
pub fn menu_by_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    rows: Query<(), MenuRow>,
    mut commands: Commands,
) {
    let Some(focused) = focus.get().filter(|&f| rows.contains(f)) else {
        return;
    };
    let pressed = |key: KeyCode, pad: GamepadButton| {
        keys.just_pressed(key) || gamepads.iter().any(|g| g.just_pressed(pad))
    };
    let key = if pressed(KeyCode::ArrowRight, GamepadButton::DPadRight) {
        Key::Right
    } else if pressed(KeyCode::ArrowLeft, GamepadButton::DPadLeft) {
        Key::Left
    } else if pressed(KeyCode::ArrowDown, GamepadButton::DPadDown) {
        Key::Down
    } else if keys.just_pressed(KeyCode::Home) {
        Key::Home
    } else if keys.just_pressed(KeyCode::End) {
        Key::End
    } else {
        return;
    };
    show_focus(&mut shown);
    commands.queue(move |world: &mut World| menu_key(world, focused, key));
}

fn menu_key(world: &mut World, focused: Entity, key: Key) {
    if world.get::<MenuTitle>(focused).is_some() {
        match key {
            Key::Down => {
                if let Some(panel) = panel_of(world, focused)
                    && world.get::<InteractionDisabled>(focused).is_none()
                {
                    open_menu(world, panel);
                }
            }
            Key::Right => step_title(world, focused, 1, false),
            Key::Left => step_title(world, focused, -1, false),
            Key::Home => edge_title(world, focused, false),
            Key::End => edge_title(world, focused, true),
        }
        return;
    }
    let Some(kind) = world.get::<MenuItem>(focused).map(|i| i.kind) else {
        return;
    };
    let Some(panel) = world.get::<ChildOf>(focused).map(ChildOf::parent) else {
        return;
    };
    let in_submenu = anchor_of(world, panel).is_some_and(|a| world.get::<MenuItem>(a).is_some());
    match key {
        Key::Right => {
            if kind == Kind::Submenu {
                if world.get::<InteractionDisabled>(focused).is_none()
                    && let Some(sub) = panel_of(world, focused)
                {
                    open_menu(world, sub);
                }
            } else if let Some(title) = title_of(world, focused) {
                step_title(world, title, 1, true);
            }
        }
        Key::Left => {
            // An item whose submenu is open (it has no item to put focus on, so focus stayed) closes
            // it first.
            if let Some(sub) = panel_of(world, focused).filter(|&sub| is_open(world, sub)) {
                close_menu(world, sub);
            } else if in_submenu {
                close_menu(world, panel);
            } else if let Some(title) = title_of(world, focused) {
                step_title(world, title, -1, true);
            }
        }
        Key::Home | Key::End => {
            let items: Vec<Entity> = world
                .get::<Children>(panel)
                .map(|k| {
                    k.iter()
                        .filter(|&e| world.get::<MenuItem>(e).is_some())
                        .collect()
                })
                .unwrap_or_default();
            let to = if key == Key::Home {
                items.first()
            } else {
                items.last()
            };
            if let Some(&to) = to {
                focus_on(world, to);
            }
        }
        Key::Down => {}
    }
}

/// The titles of the bar `title` is in, in order.
fn titles_of(world: &World, title: Entity) -> Vec<Entity> {
    world
        .get::<ChildOf>(title)
        .and_then(|up| world.get::<Children>(up.parent()))
        .map(|kids| {
            kids.iter()
                .filter(|&k| world.get::<MenuTitle>(k).is_some())
                .collect()
        })
        .unwrap_or_default()
}

/// Focus on the next (or previous) title, wrapping; with `open`, its menu opens with the one that
/// was open closed (focus first: the new menu's opener is the title the player is now on).
fn step_title(world: &mut World, from: Entity, by: isize, open: bool) {
    let titles = titles_of(world, from);
    let Some(at) = titles.iter().position(|&t| t == from) else {
        return;
    };
    let n = titles.len() as isize;
    let to = titles[(at as isize + by).rem_euclid(n) as usize];
    if let Some(panel) = panel_of(world, from)
        && is_open(world, panel)
    {
        close_menu(world, panel);
    }
    focus_on(world, to);
    if open && let Some(panel) = panel_of(world, to) {
        open_menu(world, panel);
    }
}

fn edge_title(world: &mut World, from: Entity, last: bool) {
    let titles = titles_of(world, from);
    let to = if last { titles.last() } else { titles.first() };
    if let Some(&to) = to {
        focus_on(world, to);
    }
}

/// F10, or Alt pressed and released with no other key between, puts focus on the first title of
/// the first [`MenuBar`] (the same press, with focus already in the bar, gives focus back to
/// where it came from). Runs in [`crate::GooeySystems::Navigate`].
#[allow(clippy::too_many_arguments)]
pub fn focus_menu_bar(
    keys: Res<ButtonInput<KeyCode>>,
    mut alone: Local<bool>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    mut bars: Query<(
        Entity,
        &mut MenuBar,
        &InheritedVisibility,
        Option<&Children>,
    )>,
    titles: Query<(), With<MenuTitle>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let alt = |key: &KeyCode| matches!(key, KeyCode::AltLeft | KeyCode::AltRight);
    if keys.get_just_pressed().any(|k| !alt(k)) {
        *alone = false;
    } else if keys.get_just_pressed().any(alt) {
        *alone = true;
    }
    let tapped = *alone && keys.get_just_released().any(alt);
    if tapped {
        *alone = false;
    }
    // Shift+F10 is the context menu's ([`crate::context_by_key`]).
    let f10 = keys.just_pressed(KeyCode::F10) && !Modifiers::held(&keys).shift;
    if !(tapped || f10) {
        return;
    }
    let current = focus.get();
    let inside = |bar: Entity| {
        current.is_some_and(|f| f == bar || parents.iter_ancestors(f).any(|a| a == bar))
    };
    // Focus in a bar already: let go of it, the way Escape does.
    if let Some((_, mut state, ..)) = bars.iter_mut().find(|(bar, ..)| inside(*bar)) {
        if let Some(back) = state.entered_from.take() {
            commands.queue(move |world: &mut World| {
                if world.get_entity(back).is_ok() {
                    world
                        .resource_mut::<InputFocus>()
                        .set(back, FocusCause::Navigated);
                }
            });
        }
        return;
    }
    let Some((_, mut state, _, Some(kids))) = bars.iter_mut().find(|(_, _, seen, kids)| {
        seen.get() && kids.is_some_and(|k| k.iter().any(|t| titles.contains(t)))
    }) else {
        return;
    };
    let Some(first) = kids.iter().find(|&t| titles.contains(t)) else {
        return;
    };
    state.entered_from = current;
    show_focus(&mut shown);
    commands.queue(move |world: &mut World| focus_on(world, first));
}

/// A [`Cancel`] that reaches the bar (Escape or B with focus on a title) hands focus back to where
/// F10 or Alt took it from, and goes no further when there was such a place.
pub(crate) fn bar_on_cancel(
    mut cancel: On<Cancel>,
    mut bars: Query<&mut MenuBar>,
    mut focus: ResMut<InputFocus>,
    alive: Query<()>,
) {
    let Ok(mut bar) = bars.get_mut(cancel.entity) else {
        return;
    };
    if let Some(back) = bar.entered_from.take()
        && alive.contains(back)
    {
        cancel.propagate(false);
        focus.set(back, FocusCause::Navigated);
    }
}

/// A pointer moving onto another title while a menu of the bar is open opens that title's menu
/// instead, as every desktop menu bar does. Runs in [`crate::GooeySystems::Navigate`].
pub fn switch_menus_on_hover(
    hovered: Query<(Entity, &Hovered, &ChildOf), HoveredTitle>,
    open: Query<(Entity, &ChildOf), OpenTitle>,
    mut commands: Commands,
) {
    for (title, hover, bar) in &hovered {
        if !hover.get() || open.contains(title) {
            continue;
        }
        if open.iter().any(|(_, other)| other.parent() == bar.parent()) {
            commands.queue(move |world: &mut World| {
                if world.get::<InteractionDisabled>(title).is_some() {
                    return;
                }
                let Some(panel) = panel_of(world, title) else {
                    return;
                };
                focus_on(world, title);
                open_menu(world, panel);
            });
        }
    }
}

// --- long menus ----------------------------------------------------------------------------

/// Caps an open menu's height at the room the window leaves it on the larger side of its anchor (below
/// its title, or from the top of its submenu item, or the side a context menu flips to), so a long
/// menu scrolls and a short one is unchanged. Runs in
/// [`crate::GooeySystems::Navigate`].
pub fn fit_menus(
    skin: Res<GooeySkin>,
    mut panels: Query<(&ChildOf, &PopoverPanel, &mut Node), With<MenuPanel>>,
    anchors: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
    )>,
) {
    for (parent, panel, mut node) in &mut panels {
        let Ok((computed, transform, target)) = anchors.get(parent.parent()) else {
            continue;
        };
        let (top, height) = span(computed, transform);
        let window = target.logical_size().y;
        let line = skin.border;
        // A menu opens on the side of its anchor with the room (the popover's flip), so it is
        // capped at the larger side, not at the one it prefers.
        let room = match panel.placement {
            Placement::Below | Placement::Above => {
                let below = window - (top + height) - skin.gap * 1.5 - 2.0 * line;
                let above = top - skin.gap * 1.5 - 2.0 * line;
                below.max(above)
            }
            Placement::AtPointer => (window - top).max(top) - skin.gap,
            _ => window - top - skin.gap,
        };
        let least = skin.target() * 3.0;
        // Whole pixels, so layout noise never rewrites the node.
        let cap = Val::Px(room.max(least).floor());
        if node.max_height != cap {
            node.max_height = cap;
        }
    }
}

/// Scrolls an open menu that does not fit to keep the focused item in view. Runs in
/// [`crate::GooeySystems::Navigate`].
pub fn scroll_menus_to_focus(
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    parents: Query<&ChildOf>,
    panels: Query<(), With<MenuPanel>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform), Without<MenuPanel>>,
    mut views: Query<(&ComputedNode, &UiGlobalTransform, &mut ScrollPosition), With<MenuPanel>>,
) {
    if !visible.0 || !(focus.is_changed() || visible.is_changed()) {
        return;
    }
    let Some(target) = focus.get() else {
        return;
    };
    let Some(panel) = parents.iter_ancestors(target).find(|&a| panels.contains(a)) else {
        return;
    };
    let (Ok((node, transform)), Ok((view, view_transform, mut scroll))) =
        (nodes.get(target), views.get_mut(panel))
    else {
        return;
    };
    let scale = view.inverse_scale_factor();
    let (top, height) = span(node, transform);
    let (view_top, view_height) = span(view, view_transform);
    let laid_out = view.scroll_position.y * scale;
    let max = (view.content_size().y + view.scrollbar_size.y - view.size().y).max(0.0) * scale;
    if max <= 0.0 {
        return;
    }
    let from_start = top - view_top + laid_out;
    let want = scroll_to_reveal(
        from_start,
        from_start + height,
        view_height,
        laid_out,
        max,
        0.0,
    );
    if (want - scroll.y).abs() > 0.5 {
        scroll.y = want;
    }
}

/// Enter and Space that reach a menu's panel were not taken by an enabled item (a disabled one
/// takes nothing), so they stop here. Bevy's `FocusedInput` is retargeted at each ancestor as it
/// bubbles, and the title above the panel is a `Button`, which would read the press as its own and
/// toggle the menu closed.
fn stop_activation_keys(mut input: On<FocusedInput<KeyboardInput>>) {
    if matches!(input.input.key_code, KeyCode::Enter | KeyCode::Space) {
        input.propagate(false);
    }
}

/// The bundle of a menu panel: absolute, drawn nowhere until opened, one z step above a dialog,
/// scrolling when it is taller than the room; `items` are its rows.
pub(crate) fn menu_panel(items: impl Bundle) -> impl Bundle {
    use crate::popover::{
        placement_of, popover_on_cancel, stop_acquire, stop_click, stop_press, stop_release,
    };
    use bevy::ui::FixedNode;
    use bevy::ui_widgets::observe;
    (
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            overflow: Overflow::scroll_y(),
            ..default()
        },
        Visibility::Hidden,
        Role::MenuPanel,
        MenuPanel,
        ScrollArea,
        placement_of(Placement::Below, 0.0, 0.0),
        FixedNode,
        BoxShadow::default(),
        GlobalZIndex(150),
        OverrideClip,
        observe(popover_on_cancel),
        observe(stop_activation_keys),
        (
            observe(stop_press),
            observe(stop_release),
            observe(stop_click),
            observe(stop_acquire),
        ),
        items,
    )
}
