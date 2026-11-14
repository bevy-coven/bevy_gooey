//! What was left of the editor's hand-built nodes after G-88 (G-89, #100): a node that starts closed,
//! a scroll region that follows focus and answers the Page keys, a host that drives display and
//! height, and a host-managed scroll pane, each laid out for real in a headless window.

use bevy::ecs::relationship::RelatedSpawner;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

fn headless() -> App {
    headless_at(1.0)
}

/// The same at a window scale factor, so a device pixel is not a logical one.
fn headless_at(scale: f32) -> App {
    let mut app = App::new();
    app.insert_resource(MotionSettings::off());
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        (1280.0 * scale) as u32,
                        (720.0 * scale) as u32,
                    )
                    .with_scale_factor_override(scale),
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

/// A node's rectangle in the window, logical px.
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

fn children(app: &App, entity: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

// ---- 1. a node that starts closed --------------------------------------------------------------

fn tab_stops(app: &mut App, presses: usize) -> Vec<Option<String>> {
    (0..presses)
        .map(|_| {
            bevy_gooey::testing::press(app, KeyCode::Tab);
            bevy_gooey::testing::focused_label(app)
        })
        .collect()
}

/// `Shown(false)` closes the node **at the moment it is inserted**, not at the next restyle: a node
/// spawned after the frame's style pass is already `Display::None` and `Visibility::Hidden` before any
/// layout could draw it, so there is no frame open.
#[test]
fn a_node_spawned_with_shown_false_is_closed_before_any_frame_runs() {
    let mut app = headless();
    let page = app
        .world_mut()
        .spawn((panel(), Shown(false), children![button("In")]))
        .id();
    // No update has run: the hook did it.
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::None
    );
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Hidden)
    );
    let outside = app.world_mut().spawn(button("Outside")).id();
    app.world_mut()
        .spawn(screen())
        .add_child(page)
        .add_child(outside);
    app.update();
    assert!(
        rect(&app, page).size().length() < 0.5,
        "it has no size on its first frame"
    );
    let stops = tab_stops(&mut app, 3);
    assert!(
        stops.iter().any(|s| s.as_deref() == Some("Outside")),
        "the control IS reachable, so the next assertion is not vacuous: {stops:?}"
    );
    assert!(
        stops.iter().all(|s| s.as_deref() != Some("In")),
        "no stop in the closed node: {stops:?}"
    );
    // Opening it later gives back what it had.
    app.world_mut().entity_mut(page).insert(Shown(true));
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::Flex,
        "the hook restores at insert"
    );
    settle(&mut app);
    assert!(rect(&app, page).height() > 1.0);
}

/// `ShownFollow::closed()` spawns the node closed with no host write: the host's own systems open it
/// later by writing `display`, as before.
#[test]
fn shown_follow_closed_spawns_a_closed_node_the_host_can_open() {
    let mut app = headless();
    let page = app
        .world_mut()
        .spawn((panel(), ShownFollow::closed(), children![button("In")]))
        .id();
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::None,
        "closed at spawn"
    );
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Hidden)
    );
    assert!(
        app.world().get::<StartsClosed>(page).is_none(),
        "the marker is one-shot"
    );
    let outside = app.world_mut().spawn(button("Outside")).id();
    app.world_mut()
        .spawn(screen())
        .add_child(page)
        .add_child(outside);
    app.update();
    assert!(rect(&app, page).size().length() < 0.5);
    let stops = tab_stops(&mut app, 3);
    assert!(
        stops.iter().any(|s| s.as_deref() == Some("Outside")),
        "the control IS reachable, so the next assertion is not vacuous: {stops:?}"
    );
    assert!(
        stops.iter().all(|s| s.as_deref() != Some("In")),
        "no stop in the closed node: {stops:?}"
    );
    app.world_mut()
        .get_mut::<Node>(page)
        .expect("a node")
        .display = Display::Flex;
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Inherited)
    );
    assert!(
        tab_stops(&mut app, 3)
            .iter()
            .any(|s| s.as_deref() == Some("In"))
    );
}

