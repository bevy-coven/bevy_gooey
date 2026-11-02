//! The radial menu: items on a ring round a disc, chosen by the direction of a stick, the pointer,
//! the arrows or the D-pad. Laid out for real in a headless window, driven by real keyboard,
//! pointer and gamepad input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{InteractionDisabled, Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<usize>` a game would see: the item and `is_final`.
#[derive(Resource, Default)]
struct Changes(Vec<(usize, bool)>);

const ITEMS: [&str; 8] = [
    "Pistol", "Rifle", "Shotgun", "Grenade", "Knife", "Bow", "Torch", "Medkit",
];

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
    .insert_resource(UiScaleSettings {
        reference_short_side: height as f32,
        ..default()
    })
    .init_resource::<Changes>()
    .add_observer(
        |change: On<ValueChange<usize>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.value, change.is_final))
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

/// A wheel of the eight `ITEMS`, highlighting `selected`, at the top-left of the window, and a
/// button to its right (so an arrow that would move focus has somewhere to go).
struct Wheel {
    radial: Entity,
    next: Entity,
}

fn wheel_of(app: &mut App, items: &[&str], selected: usize) -> Wheel {
    let world = app.world_mut();
    let radial = world.spawn(radial(items.iter().copied(), selected)).id();
    let next = world.spawn(button("Next")).id();
    world
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            column_gap: Val::Px(20.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[radial, next]);
    settle(app);
    Wheel { radial, next }
}

fn wheel(app: &mut App) -> Wheel {
    wheel_of(app, &ITEMS, 0)
}

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

/// The items of the wheel, in ring order.
fn slots(app: &App, radial: Entity) -> Vec<Entity> {
    let mut found: Vec<(usize, Entity)> = descendants(app.world(), radial)
        .into_iter()
        .filter_map(|e| app.world().get::<RadialSlot>(e).map(|s| (s.index(), e)))
        .collect();
    found.sort();
    found.into_iter().map(|(_, e)| e).collect()
}

fn centre_text(app: &App, radial: Entity) -> String {
    descendants(app.world(), radial)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&Role::RadialCentre))
        .and_then(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .expect("a name in the middle")
}

fn lit(app: &App, radial: Entity) -> Vec<usize> {
    slots(app, radial)
        .iter()
        .enumerate()
        .filter(|&(_, &s)| app.world().get::<Selected>(s).is_some())
        .map(|(i, _)| i)
        .collect()
}

fn value(app: &App, radial: Entity) -> usize {
    app.world().get::<RadialValue>(radial).expect("a value").0
}

fn changes(app: &App) -> Vec<(usize, bool)> {
    app.world().resource::<Changes>().0.clone()
}

fn clear_changes(app: &mut App) {
    app.world_mut().resource_mut::<Changes>().0.clear();
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

fn pointer(app: &mut App, at: Vec2, action: PointerAction) {
    let window = window(app);
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: at,
    };
    app.world_mut()
        .write_message(PointerInput::new(PointerId::Mouse, location, action));
    app.update();
}

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::X });
    app.update();
}

fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

/// The point on the disc that faces item `index` of `count`, `share` of the way to the edge.
fn toward(app: &App, radial: Entity, index: usize, count: usize, share: f32) -> Vec2 {
    let disc = rect(app, radial);
    let (x, y) = radial_place(index, count);
    disc.center() + Vec2::new(x, -y) * disc.width() * 0.5 * share
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

fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    pad_button(app, pad, button, 1.0);
    pad_button(app, pad, button, 0.0);
}

/// Pushes a stick of `pad` toward `(x, y)` (up is positive `y`).
fn stick(app: &mut App, pad: Entity, left: bool, (x, y): (f32, f32)) {
    let (ax, ay) = if left {
        (GamepadAxis::LeftStickX, GamepadAxis::LeftStickY)
    } else {
        (GamepadAxis::RightStickX, GamepadAxis::RightStickY)
    };
    for (axis, v) in [(ax, x), (ay, y)] {
        app.world_mut()
            .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                pad, axis, v,
            )));
    }
    app.update();
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

// ---- what is drawn ----

