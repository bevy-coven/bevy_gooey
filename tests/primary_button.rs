//! The primary button: the one main action on a screen, a [`button`] filled with the accent and
//! labelled in `accent_text` at rest, lifted under the pointer and lifted further while pressed,
//! dimmed like any button when disabled, and checked by the skin for the words on each of those
//! fills and for the steps between them.

use bevy::camera::NormalizedRenderTarget;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{InteractionDisabled, Pressed, UiGlobalTransform};
use bevy::ui_widgets::Activate;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn app_with(skin: GooeySkin) -> App {
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
    for _ in 0..4 {
        app.update();
    }
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .map(|b| b.0)
        .expect("a fill")
}

/// The colour of a button's words.
fn ink(app: &mut App, button: Entity) -> Color {
    let label = app.world().get::<Children>(button).expect("a label")[0];
    app.world().get::<TextColor>(label).expect("ink").0
}

/// A screen with a panel holding a primary button and a plain one: `(primary, plain)`.
fn screen_of(app: &mut App) -> (Entity, Entity) {
    let primary = app.world_mut().spawn(primary_button("Create")).id();
    let plain = app.world_mut().spawn(button("Cancel")).id();
    let card = app.world_mut().spawn(panel()).id();
    app.world_mut()
        .entity_mut(card)
        .add_children(&[primary, plain]);
    app.world_mut().spawn(screen()).add_child(card);
    settle(app);
    (primary, plain)
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

#[test]
fn a_primary_button_is_the_accent_at_rest_and_a_plain_one_is_not() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app_with(skin.clone());
        let (primary, plain) = screen_of(&mut app);
        assert_eq!(fill(&app, primary), skin.accent, "{}", preset.name());
        assert_eq!(
            ink(&mut app, primary),
            skin.accent_text,
            "{}",
            preset.name()
        );
        assert_eq!(fill(&app, plain), skin.surface_raised, "{}", preset.name());
        assert_eq!(ink(&mut app, plain), skin.text, "{}", preset.name());
    }
}

#[test]
fn a_primary_button_lifts_under_the_pointer_and_further_while_pressed() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app_with(skin.clone());
        let (primary, _) = screen_of(&mut app);
        point_at(&mut app, primary);
        assert_eq!(
            fill(&app, primary),
            skin.primary_hovered(),
            "{}",
            preset.name()
        );
        assert_eq!(ink(&mut app, primary), skin.accent_text);
        app.world_mut().entity_mut(primary).insert(Pressed);
        settle(&mut app);
        assert_eq!(
            fill(&app, primary),
            skin.primary_pressed(),
            "{}",
            preset.name()
        );
        assert_eq!(ink(&mut app, primary), skin.accent_text);
        // Three different fills, each readable under the words.
        let states = [skin.accent, skin.primary_hovered(), skin.primary_pressed()];
        assert!(states[0] != states[1] && states[1] != states[2] && states[0] != states[2]);
    }
}

#[test]
fn a_disabled_primary_button_is_dimmed_like_any_button() {
    let skin = GooeySkin::dark();
    let mut app = app_with(skin.clone());
    let (primary, _) = screen_of(&mut app);
    app.world_mut()
        .entity_mut(primary)
        .insert(InteractionDisabled);
    settle(&mut app);
    assert_eq!(fill(&app, primary), skin.surface);
    assert_eq!(ink(&mut app, primary), skin.text_muted);
    app.world_mut()
        .entity_mut(primary)
        .remove::<InteractionDisabled>();
    settle(&mut app);
    assert_eq!(fill(&app, primary), skin.accent, "enabled again");
}

#[test]
fn the_marker_makes_any_button_primary_and_taking_it_off_makes_it_plain() {
    let skin = GooeySkin::parchment();
    let mut app = app_with(skin.clone());
    let (primary, plain) = screen_of(&mut app);
    // The main action moves: the screen's other button becomes the primary one.
    app.world_mut()
        .entity_mut(primary)
        .remove::<PrimaryButton>();
    app.world_mut().entity_mut(plain).insert(PrimaryButton);
    settle(&mut app);
    assert_eq!(fill(&app, primary), skin.surface_raised);
    assert_eq!(ink(&mut app, primary), skin.text);
    assert_eq!(fill(&app, plain), skin.accent);
    assert_eq!(ink(&mut app, plain), skin.accent_text);
}

#[derive(Resource, Default)]
struct Activations(usize);

#[test]
fn a_primary_button_is_a_button_to_the_game_and_to_focus() {
    let mut app = app_with(GooeySkin::dark());
    app.init_resource::<Activations>();
    let (primary, _) = screen_of(&mut app);
    app.world_mut()
        .entity_mut(primary)
        .observe(|_: On<Activate>, mut count: ResMut<Activations>| count.0 += 1);
    app.world_mut().trigger(Activate { entity: primary });
    settle(&mut app);
    assert_eq!(app.world().resource::<Activations>().0, 1);
    assert!(app.world().get::<TabIndex>(primary).is_some(), "a tab stop");
}

#[test]
fn the_words_on_every_primary_fill_and_the_steps_between_them_are_checked() {
    for preset in SkinPreset::ALL {
        let problems = preset.skin().problems();
        assert!(problems.is_empty(), "{}: {problems:?}", preset.name());
    }
    assert!(
        GooeySkin::high_contrast()
            .high_contrast_problems()
            .is_empty()
    );
    // Words the colour of the accent's lift cannot be read on it.
    let mut skin = GooeySkin::dark();
    skin.accent_text = skin.primary_pressed();
    let problems = skin.problems();
    assert!(
        problems.iter().any(|p| matches!(
            p,
            SkinProblem::LowContrast {
                pair: "accent_text on primary pressed",
                ..
            }
        )),
        "{problems:?}"
    );
    assert!(problems.iter().any(|p| matches!(
        p,
        SkinProblem::LowContrast {
            pair: "accent_text on primary hovered",
            ..
        }
    )));
    // An accent whose lift does not move: text the colour of the accent is no lift at all.
    let mut flat = GooeySkin::dark();
    flat.text = flat.accent;
    assert!(
        flat.problems().iter().any(|p| matches!(
            p,
            SkinProblem::Indistinguishable {
                pair: "primary at rest and hovered",
                ..
            }
        )),
        "{:?}",
        flat.problems()
    );
}
