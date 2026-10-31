//! Cancel: Escape and a pad's East button back out. One event starts at the focused widget and
//! bubbles up to the dialog or screen a game observes; a dialog is the end of the road; with no
//! widget focused an open dialog still hears it and a game with no menu open does not. Laid out
//! for real in a headless window, driven by real keyboard and gamepad input.

use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{AutoFocus, FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::Checked;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `Cancel` an observer heard: who heard it, where it was at, and the widget it started at.
#[derive(Resource, Default)]
struct Heard(Vec<(&'static str, Entity, Entity)>);

/// Every `ValueChange` and `Activate` a game would see from the controls.
#[derive(Resource, Default)]
struct Moves(u32);

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
    ))
    .init_resource::<Heard>()
    .init_resource::<Moves>()
    .add_observer(|_: On<ValueChange<f32>>, mut moves: ResMut<Moves>| moves.0 += 1)
    .add_observer(|_: On<ValueChange<bool>>, mut moves: ResMut<Moves>| moves.0 += 1)
    .add_observer(|_: On<ValueChange<usize>>, mut moves: ResMut<Moves>| moves.0 += 1)
    .add_observer(|_: On<Activate>, mut moves: ResMut<Moves>| moves.0 += 1);
    app.world_mut().spawn(Camera2d);
    app
}

/// An observer that records every `Cancel` it hears as `label`, and lets it go on.
fn listens(app: &mut App, entity: Entity, label: &'static str) {
    app.world_mut().entity_mut(entity).observe(
        move |cancel: On<Cancel>, mut heard: ResMut<Heard>| {
            heard
                .0
                .push((label, cancel.entity, cancel.original_event_target()));
        },
    );
}

fn primary_window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

/// One keyboard event for `key_code`, then one update.
fn key(app: &mut App, key_code: KeyCode, state: ButtonState, repeat: bool) {
    let window = primary_window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: None,
        repeat,
        window,
    });
    app.update();
}

/// Presses and releases `key_code`, one update each.
fn tap(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed, false);
    key(app, key_code, ButtonState::Released, false);
}

/// Plugs in a gamepad, as the platform's own connection event, and returns it.
fn plug_in_pad(app: &mut App) -> Entity {
    let pad = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Test pad".into(),
            vendor_id: None,
            product_id: None,
        },
    ));
    app.update();
    pad
}

/// Presses and releases `button` on `pad` as the platform's raw input, one update each.
fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn focus_visible(app: &App) -> bool {
    app.world().resource::<InputFocusVisible>().0
}

fn heard(app: &App) -> Vec<(&'static str, Entity, Entity)> {
    app.world().resource::<Heard>().0.clone()
}

/// A screen with Play (focused at start) and Quit in a menu. Returns the screen, Play and Quit.
fn menu_screen(app: &mut App) -> [Entity; 3] {
    let world = app.world_mut();
    let play = world.spawn((button("Play"), AutoFocus)).id();
    let quit = world.spawn(button("Quit")).id();
    let column = world.spawn(menu()).add_children(&[play, quit]).id();
    let screen = world.spawn(screen()).add_child(column).id();
    [screen, play, quit]
}

#[derive(Component)]
struct Stay;

#[derive(Component)]
struct DialogPanel;

/// A dialog under `parent` with Leave and Stay (Stay focused), in a panel, spawned in one call as
/// a game does (so the dialog remembers who had focus before its `AutoFocus` child takes it).
/// Returns the dialog, its panel and Stay.
fn quit_dialog(app: &mut App, parent: Entity) -> [Entity; 3] {
    let world = app.world_mut();
    let dialog = world
        .spawn((
            dialog(),
            children![(
                panel(),
                DialogPanel,
                children![(
                    menu(),
                    children![button("Leave"), (button("Stay"), Stay, AutoFocus)]
                )]
            )],
        ))
        .id();
    world.entity_mut(parent).add_child(dialog);
    let panel = world
        .query_filtered::<Entity, With<DialogPanel>>()
        .single(world)
        .expect("the dialog's panel");
    let stay = world
        .query_filtered::<Entity, With<Stay>>()
        .single(world)
        .expect("Stay");
    [dialog, panel, stay]
}

