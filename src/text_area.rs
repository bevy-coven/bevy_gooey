//! A text area that fills its host and scrolls: the plain multi-line field (not the code editor of
//! G-71), as tall as the room its parent leaves, whose wheel and rail move the text. Its text is
//! [`crate::TextInputValue`]'s, exactly as a fixed-height area's.
//!
//! Bevy already owns the viewport's size (`sync_editable_text_viewports` makes it the field's own
//! box) and reveals the caret on every edit, every arrow, and the frame focus arrives. This module
//! owns what Bevy does not: the wheel, the rail and the rail's thumb — and one correction, because
//! the focus frame's reveal runs *before* the press that focused the field moves the caret, so a
//! click in a scrolled area snapped the view back to the stale caret at the top. So the reveal here
//! runs only when the caret itself moved ([`AreaCaret`]), which is a player's own edit, arrow, click
//! or the game's write: a wheel turn moves no caret, and a settled area writes nothing at all.
//!
//! Every length is converted at its edge, and there are three. A **skin pixel** is what a `Val::Px`
//! is written in. A **physical pixel** is what `ComputedNode` sizes, positions, `TextLayoutInfo`
//! sizes and `TextViewport` are in: `target.scale_factor()` of skin pixels, which is the window's own
//! scale times the interface's (`UiScale`). A **logical pixel** is what a pointer's position is in:
//! the physical ones over the window's scale. [`ComputedUiRenderTargetInfo::scale_factor`] and
//! `UiScale` give the window's scale between them, which is how `color` reads a pointer too.
//!
//! The rail is the list's own look — `Role::ListRail` and `Role::ListThumb` — so an area's rail and
//! a list's agree on the eye, while the text scrolls `EditableText`'s viewport rather than a node's
//! `ScrollPosition`.

use bevy::input::mouse::{MouseScrollPixelsPerLine, MouseWheel};
use bevy::input_focus::AcquireFocus;
use bevy::picking::events::{PointerDrag, PointerPress};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::text::{EditableText, TextLayoutInfo};
use bevy::ui::{ComputedUiRenderTargetInfo, UiGlobalTransform};

use crate::text::TextInputOf;

/// A [`crate::text_area_fill`]: on the row, it is the area whose field takes the room its host
/// leaves, scrolls with the wheel and its rail, and wraps a long line. Spawning the widget puts it
/// there; a host reads it to find one, and never adds it to a fixed [`crate::text_area`] (that one
/// keeps its `rows`).
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

/// What the pointer is over, in a filling area: the rail, the thumb, or the field itself.
type PointerOver = (
    Option<&'static AreaRail>,
    Option<&'static AreaThumb>,
    Option<&'static TextInputOf>,
);

/// The field a pointer over `over` scrolls: the field itself, or the field a rail or a thumb
/// belongs to. `None` for a node that is none of them.
fn field_under(over: Entity, names: &Query<PointerOver>) -> Option<Entity> {
    let (rail, thumb, of) = names.get(over).ok()?;
    if let Some(rail) = rail {
        return Some(rail.0);
    }
    if let Some(thumb) = thumb {
        return Some(thumb.0);
    }
    // The field is the node under the wheel; `TextInputOf` on it names the row (that a press on the
    // row's label or padding focuses it), not the thing that scrolls.
    of.map(|_| over)
}

/// How far down a field may scroll, in its text's own pixels: its text's height, less the part of it
/// the field draws.
fn max_offset(layout: &TextLayoutInfo, viewport: &bevy::text::TextViewport) -> f32 {
    (layout.size.y - viewport.size.y).max(0.0)
}

/// How far `logical` pixels are in a field's layout pixels.
fn to_layout(logical: f32, target: &ComputedUiRenderTargetInfo, ui_scale: f32) -> f32 {
    if ui_scale > 0.0 {
        logical * target.scale_factor() / ui_scale
    } else {
        logical
    }
}

