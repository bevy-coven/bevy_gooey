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
use bevy::ui_widgets::{
    Activate, Checkbox, SetSliderValue, Slider, SliderValueChange, Tab, TextInput, ToggleChecked,
};

use crate::choice::{ChoiceCount, Move, MoveChoice};
use crate::list::{ListView, span};
use crate::menu::{MenuItem, MenuTitle};
use crate::model::{PAD_REPEAT_DELAY, PAD_REPEAT_RATE, repeat_steps};
use crate::popover::PopoverPanel;
use crate::skin::Role;
use crate::tabs::{TabsCount, TabsValue};

/// The widgets that take Left and Right themselves: an enabled slider or choice.
type TakesSideways = (
    Or<(With<Slider>, With<ChoiceCount>)>,
    Without<InteractionDisabled>,
);

/// The widgets whose Left and Right never also move focus: [`TakesSideways`] and a tab, whose keys
/// are Bevy's and whose D-pad is [`crate::step_tabs`]'.
type KeepsSideways = (
    Or<(With<Slider>, With<ChoiceCount>, With<Tab>)>,
    Without<InteractionDisabled>,
);

/// The text fields that take Left and Right as the caret's: an enabled one. Only the keys are the
/// caret's (a pad cannot move one), so the D-pad still moves focus off it.
type TypesText = (With<TextInput>, Without<InteractionDisabled>);

/// The tabs widgets focus can be turned from: every enabled one.
type TurnableTabs = (With<TabsValue>, Without<InteractionDisabled>);

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

/// A widget a screen or dialog may start on: where it is in the UI stack and whether it shows.
type StartPoint = (
    Entity,
    &'static InheritedVisibility,
    &'static ComputedStackIndex,
);

