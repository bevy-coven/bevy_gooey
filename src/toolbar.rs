//! The toolbar: what keeps a [`crate::toggle_group`]'s state and what makes a bar of items
//! ([`crate::toolbar`], [`crate::status_bar`]) hold only the items its width has room for. Its look
//! is [`crate::apply_skin`]'s, like every widget's.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::{Checked, ComputedNode, InteractionDisabled};
use bevy::ui_widgets::{Activate, ValueChange};

use crate::icon::IconSource;
use crate::menu::{MenuItem, MenuPanel};
use crate::model::bar_fit;
use crate::skin::{GooeySkin, Role};
use crate::status::{StatusBar, StatusGroup, StatusPriority};
use crate::tooltip::Tooltip;
use crate::widgets::{menu_title, radio_item, separator};

/// Which option a [`crate::toggle_group`] has lit: an index into its options, in the order they
/// were given. It is the widget's state: read it to know the mode, set it to move the group from
/// code (the screen follows in the same frame, and no [`ValueChange`] is sent for a change the game
/// made itself). A value past the last option is pulled back to the last.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn apply(groups: Query<&ToggleGroupValue, Changed<ToggleGroupValue>>) {
///     for lit in &groups {
///         info!("tool {} is on", lit.0);
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ToggleGroupValue(pub usize);

/// How many options a [`crate::toggle_group`] has: set when it is spawned. Leave it alone.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ToggleGroupCount(pub usize);

/// An option's place in its group. Part of what [`crate::toggle_group`] spawns.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupIndex(pub(crate) usize);

/// A [`crate::toolbar`]'s root. It holds the "More" button the bar grows when its items do not
/// fit, and the items that button lists.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Toolbar {
    /// The "More" button, spawned the first time an item hides.
    more: Option<Entity>,
    /// The items hidden now, in order: what the "More" menu lists.
    hidden: Vec<Entity>,
}

/// On a toolbar's "More" button.
#[derive(Component, Clone, Copy, Debug)]
pub struct ToolbarMore;

/// On a row of a "More" menu: the hidden item it stands for. Choosing the row activates that item.
#[derive(Component, Clone, Copy, Debug)]
pub struct MoreFor(Entity);

/// On a bar's item that its width has no room for now: it is not laid out (`Display::None`), and
/// holds the display it comes back with. Query for it to know what a bar has hidden.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Overflowed {
    display: Display,
}

/// An item's last laid-out width in logical px, kept while it is shown so a hidden item still has
/// one: it is what lets it come back when the room does.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct BarWidth(f32);

/// A group's option was activated (Enter, Space, South, a click): light it, unless it is lit.
pub(crate) fn option_on_activate(
    activate: On<Activate>,
    options: Query<(&GroupIndex, &ChildOf), Without<InteractionDisabled>>,
    groups: Query<(&ToggleGroupValue, &ToggleGroupCount), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    let Ok((index, parent)) = options.get(activate.entity) else {
        return;
    };
    let Ok((lit, count)) = groups.get(parent.parent()) else {
        return;
    };
    if index.0 != lit.0 && index.0 < count.0 {
        commands.trigger(ValueChange {
            source: parent.parent(),
            value: index.0,
            is_final: true,
        });
    }
}

/// Keeps the state: a [`ValueChange<usize>`] on a group becomes its [`ToggleGroupValue`]. The
/// group's own observer, like a choice's.
pub(crate) fn group_self_update(
    change: On<ValueChange<usize>>,
    mut groups: Query<&mut ToggleGroupValue>,
) {
    if let Ok(mut value) = groups.get_mut(change.source) {
        value.set_if_neq(ToggleGroupValue(change.value));
    }
}

