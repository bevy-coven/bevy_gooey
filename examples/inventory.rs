//! A backpack and a chest with no wiring: `slot_grid(columns, items)` is the whole widget.
//!
//! - The arrows and the D-pad walk the grid (Home and End to the ends of a row, Ctrl for the grid's),
//!   Enter, Space, South or a click uses a slot, and the pointer drags an item onto another slot,
//!   across to the chest too. With no pointer Alt+M (a pad's West) lifts the focused item, the
//!   arrows steer, and Alt+M again puts it down. A slot with a picture opens its tooltip on hover
//!   and focus. The two observers in this file hear the used slot and the move.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `inventory.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10
//! frames later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from
//! `GOOEY_SHOT_FROM` (default 30) there as `f_%04d.png`. `GOOEY_SKIN=parchment`,
//! `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start. `GOOEY_KEYS=down,pickup,…` stages
//! one input every `GOOEY_KEY_EVERY` (default 12) frames from frame 30: the keys `tab`, `up`,
//! `down`, `left`, `right`, `enter`, `escape`, `home`, `end`, and `pickup` (Alt+M), and
//! `drag:b0>e1` (the pointer drags the item in slot 0 of the backpack `b`, the chest `c` or the
//! equipment `e` onto slot 1 of another, taking three steps' time). The equipment slots take one
//! kind of item each (`SlotRules`): over one that will not, the mark and ghost turn red.

use bevy::asset::RenderAssetUsages;
use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
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
    steps: Vec<Staged>,
}

/// One staged input of `GOOEY_KEYS`.
#[derive(Clone)]
enum Staged {
    Key(KeyCode, Key),
    /// Alt held, then M.
    PickUp,
    /// The pointer drags one slot's item onto another's, over three steps' time.
    Drag(Pile, usize, Pile, usize),
}

/// Which grid a staged drag means.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Pile {
    Backpack,
    Chest,
    Equipment,
}

impl Pile {
    fn of(letter: char) -> Option<Self> {
        match letter {
            'b' => Some(Self::Backpack),
            'c' => Some(Self::Chest),
            'e' => Some(Self::Equipment),
            _ => None,
        }
    }
}

/// The equipment slots: a weapon, a light and a charm, one kind each.
fn fits_equipment(item: &Slot, slot: usize) -> bool {
    item.kind.as_deref() == Some(["weapon", "light", "charm"][slot.min(2)])
}

/// `b0>e1`: slot 0 of the backpack onto slot 1 of the equipment.
fn parse_drag(text: &str) -> Option<Staged> {
    let (from, to) = text.split_once('>')?;
    let slot = |end: &str| -> Option<(Pile, usize)> {
        let mut chars = end.chars();
        Some((Pile::of(chars.next()?)?, chars.as_str().parse().ok()?))
    };
    let (from, to) = (slot(from)?, slot(to)?);
    Some(Staged::Drag(from.0, from.1, to.0, to.1))
}

/// The line that says what the player did.
#[derive(Component)]
struct Status;

/// The frame `GOOEY_KEYS` starts at, after the layout and autofocus have settled.
const KEY_FROM: u32 = 30;
/// The first frame counted into the mean: start-up frames are not a HUD's cost.
const WARM_FRAMES: u32 = 20;
/// The side of one glyph of the atlas.
const CELL: u32 = 48;
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
        title: "bevy_gooey inventory".into(),
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
    .add_systems(Startup, setup)
    .add_systems(Update, (follow_background, capture))
    // Staged input is written before `PreUpdate`, as a real keyboard's is.
    .add_systems(First, stage_keys);
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "inventory.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(KEY_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
            step_every: env("GOOEY_KEY_EVERY").unwrap_or(12),
            steps: std::env::var("GOOEY_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter_map(|k| {
                    if let Some(drag) = k.trim().strip_prefix("drag:") {
                        return parse_drag(drag);
                    }
                    Some(match k.trim() {
                        "tab" => Staged::Key(KeyCode::Tab, Key::Tab),
                        "up" => Staged::Key(KeyCode::ArrowUp, Key::ArrowUp),
                        "down" => Staged::Key(KeyCode::ArrowDown, Key::ArrowDown),
                        "left" => Staged::Key(KeyCode::ArrowLeft, Key::ArrowLeft),
                        "right" => Staged::Key(KeyCode::ArrowRight, Key::ArrowRight),
                        "enter" => Staged::Key(KeyCode::Enter, Key::Enter),
                        "escape" => Staged::Key(KeyCode::Escape, Key::Escape),
                        "home" => Staged::Key(KeyCode::Home, Key::Home),
                        "end" => Staged::Key(KeyCode::End, Key::End),
                        "pickup" => Staged::PickUp,
                        _ => return None,
                    })
                })
                .collect(),
        });
    }
    app.run();
}

