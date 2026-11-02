//! The tabs: what keeps a [`crate::tabs`] widget's state, shows its page, steps it with the pad and
//! keeps focus where the player can see it. The strip's keys, its click and its roving focus are
//! Bevy's headless `TabList` and `Tab`; its look is [`crate::apply_skin`]'s.

use bevy::input::ButtonInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::Selected;
use bevy::ui_widgets::{SelectedTab, ValueChange};

use crate::model::tab_step;
use crate::nav::{FocusScopes, show_focus};
use crate::skin::Role;

/// Which tab a [`crate::tabs`] widget has selected: an index into its labels, in the order they
/// were given. It is the widget's state: read it to know the page, set it to turn the page from
/// code (the screen follows in the same frame, and no [`ValueChange`] is sent for a change the
/// game made itself). A value past the last tab is pulled back to the last.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn follow(tabs: Query<&TabsValue, Changed<TabsValue>>) {
///     for selected in &tabs {
///         info!("the settings page is now {}", selected.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct TabsValue(pub usize);

/// How many tabs a [`crate::tabs`] widget has: set when it is spawned and read by the widget to
/// know where its ends are. Leave it alone; a widget's tabs are fixed for its life.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct TabsCount(pub usize);

/// Turns Bevy's `ValueChange<Option<Entity>>` on the strip (a click, Left and Right, Home and
/// End, Enter and Space on a tab) into the one [`ValueChange<usize>`] on the widget a game
/// observes: the tab's place instead of an entity. The strip's own observer.
pub(crate) fn tabs_on_select(
    change: On<ValueChange<Option<Entity>>>,
    strips: Query<(&ChildOf, &Children)>,
    values: Query<&TabsValue>,
    mut commands: Commands,
) {
    let Some(tab) = change.value else {
        return;
    };
    let Ok((strip, tabs)) = strips.get(change.source) else {
        return;
    };
    let root = strip.parent();
    let Some(index) = tabs.iter().position(|t| t == tab) else {
        return;
    };
    if values.get(root).is_ok_and(|value| value.0 != index) {
        commands.trigger(ValueChange {
            source: root,
            value: index,
            is_final: true,
        });
    }
}

/// Keeps the state: a [`ValueChange<usize>`] on a tabs widget becomes its [`TabsValue`]. The
/// widget's own observer, like Bevy's `checkbox_self_update`.
pub(crate) fn tabs_self_update(change: On<ValueChange<usize>>, mut values: Query<&mut TabsValue>) {
    if let Ok(mut value) = values.get_mut(change.source) {
        value.set_if_neq(TabsValue(change.value));
    }
}

/// Shows each changed widget's selected page and no other, lights its selected tab and pulls a value
/// past the last tab back to it. Runs in [`crate::GooeySystems::Style`], before
/// [`crate::apply_skin`]: the same frame the value changes, from a click, a key, the pad or the
/// game, so a tab is never lit for a page that is not shown.
///
/// Every page stands in the one grid cell, so the widget is as wide and as high as its largest page
/// and turning a page never moves the strip or the panel round it; the pages not selected are
/// hidden, which keeps their room, hides them from the pointer and takes their widgets out of every
/// kind of focus movement. A widget whose value did not change costs nothing.
pub fn update_tabs(
    mut roots: Query<(&mut TabsValue, &TabsCount, &Children), Changed<TabsValue>>,
    roles: Query<&Role>,
    children: Query<&Children>,
    mut strips: Query<&mut SelectedTab>,
    mut visibility: Query<&mut Visibility>,
    lit: Query<(), With<Selected>>,
    mut commands: Commands,
) {
    for (mut value, count, kids) in &mut roots {
        let last = count.0.saturating_sub(1);
        if value.0 > last {
            value.0 = last;
        }
        let part = |role: Role| kids.iter().find(|&kid| roles.get(kid) == Ok(&role));
        if let Some(strip) = part(Role::TabList)
            && let Ok(tabs) = children.get(strip)
        {
            let chosen = tabs.get(value.0).copied();
            if let Ok(mut selection) = strips.get_mut(strip) {
                selection.set_if_neq(SelectedTab(chosen));
            }
            // Bevy derives `Selected` from `SelectedTab` in `PostUpdate`; this frame's skin
            // already needs it, and the same answer there changes nothing.
            for &tab in tabs {
                match (Some(tab) == chosen, lit.contains(tab)) {
                    (true, false) => {
                        commands.entity(tab).insert(Selected);
                    }
                    (false, true) => {
                        commands.entity(tab).remove::<Selected>();
                    }
                    _ => {}
                }
            }
        }
        if let Some(container) = part(Role::TabPages)
            && let Ok(pages) = children.get(container)
        {
            for (index, page) in pages.iter().enumerate() {
                if let Ok(mut page) = visibility.get_mut(page) {
                    page.set_if_neq(if index == value.0 {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    });
                }
            }
        }
    }
}

