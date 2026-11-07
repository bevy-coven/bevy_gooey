//! Size motion: a section that opens grows to its height and one that closes shrinks away, with the
//! widget under it travelling, and a split's collapse (Enter on its divider) slides there; a key step
//! during it, reduced motion and a game's write land at once. In a headless window on a 60 Hz clock
//! the test sets, with motion on.

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

/// How many frames the test waits for a motion to settle: two seconds.
const PATIENCE_FRAMES: usize = 120;

/// The most of a move one frame may draw: a third of it.
const MOST_OF_A_MOVE_IN_ONE_FRAME: f32 = 1.0 / 3.0;

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

/// A logical-px height of a node as laid out.
fn height(app: &App, entity: Entity) -> f32 {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    node.size().y * node.inverse_scale_factor()
}

fn width(app: &App, entity: Entity) -> f32 {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    node.size().x * node.inverse_scale_factor()
}

/// A node's top edge as drawn, logical px (the window's scale factor is 1).
fn top(app: &App, entity: Entity) -> f32 {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size();
    app.world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation
        .y
        - size.y / 2.0
}

fn child_with_role(app: &App, parent: Entity, role: Role) -> Entity {
    app.world()
        .get::<Children>(parent)
        .and_then(|c| {
            c.iter()
                .find(|&e| app.world().get::<Role>(e) == Some(&role))
        })
        .unwrap_or_else(|| panic!("no {role:?} under {parent}"))
}

/// A section with three widgets, closed or open, and a button under it. Returns the section, its
/// body and the button.
fn section_and_button(app: &mut App, open: bool) -> (Entity, Entity, Entity) {
    let world = app.world_mut();
    let sec = world
        .spawn(section(
            "Physics",
            open,
            children![
                toggle("Wind", true),
                toggle("Rain", false),
                slider("Gravity", 9.8, 0.0..=20.0)
            ],
        ))
        .id();
    let under = world.spawn(button("Apply")).id();
    let card = world.spawn(panel()).add_children(&[sec, under]).id();
    world.spawn(screen()).add_child(card);
    frames(app, 6);
    let body = child_with_role(app, sec, Role::SectionBody);
    (sec, body, under)
}

/// The steps go one way, none more than a third of the whole, and the whole is more than `least`.
fn assert_glides(values: &[f32], least: f32, what: &str) {
    let total = values[values.len() - 1] - values[0];
    assert!(total.abs() > least, "{what}: moved only {total}");
    for pair in values.windows(2) {
        let step = (pair[1] - pair[0]) * total.signum();
        assert!(step >= -0.01, "{what}: went back by {step}");
        assert!(
            step <= MOST_OF_A_MOVE_IN_ONE_FRAME * total.abs() + 1e-3,
            "{what}: jumped {step} of {total} in one frame"
        );
    }
}

/// The spring's settle time for a move of `distance`.
fn settle_seconds(app: &App, distance: f32) -> f32 {
    let spring = app.world().resource::<GooeySkin>().motion.default;
    model::spring_settle_seconds(
        spring,
        model::SpringState {
            value: distance,
            velocity: 0.0,
        },
        0.0,
    )
}

#[test]
fn opening_a_section_grows_its_body_and_the_widget_under_it_travels() {
    // Where an open section stands, with no motion: the height to grow to and the place to reach.
    let mut still = app(MotionSettings::off());
    let (_, open_body, open_under) = section_and_button(&mut still, true);
    let natural = height(&still, open_body);
    let under_at = top(&still, open_under);

    let mut app = app(MotionSettings::default());
    let (sec, body, under) = section_and_button(&mut app, false);
    app.world_mut().entity_mut(sec).insert(SectionOpen(true));
    let mut heights = vec![0.0];
    let mut tops = vec![top(&app, under)];
    let mut settled_at = None;
    for frame in 1..=PATIENCE_FRAMES {
        app.update();
        heights.push(height(&app, body));
        tops.push(top(&app, under));
        if settled_at.is_none() && app.world().get::<SectionGlide>(body).is_none() && frame > 2 {
            settled_at = Some(frame);
        }
    }
    assert!(
        app.world().get::<SectionOpen>(sec).is_some_and(|o| o.0),
        "the state is open"
    );
    assert_glides(&heights, 40.0, "the body's height");
    assert_glides(&tops, 40.0, "the button under it");
    assert!(
        (heights[heights.len() - 1] - natural).abs() < 0.5,
        "it ends at its own height: {} against {natural}",
        heights[heights.len() - 1]
    );
    assert!((tops[tops.len() - 1] - under_at).abs() < 0.5);
    let seconds = settled_at.expect("it settles") as f32 * FRAME;
    let settle = settle_seconds(&app, natural);
    assert!(
        seconds <= settle + 3.0 * FRAME,
        "settled after {seconds} s; the spring settles in {settle} s"
    );
}