/// The G-88 review: the visibility a host sets while the node is closed is not overwritten by a stale
/// memory when it opens.
#[test]
fn a_visibility_the_host_set_while_closed_is_kept_when_it_opens() {
    let mut app = headless();
    let page = app
        .world_mut()
        .spawn((column(0.0), ShownFollow, Visibility::Visible))
        .id();
    app.world_mut().spawn(screen()).add_child(page);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Node>(page)
        .expect("a node")
        .display = Display::None;
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Hidden)
    );
    // The host decides, while it is closed, that the page is Inherited once open.
    app.world_mut()
        .entity_mut(page)
        .insert(Visibility::Inherited);
    app.world_mut()
        .get_mut::<Node>(page)
        .expect("a node")
        .display = Display::Flex;
    settle(&mut app);
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Inherited),
        "the host's value, not the stale Visible"
    );
}

/// A `ScrollGutter` that is not a number falls back to the list's own lane, not a zero lane.
#[test]
fn a_nan_scroll_gutter_falls_back_to_the_default_lane() {
    let mut app = headless();
    let mut lanes = Vec::new();
    for gutter in [None, Some(Space::Px(f32::NAN))] {
        let region = app
            .world_mut()
            .spawn(scroll_region(0.0, gutter, children![label("x")]))
            .id();
        let host = app
            .world_mut()
            .spawn(sized(Sizing {
                height: Some(Extent::Px(100.0)),
                ..default()
            }))
            .add_child(region)
            .id();
        app.world_mut().spawn(screen()).add_child(host);
        lanes.push(region);
    }
    settle(&mut app);
    let lane = |app: &App, r: Entity| {
        app.world()
            .get::<Node>(children(app, r)[0])
            .expect("a node")
            .padding
            .right
    };
    assert_ne!(lane(&app, lanes[1]), Val::Px(0.0));
    assert_eq!(lane(&app, lanes[0]), lane(&app, lanes[1]));
}

// ---- 3. display AND height written by the host --------------------------------------------------

/// A node the host drives (`display` by many systems, `height` every frame) with `ShownFollow` and
/// `HostGeometry`: the host's `display` and `height` stay through a skin change, with colour motion on
/// and off, and the visibility follows. The pattern of a terminal pane root.
#[test]
fn a_node_with_the_hosts_display_and_height_keeps_both_through_a_skin_change() {
    for motion in [false, true] {
        let mut app = headless();
        if motion {
            app.insert_resource(MotionSettings::default());
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f32(1.0 / 60.0),
            ));
        }
        let pane = app
            .world_mut()
            .spawn((
                panel(),
                ShownFollow,
                HostGeometry,
                Sizing {
                    height: Some(Extent::Px(10.0)),
                    ..default()
                },
                children![button("In")],
            ))
            .id();
        app.world_mut().spawn(screen()).add_child(pane);
        for _ in 0..10 {
            app.update();
        }
        for (height, display) in [
            (120.0, Display::Flex),
            (90.0, Display::None),
            (200.0, Display::Flex),
        ] {
            {
                let mut node = app.world_mut().get_mut::<Node>(pane).expect("a node");
                node.height = Val::Px(height);
                node.display = display;
            }
            app.world_mut().resource_mut::<GooeySkin>().surface =
                Color::srgb(height / 400.0, 0.3, 0.3);
            for _ in 0..30 {
                app.update();
            }
            let node = app.world().get::<Node>(pane).expect("a node").clone();
            assert_eq!(
                (node.height, node.display),
                (Val::Px(height), display),
                "motion {motion}"
            );
            let want = if display == Display::None {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            assert_eq!(
                app.world().get::<Visibility>(pane),
                Some(&want),
                "motion {motion}"
            );
        }
    }
}

// ---- 2. a scroll region that follows focus and pages -------------------------------------------

