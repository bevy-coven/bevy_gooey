//! The parts a tool's panels were still building from raw `Node`s (G-82, #93): a glyph that is a
//! hit target of its own, a tab with an active edge, a band that is a button, a text that is its own
//! flex item, and the world-space label a viewport pane carries. Each is laid out for real in a
//! headless window and measured on screen; the skin owns every size and colour.
#![cfg(feature = "testing")]

use bevy::camera::{NormalizedRenderTarget, Viewport};
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{IsDefaultUiCamera, Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::click;
use bevy_gooey::*;

/// How many times a widget's `Activate` was heard.
#[derive(Resource, Default)]
struct Heard(usize);

fn app_with(skin: GooeySkin) -> App {
    let mut app = App::new();
    // Colours ease on the real clock; this app reads them landed.
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
    ))
    .insert_resource(skin)
    .init_resource::<Heard>();
    app.world_mut().spawn((Camera2d, IsDefaultUiCamera));
    app
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// The size a node is laid out at, in the skin's own px: the window's scale and `UiScale` taken out.
fn size(app: &App, entity: Entity) -> Vec2 {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let ui = app.world().resource::<UiScale>().0;
    node.size() * node.inverse_scale_factor() / ui
}

/// A node's rectangle in the skin's px: `(left, top, right, bottom)`.
fn rect(app: &App, entity: Entity) -> (f32, f32, f32, f32) {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let ui = app.world().resource::<UiScale>().0;
    let k = node.inverse_scale_factor() / ui;
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation
        * k;
    let half = node.size() * k / 2.0;
    (at.x - half.x, at.y - half.y, at.x + half.x, at.y + half.y)
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .map(|b| b.0)
        .expect("a fill")
}

fn ink(app: &App, widget: Entity) -> Color {
    let words = app.world().get::<Children>(widget).expect("words")[0];
    app.world().get::<TextColor>(words).expect("ink").0
}

/// The widget's own words: its child that is `Role::ButtonText` (a tooltip's box may stand first).
fn first_child(app: &App, entity: Entity) -> Entity {
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(entity)
        .expect("a child")
        .iter()
        .collect();
    kids.into_iter()
        .find(|kid| app.world().get::<Role>(*kid) == Some(&Role::ButtonText))
        .expect("a ButtonText child")
}

/// Moves the real pointer onto `entity`, so the picking plugin's hover is what it reads.
fn point_at(app: &mut App, entity: Entity) {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation
        * node.inverse_scale_factor();
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a window");
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    for _ in 0..3 {
        app.update();
    }
}

fn count_activations(_: On<Activate>, mut heard: ResMut<Heard>) {
    heard.0 += 1;
}

// ---- 1. a glyph hit target -----------------------------------------------------------------

#[test]
fn a_glyph_button_keeps_a_hit_area_of_the_floor_whatever_its_glyph() {
    for glyph_px in [6.0, 14.0, 48.0] {
        let mut skin = GooeySkin::dark();
        skin.text_size = glyph_px;
        let floor = skin.target();
        let mut app = app_with(skin);
        let host = app.world_mut().spawn(row(1.0)).id();
        let glyph = app
            .world_mut()
            .spawn(glyph_button("v", "Fold the group"))
            .id();
        app.world_mut().entity_mut(host).add_child(glyph);
        app.world_mut().spawn(screen()).add_child(host);
        settle(&mut app);
        let hit = size(&app, glyph);
        assert!(
            hit.x >= floor - 0.01 && hit.y >= floor - 0.01,
            "a {glyph_px} px glyph's hit area is {hit}, under the floor {floor}"
        );
        // The glyph is centred in it, not stretched to it.
        let words = first_child(&app, glyph);
        let (gl, gt, gr, gb) = rect(&app, words);
        let (bl, bt, br, bb) = rect(&app, glyph);
        assert!(
            ((gl + gr) - (bl + br)).abs() < 1.5 && ((gt + gb) - (bt + bb)).abs() < 1.5,
            "a {glyph_px} px glyph is not centred: glyph {:?} in button {:?}",
            (gl, gt, gr, gb),
            (bl, bt, br, bb)
        );
        if glyph_px < 10.0 {
            let small = size(&app, words);
            assert!(
                small.x < floor / 2.0,
                "the test needs a glyph well under the floor: {small}"
            );
        }
    }
}

