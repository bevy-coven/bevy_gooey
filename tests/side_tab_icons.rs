//! The vertical tab strip with icons: an activity bar's workspaces, each a picture, a name shown on
//! hover and on focus, and a badge for what needs the person. Laid out for real in a headless window.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

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
    app.init_resource::<Heard>();
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// Every `ValueChange<usize>` the strip sent.
#[derive(Resource, Default)]
struct Heard(Vec<usize>);

fn handle() -> Handle<Image> {
    Handle::default()
}

/// Five icon tabs in a narrow strip on a screen, its pages beside it.
struct Bar {
    strip: Entity,
    tabs: Vec<Entity>,
}

fn bar(app: &mut App, selected: usize) -> Bar {
    let names = ["Files", "Search", "Source", "Debug", "Gear"];
    let tabs: Vec<SideTab> = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let tab = SideTab::new(*name).icon(IconSource::Image(handle()));
            if index == 3 { tab.badge("4") } else { tab }
        })
        .collect();
    let strip = app
        .world_mut()
        .spawn(side_tabs_icons(tabs, selected))
        .observe(|change: On<ValueChange<usize>>, mut heard: ResMut<Heard>| {
            heard.0.push(change.value);
        })
        .id();
    app.world_mut().spawn(screen()).add_child(strip);
    settle(app);
    let list = app.world().get::<Children>(strip).expect("the list")[0];
    let tabs = app
        .world()
        .get::<Children>(list)
        .expect("tabs")
        .iter()
        .collect();
    Bar { strip, tabs }
}

fn rect(app: &App, entity: Entity) -> (f32, f32, f32, f32) {
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
    (
        at.x - size.x / 2.0,
        at.y - size.y / 2.0,
        at.x + size.x / 2.0,
        at.y + size.y / 2.0,
    )
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

fn tap(app: &mut App, key_code: KeyCode) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("a window");
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

/// The acceptance: five icon tabs, the third selected, and the selection, the tooltips and one badge
/// read back.
#[test]
fn five_icon_tabs_select_the_third_and_name_themselves_on_hover() {
    let mut app = app();
    let ui = bar(&mut app, 2);
    assert_eq!(ui.tabs.len(), 5, "five tabs");

    let value = app
        .world()
        .get::<TabsValue>(ui.strip)
        .expect("the strip's state");
    assert_eq!(value.0, 2, "the third tab is selected");
    assert!(app.world().get::<Selected>(ui.tabs[2]).is_some(), "and lit");
    assert!(app.world().get::<Selected>(ui.tabs[0]).is_none());

    // Every tab is a picture, and every tab names itself in a tooltip.
    for (index, &tab) in ui.tabs.iter().enumerate() {
        assert!(
            part(&app, tab, Role::Icon).is_some(),
            "tab {index} draws an icon"
        );
        let tooltip = app
            .world()
            .get::<Tooltip>(tab)
            .unwrap_or_else(|| panic!("tab {index} has a tooltip"));
        assert_eq!(
            tooltip.text,
            ["Files", "Search", "Source", "Debug", "Gear"][index],
            "tab {index} is named"
        );
    }

    // The fourth tab carries a badge, and the others do not.
    let badge = part(&app, ui.tabs[3], Role::CornerBadge).expect("the badge");
    let words = app.world().get::<Text>(badge).expect("its count");
    assert_eq!(words.0, "4");
    for (index, &tab) in ui.tabs.iter().enumerate() {
        if index != 3 {
            assert!(
                part(&app, tab, Role::CornerBadge).is_none(),
                "tab {index} has no badge"
            );
        }
    }

    // A narrow strip: the tabs are squares, not label-wide.
    let (left, _, right, _) = rect(&app, ui.tabs[0]);
    let gooey = GooeySkin::default();
    assert!(
        right - left <= gooey.target() * 2.0,
        "an icon-only strip is {:?} wide",
        right - left
    );

    // Choosing another with the keys works as for any side strip.
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(ui.tabs[2], FocusCause::Navigated);
    settle(&mut app);
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        app.world().get::<TabsValue>(ui.strip).map(|value| value.0),
        Some(3),
        "Down selects the fourth"
    );
    assert_eq!(
        app.world().resource::<Heard>().0,
        vec![3],
        "and reports it once"
    );
}

