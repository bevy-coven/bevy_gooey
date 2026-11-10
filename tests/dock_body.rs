//! A dock the host places itself: a bare `Dock(layout)` on its own node (a bottom strip, a
//! sidebar) grows the body the layout is built in, so its registered panels are shown and it can
//! still be a drop target. Laid out for real in a headless window.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn app() -> App {
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
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn children(app: &App, entity: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

fn descendants(app: &App, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        for child in children(app, entity) {
            found.push(child);
            stack.push(child);
        }
    }
    found
}

/// The part of `root` (and everything under it) with `role`.
fn part(app: &App, root: Entity, role: Role) -> Option<Entity> {
    descendants(app, root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
}

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

/// The panels of the registry, registered once per app: a title and how often its page was filled.
fn register(app: &mut App, id: &'static str, title: &'static str) -> Arc<AtomicUsize> {
    let spawned = Arc::new(AtomicUsize::new(0));
    let count = spawned.clone();
    app.world_mut()
        .resource_mut::<PanelRegistry>()
        .register(id, title, move |page| {
            count.fetch_add(1, Ordering::SeqCst);
            page.with_child(label(title));
        });
    spawned
}

/// A bottom strip: the host's own `Node` on the dock entity, a `Dock` beside it, and a drop target.
fn bottom_strip(app: &mut App, layout: DockLayout) -> Entity {
    let world = app.world_mut();
    let dock = world
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(200.0),
                flex_shrink: 0.0,
                ..default()
            },
            Dock(layout),
            DropTarget::<String>::new(),
        ))
        .id();
    let ground = world.spawn(label("the host's own view")).id();
    let root = world
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .add_children(&[ground, dock])
        .id();
    world.spawn(screen()).add_child(root);
    settle(app);
    dock
}

/// The acceptance: a bottom dock the host built from a bare `Dock` grows its own body, and the
/// panel registered for it is built under that body.
#[test]
fn a_bare_dock_grows_a_body_and_holds_its_panels() {
    let mut app = app();
    let spawned = register(&mut app, "reload", "Reload");
    let dock = bottom_strip(&mut app, DockLayout::group(["reload"]));

    let body = children(&app, dock)
        .into_iter()
        .find(|&kid| app.world().get::<Role>(kid) == Some(&Role::DockBody))
        .expect("the dock grew a body");
    // The panel's page, and the widgets its spawn function added, stand under the body.
    let titles: Vec<String> = descendants(&app, dock)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&Role::TabText))
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert_eq!(titles, vec!["Reload".to_owned()], "the panel's tab");
    let page = part(&app, dock, Role::TabPage).expect("a page");
    let words = descendants(&app, page)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect::<Vec<_>>();
    assert!(
        words.contains(&"Reload".to_owned()),
        "the panel's own widgets: {words:?}"
    );
    assert!(descendants(&app, body).contains(&page), "under the body");
    assert_eq!(spawned.load(Ordering::SeqCst), 1, "its spawn ran once");
}

/// The body fills the node the host made, so a bottom strip is 200 px high whatever the layout in
/// it, and the host's own node is left exactly as it wrote it.
#[test]
fn the_body_fills_the_hosts_node_and_leaves_it_alone() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    let dock = bottom_strip(&mut app, DockLayout::group(["reload"]));

    let node = app.world().get::<Node>(dock).expect("the host's node");
    assert_eq!(node.height, Val::Px(200.0), "the host's height, untouched");
    assert_eq!(node.width, Val::Percent(100.0));

    let body = part(&app, dock, Role::DockBody).expect("a body");
    let (outer, inner) = (rect(&app, dock), rect(&app, body));
    assert!(
        (outer.width() - inner.width()).abs() < 0.5,
        "the body is the dock's width: {outer:?} {inner:?}"
    );
    assert!(
        (outer.height() - inner.height()).abs() < 0.5,
        "and its height: {outer:?} {inner:?}"
    );
}

/// The dock is still the host's to make a drop target: the body grows inside it and takes nothing
/// away from what the host put on the dock.
#[test]
fn the_host_can_still_make_a_bare_dock_a_drop_target() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    let dock = bottom_strip(&mut app, DockLayout::group(["reload"]));
    assert!(
        app.world().get::<DropTarget<String>>(dock).is_some(),
        "the host's target is still on the dock"
    );
    assert!(
        app.world().get::<DropTarget<String>>(dock).is_some(),
        "and only one target, never a second on its body"
    );
    let body = part(&app, dock, Role::DockBody).expect("a body");
    assert!(
        app.world().get::<DropTarget<String>>(body).is_none(),
        "the body the dock grew carries none of the host's targets"
    );
}

/// A dock with no panel registered for its layout builds an empty body and says so once per id,
/// rather than building nothing or panicking.
#[test]
fn a_bare_dock_with_no_panels_shows_nothing_and_reports_it() {
    let mut app = app();
    #[derive(Resource, Default)]
    struct Skipped(Vec<String>);
    app.init_resource::<Skipped>();
    app.add_observer(|skip: On<DockPanelSkipped>, mut seen: ResMut<Skipped>| {
        seen.0.push(skip.id.clone());
    });
    let dock = bottom_strip(&mut app, DockLayout::group(["nobody"]));

    let body = part(&app, dock, Role::DockBody).expect("a body even with nothing to show");
    assert!(children(&app, body).is_empty(), "nothing was built in it");
    assert_eq!(
        app.world().resource::<Skipped>().0,
        vec!["nobody".to_owned()],
        "the missing panel is reported once"
    );
}

/// A panel registered after the dock stands appears without the host rebuilding it.
#[test]
fn a_panel_registered_later_appears_in_a_bare_dock() {
    let mut app = app();
    let dock = bottom_strip(&mut app, DockLayout::group(["late"]));
    settle(&mut app);
    assert!(part(&app, dock, Role::TabPage).is_none(), "nothing yet");

    register(&mut app, "late", "Late");
    settle(&mut app);
    let page = part(&app, dock, Role::TabPage).expect("the panel appeared");
    let words = descendants(&app, page)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect::<Vec<_>>();
    assert!(words.contains(&"Late".to_owned()), "{words:?}");
}

/// A dock spawned whole by `dock()` keeps the body it was born with, and gets no second one.
#[test]
fn a_full_dock_keeps_its_one_body() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(800.0),
                height: Val::Px(400.0),
                ..default()
            },
            children![dock(DockLayout::group(["reload"]))],
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(container);
    settle(&mut app);
    let entity = children(&app, container)[0];
    let bodies: Vec<Entity> = children(&app, entity)
        .into_iter()
        .filter(|&kid| app.world().get::<Role>(kid) == Some(&Role::DockBody))
        .collect();
    assert_eq!(bodies.len(), 1, "one body, not two");
}