#[test]
fn a_glyph_button_is_pressed_anywhere_in_its_hit_area() {
    let mut skin = GooeySkin::dark();
    skin.text_size = 6.0;
    let mut app = app_with(skin);
    let host = app.world_mut().spawn(row(1.0)).id();
    let glyph = app
        .world_mut()
        .spawn((
            glyph_button(">", "Open the group"),
            observe(count_activations),
        ))
        .id();
    app.world_mut().entity_mut(host).add_child(glyph);
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    click(&mut app, glyph);
    assert_eq!(app.world().resource::<Heard>().0, 1);
    // The glyph's own box is a sliver of that area: the point a finger lands on, a few px off the
    // glyph, is inside the node and not inside the glyph.
    let words = first_child(&app, glyph);
    let node = app.world().get::<ComputedNode>(glyph).expect("laid out");
    let centre = app
        .world()
        .get::<UiGlobalTransform>(glyph)
        .expect("placed")
        .translation;
    let off_glyph = centre + Vec2::new(node.size().x * 0.4, node.size().y * 0.4);
    assert!(node.contains_point(
        *app.world().get::<UiGlobalTransform>(glyph).unwrap(),
        off_glyph
    ));
    let glyph_box = app.world().get::<ComputedNode>(words).expect("laid out");
    assert!(!glyph_box.contains_point(
        *app.world().get::<UiGlobalTransform>(words).unwrap(),
        off_glyph
    ));
}

// ---- 2. a tab with an active edge ----------------------------------------------------------

fn strip(app: &mut App) -> [Entity; 3] {
    let host = app.world_mut().spawn(row(0.0)).id();
    let tabs = ["Meshes", "Tiles", "Kits"].map(|name| app.world_mut().spawn(edge_tab(name)).id());
    app.world_mut().entity_mut(host).add_children(&tabs);
    app.world_mut().spawn(screen()).add_child(host);
    settle(app);
    tabs
}

fn edge(app: &App, tab: Entity) -> BorderColor {
    *app.world().get::<BorderColor>(tab).expect("an edge")
}

#[test]
fn the_selected_tab_lights_its_edge_without_moving_a_neighbour() {
    let skin = GooeySkin::dark();
    let mut app = app_with(skin.clone());
    let tabs = strip(&mut app);
    let before = tabs.map(|tab| rect(&app, tab));
    for tab in tabs {
        assert_eq!(edge(&app, tab).bottom, Color::NONE, "no tab is lit yet");
    }
    app.world_mut().entity_mut(tabs[1]).insert(Selected);
    settle(&mut app);
    assert_eq!(edge(&app, tabs[1]).bottom, skin.accent);
    assert_eq!(edge(&app, tabs[0]).bottom, Color::NONE);
    assert_eq!(edge(&app, tabs[2]).bottom, Color::NONE);
    // The edge is always reserved: lighting it changes the colour of a border that was there, so
    // no tab changes size and none beside it moves.
    let after = tabs.map(|tab| rect(&app, tab));
    for (b, a) in before.iter().zip(after.iter()) {
        assert_eq!(b, a, "a tab moved when the selection did");
    }
    app.world_mut().entity_mut(tabs[1]).remove::<Selected>();
    app.world_mut().entity_mut(tabs[2]).insert(Selected);
    settle(&mut app);
    assert_eq!(edge(&app, tabs[1]).bottom, Color::NONE);
    assert_eq!(edge(&app, tabs[2]).bottom, skin.accent);
    assert_eq!(before.map(|r| r.0), tabs.map(|t| rect(&app, t).0));
}

#[test]
fn a_tab_edge_is_heavier_than_a_frame_and_the_inset_is_the_skins() {
    let skin = GooeySkin::dark();
    let mut app = app_with(skin.clone());
    let tabs = strip(&mut app);
    let node = app.world().get::<Node>(tabs[0]).expect("a node").clone();
    let Val::Px(edge) = node.border.bottom else {
        panic!("the edge is in px: {:?}", node.border.bottom);
    };
    assert!(
        edge > skin.border,
        "the edge {edge} should read heavier than a {} px frame",
        skin.border
    );
    assert_eq!(node.border.top, Val::ZERO);
    assert_eq!(node.border.left, Val::ZERO);
    let Val::Px(inset) = node.padding.left else {
        panic!("the inset is in px: {:?}", node.padding.left);
    };
    assert!(
        inset >= skin.pad(),
        "an inset of {inset} is under the skin's pad"
    );
    // A taller skin gives a taller edge and a wider inset: nothing is a literal in the widget.
    let mut big = skin;
    big.padding *= 2.0;
    big.border = 4.0;
    let mut app = app_with(big.clone());
    let tabs = strip(&mut app);
    let wide = app.world().get::<Node>(tabs[0]).expect("a node").clone();
    let (Val::Px(wide_edge), Val::Px(wide_inset)) = (wide.border.bottom, wide.padding.left) else {
        panic!("px");
    };
    assert!(wide_edge > edge && wide_inset > inset);
}

