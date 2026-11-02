use bevy::asset::RenderAssetUsages;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
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
    Width(f32),
    Focus(String),
}

/// The frame `GOOEY_KEYS` starts at, after the layout has settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;
/// The side of one cell of the icon atlas, px.
const CELL: u32 = 32;
/// How many glyphs the atlas holds.
const GLYPHS: usize = 8;

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
        title: "bevy_gooey tools".into(),
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
    .add_systems(Update, (follow_background, capture, apply_steps))
    // Staged keys are written before `PreUpdate`, as a real keyboard's are: written in `Update`
    // they would reach a focused widget a frame after a system that reads them has acted on them.
    .add_systems(First, stage);
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "tools.png".into()),
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
            if let Some(width) = word.strip_prefix("width:") {
                return width.parse().ok().map(Step::Width);
            }
            if let Some(name) = word.strip_prefix("focus:") {
                return Some(Step::Focus(name.into()));
            }
            Some(match word {
                "tab" => Step::Key(KeyCode::Tab, Key::Tab),
                "up" => Step::Key(KeyCode::ArrowUp, Key::ArrowUp),
                "down" => Step::Key(KeyCode::ArrowDown, Key::ArrowDown),
                "left" => Step::Key(KeyCode::ArrowLeft, Key::ArrowLeft),
                "right" => Step::Key(KeyCode::ArrowRight, Key::ArrowRight),
                "escape" => Step::Key(KeyCode::Escape, Key::Escape),
                "enter" => Step::Key(KeyCode::Enter, Key::Enter),
                "backspace" => Step::Key(KeyCode::Backspace, Key::Backspace),
                _ => return None,
            })
        })
        .collect()
}

