//! The colour field: a text row whose text is a hex colour and whose swatch opens a picker (a
//! saturation and value area and strips for hue, brightness and alpha). Laid out for real in a
//! headless window, driven by real pointer, keyboard and gamepad input.

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
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// Every `ValueChange<Srgba>` a game would see: the row, the colour and whether the change ended.
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, Srgba, bool)>);

/// Every `ColorRefused`: the text and the reason.
#[derive(Resource, Default)]
struct Refusals(Vec<(String, HexError)>);

/// How many frames' writes to what a number row shows the app has seen.
#[derive(Resource, Default)]
struct Writes(u32);

/// What a frame at rest must not write.
type Written = Or<(
    Changed<Text>,
    Changed<TextInputValue>,
    Changed<ColorValue>,
    Changed<ColorPicker>,
    Changed<BackgroundGradient>,
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<TextColor>,
)>;

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
    .init_resource::<Refusals>()
    .init_resource::<Writes>()
    .add_observer(
        |change: On<ValueChange<Srgba>>, mut seen: ResMut<Changes>| {
            seen.0.push((change.source, change.value, change.is_final));
        },
    )
    .add_observer(|refused: On<ColorRefused>, mut seen: ResMut<Refusals>| {
        seen.0.push((refused.text.clone(), refused.reason));
    })
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.world_mut().spawn(Camera2d);
    app
}

fn app() -> App {
    app_sized(1280, 720)
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
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

fn part(app: &App, root: Entity, role: Role) -> Entity {
    descendants(app.world(), root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

fn field(app: &App, row: Entity) -> Entity {
    part(app, row, Role::TextField)
}

/// The row's label: the text that is the scrub handle.
fn label_of(app: &App, row: Entity) -> Entity {
    part(app, row, Role::ControlText)
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

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window")
}

fn key_event(app: &mut App, key_code: KeyCode, logical_key: Key, state: ButtonState, repeat: bool) {
    let window = window(app);
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state,
        text: None,
        repeat,
        window,
    });
    app.update();
}

/// A named key pressed and released, one update each.
fn tap_named(app: &mut App, key_code: KeyCode, logical_key: Key) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        key_event(app, key_code, logical_key.clone(), state, false);
    }
    settle(app);
}

fn tap(app: &mut App, key_code: KeyCode) {
    let key = match key_code {
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::Enter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::End => Key::End,
        _ => Key::Unidentified(NativeKey::Unidentified),
    };
    tap_named(app, key_code, key);
}

/// A key held (down without up) and later let go.
fn hold(app: &mut App, key_code: KeyCode) {
    key_event(
        app,
        key_code,
        Key::Unidentified(NativeKey::Unidentified),
        ButtonState::Pressed,
        false,
    );
}

fn let_go(app: &mut App, key_code: KeyCode) {
    key_event(
        app,
        key_code,
        Key::Unidentified(NativeKey::Unidentified),
        ButtonState::Released,
        false,
    );
}

fn type_char(app: &mut App, c: char) {
    let window = window(app);
    let text = c.to_string();
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyA,
            logical_key: Key::Character(text.as_str().into()),
            state,
            text: (state == ButtonState::Pressed).then(|| text.as_str().into()),
            repeat: false,
            window,
        });
        app.update();
    }
}

