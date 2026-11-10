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

/// A note of about `FILE_CHARS` characters, one short numbered line each, ending in a named line.
fn note() -> String {
    let mut text = String::with_capacity(FILE_CHARS + 32);
    let mut line = 0;
    while text.len() + 12 < FILE_CHARS {
        text.push_str(&format!("line {line}\n"));
        line += 1;
    }
    text.push_str("the very last line");
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
    let mut app = fresh_app();
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

    // Wheeling to the end: a hundred lines a turn (the same input as a mouse spun hard, not 400
    // frames of layout).
    let middle = rect(&app, field).center();
    for _ in 0..60 {
        wheel(&mut app, middle, 100.0);
    }

    let view = viewport(&app, field);
    let height = text_height(&app, field);
    // The furthest the field may scroll, from this frame's own measurements.
    let room = rect(&app, field).height();
    let farthest = (height - room).max(0.0);
    assert!(view.offset.y > 0.0, "the wheel scrolled it");
    assert!(
        view.size.y < height,
        "the field shows less than the text: {} of {height}",
        view.size.y
    );
    assert!(
        (view.offset.y - farthest).abs() < 2.0,
        "the viewport stands at the text's end: offset {} against {farthest} ({height} of {room})",
        view.offset.y
    );
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

/// Typing at the end of 64 KiB keeps the caret in view, and the final change carries the whole text.
#[test]
fn typing_at_the_end_keeps_the_caret_in_view_and_sends_the_whole_text() {
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

    // To the end of the document the way a player gets there: wheel down, then click the line the
    // field ends on.
    let middle = rect(&app, field).center();
    for _ in 0..60 {
        wheel(&mut app, middle, 100.0);
    }
    let field_rect = rect(&app, field);
    let last_line = Vec2::new(field_rect.center().x, field_rect.max.y - 12.0);
    pointer(&mut app, last_line, PointerAction::Move { delta: Vec2::X });
    pointer(
        &mut app,
        last_line,
        PointerAction::Press(PointerButton::Primary),
    );
    pointer(
        &mut app,
        last_line,
        PointerAction::Release(PointerButton::Primary),
    );
    settle(&mut app);

    let before = viewport(&app, field);
    type_char(&mut app, '!');
    type_char(&mut app, '!');
    settle(&mut app);

    let after = viewport(&app, field);
    let caret = app
        .world()
        .get::<TextLayoutInfo>(field)
        .expect("a layout")
        .cursor
        .expect("a caret")
        .1;
    assert!(
        after.offset.y >= before.offset.y - 0.5,
        "the viewport followed the caret: {} then {}",
        before.offset.y,
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
    assert!(
        text.contains("!!"),
        "two characters went in: {:?}",
        &text[text.len() - 40..]
    );
    assert_eq!(
        app.world()
            .resource::<Heard>()
            .0
            .last()
            .map(|(t, _)| t.as_str()),
        Some(text.as_str()),
        "an edit reports the whole text"
    );

    // Ctrl+Enter ends the edit: one final change, and it carries all of it.
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
    assert!(value.len() > FILE_CHARS - 32, "and all 64 KiB of it");
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
