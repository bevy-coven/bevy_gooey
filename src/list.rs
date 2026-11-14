//! The list: rows that scroll, and a focus that never leaves the screen. [`crate::list`] builds it
//! on Bevy's headless `ScrollArea` (the wheel and a trackpad) and `Scrollbar` (a drag and a click
//! on the track); this module keeps the rail honest ([`update_lists`]), scrolls the list to follow
//! a focus a key or the pad moved ([`scroll_to_focus`]) and pages through it ([`page_lists`]).

use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui::{ComputedNode, ScrollPosition, UiGlobalTransform};
use bevy::ui_widgets::Scrollbar;

use crate::model::{page_target, scroll_to_reveal, snap_to_row};
use crate::motion::MotionSettings;
use crate::nav::show_focus;
use crate::scroll_glide::{ScrollEdgeFade, ScrollGlide};
use crate::skin::Role;
use crate::tab_scroll::TabScroll;

/// How much list to keep in sight beyond the focused widget, in rows (a widget's height and the gap
/// beside it). One row would end the viewport flush with the next row and show no sign of more;
/// one and a half shows the next row whole and the one after it cut, at every step, so the list
/// always visibly goes on. In a viewport of `4.5` rows the focus stands the third from the top.
pub(crate) const CONTEXT_ROWS: f32 = 1.5;

/// The scrolling part of a [`crate::list`]: the clipped column its rows are children of. It stands
/// `rows` rows tall (a row is a button's height, plus a [`crate::GooeySkin::gap`] between two:
/// [`crate::model::list_height`]), so `4.5` shows four rows and half of the fifth, the cut-off that
/// tells the player there is more.
///
/// Change `rows` to resize the list live. To scroll from code, set the `ScrollPosition` on this
/// entity (the first child of the list's root); read it to remember where the player was.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy::ui::ScrollPosition;
/// # use bevy_gooey::*;
/// // Scroll every list back to its top, as when a menu is opened again.
/// fn rewind(mut lists: Query<&mut ScrollPosition, With<ListView>>) {
///     for mut scroll in &mut lists {
///         scroll.y = 0.0;
///     }
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[require(ScrollGlide, ScrollEdgeFade)]
pub struct ListView {
    /// How many rows tall the list stands; a fraction cuts the last row visible.
    pub rows: f32,
}

/// What a rail can scroll: a list's viewport, a tab strip's list, or a Markdown page's viewport.
#[cfg(feature = "markdown")]
type Scrolls = Or<(
    With<ListView>,
    With<TabScroll>,
    With<crate::box_model::ScrollGutter>,
    With<crate::markdown_view::MarkdownViewport>,
)>;
#[cfg(not(feature = "markdown"))]
type Scrolls = Or<(
    With<ListView>,
    With<TabScroll>,
    With<crate::box_model::ScrollGutter>,
)>;

/// What scrolls to follow focus and pages with PageUp and PageDown: a list's viewport, and a
/// Markdown page's, whose links are tab stops like a list's rows.
#[cfg(feature = "markdown")]
type Viewport = Or<(
    With<ListView>,
    With<crate::box_model::ScrollGutter>,
    With<crate::markdown_view::MarkdownViewport>,
)>;
#[cfg(not(feature = "markdown"))]
type Viewport = Or<(With<ListView>, With<crate::box_model::ScrollGutter>)>;

