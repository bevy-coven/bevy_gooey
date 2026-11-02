//! Toast actions, sticky toasts, merged repeats, the log and the notification center. Laid out for
//! real in a headless window on a clock the test sets, driven by real pointer and keyboard input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;
use std::time::Duration;

/// Every `ToastAction` a game would see.
#[derive(Resource, Default)]
struct Actions(Vec<(Entity, String)>);

/// How many entities' fills, borders, nodes or visibility were written, frame after frame.
#[derive(Resource, Default)]
struct Writes(u32);

type Written = Or<(
    Changed<BackgroundColor>,
    Changed<BorderColor>,
    Changed<Node>,
    Changed<Visibility>,
    Changed<Toast>,
)>;

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.1;

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
    .init_resource::<Actions>()
    .init_resource::<Writes>()
    .add_observer(|action: On<ToastAction>, mut seen: ResMut<Actions>| {
        seen.0.push((action.toast, action.id.clone()))
    })
    .add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().spawn(Camera2d);
    app
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

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = window(app);
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

fn tap(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
    key(app, key_code, ButtonState::Released);
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

fn children_of(world: &World, entity: Entity) -> Vec<Entity> {
    world
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        for child in children_of(world, entity) {
            found.push(child);
            stack.push(child);
        }
    }
    found
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn toasts(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<Toast>>()
        .iter(app.world())
        .collect()
}

fn buttons(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<ToastButton>>()
        .iter(app.world())
        .collect()
}

fn words(app: &App, toast: Entity) -> Vec<String> {
    descendants(app.world(), toast)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect()
}

fn log_texts(app: &App) -> Vec<String> {
    app.world()
        .resource::<ToastLog>()
        .entries()
        .map(|e| e.text.clone())
        .collect()
}

fn actions(app: &mut App) -> Vec<(Entity, String)> {
    std::mem::take(&mut app.world_mut().resource_mut::<Actions>().0)
}

/// A click on the button, as the pointer makes it, on the test clock (two frames).
fn press(app: &mut App, entity: Entity) {
    let at = rect(app, entity).center();
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
    pointer(app, at, PointerAction::Press(PointerButton::Primary));
    pointer(app, at, PointerAction::Release(PointerButton::Primary));
}

#[test]
fn clicking_the_action_sends_one_toast_action_and_the_toast_goes() {
    let mut app = app();
    let toast = app
        .world_mut()
        .spawn(toast("Item deleted").action("Undo", "undo"))
        .id();
    frames(&mut app, 6);
    let [button] = buttons(&mut app)[..] else {
        panic!("one button");
    };
    assert!(words(&app, toast).contains(&"Undo".to_string()));
    press(&mut app, button);
    assert_eq!(actions(&mut app), vec![(toast, "undo".to_string())]);
    // Pressed again while it fades: nothing more.
    frames(&mut app, 2);
    press(&mut app, button);
    assert!(actions(&mut app).is_empty(), "one action a toast");
    frames(&mut app, 12);
    assert!(toasts(&mut app).is_empty(), "dismissed by the press");
}

#[test]
fn enter_on_the_focused_action_does_the_same() {
    let mut app = app();
    let toast = app
        .world_mut()
        .spawn(toast("Item deleted").action("Undo", "undo"))
        .id();
    frames(&mut app, 6);
    let [button] = buttons(&mut app)[..] else {
        panic!("one button");
    };
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(button, FocusCause::Navigated);
    app.world_mut().resource_mut::<InputFocusVisible>().0 = true;
    app.update();
    tap(&mut app, KeyCode::Enter);
    frames(&mut app, 2);
    assert_eq!(actions(&mut app), vec![(toast, "undo".to_string())]);
}

#[test]
fn three_identical_toasts_show_one_with_a_count_and_the_log_holds_all_three() {
    let mut app = app();
    for _ in 0..3 {
        app.world_mut().spawn(toast("Game saved"));
        frames(&mut app, 3);
    }
    let all = toasts(&mut app);
    assert_eq!(all.len(), 1, "merged into one");
    assert_eq!(app.world().get::<Toast>(all[0]).map(Toast::count), Some(3));
    assert!(
        words(&app, all[0]).contains(&"Game saved x3".to_string()),
        "{:?}",
        words(&app, all[0])
    );
    assert_eq!(
        log_texts(&app),
        vec!["Game saved", "Game saved", "Game saved"],
        "the log keeps each"
    );
}

#[test]
fn a_repeat_is_merged_only_while_the_window_is_open_and_only_if_it_is_the_same() {
    let mut app = app();
    app.world_mut().spawn(toast("Saved"));
    frames(&mut app, 3);
    // A different message, a different kind: not merged.
    app.world_mut().spawn(toast("Loaded"));
    app.world_mut().spawn(toast_alert("Saved"));
    frames(&mut app, 3);
    assert_eq!(toasts(&mut app).len(), 3);
    // Past the window (2 s of the test clock), the same message stands again by itself.
    frames(&mut app, 25);
    app.world_mut().spawn(toast("Saved"));
    frames(&mut app, 3);
    let saved: Vec<Entity> = toasts(&mut app)
        .into_iter()
        .filter(|&t| {
            app.world()
                .get::<Toast>(t)
                .is_some_and(|t| t.text() == "Saved" && t.kind() == ToastKind::Info)
        })
        .collect();
    assert!(!saved.is_empty(), "the repeat stands");
    assert!(
        saved
            .iter()
            .all(|&t| app.world().get::<Toast>(t).map(Toast::count) == Some(1)),
        "no merge past the window"
    );
}

#[test]
fn a_merge_stands_the_toast_again_from_the_start() {
    let mut app = app();
    let first = app.world_mut().spawn(toast("Saved")).id();
    frames(&mut app, 12);
    let age = app.world().get::<Toast>(first).expect("toast").age();
    assert!(age > 1.0);
    app.world_mut().spawn(toast("Saved"));
    frames(&mut app, 1);
    let after = app.world().get::<Toast>(first).expect("still there").age();
    assert!(
        after < 0.6,
        "age went back to the start of its hold: {after}"
    );
}

#[test]
fn merging_can_be_turned_off() {
    let mut app = app();
    app.insert_resource(ToastSettings {
        merge_seconds: 0.0,
        max_visible: 5,
        ..ToastSettings::default()
    });
    for _ in 0..2 {
        app.world_mut().spawn(toast("Saved"));
        frames(&mut app, 2);
    }
    assert_eq!(toasts(&mut app).len(), 2);
}

#[test]
fn a_sticky_toast_stays_until_dismissed_and_a_newer_one_never_pushes_it_out() {
    let mut app = app();
    let sticky = app
        .world_mut()
        .spawn(toast_alert("Could not save").sticky())
        .id();
    // Far past any reading time, with more toasts than the stack holds.
    for n in 0..5 {
        app.world_mut().spawn(toast(format!("News {n}")));
        frames(&mut app, 8);
    }
    frames(&mut app, 200);
    assert!(app.world().get::<Toast>(sticky).is_some(), "still standing");
    assert_eq!(
        app.world().get::<Toast>(sticky).map(Toast::alpha),
        Some(1.0)
    );
    assert!(
        app.world()
            .get::<Toast>(sticky)
            .is_some_and(Toast::is_sticky)
    );
    let [dismiss] = buttons(&mut app)[..] else {
        panic!("a Dismiss button");
    };
    assert!(words(&app, sticky).contains(&"Dismiss".to_string()));
    press(&mut app, dismiss);
    assert!(actions(&mut app).is_empty(), "Dismiss is not an action");
    frames(&mut app, 12);
    assert!(
        app.world().get::<Toast>(sticky).is_none(),
        "gone once dismissed"
    );
}

#[test]
fn a_toast_with_an_action_stands_at_least_the_action_time_and_a_plain_one_does_not() {
    let mut app = app();
    let plain = app.world_mut().spawn(toast("Hi")).id();
    let with_button = app
        .world_mut()
        .spawn(toast("Hi there").action("Undo", "undo"))
        .id();
    frames(&mut app, 45);
    assert!(
        app.world().get::<Toast>(plain).is_none(),
        "a plain one is gone after 4.5 s"
    );
    assert_eq!(
        app.world().get::<Toast>(with_button).map(Toast::alpha),
        Some(1.0),
        "the one with a button is still whole"
    );
    frames(&mut app, 30);
    assert!(
        app.world().get::<Toast>(with_button).is_none(),
        "and goes by 7.5 s"
    );
}

#[test]
fn only_the_buttons_take_the_pointer_and_focus_and_a_plain_toast_has_none() {
    let mut app = app();
    let plain = app.world_mut().spawn(toast("Plain")).id();
    let with_button = app
        .world_mut()
        .spawn(toast("Item deleted").action("Undo", "undo"))
        .id();
    frames(&mut app, 6);
    for &toast in &[plain, with_button] {
        for node in std::iter::once(toast).chain(descendants(app.world(), toast)) {
            let is_button = app.world().get::<ToastButton>(node).is_some()
                || descendants(app.world(), with_button)
                    .iter()
                    .any(|&d| d == node && app.world().get::<Text>(node).is_some() && false);
            let pickable = app.world().get::<Pickable>(node);
            if is_button {
                assert!(app.world().get::<TabIndex>(node).is_some(), "a tab stop");
                assert!(
                    pickable.is_none_or(|p| p.should_block_lower),
                    "takes the pointer"
                );
            } else if app.world().get::<Node>(node).is_some()
                && app.world().get::<Text>(node).is_none()
                || app.world().get::<Text>(node).is_some()
                    && app
                        .world()
                        .get::<ChildOf>(node)
                        .is_some_and(|c| app.world().get::<ToastButton>(c.parent()).is_none())
            {
                assert_eq!(
                    pickable,
                    Some(&Pickable::IGNORE),
                    "{node} lets the pointer through"
                );
                assert!(app.world().get::<TabIndex>(node).is_none(), "no tab stop");
            }
        }
    }
    assert_eq!(buttons(&mut app).len(), 1);
}

#[test]
fn focus_on_a_button_that_goes_with_its_toast_is_cleared() {
    let mut app = app();
    app.world_mut()
        .spawn(toast("Item deleted").action("Undo", "undo"));
    frames(&mut app, 6);
    let [button] = buttons(&mut app)[..] else {
        panic!("one button");
    };
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(button, FocusCause::Navigated);
    frames(&mut app, 100);
    assert!(toasts(&mut app).is_empty());
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}

#[test]
fn the_log_keeps_the_last_two_hundred_with_their_time_kind_and_words() {
    let mut app = app();
    app.insert_resource(ToastSettings {
        merge_seconds: 0.0,
        ..ToastSettings::default()
    });
    for n in 0..205 {
        if n == 3 {
            app.world_mut().spawn(toast_alert(format!("Message {n}")));
        } else {
            app.world_mut().spawn(toast(format!("Message {n}")));
        }
        app.update();
    }
    let log = app.world().resource::<ToastLog>();
    assert_eq!(log.len(), ToastLog::CAPACITY);
    let first = log.entries().next().expect("an entry");
    assert_eq!(first.text, "Message 5", "the oldest five are forgotten");
    let last = log.entries().next_back().expect("an entry");
    assert_eq!(last.text, "Message 204");
    assert!(
        last.seconds > first.seconds,
        "{} {}",
        first.seconds,
        last.seconds
    );
    assert!(log.entries().all(|e| e.kind == ToastKind::Info));
}

/// The text of every row of the center, top to bottom.
fn rows(app: &mut App, center: Entity) -> Vec<String> {
    let mut found: Vec<(f32, String)> = Vec::new();
    for e in descendants(app.world(), center) {
        if app.world().get::<NotificationRow>(e).is_some() {
            let text = descendants(app.world(), e)
                .into_iter()
                .chain(std::iter::once(e))
                .find_map(|d| app.world().get::<Text>(d).map(|t| t.0.clone()))
                .expect("a row has words");
            found.push((rect(app, e).min.y, text));
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    found.into_iter().map(|(_, t)| t).collect()
}

#[test]
fn the_notification_center_lists_the_log_newest_first_and_follows_it() {
    let mut app = app();
    let center = app.world_mut().spawn(notification_center()).id();
    app.world_mut().spawn(screen()).add_child(center);
    frames(&mut app, 4);
    assert_eq!(rows(&mut app, center), vec!["No notifications yet"]);
    app.world_mut().spawn(toast("Game saved"));
    frames(&mut app, 8);
    app.world_mut().spawn(toast_alert("Could not save"));
    frames(&mut app, 4);
    let listed = rows(&mut app, center);
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert!(listed[0].ends_with("! Could not save"), "{listed:?}");
    assert!(
        listed[1].ends_with("Game saved") && !listed[1].contains('!'),
        "{listed:?}"
    );
    // The time of each, as a clock: the first at 0:00, the second a second or so later.
    assert!(listed[1].starts_with("0:0"), "{listed:?}");
    app.world_mut().resource_mut::<ToastLog>().clear();
    frames(&mut app, 2);
    assert_eq!(rows(&mut app, center), vec!["No notifications yet"]);
}

#[test]
fn a_hundred_entries_scroll_in_a_list_and_each_row_is_a_target_high_button() {
    let mut app = app();
    app.insert_resource(ToastSettings {
        merge_seconds: 0.0,
        ..ToastSettings::default()
    });
    for n in 0..100 {
        app.world_mut()
            .resource_mut::<ToastLog>()
            .push(ToastLogEntry {
                seconds: n as f32,
                kind: ToastKind::Info,
                text: format!("Entry {n}"),
            });
    }
    let center = app.world_mut().spawn(notification_center()).id();
    app.world_mut().spawn(screen()).add_child(center);
    frames(&mut app, 6);
    let listed = rows(&mut app, center);
    assert_eq!(listed.len(), 100);
    assert!(listed[0].ends_with("Entry 99"), "newest first");
    let skin = GooeySkin::default();
    let row = descendants(app.world(), center)
        .into_iter()
        .find(|&e| app.world().get::<NotificationRow>(e).is_some())
        .expect("a row");
    assert!(rect(&app, row).height() >= skin.target() - 1.0);
    assert!(
        app.world().get::<TabIndex>(row).is_some(),
        "a tab stop: the keys scroll it"
    );
    assert!(
        rect(&app, center).height() < 600.0,
        "a list, not 100 rows tall"
    );
}

#[test]
fn an_action_toast_holding_writes_nothing_after_it_has_opened() {
    let mut app = app();
    app.world_mut()
        .spawn(toast("Item deleted").action("Undo", "undo"));
    frames(&mut app, 8);
    app.world_mut().resource_mut::<Writes>().0 = 0;
    frames(&mut app, 30);
    assert_eq!(app.world().resource::<Writes>().0, 0);
}

#[test]
fn the_action_buttons_words_read_on_its_fill_in_every_look() {
    for skin in [
        GooeySkin::dark(),
        GooeySkin::parchment(),
        GooeySkin::high_contrast(),
        GooeySkin::tool(),
    ] {
        let mut app = app();
        app.insert_resource(skin.clone());
        let toast = app
            .world_mut()
            .spawn(toast("Item deleted").action("Undo", "undo"))
            .id();
        frames(&mut app, 10);
        let [button] = buttons(&mut app)[..] else {
            panic!("one button");
        };
        let fill = Srgba::from(app.world().get::<BackgroundColor>(button).expect("fill").0);
        let label = descendants(app.world(), button)
            .into_iter()
            .find(|&e| app.world().get::<Text>(e).is_some())
            .expect("label");
        let ink = Srgba::from(app.world().get::<TextColor>(label).expect("ink").0);
        let ratio = contrast_ratio(
            [ink.red, ink.green, ink.blue],
            [fill.red, fill.green, fill.blue],
        );
        assert!(ratio >= 4.5, "{ratio}");
        assert_eq!(app.world().get::<Toast>(toast).map(Toast::alpha), Some(1.0));
        assert!(rect(&app, button).height() >= skin.target() - 1.0);
    }
}
