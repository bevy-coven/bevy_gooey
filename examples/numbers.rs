//! Number fields in an inspector: a vec field for position and scale, and number fields for health,
//! speed and mass. Press a label (or a letter) and drag left or right to scrub a number; click a
//! field and type, Enter applies it and Escape puts the old number back; Up and Down step it
//! (Shift ten steps) and the D-pad's Left and Right do on a pad. The mass is a multi-selection
//! whose members differ: the field is empty with a muted dash until the player sets it. An
//! expression such as `1.5*2` is refused with its reason, shown under the panel.
//!
//! Run `cargo run --release --example numbers`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `numbers.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS` stages one step every `GOOEY_KEY_EVERY` (default 24) frames from frame 30, steps
//! separated by `;`: `drag:<row>,<dx>` scrubs a row by its label (a row is `health`, `speed`,
//! `mass` or `position.0` to `position.2`, `scale.0` to `scale.2`) over the step's frames,
//! `click:<row>` focuses its field, `type:<text>` types, and `up`, `down`, `left`, `right`, `enter`,
//! `escape`, `tab` and `shiftup` are real keys.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy_gooey::*;

/// A staged step of `GOOEY_KEYS`.
#[derive(Clone, Debug)]
enum Step {
    Key(KeyCode, Key),
    /// A key with Shift held.
    Shifted(KeyCode, Key),
    Type(String),
    /// A press and release on the row's field.
    Click(String),
    /// A drag of the row's label by this much, in layout px.
    Drag(String, f32),
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
        title: "bevy_gooey numbers".into(),
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
    .add_systems(Update, (follow_background, capture, stage.after(capture)))
    .add_observer(on_refused)
    .add_observer(on_number)
    .add_observer(on_vec);
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "numbers.png".into()),
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
            if let Some(row) = word.strip_prefix("click:") {
                return Some(Step::Click(row.into()));
            }
            if let Some(typed) = word.strip_prefix("type:") {
                return Some(Step::Type(typed.into()));
            }
            if let Some(by) = word.strip_prefix("drag:") {
                let (row, dx) = by.split_once(',')?;
                return Some(Step::Drag(row.into(), dx.trim().parse().ok()?));
            }
            Some(match word {
                "tab" => Step::Key(KeyCode::Tab, Key::Tab),
                "up" => Step::Key(KeyCode::ArrowUp, Key::ArrowUp),
                "down" => Step::Key(KeyCode::ArrowDown, Key::ArrowDown),
                "left" => Step::Key(KeyCode::ArrowLeft, Key::ArrowLeft),
                "right" => Step::Key(KeyCode::ArrowRight, Key::ArrowRight),
                "enter" => Step::Key(KeyCode::Enter, Key::Enter),
                "escape" => Step::Key(KeyCode::Escape, Key::Escape),
                "shiftup" => Step::Shifted(KeyCode::ArrowUp, Key::ArrowUp),
                _ => return None,
            })
        })
        .collect()
}

/// What the game reads back: the numbers as the player left them, and the last refusal.
#[derive(Component)]
struct Readout;

/// What the player last typed that was refused, and why.
#[derive(Component)]
struct Refusal;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            children![(
                panel(),
                children![
                    heading("Inspector"),
                    (
                        vec_field("Position", [4.0, 1.5, -2.0], 0.1, -1000.0..=1000.0),
                        Name::new("position"),
                    ),
                    (
                        vec_field("Scale", [1.0, 1.0, 1.0], 0.05, 0.05..=20.0),
                        Name::new("scale"),
                        // Y differs between the selected objects.
                        VecMixed(vec![false, true, false]),
                    ),
                    (
                        number_field("Health", 72.5, 0.5, 0.0..=100.0),
                        Name::new("health"),
                    ),
                    (
                        number_field("Speed", 4.0, 0.1, 0.0..=20.0),
                        Name::new("speed"),
                    ),
                    (
                        number_field("Mass", 1.0, 0.25, 0.0..=1000.0),
                        Name::new("mass"),
                        MixedValue,
                    ),
                    (label("Nothing changed yet"), Readout),
                    (label(" "), Refusal),
                ],
            )],
        )],
    ));
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// What the game hears of a number field: the value the player settled on.
fn on_number(
    change: On<ValueChange<f32>>,
    names: Query<&Name>,
    mut lines: Query<&mut Text, With<Readout>>,
) {
    if let (true, Ok(name)) = (change.is_final, names.get(change.source)) {
        for mut text in &mut lines {
            text.0 = format!("{name} is now {}", format_number(change.value, 0.01));
        }
    }
}

/// And of a vec field: the whole vector.
fn on_vec(change: On<VecChange>, names: Query<&Name>, mut lines: Query<&mut Text, With<Readout>>) {
    if let (true, Ok(name)) = (change.is_final, names.get(change.entity)) {
        for mut text in &mut lines {
            text.0 = format!("{name} is now {:?}", change.values);
        }
    }
}