fn type_str(app: &mut App, text: &str) {
    for c in text.chars() {
        type_char(app, c);
    }
    settle(app);
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

fn point_at(app: &mut App, to: Vec2) {
    pointer(app, to, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
}

/// Presses on `from`, moves to `to` and leaves the pointer pressed there.
fn grab_and_move(app: &mut App, from: Vec2, to: Vec2) {
    point_at(app, from);
    pointer(app, from, PointerAction::Press(PointerButton::Primary));
    pointer(app, to, PointerAction::Move { delta: to - from });
    settle(app);
}

fn release(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
    settle(app);
}

/// A press and release on `at` without moving.
fn click_at(app: &mut App, at: Vec2) {
    point_at(app, at);
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    release(app, at);
}

/// Drags from the middle of `from` by `by` pointer px along x, and lets go.
fn drag_from(app: &mut App, from: Entity, by: f32) {
    let start = rect(app, from).center();
    let to = start + Vec2::new(by, 0.0);
    grab_and_move(app, start, to);
    release(app, to);
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

fn focus_field(app: &mut App, row: Entity) {
    let field = field(app, row);
    focus(app, field);
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
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

fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
    settle(app);
}

// ---------------------------------------------------------------------------------------------

const START: &str = "#336699";

fn hex(text: &str) -> Srgba {
    Srgba::hex(text).expect("a hex colour")
}

/// A panel with a colour row (a blue), a second colour row and a button under them.
struct Rig {
    tint: Entity,
    other: Entity,
    button: Entity,
}

fn rig(app: &mut App) -> Rig {
    let tint = app.world_mut().spawn(color_field("Tint", hex(START))).id();
    let other = app
        .world_mut()
        .spawn(color_field("Glow", hex("#112233")))
        .id();
    let button = app.world_mut().spawn(button("Apply")).id();
    app.world_mut()
        .spawn(panel())
        .add_children(&[tint, other, button]);
    settle(app);
    Rig {
        tint,
        other,
        button,
    }
}

fn colour(app: &App, row: Entity) -> Srgba {
    app.world().get::<ColorValue>(row).expect("a colour").0
}

fn near(a: Srgba, b: Srgba) -> bool {
    a.to_f32_array()
        .iter()
        .zip(b.to_f32_array())
        .all(|(x, y)| (x - y).abs() < 2e-3)
}

fn text(app: &App, row: Entity) -> String {
    app.world()
        .get::<TextInputValue>(row)
        .expect("a text")
        .0
        .clone()
}

/// What the field itself holds.
fn typed(app: &App, row: Entity) -> String {
    let field = field(app, row);
    app.world()
        .get::<bevy::text::EditableText>(field)
        .expect("an editable text")
        .value()
        .to_string()
}

fn changes(app: &App) -> Vec<(Srgba, bool)> {
    app.world()
        .resource::<Changes>()
        .0
        .iter()
        .map(|(_, v, f)| (*v, *f))
        .collect()
}

fn forget(app: &mut App) {
    app.world_mut().resource_mut::<Changes>().0.clear();
    app.world_mut().resource_mut::<Refusals>().0.clear();
}

fn refusals(app: &App) -> Vec<(String, HexError)> {
    app.world().resource::<Refusals>().0.clone()
}

/// Types `words` into `row`'s field in place of what it holds, and leaves it focused.
fn enter_text(app: &mut App, row: Entity, words: &str) {
    focus_field(app, row);
    tap(app, KeyCode::End);
    for _ in 0..16 {
        tap(app, KeyCode::Backspace);
    }
    type_str(app, words);
}

fn swatch(app: &App, row: Entity) -> Entity {
    part(app, row, Role::ColorSwatch)
}

fn picker(app: &mut App) -> Option<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<ColorPicker>>()
        .iter(app.world())
        .next()
}

fn open_picker(app: &mut App, row: Entity) -> Entity {
    let at = rect(app, swatch(app, row)).center();
    click_at(app, at);
    settle(app);
    picker(app).expect("the swatch opened a picker")
}

fn area(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<ColorArea>>()
        .iter(app.world())
        .next()
        .expect("an area in the picker")
}

/// The picker's strips, top to bottom: hue, brightness, alpha.
fn strips(app: &mut App) -> Vec<Entity> {
    let mut found: Vec<Entity> = app
        .world_mut()
        .query_filtered::<Entity, With<ColorStrip>>()
        .iter(app.world())
        .collect();
    found.sort_by(|a, b| rect(app, *a).min.y.total_cmp(&rect(app, *b).min.y));
    found
}

fn hsv(app: &mut App) -> [f32; 3] {
    let picker = picker(app).expect("a picker");
    app.world().get::<ColorPicker>(picker).expect("state").hsv()
}

fn slider_value(app: &App, strip: Entity) -> f32 {
    app.world().get::<SliderValue>(strip).expect("a value").0
}

/// The marker's centre and the area's inside, in window px.
fn marker_and_inside(app: &mut App) -> (Vec2, Rect) {
    let area = area(app);
    let marker = part(app, area, Role::ColorAreaMarker);
    let node = app.world().get::<ComputedNode>(area).expect("laid out");
    let border = node.border();
    let outer = rect(app, area);
    let inside = Rect::new(
        outer.min.x + border.min_inset.x,
        outer.min.y + border.min_inset.y,
        outer.max.x - border.max_inset.x,
        outer.max.y - border.max_inset.y,
    );
    (rect(app, marker).center(), inside)
}

/// The swatch's fill on its opaque half (`alpha` false, the first child) or its alpha half.
fn half(app: &App, row: Entity, alpha: bool) -> Color {
    let swatch = swatch(app, row);
    let children = app.world().get::<Children>(swatch).expect("two halves");
    assert_eq!(children.len(), 2);
    app.world()
        .get::<BackgroundColor>(children[usize::from(alpha)])
        .expect("a fill")
        .0
}

// --- the typed colour -------------------------------------------------------------------------

#[test]
fn typing_ff8800_and_enter_sets_the_colour_in_one_final_change() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    enter_text(&mut app, ui.tint, "#ff8800");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(colour(&app, ui.tint), hex("#ff8800"));
    assert_eq!(changes(&app), vec![(hex("#ff8800"), true)]);
    assert_eq!(
        text(&app, ui.tint),
        "#FF8800",
        "shown as the field shows colours"
    );
    assert_eq!(typed(&app, ui.tint), "#FF8800");
    assert!(refusals(&app).is_empty());
    // The swatch is the colour, opaque on both halves.
    assert_eq!(half(&app, ui.tint, false), Color::Srgba(hex("#ff8800")));
    assert_eq!(half(&app, ui.tint, true), Color::Srgba(hex("#ff8800")));
}

#[test]
fn shorthand_a_missing_hash_and_an_alpha_are_taken() {
    let mut app = app();
    let ui = rig(&mut app);
    enter_text(&mut app, ui.tint, "f80");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(colour(&app, ui.tint), hex("#ff8800"));
    enter_text(&mut app, ui.tint, "#ff880080");
    tap(&mut app, KeyCode::Enter);
    let c = colour(&app, ui.tint);
    assert!((c.alpha - 128.0 / 255.0).abs() < 1e-6, "{c:?}");
    assert_eq!(text(&app, ui.tint), "#FF880080");
    // The swatch shows the colour opaque on one half and at its alpha on the other.
    assert_eq!(half(&app, ui.tint, false), Color::Srgba(hex("#ff8800")));
    assert_eq!(half(&app, ui.tint, true), Color::Srgba(c));
}

#[test]
fn a_word_a_short_digit_count_and_nothing_are_refused_with_their_reason_and_the_text_put_back() {
    let mut app = app();
    let ui = rig(&mut app);
    for (words, reason) in [
        ("orange", HexError::NotHex),
        ("#ff880", HexError::Length),
        ("#12", HexError::Length),
    ] {
        forget(&mut app);
        enter_text(&mut app, ui.tint, words);
        tap(&mut app, KeyCode::Enter);
        assert_eq!(colour(&app, ui.tint), hex(START), "{words}");
        assert!(changes(&app).is_empty(), "{words}: {:?}", changes(&app));
        assert_eq!(refusals(&app), vec![(words.to_string(), reason)]);
        assert_eq!(text(&app, ui.tint), "#336699", "{words}");
        assert_eq!(typed(&app, ui.tint), "#336699", "{words}");
    }
    forget(&mut app);
    enter_text(&mut app, ui.tint, "");
    tap(&mut app, KeyCode::Enter);
    assert_eq!(refusals(&app), vec![(String::new(), HexError::Empty)]);
}

#[test]
fn escape_puts_the_colour_back_and_sends_nothing() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    enter_text(&mut app, ui.tint, "#ff8800");
    tap(&mut app, KeyCode::Escape);
    assert_eq!(text(&app, ui.tint), "#336699");
    assert_eq!(colour(&app, ui.tint), hex(START));
    assert!(changes(&app).is_empty() && refusals(&app).is_empty());
}

