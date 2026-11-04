//! `Cooldown`: a timer on a widget, in the same headless window, on a clock the test sets.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::time::Duration;

use bevy::a11y::{AccessibilityNode, AccessibilityRequested};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{BackgroundGradient, ComputedNode, Gradient};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::model::{cooldown_sweep, cooldown_text, cooldown_words};
use bevy_gooey::*;

/// How many `CooldownReady` the game heard, and on what.
#[derive(Resource, Default)]
struct Ready(Vec<Entity>);

/// How many times the cover's gradient and the chip's words were written, in all.
#[derive(Resource, Default)]
struct Writes {
    gradient: usize,
    chip: usize,
}

/// A headless window and a clock that moves 0.1 s a frame.
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
    .init_resource::<Ready>()
    .init_resource::<Writes>()
    .add_systems(
        Last,
        |gradients: Query<(), Changed<BackgroundGradient>>,
         chips: Query<&Role, Changed<Text>>,
         mut writes: ResMut<Writes>| {
            writes.gradient += gradients.iter().count();
            writes.chip += chips.iter().filter(|r| **r == Role::CooldownText).count();
        },
    )
    .add_observer(|ready: On<CooldownReady>, mut seen: ResMut<Ready>| seen.0.push(ready.entity))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.world_mut().spawn(Camera2d);
    app.world().resource::<AccessibilityRequested>().set(true);
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// A screen with one "Fireball" button carrying a `Cooldown`, settled.
fn hotbar(app: &mut App, cooldown: Cooldown) -> Entity {
    let root = app.world_mut().spawn(screen()).id();
    let panel = app.world_mut().spawn((panel(), ChildOf(root))).id();
    let slot = app
        .world_mut()
        .spawn((button("Fireball"), cooldown, ChildOf(panel)))
        .id();
    frames(app, 4);
    slot
}

fn cover_of(app: &App, owner: Entity) -> Option<Entity> {
    app.world()
        .get::<Children>(owner)?
        .iter()
        .find(|c| app.world().get::<Role>(*c) == Some(&Role::Cooldown))
}

fn chip_text(app: &App, owner: Entity) -> String {
    let cover = cover_of(app, owner).expect("a cover");
    app.world()
        .get::<Children>(cover)
        .expect("the cover holds the chip")
        .iter()
        .find_map(|c| app.world().get::<Text>(c))
        .map(|text| text.0.clone())
        .expect("a chip with text")
}

fn visibility(app: &App, entity: Entity) -> Visibility {
    *app.world().get::<Visibility>(entity).expect("visibility")
}

/// The angle the clear wedge has opened to, read off the cover's gradient.
fn clear_angle(app: &App, owner: Entity) -> f32 {
    let cover = cover_of(app, owner).expect("a cover");
    let gradient = app
        .world()
        .get::<BackgroundGradient>(cover)
        .expect("gradient");
    let Some(Gradient::Conic(conic)) = gradient.0.first() else {
        panic!("the cover is not a conic gradient: {gradient:?}");
    };
    conic.stops[1].angle.expect("an explicit angle")
}

fn remaining(app: &App, owner: Entity) -> f32 {
    app.world()
        .get::<Cooldown>(owner)
        .expect("cooldown")
        .remaining
}

#[test]
fn a_ready_widget_has_a_hidden_cover_and_no_words() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    let cover = cover_of(&app, owner).expect("one cover exists as soon as the component does");
    assert_eq!(visibility(&app, cover), Visibility::Hidden);
    assert_eq!(chip_text(&app, owner), "");
    assert_eq!(
        app.world().get::<Children>(owner).map(|c| c
            .iter()
            .filter(|c| cover_of(&app, owner) == Some(*c))
            .count()),
        Some(1)
    );
}

#[test]
fn starting_one_dims_the_whole_widget_and_counts_in_whole_seconds() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(8.0);
    frames(&mut app, 1);
    let cover = cover_of(&app, owner).unwrap();
    assert_eq!(visibility(&app, cover), Visibility::Inherited);
    assert_eq!(chip_text(&app, owner), "8");
    assert!(clear_angle(&app, owner) < 0.15, "all dimmed at the start");
}

#[test]
fn the_clear_wedge_opens_clockwise_in_step_with_the_time_left() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(8.0);
    // One frame to see it, twenty more: two seconds of game time.
    frames(&mut app, 21);
    let left = remaining(&app, owner);
    assert!((5.7..6.1).contains(&left), "{left}");
    assert_eq!(chip_text(&app, owner), cooldown_text(left));
    let angle = clear_angle(&app, owner);
    assert!((angle - cooldown_sweep(left, 8.0)).abs() < 1e-4, "{angle}");
    assert!((angle - FRAC_PI_2).abs() < 0.2, "a quarter turn, {angle}");
}