fn scroll_app(rows: usize) -> (App, Entity, Entity, Vec<Entity>) {
    let mut app = headless();
    let region = app
        .world_mut()
        .spawn(scroll_region(
            0.5,
            None,
            Children::spawn(SpawnWith(move |p: &mut RelatedSpawner<ChildOf>| {
                for i in 0..rows {
                    p.spawn(button(format!("Row {i}")));
                }
            })),
        ))
        .id();
    let host = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(400.0)),
            height: Some(Extent::Px(300.0)),
            ..default()
        }))
        .add_child(region)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    let view = children(&app, region)[0];
    let buttons = children(&app, children(&app, view)[0]);
    (app, region, view, buttons)
}

fn scroll_y(app: &App, view: Entity) -> f32 {
    app.world()
        .get::<ScrollPosition>(view)
        .expect("a scroll position")
        .y
}

fn inside(app: &App, view: Entity, e: Entity) -> bool {
    let (v, r) = (rect(app, view), rect(app, e));
    r.min.y >= v.min.y - 0.5 && r.max.y <= v.max.y + 0.5
}

/// Tab onto rows below the fold scrolls them into view, like a list's rows.
#[test]
fn a_scroll_region_follows_keyboard_focus_into_view() {
    let (mut app, region, view, buttons) = scroll_app(30);
    assert!(scroll_y(&app, view).abs() < 0.5);
    for _ in 0..12 {
        bevy_gooey::testing::press(&mut app, KeyCode::Tab);
    }
    let focused = bevy_gooey::testing::focused(&app).expect("a focus");
    assert!(buttons.contains(&focused), "focus is on a row");
    assert!(
        scroll_y(&app, view) > 50.0,
        "it scrolled: {}",
        scroll_y(&app, view)
    );
    assert!(
        inside(&app, view, focused),
        "the focused row is in view: {:?} in {:?}",
        rect(&app, focused),
        rect(&app, view)
    );
    let rail = children(&app, region)
        .into_iter()
        .find(|c| app.world().get::<Role>(*c) == Some(&Role::ListRail))
        .expect("the rail stays");
    assert_eq!(
        app.world().get::<Role>(rail),
        Some(&Role::ListRail),
        "the rail stays"
    );
    assert_eq!(
        app.world().get::<Node>(rail).expect("a node").display,
        Display::Flex
    );
}

/// PageDown and PageUp move focus a viewport and the region follows; Home and End go to the first
/// and last row and the region to its ends.
#[test]
fn a_scroll_region_answers_page_home_and_end_with_real_keys() {
    let (mut app, _region, view, buttons) = scroll_app(30);
    bevy_gooey::testing::press(&mut app, KeyCode::Tab);
    assert_eq!(bevy_gooey::testing::focused(&app), Some(buttons[0]));
    bevy_gooey::testing::press(&mut app, KeyCode::PageDown);
    let after_page = bevy_gooey::testing::focused(&app).expect("a focus");
    let idx = buttons
        .iter()
        .position(|b| *b == after_page)
        .expect("a row");
    assert!(idx >= 3, "a page down moved several rows: {idx}");
    assert!(inside(&app, view, after_page) && scroll_y(&app, view) > 0.5);
    bevy_gooey::testing::press(&mut app, KeyCode::PageUp);
    let back = bevy_gooey::testing::focused(&app).expect("a focus");
    assert!(
        buttons.iter().position(|b| *b == back).expect("a row") < idx,
        "and up again"
    );
    bevy_gooey::testing::press(&mut app, KeyCode::End);
    assert_eq!(
        bevy_gooey::testing::focused(&app),
        Some(*buttons.last().expect("rows"))
    );
    let max = {
        let c = app.world().get::<ComputedNode>(view).expect("laid out");
        (c.content_size().y + c.scrollbar_size.y - c.size().y).max(0.0) * c.inverse_scale_factor()
    };
    assert!(
        (scroll_y(&app, view) - max).abs() < 1.5,
        "at the end: {} of {max}",
        scroll_y(&app, view)
    );
    bevy_gooey::testing::press(&mut app, KeyCode::Home);
    assert_eq!(bevy_gooey::testing::focused(&app), Some(buttons[0]));
    assert!(
        scroll_y(&app, view).abs() < 1.5,
        "back at the top: {}",
        scroll_y(&app, view)
    );
}

