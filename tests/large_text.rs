//! The player's largest text size: every widget holds when its text is doubled. WCAG 2.2 SC 1.4.4
//! and the Xbox Accessibility Guideline 101 ask that text resize to 200% "without loss of
//! content or functionality", and that every step on the way keeps that promise (failure F69: text
//! or controls "clipped, truncated or obscured"). Laid out for real in a headless window as tall
//! as a stack of every widget needs.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 4800;

/// A headless window 1280 wide and tall enough that nothing is cut by the window itself; its
/// shorter side is the reference, so the interface is drawn at 1x and every px below is logical.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WIDTH, HEIGHT).with_scale_factor_override(1.0),
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
    .insert_resource(UiScaleSettings {
        reference_short_side: WIDTH as f32,
        scale: 1.0,
    });
    app.world_mut().spawn(Camera2d);
    let _ = WindowRef::Primary;
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// Where `entity` is drawn, in logical px.
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

/// Every widget a game builds a screen from, in one panel, with the longest labels a game is
/// likely to give them.
fn everything(app: &mut App) -> Entity {
    let panel = app
        .world_mut()
        .spawn((
            panel(),
            children![
                heading("Options and settings"),
                label("A line of body text, long enough to wrap inside a panel."),
                (
                    menu(),
                    children![button("Play"), button("Resume the last game")]
                ),
                label("Health"),
                meter(60.0, 100.0),
                toggle("Subtitles", true),
                choice("Quality", ["Low", "Medium", "Ultra high"], 1),
                slider("Music volume", 0.7, 0.0..=1.0),
                binding("Jump", Binding::Key(KeyCode::Space)),
                binding("Use", Binding::Pad(GamepadButton::South)),
                text_input("Save as", "Aria of the Long Name"),
                tabs(
                    ["Audio", "Display", "Controls"],
                    0,
                    children![
                        tab_page(children![toggle("Mono audio", false)]),
                        tab_page(children![choice("Window", ["Windowed", "Fullscreen"], 0)]),
                        tab_page(children![binding("Fire", Binding::Key(KeyCode::KeyF))]),
                    ]
                ),
                list(
                    3.5,
                    children![
                        button("Autosave"),
                        button("Meadow, 12:40"),
                        button("Harbour, 09:15"),
                        button("Mill, 03:02"),
                        button("Cellar, 01:48"),
                    ]
                ),
            ],
        ))
        .id();
    let root = app.world_mut().spawn(screen()).add_child(panel).id();
    settle(app);
    root
}

/// A part of the interface that lays out on its own, apart from its parent's content: its place is
/// the game's, so it is not checked against the parent.
fn free_standing(node: &Node) -> bool {
    node.position_type == PositionType::Absolute
}

/// Whether the node cuts what lies outside it (a list's viewport): its children may be past its edge.
fn clips(node: &Node) -> bool {
    node.overflow.x != OverflowAxis::Visible || node.overflow.y != OverflowAxis::Visible
}

/// Every child that leaves its parent's box by more than a pixel, as `"parent role -> child role"`
/// with how far, below `root`.
fn escapes(app: &App, root: Entity) -> Vec<String> {
    let world = app.world();
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(parent) = stack.pop() {
        let Some(node) = world.get::<Node>(parent) else {
            continue;
        };
        if clips(node) {
            continue;
        }
        let Some(children) = world.get::<Children>(parent) else {
            continue;
        };
        let outer = rect(app, parent);
        for child in children.iter() {
            let Some(child_node) = world.get::<Node>(child) else {
                continue;
            };
            if free_standing(child_node) || world.get::<ComputedNode>(child).is_none() {
                continue;
            }
            stack.push(child);
            let inner = rect(app, child);
            let (left, right) = (outer.min.x - inner.min.x, inner.max.x - outer.max.x);
            let (top, bottom) = (outer.min.y - inner.min.y, inner.max.y - outer.max.y);
            let worst = left.max(right).max(top).max(bottom);
            if worst > 1.0 {
                found.push(format!(
                    "{:?} {:?} -> {:?} {:?}: {worst:.1} px out (parent {outer:?}, child {inner:?})",
                    world.get::<Role>(parent),
                    parent,
                    world.get::<Role>(child),
                    child
                ));
            }
        }
    }
    found
}

fn at_scale(scale: f32) -> (App, Entity) {
    let mut app = app();
    app.world_mut().resource_mut::<GooeySkin>().text_scale = scale;
    let root = everything(&mut app);
    (app, root)
}

#[test]
fn probe_every_step() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let (app, root) = at_scale(scale);
        let out = escapes(&app, root);
        println!("scale {scale}: {} escapes", out.len());
        for line in &out {
            println!("  {line}");
        }
    }
}

#[test]
fn probe_detector_sees_a_squeezed_panel() {
    let (mut app, root) = at_scale(2.0);
    let panel = app.world().get::<Children>(root).unwrap()[0];
    let before = rect(&app, panel);
    println!("panel at 2.0: {before:?}");
    let mut n = 0;
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        n += 1;
        if let Some(c) = app.world().get::<Children>(e) {
            stack.extend(c.iter());
        }
    }
    println!("nodes: {n}");
    app.world_mut().get_mut::<Node>(panel).unwrap().width = Val::Px(300.0);
    settle(&mut app);
    let out = escapes(&app, root);
    println!("squeezed: {} escapes", out.len());
    for line in out.iter().take(8) {
        println!("  {line}");
    }
}
