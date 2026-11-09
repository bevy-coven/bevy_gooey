//! A table of ten thousand rows in a dozen entities: an inventory with a header that sorts, columns
//! that size, and a cursor and a selection. Click a header to sort (again to turn it round), drag
//! a column's right edge or press Ctrl and Left or Right on a focused header, Tab to the rows and
//! walk them with the arrows, Page Up and Down, Home and End; Space selects, Enter opens a row.
//! The D-pad, the triggers and South do the same on a pad.
//!
//! Run `cargo run --release --example table`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_FIXED_CLOCK=1` makes every frame 1/30 s of game time (a deterministic recording; the
//! frame time then reads 33.333). `GOOEY_SHOT` (default `table.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start;
//! `GOOEY_MULTI=1` lets the player select any number of rows.
//! `GOOEY_KEYS=tab,down,…` stages one step every `GOOEY_KEY_EVERY` (default 10) frames from frame
//! 30: `tab`, `up`, `down`, `home`, `end`, `pageup`, `pagedown`, `space`, `enter` as real keyboard
//! input, `sort:<column>` a pointer click on that header, `row:<n>` a click on the nth shown row
//! and `drag:<column>,<dx>` a drag of that column's edge by `dx` layout px; steps separated by
//! `;` when a step has a comma in it.

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
#[derive(Clone, Copy)]
enum Step {
    Key(KeyCode),
    /// A click on this header.
    Sort(usize),
    /// A click on the nth row shown.
    Row(usize),
    /// A drag of this column's edge by this many layout px.
    Drag(usize, f32),
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

const KINDS: [&str; 6] = ["Weapon", "Armour", "Potion", "Tool", "Scroll", "Gem"];
const MAKES: [&str; 8] = [
    "Iron", "Oak", "Silver", "Ember", "Frost", "Hollow", "Gilded", "Worn",
];
const THINGS: [&str; 9] = [
    "Sword", "Shield", "Flask", "Lantern", "Map", "Ring", "Axe", "Cloak", "Key",
];

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

/// Ten thousand items, the same ones every run.
fn inventory() -> Vec<Vec<String>> {
    let mut seed = 0x2545_F491_u64;
    let mut next = move |range: u64| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % range
    };
    (0..10_000)
        .map(|i| {
            let make = MAKES[next(MAKES.len() as u64) as usize];
            let thing = THINGS[next(THINGS.len() as u64) as usize];
            let kind = KINDS[next(KINDS.len() as u64) as usize];
            vec![
                format!("{make} {thing} {}", i + 1),
                kind.to_string(),
                format!("{}", 1 + next(99)),
                format!("{}", 5 + next(2400)),
            ]
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
        title: "bevy_gooey table".into(),
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
        // GOOEY_FIXED_CLOCK=1: every frame is 1/30 s of game time, whatever the host's load, so a
        // recording of an easing interface is the same each time (the frame time then reads 33.333).
        if std::env::var_os("GOOEY_FIXED_CLOCK").is_some() {
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_micros(33_333),
            ));
        }
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "table.png".into()),
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
            if let Some(column) = word.strip_prefix("sort:") {
                return Some(Step::Sort(column.parse().ok()?));
            }
            if let Some(n) = word.strip_prefix("row:") {
                return Some(Step::Row(n.parse().ok()?));
            }
            if let Some(by) = word.strip_prefix("drag:") {
                let (column, dx) = by.split_once(',')?;
                return Some(Step::Drag(
                    column.trim().parse().ok()?,
                    dx.trim().parse().ok()?,
                ));
            }
            Some(Step::Key(match word {
                "tab" => KeyCode::Tab,
                "up" => KeyCode::ArrowUp,
                "down" => KeyCode::ArrowDown,
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

/// What the table says it holds, from the state the game reads.
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
                heading("Inventory"),
                (
                    table(
                        [
                            Column::new("Name").width(15.0),
                            Column::new("Kind").width(7.0),
                            Column::new("Qty").width(5.0),
                            Column::new("Value").width(7.0),
                        ],
                        inventory(),
                    ),
                    mode,
                    TableRows(9.5),
                ),
                (label("10,000 items"), Readout),
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
    tables: Query<(&Table, &TableSort, &TableSelection)>,
    mut texts: Query<&mut Text, With<Readout>>,
) {
    let Ok((table, sort, selection)) = tables.single() else {
        return;
    };
    let by = sort.column.map_or(String::new(), |c| {
        format!(
            ", by {} {}",
            table.columns()[c].title,
            if sort.descending { "down" } else { "up" }
        )
    });
    let line = format!("{} selected of {}{by}", selection.len(), table.len());
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
        // A fixed clock pins `Time<Real>`'s delta: the mean is a constant, not a measurement
        // (`docs/research/perf-gate.md`).
        let note = if std::env::var_os("GOOEY_FIXED_CLOCK").is_some() {
            " (fixed clock: not a measurement)"
        } else {
            ""
        };
        // The host's load beside the number: iteration 86 measured 30.7 ms at load 25-46 and 22.4 at
        // load 1.4 for the same interface, so a frame time without its load cannot be compared.
        let load = std::fs::read_to_string("/proc/loadavg")
            .ok()
            .and_then(|s| s.split_whitespace().next().map(str::to_owned))
            .map(|one| format!(" load {one}"))
            .unwrap_or_default();
        println!("gooey: frames {} mean_ms {mean:.3}{note}{load}", c.at);
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
    ui_scale: Res<UiScale>,
    parts: Query<(&Role, &ComputedNode, &UiGlobalTransform)>,
    rows: Query<(&TableRow, &ComputedNode, &UiGlobalTransform)>,
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
    let Some(&step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let phase = since % every;
    let centre_of = |role: Role, nth: usize| {
        let mut found: Vec<Vec2> = parts
            .iter()
            .filter(|(r, ..)| **r == role)
            .map(|(_, _, at)| at.translation / props.scale_factor())
            .collect();
        found.sort_by(|a, b| a.x.total_cmp(&b.x));
        found.get(nth).copied()
    };
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
    match step {
        Step::Key(key_code) => {
            let state = match phase {
                0 => ButtonState::Pressed,
                n if n == every / 2 => ButtonState::Released,
                _ => return,
            };
            keys.write(KeyboardInput {
                key_code,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window,
            });
        }
        Step::Sort(column) => match phase {
            0 => {
                if let Some(at) = centre_of(Role::TableHeaderCell, column) {
                    *grabbed = at;
                    send(at, PointerAction::Move { delta: Vec2::ZERO });
                }
            }
            2 => send(*grabbed, PointerAction::Press(PointerButton::Primary)),
            3 => send(*grabbed, PointerAction::Release(PointerButton::Primary)),
            _ => {}
        },
        Step::Row(n) => {
            let mut shown: Vec<(usize, Vec2)> = rows
                .iter()
                .map(|(r, _, at)| (r.index, at.translation / props.scale_factor()))
                .collect();
            shown.sort_by_key(|(i, _)| *i);
            match phase {
                0 => {
                    if let Some(&(_, at)) = shown.get(n) {
                        *grabbed = at;
                        send(at, PointerAction::Move { delta: Vec2::ZERO });
                    }
                }
                2 => send(*grabbed, PointerAction::Press(PointerButton::Primary)),
                3 => send(*grabbed, PointerAction::Release(PointerButton::Primary)),
                _ => {}
            }
        }
        Step::Drag(column, by) => {
            let last = every - 2;
            let by = Vec2::new(by, 0.0) * ui_scale.0;
            let (position, action) = match phase {
                0 => {
                    let Some(at) = centre_of(Role::TableEdge, column) else {
                        return;
                    };
                    *grabbed = at;
                    (at, PointerAction::Move { delta: Vec2::ZERO })
                }
                1 => (*grabbed, PointerAction::Press(PointerButton::Primary)),
                n if n < last => {
                    let t = (n - 1) as f32 / (last - 1) as f32;
                    let eased = t * t * (3.0 - 2.0 * t);
                    (
                        *grabbed + by * eased,
                        PointerAction::Move {
                            delta: by / last as f32,
                        },
                    )
                }
                n if n == last => (
                    *grabbed + by,
                    PointerAction::Release(PointerButton::Primary),
                ),
                _ => return,
            };
            send(position, action);
        }
    }
    let _ = ui_scale;
}
