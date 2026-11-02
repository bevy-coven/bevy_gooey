//! Focus regions: a viewport that keeps the arrow keys for its host, and F6 cycling the parts of a
//! screen. Laid out for real in a headless window, driven by real keyboard and gamepad input.

use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// How many times the host's own system saw each arrow key go down.
#[derive(Resource, Default)]
struct HostSaw {
    up: u32,
    down: u32,
    left: u32,
    right: u32,
}

/// The screen's parts and their widgets.
struct Scene {
    a1: Entity,
    a2: Entity,
    viewport: Entity,
    b1: Entity,
}

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
    .init_resource::<HostSaw>()
    .add_systems(Update, |keys: Res<ButtonInput<KeyCode>>, mut saw: ResMut<HostSaw>| {
        saw.up += u32::from(keys.just_pressed(KeyCode::ArrowUp));
        saw.down += u32::from(keys.just_pressed(KeyCode::ArrowDown));
        saw.left += u32::from(keys.just_pressed(KeyCode::ArrowLeft));
        saw.right += u32::from(keys.just_pressed(KeyCode::ArrowRight));
    });
    app.world_mut().spawn(Camera2d);
    app
}

/// A screen of three side-by-side parts: a menu region with two buttons (A1 over A2), a viewport
/// that keeps the arrow keys and is a tab stop itself, and a menu region with one button (B1).
fn scene(app: &mut App) -> Scene {
    let world = app.world_mut();
    let a1 = world.spawn(button("A1")).id();
    let a2 = world.spawn(button("A2")).id();
    let left = world
        .spawn((menu(), FocusRegion::navigate()))
        .add_children(&[a1, a2])
        .id();
    let viewport = world
        .spawn((
            Node {
                width: Val::Px(400.0),
                height: Val::Px(300.0),
                ..default()
            },
            FocusRegion::host(),
            TabIndex(0),
            AutoDirectionalNavigation::default(),
        ))
        .id();
    let b1 = world.spawn(button("B1")).id();
    let right = world
        .spawn((menu(), FocusRegion::navigate()))
        .add_children(&[b1])
        .id();
    let root = world
        .spawn(screen())
        .add_children(&[left, viewport, right])
        .id();
    world.entity_mut(root).insert(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    });
    settle(app);
    Scene {
        a1,
        a2,
        viewport,
        b1,
    }
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn primary_window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = primary_window(app);
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

fn tap(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
    key(app, key_code, ButtonState::Released);
}

/// `key_code` with `held` down in an earlier frame, as a player's fingers are.
fn chord(app: &mut App, held: KeyCode, key_code: KeyCode) {
    key(app, held, ButtonState::Pressed);
    tap(app, key_code);
    key(app, held, ButtonState::Released);
}

fn focus(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn set_focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    settle(app);
}

fn saw(app: &App) -> [u32; 4] {
    let s = app.world().resource::<HostSaw>();
    [s.up, s.down, s.left, s.right]
}

#[test]
fn a_host_region_keeps_the_arrow_keys_and_the_host_still_reads_them() {
    let mut app = app();
    let s = scene(&mut app);
    set_focus(&mut app, s.viewport);

    for arrow in [
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
    ] {
        tap(&mut app, arrow);
        assert_eq!(focus(&app), Some(s.viewport), "{arrow:?} moved focus");
    }
    assert_eq!(saw(&app), [1, 1, 1, 1], "the host reads every arrow key");
    assert!(
        !app.world().resource::<InputFocusVisible>().0,
        "an arrow in a host region shows no ring"
    );
}

#[test]
fn a_navigate_region_still_moves_focus_with_the_arrows() {
    let mut app = app();
    let s = scene(&mut app);
    set_focus(&mut app, s.a1);

    tap(&mut app, KeyCode::ArrowDown);

    assert_eq!(focus(&app), Some(s.a2));
}

#[test]
fn the_arrows_are_the_hosts_for_a_widget_inside_the_region_too() {
    let mut app = app();
    let s = scene(&mut app);
    let inner = app.world_mut().spawn(button("Inner")).id();
    app.world_mut().entity_mut(s.viewport).add_child(inner);
    settle(&mut app);
    set_focus(&mut app, inner);

    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::ArrowDown);

    assert_eq!(focus(&app), Some(inner));
}

#[test]
fn the_dpad_still_leaves_a_host_region() {
    let mut app = app();
    let s = scene(&mut app);
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
    set_focus(&mut app, s.viewport);

    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad,
                GamepadButton::DPadRight,
                value,
            )));
        app.update();
    }

    assert_eq!(focus(&app), Some(s.b1));
}

