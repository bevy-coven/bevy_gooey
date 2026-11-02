//! A settings form with the three small widgets a form needs: icons in buttons, a problem said
//! under the field it belongs to, and honest feedback for work that takes a while.
//!
//! - The **Name** row has a `FieldProblem::error` while it is empty; type a name and the message
//!   goes. The **Chunks** row has a `FieldProblem::warning` above 8. The game inserts and removes
//!   the component; the crate draws the words under the row, with a bar and the field's frame in
//!   the kind's colour, and the rows below make room.
//! - **Add**, **Delete** and **Save** hold an `icon`: one cell of an atlas the example draws in
//!   code (a game ships its own white-on-transparent art). Delete's is `IconTone::Danger`, the
//!   others follow the button's state like its text.
//! - The bar is a `progress` (3 of 7 steps, time left) for work of ten seconds and more; the ring is
//!   a `spinner` for two to ten.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `forms.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. `GOOEY_SKIN=parchment`, `GOOEY_TEXT_SCALE=<f>` and
//! `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_KEYS` stages one step every `GOOEY_KEY_EVERY`
//! (default 24) frames from frame 30, steps separated by `;`: `type:<text>` types, and `tab`,
//! `down`, `up`, `enter`, `backspace` are real keys. `GOOEY_PROGRESS=<f>` holds the bar at a
//! fraction instead of running it (the stills).

use bevy::asset::RenderAssetUsages;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowResolution};
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
    step_every: u32,
    steps: Vec<Step>,
}

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Debug)]
enum Step {
    Key(KeyCode, Key),
    Type(String),
}

/// The frame `GOOEY_KEYS` starts at, after the layout has settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;
/// Seconds the example's bake takes, start to end.
const BAKE_SECS: f32 = 12.0;
/// The side of one cell of the icon atlas, px.
const CELL: u32 = 32;

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
        title: "bevy_gooey forms".into(),
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
    .insert_resource(skin)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            follow_background,
            validate,
            run_bake,
            capture,
            stage.after(capture),
        ),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "forms.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(24),
            steps: steps_of(&std::env::var("GOOEY_KEYS").unwrap_or_default()),
        });
    }
    app.run();
}

fn steps_of(text: &str) -> Vec<Step> {
    text.split(';')
        .filter_map(|word| {
            let word = word.trim();
            if let Some(typed) = word.strip_prefix("type:") {
                return Some(Step::Type(typed.into()));
            }
            Some(match word {
                "tab" => Step::Key(KeyCode::Tab, Key::Tab),
                "up" => Step::Key(KeyCode::ArrowUp, Key::ArrowUp),
                "down" => Step::Key(KeyCode::ArrowDown, Key::ArrowDown),
                "enter" => Step::Key(KeyCode::Enter, Key::Enter),
                "backspace" => Step::Key(KeyCode::Backspace, Key::Backspace),
                _ => return None,
            })
        })
        .collect()
}

/// The row the game checks for a name.
#[derive(Component)]
struct NameRow;

/// The row the game checks for a number of chunks.
#[derive(Component)]
struct ChunksRow;

/// The progress bar the example runs.
#[derive(Component)]
struct Bake;

