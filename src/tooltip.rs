//! Tooltips: a line of text beside a widget while the pointer has rested on it for a moment, or at
//! once while it has visible focus.

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseWheel;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::events::PointerPress;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{ComputedUiRenderTargetInfo, OverrideClip, UiGlobalTransform};

use crate::binding::BindingSettings;
use crate::shortcut::{ChordText, Shortcut};
use crate::skin::Role;
use crate::text::TextInputOf;
use bevy::ui_widgets::popover::Popover;

/// How long the pointer rests on a widget before its tooltip opens, seconds: a desktop's half a second,
/// so moving across a toolbar does not flash a row of tooltips (Rune's `help_delay`).
pub const TOOLTIP_DELAY_SECONDS: f32 = 0.5;

/// When a pointer's tooltip opens. Visible focus is not waited for: a keyboard or pad player has no
/// hover, so their tooltip opens in the frame focus arrives. `delay_seconds` 0 opens a tooltip as
/// the pointer arrives; negative or not a number is 0.
///
/// ```
/// use bevy_gooey::TooltipSettings;
/// assert_eq!(TooltipSettings::default().delay_seconds, 0.5);
/// ```
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct TooltipSettings {
    /// Seconds the pointer rests on a widget before its tooltip opens. Default: 0.5.
    pub delay_seconds: f32,
}

impl Default for TooltipSettings {
    fn default() -> Self {
        Self {
            delay_seconds: TOOLTIP_DELAY_SECONDS,
        }
    }
}

/// How long the pointer has rested on a tooltip's widget, and whether a press, a wheel turn or a key
/// ended this visit: the box then stays shut until the pointer leaves and comes back.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct TooltipDwell {
    seconds: f32,
    quiet: bool,
}

/// Shows its text in a box beside this entity while the pointer is over it, or while it has focus
/// and focus is visible (a keyboard or pad player has moved it), so every input reaches it.
///
/// The box opens across the flow of the entity's parent: to the right of a widget in a column,
/// below one in a row, on the other side if the window would clip it, so it never covers the next
/// widget in a menu; a long line wraps to fit the room beside the widget. It sits one
/// [`crate::GooeySkin::gap`] outside the panel the widget is in (its padding and border), drawn as
/// a small raised panel with a shadow. Adding it spawns the box as a hidden child, styled as
/// [`Role::Tooltip`], placed by Bevy's `Popover` once its widget is laid out (`apply_skin` inserts the
/// placement; a box that carried one at spawn panicked Bevy on the widget's first frame).
///
/// The box is a view of this component: removing it removes the box, and inserting a new
/// `Tooltip` over this one, or editing its text in place, changes the line the box shows (an
/// open tooltip keeps its place and does not blink; [`update_tooltip_text`] copies the text).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((button("Options"), Tooltip::new("Sound, controls and display")));
/// # }
/// ```
///
/// A command that has a keyboard shortcut shows it on a keycap beside the line, in the same words
/// and on the same cap as a [`prompt`](crate::prompt) and a [`binding`](crate::binding) row
/// ([`Shortcut::label`]: "Ctrl+S", or "⌘S" on a mac), so the player learns the chord where they
/// click. The cap exists exactly while the `shortcut` does; editing the tooltip in place
/// updates it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     button("Save"),
///     Tooltip::new("Write the world to disk").with_shortcut(Shortcut::primary(KeyCode::KeyS)),
/// ));
/// # }
/// ```
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Debug, PartialEq, Clone)]
#[component(on_insert = queue_box_sync, on_remove = queue_box_sync)]
pub struct Tooltip {
    /// The line the box shows.
    pub text: String,
    /// The chord shown on a keycap beside the line, if the command has one.
    pub shortcut: Option<Shortcut>,
}

impl Tooltip {
    /// A tooltip of one line, with no shortcut.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            shortcut: None,
        }
    }

    /// The same tooltip with `shortcut` shown on a keycap beside the line.
    pub fn with_shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

