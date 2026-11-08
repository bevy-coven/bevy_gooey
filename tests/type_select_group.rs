//! Type-select in any focus group (G-68): a container with `TypeSelectGroup` type-selects its
//! focusable descendants the way a list does. Laid out for real in a headless window, driven by real
//! `KeyboardInput` on a clock the test sets.

use std::time::Duration;

use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::InteractionDisabled;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::testing::{find_by_label, focused, type_text};
use bevy_gooey::*;

/// One frame of the test's clock.
const FRAME_MS: u64 = 16;

fn app() -> App {
    let mut app = App::new();
    // Assumed owner exception (#76), as `tests/type_select.rs` has it: real-clock tests run with motion off.
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        FRAME_MS,
    )));
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// Runs frames until longer than the type-select pause has passed.
fn pause(app: &mut App) {
    let frames = (TYPE_AHEAD_SECS * 1000.0) as u64 / FRAME_MS + 10;
    for _ in 0..frames {
        app.update();
    }
}

fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    settle(app);
}

fn label(app: &mut App, text: &str) -> Entity {
    find_by_label(app, text).unwrap_or_else(|e| panic!("{text}: {e:?}"))
}

#[test]
fn typing_in_a_group_goes_to_the_row_that_starts_with_it_and_cycles_a_repeated_letter() {
    let mut app = app();
    app.world_mut().spawn((
        Node::default(),
        TypeSelectGroup,
        children![button("Alpha"), button("Beta"), button("Bravo")],
    ));
    settle(&mut app);
    let alpha = label(&mut app, "Alpha");
    focus_on(&mut app, alpha);

    type_text(&mut app, "b");
    assert_eq!(
        focused(&app),
        Some(label(&mut app, "Beta")),
        "one letter: the next row with it"
    );
    type_text(&mut app, "r");
    assert_eq!(
        focused(&app),
        Some(label(&mut app, "Bravo")),
        "within the pause the prefix is `br`"
    );
    assert!(
        app.world().resource::<InputFocusVisible>().0,
        "focus is shown, as keyboard focus is"
    );

    pause(&mut app);
    let alpha = label(&mut app, "Alpha");
    focus_on(&mut app, alpha);
    type_text(&mut app, "b");
    type_text(&mut app, "b");
    assert_eq!(
        focused(&app),
        Some(label(&mut app, "Bravo")),
        "one letter typed again goes on to the next row that starts with it"
    );
}

#[test]
fn a_hidden_row_and_a_disabled_one_are_skipped() {
    let mut app = app();
    app.world_mut().spawn((
        Node::default(),
        TypeSelectGroup,
        children![
            button("Alpha"),
            (button("Bacon"), Visibility::Hidden),
            (button("Bagel"), InteractionDisabled),
            button("Beta"),
        ],
    ));
    settle(&mut app);
    let alpha = label(&mut app, "Alpha");
    focus_on(&mut app, alpha);
    type_text(&mut app, "b");
    assert_eq!(
        focused(&app),
        Some(label(&mut app, "Beta")),
        "Bacon is hidden and Bagel is disabled"
    );
}

#[test]
fn a_text_field_in_the_group_takes_the_characters() {
    let mut app = app();
    app.world_mut().spawn((
        Node::default(),
        TypeSelectGroup,
        children![text_input("Name", ""), button("Banana"), button("Cherry")],
    ));
    settle(&mut app);
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::ui_widgets::TextInput>>()
        .single(app.world())
        .expect("one text field");
    focus_on(&mut app, field);
    type_text(&mut app, "b");
    assert_eq!(focused(&app), Some(field), "focus stays in the field");
}

#[test]
fn without_the_marker_the_same_buttons_type_select_nothing() {
    let mut app = app();
    app.world_mut().spawn((
        Node::default(),
        children![button("Alpha"), button("Beta"), button("Bravo")],
    ));
    settle(&mut app);
    let alpha = label(&mut app, "Alpha");
    focus_on(&mut app, alpha);
    type_text(&mut app, "b");
    assert_eq!(focused(&app), Some(alpha), "no group, no type-select");
}