/// No icon falls back to the label's initial, and a badge of 0 shows nothing.
#[test]
fn a_tab_without_an_icon_shows_its_first_letter_and_a_zero_badge_hides() {
    let mut app = app();
    let strip = app
        .world_mut()
        .spawn(side_tabs_icons(
            [
                SideTab::new("Files"),
                SideTab::new("Search").badge("0"),
                SideTab::new("Source").badge("7"),
            ],
            0,
        ))
        .id();
    app.world_mut().spawn(screen()).add_child(strip);
    settle(&mut app);
    let list = app.world().get::<Children>(strip).expect("the list")[0];
    let tabs: Vec<Entity> = app
        .world()
        .get::<Children>(list)
        .expect("tabs")
        .iter()
        .collect();

    // An icon-less tab is a word of one letter, not a picture.
    assert!(part(&app, tabs[0], Role::Icon).is_none(), "no picture");
    let letter = part(&app, tabs[0], Role::TabText).expect("its letter");
    assert_eq!(
        app.world().get::<Text>(letter).map(|t| t.0.as_str()),
        Some("F"),
        "the label's initial"
    );
    assert_eq!(
        app.world()
            .get::<Tooltip>(tabs[0])
            .map(|tooltip| tooltip.text.as_str()),
        Some("Files"),
        "and the whole name in the tooltip"
    );

    // A badge of 0 shows nothing; a badge of 7 shows.
    assert!(
        part(&app, tabs[1], Role::CornerBadge).is_none(),
        "a zero badge hides"
    );
    let badge = part(&app, tabs[2], Role::CornerBadge).expect("the second badge");
    assert_eq!(
        app.world().get::<Text>(badge).map(|t| t.0.as_str()),
        Some("7")
    );
}

/// The strip is a narrow column of square targets, its badge in the tab's top-right corner, and the
/// sliding indicator still on the selected tab.
#[test]
fn the_strip_is_a_narrow_column_of_square_targets_with_the_badge_in_the_corner() {
    let mut app = app();
    let ui = bar(&mut app, 1);
    let gooey = GooeySkin::default();

    // Squares: as tall as wide, and as tall as a button's target.
    for (index, &tab) in ui.tabs.iter().enumerate() {
        let (left, top, right, bottom) = rect(&app, tab);
        let (width, height) = (right - left, bottom - top);
        assert!(
            (width - height).abs() < 1.0,
            "tab {index} is {width} by {height}, not a square"
        );
        assert!(
            height >= gooey.target() - 1.0,
            "tab {index} is {height} high, under the target"
        );
    }
    // A column: each tab stands below the one before, and they all share a left edge.
    let first = rect(&app, ui.tabs[0]);
    for pair in ui.tabs.windows(2) {
        let (a, b) = (rect(&app, pair[0]), rect(&app, pair[1]));
        assert!(b.1 >= a.3 - 0.5, "one above the other: {a:?} {b:?}");
        assert!((b.0 - first.0).abs() < 1.0, "one column: {a:?} {b:?}");
    }

    // The fourth tab's badge is in its top-right corner: right of the tab's middle and above it.
    let badge = part(&app, ui.tabs[3], Role::CornerBadge).expect("the badge");
    let tab = rect(&app, ui.tabs[3]);
    let (left, top, right, _) = rect(&app, badge);
    assert!(
        left > (tab.0 + tab.2) / 2.0 - 1.0 && right >= tab.2 - 1.0,
        "the badge is at the tab's right: {left}..{right} in {tab:?}"
    );
    assert!(top <= tab.1 + 1.0, "and at its top: {top} in {tab:?}");

    // The sliding indicator is the strip's own, spawned beside its tab list (it is hidden at rest:
    // the lit tab carries the look, and only a travel shows the puck).
    let indicator = app
        .world()
        .get::<Children>(ui.strip)
        .expect("the strip's children")
        .iter()
        .find(|&kid| app.world().get::<SelectionIndicator>(kid).is_some());
    assert!(indicator.is_some(), "the strip has its selection indicator");
}
