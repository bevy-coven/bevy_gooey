//! Ready-made widgets: free functions returning a bundle, so they compose inside `children![]` and
//! with [`crate::observe`]. Each carries a [`Role`], and [`crate::apply_skin`] styles it from the
//! skin; none of them takes a colour or a size.

use std::ops::RangeInclusive;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui_widgets::{
    Checkbox, Slider, SliderOrientation, SliderRange, SliderStep, SliderThumb, SliderValue,
    TrackClick, checkbox_self_update, observe, slider_self_update,
};

use crate::meter::Meter;
use crate::model::{meter_fraction, slider_fraction, slider_text};
use crate::nav::Dialog;
use crate::skin::Role;
use crate::toast::{Toast, ToastKind};

/// How many keyboard or D-pad steps span a [`slider`]'s range.
const SLIDER_STEPS: f32 = 20.0;

/// A toggle's state at spawn. A bundle cannot carry [`Checked`] only when the toggle starts on
/// (`Option<Checked>` is not a bundle), so this puts it on the entity as it is added and goes.
#[derive(Component)]
#[component(on_add = start_state)]
struct StartsOn(bool);

fn start_state(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let on = world.get::<StartsOn>(entity).is_some_and(|start| start.0);
    let mut commands = world.commands();
    let mut toggle = commands.entity(entity);
    toggle.remove::<StartsOn>();
    if on {
        toggle.insert(Checked);
    }
}

/// A full-window column that centres its children: the root of a menu screen or a HUD. It is a tab
/// group, so Tab cycles through the buttons on it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((screen(), children![label("Hello")]));
/// # }
/// ```
///
/// It is layout, not a surface: its empty part is [`Pickable::IGNORE`], so a HUD built on it lets
/// the pointer through to the sprites, meshes and other UI roots under it, while the widgets on it
/// ([`panel`], [`button`], [`meter`]…) still take the pointer. A click on the empty part still
/// blurs the interface: it lands on whatever is below, or on the window, and Bevy clears focus
/// there. A menu screen that must swallow the pointer opts back in on the entity (a tuple bundle
/// cannot carry two `Pickable`s, so `(screen(), Pickable::default())` panics):
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(screen()).insert(Pickable::default());
/// # }
/// ```
pub fn screen() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Role::Screen,
        TabGroup::new(0),
        Pickable::IGNORE,
    )
}

/// A framed, padded column on the skin's surface colour.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((panel(), children![heading("Inventory"), label("Empty")]));
/// # }
/// ```
pub fn panel() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Role::Panel,
    )
}

/// A column of buttons, stretched to the widest.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((menu(), children![button("New game"), button("Continue")]));
/// # }
/// ```
pub fn menu() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            ..default()
        },
        Role::Menu,
    )
}

/// Heading text, in the skin's heading size and font.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(heading("Paused"));
/// # }
/// ```
pub fn heading(text: impl Into<String>) -> impl Bundle {
    (Text::new(text), Role::Heading)
}

/// Body text, in the skin's text size and font.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(label("Health"));
/// # }
/// ```
pub fn label(text: impl Into<String>) -> impl Bundle {
    (Text::new(text), Role::Label)
}

/// A button: Bevy's headless [`bevy::ui_widgets::Button`], reachable by Tab and by arrows and the
/// D-pad, styled by its state. Observe [`crate::Activate`] on it to act.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((button("Resume"), observe(|_: On<Activate>| info!("resume"))));
/// # }
/// ```
pub fn button(text: impl Into<String>) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        bevy::ui_widgets::Button,
        Role::Button,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        children![(Text::new(text), Role::ButtonText)],
    )
}

