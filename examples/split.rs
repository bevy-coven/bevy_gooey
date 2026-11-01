//! Split panes: a sidebar beside a viewport with a log under it, two dividers. Drag a divider, or
//! Tab to one and use the arrows (along its axis), Home, End and Enter; the D-pad and South do the
//! same on a pad, and a double click collapses the sidebar to its minimum and gives it back. The
//! sidebar keeps 240 px and the viewport 360 px however small the window gets.
//!
//! Run `cargo run --release --example split`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `split.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS=tab,right,…` stages one step every `GOOEY_KEY_EVERY` (default 10) frames from frame
//! 30: `tab`, `up`, `down`, `left`, `right`, `home`, `end`, `enter` as real keyboard input, and a
//! drag of a divider with the pointer, spread over the step's frames: `drag:<dx>,<dy>` for the
//! sidebar's divider and `dragv:<dx>,<dy>` for the log's, in layout px; with a drag in it the
//! steps are separated by `;`, as in `GOOEY_KEYS='drag:120,0;tab;right'`.

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
    /// A drag of the divider of this axis by this much, in layout px.
    Drag(SplitAxis, Vec2),
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

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn pair(text: &str) -> Option<Vec2> {
    let (x, y) = text.split_once(',')?;
    Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
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
        title: "bevy_gooey split".into(),
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
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "split.png".into()),
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

/// `GOOEY_KEYS`: words separated by `;` or `,`, a drag's `x,y` taking the comma, so a drag is
/// written `drag:120,0` and the steps are split on `;` when there is one.
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
            if let Some(by) = word.strip_prefix("drag:") {
                return Some(Step::Drag(SplitAxis::Horizontal, pair(by)?));
            }
            if let Some(by) = word.strip_prefix("dragv:") {
                return Some(Step::Drag(SplitAxis::Vertical, pair(by)?));
            }
            Some(Step::Key(match word {
                "tab" => KeyCode::Tab,
                "up" => KeyCode::ArrowUp,
                "down" => KeyCode::ArrowDown,
                "left" => KeyCode::ArrowLeft,
                "right" => KeyCode::ArrowRight,
                "home" => KeyCode::Home,
                "end" => KeyCode::End,
                "enter" => KeyCode::Enter,
                _ => return None,
            }))
        })
        .collect()
}

/// The sidebar and the viewport's share, as the window shows it.
#[derive(Component)]
struct Readout;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            children![(
                split(
                    SplitAxis::Horizontal,
                    (
                        panel(),
                        children![
                            heading("Library"),
                            (
                                menu(),
                                children![
                                    button("Meadow, 12:40"),
                                    button("Harbour, 09:15"),
                                    button("Mill, 03:02"),
                                    button("Cellar, 01:48"),
                                ]
                            ),
                            toggle("Autosave", true),
                        ],
                    ),
                    (
                        split(
                            SplitAxis::Vertical,
                            (
                                panel(),
                                children![
                                    heading("Viewport"),
                                    (label("Sidebar and viewport"), Readout),
                                    slider("Zoom", 0.6, 0.0..=1.0),
                                ],
                            ),
                            (
                                panel(),
                                children![
                                    heading("Log"),
                                    label("12:40  Loaded Meadow"),
                                    label("12:41  Autosaved"),
                                    label("12:44  Saved Harbour"),
                                ],
                            ),
                        ),
                        SplitFraction(0.6),
                        SplitMin::new(140.0, 100.0),
                    ),
                ),
                SplitFraction(0.28),
                SplitMin::new(240.0, 360.0),
            )],
        )],
    ));
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// Says where the dividers are, from the state the game reads, never from the layout.
fn readout(splits: Query<(&Split, &SplitFraction)>, mut texts: Query<&mut Text, With<Readout>>) {
    let share = |axis| {
        splits
            .iter()
            .find(|(split, _)| split.axis() == axis)
            .map_or(0.0, |(_, fraction)| fraction.0 * 100.0)
    };
    let line = format!(
        "Sidebar {:.0}%,  viewport {:.0}% of its column",
        share(SplitAxis::Horizontal),
        share(SplitAxis::Vertical)
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

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames. A key is pressed and then released as
/// the window's own keyboard input. A drag moves the pointer onto the divider, presses, moves a
/// little each frame to `by` and releases, as the window's own pointer input.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<(Entity, &Window), With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    dividers: Query<(&SplitDivider, &ComputedNode, &UiGlobalTransform)>,
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
        Step::Drag(axis, by) => {
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let last = every - 2;
            let (position, action) = match phase {
                0 => {
                    // Onto the divider, where it stands now.
                    let Some((_, _, at)) = dividers.iter().find(|(d, ..)| d.axis() == axis) else {
                        return;
                    };
                    // Held a little off the middle: the other divider crosses the middle, and
                    // the pointer, a frame ahead of the one it drags, would light it in passing.
                    let off = match axis {
                        SplitAxis::Horizontal => Vec2::new(0.0, -240.0),
                        SplitAxis::Vertical => Vec2::new(-300.0, 0.0),
                    };
                    *grabbed = at.translation / props.scale_factor() + off * ui_scale.0;
                    (*grabbed, PointerAction::Move { delta: Vec2::ZERO })
                }
                1 => (*grabbed, PointerAction::Press(PointerButton::Primary)),
                n if n < last => {
                    // Eased along the way, a window's logical px.
                    let t = (n - 1) as f32 / (last - 1) as f32;
                    let eased = t * t * (3.0 - 2.0 * t);
                    let to = *grabbed + by * ui_scale.0 * eased;
                    (
                        to,
                        PointerAction::Move {
                            delta: by * ui_scale.0 / last as f32,
                        },
                    )
                }
                n if n == last => (
                    *grabbed + by * ui_scale.0,
                    PointerAction::Release(PointerButton::Primary),
                ),
                _ => return,
            };
            pointer.write(PointerInput::new(
                PointerId::Mouse,
                Location {
                    target: NormalizedRenderTarget::Window(target),
                    position,
                },
                action,
            ));
        }
    }
}
