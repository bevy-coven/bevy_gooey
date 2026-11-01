//! Context menus: a menu that belongs to a thing (a sprite in the world, a row of a list, a
//! button), opened where the player points or where focus is.
//!
//! A context menu is a menu panel of its own ([`crate::context_menu`]) and a thing names it with
//! [`ContextMenu`]. A secondary click that reaches the thing opens it at the pointer; Shift+F10 and
//! the Menu key open it below the focused widget. Opening is a menu's (see [`crate::menu`]): the
//! panel becomes a modal popover, so placement, the outside press, `Cancel`, focus in and focus
//! back are the popover's.

use bevy::input::keyboard::KeyCode;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::events::PointerClick;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;

use crate::menu::{MenuPanel, close_menu, show_menu};
use crate::nav::show_focus;
use crate::popover::{Placement, PopoverPanel};
use crate::shortcut::{Modifiers, Shortcut};

/// On any entity (a UI node, a sprite, a mesh): the [`crate::context_menu`] panel that is its menu.
/// A secondary click that reaches the entity opens it at the pointer, and Shift+F10 or the Menu key
/// with focus on the entity or a widget under it opens it below that widget. The nearest entity
/// above the click or the focus that has one is the one used. One menu may be named by many
/// entities (every row of a list); [`ContextTarget`] says which it was opened for.
///
/// A sprite or mesh is picked only if it carries `Pickable`. A menu the game has despawned opens
/// nothing.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn setup(mut commands: Commands) {
/// let menu = commands
///     .spawn(context_menu(children![
///         (
///             menu_item("Delete"),
///             observe(|activate: On<Activate>,
///                      parents: Query<&ChildOf>,
///                      targets: Query<&ContextTarget>,
///                      mut commands: Commands| {
///                 if let Some(thing) = ContextTarget::of(activate.entity, &parents, &targets) {
///                     commands.entity(thing).despawn();
///                 }
///             }),
///         ),
///     ]))
///     .id();
/// commands.spawn((
///     Sprite::from_color(Color::WHITE, Vec2::splat(64.0)),
///     Pickable::default(),
///     ContextMenu(menu),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub struct ContextMenu(pub Entity);

/// On a context menu's panel exactly while it is open: the entity that has the [`ContextMenu`] it
/// was opened for (never the widget a key opened it beside, when that is one of its descendants).
/// Read it in an item's `Activate` observer with [`ContextTarget::of`], or in an
/// `On<Add<ContextTarget>>` observer, which runs before the menu is first drawn, to enable,
/// disable or relabel the items for this thing.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Clone, PartialEq)]
pub struct ContextTarget(pub Entity);

impl ContextTarget {
    /// The thing the context menu holding `entity` (a menu item, say) is open for: the
    /// `ContextTarget` of its nearest ancestor that has one.
    pub fn of(
        entity: Entity,
        parents: &Query<&ChildOf>,
        targets: &Query<&ContextTarget>,
    ) -> Option<Entity> {
        parents
            .iter_ancestors(entity)
            .find_map(|up| targets.get(up).ok())
            .map(|target| target.0)
    }
}

/// Where a context menu opens.
#[derive(Clone, Copy, Debug)]
enum At {
    /// At the pointer ([`Placement::AtPointer`]).
    Pointer,
    /// Below a widget ([`Placement::Below`]): the focused one.
    Widget(Entity),
}

/// A secondary click that reaches a thing with a [`ContextMenu`] opens it at the pointer, and goes
/// no further: the nearest thing up the hierarchy is the one.
pub(crate) fn context_on_click(
    mut click: On<PointerClick>,
    things: Query<&ContextMenu>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Secondary {
        return;
    }
    let thing = click.entity;
    let Ok(&ContextMenu(menu)) = things.get(thing) else {
        return;
    };
    click.propagate(false);
    commands.queue(move |world: &mut World| open_context(world, menu, thing, At::Pointer));
}

/// Shift+F10 or the Menu key opens the context menu of the focused widget, or of its nearest
/// ancestor that has one, below the focused widget, with focus on its first item. A widget inside
/// a menu is not asked. Runs in [`crate::GooeySystems::Navigate`].
pub fn context_by_key(
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    things: Query<&ContextMenu>,
    menus: Query<(), With<MenuPanel>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let asked = keys.just_pressed(KeyCode::ContextMenu)
        || Shortcut::new(Modifiers::SHIFT, KeyCode::F10).just_pressed(&keys);
    if !asked {
        return;
    }
    let Some(focused) = focus.get() else {
        return;
    };
    let up = std::iter::once(focused).chain(parents.iter_ancestors(focused));
    let mut found = None;
    for entity in up {
        if menus.contains(entity) {
            return;
        }
        if found.is_none()
            && let Ok(&ContextMenu(menu)) = things.get(entity)
        {
            found = Some((entity, menu));
        }
    }
    let Some((thing, menu)) = found else {
        return;
    };
    show_focus(&mut shown);
    commands.queue(move |world: &mut World| open_context(world, menu, thing, At::Widget(focused)));
}

/// Opens `menu` for `thing`, at the pointer or below a widget, closing it first if it is open (a
/// second right-click moves it) and any other context menu (one at a time).
fn open_context(world: &mut World, menu: Entity, thing: Entity, at: At) {
    if world.get_entity(menu).is_err()
        || world.get_entity(thing).is_err()
        || world.get::<MenuPanel>(menu).is_none()
    {
        return;
    }
    let others: Vec<Entity> = world
        .query_filtered::<Entity, (With<MenuPanel>, With<ContextTarget>)>()
        .iter(world)
        .collect();
    for other in others {
        close_menu(world, other);
    }
    if world.get::<PopoverPanel>(menu).is_some() {
        close_menu(world, menu);
    }
    world.entity_mut(menu).insert(ContextTarget(thing));
    match at {
        At::Pointer => show_menu(world, menu, thing, Placement::AtPointer),
        At::Widget(widget) => {
            world.entity_mut(menu).insert(ChildOf(widget));
            show_menu(world, menu, widget, Placement::Below);
        }
    }
}