/// Lights each changed group's option and no other (Bevy's [`Checked`], which [`crate::apply_skin`]
/// draws as the selected tab), and pulls a value past the last option back to it. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`]: the same frame the value changes,
/// from an activation or from the game; a group whose value did not change costs nothing.
pub fn update_toggle_groups(
    mut groups: Query<
        (&mut ToggleGroupValue, &ToggleGroupCount, &Children),
        Changed<ToggleGroupValue>,
    >,
    options: Query<(&GroupIndex, Has<Checked>)>,
    mut commands: Commands,
) {
    for (mut value, count, children) in &mut groups {
        let last = count.0.saturating_sub(1);
        if value.0 > last {
            value.0 = last;
        }
        for &option in children {
            let Ok((index, lit)) = options.get(option) else {
                continue;
            };
            let should = index.0 == value.0;
            if should && !lit {
                commands.entity(option).insert(Checked);
            } else if !should && lit {
                commands.entity(option).remove::<Checked>();
            }
        }
    }
}

/// What a hidden item is called in the "More" menu: its tooltip's words, else the first text under
/// it. An item with neither has no name to list.
fn name_of(world: &World, item: Entity) -> Option<String> {
    if let Some(tip) = world.get::<Tooltip>(item) {
        return Some(tip.text.clone());
    }
    let mut stack = vec![item];
    while let Some(at) = stack.pop() {
        if let Some(text) = world.get::<Text>(at) {
            return Some(text.0.clone());
        }
        if let Some(kids) = world.get::<Children>(at) {
            stack.extend(kids.iter().rev());
        }
    }
    None
}

/// Fills the "More" menu's panel with a row for each hidden item: a command row that activates the
/// item, a radio row for each option of a hidden toggle group, a line for a hidden separator.
fn rebuild_more(world: &mut World, more: Entity, hidden: Vec<Entity>) {
    let Some(panel) = world.get::<Children>(more).and_then(|kids| {
        kids.iter()
            .find(|&kid| world.get::<MenuPanel>(kid).is_some())
    }) else {
        return;
    };
    let rows: Vec<Entity> = world
        .get::<Children>(panel)
        .map(|kids| kids.iter().collect())
        .unwrap_or_default();
    for row in rows {
        world.despawn(row);
    }
    let mut last_was_line = true;
    for item in hidden {
        if world.get::<Role>(item) == Some(&Role::ToolbarSeparator) {
            if !last_was_line {
                world.spawn((separator(), ChildOf(panel)));
                last_was_line = true;
            }
            continue;
        }
        let targets: Vec<(Entity, Option<bool>)> = if world.get::<ToggleGroupValue>(item).is_some()
        {
            let lit = world.get::<ToggleGroupValue>(item).map_or(0, |v| v.0);
            let mut options: Vec<(usize, Entity)> = world
                .get::<Children>(item)
                .into_iter()
                .flat_map(|kids| kids.iter())
                .filter_map(|kid| world.get::<GroupIndex>(kid).map(|i| (i.0, kid)))
                .collect();
            options.sort_by_key(|(i, _)| *i);
            options
                .into_iter()
                .map(|(i, option)| (option, Some(i == lit)))
                .collect()
        } else {
            vec![(item, None)]
        };
        for (target, checked) in targets {
            let Some(name) = name_of(world, target) else {
                continue;
            };
            let off = world.get::<InteractionDisabled>(target).is_some()
                || world.get::<InteractionDisabled>(item).is_some();
            let row = match checked {
                Some(on) => world
                    .spawn((radio_item(name, on), MoreFor(target), ChildOf(panel)))
                    .id(),
                None => world
                    .spawn((MenuItem::command(name), MoreFor(target), ChildOf(panel)))
                    .id(),
            };
            if off {
                world.entity_mut(row).insert(InteractionDisabled);
            }
            last_was_line = false;
        }
    }
}

/// A row of a "More" menu was chosen: the hidden item it stands for is activated, as if the player
/// had pressed it where it would have been.
pub(crate) fn more_on_activate(
    activate: On<Activate>,
    rows: Query<&MoreFor>,
    mut commands: Commands,
) {
    if let Ok(row) = rows.get(activate.entity) {
        commands.trigger(Activate { entity: row.0 });
    }
}

/// One item of a bar, as [`fit_bars`] sees it.
struct Slot {
    item: Entity,
    rank: u32,
}

