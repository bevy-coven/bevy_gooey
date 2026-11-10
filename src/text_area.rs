//! A text area that fills its host and scrolls: the plain multi-line field (not the code editor of
//! G-71), as tall as the room its parent leaves, whose wheel, rail and caret move the text. Its
//! text is [`crate::TextInputValue`]'s, exactly as a fixed-height area's.
//!
//! Bevy draws `EditableText`'s glyphs at `-viewport.offset` and reveals the caret only on the frame
//! focus arrives, so a field taller than its text needs two things this module brings: one writer
//! of the offset ([`scroll_text_areas`]: the wheel and the rail set the same field, never a second
//! copy), and a caret reveal that runs while the player types and moves the caret (Bevy's own runs
//! on a focus change only).
//!
//! The rail is the list's own look — `Role::ListRail` and `Role::ListThumb` — and its own input, so
//! an area's rail and a list's agree on the eye while the text scrolls `EditableText`'s viewport
//! rather than a node's `ScrollPosition`.

use bevy::input::mouse::{MouseScrollPixelsPerLine, MouseWheel};
use bevy::input_focus::{AcquireFocus, InputFocus};
use bevy::picking::events::{PointerDrag, PointerPress};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::text::{EditableText, TextLayoutInfo};
use bevy::ui::UiGlobalTransform;

use crate::skin::Role;
use crate::text::TextInputOf;

/// A [`crate::text_area`] that fills its host: put it on the row (or spawn the row with
/// [`crate::text_area_fill`]) and its field is as tall as the room the row's parent gives it, with
/// its `rows` a *least* height (a host that gives it nothing gets a field that many lines high
/// rather than one of none).
///
/// Its wheel, its rail and the arrow keys scroll it, the caret is kept in view while the player
/// types or moves it, and a very long line wraps — there is no horizontal scrolling, so a file of
/// long paragraphs reads as a document. Everything else is the fixed area's: `TextInputValue`,
/// `ValueChange<String>` (one per edit, one final at Ctrl+Enter), `TextLimit`, `TextHint`,
/// `FieldProblem`, `TextEditing`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn setup(mut commands: Commands) {
/// commands.spawn((
///     text_area("Notes", "", 5),
///     TextAreaFill,
///     Node {
///         flex_grow: 1.0,
///         ..default()
///     },
/// ));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub struct TextAreaFill;

/// The rail of the field `0`: the lane a press or a drag scrolls the text along, shown while there
/// is more text than the field draws.
#[derive(Component, Clone, Copy, Debug)]
pub struct AreaRail(pub Entity);

/// The thumb of the field `0`: the share of the text the field shows, and how far along it stands.
#[derive(Component, Clone, Copy, Debug)]
pub struct AreaThumb(pub Entity);