/// A wheel turn scrolls the filling area the pointer is over.
///
/// The wheel is read from the platform's own messages, not from a picking event on the field: a text
/// field's own handler is what a scroll over an editable node reaches, and a listener for a
/// `PointerScroll` on the field hears nothing. The pointer's last hover (Bevy's `HoverMap`) says
/// which node it is over, so a wheel over the rail, the thumb or the field all scroll the field they
/// belong to, and one over the row's label or the panel behind it scrolls nothing.
///
/// One line is [`MouseScrollPixelsPerLine`] logical pixels; an offset is in the field's layout
/// pixels.
pub(crate) fn area_on_scroll(
    // `Option`: a style-only app (a test that adds `GooeyPlugins` and nothing else) has no
    // `MouseWheel` messages, and a bare reader panics its system. `tooltip.rs` reads the wheel the
    // same way.
    mut wheels: Option<MessageReader<MouseWheel>>,
    hover: Option<Res<bevy::picking::hover::HoverMap>>,
    names: Query<PointerOver>,
    mut fields: Query<(
        &ComputedNode,
        &TextLayoutInfo,
        &ComputedUiRenderTargetInfo,
        &mut EditableText,
    )>,
    parents: Query<&ChildOf>,
    // Both are `Option` for the same reason as the wheel: a style-only app has neither.
    ratio: Option<Res<MouseScrollPixelsPerLine>>,
    ui_scale: Option<Res<UiScale>>,
) {
    let lines = ratio.as_deref().copied().unwrap_or_default();
    let Some(wheels) = wheels.as_mut() else {
        return;
    };
    let mut pixels = 0.0;
    for wheel in wheels.read() {
        pixels += match wheel.unit {
            // One line is `MouseScrollPixelsPerLine` logical pixels.
            bevy::input::mouse::MouseScrollUnit::Line => wheel.y * lines,
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
        // The frontmost node under the pointer: the first that names a filling area scrolls it.
        let Some((&hit, _)) = hits.iter().min_by(|a, b| a.1.depth.total_cmp(&b.1.depth)) else {
            continue;
        };
        let field = std::iter::once(hit)
            .chain(parents.iter_ancestors(hit))
            .find_map(|entity| field_under(entity, &names));
        let Some(field) = field else {
            continue;
        };
        let Ok((_, layout, target, mut text)) = fields.get_mut(field) else {
            continue;
        };
        let scroll = to_layout(
            pixels,
            target,
            ui_scale.as_ref().map_or(1.0, |scale| scale.0),
        );
        let max = max_offset(layout, &text.viewport);
        let wanted = (text.viewport.offset.y - scroll).clamp(0.0, max);
        if (text.viewport.offset.y - wanted).abs() > 0.01 {
            text.viewport.offset.y = wanted;
        }
    }
}

/// How far down its lane a pointer is, from 0 at the top to 1 at the bottom: the pointer's share of
/// the rail's own box, read through the rail's transform so a scaled or scrolled rail is right.
fn share_along(
    pointer: Vec2,
    lane: &ComputedNode,
    at: &UiGlobalTransform,
    target: &ComputedUiRenderTargetInfo,
    ui_scale: f32,
) -> f32 {
    let at_layout = if ui_scale > 0.0 {
        pointer * target.scale_factor() / ui_scale
    } else {
        pointer
    };
    lane.normalize_point(*at, at_layout)
        .map(|normalized| (normalized.y + 0.5).clamp(0.0, 1.0))
        .unwrap_or(0.0)
}

/// The rail a pointer at `target` is on, walking up from what was pressed: the thumb covers the lane
/// (a rail's lane is one target, however thin its thumb), and a press inside the rail's subtree must
/// land on the rail whatever node is under it.
fn rail_under<'a>(
    target: Entity,
    rails: &'a Query<(
        &AreaRail,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
    )>,
    parents: &Query<&ChildOf>,
) -> Option<(
    &'a AreaRail,
    &'a ComputedNode,
    &'a UiGlobalTransform,
    &'a ComputedUiRenderTargetInfo,
)> {
    let rail = std::iter::once(target)
        .chain(parents.iter_ancestors(target))
        .find(|&entity| rails.get(entity).is_ok())?;
    rails.get(rail).ok()
}

/// The rail reads and writes [`scroll_to_pointer`] shares with its two callers.
type RailGeometry<'a> = (
    &'a AreaRail,
    &'a ComputedNode,
    &'a UiGlobalTransform,
    &'a ComputedUiRenderTargetInfo,
);

/// Scrolls the field a rail belongs to so its share of the text matches the pointer's place on the
/// lane.
fn scroll_to_pointer(
    geometry: RailGeometry<'_>,
    pointer: Vec2,
    fields: &Query<(&ComputedNode, &TextLayoutInfo)>,
    texts: &mut Query<&mut EditableText>,
    ui_scale: f32,
) {
    let (rail, lane, at, target) = geometry;
    let Ok((_, layout)) = fields.get(rail.0) else {
        return;
    };
    let Ok(mut text) = texts.get_mut(rail.0) else {
        return;
    };
    let wanted =
        share_along(pointer, lane, at, target, ui_scale) * max_offset(layout, &text.viewport);
    if (text.viewport.offset.y - wanted).abs() > 0.01 {
        text.viewport.offset.y = wanted;
    }
}

/// A press on a filling area's rail — or on the thumb in it — scrolls the text there: the pointer's
/// share of the lane is the viewport's share of what the field does not show.
///
/// A **global** observer, so it is seen at the rail whatever the press first landed on.
pub(crate) fn rail_on_press(
    mut press: On<PointerPress>,
    rails: Query<(
        &AreaRail,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
    )>,
    fields: Query<(&ComputedNode, &TextLayoutInfo)>,
    mut texts: Query<&mut EditableText>,
    parents: Query<&ChildOf>,
    ui_scale: Option<Res<UiScale>>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let target = press.original_event_target();
    let Some(geometry) = rail_under(target, &rails, &parents) else {
        return;
    };
    press.propagate(false);
    scroll_to_pointer(
        geometry,
        press.pointer.position,
        &fields,
        &mut texts,
        ui_scale.as_ref().map_or(1.0, |scale| scale.0),
    );
}

