//! The options screen's rows for the settings the crate itself reads: `text_size`,
//! `interface_size` and `pad_style`. Each is a choice that edits one field of a resource and shows
//! it, so the setting lives once. Laid out for real in a headless window, driven by real keyboard
//! input.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::InteractionDisabled;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<usize>` a game would see: the row, the option and `is_final`.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, usize, bool)>);

/// How many frames each setting was written in, counted by the systems below.
#[derive(Resource, Default)]
struct Writes {
    skin: u32,
    scale: u32,
    binding: u32,
    ui_scale: u32,
    choices: u32,
}

/// A headless app with a 1280 x 720 window, so UI layout runs, and the options a game keeps.
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
    ))
    .init_resource::<Changes>()
    .init_resource::<Writes>()
    .add_observer(
        |change: On<ValueChange<usize>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.source, change.value, change.is_final))
        },
    )
    .add_systems(
        Update,
        (
            |skin: Res<GooeySkin>, mut writes: ResMut<Writes>| {
                writes.skin += u32::from(skin.is_changed())
            },
            |scale: Res<UiScaleSettings>, mut writes: ResMut<Writes>| {
                writes.scale += u32::from(scale.is_changed())
            },
            |binding: Res<BindingSettings>, mut writes: ResMut<Writes>| {
                writes.binding += u32::from(binding.is_changed())
            },
            |ui_scale: Res<UiScale>, mut writes: ResMut<Writes>| {
                writes.ui_scale += u32::from(ui_scale.is_changed())
            },
            |changed: Query<(), Changed<ChoiceValue>>, mut writes: ResMut<Writes>| {
                writes.choices += changed.iter().count() as u32
            },
        ),
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn tap(app: &mut App, key_code: KeyCode) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
}

/// Spawns `rows` in a panel on a screen and lays them out.
fn screen_of(app: &mut App, rows: Vec<Entity>) -> Entity {
    let world = app.world_mut();
    let card = world.spawn(panel()).add_children(&rows).id();
    world.spawn(screen()).add_child(card);
    settle(app);
    card
}

fn selected(app: &App, row: Entity) -> usize {
    app.world().get::<ChoiceValue>(row).expect("a value").0
}

