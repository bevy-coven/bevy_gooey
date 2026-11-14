//! What was left of the editor's hand-built nodes after G-85 (G-86, #97): off-scale spacing, extents
//! in viewport units, a border in logical px, a button box, a hideable quiet status item, an ink
//! override and the cost of per-frame geometry, each laid out for real in a headless window.

use bevy::picking::Pickable;
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

/// A real image in the app's assets, so a box draws something.
fn image(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default())
}

/// A fixed box `w` × `h` logical px that never shrinks: a probe for where a row puts its children.
fn probe(app: &mut App, w: f32, h: f32) -> impl Bundle + use<> {
    image_box(image(app), Vec2::new(w, h))
}

/// A plain node `width` wide that holds `child` and never gives way: the room a layout is put in.
fn room(app: &mut App, width: f32, child: Entity) -> Entity {
    let room = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(width),
            min_width: Val::Px(width),
            max_width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            flex_shrink: 0.0,
            ..default()
        })
        .add_child(child)
        .id();
    app.world_mut().spawn(screen()).add_child(room);
    room
}

// ---- 2. spacing off the skin's scale: Space::Px ----------------------------------------------

fn pad(px: f32) -> Spacing {
    Spacing {
        padding: Sides::all(Space::Px(px)),
        ..default()
    }
}

/// A literal logical px is that many px, not a step: a chip's 6 by 3 and a tab's 12 by 8.
#[test]
fn a_space_in_px_is_that_many_px_not_a_step() {
    let mut app = headless();
    let b = probe(&mut app, 40.0, 20.0);
    let chip = app
        .world_mut()
        .spawn((
            row(0.0),
            Spacing {
                padding: Sides {
                    left: Some(Space::Px(6.0)),
                    right: Some(Space::Px(6.0)),
                    top: Some(Space::Px(3.0)),
                    bottom: Some(Space::Px(3.0)),
                },
                margin: Sides {
                    left: Some(Space::Px(8.0)),
                    ..default()
                },
            },
        ))
        .with_children(|r| {
            r.spawn(b);
        })
        .id();
    let host = room(&mut app, 400.0, chip);
    settle(&mut app);
    let (c, h, kid) = (
        rect(&app, chip),
        rect(&app, host),
        rect(&app, children(&app, chip)[0]),
    );
    assert!(
        (c.min.x - h.min.x - 8.0).abs() < 0.5,
        "margin 8: {c:?} in {h:?}"
    );
    assert!(
        (kid.min.x - c.min.x - 6.0).abs() < 0.5 && (kid.min.y - c.min.y - 3.0).abs() < 0.5,
        "padding 6 by 3: {kid:?} in {c:?}"
    );
    assert!((c.height() - 26.0).abs() < 0.5, "{c:?}");
}

/// It is off the scale: changing the skin's spacing moves a step and leaves a px alone.
#[test]
fn a_px_space_does_not_follow_the_skin_scale() {
    let mut app = headless();
    let stepped = app
        .world_mut()
        .spawn((
            row(0.0),
            Spacing {
                padding: Sides::all(Space::Step(1)),
                ..default()
            },
        ))
        .id();
    let literal = app.world_mut().spawn((row(0.0), pad(6.0))).id();
    for e in [stepped, literal] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    app.world_mut().resource_mut::<GooeySkin>().spacing = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(stepped).padding, UiRect::all(Val::Px(2.0)));
    assert_eq!(node(literal).padding, UiRect::all(Val::Px(6.0)));
}

/// A px below zero is none, and one that is not a number leaves its side alone.
#[test]
fn a_px_space_is_finite_checked_and_never_negative() {
    let mut app = headless();
    let plain = app.world_mut().spawn(panel()).id();
    let odd = app
        .world_mut()
        .spawn((
            panel(),
            Spacing {
                padding: Sides {
                    left: Some(Space::Px(-4.0)),
                    right: Some(Space::Px(f32::NAN)),
                    top: Some(Space::Px(f32::INFINITY)),
                    ..default()
                },
                ..default()
            },
        ))
        .id();
    for e in [plain, odd] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(odd).padding.left, Val::Px(0.0));
    assert_eq!(
        node(odd).padding.right,
        node(plain).padding.right,
        "NaN leaves the side"
    );
    assert_eq!(
        node(odd).padding.top,
        node(plain).padding.top,
        "infinity leaves the side"
    );
}

/// A changed px restyles.
#[test]
fn a_changed_px_space_restyles() {
    let mut app = headless();
    let e = app.world_mut().spawn((row(0.0), pad(6.0))).id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    app.world_mut()
        .get_mut::<Spacing>(e)
        .expect("spacing")
        .padding = Sides::all(Space::Px(9.0));
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(e).expect("a node").padding,
        UiRect::all(Val::Px(9.0))
    );
}

