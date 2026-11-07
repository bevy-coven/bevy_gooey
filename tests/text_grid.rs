//! Text on the device-pixel grid: every widget's text size is a whole number of device pixels at
//! the node's scale (the window's scale factor times `UiScale`), never below the size the skin
//! asks for and less than one device pixel above it, set again when the scale changes. Laid out
//! for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::text::FontSize;
use bevy::ui::{ComputedNode, ComputedUiRenderTargetInfo};
use bevy::window::{PrimaryWindow, WindowResolution, WindowScaleFactorChanged};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// The player's text size the acceptance names: 10% larger text.
const TEXT_SCALE: f32 = 1.1;

/// The window scale factors the acceptance names.
const SCALE_FACTORS: [f32; 4] = [1.0, 1.25, 1.5, 2.0];

/// How far past a whole device pixel a size may read and still be on it: the `f32` noise of a
/// logical size times a scale, well under any size a glyph could be drawn at.
const ON_THE_GRID: f32 = 1.0e-3;

/// A headless app whose window is 1280 × 720 logical px at `scale_factor`, with a camera.
fn app(scale_factor: f32) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        (1280.0 * scale_factor) as u32,
                        (720.0 * scale_factor) as u32,
                    )
                    .with_scale_factor_override(scale_factor),
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>(),
        GooeyPlugins,
    ))
    .insert_resource(GooeySkin {
        text_scale: TEXT_SCALE,
        ..GooeySkin::dark()
    });
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

/// A panel of widgets whose texts use three of the skin's sizes: the heading's, the body's (a
/// label, buttons, a toggle, a slider and a text field) and a type-scale step (a cooldown's chip).
fn widgets(app: &mut App) {
    let world = app.world_mut();
    let parts = [
        world.spawn(heading("Options")).id(),
        world.spawn(label("Sound and display")).id(),
        world.spawn(button("Play")).id(),
        world.spawn((button("Fire"), Cooldown::new(8.0))).id(),
        world.spawn(toggle("Subtitles", true)).id(),
        world.spawn(slider("Volume", 0.5, 0.0..=1.0)).id(),
        world.spawn(text_input("Name", "Ada")).id(),
    ];
    let card = world.spawn(panel()).add_children(&parts).id();
    world.spawn(screen()).add_child(card);
    settle(app);
}

/// Every text size the skin can ask a widget for, logical px, before snapping.
fn asked_sizes(skin: &GooeySkin) -> Vec<f32> {
    let mut sizes = vec![skin.text_size, skin.heading_size];
    sizes.extend(skin.type_scale);
    sizes.into_iter().map(|s| s * skin.text_scale).collect()
}

/// Each widget text: its logical size and the scale it is laid out at.
fn widget_texts(app: &mut App) -> Vec<(f32, f32)> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&TextFont, &ComputedUiRenderTargetInfo), With<Role>>();
    query
        .iter(app.world())
        .map(|(font, target)| match font.font_size {
            FontSize::Px(px) => (px, target.scale_factor()),
            other => panic!("a widget's text size is in logical px, not {other:?}"),
        })
        .collect()
}

/// `size` at `scale` is a whole number of device pixels, at or above one of the skin's asked
/// sizes and less than one device pixel over it.
fn assert_on_the_grid(size: f32, scale: f32, asked: &[f32]) {
    let device = size * scale;
    assert!(
        (device - device.round()).abs() < ON_THE_GRID,
        "{size} px at {scale}x is {device} device px, not a whole number"
    );
    assert!(
        asked.iter().any(|&a| {
            let unrounded = a * scale;
            device >= unrounded - ON_THE_GRID && device < unrounded + 1.0
        }),
        "{size} px at {scale}x ({device} device px) is not the rounding up of any asked size {asked:?}"
    );
}

/// The size `role`'s text asks for, logical px, before snapping: the heading's, a cooldown chip's
/// type-scale step, else the body's.
fn asked_by(role: &Role, skin: &GooeySkin) -> f32 {
    let size = match role {
        Role::Heading => skin.heading_size,
        Role::CooldownText => skin.type_scale[1],
        _ => skin.text_size,
    };
    size * skin.text_scale
}

#[test]
fn every_widget_text_is_a_whole_number_of_device_pixels_at_every_scale() {
    for scale_factor in SCALE_FACTORS {
        let mut app = app(scale_factor);
        widgets(&mut app);
        let skin = app.world().resource::<GooeySkin>().clone();
        let ui_scale = app.world().resource::<UiScale>().0;
        let mut query = app
            .world_mut()
            .query::<(&Role, &TextFont, &ComputedUiRenderTargetInfo)>();
        let mut roles = Vec::new();
        for (role, font, target) in query.iter(app.world()) {
            let FontSize::Px(size) = font.font_size else {
                panic!("{role:?}'s text size is not in logical px");
            };
            let scale = target.scale_factor();
            assert_eq!(
                scale,
                scale_factor * ui_scale,
                "the text is laid out at the window's scale factor times UiScale"
            );
            let device = size * scale;
            let unrounded = asked_by(role, &skin) * scale;
            assert!(
                (device - device.round()).abs() < ON_THE_GRID,
                "{role:?} at {scale}x is {device} device px, not a whole number"
            );
            assert!(
                device >= unrounded - ON_THE_GRID && device < unrounded + 1.0,
                "{role:?} at {scale}x is {device} device px against its own {unrounded}"
            );
            roles.push(*role);
        }
        for role in [Role::Heading, Role::CooldownText, Role::ButtonText] {
            assert!(
                roles.contains(&role),
                "at {scale_factor}x no {role:?} text was checked: {roles:?}"
            );
        }
    }
}