#[test]
fn escape_reaches_the_screen_from_the_focused_widget_and_shows_focus() {
    let mut app = app();
    let [screen, play, _] = menu_screen(&mut app);
    listens(&mut app, screen, "screen");
    settle(&mut app);
    assert_eq!(focus(&app), Some(play));
    assert!(!focus_visible(&app), "a pointer-less start shows no ring");

    tap(&mut app, KeyCode::Escape);

    assert_eq!(heard(&app), vec![("screen", screen, play)]);
    assert!(focus_visible(&app), "a key shows focus");
    assert_eq!(focus(&app), Some(play), "Back moves nothing by itself");
}

#[test]
fn a_pads_east_button_is_the_same_event() {
    let mut app = app();
    let [screen, play, _] = menu_screen(&mut app);
    listens(&mut app, screen, "screen");
    let pad = plug_in_pad(&mut app);
    settle(&mut app);

    press(&mut app, pad, GamepadButton::East);

    assert_eq!(heard(&app), vec![("screen", screen, play)]);
    assert!(focus_visible(&app), "the pad shows focus");

    // South is Activate's, West and North are the game's: none of them is a Back.
    for other in [GamepadButton::West, GamepadButton::North] {
        press(&mut app, pad, other);
    }
    assert_eq!(heard(&app).len(), 1);
}

#[test]
fn one_press_is_one_cancel_however_long_it_is_held() {
    let mut app = app();
    let [screen, _, _] = menu_screen(&mut app);
    listens(&mut app, screen, "screen");
    settle(&mut app);

    key(&mut app, KeyCode::Escape, ButtonState::Pressed, false);
    for _ in 0..5 {
        key(&mut app, KeyCode::Escape, ButtonState::Pressed, true);
    }
    for _ in 0..5 {
        app.update();
    }
    key(&mut app, KeyCode::Escape, ButtonState::Released, false);
    assert_eq!(heard(&app).len(), 1, "a held Escape and its repeats");

    tap(&mut app, KeyCode::Escape);
    assert_eq!(heard(&app).len(), 2, "a second press is a second Back");
}

