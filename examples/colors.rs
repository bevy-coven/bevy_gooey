//! Colour fields in a material editor: a colour is a swatch and a hex field. Click the field and
//! type `#ff8800` (or `f80`, or `#ff880080` with an alpha) and Enter; press the swatch to open the
//! picker: a saturation and value area, and strips for hue, brightness and alpha. The emission is a
//! multi-selection whose members differ: the field is empty with a muted `--` until the player sets
//! it. Typing something that is not a colour is refused with its reason, shown under the panel.
//!
//! Run `cargo run --release --example colors`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `colors.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png` (the directory must exist). `GOOEY_SKIN=parchment`,
//! `high_contrast` or `tool`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//! `GOOEY_KEYS` stages one step every `GOOEY_KEY_EVERY` (default 24) frames from frame 30, steps
//! separated by `;`: `swatch:<row>` presses a row's swatch (`albedo`, `emission` or `tint`),
//! `click:<row>` focuses its field, `type:<text>` types, `area:<s>,<v>` drags the picker's area to
//! that saturation and value (each 0 to 1) over the step's frames, `hue:<n>`, `brightness:<n>` and
//! `alpha:<n>` press a strip at that fraction, `away` clicks on empty ground, and `up`, `down`,
//! `left`, `right`, `enter`, `escape` and `tab` are real keys.

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
    Type(String),
    /// A press and release on the row's field.
    Click(String),
    /// A press and release on the row's swatch.
    Swatch(String),
    /// A drag of the picker's area, from its middle to a saturation and value.
    Area(f32, f32),
    /// A press on the picker's `n`th strip (0 hue, 1 brightness, 2 alpha) at a fraction.
    Strip(usize, f32),
    /// A press on empty ground, away from everything.
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
        title: "bevy_gooey colors".into(),
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
    .add_observer(on_colour);
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "colors.png".into()),
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
            if let Some(row) = word.strip_prefix("swatch:") {
                return Some(Step::Swatch(row.into()));
            }
            if let Some(typed) = word.strip_prefix("type:") {
                return Some(Step::Type(typed.into()));
            }
            if let Some(at) = word.strip_prefix("area:") {
                let (s, v) = at.split_once(',')?;
                return Some(Step::Area(s.trim().parse().ok()?, v.trim().parse().ok()?));
            }
            for (name, index) in [("hue:", 0), ("brightness:", 1), ("alpha:", 2)] {
                if let Some(at) = word.strip_prefix(name) {
                    return Some(Step::Strip(index, at.trim().parse().ok()?));
                }
            }
            Some(match word {
                "away" => Step::Away,
                "tab" => Step::Key(KeyCode::Tab, Key::Tab),
                "up" => Step::Key(KeyCode::ArrowUp, Key::ArrowUp),
                "down" => Step::Key(KeyCode::ArrowDown, Key::ArrowDown),
                "left" => Step::Key(KeyCode::ArrowLeft, Key::ArrowLeft),
                "right" => Step::Key(KeyCode::ArrowRight, Key::ArrowRight),
                "enter" => Step::Key(KeyCode::Enter, Key::Enter),
                "escape" => Step::Key(KeyCode::Escape, Key::Escape),
                _ => return None,
            })
        })
        .collect()
}

