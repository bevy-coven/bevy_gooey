//! A rebuilt, re-skinned strip glides every tab, not only the first (G-83, #94): the skin's first
//! look at a node is not a change in layout that "lands" a move. Found by measurement in
//! bevy_dark_factory: a dock strip whose tabs were spawned again and reordered moved its leading
//! tab over the glide and the one behind it in two frames, because the new tabs were laid out once
//! before they were skinned at the window's scale and again after, the leading tab grew, and the
//! second tab's corner moved inside `LAYOUT_QUIET_FRAMES`. On a clock the test sets (30 ms a frame).

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const FRAME: f32 = 0.03;

/// The frames a tab that moves must take to arrive, at the least: the skin's default spring runs for
/// well over this, and a tab that lands in two frames is far under it.
const LEAST_FRAMES: usize = 6;

/// The skin the issue measured under: 11 px text and a 1 px line, which at an interface size of 1.2
/// snap to 11.67 and 0.83 px.
fn skin() -> GooeySkin {
    let mut skin = GooeySkin::tool();
    skin.text_size = 11.0;
    skin.border = 1.0;
    skin
}

fn app(scale: f32) -> App {
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
    ));
    app.insert_resource(skin());
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().resource_mut::<UiScaleSettings>().scale = scale;
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// The titles of the dock's panels, in the order the group starts in.
const TITLES: [(&str, &str); 3] = [("a", "Alpha"), ("b", "Bravo"), ("c", "Charlie")];

/// A dock of one group of three tabs; returns the dock.
fn rig(app: &mut App) -> Entity {
    for (id, title) in TITLES {
        app.world_mut()
            .resource_mut::<PanelRegistry>()
            .register(id, title, move |page| {
                page.with_child(label(title));
            });
    }
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(1000.0),
                height: Val::Px(500.0),
                ..default()
            },
            children![dock(DockLayout::group(["a", "b", "c"]))],
        ))
        .id();
    frames(app, 30);
    app.world().get::<Children>(container).expect("dock")[0]
}

fn tab(app: &mut App, title: &str) -> Entity {
    let mut query = app.world_mut().query::<(&Text, &Role, &ChildOf)>();
    query
        .iter(app.world())
        .find(|(text, role, _)| **role == Role::TabText && text.0 == title)
        .map(|(_, _, parent)| parent.parent())
        .unwrap_or_else(|| panic!("no tab titled {title}"))
}

/// Where a tab's centre is drawn now, in layout px: travel included.
fn centre_x(app: &mut App, title: &str) -> f32 {
    let entity = tab(app, title);
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    at.x * node.inverse_scale_factor()
}

/// Each tab's centre on each frame after the dock's tabs are reordered to `order`: the row
/// `before` is the frame it was written in.
fn reorder_trace(scale: f32, order: [&str; 3]) -> Vec<(&'static str, Vec<f32>)> {
    let mut app = app(scale);
    let dock = rig(&mut app);
    let titles = TITLES.map(|(_, title)| title);
    let mut trace: Vec<(&'static str, Vec<f32>)> = titles
        .iter()
        .map(|title| (*title, vec![centre_x(&mut app, title)]))
        .collect();
    app.world_mut()
        .entity_mut(dock)
        .insert(Dock(DockLayout::group(order)));
    for _ in 0..60 {
        app.update();
        for (title, xs) in &mut trace {
            let x = centre_x(&mut app, title);
            xs.push(x);
        }
    }
    trace
}

/// Every tab that has somewhere to go takes at least [`LEAST_FRAMES`] frames to get there and never
/// covers more than half the way in one.
fn assert_every_moved_tab_glides(trace: &[(&str, Vec<f32>)], what: &str) {
    let mut moved = 0;
    for (title, xs) in trace {
        let (start, end) = (xs[0], xs[xs.len() - 1]);
        let distance = (end - start).abs();
        if distance < 4.0 {
            continue;
        }
        moved += 1;
        let widest = xs
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            widest <= distance / 2.0 + 0.5,
            "{what}: {title} covered {widest} of its {distance} px in one frame ({xs:?})"
        );
        let arrived = xs
            .iter()
            .position(|x| (x - end).abs() < 1.0)
            .expect("the tab arrives");
        assert!(
            arrived >= LEAST_FRAMES,
            "{what}: {title} arrived in {arrived} frames, under {LEAST_FRAMES} ({xs:?})"
        );
    }
    assert!(moved >= 2, "{what}: the tabs that swap move ({moved})");
}