/// A white-on-transparent atlas of eight glyphs drawn with distance fields: sword, potion, gem,
/// key, arrow, torch, coin, scroll. A game ships its own art; the skin tints whatever is white.
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
    let glyph = |index| IconSource::Atlas {
        image: image.clone(),
        layout: layout.clone(),
        index,
    };
    let sword = || {
        Slot::new("Iron sword")
            .icon(glyph(0))
            .tooltip("Iron sword, +3 damage")
            .kind("weapon")
    };
    let potion = |n| Slot::new("Health potion").icon(glyph(1)).count(n);
    let gem = || {
        Slot::new("Ruby")
            .icon(glyph(2))
            .tooltip("A flawless ruby, worth 400")
            .kind("charm")
    };
    let key = || Slot::new("Cellar key").icon(glyph(3));
    let arrows = |n| Slot::new("Arrows").icon(glyph(4)).count(n);
    let torch = |n| Slot::new("Torch").icon(glyph(5)).count(n).kind("light");
    let coin = |n| Slot::new("Gold").icon(glyph(6)).count(n);
    let scroll = || Slot::new("Old map").icon(glyph(7));

    #[rustfmt::skip]
    let backpack = vec![
        Some(sword()), Some(potion(3)), Some(arrows(24)), None, Some(gem()),
        Some(key()), None, Some(torch(2)), Some(coin(120)), None,
        Some(Slot::new("Bread").count(4)), Some(scroll()), None, None, Some(Slot::new("Rope")),
        None, None, None, None, None,
    ];
    #[rustfmt::skip]
    let chest = vec![
        Some(coin(850)), Some(potion(12)), None, Some(Slot::new("Lantern")),
        None, Some(torch(6)), None, None,
    ];

    let equipment = vec![None, Some(torch(1)), None];

    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            panel(),
            children![
                heading("Inventory"),
                (
                    Node {
                        column_gap: Val::Px(24.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },
                    children![
                        (
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                ..default()
                            },
                            children![label("Backpack"), (slot_grid(5, backpack), Pile::Backpack)],
                        ),
                        (
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                ..default()
                            },
                            children![
                                label("Chest"),
                                (slot_grid(4, chest), Pile::Chest),
                                label("Equipment"),
                                (
                                    slot_grid(3, equipment),
                                    Pile::Equipment,
                                    SlotRules::new(fits_equipment)
                                ),
                            ],
                        ),
                    ],
                ),
                (
                    label("Arrows walk. Enter uses. Alt+M lifts and drops."),
                    Status
                ),
            ],
        ),],
    ));
    // The grids' events reach the whole app: one observer for a use and one for a move.
    commands.add_observer(
        |chosen: On<ValueChange<usize>>,
         grids: Query<&SlotItems>,
         mut status: Single<&mut Text, With<Status>>| {
            if !chosen.is_final {
                return;
            }
            if let Ok(items) = grids.get(chosen.source) {
                status.0 = match items.0.get(chosen.value) {
                    Some(Some(item)) => format!("Used {}", item.name),
                    _ => "Nothing there".to_string(),
                };
            }
        },
    );
    commands.add_observer(
        |refused: On<SlotRefused>,
         grids: Query<&SlotItems>,
         mut status: Single<&mut Text, With<Status>>| {
            let name = grids
                .get(refused.from.grid)
                .ok()
                .and_then(|items| items.0.get(refused.from.index).cloned().flatten())
                .map_or_else(|| "That".into(), |item| item.name);
            status.0 = format!("{name} does not fit there");
        },
    );
    commands.add_observer(
        |moved: On<SlotMoved>,
         mut grids: Query<&mut SlotItems>,
         mut status: Single<&mut Text, With<Status>>| {
            let name = grids
                .get(moved.from.grid)
                .ok()
                .and_then(|items| items.0.get(moved.from.index).cloned().flatten())
                .map(|item| item.name);
            if moved.apply(&mut grids) {
                status.0 = format!("Moved {}", name.unwrap_or_else(|| "nothing".into()));
            }
        },
    );
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