#[test]
fn the_highlighted_item_alone_is_lit_and_its_name_is_in_the_middle() {
    let mut app = app();
    let ui = wheel_of(&mut app, &ITEMS, 2);
    assert_eq!(value(&app, ui.radial), 2);
    assert_eq!(lit(&app, ui.radial), [2]);
    assert_eq!(centre_text(&app, ui.radial), "Shotgun");

    // A value the game sets moves the highlight and the name the same frame, with no event.
    app.world_mut()
        .get_mut::<RadialValue>(ui.radial)
        .expect("a value")
        .0 = 5;
    app.update();
    assert_eq!(lit(&app, ui.radial), [5]);
    assert_eq!(centre_text(&app, ui.radial), "Bow");
    assert!(
        changes(&app).is_empty(),
        "the game's own write says nothing"
    );

    // One past the last item reads as the last.
    app.world_mut()
        .get_mut::<RadialValue>(ui.radial)
        .expect("a value")
        .0 = 99;
    settle(&mut app);
    assert_eq!(value(&app, ui.radial), 7);
    assert_eq!(lit(&app, ui.radial), [7]);
}

#[test]
fn the_lit_item_is_the_accent_with_inverted_text_and_the_others_a_buttons_fill() {
    let mut app = app();
    let ui = wheel_of(&mut app, &ITEMS, 1);
    let skin = app.world().resource::<GooeySkin>().clone();
    let slots = slots(&app, ui.radial);
    for (index, &slot) in slots.iter().enumerate() {
        let fill = app.world().get::<BackgroundColor>(slot).expect("a fill").0;
        let text = descendants(app.world(), slot)
            .into_iter()
            .find_map(|e| app.world().get::<TextColor>(e).map(|c| c.0))
            .expect("a label");
        if index == 1 {
            assert_eq!(fill, skin.accent);
            assert_eq!(text, skin.accent_text);
        } else {
            assert_eq!(fill, skin.surface_raised);
            assert_eq!(text, skin.text);
        }
    }
}

/// Checks a wheel of `ITEMS` in a window `width` × `height` at text scale `step`: the disc is 26
/// text sizes square, or the 92% of the window's shorter side it is capped to, the items stand on
/// the ring, inside the round disc and a target high, and no two overlap.
fn check_wheel(width: u32, height: u32, step: f32) {
    let mut app = app_sized(width, height);
    app.world_mut().resource_mut::<GooeySkin>().text_scale = step;
    let ui = wheel(&mut app);
    let disc = rect(&app, ui.radial);
    let skin = app.world().resource::<GooeySkin>().clone();
    let short = width.min(height) as f32;
    let across = (skin.text_size * step * 26.0).min(short * 0.92);
    let at = format!("{width}x{height} at {step}");
    assert!(
        (disc.width() - across).abs() < 1.0 && (disc.height() - across).abs() < 1.0,
        "{at}: the disc is {across} square, {disc:?}"
    );
    assert!(
        disc.width() <= short,
        "{at}: the disc is whole on the screen"
    );
    let slots = slots(&app, ui.radial);
    assert_eq!(slots.len(), 8);
    let rects: Vec<Rect> = slots.iter().map(|&s| rect(&app, s)).collect();
    for (index, item) in rects.iter().enumerate() {
        let (x, y) = radial_place(index, 8);
        let want = disc.center() + Vec2::new(x, -y) * across * 0.36;
        // The ring is 36% of the padding box, which the frame trims from the disc's width.
        assert!(
            (item.center() - want).length() < 0.36 * 2.0 * skin.border * 1.5 + 1.0,
            "{at}: item {index} stands on the ring, {:?} vs {want:?}",
            item.center()
        );
        for corner in [
            item.min,
            item.max,
            Vec2::new(item.min.x, item.max.y),
            Vec2::new(item.max.x, item.min.y),
        ] {
            assert!(
                (corner - disc.center()).length() <= disc.width() * 0.5,
                "{at}: item {index} is inside the round disc, corner {corner:?}"
            );
        }
        assert!(
            item.height() + 0.5 >= skin.target(),
            "{at}: item {index} is a target high"
        );
        for (other, rival) in rects.iter().enumerate().skip(index + 1) {
            let overlap = item.intersect(*rival);
            assert!(
                overlap.is_empty() || overlap.width().min(overlap.height()) < 1.0,
                "{at}: items {index} and {other} overlap by {overlap:?}"
            );
        }
    }
}

