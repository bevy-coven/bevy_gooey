//! Ready-made widgets: free functions returning a bundle, so they compose inside `children![]` and
//! with [`crate::observe`]. Each carries a [`Role`], and [`crate::apply_skin`] styles it from the
//! skin; none of them takes a colour or a size.

use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;

use crate::meter::Meter;
use crate::model::meter_fraction;
use crate::nav::Dialog;
use crate::skin::Role;

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

/// A HUD meter (health, stamina, progress): a track with a fill `value / max` of its length, drawn
/// inside it with concentric rounded corners. It is [`crate::GooeySkin::meter_height`] high times
/// the text scale, so it grows with the player's size option. Change its [`Meter`] component to
/// move it.
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
