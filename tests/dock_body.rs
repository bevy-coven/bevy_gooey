//! A dock the host places itself: a bare `Dock(layout)` on its own node (a bottom strip, a
//! sidebar) grows the body the layout is built in, so its registered panels are shown and it can
//! still be a drop target. Laid out for real in a headless window.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
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
fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

/// A pointer position for a point of the layout: the layout is the window's size over `UiScale`.
fn at(app: &App, layout: Vec2) -> Vec2 {
    layout * app.world().resource::<UiScale>().0
}

fn location(app: &mut App, at: Vec2) -> Location {
    let window = window(app);
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let position = location(app, at);
    app.world_mut().write_message(PointerInput {
        pointer_id: PointerId::Mouse,
        location: position,
        action,
    });
    app.update();
}

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

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

/// The host's own children stand beside the grown body, untouched, and the dock carries the crate's
/// own `Role::Dock` so every dock is a dock.
#[test]
fn the_hosts_children_stand_beside_the_grown_body() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    let world = app.world_mut();
    let dock = world
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(220.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    // The host's own bits, before the dock exists as one: a header label and its own child.
    let header = world.spawn(label("Reloading")).id();
    world.entity_mut(dock).add_child(header);
    world.entity_mut(dock).insert((
        Dock(DockLayout::group(["reload"])),
        DropTarget::<String>::new(),
    ));
    let root = world
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    world.entity_mut(root).add_children(&[dock]);
    world.spawn(screen()).add_child(root);
    settle(&mut app);

    let kids = children(&app, dock);
    assert!(kids.contains(&header), "the host's label is still a child");
    assert_eq!(
        app.world()
            .get::<Children>(dock)
            .expect("children")
            .iter()
            .filter(|&kid| kid == header)
            .count(),
        1,
        "exactly once"
    );
    assert!(
        app.world().get::<Node>(header).expect("a node").display != Display::None,
        "and it is still shown"
    );
    assert!(
        part(&app, dock, Role::DockBody).is_some(),
        "with a body too"
    );
    assert_eq!(
        app.world().get::<Role>(dock),
        Some(&Role::Dock),
        "the dock carries the crate's role, which the host did not set"
    );
}

/// A piece dropped on a bare dock reaches the host's own `DropTarget`, which still stands on the
/// dock after its body grew.
#[test]
fn a_drop_on_a_bare_dock_reaches_the_hosts_target() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    let dock = bottom_strip(&mut app, DockLayout::group(["reload"]));
    assert!(
        app.world().get::<DropTarget<String>>(dock).is_some(),
        "the host's target is still on the dock"
    );

    #[derive(Resource, Default)]
    struct Heard(Vec<String>);
    app.init_resource::<Heard>();
    app.add_observer(|dropped: On<Dropped<String>>, mut heard: ResMut<Heard>| {
        heard.0.push(dropped.payload.clone());
    });

    // A source beside the dock, let go over the middle of the dock's card.
    let world = app.world_mut();
    let source = world
        .spawn((
            Node {
                width: Val::Px(60.0),
                height: Val::Px(30.0),
                ..default()
            },
            DragPayload("a piece".to_owned()),
        ))
        .id();
    let holder = world
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(60.0),
            ..default()
        })
        .add_child(source)
        .id();
    // The dock moves under the holder: a column of the holder over the strip.
    let root = world
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let parent = world
        .get::<ChildOf>(dock)
        .expect("the dock's parent")
        .parent();
    world.entity_mut(dock).remove::<ChildOf>();
    world.entity_mut(root).add_children(&[holder, dock]);
    world.entity_mut(parent).add_child(root);
    settle(&mut app);

    let from = at(&app, rect(&app, source).center());
    let to = at(
        &app,
        rect(&app, part(&app, dock, Role::DockBody).expect("a body")).center(),
    );
    point_at(&mut app, from);
    pointer(&mut app, from, PointerAction::Press(PointerButton::Primary));
    // Past the lift distance first, then over the dock: the lift is 6 px of travel.
    pointer(
        &mut app,
        from + Vec2::X * 12.0,
        PointerAction::Move {
            delta: Vec2::X * 12.0,
        },
    );
    settle(&mut app);
    pointer(&mut app, to, PointerAction::Move { delta: to - from });
    settle(&mut app);
    pointer(&mut app, to, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);

    assert_eq!(
        app.world().resource::<Heard>().0,
        vec!["a piece".to_owned()],
        "the drop reached the host's target on the dock"
    );
}

