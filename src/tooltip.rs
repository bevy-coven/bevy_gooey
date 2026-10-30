//! Tooltips: a line of text beside a widget while the pointer is over it or it has visible focus.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::popover::Popover;

use crate::skin::Role;

/// Shows its text in a box beside this entity while the pointer is over it, or while it has focus
/// and focus is visible (a keyboard or pad player has moved it), so every input reaches it.
///
/// The box opens across the flow of the entity's parent, one [`crate::GooeySkin::gap`] away: to
/// the right of a widget in a column, below one in a row, on the other side if the window would
/// clip it, so it never covers the next widget in a menu. Adding it spawns the box as a hidden
/// child, styled as [`Role::Tooltip`] and placed by Bevy's `Popover`. Change the text by editing
/// the box's [`Text`] child, or replace the entity's widgets.
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

/// The box a [`Tooltip`] spawned, a child of the widget it describes: query it to find the box
/// (its [`Text`] is its child).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct TooltipBox {
    /// The widget's parent lays its children out in a row, so the box opens below or above it
    /// rather than beside it. [`crate::apply_skin`] places the box from it.
    pub(crate) in_row: bool,
}

fn tooltip_on_add(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(text) = world.get::<Tooltip>(entity).map(|t| t.0.clone()) else {
        return;
    };
    let mut commands = world.commands();
    // Hover is read from `Hovered`, which `button()` already carries; any other widget gets one.
    commands.entity(entity).insert_if_new(Hovered::default());
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        Role::Tooltip,
        Popover::default(),
        GlobalZIndex(200),
        TooltipBox::default(),
        ChildOf(entity),
        children![(Text::new(text), Role::Label)],
    ));
}

/// Shows each [`Tooltip`] box while its widget is hovered or has visible focus, and hides it
/// otherwise; records whether the widget sits in a row, for the box's placement. Writes only what
/// changed. Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`].
pub fn show_tooltips(
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
    widgets: Query<(Option<&Hovered>, Option<&ChildOf>), Without<TooltipBox>>,
    flows: Query<&Node, Without<TooltipBox>>,
    mut boxes: Query<(&ChildOf, &mut Node, &mut TooltipBox)>,
) {
    let focused = visible
        .is_some_and(|v| v.0)
        .then(|| focus.and_then(|f| f.get()))
        .flatten();
    for (owner, mut node, mut tip) in &mut boxes {
        let widget = owner.parent();
        let Ok((hovered, parent)) = widgets.get(widget) else {
            continue;
        };
        let shown = hovered.is_some_and(|h| h.get()) || focused == Some(widget);
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let in_row = parent
            .and_then(|p| flows.get(p.parent()).ok())
            .is_some_and(|n| {
                matches!(
                    n.flex_direction,
                    FlexDirection::Row | FlexDirection::RowReverse
                )
            });
        tip.set_if_neq(TooltipBox { in_row });
    }
}
