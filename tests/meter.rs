//! A meter is one framed shape: its fill sits inside the frame with a gap all round, rounded
//! concentrically, as long as its value says, and the whole bar grows with the player's text size.
//! Laid out for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A headless app with a real window size, so UI layout runs.
fn app(skin: GooeySkin) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
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
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

/// A meter reading `value` of 100 in a panel, laid out; returns the track and its fill.
fn laid_out_meter(app: &mut App, value: f32) -> (Entity, Entity) {
    let track = app.world_mut().spawn(meter(value, 100.0)).id();
    let panel = app.world_mut().spawn(panel()).add_child(track).id();
    app.world_mut().spawn(screen()).add_child(panel);
    for _ in 0..3 {
        app.update();
    }
    let fill = app.world().get::<Children>(track).expect("a fill")[0];
    (track, fill)
}

/// Where `entity` is drawn, in logical px (the window's scale factor is 1).
fn rect(app: &App, entity: Entity) -> Rect {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size();
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    Rect::from_center_size(at, size)
}

#[test]
fn a_fill_sits_inside_its_frame_with_a_gap_all_round_and_concentric_corners() {
    let skin = GooeySkin::default();
    let mut app = app(skin.clone());
    let (track, fill) = laid_out_meter(&mut app, 60.0);
    let (outer, inner) = (rect(&app, track), rect(&app, fill));
    let inset = 2.0 * skin.border;
    // The frame and the gap on every side: the fill never touches the frame or pokes out.
    assert!(
        (inner.min.x - outer.min.x - inset).abs() < 0.5,
        "{outer:?} {inner:?}"
    );
    assert!(
        (inner.min.y - outer.min.y - inset).abs() < 0.5,
        "{outer:?} {inner:?}"
    );
    assert!(
        (outer.max.y - inner.max.y - inset).abs() < 0.5,
        "{outer:?} {inner:?}"
    );
    // As long as its value says, of the room inside the frame and gap.
    let room = outer.width() - 2.0 * inset;
    assert!(
        (inner.width() - 0.6 * room).abs() < 1.0,
        "{outer:?} {inner:?}"
    );
    // Framed in a colour the panel cannot hide.
    assert_eq!(
        app.world().get::<BorderColor>(track),
        Some(&BorderColor::all(skin.outline))
    );

    // Its corners are the frame's curve less the frame and the gap (Bevy draws a border's inner
    // curve at the outer radius less the border, as CSS does), and still round.
    let world = app.world();
    let frame = world.get::<ComputedNode>(track).expect("laid out");
    let fill = world.get::<ComputedNode>(fill).expect("laid out");
    let frame_outer = frame.border_radius.top_left.x;
    let fill_corner = fill.border_radius.top_left.x;
    assert!(fill_corner > 0.0, "a square fill: {fill_corner}");
    assert!(
        (frame_outer - inset - fill_corner).abs() < 0.01,
        "frame's radius {frame_outer}, fill's {fill_corner}"
    );
}

#[test]
fn a_meter_grows_with_the_players_text_scale() {
    let skin = GooeySkin::default();
    let mut normal = app(skin.clone());
    let (track, fill) = laid_out_meter(&mut normal, 60.0);
    let (normal_track, normal_fill) = (rect(&normal, track), rect(&normal, fill));

    let mut large = app(GooeySkin {
        text_scale: 1.5,
        ..skin.clone()
    });
    let (track, fill) = laid_out_meter(&mut large, 60.0);
    let (large_track, large_fill) = (rect(&large, track), rect(&large, fill));

    assert!((normal_track.height() - skin.meter_height).abs() < 0.5);
    assert!((large_track.height() - 1.5 * skin.meter_height).abs() < 0.5);
    // The frame and gap stay one border each, so the fill takes all the growth.
    let frames = 4.0 * skin.border;
    assert!((large_fill.height() - (1.5 * skin.meter_height - frames)).abs() < 0.5);
    assert!(large_fill.height() > normal_fill.height());
    assert!(large_track.width() >= 1.5 * normal_track.width() - 0.5);
}

#[test]
fn a_meter_near_empty_still_draws_a_sliver_and_an_empty_one_draws_none() {
    let mut app = app(GooeySkin::default());
    let (_, sliver) = laid_out_meter(&mut app, 2.0);
    let (_, empty) = laid_out_meter(&mut app, 0.0);
    let sliver = rect(&app, sliver);
    assert!(sliver.width() > 2.0 && sliver.width() < 6.0, "{sliver:?}");
    assert_eq!(rect(&app, empty).width(), 0.0);
}
