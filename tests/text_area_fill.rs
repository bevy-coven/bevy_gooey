//! A text area sized by its host: it fills the room its parent gives it, scrolls with the wheel and
//! its own rail, keeps the caret in view while typing, wraps a very long line, and reports the whole
//! text through `ValueChange<String>`. Laid out for real in a headless window, driven by real
//! keyboard and pointer input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::text::{EditableText, TextLayoutInfo};
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// The issue's file size, in one text.
const FILE_CHARS: usize = 64 * 1024;

fn fresh_app() -> App {
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
    for _ in 0..6 {
        app.update();
    }
}

fn descendants(app: &App, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if let Some(children) = app.world().get::<Children>(entity) {
            for child in children.iter() {
                found.push(child);
                stack.push(child);
            }
        }
    }
    found
}

fn part(app: &App, root: Entity, role: Role) -> Entity {
    descendants(app, root)
        .into_iter()
        .find(|&e| app.world().get::<Role>(e) == Some(&role))
        .unwrap_or_else(|| panic!("a {role:?} under the widget"))
}

fn field(app: &App, row: Entity) -> Entity {
    part(app, row, Role::TextField)
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

fn tap_named(app: &mut App, key_code: KeyCode, logical: Key) {
    let window = window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code,
            logical_key: logical.clone(),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
    }
    app.update();
}

/// The platform's command key held while `key_code` is tapped, then let go: Ctrl off macOS and Cmd
/// on it, which is Bevy's own split, so `chord(End)` is the document's end on both.
fn chord(app: &mut App, key_code: KeyCode, logical: Key) {
    let window = window(app);
    let (command_code, command_key) = if cfg!(target_os = "macos") {
        (KeyCode::SuperLeft, Key::Super)
    } else {
        (KeyCode::ControlLeft, Key::Control)
    };
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: command_code,
            logical_key: command_key.clone(),
            state,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        if state == ButtonState::Pressed {
            tap_named(app, key_code, logical.clone());
        }
    }
    settle(app);
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
    app.update();
}

fn focus_on(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
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
    let position = location(app, at);
    app.world_mut().write_message(PointerInput {
        pointer_id: PointerId::Mouse,
        location: position,
        action,
    });
    app.update();
}

/// One notch of the wheel over `at`, `lines` lines down: the platform's own message, which is what
/// a real mouse sends.
fn wheel(app: &mut App, at: Vec2, lines: f32) {
    point_at(app, at);
    let window = window(app);
    app.world_mut().write_message(MouseWheel {
        unit: MouseScrollUnit::Line,
        x: 0.0,
        y: -lines,
        window,
        phase: TouchPhase::Moved,
    });
    settle(app);
}

fn point_at(app: &mut App, at: Vec2) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
}

/// A node the crate wrote this frame: a rail's lane or the thumb in it.
type LaneWritten = Or<(Changed<Node>, Changed<Visibility>)>;

/// How many frames wrote something, over the `Writes` test.
#[derive(Resource, Default)]
pub struct Writes {
    lanes: u32,
    offset: u32,
}

/// A note of about `FILE_CHARS` characters, one short numbered line each, ending in a named line.
/// It stays **under** 65,536 bytes: Bevy's text layout (parley) loses the caret's place in a text
/// past that, so a caret at its end reads as `len - 65536`, and 64 KiB is the issue's own size.
fn note() -> String {
    const LAST: &str = "the very last line";
    let mut text = String::with_capacity(FILE_CHARS);
    let mut line = 0;
    while text.len() + 12 + LAST.len() < FILE_CHARS {
        text.push_str(&format!("line {line}\n"));
        line += 1;
    }
    text.push_str(LAST);
    text
}

struct Bench {
    row: Entity,
    field: Entity,
}

