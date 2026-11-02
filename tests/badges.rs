//! The shortcut overlay: hold a modifier and every shortcut in view shows itself on its widget.
//! Laid out for real in a headless window, driven by real `KeyboardInput` on a clock the test sets.

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::model::contrast_ratio;
use bevy_gooey::*;

const FRAME_MS: u64 = 100;

fn app_with(skin: GooeySkin, badges: ShortcutBadges) -> App {
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
    .insert_resource(skin)
    .insert_resource(badges)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        FRAME_MS,
    )));
    app.world_mut().spawn(Camera2d);
    app
}

fn instant() -> ShortcutBadges {
    ShortcutBadges {
        hold: Modifiers::ALT,
        delay_seconds: 0.0,
    }
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// The widgets of a toolbar as a game builds them.
struct Tools {
    save: Entity,
    open: Entity,
    plain: Entity,
    off: Entity,
}

fn tools(app: &mut App) -> Tools {
    let w = app.world_mut();
    let save = w
        .spawn((button("Save"), Shortcut::primary(KeyCode::KeyS)))
        .id();
    let open = w
        .spawn((
            button("Open"),
            Tooltip::new("Open a file").with_shortcut(Shortcut::primary(KeyCode::KeyO)),
        ))
        .id();
    let plain = w.spawn(button("Plain")).id();
    let off = w
        .spawn((
            button("Export"),
            Shortcut::new(Modifiers::CTRL | Modifiers::SHIFT, KeyCode::KeyE),
            InteractionDisabled,
        ))
        .id();
    let card = w
        .spawn(panel())
        .add_children(&[save, open, plain, off])
        .id();
    w.spawn(screen()).add_child(card);
    settle(app);
    Tools {
        save,
        open,
        plain,
        off,
    }
}

fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(app.world())
        .expect("one window");
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

fn down(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Pressed);
}

fn up(app: &mut App, key_code: KeyCode) {
    key(app, key_code, ButtonState::Released);
}