/// The box a [`Tooltip`] spawned, a child of the widget it describes: query it to find the box
/// (its [`Text`] is its child).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct TooltipBox {
    /// The widget's parent lays its children out in a row, so the box opens below or above it
    /// rather than beside it. [`crate::apply_skin`] places the box from it.
    pub(crate) in_row: bool,
    /// The widest gap between the widget and the window's left or right edge, logical px:
    /// [`crate::apply_skin`] wraps the box to fit in it.
    pub(crate) room: f32,
    /// The keycap's width, logical px, as last laid out (0 while the tooltip has no shortcut or has
    /// never been shown): the box is never narrower than the cap and a few words beside it.
    pub(crate) cap: f32,
}

/// Both hooks queue [`sync_box`] rather than act on what they saw: a flush that removes a
/// `Tooltip` and inserts another fires two hooks, and the command reads the entity when it runs.
fn queue_box_sync(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .queue(move |world: &mut World| sync_box(world, entity));
}

/// Makes the entity's [`TooltipBox`] exist exactly while it has a [`Tooltip`]: spawns one when
/// there is none, despawns it when the `Tooltip` is gone, and does nothing when the entity itself
/// is gone (Bevy has despawned its children). Idempotent, so any order of hooks ends in one box.
fn sync_box(world: &mut World, entity: Entity) {
    let Ok(owner) = world.get_entity(entity) else {
        return;
    };
    let tip = owner.get::<Tooltip>().cloned();
    let boxes: Vec<Entity> = owner
        .get::<Children>()
        .into_iter()
        .flat_map(|children| children.iter())
        .filter(|child| world.get::<TooltipBox>(*child).is_some())
        .collect();
    match (tip, boxes.is_empty()) {
        (Some(tip), true) => {
            // Hover is read from `Hovered`, which `button()` already carries; any other widget
            // gets one.
            world.entity_mut(entity).insert_if_new(Hovered::default());
            let tip_box = world
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        display: Display::None,
                        ..default()
                    },
                    Role::Tooltip,
                    // Born without a `Popover`: Bevy places every one it sees, and on the frame a
                    // widget is spawned its `ComputedNode` is a layout behind, so `size - border` is
                    // negative and `position_popover` panics ("Rect size must be positive", measured on
                    // `examples/inspector`, iteration 97). `apply_skin` inserts the placement once the
                    // anchor is laid out.
                    // A tooltip beside a row of a scrolling list stands outside the list, so the
                    // list's clip must not reach it.
                    OverrideClip,
                    BoxShadow::default(),
                    GlobalZIndex(200),
                    TooltipBox::default(),
                    TooltipDwell::default(),
                    ChildOf(entity),
                    children![(Text::new(tip.text), Role::Label)],
                ))
                .id();
            if let Some(shortcut) = tip.shortcut {
                let words = world
                    .get_resource::<BindingSettings>()
                    .map_or_else(|| shortcut.label(), |settings| settings.chord(shortcut));
                world.spawn((keycap(shortcut, words), ChildOf(tip_box)));
            }
        }
        (None, false) => {
            for tip in boxes {
                world.entity_mut(tip).despawn();
            }
        }
        _ => {}
    }
}

/// The keycap a tooltip's shortcut is drawn on: the prompts' cap, one line, in the host's own
/// spelling ([`ChordText`] rewrites it when the settings change).
fn keycap(shortcut: Shortcut, words: String) -> impl Bundle {
    (
        Text::new(words),
        ChordText(shortcut),
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
        Role::PromptKey,
    )
}

