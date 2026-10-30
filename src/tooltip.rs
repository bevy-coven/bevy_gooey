//! Tooltips: a line of text shown under a widget while the pointer is over it.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::picking::events::{PointerOut, PointerOver};
use bevy::prelude::*;

use crate::skin::Role;

/// Shows its text in a box under this entity while the pointer is over it.
///
/// Adding it spawns the box as a hidden child and observes the pointer on this entity; the box is
/// styled as [`Role::Tooltip`]. Change the text by editing the box's [`Text`] child, or replace the
/// entity's widgets.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((button("Options"), Tooltip("Sound, controls and display".into())));
/// # }
/// ```
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Debug, Clone)]
#[component(on_add = tooltip_on_add)]
pub struct Tooltip(pub String);

/// Marks the box a [`Tooltip`] spawned.
#[derive(Component)]
struct TooltipBox;

fn tooltip_on_add(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(text) = world.get::<Tooltip>(entity).map(|t| t.0.clone()) else {
        return;
    };
    let mut commands = world.commands();
    let tip = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(100.0),
                left: Val::Px(0.0),
                display: Display::None,
                ..default()
            },
            Role::Tooltip,
            GlobalZIndex(200),
            TooltipBox,
            ChildOf(entity),
            children![(Text::new(text), Role::Label)],
        ))
        .id();
    commands
        .entity(entity)
        .observe(move |_: On<PointerOver>, mut nodes: Query<&mut Node>| {
            if let Ok(mut node) = nodes.get_mut(tip) {
                node.display = Display::Flex;
            }
        })
        .observe(move |_: On<PointerOut>, mut nodes: Query<&mut Node>| {
            if let Ok(mut node) = nodes.get_mut(tip) {
                node.display = Display::None;
            }
        });
}
