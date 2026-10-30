//! A modal dialog keeps focus: it takes focus when it opens, the arrow keys cannot move it behind
//! the scrim, and closing the dialog gives it back. Laid out for real in a headless window.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{AutoFocus, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A headless app with a real window size, so UI layout and directional navigation run.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>(),
        GooeyPlugins,
    ));
    app.world_mut().spawn(Camera2d);
    app
}

/// Presses and releases `key` as the window's own keyboard input, one update each.
fn tap(app: &mut App, key_code: KeyCode) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

/// The menu behind: Play, Options, Quit, in a column. Returns them in that order.
fn menu_screen(app: &mut App) -> [Entity; 3] {
    let world = app.world_mut();
    let play = world.spawn((button("Play"), AutoFocus)).id();
    let options = world.spawn(button("Options")).id();
    let quit = world.spawn(button("Quit")).id();
    let column = world.spawn(menu()).add_children(&[play, options, quit]).id();
    world.spawn(screen()).add_child(column);
    [play, options, quit]
}

#[derive(Component)]
struct Leave;

#[derive(Component)]
struct Stay;

/// A dialog over it with Leave above Stay, spawned in one call as a game does. Returns the dialog,
/// Leave and Stay.
fn quit_dialog(app: &mut App, autofocus_stay: bool) -> [Entity; 3] {
    let world = app.world_mut();
    let dialog = if autofocus_stay {
        world.spawn((
            dialog(),
            children![(
                panel(),
                children![(
                    menu(),
                    children![(button("Leave"), Leave), (button("Stay"), Stay, AutoFocus)]
                )]
            )],
        ))
    } else {
        world.spawn((
            dialog(),
            children![(
                panel(),
                children![(
                    menu(),
                    children![(button("Leave"), Leave), (button("Stay"), Stay)]
                )]
            )],
        ))
    }
    .id();
    let leave = world.query_filtered::<Entity, With<Leave>>().single(world);
    let stay = world.query_filtered::<Entity, With<Stay>>().single(world);
    [dialog, leave.expect("Leave"), stay.expect("Stay")]
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

#[test]
fn arrows_never_move_focus_behind_an_open_dialog() {
    let mut app = app();
    let [play, options, quit] = menu_screen(&mut app);
    settle(&mut app);
    assert_eq!(focus(&app), Some(play));
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focus(&app), Some(options), "no dialog: arrows reach the menu");
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focus(&app), Some(quit));

    let [dialog, leave, stay] = quit_dialog(&mut app, true);
    settle(&mut app);
    assert_eq!(focus(&app), Some(stay));
    for key in [
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
    ] {
        tap(&mut app, key);
        assert_eq!(focus(&app), Some(stay), "{key:?} left the dialog");
    }
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focus(&app), Some(leave), "Up moves within the dialog");
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focus(&app), Some(leave), "Up from the top stays in the dialog");
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focus(&app), Some(stay));
    tap(&mut app, KeyCode::Tab);
    tap(&mut app, KeyCode::Tab);
    assert_eq!(focus(&app), Some(stay), "Tab cycles Leave and Stay only");

    app.world_mut().entity_mut(dialog).despawn();
    settle(&mut app);
    assert_eq!(
        focus(&app),
        Some(quit),
        "the opener, not the AutoFocus child"
    );
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focus(&app), Some(options), "the menu navigates again");
}

#[test]
fn an_opening_dialog_takes_focus_and_gives_it_back_when_it_closes() {
    let mut app = app();
    let [play, _, quit] = menu_screen(&mut app);
    settle(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(quit, bevy::input_focus::FocusCause::Navigated);

    let [dialog, leave, _] = quit_dialog(&mut app, false);
    settle(&mut app);
    assert_eq!(focus(&app), Some(leave), "no AutoFocus: its first button");

    app.world_mut().entity_mut(dialog).despawn();
    settle(&mut app);
    assert_eq!(focus(&app), Some(quit), "closing returns focus to the opener");

    // A host that moves focus itself before closing keeps its choice.
    let [dialog, _, stay] = quit_dialog(&mut app, true);
    settle(&mut app);
    assert_eq!(focus(&app), Some(stay), "AutoFocus picks the dialog's default");
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(play, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().entity_mut(dialog).despawn();
    settle(&mut app);
    assert_eq!(focus(&app), Some(play));
}
