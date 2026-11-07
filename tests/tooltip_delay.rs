//! A tooltip on a desktop's timing: the pointer rests half a second before it opens, a press, a wheel
//! turn or a key ends the visit until the pointer leaves and returns, and visible focus opens it at
//! once. On a 60 Hz clock the test sets.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

fn app(settings: TooltipSettings) -> App {
    let mut app = App::new();
    app.insert_resource(MotionSettings::off());
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )))
    .insert_resource(settings);
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

/// A screen with one button that has a tooltip, and the button.
fn with_tooltip(app: &mut App) -> Entity {
    let widget = app
        .world_mut()
        .spawn((button("Options"), Tooltip::new("Change the settings")))
        .id();
    let card = app.world_mut().spawn(panel()).add_child(widget).id();
    app.world_mut().spawn(screen()).add_child(card);
    frames(app, 4);
    widget
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a window")
}

fn location(app: &mut App, at: Vec2) -> Location {
    let window = window(app);
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

/// Moves the pointer to `at` and runs one frame, so hover is read.
fn point_at(app: &mut App, at: Vec2) {
    let location = location(app, at);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    app.update();
}

fn centre(app: &App, entity: Entity) -> Vec2 {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    app.world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation
        * node.inverse_scale_factor()
}

fn shown(app: &mut App) -> bool {
    let world = app.world_mut();
    let tip = world
        .query_filtered::<Entity, With<TooltipBox>>()
        .single(world)
        .expect("one tooltip box");
    world.get::<Node>(tip).expect("a node").display != Display::None
}

fn press_pointer(app: &mut App, at: Vec2) {
    let location = location(app, at);
    for action in [
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.update();
    }
}

#[test]
fn the_box_opens_after_half_a_second_of_rest_and_not_before() {
    let mut app = app(TooltipSettings::default());
    let widget = with_tooltip(&mut app);
    let over = centre(&app, widget);
    point_at(&mut app, over);
    assert!(!shown(&mut app), "the frame the pointer arrives");
    // 0.4 s in (the arrival frame counted): still shut.
    frames(&mut app, (0.4 / FRAME) as usize - 1);
    assert!(!shown(&mut app), "0.4 s");
    // 29 frames in (0.483 s, the arrival frame counted): not yet. 31 (0.517 s): open.
    frames(&mut app, 5);
    assert!(!shown(&mut app), "just under 0.5 s");
    frames(&mut app, 2);
    assert!(shown(&mut app), "0.5 s and a frame");
    point_at(&mut app, Vec2::new(5.0, 5.0));
    assert!(!shown(&mut app), "leaving shuts it");
}

#[test]
fn a_press_a_wheel_turn_and_a_key_end_the_visit_until_the_pointer_comes_back() {
    let mut app = app(TooltipSettings::default());
    let widget = with_tooltip(&mut app);
    let over = centre(&app, widget);
    for ending in 0..3 {
        point_at(&mut app, over);
        frames(&mut app, 40);
        assert!(shown(&mut app), "visit {ending} opens it");
        match ending {
            0 => press_pointer(&mut app, over),
            1 => {
                let window = window(&mut app);
                app.world_mut().write_message(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.0,
                    y: -1.0,
                    window,
                    phase: bevy::input::touch::TouchPhase::Moved,
                });
                app.update();
            }
            _ => {
                let window = window(&mut app);
                app.world_mut().write_message(KeyboardInput {
                    key_code: KeyCode::KeyA,
                    logical_key: Key::Unidentified(NativeKey::Unidentified),
                    state: ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                });
                app.update();
            }
        }
        assert!(!shown(&mut app), "ending {ending} hides it");
        frames(&mut app, 60);
        assert!(
            !shown(&mut app),
            "and it stays hidden while the pointer rests"
        );
        point_at(&mut app, Vec2::new(5.0, 5.0));
        point_at(&mut app, over);
        assert!(
            !shown(&mut app),
            "back on the widget it is not open at once"
        );
    }
    frames(&mut app, 40);
    assert!(
        shown(&mut app),
        "half a second after coming back it opens again"
    );
}

#[test]
fn visible_focus_opens_it_in_that_frame_and_a_zero_delay_opens_it_on_arrival() {
    let mut app = app(TooltipSettings::default());
    let widget = with_tooltip(&mut app);
    // The pointer rests on the widget, and a key (Tab) moves focus onto it in the same frame: the key
    // ends the pointer's visit but visible focus opens the box in that frame.
    let over = centre(&app, widget);
    point_at(&mut app, over);
    let window = window(&mut app);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Tab,
        logical_key: Key::Tab,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(widget, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
    assert!(shown(&mut app), "Tab onto the widget: shown in that frame");

    let mut quick = self::app(TooltipSettings { delay_seconds: 0.0 });
    let widget = with_tooltip(&mut quick);
    let over = centre(&quick, widget);
    point_at(&mut quick, over);
    frames(&mut quick, 1);
    assert!(shown(&mut quick), "no delay: open as the pointer arrives");
}
