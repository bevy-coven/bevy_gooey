//! The choice: a settings row that steps through a few named options. It keeps its state, shows one
//! option, takes the keys, the pad and the pointer, and never moves under the player's thumb. Laid
//! out for real in a headless window, driven by real keyboard, pointer and gamepad input.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{InteractionDisabled, Pressed, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<usize>` a game would see: the option and `is_final`.
#[derive(Resource, Default)]
struct Changes(Vec<(usize, bool)>);

/// Every `UiFeedback` written.
#[derive(Resource, Default)]
struct Feedback(Vec<UiFeedback>);

/// A headless app with a `width` × `height` window, so UI layout and picking run.
fn app_sized(width: u32, height: u32) -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(width, height)
                        .with_scale_factor_override(1.0),
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
    .init_resource::<Feedback>()
    .add_observer(
        |change: On<ValueChange<usize>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.value, change.is_final))
        },
    )
    .add_systems(
        Update,
        |mut messages: MessageReader<UiFeedback>, mut seen: ResMut<Feedback>| {
            seen.0.extend(messages.read().copied());
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// An options column as a game builds it, centred on a `screen()`: a switch, a choice of three and
/// a button.
struct Options {
    subtitles: Entity,
    quality: Entity,
    back: Entity,
}

fn options(app: &mut App) -> Options {
    let world = app.world_mut();
    let subtitles = world.spawn(toggle("Subtitles", true)).id();
    let quality = world
        .spawn(choice("Quality", ["Low", "Medium", "High"], 1))
        .id();
    let back = world.spawn(button("Back")).id();
    let card = world
        .spawn(panel())
        .add_children(&[subtitles, quality, back])
        .id();
    world.spawn(screen()).add_child(card);
    settle(app);
    Options {
        subtitles,
        quality,
        back,
    }
}

/// A choice with a button to its right, in a row at the window's top-left corner, clear of the
/// options column: the layout in which Right would leave the choice if it were navigation.
fn beside(app: &mut App) -> (Entity, Entity) {
    let world = app.world_mut();
    let side = world.spawn(choice("Side", ["One", "Two", "Three"], 0)).id();
    let next = world.spawn(button("Next")).id();
    world
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            column_gap: Val::Px(10.0),
            ..default()
        })
        .add_children(&[side, next]);
    settle(app);
    let (s, n) = (rect(app, side), rect(app, next));
    assert!(n.min.x > s.max.x, "the button is to the choice's right");
    (side, next)
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            for child in children.iter() {
                found.push(child);
                stack.push(child);
            }
        }
    }
    found
}

/// Every part of `root` with `role`, left to right.
fn parts(app: &App, root: Entity, role: Role) -> Vec<Entity> {
    let mut found: Vec<Entity> = descendants(app.world(), root)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&role))
        .collect();
    found.sort_by(|&a, &b| rect(app, a).center().x.total_cmp(&rect(app, b).center().x));
    found
}

/// The part of `root` with `role`.
fn part(app: &App, root: Entity, role: Role) -> Entity {
    *parts(app, root, role)
        .first()
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

/// Where `entity` is drawn, in logical px (the window's scale factor and `UiScale` are 1).
fn rect(app: &App, entity: Entity) -> Rect {
    let size = app
        .world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size();
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    Rect::from_center_size(at, size)
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

fn location(app: &mut App, at: Vec2) -> Location {
    let window = window(app);
    Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    }
}

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let at = location(app, at);
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, at, action));
    app.update();
}

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

fn plug_in_pad(app: &mut App) -> Entity {
    let pad = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(GamepadConnectionEvent::new(
        pad,
        GamepadConnection::Connected {
            name: "Test pad".into(),
            vendor_id: None,
            product_id: None,
        },
    ));
    app.update();
    pad
}

fn pad_button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
    app.world_mut()
        .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            pad, button, value,
        )));
    app.update();
}

/// Presses and releases `button` on `pad`, one update each.
fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    pad_button(app, pad, button, 1.0);
    pad_button(app, pad, button, 0.0);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

fn selected(app: &App, choice: Entity) -> usize {
    app.world().get::<ChoiceValue>(choice).expect("a value").0
}

/// The texts of the options a choice shows: exactly one, whatever its value.
fn shown(app: &App, choice: Entity) -> Vec<String> {
    descendants(app.world(), choice)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&Role::ChoiceOption))
        .filter(|&e| app.world().get::<Visibility>(e) != Some(&Visibility::Hidden))
        .map(|e| app.world().get::<Text>(e).expect("text").0.clone())
        .collect()
}