/// A body the host despawned grows again, and the dock builds its panels into the new one rather
/// than skipping the rebuild because the layout it holds did not change.
#[test]
fn a_despawned_body_grows_again_with_its_panels() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    let dock = bottom_strip(&mut app, DockLayout::group(["reload"]));
    let first = part(&app, dock, Role::DockBody).expect("a body");
    assert!(part(&app, dock, Role::TabPage).is_some(), "with a page");

    app.world_mut().entity_mut(first).despawn();
    settle(&mut app);

    let body = part(&app, dock, Role::DockBody).expect("a new body");
    assert_ne!(body, first, "a new one");
    let page = part(&app, dock, Role::TabPage).expect("the panel is built into it again");
    assert!(
        descendants(&app, body).contains(&page),
        "the page stands under the new body"
    );
}

/// **The acceptance the review found missing:** a tab dragged in a host-placed dock shows the
/// landing the panel will take and the card with its title, so rearranging a bottom dock looks like
/// rearranging any other.
#[test]
fn a_tab_dragged_in_a_bare_dock_lights_a_landing_and_carries_its_title() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    register(&mut app, "output", "Output");
    let dock = bottom_strip(
        &mut app,
        DockLayout::split(
            SplitAxis::Horizontal,
            0.5,
            DockLayout::group(["reload", "output"]),
            DockLayout::group(["reload"]),
        ),
    );

    // The dock's own parts: the landing it lights and the ghost it carries, both hidden at rest.
    let landing = part(&app, dock, Role::DockDrop).expect("the dock grew a landing");
    let ghost = part(&app, dock, Role::DockGhost).expect("and a ghost");
    assert_eq!(
        app.world().get::<Node>(landing).expect("a node").display,
        Display::None,
        "hidden until a tab is carried"
    );
    assert_eq!(
        app.world().get::<Node>(ghost).expect("a node").display,
        Display::None
    );

    // Pick up the "Reload" tab and carry it towards the other group.
    let tab = descendants(&app, dock)
        .into_iter()
        .find(|&e| {
            app.world().get::<Role>(e) == Some(&Role::TabText)
                && app.world().get::<Text>(e).map(|t| t.0.clone()) == Some("Reload".to_owned())
        })
        .map(|text| app.world().get::<ChildOf>(text).expect("a tab").parent())
        .expect("the Reload tab");
    let from = at(&app, rect(&app, tab).center());
    point_at(&mut app, from);
    pointer(&mut app, from, PointerAction::Press(PointerButton::Primary));
    // Past the lift distance, towards the other group's card.
    let to = Vec2::new(from.x + 220.0, from.y);
    pointer(
        &mut app,
        from,
        PointerAction::Move {
            delta: Vec2::X * 4.0,
        },
    );
    pointer(&mut app, to, PointerAction::Move { delta: to - from });
    pointer(&mut app, to, PointerAction::Move { delta: Vec2::X });
    settle(&mut app);

    let landing_node = app.world().get::<Node>(landing).expect("a node");
    assert_eq!(
        landing_node.display,
        Display::Flex,
        "the landing is lit while the tab is carried"
    );
    assert!(
        landing_node.width != Val::Auto,
        "with the part it will take"
    );
    let ghost_node = app.world().get::<Node>(ghost).expect("a node");
    assert_eq!(
        ghost_node.display,
        Display::Flex,
        "the ghost follows the pointer"
    );
    let words = app
        .world()
        .get::<Text>(ghost)
        .map(|text| text.0.clone())
        .unwrap_or_default();
    assert_eq!(words, "Reload", "carrying the tab's own title");

    // Let go: the overlay goes back to hidden and the panel moved.
    pointer(&mut app, to, PointerAction::Release(PointerButton::Primary));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(landing).expect("a node").display,
        Display::None,
        "nothing is lit once it is let go"
    );
    assert_eq!(
        app.world().get::<Node>(ghost).expect("a node").display,
        Display::None
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

/// A role the host picked itself is kept: the dock grows its parts and takes `Role::Dock` only when
/// there is no role at all, never replacing one the host wrote.
#[test]
fn a_hosts_own_role_on_the_dock_is_left_alone() {
    let mut app = app();
    register(&mut app, "reload", "Reload");
    let world = app.world_mut();
    // A host that styles its own node with a role of its own (a split's pane, a surface): the dock
    // must not overwrite it.
    let dock = world
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(220.0),
                ..default()
            },
            Role::Surface(1),
            Dock(DockLayout::group(["reload"])),
        ))
        .id();
    world.spawn(screen()).add_child(dock);
    settle(&mut app);

    assert_eq!(
        app.world().get::<Role>(dock),
        Some(&Role::Surface(1)),
        "the host's own role stands"
    );
    // And the dock still grew the parts its layout needs.
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(dock)
        .map(|kids| kids.iter().collect())
        .unwrap_or_default();
    assert!(
        kids.iter()
            .any(|&kid| app.world().get::<Role>(kid) == Some(&Role::DockBody)),
        "the body is there: the dock is a dock"
    );
}
