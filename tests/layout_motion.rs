//! Layout motion (`MovesSmoothly`): a widget that layout moves travels to its new place on the
//! skin's default spring instead of jumping, with its children riding it, while a resize, a size
//! change that leaves the corner where it was, reduced motion and a node with no mark land at once.
//! Laid out for real in a headless window on a 60 Hz clock the test sets.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResized, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// How many frames the test waits for a move to settle: a second.
const PATIENCE_FRAMES: usize = 60;

/// The most of a move one frame may draw: a third of it, so a move is seen as travel.
const MOST_OF_A_MOVE_IN_ONE_FRAME: f32 = 1.0 / 3.0;

/// The height of each block in the column, logical px.
const BLOCK_PX: f32 = 60.0;

/// The gap between the column's blocks, logical px.
const GAP_PX: f32 = 10.0;

fn app(settings: MotionSettings) -> App {
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )))
    .insert_resource(settings);
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// A block of the column, with a node inside that rides it.
fn block() -> impl Bundle {
    (
        Node {
            width: Val::Px(200.0),
            height: Val::Px(BLOCK_PX),
            ..default()
        },
        children![Node {
            width: Val::Px(20.0),
            height: Val::Px(20.0),
            ..default()
        }],
    )
}

/// Three blocks in a column at the window's top left; returns them top first.
fn column(app: &mut App, marked: bool) -> [Entity; 3] {
    let world = app.world_mut();
    let blocks = [(); 3].map(|_| world.spawn(block()).id());
    for &b in &blocks {
        if marked {
            world.entity_mut(b).insert(MovesSmoothly);
        }
    }
    world
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(GAP_PX),
            ..default()
        })
        .add_children(&blocks);
    frames(app, 3);
    blocks
}

/// Where `entity` is drawn: its top edge, logical px (the window's scale factor is 1).
fn top(app: &App, entity: Entity) -> f32 {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size();
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed");
    at.translation.y - size.y / 2.0
}

/// `entity`'s drawn top, frame by frame, from now until it has stood still for a while; `each`
/// is asked after every frame.
fn follow_checking(app: &mut App, entity: Entity, mut each: impl FnMut(&App, usize)) -> Vec<f32> {
    let mut tops = vec![top(app, entity)];
    for frame in 0..PATIENCE_FRAMES {
        app.update();
        tops.push(top(app, entity));
        each(app, frame);
    }
    tops
}

/// The moves are a glide: in the same direction every frame, none more than a third of the whole.
fn assert_glides(tops: &[f32], what: &str) {
    let total = tops[tops.len() - 1] - tops[0];
    assert!(total.abs() > 1.0, "{what}: moved only {total}");
    for pair in tops.windows(2) {
        let step = (pair[1] - pair[0]) * total.signum();
        assert!(step >= -0.01, "{what}: went back by {step}");
        assert!(
            step <= MOST_OF_A_MOVE_IN_ONE_FRAME * total.abs() + 1e-3,
            "{what}: jumped {step} of {total} in one frame"
        );
    }
}

#[test]
fn a_block_whose_neighbour_is_taken_out_travels_up_and_its_child_rides_it() {
    let mut app = app(MotionSettings::default());
    let [first, second, _] = column(&mut app, true);
    let child = app.world().get::<Children>(second).expect("a child")[0];
    let before = top(&app, second);
    let child_gap = top(&app, child) - before;

    app.world_mut().entity_mut(first).despawn();
    let tops = follow_checking(&mut app, second, |app, frame| {
        assert!(
            (top(app, child) - top(app, second) - child_gap).abs() < 0.01,
            "the child rides it on every frame (frame {frame})"
        );
    });
    assert_glides(&tops, "the second block");
    let moved = before - tops[tops.len() - 1];
    assert!(
        (moved - (BLOCK_PX + GAP_PX)).abs() < 0.5,
        "it ends where layout put it: moved {moved}"
    );
    assert!(
        (top(&app, child) - top(&app, second) - child_gap).abs() < 0.01,
        "the child rode it"
    );

    // Settled: at its place and at rest, within the default spring's time to settle.
    let spring = app.world().resource::<GooeySkin>().motion.default;
    let settle = model::spring_settle_seconds(
        spring,
        model::SpringState {
            value: BLOCK_PX + GAP_PX,
            velocity: 0.0,
        },
        0.0,
    );
    let still = tops
        .windows(2)
        .position(|pair| {
            pair.iter()
                .all(|&t| (t - tops[tops.len() - 1]).abs() < 1e-3)
        })
        .expect("it settles");
    assert!(
        still as f32 * FRAME <= settle + FRAME,
        "settled after {} s, the spring settles in {settle} s",
        still as f32 * FRAME
    );
    assert_eq!(
        app.world()
            .get::<LayoutTravel>(second)
            .map(LayoutTravel::offset),
        Some(Vec2::ZERO)
    );
}

