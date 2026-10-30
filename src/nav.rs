//! Keyboard and gamepad navigation: arrows and the D-pad move focus between widgets, the pad's
//! South button activates the focused one. A pad never steers a cursor. Keys and the pad show
//! focus (`InputFocusVisible`), a pointer press hides it, and a direction press with nothing
//! focused lands where the screen starts. An open [`crate::dialog`] holds focus: it takes it when
//! it opens, keeps it, and gives it back when it closes.

use bevy::ecs::system::SystemParam;
use bevy::input_focus::directional_navigation::{
    AutoNavigationConfig, DirectionalNavigation, DirectionalNavigationError, FocusableArea,
};
use bevy::input_focus::navigator::find_best_candidate;
use bevy::input_focus::tab_navigation::{NavAction, TabGroup, TabIndex, TabNavigation};
use bevy::input_focus::{AutoFocus, FocusCause, InputFocus, InputFocusVisible};
use bevy::math::CompassOctant;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{
    ComputedNode, ComputedStackIndex, ComputedUiTargetCamera, InteractionDisabled,
    UiGlobalTransform,
};
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

/// Where focus may go: the tab stops and groups Bevy's Tab navigation sees, which group is an
/// open dialog, and where each group starts. Part of [`navigate_by_input`]'s parameters.
#[derive(SystemParam)]
pub struct FocusScopes<'w, 's> {
    groups: Query<'w, 's, &'static TabGroup>,
    parents: Query<'w, 's, &'static ChildOf>,
    tabs: TabNavigation<'w, 's>,
    /// Tab groups with their visibility and place in the UI stack, to find the dialog on top.
    layers: Query<
        'w,
        's,
        (
            Entity,
            &'static TabGroup,
            &'static InheritedVisibility,
            &'static ComputedStackIndex,
        ),
    >,
    /// The widgets a host marked as where their screen or dialog starts.
    starts: Query<
        'w,
        's,
        (
            Entity,
            &'static InheritedVisibility,
            &'static ComputedStackIndex,
        ),
        (With<AutoFocus>, With<TabIndex>),
    >,
    /// Everything focus can rest on.
    stops: Query<'w, 's, Option<&'static InheritedVisibility>, With<TabIndex>>,
}

impl FocusScopes<'_, '_> {
    /// The modal `TabGroup` an entity is inside, `None` outside every modal group. Like Tab, only
    /// the nearest group counts.
    fn of(&self, entity: Entity) -> Option<Entity> {
        self.parents
            .iter_ancestors(entity)
            .find_map(|a| self.groups.get(a).ok().map(|g| (a, g.modal)))
            .and_then(|(a, modal)| modal.then_some(a))
    }

    /// Whether focus on `entity` is somewhere to move from: a live, visible tab stop. Not so for
    /// nothing, a despawned or hidden widget, or the window Bevy focuses at start-up.
    fn holds(&self, entity: Entity) -> bool {
        self.stops
            .get(entity)
            .is_ok_and(|visible| visible.is_none_or(|v| v.get()))
    }

    /// The scope the player is looking at: the topmost open modal group (a dialog), or `None`,
    /// everything outside modal groups.
    fn active(&self) -> Option<Entity> {
        self.layers
            .iter()
            .filter(|(_, group, visible, _)| group.modal && visible.get())
            .max_by_key(|(.., stack)| stack.0)
            .map(|(entity, ..)| entity)
    }

    /// Where focus starts in `scope`: its topmost visible `AutoFocus` widget, or else its first
    /// widget in Tab order.
    fn start(&self, scope: Option<Entity>) -> Option<Entity> {
        self.starts
            .iter()
            .filter(|&(entity, visible, _)| visible.get() && self.of(entity) == scope)
            .max_by_key(|(_, _, stack)| stack.0)
            .map(|(entity, ..)| entity)
            .or_else(|| match scope {
                Some(group) => self.tabs.initialize(group, NavAction::First).ok(),
                None => self
                    .tabs
                    .navigate(&InputFocus::default(), NavAction::First)
                    .ok(),
            })
    }
}