#[test]
fn the_disc_is_round_and_its_items_stand_on_the_ring_clear_of_one_another() {
    // A window tall enough that the disc is never capped, at every text size.
    for step in TEXT_SIZE_STEPS {
        check_wheel(1280, 1080, step);
    }
}

#[test]
fn a_small_window_at_large_text_gets_a_whole_wheel_not_a_bigger_one() {
    // 200% text would make a 1040 px disc in 720 px; it stops at 92% of the window and the labels
    // stop growing with it, so nothing overlaps or leaves the disc.
    for step in TEXT_SIZE_STEPS {
        check_wheel(1280, 720, step);
    }
    check_wheel(960, 540, 2.0);
}

#[test]
fn the_items_stand_on_a_quiet_track_round_the_middle_that_the_pointer_goes_through() {
    let mut app = app();
    let ui = wheel(&mut app);
    let disc = rect(&app, ui.radial);
    let track = descendants(app.world(), ui.radial)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&Role::RadialTrack))
        .expect("a track");
    let track_rect = rect(&app, track);
    assert!((track_rect.center() - disc.center()).length() < 1.0);
    // Its line runs through the items' middles: a circle of the ring's radius.
    let skin = app.world().resource::<GooeySkin>().clone();
    let want = (disc.width() - 2.0 * skin.border) * 0.36 * 2.0;
    assert!(
        (track_rect.width() - want).abs() < 1.5,
        "{track_rect:?} vs {want}"
    );
    assert_eq!(
        app.world().get::<Pickable>(track),
        Some(&Pickable::IGNORE),
        "the pointer reaches the disc through it"
    );
    assert_eq!(
        *app.world().get::<BorderColor>(track).expect("a border"),
        BorderColor::all(skin.surface_raised)
    );
}

#[test]
fn ten_short_items_fit_the_ring_too() {
    let mut app = app_sized(1280, 1080);
    let labels = [
        "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten",
    ];
    let ui = wheel_of(&mut app, &labels, 0);
    let rects: Vec<Rect> = slots(&app, ui.radial)
        .iter()
        .map(|&s| rect(&app, s))
        .collect();
    assert_eq!(rects.len(), 10);
    for (a, first) in rects.iter().enumerate() {
        for (b, second) in rects.iter().enumerate().skip(a + 1) {
            let overlap = first.intersect(*second);
            assert!(
                overlap.is_empty() || overlap.width().min(overlap.height()) < 1.0,
                "items {a} and {b} overlap by {overlap:?}"
            );
        }
    }
}

#[test]
fn the_disc_wears_the_ring_only_while_it_has_focus() {
    let mut app = app();
    let ui = wheel(&mut app);
    let skin = app.world().resource::<GooeySkin>().clone();
    let edge = |app: &App| *app.world().get::<BorderColor>(ui.radial).expect("a border");
    assert_eq!(edge(&app), BorderColor::all(skin.surface_raised));
    focus(&mut app, ui.radial);
    settle(&mut app);
    assert_eq!(edge(&app), BorderColor::all(skin.focus_ring));
    focus(&mut app, ui.next);
    settle(&mut app);
    assert_eq!(edge(&app), BorderColor::all(skin.surface_raised));
}

// ---- a stick ----

#[test]
fn a_stick_points_at_the_item_it_faces_and_says_each_move_once() {
    let mut app = app();
    let ui = wheel(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);
    clear_changes(&mut app);

    for item in [2, 5, 7, 3, 0] {
        let (x, y) = radial_place(item, ITEMS.len());
        stick(&mut app, pad, true, (x, y));
        // Holding the stick there says nothing more.
        stick(&mut app, pad, true, (x, y));
        settle(&mut app);
        assert_eq!(value(&app, ui.radial), item, "the stick faces item {item}");
        assert_eq!(lit(&app, ui.radial), [item]);
        assert_eq!(centre_text(&app, ui.radial), ITEMS[item]);
    }
    assert_eq!(
        changes(&app),
        [(2, false), (5, false), (7, false), (3, false), (0, false)],
        "one non-final change per move"
    );
}

#[test]
fn the_right_stick_points_too_and_the_one_pushed_further_wins() {
    let mut app = app();
    let ui = wheel(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);

    stick(&mut app, pad, false, (1.0, 0.0));
    assert_eq!(value(&app, ui.radial), 2, "the right stick, to the right");

    // A hard push down on the left stick beats a light brush up on the right.
    stick(&mut app, pad, false, (0.0, 0.6));
    stick(&mut app, pad, true, (0.0, -1.0));
    settle(&mut app);
    assert_eq!(value(&app, ui.radial), 4, "down is item 4 of 8");
}

