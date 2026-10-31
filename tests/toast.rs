//! A toast, as a game sees it: one spawn from anywhere, stood in one stack, kept for as long as its
//! message takes to read at the player's setting, faded in and out, capped, drawn inside the
//! window, and never in the way of the input under it. Laid out for real in a headless window on
//! a clock the test sets.

use std::collections::HashSet;
use std::time::Duration;

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{AutoFocus, InputFocus, InputFocusVisible};
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 0.1;

/// A headless app with a real window size, so UI layout and picking run, and a clock that
/// advances `FRAME` a frame.
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
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().spawn(Camera2d);
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// How many toasts stand.
fn standing(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<Toast>>()
        .iter(app.world())
        .count()
}

/// The toasts in their stack, oldest first.
fn toasts(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let Ok(stack) = world
        .query_filtered::<Entity, With<ToastStack>>()
        .single(world)
    else {
        return Vec::new();
    };
    world
        .get::<Children>(stack)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

fn the_stack(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<ToastStack>>()
        .single(world)
        .expect("one toast stack")
}

fn alpha(app: &App, toast: Entity) -> f32 {
    app.world().get::<Toast>(toast).expect("a toast").alpha()
}

/// A toast's outer rectangle in the window, logical px (the window's scale factor is 1):
/// `(left, top, right, bottom)`.
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

fn child_with_role(app: &mut App, parent: Entity, role: Role) -> Entity {
    let world = app.world_mut();
    let kids: Vec<Entity> = world
        .get::<Children>(parent)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    kids.into_iter()
        .find(|&k| world.get::<Role>(k) == Some(&role))
        .unwrap_or_else(|| panic!("no {role:?} under {parent}"))
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

fn point_at(app: &mut App, at: Vec2) {
    let window = window(app);
    let target = WindowRef::Entity(window)
        .normalize(None)
        .expect("a window ref");
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(target),
            position: at,
        },
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    frames(app, 2);
}

/// A column of three buttons on a screen, the first focused; the buttons, top to bottom.
fn menu_screen(app: &mut App) -> Vec<Entity> {
    app.world_mut().spawn((
        screen(),
        children![(
            panel(),
            children![(
                menu(),
                children![
                    (button("Play"), AutoFocus),
                    button("Options"),
                    button("Quit"),
                ]
            )]
        )],
    ));
    frames(app, 3);
    let world = app.world_mut();
    let mut buttons: Vec<Entity> = world
        .query::<(Entity, &Role)>()
        .iter(world)
        .filter(|(_, role)| **role == Role::Button)
        .map(|(e, _)| e)
        .collect();
    // Spawned in order, so entity order is top to bottom.
    buttons.sort_by_key(|e| e.index());
    buttons
}

#[test]
fn a_spawned_toast_joins_the_one_stack_in_the_same_flush() {
    let mut app = app();
    // Three toasts in one command buffer: they share one stack, in spawn order.
    let ids: Vec<Entity> = {
        let mut commands = app.world_mut().commands();
        ["First", "Second", "Third"]
            .into_iter()
            .map(|text| commands.spawn(toast(text)).id())
            .collect()
    };
    app.world_mut().flush();
    // Before any frame has drawn.
    let stacks = app
        .world_mut()
        .query_filtered::<Entity, With<ToastStack>>()
        .iter(app.world())
        .count();
    assert_eq!(stacks, 1);
    assert_eq!(toasts(&mut app), ids);
    // The stack is made again when a scene reset despawned it.
    let stack = the_stack(&mut app);
    app.world_mut().entity_mut(stack).despawn();
    app.world_mut().spawn(toast("Again"));
    app.world_mut().flush();
    assert_eq!(toasts(&mut app).len(), 1);
}

#[test]
fn a_toast_stands_as_long_as_its_message_takes_to_read_then_goes() {
    let mut app = app();
    // 10 characters at the defaults: 2 s to notice, half a second to read, a fade each end.
    app.world_mut().spawn(toast("Game saved"));
    frames(&mut app, 5);
    let only = toasts(&mut app)[0];
    assert_eq!(alpha(&app, only), 1.0, "faded in after half a second");
    frames(&mut app, 22);
    assert_eq!(standing(&mut app), 1, "still there 2.7 s in");
    frames(&mut app, 8);
    assert_eq!(
        standing(&mut app),
        0,
        "gone by 3.5 s: 2.5 s held and a fade each end"
    );
}

#[test]
fn a_longer_message_stands_longer() {
    let mut app = app();
    app.world_mut().spawn(toast("Game saved"));
    app.world_mut().spawn(toast("x".repeat(100)));
    frames(&mut app, 40);
    assert_eq!(standing(&mut app), 1, "the short one is gone at 4 s");
    frames(&mut app, 30);
    assert_eq!(
        standing(&mut app),
        1,
        "100 characters hold 7 s: still there at 7 s"
    );
    frames(&mut app, 10);
    assert_eq!(standing(&mut app), 0, "and gone by 8 s");
}

#[test]
fn the_players_scale_stretches_a_toast_already_standing() {
    let mut app = app();
    app.world_mut().spawn(toast("Game saved"));
    frames(&mut app, 20);
    app.world_mut()
        .resource_mut::<ToastSettings>()
        .duration_scale = 3.0;
    frames(&mut app, 40);
    assert_eq!(
        standing(&mut app),
        1,
        "6 s in, with 7.5 s to hold at three times"
    );
    frames(&mut app, 30);
    assert_eq!(standing(&mut app), 0);
}

#[test]
fn a_fourth_toast_sends_the_oldest_into_its_fade_at_once() {
    let mut app = app();
    for text in ["One", "Two", "Three"] {
        app.world_mut().spawn(toast(text));
    }
    frames(&mut app, 10);
    let [one, two, three] = toasts(&mut app)[..] else {
        panic!("three toasts stand");
    };
    assert_eq!(alpha(&app, one), 1.0);
    app.world_mut().spawn(toast("Four"));
    frames(&mut app, 3);
    assert!(alpha(&app, one) < 1.0, "the oldest is fading");
    for kept in [two, three] {
        assert_eq!(alpha(&app, kept), 1.0, "the others are untouched");
    }
    frames(&mut app, 5);
    assert!(app.world().get_entity(one).is_err(), "and it is gone");
    assert_eq!(standing(&mut app), 3);
}

#[test]
fn no_fade_means_no_rise_and_a_toast_that_appears_whole_and_goes() {
    let mut app = app();
    app.world_mut().resource_mut::<ToastSettings>().fade_seconds = 0.0;
    app.world_mut().spawn(toast("Game saved"));
    frames(&mut app, 2);
    let only = toasts(&mut app)[0];
    assert_eq!(alpha(&app, only), 1.0);
    assert_eq!(app.world().get::<Toast>(only).map(|t| t.lift()), Some(0.0));
    frames(&mut app, 25);
    assert_eq!(standing(&mut app), 0, "held 2.5 s, then gone");
}

#[test]
fn a_toast_fades_in_while_rising_into_place_from_the_edge_it_stands_at() {
    for (anchor, below) in [
        (ToastAnchor::BottomCenter, true),
        (ToastAnchor::TopRight, false),
    ] {
        let mut app = app();
        app.world_mut().resource_mut::<ToastSettings>().anchor = anchor;
        app.world_mut().spawn(toast("Game saved"));
        frames(&mut app, 2);
        let only = toasts(&mut app)[0];
        let moving = app
            .world()
            .get::<UiTransform>(only)
            .expect("moves")
            .translation;
        let Val::Px(y) = moving.y else {
            panic!("the rise is in px");
        };
        let gap = GooeySkin::default().gap;
        assert_eq!(y.signum(), if below { 1.0 } else { -1.0 }, "{anchor:?}");
        assert!(y.abs() <= gap && y.abs() > 0.0, "{anchor:?}: {y}");
        assert!(alpha(&app, only) > 0.0 && alpha(&app, only) < 1.0);
        frames(&mut app, 3);
        let settled = app
            .world()
            .get::<UiTransform>(only)
            .expect("moves")
            .translation;
        assert_eq!(settled, Val2::ZERO, "{anchor:?}: in place");
    }
}

#[test]
fn a_holding_toast_is_not_written_or_restyled() {
    let mut app = app();
    app.world_mut().spawn(toast("Game saved"));
    frames(&mut app, 8);
    let only = toasts(&mut app)[0];
    let changed = |app: &App| {
        app.world()
            .entity(only)
            .get_ref::<Toast>()
            .expect("a toast")
            .last_changed()
            .get()
    };
    let before = changed(&app);
    frames(&mut app, 10);
    assert_eq!(changed(&app), before, "ten holding frames wrote nothing");
    // And the mark and the text were left as they were.
    let text = child_with_role(&mut app, only, Role::ToastText);
    let seen = app
        .world()
        .entity(text)
        .get_ref::<TextColor>()
        .expect("coloured")
        .last_changed()
        .get();
    frames(&mut app, 5);
    let after = app
        .world()
        .entity(text)
        .get_ref::<TextColor>()
        .expect("coloured")
        .last_changed()
        .get();
    assert_eq!(seen, after);
}

#[test]
fn a_toast_takes_no_input_and_no_focus_and_the_pointer_goes_through_it() {
    let mut app = app();
    let buttons = menu_screen(&mut app);
    let options = buttons[1];
    // The game world under the interface, where the pointer must be able to land.
    let world_below = app
        .world_mut()
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(-1),
        ))
        .id();
    tap(&mut app, KeyCode::ArrowDown);
    let focus_before = app.world().resource::<InputFocus>().get();
    let visible_before = app.world().resource::<InputFocusVisible>().0;
    assert_eq!(focus_before, Some(options));

    app.world_mut().spawn(toast("Game saved"));
    app.world_mut().spawn(toast_alert("Inventory full"));
    frames(&mut app, 8);
    assert_eq!(app.world().resource::<InputFocus>().get(), focus_before);
    assert_eq!(
        app.world().resource::<InputFocusVisible>().0,
        visible_before
    );

    // Every node a toast has lets the pointer through, so a press at its middle lands below it.
    let toast_nodes: HashSet<Entity> = {
        let world = app.world_mut();
        world
            .query::<(Entity, &Role)>()
            .iter(world)
            .filter(|(_, role)| {
                matches!(
                    role,
                    Role::Toast | Role::ToastMark | Role::ToastText | Role::ToastStack
                )
            })
            .map(|(e, _)| e)
            .collect()
    };
    assert_eq!(
        toast_nodes.len(),
        7,
        "a stack, and a toast, mark and text twice"
    );
    for &node in &toast_nodes {
        assert_eq!(
            app.world().get::<Pickable>(node),
            Some(&Pickable::IGNORE),
            "{node}"
        );
    }
    let first = toasts(&mut app)[0];
    let (l, t, r, b) = rect(&app, first);
    point_at(&mut app, Vec2::new((l + r) / 2.0, (t + b) / 2.0));
    let hovered: HashSet<Entity> = app
        .world()
        .resource::<HoverMap>()
        .get(&PointerId::Mouse)
        .expect("the mouse has a hover set")
        .keys()
        .copied()
        .collect();
    assert!(
        hovered.contains(&world_below),
        "the world is under the pointer"
    );
    assert!(
        hovered.is_disjoint(&toast_nodes),
        "no toast is: {hovered:?}"
    );

    // Arrows and Tab move through the buttons as without a toast, and never onto one.
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(
        app.world()
            .resource::<InputFocus>()
            .get()
            .map(|e| buttons.contains(&e)),
        Some(true)
    );
    tap(&mut app, KeyCode::ArrowUp);
    tap(&mut app, KeyCode::Tab);
    let focused = app.world().resource::<InputFocus>().get();
    assert!(
        focused.is_some_and(|e| buttons.contains(&e)),
        "{focused:?} is not one of {buttons:?}"
    );
    // Through its whole life, to its going.
    frames(&mut app, 40);
    assert_eq!(standing(&mut app), 0);
    assert!(
        app.world()
            .resource::<InputFocus>()
            .get()
            .is_some_and(|e| buttons.contains(&e))
    );
}