/// Shows the focus indicator: the player is on a key or the pad. Writes `InputFocusVisible` only
/// when it changes (reading through `ResMut` does not mark it changed), so the skin does not
/// restyle every widget on every press.
fn show_focus(shown: &mut ResMut<InputFocusVisible>) {
    if !shown.0 {
        shown.0 = true;
    }
}

/// Moves focus to the nearest widget in the direction the arrow keys or any gamepad's D-pad were
/// just pressed (diagonals included), and shows focus (`InputFocusVisible`), which a pointer press
/// hides. Runs in [`crate::GooeySystems::Navigate`]; no neighbour that way leaves focus where it
/// is.
///
/// Only widgets in the focused one's scope are candidates, the same set Tab cycles: inside an open
/// [`crate::dialog`] (any modal `TabGroup`), that dialog's widgets; elsewhere, every widget outside
/// a modal group. Edges a host set in Bevy's `DirectionalNavigationMap` are followed first.
/// Candidates are scored by Bevy's own `find_best_candidate` with the app's
/// `AutoNavigationConfig`.
///
/// With nothing to move from (no focus, a click on empty space cleared it, the focused widget was
/// despawned or hidden), the press lands where the scope the player sees starts: the open dialog
/// on top, or else the screen; its `AutoFocus` widget, or else its first widget in Tab order. A
/// focused tab stop without `AutoDirectionalNavigation` (a text field) keeps its arrows.
///
/// The D-pad always navigates; a game whose pad also drives play gates the set, e.g.
/// `configure_sets(Update, GooeySystems::Navigate.run_if(in_state(Menu)))`.
pub fn navigate_by_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut nav: DirectionalNavigation,
    mut shown: ResMut<InputFocusVisible>,
    config: Res<AutoNavigationConfig>,
    navigable: Query<Navigable, With<AutoDirectionalNavigation>>,
    scopes: FocusScopes,
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
    show_focus(&mut shown);
    let Some(from) = nav.focus.get().filter(|&f| scopes.holds(f)) else {
        if let Some(start) = scopes.start(scopes.active()) {
            nav.focus.set(start, FocusCause::Navigated);
        }
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
    let scope = scopes.of(from);
    let candidates: Vec<FocusableArea> = navigable
        .iter()
        .filter(|(entity, _, node, _, visible)| {
            !node.is_empty() && visible.get() && scopes.of(*entity) == scope
        })
        .filter_map(focusable_area)
        .filter_map(|(c, area)| (c == camera).then_some(area))
        .collect();
    if let Some(to) = find_best_candidate(&origin, direction, &candidates, &config) {
        nav.focus.set(to, FocusCause::Navigated);
    }
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
/// already inside it (an `AutoFocus` child picked the default), and shows focus
/// (`InputFocusVisible`), so a dialog opened with the pointer still shows which button Enter or
/// South will press. Runs in [`crate::GooeySystems::Navigate`]. A dialog spawned this frame is not
/// in Tab order until its visibility propagates, so a hidden one is tried again next frame; a
/// visible one with nothing to focus is left alone.
pub fn focus_opened_dialogs(
    mut dialogs: Query<(Entity, &mut Dialog, &InheritedVisibility)>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
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
        let holds = inside(focus.get(), entity, &parents);
        dialog.entered = visible.get() || holds;
        if holds {
            show_focus(&mut shown);
        }
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
/// PlayStation) is just pressed, and shows focus (`InputFocusVisible`). Enter and Space are
/// `bevy_ui_widgets`' own, so they are not handled here: a second handler would activate twice.
pub fn gamepad_select(
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
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
    show_focus(&mut shown);
    if let Some(entity) = focus.get()
        && buttons.contains(entity)
    {
        commands.trigger(Activate { entity });
    }
}
