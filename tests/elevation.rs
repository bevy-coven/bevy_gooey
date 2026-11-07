//! Elevation as data (G-48): each surface casts its height's stack from the skin, a CSS blur
//! converts to Bevy's deviation, the high-contrast skin casts none, and a shadow that cannot be drawn
//! is a problem. Laid out in a headless window.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{BoxShadow, ShadowStyle};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

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
    for _ in 0..6 {
        app.update();
    }
}

fn shadow(app: &App, entity: Entity) -> BoxShadow {
    app.world()
        .get::<BoxShadow>(entity)
        .cloned()
        .unwrap_or_else(|| panic!("{entity:?} casts no BoxShadow"))
}

/// The entity with `role` there is exactly one of.
fn with_role(app: &mut App, role: Role) -> Entity {
    let found: Vec<Entity> = app
        .world_mut()
        .query::<(Entity, &Role)>()
        .iter(app.world())
        .filter(|(_, r)| **r == role)
        .map(|(e, _)| e)
        .collect();
    assert_eq!(found.len(), 1, "one {role:?}: {found:?}");
    found[0]
}

/// A screen's panel, a host's own panel, a card and a dialog's card.
fn spawn_surfaces(app: &mut App) -> (Entity, Entity, Entity, Entity) {
    let screen_panel = app.world_mut().spawn(panel()).id();
    app.world_mut()
        .spawn((card_grid([Card::new("Ruins")]), ChildOf(screen_panel)));
    let host_panel = app.world_mut().spawn((Node::default(), Role::Panel)).id();
    let scrim = app.world_mut().spawn(dialog()).id();
    let dialog_card = app.world_mut().spawn((panel(), ChildOf(scrim))).id();
    settle(app);
    let card = with_role(app, Role::CardCell);
    (screen_panel, host_panel, card, dialog_card)
}

#[test]
fn each_surface_casts_its_heights_stack_from_the_skin() {
    let skin = GooeySkin::dark();
    let mut app = app(skin.clone());
    let (screen_panel, host_panel, card, dialog_card) = spawn_surfaces(&mut app);
    let raised = skin.elevation.box_shadow(ElevationLevel::Raised, 1.0);
    let overlay = skin.elevation.box_shadow(ElevationLevel::Overlay, 1.0);
    assert!(
        !raised.0.is_empty() && raised != overlay,
        "the dark skin's stacks differ"
    );
    assert_eq!(
        shadow(&app, screen_panel),
        raised,
        "a screen's panel is raised"
    );
    assert_eq!(
        shadow(&app, host_panel),
        raised,
        "so is a host's own Role::Panel"
    );
    assert_eq!(shadow(&app, card), raised, "and a card");
    assert_eq!(
        shadow(&app, dialog_card),
        overlay,
        "a dialog's card is an overlay"
    );

    // A popover floats.
    let anchor = app.world_mut().spawn(button("Anchor")).id();
    app.world_mut().spawn(popover(anchor, Placement::Below));
    settle(&mut app);
    let card = with_role(&mut app, Role::Popover);
    assert_eq!(
        shadow(&app, card),
        skin.elevation.box_shadow(ElevationLevel::Floating, 1.0)
    );

    // A toast is an overlay: the same layers, its colours faded with the toast.
    app.world_mut().spawn(toast("Saved"));
    settle(&mut app);
    let toast_row = with_role(&mut app, Role::Toast);
    let toast_alpha = app
        .world()
        .get::<Toast>(toast_row)
        .expect("the toast row is the toast")
        .alpha();
    assert!(toast_alpha < 1.0, "still fading in: {toast_alpha}");
    let cast = shadow(&app, toast_row);
    assert_eq!(cast.0.len(), overlay.0.len());
    for (cast, wanted) in cast.0.iter().zip(&overlay.0) {
        assert_eq!(
            (
                cast.x_offset,
                cast.y_offset,
                cast.blur_radius,
                cast.spread_radius
            ),
            (
                wanted.x_offset,
                wanted.y_offset,
                wanted.blur_radius,
                wanted.spread_radius
            )
        );
        let faded = wanted.color.alpha() * toast_alpha;
        assert!(
            (cast.color.alpha() - faded).abs() < 1e-5,
            "the shadow fades with the toast: {} against {faded}",
            cast.color.alpha()
        );
    }
}