/// The room a bar's content box has, in logical px.
fn room_of(bar: &ComputedNode) -> f32 {
    bar.content_box().width().max(0.0) * bar.inverse_scale_factor
}

/// Measures the shown items of `slots`, decides which stay ([`bar_fit`]), and hides and shows the
/// rest. Writes only a difference. Returns who is shown, in `slots` order.
fn fit(
    slots: &[Slot],
    room: f32,
    gap: f32,
    reserve: f32,
    items: &mut Query<(
        &ComputedNode,
        &mut Node,
        Option<&Overflowed>,
        Option<&BarWidth>,
    )>,
    commands: &mut Commands,
) -> Vec<bool> {
    let widths: Vec<f32> = slots
        .iter()
        .map(|slot| {
            let Ok((computed, mut node, over, cached)) = items.get_mut(slot.item) else {
                return 0.0;
            };
            // An item's width is its content's, never squeezed: the room it needs is the room it
            // takes, which is what this measures and what hiding it gives back.
            if node.flex_shrink != 0.0 {
                node.flex_shrink = 0.0;
            }
            if over.is_none() {
                let width = computed.size().x * computed.inverse_scale_factor;
                if width > 0.5 && cached.is_none_or(|c| (c.0 - width).abs() > 0.25) {
                    commands.entity(slot.item).insert(BarWidth(width));
                    return width;
                }
            }
            cached.map_or(0.0, |c| c.0)
        })
        .collect();
    let ranks: Vec<u32> = slots.iter().map(|slot| slot.rank).collect();
    let shown = bar_fit(&widths, &ranks, gap, room, reserve);
    for (slot, &keep) in slots.iter().zip(&shown) {
        let Ok((_, mut node, over, _)) = items.get_mut(slot.item) else {
            continue;
        };
        match (keep, over) {
            (false, None) => {
                let display = node.display;
                node.display = Display::None;
                commands.entity(slot.item).insert(Overflowed { display });
            }
            (true, Some(over)) => {
                node.display = over.display;
                commands.entity(slot.item).remove::<Overflowed>();
            }
            _ => {}
        }
    }
    shown
}