/// `GOOEY_KEYS`: one input every `GOOEY_KEY_EVERY` frames, pressed and then released as the
/// window's own keyboard input, so Bevy's input dispatch and the crate's navigation see it.
fn stage_keys(
    capture: Option<Res<Capture>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
    mut pointer: MessageWriter<PointerInput>,
    piles: Query<(&Pile, &Children)>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    scale: Res<UiScale>,
) {
    let (Some(c), Ok(window)) = (capture, window.single()) else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.step_every.max(8);
    // A drag takes three steps' time: the pointer needs the frames to be seen travelling.
    let length = |step: &Staged| {
        if matches!(step, Staged::Drag(..)) {
            3 * every
        } else {
            every
        }
    };
    let (mut start, mut found) = (0, None);
    for step in &c.steps {
        if since < start + length(step) {
            found = Some(step);
            break;
        }
        start += length(step);
    }
    let Some(step) = found else {
        return;
    };
    let phase = since - start;
    let mut key = |key_code: KeyCode, logical_key: Key, state: ButtonState| {
        keys.write(KeyboardInput {
            key_code,
            logical_key,
            state,
            text: None,
            repeat: false,
            window,
        });
    };
    match step {
        Staged::Key(key_code, logical) => match phase {
            0 => key(*key_code, logical.clone(), ButtonState::Pressed),
            n if n == every / 2 => key(*key_code, logical.clone(), ButtonState::Released),
            _ => {}
        },
        Staged::Drag(from, from_slot, to, to_slot) => {
            let centre = |pile: Pile, slot: usize| -> Option<Vec2> {
                let (_, cells) = piles.iter().find(|(p, _)| **p == pile)?;
                let (node, at) = nodes.get(*cells.get(slot)?).ok()?;
                Some(at.translation * node.inverse_scale_factor() * scale.0)
            };
            let (Some(a), Some(b)) = (centre(*from, *from_slot), centre(*to, *to_slot)) else {
                return;
            };
            let end = length(step);
            let travel = (end - 8) as f32 - 4.0;
            let Some(target) = WindowRef::Entity(window).normalize(None) else {
                return;
            };
            let at = |position: Vec2, action: PointerAction| {
                PointerInput::new(
                    PointerId::Mouse,
                    Location {
                        target: NormalizedRenderTarget::Window(target),
                        position,
                    },
                    action,
                )
            };
            if phase == 0 {
                pointer.write(at(a, PointerAction::Move { delta: Vec2::X }));
            } else if phase == 2 {
                pointer.write(at(a, PointerAction::Press(PointerButton::Primary)));
            } else if (4..end - 4).contains(&phase) {
                let t = ((phase - 4) as f32 / travel).clamp(0.0, 1.0);
                let position = a.lerp(b, t);
                pointer.write(at(
                    position,
                    PointerAction::Move {
                        delta: (b - a) / travel,
                    },
                ));
            } else if phase == end - 2 {
                pointer.write(at(b, PointerAction::Release(PointerButton::Primary)));
            }
        }
        Staged::PickUp => match phase {
            0 => key(KeyCode::AltLeft, Key::Alt, ButtonState::Pressed),
            2 => key(
                KeyCode::KeyM,
                Key::Character("m".into()),
                ButtonState::Pressed,
            ),
            n if n == every / 2 => key(
                KeyCode::KeyM,
                Key::Character("m".into()),
                ButtonState::Released,
            ),
            n if n == every / 2 + 1 => key(KeyCode::AltLeft, Key::Alt, ButtonState::Released),
            _ => {}
        },
    }
}