#[test]
fn every_tab_of_a_reordered_strip_glides_at_interface_size_one() {
    let trace = reorder_trace(1.0, ["c", "a", "b"]);
    assert_every_moved_tab_glides(&trace, "UiScale 1.0");
}

#[test]
fn every_tab_of_a_reordered_strip_glides_at_interface_size_one_point_two() {
    let trace = reorder_trace(1.2, ["c", "a", "b"]);
    assert_every_moved_tab_glides(&trace, "UiScale 1.2");
}

#[test]
fn every_tab_glides_when_the_order_is_reversed() {
    let trace = reorder_trace(1.2, ["c", "b", "a"]);
    assert_every_moved_tab_glides(&trace, "reversed");
}

/// What the skin wrote to a node and the size it was laid out at: its frame (border), its inset
/// (padding), its text size and its `ComputedNode` size.
#[derive(Debug, PartialEq)]
struct Look {
    border: UiRect,
    padding: UiRect,
    font: Option<FontSize>,
    size: Vec2,
}

/// The look of every node with a role, in a stable order, with a name for the report.
fn looks(app: &mut App) -> Vec<(String, Look)> {
    let mut query = app.world_mut().query::<(
        Entity,
        &Role,
        &Node,
        &ComputedNode,
        Option<&TextFont>,
        Option<&Text>,
    )>();
    let mut all: Vec<(Entity, String, Look)> = query
        .iter(app.world())
        .map(|(entity, role, node, computed, font, text)| {
            (
                entity,
                format!("{role:?} {:?}", text.map(|t| t.0.clone())),
                Look {
                    border: node.border,
                    padding: node.padding,
                    font: font.map(|f| f.font_size),
                    size: computed.size(),
                },
            )
        })
        .collect();
    all.sort_by_key(|(entity, _, _)| *entity);
    all.into_iter()
        .map(|(_, name, look)| (name, look))
        .collect()
}

#[test]
fn a_freshly_spawned_skinned_widget_keeps_its_look_on_the_second_frame() {
    for scale in [1.0, 1.2, 2.0] {
        let mut app = app(scale);
        frames(&mut app, 4);
        let known = looks(&mut app).len();
        app.world_mut().spawn((
            screen(),
            children![(
                panel(),
                children![
                    heading("Kit"),
                    label("A label"),
                    button("A button"),
                    tabs(
                        ["One", "Two", "Three"],
                        0,
                        children![
                            tab_page(children![label("a")]),
                            tab_page(children![label("b")]),
                            tab_page(children![label("c")]),
                        ],
                    ),
                ],
            )],
        ));
        app.update();
        let first = looks(&mut app);
        app.update();
        let second = looks(&mut app);
        assert!(first.len() > known, "the widget was laid out on frame 1");
        assert_eq!(first.len(), second.len());
        let changed: Vec<String> = first
            .iter()
            .zip(second.iter())
            .filter(|((_, was), (_, now))| {
                was.border != now.border
                    || was.padding != now.padding
                    || was.font != now.font
                    || (was.size - now.size).abs().max_element() >= 0.01
            })
            .map(|((name, was), (_, now))| format!("{name}: {was:?} -> {now:?}"))
            .collect();
        assert!(
            changed.is_empty(),
            "UiScale {scale}: these nodes were skinned or laid out again on frame 2: {changed:#?}"
        );
    }
}
