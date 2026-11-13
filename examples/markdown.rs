//! The Markdown view: a style guide's page with headings, pictures, tables, lists, a quote, code and
//! links, read in a scrolling viewport. Scroll with the wheel or the rail, Tab to a link and press
//! Enter (the log line says what the page asked for), and `GOOEY_JUMP=<heading id>` jumps to a
//! heading the way an outline would.
//!
//! Run `cargo run --release --example markdown --features markdown`.
//!
//! Capture mode, for a still and a GIF: `GOOEY_FRAMES=<n>` saves a screenshot at frame `n` to
//! `GOOEY_SHOT` (default `markdown.png`), prints `gooey: frames <n> mean_ms <x>` and exits 10 frames
//! later; `GOOEY_SHOT_DIR=<dir>` also keeps every `GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM`
//! (default 30) there as `f_%04d.png`. `GOOEY_JUMP=<id>,<id>…` scrolls to one heading every
//! `GOOEY_JUMP_EVERY` (default 60) frames from frame 30. `GOOEY_SKIN=parchment`,
//! `GOOEY_TEXT_SCALE=<f>` and `GOOEY_SIZE=<w>x<h>` set the start.
//!
//! `GOOEY_BUILD_TIME=1` opens no window: it builds a 75 KB page of 40 tables (the issue's size) in a
//! bare app, once, and prints what the parse and the spawning of its blocks took, as
//! `gooey: markdown parsed … spawned …`; run it with `--release` for the number the docs quote.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::WindowResolution;
use bevy_gooey::*;

const PAGE: &str = "\
# The style guide

How the game looks and why. Every picture is next to the words about it, so a **reviewer** reads
the page once. See [the palette](#palette), [the type scale](#type) or [the repository](https://example.com/guide).

## Palette

The three inks, drawn from one seed. ![The swatches](img/swatches.png)

| Token | Hex | Used for |
|-------|-----|----------|
| `ink` | #1b1b24 | Body text and rules |
| `paper` | #ece7d8 | Page ground |
| `ember` | #d8572a | The one accent, ~~never~~ rarely a fill |

## Type

Headings step down a scale, the body is the middle step and code is monospaced:

```rust
let size = skin.type_size(2);
```

> A page keeps its room while a picture loads, and a picture that is missing says where it was looked for.

![A picture that is not on the shelf](img/missing.png)

## Checklist

- [x] Headings have stable ids
- [x] Tables hold links, [pictures](#palette) and `code`
- [ ] Every picture has a path
  - nested items keep their own column

1. Write the page
2. Read it rendered
3. Keep the pictures beside it

---

The end of the page.
";

fn env<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|s| s.parse().ok())
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
}

/// The headings `GOOEY_JUMP` visits, one every `every` frames.
#[derive(Resource)]
struct Jumps {
    ids: Vec<String>,
    every: u32,
    frame: u32,
}

/// The first frame counted into the mean, and the one jumps start at.
const WARM_FRAMES: u32 = 20;
const JUMP_FROM: u32 = 30;

/// A page like a style guide: `tables` tables of 22 rows each and prose between them (about 75 KB
/// for 40).
fn long_page(tables: usize) -> String {
    let mut page = String::new();
    for table in 0..tables {
        page.push_str(&format!("## Section {table}\n\n"));
        page.push_str(
            "A paragraph of prose that explains the section at some length, with a \
             **strong** word, a `code` word and an *emphasised* one, so that a reader has \
             something to read between the tables of the guide.\n\n",
        );
        page.push_str("| Name | Value | Use | Note |\n|------|-------|-----|------|\n");
        for row in 0..22 {
            page.push_str(&format!(
                "| token-{table}-{row} | #{row:02x}{row:02x}{row:02x} | surface {row} | the note for row {row} of table {table} |\n"
            ));
        }
        page.push('\n');
    }
    page
}