#[test]
fn leaving_the_field_after_an_edit_applies_it_once() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    enter_text(&mut app, ui.tint, "#00ff00");
    let other = field(&app, ui.other);
    focus(&mut app, other);
    assert_eq!(colour(&app, ui.tint), hex("#00ff00"));
    assert_eq!(changes(&app), vec![(hex("#00ff00"), true)]);
}

#[test]
fn a_colour_the_game_sets_is_shown_at_once_with_no_event() {
    let mut app = app();
    let ui = rig(&mut app);
    forget(&mut app);
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(ColorValue(hex("#ff00ff")));
    settle(&mut app);
    assert_eq!(text(&app, ui.tint), "#FF00FF");
    assert_eq!(typed(&app, ui.tint), "#FF00FF");
    assert_eq!(half(&app, ui.tint, false), Color::Srgba(hex("#ff00ff")));
    assert!(changes(&app).is_empty());
}

#[test]
fn thirty_idle_frames_write_nothing_with_the_picker_closed_or_open() {
    let mut app = app();
    let ui = rig(&mut app);
    for _ in 0..30 {
        app.update();
    }
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, before, "closed");
    open_picker(&mut app, ui.tint);
    settle(&mut app);
    let before = app.world().resource::<Writes>().0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, before, "open");
}