#[test]
fn an_unmarked_block_jumps_to_its_new_place() {
    let mut app = app(MotionSettings::default());
    let [first, second, _] = column(&mut app, false);
    let before = top(&app, second);
    app.world_mut().entity_mut(first).despawn();
    app.update();
    assert!(
        (before - top(&app, second) - (BLOCK_PX + GAP_PX)).abs() < 0.5,
        "with no MovesSmoothly the block is at its new place in one frame"
    );
}

#[test]
fn reduced_motion_lands_a_move_at_once() {
    let mut app = app(MotionSettings::reduced());
    let [first, second, _] = column(&mut app, true);
    let before = top(&app, second);
    app.world_mut().entity_mut(first).despawn();
    app.update();
    assert!((before - top(&app, second) - (BLOCK_PX + GAP_PX)).abs() < 0.5);
}

#[test]
fn a_block_that_grows_does_not_move_and_the_one_under_it_travels() {
    let mut app = app(MotionSettings::default());
    let [first, second, _] = column(&mut app, true);
    let first_top = top(&app, first);
    app.world_mut()
        .get_mut::<Node>(first)
        .expect("a node")
        .height = Val::Px(BLOCK_PX * 2.0);
    let tops = follow_checking(&mut app, second, |app, frame| {
        assert_eq!(
            top(app, first),
            first_top,
            "its corner stands still on every frame (frame {frame})"
        );
    });
    assert_glides(&tops, "the block under it");
}

#[test]
fn resizing_the_window_moves_nothing_through_motion() {
    let mut app = app(MotionSettings::default());
    // Centred, so a new window size moves every block.
    let blocks = [(); 2].map(|_| app.world_mut().spawn((block(), MovesSmoothly)).id());
    app.world_mut()
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .add_children(&blocks);
    frames(&mut app, 3);
    let before = top(&app, blocks[0]);

    let (window, mut state) = app
        .world_mut()
        .query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>()
        .single_mut(app.world_mut())
        .expect("one window");
    state.resolution = WindowResolution::new(1600, 900).with_scale_factor_override(1.0);
    app.world_mut().write_message(WindowResized {
        window,
        width: 1600.0,
        height: 900.0,
    });
    app.update();
    let after = top(&app, blocks[0]);
    assert!(
        after - before > 50.0,
        "the resize moved the block: {before} to {after}"
    );
    for &b in &blocks {
        assert_eq!(
            app.world().get::<LayoutTravel>(b).map(LayoutTravel::offset),
            Some(Vec2::ZERO),
            "and no motion carried it"
        );
    }
}

#[test]
fn tabs_are_marked_and_parts_moved_by_a_fitting_pass_are_not() {
    let mut app = app(MotionSettings::default());
    let tools = app
        .world_mut()
        .spawn(toolbar(children![button("Cut"), button("Copy")]))
        .id();
    app.world_mut().spawn(tabs(
        ["One", "Two"],
        0,
        children![tab_page(()), tab_page(())],
    ));
    app.world_mut().spawn(toast("Game saved"));
    frames(&mut app, 2);
    let mut tabs_found = 0;
    let mut query = app.world_mut().query::<(&Role, Has<MovesSmoothly>)>();
    for (role, marked) in query.iter(app.world()) {
        match role {
            Role::Tab => {
                tabs_found += 1;
                assert!(marked, "a tab travels");
            }
            Role::Toast | Role::StatusItem => assert!(!marked, "{role:?} is not marked"),
            _ => {}
        }
    }
    assert_eq!(tabs_found, 2);
    for &item in app.world().get::<Children>(tools).expect("items") {
        assert!(
            app.world().get::<MovesSmoothly>(item).is_none(),
            "a toolbar item is moved by the overflow pass, not marked"
        );
    }
}

