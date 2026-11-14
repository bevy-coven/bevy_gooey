//! The small parts of a tool's panel, with no `Node` of the example's own inside it: a strip of
//! `edge_tab`s (click one: the edge along its bottom moves and nothing else does), a grouped list
//! whose `band` headers fold their group on a click, a `fill_text` name that wraps inside its own
//! column beside a `count_text` that keeps its words whole, and a `glyph_button` whose hit area is
//! the skin's target however small the mark in it is.
//!
//! Run `cargo run --release --example kit`.
//!
//! Capture mode, for a still: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to `GOOEY_SHOT`
//! (default `kit.png`) and exits 10 frames later. `GOOEY_SKIN=parchment`, `high_contrast`, `tool`
//! or `desk`, `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.

use bevy::ecs::spawn::SpawnIter;
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui::Selected;
use bevy::window::WindowResolution;
use bevy_gooey::*;

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
}

fn main() {
    let size = std::env::var("GOOEY_SIZE").unwrap_or_else(|_| "560x640".into());
    let (w, h) = size.split_once('x').unwrap_or(("560", "640"));
    let mut skin = match std::env::var("GOOEY_SKIN").as_deref() {
        Ok("parchment") => GooeySkin::parchment(),
        Ok("high_contrast") => GooeySkin::high_contrast(),
        Ok("tool") => GooeySkin::tool(),
        Ok("desk") => GooeySkin::desk(),
        _ => GooeySkin::dark(),
    };
    if let Some(scale) = env::<f32>("GOOEY_TEXT_SCALE") {
        skin.text_scale = scale;
    }
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "kit".into(),
                    resolution: WindowResolution::new(
                        w.parse().unwrap_or(560),
                        h.parse().unwrap_or(640),
                    )
                    .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            GooeyPlugins,
        ))
        .insert_resource(skin)
        .add_systems(Startup, setup)
        .add_systems(Update, (follow_background, capture))
        .run();
}

/// The rows of a group, which its band folds.
#[derive(Component)]
struct GroupRows;

/// A tab of the strip.
#[derive(Component)]
struct StripTab;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            // The one host node: how wide the panel is allowed to be, so a long name has to wrap.
            Node {
                width: Val::Px(440.0),
                ..default()
            },
            children![(
                panel(),
                children![
                    heading("Kit"),
                    strip(),
                    group(
                        "kenney_prototype-kit/Models/GLB format",
                        145,
                        [
                            "wall.glb",
                            "door.glb",
                            "a_floor_with_a_very_long_name_that_wraps.glb"
                        ],
                    ),
                    group(
                        "low_poly_furniture/glb/ Electronics",
                        8,
                        ["lamp.glb", "tv.glb"]
                    ),
                ],
            )],
        )],
    ));
}

/// The strip: the first tab is the selected one, a click moves the selection.
fn strip() -> impl Bundle {
    (
        row(0.0),
        children![
            (edge_tab("Meshes"), StripTab, Selected, observe(pick)),
            (edge_tab("Tiles"), StripTab, observe(pick)),
            (edge_tab("Kits"), StripTab, observe(pick)),
        ],
    )
}

/// A click on a tab selects it and no other: the host writes `Selected`, the skin lights the edge.
fn pick(click: On<PointerClick>, tabs: Query<Entity, With<StripTab>>, mut commands: Commands) {
    for tab in &tabs {
        if tab == click.entity {
            commands.entity(tab).insert(Selected);
        } else {
            commands.entity(tab).remove::<Selected>();
        }
    }
}

/// A band and the rows it folds.
fn group<const N: usize>(name: &'static str, count: u32, files: [&'static str; N]) -> impl Bundle {
    (
        column(0.0),
        children![
            (
                band(),
                observe(fold),
                children![
                    (Text::new("v"), Role::ButtonText),
                    fill_text(name),
                    count_text(count.to_string()),
                ],
            ),
            (
                GroupRows,
                column(0.5),
                Children::spawn(SpawnIter(files.into_iter().map(file_row))),
            ),
        ],
    )
}

/// A file's row: its name, and a mark that is a hit target of the skin's size.
fn file_row(file: &'static str) -> impl Bundle {
    (
        row(1.0),
        children![fill_text(file), glyph_button("x", format!("Remove {file}"))],
    )
}

/// A press on a band shows or hides the rows of its own group.
fn fold(
    press: On<Activate>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    mut rows: Query<&mut Node, With<GroupRows>>,
) {
    let Ok(group) = parents.get(press.entity) else {
        return;
    };
    let Ok(kids) = children.get(group.parent()) else {
        return;
    };
    for kid in kids {
        if let Ok(mut node) = rows.get_mut(*kid) {
            node.display = if node.display == Display::None {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

fn capture(mut frame: Local<u32>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    *frame += 1;
    let Some(at) = env::<u32>("GOOEY_FRAMES") else {
        return;
    };
    if *frame == at {
        let path = std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "kit.png".into());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if *frame > at + 10 {
        exit.write(AppExit::Success);
    }
}
