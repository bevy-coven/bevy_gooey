//! A tree of ten thousand nodes in a dozen entities: an outliner of a game world, regions of areas
//! of rooms of props. Click a chevron or press Right to open a node and Left to close it (Left on
//! a closed node goes to its parent), Tab to the tree and walk it with the arrows, Home and End;
//! type to go to the next node that starts with it, `*` opens every sibling, Space selects, Enter
//! opens a row. The D-pad and South do the same on a pad.
//!
//! Run `cargo run --release --example tree`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `tree.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start;
//! `GOOEY_MULTI=1` lets the player select any number of nodes.
//! `GOOEY_KEYS=tab,down,…` stages one step every `GOOEY_KEY_EVERY` (default 10) frames from frame
//! 30: `tab`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`, `space`, `enter`
//! and `star` as real keyboard input, `type:<text>` the characters one by one, `row:<n>` a click on
//! the nth shown row's label and `chevron:<n>` a click on its chevron; steps separated by `;` when
//! a step has a colon in it.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy_gooey::*;

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone)]
enum Step {
    Key(KeyCode),
    /// `*`.
    Star,
    /// These characters, one after another.
    Type(String),
    /// A click on the label of the nth row shown.
    Row(usize),
    /// A click on the chevron of the nth row shown.
    Chevron(usize),
}

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

/// The frame `GOOEY_KEYS` starts at, after the layout has settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;

const REGIONS: [&str; 10] = [
    "Ashfall",
    "Brindle",
    "Cinder Reach",
    "Dunmoor",
    "Emberglen",
    "Frostwatch",
    "Gallow",
    "Hollowmere",
    "Ironvale",
    "Jade Coast",
];
const AREAS: [&str; 10] = [
    "Market", "Docks", "Keep", "Chapel", "Mine", "Barrow", "Orchard", "Tavern", "Ruins", "Gate",
];
const ROOMS: [&str; 10] = [
    "Cellar", "Hall", "Armoury", "Study", "Kitchen", "Vault", "Loft", "Cell", "Garden", "Crypt",
];
const PROPS: [&str; 9] = [
    "Barrel", "Crate", "Lantern", "Chest", "Table", "Banner", "Anvil", "Bookcase", "Brazier",
];

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

/// Ten thousand one hundred and ten nodes, the same ones every run. The first region and its
/// first two areas start open.
fn world() -> Vec<TreeNode> {
    let mut key = 0;
    let mut next = || {
        key += 1;
        key
    };
    REGIONS
        .iter()
        .enumerate()
        .map(|(r, region)| {
            let mut node = TreeNode::new(next(), *region).children(AREAS.iter().enumerate().map(
                |(a, area)| {
                    let mut node = TreeNode::new(next(), format!("{region} {area}")).children(
                        ROOMS.iter().map(|room| {
                            TreeNode::new(next(), format!("{area} {room}"))
                                .children(PROPS.iter().map(|prop| TreeNode::new(next(), *prop)))
                        }),
                    );
                    if r == 0 && a < 2 {
                        node = node.expanded();
                    }
                    node
                },
            ));
            if r == 0 {
                node = node.expanded();
            }
            node
        })
        .collect()
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
        title: "bevy_gooey tree".into(),
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
        (follow_background, readout, capture, stage.after(capture)),
    );
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "tree.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(10),
            steps: steps_of(&std::env::var("GOOEY_KEYS").unwrap_or_default()),
        });
    }
    app.run();
}

fn steps_of(text: &str) -> Vec<Step> {
    let words: Vec<&str> = if text.contains(';') || text.contains(':') {
        text.split(';').collect()
    } else {
        text.split(',').collect()
    };
    words
        .into_iter()
        .filter_map(|word| {
            let word = word.trim();
            if let Some(typed) = word.strip_prefix("type:") {
                return Some(Step::Type(typed.to_string()));
            }
            if let Some(n) = word.strip_prefix("row:") {
                return Some(Step::Row(n.parse().ok()?));
            }
            if let Some(n) = word.strip_prefix("chevron:") {
                return Some(Step::Chevron(n.parse().ok()?));
            }
            Some(Step::Key(match word {
                "star" => return Some(Step::Star),
                "tab" => KeyCode::Tab,
                "up" => KeyCode::ArrowUp,
                "down" => KeyCode::ArrowDown,
                "left" => KeyCode::ArrowLeft,
                "right" => KeyCode::ArrowRight,
                "home" => KeyCode::Home,
                "end" => KeyCode::End,
                "pageup" => KeyCode::PageUp,
                "pagedown" => KeyCode::PageDown,
                "space" => KeyCode::Space,
                "enter" => KeyCode::Enter,
                _ => return None,
            }))
        })
        .collect()
}