/// A window-sized panel holding a `text_area` that fills what the panel leaves.
fn new_bench(app: &mut App, rows: usize, text: String) -> Bench {
    let world = app.world_mut();
    let row = world.spawn(text_area_fill("Notes", text, rows)).id();
    // A column filling the window: the row grows to what the column leaves, the field to what the
    // row leaves under its label.
    let card = world
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            Role::Panel,
        ))
        .add_child(row)
        .id();
    world.spawn(screen()).add_child(card);
    settle(app);
    let field = field(app, row);
    Bench { row, field }
}

fn text_of(app: &App, field: Entity) -> String {
    app.world()
        .get::<EditableText>(field)
        .expect("a field")
        .editor()
        .raw_text()
        .to_string()
}

fn viewport(app: &App, field: Entity) -> bevy::text::TextViewport {
    app.world()
        .get::<EditableText>(field)
        .expect("a field")
        .viewport
}

/// The laid-out height of the whole text, in layout px.
fn text_height(app: &App, field: Entity) -> f32 {
    app.world()
        .get::<TextLayoutInfo>(field)
        .expect("a laid-out field")
        .size
        .y
}

/// The acceptance: 64 KiB in a text area 40 rows tall in a window-sized host, the wheel to the end,
/// and the last line is in the field because the viewport has reached the bottom of the text.
#[test]
fn sixty_four_kibibytes_wheel_to_the_end_and_the_last_line_is_inside_the_field() {
    /// Every `ValueChange<String>` the document sent, with whether it ended the edit.
    #[derive(Resource, Default)]
    struct Heard(Vec<(String, bool)>);

    let mut app = fresh_app();
    app.init_resource::<Heard>();
    app.add_observer(
        |change: On<ValueChange<String>>, mut heard: ResMut<Heard>| {
            heard.0.push((change.value.clone(), change.is_final));
        },
    );
    let bench = new_bench(&mut app, 40, note());
    let field = bench.field;

    let text = text_of(&app, field);
    assert!(
        text.chars().count() > FILE_CHARS - 32,
        "the whole 64 KiB went in: {} chars",
        text.chars().count()
    );
    assert!(text.ends_with("the very last line"), "and it ends there");
    assert_eq!(
        viewport(&app, field).offset.y,
        0.0,
        "a fresh area rests at its top"
    );

    // Focus before the scroll: Bevy reveals the caret on the frame focus arrives, so a field focused
    // after the wheel would be scrolled back to its caret first. Then the caret goes to the top
    // (Ctrl+Home), so the wheel and the caret are two different places, as they are for a reader.
    focus_on(&mut app, field);
    chord(&mut app, KeyCode::Home, Key::Home);
    assert_eq!(
        viewport(&app, field).offset.y,
        0.0,
        "Ctrl+Home put the caret and the view at the top"
    );
    // Wheeling to the end: a hundred lines a turn (the same input as a mouse spun hard, not 400
    // frames of layout).
    let middle = rect(&app, field).center();
    for _ in 0..60 {
        wheel(&mut app, middle, 100.0);
    }

    let view = viewport(&app, field);
    let height = text_height(&app, field);
    // The furthest the field may scroll, from this frame's own measurements: the text's height less
    // the part of it the field draws (Bevy's own viewport size, not the field's border box).
    let farthest = (height - view.size.y).max(0.0);
    assert!(view.offset.y > 0.0, "the wheel scrolled it");
    assert!(
        view.size.y < height,
        "the field shows less than the text: {} of {height}",
        view.size.y
    );
    assert!(
        (view.offset.y - farthest).abs() < 2.0,
        "the viewport stands at the text's end: offset {} against {farthest} ({height} tall, {room} drawn)",
        view.offset.y,
        room = view.size.y
    );

    // The issue's second half: **types at the end and the value-change event carries the whole
    // text.** Ctrl+End is the keyboard's way to the end of a document (on Linux Ctrl is Bevy's
    // command key; on macOS Cmd+End is the same edit), and it is the one the acceptance names.
    chord(&mut app, KeyCode::End, Key::End);
    let caret_at_end = app
        .world()
        .get::<TextLayoutInfo>(field)
        .expect("a layout")
        .cursor
        .expect("a caret")
        .1;
    let height = text_height(&app, field);
    assert!(
        caret_at_end.center().y > height - caret_at_end.height() * 2.0,
        "Ctrl+End put the caret in the document's last line: {caret_at_end:?} of {height}"
    );

    // One character typed there lands at the end of the document, and the change carries all of it.
    type_char(&mut app, '!');
    settle(&mut app);
    let text = text_of(&app, field);
    assert!(
        text.ends_with('!') && text.contains("the very last line"),
        "the character went in at the end: {:?}",
        &text[text.len() - 40..]
    );
    let view = viewport(&app, field);
    let caret = app
        .world()
        .get::<TextLayoutInfo>(field)
        .expect("a layout")
        .cursor
        .expect("a caret")
        .1;
    let farthest = (text_height(&app, field) - view.size.y).max(0.0);
    assert!(
        caret.center().y >= view.offset.y - 1.0
            && caret.center().y <= view.offset.y + view.size.y + 1.0,
        "the caret is inside the viewport: {caret:?} of offset {} size {}",
        view.offset.y,
        view.size.y
    );
    assert!(
        view.offset.y > farthest - view.size.y * 0.2,
        "and the view stands at the text's end: {} of {farthest}",
        view.offset.y
    );
    let heard = app.world().resource::<Heard>();
    assert_eq!(
        heard.0.last().map(|(t, _)| t.as_str()),
        Some(text.as_str()),
        "the last change carries the whole 64 KiB document"
    );
    assert!(text.len() > FILE_CHARS - 32, "all of it, still");
}

