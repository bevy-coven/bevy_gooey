//! A tooltip opens for a keyboard or pad player as it does for the pointer, beside its widget and
//! clear of every other widget, and flips at the window's edge. Laid out for real in a headless
//! window.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{AutoFocus, InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const WIDTH: f32 = 1280.0;

/// A headless app with a real window size, so UI layout and popover placement run.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WIDTH as u32, 720)
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
    ));
    app.world_mut().spawn(Camera2d);
    app
}

/// Presses and releases `key` as the window's own keyboard input, one update each.
fn tap(app: &mut App, key_code: KeyCode) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
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

/// Moves the mouse to `at`, in window px, as the window's own pointer input.
fn point_at(app: &mut App, at: Vec2) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one primary window");
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
    for _ in 0..2 {
        app.update();
    }
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

fn tooltip_box(app: &mut App) -> Entity {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<TooltipBox>>()
        .single(world)
        .expect("one tooltip box")
}

fn shown(app: &mut App) -> bool {
    let tip = tooltip_box(app);
    app.world().get::<Node>(tip).expect("a node").display != Display::None
}

/// Where `entity` is drawn, in logical px (the window's scale factor is 1).
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

fn tip_rect(app: &mut App) -> Rect {
    let tip = tooltip_box(app);
    rect(app, tip)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    !a.intersect(b).is_empty()
}

/// Play, Options (with a tooltip), Quit in a column inside a panel, as a pause menu is.
fn column_menu(app: &mut App) -> [Entity; 3] {
    let world = app.world_mut();
    let play = world.spawn((button("Play"), AutoFocus)).id();
    let options = world
        .spawn((
            button("Options"),
            Tooltip("Sound, controls and display".into()),
        ))
        .id();
    let quit = world.spawn(button("Quit")).id();
    let column = world
        .spawn(menu())
        .add_children(&[play, options, quit])
        .id();
    let card = world.spawn(panel()).add_child(column).id();
    world.spawn(screen()).add_child(card);
    [play, options, quit]
}

#[test]
fn focus_opens_a_tooltip_beside_its_widget_clear_of_the_next() {
    let mut app = app();
    let [play, options, quit] = column_menu(&mut app);
    settle(&mut app);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(play));
    assert!(!shown(&mut app), "Play has focus: Options' tooltip is shut");

    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(options));
    app.update();
    assert!(
        shown(&mut app),
        "focus on Options opens its tooltip, no pointer needed"
    );

    let tip = tip_rect(&mut app);
    let widget = rect(&app, options);
    let skin = GooeySkin::default();
    for other in [play, quit] {
        assert!(
            !overlaps(tip, rect(&app, other)),
            "the tooltip covers a neighbour: {tip:?}"
        );
    }
    let gap = tip.min.x - widget.max.x;
    let outside_panel = skin.gap + skin.padding + skin.border;
    assert!(
        (gap - outside_panel).abs() < 1.0,
        "it opens to the right, one gap outside the panel: {gap} px, wanted {outside_panel}"
    );
    assert!(
        (tip.center().y - widget.center().y).abs() < 1.0,
        "it is centred on its widget"
    );

    tap(&mut app, KeyCode::ArrowDown);
    app.update();
    assert!(!shown(&mut app), "focus left Options: its tooltip shuts");

    tap(&mut app, KeyCode::ArrowUp);
    app.update();
    assert!(shown(&mut app));
    app.insert_resource(InputFocusVisible(false));
    app.update();
    assert!(
        !shown(&mut app),
        "a pointer player's focus is not visible, so focus alone does not open it"
    );
}

#[test]
fn the_pointer_opens_a_tooltip_without_visible_focus() {
    let mut app = app();
    let [_, options, _] = column_menu(&mut app);
    settle(&mut app);
    assert!(!shown(&mut app));

    let over = rect(&app, options).center();
    point_at(&mut app, over);
    assert!(shown(&mut app), "hovering Options opens its tooltip");
    point_at(&mut app, Vec2::new(5.0, 5.0));
    assert!(!shown(&mut app), "leaving it shuts the tooltip");
}