#[test]
fn a_body_text_between_two_pixels_is_set_at_the_next_one_up() {
    // 14 px body text at a 1.1 text size is 15.4 px; at 1.25x that is 19.25 device px, set at 20.
    let mut app = app(1.25);
    app.insert_resource(GooeySkin {
        text_size: 14.0,
        text_scale: TEXT_SCALE,
        ..GooeySkin::dark()
    });
    let words = app.world_mut().spawn(label("Volume")).id();
    app.world_mut().spawn(screen()).add_child(words);
    settle(&mut app);
    assert_eq!(app.world().resource::<UiScale>().0, 1.0);
    let font = app.world().get::<TextFont>(words).unwrap();
    assert_eq!(font.font_size, FontSize::Px(16.0));
}

#[test]
fn a_new_scale_sets_every_text_again() {
    let mut app = app(1.25);
    widgets(&mut app);
    let before = widget_texts(&mut app);
    for (size, scale) in &before {
        assert_eq!(*scale, 1.25);
        assert_eq!((size * scale).fract(), 0.0, "{size} px at 1.25x");
    }

    // The window moves to a 1.5x display, told as winit tells it.
    let (window, mut state) = app
        .world_mut()
        .query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>()
        .single_mut(app.world_mut())
        .unwrap();
    state.resolution = WindowResolution::new(1920, 1080).with_scale_factor_override(1.5);
    app.world_mut().write_message(WindowScaleFactorChanged {
        window,
        scale_factor: 1.5,
    });
    settle(&mut app);
    let asked = asked_sizes(app.world().resource::<GooeySkin>());
    let after = widget_texts(&mut app);
    assert_eq!(after.len(), before.len());
    for (size, scale) in after {
        assert_eq!(scale, 1.5);
        assert_on_the_grid(size, scale, &asked);
    }

    // A player's interface size moves UiScale, and the texts follow it too.
    app.insert_resource(UiScaleSettings {
        scale: 1.1,
        ..default()
    });
    settle(&mut app);
    let ui_scale = app.world().resource::<UiScale>().0;
    assert_ne!(ui_scale, 1.0);
    for (size, scale) in widget_texts(&mut app) {
        assert_eq!(scale, 1.5 * ui_scale);
        assert_on_the_grid(size, scale, &asked);
    }
}

#[test]
fn a_hosts_own_text_keeps_its_size() {
    let mut app = app(1.25);
    let own = app
        .world_mut()
        .spawn((
            Text::new("score 120"),
            TextFont {
                font_size: FontSize::Px(13.3),
                ..default()
            },
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(own);
    settle(&mut app);
    assert_eq!(
        app.world().get::<TextFont>(own).unwrap().font_size,
        FontSize::Px(13.3)
    );
}

#[test]
fn snap_text_size_rounds_up_to_the_next_whole_device_pixel_and_no_further() {
    for scale in [1.0, 1.1, 1.25, 4.0 / 3.0, 1.5, 1.75, 2.0, 2.5, 3.0] {
        for tenth in 50..400 {
            let size = tenth as f32 / 10.0;
            let snapped = snap_text_size(size, scale);
            let device = snapped * scale;
            // Exact where an `f32` allows it, else the least size above: never a hair short.
            assert!(
                device >= device.round() && device - device.round() < ON_THE_GRID,
                "{size} px at {scale}x: {device}"
            );
            assert!(
                device >= size * scale - ON_THE_GRID && device < size * scale + 1.0,
                "{size} px at {scale}x: {device} device px against {}",
                size * scale
            );
        }
    }
    // On the grid already: unchanged, not a pixel up.
    assert_eq!(snap_text_size(16.0, 1.5), 16.0);
    assert_eq!(snap_text_size(14.0, 1.0), 14.0);
    // Not a usable size or scale: returned as given.
    assert_eq!(snap_text_size(0.0, 1.5), 0.0);
    assert!(snap_text_size(f32::NAN, 1.5).is_nan());
    assert_eq!(snap_text_size(12.0, 0.0), 12.0);
}

#[test]
fn a_list_of_as_many_rows_as_it_stands_tall_does_not_scroll_when_its_text_is_rounded_up() {
    // The Desk's 13 px text is 16.25 device px at 1.25x and is set at 17: each row is that much
    // taller, and the list's height must count the rows as they are set.
    const ROWS: usize = 8;
    let mut app = app(1.25);
    app.insert_resource(GooeySkin::desk());
    let world = app.world_mut();
    let root = world
        .spawn(list(
            ROWS as f32,
            Children::spawn(SpawnIter((0..ROWS).map(|i| button(format!("Slot {i}"))))),
        ))
        .id();
    world.spawn(screen()).add_child(root);
    settle(&mut app);
    let mut views = app.world_mut().query::<(&ListView, &ComputedNode)>();
    let (_, view) = views.iter(app.world()).next().expect("the list has a view");
    let overflow = (view.content_size().y - view.size().y) * view.inverse_scale_factor();
    assert!(
        overflow < 0.5,
        "{ROWS} rows overflow a list {ROWS} rows tall by {overflow} logical px"
    );
}