fn changes(app: &App) -> Vec<usize> {
    app.world()
        .resource::<Changes>()
        .0
        .iter()
        .map(|&(value, _)| value)
        .collect()
}

fn activations(app: &App) -> usize {
    app.world()
        .resource::<Feedback>()
        .0
        .iter()
        .filter(|f| f.kind == UiFeedbackKind::Activate)
        .count()
}

fn skin() -> GooeySkin {
    GooeySkin::default()
}

fn border_of(app: &App, entity: Entity) -> BorderColor {
    *app.world().get::<BorderColor>(entity).expect("a border")
}

/// The colour a choice's chevrons are drawn in: `(back, forward)`.
fn chevrons(app: &App, choice: Entity) -> (BorderColor, BorderColor) {
    (
        border_of(app, part(app, choice, Role::ChoiceBack)),
        border_of(app, part(app, choice, Role::ChoiceForward)),
    )
}

// ---- the state ----

#[test]
fn a_choice_keeps_its_option_and_says_each_change_once() {
    let mut app = app();
    let ui = options(&mut app);
    assert_eq!(selected(&app, ui.quality), 1, "spawned on Medium");
    assert_eq!(shown(&app, ui.quality), ["Medium"], "one option at a time");

    focus(&mut app, ui.quality);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(selected(&app, ui.quality), 2);
    assert_eq!(shown(&app, ui.quality), ["High"]);
    assert_eq!(changes(&app), [2]);
    assert!(app.world().resource::<Changes>().0[0].1, "a step is final");

    // Against the last option Right is the end: nothing changes and nothing is said.
    tap(&mut app, KeyCode::ArrowRight);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(selected(&app, ui.quality), 2);
    assert_eq!(changes(&app), [2]);

    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::ArrowLeft);
    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(
        selected(&app, ui.quality),
        0,
        "Left stops at the first, no wrap"
    );
    assert_eq!(shown(&app, ui.quality), ["Low"]);
    assert_eq!(changes(&app), [2, 1, 0]);
    assert_eq!(activations(&app), 3, "one UiFeedback::Activate a change");

    tap(&mut app, KeyCode::End);
    assert_eq!(selected(&app, ui.quality), 2, "End jumps to the last");
    tap(&mut app, KeyCode::End);
    tap(&mut app, KeyCode::Home);
    assert_eq!(selected(&app, ui.quality), 0, "Home jumps to the first");
    assert_eq!(changes(&app), [2, 1, 0, 2, 0]);
}

#[test]
fn a_choice_the_game_sets_follows_at_once_and_a_value_past_the_end_is_pulled_back() {
    let mut app = app();
    let ui = options(&mut app);

    app.world_mut()
        .entity_mut(ui.quality)
        .insert(ChoiceValue(0));
    app.update();
    assert_eq!(shown(&app, ui.quality), ["Low"]);
    assert!(
        changes(&app).is_empty(),
        "a change the game made itself is not reported back to it"
    );

    app.world_mut()
        .entity_mut(ui.quality)
        .insert(ChoiceValue(7));
    app.update();
    assert_eq!(selected(&app, ui.quality), 2, "pulled back to the last");
    assert_eq!(shown(&app, ui.quality), ["High"]);

    // Spawned past the end, it starts on the last.
    let late = app
        .world_mut()
        .spawn(choice("Late", ["A", "B", "C"], 9))
        .id();
    settle(&mut app);
    assert_eq!(selected(&app, late), 2);
    assert_eq!(shown(&app, late), ["C"]);
}

#[test]
fn a_choice_with_no_options_does_nothing_and_does_not_panic() {
    let mut app = app();
    let none = app
        .world_mut()
        .spawn(choice("Empty", Vec::<String>::new(), 0))
        .id();
    settle(&mut app);
    focus(&mut app, none);
    for key in [
        KeyCode::ArrowRight,
        KeyCode::ArrowLeft,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Enter,
    ] {
        tap(&mut app, key);
    }
    assert!(changes(&app).is_empty());
    assert!(shown(&app, none).is_empty());
    let (back, forward) = chevrons(&app, none);
    assert_eq!(back, forward, "both arrows read as spent");
    assert_eq!(back, BorderColor::all(skin().text_muted));
}

