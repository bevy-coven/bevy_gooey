//! A tooltip with a shortcut holds still: once its box is open, its rectangle and its keycap's stay
//! identical frame after frame, at every window scale factor and `UiScale` (#79). The keycap's
//! centring, the cap width the box is sized from and the skin that restyles the box once fed each
//! other, so the box flickered between two widths a few logical px apart.

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::text::TextLayoutInfo;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A headless window of `physical` px at `scale_factor`, the player's interface size on top.
fn app(physical: (u32, u32), scale_factor: f32, interface_size: f32) -> App {
    let mut app = App::new();
    app.insert_resource(MotionSettings::off());
    app.insert_resource(TooltipSettings { delay_seconds: 0.0 });
    app.insert_resource(UiScaleSettings {
        scale: interface_size,
        ..default()
    });
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(physical.0, physical.1)
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
    ));
    app.world_mut().spawn(Camera2d);
    app
}

/// What a frame shows of the open box: the box's and the cap's laid-out size and place, physical.
type Frame = ([f32; 2], [f32; 2], [f32; 2], [f32; 2], Vec<f32>);

fn frame(app: &App, tip: Entity, cap: Entity) -> Frame {
    let of = |e: Entity| {
        let size = app.world().get::<ComputedNode>(e).expect("laid out").size();
        let at = app
            .world()
            .get::<UiGlobalTransform>(e)
            .expect("placed")
            .translation;
        ([size.x, size.y], [at.x, at.y])
    };
    let (box_size, box_at) = of(tip);
    let (cap_size, cap_at) = of(cap);
    let glyphs = app
        .world()
        .get::<TextLayoutInfo>(cap)
        .map(|info| info.glyphs.iter().map(|g| g.position.x).collect())
        .unwrap_or_default();
    (box_size, box_at, cap_size, cap_at, glyphs)
}

fn key_cap(app: &App, tip: Entity) -> Entity {
    app.world()
        .get::<Children>(tip)
        .expect("the box's children")
        .iter()
        .find(|&c| app.world().get::<Role>(c) == Some(&Role::PromptKey))
        .expect("the shortcut's keycap")
}

#[test]
fn an_open_tooltip_with_a_shortcut_stays_still_at_every_scale() {
    // (physical window, window scale factor, the player's interface size)
    for (physical, scale_factor, interface_size) in [
        ((1280, 720), 1.0, 1.0),
        ((2560, 1406), 2.0, 1.2),
        ((1920, 1080), 1.5, 1.0),
        ((2560, 1440), 2.0, 1.5),
    ] {
        let mut app = app(physical, scale_factor, interface_size);
        let world = app.world_mut();
        let left = world.spawn(button("Place")).id();
        let erase = world
            .spawn((
                button("Erase"),
                Tooltip::new("Remove mode").with_shortcut(Shortcut::new(Modifiers::NONE, KeyCode::KeyX)),
            ))
            .id();
        let row = world
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .add_children(&[left, erase])
            .id();
        world.spawn(screen()).add_child(row);
        for _ in 0..6 {
            app.update();
        }
        // The pointer rests on the button (a window position is logical).
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .expect("one primary window");
        let at = app
            .world()
            .get::<UiGlobalTransform>(erase)
            .expect("placed")
            .translation
            / scale_factor;
        let target = WindowRef::Entity(window)
            .normalize(None)
            .expect("a window ref");
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::Window(target),
                position: at,
            },
            PointerAction::Move { delta: Vec2::ZERO },
        ));
        for _ in 0..10 {
            app.update();
        }
        let tip = app
            .world_mut()
            .query_filtered::<Entity, With<TooltipBox>>()
            .single(app.world())
            .expect("one tooltip box");
        assert_ne!(
            app.world().get::<Node>(tip).expect("a node").display,
            Display::None,
            "the box is open"
        );
        let cap = key_cap(&app, tip);
        let first = frame(&app, tip, cap);
        assert!(!first.4.is_empty(), "the cap's words are shaped");
        for n in 1..=30 {
            app.update();
            assert_eq!(
                frame(&app, tip, cap),
                first,
                "{physical:?} at {scale_factor}x, interface {interface_size}: frame {n} differs"
            );
        }
        // And the words are still in the middle of their cap (the run's own width, from the content
        // box, against the node's middle; physical px, so half a device pixel of slack).
        let node = app.world().get::<ComputedNode>(cap).expect("laid out");
        let run = app.world().get::<TextLayoutInfo>(cap).expect("a run").size.x;
        let off = node.content_box().min.x + run / 2.0 - node.border_box().center().x;
        assert!(
            off.abs() <= 0.6,
            "{physical:?} at {scale_factor}x, interface {interface_size}: the words are {off} px off the middle"
        );
    }
}