/// Copies each [`Tooltip`]'s text and shortcut into its box when the `Tooltip` changed: a new
/// Marks a tooltip box for a restyle the frame its anchor is first laid out.
///
/// A box is spawned without a `Popover` (Bevy's `position_popover` panics on an anchor whose border is
/// as big as its size, which is what a widget looks like on its first frame — iteration 97), and
/// [`crate::apply_skin`] inserts the placement once the anchor is sized. The box is not otherwise
/// restyled then: the node that changed is its *parent*, and `Changed<ComputedNode>` in `Restyle`
/// filters the box. Writing the box's `Role` is the crate's own invalidation for that, and it writes
/// only while the anchor is sized and the box has no `Popover` yet, so a settled tooltip costs nothing.
#[allow(clippy::type_complexity)]
pub fn restyle_sized_tooltips(
    mut boxes: Query<(&ChildOf, &mut Role), (With<TooltipBox>, Without<Popover>)>,
    anchors: Query<&ComputedNode>,
) {
    for (parent, mut role) in &mut boxes {
        if anchors
            .get(parent.parent())
            .is_ok_and(|node| node.size().x > 1.0 && node.size().y > 1.0)
        {
            role.set_changed();
        }
    }
}

/// `Tooltip` inserted over the old one, or an edit in place (a reflection-driven inspector makes
/// one, and no component hook runs for it). The keycap is spawned when a shortcut appears and
/// despawned when it goes. Writes only when the line differs. Runs in
/// [`crate::GooeySystems::Style`], before [`show_tooltips`].
pub fn update_tooltip_text(
    settings: Res<BindingSettings>,
    tooltips: Query<(&Tooltip, &Children), Changed<Tooltip>>,
    boxes: Query<&Children, With<TooltipBox>>,
    mut texts: Query<(&Role, &mut Text, Option<&mut ChordText>)>,
    mut commands: Commands,
) {
    for (tooltip, children) in &tooltips {
        for (tip_box, lines) in children
            .iter()
            .filter_map(|child| Some((child, boxes.get(child).ok()?)))
        {
            let mut cap = None;
            for line in lines {
                let Ok((role, mut text, chord)) = texts.get_mut(*line) else {
                    continue;
                };
                match *role {
                    Role::PromptKey => {
                        cap = Some(*line);
                        if let Some(shortcut) = tooltip.shortcut {
                            if let Some(mut chord) = chord {
                                chord.set_if_neq(ChordText(shortcut));
                            }
                            let label = settings.chord(shortcut);
                            if text.0 != label {
                                text.0 = label;
                            }
                        }
                    }
                    _ if text.0 != tooltip.text => text.0.clone_from(&tooltip.text),
                    _ => {}
                }
            }
            match (tooltip.shortcut, cap) {
                (Some(shortcut), None) => {
                    commands.spawn((keycap(shortcut, settings.chord(shortcut)), ChildOf(tip_box)));
                }
                (None, Some(cap)) => commands.entity(cap).despawn(),
                _ => {}
            }
        }
    }
}

