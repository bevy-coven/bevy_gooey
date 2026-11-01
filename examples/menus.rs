//! Menu bar: File, Edit, View and Help across the top. F10 (or Alt, tapped) puts focus on the bar,
//! Left and Right walk it, Down opens a menu with focus on its first item, Right opens a submenu
//! (Recent, Panels), Left closes it, Escape peels one level, and Enter chooses. Redo is disabled
//! and says why when focus or the pointer is on it; View has checks and a radio run; Help's
//! submenu is empty.
//!
//! Run `cargo run --release --example menus`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `menus.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=f10,right,down,enter,…` stages one step every `GOOEY_KEY_EVERY` (default 10) frames
//! from frame 30: `up`, `down`, `left`, `right`, `enter`, `space`, `escape`, `home`, `end`, `f10` and
//! `alt` (a tap) as real keyboard input.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy_gooey::*;

/// A staged step of `GOOEY_KEYS`: a key pressed and let go.
type Step = KeyCode;

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

/// The frame `GOOEY_KEYS` starts at, after the layout and autofocus have settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;

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
        title: "bevy_gooey menus".into(),
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
    .add_systems(Update, (follow_background, capture, stage.after(capture)));
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "menus.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(10),
            steps: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|k| match k.trim() {
                    "up" => Some(KeyCode::ArrowUp),
                    "down" => Some(KeyCode::ArrowDown),
                    "left" => Some(KeyCode::ArrowLeft),
                    "right" => Some(KeyCode::ArrowRight),
                    "enter" => Some(KeyCode::Enter),
                    "space" => Some(KeyCode::Space),
                    "escape" => Some(KeyCode::Escape),
                    "home" => Some(KeyCode::Home),
                    "end" => Some(KeyCode::End),
                    "f10" => Some(KeyCode::F10),
                    "alt" => Some(KeyCode::AltLeft),
                    _ => None,
                })
                .collect(),
        });
    }
    app.run();
}

/// The line the screen reports the last command on.
#[derive(Component)]
struct Last;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let command = |name: &'static str| {
        observe(
            move |_: On<Activate>, mut last: Single<&mut Text, With<Last>>| {
                last.0 = format!("Last command: {name}");
            },
        )
    };
    let toggled = |name: &'static str| {
        observe(
            move |change: On<ValueChange<bool>>, mut last: Single<&mut Text, With<Last>>| {
                last.0 = format!("{name}: {}", if change.value { "on" } else { "off" });
            },
        )
    };
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        Pickable::IGNORE,
        children![
            menu_bar(children![
                menu_title(
                    "File",
                    children![
                        (
                            menu_item("New").with_shortcut(Shortcut::primary(KeyCode::KeyN)),
                            command("New")
                        ),
                        (
                            menu_item("Open...").with_shortcut(Shortcut::primary(KeyCode::KeyO)),
                            command("Open")
                        ),
                        (
                            menu_item("Save").with_shortcut(Shortcut::primary(KeyCode::KeyS)),
                            command("Save")
                        ),
                        separator(),
                        submenu(
                            "Recent",
                            children![
                                (menu_item("level-1.ron"), command("level-1.ron")),
                                (menu_item("level-2.ron"), command("level-2.ron")),
                                (menu_item("level-3.ron"), command("level-3.ron")),
                            ]
                        ),
                        separator(),
                        (
                            menu_item("Quit").with_shortcut(Shortcut::primary(KeyCode::KeyQ)),
                            command("Quit")
                        ),
                    ]
                ),
                menu_title(
                    "Edit",
                    children![
                        (
                            menu_item("Undo").with_shortcut(Shortcut::primary(KeyCode::KeyZ)),
                            command("Undo")
                        ),
                        (
                            menu_item("Redo").with_shortcut(Shortcut::new(
                                Modifiers::PRIMARY | Modifiers::SHIFT,
                                KeyCode::KeyZ
                            )),
                            Disabled::new("Nothing to redo"),
                            command("Redo")
                        ),
                        separator(),
                        (
                            menu_item("Cut").with_shortcut(Shortcut::primary(KeyCode::KeyX)),
                            command("Cut")
                        ),
                        (
                            menu_item("Copy").with_shortcut(Shortcut::primary(KeyCode::KeyC)),
                            command("Copy")
                        ),
                        (
                            menu_item("Paste").with_shortcut(Shortcut::primary(KeyCode::KeyV)),
                            command("Paste")
                        ),
                    ]
                ),
                menu_title(
                    "View",
                    children![
                        (check_item("Grid", true), toggled("Grid")),
                        (check_item("Snap to grid", false), toggled("Snap to grid")),
                        separator(),
                        (radio_item("Small", false), toggled("Small")),
                        (radio_item("Medium", true), toggled("Medium")),
                        (radio_item("Large", false), toggled("Large")),
                        separator(),
                        submenu(
                            "Panels",
                            children![
                                (check_item("Inspector", true), toggled("Inspector")),
                                (check_item("Console", false), toggled("Console")),
                            ]
                        ),
                    ]
                ),
                menu_title(
                    "Help",
                    children![
                        (menu_item("About"), command("About")),
                        submenu("Tips", children![]),
                    ]
                ),
            ]),
            (
                Node {
                    margin: UiRect::all(Val::Px(24.0)),
                    align_self: AlignSelf::FlexStart,
                    ..default()
                },
                children![(panel(), children![(label("Last command: none"), Last)])],
            ),
        ],
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

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames. A key is pressed and then released as
/// the window's own keyboard input.
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
    let every = c.step_every.max(4);
    let Some(&key_code) = c.steps.get((since / every) as usize) else {
        return;
    };
    let state = match since % every {
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