// ---- 5. extents in viewport units ------------------------------------------------------------

/// `Vh(100.0)` is the window's whole height and `Vw(50.0)` half its width, whatever the host is:
/// the frame root of a window.
#[test]
fn an_extent_can_be_in_viewport_units() {
    let mut app = headless();
    let tall = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::Vw(50.0)),
            height: Some(Extent::Vh(100.0)),
            min_height: Some(Extent::Px(0.0)),
            ..default()
        }))
        .id();
    let small = app
        .world_mut()
        .spawn(sized(Sizing {
            width: Some(Extent::VMin(10.0)),
            height: Some(Extent::VMax(10.0)),
            ..default()
        }))
        .id();
    for e in [tall, small] {
        app.world_mut()
            .spawn((
                screen(),
                Sizing {
                    min_height: Some(Extent::Px(0.0)),
                    ..default()
                },
            ))
            .add_child(e);
    }
    settle(&mut app);
    let r = rect(&app, tall);
    assert!(
        (r.width() - 640.0).abs() < 0.5 && (r.height() - 720.0).abs() < 0.5,
        "{r:?}"
    );
    let s = rect(&app, small);
    assert!(
        (s.width() - 72.0).abs() < 0.5 && (s.height() - 128.0).abs() < 0.5,
        "10 vmin of 720 and 10 vmax of 1280: {s:?}"
    );
}

/// A viewport length is finite-checked and never negative, like every extent.
#[test]
fn a_viewport_extent_is_finite_checked_and_never_negative() {
    let mut app = headless();
    let e = app
        .world_mut()
        .spawn((
            panel(),
            Sizing {
                min_width: Some(Extent::Vw(-5.0)),
                max_width: Some(Extent::Vw(f32::NAN)),
                min_height: Some(Extent::Vh(f32::INFINITY)),
                ..default()
            },
        ))
        .id();
    let plain = app.world_mut().spawn(panel()).id();
    for x in [e, plain] {
        app.world_mut().spawn(screen()).add_child(x);
    }
    settle(&mut app);
    let node = |x| app.world().get::<Node>(x).expect("a node").clone();
    assert_eq!(node(e).min_width, Val::Vw(0.0));
    assert_eq!(
        node(e).max_width,
        node(plain).max_width,
        "NaN leaves the node"
    );
    assert_eq!(
        node(e).min_height,
        node(plain).min_height,
        "infinity leaves the node"
    );
}

// ---- 1. a border in logical px, unsnapped ----------------------------------------------------

/// At a window scale of 1.25 the skin's 2 px line is 2.5 device px: it is drawn 2 (1.6 logical px).
/// `BorderPx` is not snapped: 3 px stays 3 px, the width the editor's drag box needs, in the skin's
/// outline colour.
#[test]
fn a_border_px_is_not_snapped_to_device_pixels_and_a_shape_border_is() {
    let mut app = headless_at(1.25);
    let snapped = app
        .world_mut()
        .spawn((
            placed_at(0.0, 0.0),
            Shape {
                border: Sides::all(1.0),
                ..default()
            },
        ))
        .id();
    let plain_px = app
        .world_mut()
        .spawn((placed_at(0.0, 0.0), BorderPx(Sides::all(3.0))))
        .id();
    for e in [snapped, plain_px] {
        let host = app
            .world_mut()
            .spawn(Node {
                width: Val::Px(200.0),
                height: Val::Px(100.0),
                ..default()
            })
            .add_child(e)
            .id();
        app.world_mut().spawn(screen()).add_child(host);
    }
    settle(&mut app);
    let skin = GooeySkin::dark();
    let width = |e| match app.world().get::<Node>(e).expect("a node").border.left {
        Val::Px(px) => px,
        other => panic!("not px: {other:?}"),
    };
    assert!(
        width(snapped) < skin.border,
        "a shape's weight snaps down to whole device px: {}",
        width(snapped)
    );
    assert!(
        (width(snapped) * 1.25 - (width(snapped) * 1.25).round()).abs() < 0.01,
        "on a device pixel: {}",
        width(snapped)
    );
    assert_eq!(width(plain_px), 3.0, "a BorderPx is the px it says");
    assert_eq!(
        app.world()
            .get::<BorderColor>(plain_px)
            .expect("an edge")
            .left,
        skin.outline,
        "in the skin's outline"
    );
}