/// Shows each [`Tooltip`] box while its widget is hovered or has visible focus (a
/// [`crate::text_input`] row has it while its field does), and hides it otherwise; records whether
/// the widget sits in a row and how much room the window leaves beside it, for
/// [`crate::apply_skin`] to place and fit the box. Writes only what changed. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`].
#[allow(clippy::too_many_arguments)]
pub fn show_tooltips(
    focus: Option<Res<InputFocus>>,
    visible: Option<Res<InputFocusVisible>>,
    settings: Option<Res<TooltipSettings>>,
    time: Option<Res<Time<Real>>>,
    mut wheel: Option<MessageReader<MouseWheel>>,
    mut keys: Option<MessageReader<KeyboardInput>>,
    widgets: Query<Described, Without<TooltipBox>>,
    flows: Query<&Node, Without<TooltipBox>>,
    fields: Query<&TextInputOf>,
    caps: Query<(&Role, &ComputedNode)>,
    mut boxes: Query<(
        &ChildOf,
        &mut Node,
        &mut TooltipBox,
        &mut TooltipDwell,
        &Children,
    )>,
) {
    let wanted = settings.map_or(TOOLTIP_DELAY_SECONDS, |s| s.delay_seconds);
    let delay = if wanted.is_finite() {
        wanted.max(0.0)
    } else {
        0.0
    };
    let dt = time.map_or(0.0, |t| t.delta_secs());
    // A wheel turn or a key press ends every visit that is on: the player is doing something else.
    let turned = wheel.as_mut().is_some_and(|w| w.read().count() > 0);
    let typed = keys
        .as_mut()
        .is_some_and(|k| k.read().any(|key| key.state == ButtonState::Pressed));
    let interrupted = turned || typed;
    let focused = visible
        .is_some_and(|v| v.0)
        .then(|| focus.and_then(|f| f.get()))
        .flatten();
    let focused_row = focused.and_then(|f| fields.get(f).ok()).map(|of| of.0);
    for (owner, mut node, mut tip, mut dwell, lines) in &mut boxes {
        let widget = owner.parent();
        let Ok((hovered, parent, placed)) = widgets.get(widget) else {
            continue;
        };
        let over = hovered.is_some_and(|h| h.get());
        // The visit: time on the widget, ended by a press, a wheel turn or a key, and over when the
        // pointer leaves.
        let mut visit = *dwell;
        if !over {
            visit = TooltipDwell::default();
        } else if interrupted {
            visit.quiet = true;
        } else if !visit.quiet && visit.seconds < delay {
            visit.seconds = (visit.seconds + dt).min(delay);
        }
        dwell.set_if_neq(visit);
        let shown = (over && !visit.quiet && visit.seconds >= delay)
            || focused == Some(widget)
            || focused_row == Some(widget);
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let in_row = parent
            .and_then(|p| flows.get(p.parent()).ok())
            .is_some_and(|n| {
                matches!(
                    n.flex_direction,
                    FlexDirection::Row | FlexDirection::RowReverse
                )
            });
        let room = placed.map_or(f32::INFINITY, |(computed, at, target)| {
            let scale = computed.inverse_scale_factor;
            let rect = Rect::from_center_size(at.translation * scale, computed.size() * scale);
            let window = target.logical_size();
            // Whole pixels, so sub-pixel layout noise never restyles the box.
            (window.x - rect.max.x).max(rect.min.x).floor()
        });
        // A hidden box lays nothing out: the cap's width stays what it was when last seen.
        let cap = lines
            .iter()
            .filter_map(|line| caps.get(line).ok())
            .find(|(role, _)| **role == Role::PromptKey)
            .map_or(0.0, |(_, computed)| {
                let seen = (computed.size().x * computed.inverse_scale_factor).ceil();
                if seen > 0.0 { seen } else { tip.cap }
            });
        tip.set_if_neq(TooltipBox { in_row, room, cap });
    }
}

/// A widget a tooltip describes: whether the pointer is over it, its parent, and where it is laid
/// out (its size, its position and the window it is drawn to).
type Described = (
    Option<&'static Hovered>,
    Option<&'static ChildOf>,
    Option<(
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static ComputedUiRenderTargetInfo,
    )>,
);

/// A press on a widget, or on anything inside it, ends its tooltip's visit: the box stays shut until
/// the pointer leaves and comes back. A global observer.
pub(crate) fn tooltip_on_press(
    press: On<PointerPress>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    mut dwells: Query<&mut TooltipDwell, With<TooltipBox>>,
) {
    // The press bubbles, and every global observer runs at each hop: act on the first only.
    if press.entity != press.original_event_target() {
        return;
    }
    let mut at = Some(press.entity);
    while let Some(entity) = at {
        for kid in children.get(entity).into_iter().flat_map(|c| c.iter()) {
            if let Ok(mut dwell) = dwells.get_mut(kid)
                && !dwell.quiet
            {
                dwell.quiet = true;
            }
        }
        at = parents.get(entity).ok().map(ChildOf::parent);
    }
}