#[test]
fn a_css_blur_is_twice_the_deviation_bevy_draws() {
    let layer = ShadowLayer {
        x: 0.0,
        y: 8.0,
        blur: 24.0,
        spread: 0.0,
        color: Color::srgba(0.0, 0.0, 0.0, 0.4),
    };
    let ShadowStyle {
        x_offset,
        y_offset,
        spread_radius,
        blur_radius,
        color,
    } = layer.to_bevy(1.0);
    assert_eq!(
        (x_offset, y_offset, spread_radius, blur_radius),
        (Val::Px(0.0), Val::Px(8.0), Val::Px(0.0), Val::Px(12.0)),
        "box-shadow: 0 8px 24px is a deviation of 12 px"
    );
    assert!((color.alpha() - 0.4).abs() < 1e-6);
    assert!((layer.to_bevy(0.5).color.alpha() - 0.2).abs() < 1e-6);
}

#[test]
fn the_high_contrast_skin_casts_no_shadow_and_a_skin_change_restyles_it() {
    let mut app = app(GooeySkin::dark());
    let (screen_panel, _, _, dialog_card) = spawn_surfaces(&mut app);
    assert!(!shadow(&app, screen_panel).0.is_empty());
    app.insert_resource(GooeySkin::high_contrast());
    settle(&mut app);
    assert!(
        shadow(&app, screen_panel).0.is_empty(),
        "its edges carry depth"
    );
    assert!(shadow(&app, dialog_card).0.is_empty());
}

#[test]
fn every_preset_has_stacks_that_rise_but_high_contrast() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        assert!(
            skin.problems().is_empty(),
            "{}: {:?}",
            preset.name(),
            skin.problems()
        );
        let e = &skin.elevation;
        if skin == GooeySkin::high_contrast() {
            assert_eq!(*e, Elevation::none());
            continue;
        }
        let drop = |stack: &[ShadowLayer]| stack.iter().map(|l| l.blur).fold(0.0, f32::max);
        assert!(
            drop(&e.raised) < drop(&e.floating) && drop(&e.floating) < drop(&e.overlay),
            "{}: each height blurs more than the one below",
            preset.name()
        );
    }
}

#[test]
fn a_shadow_that_cannot_be_drawn_is_a_problem() {
    let mut skin = GooeySkin::dark();
    skin.elevation.floating[0].blur = f32::NAN;
    skin.elevation.overlay[0].blur = -1.0;
    let problems = skin.problems();
    for field in ["elevation.floating", "elevation.overlay"] {
        assert!(
            problems.contains(&SkinProblem::BadSize { field }),
            "{field}: {problems:?}"
        );
    }
    assert!(!problems.contains(&SkinProblem::BadSize {
        field: "elevation.raised"
    }));
}

#[test]
fn a_theme_step_carries_the_stacks_of_the_look_it_takes() {
    let dark = GooeySkin::dark();
    assert_eq!(
        dark.clone()
            .with_colours_of(&GooeySkin::high_contrast())
            .elevation,
        Elevation::none(),
        "a step onto high contrast casts no shadow"
    );
    assert_eq!(
        dark.with_colours_of(&GooeySkin::parchment()).elevation,
        GooeySkin::parchment().elevation,
        "a step onto parchment takes its light stacks"
    );
}

#[test]
fn a_role_that_stops_casting_gives_its_shadow_back() {
    let mut app = app(GooeySkin::dark());
    let node = app.world_mut().spawn((Node::default(), Role::Panel)).id();
    settle(&mut app);
    assert!(!shadow(&app, node).0.is_empty());
    app.world_mut().entity_mut(node).insert(Role::Surface(0));
    settle(&mut app);
    assert!(
        shadow(&app, node).0.is_empty(),
        "a surface casts no shadow, and the panel's is taken back"
    );
    // A host's own shadow on a role that casts none is left alone.
    let own = BoxShadow::new(
        Color::BLACK,
        Val::Px(2.0),
        Val::Px(2.0),
        Val::ZERO,
        Val::Px(4.0),
    );
    let host = app
        .world_mut()
        .spawn((Node::default(), Role::Surface(1), own.clone()))
        .id();
    settle(&mut app);
    assert_eq!(shadow(&app, host), own);
}