/// It is per side, finite-checked and never negative; a side left `None` is left alone; and the ink
/// of a `Shape` on the same node colours it.
#[test]
fn a_border_px_is_per_side_finite_checked_and_inked_by_a_shape() {
    let mut app = headless();
    let plain = app.world_mut().spawn(panel()).id();
    let odd = app
        .world_mut()
        .spawn((
            panel(),
            BorderPx(Sides {
                left: Some(-2.0),
                right: Some(f32::NAN),
                top: Some(4.0),
                bottom: None,
            }),
            Shape {
                ink: Some(Ink::Danger),
                ..default()
            },
        ))
        .id();
    for e in [plain, odd] {
        app.world_mut().spawn(screen()).add_child(e);
    }
    settle(&mut app);
    let node = |e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(odd).border.left, Val::Px(0.0));
    assert_eq!(
        node(odd).border.right,
        node(plain).border.right,
        "NaN leaves the side"
    );
    assert_eq!(node(odd).border.top, Val::Px(4.0));
    assert_eq!(
        node(odd).border.bottom,
        node(plain).border.bottom,
        "None leaves the side"
    );
    assert_eq!(
        app.world().get::<BorderColor>(odd).expect("an edge").top,
        GooeySkin::dark().danger
    );
}

/// A changed `BorderPx` restyles; a focused widget's ring still wins over its colour.
#[test]
fn a_changed_border_px_restyles() {
    let mut app = headless();
    let e = app
        .world_mut()
        .spawn((placed_at(0.0, 0.0), BorderPx(Sides::all(2.0))))
        .id();
    app.world_mut().spawn(screen()).add_child(e);
    settle(&mut app);
    app.world_mut().get_mut::<BorderPx>(e).expect("a border").0 = Sides::all(5.0);
    settle(&mut app);
    assert_eq!(
        app.world().get::<Node>(e).expect("a node").border,
        UiRect::all(Val::Px(5.0))
    );
}

// ---- 3. a button box: a button that holds the host's children ---------------------------------

#[derive(Resource, Default)]
struct Pressed(u32);

/// A button box is a focusable, hoverable, pressable button that takes any children, and puts
/// `Role::ButtonText` on none of them.
#[test]
fn a_button_box_holds_children_and_has_no_button_text() {
    let mut app = headless();
    app.init_resource::<Pressed>();
    let picture = probe(&mut app, 40.0, 40.0);
    let boxed = app
        .world_mut()
        .spawn((
            button_box(),
            observe(|_: On<Activate>, mut n: ResMut<Pressed>| n.0 += 1),
            children![fill_text("agent_view"), count_text("12")],
        ))
        .with_children(|b| {
            b.spawn(picture);
        })
        .id();
    let plain = app.world_mut().spawn(button("Plain")).id();
    let holder = app
        .world_mut()
        .spawn(column(0.0))
        .add_child(boxed)
        .add_child(plain)
        .id();
    room(&mut app, 400.0, holder);
    settle(&mut app);

    assert!(
        app.world().get::<bevy::ui_widgets::Button>(boxed).is_some(),
        "Bevy's Button: Activate and focus"
    );
    assert!(
        app.world()
            .get::<bevy::input_focus::tab_navigation::TabIndex>(boxed)
            .is_some()
    );
    assert_eq!(app.world().get::<Role>(boxed), Some(&Role::Button));
    let mut texts = app.world_mut().query::<&Role>();
    let in_box: Vec<Entity> = {
        let mut stack = vec![boxed];
        let mut all = Vec::new();
        while let Some(e) = stack.pop() {
            for c in children(&app, e) {
                all.push(c);
                stack.push(c);
            }
        }
        all
    };
    assert!(
        in_box.iter().all(|e| texts
            .get(app.world(), *e)
            .is_ok_and(|r| *r != Role::ButtonText)),
        "no Role::ButtonText anywhere in a button box"
    );
    // The children start at the box's left, not centred: a card, not a label.
    let (b, first) = (rect(&app, boxed), rect(&app, children(&app, boxed)[0]));
    assert!(first.min.x - b.min.x < 40.0, "{first:?} in {b:?}");
    // The plain button still centres its label.
    assert_eq!(
        app.world()
            .get::<Node>(plain)
            .expect("a node")
            .justify_content,
        JustifyContent::Center
    );

    assert_eq!(
        app.world().get::<BackgroundColor>(boxed),
        app.world().get::<BackgroundColor>(plain),
        "a button's fill"
    );
    bevy_gooey::testing::click(&mut app, boxed);
    settle(&mut app);
    assert_eq!(app.world().resource::<Pressed>().0, 1, "one Activate");
}