#[test]
fn the_stack_stands_above_a_dialog_and_below_a_tooltip() {
    let mut app = app();
    let stack_z = {
        app.world_mut().spawn(toast("Game saved"));
        let stack = the_stack(&mut app);
        app.world()
            .get::<GlobalZIndex>(stack)
            .copied()
            .expect("layered")
    };
    let dialog_z = {
        let id = app.world_mut().spawn(dialog()).id();
        app.world()
            .get::<GlobalZIndex>(id)
            .copied()
            .expect("layered")
    };
    let tooltip_z = {
        app.world_mut()
            .spawn((button("Help"), Tooltip("Opens the manual".into())));
        frames(&mut app, 2);
        let world = app.world_mut();
        let tip = world
            .query_filtered::<Entity, With<TooltipBox>>()
            .single(world)
            .expect("one tooltip box");
        world.get::<GlobalZIndex>(tip).copied().expect("layered")
    };
    assert!(dialog_z.0 < stack_z.0, "{dialog_z:?} under {stack_z:?}");
    assert!(stack_z.0 < tooltip_z.0, "{stack_z:?} under {tooltip_z:?}");
}

#[test]
fn toasts_stand_inside_the_window_in_even_rows_newest_nearest_the_edge() {
    let skin = GooeySkin::default();
    let mut app = app();
    for text in ["Oldest", "Middle", "Newest"] {
        app.world_mut().spawn(toast(text));
        frames(&mut app, 5);
    }
    frames(&mut app, 2);
    let rows = toasts(&mut app);
    assert_eq!(rows.len(), 3);
    let rects: Vec<_> = rows.iter().map(|&e| rect(&app, e)).collect();
    let margin = skin.padding * 2.0;
    for (i, (left, top, right, bottom)) in rects.iter().enumerate() {
        assert!(
            (640.0 - (left + right) / 2.0).abs() <= 1.0,
            "row {i} is centred: {left}..{right}"
        );
        assert!(
            *top >= margin && *bottom <= 720.0 - margin + 1.0,
            "row {i}: {top}..{bottom}"
        );
    }
    // Bottom to top is newest to oldest, with the stack's own padding under the newest and one gap
    // between rows.
    assert!(
        (rects[2].3 - (720.0 - margin)).abs() <= 1.0,
        "newest {}",
        rects[2].3
    );
    for pair in rects.windows(2) {
        let gap = pair[1].1 - pair[0].3;
        assert!(
            (gap - skin.gap).abs() <= 1.0,
            "rows {gap} apart, wanted {}",
            skin.gap
        );
    }
    // A top-right stack hugs the other corner, and puts the newest at the top.
    app.world_mut().resource_mut::<ToastSettings>().anchor = ToastAnchor::TopRight;
    frames(&mut app, 3);
    let rects: Vec<_> = toasts(&mut app).iter().map(|&e| rect(&app, e)).collect();
    assert!(
        (rects[2].1 - margin).abs() <= 1.0,
        "newest at the top: {}",
        rects[2].1
    );
    for r in &rects {
        assert!((r.2 - (1280.0 - margin)).abs() <= 1.0, "right edge {}", r.2);
    }
    assert!(
        rects[0].1 > rects[2].1,
        "the oldest is furthest from the edge"
    );
}