#[test]
fn a_stick_inside_the_dead_zone_leaves_the_highlight_where_it_is() {
    let mut app = app();
    let ui = wheel_of(&mut app, &ITEMS, 3);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);
    clear_changes(&mut app);

    // A brush at 40% of its travel, toward item 6, is not a point; neither is the stick coming
    // back through the middle.
    let (x, y) = radial_place(6, ITEMS.len());
    stick(&mut app, pad, true, (x * 0.4, y * 0.4));
    stick(&mut app, pad, true, (0.0, 0.0));
    settle(&mut app);
    assert_eq!(value(&app, ui.radial), 3);
    assert!(changes(&app).is_empty());

    stick(&mut app, pad, true, (x * 0.6, y * 0.6));
    assert_eq!(value(&app, ui.radial), 6, "past half its travel is a point");
}

#[test]
fn a_stick_moves_nothing_unless_the_radial_has_focus() {
    let mut app = app();
    let ui = wheel(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.next);
    stick(&mut app, pad, true, (1.0, 0.0));
    settle(&mut app);
    assert_eq!(value(&app, ui.radial), 0);
}

#[test]
fn a_stick_shows_focus() {
    let mut app = app();
    let ui = wheel(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);
    app.world_mut().insert_resource(InputFocusVisible(false));
    stick(&mut app, pad, true, (1.0, 0.0));
    assert!(app.world().resource::<InputFocusVisible>().0);
}

// ---- the D-pad and the keys ----

#[test]
fn the_d_pad_steps_round_the_ring_and_moves_no_focus() {
    let mut app = app();
    let ui = wheel(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);

    press(&mut app, pad, GamepadButton::DPadRight);
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(value(&app, ui.radial), 2, "Right then Down go forward");
    press(&mut app, pad, GamepadButton::DPadLeft);
    press(&mut app, pad, GamepadButton::DPadUp);
    press(&mut app, pad, GamepadButton::DPadUp);
    assert_eq!(value(&app, ui.radial), 7, "back round the start");
    settle(&mut app);
    assert_eq!(focused(&app), Some(ui.radial), "the radial keeps focus");
}

#[test]
fn the_arrows_step_and_wrap_home_and_end_jump_and_no_focus_moves() {
    let mut app = app();
    let ui = wheel(&mut app);
    focus(&mut app, ui.radial);

    tap(&mut app, KeyCode::ArrowLeft);
    assert_eq!(
        value(&app, ui.radial),
        7,
        "back from the first wraps to the last"
    );
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(value(&app, ui.radial), 0);
    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(value(&app, ui.radial), 2);
    tap(&mut app, KeyCode::ArrowUp);
    assert_eq!(value(&app, ui.radial), 1);
    tap(&mut app, KeyCode::End);
    assert_eq!(value(&app, ui.radial), 7);
    tap(&mut app, KeyCode::Home);
    assert_eq!(value(&app, ui.radial), 0);
    settle(&mut app);
    assert_eq!(
        focused(&app),
        Some(ui.radial),
        "Right and Down never leave for the button beside it"
    );
    assert!(
        changes(&app).iter().all(|&(_, last)| !last),
        "steps are not choices"
    );
}

#[test]
fn an_arrow_on_a_radial_shows_focus() {
    let mut app = app();
    let ui = wheel(&mut app);
    focus(&mut app, ui.radial);
    app.world_mut().insert_resource(InputFocusVisible(false));
    tap(&mut app, KeyCode::ArrowRight);
    assert!(app.world().resource::<InputFocusVisible>().0);
}

// ---- choosing ----

#[test]
fn enter_space_and_south_choose_the_highlight_once_each_even_when_it_did_not_move() {
    let mut app = app();
    let ui = wheel_of(&mut app, &ITEMS, 4);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);
    clear_changes(&mut app);

    tap(&mut app, KeyCode::Enter);
    assert_eq!(changes(&app), [(4, true)]);
    tap(&mut app, KeyCode::Space);
    press(&mut app, pad, GamepadButton::South);
    assert_eq!(changes(&app), [(4, true), (4, true), (4, true)]);
    assert_eq!(value(&app, ui.radial), 4, "choosing moves nothing");
}