/// Shows each list's rail (and each scrolling tab strip's, [`crate::TabScroll`]) only while its
/// content does not fit, the way a browser's bar comes and
/// goes: `Display::None` when there is nowhere to scroll, so it costs no draw and no hit test.
/// Writes a rail's node only when the answer changes, so a list at rest costs a comparison.
///
/// Reads the viewport's layout from the last frame, so a list that grows or shrinks shows or hides
/// its rail one frame later. Runs in [`crate::GooeySystems::Style`].
pub fn update_lists(
    mut rails: Query<(&Role, &Scrollbar, &mut Node)>,
    views: Query<&ComputedNode, Scrolls>,
) {
    for (role, bar, mut node) in &mut rails {
        if *role != Role::ListRail {
            continue;
        }
        let Ok(view) = views.get(bar.target) else {
            continue;
        };
        // The same clamp Bevy's own scrollbar and wheel use, so the rail is there exactly when the
        // wheel has somewhere to go, a pixel neither early nor late.
        let shown = view.content_size().y > view.size().y - view.scrollbar_size.y + 0.5;
        let want = if shown { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
}

/// A node's vertical extent in logical pixels, from its top: `(top, height)`, in the screen's
/// coordinates.
pub(crate) fn span(node: &ComputedNode, transform: &UiGlobalTransform) -> (f32, f32) {
    let scale = node.inverse_scale_factor();
    let height = node.size().y * scale;
    (
        transform.affine().translation.y * scale - height * 0.5,
        height,
    )
}

/// The list a widget is in, if any: its nearest ancestor with a [`ListView`] (or a Markdown
/// page's viewport).
fn list_of(
    entity: Entity,
    parents: &Query<&ChildOf>,
    views: &Query<(), Viewport>,
) -> Option<Entity> {
    parents.iter_ancestors(entity).find(|&a| views.contains(a))
}

/// Scrolls a list to follow the focus a key, the pad or Tab moved onto one of its widgets: the least
/// scroll that leaves a row and a half of list beyond it (a row is the widget's height and the
/// gap), so the player always sees the next row whole and the one after cut: the list visibly goes
/// on ([`scroll_to_reveal`]). A widget taller than the list scrolls to its top; a jump of more than
/// a viewport (PageDown, a focus set from code) centres it.
///
/// Only a focus the player can see ([`InputFocusVisible`]) is followed: a press of the pointer hides
/// it, and a press that scrolled the content under the pointer would send its release to another
/// row, so a click focuses a half-visible row where it stands and the next key brings it in.
/// Writes `ScrollPosition` only when it must move by more than half a pixel. Runs in
/// [`crate::GooeySystems::Navigate`], after the systems that move focus.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn scroll_to_focus(
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    parents: Query<&ChildOf>,
    lists: Query<(), Viewport>,
    nodes: Query<
        (&ComputedNode, &UiGlobalTransform),
        (Without<ListView>, Without<crate::box_model::ScrollGutter>),
    >,
    columns: Query<(&Node, &Children)>,
    mut views: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &Children,
            &mut ScrollPosition,
            &mut ScrollGlide,
        ),
        Viewport,
    >,
    settings: Res<MotionSettings>,
    time: Option<Res<Time<Real>>>,
) {
    if !visible.0 || !(focus.is_changed() || visible.is_changed()) {
        return;
    }
    let Some(target) = focus.get() else {
        return;
    };
    let Some(list) = list_of(target, &parents, &lists) else {
        return;
    };
    let (Ok((node, transform)), Ok((view, view_transform, view_children, mut scroll, mut glide))) =
        (nodes.get(target), views.get_mut(list))
    else {
        return;
    };
    let scale = view.inverse_scale_factor();
    let (top, height) = span(node, transform);
    let (view_top, view_height) = span(view, view_transform);
    // Row geometry was laid out at the scroll the layout last applied, not at the component's.
    let laid_out = view.scroll_position.y * scale;
    let max = (view.content_size().y + view.scrollbar_size.y - view.size().y).max(0.0) * scale;
    let from_start = top - view_top + laid_out;
    // Where the scroll is heading: a glide's goal, so a row asked for mid-glide is revealed from
    // where the scroll will be and never pulls the goal back.
    let base = glide.goal().unwrap_or(laid_out);
    // The rows' column holds their gap and the rows; a list's context is counted in rows, gap
    // included.
    let column = view_children
        .first()
        .and_then(|&column| columns.get(column).ok());
    let gap = column.map_or(0.0, |(column, _)| match column.row_gap {
        Val::Px(gap) => gap,
        _ => 0.0,
    });
    let want = scroll_to_reveal(
        from_start,
        from_start + height,
        view_height,
        base,
        max,
        CONTEXT_ROWS * (height + gap),
    );
    // Stand on a row's top, as at rest, so the row the list cuts at the bottom is cut the same way
    // at every step. Not at the end (the list ends flush with its last row), and not if the snap
    // would push the widget itself out of view (a viewport only a row or two tall).
    let tops: Vec<f32> = column
        .into_iter()
        .flat_map(|(_, rows)| rows.iter())
        .filter_map(|row| nodes.get(row).ok())
        .map(|(node, transform)| span(node, transform).0 - view_top + laid_out)
        .collect();
    let want = snap_to_row(want, base, &tops)
        .filter(|&snapped| {
            want < max - 0.5
                && snapped <= max
                && from_start >= snapped - 0.5
                && from_start + height <= snapped + view_height + 0.5
        })
        .unwrap_or(want);
    if (want - glide.goal().unwrap_or(scroll.y)).abs() > 0.5 {
        if ScrollGlide::glides(&settings, time.as_deref()) {
            glide.go(&scroll, want);
        } else {
            scroll.y = want;
        }
    }
}