/// A typed word the field would not take: said under the panel, with the reason.
fn on_refused(refused: On<NumberRefused>, mut lines: Query<&mut Text, With<Refusal>>) {
    for mut text in &mut lines {
        text.0 = format!("Refused: {}", refused.reason);
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

/// Where a staged row is: its row (or, for `position.1`, that component's row) and its field, in
/// window px.
fn find(
    row: &str,
    names: &Query<(Entity, &Name)>,
    parts: &Query<(Entity, &VecPart)>,
    fields: &Query<(&TextInputOf, &UiGlobalTransform, &ComputedNode)>,
    layouts: &Query<(&UiGlobalTransform, &ComputedNode)>,
) -> Option<(Vec2, Vec2)> {
    let entity = if let Some((vec, index)) = row.split_once('.') {
        let root = names.iter().find(|(_, n)| n.as_str() == vec)?.0;
        let index: usize = index.parse().ok()?;
        parts
            .iter()
            .find(|(_, p)| p.root == root && p.index == index)?
            .0
    } else {
        names.iter().find(|(_, n)| n.as_str() == row)?.0
    };
    let (at, node) = layouts.get(entity).ok()?;
    // The label end of the row: its padding, which the row itself takes the press in.
    let handle = at.translation - Vec2::new(node.size().x / 2.0 - 6.0, 0.0);
    let field = fields
        .iter()
        .find(|(of, ..)| of.0 == entity)
        .map(|(_, at, _)| at.translation)?;
    Some((handle, field))
}

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames, as the window's own input. A drag moves
/// the pointer onto the label, presses, moves a little each frame and releases.
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<(Entity, &Window), With<PrimaryWindow>>,
    names: Query<(Entity, &Name)>,
    parts: Query<(Entity, &VecPart)>,
    fields: Query<(&TextInputOf, &UiGlobalTransform, &ComputedNode)>,
    layouts: Query<(&UiGlobalTransform, &ComputedNode)>,
    ui_scale: Res<UiScale>,
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
    let key = |keys: &mut MessageWriter<KeyboardInput>,
               key_code: KeyCode,
               logical_key: Key,
               text: Option<&str>,
               state: ButtonState| {
        keys.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: text.map(Into::into),
            repeat: false,
            window,
        });
    };
    let Some(target) = WindowRef::Entity(window).normalize(None) else {
        return;
    };
    let mut point = |position: Vec2, action: PointerAction| {
        pointer.write(PointerInput::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::Window(target),
                position,
            },
            action,
        ));
    };
    match step {
        Step::Key(key_code, logical) => match phase {
            0 => key(
                &mut keys,
                *key_code,
                logical.clone(),
                None,
                ButtonState::Pressed,
            ),
            n if n == every / 2 => key(
                &mut keys,
                *key_code,
                logical.clone(),
                None,
                ButtonState::Released,
            ),
            _ => {}
        },
        Step::Shifted(key_code, logical) => match phase {
            0 => key(
                &mut keys,
                KeyCode::ShiftLeft,
                Key::Shift,
                None,
                ButtonState::Pressed,
            ),
            2 => key(
                &mut keys,
                *key_code,
                logical.clone(),
                None,
                ButtonState::Pressed,
            ),
            4 => key(
                &mut keys,
                *key_code,
                logical.clone(),
                None,
                ButtonState::Released,
            ),
            6 => key(
                &mut keys,
                KeyCode::ShiftLeft,
                Key::Shift,
                None,
                ButtonState::Released,
            ),
            _ => {}
        },
        Step::Type(text) => {
            // A character every two frames from the step's start.
            if phase % 2 == 0
                && let Some(c) = text.chars().nth((phase / 2) as usize)
            {
                let s = c.to_string();
                let logical = Key::Character(s.as_str().into());
                key(
                    &mut keys,
                    KeyCode::KeyA,
                    logical.clone(),
                    Some(&s),
                    ButtonState::Pressed,
                );
                key(
                    &mut keys,
                    KeyCode::KeyA,
                    logical,
                    None,
                    ButtonState::Released,
                );
            }
        }
        Step::Click(row) => {
            let Some((_, field)) = find(row, &names, &parts, &fields, &layouts) else {
                return;
            };
            let at = field / props.scale_factor();
            match phase {
                0 => point(at, PointerAction::Move { delta: Vec2::ZERO }),
                2 => point(at, PointerAction::Press(PointerButton::Primary)),
                4 => point(at, PointerAction::Release(PointerButton::Primary)),
                _ => {}
            }
        }
        Step::Drag(row, dx) => {
            let last = every - 2;
            let (position, action) = match phase {
                0 => {
                    let Some((handle, _)) = find(row, &names, &parts, &fields, &layouts) else {
                        return;
                    };
                    *grabbed = handle / props.scale_factor();
                    (*grabbed, PointerAction::Move { delta: Vec2::ZERO })
                }
                1 => (*grabbed, PointerAction::Press(PointerButton::Primary)),
                n if n < last => {
                    let t = (n - 1) as f32 / (last - 1) as f32;
                    let eased = t * t * (3.0 - 2.0 * t);
                    (
                        *grabbed + Vec2::new(*dx, 0.0) * ui_scale.0 * eased,
                        PointerAction::Move {
                            delta: Vec2::new(*dx, 0.0) * ui_scale.0 / last as f32,
                        },
                    )
                }
                n if n == last => (
                    *grabbed + Vec2::new(*dx, 0.0) * ui_scale.0,
                    PointerAction::Release(PointerButton::Primary),
                ),
                _ => return,
            };
            point(position, action);
        }
    }
}
