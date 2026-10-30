use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

#[test]
fn dump() {
    for (w, h) in [(1280u32, 720u32), (1920, 1080), (1280, 800)] {
        let mut app = App::new();
        app.add_plugins((
            DefaultPlugins
                .set(WindowPlugin { primary_window: Some(Window { resolution: WindowResolution::new(w, h).with_scale_factor_override(1.0), ..default() }), ..default() })
                .set(RenderPlugin { render_creation: WgpuSettings { backends: None, ..default() }.into(), ..default() })
                .disable::<WinitPlugin>(),
            GooeyPlugins,
        ));
        app.world_mut().spawn(Camera2d);
        let m = app.world_mut().spawn(meter(60.0, 100.0)).id();
        let p = app.world_mut().spawn(panel()).add_child(m).id();
        app.world_mut().spawn(screen()).add_child(p);
        for _ in 0..4 { app.update(); }
        let mut q = app.world_mut().query::<(Entity, &ComputedNode, &UiGlobalTransform, Option<&Role>, Option<&Node>)>();
        eprintln!("=== {w}x{h} scale {}", app.world().resource::<UiScale>().0);
        for (e, n, t, r, node) in q.iter(app.world()) {
            eprintln!("{e:?} {r:?} size {:?} center {:?} border {:?} padding {:?} node_h {:?} node_pad {:?}", n.size(), t.translation, n.border(), n.padding(), node.map(|n| n.height), node.map(|n| n.padding.top));
        }
    }
}
