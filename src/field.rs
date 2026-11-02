//! Field validation messages: what is wrong with a text row, said under it in words.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use crate::skin::Role;

/// How serious a [`FieldProblem`] is. The words are what say it; the kind picks the second cue,
/// the colour of the field's frame and of the bar beside the message.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FieldProblemKind {
    /// The value is allowed but probably not meant: [`crate::GooeySkin::accent`].
    Warning,
    /// The value cannot be used: [`crate::GooeySkin::danger`].
    #[default]
    Error,
}

/// What is wrong with a text row ([`text_input`](crate::text_input), [`number_field`](crate::number_field),
/// [`color_field`](crate::color_field), a [`vec_field`](crate::vec_field)'s component): put it on
/// the row and `text` is drawn under it in words, with a bar in the kind's colour beside them,
/// and the field's frame takes that colour while the field has no focus (the ring is the ring).
/// Remove it and the message is gone in the same frame; insert another over it and the line
/// changes in place.
///
/// The words are in the text colour on purpose: the skin's danger and accent are held to 3:1
/// against the panel, which a bar and a frame need, and small text needs 4.5:1. The message is
/// what WCAG 3.3.1 asks for; the colour is the cue beside it.
///
/// The message takes its own room under the row (the rows below move down), wraps at the row's
/// width and takes no input and no focus.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     text_input(Some("Name"), "", None),
///     FieldProblem::error("A name is needed"),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[component(on_insert = queue_message_sync, on_remove = queue_message_sync)]
#[require(ProblemRoom)]
pub struct FieldProblem {
    /// What is wrong, and ideally what to do about it ("Use 3 to 20 letters").
    pub text: String,
    /// How serious it is.
    pub kind: FieldProblemKind,
}

impl FieldProblem {
    /// A problem of `kind` with `text`.
    pub fn new(kind: FieldProblemKind, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }

    /// A value that cannot be used.
    pub fn error(text: impl Into<String>) -> Self {
        Self::new(FieldProblemKind::Error, text)
    }

    /// A value that is allowed but probably not meant.
    pub fn warning(text: impl Into<String>) -> Self {
        Self::new(FieldProblemKind::Warning, text)
    }
}

/// How tall the message under a row stands, logical px as last laid out: the room the row keeps
/// below itself so the next row does not sit on the message ([`crate::apply_skin`] writes the row's
/// bottom margin from it).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ProblemRoom(pub(crate) f32);

/// Both hooks queue [`sync_message`] rather than act on what they saw, as a tooltip's do.
fn queue_message_sync(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .queue(move |world: &mut World| sync_message(world, entity));
}

/// Makes the row's message exist exactly while it has a [`FieldProblem`]: spawns it, changes its
/// text in place, or despawns it. Idempotent, so any order of hooks ends in one message.
fn sync_message(world: &mut World, entity: Entity) {
    let Ok(row) = world.get_entity(entity) else {
        return;
    };
    let problem = row.get::<FieldProblem>().map(|p| p.text.clone());
    let message = row
        .get::<Children>()
        .into_iter()
        .flat_map(|children| children.iter())
        .find(|child| world.get::<Role>(*child) == Some(&Role::FieldMessage));
    match (problem, message) {
        (Some(text), None) => {
            world.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Percent(100.0),
                    ..default()
                },
                Role::FieldMessage,
                Pickable::IGNORE,
                ChildOf(entity),
                children![
                    (Node::default(), Role::FieldMark, Pickable::IGNORE),
                    (
                        Node {
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            min_width: Val::Px(0.0),
                            ..default()
                        },
                        Text::new(text),
                        Role::Label,
                        Pickable::IGNORE
                    ),
                ],
            ));
        }
        (Some(text), Some(message)) => {
            let line = world
                .get::<Children>(message)
                .into_iter()
                .flat_map(|children| children.iter())
                .find(|child| world.get::<Text>(*child).is_some());
            if let Some(mut line) = line.and_then(|line| world.get_mut::<Text>(line))
                && line.0 != text
            {
                line.0 = text;
            }
        }
        (None, Some(message)) => {
            world.despawn(message);
            world.entity_mut(entity).remove::<ProblemRoom>();
        }
        (None, None) => {}
    }
}

/// Measures each message so its row keeps the room below itself. Runs in
/// [`crate::GooeySystems::Style`] before [`crate::apply_skin`], which writes the margin.
pub fn update_field_problems(
    mut rows: Query<(&Children, &mut ProblemRoom), With<FieldProblem>>,
    parts: Query<(&Role, &ComputedNode)>,
) {
    for (children, mut room) in &mut rows {
        let height = children
            .iter()
            .filter_map(|child| parts.get(child).ok())
            .find(|(role, _)| **role == Role::FieldMessage)
            .map_or(0.0, |(_, node)| node.size().y * node.inverse_scale_factor());
        if (room.0 - height).abs() > 0.25 {
            room.0 = height;
        }
    }
}