/// What the game reads back: the colour the player last settled on.
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
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::Center,
                padding: UiRect::top(Val::Px(24.0)),
                ..default()
            },
            children![(
                panel(),
                children![
                    heading("Material"),
                    (
                        color_field("Albedo", Srgba::hex("#ff8800").unwrap_or(Srgba::WHITE)),
                        Name::new("albedo"),
                    ),
                    (
                        color_field("Emission", Srgba::hex("#33cc66").unwrap_or(Srgba::WHITE)),
                        Name::new("emission"),
                        // The selected materials glow differently.
                        MixedValue,
                    ),
                    (
                        color_field("Tint", Srgba::hex("#3366cc99").unwrap_or(Srgba::WHITE)),
                        Name::new("tint"),
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

/// What the game hears of a colour field: the colour the player settled on.
fn on_colour(
    change: On<ValueChange<Srgba>>,
    names: Query<&Name>,
    mut lines: Query<&mut Text, With<Readout>>,
) {
    if let (true, Ok(name)) = (change.is_final, names.get(change.source)) {
        for mut text in &mut lines {
            text.0 = format!("{name} is now {}", hex_text(change.value.to_f32_array()));
        }
    }
}

/// A typed word the field would not take: said under the panel, with the reason.
fn on_refused(refused: On<ColorRefused>, mut lines: Query<&mut Text, With<Refusal>>) {
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

/// The row named `name`, and its field's and swatch's places in window px.
fn find(
    name: &str,
    names: &Query<(Entity, &Name)>,
    fields: &Query<(&TextInputOf, &UiGlobalTransform)>,
    swatches: &Query<(&Role, &ChildOf, &UiGlobalTransform)>,
) -> Option<(Vec2, Vec2)> {
    let row = names.iter().find(|(_, n)| n.as_str() == name)?.0;
    let field = fields.iter().find(|(of, _)| of.0 == row)?.1.translation;
    let swatch = swatches
        .iter()
        .find(|(role, parent, _)| **role == Role::ColorSwatch && parent.0 == row)?
        .2
        .translation;
    Some((field, swatch))
}

/// `GOOEY_KEYS`: one step every `GOOEY_KEY_EVERY` frames, as the window's own input.
#[allow(clippy::too_many_arguments)]
fn stage(
    capture: Option<Res<Capture>>,
    window: Query<(Entity, &Window), With<PrimaryWindow>>,
    names: Query<(Entity, &Name)>,
    fields: Query<(&TextInputOf, &UiGlobalTransform)>,
    swatches: Query<(&Role, &ChildOf, &UiGlobalTransform)>,
    areas: Query<(&UiGlobalTransform, &ComputedNode), With<ColorArea>>,
    strips: Query<(&UiGlobalTransform, &ComputedNode), With<ColorStrip>>,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
    mut from: Local<Vec2>,
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
    let scale = props.scale_factor();
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
        Step::Click(row) | Step::Swatch(row) => {
            let Some((field, swatch)) = find(row, &names, &fields, &swatches) else {
                return;
            };
            let at = if matches!(step, Step::Click(_)) {
                field
            } else {
                swatch
            } / scale;
            match phase {
                0 => point(at, PointerAction::Move { delta: Vec2::ZERO }),
                2 => point(at, PointerAction::Press(PointerButton::Primary)),
                4 => point(at, PointerAction::Release(PointerButton::Primary)),
                _ => {}
            }
        }
        Step::Away => {
            let at = Vec2::new(8.0, 8.0);
            match phase {
                0 => point(at, PointerAction::Move { delta: Vec2::ZERO }),
                2 => point(at, PointerAction::Press(PointerButton::Primary)),
                4 => point(at, PointerAction::Release(PointerButton::Primary)),
                _ => {}
            }
        }
        Step::Strip(index, fraction) => {
            let mut found: Vec<(&UiGlobalTransform, &ComputedNode)> = strips.iter().collect();
            found.sort_by(|a, b| a.0.translation.y.total_cmp(&b.0.translation.y));
            let Some((at, node)) = found.get(*index) else {
                return;
            };
            // Along the rail, inside the thumb's half-width at each end.
            let x = at.translation.x + (fraction - 0.5) * (node.size().x - 40.0);
            let position = Vec2::new(x, at.translation.y + node.size().y / 4.0) / scale;
            match phase {
                0 => point(position, PointerAction::Move { delta: Vec2::ZERO }),
                2 => point(position, PointerAction::Press(PointerButton::Primary)),
                4 => point(position, PointerAction::Release(PointerButton::Primary)),
                _ => {}
            }
        }
        Step::Area(s, v) => {
            let Some((at, node)) = areas.iter().next() else {
                return;
            };
            let size = node.size();
            let middle = at.translation;
            let to = at.translation - size / 2.0 + Vec2::new(s * size.x, (1.0 - v) * size.y);
            let last = every - 2;
            let (position, action) = match phase {
                0 => {
                    *from = middle;
                    (middle / scale, PointerAction::Move { delta: Vec2::ZERO })
                }
                1 => (middle / scale, PointerAction::Press(PointerButton::Primary)),
                n if n < last => {
                    let t = (n - 1) as f32 / (last - 1) as f32;
                    let eased = t * t * (3.0 - 2.0 * t);
                    (
                        (*from + (to - *from) * eased) / scale,
                        PointerAction::Move {
                            delta: (to - *from) / scale / last as f32,
                        },
                    )
                }
                n if n == last => (to / scale, PointerAction::Release(PointerButton::Primary)),
                _ => return,
            };
            point(position, action);
        }
    }
}
