//! Double press and click counts (G-55): two clicks close in time and in place are one
//! `DoublePress`, the first click still lands at once, a third starts a new pair, and a press held,
//! dragged or far off pairs with nothing. Laid out for real in a headless window, driven by real
//! pointer and keyboard input on a clock the test sets.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::events::{PointerClick, PointerDragStart};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test's clock.
const FRAME_MS: u64 = 10;

#[derive(Resource, Default)]
struct Log {
    /// Every `DoublePress` the button heard, with the frame it came in.
    doubles: Vec<u32>,
    /// Every primary click on the button, with the frame it came in.
    clicks: Vec<u32>,
    /// The `ClickCount` a drag observer read when its drag started.
    drag_counts: Vec<Option<u8>>,
    activations: u32,
}

#[derive(Resource, Default)]
struct Frame(u32);

fn app() -> (App, Entity) {
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        FRAME_MS,
    )))
    .init_resource::<Log>()
    .init_resource::<Frame>()
    .add_systems(First, |mut frame: ResMut<Frame>| frame.0 += 1);
    app.world_mut().spawn(Camera2d);
    let mut button_entity = Entity::PLACEHOLDER;
    app.world_mut().spawn(screen()).with_children(|screen| {
        screen.spawn(panel()).with_children(|panel| {
            button_entity = panel.spawn(button("Open")).id();
        });
    });
    app.world_mut()
        .entity_mut(button_entity)
        .observe(
            |_: On<DoublePress>, frame: Res<Frame>, mut log: ResMut<Log>| {
                log.doubles.push(frame.0);
            },
        )
        .observe(
            |click: On<PointerClick>, frame: Res<Frame>, mut log: ResMut<Log>| {
                if click.button == PointerButton::Primary {
                    log.clicks.push(frame.0);
                }
            },
        )
        .observe(
            |start: On<PointerDragStart>, counts: Query<&ClickCount>, mut log: ResMut<Log>| {
                // The count is on the entity pressed, which may be a child of the observer's.
                log.drag_counts.push(
                    counts
                        .get(start.original_event_target())
                        .ok()
                        .map(|count| count.0),
                );
            },
        )
        .observe(|_: On<Activate>, mut log: ResMut<Log>| log.activations += 1);
    for _ in 0..6 {
        app.update();
    }
    (app, button_entity)
}

fn centre(app: &App, entity: Entity) -> Vec2 {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    at * node.inverse_scale_factor()
}

fn location(app: &mut App, at: Vec2) -> Location {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let at = location(app, at);
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, at, action));
    app.update();
}

fn press(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
}

fn release(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
}

fn click(app: &mut App, at: Vec2) {
    press(app, at);
    release(app, at);
}

/// Runs frames until `seconds` have passed since `from` on the app's real clock.
fn wait_until(app: &mut App, from: f64, seconds: f64) {
    while now(app) - from < seconds - 1e-6 {
        app.update();
    }
}

/// Runs frames until `seconds` have passed from now.
fn pause(app: &mut App, seconds: f64) {
    let from = now(app);
    wait_until(app, from, seconds);
}

fn now(app: &App) -> f64 {
    app.world().resource::<Time<Real>>().elapsed_secs_f64()
}

fn log(app: &App) -> &Log {
    app.world().resource::<Log>()
}

/// The time of the last press: the frame `press` ran its `Press` in.
fn press_and_time(app: &mut App, at: Vec2) -> f64 {
    press(app, at);
    now(app)
}

#[test]
fn two_clicks_close_in_time_and_place_are_one_double_press_and_the_first_lands_at_once() {
    let (mut app, button) = app();
    let at = centre(&app, button);
    let first = press_and_time(&mut app, at);
    release(&mut app, at);
    assert_eq!(log(&app).clicks.len(), 1, "the first click lands at once");
    assert!(log(&app).doubles.is_empty());
    wait_until(&mut app, first, 0.3);
    click(&mut app, at + Vec2::new(3.0, -3.0));
    for _ in 0..3 {
        app.update();
    }
    let log = log(&app);
    assert_eq!(log.clicks.len(), 2, "both clicks are clicks");
    assert_eq!(log.doubles.len(), 1, "one double press");
    assert_eq!(
        log.doubles[0], log.clicks[1],
        "the double press comes in the frame of the second click"
    );
}