#[test]
fn a_tooltip_at_the_window_edge_opens_on_the_other_side() {
    let mut app = app();
    // A game that wants the ring from the first frame inserts this itself.
    app.insert_resource(InputFocusVisible(true));
    let world = app.world_mut();
    let widget = world
        .spawn((
            button("Map"),
            Tooltip("The world and your quests".into()),
            AutoFocus,
        ))
        .id();
    let root = world.spawn(screen()).id();
    world
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                top: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ChildOf(root),
        ))
        .add_child(widget);
    settle(&mut app);
    assert!(shown(&mut app));

    let tip = tip_rect(&mut app);
    let widget = rect(&app, widget);
    assert!(
        tip.max.x < widget.min.x,
        "no room on the right: it opens on the left ({tip:?} beside {widget:?})"
    );
    assert!(tip.min.x >= 0.0 && tip.max.x <= WIDTH, "inside the window");
}

#[test]
fn in_a_row_a_tooltip_opens_below_clear_of_its_neighbours() {
    let mut app = app();
    // A game that wants the ring from the first frame inserts this itself.
    app.insert_resource(InputFocusVisible(true));
    let world = app.world_mut();
    let left = world.spawn(button("Sword")).id();
    let middle = world
        .spawn((
            button("Shield"),
            Tooltip("Blocks the next hit".into()),
            AutoFocus,
        ))
        .id();
    let right = world.spawn(button("Potion")).id();
    let row = world
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .add_children(&[left, middle, right])
        .id();
    world.spawn(screen()).add_child(row);
    settle(&mut app);
    assert!(shown(&mut app));

    let tip = tip_rect(&mut app);
    let widget = rect(&app, middle);
    assert!(
        tip.min.y > widget.max.y,
        "in a row it opens below: {tip:?} under {widget:?}"
    );
    for other in [left, right] {
        assert!(!overlaps(tip, rect(&app, other)), "it covers a neighbour");
    }
}

/// Plugs in a gamepad, as the platform's own connection event, and returns it.
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

/// Presses and releases `button` on `pad` as the platform's raw input, one update each.
fn press(app: &mut App, pad: Entity, button: GamepadButton) {
    for value in [1.0, 0.0] {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
        app.update();
    }
}

fn focus_visible(app: &App) -> bool {
    app.world().resource::<InputFocusVisible>().0
}

#[test]
fn an_arrow_key_shows_focus_even_after_a_click() {
    let mut app = app();
    // InputFocusVisible starts false, and every pointer press sets it false again.
    let [_, options, _] = column_menu(&mut app);
    settle(&mut app);

    tap(&mut app, KeyCode::ArrowDown);
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(options));
    assert!(
        focus_visible(&app),
        "an arrow key is keyboard navigation: focus must show"
    );
    assert!(
        shown(&mut app),
        "focus from an arrow key opens Options' tooltip"
    );
    assert_eq!(
        app.world().get::<BorderColor>(options),
        Some(&BorderColor::all(GooeySkin::default().focus_ring))
    );
}

#[test]
fn the_d_pad_and_south_show_focus_and_a_press_with_it_shown_restyles_nothing() {
    let mut app = app();
    let [_, options, quit] = column_menu(&mut app);
    let pad = plug_in_pad(&mut app);
    settle(&mut app);

    press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(options));
    assert!(focus_visible(&app), "the D-pad shows focus");
    app.update();
    assert!(shown(&mut app), "focus from the D-pad opens Options' tooltip");

    // A pointer press hides focus; the pad's South button alone brings the ring back.
    app.insert_resource(InputFocusVisible(false));
    app.update();
    press(&mut app, pad, GamepadButton::South);
    assert!(focus_visible(&app), "South is pad input: focus must show");
    assert_eq!(
        app.world().get::<BorderColor>(options),
        Some(&BorderColor::all(GooeySkin::default().focus_ring))
    );

    // Focus already shows: the next press leaves the flag untouched, so the skin does not
    // restyle every widget on every press.
    let shown_since = app
        .world()
        .resource_ref::<InputFocusVisible>()
        .last_changed();
    press(&mut app, pad, GamepadButton::DPadDown);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(quit));
    assert_eq!(
        app.world()
            .resource_ref::<InputFocusVisible>()
            .last_changed(),
        shown_since
    );
}