#[test]
fn a_game_chooses_for_the_player_by_triggering_activate() {
    let mut app = app();
    let ui = wheel(&mut app);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);
    // The player holds a button to open the wheel, points, and lets go.
    stick(&mut app, pad, true, radial_place(6, ITEMS.len()));
    clear_changes(&mut app);
    app.world_mut()
        .commands()
        .trigger(bevy::ui_widgets::Activate { entity: ui.radial });
    settle(&mut app);
    assert_eq!(changes(&app), [(6, true)]);
}

#[test]
fn the_pointer_over_a_wedge_points_at_it_and_a_click_chooses_it_once() {
    let mut app = app();
    let ui = wheel(&mut app);
    clear_changes(&mut app);

    for item in [3, 6, 1] {
        let at = toward(&app, ui.radial, item, ITEMS.len(), 0.7);
        point_at(&mut app, at);
        assert_eq!(
            value(&app, ui.radial),
            item,
            "the pointer faces item {item}"
        );
    }
    assert_eq!(changes(&app), [(3, false), (6, false), (1, false)]);

    clear_changes(&mut app);
    let at = toward(&app, ui.radial, 5, ITEMS.len(), 0.7);
    click_at(&mut app, at);
    assert_eq!(
        changes(&app),
        [(5, false), (5, true)],
        "the press points, the click chooses, once"
    );
}

#[test]
fn the_whole_wedge_is_the_target_not_just_the_label() {
    let mut app = app();
    let ui = wheel(&mut app);
    // Near the rim and well off the ring's label, still item 2 (right).
    let disc = rect(&app, ui.radial);
    let near_rim = disc.center() + Vec2::new(disc.width() * 0.46, disc.height() * 0.05);
    point_at(&mut app, near_rim);
    assert_eq!(value(&app, ui.radial), 2);
    // And just inside the dead circle, nothing changes.
    clear_changes(&mut app);
    let inside = disc.center() + Vec2::new(-disc.width() * 0.05, 0.0);
    point_at(&mut app, inside);
    assert_eq!(
        value(&app, ui.radial),
        2,
        "the dead circle holds the highlight"
    );
    assert!(changes(&app).is_empty());
}

#[test]
fn a_press_in_the_middle_chooses_what_is_already_highlighted() {
    let mut app = app();
    let ui = wheel_of(&mut app, &ITEMS, 6);
    clear_changes(&mut app);
    let at = rect(&app, ui.radial).center();
    click_at(&mut app, at);
    assert_eq!(changes(&app), [(6, true)]);
}

#[test]
fn a_press_takes_focus_and_a_pointer_that_rests_says_nothing() {
    let mut app = app();
    let ui = wheel(&mut app);
    let at = toward(&app, ui.radial, 2, ITEMS.len(), 0.7);
    click_at(&mut app, at);
    assert_eq!(focused(&app), Some(ui.radial));
    clear_changes(&mut app);
    let at = toward(&app, ui.radial, 2, ITEMS.len(), 0.7);
    for _ in 0..5 {
        point_at(&mut app, at);
    }
    assert!(changes(&app).is_empty());
}

// ---- disabled and idle ----

#[test]
fn a_disabled_radial_takes_nothing_and_reads_dim() {
    let mut app = app();
    let ui = wheel_of(&mut app, &ITEMS, 1);
    let pad = plug_in_pad(&mut app);
    focus(&mut app, ui.radial);
    app.world_mut()
        .entity_mut(ui.radial)
        .insert(InteractionDisabled);
    settle(&mut app);
    clear_changes(&mut app);

    for key in [
        KeyCode::ArrowRight,
        KeyCode::ArrowLeft,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Enter,
    ] {
        tap(&mut app, key);
    }
    press(&mut app, pad, GamepadButton::DPadRight);
    press(&mut app, pad, GamepadButton::South);
    stick(&mut app, pad, true, (1.0, 0.0));
    let at = toward(&app, ui.radial, 4, ITEMS.len(), 0.7);
    click_at(&mut app, at);
    assert_eq!(value(&app, ui.radial), 1);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));

    let skin = app.world().resource::<GooeySkin>().clone();
    let slots = slots(&app, ui.radial);
    let fill = app
        .world()
        .get::<BackgroundColor>(slots[1])
        .expect("fill")
        .0;
    assert_eq!(fill, skin.surface, "not lit while disabled");
}