/// What paging reads of a node: where it is, whether it shows, whether a direction can land on it
/// and whether it is a list itself.
type PageNode = (
    Entity,
    &'static ComputedNode,
    &'static UiGlobalTransform,
    &'static InheritedVisibility,
    Has<AutoDirectionalNavigation>,
    Has<ListView>,
    Has<crate::box_model::ScrollGutter>,
);

/// PageUp and PageDown, and the pad's left and right triggers, move focus a viewport up or down
/// the list it is in, to the row that far away ([`page_target`]), and show focus; the list then
/// follows ([`scroll_to_focus`]). A single press, as every navigation here is. Focus outside a list
/// is left alone. With Ctrl held the page keys are [`crate::step_tabs`]'s: they turn the page of a
/// [`crate::tabs`] widget and never also page a list. Runs in [`crate::GooeySystems::Navigate`].
#[allow(clippy::too_many_arguments)]
pub fn page_lists(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    parents: Query<&ChildOf>,
    lists: Query<(), Viewport>,
    nodes: Query<PageNode>,
    regions: Query<(), With<crate::box_model::ScrollGutter>>,
    roles: Query<&Role>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let pressed = |key: KeyCode, pad: GamepadButton| {
        (!ctrl && keys.just_pressed(key)) || gamepads.iter().any(|g| g.just_pressed(pad))
    };
    let down = pressed(KeyCode::PageDown, GamepadButton::RightTrigger2);
    let up = pressed(KeyCode::PageUp, GamepadButton::LeftTrigger2);
    if down == up {
        return;
    }
    let Some(from) = focus.get() else {
        return;
    };
    let Some(list) = list_of(from, &parents, &lists) else {
        return;
    };
    // A scroll region holds forms and panes, not only rows: a widget in it that pages itself keeps
    // its keys (`keeps_nav_keys`), as it does for Home and End.
    if regions.contains(list) && keeps_nav_keys(from, list, &roles, &parents) {
        return;
    }
    let (Ok((_, from_node, from_transform, ..)), Ok((_, view, view_transform, ..))) =
        (nodes.get(from), nodes.get(list))
    else {
        return;
    };
    show_focus(&mut shown);
    let (from_top, from_height) = span(from_node, from_transform);
    let (_, view_height) = span(view, view_transform);
    let page = (view_height - from_height).max(from_height);
    let candidates: Vec<(Entity, f32)> = nodes
        .iter()
        .filter(
            |&(entity, node, _, visible, navigable, is_list, is_region)| {
                navigable
                    && !is_list
                    && !is_region
                    && !node.is_empty()
                    && visible.get()
                    && list_of(entity, &parents, &lists) == Some(list)
            },
        )
        .map(|(entity, node, transform, ..)| {
            let (top, height) = span(node, transform);
            (entity, top + height * 0.5)
        })
        .collect();
    let centres: Vec<f32> = candidates.iter().map(|&(_, centre)| centre).collect();
    let signed = if down { page } else { -page };
    if let Some(to) = page_target(from_top + from_height * 0.5, &centres, signed) {
        focus.set(candidates[to].0, FocusCause::Navigated);
    }
}

