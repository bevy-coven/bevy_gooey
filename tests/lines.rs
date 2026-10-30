//! Every line is one weight: at any window scale, each border (a panel's, a button's focus ring, a
//! meter's frame, a tooltip's) and the meter's gap are a whole number of device pixels, the same on
//! all four sides, and the meter's fill sits centred in its slot. Laid out for real in a headless
//! window.

use bevy::input_focus::{AutoFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A headless app with a `width` × `height` window, so UI layout runs.
fn app(width: u32, height: u32) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(width, height)
                        .with_scale_factor_override(1.0),
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
    .insert_resource(InputFocusVisible(true));
    app.world_mut().spawn(Camera2d);
    app
}

/// The gallery's menu, with Options focused so its tooltip is open, and a meter at 60. Returns the
/// meter.
fn gallery(app: &mut App) -> Entity {
    let world = app.world_mut();
    let play = world.spawn(button("Play")).id();
    let options = world
        .spawn((
            button("Options"),
            Tooltip("Sound, controls and display".into()),
            AutoFocus,
        ))
        .id();
    let quit = world.spawn(button("Quit")).id();
    let column = world
        .spawn(menu())
        .add_children(&[play, options, quit])
        .id();
    let health = world.spawn(meter(60.0, 100.0)).id();
    let title = world.spawn(heading("bevy_gooey")).id();
    let card = world
        .spawn(panel())
        .add_children(&[title, column, health])
        .id();
    world.spawn(screen()).add_child(card);
    settle(app);
    health
}

/// The gallery's Quit dialog over it: a centred panel with a row of two buttons.
fn open_dialog(app: &mut App) {
    let world = app.world_mut();
    let leave = world.spawn(button("Leave")).id();
    let stay = world.spawn(button("Stay")).id();
    let row = world
        .spawn(Node {
            column_gap: Val::Px(10.0),
            ..default()
        })
        .add_children(&[leave, stay])
        .id();
    let question = world.spawn(label("Leave the game?")).id();
    let card = world.spawn(panel()).add_children(&[question, row]).id();
    world.spawn(dialog()).add_child(card);
    settle(app);
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

fn resize(app: &mut App, width: f32, height: f32) {
    let mut windows = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(app.world_mut())
        .expect("one primary window")
        .resolution
        .set(width, height);
    app.update();
    app.update();
}

/// Every framed widget on screen has a `device` px border on all four sides, and the meter a
/// `device` px gap; `framed` of them are on screen (a shut tooltip is not).
fn assert_lines(app: &mut App, device: f32, framed: usize, at: &str) {
    let whole = (Vec2::splat(device), Vec2::splat(device));
    let mut nodes = app.world_mut().query::<(&Role, &ComputedNode)>();
    let mut seen = 0;
    for (role, node) in nodes.iter(app.world()) {
        let lined = matches!(
            role,
            Role::Panel | Role::Button | Role::Tooltip | Role::Meter
        );
        if !lined || node.is_empty() {
            continue;
        }
        let border = node.border;
        assert_eq!(
            (border.min_inset, border.max_inset),
            whole,
            "{at}: a {role:?}'s border, (left, top) and (right, bottom)"
        );
        if *role == Role::Meter {
            let gap = node.padding;
            assert_eq!(
                (gap.min_inset, gap.max_inset),
                whole,
                "{at}: the meter's gap, (left, top) and (right, bottom)"
            );
        }
        seen += 1;
    }
    assert_eq!(seen, framed, "{at}: framed widgets on screen");
}

/// The fill sits two lines (frame and gap) inside the meter's edge, above, below and on the left.
fn assert_fill_centred(app: &App, meter: Entity, device: f32, at: &str) {
    let rect = |entity: Entity| {
        let node = app.world().get::<ComputedNode>(entity).expect("laid out");
        let at = app
            .world()
            .get::<UiGlobalTransform>(entity)
            .expect("placed")
            .translation;
        Rect::from_center_size(at, node.size())
    };
    let fill = app.world().get::<Children>(meter).expect("a fill")[0];
    let (track, fill) = (rect(meter), rect(fill));
    assert_eq!(
        fill.min.x - track.min.x,
        2.0 * device,
        "{at}: left of the fill"
    );
    assert_eq!(
        fill.min.y - track.min.y,
        2.0 * device,
        "{at}: above the fill"
    );
    assert_eq!(
        track.max.y - fill.max.y,
        2.0 * device,
        "{at}: below the fill"
    );
}

#[test]
fn every_line_is_one_whole_weight_at_every_window_scale() {
    // Window, and the default 3 px border in device px: rounded down where 3 × scale is not whole.
    for (width, height, device) in [
        (1280, 720, 3.0),
        (1280, 800, 3.0),
        (1600, 900, 3.0),
        (1920, 1080, 4.0),
        (2560, 1440, 6.0),
        (3840, 2160, 9.0),
    ] {
        let at = format!("{width}x{height}");
        let mut app = app(width, height);
        let meter = gallery(&mut app);
        // A panel, three buttons, the meter and Options' tooltip.
        assert_lines(&mut app, device, 6, &at);
        assert_fill_centred(&app, meter, device, &at);
        // Two panels, five buttons and the meter: the dialog took focus, so the tooltip is shut.
        open_dialog(&mut app);
        assert_lines(&mut app, device, 8, &format!("{at}, dialog open"));
    }
}

#[test]
fn a_resized_window_redraws_its_lines_whole_at_the_new_scale() {
    let mut app = app(1280, 720);
    let meter = gallery(&mut app);
    assert_lines(&mut app, 3.0, 6, "1280x720");
    resize(&mut app, 1920.0, 1080.0);
    assert_lines(&mut app, 4.0, 6, "resized to 1920x1080");
    assert_fill_centred(&app, meter, 4.0, "resized to 1920x1080");
}

#[test]
fn the_players_interface_size_snaps_its_lines_too() {
    // 1.25 × 1.5 = 1.875: 3 px is 5.625 device px, drawn 5.
    let mut app = app(1920, 1080);
    app.insert_resource(UiScaleSettings {
        scale: 1.25,
        ..default()
    });
    let meter = gallery(&mut app);
    assert_lines(&mut app, 5.0, 6, "1920x1080 at interface size 1.25");
    assert_fill_centred(&app, meter, 5.0, "1920x1080 at interface size 1.25");
}
