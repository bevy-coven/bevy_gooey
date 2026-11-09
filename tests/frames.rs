//! Art frames (`GooeyFrames`): a panel and a button draw from a nine-slice image tinted by the skin,
//! the tint follows every state colour, layout does not change, a flat button stays flat, a host's
//! own image is left alone, and taking the frames away gives the fills back. Laid out for real in a
//! headless window, with no motion so a colour is the one the skin wrote.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{ComputedNode, InteractionDisabled};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn app() -> App {
    let mut app = App::new();
    app.insert_resource(MotionSettings::off());
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

fn art(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

struct Scene {
    panel: Entity,
    button: Entity,
    ghost: Entity,
    host: Entity,
}

/// A panel holding a button, a ghost button and a button with an image of the game's own.
fn scene(app: &mut App, host_art: Handle<Image>) -> Scene {
    let play = app.world_mut().spawn(button("Play")).id();
    let ghost = app
        .world_mut()
        .spawn((button("Skip"), ButtonVariant::Ghost))
        .id();
    let host = app
        .world_mut()
        .spawn((button("Own"), ImageNode::new(host_art)))
        .id();
    let panel = app
        .world_mut()
        .spawn(panel())
        .add_children(&[play, ghost, host])
        .id();
    app.world_mut().spawn(screen()).add_child(panel);
    frames(app, 6);
    Scene {
        panel,
        button: play,
        ghost,
        host,
    }
}

/// The art layer of a framed widget.
fn layer(app: &App, entity: Entity) -> Option<&ImageNode> {
    let framed = app.world().get::<Framed>(entity)?;
    app.world().get::<ImageNode>(framed.0)
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn size(app: &App, entity: Entity) -> Vec2 {
    app.world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size()
}

/// Two apps with the same scene, one framed, one flat.
fn pair() -> (App, Scene, App, Scene, Handle<Image>) {
    let mut flat = app();
    let host = art(&mut flat);
    let flat_scene = scene(&mut flat, host);
    let mut framed = app();
    let host_art = art(&mut framed);
    let panel_art = art(&mut framed);
    {
        let mut frames_res = framed.world_mut().resource_mut::<GooeyFrames>();
        frames_res.panel = Some(Frame::new(panel_art.clone(), 12.0));
        frames_res.button = Some(Frame::new(panel_art.clone(), 8.0));
    }
    let framed_scene = scene(&mut framed, host_art);
    (flat, flat_scene, framed, framed_scene, panel_art)
}

#[test]
fn a_panel_and_a_button_draw_from_a_layer_under_their_children_tinted_by_the_skin_and_layout_is_unchanged()
 {
    let (flat, flat_scene, framed, framed_scene, panel_art) = pair();
    for (flat_id, framed_id, slice) in [
        (flat_scene.panel, framed_scene.panel, 12.0),
        (flat_scene.button, framed_scene.button, 8.0),
    ] {
        let image = layer(&framed, framed_id).expect("framed");
        assert_eq!(image.image, panel_art);
        assert!(
            matches!(&image.image_mode, NodeImageMode::Sliced(s) if s.border.min_inset.x == slice),
            "sliced at {slice}: {:?}",
            image.image_mode
        );
        assert_eq!(image.color, fill(&flat, flat_id), "the tint is the fill");
        assert_eq!(
            fill(&framed, framed_id),
            Color::NONE,
            "no flat fill under the art"
        );
        assert_eq!(
            size(&framed, framed_id),
            size(&flat, flat_id),
            "layout is the same"
        );
    }
    // A panel's outline goes with its fill; a button's border is the focus ring and stays.
    let outline = framed
        .world()
        .get::<BorderColor>(framed_scene.panel)
        .expect("border");
    assert_eq!(*outline, BorderColor::all(Color::NONE));
}

#[test]
fn the_tint_follows_a_state_colour() {
    let (mut flat, flat_scene, mut framed, framed_scene, _) = pair();
    let rest = layer(&framed, framed_scene.button).expect("framed").color;
    for (app, scene) in [(&mut flat, &flat_scene), (&mut framed, &framed_scene)] {
        app.world_mut()
            .entity_mut(scene.button)
            .insert(InteractionDisabled);
        frames(app, 4);
    }
    let disabled = layer(&framed, framed_scene.button).expect("framed").color;
    assert_ne!(disabled, rest, "the state moved the tint");
    assert_eq!(disabled, fill(&flat, flat_scene.button));
}

#[test]
fn a_flat_button_and_a_hosts_own_image_are_left_alone() {
    let (_, _, framed, scene, panel_art) = pair();
    assert!(
        framed.world().get::<Framed>(scene.ghost).is_none(),
        "a ghost button has no fill to tint"
    );
    let own = framed
        .world()
        .get::<ImageNode>(scene.host)
        .expect("its own image");
    assert_ne!(own.image, panel_art, "the host's art stays");
    assert!(framed.world().get::<Framed>(scene.host).is_none());
}

#[test]
fn taking_the_frames_away_gives_the_fills_back() {
    let (flat, flat_scene, mut framed, scene, _) = pair();
    framed.world_mut().resource_mut::<GooeyFrames>().button = None;
    frames(&mut framed, 4);
    assert!(framed.world().get::<Framed>(scene.button).is_none());
    assert_eq!(
        framed
            .world()
            .get::<Children>(scene.button)
            .map_or(0, |kids| kids.len()),
        flat.world()
            .get::<Children>(flat_scene.button)
            .map_or(0, |kids| kids.len()),
        "the art layer went"
    );
    assert_eq!(fill(&framed, scene.button), fill(&flat, flat_scene.button));
    // The panel keeps its own frame.
    assert!(framed.world().get::<Framed>(scene.panel).is_some());
}

#[test]
fn a_settled_framed_interface_writes_nothing() {
    #[derive(Resource, Default)]
    struct Writes(u32);
    let (_, _, mut framed, _, _) = pair();
    type Written = Or<(Changed<ImageNode>, Changed<BackgroundColor>)>;
    framed.init_resource::<Writes>().add_systems(
        Last,
        |changed: Query<(), Written>, mut n: ResMut<Writes>| n.0 += changed.iter().count() as u32,
    );
    frames(&mut framed, 3);
    let before = framed.world().resource::<Writes>().0;
    frames(&mut framed, 30);
    assert_eq!(framed.world().resource::<Writes>().0, before);
}

#[test]
fn what_floats_has_its_own_frame_and_an_unframed_role_stays_flat() {
    let mut flat = app();
    let mut framed = app();
    let art = art(&mut framed);
    framed.world_mut().resource_mut::<GooeyFrames>().popup = Some(Frame::new(art, 6.0));
    let mut ids = Vec::new();
    for app in [&mut flat, &mut framed] {
        let tip = app.world_mut().spawn((Node::default(), Role::Tooltip)).id();
        let menu = app
            .world_mut()
            .spawn((Node::default(), Role::MenuPanel))
            .id();
        let toast = app
            .world_mut()
            .spawn((Node::default(), Role::ToastBody))
            .id();
        let field = app
            .world_mut()
            .spawn((Node::default(), Role::TextField))
            .id();
        frames(app, 6);
        ids.push([tip, menu, toast, field]);
    }
    for (i, (&flat_id, &framed_id)) in ids[0].iter().zip(&ids[1]).take(3).enumerate() {
        let image = layer(&framed, framed_id).expect("framed");
        assert_eq!(
            image.color,
            fill(&flat, flat_id),
            "role {i}: the tint is the fill"
        );
        assert_eq!(fill(&framed, framed_id), Color::NONE);
        // The outline goes with the fill: the art has its own edge.
        assert_eq!(
            *framed
                .world()
                .get::<BorderColor>(framed_id)
                .expect("border"),
            BorderColor::all(Color::NONE)
        );
    }
    // A text field is measured as a leaf by Bevy, so a child layer would collapse it: it is flat.
    assert!(framed.world().get::<Framed>(ids[1][3]).is_none());
}
