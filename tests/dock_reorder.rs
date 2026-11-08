//! A dock tab carried within its strip (#70): it follows the pointer, the tabs it passes step aside
//! so a gap stands where it would land, and when it is let go it settles into the gap from where it
//! was drawn; Escape and the keyboard move go home and glide too. On a clock the test sets
//! (30 ms a frame).

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
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

const FRAME: f32 = 0.03;

#[derive(Resource, Default)]
struct Reports(Vec<DockLayout>);

fn app(motion: MotionSettings) -> App {
    let mut app = App::new();
    app.insert_resource(motion);
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
    .init_resource::<Reports>()
    .add_observer(|changed: On<DockLayoutChanged>, mut log: ResMut<Reports>| {
        log.0.push(changed.layout.clone());
    });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// A dock of one group (a, b, c) beside another (d).
fn rig(app: &mut App) -> Entity {
    for (id, title) in [
        ("a", "Alpha"),
        ("b", "Bravo"),
        ("c", "Charlie"),
        ("d", "Delta"),
    ] {
        app.world_mut()
            .resource_mut::<PanelRegistry>()
            .register(id, title, move |page| {
                page.with_child(label(title));
            });
    }
    let layout = DockLayout::split(
        SplitAxis::Horizontal,
        0.7,
        DockLayout::group(["a", "b", "c"]),
        DockLayout::group(["d"]),
    );
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(1000.0),
                height: Val::Px(500.0),
                ..default()
            },
            children![dock(layout)],
        ))
        .id();
    frames(app, 30);
    app.world().get::<Children>(container).expect("dock")[0]
}

fn tab(app: &mut App, title: &str) -> Entity {
    let mut query = app.world_mut().query::<(&Text, &Role, &ChildOf)>();
    query
        .iter(app.world())
        .find(|(text, role, _)| **role == Role::TabText && text.0 == title)
        .map(|(_, _, parent)| parent.parent())
        .unwrap_or_else(|| panic!("no tab titled {title}"))
}

/// Where a node is drawn now, in layout px: travel and transforms included.
fn drawn(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(at * scale, node.size() * scale)
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window(app);
    let target = WindowRef::Entity(window)
        .normalize(None)
        .expect("a window ref");
    let position = at * app.world().resource::<UiScale>().0;
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(target),
            position,
        },
        action,
    ));
    app.update();
}