/// A drag on the rail follows the pointer: its share of the lane is the viewport's share again.
pub(crate) fn rail_on_drag(
    mut drag: On<PointerDrag>,
    rails: Query<(
        &AreaRail,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
    )>,
    fields: Query<(&ComputedNode, &TextLayoutInfo)>,
    mut texts: Query<&mut EditableText>,
    parents: Query<&ChildOf>,
    ui_scale: Option<Res<UiScale>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let target = drag.original_event_target();
    let Some(geometry) = rail_under(target, &rails, &parents) else {
        return;
    };
    drag.propagate(false);
    scroll_to_pointer(
        geometry,
        drag.pointer.position,
        &fields,
        &mut texts,
        ui_scale.as_ref().map_or(1.0, |scale| scale.0),
    );
}

/// A press on a filling area's rail does not focus its field: a scrollbar press scrolls, it does not
/// move the caret, and focus would put the caret — and so the view — back where the caret is.
///
/// A global `AcquireFocus` observer, which runs before `text_row_on_acquire`'s: the row under a rail
/// turns a press into focus for its field, so this stops it when the press began in a rail.
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

/// Where a filling area's caret stood when it was last looked at: the reveal runs when it moves, so
/// a wheel turn or a settled frame never fights Bevy's own viewport.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct AreaCaret {
    seen: Option<Rect>,
}

/// The least share of its lane a thumb takes, so a long text's thumb is still a target and a mark.
const MIN_THUMB_SHARE: f32 = 0.06;

/// What [`update_area_rails`] reads of a filling area's field.
type Area = (
    Entity,
    &'static ComputedNode,
    &'static TextLayoutInfo,
    &'static mut EditableText,
    &'static TextInputOf,
    &'static mut AreaCaret,
);

/// Keeps every filling area's view honest: the caret is brought into the field when it moves, and the
/// rail shows while there is more text than the field draws with its thumb the share shown, standing
/// where the field stands.
///
/// Runs in [`crate::GooeySystems::Style`] after [`crate::apply_skin`], so it reads this frame's
/// layout. Every write is on a difference — the viewport only when the caret moved out of it, a
/// rail's `Visibility` and its thumb's `Node` only when the answer changed — so a settled area costs
/// a comparison and no write.
pub(crate) fn update_area_rails(
    mut areas: Query<Area>,
    fills: Query<(), With<TextAreaFill>>,
    mut rails: Query<(&AreaRail, &mut Visibility, &mut Node), Without<AreaThumb>>,
    mut thumbs: Query<(&AreaThumb, &mut Node), Without<AreaRail>>,
) {
    for (field, node, layout, mut text, of, mut shown) in &mut areas {
        // Only a filling area: a fixed one is as tall as its rows and scrolls itself.
        if !fills.contains(of.0) {
            continue;
        }
        let max = max_offset(layout, &text.viewport);
        // The caret is brought into view when it has moved, and never otherwise: Bevy's own reveal on
        // a focus change reads the caret *before* the press that focused the field has moved it, and
        // that reveal is what snapped a scrolled area back to the top.
        let caret = layout.cursor.map(|(_, rect)| rect);
        if caret != shown.seen {
            shown.seen = caret;
            if let Some(caret) = caret {
                reveal(&mut text.viewport, caret, max);
            }
        }
        // The thumb and its travel are a `Val` (skin pixels) and the text is in the field's own
        // pixels: the node's `inverse_scale_factor` is the one between them.
        let to_skin = node.inverse_scale_factor();
        let view = text.viewport.size.y * to_skin;
        let height = (layout.size.y * to_skin).max(1.0);
        let share = (view / height).clamp(MIN_THUMB_SHARE, 1.0);
        let length = view * share;
        let travel = (view - length).max(0.0);
        let place = if max > 0.0 {
            (text.viewport.offset.y / max).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // A hidden rail keeps no room in the cell: it is a flex item beside the field, so one that
        // only went `Visibility::Hidden` would still hold its width.
        let drawn = max > 0.5;
        for (rail, mut visibility, mut lane) in &mut rails {
            if rail.0 != field {
                continue;
            }
            visibility.set_if_neq(if drawn {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
            lane.set_if_neq(Node {
                display: if drawn { Display::Flex } else { Display::None },
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

/// Moves the viewport by the least that leaves `caret` inside the field, half a line of margin at
/// each edge, and never past the text's end.
fn reveal(viewport: &mut bevy::text::TextViewport, caret: Rect, max: f32) {
    let margin = caret.height().max(1.0) * 0.5;
    let offset = &mut viewport.offset.y;
    let height = viewport.size.y;
    if caret.min.y - margin < *offset {
        *offset = (caret.min.y - margin).max(0.0);
    } else if caret.max.y + margin > *offset + height {
        *offset = (caret.max.y + margin - height).max(0.0);
    }
    *offset = offset.clamp(0.0, max);
}