/// Every badge: the widget it stands on and the words it shows.
fn shown(app: &mut App) -> Vec<(Entity, String)> {
    let world = app.world_mut();
    let mut found: Vec<(Entity, String)> = world
        .query_filtered::<(&ChildOf, &Text), With<ShortcutBadge>>()
        .iter(world)
        .map(|(parent, text)| (parent.parent(), text.0.clone()))
        .collect();
    found.sort();
    found
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

#[test]
fn holding_the_modifier_shows_one_badge_per_widget_with_a_shortcut_and_releasing_removes_them() {
    let mut app = app_with(GooeySkin::dark(), instant());
    let t = tools(&mut app);
    assert!(shown(&mut app).is_empty(), "nothing before the hold");

    down(&mut app, KeyCode::AltLeft);
    app.update();
    let mut want = vec![
        (t.save, Shortcut::primary(KeyCode::KeyS).label()),
        (t.open, Shortcut::primary(KeyCode::KeyO).label()),
    ];
    want.sort();
    assert_eq!(shown(&mut app), want, "the component and the tooltip's chord; not the plain or the disabled one");
    assert!(
        shown(&mut app).iter().all(|(w, _)| *w != t.plain && *w != t.off),
        "a widget with no shortcut and a disabled one show none"
    );

    up(&mut app, KeyCode::AltLeft);
    assert!(shown(&mut app).is_empty(), "released: gone in the same frame");
    // And again: the overlay is a mode the player holds.
    down(&mut app, KeyCode::AltRight);
    app.update();
    assert_eq!(shown(&mut app).len(), 2, "either side of the keyboard");
}

#[test]
fn the_badge_waits_for_the_delay_and_a_quick_tap_never_shows_it() {
    let mut app = app_with(
        GooeySkin::dark(),
        ShortcutBadges {
            hold: Modifiers::ALT,
            delay_seconds: 0.35,
        },
    );
    tools(&mut app);
    down(&mut app, KeyCode::AltLeft);
    app.update();
    assert!(shown(&mut app).is_empty(), "0.2 s held");
    app.update();
    assert!(shown(&mut app).is_empty(), "0.3 s held");
    app.update();
    assert_eq!(shown(&mut app).len(), 2, "0.4 s held");
    up(&mut app, KeyCode::AltLeft);
    assert!(shown(&mut app).is_empty());

    // A tap shorter than the delay, then the same again: the clock starts over each hold.
    down(&mut app, KeyCode::AltLeft);
    app.update();
    up(&mut app, KeyCode::AltLeft);
    for _ in 0..4 {
        app.update();
    }
    assert!(shown(&mut app).is_empty());
    down(&mut app, KeyCode::AltLeft);
    app.update();
    assert!(shown(&mut app).is_empty(), "the earlier tap does not count");
}

#[test]
fn only_exactly_the_configured_modifiers_reveal_them() {
    let mut app = app_with(GooeySkin::dark(), instant());
    tools(&mut app);
    // Another modifier alone, and Alt with another one, is not the hold.
    down(&mut app, KeyCode::ControlLeft);
    app.update();
    assert!(shown(&mut app).is_empty(), "Ctrl is not Alt");
    down(&mut app, KeyCode::AltLeft);
    app.update();
    assert!(shown(&mut app).is_empty(), "Ctrl+Alt is not Alt");
    up(&mut app, KeyCode::ControlLeft);
    app.update();
    assert_eq!(shown(&mut app).len(), 2, "Alt alone again");
    up(&mut app, KeyCode::AltLeft);

    // A chord of two.
    app.insert_resource(ShortcutBadges {
        hold: Modifiers::CTRL | Modifiers::SHIFT,
        delay_seconds: 0.0,
    });
    down(&mut app, KeyCode::ControlLeft);
    app.update();
    assert!(shown(&mut app).is_empty(), "Ctrl alone is not Ctrl+Shift");
    down(&mut app, KeyCode::ShiftRight);
    app.update();
    assert_eq!(shown(&mut app).len(), 2);
    up(&mut app, KeyCode::ControlLeft);
    up(&mut app, KeyCode::ShiftRight);

    // None configured: the overlay is off.
    app.insert_resource(ShortcutBadges {
        hold: Modifiers::NONE,
        delay_seconds: 0.0,
    });
    down(&mut app, KeyCode::AltLeft);
    app.update();
    assert!(shown(&mut app).is_empty());
}

#[test]
fn a_hidden_a_folded_away_and_an_off_screen_widget_show_none() {
    let mut app = app_with(GooeySkin::dark(), instant());
    let t = tools(&mut app);
    let w = app.world_mut();
    let hidden = w
        .spawn((
            button("Hidden"),
            Shortcut::primary(KeyCode::KeyH),
            Visibility::Hidden,
        ))
        .id();
    let folded = w
        .spawn((
            Node {
                display: Display::None,
                ..default()
            },
            Role::Button,
            Shortcut::primary(KeyCode::KeyF),
        ))
        .id();
    let away = w
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-900.0),
                top: Val::Px(100.0),
                width: Val::Px(100.0),
                height: Val::Px(50.0),
                ..default()
            },
            Role::Button,
            Shortcut::primary(KeyCode::KeyA),
        ))
        .id();
    let card = w
        .spawn(panel())
        .add_children(&[hidden, folded, away])
        .id();
    w.spawn(screen()).add_child(card);
    settle(&mut app);

    down(&mut app, KeyCode::AltLeft);
    app.update();
    let on: Vec<Entity> = shown(&mut app).into_iter().map(|(w, _)| w).collect();
    assert!(on.contains(&t.save) && on.contains(&t.open));
    for (name, e) in [("hidden", hidden), ("folded", folded), ("off screen", away)] {
        assert!(!on.contains(&e), "{name} shows no badge");
    }
}

#[test]
fn the_component_wins_over_a_tooltips_chord_and_a_change_edits_the_badge_in_place() {
    let mut app = app_with(GooeySkin::dark(), instant());
    let w = app.world_mut();
    let both = w
        .spawn((
            button("Both"),
            Shortcut::new(Modifiers::NONE, KeyCode::F5),
            Tooltip::new("Run").with_shortcut(Shortcut::primary(KeyCode::KeyR)),
        ))
        .id();
    let card = w.spawn(panel()).add_child(both).id();
    w.spawn(screen()).add_child(card);
    settle(&mut app);

    down(&mut app, KeyCode::AltLeft);
    app.update();
    assert_eq!(shown(&mut app), [(both, "F5".to_owned())]);
    let badge = app
        .world_mut()
        .query_filtered::<Entity, With<ShortcutBadge>>()
        .single(app.world())
        .expect("one badge");

    app.world_mut()
        .entity_mut(both)
        .insert(Shortcut::new(Modifiers::NONE, KeyCode::F9));
    app.update();
    assert_eq!(shown(&mut app), [(both, "F9".to_owned())]);
    let again = app
        .world_mut()
        .query_filtered::<Entity, With<ShortcutBadge>>()
        .single(app.world())
        .expect("one badge");
    assert_eq!(badge, again, "the words changed on the same entity");
    // The component goes: the tooltip's chord is shown.
    app.world_mut().entity_mut(both).remove::<Shortcut>();
    app.update();
    assert_eq!(
        shown(&mut app),
        [(both, Shortcut::primary(KeyCode::KeyR).label())]
    );
}