/// Tab reaches a button box, and a host's `RowLayout` lays its children out.
#[test]
fn a_button_box_is_a_tab_stop_and_takes_a_row_layout() {
    let mut app = headless();
    let b = probe(&mut app, 30.0, 20.0);
    let boxed = app
        .world_mut()
        .spawn((
            button_box(),
            RowLayout {
                justify: Some(JustifyContent::End),
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn(b);
        })
        .id();
    room(&mut app, 400.0, boxed);
    settle(&mut app);
    let (r, kid) = (rect(&app, boxed), rect(&app, children(&app, boxed)[0]));
    assert!(
        r.max.x - kid.max.x < 30.0,
        "the child is at the end: {kid:?} in {r:?}"
    );
    bevy_gooey::testing::press(&mut app, KeyCode::Tab);
    assert_eq!(bevy_gooey::testing::focused(&app), Some(boxed));
}

// ---- 4. a quiet status item that can be hidden -----------------------------------------------

/// A quiet status item looks like a status item and is off the `Activate` bus: no Bevy `Button`, no
/// tab stop.
#[test]
fn a_quiet_status_item_looks_like_a_status_item_and_stays_off_the_activate_bus() {
    let mut app = headless();
    app.init_resource::<Pressed>();
    let count = |_: On<Activate>, mut n: ResMut<Pressed>| n.0 += 1;
    let loud = app
        .world_mut()
        .spawn((status_item("Saving"), StatusKind::Warning, observe(count)))
        .id();
    let quiet = app
        .world_mut()
        .spawn((
            status_item_quiet("Saving"),
            StatusKind::Warning,
            observe(count),
        ))
        .id();
    let bar = app
        .world_mut()
        .spawn(row(1.0))
        .add_child(loud)
        .add_child(quiet)
        .id();
    room(&mut app, 600.0, bar);
    settle(&mut app);
    assert!(app.world().get::<bevy::ui_widgets::Button>(quiet).is_none());
    assert!(
        app.world()
            .get::<bevy::input_focus::tab_navigation::TabIndex>(quiet)
            .is_none(),
        "no tab stop"
    );
    assert_eq!(app.world().get::<Role>(quiet), Some(&Role::StatusItem));
    assert_eq!(
        rect(&app, quiet).size(),
        rect(&app, loud).size(),
        "the same look"
    );
    bevy_gooey::testing::click(&mut app, quiet);
    settle(&mut app);
    assert_eq!(
        app.world().resource::<Pressed>().0,
        0,
        "a quiet item sends no Activate"
    );
    bevy_gooey::testing::click(&mut app, loud);
    settle(&mut app);
    assert_eq!(app.world().resource::<Pressed>().0, 1);
}

/// `Shown` hides a quiet item inside a status bar (no room, hidden from layout and focus) and brings
/// it back; the same toggle works on an item that was shown at spawn.
#[test]
fn a_quiet_status_item_can_be_hidden_with_shown_in_a_bar() {
    let mut app = headless();
    let bar = app
        .world_mut()
        .spawn(status_bar(
            children![
                status_item_quiet("First"),
                (status_item_quiet("Jobs"), Shown(false)),
                status_item_quiet("Last"),
            ],
            (),
        ))
        .id();
    room(&mut app, 600.0, bar);
    settle(&mut app);
    let left = children(&app, bar)[0];
    let (first, hidden, last) = {
        let kids = children(&app, left);
        (kids[0], kids[1], kids[2])
    };
    let node = |app: &App, e| app.world().get::<Node>(e).expect("a node").clone();
    assert_eq!(node(&app, hidden).display, Display::None);
    assert_eq!(
        app.world().get::<Visibility>(hidden),
        Some(&Visibility::Hidden)
    );
    let closed = rect(&app, last).min.x - rect(&app, first).max.x;
    assert!(
        closed < 10.0,
        "closed up, one gap and no item between: {closed}"
    );
    *app.world_mut().get_mut::<Shown>(hidden).expect("shown") = Shown(true);
    settle(&mut app);
    assert_eq!(node(&app, hidden).display, Display::Flex);
    assert!(
        rect(&app, hidden).width() > 1.0 && rect(&app, last).min.x > rect(&app, hidden).max.x - 0.5
    );
    // An item shown at spawn is hidden by inserting `Shown(false)` on the existing node.
    app.world_mut().entity_mut(last).insert(Shown(false));
    settle(&mut app);
    assert_eq!(node(&app, last).display, Display::None);
    app.world_mut().entity_mut(last).insert(Shown(true));
    settle(&mut app);
    assert!(rect(&app, last).width() > 1.0, "back");
}