#[test]
fn a_move_in_a_nodes_first_frames_lands() {
    let mut app = app(MotionSettings::default());
    let blocks = [(); 2].map(|_| app.world_mut().spawn((block(), MovesSmoothly)).id());
    let holder = app
        .world_mut()
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .add_children(&blocks)
        .id();
    app.update();
    // A fitting pass in the next frame pushes everything down.
    app.world_mut()
        .get_mut::<Node>(holder)
        .expect("a node")
        .padding = UiRect::top(Val::Px(40.0));
    let before = top(&app, blocks[1]);
    app.update();
    assert!(
        (top(&app, blocks[1]) - before - 40.0).abs() < 0.5,
        "a move in the frame after a node first appears lands"
    );
}

#[test]
fn a_settled_column_writes_nothing() {
    #[derive(Resource, Default)]
    struct Writes(u32);
    let mut app = app(MotionSettings::default());
    app.init_resource::<Writes>().add_systems(
        Last,
        |changed: Query<(), Changed<LayoutTravel>>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    let [first, ..] = column(&mut app, true);
    app.world_mut().entity_mut(first).despawn();
    frames(&mut app, PATIENCE_FRAMES);
    assert!(
        app.world().resource::<Writes>().0 > 0,
        "the move was written"
    );
    app.world_mut().resource_mut::<Writes>().0 = 0;
    frames(&mut app, 30);
    assert_eq!(
        app.world().resource::<Writes>().0,
        0,
        "and once settled, nothing"
    );
}

#[test]
fn taking_the_mark_off_stops_the_motion() {
    let mut app = app(MotionSettings::default());
    let [first, second, _] = column(&mut app, true);
    app.world_mut().entity_mut(first).despawn();
    frames(&mut app, 2);
    app.world_mut().entity_mut(second).remove::<MovesSmoothly>();
    app.update();
    let laid = top(&app, second);
    app.update();
    assert_eq!(
        top(&app, second),
        laid,
        "the block stands where layout puts it"
    );
}

#[test]
fn a_motion_target_on_the_node_is_not_read_as_a_move() {
    let mut app = app(MotionSettings::default());
    let [_, second, _] = column(&mut app, true);
    app.world_mut()
        .entity_mut(second)
        .insert(MotionTarget::default());
    frames(&mut app, 3);
    app.world_mut().entity_mut(second).insert(MotionTarget {
        translation: Vec2::new(0.0, 40.0),
        ..default()
    });
    let mut offsets = Vec::new();
    for _ in 0..PATIENCE_FRAMES {
        app.update();
        offsets.push(
            app.world()
                .get::<LayoutTravel>(second)
                .map(LayoutTravel::offset)
                .expect("a travel"),
        );
    }
    assert!(
        offsets.iter().all(|o| *o == Vec2::ZERO),
        "the slide is the MotionTarget's alone: {offsets:?}"
    );
}

#[test]
fn a_block_shown_again_lands_where_it_is_and_does_not_fly_in() {
    let mut app = app(MotionSettings::default());
    let [_, second, _] = column(&mut app, true);
    let shown_top = top(&app, second);
    app.world_mut()
        .get_mut::<Node>(second)
        .expect("a node")
        .display = Display::None;
    frames(&mut app, 3);
    app.world_mut()
        .get_mut::<Node>(second)
        .expect("a node")
        .display = Display::Flex;
    app.update();
    assert!(
        (top(&app, second) - shown_top).abs() < 0.5,
        "shown at its place in its first frame back: {} against {shown_top}",
        top(&app, second)
    );
}