/// A name the staged `focus:` steps find a widget by.
#[derive(Component)]
struct Named(&'static str);

/// The box both bars stand in: its width is what `width:<px>` sets, as a narrower window would.
#[derive(Component)]
struct Bars;

/// A white-on-transparent atlas of eight glyphs, drawn with distance fields: plus, bin, check,
/// cross, move, pencil, undo, select. A game ships its own art; the skin tints whatever is white.
fn atlas() -> Image {
    let segments: [&[([f32; 2], [f32; 2])]; GLYPHS] = [
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
        &[
            ([16.0, 5.0], [16.0, 27.0]),
            ([5.0, 16.0], [27.0, 16.0]),
            ([12.0, 9.0], [16.0, 5.0]),
            ([20.0, 9.0], [16.0, 5.0]),
            ([12.0, 23.0], [16.0, 27.0]),
            ([20.0, 23.0], [16.0, 27.0]),
            ([9.0, 12.0], [5.0, 16.0]),
            ([9.0, 20.0], [5.0, 16.0]),
            ([23.0, 12.0], [27.0, 16.0]),
            ([23.0, 20.0], [27.0, 16.0]),
        ],
        &[
            ([8.0, 24.0], [22.0, 10.0]),
            ([22.0, 10.0], [25.0, 7.0]),
            ([7.0, 25.0], [8.0, 24.0]),
            ([11.0, 27.0], [8.0, 24.0]),
        ],
        &[
            ([10.0, 13.0], [6.0, 17.0]),
            ([10.0, 21.0], [6.0, 17.0]),
            ([6.0, 17.0], [20.0, 17.0]),
            ([20.0, 17.0], [26.0, 21.0]),
            ([26.0, 21.0], [26.0, 26.0]),
        ],
        &[
            ([7.0, 7.0], [25.0, 7.0]),
            ([25.0, 7.0], [25.0, 25.0]),
            ([25.0, 25.0], [7.0, 25.0]),
            ([7.0, 25.0], [7.0, 7.0]),
        ],
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
        GLYPHS as u32,
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
    let tool = |name: &'static str, glyph: usize, tip: Tooltip| {
        (icon_button(cell(glyph), tip), Named(name))
    };
    let modes = toggle_group(
        [
            GroupOption::icon(cell(4), Tooltip::new("Move")),
            GroupOption::icon(cell(5), Tooltip::new("Draw")),
            GroupOption::icon(cell(7), Tooltip::new("Select")),
            GroupOption::icon(cell(3), Tooltip::new("Erase")),
        ],
        0,
    );
    let bar = commands
        .spawn(toolbar(children![
            tool(
                "save",
                2,
                Tooltip::new("Save").with_shortcut(Shortcut::primary(KeyCode::KeyS)),
            ),
            tool(
                "open",
                0,
                Tooltip::new("New").with_shortcut(Shortcut::primary(KeyCode::KeyN)),
            ),
            tool(
                "undo",
                6,
                Tooltip::new("Undo").with_shortcut(Shortcut::primary(KeyCode::KeyZ)),
            ),
            toolbar_separator(),
            (modes, Named("mode")),
            toolbar_separator(),
            tool("bin", 1, Tooltip::new("Delete")),
            tool("plus", 0, Tooltip::new("Add layer")),
            tool("check", 2, Tooltip::new("Validate")),
        ]))
        .id();
    const LANGUAGES: [&str; 12] = [
        "English",
        "Deutsch",
        "Espanol",
        "Francais",
        "Italiano",
        "Nederlands",
        "Polski",
        "Portugues",
        "Svenska",
        "Dansk",
        "Suomi",
        "Norsk",
    ];
    let card = commands
        .spawn(panel())
        .with_children(|card| {
            card.spawn(heading("Project"));
            card.spawn((select("Language", LANGUAGES, 0), Named("language")));
            card.spawn(label(
                "Hover a tool for its name; Enter on Language opens the list.",
            ));
        })
        .id();
    let centre = commands
        .spawn(Node {
            flex_grow: 1.0,
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .add_child(card)
        .id();
    let status = commands
        .spawn(status_bar(
            children![
                (status_item("main"), Tooltip::new("Current branch")),
                (
                    status_item("2 problems"),
                    StatusKind::Warning,
                    Tooltip::new("Open the problems list"),
                ),
                status_progress("Baking", 0.4),
            ],
            children![
                (status_item("UTF-8"), StatusPriority(10)),
                (status_item("Spaces: 4"), StatusPriority(50)),
                (status_item("Ln 4, Col 12"), StatusPriority(200)),
            ],
        ))
        .id();
    let bars = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Bars,
        ))
        .add_children(&[bar, centre, status])
        .id();
    commands.spawn(screen()).add_child(bars);
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// The steps that are not keys: a width for the bars, a widget to focus.
fn apply_steps(
    capture: Option<Res<Capture>>,
    mut bars: Query<&mut Node, With<Bars>>,
    named: Query<(Entity, &Named)>,
    children: Query<&Children>,
    titles: Query<Entity, With<MenuTitle>>,
    mut focus: ResMut<InputFocus>,
    mut visible: ResMut<InputFocusVisible>,
) {
    let Some(c) = capture else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(8);
    if since % every != 0 {
        return;
    }
    match c.steps.get((since / every) as usize) {
        Some(Step::Width(px)) => {
            for mut node in &mut bars {
                node.width = Val::Px(*px);
            }
        }
        Some(Step::Focus(name)) => {
            let found = if name == "more" {
                // The toolbar's own "More" button is the one menu title in this example.
                titles.iter().next()
            } else {
                named.iter().find(|(_, n)| n.0 == name).map(|(e, _)| e)
            };
            // A group is not a stop: the ring goes to its first option.
            let found = found.map(|entity| {
                if name == "mode" {
                    children
                        .get(entity)
                        .ok()
                        .and_then(|kids| kids.first().copied())
                        .unwrap_or(entity)
                } else {
                    entity
                }
            });
            if let Some(entity) = found {
                focus.set(entity, FocusCause::Navigated);
                visible.0 = true;
            }
        }
        _ => {}
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
        Step::Width(_) | Step::Focus(_) => {}
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
