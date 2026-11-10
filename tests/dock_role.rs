//! A dock the host places itself keeps the `Role` the host gave it: `update_docks` grows the parts a
//! dock needs and adds `Role::Dock` only when the node has no role at all. Laid out for real in a
//! headless window.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
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

/// Registers the panel `id`, titled `title`, and counts how often its page was filled.
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

/// A host that styles its own dock node with a role of its own (a split's pane, a surface) keeps it:
/// the dock grows the body its layout is built in, and never overwrites what the host wrote.
#[test]
fn a_hosts_own_role_on_the_dock_is_left_alone() {
    let mut app = app();
    let spawned = register(&mut app, "reload", "Reload");
    let world = app.world_mut();
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
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(dock)
        .map(|kids| kids.iter().collect())
        .unwrap_or_default();
    let body = kids
        .iter()
        .find(|&&kid| app.world().get::<Role>(kid) == Some(&Role::DockBody));
    assert!(body.is_some(), "the dock grew its body: {:?}", kids.len());
    assert_eq!(spawned.load(Ordering::SeqCst), 1, "its panel was built");
}