#[test]
fn a_badge_stays_inside_its_widgets_frame_so_the_focus_ring_is_whole() {
    for preset in SkinPreset::ALL {
        for text_scale in [1.0_f32, 1.5, 2.0] {
            let skin = GooeySkin {
                text_scale,
                ..preset.skin()
            };
            let mut app = app_with(skin, instant());
            let t = tools(&mut app);
            // The focused widget wears its ring while the overlay is up.
            app.insert_resource(InputFocusVisible(true));
            app.world_mut().resource_mut::<InputFocus>().set(t.save, FocusCause::Navigated);
            down(&mut app, KeyCode::AltLeft);
            app.update();
            app.update();

            let badges: Vec<(Entity, Entity)> = {
                let w = app.world_mut();
                w.query_filtered::<(Entity, &ChildOf), With<ShortcutBadge>>()
                    .iter(w)
                    .map(|(b, p)| (b, p.parent()))
                    .collect()
            };
            assert_eq!(badges.len(), 2, "{} at {text_scale}", preset.name());
            for (badge, widget) in badges {
                let frame = app.world().get::<ComputedNode>(widget).expect("laid out").border();
                let outer = rect(&app, widget);
                let inner = Rect {
                    min: outer.min + frame.min_inset,
                    max: outer.max - frame.max_inset,
                };
                let at = rect(&app, badge);
                let eps = 0.75;
                assert!(
                    at.min.x >= inner.min.x - eps
                        && at.max.x <= inner.max.x + eps
                        && at.min.y >= inner.min.y - eps
                        && at.max.y <= inner.max.y + eps,
                    "{} at {text_scale}: badge {at:?} leaves the widget's frame {inner:?}",
                    preset.name()
                );
                // On the right half: it is a corner badge, not over the centred label.
                assert!(at.center().x > outer.center().x);
            }
        }
    }
}

#[test]
fn the_badges_words_read_at_4_5_to_1_on_their_fill_in_every_look() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app_with(skin.clone(), instant());
        tools(&mut app);
        down(&mut app, KeyCode::AltLeft);
        app.update();
        app.update();
        let w = app.world_mut();
        let rows: Vec<(Color, Color)> = w
            .query_filtered::<(&BackgroundColor, &TextColor), With<ShortcutBadge>>()
            .iter(w)
            .map(|(b, t)| (b.0, t.0))
            .collect();
        assert_eq!(rows.len(), 2);
        for (fill, ink) in rows {
            let (fill, ink) = (Srgba::from(fill), Srgba::from(ink));
            let ratio = contrast_ratio(
                [ink.red, ink.green, ink.blue],
                [fill.red, fill.green, fill.blue],
            );
            assert!(ratio >= 4.5, "{}: {ratio}", preset.name());
        }
    }
}

#[test]
fn a_frame_at_rest_writes_nothing_and_spawns_nothing() {
    let mut app = app_with(GooeySkin::dark(), ShortcutBadges::default());
    tools(&mut app);
    for _ in 0..3 {
        app.update();
    }
    let before = app.world().entities().len();
    // Count what changed over thirty idle frames.
    #[derive(Resource, Default)]
    struct Writes(u32);
    app.init_resource::<Writes>().add_systems(
        Update,
        |changed: Query<(), Or<(Changed<Text>, Changed<Node>, Changed<BackgroundColor>)>>,
         mut writes: ResMut<Writes>| writes.0 += changed.iter().count() as u32,
    );
    app.update();
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
    assert_eq!(app.world().entities().len(), before);
}
