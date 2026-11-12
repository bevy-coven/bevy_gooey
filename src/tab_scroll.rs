//! A tab strip with more tabs than room: [`crate::side_tabs`] and [`crate::side_tabs_icons`] scroll.
//!
//! The strip's `TabList` is its own viewport ([`TabScroll`]): it clips, the wheel and a trackpad
//! scroll it (Bevy's `ScrollArea`), a rail down its side shows while the tabs do not fit
//! ([`crate::update_lists`], the same rail a [`crate::list`] has) and [`scroll_tabs_to_reveal`]
//! keeps the tab the player moved to, and the one that is selected, in sight. The strip takes the
//! room its host gives it: it never grows past a height the host's node has (`max_height` 100%), so
//! a strip in a host with no height of its own is as tall as its tabs and has nothing to scroll.

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, ScrollPosition, Selected, UiGlobalTransform};
use bevy::ui_widgets::ScrollArea;

use crate::list::{CONTEXT_ROWS, span};
use crate::model::scroll_to_reveal;
use crate::motion::MotionSettings;
use crate::scroll_glide::ScrollGlide;

/// On the `TabList` of a [`crate::side_tabs`] or [`crate::side_tabs_icons`] strip: its tabs scroll
/// inside it when the host leaves it less height than they need. Part of what those spawn; a game
/// reads or sets the `ScrollPosition` on this entity (the strip's first child) to scroll from code,
/// and can call [`ScrollGlide::go`] on it to have the scroll glide.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
#[require(ScrollGlide, ScrollArea)]
pub struct TabScroll {
    /// The selected tab the strip last brought into view, so a selection is revealed once and the
    /// player's own wheel after it is not undone.
    revealed: Option<Entity>,
}

/// Keeps a scrolling tab strip's focused and selected tab in view: the tab a key, the pad or Tab
/// moved focus onto (while focus is visible), and a tab that has become the selected one (a click,
/// or the game writing [`crate::TabsValue`]; also the first selected tab of a strip that is laid out
/// for the first time). The scroll is the least that leaves a tab and a half of strip beyond the
/// tab, as a [`crate::list`] does ([`scroll_to_reveal`]), and it glides on the skin's spring.
///
/// A strip that fits has no scroll to move and nothing is written. Reads the layout of the last
/// frame; a tab with no layout yet waits for the next. Runs in [`crate::GooeySystems::Style`],
/// after [`crate::update_tabs`] has lit the selected tab and before [`crate::update_scroll_glides`]
/// steps the glide.
#[allow(clippy::too_many_arguments)]
pub fn scroll_tabs_to_reveal(
    focus: Res<InputFocus>,
    visible: Res<InputFocusVisible>,
    settings: Res<MotionSettings>,
    time: Option<Res<Time<Real>>>,
    tabs: Query<(&ComputedNode, &UiGlobalTransform, Has<Selected>), Without<TabScroll>>,
    mut strips: Query<(
        &Node,
        &Children,
        &ComputedNode,
        &UiGlobalTransform,
        &mut TabScroll,
        &mut ScrollPosition,
        &mut ScrollGlide,
    )>,
) {
    let focused = (visible.0 && (focus.is_changed() || visible.is_changed()))
        .then(|| focus.get())
        .flatten();
    for (node, children, view, view_at, mut state, mut scroll, mut glide) in &mut strips {
        let selected = children
            .iter()
            .find(|&tab| tabs.get(tab).is_ok_and(|(_, _, lit)| lit));
        let pending = selected.filter(|&tab| Some(tab) != state.revealed);
        let Some(target) = focused.filter(|tab| children.contains(tab)).or(pending) else {
            continue;
        };
        let Ok((tab, tab_at, _)) = tabs.get(target) else {
            continue;
        };
        // Nothing laid out yet: the tab is asked for again next frame.
        if tab.size().y <= 0.0 || view.size().y <= 0.0 {
            continue;
        }
        if pending.is_some() && state.revealed != selected {
            state.revealed = selected;
        }
        let scale = view.inverse_scale_factor();
        let max = (view.content_size().y + view.scrollbar_size.y - view.size().y).max(0.0) * scale;
        if max <= 0.0 {
            continue;
        }
        let (top, height) = span(tab, tab_at);
        let (view_top, view_height) = span(view, view_at);
        // Tab geometry was laid out at the scroll the layout last applied, not at the component's.
        let laid_out = view.scroll_position.y * scale;
        let from_start = top - view_top + laid_out;
        // Where the scroll is heading, so a tab asked for mid-glide is revealed from where the
        // scroll will be.
        let base = glide.goal().unwrap_or(laid_out);
        let gap = match node.row_gap {
            Val::Px(gap) => gap,
            _ => 0.0,
        };
        let want = scroll_to_reveal(
            from_start,
            from_start + height,
            view_height,
            base,
            max,
            CONTEXT_ROWS * (height + gap),
        );
        if (want - glide.goal().unwrap_or(scroll.y)).abs() > 0.5 {
            if ScrollGlide::glides(&settings, time.as_deref()) {
                glide.go(&scroll, want);
            } else {
                scroll.y = want;
            }
        }
    }
}