/// What a rail was scrolled to when its press came down: kept so a host can read the drag (the
/// rail's look is the list thumb's).
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct RailGrab(#[allow(dead_code)] f32);

/// What the pointer is over, in a filling area: the rail, the thumb, or the field itself.
type PointerOver = (
    Option<&'static AreaRail>,
    Option<&'static AreaThumb>,
    Option<&'static TextInputOf>,
);

/// The field a pointer over `over` scrolls: the field itself, or the field a rail or a thumb
/// belongs to.
fn field_under(over: Entity, names: &Query<PointerOver>) -> Option<Entity> {
    let (rail, thumb, of) = names.get(over).ok()?;
    if let Some(rail) = rail {
        return Some(rail.0);
    }
    if let Some(thumb) = thumb {
        return Some(thumb.0);
    }
    // The field is the node under the wheel; `TextInputOf` on it names the row (for a press on the
    // row's label or padding to focus it), not the thing that scrolls.
    of.map(|_| over)
}

/// The text's laid-out height: the editor is wrapped at the field's width by
/// [`scroll_text_areas`], so the layout's own height is it.
fn content_height(layout: &TextLayoutInfo) -> f32 {
    layout.size.y.max(0.0)
}

/// How far the field may scroll: its text's height, less the field's own.
fn max_offset(node: &ComputedNode, layout: &TextLayoutInfo) -> f32 {
    let size = node.size() * node.inverse_scale_factor();
    (content_height(layout) - size.y).max(0.0)
}

/// A wheel turn scrolls the filling area the pointer is over.
///
/// The wheel is read from the platform's own messages, not from a picking event on the field: a
/// text field's own handler is what a scroll over an editable node reaches, and a crate that
/// listened for a `PointerScroll` on the field heard nothing. The pointer's last hover (Bevy's
/// `HoverMap`) says which node it is over, so a rail, a thumb and the field all scroll the field
/// they belong to.
pub(crate) fn area_on_scroll(
    mut wheels: MessageReader<MouseWheel>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    names: Query<PointerOver>,
    mut fields: Query<(&ComputedNode, &TextLayoutInfo, &mut EditableText)>,
    parents: Query<&ChildOf>,
    ratio: Res<MouseScrollPixelsPerLine>,
) {
    let mut pixels = 0.0;
    for wheel in wheels.read() {
        pixels += match wheel.unit {
            bevy::input::mouse::MouseScrollUnit::Line => wheel.y * *ratio,
            bevy::input::mouse::MouseScrollUnit::Pixel => wheel.y,
        };
    }
    if pixels == 0.0 {
        return;
    }
    let Some(hover) = hover else {
        return;
    };
    for (_, hits) in hover.iter() {
        // The first node under the pointer, front to back, and the ones above it: the first that
        // names a filling area scrolls it (a wheel over the row's label does not scroll, one over
        // its field does).
        let front = hits.iter().min_by(|a, b| a.1.depth.total_cmp(&b.1.depth));
        let Some((&hit, _)) = front else {
            continue;
        };
        let field = std::iter::once(hit)
            .chain(parents.iter_ancestors(hit))
            .find_map(|entity| field_under(entity, &names));
        let Some(field) = field else {
            continue;
        };
        let Ok((node, layout, mut text)) = fields.get_mut(field) else {
            continue;
        };
        let max = max_offset(node, layout);
        let wanted = (text.viewport.offset.y - pixels).clamp(0.0, max);
        if (text.viewport.offset.y - wanted).abs() > 0.01 {
            text.viewport.offset.y = wanted;
        }
    }
}

/// A press on a filling area's rail — or on the thumb in it — scrolls the text there: the pointer's
/// share of the lane is the viewport's share of what the field does not show.
///
/// A **global** observer, walking up from what was pressed to the rail: the thumb covers the lane
/// (a rail's lane is one target, however thin its thumb), and a press inside the rail's subtree must
/// land on the rail whatever node is under it.
pub(crate) fn rail_on_press(
    mut press: On<PointerPress>,
    rails: Query<(&AreaRail, &ComputedNode, &UiGlobalTransform)>,
    fields: Query<(&ComputedNode, &TextLayoutInfo)>,
    mut texts: Query<&mut EditableText>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let target = press.original_event_target();
    let Some(rail) = std::iter::once(target)
        .chain(parents.iter_ancestors(target))
        .find(|&entity| rails.get(entity).is_ok())
    else {
        return;
    };
    let Ok((rail, lane, at)) = rails.get(rail) else {
        return;
    };
    let Ok((node, layout)) = fields.get(rail.0) else {
        return;
    };
    let Ok(mut text) = texts.get_mut(rail.0) else {
        return;
    };
    press.propagate(false);
    let share = share_along(press.pointer.position.y, lane, at);
    let wanted = share * max_offset(node, layout);
    if (text.viewport.offset.y - wanted).abs() > 0.01 {
        text.viewport.offset.y = wanted;
    }
    commands.entity(target).insert(RailGrab(wanted));
}

/// A drag on the rail follows the pointer: its share of the lane is the viewport's share again.
pub(crate) fn rail_on_drag(
    mut drag: On<PointerDrag>,
    rails: Query<(&AreaRail, &ComputedNode, &UiGlobalTransform)>,
    fields: Query<(&ComputedNode, &TextLayoutInfo)>,
    mut texts: Query<&mut EditableText>,
    parents: Query<&ChildOf>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let target = drag.original_event_target();
    let Some(rail) = std::iter::once(target)
        .chain(parents.iter_ancestors(target))
        .find(|&entity| rails.get(entity).is_ok())
    else {
        return;
    };
    let Ok((rail, lane, at)) = rails.get(rail) else {
        return;
    };
    let Ok((node, layout)) = fields.get(rail.0) else {
        return;
    };
    let Ok(mut text) = texts.get_mut(rail.0) else {
        return;
    };
    drag.propagate(false);
    let wanted = share_along(drag.pointer.position.y, lane, at) * max_offset(node, layout);
    if (text.viewport.offset.y - wanted).abs() > 0.01 {
        text.viewport.offset.y = wanted;
    }
}

/// A press on a filling area's rail does not focus its field: a scrollbar press scrolls, it does not
/// move the caret, and the field taking focus would scroll the text back to where the caret is.
///
/// A global `AcquireFocus` observer, which the press bubbles as `text_row_on_acquire`'s sibling
/// does: the row under a rail turns a press into focus for its field, so this takes it first when
/// the press began in a rail.
pub(crate) fn rail_on_acquire(
    mut acquire: On<AcquireFocus>,
    rails: Query<&AreaRail>,
    parents: Query<&ChildOf>,
) {
    let target = acquire.original_event_target();
    let on_rail = std::iter::once(target)
        .chain(parents.iter_ancestors(target))
        .any(|entity| rails.get(entity).is_ok());
    if on_rail {
        acquire.propagate(false);
    }
}

/// How far down its lane a pointer is, from 0 at the top to 1 at the bottom.
fn share_along(pointer: f32, lane: &ComputedNode, at: &UiGlobalTransform) -> f32 {
    let scale = lane.inverse_scale_factor();
    let rect = Rect::from_center_size(at.translation * scale, lane.size() * scale);
    ((pointer - rect.min.y) / rect.height().max(1.0)).clamp(0.0, 1.0)
}

/// What [`scroll_text_areas`] reads of a filling area's field.
type Area = (
    Entity,
    &'static ComputedNode,
    &'static TextLayoutInfo,
    &'static mut EditableText,
    &'static TextInputOf,
);

/// Keeps every filling area's text inside its field, and its rail honest: the viewport is the
/// field's size, the caret of the field the keys are going into is revealed while the player types
/// or moves it, the rail shows while there is more text than the field draws, and the thumb is the
/// share shown, standing where the field stands.
///
/// Runs in [`crate::GooeySystems::Style`] **after [`crate::apply_skin`]**, so it reads this frame's
/// layout. It writes the viewport (drawing, not layout) and a rail's nodes only on a difference, so
/// a settled area costs a comparison.
pub fn scroll_text_areas(
    focus: Option<Res<InputFocus>>,
    mut areas: Query<Area>,
    rows: Query<&Role>,
    mut rails: Query<(&AreaRail, &mut Visibility, &mut Node), Without<AreaThumb>>,
    mut thumbs: Query<(&AreaThumb, &mut Node), Without<AreaRail>>,
) {
    let editing = focus.as_ref().and_then(|focus| focus.get());
    for (field, node, layout, mut text, of) in &mut areas {
        // Only a filling area: an area of fixed height is left exactly as it was.
        if !rows.get(of.0).is_ok_and(|role| *role == Role::TextArea) {
            continue;
        }
        let size = node.size() * node.inverse_scale_factor();
        let mine = editing == Some(field);
        if text.viewport.size != size {
            text.viewport.size = size;
        }
        text.editor.set_width(Some(size.x));
        // A layout one frame behind the scroll is no reason to undo it: the text's height only
        // grows as the editor wraps a longer text, so an offset past a stale `max` is held until the
        // next layout says where the end is (a wheel to the end lands there in one step).
        let max = max_offset(node, layout);
        if mine && let Some((_, caret)) = layout.cursor {
            reveal(&mut text.viewport.offset.y, caret, size.y, max);
        }
        let offset = text.viewport.offset.y.max(0.0);
        if (text.viewport.offset.y - offset).abs() > 0.01 {
            text.viewport.offset.y = offset;
        }

        let height = content_height(layout);
        let share = (size.y / height.max(1.0)).clamp(MIN_THUMB_SHARE, 1.0);
        let length = (size.y * share).max(0.0);
        let travel = (size.y - length).max(0.0);
        let place = if max > 0.0 { offset / max } else { 0.0 };
        // The rail stands while there is more text than the field draws.
        // The rail stands while there is more text than the field draws: `Visibility` for a drawn
        // lane, `Display` so a hidden one takes no room in the cell (a rail is a flex item beside
        // the field, and a hidden one would still hold its width).
        for (rail, mut visibility, mut lane) in &mut rails {
            if rail.0 != field {
                continue;
            }
            let shown = max > 0.5;
            visibility.set_if_neq(if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            lane.set_if_neq(Node {
                display: if shown { Display::Flex } else { Display::None },
                ..lane.clone()
            });
        }
        for (thumb, mut lane_node) in &mut thumbs {
            if thumb.0 != field {
                continue;
            }
            let mut next = lane_node.clone();
            next.height = Val::Px(length);
            next.margin = UiRect::top(Val::Px(travel * place));
            lane_node.set_if_neq(next);
        }
    }
}

/// The least share of its lane a thumb takes, so a long text's thumb is still a target and a mark.
const MIN_THUMB_SHARE: f32 = 0.06;

/// Moves `offset` by the least that leaves `caret` inside the field, half a line of margin at each
/// edge, never past the text's end.
fn reveal(offset: &mut f32, caret: Rect, height: f32, max: f32) {
    let margin = caret.height().max(1.0) * 0.5;
    if caret.min.y - margin < *offset {
        *offset = (caret.min.y - margin).max(0.0);
    } else if caret.max.y + margin > *offset + height {
        *offset = (caret.max.y + margin - height).max(0.0);
    }
    *offset = offset.clamp(0.0, max);
}