#[test]
fn tab_leaves_a_host_region() {
    let mut app = app();
    let s = scene(&mut app);
    set_focus(&mut app, s.viewport);

    tap(&mut app, KeyCode::Tab);

    assert_ne!(focus(&app), Some(s.viewport), "Tab never traps");
    assert!(focus(&app).is_some());
}

#[test]
fn f6_goes_to_the_next_regions_first_widget_and_wraps() {
    let mut app = app();
    let s = scene(&mut app);
    set_focus(&mut app, s.a2);

    tap(&mut app, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.viewport), "the viewport is its own stop");
    assert!(
        app.world().resource::<InputFocusVisible>().0,
        "F6 shows focus"
    );

    tap(&mut app, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.b1));

    tap(&mut app, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.a1), "the first widget of the first region");
}

#[test]
fn shift_f6_goes_back_and_wraps() {
    let mut app = app();
    let s = scene(&mut app);
    set_focus(&mut app, s.b1);

    chord(&mut app, KeyCode::ShiftLeft, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.viewport));

    chord(&mut app, KeyCode::ShiftLeft, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.a1));

    chord(&mut app, KeyCode::ShiftLeft, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.b1), "back past the first wraps to the last");
}

#[test]
fn with_focus_in_no_region_f6_starts_at_either_end() {
    let mut app = app();
    let s = scene(&mut app);
    app.world_mut().resource_mut::<InputFocus>().clear();
    settle(&mut app);

    tap(&mut app, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.a1));

    app.world_mut().resource_mut::<InputFocus>().clear();
    settle(&mut app);
    chord(&mut app, KeyCode::ShiftRight, KeyCode::F6);
    assert_eq!(focus(&app), Some(s.b1));
}

#[test]
fn f6_with_another_modifier_is_not_ours() {
    let mut app = app();
    let s = scene(&mut app);
    set_focus(&mut app, s.a1);

    chord(&mut app, KeyCode::ControlLeft, KeyCode::F6);
    chord(&mut app, KeyCode::AltLeft, KeyCode::F6);

    assert_eq!(focus(&app), Some(s.a1));
}

#[test]
fn a_region_with_nothing_to_focus_is_skipped() {
    let mut app = app();
    let s = scene(&mut app);
    let empty = app.world_mut().spawn((Node::default(), FocusRegion::navigate())).id();
    // Between the viewport and the right-hand menu in tree order.
    let parent = app
        .world()
        .get::<ChildOf>(s.viewport)
        .expect("the viewport has a parent")
        .parent();
    app.world_mut().entity_mut(parent).add_child(empty);
    settle(&mut app);
    set_focus(&mut app, s.b1);

    tap(&mut app, KeyCode::F6);

    assert_eq!(focus(&app), Some(s.a1), "wrapped past the empty region");
}

#[test]
fn an_open_dialog_holds_f6_to_its_own_regions() {
    let mut app = app();
    let s = scene(&mut app);
    let world = app.world_mut();
    let yes = world.spawn(button("Yes")).id();
    let no = world.spawn(button("No")).id();
    let dialog = world
        .spawn((
            dialog(),
            children![(
                panel(),
                children![(menu(), FocusRegion::navigate(), children![])]
            )],
        ))
        .id();
    let menu_entity = world
        .get::<Children>(dialog)
        .and_then(|kids| kids.first().copied())
        .and_then(|panel| world.get::<Children>(panel).and_then(|k| k.first().copied()))
        .expect("the dialog's menu");
    world.entity_mut(menu_entity).add_children(&[yes, no]);
    settle(&mut app);
    set_focus(&mut app, yes);

    tap(&mut app, KeyCode::F6);
    assert_eq!(focus(&app), Some(yes), "one region in the dialog: F6 stays in it");

    // The screen's regions are out of reach: a dialog holds focus over every region.
    assert_ne!(focus(&app), Some(s.viewport));
    assert_ne!(focus(&app), Some(s.a1));
}

#[test]
fn nothing_is_written_while_idle() {
    let mut app = app();
    let s = scene(&mut app);
    set_focus(&mut app, s.viewport);
    settle(&mut app);
    app.world_mut().clear_trackers();

    for _ in 0..30 {
        app.update();
    }

    assert!(!app.world().resource_ref::<InputFocus>().is_changed());
    assert!(!app.world().resource_ref::<InputFocusVisible>().is_changed());
}