// ---- 4. a host-managed scroll pane --------------------------------------------------------------

/// The pattern for a pane whose rows the host rebuilds every frame (a terminal body): put them in the
/// region's items column and replace them (despawn and spawn in one command flush). The viewport's
/// `ScrollPosition` is the offset and it is not the rows', so it survives, and so does what is drawn.
#[test]
fn a_scroll_view_whose_children_the_host_replaces_every_frame_keeps_its_offset() {
    let (mut app, _region, view, _buttons) = scroll_app(40);
    let column = children(&app, view)[0];
    app.world_mut()
        .get_mut::<ScrollPosition>(view)
        .expect("a scroll position")
        .y = 300.0;
    settle(&mut app);
    for frame in 0..8 {
        let old = children(&app, column);
        let mut commands = app.world_mut().commands();
        for e in old {
            commands.entity(e).despawn();
        }
        for i in 0..40 {
            commands
                .entity(column)
                .with_child(button(format!("Row {i} (frame {frame})")));
        }
        app.update();
        assert!(
            (scroll_y(&app, view) - 300.0).abs() < 0.5,
            "frame {frame}: {}",
            scroll_y(&app, view)
        );
    }
    settle(&mut app);
    let computed = app.world().get::<ComputedNode>(view).expect("laid out");
    assert!(
        (computed.scroll_position.y * computed.inverse_scale_factor - 300.0).abs() < 1.5,
        "what is drawn is scrolled too"
    );
}

// ---- review: widgets inside a region keep their own keys ---------------------------------------

/// A region holding `widget` first and thirty buttons after it, settled; the app and the viewport.
fn region_with(widget: impl Bundle) -> (App, Entity) {
    let mut app = headless();
    let region = app
        .world_mut()
        .spawn(scroll_region(
            0.5,
            None,
            Children::spawn(SpawnWith(move |p: &mut RelatedSpawner<ChildOf>| {
                p.spawn(widget);
                for i in 0..30 {
                    p.spawn(button(format!("Row {i}")));
                }
            })),
        ))
        .id();
    let host = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Px(500.0)),
            height: Some(Extent::Px(300.0)),
            ..default()
        }))
        .add_child(region)
        .id();
    app.world_mut().spawn(screen()).add_child(host);
    settle(&mut app);
    let view = children(&app, region)[0];
    (app, view)
}

fn within(app: &App, e: Entity, root: Entity) -> bool {
    e == root
        || app
            .world()
            .entity(e)
            .get::<ChildOf>()
            .is_some_and(|p| within(app, p.parent(), root))
}

fn first_with_role(app: &App, role: Role) -> Entity {
    app.world()
        .iter_entities()
        .find(|e| e.get::<Role>() == Some(&role))
        .map(|e| e.id())
        .expect("a widget with that role")
}

/// Focus the widget with Tab (it is the region's first stop) and press every key the region would
/// otherwise take: focus stays in the widget and the region does not scroll.
fn keeps_its_keys(role: Role, widget: impl Bundle, what: &str) {
    let (mut app, view) = region_with(widget);
    bevy_gooey::testing::press(&mut app, KeyCode::Tab);
    let at = bevy_gooey::testing::focused(&app).expect("a focus");
    let root = first_with_role(&app, role);
    assert!(
        within(&app, at, root),
        "{what}: Tab reached the widget first"
    );
    for key in [
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::End,
        KeyCode::Home,
    ] {
        bevy_gooey::testing::press(&mut app, key);
        let now = bevy_gooey::testing::focused(&app).expect("a focus");
        assert!(
            within(&app, now, root),
            "{what}: {key:?} took focus out of the widget to {now:?}"
        );
        assert!(
            scroll_y(&app, view).abs() < 0.5,
            "{what}: {key:?} scrolled the region: {}",
            scroll_y(&app, view)
        );
    }
}

