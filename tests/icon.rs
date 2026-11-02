//! Icons: the host's own image, sized from the skin's type scale and tinted by a colour of the
//! skin. Laid out for real in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A headless app with a real window size, so UI layout runs.
fn app(skin: GooeySkin) -> App {
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
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

/// A real image in the app's assets, so the icon is not missing.
fn image(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
}

/// A button in a panel with one icon in it: returns the button and the icon.
fn icon_in_button(app: &mut App, tone: Option<IconTone>) -> (Entity, Entity) {
    let source = IconSource::Image(image(app));
    let button = app.world_mut().spawn(button("Delete")).id();
    let icon = app.world_mut().spawn((icon(source), ChildOf(button))).id();
    if let Some(tone) = tone {
        app.world_mut().entity_mut(icon).insert(tone);
    }
    let panel = app.world_mut().spawn(panel()).add_child(button).id();
    app.world_mut().spawn(screen()).add_child(panel);
    settle(app);
    (button, icon)
}

fn tint(app: &App, icon: Entity) -> Color {
    app.world().get::<ImageNode>(icon).expect("an image").color
}

fn side(app: &App, icon: Entity) -> Vec2 {
    app.world()
        .get::<ComputedNode>(icon)
        .expect("laid out")
        .size()
}

#[test]
fn an_icon_in_a_danger_toned_button_takes_the_skins_danger_colour_in_every_look() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app(skin.clone());
        let (_, icon) = icon_in_button(&mut app, Some(IconTone::Danger));
        assert_eq!(tint(&app, icon), skin.danger, "{}", preset.name());
        // A new skin recolours it the same frame.
        let other = if preset == SkinPreset::Dark {
            GooeySkin::parchment()
        } else {
            GooeySkin::dark()
        };
        app.insert_resource(other.clone());
        app.update();
        assert_eq!(tint(&app, icon), other.danger);
    }
}

#[test]
fn the_other_tones_are_the_muted_text_and_the_accent() {
    let skin = GooeySkin::dark();
    let mut app = app(skin.clone());
    let (_, muted) = icon_in_button(&mut app, Some(IconTone::Muted));
    let (_, accent) = icon_in_button(&mut app, Some(IconTone::Accent));
    assert_eq!(tint(&app, muted), skin.text_muted);
    assert_eq!(tint(&app, accent), skin.accent);
}

#[test]
fn a_default_icon_follows_its_button_as_the_buttons_text_does() {
    let skin = GooeySkin::dark();
    let mut app = app(skin.clone());
    let (button, icon) = icon_in_button(&mut app, None);
    assert_eq!(tint(&app, icon), skin.text);
    app.world_mut().entity_mut(button).insert(Pressed);
    app.update();
    assert_eq!(tint(&app, icon), skin.accent_text);
    app.world_mut().entity_mut(button).remove::<Pressed>();
    app.world_mut()
        .entity_mut(button)
        .insert(InteractionDisabled);
    app.update();
    assert_eq!(tint(&app, icon), skin.text_muted);
    app.world_mut()
        .entity_mut(button)
        .remove::<InteractionDisabled>();
    app.update();
    assert_eq!(tint(&app, icon), skin.text);
}

#[test]
fn an_icon_is_a_step_of_the_type_scale_square_and_grows_with_the_text_size() {
    for scale in [1.0, 2.0] {
        let skin = GooeySkin {
            text_scale: scale,
            ..GooeySkin::dark()
        };
        let mut app = app(skin.clone());
        let (_, body) = icon_in_button(&mut app, None);
        let (_, small) = icon_in_button(&mut app, Some(IconTone::Text));
        app.world_mut().entity_mut(small).insert(IconSize(0));
        settle(&mut app);
        let want = skin.type_size(2);
        assert!(
            (side(&app, body) - Vec2::splat(want)).abs().max_element() < 1.0,
            "{scale}"
        );
        let small_want = skin.type_size(0);
        assert!(
            (side(&app, small) - Vec2::splat(small_want))
                .abs()
                .max_element()
                < 1.0,
            "{scale}: {:?} {small_want}",
            side(&app, small)
        );
        assert!(side(&app, small).x < side(&app, body).x);
    }
}

#[test]
fn an_atlas_icon_draws_its_cell_and_changing_it_changes_the_picture_the_same_frame() {
    let mut app = app(GooeySkin::dark());
    let image = image(&mut app);
    let layout = app
        .world_mut()
        .resource_mut::<Assets<TextureAtlasLayout>>()
        .add(TextureAtlasLayout::from_grid(
            UVec2::splat(16),
            4,
            4,
            None,
            None,
        ));
    let id = app
        .world_mut()
        .spawn(icon(IconSource::Atlas {
            image: image.clone(),
            layout: layout.clone(),
            index: 5,
        }))
        .id();
    settle(&mut app);
    let drawn = app.world().get::<ImageNode>(id).expect("an image").clone();
    assert_eq!(drawn.image, image);
    assert_eq!(
        drawn
            .texture_atlas
            .as_ref()
            .map(|a| (a.layout.clone(), a.index)),
        Some((layout.clone(), 5))
    );
    app.world_mut()
        .entity_mut(id)
        .insert(Icon(IconSource::Atlas {
            image: image.clone(),
            layout: layout.clone(),
            index: 9,
        }));
    app.update();
    let drawn = app.world().get::<ImageNode>(id).expect("an image");
    assert_eq!(drawn.texture_atlas.as_ref().map(|a| a.index), Some(9));
    // A whole image drops the atlas.
    app.world_mut()
        .entity_mut(id)
        .insert(Icon(IconSource::Image(image)));
    app.update();
    assert!(
        app.world()
            .get::<ImageNode>(id)
            .expect("an image")
            .texture_atlas
            .is_none()
    );
}

#[test]
fn a_missing_image_is_an_empty_box_of_the_right_size_marked_once_and_cleared_by_a_real_one() {
    let skin = GooeySkin::dark();
    let mut app = app(skin.clone());
    let ghost: Handle<Image> = bevy::asset::uuid_handle!("12345678-9abc-def0-1234-56789abcdef0");
    let id = app.world_mut().spawn(icon(IconSource::Image(ghost))).id();
    settle(&mut app);
    assert!(app.world().get::<IconMissing>(id).is_some());
    let want = skin.type_size(2);
    assert!((side(&app, id) - Vec2::splat(want)).abs().max_element() < 1.0);
    let real = image(&mut app);
    app.world_mut()
        .entity_mut(id)
        .insert(Icon(IconSource::Image(real)));
    app.update();
    assert!(app.world().get::<IconMissing>(id).is_none());
}

#[test]
fn thirty_idle_frames_write_nothing_for_an_icon() {
    let mut app = app(GooeySkin::dark());
    let (_, icon) = icon_in_button(&mut app, Some(IconTone::Danger));
    for _ in 0..5 {
        app.update();
    }
    let mut writes = 0;
    for _ in 0..30 {
        app.update();
        let world = app.world_mut();
        let mut query = world
            .query_filtered::<Entity, Or<(Changed<ImageNode>, Changed<Node>, Changed<Icon>)>>();
        writes += query.iter(world).filter(|e| *e == icon).count();
    }
    assert_eq!(writes, 0);
}
