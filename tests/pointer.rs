//! A HUD lets the pointer through to the game under it: the empty part of a `screen()` is not
//! hit, while every widget on it, and a dialog's scrim, still take the pointer. Laid out for real
//! in a headless window, driven by real pointer input.

use std::collections::HashSet;

use bevy::camera::NormalizedRenderTarget;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::events::PointerPress;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// A corner of the window, away from the centred widgets.
const CORNER: Vec2 = Vec2::new(5.0, 5.0);

/// A headless app with a real window size, so UI layout and picking run.
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
    for _ in 0..3 {
        app.update();
    }
}

fn location(app: &mut App, at: Vec2) -> Location {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

/// Moves the mouse to `at`, in window px, as the window's own pointer input.
fn point_at(app: &mut App, at: Vec2) {
    let at = location(app, at);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        at,
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    for _ in 0..2 {
        app.update();
    }
}

/// A primary click at `at`: the mouse moves there and settles, then presses and releases.
fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    for action in [
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        let at = location(app, at);
        app.world_mut()
            .write_message(PointerInput::new(PointerId::Mouse, at, action));
        app.update();
    }
    settle(app);
}

/// Every entity the mouse is over.
fn hovered(app: &App) -> HashSet<Entity> {
    app.world()
        .resource::<HoverMap>()
        .get(&PointerId::Mouse)
        .expect("the mouse has a hover set")
        .keys()
        .copied()
        .collect()
}

/// Where `entity` is drawn, in logical px (the window's scale factor is 1).
fn centre(app: &App, entity: Entity) -> Vec2 {
    app.world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation
}

/// A full-window node under the HUD: stands in for the game world, anything the pointer must reach.
fn world_below(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(-1),
        ))
        .id()
}

/// The widgets of a HUD, centred on its screen.
struct Hud {
    screen: Entity,
    panel: Entity,
    pause: Entity,
    bar: Entity,
}

/// A HUD as a game builds one: a panel holding a button, and a meter, on a `screen()`.
fn hud(app: &mut App) -> Hud {
    let world = app.world_mut();
    let pause = world.spawn(button("Pause")).id();
    let panel = world.spawn(panel()).add_child(pause).id();
    let bar = world.spawn(meter(50.0, 100.0)).id();
    let screen = world.spawn(screen()).add_children(&[panel, bar]).id();
    Hud {
        screen,
        panel,
        pause,
        bar,
    }
}

#[test]
fn the_empty_part_of_a_screen_lets_the_pointer_through() {
    let mut app = app();
    let below = world_below(&mut app);
    let hud = hud(&mut app);
    settle(&mut app);
    point_at(&mut app, CORNER);

    let over = hovered(&app);
    assert!(
        !over.contains(&hud.screen),
        "an empty HUD root claims the pointer"
    );
    assert!(over.contains(&below), "what is under the HUD is reachable");
}

#[test]
fn what_a_screen_holds_still_takes_the_pointer() {
    let mut app = app();
    let below = world_below(&mut app);
    let hud = hud(&mut app);
    settle(&mut app);

    for (name, widget) in [
        ("a panel", hud.panel),
        ("a button", hud.pause),
        ("a meter", hud.bar),
    ] {
        let at = centre(&app, widget);
        point_at(&mut app, at);
        let over = hovered(&app);
        assert!(!over.is_empty(), "{name}: nothing is under the pointer");
        assert!(
            !over.contains(&below),
            "{name} lets the pointer through to the world"
        );
    }
}

#[test]
fn a_dialogs_scrim_takes_the_pointer_even_where_it_is_empty() {
    let mut app = app();
    let below = world_below(&mut app);
    let scrim = app
        .world_mut()
        .spawn((dialog(), children![(panel(), children![button("Stay")])]))
        .id();
    settle(&mut app);
    point_at(&mut app, CORNER);

    let over = hovered(&app);
    assert!(over.contains(&scrim), "the scrim is not under the pointer");
    assert!(!over.contains(&below), "the scrim lets the pointer through");
}

#[test]
fn a_screen_that_must_swallow_the_pointer_opts_back_in_with_pickable() {
    let mut app = app();
    let below = world_below(&mut app);
    let hud = hud(&mut app);
    app.world_mut()
        .entity_mut(hud.screen)
        .insert(Pickable::default());
    settle(&mut app);
    point_at(&mut app, CORNER);

    let over = hovered(&app);
    assert!(over.contains(&hud.screen));
    assert!(!over.contains(&below));
}

#[derive(Resource, Default)]
struct Presses {
    world: u32,
    screen: u32,
}

#[test]
fn a_click_on_the_empty_part_of_a_hud_reaches_the_world_and_blurs_the_interface() {
    let mut app = app();
    app.init_resource::<Presses>();
    let below = world_below(&mut app);
    let hud = hud(&mut app);
    app.world_mut().entity_mut(below).observe(
        |_: On<PointerPress>, mut presses: ResMut<Presses>| presses.world += 1,
    );
    app.world_mut().entity_mut(hud.screen).observe(
        |_: On<PointerPress>, mut presses: ResMut<Presses>| presses.screen += 1,
    );
    settle(&mut app);
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(hud.pause, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;

    click_at(&mut app, CORNER);

    let presses = app.world().resource::<Presses>();
    assert_eq!(presses.world, 1, "the press did not reach the world");
    assert_eq!(presses.screen, 0, "the HUD root took the press");
    assert_eq!(
        app.world().resource::<InputFocus>().get(),
        None,
        "a click on the world leaves focus on the HUD"
    );
    assert!(
        !app.world().resource::<InputFocusVisible>().0,
        "a pointer press leaves the focus ring showing"
    );
}