/// A wheel turn over a filled area scrolls it, and over a short one (nothing below the field) moves
/// nothing.
#[test]
fn the_wheel_scrolls_a_filled_area_and_not_a_short_one() {
    let mut app = fresh_app();
    let short = new_bench(&mut app, 3, "one\ntwo\nthree".to_owned());
    let middle = rect(&app, short.field).center();
    wheel(&mut app, middle, 8.0);
    assert_eq!(
        viewport(&app, short.field).offset.y,
        0.0,
        "nothing to scroll"
    );

    let long = new_bench(
        &mut app,
        10,
        (0..400).map(|i| format!("line {i}\n")).collect(),
    );
    let middle = rect(&app, long.field).center();
    wheel(&mut app, middle, 3.0);
    assert!(
        viewport(&app, long.field).offset.y > 0.0,
        "three lines moved it"
    );
}

/// An area fills the room its host gives it, past the rows it was spawned with.
#[test]
fn a_filled_area_is_as_tall_as_its_host() {
    let mut app = fresh_app();
    let bench = new_bench(&mut app, 5, "short".to_owned());
    let card = app
        .world()
        .get::<ChildOf>(bench.row)
        .expect("a parent")
        .parent();
    // The card's padding box: its own frame and padding come off its height.
    let node = app.world().get::<ComputedNode>(card).expect("laid out");
    let inside = node.size().y - 2.0 * node.border().min_inset.y - 2.0 * node.padding().min_inset.y;
    let skin = GooeySkin::dark();
    let field = rect(&app, bench.field);
    assert!(
        field.height() > skin.target() * 5.0,
        "the field grew past its five rows: {field:?} in {inside}"
    );
    assert!(
        (rect(&app, bench.row).height() - inside).abs() < 2.0,
        "the row fills the card's room: {:?} of {inside}",
        rect(&app, bench.row)
    );
    // The label stands over the field inside the same room: the field is the host less the label's
    // line and the half-gap between them, within the line's rounding.
    let label = part(&app, bench.row, Role::ControlText);
    let label_height = rect(&app, label).height();
    assert!(
        (field.height() + label_height - inside).abs() <= skin.gap * 0.5 + skin.text_size,
        "the field is the room the row's label leaves: {field:?} and {label_height} in {inside}"
    );
}

