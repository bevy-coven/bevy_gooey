//! Interface that follows things in the world: a health bar over each of five wandering enemies
//! (`WorldAnchor::above`, hidden when the enemy leaves the screen) and a beacon for a camp far to
//! the right that stays on the screen's edge, pointing, until the camera sweeps near enough to see it
//! (`WorldAnchor::pinned`, `AnchorPinned`).
//!
//! The camera sweeps left and right on its own; every position comes from the frame count, so a
//! capture is the same run every time. Run `cargo run --release --example world`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `world.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. `GOOEY_SKIN=parchment`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_START=<frame>` starts the sweep at that frame.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::WindowResolution;
use bevy_gooey::*;

/// Capture mode's state, present only when `GOOEY_FRAMES` is set.
#[derive(Resource)]
struct Capture {
    at: u32,
    shot: String,
    dir: Option<String>,
    from: u32,
    every: u32,
    kept: u32,
    frame: u32,
    total_ms: f64,
    counted: u32,
}

/// The frame the first captured frame is at, after the layout has settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;
/// Frames of one full camera sweep (there and back).
const SWEEP_FRAMES: f32 = 480.0;
/// How far right of the origin the camera goes, in world units.
const SWEEP_REACH: f32 = 760.0;
/// The camp's world position: out of view until the camera is well along its sweep.
const CAMP: Vec3 = Vec3::new(1180.0, 40.0, 0.0);

/// The frame counter every position is a function of.
#[derive(Resource)]
struct Clock(f32);

/// A wandering enemy: its home and its phase.
#[derive(Component)]
struct Wanderer {
    home: Vec2,
    phase: f32,
}

/// The beacon's arrow text, which turns with [`AnchorPinned`].
#[derive(Component)]
struct Arrow;

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn main() {
    let mut skin = match std::env::var("GOOEY_SKIN").as_deref() {
        Ok("parchment") => GooeySkin::parchment(),
        Ok("high_contrast") => GooeySkin::high_contrast(),
        _ => GooeySkin::dark(),
    };
    if let Some(scale) = env("GOOEY_TEXT_SCALE") {
        skin.text_scale = scale;
    }
    let mut window = Window {
        title: "bevy_gooey world".into(),
        ..default()
    };
    if let Some((w, h)) = std::env::var("GOOEY_SIZE")
        .ok()
        .and_then(|s| s.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
        .and_then(|(w, h)| Some((w.ok()?, h.ok()?)))
    {
        window.resolution = WindowResolution::new(w, h).with_scale_factor_override(1.0);
    }
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }),
        GooeyPlugins,
    ))
    .insert_resource(ClearColor(skin.background))
    .insert_resource(skin)
    .insert_resource(Clock(env("GOOEY_START").unwrap_or(0.0)))
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (follow_background, sweep, point_arrow.after(sweep), capture),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "world.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
        });
    }
    app.run();
}

/// The world: a camera, five enemies each with a health bar over it, the camp and its beacon, and
/// the legend.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let names = [
        ("Wolf", 0.85, Color::srgb(0.75, 0.35, 0.30)),
        ("Boar", 0.40, Color::srgb(0.70, 0.55, 0.30)),
        ("Wolf", 0.62, Color::srgb(0.75, 0.35, 0.30)),
        ("Bandit", 0.20, Color::srgb(0.45, 0.50, 0.75)),
        ("Boar", 1.00, Color::srgb(0.70, 0.55, 0.30)),
    ];
    for (i, (name, health, colour)) in names.into_iter().enumerate() {
        let home = Vec2::new(-420.0 + i as f32 * 340.0, -70.0 + (i % 2) as f32 * 150.0);
        let enemy = commands
            .spawn((
                Sprite::from_color(colour, Vec2::splat(56.0)),
                Transform::from_translation(home.extend(0.0)),
                Wanderer {
                    home,
                    phase: i as f32 * 1.7,
                },
            ))
            .id();
        // The nameplate: a column of the name and the bar, its foot 44 units above the enemy.
        commands.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            WorldAnchor::above(enemy, 44.0),
            children![label(name), meter(health, 1.0)],
        ));
    }
    let camp = commands
        .spawn((
            Sprite::from_color(Color::srgb(0.85, 0.75, 0.35), Vec2::new(90.0, 60.0)),
            Transform::from_translation(CAMP),
        ))
        .id();
    commands.spawn((
        panel(),
        WorldAnchor::above(camp, 48.0).pinned(),
        children![(label("Camp  >"), Arrow)],
    ));
    commands.spawn((
        screen(),
        children![(
            prompts(),
            children![
                prompt("Select", Prompt::SELECT),
                prompt("Back", Prompt::BACK)
            ],
        )],
    ));
}

/// The camera sweeps out and back, and each enemy wanders on a small loop round its home.
fn sweep(
    mut clock: ResMut<Clock>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
    mut wanderers: Query<(&mut Transform, &Wanderer), Without<Camera2d>>,
) {
    clock.0 += 1.0;
    let turn = clock.0 / SWEEP_FRAMES * std::f32::consts::TAU;
    camera.translation.x = SWEEP_REACH * (0.5 - 0.5 * turn.cos());
    for (mut at, w) in &mut wanderers {
        let t = clock.0 / 70.0 + w.phase;
        at.translation.x = w.home.x + 60.0 * t.cos();
        at.translation.y = w.home.y + 30.0 * (t * 1.3).sin();
    }
}

/// The beacon's arrow says which way the camp lies while it is pinned, and nothing once it is seen.
fn point_arrow(
    mut arrows: Query<(&mut Text, &ChildOf), With<Arrow>>,
    pinned: Query<&AnchorPinned>,
) {
    for (mut text, parent) in &mut arrows {
        let want = match pinned.get(parent.parent()) {
            Ok(p) if p.direction.x > 0.0 => "Camp  >",
            Ok(_) => "<  Camp",
            Err(_) => "Camp",
        };
        if text.0 != want {
            text.0 = want.into();
        }
    }
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

fn capture(
    capture: Option<ResMut<Capture>>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
) {
    let Some(mut c) = capture else {
        return;
    };
    c.frame += 1;
    if c.frame > WARM_FRAMES && c.frame <= c.at {
        c.total_ms += time.delta_secs_f64() * 1000.0;
        c.counted += 1;
    }
    if c.frame == c.at {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(c.shot.clone()));
        let mean = if c.counted > 0 {
            c.total_ms / f64::from(c.counted)
        } else {
            0.0
        };
        println!("gooey: frames {} mean_ms {mean:.3}", c.at);
    }
    if let Some(dir) = c.dir.clone() {
        let every = c.every.max(1);
        if c.frame >= c.from && (c.frame - c.from) % every == 0 {
            c.kept += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{dir}/f_{:04}.png", c.kept)));
        }
        if c.frame == c.at {
            println!("gooey: kept {} frames in {dir}", c.kept);
        }
    }
    if c.frame == c.at + 10 {
        exit.write(AppExit::Success);
    }
}