/// The texts of the options a row shows: exactly one, whatever its value.
fn shown(app: &App, row: Entity) -> Vec<String> {
    let mut found = Vec::new();
    let mut stack = vec![row];
    while let Some(entity) = stack.pop() {
        if app.world().get::<Role>(entity) == Some(&Role::ChoiceOption)
            && app.world().get::<Visibility>(entity) != Some(&Visibility::Hidden)
        {
            found.push(app.world().get::<Text>(entity).expect("text").0.clone());
        }
        if let Some(children) = app.world().get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    found
}

fn changes(app: &App) -> Vec<(Entity, usize)> {
    app.world()
        .resource::<Changes>()
        .0
        .iter()
        .map(|&(row, value, _)| (row, value))
        .collect()
}

fn writes(app: &App) -> (u32, u32, u32, u32, u32) {
    let w = app.world().resource::<Writes>();
    (w.skin, w.scale, w.binding, w.ui_scale, w.choices)
}

fn forget_writes(app: &mut App) {
    *app.world_mut().resource_mut::<Writes>() = Writes::default();
}

fn text_scale(app: &App) -> f32 {
    app.world().resource::<GooeySkin>().text_scale
}

// ---- a row shows the setting ----

#[test]
fn a_row_spawned_over_a_loaded_setting_shows_it_on_its_first_frame() {
    let mut app = app();
    // The game has been running, and loads its settings: no resource changes on the frame a menu
    // opens, so the row shows the setting because it was just spawned.
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 1.5;
    app.world_mut().resource_mut::<UiScaleSettings>().scale = 1.25;
    app.world_mut().resource_mut::<BindingSettings>().pad_style = PadStyle::PlayStation;
    settle(&mut app);
    let text = app.world_mut().spawn(text_size("Text size")).id();
    let interface = app.world_mut().spawn(interface_size("Interface size")).id();
    let pad = app.world_mut().spawn(pad_style("Controller")).id();
    // One frame, not six: the row never draws at the step it was spawned on.
    app.update();
    assert_eq!(selected(&app, text), 2);
    assert_eq!(shown(&app, text), ["150%"]);
    assert_eq!(selected(&app, interface), 2);
    assert_eq!(shown(&app, interface), ["125%"]);
    assert_eq!(selected(&app, pad), 1);
    assert_eq!(shown(&app, pad), ["PlayStation"]);
    assert!(changes(&app).is_empty(), "showing a setting is no change");
}

#[test]
fn the_steps_are_the_documented_ones_in_order() {
    let mut app = app();
    let text = app.world_mut().spawn(text_size("Text size")).id();
    let interface = app.world_mut().spawn(interface_size("Interface size")).id();
    let pad = app.world_mut().spawn(pad_style("Controller")).id();
    screen_of(&mut app, vec![text, interface, pad]);
    for (row, last) in [(text, 3), (interface, 3), (pad, 2)] {
        focus(&mut app, row);
        tap(&mut app, KeyCode::Home);
        let mut seen = vec![shown(&app, row).remove(0)];
        for _ in 0..last {
            tap(&mut app, KeyCode::ArrowRight);
            seen.push(shown(&app, row).remove(0));
        }
        let expected: &[&str] = if row == pad {
            &["Xbox", "PlayStation", "Switch"]
        } else if row == text {
            &["100%", "125%", "150%", "200%"]
        } else {
            &["75%", "100%", "125%", "150%"]
        };
        assert_eq!(seen, expected);
    }
}

#[test]
fn the_setting_the_game_sets_moves_the_row_with_no_event() {
    let mut app = app();
    let text = app.world_mut().spawn(text_size("Text size")).id();
    let interface = app.world_mut().spawn(interface_size("Interface size")).id();
    let pad = app.world_mut().spawn(pad_style("Controller")).id();
    screen_of(&mut app, vec![text, interface, pad]);
    assert_eq!(selected(&app, text), 0);

    app.world_mut().resource_mut::<GooeySkin>().text_scale = 2.0;
    app.world_mut().resource_mut::<UiScaleSettings>().scale = 0.75;
    app.world_mut().resource_mut::<BindingSettings>().pad_style = PadStyle::Switch;
    app.update();
    assert_eq!(
        (
            selected(&app, text),
            selected(&app, interface),
            selected(&app, pad)
        ),
        (3, 0, 2)
    );
    assert_eq!(shown(&app, text), ["200%"]);
    assert_eq!(shown(&app, interface), ["75%"]);
    assert_eq!(shown(&app, pad), ["Switch"]);

    // A whole new skin, as a theme switch makes, leaves the row where its text scale says.
    *app.world_mut().resource_mut::<GooeySkin>() = GooeySkin {
        text_scale: 1.25,
        ..GooeySkin::parchment()
    };
    app.update();
    assert_eq!(selected(&app, text), 1);
    assert!(changes(&app).is_empty(), "the game's writes say nothing");
}

#[test]
fn a_value_between_two_steps_shows_the_nearest_and_is_left_alone() {
    let mut app = app();
    app.world_mut().resource_mut::<GooeySkin>().text_scale = 1.1;
    let text = app.world_mut().spawn(text_size("Text size")).id();
    screen_of(&mut app, vec![text]);
    assert_eq!(shown(&app, text), ["100%"], "1.1 is nearer 100% than 125%");
    assert_eq!(
        text_scale(&app),
        1.1,
        "the row does not round the game's value"
    );

    focus(&mut app, text);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(text_scale(&app), 1.25);
    assert_eq!(changes(&app), [(text, 1)]);
}

#[test]
fn a_second_row_follows_the_first_with_no_event_of_its_own() {
    let mut app = app();
    let one = app.world_mut().spawn(text_size("Text size")).id();
    let two = app.world_mut().spawn(text_size("Subtitle size")).id();
    screen_of(&mut app, vec![one, two]);
    focus(&mut app, one);
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!((selected(&app, one), selected(&app, two)), (2, 2));
    assert_eq!(shown(&app, two), ["150%"]);
    assert_eq!(
        changes(&app),
        [(one, 1), (one, 2)],
        "only the player's rows speak"
    );
}

// ---- the player's step ----

#[test]
fn a_step_sets_the_text_scale_and_no_other_field() {
    let mut app = app();
    let theirs = GooeySkin {
        text_size: 22.0,
        gap: 14.0,
        ..GooeySkin::parchment()
    };
    *app.world_mut().resource_mut::<GooeySkin>() = theirs.clone();
    let text = app.world_mut().spawn(text_size("Text size")).id();
    screen_of(&mut app, vec![text]);
    focus(&mut app, text);
    forget_writes(&mut app);

    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        *app.world().resource::<GooeySkin>(),
        GooeySkin {
            text_scale: 1.25,
            ..theirs.clone()
        }
    );
    assert_eq!(changes(&app), [(text, 1)], "one step, one change");
    assert_eq!(writes(&app).0, 1, "and one write of the skin");

    for expected in [1.5, 2.0] {
        tap(&mut app, KeyCode::ArrowRight);
        assert_eq!(text_scale(&app), expected);
    }
    // The end is the end: nothing is written, nothing said.
    forget_writes(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(text_scale(&app), 2.0);
    assert_eq!(writes(&app), (0, 0, 0, 0, 0));
    assert_eq!(changes(&app).len(), 3);

    tap(&mut app, KeyCode::Home);
    assert_eq!(
        *app.world().resource::<GooeySkin>(),
        GooeySkin {
            text_scale: 1.0,
            ..theirs
        },
        "Home is the default size again"
    );
}

#[test]
fn a_step_of_the_interface_size_sets_the_scale_and_not_the_reference() {
    let mut app = app();
    *app.world_mut().resource_mut::<UiScaleSettings>() = UiScaleSettings {
        reference_short_side: 720.0,
        scale: 1.0,
    };
    let row = app.world_mut().spawn(interface_size("Interface size")).id();
    screen_of(&mut app, vec![row]);
    assert_eq!(app.world().resource::<UiScale>().0, 1.0);
    focus(&mut app, row);

    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        *app.world().resource::<UiScaleSettings>(),
        UiScaleSettings {
            reference_short_side: 720.0,
            scale: 1.25
        }
    );
    // The window is 720 high, so the interface is drawn at exactly the player's size, the frame
    // after the step.
    assert_eq!(app.world().resource::<UiScale>().0, 1.25);

    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(app.world().resource::<UiScale>().0, 0.75);
    assert_eq!(
        app.world()
            .resource::<UiScaleSettings>()
            .reference_short_side,
        720.0
    );
}