#[test]
fn a_long_message_wraps_at_the_measure_and_stays_in_its_toast() {
    let skin = GooeySkin::default();
    let mut app = app();
    app.world_mut().spawn(toast("Saved"));
    app.world_mut().spawn(toast("lorem ipsum ".repeat(20)));
    frames(&mut app, 6);
    let rows = toasts(&mut app);
    let short = rect(&app, rows[0]);
    let long = rect(&app, rows[1]);
    let measure = skin.text_size * 24.0;
    assert!(
        long.2 - long.0 <= measure + 1.0,
        "{} wide, measure {measure}",
        long.2 - long.0
    );
    assert!(
        long.2 - long.0 >= measure * 0.8,
        "wrapped at the measure, not narrower"
    );
    assert!(
        long.3 - long.1 > 2.0 * (short.3 - short.1) - skin.padding,
        "several lines, not clipped to one: {} high against {}",
        long.3 - long.1,
        short.3 - short.1
    );
    // The text fits in its toast.
    let text = child_with_role(&mut app, rows[1], Role::ToastText);
    let text_box = rect(&app, text);
    assert!(
        text_box.2 <= long.2 + 0.5 && text_box.3 <= long.3 + 0.5,
        "{text_box:?} in {long:?}"
    );
}

#[test]
fn the_mark_says_the_kind_and_the_look_is_the_skins() {
    let skin = GooeySkin::default();
    let mut app = app();
    app.world_mut().spawn(toast("Game saved"));
    app.world_mut().spawn(toast_alert("Inventory full"));
    frames(&mut app, 8);
    let [info, alert] = toasts(&mut app)[..] else {
        panic!("two toasts");
    };
    assert_eq!(
        app.world().get::<Toast>(info).map(|t| t.kind()),
        Some(ToastKind::Info)
    );
    assert_eq!(
        app.world().get::<Toast>(alert).map(|t| t.kind()),
        Some(ToastKind::Alert)
    );
    for (toast, mark) in [(info, skin.accent), (alert, skin.danger)] {
        let world = app.world();
        assert_eq!(
            world.get::<BackgroundColor>(toast),
            Some(&BackgroundColor(skin.solid_surface()))
        );
        assert_eq!(
            world.get::<BorderColor>(toast),
            Some(&BorderColor::all(skin.surface_raised))
        );
        let mark_node = child_with_role(&mut app, toast, Role::ToastMark);
        let text = child_with_role(&mut app, toast, Role::ToastText);
        let world = app.world();
        assert_eq!(
            world.get::<BackgroundColor>(mark_node),
            Some(&BackgroundColor(mark))
        );
        assert_eq!(world.get::<TextColor>(text), Some(&TextColor(skin.text)));
    }
    // The mark is the toast's whole height and as wide as four lines, flush with its left edge.
    let mark_node = child_with_role(&mut app, info, Role::ToastMark);
    let (tl, tt, _, tb) = rect(&app, info);
    let (ml, mt, mr, mb) = rect(&app, mark_node);
    assert!(
        (mr - ml - 4.0 * skin.border).abs() <= 0.5,
        "{} wide",
        mr - ml
    );
    assert!((ml - (tl + skin.border)).abs() <= 0.5);
    assert!((mt - (tt + skin.border)).abs() <= 0.5 && (mb - (tb - skin.border)).abs() <= 0.5);
}