/// Home and End in a [`crate::scroll_region`]: focus goes to the first or last widget of the region
/// that can take it and the region follows ([`scroll_to_focus`]), so the content is at its start or
/// its end. Acts only when focus is on a widget inside a region **and that widget does not keep Home
/// and End for itself** (a slider, a choice, a tab, a divider, a text field: their own handlers have
/// them). Show focus. Runs in [`crate::GooeySystems::Navigate`] under the same conditions as paging.
pub fn scroll_region_ends(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<InputFocus>,
    mut shown: ResMut<InputFocusVisible>,
    parents: Query<&ChildOf>,
    regions: Query<(), With<crate::box_model::ScrollGutter>>,
    roles: Query<&Role>,
    nodes: Query<PageNode>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if ctrl {
        return;
    }
    let (home, end) = (
        keys.just_pressed(KeyCode::Home),
        keys.just_pressed(KeyCode::End),
    );
    if home == end {
        return;
    }
    let Some(from) = focus.get() else {
        return;
    };
    let Some(region) = parents.iter_ancestors(from).find(|&a| regions.contains(a)) else {
        return;
    };
    if keeps_nav_keys(from, region, &roles, &parents) {
        return;
    }
    let mut candidates: Vec<(Entity, f32)> = nodes
        .iter()
        .filter(
            |&(entity, node, _, visible, navigable, is_list, is_region)| {
                navigable
                    && !is_list
                    && !is_region
                    && !node.is_empty()
                    && visible.get()
                    && parents.iter_ancestors(entity).any(|a| a == region)
            },
        )
        .map(|(entity, node, transform, ..)| {
            let (top, height) = span(node, transform);
            (entity, top + height * 0.5)
        })
        .collect();
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
    let to = if home {
        candidates.first()
    } else {
        candidates.last()
    };
    if let Some(&(to, _)) = to {
        show_focus(&mut shown);
        focus.set(to, FocusCause::Navigated);
    }
}

/// Whether the widget that has focus keeps PageUp, PageDown, Home and End for itself, so a
/// [`crate::scroll_region`] around it must not take them: the one predicate [`page_lists`] (for a
/// region's viewport) and [`scroll_region_ends`] share. True when the focused widget **or any widget
/// between it and the region** is one that handles those keys: a slider, a choice, a binding, a split
/// divider, a tab, a radial, a text input, text area or number field (and their cells), a table or tree
/// (and its rows and header), a code editor, a colour area, a palette (field and rows), a menu item,
/// or a slot. A plain button, a toggle or a card is not: the region pages and ends over them.
pub(crate) fn keeps_nav_keys(
    from: Entity,
    region: Entity,
    roles: &Query<&Role>,
    parents: &Query<&ChildOf>,
) -> bool {
    std::iter::once(from)
        .chain(parents.iter_ancestors(from).take_while(|&a| a != region))
        .any(|e| {
            roles.get(e).is_ok_and(|role| {
                matches!(
                    role,
                    Role::Slider
                        | Role::Choice
                        | Role::ChoiceCell
                        | Role::ChoiceOption
                        | Role::Binding
                        | Role::SplitDivider
                        | Role::Tab
                        | Role::IconTab
                        | Role::Radial
                        | Role::TextInput
                        | Role::TextArea
                        | Role::TextCell
                        | Role::NumberPart
                        | Role::NumberParts
                        | Role::Table
                        | Role::TableView
                        | Role::TableRow
                        | Role::TableHeaderCell
                        | Role::CodeEditor
                        | Role::CodeArea
                        | Role::CodeContent
                        | Role::ColorArea
                        | Role::ColorStripBox
                        | Role::Palette
                        | Role::PaletteField
                        | Role::PaletteRow
                        | Role::MenuItem
                        | Role::Slot
                        | Role::SlotEmpty
                )
            })
        })
}