#[test]
fn a_text_area_in_a_scroll_region_keeps_its_page_home_and_end() {
    keeps_its_keys(
        Role::TextArea,
        text_area("Notes", "one\ntwo\nthree", 4),
        "text area",
    );
}

#[test]
fn a_table_in_a_scroll_region_keeps_its_page_home_and_end() {
    let rows: Vec<Vec<String>> = (0..50).map(|i| vec![format!("Item {i}")]).collect();
    keeps_its_keys(Role::Table, table([Column::new("Name")], rows), "table");
}

#[test]
fn a_slider_in_a_scroll_region_keeps_its_home_and_end() {
    keeps_its_keys(Role::Slider, slider("Volume", 0.5, 0.0..=1.0), "slider");
}

#[test]
fn a_code_editor_in_a_scroll_region_keeps_its_page_home_and_end() {
    let text = (0..40).map(|i| format!("line {i}\n")).collect::<String>();
    keeps_its_keys(Role::CodeEditor, code_editor("Script", text), "code editor");
}

/// A palette's field has focus (set by hand: the palette is a modal card) inside a region: the Page
/// keys are the palette's, not the region's.
#[test]
fn a_palette_field_in_a_scroll_region_keeps_its_page_keys() {
    let (mut app, view) = region_with(command_palette());
    // An open palette puts focus in its own field; that field is inside the `PaletteField` row.
    let root = first_with_role(&app, Role::PaletteField);
    let field = bevy_gooey::testing::focused(&app).expect("the palette took focus");
    assert!(within(&app, field, root), "focus is in the palette's field");
    for key in [
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::End,
        KeyCode::Home,
    ] {
        bevy_gooey::testing::press(&mut app, key);
        let now = bevy_gooey::testing::focused(&app).expect("a focus");
        assert!(
            within(&app, now, root),
            "{key:?} took focus out of the palette field to {:?}",
            bevy_gooey::testing::focused_label(&app)
        );
        assert!(
            scroll_y(&app, view).abs() < 0.5,
            "{key:?} scrolled the region"
        );
    }
}

/// The control: a plain button row in the same kind of region still pages, and ends.
#[test]
fn a_plain_button_row_in_a_scroll_region_still_pages_and_ends() {
    let (mut app, view) = region_with(button("First"));
    bevy_gooey::testing::press(&mut app, KeyCode::Tab);
    bevy_gooey::testing::press(&mut app, KeyCode::PageDown);
    assert!(
        scroll_y(&app, view) > 0.5,
        "paged: {}",
        scroll_y(&app, view)
    );
    bevy_gooey::testing::press(&mut app, KeyCode::End);
    assert_eq!(
        bevy_gooey::testing::focused_label(&app).as_deref(),
        Some("Row 29")
    );
}

/// Two `Shown(false)` inserts in one command flush: the second must not record the first's
/// `Display::None` as the display to come back to.
#[test]
fn two_shown_false_inserts_in_one_flush_still_open_to_the_original_display() {
    let mut app = headless();
    let page = app
        .world_mut()
        .spawn((panel(), children![button("In")]))
        .id();
    let outside = app.world_mut().spawn(button("Outside")).id();
    app.world_mut()
        .spawn(screen())
        .add_child(page)
        .add_child(outside);
    settle(&mut app);
    {
        let mut commands = app.world_mut().commands();
        commands.entity(page).insert(Shown(false));
        commands.entity(page).insert(Shown(false));
    }
    app.world_mut().flush();
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::None
    );
    app.world_mut().entity_mut(page).insert(Shown(true));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(page).expect("a node").display,
        Display::Flex,
        "not flattened to None"
    );
    assert_eq!(
        app.world().get::<Visibility>(page),
        Some(&Visibility::Inherited)
    );
    assert!(rect(&app, page).height() > 1.0);
    assert!(
        tab_stops(&mut app, 3)
            .iter()
            .any(|s| s.as_deref() == Some("In"))
    );
}