#[test]
fn an_idle_radial_writes_nothing() {
    let mut app = app();
    let ui = wheel(&mut app);
    focus(&mut app, ui.radial);
    settle(&mut app);
    app.world_mut().clear_trackers();
    for _ in 0..30 {
        app.update();
    }
    let world = app.world();
    assert!(
        !world
            .entity(ui.radial)
            .get_ref::<RadialValue>()
            .expect("v")
            .is_changed()
    );
    assert!(
        !world
            .entity(ui.radial)
            .get_ref::<Node>()
            .expect("n")
            .is_changed()
    );
    for slot in slots(&app, ui.radial) {
        let e = world.entity(slot);
        assert!(!e.get_ref::<Node>().expect("n").is_changed());
        assert!(!e.get_ref::<BackgroundColor>().expect("b").is_changed());
    }
}

/// A wheel of the eight `ITEMS` where every item in `with_picture` has a picture and the rest are
/// text, highlighting `selected`, placed as `wheel_of` does.
fn picture_wheel(app: &mut App, with_picture: &[usize], selected: usize) -> Wheel {
    let source = IconSource::Image(
        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default()),
    );
    let items: Vec<RadialItem> = ITEMS
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let item = RadialItem::new(*name);
            if with_picture.contains(&index) {
                item.icon(source.clone())
            } else {
                item
            }
        })
        .collect();
    let world = app.world_mut();
    let radial = world.spawn(radial(items, selected)).id();
    let next = world.spawn(button("Next")).id();
    world
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            column_gap: Val::Px(20.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .add_children(&[radial, next]);
    settle(app);
    Wheel { radial, next }
}

fn picture_of(app: &App, slot: Entity) -> Option<Entity> {
    descendants(app.world(), slot)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&Role::RadialIcon))
}

fn has_text(app: &App, slot: Entity) -> bool {
    descendants(app.world(), slot)
        .into_iter()
        .any(|e| app.world().get::<Text>(e).is_some())
}

#[test]
fn an_item_with_a_picture_draws_the_picture_and_no_words_and_the_centre_still_names_it() {
    let mut app = app();
    let all: Vec<usize> = (0..ITEMS.len()).collect();
    let ui = picture_wheel(&mut app, &all, 2);
    for &slot in &slots(&app, ui.radial) {
        let picture = picture_of(&app, slot).expect("a picture");
        assert!(app.world().get::<ImageNode>(picture).is_some());
        assert!(
            !has_text(&app, slot),
            "a picture item has no words of its own"
        );
    }
    assert_eq!(centre_text(&app, ui.radial), "Shotgun");
    tap_focused(&mut app, ui.radial, KeyCode::ArrowRight);
    assert_eq!(centre_text(&app, ui.radial), "Grenade");
    app.world_mut().entity_mut(ui.radial).insert(RadialValue(6));
    settle(&mut app);
    assert_eq!(
        centre_text(&app, ui.radial),
        "Torch",
        "the game's own value too"
    );
    assert_eq!(lit(&app, ui.radial), vec![6]);
}

/// Focus the wheel, then tap `key`.
fn tap_focused(app: &mut App, radial: Entity, key: KeyCode) {
    focus(app, radial);
    tap(app, key);
}

#[test]
fn a_mixed_wheel_keeps_its_words_where_it_has_no_picture() {
    let mut app = app();
    let ui = picture_wheel(&mut app, &[0, 2, 4], 1);
    let slots = slots(&app, ui.radial);
    for (index, &slot) in slots.iter().enumerate() {
        let pictured = [0, 2, 4].contains(&index);
        assert_eq!(picture_of(&app, slot).is_some(), pictured, "item {index}");
        assert_eq!(has_text(&app, slot), !pictured, "item {index}");
    }
    assert_eq!(
        centre_text(&app, ui.radial),
        "Rifle",
        "a text item still names itself"
    );
    tap_focused(&mut app, ui.radial, KeyCode::ArrowRight);
    assert_eq!(
        centre_text(&app, ui.radial),
        "Shotgun",
        "and a picture item names itself too"
    );
}

