//! A hotbar: four abilities in a `slot_grid` with a `Cooldown` on each cell, the radial sweep every
//! action game's bar has. Press 1 to 4 (or click, or Enter, or South on a focused slot) to cast one; it dims, a clear wedge opens
//! clockwise from twelve o'clock, the seconds stand on a chip in the middle, and when it ends the
//! game hears `CooldownReady` (here it writes a line under the bar). A slot on a cooldown can still
//! be pressed: the game's own observer checks `Cooldown::is_ready`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `hotbar.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. Capture runs on a clock of 1/30 s a frame so it is the same
//! run every time, and casts the four abilities itself at frames 36, 48, 60 and 72.
//! `GOOEY_SKIN=parchment`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.

use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::time::TimeUpdateStrategy;
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

/// What an ability is called, how long it takes to come back and the key that casts it.
struct Ability {
    name: &'static str,
    seconds: f32,
    glyph: usize,
    key: KeyCode,
}

/// The bar, left to right.
const BAR: [Ability; 4] = [
    Ability {
        name: "Fireball",
        seconds: 6.0,
        glyph: 5,
        key: KeyCode::Digit1,
    },
    Ability {
        name: "Frost nova",
        seconds: 4.0,
        glyph: 2,
        key: KeyCode::Digit2,
    },
    Ability {
        name: "Dash",
        seconds: 2.5,
        glyph: 4,
        key: KeyCode::Digit3,
    },
    Ability {
        name: "Heal",
        seconds: 5.0,
        glyph: 1,
        key: KeyCode::Digit4,
    },
];

/// The line under the bar that says what happened.
#[derive(Component)]
struct Status;

const KEY_FROM: u32 = 30;
const WARM_FRAMES: u32 = 20;
/// The side of one glyph of the atlas.
const CELL: u32 = 48;
const GLYPHS: usize = 8;
/// Capture's casts: the frame each of the four abilities is used at.
const CAST_AT: [u32; 4] = [36, 48, 60, 72];

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn main() {
    let mut skin = match std::env::var("GOOEY_SKIN").as_deref() {
        Ok("parchment") => GooeySkin::parchment(),
        Ok("high_contrast") => GooeySkin::high_contrast(),
        Ok("tool") => GooeySkin::tool(),
        _ => GooeySkin::dark(),
    };
    if let Some(scale) = env("GOOEY_TEXT_SCALE") {
        skin.text_scale = scale;
    }
    let mut window = Window {
        title: "bevy_gooey hotbar".into(),
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
    .add_observer(on_ready)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (arm_cells, cast_by_key, follow_background, capture).chain(),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_micros(
            33_333,
        )))
        .insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "hotbar.png".into()),
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

