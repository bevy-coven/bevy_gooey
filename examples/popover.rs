//! Popovers: a menu bar's File and View open a modal menu under their button (focus moves in, the
//! arrows stay inside, Escape closes it and focus goes back); Edit is pinned to the window's right
//! edge, so its menu slides to end at the button's edge; a right-click on the canvas opens a
//! context menu at the pointer, and near the corner it flips up and to the left of it. A press
//! anywhere else closes a menu.
//!
//! Run `cargo run --release --example popover`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `popover.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=enter,down,escape,…` stages one step every `GOOEY_KEY_EVERY` (default 10) frames
//! from frame 30: `up`, `down`, `left`, `right`, `enter`, `escape` as real keyboard input,
//! `context` (a right-click at `GOOEY_POINT=<x>,<y>`, default 1180,640) and `away` (a primary click
//! on the empty ground at the left).

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::AutoFocus;
use bevy::picking::events::PointerPress;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy_gooey::*;

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Copy)]
enum Step {
    Key(KeyCode),
    /// A right-click at the capture point.
    Context,
    /// A primary click on the empty ground.
    Away,
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
    point: Vec2,
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
        title: "bevy_gooey popover".into(),
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
        let point = std::env::var("GOOEY_POINT")
            .ok()
            .and_then(|s| s.split_once(',').map(|(x, y)| (x.parse(), y.parse())))
            .and_then(|(x, y)| Some(Vec2::new(x.ok()?, y.ok()?)))
            .unwrap_or(Vec2::new(1180.0, 640.0));
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "popover.png".into()),
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
                    "up" => Some(Step::Key(KeyCode::ArrowUp)),
                    "down" => Some(Step::Key(KeyCode::ArrowDown)),
                    "left" => Some(Step::Key(KeyCode::ArrowLeft)),
                    "right" => Some(Step::Key(KeyCode::ArrowRight)),
                    "enter" => Some(Step::Key(KeyCode::Enter)),
                    "escape" => Some(Step::Key(KeyCode::Escape)),
                    "context" => Some(Step::Context),
                    "away" => Some(Step::Away),
                    _ => None,
                })
                .collect(),
            point,
        });
    }
    app.run();
}

/// The menus: each is the items under one anchor.
const FILE: &[&str] = &["New", "Open...", "Save", "Quit"];
const VIEW: &[&str] = &["Zoom in", "Zoom out", "Full screen"];
const EDIT: &[&str] = &["Undo", "Redo", "Cut", "Copy", "Paste"];
const CONTEXT: &[&str] = &["Inspect", "Duplicate", "Delete"];

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    // The menu bar: File and View on the left, Edit pinned to the right edge.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(16.0),
            column_gap: Val::Px(8.0),
            ..default()
        },
        children![
            (
                button("File"),
                AutoFocus,
                observe(
                    |activate: On<Activate>,
                     panels: Query<(Entity, &PopoverPanel)>,
                     mut commands: Commands| {
                        toggle(
                            &mut commands,
                            &panels,
                            activate.entity,
                            Placement::Below,
                            FILE,
                        );
                    }
                ),
            ),
            (
                button("View"),
                observe(
                    |activate: On<Activate>,
                     panels: Query<(Entity, &PopoverPanel)>,
                     mut commands: Commands| {
                        toggle(
                            &mut commands,
                            &panels,
                            activate.entity,
                            Placement::Below,
                            VIEW,
                        );
                    }
                ),
            ),
        ],
    ));
    let edit = commands
        .spawn((
            button("Edit"),
            observe(
                |activate: On<Activate>,
                 panels: Query<(Entity, &PopoverPanel)>,
                 mut commands: Commands| {
                    toggle(
                        &mut commands,
                        &panels,
                        activate.entity,
                        Placement::Below,
                        EDIT,
                    );
                },
            ),
        ))
        .id();
    commands
        .entity(edit)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.top = Val::Px(16.0);
            node.right = Val::Px(16.0);
        });
    // The canvas: a right-click opens a context menu at the pointer.
    commands.spawn((
        Role::Surface(1),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(96.0),
            left: Val::Px(16.0),
            right: Val::Px(16.0),
            bottom: Val::Px(16.0),
            padding: UiRect::all(Val::Px(24.0)),
            ..default()
        },
        observe(
            |press: On<PointerPress>,
             mut commands: Commands,
             panels: Query<(Entity, &PopoverPanel)>| {
                if press.button == PointerButton::Secondary {
                    // A new menu replaces an open one.
                    for (entity, panel) in &panels {
                        if panel.placement == Placement::AtPointer {
                            commands.entity(entity).despawn();
                        }
                    }
                    open(&mut commands, press.entity, Placement::AtPointer, CONTEXT);
                }
            },
        ),
        children![label("Right-click the canvas for a context menu")],
    ));
}

/// Opens `items` as a menu on `anchor`, or closes the one that is open there: the anchor's own
/// `Activate` is the toggle, and a press on the anchor is never "outside".
fn toggle(
    commands: &mut Commands,
    panels: &Query<(Entity, &PopoverPanel)>,
    anchor: Entity,
    placement: Placement,
    items: &'static [&'static str],
) {
    if let Some((open, _)) = panels.iter().find(|(_, panel)| panel.anchor == anchor) {
        commands.entity(open).despawn();
    } else {
        open(commands, anchor, placement, items);
    }
}

/// A menu: a modal popover of buttons. Choosing one closes the menu.
fn open(
    commands: &mut Commands,
    anchor: Entity,
    placement: Placement,
    items: &'static [&'static str],
) {
    commands
        .spawn(popover_modal(anchor, placement))
        .with_children(|menu| {
            for &item in items {
                menu.spawn((
                    button(item),
                    observe(
                        |activate: On<Activate>,
                         parents: Query<&ChildOf>,
                         panels: Query<(), With<PopoverPanel>>,
                         mut commands: Commands| {
                            if let Some(panel) = parents
                                .iter_ancestors(activate.entity)
                                .find(|&a| panels.contains(a))
                            {
                                commands.entity(panel).despawn();
                            }
                        },
                    ),
                ));
            }
        });
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
/// the window's own keyboard input; a click moves the pointer first, then presses, then releases,
/// as the window's own pointer input.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(4);
    let Some(&step) = c.steps.get((since / every) as usize) else {
        return;
    };
    let phase = since % every;
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
        Step::Context | Step::Away => {
            let (at, button) = match step {
                Step::Context => (c.point, PointerButton::Secondary),
                _ => (Vec2::new(8.0, 400.0), PointerButton::Primary),
            };
            let action = match phase {
                0 => PointerAction::Move { delta: Vec2::ZERO },
                n if n == every / 2 => PointerAction::Press(button),
                n if n == every / 2 + 1 => PointerAction::Release(button),
                _ => return,
            };
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            pointer.write(PointerInput::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::Window(target),
                    position: at,
                },
                action,
            ));
        }
    }
}