/// `GOOEY_BUILD_TIME`: the cost of parsing and spawning a big page, with no window and no layout.
fn build_time() {
    let source = long_page(40);
    let started = std::time::Instant::now();
    let document = bevy_gooey::markdown::parse(&source);
    let parsed = started.elapsed();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(GooeySkin::dark())
        .add_systems(Update, build_markdown);
    app.world_mut().spawn(markdown_view(
        &source,
        MarkdownBase::new("guide", |_: &str| None),
    ));
    let before = app.world().entities().len();
    let started = std::time::Instant::now();
    app.update();
    let spawned = started.elapsed();
    println!(
        "gooey: markdown parsed {} bytes, {} tables, in {parsed:?}; parsed again and spawned {} entities in {spawned:?}",
        source.len(),
        document.tables(),
        app.world().entities().len() - before
    );
}

fn main() {
    if std::env::var("GOOEY_BUILD_TIME").is_ok() {
        build_time();
        return;
    }
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
        title: "bevy_gooey markdown".into(),
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
    .add_systems(Update, (follow_background, capture, jump));
    if let Ok(ids) = std::env::var("GOOEY_JUMP") {
        app.insert_resource(Jumps {
            ids: ids.split(',').map(str::to_owned).collect(),
            every: env("GOOEY_JUMP_EVERY").unwrap_or(60),
            frame: 0,
        });
    }
    if let Some(at) = env("GOOEY_FRAMES") {
        app.insert_resource(Capture {
            at,
            shot: std::env::var("GOOEY_SHOT").unwrap_or_else(|_| "markdown.png".into()),
            dir: std::env::var("GOOEY_SHOT_DIR").ok(),
            from: env("GOOEY_SHOT_FROM").unwrap_or(JUMP_FROM),
            every: env("GOOEY_SHOT_EVERY").unwrap_or(2),
            kept: 0,
            frame: 0,
            total_ms: 0.0,
            counted: 0,
        });
    }
    app.run();
}

/// A picture of flat bars of colour, `width` by `height`, so the page has something to draw.
fn bars(width: u32, height: u32, colours: &[[u8; 4]]) -> Image {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..height {
        for x in 0..width {
            data.extend_from_slice(&colours[(x * colours.len() as u32 / width) as usize]);
        }
    }
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// The screen: the page in a panel, its pictures from a shelf of generated images.
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut shelf: HashMap<String, Handle<Image>> = HashMap::new();
    shelf.insert(
        "guide/img/swatches.png".into(),
        images.add(bars(
            240,
            40,
            &[[27, 27, 36, 255], [236, 231, 216, 255], [216, 87, 42, 255]],
        )),
    );
    let base = MarkdownBase::new("guide", move |path: &str| shelf.get(path).cloned());
    commands.spawn(Camera2d);
    commands.spawn((
        screen(),
        children![(
            Node {
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Stretch,
                padding: UiRect::vertical(Val::Vh(3.0)),
                ..default()
            },
            children![(
                Node {
                    width: Val::Px(760.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                Role::Panel,
                children![(
                    markdown_view(PAGE, base),
                    observe(|link: On<MarkdownLink>| {
                        info!("the page asked for {:?} {}", link.kind, link.dest);
                    }),
                )],
            )],
        )],
    ));
}

fn follow_background(skin: Res<GooeySkin>, mut clear: ResMut<ClearColor>) {
    if skin.is_changed() {
        clear.0 = skin.background;
    }
}

/// `GOOEY_JUMP`: the view scrolls to one heading after another.
fn jump(
    jumps: Option<ResMut<Jumps>>,
    views: Query<Entity, With<MarkdownView>>,
    mut commands: Commands,
) {
    let Some(mut jumps) = jumps else {
        return;
    };
    jumps.frame += 1;
    let Some(since) = jumps.frame.checked_sub(JUMP_FROM) else {
        return;
    };
    if since % jumps.every.max(1) != 0 {
        return;
    }
    let Some(id) = jumps.ids.get((since / jumps.every.max(1)) as usize) else {
        return;
    };
    for view in &views {
        commands.trigger(scroll_to_heading(view, id.clone()));
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