/// Turns the page of a tabs widget with the pad's bumpers, `LB` and `RB`, and with Ctrl+PageUp and
/// Ctrl+PageDown: to the previous and next tab, wrapping round, one step a press
/// ([`crate::tab_step`]); and steps it with the D-pad's Left and Right while a tab has focus. Shows
/// focus (`InputFocusVisible`). Runs in [`crate::GooeySystems::Navigate`]. Left and Right with the
/// keyboard on a focused tab are Bevy's own (they move focus and select), so they are not handled
/// here; PageUp and PageDown without Ctrl are [`crate::page_lists`]', and the pad's triggers are
/// its too.
///
/// Which widget turns: the tabs the focus is inside, wherever in them it is (a slider on the page, a
/// tab), or else the only visible tabs widget in the focus's scope (an open [`crate::dialog`] is its
/// own scope), so a Back button beside the tabs does not stop the bumpers; with none, or with two to
/// choose from, nothing. Focus on a tab moves to the tab now selected; focus on the page stays on
/// the page and lands on the new page's first widget ([`land_tab_focus`]).
pub fn step_tabs(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    scopes: FocusScopes,
    mut commands: Commands,
) {
    let pad = |button: GamepadButton| gamepads.iter().any(|g| g.just_pressed(button));
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let step = |plus: bool, minus: bool| i32::from(plus) - i32::from(minus);
    let mut delta = step(
        pad(GamepadButton::RightTrigger),
        pad(GamepadButton::LeftTrigger),
    );
    if ctrl {
        delta += step(
            keys.just_pressed(KeyCode::PageDown),
            keys.just_pressed(KeyCode::PageUp),
        );
    }
    let Some(from) = focus.get() else {
        return;
    };
    let on_tab = scopes.is_tab(from);
    if on_tab {
        delta += step(pad(GamepadButton::DPadRight), pad(GamepadButton::DPadLeft));
    }
    let delta = delta.clamp(-1, 1);
    if delta == 0 {
        return;
    }
    let Some((root, selected, count)) = scopes.tabs_for(from).and_then(|root| {
        scopes
            .tabs_state(root)
            .map(|(value, count)| (root, value, count))
    }) else {
        return;
    };
    show_focus(&mut shown);
    let to = tab_step(selected, count, delta);
    if to == selected {
        return;
    }
    commands.trigger(ValueChange {
        source: root,
        value: to,
        is_final: true,
    });
    // Focus on a tab goes with the selection, as Bevy's own Left and Right do; focus in the page
    // is taken to the new page once it shows.
    if on_tab && let Some(tab) = scopes.tab_at(root, to) {
        focus.set(tab, FocusCause::Navigated);
    }
}

/// What [`land_tab_focus`] reads about an entity on the way to the page's first widget.
pub(crate) type StopQuery = (
    Option<&'static TabIndex>,
    Option<&'static InheritedVisibility>,
    Option<&'static Children>,
);

/// Keeps focus off a page that is not shown. When focus rests on a widget in a page the widget's
/// tabs have turned away from (the player pressed a bumper in it, or the game set
/// [`TabsValue`]), it moves to the first widget of the page now shown, in tab order, or to the
/// selected tab when the page has none; the ring stays as visible as it was. Runs in
/// [`crate::GooeySystems::Navigate`], before [`crate::navigate_by_input`], and reads the page's
/// visibility from the frame before, so the move comes one frame after the page turned. Focus the
/// pointer set can never be there: a hidden page takes no pointer.
pub fn land_tab_focus(
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    roles: Query<&Role>,
    children: Query<&Children>,
    values: Query<(&TabsValue, &TabsCount)>,
    visible: Query<&InheritedVisibility>,
    stops: Query<StopQuery>,
) {
    let Some(from) = focus.get() else {
        return;
    };
    // The nearest page `from` is in, its container and the widget it belongs to.
    let Some(page) = parents
        .iter_ancestors(from)
        .find(|&a| roles.get(a) == Ok(&Role::TabPage))
    else {
        return;
    };
    let Some(container) = parents.get(page).ok().map(ChildOf::parent) else {
        return;
    };
    let Some(root) = parents.get(container).ok().map(ChildOf::parent) else {
        return;
    };
    let (Ok((value, count)), Ok(pages)) = (values.get(root), children.get(container)) else {
        return;
    };
    let chosen = value.0.min(count.0.saturating_sub(1));
    if pages.get(chosen).is_none_or(|&shown| shown == page) {
        return;
    }
    let Some(&shown) = pages.get(chosen) else {
        return;
    };
    // The page about to be shown is hidden until its visibility has propagated: wait for it.
    if !visible.get(shown).is_ok_and(|v| v.get()) {
        return;
    }
    let to = first_stop(shown, &stops).or_else(|| {
        let strip = children
            .get(root)
            .ok()?
            .iter()
            .find(|&kid| roles.get(kid) == Ok(&Role::TabList))?;
        children.get(strip).ok()?.get(chosen).copied()
    });
    if let Some(to) = to {
        focus.set(to, FocusCause::Navigated);
    }
}

/// The widget Tab would reach first inside `page`: the lowest non-negative [`TabIndex`] among its
/// visible descendants, the first in tree order among equals. A hidden subtree is skipped whole,
/// as Bevy's Tab navigation skips it. Bevy's own `TabNavigation::initialize` needs a `TabGroup`
/// to start from; a page is not one.
pub(crate) fn first_stop(page: Entity, stops: &Query<StopQuery>) -> Option<Entity> {
    let mut best: Option<(i32, Entity)> = None;
    let mut pending = vec![page];
    // Depth first, in tree order: a stack, with the children pushed last first.
    while let Some(entity) = pending.pop() {
        let Ok((index, visible, kids)) = stops.get(entity) else {
            continue;
        };
        if visible.is_some_and(|v| !v.get()) {
            continue;
        }
        if let Some(index) = index.filter(|index| index.0 >= 0)
            && best.is_none_or(|(lowest, _)| index.0 < lowest)
        {
            best = Some((index.0, entity));
        }
        if let Some(kids) = kids {
            pending.extend(kids.iter().rev());
        }
    }
    best.map(|(_, entity)| entity)
}