/// A white-on-transparent atlas of eight glyphs drawn with distance fields: sword, potion, gem,
/// key, arrow, torch, coin, scroll (the hotbar uses the potion, gem, arrow and torch). A game ships its own art; the skin tints whatever is white.
fn atlas() -> Image {
    // Signed distances to the shapes, in the cell's pixels: negative inside.
    let circle = |p: Vec2, c: Vec2, r: f32| p.distance(c) - r;
    let capsule = |p: Vec2, a: Vec2, b: Vec2, r: f32| {
        let ab = b - a;
        let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        p.distance(a + ab * t) - r
    };
    let bar = |p: Vec2, c: Vec2, h: Vec2| {
        let d = (p - c).abs() - h;
        d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.0)
    };
    let diamond = |p: Vec2, c: Vec2, h: Vec2| {
        let q = (p - c).abs();
        (q.x / h.x + q.y / h.y - 1.0) * h.x.min(h.y) * 0.7
    };
    let v = Vec2::new;
    let shape = |glyph: usize, p: Vec2| -> f32 {
        match glyph {
            // Sword: a blade, a cross guard, a grip and a pommel on the diagonal.
            0 => capsule(p, v(39.0, 9.0), v(19.0, 29.0), 2.8)
                .min(capsule(p, v(13.0, 23.0), v(25.0, 35.0), 2.2))
                .min(capsule(p, v(19.0, 29.0), v(10.0, 38.0), 2.4))
                .min(circle(p, v(8.0, 40.0), 3.2)),
            // Potion: a round flask with a neck and a cork.
            1 => circle(p, v(24.0, 30.0), 12.5)
                .min(bar(p, v(24.0, 15.0), v(4.0, 6.0)))
                .min(bar(p, v(24.0, 8.5), v(5.5, 2.5))),
            // Gem: a cut stone, a table and two facets cut into it.
            2 => diamond(p, v(24.0, 25.0), v(18.0, 19.0))
                .max(-capsule(p, v(11.0, 21.0), v(37.0, 21.0), 0.9))
                .max(-capsule(p, v(15.0, 21.0), v(24.0, 41.0), 0.9))
                .max(-capsule(p, v(33.0, 21.0), v(24.0, 41.0), 0.9)),
            // Key: a ring, a shaft and two teeth.
            3 => circle(p, v(15.0, 17.0), 8.5)
                .max(-circle(p, v(15.0, 17.0), 3.6))
                .min(capsule(p, v(21.0, 23.0), v(38.0, 40.0), 2.4))
                .min(capsule(p, v(32.0, 34.0), v(37.0, 29.0), 2.0))
                .min(capsule(p, v(36.0, 38.0), v(40.0, 34.0), 2.0)),
            // Arrow: a shaft, a head and fletching.
            4 => capsule(p, v(9.0, 39.0), v(35.0, 13.0), 1.8)
                .min(diamond(p, v(37.0, 11.0), v(7.0, 7.0)))
                .min(capsule(p, v(9.0, 39.0), v(9.0, 31.0), 1.6))
                .min(capsule(p, v(9.0, 39.0), v(17.0, 39.0), 1.6)),
            // Torch: a handle and a flame.
            5 => capsule(p, v(24.0, 42.0), v(24.0, 26.0), 3.6)
                .min(circle(p, v(24.0, 17.0), 8.5).min(diamond(p, v(24.0, 11.0), v(7.0, 11.0)))),
            // Coin: a disc with a ring cut into its face.
            6 => circle(p, v(24.0, 24.0), 16.0)
                .max(-((p.distance(v(24.0, 24.0)) - 10.5).abs() - 1.3)),
            // Scroll: a sheet with two rolled ends and lines of writing.
            _ => bar(p, v(24.0, 24.0), v(13.0, 15.0))
                .max(-bar(p, v(24.0, 18.0), v(8.0, 1.5)))
                .max(-bar(p, v(24.0, 25.0), v(8.0, 1.5)))
                .max(-bar(p, v(24.0, 32.0), v(5.0, 1.5)))
                .min(capsule(p, v(9.0, 9.0), v(39.0, 9.0), 2.6))
                .min(capsule(p, v(9.0, 39.0), v(39.0, 39.0), 2.6)),
        }
    };
    let side = CELL as usize;
    let mut data = vec![0u8; side * side * 4 * GLYPHS];
    for glyph in 0..GLYPHS {
        for y in 0..side {
            for x in 0..side {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let alpha = (0.5 - shape(glyph, p)).clamp(0.0, 1.0);
                let at = (y * side * GLYPHS + glyph * side + x) * 4;
                data[at..at + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
            }
        }
    }
    Image::new(
        Extent3d {
            width: CELL * GLYPHS as u32,
            height: CELL,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// A cooldown on every cell of the bar, as the grid builds them.
fn arm_cells(mut commands: Commands, cells: Query<(Entity, &SlotRef), Added<SlotRef>>) {
    for (cell, _) in &cells {
        commands.entity(cell).insert(Cooldown::default());
    }
}

/// Uses the ability in `slot` if it is ready; says so on the status line either way.
fn cast(
    slot: usize,
    cells: &mut Query<(&SlotRef, &mut Cooldown)>,
    status: &mut Query<&mut Text, With<Status>>,
) {
    let (Some(ability), Some((_, mut cooldown))) = (
        BAR.get(slot),
        cells.iter_mut().find(|(cell, _)| cell.index == slot),
    ) else {
        return;
    };
    let line = if cooldown.is_ready() {
        cooldown.start(ability.seconds);
        format!("{} cast", ability.name)
    } else {
        format!("{} is not ready", ability.name)
    };
    for mut text in status {
        text.0.clone_from(&line);
    }
}

/// A click, Enter or South on a slot: the grid says which.
fn on_use(
    chosen: On<ValueChange<usize>>,
    mut cells: Query<(&SlotRef, &mut Cooldown)>,
    mut status: Query<&mut Text, With<Status>>,
) {
    if chosen.is_final {
        cast(chosen.value, &mut cells, &mut status);
    }
}

/// The game's sound would play here: the status line says it.
fn on_ready(
    ready: On<CooldownReady>,
    cells: Query<&SlotRef>,
    mut status: Query<&mut Text, With<Status>>,
) {
    if let Some(ability) = cells.get(ready.entity).ok().and_then(|c| BAR.get(c.index)) {
        for mut text in &mut status {
            text.0 = format!("{} ready", ability.name);
        }
    }
}

/// Keys 1 to 4, and in capture mode the staged casts.
fn cast_by_key(
    keys: Res<ButtonInput<KeyCode>>,
    capture: Option<Res<Capture>>,
    mut cells: Query<(&SlotRef, &mut Cooldown)>,
    mut status: Query<&mut Text, With<Status>>,
) {
    let staged = capture.and_then(|c| CAST_AT.iter().position(|at| *at == c.frame));
    for (slot, ability) in BAR.iter().enumerate() {
        if keys.just_pressed(ability.key) || staged == Some(slot) {
            cast(slot, &mut cells, &mut status);
        }
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = images.add(atlas());
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(CELL),
        GLYPHS as u32,
        1,
        None,
        None,
    ));
    let items = BAR.iter().map(|ability| {
        Some(
            Slot::new(ability.name)
                .icon(IconSource::Atlas {
                    image: image.clone(),
                    layout: layout.clone(),
                    index: ability.glyph,
                })
                .tooltip(ability.name),
        )
    });
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            panel(),
            children![
                heading("Hotbar"),
                label("Press 1 to 4, or click a slot."),
                (slot_grid(4, items), observe(on_use)),
                (label("Everything is ready"), Status),
            ],
        )],
    ));
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