/// Typing keeps the caret in view: the wheel may leave the view anywhere, and the moment the player
/// types (or steps) the caret moves, the view comes back to it. And an edit reports the whole text;
/// Ctrl+Enter reports it once, final.
#[test]
fn typing_keeps_the_caret_in_view_and_sends_the_whole_text() {
    let mut app = fresh_app();
    #[derive(Resource, Default)]
    struct Heard(Vec<(String, bool)>);
    app.init_resource::<Heard>();
    app.add_observer(
        |change: On<ValueChange<String>>, mut heard: ResMut<Heard>| {
            heard.0.push((change.value.clone(), change.is_final));
        },
    );
    let bench = new_bench(&mut app, 20, note());
    let field = bench.field;
    focus_on(&mut app, field);
    let caret_at = |app: &App| {
        app.world()
            .get::<TextLayoutInfo>(field)
            .expect("a layout")
            .cursor
            .expect("a caret")
            .1
    };
    assert!(
        caret_at(&app).min.y < 100.0,
        "a fresh area's caret is at the document's start: {:?}",
        caret_at(&app)
    );

    // Wheel to the document's end: the view leaves the caret far behind.
    let middle = rect(&app, field).center();
    for _ in 0..60 {
        wheel(&mut app, middle, 100.0);
    }
    let away = viewport(&app, field);
    assert!(
        away.offset.y > 1_000.0,
        "the wheel took the view down: {}",
        away.offset.y
    );

    // One keystroke: the caret moves, and the view comes back to it.
    type_char(&mut app, '!');
    settle(&mut app);
    let after = viewport(&app, field);
    let caret = caret_at(&app);
    assert!(
        (after.offset.y - away.offset.y).abs() > 1.0,
        "the view followed the caret: {} then {}",
        away.offset.y,
        after.offset.y
    );
    assert!(
        caret.center().y >= after.offset.y - 1.0
            && caret.center().y <= after.offset.y + after.size.y + 1.0,
        "the caret is inside the viewport: {caret:?} of offset {} size {}",
        after.offset.y,
        after.size.y
    );

    let text = text_of(&app, field);
    assert_eq!(
        app.world()
            .resource::<Heard>()
            .0
            .last()
            .map(|(t, _)| t.as_str()),
        Some(text.as_str()),
        "one edit reports the whole text"
    );
    assert!(text.len() > FILE_CHARS - 32, "all 64 KiB of it");

    // Ctrl+Enter ends the edit: one final change, carrying all of it.
    let first = window(&mut app);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::ControlLeft,
        logical_key: Key::Control,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: first,
    });
    app.update();
    tap_named(&mut app, KeyCode::Enter, Key::Enter);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::ControlLeft,
        logical_key: Key::Control,
        state: ButtonState::Released,
        text: None,
        repeat: false,
        window: first,
    });
    settle(&mut app);
    let heard = app.world().resource::<Heard>();
    let (value, _) = heard
        .0
        .iter()
        .rev()
        .find(|(_, final_)| *final_)
        .expect("a final change");
    assert_eq!(value, &text_of(&app, field), "the whole text, unchanged");
}