/// What a host puts on the widget its screen or dialog starts on: Bevy's `AutoFocus` on a tab stop.
type StartMark = (With<AutoFocus>, With<TabIndex>);

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
    /// The popovers open: a `Cancel` from the widget one is anchored to reaches the popover.
    popovers: Query<
        'w,
        's,
        (
            Entity,
            &'static PopoverPanel,
            &'static InheritedVisibility,
            &'static ComputedStackIndex,
        ),
    >,
    /// The widgets a host marked as where their screen or dialog starts.
    starts: Query<'w, 's, StartPoint, StartMark>,
    /// Everything focus can rest on.
    stops: Query<'w, 's, Option<&'static InheritedVisibility>, With<TabIndex>>,
    /// The sliders, choices and tabs that take Left and Right themselves.
    sideways: Query<'w, 's, (), KeepsSideways>,
    /// Menu titles and items, whose Left, Right, Home and End are [`crate::menu_by_input`]'s.
    menu_rows: Query<'w, 's, (), Or<(With<MenuTitle>, With<MenuItem>)>>,
    /// Menu titles, whose Down opens their menu.
    menu_titles: Query<'w, 's, (), With<MenuTitle>>,
    /// The text fields, which take the keys Left and Right themselves.
    typing: Query<'w, 's, (), TypesText>,
    /// The lists, to tell a widget one has clipped away from one in view.
    lists: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform), With<ListView>>,
    /// A tab strip's tabs, and the tab widgets that hold them.
    tab_stops: Query<'w, 's, &'static ChildOf, With<Tab>>,
    tab_roots: Query<'w, 's, (Entity, &'static InheritedVisibility), TurnableTabs>,
    tab_values: Query<'w, 's, (&'static TabsValue, &'static TabsCount)>,
    kids: Query<'w, 's, &'static Children>,
    roles: Query<'w, 's, &'static Role>,
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

    /// Whether `entity` is a slider or a choice that takes Left and Right itself: those presses
    /// adjust it and never move focus off it. A disabled one takes nothing.
    fn keeps_sideways(&self, entity: Entity) -> bool {
        self.menu_rows.contains(entity) || self.keeps_own_sideways(entity)
    }

    fn keeps_own_sideways(&self, entity: Entity) -> bool {
        self.sideways.contains(entity)
    }

    /// Whether `entity` is a text field whose caret takes the Left and Right keys. A disabled
    /// one takes nothing.
    fn types_text(&self, entity: Entity) -> bool {
        self.typing.contains(entity)
    }

    /// Whether `entity` is one of a tab strip's tabs.
    pub(crate) fn is_tab(&self, entity: Entity) -> bool {
        self.tab_stops.contains(entity)
    }

    /// The tabs widget a bumper or Ctrl+PageUp turns for focus on `from`: the nearest one `from` is
    /// inside, or else the only visible one in `from`'s scope; none when there is none or two.
    pub(crate) fn tabs_for(&self, from: Entity) -> Option<Entity> {
        if let Some(root) = self
            .parents
            .iter_ancestors(from)
            .find(|&a| self.tab_roots.contains(a))
        {
            return Some(root);
        }
        if !self.holds(from) {
            return None;
        }
        let scope = self.of(from);
        let mut here = self
            .tab_roots
            .iter()
            .filter(|&(root, visible)| visible.get() && self.of(root) == scope)
            .map(|(root, _)| root);
        let only = here.next()?;
        here.next().is_none().then_some(only)
    }

    /// A tabs widget's selected tab (clamped to its last) and how many tabs it has.
    pub(crate) fn tabs_state(&self, root: Entity) -> Option<(usize, usize)> {
        let (value, count) = self.tab_values.get(root).ok()?;
        Some((value.0.min(count.0.saturating_sub(1)), count.0))
    }

    /// The `index`th tab of a tabs widget.
    pub(crate) fn tab_at(&self, root: Entity, index: usize) -> Option<Entity> {
        let strip = self
            .kids
            .get(root)
            .ok()?
            .iter()
            .find(|&kid| self.roles.get(kid) == Ok(&Role::TabList))?;
        self.kids.get(strip).ok()?.get(index).copied()
    }

    /// Where focus lands when it reaches the tab `to` from `from`: the strip's selected tab, when
    /// `from` is not one of the same strip's tabs (APG: focus moving into the tab list "places focus
    /// on the active tab element"), else `to`.
    fn landing(&self, from: Entity, to: Entity) -> Entity {
        let Ok(strip) = self.tab_stops.get(to).map(ChildOf::parent) else {
            return to;
        };
        if self
            .tab_stops
            .get(from)
            .is_ok_and(|tab| tab.parent() == strip)
        {
            return to;
        }
        self.parents
            .get(strip)
            .ok()
            .and_then(|root| {
                let (selected, _) = self.tabs_state(root.parent())?;
                self.tab_at(root.parent(), selected)
            })
            .unwrap_or(to)
    }

    /// The list `entity` is a row of, if any: its nearest ancestor with a [`crate::ListView`].
    fn list_of(&self, entity: Entity) -> Option<Entity> {
        self.parents
            .iter_ancestors(entity)
            .find(|&a| self.lists.contains(a))
    }

    /// Whether a list has scrolled `candidate` out of sight: it stands in a list that focus (`from`)
    /// is not in, and none of it is inside that list's viewport. Such a row is no neighbour from
    /// outside: the button below a list goes Up to the last row the player sees, not to a row that
    /// happens to lie nearer, clipped. Inside its own list focus does reach the next row, and the
    /// list scrolls to bring it in ([`crate::scroll_to_focus`]).
    fn hides(
        &self,
        candidate: Entity,
        node: &ComputedNode,
        transform: &UiGlobalTransform,
        from: Entity,
    ) -> bool {
        let Some(list) = self.list_of(candidate) else {
            return false;
        };
        if self.list_of(from) == Some(list) {
            return false;
        }
        let Ok((view, view_transform)) = self.lists.get(list) else {
            return false;
        };
        let (top, height) = span(node, transform);
        let (view_top, view_height) = span(view, view_transform);
        top >= view_top + view_height || top + height <= view_top
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

    /// Where a [`crate::Cancel`] starts: the focused widget when it is a live, visible tab stop in
    /// the scope the player sees, else that scope's open dialog (so a dialog can always be
    /// dismissed), else nowhere.
    pub(crate) fn cancel_target(&self, focus: Option<Entity>) -> Option<Entity> {
        let dialog = self.active();
        focus
            .filter(|&f| self.holds(f) && self.of(f) == dialog)
            .map(|f| self.popover_of(f).unwrap_or(f))
            .or(dialog)
    }

    /// The topmost open, non-modal popover a `Cancel` from focus on `focus` should close: the one
    /// anchored to `focus` or to a widget it is inside, when `focus` is not already inside it (a
    /// press from inside reaches the panel by itself, and a modal one is its own scope).
    fn popover_of(&self, focus: Entity) -> Option<Entity> {
        self.popovers
            .iter()
            .filter(|(entity, panel, visible, _)| {
                !panel.modal
                    && visible.get()
                    && !self.parents.iter_ancestors(focus).any(|a| a == *entity)
                    && (focus == panel.anchor
                        || self
                            .parents
                            .iter_ancestors(focus)
                            .any(|a| a == panel.anchor))
            })
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
pub(crate) fn show_focus(shown: &mut ResMut<InputFocusVisible>) {
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
    let pad = |button: GamepadButton| gamepads.iter().any(|g| g.just_pressed(button));
    let axis = |plus: bool, minus: bool| plus as i8 as f32 - minus as i8 as f32;
    let net_x = axis(
        pressed(KeyCode::ArrowRight, GamepadButton::DPadRight),
        pressed(KeyCode::ArrowLeft, GamepadButton::DPadLeft),
    );
    let net_y = axis(
        pressed(KeyCode::ArrowUp, GamepadButton::DPadUp),
        pressed(KeyCode::ArrowDown, GamepadButton::DPadDown),
    );
    if net_x == 0.0 && net_y == 0.0 {
        return;
    }
    show_focus(&mut shown);
    let from = nav.focus.get().filter(|&f| scopes.holds(f));
    // Left and Right on a slider or a choice are its own (the keys through its observers, the
    // D-pad through `gamepad_adjust`): they never also move focus off it. Up and Down still do.
    let net_x = if from.is_some_and(|f| scopes.keeps_sideways(f)) {
        0.0
    } else if from.is_some_and(|f| scopes.types_text(f)) {
        axis(pad(GamepadButton::DPadRight), pad(GamepadButton::DPadLeft))
    } else {
        net_x
    };
    // Down on a menu title opens its menu (`menu_by_input`), and moves nothing.
    let net_y = if from.is_some_and(|f| scopes.menu_titles.contains(f)) {
        0.0
    } else {
        net_y
    };
    let Some(direction) = Dir2::from_xy(net_x, net_y).ok().map(CompassOctant::from) else {
        return;
    };
    let Some(from) = from else {
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
        .filter(|(entity, _, node, transform, visible)| {
            !node.is_empty()
                && visible.get()
                && scopes.of(*entity) == scope
                && !scopes.hides(*entity, node, transform, from)
        })
        .filter_map(focusable_area)
        .filter_map(|(c, area)| (c == camera).then_some(area))
        .collect();
    // Rows the list has clipped lie further off than the button under it, so geometry alone would
    // step out of a list past rows it still holds: Up and Down stay among its rows until they run
    // out, and only then look beyond them.
    if let Some(list) = scopes.list_of(from)
        && matches!(direction, CompassOctant::North | CompassOctant::South)
    {
        let rows: Vec<FocusableArea> = candidates
            .iter()
            .filter(|area| scopes.list_of(area.entity) == Some(list))
            .copied()
            .collect();
        if let Some(to) = find_best_candidate(&origin, direction, &rows, &config) {
            nav.focus.set(to, FocusCause::Navigated);
            return;
        }
    }
    if let Some(to) = find_best_candidate(&origin, direction, &candidates, &config) {
        nav.focus
            .set(scopes.landing(from, to), FocusCause::Navigated);
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

/// Activates the focused, enabled button, or flips the focused, enabled [`crate::toggle`], when
/// any gamepad's South button (A on Xbox, Cross on PlayStation) is just pressed, and shows focus
/// (`InputFocusVisible`). A [`crate::choice`] is a button that cycles to its next option when
/// activated, so South cycles it. Enter and Space are `bevy_ui_widgets`' own, so they are not
/// handled here: a second handler would act twice.
pub fn gamepad_select(
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    gamepads: Query<&Gamepad>,
    buttons: Query<(), (With<bevy::ui_widgets::Button>, Without<InteractionDisabled>)>,
    toggles: Query<(), (With<Checkbox>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if !gamepads
        .iter()
        .any(|g| g.just_pressed(GamepadButton::South))
    {
        return;
    }
    show_focus(&mut shown);
    if let Some(entity) = focus.get() {
        if buttons.contains(entity) {
            commands.trigger(Activate { entity });
        } else if toggles.contains(entity) {
            commands.trigger(ToggleChecked { entity });
        }
    }
}

/// Steps the focused, enabled [`crate::slider`] or [`crate::choice`] with the D-pad: Left and Right
/// take one step a press, and a direction held down repeats after [`PAD_REPEAT_DELAY`] at
/// [`PAD_REPEAT_RATE`] a second, so a pad player sweeps a volume without twenty presses and never
/// holds a second button to keep the control selected. A choice stops at its ends. Shows focus
/// (`InputFocusVisible`). Runs in [`crate::GooeySystems::Navigate`]. The keys are the controls'
/// own (Left and Right step, Home and End jump) and are not handled here.
///
/// Up and Down move focus off the control ([`navigate_by_input`] leaves them alone), and a press
/// with either held stops the repeat. The repeat counts real time, so a paused game's menu still
/// repeats.
pub fn gamepad_adjust(
    time: Res<Time<Real>>,
    focus: Res<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    gamepads: Query<&Gamepad>,
    controls: Query<Has<Slider>, TakesSideways>,
    mut held: Local<(i8, f32)>,
    mut commands: Commands,
) {
    let down = |button| gamepads.iter().any(|g| g.pressed(button));
    let direction = down(GamepadButton::DPadRight) as i8 - down(GamepadButton::DPadLeft) as i8;
    let vertical = down(GamepadButton::DPadUp) || down(GamepadButton::DPadDown);
    let Some((control, is_slider)) = focus
        .get()
        .filter(|_| direction != 0 && !vertical)
        .and_then(|f| controls.get(f).ok().map(|slider| (f, slider)))
    else {
        *held = (0, 0.0);
        return;
    };
    // `held` is the direction held and for how many seconds.
    let steps = if held.0 == direction {
        let before = held.1;
        held.1 += time.delta_secs();
        repeat_steps(before, held.1, PAD_REPEAT_DELAY, PAD_REPEAT_RATE)
    } else {
        *held = (direction, 0.0);
        1
    };
    if steps > 0 {
        show_focus(&mut shown);
        if is_slider {
            commands.trigger(SetSliderValue {
                entity: control,
                change: SliderValueChange::RelativeStep(f32::from(direction) * steps as f32),
            });
        } else {
            commands.trigger(MoveChoice {
                entity: control,
                to: Move::Step(i32::from(direction) * steps as i32),
            });
        }
    }
}