#[test]
fn closing_a_section_shrinks_its_body_away_and_takes_it_out_of_layout() {
    let mut app = app(MotionSettings::default());
    let (sec, body, under) = section_and_button(&mut app, true);
    let start = height(&app, body);
    app.world_mut().entity_mut(sec).insert(SectionOpen(false));
    let mut heights = vec![start];
    let mut tops = vec![top(&app, under)];
    for _ in 0..PATIENCE_FRAMES {
        app.update();
        heights.push(height(&app, body));
        tops.push(top(&app, under));
    }
    assert_glides(&heights, 40.0, "the body's height");
    assert_glides(&tops, 40.0, "the button under it");
    let node = app.world().get::<Node>(body).expect("a node");
    assert_eq!(node.display, Display::None, "shut: out of layout");
    assert_eq!(
        app.world().get::<Visibility>(body),
        Some(&Visibility::Hidden),
        "and out of the Tab order"
    );
    assert_eq!(node.height, Val::Auto, "its own height given back");
}

#[test]
fn a_section_toggled_again_on_its_way_turns_round_without_a_jump() {
    let mut app = app(MotionSettings::default());
    let (sec, body, _) = section_and_button(&mut app, false);
    app.world_mut().entity_mut(sec).insert(SectionOpen(true));
    frames(&mut app, 6);
    let partway = height(&app, body);
    assert!(partway > 1.0, "opening: {partway}");
    app.world_mut().entity_mut(sec).insert(SectionOpen(false));
    app.update();
    let next = height(&app, body);
    assert!(
        (next - partway).abs() < partway / 2.0,
        "it turns round from where it was: {partway} then {next}"
    );
    frames(&mut app, PATIENCE_FRAMES);
    assert_eq!(
        app.world().get::<Node>(body).expect("a node").display,
        Display::None
    );
}