fn tap(app: &mut App, key_code: KeyCode) {
    let window = window(app);
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

fn reports(app: &App) -> usize {
    app.world().resource::<Reports>().0.len()
}

/// Picks `tab` up where its centre is and carries it `dx` along its strip, leaving the pointer
/// pressed.
fn carry(app: &mut App, from: Vec2, dx: f32) {
    pointer(app, from, PointerAction::Move { delta: Vec2::ZERO });
    pointer(app, from, PointerAction::Press(PointerButton::Primary));
    let mut at = from;
    // In steps, as a hand moves: the carried tab follows each.
    for _ in 0..6 {
        at.x += dx / 6.0;
        pointer(
            app,
            at,
            PointerAction::Move {
                delta: Vec2::new(dx / 6.0, 0.0),
            },
        );
    }
    frames(app, 12);
}

fn offset(app: &App, tab: Entity, layout_centre: Vec2) -> Vec2 {
    drawn(app, tab).center() - layout_centre
}

#[test]
fn the_tab_follows_the_pointer_and_the_one_it_passes_steps_aside() {
    let mut app = app(MotionSettings::default());
    let _dock = rig(&mut app);
    let (alpha, bravo, charlie) = (
        tab(&mut app, "Alpha"),
        tab(&mut app, "Bravo"),
        tab(&mut app, "Charlie"),
    );
    let (a0, b0, c0) = (drawn(&app, alpha), drawn(&app, bravo), drawn(&app, charlie));
    // Past Bravo's centre, short of Charlie's.
    let to = b0.center().x + 10.0 - a0.center().x;
    carry(&mut app, a0.center(), to);
    let step = a0.width() + (b0.min.x - a0.max.x);
    // Bravo travelled left by the carried tab's width and the gap, and Charlie stayed.
    let bravo_off = offset(&app, bravo, b0.center());
    assert!(
        (bravo_off.x + step).abs() < 1.5,
        "Bravo moved {} of {}",
        bravo_off.x,
        -step
    );
    assert!(
        offset(&app, charlie, c0.center()).length() < 0.6,
        "Charlie stays"
    );
    // Alpha is under the pointer, wherever it took hold.
    let alpha_off = offset(&app, alpha, a0.center());
    assert!(
        (alpha_off.x - to).abs() < 1.0,
        "Alpha is {} along, the pointer {}",
        alpha_off.x,
        to
    );
    // Layout is untouched until it is let go.
    assert_eq!(reports(&app), 0);
    // Let go: one report, and no jump.
    let before = drawn(&app, alpha).center();
    pointer(
        &mut app,
        a0.center() + Vec2::new(to, 0.0),
        PointerAction::Release(PointerButton::Primary),
    );
    assert_eq!(reports(&app), 1, "one DockLayoutChanged");
    let alpha = tab(&mut app, "Alpha");
    let first = drawn(&app, alpha).center();
    // It settles into the gap: Bravo's old place plus its width.
    frames(&mut app, 40);
    let settled = drawn(&app, alpha);
    // No jump: the frame it was let go it had moved by one step of its spring, not to its slot.
    assert!(
        (first - before).length() < 0.3 * (settled.center() - before).length().max(1.0),
        "the tab jumped on release: {before:?}, then {first:?}, ending at {:?}",
        settled.center()
    );
    let bravo = tab(&mut app, "Bravo");
    assert!(
        settled.min.x > drawn(&app, bravo).max.x,
        "Alpha is after Bravo now"
    );
    assert!(
        (settled.min.x - drawn(&app, bravo).max.x).abs() < step,
        "in the gap"
    );
    // And nothing is left moving.
    assert_eq!(
        app.world().get::<LayoutTravel>(alpha).map(|t| t.offset()),
        Some(Vec2::ZERO)
    );
}

#[test]
fn the_tabs_go_back_when_the_pointer_leaves_the_strip() {
    let mut app = app(MotionSettings::default());
    let _dock = rig(&mut app);
    let (alpha, bravo) = (tab(&mut app, "Alpha"), tab(&mut app, "Bravo"));
    let (a0, b0) = (drawn(&app, alpha), drawn(&app, bravo));
    carry(&mut app, a0.center(), b0.center().x + 10.0 - a0.center().x);
    assert!(
        offset(&app, bravo, b0.center()).x < -10.0,
        "Bravo stepped aside"
    );
    // Down into the page: no gap stands any more.
    let at = Vec2::new(b0.center().x + 10.0, a0.center().y + 200.0);
    pointer(
        &mut app,
        at,
        PointerAction::Move {
            delta: Vec2::new(0.0, 200.0),
        },
    );
    frames(&mut app, 20);
    assert!(
        offset(&app, bravo, b0.center()).length() < 0.6,
        "Bravo is home"
    );
}

#[test]
fn escape_sends_everything_home_and_reports_nothing() {
    let mut app = app(MotionSettings::default());
    let _dock = rig(&mut app);
    let (alpha, bravo) = (tab(&mut app, "Alpha"), tab(&mut app, "Bravo"));
    let (a0, b0) = (drawn(&app, alpha), drawn(&app, bravo));
    carry(&mut app, a0.center(), b0.center().x + 10.0 - a0.center().x);
    // Escape is Back: it goes to the focused tab.
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(alpha, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    frames(&mut app, 2);
    tap(&mut app, KeyCode::Escape);
    // The tab springs home from where it was drawn: not at once, and then all the way.
    assert!(
        offset(&app, alpha, a0.center()).x.abs() > 1.0,
        "it does not jump"
    );
    frames(&mut app, 40);
    assert!(offset(&app, alpha, a0.center()).length() < 0.6);
    assert!(offset(&app, bravo, b0.center()).length() < 0.6);
    assert_eq!(reports(&app), 0);
}

#[test]
fn the_keyboard_move_glides_like_a_drop() {
    let mut app = app(MotionSettings::default());
    let _dock = rig(&mut app);
    let bravo = tab(&mut app, "Bravo");
    let before = drawn(&app, bravo).center();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(bravo, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    frames(&mut app, 2);
    // Ctrl+Shift+Right moves it to the next group.
    for key in [KeyCode::ControlLeft, KeyCode::ShiftLeft] {
        let window = window(&mut app);
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(reports(&app), 1);
    frames(&mut app, 2);
    let bravo = tab(&mut app, "Bravo");
    let early = drawn(&app, bravo).center();
    frames(&mut app, 40);
    let settled = drawn(&app, bravo).center();
    assert!(
        (early - before).length() < (settled - before).length() - 20.0,
        "it starts near where it was ({early:?}) and ends in the other group ({settled:?}, from {before:?})"
    );
}

#[test]
fn with_no_motion_a_drop_lands_in_its_slot_at_once() {
    let mut app = app(MotionSettings::off());
    let _dock = rig(&mut app);
    let (alpha, bravo) = (tab(&mut app, "Alpha"), tab(&mut app, "Bravo"));
    let (a0, b0) = (drawn(&app, alpha), drawn(&app, bravo));
    let to = b0.center().x + 10.0 - a0.center().x;
    carry(&mut app, a0.center(), to);
    pointer(
        &mut app,
        a0.center() + Vec2::new(to, 0.0),
        PointerAction::Release(PointerButton::Primary),
    );
    frames(&mut app, 2);
    let alpha = tab(&mut app, "Alpha");
    assert_eq!(
        app.world().get::<LayoutTravel>(alpha).map(|t| t.offset()),
        Some(Vec2::ZERO),
        "nothing glides under no motion"
    );
}