#[test]
fn it_ends_hidden_and_sends_one_ready_on_the_widget() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(1.0);
    frames(&mut app, 25);
    assert!(app.world().get::<Cooldown>(owner).unwrap().is_ready());
    assert_eq!(app.world().resource::<Ready>().0, vec![owner]);
    let cover = cover_of(&app, owner).unwrap();
    assert_eq!(visibility(&app, cover), Visibility::Hidden);
    assert_eq!(chip_text(&app, owner), "");
    frames(&mut app, 10);
    assert_eq!(
        app.world().resource::<Ready>().0.len(),
        1,
        "once, not every frame"
    );
}

#[test]
fn a_game_that_sets_zero_itself_sends_nothing() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(5.0);
    frames(&mut app, 3);
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .remaining = 0.0;
    frames(&mut app, 3);
    let cover = cover_of(&app, owner).unwrap();
    assert_eq!(visibility(&app, cover), Visibility::Hidden);
    assert!(app.world().resource::<Ready>().0.is_empty());
}

#[test]
fn a_paused_game_holds_its_cooldowns() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(8.0);
    frames(&mut app, 3);
    let before = remaining(&app, owner);
    app.world_mut().resource_mut::<Time<Virtual>>().pause();
    frames(&mut app, 10);
    assert_eq!(remaining(&app, owner), before);
    app.world_mut().resource_mut::<Time<Virtual>>().unpause();
    frames(&mut app, 5);
    assert!(remaining(&app, owner) < before);
}

#[test]
fn starting_again_restarts_the_sweep() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::new(4.0));
    frames(&mut app, 20);
    assert!(remaining(&app, owner) < 2.5);
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(10.0);
    frames(&mut app, 1);
    assert_eq!(chip_text(&app, owner), "10");
    assert!(clear_angle(&app, owner) < 0.15);
}

#[test]
fn removing_the_component_removes_the_cover_in_the_same_flush() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::new(5.0));
    let cover = cover_of(&app, owner).unwrap();
    app.world_mut().entity_mut(owner).remove::<Cooldown>();
    app.world_mut().flush();
    assert!(app.world().get_entity(cover).is_err());
    // And inserting one again brings exactly one back.
    app.world_mut().entity_mut(owner).insert(Cooldown::new(3.0));
    app.world_mut().flush();
    frames(&mut app, 1);
    assert!(cover_of(&app, owner).is_some());
    assert_eq!(
        app.world()
            .get::<Children>(owner)
            .unwrap()
            .iter()
            .filter(|c| app.world().get::<Role>(*c) == Some(&Role::Cooldown))
            .count(),
        1
    );
}

#[test]
fn a_running_cooldown_moves_nothing() {
    let mut app = app();
    let idle = hotbar(&mut app, Cooldown::default());
    let size = |app: &App| app.world().get::<ComputedNode>(idle).unwrap().size();
    let before = size(&app);
    app.world_mut()
        .get_mut::<Cooldown>(idle)
        .unwrap()
        .start(99.0);
    frames(&mut app, 5);
    assert_eq!(size(&app), before, "an absolute cover takes no room");
}

#[test]
fn the_cover_fills_the_widgets_box_and_the_chip_sits_in_its_middle() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::new(30.0));
    frames(&mut app, 2);
    let cover = cover_of(&app, owner).unwrap();
    let world = app.world();
    let owner_box = world.get::<ComputedNode>(owner).unwrap().size();
    let cover_box = world.get::<ComputedNode>(cover).unwrap().size();
    let border = world.get::<ComputedNode>(owner).unwrap().border();
    let inside = Vec2::new(
        owner_box.x - border.min_inset.x - border.max_inset.x,
        owner_box.y - border.min_inset.y - border.max_inset.y,
    );
    assert!(
        (cover_box - inside).abs().max_element() <= 1.0,
        "{cover_box} in {inside}"
    );
    let chip = world.get::<Children>(cover).unwrap()[0];
    let chip_box = world.get::<ComputedNode>(chip).unwrap().size();
    assert!(chip_box.x > 0.0 && chip_box.x < cover_box.x, "{chip_box}");
}

#[test]
fn a_cover_and_its_chip_let_the_pointer_through() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::new(30.0));
    let cover = cover_of(&app, owner).unwrap();
    assert_eq!(app.world().get::<Pickable>(cover), Some(&Pickable::IGNORE));
    let chip = app.world().get::<Children>(cover).unwrap()[0];
    assert_eq!(app.world().get::<Pickable>(chip), Some(&Pickable::IGNORE));
}

