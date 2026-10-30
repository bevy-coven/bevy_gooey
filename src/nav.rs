//! Keyboard and gamepad navigation: arrows and the D-pad move focus between widgets, the pad's
//! South button activates the focused one. A pad never steers a cursor. An open [`crate::dialog`]
//! holds focus: it takes it when it opens, keeps it, and gives it back when it closes.

use bevy::input_focus::directional_navigation::{
    AutoNavigationConfig, DirectionalNavigation, DirectionalNavigationError, FocusableArea,
};
use bevy::input_focus::navigator::find_best_candidate;
use bevy::input_focus::tab_navigation::{NavAction, TabGroup, TabNavigation};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::math::CompassOctant;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{ComputedNode, ComputedUiTargetCamera, InteractionDisabled, UiGlobalTransform};
use bevy::ui_widgets::Activate;

/// What a [`crate::dialog`] remembers: the widget that had focus when it opened, which gets focus
/// back when the dialog closes, and whether focus has moved in yet. Part of `dialog()`; a game
/// never builds one.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Dialog {
    opener: Option<Entity>,
    entered: bool,
}

/// The widgets directional navigation can land on, with what it needs to place them.
type Navigable = (
    Entity,
    &'static ComputedUiTargetCamera,
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static InheritedVisibility,
);

/// Moves focus to the nearest widget in the direction the arrow keys or any gamepad's D-pad were
/// just pressed (diagonals included). Runs in [`crate::GooeySystems::Navigate`]; no neighbour that
/// way leaves focus where it is.
///
/// Only widgets in the focused one's scope are candidates, the same set Tab cycles: inside an open
/// [`crate::dialog`] (any modal `TabGroup`), that dialog's widgets; elsewhere, every widget outside
/// a modal group. Edges a host set in Bevy's `DirectionalNavigationMap` are followed first.
/// Candidates are scored by Bevy's own `find_best_candidate` with the app's
/// `AutoNavigationConfig`.
pub fn navigate_by_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut nav: DirectionalNavigation,
    config: Res<AutoNavigationConfig>,
    navigable: Query<Navigable, With<AutoDirectionalNavigation>>,
    groups: Query<&TabGroup>,
    parents: Query<&ChildOf>,
) {
    let pressed = |key: KeyCode, pad: GamepadButton| {
        keys.just_pressed(key) || gamepads.iter().any(|g| g.just_pressed(pad))
    };
    let axis = |plus: bool, minus: bool| plus as i8 as f32 - minus as i8 as f32;
    let net_x = axis(
        pressed(KeyCode::ArrowRight, GamepadButton::DPadRight),
        pressed(KeyCode::ArrowLeft, GamepadButton::DPadLeft),
    );
    let net_y = axis(
        pressed(KeyCode::ArrowUp, GamepadButton::DPadUp),
        pressed(KeyCode::ArrowDown, GamepadButton::DPadDown),
    );
    let Some(direction) = Dir2::from_xy(net_x, net_y).ok().map(CompassOctant::from) else {
        return;
    };
    let Some(from) = nav.focus.get() else {
        return;
    };
    // A manual edge either moved focus or blocks this direction; only "no edge" falls through.
    if !matches!(
        nav.navigate(direction),
        Err(DirectionalNavigationError::NoNeighborInDirection { .. })
    ) {
        return;
    }
    let Some((camera, origin)) = navigable.get(from).ok().and_then(focusable_area) else {
        return;
    };
    let scope = modal_scope(from, &groups, &parents);
    let candidates: Vec<FocusableArea> = navigable
        .iter()
        .filter(|(entity, _, node, _, visible)| {
            !node.is_empty()
                && visible.get()
                && modal_scope(*entity, &groups, &parents) == scope
        })
        .filter_map(focusable_area)
        .filter_map(|(c, area)| (c == camera).then_some(area))
        .collect();
    if let Some(to) = find_best_candidate(&origin, direction, &candidates, &config) {
        nav.focus.set(to, FocusCause::Navigated);
    }
}