#[test]
fn a_tab_takes_its_fill_and_ink_from_the_skin_by_state() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app_with(skin.clone());
        let tabs = strip(&mut app);
        assert_eq!(
            fill(&app, tabs[0]),
            skin.surface_raised,
            "{}",
            preset.name()
        );
        app.world_mut().entity_mut(tabs[0]).insert(Selected);
        settle(&mut app);
        // The selected tab continues the surface under it, so its words are the plain text colour.
        assert_eq!(fill(&app, tabs[0]), skin.surface, "{}", preset.name());
        assert_eq!(ink(&app, tabs[0]), skin.text, "{}", preset.name());
        point_at(&mut app, tabs[2]);
        assert_eq!(fill(&app, tabs[2]), skin.hovered(), "{}", preset.name());
        assert_eq!(ink(&app, tabs[2]), skin.text, "{}", preset.name());
    }
}

// ---- 3. a band that is a button ------------------------------------------------------------

fn bands(app: &mut App) -> (Entity, Entity) {
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(400.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let band_entity = app
        .world_mut()
        .spawn((band(), observe(count_activations)))
        .with_children(|parts| {
            parts.spawn((Text::new("v"), Role::ButtonText));
            parts.spawn((Text::new("kenney_prototype-kit"), Role::ButtonText));
        })
        .id();
    let plain = app.world_mut().spawn(button("A row")).id();
    app.world_mut()
        .entity_mut(host)
        .add_children(&[band_entity, plain]);
    app.world_mut().spawn(screen()).add_child(host);
    settle(app);
    (band_entity, plain)
}

#[test]
fn a_band_is_as_wide_as_its_host_and_shades_like_a_row_on_hover() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app_with(skin.clone());
        let (band_entity, plain) = bands(&mut app);
        assert!(
            (size(&app, band_entity).x - 400.0).abs() < 0.5,
            "{}: the band is {} wide in a 400 px host",
            preset.name(),
            size(&app, band_entity).x
        );
        let rest = fill(&app, band_entity);
        assert_eq!(rest, skin.surface_raised, "{}", preset.name());
        assert_eq!(rest, fill(&app, plain), "{}: the row's rest", preset.name());
        point_at(&mut app, band_entity);
        let hovered = fill(&app, band_entity);
        assert_eq!(hovered, skin.hovered(), "{}", preset.name());
        point_at(&mut app, plain);
        assert_eq!(
            fill(&app, plain),
            hovered,
            "{}: the row's hover",
            preset.name()
        );
        assert_eq!(fill(&app, band_entity), rest, "{}", preset.name());
    }
}

#[test]
fn a_band_toggles_its_host_once_a_press() {
    let mut app = app_with(GooeySkin::dark());
    let (band_entity, _) = bands(&mut app);
    click(&mut app, band_entity);
    assert_eq!(app.world().resource::<Heard>().0, 1);
    click(&mut app, band_entity);
    assert_eq!(app.world().resource::<Heard>().0, 2);
}

// ---- 4. a text that is its own flex item ---------------------------------------------------

/// A 300 px host holding one row of a growing text and a count, and the row.
fn tally_row(app: &mut App, name: &str, count: &str) -> (Entity, Entity, Entity) {
    let host = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let line = app.world_mut().spawn(row(1.0)).id();
    let name = app.world_mut().spawn(fill_text(name)).id();
    let count = app.world_mut().spawn(count_text(count)).id();
    app.world_mut()
        .entity_mut(line)
        .add_children(&[name, count]);
    app.world_mut().entity_mut(host).add_child(line);
    app.world_mut().spawn(screen()).add_child(host);
    settle(app);
    (line, name, count)
}