#[test]
fn enter_space_and_south_cycle_round_and_wrap() {
    let mut app = app();
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.quality);

    tap(&mut app, KeyCode::Enter);
    assert_eq!(selected(&app, ui.quality), 2);
    tap(&mut app, KeyCode::Enter);
    assert_eq!(
        selected(&app, ui.quality),
        0,
        "Enter wraps from the last to the first"
    );
    tap(&mut app, KeyCode::Space);
    assert_eq!(selected(&app, ui.quality), 1);
    press(&mut app, pad, GamepadButton::South);
    assert_eq!(selected(&app, ui.quality), 2);
    press(&mut app, pad, GamepadButton::South);
    assert_eq!(selected(&app, ui.quality), 0, "South wraps too");
    assert_eq!(
        changes(&app),
        [2, 0, 1, 2, 0],
        "one ValueChange per press, no press counted twice"
    );
    assert_eq!(activations(&app), 5);
}

#[test]
fn the_d_pad_steps_a_choice_and_stops_at_its_ends() {
    let mut app = app();
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.quality);

    press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(selected(&app, ui.quality), 2);
    press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(selected(&app, ui.quality), 2, "the last option is the end");
    press(&mut app, pad, GamepadButton::DPadLeft);
    assert_eq!(selected(&app, ui.quality), 1);
    assert_eq!(
        focused(&app),
        Some(ui.quality),
        "Left and Right never leave it"
    );
    assert_eq!(changes(&app), [2, 1]);
}

#[test]
fn a_held_d_pad_direction_walks_to_the_end_and_stops_there() {
    let mut app = app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.quality);
    tap(&mut app, KeyCode::Home);
    assert_eq!(selected(&app, ui.quality), 0);

    pad_button(&mut app, pad, GamepadButton::DPadRight, 1.0);
    assert_eq!(selected(&app, ui.quality), 1, "the press itself steps once");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        selected(&app, ui.quality),
        1,
        "nothing repeats before {PAD_REPEAT_DELAY} s"
    );
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(selected(&app, ui.quality), 2, "the repeats reach the end");
    assert_eq!(
        changes(&app),
        [0, 1, 2],
        "and say nothing while held against it"
    );
}

// ---- the pointer ----

#[test]
fn a_click_on_an_arrow_steps_once_and_a_dimmed_arrow_does_nothing() {
    let mut app = app();
    let ui = options(&mut app);
    let arrows = parts(&app, ui.quality, Role::ChoiceArrow);
    assert_eq!(arrows.len(), 2);
    let (back, forward) = (
        rect(&app, arrows[0]).center(),
        rect(&app, arrows[1]).center(),
    );

    click_at(&mut app, forward);
    assert_eq!(selected(&app, ui.quality), 2, "the forward arrow steps on");
    assert_eq!(changes(&app), [2]);
    assert_eq!(activations(&app), 1);
    assert_eq!(
        focused(&app),
        Some(ui.quality),
        "a press on an arrow focuses the row"
    );
    assert!(
        !app.world().resource::<InputFocusVisible>().0,
        "a pointer press hides the ring"
    );
    assert!(
        app.world().get::<Pressed>(ui.quality).is_none(),
        "the row is not left pressed"
    );

    // The last option: the forward arrow is dimmed, and a click on it is no click on the row
    // either (which would cycle round to the first).
    click_at(&mut app, forward);
    assert_eq!(selected(&app, ui.quality), 2);
    assert_eq!(changes(&app), [2]);

    click_at(&mut app, back);
    assert_eq!(selected(&app, ui.quality), 1, "the back arrow steps back");
    click_at(&mut app, back);
    click_at(&mut app, back);
    assert_eq!(selected(&app, ui.quality), 0, "and stops at the first");
    assert_eq!(changes(&app), [2, 1, 0]);
}

#[test]
fn a_click_on_the_label_or_the_value_cycles_to_the_next_option() {
    let mut app = app();
    let ui = options(&mut app);

    let label = rect(&app, part(&app, ui.quality, Role::ControlText)).center();
    click_at(&mut app, label);
    assert_eq!(selected(&app, ui.quality), 2, "a click on the label");
    let value = rect(&app, part(&app, ui.quality, Role::ChoiceCell)).center();
    click_at(&mut app, value);
    assert_eq!(
        selected(&app, ui.quality),
        0,
        "a click on the value wraps round"
    );
    assert_eq!(changes(&app), [2, 0]);
    assert_eq!(focused(&app), Some(ui.quality));
}