#[test]
fn clicks_too_far_apart_in_time_or_place_pair_with_nothing() {
    let (mut app, button) = app();
    let at = centre(&app, button);
    let first = press_and_time(&mut app, at);
    release(&mut app, at);
    wait_until(&mut app, first, 0.6);
    click(&mut app, at);
    assert!(log(&app).doubles.is_empty(), "0.6 s apart is two clicks");
    // The second click above is a first of its own; wait it out.
    pause(&mut app, 0.6);
    click(&mut app, at);
    click(&mut app, at + Vec2::new(5.0, 0.0));
    assert!(log(&app).doubles.is_empty(), "5 px apart is two clicks");
    assert_eq!(log(&app).clicks.len(), 4);
}

#[test]
fn a_third_quick_click_starts_a_new_pair() {
    let (mut app, button) = app();
    let at = centre(&app, button);
    for _ in 0..3 {
        click(&mut app, at);
    }
    assert_eq!(log(&app).doubles.len(), 1, "three quick clicks: one pair");
    click(&mut app, at);
    assert_eq!(
        log(&app).doubles.len(),
        2,
        "the fourth closes the second pair"
    );
}

#[test]
fn a_press_held_past_the_interval_or_dragged_is_neither_half_of_a_pair() {
    let (mut app, button) = app();
    let at = centre(&app, button);
    let held = press_and_time(&mut app, at);
    wait_until(&mut app, held, 0.55);
    release(&mut app, at);
    click(&mut app, at);
    assert!(
        log(&app).doubles.is_empty(),
        "a press held past 0.5 s, then a click: no pair"
    );
    // The click above is the first of a new pair, but a second press that travels does not close it.
    press(&mut app, at);
    pointer(
        &mut app,
        at + Vec2::new(6.0, 0.0),
        PointerAction::Move {
            delta: Vec2::new(6.0, 0.0),
        },
    );
    pointer(&mut app, at, PointerAction::Move { delta: Vec2::ZERO });
    release(&mut app, at);
    assert!(
        log(&app).doubles.is_empty(),
        "a press that travelled 6 px pairs with nothing"
    );
}

#[test]
fn keys_never_pair() {
    let (mut app, button) = app();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(button, FocusCause::Navigated);
    testing::press(&mut app, KeyCode::Enter);
    testing::press(&mut app, KeyCode::Enter);
    assert_eq!(
        log(&app).activations,
        2,
        "Enter presses the button each time"
    );
    assert!(
        log(&app).doubles.is_empty(),
        "Enter twice is no double press"
    );
}

#[test]
fn a_press_run_counts_its_clicks_for_a_drag_and_a_pause_starts_again() {
    let (mut app, button) = app();
    let at = centre(&app, button);
    // The one entity holding a count is the one pressed: the button or the label in it.
    let count = |app: &mut App| {
        let counts: Vec<u8> = app
            .world_mut()
            .query::<&ClickCount>()
            .iter(app.world())
            .map(|count| count.0)
            .collect();
        assert!(counts.len() <= 1, "one press, one count: {counts:?}");
        counts.first().copied()
    };
    let mut counts = Vec::new();
    for _ in 0..3 {
        press(&mut app, at);
        counts.push(count(&mut app));
        release(&mut app, at);
    }
    assert_eq!(counts, vec![Some(1), Some(2), Some(3)]);
    assert_eq!(
        count(&mut app),
        None,
        "the count is gone once the press is let go"
    );
    pause(&mut app, 0.6);
    press(&mut app, at);
    assert_eq!(
        count(&mut app),
        Some(1),
        "after a pause the run starts again"
    );
    // A drag started in that press reads the count.
    pointer(
        &mut app,
        at + Vec2::new(8.0, 0.0),
        PointerAction::Move {
            delta: Vec2::new(8.0, 0.0),
        },
    );
    release(&mut app, at + Vec2::new(8.0, 0.0));
    assert_eq!(log(&app).drag_counts, vec![Some(1)]);
}

#[test]
fn the_interval_and_the_slop_are_settings() {
    let (mut app, button) = app();
    assert_eq!(
        *app.world().resource::<PointerSettings>(),
        PointerSettings {
            double_click_seconds: 0.5,
            double_click_slop_px: 4.0,
        }
    );
    app.world_mut().insert_resource(PointerSettings {
        double_click_seconds: 1.0,
        double_click_slop_px: 8.0,
    });
    let at = centre(&app, button);
    let first = press_and_time(&mut app, at);
    release(&mut app, at);
    wait_until(&mut app, first, 0.8);
    click(&mut app, at + Vec2::new(7.0, 0.0));
    assert_eq!(log(&app).doubles.len(), 1);
}