/// A white-on-transparent atlas of four glyphs, drawn with distance fields: plus, bin, check,
/// cross. A game ships its own art; the skin tints whatever is white.
fn atlas() -> Image {
    let segments: [&[([f32; 2], [f32; 2])]; 4] = [
        &[([16.0, 6.0], [16.0, 26.0]), ([6.0, 16.0], [26.0, 16.0])],
        &[
            ([6.0, 9.0], [26.0, 9.0]),
            ([13.0, 9.0], [13.0, 5.0]),
            ([13.0, 5.0], [19.0, 5.0]),
            ([19.0, 5.0], [19.0, 9.0]),
            ([9.0, 12.0], [10.5, 27.0]),
            ([10.5, 27.0], [21.5, 27.0]),
            ([21.5, 27.0], [23.0, 12.0]),
            ([14.0, 15.0], [14.0, 24.0]),
            ([18.0, 15.0], [18.0, 24.0]),
        ],
        &[([6.0, 17.0], [13.0, 24.0]), ([13.0, 24.0], [26.0, 8.0])],
        &[([8.0, 8.0], [24.0, 24.0]), ([24.0, 8.0], [8.0, 24.0])],
    ];
    let side = CELL as usize;
    let mut data = vec![0u8; side * side * 4 * segments.len()];
    for (cell, lines) in segments.iter().enumerate() {
        for y in 0..side {
            for x in 0..side {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let distance = lines
                    .iter()
                    .map(|(a, b)| {
                        let (a, b) = (Vec2::from(*a), Vec2::from(*b));
                        let ab = b - a;
                        let t = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
                        p.distance(a + ab * t)
                    })
                    .fold(f32::MAX, f32::min);
                let alpha = (1.8 - distance).clamp(0.0, 1.0);
                let at = (y * side * segments.len() + cell * side + x) * 4;
                data[at..at + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
            }
        }
    }
    Image::new(
        Extent3d {
            width: CELL * segments.len() as u32,
            height: CELL,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = images.add(atlas());
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(CELL),
        4,
        1,
        None,
        None,
    ));
    let cell = |index| IconSource::Atlas {
        image: image.clone(),
        layout: layout.clone(),
        index,
    };
    commands.spawn(Camera2d);
    // An icon is the first child of its button, before the label.
    let with_icon = |commands: &mut Commands, label: &str, source, tone| {
        let button = commands.spawn(button(label)).id();
        let icon = commands.spawn((icon(source), tone)).id();
        commands.entity(button).insert_children(0, &[icon]);
        button
    };
    let add = with_icon(&mut commands, "Add", cell(0), IconTone::Text);
    let delete = with_icon(&mut commands, "Delete", cell(1), IconTone::Danger);
    let save = with_icon(&mut commands, "Save", cell(2), IconTone::Text);
    let buttons = commands
        .spawn(Node {
            column_gap: Val::Px(12.0),
            ..default()
        })
        .add_children(&[add, delete, save])
        .id();
    let syncing = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn(spinner());
            row.spawn(label("Syncing with the server"));
        })
        .id();
    let card = commands
        .spawn((panel(), TabGroup::default()))
        .with_children(|card| {
            card.spawn(heading("New world"));
            card.spawn((text_input("Name", ""), NameRow));
            card.spawn((number_field("Chunks", 12.0, 1.0, 1.0..=16.0), ChunksRow));
        })
        .add_children(&[buttons])
        .with_children(|card| {
            card.spawn((progress(0.0), Bake));
        })
        .add_children(&[syncing])
        .id();
    let centre = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .add_child(card)
        .id();
    commands.spawn(screen()).add_child(centre);
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// The game's own rules: a name is needed, and more than 8 chunks is slow. It only inserts and
/// removes the component; drawing it is the crate's.
#[allow(clippy::type_complexity)]
fn validate(
    mut commands: Commands,
    names: Query<
        (Entity, &TextInputValue, Has<FieldProblem>),
        (With<NameRow>, Changed<TextInputValue>),
    >,
    chunks: Query<
        (Entity, &NumberValue, Has<FieldProblem>),
        (With<ChunksRow>, Changed<NumberValue>),
    >,
) {
    for (row, name, has) in &names {
        match (name.0.trim().is_empty(), has) {
            (true, _) => {
                commands
                    .entity(row)
                    .insert(FieldProblem::error("A name is needed"));
            }
            (false, true) => {
                commands.entity(row).remove::<FieldProblem>();
            }
            _ => {}
        }
    }
    for (row, chunks, has) in &chunks {
        match (chunks.0 > 8.0, has) {
            (true, _) => {
                commands.entity(row).insert(FieldProblem::warning(format!(
                    "{} chunks is slow; 8 or fewer is quick",
                    chunks.0
                )));
            }
            (false, true) => {
                commands.entity(row).remove::<FieldProblem>();
            }
            _ => {}
        }
    }
}

/// A bake that runs [`BAKE_SECS`] in seven steps and starts again, or holds at
/// `GOOEY_PROGRESS`.
fn run_bake(
    time: Res<Time<Real>>,
    mut bars: Query<(&mut Progress, &mut ProgressText), With<Bake>>,
) {
    const STEPS: [&str; 7] = [
        "Reading the map",
        "Measuring meshes",
        "Baking light",
        "Packing atlases",
        "Compressing textures",
        "Writing chunks",
        "Verifying",
    ];
    let held: Option<f32> = env("GOOEY_PROGRESS");
    let fraction = held.unwrap_or_else(|| (time.elapsed_secs() % BAKE_SECS) / BAKE_SECS);
    let step = ((fraction * 7.0) as usize).min(6);
    let left = ((1.0 - fraction) * BAKE_SECS).ceil();
    for (mut value, mut words) in &mut bars {
        if (value.0 - fraction).abs() > f32::EPSILON {
            value.0 = fraction;
        }
        let next = ProgressText {
            step: Some((step as u32 + 1, 7, STEPS[step].into())),
            eta_seconds: Some(left),
        };
        if *words != next {
            *words = next;
        }
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

/// `GOOEY_KEYS`: real keyboard input, one step every `GOOEY_KEY_EVERY` frames.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(8);
    let Some(step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let phase = since % every;
    let mut key = |key_code: KeyCode, logical_key: Key, text: Option<&str>, state: ButtonState| {
        keys.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: text.map(Into::into),
            repeat: false,
            window,
        });
    };
    match step {
        Step::Key(key_code, logical) => match phase {
            0 => key(*key_code, logical.clone(), None, ButtonState::Pressed),
            n if n == every / 2 => key(*key_code, logical.clone(), None, ButtonState::Released),
            _ => {}
        },
        Step::Type(text) => {
            // A character every four frames from the step's start.
            if phase % 4 == 0
                && let Some(c) = text.chars().nth((phase / 4) as usize)
            {
                let s = c.to_string();
                let logical = Key::Character(s.as_str().into());
                key(
                    KeyCode::KeyA,
                    logical.clone(),
                    Some(&s),
                    ButtonState::Pressed,
                );
                key(KeyCode::KeyA, logical, None, ButtonState::Released);
            }
        }
    }
}
