//! A menu opens from its title and leaves with a fade (#71): the frame its panel is first drawn it
//! is at 0.96 scale and nearly transparent; Escape takes the keyboard and the pointer from it at
//! once, fades it out where it stands and then puts it out of sight, ready for the next opening; a
//! menu opened again while it fades comes back. On a clock the test sets (30 ms a frame).

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const FRAME: f32 = 0.03;

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
    ));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().spawn(Camera2d);
    app.world_mut().spawn(menu_bar(children![menu_title(
        "File",
        children![menu_item("New"), menu_item("Open"), menu_item("Quit")]
    )]));
    // The first update only starts the clock.
    for _ in 0..6 {
        app.update();
    }
    app
}

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

fn panel(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<MenuPanel>>()
        .single(world)
        .expect("one menu panel")
}

fn items(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<MenuItem>>()
        .iter(world)
        .collect()
}

fn drawn(app: &App, panel: Entity) -> bool {
    *app.world().get::<Visibility>(panel).expect("visibility") != Visibility::Hidden
        && app.world().get::<Node>(panel).expect("node").display != Display::None
}

fn scale(app: &App, panel: Entity) -> f32 {
    app.world().get::<UiTransform>(panel).expect("node").scale.x
}

fn opacity(app: &App, panel: Entity) -> f32 {
    app.world()
        .get::<Opacity>(panel)
        .map_or(1.0, Opacity::factor)
}

fn open(app: &mut App) {
    tap(app, KeyCode::F10);
    tap(app, KeyCode::ArrowDown);
}

#[test]
fn a_menu_opens_small_and_faint_and_whole_by_the_snappy_spring() {
    let mut app = app(MotionSettings::default());
    let panel = panel(&mut app);
    open(&mut app);
    let mut first = None;
    for _ in 0..8 {
        if drawn(&app, panel) && first.is_none() {
            first = Some((scale(&app, panel), opacity(&app, panel)));
        }
        app.update();
    }
    let (scale_then, opacity_then) = first.expect("the menu is drawn");
    assert!(
        (scale_then - 0.96).abs() <= 0.01,
        "first drawn at {scale_then}"
    );
    assert!(opacity_then <= 0.5, "first drawn at opacity {opacity_then}");
    let snappy = app.world().resource::<GooeySkin>().motion.snappy.duration;
    for _ in 0..(snappy / FRAME).ceil() as usize + 1 {
        app.update();
    }
    assert_eq!(scale(&app, panel), 1.0);
    assert!(app.world().get::<Opacity>(panel).is_none());
}

#[test]
fn escape_fades_the_menu_out_where_it_stands_and_puts_it_out_of_sight() {
    let mut app = app(MotionSettings::default());
    let panel = panel(&mut app);
    let rows = items(&mut app);
    open(&mut app);
    for _ in 0..20 {
        app.update();
    }
    assert!(drawn(&app, panel));
    tap(&mut app, KeyCode::Escape);
    // Out of the keyboard's and the pointer's reach at once, still drawn, fading.
    assert!(app.world().get::<PopoverPanel>(panel).is_none());
    assert!(drawn(&app, panel), "it fades where it stands");
    assert!(opacity(&app, panel) < 1.0, "and it is fading");
    for &row in &rows {
        assert_eq!(
            app.world().get::<Pickable>(row),
            Some(&Pickable::IGNORE),
            "no pointer for a leaving menu"
        );
        assert!(
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(row)
                .is_none()
        );
    }
    let fade = app.world().resource::<GooeySkin>().motion.fade_seconds;
    for _ in 0..(fade / FRAME).ceil() as usize + 4 {
        app.update();
    }
    assert!(!drawn(&app, panel), "gone from sight");
    assert!(app.world().get::<Leaving>(panel).is_none());
    assert!(app.world().get::<Opacity>(panel).is_none());
    assert!(
        app.world().get_entity(panel).is_ok(),
        "the panel stays for the next opening"
    );
    for &row in &rows {
        assert_ne!(app.world().get::<Pickable>(row), Some(&Pickable::IGNORE));
        assert!(
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(row)
                .is_some(),
            "the rows are focusable again"
        );
    }
    // And it opens again, whole (focus is back on the title: Down opens it).
    tap(&mut app, KeyCode::ArrowDown);
    for _ in 0..30 {
        app.update();
    }
    assert!(drawn(&app, panel));
    assert_eq!(opacity(&app, panel), 1.0);
    assert_eq!(scale(&app, panel), 1.0);
}

#[test]
fn a_menu_opened_again_while_it_fades_comes_back() {
    let mut app = app(MotionSettings::default());
    let panel = panel(&mut app);
    open(&mut app);
    for _ in 0..20 {
        app.update();
    }
    tap(&mut app, KeyCode::Escape);
    assert!(app.world().get::<Leaving>(panel).is_some());
    // Down again, inside the fade: focus is back on the title.
    tap(&mut app, KeyCode::ArrowDown);
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world().get::<PopoverPanel>(panel).is_some(),
        "it is open"
    );
    assert!(drawn(&app, panel));
    assert_eq!(opacity(&app, panel), 1.0, "it came back whole");
    assert!(app.world().get::<Leaving>(panel).is_none());
}

#[test]
fn with_no_motion_a_menu_comes_and_goes_in_a_frame() {
    let mut app = app(MotionSettings::off());
    let panel = panel(&mut app);
    open(&mut app);
    app.update();
    app.update();
    assert!(drawn(&app, panel));
    assert_eq!(scale(&app, panel), 1.0);
    assert_eq!(opacity(&app, panel), 1.0);
    tap(&mut app, KeyCode::Escape);
    assert!(!drawn(&app, panel), "hidden in the frame it closes");
    assert!(app.world().get::<Leaving>(panel).is_none());
}