/// The modal `TabGroup` an entity is inside, `None` outside every modal group. Like Tab, only the
/// nearest group counts.
fn modal_scope(
    entity: Entity,
    groups: &Query<&TabGroup>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    parents
        .iter_ancestors(entity)
        .find_map(|a| groups.get(a).ok().map(|g| (a, g.modal)))
        .and_then(|(a, modal)| modal.then_some(a))
}

/// A navigable node's camera and its area in logical pixels, rotation included, as Bevy's own
/// `AutoDirectionalNavigator` measures it.
fn focusable_area(
    (entity, camera, node, transform, _): (
        Entity,
        &ComputedUiTargetCamera,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    ),
) -> Option<(Entity, FocusableArea)> {
    let (scale, rotation, translation) = transform.to_scale_angle_translation();
    let size = node.size() * node.inverse_scale_factor() * scale;
    let (sin, cos) = (rotation.sin().abs(), rotation.cos().abs());
    Some((
        camera.get()?,
        FocusableArea {
            entity,
            position: translation * node.inverse_scale_factor(),
            size: Vec2::new(size.x * cos + size.y * sin, size.x * sin + size.y * cos),
        },
    ))
}

/// Remembers who had focus when a dialog opened. An observer on [`Dialog`]'s `Add`, which runs
/// before the dialog's children spawn, so an `AutoFocus` inside it has not taken focus yet.
pub(crate) fn remember_opener(
    add: On<Add<Dialog>>,
    focus: Option<Res<InputFocus>>,
    mut dialogs: Query<&mut Dialog>,
) {
    if let (Some(focus), Ok(mut dialog)) = (focus, dialogs.get_mut(add.entity)) {
        dialog.opener = focus.get();
    }
}

/// Moves focus into a dialog that just opened, to its first widget in tab order, unless focus is
/// already inside it (an `AutoFocus` child picked the default). Runs in
/// [`crate::GooeySystems::Navigate`]. A dialog spawned this frame is not in Tab order until its
/// visibility propagates, so a hidden one is tried again next frame; a visible one with nothing to
/// focus is left alone.
pub fn focus_opened_dialogs(
    mut dialogs: Query<(Entity, &mut Dialog, &InheritedVisibility)>,
    mut focus: ResMut<InputFocus>,
    tabs: TabNavigation,
    parents: Query<&ChildOf>,
) {
    for (entity, mut dialog, visible) in &mut dialogs {
        if dialog.entered {
            continue;
        }
        if !inside(focus.get(), entity, &parents)
            && let Ok(first) = tabs.initialize(entity, NavAction::First)
        {
            focus.set(first, FocusCause::Navigated);
        }
        dialog.entered = visible.get() || inside(focus.get(), entity, &parents);
    }
}

/// Gives focus back to the dialog's opener when it closes, if focus was inside the dialog (or went
/// with it). A host that moved focus elsewhere first keeps its choice.
pub(crate) fn return_focus(
    remove: On<Remove<Dialog>>,
    dialogs: Query<&Dialog>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    alive: Query<()>,
) {
    let Some(opener) = dialogs
        .get(remove.entity)
        .ok()
        .and_then(|d| d.opener)
        .filter(|&o| alive.contains(o))
    else {
        return;
    };
    let current = focus.get();
    if current.is_none_or(|f| !alive.contains(f)) || inside(current, remove.entity, &parents) {
        focus.set(opener, FocusCause::Navigated);
    }
}

/// Whether `focus` is `root` or one of its descendants.
fn inside(focus: Option<Entity>, root: Entity, parents: &Query<&ChildOf>) -> bool {
    focus.is_some_and(|f| f == root || parents.iter_ancestors(f).any(|a| a == root))
}

/// Activates the focused, enabled button when any gamepad's South button (A on Xbox, Cross on
/// PlayStation) is just pressed. Enter and Space are `bevy_ui_widgets`' own, so they are not
/// handled here: a second handler would activate twice.
pub fn gamepad_select(
    focus: Res<InputFocus>,
    gamepads: Query<&Gamepad>,
    buttons: Query<(), (With<bevy::ui_widgets::Button>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if !gamepads
        .iter()
        .any(|g| g.just_pressed(GamepadButton::South))
    {
        return;
    }
    if let Some(entity) = focus.get()
        && buttons.contains(entity)
    {
        commands.trigger(Activate { entity });
    }
}