#[test]
fn a_count_never_widens_its_row_however_long_the_name() {
    let mut app = app_with(GooeySkin::dark());
    let (_, _, alone) = tally_row(&mut app, "kit", "145");
    let natural = size(&app, alone).x;
    for name in [
        "kit",
        "low_poly_furniture/glb/ Electronics and some more words that go on",
        "kenney_prototype-kit/Models/GLB/format/with_one_very_long_unbreakable_word_in_it",
    ] {
        let mut app = app_with(GooeySkin::dark());
        let (line, words, count) = tally_row(&mut app, name, "145");
        assert!(
            (size(&app, line).x - 300.0).abs() < 0.5,
            "the row is {} wide for {name:?}",
            size(&app, line).x
        );
        assert!(
            (size(&app, count).x - natural).abs() < 0.5,
            "the count was squeezed to {} for {name:?}",
            size(&app, count).x
        );
        let (_, _, right_of_name, _) = rect(&app, words);
        let (left_of_count, _, right_of_count, _) = rect(&app, count);
        assert!(
            right_of_name <= left_of_count + 0.5,
            "{name:?} runs into its count"
        );
        let (_, _, row_right, _) = rect(&app, line);
        assert!(right_of_count <= row_right + 0.5, "the count left the row");
    }
}

#[test]
fn a_long_name_wraps_inside_its_own_column() {
    let mut app = app_with(GooeySkin::dark());
    let (_, short, _) = tally_row(&mut app, "kit", "145");
    let one_line = size(&app, short).y;
    let mut app = app_with(GooeySkin::dark());
    let (_, long, _) = tally_row(
        &mut app,
        "low_poly_furniture/glb/ Electronics and some more words that go on and on",
        "145",
    );
    assert!(
        size(&app, long).y > one_line * 1.5,
        "the name did not wrap: {} against a line of {one_line}",
        size(&app, long).y
    );
}

#[test]
fn a_count_keeps_one_line_and_a_name_wraps_by_the_skin() {
    let mut app = app_with(GooeySkin::dark());
    let (_, name, count) = tally_row(&mut app, "a name", "8 pack(s), 120 mesh(es)");
    let layout = |entity| {
        app.world()
            .get::<TextLayout>(entity)
            .expect("a layout")
            .linebreak
    };
    assert_eq!(layout(count), LineBreak::NoWrap);
    assert_eq!(layout(name), LineBreak::WordOrCharacter);
    // The ink is the skin's text colour, and the count stands a gap off the name.
    let skin = GooeySkin::dark();
    assert_eq!(app.world().get::<TextColor>(name).unwrap().0, skin.text);
    assert_eq!(app.world().get::<TextColor>(count).unwrap().0, skin.text);
    let (_, _, name_right, _) = rect(&app, name);
    let (count_left, _, _, _) = rect(&app, count);
    assert!(count_left - name_right >= skin.gap * 0.5 - 0.5);
}

// ---- 5. a world-space label over a viewport pane -------------------------------------------

#[test]
fn a_label_in_a_viewport_pane_stands_on_the_point_its_camera_projects() {
    let mut app = app_with(GooeySkin::dark());
    // The 3D view is a 600 x 400 rectangle at (300, 100) of the window, its own camera.
    let stage = app
        .world_mut()
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                viewport: Some(Viewport {
                    physical_position: UVec2::new(300, 100),
                    physical_size: UVec2::new(600, 400),
                    ..default()
                }),
                ..default()
            },
        ))
        .id();
    let slot = app
        .world_mut()
        .spawn(Transform::from_xyz(100.0, 50.0, 0.0))
        .id();
    // The pane stands exactly on that rectangle; the label is its child, so its place is the
    // pane's own px, which is the camera's viewport.
    let pane = app
        .world_mut()
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(300.0),
            top: Val::Px(100.0),
            width: Val::Px(600.0),
            height: Val::Px(400.0),
            ..default()
        })
        .id();
    let name = app
        .world_mut()
        .spawn((label("slot 3"), WorldAnchor::new(slot).camera(stage)))
        .id();
    app.world_mut().entity_mut(pane).add_child(name);
    settle(&mut app);
    // The camera's middle is (600, 300) in the window; the point is 100 right and 50 up of it.
    let (l, _, r, b) = rect(&app, name);
    let foot = Vec2::new((l + r) / 2.0, b);
    assert!(
        (foot - Vec2::new(700.0, 250.0)).abs().max_element() < 1.0,
        "the label's foot is at {foot}, not on the projected point (700, 250)"
    );
    // Hidden until the camera can place it: a point the camera cannot see is not drawn.
    app.world_mut()
        .get_mut::<Transform>(slot)
        .expect("a transform")
        .translation = Vec3::new(5000.0, 0.0, 0.0);
    settle(&mut app);
    assert_eq!(
        *app.world().get::<Visibility>(name).expect("a visibility"),
        Visibility::Hidden
    );
}