/// The rail of a filled area: it shows while the text is longer than the field, its thumb is the
/// share of the text the field shows, and a press on the rail jumps the viewport there.
#[test]
fn the_rail_shows_while_the_text_overflows_and_a_press_on_it_jumps_the_view() {
    let mut app = fresh_app();
    let bench = new_bench(&mut app, 8, note());
    let host = app
        .world()
        .get::<ChildOf>(bench.row)
        .expect("a parent")
        .parent();
    let rail = part(&app, host, Role::ListRail);
    let node = app.world().get::<Node>(rail).expect("a rail node");
    assert_eq!(
        node.display,
        Display::Flex,
        "there is more than the field shows"
    );

    let thumb = part(&app, rail, Role::ListThumb);
    let height = text_height(&app, bench.field);
    let view = viewport(&app, bench.field);
    let share = view.size.y / height;
    let rail_rect = rect(&app, rail);
    let thumb_rect = rect(&app, thumb);
    // The thumb is the share of the text the field draws, never thinner than a target-mark.
    let expected = (share).clamp(0.06, 1.0);
    assert!(
        (thumb_rect.height() / rail_rect.height() - expected).abs() < 0.02,
        "the thumb is the share shown: {} of {} against {expected}",
        thumb_rect.height(),
        rail_rect.height()
    );

    // A press three quarters down the rail jumps the viewport most of the way in.
    let spot = Vec2::new(
        rail_rect.center().x,
        rail_rect.min.y + rail_rect.height() * 0.75,
    );
    pointer(&mut app, spot, PointerAction::Move { delta: Vec2::X });
    pointer(&mut app, spot, PointerAction::Press(PointerButton::Primary));
    pointer(
        &mut app,
        spot,
        PointerAction::Release(PointerButton::Primary),
    );
    settle(&mut app);
    let jumped = viewport(&app, bench.field);
    assert!(
        jumped.offset.y > 0.4 * (height - view.size.y),
        "a press three quarters down went most of the way: {} of {}",
        jumped.offset.y,
        height - view.size.y
    );
    let moved_thumb = rect(&app, thumb);
    assert!(
        moved_thumb.min.y > thumb_rect.min.y + 1.0,
        "and the thumb moved with it: {thumb_rect:?} then {moved_thumb:?}"
    );

    // A field with nothing past it has no rail: a host shorter than the text it holds is where a
    // rail would be, and two lines in a 40-row host are all shown.
    let mut plain = fresh_app();
    let small = new_bench(&mut plain, 40, "one\ntwo".to_owned());
    let small_host = plain
        .world()
        .get::<ChildOf>(small.row)
        .expect("a parent")
        .parent();
    let rail = part(&plain, small_host, Role::ListRail);
    assert_eq!(
        plain.world().get::<Node>(rail).expect("a rail").display,
        Display::None,
        "nothing to scroll, no rail"
    );
}

/// A very long single line wraps at the field's width and scrolls vertically: the field never grows
/// wider than its host, and there is no horizontal scrolling.
#[test]
fn a_very_long_line_wraps_and_scrolls_vertically() {
    let mut app = fresh_app();
    let line: String = "abcdefghij".repeat(1_200);
    let bench = new_bench(&mut app, 10, line);
    let field = bench.field;
    let card = app
        .world()
        .get::<ChildOf>(bench.row)
        .expect("a parent")
        .parent();

    let field_rect = rect(&app, field);
    let card_rect = rect(&app, card);
    assert!(
        field_rect.width() <= card_rect.width(),
        "the field stays inside its host: {field_rect:?} in {card_rect:?}"
    );
    let view = viewport(&app, field);
    assert_eq!(view.offset.x, 0.0, "a long line is wrapped, not panned");
    assert!(
        text_height(&app, field) > view.size.y,
        "and it is taller than the field"
    );

    let middle = field_rect.center();
    wheel(&mut app, middle, 2.0);
    let after = viewport(&app, field);
    assert!(after.offset.y > 0.0, "the wheel scrolls it");
    assert_eq!(after.offset.x, 0.0);
}

/// A window whose shorter side is 1080 px: the interface is drawn at 1.5 (the default reference
/// side is 720), so every node is 1.5 device pixels per skin pixel and a pointer's window pixels
/// are not the field's layout pixels.
fn tall_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
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