// --- the picker -------------------------------------------------------------------------------

#[test]
fn the_swatch_opens_the_picker_and_a_second_press_or_escape_closes_it() {
    let mut app = app();
    let ui = rig(&mut app);
    assert!(picker(&mut app).is_none());
    let opened = open_picker(&mut app, ui.tint);
    assert_eq!(
        app.world().get::<ColorPicker>(opened).map(|p| p.field()),
        Some(ui.tint)
    );
    // Beside the swatch, below it, and whole inside the window.
    let card = rect(&app, opened);
    let swatch_rect = rect(&app, swatch(&app, ui.tint));
    assert!(
        card.min.y >= swatch_rect.max.y - 1.0,
        "{card:?} below {swatch_rect:?}"
    );
    assert!(
        card.min.x >= 0.0 && card.max.x <= 1280.0 && card.max.y <= 720.0,
        "{card:?}"
    );
    // Focus is in the picker.
    let held = focused(&app).expect("focus");
    assert!(
        descendants(app.world(), opened).contains(&held),
        "focus inside the picker"
    );
    // The swatch again closes it.
    let at = rect(&app, swatch(&app, ui.tint)).center();
    click_at(&mut app, at);
    assert!(picker(&mut app).is_none(), "a second press closes it");
    // And Escape does, with focus back on the swatch.
    let s = swatch(&app, ui.tint);
    focus(&mut app, s);
    tap(&mut app, KeyCode::Enter);
    assert!(
        picker(&mut app).is_some(),
        "Enter on the focused swatch opens it"
    );
    tap(&mut app, KeyCode::Escape);
    assert!(picker(&mut app).is_none(), "Escape closes it");
    assert_eq!(focused(&app), Some(s), "focus is back on the swatch");
}

#[test]
fn a_disabled_row_opens_nothing_and_takes_no_text() {
    let mut app = app();
    let ui = rig(&mut app);
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(InteractionDisabled);
    settle(&mut app);
    let at = rect(&app, swatch(&app, ui.tint)).center();
    click_at(&mut app, at);
    assert!(picker(&mut app).is_none());
}

#[test]
fn a_press_in_the_areas_corners_is_saturation_and_value_at_their_ends_and_the_ring_is_under_it() {
    let mut app = app();
    let ui = rig(&mut app);
    open_picker(&mut app, ui.tint);
    forget(&mut app);
    let a = area(&mut app);
    let r = rect(&app, a);
    // Past the top right: full saturation, full value. (The corner pixels are outside the rounded
    // frame, so the press lands in the middle and the drag takes it out.)
    let mid = r.center();
    let tr = Vec2::new(r.max.x + 40.0, r.min.y - 40.0);
    grab_and_move(&mut app, mid, tr);
    let [_, s, v] = hsv(&mut app);
    assert_eq!((s, v), (1.0, 1.0));
    let (marker, inside) = marker_and_inside(&mut app);
    assert!(
        (marker - Vec2::new(inside.max.x, inside.min.y))
            .abs()
            .max_element()
            < 1.5,
        "{marker:?} in {inside:?}"
    );
    // A press is not the end: no final yet.
    assert!(changes(&app).iter().all(|(_, f)| !f), "{:?}", changes(&app));
    release(&mut app, tr);
    let seen = changes(&app);
    assert_eq!(seen.iter().filter(|(_, f)| *f).count(), 1, "{seen:?}");
    assert!(
        near(
            seen.last().expect("a change").0,
            Srgba::new(0.0, 0.5, 1.0, 1.0)
        ),
        "{seen:?}"
    );
    // Bottom left: black.
    let bl = Vec2::new(r.min.x - 40.0, r.max.y + 40.0);
    grab_and_move(&mut app, mid, bl);
    release(&mut app, bl);
    let [_, s, v] = hsv(&mut app);
    assert_eq!((s, v), (0.0, 0.0));
    assert_eq!(colour(&app, ui.tint), hex("#000000"));
}