#[test]
fn a_dialog_hears_it_and_the_screen_it_was_spawned_under_does_not() {
    let mut app = app();
    let [screen, _, quit] = menu_screen(&mut app);
    listens(&mut app, screen, "screen");
    settle(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(quit, FocusCause::Navigated);
    let [dialog, _, stay] = quit_dialog(&mut app, screen);
    listens(&mut app, dialog, "dialog");
    settle(&mut app);
    assert_eq!(focus(&app), Some(stay), "the dialog took focus");

    tap(&mut app, KeyCode::Escape);

    assert_eq!(
        heard(&app),
        vec![("dialog", dialog, stay)],
        "the dialog is a scope: the screen above it never hears a Back inside it"
    );
}

#[test]
fn the_innermost_observer_can_keep_it_from_the_layer_above() {
    let mut app = app();
    let [screen, ..] = menu_screen(&mut app);
    settle(&mut app);
    let [dialog, panel, stay] = quit_dialog(&mut app, screen);
    listens(&mut app, dialog, "dialog");
    app.world_mut().entity_mut(panel).observe(
        |mut cancel: On<Cancel>, mut heard: ResMut<Heard>| {
            heard
                .0
                .push(("panel", cancel.entity, cancel.original_event_target()));
            cancel.propagate(false);
        },
    );
    settle(&mut app);

    tap(&mut app, KeyCode::Escape);

    assert_eq!(heard(&app), vec![("panel", panel, stay)]);
}

#[test]
fn an_open_dialog_hears_it_with_nothing_focused_or_with_focus_behind_it() {
    let mut app = app();
    let [screen, _, quit] = menu_screen(&mut app);
    listens(&mut app, screen, "screen");
    settle(&mut app);
    let [dialog, ..] = quit_dialog(&mut app, screen);
    listens(&mut app, dialog, "dialog");
    settle(&mut app);

    app.world_mut().resource_mut::<InputFocus>().clear();
    tap(&mut app, KeyCode::Escape);
    assert_eq!(
        heard(&app),
        vec![("dialog", dialog, dialog)],
        "nothing focused: a dialog can still be dismissed"
    );

    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(quit, FocusCause::Navigated);
    tap(&mut app, KeyCode::Escape);
    assert_eq!(
        heard(&app).last(),
        Some(&("dialog", dialog, dialog)),
        "focus behind the dialog: the dialog still hears it, not the button there"
    );
    assert_eq!(heard(&app).len(), 2);
}

#[test]
fn with_no_dialog_and_nothing_focused_escape_is_the_games_own() {
    let mut app = app();
    let [screen, ..] = menu_screen(&mut app);
    listens(&mut app, screen, "screen");
    settle(&mut app);

    app.world_mut().resource_mut::<InputFocus>().clear();
    tap(&mut app, KeyCode::Escape);
    let window = primary_window(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(window, FocusCause::Navigated);
    tap(&mut app, KeyCode::Escape);

    assert!(
        heard(&app).is_empty(),
        "no focus, or the window Bevy focuses at start-up, is not a widget to back out of"
    );
    assert!(
        !focus_visible(&app),
        "and a press that did nothing shows nothing"
    );
}

#[test]
fn a_dialog_closed_by_its_cancel_gives_focus_back_to_the_button_that_opened_it() {
    let mut app = app();
    let [screen, _, quit] = menu_screen(&mut app);
    settle(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(quit, FocusCause::Navigated);
    let [dialog, _, stay] = quit_dialog(&mut app, screen);
    app.world_mut()
        .entity_mut(dialog)
        .observe(|cancel: On<Cancel>, mut commands: Commands| {
            commands.entity(cancel.entity).despawn();
        });
    settle(&mut app);
    assert_eq!(focus(&app), Some(stay));

    let pad = plug_in_pad(&mut app);
    press(&mut app, pad, GamepadButton::East);

    assert!(
        app.world().get_entity(dialog).is_err(),
        "the observer closed the dialog"
    );
    assert_eq!(focus(&app), Some(quit), "focus is on Quit again");
    assert!(focus_visible(&app), "and the ring shows where it is");
}

#[test]
fn a_back_press_changes_no_control() {
    let mut app = app();
    let world = app.world_mut();
    let volume = world.spawn(slider("Volume", 0.5, 0.0..=1.0)).id();
    let quality = world.spawn(choice("Quality", ["Low", "High"], 0)).id();
    let subtitles = world.spawn(toggle("Subtitles", true)).id();
    let tabs = world
        .spawn(tabs(
            ["Audio", "Display"],
            0,
            children![tab_page(()), tab_page(())],
        ))
        .id();
    let column = world
        .spawn(menu())
        .add_children(&[volume, quality, subtitles, tabs])
        .id();
    let screen = world.spawn(screen()).add_child(column).id();
    listens(&mut app, screen, "screen");
    let pad = plug_in_pad(&mut app);
    settle(&mut app);

    for control in [volume, quality, subtitles] {
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(control, FocusCause::Navigated);
        app.update();
        tap(&mut app, KeyCode::Escape);
        press(&mut app, pad, GamepadButton::East);
    }

    assert_eq!(heard(&app).len(), 6, "each one backs out from the control");
    assert_eq!(app.world().resource::<Moves>().0, 0, "and none changed");
    assert_eq!(
        app.world().get::<SliderValue>(volume),
        Some(&SliderValue(0.5))
    );
    assert_eq!(
        app.world().get::<ChoiceValue>(quality),
        Some(&ChoiceValue(0))
    );
    assert!(app.world().get::<Checked>(subtitles).is_some());
}

#[test]
fn a_frame_with_no_press_sends_nothing() {
    let mut app = app();
    let [screen, play, _] = menu_screen(&mut app);
    listens(&mut app, screen, "screen");
    settle(&mut app);
    for _ in 0..30 {
        app.update();
    }
    assert!(heard(&app).is_empty());
    assert_eq!(focus(&app), Some(play));
    assert!(!focus_visible(&app));
}