#[test]
fn an_arrow_under_the_pointer_lights_up_unless_it_is_spent() {
    let mut app = app();
    let ui = options(&mut app);
    let arrows = parts(&app, ui.quality, Role::ChoiceArrow);
    let (back, forward) = (
        rect(&app, arrows[0]).center(),
        rect(&app, arrows[1]).center(),
    );
    let (live, accent, muted) = (
        BorderColor::all(skin().text),
        BorderColor::all(skin().accent),
        BorderColor::all(skin().text_muted),
    );
    assert_eq!(
        chevrons(&app, ui.quality),
        (live, live),
        "at rest, both live on Medium"
    );

    point_at(&mut app, forward);
    assert_eq!(
        chevrons(&app, ui.quality),
        (live, accent),
        "the arrow under the pointer"
    );
    point_at(&mut app, back);
    assert_eq!(chevrons(&app, ui.quality), (accent, live));

    tap_home_and_point(&mut app, ui.quality, back);
    assert_eq!(
        chevrons(&app, ui.quality),
        (muted, live),
        "the back arrow on the first option stays dim under the pointer"
    );
}

fn tap_home_and_point(app: &mut App, choice: Entity, at: Vec2) {
    focus(app, choice);
    tap(app, KeyCode::Home);
    point_at(app, at);
    settle(app);
}

// ---- disabled ----

#[test]
fn a_disabled_choice_ignores_keys_the_pad_and_the_pointer_and_reads_dim() {
    let mut app = app();
    let ui = options(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.quality);
    app.world_mut()
        .entity_mut(ui.quality)
        .insert(InteractionDisabled);
    settle(&mut app);

    for key in [
        KeyCode::ArrowRight,
        KeyCode::ArrowLeft,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Enter,
        KeyCode::Space,
    ] {
        tap(&mut app, key);
    }
    press(&mut app, pad, GamepadButton::DPadLeft);
    press(&mut app, pad, GamepadButton::South);
    let arrows = parts(&app, ui.quality, Role::ChoiceArrow);
    for target in [
        rect(&app, arrows[0]).center(),
        rect(&app, arrows[1]).center(),
        rect(&app, part(&app, ui.quality, Role::ChoiceCell)).center(),
    ] {
        click_at(&mut app, target);
    }

    assert_eq!(selected(&app, ui.quality), 1);
    assert!(changes(&app).is_empty());
    let muted = BorderColor::all(skin().text_muted);
    assert_eq!(chevrons(&app, ui.quality), (muted, muted));
    let option = parts(&app, ui.quality, Role::ChoiceOption)[1];
    assert_eq!(
        app.world().get::<TextColor>(option).map(|c| c.0),
        Some(skin().text_muted),
        "the value dims with the row"
    );
}

// ---- navigation ----

#[test]
fn left_and_right_never_leave_a_choice_and_up_and_down_move_on() {
    let mut app = app();
    let ui = options(&mut app);
    focus(&mut app, ui.quality);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(ui.back), "Down moves focus on");
    focus(&mut app, ui.quality);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(focused(&app), Some(ui.subtitles), "Up moves focus back");

    let mut app = app_sized(1280, 720);
    let (side, next) = beside(&mut app);
    focus(&mut app, side);
    tap(&mut app, KeyCode::End);
    assert_eq!(selected(&app, side), 2);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(
        focused(&app),
        Some(side),
        "Right at the last option is the end of the choice, not a move to {next:?}"
    );
    let pad = plug_in_pad(&mut app);
    press(&mut app, pad, GamepadButton::DPadRight);
    assert_eq!(focused(&app), Some(side), "and so is the D-pad's");
}

#[test]
fn a_choice_gaining_focus_writes_feedback_and_shows_the_ring() {
    let mut app = app();
    let ui = options(&mut app);
    focus(&mut app, ui.quality);
    app.update();
    let feedback = &app.world().resource::<Feedback>().0;
    assert!(
        feedback.contains(&UiFeedback {
            entity: ui.quality,
            kind: UiFeedbackKind::Focus
        }),
        "focus on a choice is heard like focus on a button"
    );
    assert_eq!(
        border_of(&app, ui.quality),
        BorderColor::all(skin().focus_ring),
        "the row's border is the ring"
    );
    focus(&mut app, ui.back);
    assert_eq!(
        border_of(&app, ui.quality),
        BorderColor::all(Color::NONE),
        "and nothing at rest"
    );
}

