//! Collapsible sections: the state ([`SectionOpen`], [`SectionId`], [`SectionStates`]) and the
//! systems of [`crate::section`]. Its look is [`crate::apply_skin`]'s.

use std::collections::BTreeMap;

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled};
use bevy::ui_widgets::{Activate, ValueChange};

use crate::motion::MotionSettings;
use crate::size_motion::SectionGlide;
use crate::skin::Role;

/// Whether a [`crate::section`] is open: the widget's state. Read it to know, set it to open or
/// close the section from code (the body grows or shrinks to it from that frame, on the skin's
/// default spring, or at once under reduced motion; no [`ValueChange`] is sent for a change the game
/// made itself). The player's Enter, Space, South or click sends one
/// [`ValueChange<bool>`](ValueChange) on the section and flips it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn fold(mut sections: Query<&mut SectionOpen>) {
///     for mut open in &mut sections {
///         open.0 = false;
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct SectionOpen(pub bool);

/// A name that keeps a [`crate::section`]'s state across a despawn and a respawn: put it on the
/// section's entity (`(section("Physics", true, ..), SectionId::new("physics"))`) and the section
/// is open or closed as the player last left the section of that name, whatever the `open` it was
/// spawned with (that is only its first choice). Every section with the same id shares the state,
/// so an inspector that is rebuilt for another thing and then this one again comes back as it was.
/// The states are in [`SectionStates`].
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Debug, PartialEq, Hash, Clone)]
pub struct SectionId(pub String);

impl SectionId {
    /// An id from any string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// The last state of every [`SectionId`]: a section with an id writes it when it changes and a new
/// section of the same id starts from it. It is plain data (with the `serde` feature, serde data),
/// so a game can save it with the player's settings and insert it again at start-up, and clear it
/// to forget what the player folded.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn remembered(states: Res<SectionStates>) -> Option<bool> {
///     states.open("physics")
/// }
/// ```
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionStates {
    open: BTreeMap<String, bool>,
}

impl SectionStates {
    /// What the player last left the section called `id` as: `Some(true)` open, `Some(false)`
    /// closed, `None` never seen.
    pub fn open(&self, id: &str) -> Option<bool> {
        self.open.get(id).copied()
    }

    /// Sets what the section called `id` starts as the next time one is spawned.
    pub fn set(&mut self, id: impl Into<String>, open: bool) {
        self.open.insert(id.into(), open);
    }

