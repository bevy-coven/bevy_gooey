//! The LED (`led`): four states with four silhouettes and their words, a state written by the game
//! showing in the next frame, a busy glow that pulses under default motion and holds steady under
//! reduced motion, a settled LED that writes nothing, and the colours it draws in held to 3:1 on the
//! panel in every preset. Laid out for real in a headless window on a 100 ms clock.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiTransform};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.1;

/// How many frames the dot's fill, glow or transform changed in.
#[derive(Resource, Default)]
struct Writes(u32);

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
    .insert_resource(settings)
    .init_resource::<Writes>()
    .add_systems(
        Last,
        |changed: Query<(), DotWritten>, mut n: ResMut<Writes>| {
            n.0 += changed.iter().count() as u32;
        },
    );
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

/// A dot whose fill, glow or transform changed.
type DotWritten = (
    With<LedDotMarker>,
    Or<(
        Changed<BackgroundColor>,
        Changed<BoxShadow>,
        Changed<UiTransform>,
    )>,
);

/// Tags the dot so the write counter reads it alone.
#[derive(Component)]
struct LedDotMarker;

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// An LED on a screen; returns its row and its dot.
fn lit(app: &mut App, state: LedState) -> (Entity, Entity) {
    let row = app.world_mut().spawn(led("Agent", state)).id();
    app.world_mut().spawn(screen()).add_child(row);
    frames(app, 4);
    let dot = dot_of(app, row);
    app.world_mut().entity_mut(dot).insert(LedDotMarker);
    (row, dot)
}

fn dot_of(app: &App, row: Entity) -> Entity {
    app.world()
        .get::<Children>(row)
        .expect("children")
        .iter()
        .find(|c| app.world().get::<Role>(*c) == Some(&Role::LedDot))
        .expect("a dot")
}

fn word(app: &mut App, row: Entity) -> String {
    let world = app.world_mut();
    let kids: Vec<Entity> = world
        .get::<Children>(row)
        .expect("children")
        .iter()
        .collect();
    kids.into_iter()
        .find_map(|k| world.get::<LedWord>(k).and_then(|_| world.get::<Text>(k)))
        .expect("a word")
        .0
        .clone()
}

/// What tells a state's silhouette apart: its corner radius, its turn and whether it is hollow.
fn silhouette(app: &App, dot: Entity) -> (String, f32, f32, bool) {
    let node = app.world().get::<Node>(dot).expect("node");
    let at = app.world().get::<UiTransform>(dot).expect("transform");
    (
        format!("{:?}", node.border_radius.top_left),
        at.rotation.as_degrees(),
        at.scale.x,
        app.world().get::<BackgroundColor>(dot).expect("fill").0 == Color::NONE,
    )
}

fn glow_alpha(app: &App, dot: Entity) -> Option<f32> {
    app.world()
        .get::<BoxShadow>(dot)
        .and_then(|s| s.0.first())
        .map(|s| s.color.to_srgba().alpha)
}

fn set(app: &mut App, row: Entity, state: LedState) {
    app.world_mut().entity_mut(row).insert(Led(state));
    frames(app, 2);
}

#[test]
fn each_state_has_its_own_silhouette_and_its_word() {
    let mut app = app(MotionSettings::default());
    let (row, dot) = lit(&mut app, LedState::Off);
    let mut seen = Vec::new();
    for state in [
        LedState::Off,
        LedState::Live,
        LedState::Busy,
        LedState::Failed,
    ] {
        set(&mut app, row, state);
        assert_eq!(word(&mut app, row), state.word());
        let shape = silhouette(&app, dot);
        assert!(
            !seen.contains(&shape),
            "{state:?} repeats a silhouette {shape:?}"
        );
        seen.push(shape);
    }
    set(&mut app, row, LedState::Off);
    assert_eq!(
        app.world().get::<BackgroundColor>(dot).map(|b| b.0),
        Some(Color::NONE)
    );
    assert_eq!(glow_alpha(&app, dot), None, "an unlit lens has no glow");
    set(&mut app, row, LedState::Live);
    assert_eq!(
        app.world().get::<BackgroundColor>(dot).map(|b| b.0),
        Some(app.world().resource::<GooeySkin>().live())
    );
    assert!(
        glow_alpha(&app, dot).is_some_and(|a| a > 0.3),
        "a lit lens glows"
    );
}

#[test]
fn the_dot_is_a_square_of_its_text_sizes() {
    let mut app = app(MotionSettings::default());
    let (_, dot) = lit(&mut app, LedState::Live);
    let size = app
        .world()
        .get::<ComputedNode>(dot)
        .expect("laid out")
        .size();
    let skin = app.world().resource::<GooeySkin>();
    let expected = skin.text_size * skin.text_scale * 1.1;
    assert!(
        (size.x - size.y).abs() < 1.0 && (size.x - expected).abs() < 1.5,
        "{size:?} vs {expected}"
    );
}

#[test]
fn a_busy_glow_pulses_and_a_reduced_one_holds_still() {
    let mut pulsing = app(MotionSettings::default());
    let (_, dot) = lit(&mut pulsing, LedState::Busy);
    let mut alphas = Vec::new();
    for _ in 0..12 {
        frames(&mut pulsing, 1);
        alphas.push(glow_alpha(&pulsing, dot).expect("a glow"));
    }
    let (lo, hi) = alphas
        .iter()
        .fold((f32::MAX, f32::MIN), |(l, h), a| (l.min(*a), h.max(*a)));
    assert!(hi - lo > 0.3, "it swings over a period: {lo}..{hi}");
    assert!(lo >= 0.24 && hi <= 0.86, "inside its range: {lo}..{hi}");
    for settings in [MotionSettings::reduced(), MotionSettings::off()] {
        let mut still = app(settings);
        let (_, dot) = lit(&mut still, LedState::Busy);
        let first = glow_alpha(&still, dot);
        frames(&mut still, 12);
        assert_eq!(glow_alpha(&still, dot), first, "steady");
        assert!(first.is_some());
    }
}

#[test]
fn a_settled_led_writes_nothing() {
    for state in [LedState::Off, LedState::Live, LedState::Failed] {
        let mut app = app(MotionSettings::default());
        lit(&mut app, state);
        frames(&mut app, 6);
        let writes = app.world().resource::<Writes>().0;
        frames(&mut app, 30);
        assert_eq!(app.world().resource::<Writes>().0, writes, "{state:?}");
    }
}

#[test]
fn the_colours_it_draws_in_hold_three_to_one_on_the_panel_in_every_preset() {
    for (name, skin) in [
        ("dark", GooeySkin::dark()),
        ("parchment", GooeySkin::parchment()),
        ("high_contrast", GooeySkin::high_contrast()),
        ("tool", GooeySkin::tool()),
    ] {
        let ground = Srgba::from(skin.solid_surface());
        for (what, colour) in [
            ("live", skin.live()),
            ("warning", skin.warning()),
            ("danger", skin.danger),
            ("outline", skin.outline),
        ] {
            let c = Srgba::from(colour);
            let ratio = bevy_gooey::model::contrast_ratio(
                [c.red, c.green, c.blue],
                [ground.red, ground.green, ground.blue],
            );
            assert!(ratio >= 3.0, "{name}: {what} is {ratio:.2}:1 on the panel");
        }
    }
}