#[test]
fn a_table_row_opens_on_a_double_press_and_not_on_two_clicks_apart() {
    let (mut app, _) = app();
    app.add_observer(|opened: On<RowActivated>, mut log: ResMut<Opened>| {
        log.0.push(opened.key);
    });
    app.init_resource::<Opened>();
    app.world_mut().spawn((
        panel(),
        children![table(
            [Column::new("Name").width(12.0)],
            (0..20)
                .map(|i| vec![format!("Item {i}")])
                .collect::<Vec<_>>(),
        )],
    ));
    for _ in 0..6 {
        app.update();
    }
    let row = app
        .world_mut()
        .query::<(Entity, &TableRow)>()
        .iter(app.world())
        .find(|(_, row)| row.index == 2)
        .map(|(entity, _)| entity)
        .expect("a pool row for the third item");
    let at = centre(&app, row) * app.world().resource::<UiScale>().0;
    let opened = |app: &App| app.world().resource::<Opened>().0.clone();
    // Too slow on the app's clock: Bevy's own count, on the wall clock, called this a double click.
    let first = press_and_time(&mut app, at);
    release(&mut app, at);
    wait_until(&mut app, first, 0.6);
    click(&mut app, at);
    assert!(opened(&app).is_empty(), "0.6 s apart opens nothing");
    pause(&mut app, 0.6);
    // Too far apart: Bevy's count does not look at the place at all.
    click(&mut app, at);
    click(&mut app, at + Vec2::new(5.0, 0.0));
    assert!(opened(&app).is_empty(), "5 px apart opens nothing");
    pause(&mut app, 0.6);
    click(&mut app, at);
    click(&mut app, at);
    assert_eq!(opened(&app), vec![2], "a double press opens the row");
}

#[derive(Resource, Default)]
struct Opened(Vec<u64>);

/// A tree of three nodes, the first open, in its own panel; returns the shown rows by key.
fn spawn_tree(app: &mut App) -> Vec<(u64, Entity)> {
    app.world_mut().spawn((
        panel(),
        children![tree(vec![TreeNode::new(1, "World").expanded().child(
            TreeNode::new(2, "Camera").child(TreeNode::new(3, "Lens"))
        ),])],
    ));
    for _ in 0..6 {
        app.update();
    }
    app.world_mut()
        .query::<(Entity, &TableRow)>()
        .iter(app.world())
        .map(|(entity, row)| (row.key, entity))
        .collect()
}

/// A tree row's chevron box and its label, the first and last parts of its first cell.
fn tree_parts(app: &App, row: Entity) -> (Entity, Entity) {
    let cells = app.world().get::<Children>(row).expect("cells");
    let parts = app.world().get::<Children>(cells[0]).expect("cell parts");
    (parts[0], *parts.last().expect("a label"))
}

fn ui_centre(app: &App, entity: Entity) -> Vec2 {
    centre(app, entity) * app.world().resource::<UiScale>().0
}

#[test]
fn a_tree_row_opens_on_a_double_press_on_its_label_and_not_on_its_chevron() {
    let (mut app, _) = app();
    app.init_resource::<Opened>();
    app.add_observer(|opened: On<RowActivated>, mut log: ResMut<Opened>| {
        log.0.push(opened.key);
    });
    let rows = spawn_tree(&mut app);
    let camera = rows.iter().find(|(key, _)| *key == 2).expect("Camera").1;
    let (chevron, label) = tree_parts(&app, camera);
    let at = ui_centre(&app, chevron);
    click(&mut app, at);
    click(&mut app, at);
    assert!(
        app.world().resource::<Opened>().0.is_empty(),
        "two quick clicks on a chevron open and close the node, and open no row"
    );
    pause(&mut app, 0.6);
    let at = ui_centre(&app, label);
    click(&mut app, at);
    click(&mut app, at);
    assert_eq!(app.world().resource::<Opened>().0, vec![2]);
}

#[test]
fn a_drag_after_a_quick_press_reads_a_count_of_two_and_letting_go_off_the_entity_clears_it() {
    let (mut app, button) = app();
    let at = centre(&app, button);
    click(&mut app, at);
    press(&mut app, at);
    let away = Vec2::new(1200.0, 700.0);
    pointer(
        &mut app,
        at + Vec2::new(8.0, 0.0),
        PointerAction::Move {
            delta: Vec2::new(8.0, 0.0),
        },
    );
    assert_eq!(log(&app).drag_counts, vec![Some(2)]);
    // Off the button and let go over empty ground: Bevy sends the button no release.
    pointer(&mut app, away, PointerAction::Move { delta: away - at });
    release(&mut app, away);
    let left = app
        .world_mut()
        .query::<&ClickCount>()
        .iter(app.world())
        .count();
    assert_eq!(left, 0, "the count goes when the drag ends");
}