#[test]
fn a_toast_fades_all_its_parts_together_and_the_skin_change_keeps_the_fade() {
    let skin = GooeySkin::default();
    let mut app = app();
    app.world_mut().spawn(toast("Game saved"));
    frames(&mut app, 2);
    let only = toasts(&mut app)[0];
    let half = alpha(&app, only);
    assert!(half > 0.0 && half < 1.0);
    let text = child_with_role(&mut app, only, Role::ToastText);
    let mark = child_with_role(&mut app, only, Role::ToastMark);
    let opacity = |app: &App| {
        let alpha_of = |c: Color| Srgba::from(c).alpha;
        (
            alpha_of(app.world().get::<BackgroundColor>(only).expect("fill").0),
            alpha_of(app.world().get::<BackgroundColor>(mark).expect("mark").0),
            alpha_of(app.world().get::<TextColor>(text).expect("text").0),
        )
    };
    let (fill, mark_alpha, text_alpha) = opacity(&app);
    for a in [fill, mark_alpha, text_alpha] {
        assert!((a - half).abs() < 0.01, "{a} is not the toast's {half}");
    }
    // Changing the skin in the middle of a fade restyles at the same opacity, not at full.
    app.world_mut().insert_resource(GooeySkin::parchment());
    app.update();
    let now = alpha(&app, only);
    let (fill, mark_alpha, text_alpha) = opacity(&app);
    for a in [fill, mark_alpha, text_alpha] {
        assert!((a - now).abs() < 0.01, "{a} is not the toast's {now}");
    }
    // Held, then, it is the new skin whole.
    frames(&mut app, 6);
    let parchment = GooeySkin::parchment();
    assert_eq!(
        app.world().get::<BackgroundColor>(only),
        Some(&BackgroundColor(parchment.solid_surface()))
    );
    assert_ne!(parchment.solid_surface(), skin.solid_surface());
}

#[test]
fn the_toast_marks_are_checked_on_the_surface_they_stand_on() {
    assert_eq!(GooeySkin::dark().problems(), vec![]);
    assert_eq!(GooeySkin::parchment().problems(), vec![]);
    let skin = GooeySkin::default();
    let problems = GooeySkin {
        accent: skin.solid_surface(),
        danger: skin.solid_surface(),
        ..default()
    }
    .problems();
    for pair in ["accent on surface", "danger on surface"] {
        assert!(
            problems
                .iter()
                .any(|p| matches!(p, SkinProblem::LowContrast { pair: q, .. } if *q == pair)),
            "{pair} not reported: {problems:?}"
        );
    }
}
