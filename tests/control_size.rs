//! Control sizes (`ControlSize`): under a compact skin (the Desk), a control is 24, 28 or 36 logical
//! px tall by its size; under a comfortable one every size is the skin's 44 px target; no size keeps
//! the skin's own height, never under its target; a new size, or one taken off, restyles the
//! control and its parts in the frame. Laid out for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::ComputedNode;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// How far a measured height may be from the size it asks for, logical px.
const TOLERANCE_PX: f32 = 0.5;

fn app(skin: GooeySkin, interface: f32) -> App {
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
    .insert_resource(skin)
    .insert_resource(UiScaleSettings {
        scale: interface,
        ..default()
    });
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// The controls whose height is the size, each spawned with `size` (or none): a button, an icon
/// button, a toggle, a select and a text field. Returns `(name, the node whose height is measured)`.
fn controls(app: &mut App, size: Option<ControlSize>) -> Vec<(&'static str, Entity)> {
    let world = app.world_mut();
    let roots: Vec<(&'static str, Entity)> = vec![
        ("button", world.spawn(button("Save")).id()),
        (
            "icon button",
            world
                .spawn(icon_button(
                    IconSource::Image(Handle::default()),
                    Tooltip::new("Pan"),
                ))
                .id(),
        ),
        ("toggle", world.spawn(toggle("Snap", true)).id()),
        (
            "select",
            world.spawn(select("Quality", ["Low", "High"], 0)).id(),
        ),
        ("text input", world.spawn(text_input("Name", "Ada")).id()),
    ];
    if let Some(size) = size {
        for &(_, root) in &roots {
            world.entity_mut(root).insert(size);
        }
    }
    let ids: Vec<Entity> = roots.iter().map(|&(_, e)| e).collect();
    let card = world.spawn(panel()).add_children(&ids).id();
    world.spawn(screen()).add_child(card);
    settle(app);
    roots
        .into_iter()
        .map(|(name, root)| {
            // A text input's control is its framed field; the row round it has its label's air.
            let measured = if name == "text input" {
                field_of(app, root)
            } else {
                root
            };
            (name, measured)
        })
        .collect()
}

fn field_of(app: &App, root: Entity) -> Entity {
    let mut stack = vec![root];
    while let Some(e) = stack.pop() {
        if app.world().get::<Role>(e) == Some(&Role::TextField) {
            return e;
        }
        if let Some(kids) = app.world().get::<Children>(e) {
            stack.extend(kids.iter());
        }
    }
    panic!("a text input has a field");
}

/// A node's height: logical px, and physical px.
fn height(app: &App, entity: Entity) -> (f32, f32) {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    (node.size().y * node.inverse_scale_factor(), node.size().y)
}

#[test]
fn under_the_desk_the_three_sizes_are_24_28_and_36_px_and_twice_that_at_ui_scale_2() {
    for size in ControlSize::ALL {
        for interface in [1.0, 2.0] {
            let mut app = app(GooeySkin::desk(), interface);
            let measured = controls(&mut app, Some(size));
            assert_eq!(
                app.world().resource::<UiScale>().0,
                interface,
                "a 1280x720 window at this interface size is that UiScale"
            );
            for (name, entity) in measured {
                let (logical, physical) = height(&app, entity);
                assert!(
                    (logical - size.px()).abs() <= TOLERANCE_PX,
                    "{name} at {size:?} and {interface}x: {logical} logical px, wanted {}",
                    size.px()
                );
                assert!(
                    (physical - size.px() * interface).abs() <= TOLERANCE_PX * interface,
                    "{name} at {size:?} and {interface}x: {physical} physical px"
                );
            }
        }
    }
}

#[test]
fn under_a_comfortable_skin_every_size_is_the_44_px_target() {
    let target = GooeySkin::dark().target();
    assert!(target >= 44.0);
    for size in ControlSize::ALL {
        let mut app = app(GooeySkin::dark(), 1.0);
        for (name, entity) in controls(&mut app, Some(size)) {
            let (logical, _) = height(&app, entity);
            assert!(
                logical >= 44.0 - TOLERANCE_PX,
                "{name} at {size:?} under dark: {logical}"
            );
        }
    }
}

#[test]
fn no_size_is_never_under_the_skins_target() {
    for skin in [GooeySkin::desk(), GooeySkin::dark()] {
        let target = skin.target();
        let mut app = app(skin, 1.0);
        for (name, entity) in controls(&mut app, None) {
            let (logical, _) = height(&app, entity);
            assert!(
                logical >= target - TOLERANCE_PX,
                "{name} with no size: {logical}, the target is {target}"
            );
        }
    }
}

#[test]
fn a_choice_and_a_number_field_grow_with_their_size() {
    let mut heights = Vec::new();
    for size in ControlSize::ALL {
        let mut app = app(GooeySkin::desk(), 1.0);
        let world = app.world_mut();
        let pick = world
            .spawn((choice("Quality", ["Low", "High"], 0), size))
            .id();
        let number = world
            .spawn((number_field("Speed", 1.0, 0.1, 0.0..=10.0), size))
            .id();
        let card = world.spawn(panel()).add_children(&[pick, number]).id();
        world.spawn(screen()).add_child(card);
        settle(&mut app);
        let (pick_px, _) = height(&app, pick);
        let (number_px, _) = height(&app, number);
        assert!(
            pick_px >= size.px() - TOLERANCE_PX,
            "choice at {size:?}: {pick_px}"
        );
        assert!(
            number_px >= size.px() - TOLERANCE_PX,
            "number field at {size:?}: {number_px}"
        );
        heights.push((pick_px, number_px));
    }
    for pair in heights.windows(2) {
        assert!(pair[1].0 > pair[0].0, "the choice grows: {heights:?}");
        assert!(pair[1].1 > pair[0].1, "the number field grows: {heights:?}");
    }
}

#[test]
fn a_new_size_and_a_removed_one_restyle_the_controls_parts_in_their_frame() {
    let mut app = app(GooeySkin::desk(), 1.0);
    let bare: Vec<f32> = controls(&mut app, None)
        .into_iter()
        .map(|(_, e)| height(&app, e).0)
        .collect();
    let mut app = self::app(GooeySkin::desk(), 1.0);
    let measured = controls(&mut app, Some(ControlSize::Small));
    // The select's arrow and the text input's field are parts that read their control's size.
    let roots: Vec<Entity> = measured
        .iter()
        .map(|&(name, e)| {
            if name == "text input" {
                root_of_field(&app, e)
            } else {
                e
            }
        })
        .collect();
    for &root in &roots {
        app.world_mut().entity_mut(root).insert(ControlSize::Large);
    }
    app.update();
    for &(name, entity) in &measured {
        let (logical, _) = height(&app, entity);
        assert!(
            (logical - ControlSize::Large.px()).abs() <= TOLERANCE_PX,
            "{name} after a new size: {logical}"
        );
    }
    for &root in &roots {
        app.world_mut().entity_mut(root).remove::<ControlSize>();
    }
    app.update();
    for (&(name, entity), &before) in measured.iter().zip(&bare) {
        let (logical, _) = height(&app, entity);
        assert!(
            (logical - before).abs() <= TOLERANCE_PX,
            "{name} with its size taken off: {logical}, unsized it is {before}"
        );
    }
}

#[test]
fn a_sized_vec_fields_components_are_the_size() {
    for size in ControlSize::ALL {
        let mut app = app(GooeySkin::desk(), 1.0);
        let world = app.world_mut();
        let root = world
            .spawn((vec_field("Position", [0.0; 3], 0.1, -10.0..=10.0), size))
            .id();
        let card = world.spawn(panel()).add_child(root).id();
        world.spawn(screen()).add_child(card);
        settle(&mut app);
        let field = field_of(&app, root);
        let (logical, _) = height(&app, field);
        assert!(
            (logical - size.px()).abs() <= TOLERANCE_PX,
            "a vec component's field at {size:?}: {logical}"
        );
    }
}

/// The text input row a framed field stands in.
fn root_of_field(app: &App, field: Entity) -> Entity {
    let mut at = field;
    while let Some(parent) = app.world().get::<ChildOf>(at).map(ChildOf::parent) {
        if app.world().get::<Role>(parent) == Some(&Role::TextInput) {
            return parent;
        }
        at = parent;
    }
    panic!("a field stands in a text input");
}
