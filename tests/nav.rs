//! The pad's South button activates the focused button, and never a disabled one.

use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_gooey::*;

#[derive(Resource, Default)]
struct Count(u32);

#[test]
fn gamepad_south_activates_the_focused_button() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<InputFocus>()
        .init_resource::<InputFocusVisible>()
        .init_resource::<Count>()
        .add_systems(Update, bevy_gooey::gamepad_select);
    let go = app
        .world_mut()
        .spawn((
            button("Go"),
            observe(|_: On<Activate>, mut count: ResMut<Count>| count.0 += 1),
        ))
        .id();
    app.insert_resource(InputFocus::from_entity(go));
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    let press = |app: &mut App| {
        if let Some(mut gamepad) = app.world_mut().get_mut::<Gamepad>(pad) {
            gamepad.digital_mut().release(GamepadButton::South);
            gamepad.digital_mut().press(GamepadButton::South);
        }
    };

    press(&mut app);
    app.update();
    assert_eq!(app.world().resource::<Count>().0, 1);

    app.world_mut().entity_mut(go).insert(InteractionDisabled);
    press(&mut app);
    app.update();
    assert_eq!(app.world().resource::<Count>().0, 1);
}