#[test]
fn the_pointer_in_the_middle_of_the_area_is_half_and_half_and_a_drag_holds_at_the_edge() {
    let mut app = app();
    let ui = rig(&mut app);
    open_picker(&mut app, ui.tint);
    let a = area(&mut app);
    let (_, inside) = marker_and_inside(&mut app);
    let mid = inside.center();
    grab_and_move(&mut app, mid, mid);
    let [_, s, v] = hsv(&mut app);
    assert!((s - 0.5).abs() < 0.02 && (v - 0.5).abs() < 0.02, "{s} {v}");
    // Out past the right and the top: held at the edge.
    let far = Vec2::new(inside.max.x + 400.0, inside.min.y - 200.0);
    pointer(&mut app, far, PointerAction::Move { delta: far - mid });
    settle(&mut app);
    let [_, s, v] = hsv(&mut app);
    assert_eq!((s, v), (1.0, 1.0));
    forget(&mut app);
    release(&mut app, far);
    let seen = changes(&app);
    assert_eq!(
        seen.iter().filter(|(_, f)| *f).count(),
        1,
        "one final: {seen:?}"
    );
    assert!(app.world().get::<Role>(a).is_some());
}

#[test]
fn dragging_to_black_and_back_keeps_the_hue() {
    let mut app = app();
    let ui = rig(&mut app);
    open_picker(&mut app, ui.tint);
    let [h0, ..] = hsv(&mut app);
    let (_, inside) = marker_and_inside(&mut app);
    let right = Vec2::new(inside.max.x - 1.0, inside.center().y);
    let black = Vec2::new(inside.center().x, inside.max.y + 5.0);
    grab_and_move(&mut app, right, black);
    assert_eq!(
        colour(&app, ui.tint).red + colour(&app, ui.tint).green + colour(&app, ui.tint).blue,
        0.0
    );
    let back = Vec2::new(inside.max.x - 1.0, inside.min.y + 1.0);
    pointer(
        &mut app,
        back,
        PointerAction::Move {
            delta: back - black,
        },
    );
    release(&mut app, back);
    let [h1, ..] = hsv(&mut app);
    assert!((h1 - h0).abs() < 1e-3, "{h0} came back {h1}");
}

#[test]
fn a_grey_typed_as_hex_keeps_the_hue_the_picker_was_on() {
    let mut app = app();
    let ui = rig(&mut app);
    let opened = open_picker(&mut app, ui.tint);
    let [h0, ..] = hsv(&mut app);
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(ColorValue(hex("#808080")));
    settle(&mut app);
    let [h1, s, v] = hsv(&mut app);
    assert!((h1 - h0).abs() < 1e-3, "{h0} -> {h1}");
    assert_eq!(s, 0.0);
    assert!((v - 128.0 / 255.0).abs() < 1e-3);
    let _ = opened;
}

#[test]
fn the_pickers_marker_hue_and_strips_follow_a_colour_the_game_sets_while_it_is_open() {
    let mut app = app();
    let ui = rig(&mut app);
    open_picker(&mut app, ui.tint);
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(ColorValue(hex("#00ff00").with_alpha(0.25)));
    settle(&mut app);
    let [h, s, v] = hsv(&mut app);
    assert!(
        (h - 120.0).abs() < 1e-2 && s == 1.0 && v == 1.0,
        "{h} {s} {v}"
    );
    let found = strips(&mut app);
    assert_eq!(found.len(), 3);
    assert!((slider_value(&app, found[0]) - 120.0).abs() < 1e-2);
    assert!((slider_value(&app, found[1]) - 1.0).abs() < 1e-6);
    assert!((slider_value(&app, found[2]) - 0.25).abs() < 1e-6);
    let a = area(&mut app);
    assert_eq!(
        app.world().get::<BackgroundColor>(a).map(|b| b.0),
        Some(Color::srgb(0.0, 1.0, 0.0)),
        "the area is the pure hue"
    );
    let (marker, inside) = marker_and_inside(&mut app);
    assert!(
        (marker - Vec2::new(inside.max.x, inside.min.y))
            .abs()
            .max_element()
            < 1.5
    );
}

