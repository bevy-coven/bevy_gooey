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
    // A text field has no frame of its own here: it was not given one.
    assert!(framed.world().get::<Framed>(ids[1][3]).is_none());
}

#[test]
fn a_text_field_draws_its_art_on_its_cell_and_keeps_its_size() {
    let mut flat = app();
    let mut framed = app();
    let art = art(&mut framed);
    framed.world_mut().resource_mut::<GooeyFrames>().field = Some(Frame::new(art, 4.0));
    let mut fields = Vec::new();
    for app in [&mut flat, &mut framed] {
        let row = app.world_mut().spawn(text_input("Name", "Camp 1")).id();
        app.world_mut().spawn(screen()).add_child(row);
        frames(app, 8);
        let field = app
            .world_mut()
            .query_filtered::<Entity, With<bevy::ui_widgets::TextInput>>()
            .single(app.world())
            .expect("one field");
        fields.push(field);
    }
    let framed_layer = framed.world().get::<Framed>(fields[1]).expect("framed").0;
    let cell = framed
        .world()
        .get::<ChildOf>(fields[1])
        .expect("cell")
        .parent();
    assert_eq!(
        framed
            .world()
            .get::<ChildOf>(framed_layer)
            .map(ChildOf::parent),
        Some(cell),
        "the layer is on the field's cell"
    );
    let image = layer(&framed, fields[1]).expect("framed");
    assert_eq!(
        image.color,
        fill(&flat, fields[0]),
        "the tint is the field's fill"
    );
    assert_eq!(fill(&framed, fields[1]), Color::NONE);
    assert_eq!(
        size(&framed, fields[1]),
        size(&flat, fields[0]),
        "the field keeps its size"
    );
    assert_eq!(
        framed.world().get::<BorderColor>(fields[1]),
        flat.world().get::<BorderColor>(fields[0]),
        "its outline and ring stay"
    );
}

#[test]
fn a_meter_track_draws_its_art_under_the_fill_and_keeps_the_fill() {
    let mut flat = app();
    let mut framed = app();
    let art = art(&mut framed);
    framed.world_mut().resource_mut::<GooeyFrames>().meter = Some(Frame::new(art, 6.0));
    let mut tracks = Vec::new();
    for app in [&mut flat, &mut framed] {
        let bar = app.world_mut().spawn(meter(60.0, 100.0)).id();
        app.world_mut().spawn(screen()).add_child(bar);
        frames(app, 6);
        tracks.push(bar);
    }
    let image = layer(&framed, tracks[1]).expect("the track is framed");
    assert_eq!(
        image.color,
        fill(&flat, tracks[0]),
        "the tint is the track's fill"
    );
    assert_eq!(fill(&framed, tracks[1]), Color::NONE);
    assert_eq!(
        size(&framed, tracks[1]),
        size(&flat, tracks[0]),
        "layout is the same"
    );
    // The fill keeps the skin's colour and its place: the art is under it.
    let fills = |app: &App, bar: Entity| -> Vec<Color> {
        app.world()
            .get::<Children>(bar)
            .into_iter()
            .flat_map(|kids| kids.iter())
            .filter(|kid| app.world().get::<Role>(*kid) == Some(&Role::MeterFill))
            .filter_map(|kid| app.world().get::<BackgroundColor>(kid).map(|b| b.0))
            .collect()
    };
    assert_eq!(fills(&framed, tracks[1]), fills(&flat, tracks[0]));
    assert!(!fills(&framed, tracks[1]).is_empty());
}

#[test]
fn slider_switch_and_hold_tracks_draw_their_art_and_keep_their_parts() {
    let mut flat = app();
    let mut framed = app();
    let art = art(&mut framed);
    framed.world_mut().resource_mut::<GooeyFrames>().track = Some(Frame::new(art, 3.0));
    let mut roles = Vec::new();
    for app in [&mut flat, &mut framed] {
        let column = app
            .world_mut()
            .spawn(screen())
            .with_children(|c| {
                c.spawn(slider("Volume", 0.4, 0.0..=1.0));
                c.spawn(toggle("Subtitles", true));
                c.spawn(hold_button("Delete", 1.0));
            })
            .id();
        frames(app, 8);
        let find = |app: &mut App, role: Role| -> Entity {
            let mut q = app.world_mut().query::<(Entity, &Role)>();
            let hits: Vec<Entity> = q
                .iter(app.world())
                .filter(|(e, r)| **r == role && app.world().get::<ChildOf>(*e).is_some())
                .map(|(e, _)| e)
                .collect();
            assert!(!hits.is_empty(), "{role:?} under {column:?}");
            hits[0]
        };
        roles.push([
            find(app, Role::SliderRail),
            find(app, Role::ToggleTrack),
            find(app, Role::HoldTrack),
        ]);
    }
    for (&plain, &art) in roles[0].iter().zip(&roles[1]) {
        let image = layer(&framed, art).expect("the track is framed");
        assert_eq!(image.color, fill(&flat, plain), "tint is the flat fill");
        assert_eq!(fill(&framed, art), Color::NONE);
        assert_eq!(size(&framed, art), size(&flat, plain));
    }
    // A switch that is on is still lit by the accent: the tint, not a new colour.
    let on = layer(&framed, roles[1][1]).expect("framed").color;
    let skin = framed.world().resource::<GooeySkin>();
    assert_eq!(on, skin.accent);
}