/// Holds each bar's items to the room its width has ([`crate::toolbar`] and [`crate::status_bar`]).
///
/// A toolbar hides its **last** items first and grows a "More" button whose menu lists them; a
/// status bar hides the lowest [`StatusPriority`] first, the right group's before the left's, and
/// has no button. A hidden item is `Display::None` and carries [`Overflowed`]; it keeps the width
/// it had, so it comes back when the room does. Focus on an item that hides moves to "More".
/// Runs in [`crate::GooeySystems::Style`] before [`crate::apply_skin`], one frame behind layout.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn fit_bars(
    skin: Res<GooeySkin>,
    mut focus: ResMut<InputFocus>,
    mut toolbars: Query<(Entity, &ComputedNode, &mut Toolbar, &Children), Without<StatusBar>>,
    statuses: Query<(&ComputedNode, &Children), With<StatusBar>>,
    groups: Query<(&StatusGroup, &Children)>,
    priorities: Query<&StatusPriority>,
    marks: Query<(), With<ToolbarMore>>,
    mut items: Query<(
        &ComputedNode,
        &mut Node,
        Option<&Overflowed>,
        Option<&BarWidth>,
    )>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let gap = skin.gap * 0.5;
    for (bar, computed, mut toolbar, children) in &mut toolbars {
        // A bar that has not been laid out has no room yet: judging it would hide everything.
        if computed.size().x <= 0.0 {
            continue;
        }
        let slots: Vec<Slot> = children
            .iter()
            .filter(|&kid| !marks.contains(kid) && items.contains(kid))
            .map(|item| Slot { item, rank: 0 })
            .collect();
        // The button's own room, once it has been laid out; before that, a target and a gap.
        let reserve = toolbar
            .more
            .and_then(|more| items.get(more).ok())
            .and_then(|(_, _, _, width)| width.map(|w| w.0))
            .unwrap_or(skin.target() * 1.5)
            + gap;
        let shown = fit(
            &slots,
            room_of(computed),
            gap,
            reserve,
            &mut items,
            &mut commands,
        );
        let hidden: Vec<Entity> = slots
            .iter()
            .zip(&shown)
            .filter(|(_, keep)| !**keep)
            .map(|(slot, _)| slot.item)
            .collect();
        if hidden.is_empty() && toolbar.more.is_none() {
            continue;
        }
        let more = match toolbar.more {
            Some(more) if items.contains(more) => more,
            _ => {
                // A title is a button that opens a menu: one word, so it is its own name. It is
                // drawn as the bar's other buttons are (raised, not the menu bar's bare title),
                // lit while its menu is open.
                let more = commands
                    .spawn((menu_title("More", ()), ToolbarMore, ChildOf(bar)))
                    .insert(Role::Button)
                    .id();
                toolbar.more = Some(more);
                more
            }
        };
        if let Ok((button, mut node, _, cached)) = items.get_mut(more) {
            // Its own width is what the next frame keeps free for it.
            let width = button.size().x * button.inverse_scale_factor;
            if node.display != Display::None
                && width > 0.5
                && cached.is_none_or(|c| (c.0 - width).abs() > 0.25)
            {
                commands.entity(more).insert(BarWidth(width));
            }
            let want = if hidden.is_empty() {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != want {
                node.display = want;
            }
            if node.flex_shrink != 0.0 {
                node.flex_shrink = 0.0;
            }
        }
        if hidden != toolbar.hidden {
            // Focus on a hidden item would be on nothing: it goes to the button that lists it.
            if let Some(focused) = focus.get()
                && hidden.iter().any(|&item| {
                    focused == item || parents.iter_ancestors(focused).any(|a| a == item)
                })
            {
                focus.set(more, FocusCause::Navigated);
            }
            toolbar.hidden.clone_from(&hidden);
            let listed = hidden;
            commands.queue(move |world: &mut World| rebuild_more(world, more, listed));
        }
    }
    for (computed, children) in &statuses {
        if computed.size().x <= 0.0 {
            continue;
        }
        let mut slots = Vec::new();
        for &group in children {
            let Ok((side, kids)) = groups.get(group) else {
                continue;
            };
            for &item in kids {
                let priority = priorities
                    .get(item)
                    .map_or(StatusPriority::default().0, |p| p.0);
                // The left group is worth more than any right item: it goes last.
                let rank = u32::from(priority) + if side.left { 256 } else { 0 };
                slots.push(Slot { item, rank });
            }
        }
        let shown = fit(
            &slots,
            room_of(computed),
            gap,
            0.0,
            &mut items,
            &mut commands,
        );
        if let Some(focused) = focus.get()
            && let Some(at) = slots.iter().position(|slot| {
                focused == slot.item || parents.iter_ancestors(focused).any(|a| a == slot.item)
            })
            && !shown[at]
        {
            // Nothing to hand focus to in a bar that has no button: let it go.
            focus.clear();
        }
    }
}

/// One option of a [`crate::toggle_group`]: a picture with its name, or a word or two. Build it
/// with [`GroupOption::icon`] or [`GroupOption::text`].
#[derive(Clone, Debug, PartialEq)]
pub struct GroupOption {
    pub(crate) content: OptionContent,
    pub(crate) tooltip: Option<Tooltip>,
}

/// What an option draws.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OptionContent {
    Text(String),
    Icon(IconSource),
}

impl GroupOption {
    /// An option drawn as a picture. `tooltip` is its name: shown on hover and on focus, and the
    /// word a hidden option has in a toolbar's "More" menu.
    #[must_use]
    pub fn icon(source: IconSource, tooltip: Tooltip) -> Self {
        Self {
            content: OptionContent::Icon(source),
            tooltip: Some(tooltip),
        }
    }

    /// An option drawn as a word or two.
    #[must_use]
    pub fn text(label: impl Into<String>) -> Self {
        Self {
            content: OptionContent::Text(label.into()),
            tooltip: None,
        }
    }

    /// The same option with a tooltip (a chord, a longer name).
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: Tooltip) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
}
