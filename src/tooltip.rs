//! Tooltips: a line of text beside a widget while the pointer is over it or it has visible focus.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{ComputedUiRenderTargetInfo, OverrideClip, UiGlobalTransform};
use bevy::ui_widgets::popover::Popover;

use crate::skin::Role;
use crate::text::TextInputOf;

/// Shows its text in a box beside this entity while the pointer is over it, or while it has focus
/// and focus is visible (a keyboard or pad player has moved it), so every input reaches it.
///
/// The box opens across the flow of the entity's parent: to the right of a widget in a column,
/// below one in a row, on the other side if the window would clip it, so it never covers the next
/// widget in a menu; a long line wraps to fit the room beside the widget. It sits one
/// [`crate::GooeySkin::gap`] outside the panel the widget is in (its padding and border), drawn as
/// a small raised panel with a shadow. Adding it spawns the box as a hidden child, styled as
/// [`Role::Tooltip`] and placed by Bevy's `Popover`.
///
/// The box is a view of this component: removing it removes the box, and inserting a new
/// `Tooltip` over this one, or editing its text in place, changes the line the box shows (an
/// open tooltip keeps its place and does not blink; [`update_tooltip_text`] copies the text).
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
#[component(on_insert = queue_box_sync, on_remove = queue_box_sync)]
pub struct Tooltip(pub String);

/// The box a [`Tooltip`] spawned, a child of the widget it describes: query it to find the box
/// (its [`Text`] is its child).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct TooltipBox {
    /// The widget's parent lays its children out in a row, so the box opens below or above it
    /// rather than beside it. [`crate::apply_skin`] places the box from it.
    pub(crate) in_row: bool,
    /// The widest gap between the widget and the window's left or right edge, logical px:
    /// [`crate::apply_skin`] wraps the box to fit in it.
    pub(crate) room: f32,
}

/// Both hooks queue [`sync_box`] rather than act on what they saw: a flush that removes a
/// `Tooltip` and inserts another fires two hooks, and the command reads the entity when it runs.
fn queue_box_sync(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .queue(move |world: &mut World| sync_box(world, entity));
}

/// Makes the entity's [`TooltipBox`] exist exactly while it has a [`Tooltip`]: spawns one when
/// there is none, despawns it when the `Tooltip` is gone, and does nothing when the entity itself
/// is gone (Bevy has despawned its children). Idempotent, so any order of hooks ends in one box.
fn sync_box(world: &mut World, entity: Entity) {
    let Ok(owner) = world.get_entity(entity) else {
        return;
    };
    let text = owner.get::<Tooltip>().map(|tip| tip.0.clone());
    let boxes: Vec<Entity> = owner
        .get::<Children>()
        .into_iter()
        .flat_map(|children| children.iter())
        .filter(|child| world.get::<TooltipBox>(*child).is_some())
        .collect();
    match (text, boxes.is_empty()) {
        (Some(text), true) => {
            // Hover is read from `Hovered`, which `button()` already carries; any other widget
            // gets one.
            world.entity_mut(entity).insert_if_new(Hovered::default());
            world.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                Role::Tooltip,
                Popover::default(),
                // A tooltip beside a row of a scrolling list stands outside the list, so the
                // list's clip must not reach it.
                OverrideClip,
                BoxShadow::default(),
                GlobalZIndex(200),
                TooltipBox::default(),
                ChildOf(entity),
                children![(Text::new(text), Role::Label)],
            ));
        }
        (None, false) => {
            for tip in boxes {
                world.entity_mut(tip).despawn();
            }
        }
        _ => {}
    }
}

/// Copies each [`Tooltip`]'s text into its box when the `Tooltip` changed: a new `Tooltip`
/// inserted over the old one, or an edit in place (a reflection-driven inspector makes one, and
/// no component hook runs for it). Writes only when the line differs. Runs in
/// [`crate::GooeySystems::Style`], before [`show_tooltips`].
pub fn update_tooltip_text(
    tooltips: Query<(&Tooltip, &Children), Changed<Tooltip>>,
    boxes: Query<&Children, With<TooltipBox>>,
    mut texts: Query<&mut Text>,
) {
    for (tooltip, children) in &tooltips {
        for lines in children.iter().filter_map(|child| boxes.get(child).ok()) {
            for line in lines {
                if let Ok(mut text) = texts.get_mut(*line)
                    && text.0 != tooltip.0
                {
                    text.0.clone_from(&tooltip.0);
                }
            }
        }
    }
}

/// Shows each [`Tooltip`] box while its widget is hovered or has visible focus (a
/// [`crate::text_input`] row has it while its field does), and hides it otherwise; records whether
/// the widget sits in a row and how much room the window leaves beside it, for
/// [`crate::apply_skin`] to place and fit the box. Writes only what changed. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`].
pub fn show_tooltips(
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
    widgets: Query<Described, Without<TooltipBox>>,
    flows: Query<&Node, Without<TooltipBox>>,
    fields: Query<&TextInputOf>,
    mut boxes: Query<(&ChildOf, &mut Node, &mut TooltipBox)>,
) {
    let focused = visible
        .is_some_and(|v| v.0)
        .then(|| focus.and_then(|f| f.get()))
        .flatten();
    let focused_row = focused.and_then(|f| fields.get(f).ok()).map(|of| of.0);
    for (owner, mut node, mut tip) in &mut boxes {
        let widget = owner.parent();
        let Ok((hovered, parent, placed)) = widgets.get(widget) else {
            continue;
        };
        let shown = hovered.is_some_and(|h| h.get())
            || focused == Some(widget)
            || focused_row == Some(widget);
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
        let room = placed.map_or(f32::INFINITY, |(computed, at, target)| {
            let scale = computed.inverse_scale_factor;
            let rect = Rect::from_center_size(at.translation * scale, computed.size() * scale);
            let window = target.logical_size();
            // Whole pixels, so sub-pixel layout noise never restyles the box.
            (window.x - rect.max.x).max(rect.min.x).floor()
        });
        tip.set_if_neq(TooltipBox { in_row, room });
    }
}

/// A widget a tooltip describes: whether the pointer is over it, its parent, and where it is laid
/// out (its size, its position and the window it is drawn to).
type Described = (
    Option<&'static Hovered>,
    Option<&'static ChildOf>,
    Option<(
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static ComputedUiRenderTargetInfo,
    )>,
);