#[test]
fn a_step_of_the_controller_renames_every_pad_button_and_changes_nothing_else() {
    let mut app = app();
    *app.world_mut().resource_mut::<BindingSettings>() = BindingSettings {
        key_prompt: "Hit a key ({exit} leaves)".to_owned(),
        ..default()
    };
    let pad = app.world_mut().spawn(pad_style("Controller")).id();
    let use_row = app
        .world_mut()
        .spawn(binding("Use", Binding::Pad(GamepadButton::South)))
        .id();
    screen_of(&mut app, vec![pad, use_row]);
    let cap = |app: &App| -> String {
        let mut stack = vec![use_row];
        while let Some(entity) = stack.pop() {
            if app.world().get::<Role>(entity) == Some(&Role::BindingText) {
                return app.world().get::<Text>(entity).expect("text").0.clone();
            }
            if let Some(children) = app.world().get::<Children>(entity) {
                stack.extend(children.iter());
            }
        }
        panic!("a keycap under the binding row");
    };
    assert_eq!(cap(&app), "A");

    focus(&mut app, pad);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(cap(&app), "Cross", "the same frame the player steps");
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(cap(&app), "B", "South is B on a Switch pad");
    let settings = app.world().resource::<BindingSettings>();
    assert_eq!(settings.pad_style, PadStyle::Switch);
    assert_eq!(settings.key_prompt, "Hit a key ({exit} leaves)");
    assert_eq!(settings.cancel_key, KeyCode::Escape);
}

#[test]
fn a_disabled_row_does_nothing() {
    let mut app = app();
    let text = app.world_mut().spawn(text_size("Text size")).id();
    screen_of(&mut app, vec![text]);
    app.world_mut().entity_mut(text).insert(InteractionDisabled);
    focus(&mut app, text);
    forget_writes(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::End);
    assert_eq!(text_scale(&app), 1.0);
    assert_eq!(selected(&app, text), 0);
    assert!(changes(&app).is_empty());
    assert_eq!(writes(&app), (0, 0, 0, 0, 0));
}

// ---- a plugin left out ----

#[test]
fn a_row_whose_plugin_is_left_out_steps_and_breaks_nothing() {
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
        GooeyStylePlugin,
        GooeyNavPlugin,
    ));
    app.world_mut().spawn(Camera2d);
    let row = app.world_mut().spawn(interface_size("Interface size")).id();
    let text = app.world_mut().spawn(text_size("Text size")).id();
    screen_of(&mut app, vec![row, text]);
    assert!(app.world().get_resource::<UiScaleSettings>().is_none());
    focus(&mut app, row);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(selected(&app, row), 1, "it steps, as any choice does");
    assert!(app.world().get_resource::<UiScaleSettings>().is_none());
    focus(&mut app, text);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        text_scale(&app),
        1.25,
        "and the plugins that are there still work"
    );
}

// ---- cost ----

#[test]
fn nothing_is_written_while_the_rows_idle() {
    let mut app = app();
    let text = app.world_mut().spawn(text_size("Text size")).id();
    let interface = app.world_mut().spawn(interface_size("Interface size")).id();
    let pad = app.world_mut().spawn(pad_style("Controller")).id();
    screen_of(&mut app, vec![text, interface, pad]);
    focus(&mut app, text);
    tap(&mut app, KeyCode::ArrowRight);
    settle(&mut app);
    forget_writes(&mut app);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(writes(&app), (0, 0, 0, 0, 0));
}