#[test]
fn each_strip_changes_its_own_channel_and_no_other() {
    let mut app = app();
    let ui = rig(&mut app);
    // Start on a mid colour so no channel is at an end.
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(ColorValue(Srgba::new(0.8, 0.4, 0.2, 0.5)));
    settle(&mut app);
    open_picker(&mut app, ui.tint);
    let [h0, s0, v0] = hsv(&mut app);
    let [hue, bright, alpha] = strips(&mut app)[..] else {
        panic!("three strips")
    };
    focus(&mut app, hue);
    tap(&mut app, KeyCode::ArrowRight);
    let [h1, s1, v1] = hsv(&mut app);
    assert!(h1 > h0 && s1 == s0 && v1 == v0, "hue only: {h0}->{h1}");
    assert_eq!(colour(&app, ui.tint).alpha, 0.5);
    focus(&mut app, bright);
    tap(&mut app, KeyCode::ArrowLeft);
    let [h2, s2, v2] = hsv(&mut app);
    assert!(v2 < v1 && h2 == h1 && s2 == s1, "brightness only");
    focus(&mut app, alpha);
    forget(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    let c = colour(&app, ui.tint);
    assert!(c.alpha > 0.5, "{c:?}");
    let [h3, s3, v3] = hsv(&mut app);
    assert_eq!(
        (h3, s3, v3),
        (h2, s2, v2),
        "alpha leaves the hue, saturation and value"
    );
    assert_eq!(changes(&app).len(), 1, "{:?}", changes(&app));
    // The hue strip ends where it began: red at both ends.
    focus(&mut app, hue);
    tap(&mut app, KeyCode::End);
    let end = colour(&app, ui.tint);
    assert!(end.red > end.green && end.red > end.blue, "{end:?}");
}

#[test]
fn the_arrows_on_the_area_step_a_hundredth_shift_a_tenth_and_move_no_focus() {
    let mut app = app();
    let ui = rig(&mut app);
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(ColorValue(Srgba::new(0.8, 0.4, 0.2, 1.0)));
    settle(&mut app);
    open_picker(&mut app, ui.tint);
    let a = area(&mut app);
    focus(&mut app, a);
    let [_, s0, v0] = hsv(&mut app);
    forget(&mut app);
    tap(&mut app, KeyCode::ArrowRight);
    let [_, s1, v1] = hsv(&mut app);
    assert!((s1 - s0 - 0.01).abs() < 1e-5 && v1 == v0, "{s0} {s1}");
    tap(&mut app, KeyCode::ArrowDown);
    let [_, s2, v2] = hsv(&mut app);
    assert!((v1 - v2 - 0.01).abs() < 1e-5 && s2 == s1, "{v1} {v2}");
    hold(&mut app, KeyCode::ShiftLeft);
    tap(&mut app, KeyCode::ArrowUp);
    let_go(&mut app, KeyCode::ShiftLeft);
    let [_, _, v3] = hsv(&mut app);
    assert!((v3 - v2 - 0.1).abs() < 1e-4, "{v2} {v3}");
    assert_eq!(focused(&app), Some(a), "no arrow moved focus");
    assert!(
        changes(&app).iter().all(|(_, f)| *f),
        "each key is one final change"
    );
    assert_eq!(changes(&app).len(), 3);
}

#[test]
fn the_d_pad_steps_the_areas_saturation_and_its_up_and_down_move_focus_on() {
    let mut app = app();
    let ui = rig(&mut app);
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(ColorValue(Srgba::new(0.8, 0.4, 0.2, 1.0)));
    settle(&mut app);
    open_picker(&mut app, ui.tint);
    let pad = plug_in_pad(&mut app);
    let a = area(&mut app);
    focus(&mut app, a);
    let [_, s0, v0] = hsv(&mut app);
    press(&mut app, pad, GamepadButton::DPadRight);
    let [_, s1, v1] = hsv(&mut app);
    assert!((s1 - s0 - 0.05).abs() < 1e-5 && v1 == v0, "{s0} {s1}");
    press(&mut app, pad, GamepadButton::DPadLeft);
    press(&mut app, pad, GamepadButton::DPadLeft);
    let [_, s2, _] = hsv(&mut app);
    assert!((s0 - s2 - 0.05).abs() < 1e-5, "{s0} {s2}");
    assert_eq!(focused(&app), Some(a));
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_ne!(focused(&app), Some(a), "Down leaves the area");
}

#[test]
fn tab_walks_the_area_and_the_strips_and_never_leaves_the_picker() {
    let mut app = app();
    let ui = rig(&mut app);
    let opened = open_picker(&mut app, ui.tint);
    let a = area(&mut app);
    focus(&mut app, a);
    let mut order = vec![a];
    for _ in 0..4 {
        tap_named(&mut app, KeyCode::Tab, Key::Tab);
        order.push(focused(&app).expect("focus"));
    }
    let found = strips(&mut app);
    assert_eq!(&order[1..4], &found[..], "area, hue, brightness, alpha");
    assert_eq!(order[4], a, "wraps inside the picker, never out of it");
    assert!(descendants(app.world(), opened).contains(&order[4]));
}

// --- a mixed colour ---------------------------------------------------------------------------

#[test]
fn a_mixed_colour_is_an_empty_field_with_the_hint_and_a_clear_swatch_until_the_player_picks() {
    let mut app = app();
    let ui = rig(&mut app);
    app.world_mut().entity_mut(ui.tint).insert(MixedValue);
    settle(&mut app);
    assert_eq!(text(&app, ui.tint), "");
    assert_eq!(half(&app, ui.tint, false), Color::NONE);
    assert_eq!(half(&app, ui.tint, true), Color::NONE);
    let hint = part(&app, ui.tint, Role::TextHint);
    assert_eq!(
        app.world().get::<Text>(hint).map(|t| t.0.as_str()),
        Some(MIXED_HINT)
    );
    // Enter on the untouched field changes nothing.
    forget(&mut app);
    focus_field(&mut app, ui.tint);
    tap(&mut app, KeyCode::Enter);
    assert!(changes(&app).is_empty(), "{:?}", changes(&app));
    // A pick is one value for everyone, and the mark is gone.
    open_picker(&mut app, ui.tint);
    let a = area(&mut app);
    focus(&mut app, a);
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(changes(&app).len(), 1);
    assert!(app.world().get::<MixedValue>(ui.tint).is_none());
    assert_ne!(text(&app, ui.tint), "");
}

// --- the look ---------------------------------------------------------------------------------

#[test]
fn the_swatch_is_a_target_high_the_row_is_the_text_rows_height_and_nothing_moves_when_it_changes() {
    let mut app = app();
    let ui = rig(&mut app);
    let s = rect(&app, swatch(&app, ui.tint));
    assert!(
        s.height() >= 44.0 - 1.0 && (s.width() - s.height()).abs() < 1.0,
        "{s:?}"
    );
    // The row is as high as a number field's: the swatch added no height.
    let parent = app.world().get::<ChildOf>(ui.tint).expect("in a panel").0;
    let number = app
        .world_mut()
        .spawn(number_field("Health", 1.0, 1.0, 0.0..=9.0))
        .id();
    app.world_mut().entity_mut(number).insert(ChildOf(parent));
    settle(&mut app);
    let row = rect(&app, ui.tint);
    assert!(
        (row.height() - rect(&app, number).height()).abs() < 1.0 && row.height() >= s.height(),
        "{row:?} against {:?}",
        rect(&app, number)
    );
    // The swatch is at the right, with the field after it; the label is on the left.
    let f = rect(&app, field(&app, ui.tint));
    let label = rect(&app, label_of(&app, ui.tint));
    assert!(
        label.max.x <= s.min.x && s.max.x <= f.min.x,
        "{label:?} {s:?} {f:?}"
    );
    let before = (row, s, f);
    app.world_mut()
        .entity_mut(ui.tint)
        .insert(ColorValue(hex("#ff8800").with_alpha(0.3)));
    settle(&mut app);
    let after = (
        rect(&app, ui.tint),
        rect(&app, swatch(&app, ui.tint)),
        rect(&app, field(&app, ui.tint)),
    );
    assert_eq!(before.0.size(), after.0.size());
    assert_eq!(before.1, after.1);
}

#[test]
fn the_pickers_ring_has_two_tones_that_differ_and_the_area_and_swatch_are_framed_at_3_to_1() {
    let mut app = app();
    let ui = rig(&mut app);
    open_picker(&mut app, ui.tint);
    let a = area(&mut app);
    let outer = part(&app, a, Role::ColorAreaMarker);
    let inner = part(&app, a, Role::ColorAreaMarkerInner);
    let tone = |entity| match app.world().get::<BorderColor>(entity).expect("a ring").top {
        Color::Srgba(c) => [c.red, c.green, c.blue],
        other => other.to_srgba().to_f32_array()[..3]
            .try_into()
            .expect("rgb"),
    };
    let (o, i) = (tone(outer), tone(inner));
    assert!(contrast_ratio(o, i) >= 3.0, "{o:?} on {i:?}");
    // The frames are the skin's outline on the panel, in every look.
    let skin = app.world().resource::<GooeySkin>().clone();
    let rgb = |c: Color| {
        let c = c.to_srgba();
        [c.red, c.green, c.blue]
    };
    for frame in [swatch(&app, ui.tint), a] {
        let border = app.world().get::<BorderColor>(frame).expect("a frame").top;
        assert!(
            contrast_ratio(rgb(border), rgb(skin.solid_surface())) >= NON_TEXT_CONTRAST,
            "{border:?}"
        );
    }
}

#[test]
fn the_focus_ring_is_on_the_swatch_and_the_area_when_they_have_focus() {
    let mut app = app();
    let ui = rig(&mut app);
    let s = swatch(&app, ui.tint);
    let skin = app.world().resource::<GooeySkin>().clone();
    focus(&mut app, s);
    assert_eq!(
        app.world().get::<BorderColor>(s).expect("a frame").top,
        skin.focus_ring
    );
    open_picker(&mut app, ui.tint);
    let a = area(&mut app);
    focus(&mut app, a);
    assert_eq!(
        app.world().get::<BorderColor>(a).expect("a frame").top,
        skin.focus_ring
    );
    let other = app.world().get::<BorderColor>(s).expect("a frame").top;
    assert_ne!(other, skin.focus_ring, "the swatch's ring goes with focus");
    let _ = ui.other;
    let _ = ui.button;
}

#[test]
fn the_picker_stays_inside_the_window_at_every_text_size_that_fits_and_in_every_look() {
    for preset in SkinPreset::ALL {
        for scale in [1.0, 1.5] {
            let mut app = app();
            let mut skin = preset.skin();
            skin.text_scale = scale;
            app.insert_resource(skin);
            let ui = rig(&mut app);
            let opened = open_picker(&mut app, ui.tint);
            settle(&mut app);
            let card = rect(&app, opened);
            assert!(
                card.min.x >= -0.5
                    && card.max.x <= 1280.5
                    && card.min.y >= -0.5
                    && card.max.y <= 720.5,
                "{} at {scale}: {card:?}",
                preset.name()
            );
            // Every part is inside the card (the ring on the area is the one thing that overhangs).
            for part in descendants(app.world(), opened) {
                if app.world().get::<Role>(part).is_some_and(|r| {
                    matches!(r, Role::ColorAreaMarker | Role::ColorAreaMarkerInner)
                }) || app.world().get::<ComputedNode>(part).is_none()
                {
                    continue;
                }
                let r = rect(&app, part);
                assert!(
                    r.min.x >= card.min.x - 1.0
                        && r.max.x <= card.max.x + 1.0
                        && r.max.y <= card.max.y + 1.0,
                    "{} at {scale}: {part:?} {r:?} outside {card:?}",
                    preset.name()
                );
            }
        }
    }
}
