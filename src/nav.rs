//! Keyboard and gamepad navigation: arrows and the D-pad move focus between widgets, the pad's
//! South button activates the focused one. A pad never steers a cursor.

use bevy::input_focus::InputFocus;
use bevy::math::CompassOctant;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigator;
use bevy::ui_widgets::Activate;

/// Moves focus to the nearest widget in the direction the arrow keys or any gamepad's D-pad were
/// just pressed (diagonals included). Runs in [`crate::GooeySystems::Navigate`]; no neighbour that
/// way leaves focus where it is.
pub fn navigate_by_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut nav: AutoDirectionalNavigator,
) {
    let pressed = |key: KeyCode, pad: GamepadButton| {
        keys.just_pressed(key) || gamepads.iter().any(|g| g.just_pressed(pad))
    };
    let axis = |plus: bool, minus: bool| plus as i8 as f32 - minus as i8 as f32;
    let net_x = axis(
        pressed(KeyCode::ArrowRight, GamepadButton::DPadRight),
        pressed(KeyCode::ArrowLeft, GamepadButton::DPadLeft),
    );
    let net_y = axis(
        pressed(KeyCode::ArrowUp, GamepadButton::DPadUp),
        pressed(KeyCode::ArrowDown, GamepadButton::DPadDown),
    );
    if let Some(direction) = Dir2::from_xy(net_x, net_y).ok().map(CompassOctant::from) {
        // An error is "no neighbour that way": focus stays, which is the answer.
        let _ = nav.navigate(direction);
    }
}

/// Activates the focused, enabled button when any gamepad's South button (A on Xbox, Cross on
/// PlayStation) is just pressed. Enter and Space are `bevy_ui_widgets`' own, so they are not
/// handled here: a second handler would activate twice.
pub fn gamepad_select(
    focus: Res<InputFocus>,
    gamepads: Query<&Gamepad>,
    buttons: Query<(), (With<bevy::ui_widgets::Button>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if !gamepads
        .iter()
        .any(|g| g.just_pressed(GamepadButton::South))
    {
        return;
    }
    if let Some(entity) = focus.get()
        && buttons.contains(entity)
    {
        commands.trigger(Activate { entity });
    }
}