    /// Forgets every state: the next sections start as the game spawns them.
    pub fn clear(&mut self) {
        self.open.clear();
    }
}

/// On a section's header, the button that opens and closes it. Part of what [`crate::section`]
/// spawns; query for it to find the header of a section.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct SectionHeader;

/// The header was activated (Enter, Space, South, a click): ask for the opposite state.
pub(crate) fn header_on_activate(
    activate: On<Activate>,
    parents: Query<&ChildOf>,
    sections: Query<&SectionOpen, Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok(parent) = parents.get(activate.entity) else {
        return;
    };
    if let Ok(open) = sections.get(parent.parent()) {
        commands.trigger(ValueChange {
            source: parent.parent(),
            value: !open.0,
            is_final: true,
        });
    }
}

/// Keeps the state: a [`ValueChange<bool>`] on a section becomes its [`SectionOpen`]. The
/// section's own observer, like a choice's.
pub(crate) fn section_self_update(
    change: On<ValueChange<bool>>,
    mut sections: Query<&mut SectionOpen>,
) {
    if let Ok(mut open) = sections.get_mut(change.source) {
        open.set_if_neq(SectionOpen(change.value));
    }
}

/// Keeps every section as its state says: the id's remembered state, the body laid out or not, the
/// chevron's direction, "Empty" for an open section with nothing in it, and focus kept out of a
/// closed body. Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`]: the frame
/// the state changes, from the player or from the game; a section whose state did not change
/// writes nothing.
///
/// A closed body is `Display::None` (no layout) and `Visibility::Hidden` (Bevy's Tab navigation and
/// the spatial navigation skip what is not visible): `Display::None` alone leaves a hidden widget a
/// stop.
#[allow(clippy::too_many_arguments)]
pub fn update_sections(
    mut sections: Query<(&mut SectionOpen, Option<&SectionId>, &Children)>,
    headers: Query<&Children, With<SectionHeader>>,
    roles: Query<&Role>,
    bodies: Query<Option<&Children>, With<Role>>,
    mut nodes: Query<&mut Node>,
    mut visibility: Query<&mut Visibility>,
    parents: Query<&ChildOf>,
    states: Option<ResMut<SectionStates>>,
    mut focus: ResMut<InputFocus>,
    motion: Res<MotionSettings>,
    glides: Query<&SectionGlide>,
    laid_out: Query<&ComputedNode>,
    mut commands: Commands,
) {
    let mut states = states;
    for (mut open, id, children) in &mut sections {
        if let (Some(id), Some(states)) = (id, states.as_deref_mut()) {
            if open.is_added() {
                match states.open(&id.0) {
                    Some(saved) if saved != open.0 => open.0 = saved,
                    Some(_) => {}
                    None => states.set(id.0.clone(), open.0),
                }
            } else if open.is_changed() && states.open(&id.0) != Some(open.0) {
                states.set(id.0.clone(), open.0);
            }
        }
        let is_open = open.0;
        let body = children
            .iter()
            .find(|&child| roles.get(child) == Ok(&Role::SectionBody));
        let empty = body.is_some_and(|body| {
            bodies
                .get(body)
                .is_ok_and(|kids| kids.is_none_or(|kids| kids.is_empty()))
        });
        for &child in children {
            if let Ok(header) = headers.get(child) {
                for &part in header {
                    let shown = match roles.get(part) {
                        Ok(Role::TreeClosed) => !is_open,
                        Ok(Role::TreeOpen) => is_open,
                        _ => continue,
                    };
                    show(part, shown, &mut nodes, None);
                }
            } else {
                match roles.get(child) {
                    Ok(Role::SectionBody) => {
                        // A player's or a game's toggle after the first frame grows or shrinks the
                        // body (size motion); the state is the new one at once.
                        // A write that keeps the value (a game setting every section each frame) is
                        // no toggle: only a body shown or hidden against the state starts a glide.
                        let shown_now = nodes.get(child).is_ok_and(|n| n.display != Display::None);
                        let toggled = open.is_changed() && !open.is_added() && shown_now != is_open;
                        let glide = glides.get(child).ok().cloned();
                        match glide {
                            Some(glide) if glide.opening() != is_open => {
                                commands.entity(child).insert(glide.turned(is_open));
                            }
                            Some(_) => {}
                            None if toggled && !motion.lands_travel() => {
                                let Ok(node) = nodes.get(child).cloned() else {
                                    continue;
                                };
                                let from = if is_open {
                                    0.0
                                } else {
                                    laid_out
                                        .get(child)
                                        .map_or(0.0, |c| c.size().y * c.inverse_scale_factor())
                                };
                                commands
                                    .entity(child)
                                    .insert(SectionGlide::new(from, is_open, &node));
                                if is_open {
                                    show(child, true, &mut nodes, Some(&mut visibility));
                                }
                            }
                            None => show(child, is_open, &mut nodes, Some(&mut visibility)),
                        }
                    }
                    Ok(Role::SectionNote) => show(child, is_open && empty, &mut nodes, None),
                    _ => {}
                }
            }
        }
        if !is_open
            && let Some(body) = body
            && let Some(focused) = focus.get()
            && parents.iter_ancestors(focused).any(|a| a == body)
            && let Some(header) = children.iter().find(|&c| headers.contains(c))
        {
            focus.set(header, FocusCause::Navigated);
        }
    }
}

/// Lays `entity` out (`Display::Flex`) or takes it out (`Display::None`), and shows or hides it
/// with `visibility`, writing only a difference.
fn show(
    entity: Entity,
    shown: bool,
    nodes: &mut Query<&mut Node>,
    visibility: Option<&mut Query<&mut Visibility>>,
) {
    if let Ok(mut node) = nodes.get_mut(entity) {
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    if let Some(visibility) = visibility
        && let Ok(mut visible) = visibility.get_mut(entity)
    {
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visible != want {
            *visible = want;
        }
    }
}