// ---- the look ----

#[test]
fn the_row_is_as_high_as_a_toggle_and_lines_up_with_it() {
    let mut app = app();
    let ui = options(&mut app);
    let (toggle_row, row) = (rect(&app, ui.subtitles), rect(&app, ui.quality));
    assert!(
        (row.height() - toggle_row.height()).abs() <= 1.0,
        "{} high against a toggle's {}",
        row.height(),
        toggle_row.height()
    );
    assert!(
        (row.width() - toggle_row.width()).abs() < 0.5,
        "one column, one width"
    );

    let label_x = |root| rect(&app, part(&app, root, Role::ControlText)).min.x;
    assert!(
        (label_x(ui.quality) - label_x(ui.subtitles)).abs() < 0.5,
        "the labels start at one x"
    );

    // The forward chevron's ink ends where the switch's track does.
    let track = rect(&app, part(&app, ui.subtitles, Role::ToggleTrack));
    let (ink_start, ink_width) = ink(&app, ui.quality, Role::ChoiceForward);
    assert!(
        (ink_start + ink_width - track.max.x).abs() < 2.0,
        "the chevron ends at {} and the switch at {}",
        ink_start + ink_width,
        track.max.x
    );
}

#[test]
fn the_row_scales_with_the_window() {
    let mut app = app_sized(2560, 1440);
    let ui = options(&mut app);
    let (toggle_row, row) = (rect(&app, ui.subtitles), rect(&app, ui.quality));
    assert!((row.height() - toggle_row.height()).abs() <= 1.0);
    assert!(
        (row.height() - 2.0 * 52.8).abs() < 1.5,
        "twice the 720p row's 52.8 px, not {}",
        row.height()
    );
    for arrow in parts(&app, ui.quality, Role::ChoiceArrow) {
        let target = rect(&app, arrow);
        assert!(target.width() >= 88.0 - 0.5 && target.height() >= 88.0 - 0.5);
    }
}

#[test]
fn the_arrows_are_targets_a_thumb_can_hit() {
    let mut app = app();
    let ui = options(&mut app);
    for arrow in parts(&app, ui.quality, Role::ChoiceArrow) {
        let target = rect(&app, arrow);
        assert!(
            target.width() >= COMFORT_TARGET_PX - 0.5 && target.height() >= COMFORT_TARGET_PX - 0.5,
            "an arrow {} x {}",
            target.width(),
            target.height()
        );
    }
}

#[test]
fn stepping_through_every_option_moves_nothing() {
    let mut app = app();
    let ui = options(&mut app);
    // A wider option than the rest is the case that would shift the arrows if it were laid out
    // alone.
    let wide = app
        .world_mut()
        .spawn(choice("Mode", ["Off", "A fairly long name", "On"], 0))
        .id();
    let card = app
        .world()
        .get::<ChildOf>(ui.back)
        .expect("in the card")
        .parent();
    app.world_mut().entity_mut(card).add_child(wide);
    settle(&mut app);

    for choice in [ui.quality, wide] {
        focus(&mut app, choice);
        tap(&mut app, KeyCode::Home);
        let layout = |app: &App| {
            let arrows = parts(app, choice, Role::ChoiceArrow);
            (
                rect(app, choice),
                rect(app, arrows[0]),
                rect(app, arrows[1]),
                rect(app, part(app, choice, Role::ChoiceCell)),
            )
        };
        let before = layout(&app);
        for _ in 0..2 {
            tap(&mut app, KeyCode::ArrowRight);
            settle(&mut app);
            assert_eq!(
                layout(&app),
                before,
                "the row, arrows and cell stay where they are"
            );
        }
    }
    let cell = rect(&app, part(&app, wide, Role::ChoiceCell)).width();
    let widest = descendants(app.world(), wide)
        .into_iter()
        .filter(|&e| app.world().get::<Role>(e) == Some(&Role::ChoiceOption))
        .map(|e| rect(&app, e).width())
        .fold(0.0, f32::max);
    assert!(
        cell >= widest,
        "the cell ({cell}) holds the widest option ({widest})"
    );
}