/// What the tree says it holds, from the state the game reads.
#[derive(Component)]
struct Readout;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let mode = if std::env::var("GOOEY_MULTI").is_ok() {
        SelectMode::Multi
    } else {
        SelectMode::Single
    };
    commands.spawn((
        screen(),
        children![(
            panel(),
            children![
                heading("World"),
                (tree(world()), mode, TableRows(9.5)),
                (label("10,110 nodes"), Readout),
            ],
        )],
    ));
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

fn readout(
    trees: Query<(&Table, &TableSelection, &TableCursor)>,
    mut texts: Query<&mut Text, With<Readout>>,
) {
    let Ok((table, selection, cursor)) = trees.single() else {
        return;
    };
    let Some(tree) = table.source::<Tree>() else {
        return;
    };
    let at = cursor
        .0
        .and_then(|key| tree.label(key))
        .map_or(String::new(), |name| format!(", at {name}"));
    let line = format!(
        "{} of {} nodes shown, {} selected{at}",
        table.len(),
        tree.node_count(),
        selection.len(),
    );
    for mut text in &mut texts {
        if text.0 != line {
            text.0 = line.clone();
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

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames, as the window's own keyboard and pointer
/// input.
#[allow(clippy::too_many_arguments)]
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<(Entity, &Window), With<PrimaryWindow>>,
    rows: Query<(&TableRow, &Children)>,
    cells: Query<&Children>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
    mut grabbed: Local<Vec2>,
) {
    let (Some(c), Ok((window, props))) = (capture, window.single()) else {
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
    let mut send = |position: Vec2, action: PointerAction| {
        if let Some(target) = WindowRef::Entity(window).normalize(None) {
            pointer.write(PointerInput::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::Window(target),
                    position,
                },
                action,
            ));
        }
    };
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
    let press_phase = |phase: u32| match phase {
        0 => Some(ButtonState::Pressed),
        n if n == every / 2 => Some(ButtonState::Released),
        _ => None,
    };
    match step {
        Step::Key(key_code) => {
            if let Some(state) = press_phase(phase) {
                key(
                    *key_code,
                    Key::Unidentified(NativeKey::Unidentified),
                    None,
                    state,
                );
            }
        }
        Step::Star => {
            if let Some(state) = press_phase(phase) {
                key(
                    KeyCode::Digit8,
                    Key::Character("*".into()),
                    Some("*"),
                    state,
                );
            }
        }
        Step::Type(typed) => {
            // A character every four frames: a press and, two frames later, its release.
            let at = (phase / 4) as usize;
            if let Some(ch) = typed.chars().nth(at) {
                let s = ch.to_string();
                let state = match phase % 4 {
                    0 => ButtonState::Pressed,
                    2 => ButtonState::Released,
                    _ => return,
                };
                key(
                    KeyCode::KeyA,
                    Key::Character(s.as_str().into()),
                    Some(s.as_str()),
                    state,
                );
            }
        }
        Step::Row(n) | Step::Chevron(n) => {
            let mut shown: Vec<(usize, Vec2)> = rows
                .iter()
                .filter_map(|(row, parts)| {
                    let cell = cells.get(*parts.first()?).ok()?;
                    // A row is its cell, which holds the chevron box and then the text.
                    let part = if matches!(step, Step::Chevron(_)) {
                        *cell.first()?
                    } else {
                        *cell.last()?
                    };
                    let (_, at) = nodes.get(part).ok()?;
                    Some((row.index, at.translation / props.scale_factor()))
                })
                .collect();
            shown.sort_by_key(|(i, _)| *i);
            match phase {
                0 => {
                    if let Some(&(_, at)) = shown.get(*n) {
                        *grabbed = at;
                        send(at, PointerAction::Move { delta: Vec2::ZERO });
                    }
                }
                2 => send(*grabbed, PointerAction::Press(PointerButton::Primary)),
                3 => send(*grabbed, PointerAction::Release(PointerButton::Primary)),
                _ => {}
            }
        }
    }
}