#[test]
fn reduced_motion_opens_a_section_at_once() {
    let mut app = app(MotionSettings::reduced());
    let (sec, body, _) = section_and_button(&mut app, false);
    app.world_mut().entity_mut(sec).insert(SectionOpen(true));
    frames(&mut app, 2);
    assert!(app.world().get::<SectionGlide>(body).is_none());
    assert!(height(&app, body) > 40.0, "open in the frame it is asked");
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one window")
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

/// A horizontal split at 0.5 with a 100 px least first pane, focused on its divider. Returns the
/// split, its first pane and its divider.
fn focused_split(app: &mut App) -> (Entity, Entity, Entity) {
    let world = app.world_mut();
    let root = world
        .spawn((
            split(SplitAxis::Horizontal, panel(), panel()),
            SplitMin::new(100.0, 0.0),
        ))
        .id();
    world.spawn(screen()).add_child(root);
    frames(app, 6);
    let first = child_with_role(app, root, Role::SplitPane);
    let divider = child_with_role(app, root, Role::SplitDivider);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(divider, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
    (root, first, divider)
}

#[test]
fn enter_on_a_divider_slides_the_first_pane_to_its_least_and_the_state_is_new_at_once() {
    let mut app = app(MotionSettings::default());
    let (root, first, _) = focused_split(&mut app);
    let wide = width(&app, first);
    tap(&mut app, KeyCode::Enter);
    let target = app
        .world()
        .get::<SplitFraction>(root)
        .expect("a fraction")
        .0;
    assert!(
        target < 0.5,
        "the fraction is the collapsed one at once: {target}"
    );
    let mut widths = vec![wide];
    for _ in 0..PATIENCE_FRAMES {
        app.update();
        widths.push(width(&app, first));
    }
    assert_glides(&widths, 100.0, "the first pane");
    assert!(
        (widths[widths.len() - 1] - 100.0).abs() < 1.0,
        "collapsed to its least: {}",
        widths[widths.len() - 1]
    );
    assert!(app.world().get::<SplitGlide>(root).is_none(), "and at rest");
}

#[test]
fn a_key_step_during_the_slide_lands_at_once() {
    let mut app = app(MotionSettings::default());
    let (root, first, _) = focused_split(&mut app);
    tap(&mut app, KeyCode::Enter);
    frames(&mut app, 3);
    assert!(app.world().get::<SplitGlide>(root).is_some(), "sliding");
    tap(&mut app, KeyCode::ArrowRight);
    app.update();
    assert!(
        app.world().get::<SplitGlide>(root).is_none(),
        "the key step took the slide off"
    );
    let fraction = app
        .world()
        .get::<SplitFraction>(root)
        .expect("a fraction")
        .0;
    let room = width(&app, root);
    let laid = width(&app, first);
    assert!(
        laid > 100.0 && (laid / room - fraction).abs() < 0.05,
        "the pane is where the fraction says at once: {laid} of {room}, fraction {fraction}"
    );
}

#[test]
fn reduced_motion_collapses_a_split_at_once() {
    let mut app = app(MotionSettings::reduced());
    let (root, first, _) = focused_split(&mut app);
    tap(&mut app, KeyCode::Enter);
    assert!(app.world().get::<SplitGlide>(root).is_none());
    assert!((width(&app, first) - 100.0).abs() < 1.0);
}

#[test]
fn a_double_press_on_the_divider_slides_the_collapse() {
    let mut app = app(MotionSettings::default());
    let (root, first, divider) = focused_split(&mut app);
    let wide = width(&app, first);
    app.world_mut().trigger(DoublePress { entity: divider });
    app.update();
    assert!(
        app.world().get::<SplitGlide>(root).is_some(),
        "a double press slides the collapse"
    );
    assert!(
        width(&app, first) > 100.0 + (wide - 100.0) / 3.0,
        "not there in one frame"
    );
    frames(&mut app, PATIENCE_FRAMES);
    assert!((width(&app, first) - 100.0).abs() < 1.0);
}

#[test]
fn a_game_writing_the_same_state_every_frame_starts_nothing() {
    #[derive(Resource, Default)]
    struct Writes(u32);
    let mut app = app(MotionSettings::default());
    let (sec, body, _) = section_and_button(&mut app, false);
    app.init_resource::<Writes>().add_systems(
        Last,
        |nodes: Query<(), Changed<Node>>, mut writes: ResMut<Writes>| {
            writes.0 += nodes.iter().count() as u32
        },
    );
    frames(&mut app, 2);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.world_mut().entity_mut(sec).insert(SectionOpen(false));
        app.update();
        assert!(app.world().get::<SectionGlide>(body).is_none());
    }
    assert_eq!(
        app.world().resource::<Writes>().0,
        0,
        "nothing is laid out again"
    );
}

#[test]
fn a_shrinking_body_takes_no_tab_stop_and_gives_them_back() {
    let mut app = app(MotionSettings::default());
    let (sec, body, _) = section_and_button(&mut app, true);
    let toggle = app.world().get::<Children>(body).expect("widgets")[0];
    let tab = app
        .world()
        .get::<bevy::input_focus::tab_navigation::TabIndex>(toggle)
        .copied();
    assert!(tab.is_some(), "an open body's toggle is a Tab stop");
    app.world_mut().entity_mut(sec).insert(SectionOpen(false));
    frames(&mut app, 2);
    assert!(app.world().get::<SectionGlide>(body).is_some(), "shrinking");
    assert!(
        app.world()
            .get::<bevy::input_focus::tab_navigation::TabIndex>(toggle)
            .is_none(),
        "no Tab stop while it shrinks"
    );
    // Turned round, the stops come back.
    app.world_mut().entity_mut(sec).insert(SectionOpen(true));
    frames(&mut app, PATIENCE_FRAMES);
    assert_eq!(
        app.world()
            .get::<bevy::input_focus::tab_navigation::TabIndex>(toggle)
            .copied(),
        tab
    );
}