#[test]
fn the_chevron_is_drawn_in_one_line_weight_and_its_ink_is_centred_in_its_arrow() {
    let mut app = app();
    let ui = options(&mut app);
    let line = app
        .world()
        .get::<ComputedNode>(ui.quality)
        .expect("laid out")
        .border
        .min_inset
        .x;
    assert_eq!(line, 3.0, "the row's own frame is the skin's border");
    let arrows = parts(&app, ui.quality, Role::ChoiceArrow);

    for (arrow, role) in arrows
        .into_iter()
        .zip([Role::ChoiceBack, Role::ChoiceForward])
    {
        let mark = part(&app, ui.quality, role);
        let node = app.world().get::<ComputedNode>(mark).expect("laid out");
        let side = node.size().x;
        let sides = node.border;
        let forward = role == Role::ChoiceForward;
        let (kept, dropped) = if forward {
            (
                [sides.max_inset.x, sides.min_inset.y],
                [sides.min_inset.x, sides.max_inset.y],
            )
        } else {
            (
                [sides.min_inset.x, sides.max_inset.y],
                [sides.max_inset.x, sides.min_inset.y],
            )
        };
        assert_eq!(kept, [line, line], "{role:?}: two sides, the skin's weight");
        assert_eq!(dropped, [0.0, 0.0], "{role:?}: and no other");
        assert!(
            side > 0.0 && (node.size().y - side).abs() < 0.01,
            "a square"
        );

        let (start, width) = ink(&app, ui.quality, role);
        let middle = start + width / 2.0;
        assert!(
            (middle - rect(&app, arrow).center().x).abs() < 1.0,
            "{role:?}: the ink's middle {middle} in an arrow centred at {}",
            rect(&app, arrow).center().x
        );
    }
}

/// Where a chevron's ink is, left to right: its start and its width. The square is turned an
/// eighth of a turn about its centre, so its two drawn sides run from the centre line to the
/// corner, half a diagonal away, on the side the chevron points to.
fn ink(app: &App, choice: Entity, role: Role) -> (f32, f32) {
    let mark = part(app, choice, role);
    let side = rect(app, mark).width();
    let centre = app
        .world()
        .get::<UiGlobalTransform>(mark)
        .expect("placed")
        .translation
        .x;
    let reach = side * std::f32::consts::FRAC_1_SQRT_2;
    if role == Role::ChoiceForward {
        (centre, reach)
    } else {
        (centre - reach, reach)
    }
}

#[test]
fn the_chevron_keeps_the_rows_whole_pixel_weight_at_fractional_scales() {
    // 1920x1080 draws at 1.5x, 1280x800 at 1.11x: the scales at which a border can split.
    for (width, height) in [(1920, 1080), (1280, 800), (2560, 1440)] {
        let mut app = app_sized(width, height);
        let ui = options(&mut app);
        let row = app
            .world()
            .get::<ComputedNode>(ui.quality)
            .expect("laid out")
            .border;
        assert_eq!(row.min_inset.x, row.max_inset.x);
        assert_eq!(row.min_inset.x, row.min_inset.x.round(), "a whole pixel");
        for (role, kept) in [(Role::ChoiceBack, true), (Role::ChoiceForward, false)] {
            let mark = app
                .world()
                .get::<ComputedNode>(part(&app, ui.quality, role))
                .expect("laid out")
                .border;
            let (first, second) = if kept {
                (mark.min_inset.x, mark.max_inset.y)
            } else {
                (mark.max_inset.x, mark.min_inset.y)
            };
            assert_eq!(
                [first, second],
                [row.min_inset.x; 2],
                "{role:?} at {width}x{height}: both sides the weight of the row's frame"
            );
        }
    }
}

#[test]
fn a_choice_has_a_role_for_every_part_and_the_skin_has_no_problem_with_it() {
    let mut app = app();
    let ui = options(&mut app);
    for role in [
        Role::Choice,
        Role::ControlText,
        Role::ChoiceArrow,
        Role::ChoiceBack,
        Role::ChoiceForward,
        Role::ChoiceCell,
        Role::ChoiceOption,
    ] {
        let found = descendants(app.world(), ui.quality)
            .into_iter()
            .chain([ui.quality])
            .any(|e| app.world().get::<Role>(e) == Some(&role));
        assert!(found, "a {role:?} on a choice");
    }
    assert!(GooeySkin::dark().problems().is_empty());
    assert!(GooeySkin::parchment().problems().is_empty());
}
