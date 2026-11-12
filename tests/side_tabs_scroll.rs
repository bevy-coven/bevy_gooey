//! A vertical tab strip with more tabs than room scrolls: the wheel moves it, a rail shows while the
//! tabs do not fit, and the tab focus moves to, or that is selected, is kept in sight (#87, G-76:
//! "many tabs scroll"). Laid out for real in a headless window, driven by real keyboard and pointer
//! input.

use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::touch::TouchPhase;
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, ScrollPosition, Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// The height the host gives the strip, logical px: room for about four icon tabs.
const HOST_HEIGHT: f32 = 220.0;

fn app_with(settings: MotionSettings) -> App {
    let mut app = App::new();
    app.insert_resource(settings);
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
        16,
    )));
    app.world_mut().spawn(Camera2d);
    app
}

/// A headless app with motion off, so a scroll lands in the frame it is asked for.
fn app() -> App {
    app_with(MotionSettings::off())
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// A strip in a host of `HOST_HEIGHT`, its tabs and its scrolling list.
struct Bar {
    strip: Entity,
    list: Entity,
    tabs: Vec<Entity>,
}

fn labels(count: usize) -> Vec<SideTab> {
    (0..count)
        .map(|n| SideTab::new(format!("Tab {n}")).icon(IconSource::Image(Handle::default())))
        .collect()
}

/// `count` tabs in a strip (the icon strip, or the labelled one) standing in a host of
/// `HOST_HEIGHT`, with `selected` lit.
fn bar_of(app: &mut App, count: usize, selected: usize, icons: bool) -> Bar {
    let host = app
        .world_mut()
        .spawn(Node {
            height: Val::Px(HOST_HEIGHT),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let strip = if icons {
        app.world_mut()
            .spawn(side_tabs_icons(labels(count), selected))
            .id()
    } else {
        app.world_mut()
            .spawn(side_tabs(labels(count), selected))
            .id()
    };
    app.world_mut().entity_mut(host).add_child(strip);
    app.world_mut().spawn(screen()).add_child(host);
    settle(app);
    let list = app.world().get::<Children>(strip).expect("the list")[0];
    let tabs = app
        .world()
        .get::<Children>(list)
        .expect("tabs")
        .iter()
        .collect();
    Bar { strip, list, tabs }
}

fn bar(app: &mut App, count: usize, selected: usize) -> Bar {
    bar_of(app, count, selected, true)
}

fn rect(app: &App, entity: Entity) -> Rect {
    let node = app.world().get::<ComputedNode>(entity).expect("laid out");
    let at = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("placed")
        .translation;
    let scale = node.inverse_scale_factor();
    Rect::from_center_size(at * scale, node.size() * scale)
}

fn part(app: &App, root: Entity, role: Role) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if app.world().get::<Role>(entity) == Some(&role) {
            return Some(entity);
        }
        if let Some(children) = app.world().get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    None
}

fn scroll_of(app: &App, list: Entity) -> f32 {
    app.world()
        .get::<ScrollPosition>(list)
        .expect("a scroll position")
        .y
}

/// How far the strip can scroll, logical px.
fn max_scroll(app: &App, list: Entity) -> f32 {
    let node = app.world().get::<ComputedNode>(list).expect("laid out");
    (node.content_size().y - node.size().y).max(0.0) * node.inverse_scale_factor()
}

fn rail_shown(app: &App, strip: Entity) -> bool {
    let rail = part(app, strip, Role::ListRail).expect("a rail");
    app.world().get::<Node>(rail).expect("a node").display != Display::None
}

fn window(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a window")
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
    settle(app);
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

/// One notch of the wheel over `at`, `lines` lines down.
fn wheel(app: &mut App, at: Vec2, lines: f32) {
    pointer(app, at, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
    pointer(
        app,
        at,
        PointerAction::Scroll {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -lines,
            phase: TouchPhase::Moved,
        },
    );
    settle(app);
}

/// Focus on a tab, as a key would have put it there, shown.
fn focus(app: &mut App, tab: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(tab, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(app);
}

/// Whether a tab is wholly inside the strip's viewport.
fn in_sight(app: &App, bar: &Bar, tab: Entity) -> bool {
    let (view, tab) = (rect(app, bar.list), rect(app, tab));
    tab.min.y >= view.min.y - 0.5 && tab.max.y <= view.max.y + 0.5
}

#[test]
fn a_strip_with_more_tabs_than_room_takes_the_hosts_height_and_shows_a_rail() {
    let mut app = app();
    let ui = bar(&mut app, 12, 0);
    let strip = rect(&app, ui.strip);
    assert!(
        strip.height() <= HOST_HEIGHT + 0.5,
        "the strip is {} tall in a host of {HOST_HEIGHT}",
        strip.height()
    );
    assert!(
        max_scroll(&app, ui.list) > 100.0,
        "and has more to scroll to: {}",
        max_scroll(&app, ui.list)
    );
    assert!(
        rail_shown(&app, ui.strip),
        "the rail shows while the tabs do not fit"
    );
    assert_eq!(scroll_of(&app, ui.list), 0.0, "at the top");
    // The tabs keep their own size: none was squeezed to fit.
    let first = rect(&app, ui.tabs[0]);
    let last = rect(&app, ui.tabs[11]);
    assert!(
        (first.width() - first.height()).abs() < 0.5
            && (last.height() - first.height()).abs() < 0.5,
        "square tabs: {first:?} and {last:?}"
    );
    assert!(app.world().get::<TabScroll>(ui.list).is_some());
}

#[test]
fn a_strip_that_fits_has_no_rail_and_nothing_to_scroll() {
    let mut app = app();
    let ui = bar(&mut app, 3, 0);
    assert!(
        rect(&app, ui.strip).height() < HOST_HEIGHT - 20.0,
        "only as tall as its tabs"
    );
    assert!(!rail_shown(&app, ui.strip), "no rail");
    assert_eq!(max_scroll(&app, ui.list), 0.0);
    focus(&mut app, ui.tabs[2]);
    assert_eq!(scroll_of(&app, ui.list), 0.0, "and focus moves nothing");
}

#[test]
fn the_wheel_scrolls_the_strip_and_stops_at_both_ends() {
    let mut app = app();
    let ui = bar(&mut app, 12, 0);
    let over = rect(&app, ui.tabs[1]).center();
    wheel(&mut app, over, 1.0);
    let one = scroll_of(&app, ui.list);
    assert!(one > 20.0, "a notch scrolls {one}");
    for _ in 0..14 {
        wheel(&mut app, over, 1.0);
    }
    assert!(
        (scroll_of(&app, ui.list) - max_scroll(&app, ui.list)).abs() < 1.0,
        "the end"
    );
    assert!(
        in_sight(&app, &ui, ui.tabs[11]),
        "the last tab shows at the end"
    );
    for _ in 0..16 {
        wheel(&mut app, over, -1.0);
    }
    assert_eq!(scroll_of(&app, ui.list), 0.0, "and the top");
}

#[test]
fn stepping_through_the_tabs_with_the_keys_keeps_the_focused_one_in_sight() {
    let mut app = app();
    let ui = bar(&mut app, 12, 0);
    focus(&mut app, ui.tabs[0]);
    for step in 1..12 {
        tap(&mut app, KeyCode::ArrowDown);
        assert_eq!(
            app.world().get::<TabsValue>(ui.strip).map(|v| v.0),
            Some(step),
            "Down selects tab {step}"
        );
        assert!(
            in_sight(&app, &ui, ui.tabs[step]),
            "tab {step} is in sight at scroll {}",
            scroll_of(&app, ui.list)
        );
    }
    assert!(scroll_of(&app, ui.list) > 100.0, "the strip followed");
    // And back up to the first.
    for step in (0..11).rev() {
        tap(&mut app, KeyCode::ArrowUp);
        assert!(
            in_sight(&app, &ui, ui.tabs[step]),
            "tab {step} on the way up"
        );
    }
    assert_eq!(scroll_of(&app, ui.list), 0.0, "back at the top");
}

#[test]
fn focus_a_key_moved_shows_the_next_tab_too() {
    let mut app = app();
    let ui = bar(&mut app, 12, 0);
    focus(&mut app, ui.tabs[5]);
    // The strip shows the tab after the focused one, whole or cut.
    let view = rect(&app, ui.list);
    let next = rect(&app, ui.tabs[6]);
    assert!(
        next.min.y < view.max.y,
        "the tab after the focused one shows: {next:?} in {view:?}"
    );
    assert!(in_sight(&app, &ui, ui.tabs[5]), "and the focused one whole");
}

#[test]
fn a_tab_selected_from_code_is_brought_into_sight_and_a_strip_opens_on_its_selected_tab() {
    let mut app = app();
    let ui = bar(&mut app, 12, 0);
    app.world_mut().entity_mut(ui.strip).insert(TabsValue(10));
    settle(&mut app);
    assert!(
        app.world().get::<Selected>(ui.tabs[10]).is_some(),
        "the tenth is lit"
    );
    assert!(
        in_sight(&app, &ui, ui.tabs[10]),
        "and in sight at scroll {}",
        scroll_of(&app, ui.list)
    );

    // A strip made with the tenth selected opens with it in view.
    let mut app = self::app();
    let ui = bar(&mut app, 12, 10);
    assert!(
        in_sight(&app, &ui, ui.tabs[10]),
        "opened on the tenth: scroll {}",
        scroll_of(&app, ui.list)
    );
    // The player's wheel after that is not undone.
    let over = rect(&app, ui.tabs[10]).center();
    wheel(&mut app, over, -3.0);
    let after = scroll_of(&app, ui.list);
    settle(&mut app);
    assert_eq!(
        scroll_of(&app, ui.list),
        after,
        "the wheel stays where it left it"
    );
}

#[test]
fn the_labelled_strip_scrolls_too() {
    let mut app = app();
    let ui = bar_of(&mut app, 12, 0, false);
    assert!(rect(&app, ui.strip).height() <= HOST_HEIGHT + 0.5);
    assert!(rail_shown(&app, ui.strip));
    focus(&mut app, ui.tabs[0]);
    for _ in 0..11 {
        tap(&mut app, KeyCode::ArrowDown);
    }
    assert!(
        in_sight(&app, &ui, ui.tabs[11]),
        "the last labelled tab is in sight"
    );
}

#[test]
fn a_reveal_glides_when_motion_is_on_and_lands_where_it_was_asked() {
    let mut app = app_with(MotionSettings::default());
    let ui = bar(&mut app, 12, 0);
    for _ in 0..60 {
        app.update();
    }
    app.world_mut().entity_mut(ui.strip).insert(TabsValue(9));
    app.update();
    app.update();
    let glide = app.world().get::<ScrollGlide>(ui.list).expect("a glide");
    let goal = glide.goal().expect("a reveal glides");
    assert!(
        scroll_of(&app, ui.list) < goal,
        "it is on its way, not there: {} of {goal}",
        scroll_of(&app, ui.list)
    );
    for _ in 0..120 {
        app.update();
    }
    assert!(
        (scroll_of(&app, ui.list) - goal).abs() < 1.0,
        "and lands on it"
    );
    assert!(in_sight(&app, &ui, ui.tabs[9]));
}

#[test]
fn focus_put_on_a_tab_before_the_strip_is_laid_out_is_revealed_when_it_is() {
    let mut app = app();
    let host = app
        .world_mut()
        .spawn(Node {
            height: Val::Px(HOST_HEIGHT),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let strip = app.world_mut().spawn(side_tabs_icons(labels(12), 0)).id();
    app.world_mut().entity_mut(host).add_child(strip);
    app.world_mut().spawn(screen()).add_child(host);
    let list = app.world().get::<Children>(strip).expect("the list")[0];
    let tabs: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .expect("tabs")
        .iter()
        .collect();
    // No frame has run: nothing is laid out yet.
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(tabs[11], FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    settle(&mut app);
    let ui = Bar { strip, list, tabs };
    assert!(
        in_sight(&app, &ui, ui.tabs[11]),
        "the focused last tab is in sight at scroll {}",
        scroll_of(&app, ui.list)
    );
}

#[test]
fn tabs_taken_away_while_scrolled_to_the_end_leave_the_strip_in_range_and_revealing() {
    let mut app = app();
    let ui = bar(&mut app, 12, 0);
    let over = rect(&app, ui.tabs[1]).center();
    for _ in 0..14 {
        wheel(&mut app, over, 1.0);
    }
    for &tab in &ui.tabs[6..] {
        app.world_mut().entity_mut(tab).despawn();
    }
    settle(&mut app);
    assert!(
        scroll_of(&app, ui.list) <= max_scroll(&app, ui.list) + 1.0,
        "the scroll {} is within what is left, {}",
        scroll_of(&app, ui.list),
        max_scroll(&app, ui.list)
    );
    focus(&mut app, ui.tabs[0]);
    assert!(in_sight(&app, &ui, ui.tabs[0]), "tab 0 is brought back");
    assert_eq!(scroll_of(&app, ui.list), 0.0);
}