#[test]
fn the_picture_is_the_texts_colour_and_the_accents_ink_on_the_lit_item_and_dim_when_disabled() {
    let mut app = app();
    let all: Vec<usize> = (0..ITEMS.len()).collect();
    let ui = picture_wheel(&mut app, &all, 3);
    let skin = app.world().resource::<GooeySkin>().clone();
    let tint = |app: &App, slot: Entity| {
        let picture = picture_of(app, slot).expect("a picture");
        app.world().get::<ImageNode>(picture).expect("image").color
    };
    for (index, &slot) in slots(&app, ui.radial).iter().enumerate() {
        let want = if index == 3 {
            skin.accent_text
        } else {
            skin.text
        };
        assert_eq!(tint(&app, slot), want, "item {index}");
    }
    app.world_mut()
        .entity_mut(ui.radial)
        .insert(InteractionDisabled);
    settle(&mut app);
    for (index, &slot) in slots(&app, ui.radial).iter().enumerate() {
        assert_eq!(tint(&app, slot), skin.text_muted, "disabled item {index}");
    }
}

/// A wheel of eight pictures in a window `width` × `height` at text scale `step`: the pictures
/// are square, 3 text sizes of the wheel's own size, the items stand clear of one another and
/// inside the round disc, and each is a target high.
fn check_picture_wheel(width: u32, height: u32, step: f32) {
    let mut app = app_sized(width, height);
    app.world_mut().resource_mut::<GooeySkin>().text_scale = step;
    let all: Vec<usize> = (0..ITEMS.len()).collect();
    let ui = picture_wheel(&mut app, &all, 0);
    let disc = rect(&app, ui.radial);
    let skin = app.world().resource::<GooeySkin>().clone();
    let short = width.min(height) as f32;
    let wheel_px = (skin.text_size * step).min(short * 0.92 / 26.0);
    let at = format!("{width}x{height} at {step}");
    let slots = slots(&app, ui.radial);
    let rects: Vec<Rect> = slots.iter().map(|&s| rect(&app, s)).collect();
    for (index, &slot) in slots.iter().enumerate() {
        let picture = rect(&app, picture_of(&app, slot).expect("a picture"));
        assert!(
            (picture.width() - wheel_px * 3.0).abs() < 1.0
                && (picture.height() - wheel_px * 3.0).abs() < 1.0,
            "{at}: item {index}'s picture is {} square, {picture:?}",
            wheel_px * 3.0
        );
        let item = rects[index];
        assert!(
            item.contains(picture.min) && item.contains(picture.max),
            "{at}: item {index} holds its picture"
        );
        assert!(item.height() + 0.5 >= skin.target(), "{at}: a target high");
        assert!(
            (item.width() - item.height()).abs() < 1.5,
            "{at}: item {index} is a square, {item:?}"
        );
        for corner in [
            item.min,
            item.max,
            Vec2::new(item.min.x, item.max.y),
            Vec2::new(item.max.x, item.min.y),
        ] {
            assert!(
                (corner - disc.center()).length() <= disc.width() * 0.5,
                "{at}: item {index} is inside the round disc"
            );
        }
        for (other, rival) in rects.iter().enumerate().skip(index + 1) {
            let overlap = item.intersect(*rival);
            assert!(
                overlap.is_empty() || overlap.width().min(overlap.height()) < 1.0,
                "{at}: items {index} and {other} overlap by {overlap:?}"
            );
        }
    }
}

#[test]
fn eight_pictures_fit_the_ring_at_every_text_size_and_in_a_short_window() {
    for step in TEXT_SIZE_STEPS {
        check_picture_wheel(1280, 1080, step);
    }
    check_picture_wheel(1280, 720, 2.0);
}

#[test]
fn an_idle_wheel_of_pictures_writes_nothing() {
    let mut app = app();
    let all: Vec<usize> = (0..ITEMS.len()).collect();
    let ui = picture_wheel(&mut app, &all, 0);
    focus(&mut app, ui.radial);
    settle(&mut app);
    app.world_mut().clear_trackers();
    for _ in 0..30 {
        app.update();
    }
    let world = app.world();
    for slot in slots(&app, ui.radial) {
        let picture = picture_of(&app, slot).expect("a picture");
        let e = world.entity(picture);
        assert!(!e.get_ref::<Node>().expect("n").is_changed());
        assert!(!e.get_ref::<ImageNode>().expect("i").is_changed());
    }
}