#[test]
fn the_dim_is_the_skins_background_and_follows_a_scope() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::new(30.0));
    frames(&mut app, 2);
    let dim = |app: &App| {
        let cover = cover_of(app, owner).unwrap();
        let gradient = app.world().get::<BackgroundGradient>(cover).unwrap();
        let Some(Gradient::Conic(conic)) = gradient.0.first() else {
            panic!("not conic");
        };
        Srgba::from(conic.stops[4].color)
    };
    let dark = GooeySkin::dark();
    let expected = Srgba::from(dark.background);
    let seen = dim(&app);
    assert!((seen.red - expected.red).abs() < 1e-4 && (seen.blue - expected.blue).abs() < 1e-4);
    assert!(seen.alpha > 0.6 && seen.alpha < 0.95, "{}", seen.alpha);
    app.world_mut()
        .entity_mut(owner)
        .insert(SkinScope(GooeySkin::parchment()));
    frames(&mut app, 2);
    let parchment = Srgba::from(GooeySkin::parchment().background);
    let seen = dim(&app);
    assert!(
        (seen.red - parchment.red).abs() < 1e-4 && (seen.red - expected.red).abs() > 0.05,
        "{seen:?}"
    );
}

#[test]
fn the_accessible_name_carries_the_seconds_and_goes_back() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    let name = |app: &App| {
        app.world()
            .get::<AccessibilityNode>(owner)
            .expect("a node")
            .0
            .label()
            .map(str::to_owned)
    };
    assert_eq!(name(&app).as_deref(), Some("Fireball"));
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(8.0);
    frames(&mut app, 3);
    let words = cooldown_words(remaining(&app, owner));
    assert_eq!(name(&app), Some(format!("Fireball, {words}")));
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .remaining = 0.0;
    frames(&mut app, 3);
    assert_eq!(name(&app).as_deref(), Some("Fireball"));
}

#[test]
fn a_ready_widget_writes_nothing_and_a_running_one_only_what_changed() {
    let mut app = app();
    let owner = hotbar(&mut app, Cooldown::default());
    frames(&mut app, 2);
    let written = |app: &App| {
        let writes = app.world().resource::<Writes>();
        (writes.gradient, writes.chip)
    };
    let before = written(&app);
    frames(&mut app, 30);
    assert_eq!(written(&app), before, "a ready widget writes nothing");
    // A sixty-second cooldown: the sweep moves every frame, the words once a second.
    app.world_mut()
        .get_mut::<Cooldown>(owner)
        .unwrap()
        .start(60.0);
    frames(&mut app, 2);
    let before = written(&app);
    frames(&mut app, 8);
    let (gradient, chip) = written(&app);
    assert!(gradient - before.0 >= 6, "the sweep follows the time");
    assert!(
        chip - before.1 <= 1,
        "the chip's words stand while the second does"
    );
}

#[test]
fn every_preset_reads_the_chip_and_a_bad_text_colour_is_reported() {
    for skin in [
        GooeySkin::dark(),
        GooeySkin::parchment(),
        GooeySkin::high_contrast(),
        GooeySkin::tool(),
    ] {
        assert!(skin.problems().is_empty(), "{:?}", skin.problems());
    }
    assert!(
        GooeySkin::high_contrast()
            .high_contrast_problems()
            .is_empty()
    );
    // Text as light as the chip over a white icon: the chip is no longer readable.
    let mut skin = GooeySkin::dark();
    skin.text = skin.solid_surface();
    assert!(
        skin.problems().iter().any(|p| matches!(
            p,
            SkinProblem::LowContrast {
                pair: "text on cooldown chip",
                ..
            }
        )),
        "{:?}",
        skin.problems()
    );
}

#[test]
fn the_words_are_read_in_whole_seconds_then_tenths() {
    assert_eq!(cooldown_text(59.0), "59");
    assert_eq!(cooldown_text(3.0), "3");
    assert_eq!(cooldown_text(2.99), "3.0");
    assert_eq!(cooldown_text(0.05), "0.1");
    assert_eq!(cooldown_text(f32::NAN), "");
    assert_eq!(cooldown_text(-1.0), "");
    assert_eq!(cooldown_words(1.0), "1 second remaining");
    assert_eq!(cooldown_words(120.0), "2 minutes remaining");
    assert_eq!(cooldown_sweep(0.0, 8.0), TAU);
    assert_eq!(cooldown_sweep(4.0, 0.0), 0.0);
    assert_eq!(cooldown_sweep(100.0, 8.0), 0.0);
}