/// The scroll of a filled area is measured in the same pixels as its text and its viewport,
/// whatever the interface's scale: a press a quarter down the rail lands a quarter into the text,
/// and the wheel stops at the text's last line rather than past it.
#[test]
fn a_scaled_interface_scrolls_its_text_by_its_own_pixels() {
    let mut app = tall_app();
    let bench = new_bench(&mut app, 8, note());
    let ui_scale = app.world().resource::<UiScale>().0;
    assert!(
        (ui_scale - 1.5).abs() < 0.01,
        "1080p over a 720 reference draws the interface at 1.5: {ui_scale}"
    );
    let host = app
        .world()
        .get::<ChildOf>(bench.row)
        .expect("a parent")
        .parent();
    let rail = part(&app, host, Role::ListRail);
    // A node's rect is in physical px and a pointer's position in the window's logical px, which the
    // window's own scale factor is the one between (`UiScale` is already in both).
    let primary = window(&mut app);
    let window_scale = app
        .world()
        .get::<Window>(primary)
        .expect("a window")
        .scale_factor();
    let rail_rect = rect(&app, rail);
    let rail_logical = Rect::from_center_size(
        rail_rect.center() / window_scale,
        rail_rect.size() / window_scale,
    );

    let view = viewport(&app, bench.field);
    let farthest = (text_height(&app, bench.field) - view.size.y).max(0.0);
    assert!(
        farthest > 100.0,
        "there is a long way to scroll: {farthest}"
    );

    // A quarter of the way down the rail is a quarter of the way into the text.
    let spot = Vec2::new(
        rail_logical.center().x,
        rail_logical.min.y + rail_logical.height() * 0.25,
    );
    pointer(&mut app, spot, PointerAction::Move { delta: Vec2::X });
    pointer(&mut app, spot, PointerAction::Press(PointerButton::Primary));
    pointer(
        &mut app,
        spot,
        PointerAction::Release(PointerButton::Primary),
    );
    settle(&mut app);
    let quarter = viewport(&app, bench.field).offset.y;
    assert!(
        (quarter / farthest - 0.25).abs() < 0.1,
        "a press a quarter down the rail lands a quarter in: {quarter} of {farthest}"
    );

    // The wheel to the end stops where the text ends: the last line's bottom is the field's.
    let middle = rect(&app, bench.field).center() / window_scale;
    for _ in 0..60 {
        wheel(&mut app, middle, 100.0);
    }
    let view = viewport(&app, bench.field);
    let farthest = (text_height(&app, bench.field) - view.size.y).max(0.0);
    assert!(
        (view.offset.y - farthest).abs() < 2.0,
        "the wheel reaches the text's end: {} of {farthest}",
        view.offset.y
    );
}

/// A settled area costs nothing: thirty frames of a still field write no rail node, and never move the
/// viewport. (Bevy marks `EditableText` changed every caret blink, so that is not what is counted.)
#[test]
fn a_settled_area_writes_nothing() {
    let mut app = fresh_app();
    let bench = new_bench(&mut app, 8, note());
    let host = app
        .world()
        .get::<ChildOf>(bench.row)
        .expect("a parent")
        .parent();
    let rail = part(&app, host, Role::ListRail);
    let thumb = part(&app, rail, Role::ListThumb);
    settle(&mut app);

    app.init_resource::<Writes>();
    app.add_systems(
        Update,
        move |lanes: Query<Entity, LaneWritten>,
              field: Single<&EditableText>,
              mut writes: ResMut<Writes>| {
            writes.lanes += lanes
                .iter()
                .filter(|lane| *lane == rail || *lane == thumb)
                .count() as u32;
            if field.viewport.offset != Vec2::ZERO {
                writes.offset += 1;
            }
        },
    );
    // The counting system's own first run sees every component as changed (its tick starts at
    // zero): one frame primes it, and the count starts from there.
    app.update();
    *app.world_mut().resource_mut::<Writes>() = Writes::default();
    for _ in 0..30 {
        app.update();
    }
    let writes = app.world().resource::<Writes>();
    assert_eq!(writes.lanes, 0, "no rail node was written");
    assert_eq!(writes.offset, 0, "the viewport did not move");
}