/// A HUD meter (health, stamina, progress): a framed track with a fill `value / max` of its length.
/// The frame marks the whole length against the panel; the fill sits one border inside it, with
/// concentric rounded corners. It is [`crate::GooeySkin::meter_height`] high times the text scale,
/// so it grows with the player's size option. Change its [`Meter`] component to move it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn(meter(75.0, 100.0));
/// # }
/// ```
pub fn meter(value: f32, max: f32) -> impl Bundle {
    (
        Node::default(),
        Role::Meter,
        Meter { value, max },
        children![(
            Node {
                width: Val::Percent(meter_fraction(value, max) * 100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Role::MeterFill
        )],
    )
}

/// A switch for an option that takes effect the moment it flips (Fullscreen, Subtitles): a row with
/// the label on the left and a framed track with a sliding knob on the right. It is Bevy's headless
/// [`bevy::ui_widgets::Checkbox`], so a click anywhere on the row, Enter, Space and the pad's South
/// button flip it, and Tab, the arrows and the D-pad reach it. It keeps its own state, [`Checked`]
/// on the entity; observe `ValueChange<bool>` on it to apply the setting. The knob's place says
/// whether it is on, so the state survives a colour-blind player; the label never changes with it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     toggle("Subtitles", true),
///     observe(|change: On<ValueChange<bool>>| info!("subtitles: {}", change.value)),
/// ));
/// # }
/// ```
pub fn toggle(label: impl Into<String>, on: bool) -> impl Bundle {
    (
        Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        Checkbox,
        StartsOn(on),
        Role::Toggle,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(checkbox_self_update),
        children![
            (Text::new(label), Role::ControlText),
            (
                Node {
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::ToggleTrack,
                children![(Node::default(), Role::ToggleKnob)],
            ),
        ],
    )
}

/// A slider for a value the player scrubs (volume, brightness, sensitivity): a row with the label
/// and the value on one line and a rail under them with a thumb on it. It is Bevy's headless
/// [`bevy::ui_widgets::Slider`], and the whole row is the target: a click anywhere on it puts the
/// thumb under the pointer and a drag goes on from there. With it focused, Left and Right (keys or
/// the D-pad, held to repeat) step the value by a twentieth of the range, Home and End jump to the
/// ends, and Up and Down still move focus on, so one press is enough on a pad.
///
/// The value reads as a percentage for a range of 0 to 1 and as the number otherwise
/// ([`crate::slider_text`]). It keeps its own value, [`SliderValue`] on the entity; observe
/// `ValueChange<f32>` on it to apply the setting as it changes (`is_final` is false while a drag is
/// going). To step by something else, insert a [`SliderStep`] after spawning.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((
///     slider("Music", 0.7, 0.0..=1.0),
///     observe(|change: On<ValueChange<f32>>| info!("music: {}", change.value)),
/// ));
/// # }
/// ```
pub fn slider(label: impl Into<String>, value: f32, range: RangeInclusive<f32>) -> impl Bundle {
    let (min, max) = range.into_inner();
    let fraction = slider_fraction(value, min, max);
    (
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Slider {
            track_click: TrackClick::Snap,
            orientation: SliderOrientation::Horizontal,
        },
        SliderValue(value),
        SliderRange::new(min, max),
        SliderStep((max - min) / SLIDER_STEPS),
        Role::Slider,
        Hovered::default(),
        TabIndex(0),
        AutoDirectionalNavigation::default(),
        observe(slider_self_update),
        children![
            (
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::SliderHeader,
                children![
                    (Text::new(label), Role::ControlText),
                    (Text::new(slider_text(value, min, max)), Role::SliderReadout),
                ],
            ),
            (
                Node {
                    align_items: AlignItems::Center,
                    ..default()
                },
                Role::SliderTrack,
                children![
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        Role::SliderRail,
                        children![(
                            Node {
                                width: Val::Percent(fraction * 100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            Role::SliderFill,
                        )],
                    ),
                    (
                        Node {
                            flex_grow: fraction,
                            ..default()
                        },
                        Role::SliderGap,
                    ),
                    (Node::default(), Role::SliderThumb, SliderThumb),
                ],
            ),
        ],
    )
}

/// A modal dialog: a veil over the whole window that centres its children and holds focus. When it
/// opens, focus moves to its first button (or to the one carrying Bevy's `AutoFocus`, the way to
/// pick the least destructive default); Tab, the arrows and the D-pad stay inside it; and when it
/// closes, focus returns to the button that opened it. Its contents are yours, in `children![…]`;
/// close it with `commands.entity(dialog).despawn()`.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.spawn((dialog(), children![(panel(), children![label("Leave?"), button("Stay")])]));
/// # }
/// ```
pub fn dialog() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Role::Scrim,
        Dialog::default(),
        GlobalZIndex(100),
        TabGroup {
            order: 0,
            modal: true,
        },
    )
}

/// A line of news at the edge of the screen: "Game saved", "Quest updated". Spawn it from any
/// system, with no layout and nothing else to set up; it stands as long as its message takes to
/// read, fades out and removes itself ([`Toast`], [`crate::ToastSettings`]).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn save(mut commands: Commands) {
///     commands.spawn(toast("Game saved"));
/// }
/// ```
///
/// It takes no input and no focus, so it never gets in the way of what is under it; it is not
/// announced to screen readers. See [`Toast`] for its lifecycle.
pub fn toast(text: impl Into<String>) -> impl Bundle {
    toast_of(ToastKind::Info, text.into())
}

/// A [`toast`] for a refusal or a failure ("Inventory full", "Could not save"): the same line with
/// a mark in [`crate::GooeySkin::danger`] where an ordinary one's is the accent. The words say it
/// too; the mark is never the only sign.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn full(mut commands: Commands) {
///     commands.spawn(toast_alert("Inventory full"));
/// }
/// ```
pub fn toast_alert(text: impl Into<String>) -> impl Bundle {
    toast_of(ToastKind::Alert, text.into())
}

/// A toast's nodes: a clipped column that holds the room (its height is opened and closed by
/// [`crate::apply_skin`], and starts closed, so a toast never takes room before it is styled) around
/// the body, a framed row of a mark and the message. Everything starts transparent and
/// transparent to the pointer.
fn toast_of(kind: ToastKind, text: String) -> impl Bundle {
    (
        Toast::new(kind, &text),
        Node {
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            max_height: Val::Px(0.0),
            ..default()
        },
        Role::Toast,
        Pickable::IGNORE,
        children![(
            Node {
                flex_direction: FlexDirection::Row,
                ..default()
            },
            Role::ToastBody,
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Pickable::IGNORE,
            children![
                (
                    Node::default(),
                    Role::ToastMark,
                    BackgroundColor(Color::NONE),
                    Pickable::IGNORE,
                ),
                (
                    Text::new(text),
                    TextColor(Color::NONE),
                    Role::ToastText,
                    Pickable::IGNORE,
                ),
            ],
        )],
    )
}
