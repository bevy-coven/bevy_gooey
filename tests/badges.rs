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
        hold: Reveal::Modifier(Modifiers::ALT),
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
    assert_eq!(
        shown(&mut app),
        want,
        "the component and the tooltip's chord; not the plain or the disabled one"
    );
    assert!(
        shown(&mut app)
            .iter()
            .all(|(w, _)| *w != t.plain && *w != t.off),
        "a widget with no shortcut and a disabled one show none"
    );

    up(&mut app, KeyCode::AltLeft);
    assert!(
        shown(&mut app).is_empty(),
        "released: gone in the same frame"
    );
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
            hold: Reveal::Modifier(Modifiers::ALT),
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
        hold: (Modifiers::CTRL | Modifiers::SHIFT).into(),
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
        hold: Modifiers::NONE.into(),
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
    let card = w.spawn(panel()).add_children(&[hidden, folded, away]).id();
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
            app.world_mut()
                .resource_mut::<InputFocus>()
                .set(t.save, FocusCause::Navigated);
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
                let frame = app
                    .world()
                    .get::<ComputedNode>(widget)
                    .expect("laid out")
                    .border();
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
    type Written = Or<(Changed<Text>, Changed<Node>, Changed<BackgroundColor>)>;
    app.init_resource::<Writes>().add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.update();
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Writes>().0, 0);
    assert_eq!(app.world().entities().len(), before);
}

fn on_key(key: KeyCode) -> ShortcutBadges {
    ShortcutBadges {
        hold: Reveal::Key(key),
        delay_seconds: 0.0,
    }
}

fn is_shown(app: &App) -> bool {
    app.world().resource::<BadgesShown>().0
}

#[test]
fn a_plain_key_reveals_the_badges_after_the_delay_and_letting_go_hides_them() {
    let mut app = app_with(
        GooeySkin::dark(),
        ShortcutBadges {
            hold: Reveal::Key(KeyCode::KeyK),
            delay_seconds: 0.25,
        },
    );
    let t = tools(&mut app);
    down(&mut app, KeyCode::KeyK);
    assert!(shown(&mut app).is_empty(), "0.1 s held");
    assert!(!is_shown(&app));
    app.update();
    app.update();
    let on_show = shown(&mut app);
    assert_eq!(on_show.len(), 2, "0.3 s held: {on_show:?}");
    assert!(on_show.iter().any(|(w, _)| *w == t.save));
    assert!(is_shown(&app));
    up(&mut app, KeyCode::KeyK);
    assert!(shown(&mut app).is_empty(), "gone the frame the key goes up");
    assert!(!is_shown(&app));
    // Another key is not the reveal.
    down(&mut app, KeyCode::KeyJ);
    app.update();
    assert!(shown(&mut app).is_empty() && !is_shown(&app));
}

#[test]
fn a_plain_key_never_reveals_with_a_modifier_down_a_field_typing_or_a_row_waiting() {
    let mut app = app_with(GooeySkin::dark(), on_key(KeyCode::KeyK));
    tools(&mut app);
    // Ctrl+K is the game's own chord, in either order.
    down(&mut app, KeyCode::ControlLeft);
    down(&mut app, KeyCode::KeyK);
    app.update();
    assert!(shown(&mut app).is_empty() && !is_shown(&app), "Ctrl then K");
    up(&mut app, KeyCode::KeyK);
    up(&mut app, KeyCode::ControlLeft);
    down(&mut app, KeyCode::KeyK);
    assert!(is_shown(&app));
    down(&mut app, KeyCode::ShiftLeft);
    assert!(
        shown(&mut app).is_empty() && !is_shown(&app),
        "K then Shift"
    );
    up(&mut app, KeyCode::ShiftLeft);
    assert!(is_shown(&app), "Shift let go: K alone again");
    up(&mut app, KeyCode::KeyK);

    // A field that has focus is being typed in: the "k" is a letter.
    let field = app.world_mut().spawn(TextEditing).id();
    down(&mut app, KeyCode::KeyK);
    app.update();
    assert!(shown(&mut app).is_empty() && !is_shown(&app), "typing");
    up(&mut app, KeyCode::KeyK);
    app.world_mut().entity_mut(field).despawn();

    // A row waiting for a press binds the key; it is no overlay.
    let row = app.world_mut().spawn(Capturing).id();
    down(&mut app, KeyCode::KeyK);
    app.update();
    assert!(shown(&mut app).is_empty() && !is_shown(&app), "waiting");
    up(&mut app, KeyCode::KeyK);
    app.world_mut().entity_mut(row).despawn();

    // The same key held while typing stops, starts the overlay.
    down(&mut app, KeyCode::KeyK);
    let field = app.world_mut().spawn(TextEditing).id();
    app.update();
    assert!(!is_shown(&app), "typing began with the key down");
    app.world_mut().entity_mut(field).despawn();
    app.update();
    assert!(is_shown(&app) && shown(&mut app).len() == 2);
}

#[test]
fn a_released_key_hides_the_badges_wherever_focus_went() {
    let mut app = app_with(GooeySkin::dark(), on_key(KeyCode::KeyK));
    let t = tools(&mut app);
    down(&mut app, KeyCode::KeyK);
    assert!(is_shown(&app));
    // Focus moves to another widget while the key is down.
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(t.open, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    app.update();
    assert!(is_shown(&app), "focus does not matter");
    up(&mut app, KeyCode::KeyK);
    assert!(shown(&mut app).is_empty() && !is_shown(&app));
}

#[test]
fn never_shows_nothing_holds_no_time_and_switching_the_reveal_drops_the_badges_at_once() {
    let mut app = app_with(
        GooeySkin::dark(),
        ShortcutBadges {
            hold: Reveal::Never,
            delay_seconds: 0.35,
        },
    );
    tools(&mut app);
    for code in [KeyCode::AltLeft, KeyCode::KeyK, KeyCode::ControlLeft] {
        down(&mut app, code);
    }
    for _ in 0..6 {
        app.update();
    }
    assert!(shown(&mut app).is_empty() && !is_shown(&app));
    for code in [KeyCode::AltLeft, KeyCode::KeyK, KeyCode::ControlLeft] {
        up(&mut app, code);
    }

    // Switch to the key with it already held: the time held under `Never` is not counted.
    app.insert_resource(ShortcutBadges {
        hold: Reveal::Key(KeyCode::KeyK),
        delay_seconds: 0.35,
    });
    down(&mut app, KeyCode::KeyK);
    assert!(
        !is_shown(&app),
        "the clock starts at the key, not at the switch"
    );
    app.update();
    app.update();
    app.update();
    assert!(is_shown(&app));

    // Back to Never while it is held: the badges go in that frame.
    app.insert_resource(ShortcutBadges {
        hold: Reveal::Never,
        delay_seconds: 0.0,
    });
    app.update();
    assert!(shown(&mut app).is_empty() && !is_shown(&app));
}

#[test]
fn a_host_reads_shown_while_the_badges_stand_even_with_nothing_to_show_and_it_is_written_only_on_a_change()
 {
    let mut app = app_with(GooeySkin::dark(), instant());
    // No widget has a chord: the overlay is still open, for a host's legend.
    assert!(!is_shown(&app));
    down(&mut app, KeyCode::AltLeft);
    assert!(shown(&mut app).is_empty());
    assert!(is_shown(&app), "up with nothing to label");
    app.update();
    app.world_mut().clear_trackers();
    for _ in 0..5 {
        app.update();
        assert!(
            !app.world().resource_ref::<BadgesShown>().is_changed(),
            "a steady overlay writes nothing"
        );
    }
    up(&mut app, KeyCode::AltLeft);
    assert!(!is_shown(&app));
}

fn legend_rows() -> ShortcutLegend {
    ShortcutLegend {
        rows: vec![
            LegendRow::new(Shortcut::primary(KeyCode::KeyZ), "Undo"),
            LegendRow::new(
                Shortcut::new(Modifiers::SHIFT, KeyCode::KeyG),
                "Snap to the grid",
            ),
            LegendRow::new(
                Shortcut::new(Modifiers::CTRL | Modifiers::SHIFT, KeyCode::KeyS),
                "Save every open document",
            ),
        ],
    }
}

fn cards(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<ShortcutLegendCard>>()
        .iter(world)
        .collect()
}

/// The legend's keycaps and words, row by row: `(chord text, its rect, words text, its rect)`.
fn legend_cells(app: &mut App, card: Entity) -> Vec<(String, Rect, String, Rect)> {
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(card)
        .expect("a card has rows")
        .iter()
        .collect();
    kids.chunks(2)
        .map(|pair| {
            let text = |e: Entity| app.world().get::<Text>(e).expect("text").0.clone();
            (
                text(pair[0]),
                rect(app, pair[0]),
                text(pair[1]),
                rect(app, pair[1]),
            )
        })
        .collect()
}

#[test]
fn the_legend_stands_with_the_overlay_and_a_chord_with_no_widget_has_its_row() {
    let mut app = app_with(GooeySkin::dark(), instant());
    app.insert_resource(legend_rows());
    tools(&mut app);
    settle(&mut app);
    assert!(cards(&mut app).is_empty(), "nothing before the reveal");
    down(&mut app, KeyCode::AltLeft);
    app.update();
    let found = cards(&mut app);
    assert_eq!(found.len(), 1);
    let card = found[0];
    let cells = legend_cells(&mut app, card);
    let words: Vec<&str> = cells.iter().map(|c| c.2.as_str()).collect();
    assert_eq!(
        words,
        ["Undo", "Snap to the grid", "Save every open document"]
    );
    for (i, row) in legend_rows().rows.iter().enumerate() {
        assert_eq!(cells[i].0, row.chord.label());
    }
    assert_eq!(
        shown(&mut app).len(),
        2,
        "the widgets' badges stand as well"
    );
    up(&mut app, KeyCode::AltLeft);
    assert!(
        cards(&mut app).is_empty(),
        "gone with the overlay, in that frame"
    );
}

#[test]
fn the_legend_is_a_card_in_the_middle_of_the_window_with_keycaps_over_words_in_rows() {
    let mut app = app_with(GooeySkin::dark(), instant());
    app.insert_resource(legend_rows());
    settle(&mut app);
    down(&mut app, KeyCode::AltLeft);
    settle(&mut app);
    let card = cards(&mut app)[0];
    let frame = rect(&app, card);
    let window = Rect::new(0.0, 0.0, 1280.0, 720.0);
    assert!(
        window.contains(frame.min) && window.contains(frame.max),
        "{frame:?}"
    );
    assert!(
        (frame.center() - window.center()).abs().max_element() < 1.5,
        "centred: {frame:?}"
    );
    let cells = legend_cells(&mut app, card);
    let mut right_edge = f32::MIN;
    for (i, (_, cap, _, words)) in cells.iter().enumerate() {
        assert!(cap.max.x <= words.min.x + 0.5, "row {i}: cap then words");
        assert!(
            frame.contains(cap.min) && frame.contains(words.max),
            "row {i} inside the card"
        );
        assert!(
            words.min.x >= right_edge.min(words.min.x),
            "one words column"
        );
        right_edge = cap.max.x;
        if let Some(next) = cells.get(i + 1) {
            assert!(next.1.min.y >= cap.max.y - 0.5, "rows do not overlap");
            assert!(
                (next.1.max.x - cap.max.x).abs() < 1.5,
                "keycaps end in one column"
            );
            assert!(
                (next.3.min.x - words.min.x).abs() < 1.5,
                "words start in one column"
            );
        }
    }
    // The pointer goes through it and no one can focus it.
    let world = app.world();
    assert_eq!(world.get::<Pickable>(card), Some(&Pickable::IGNORE));
    assert!(
        world
            .get::<bevy::input_focus::tab_navigation::TabIndex>(card)
            .is_none()
    );
    // Opaque: every pixel of its fill is the skin's solid surface, so words read over any ground.
    let fill = world.get::<BackgroundColor>(card).expect("filled").0;
    assert_eq!(fill, GooeySkin::dark().solid_surface());
    assert_eq!(fill.alpha(), 1.0);
}

#[test]
fn the_legend_follows_the_rows_the_host_writes_and_a_settled_one_writes_nothing() {
    let mut app = app_with(GooeySkin::dark(), instant());
    app.insert_resource(legend_rows());
    settle(&mut app);
    down(&mut app, KeyCode::AltLeft);
    settle(&mut app);
    let card = cards(&mut app)[0];
    #[derive(Resource, Default)]
    struct Writes(u32);
    type Written = Or<(Changed<Text>, Changed<Node>, Changed<BackgroundColor>)>;
    app.init_resource::<Writes>().add_systems(
        Update,
        |changed: Query<(), Written>, mut writes: ResMut<Writes>| {
            writes.0 += changed.iter().count() as u32
        },
    );
    app.update();
    app.world_mut().resource_mut::<Writes>().0 = 0;
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(cards(&mut app), [card], "the same card, not rebuilt");
    assert_eq!(app.world().resource::<Writes>().0, 0);

    app.insert_resource(ShortcutLegend {
        rows: vec![LegendRow::new(
            Shortcut::new(Modifiers::NONE, KeyCode::KeyH),
            "Help",
        )],
    });
    app.update();
    let found = cards(&mut app);
    assert_eq!(found.len(), 1);
    let cells = legend_cells(&mut app, found[0]);
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].2, "Help");

    // No rows: no card, though the overlay stands.
    app.insert_resource(ShortcutLegend::default());
    app.update();
    assert!(cards(&mut app).is_empty() && is_shown(&app));
}

#[test]
fn the_legend_stays_inside_the_window_at_every_text_size_and_reads_at_4_5_to_1_in_every_look() {
    use bevy_gooey::model::TEXT_SIZE_STEPS;
    for preset in SkinPreset::ALL {
        for step in TEXT_SIZE_STEPS {
            let mut skin = preset.skin();
            skin.text_scale = step;
            let mut app = app_with(skin.clone(), instant());
            app.insert_resource(legend_rows());
            settle(&mut app);
            down(&mut app, KeyCode::AltLeft);
            settle(&mut app);
            let card = cards(&mut app)[0];
            let frame = rect(&app, card);
            let window = Rect::new(0.0, 0.0, 1280.0, 720.0);
            assert!(
                window.contains(frame.min) && window.contains(frame.max),
                "{} at {step}: {frame:?}",
                preset.name()
            );
            for (_, cap, _, words) in legend_cells(&mut app, card) {
                assert!(cap.size().y >= 1.0 && words.max.x <= frame.max.x + 0.5);
            }
        }
        // The words' ink against the card's fill, as drawn.
        let mut app = app_with(preset.skin(), instant());
        app.insert_resource(legend_rows());
        settle(&mut app);
        down(&mut app, KeyCode::AltLeft);
        settle(&mut app);
        let card = cards(&mut app)[0];
        let fill = Srgba::from(app.world().get::<BackgroundColor>(card).expect("fill").0);
        let words = app.world().get::<Children>(card).expect("rows")[1];
        let ink = Srgba::from(app.world().get::<TextColor>(words).expect("ink").0);
        let ratio = contrast_ratio(
            [ink.red, ink.green, ink.blue],
            [fill.red, fill.green, fill.blue],
        );
        assert!(ratio >= 4.5, "{}: {ratio}", preset.name());
    }
}

/// Every chord word the overlay draws: the badges on the widgets and the legend's caps (#45).
fn chords_on_screen(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let mut words: Vec<String> = world
        .query_filtered::<&Text, With<ShortcutBadge>>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect();
    let caps: Vec<String> = world
        .query::<(&Role, &Text, &ChildOf)>()
        .iter(world)
        .filter(|(role, _, _)| **role == Role::PromptKey)
        .map(|(_, text, _)| text.0.clone())
        .collect();
    words.extend(caps);
    words.sort();
    words
}

#[test]
fn the_overlay_spells_its_chords_the_hosts_way_and_follows_a_change() {
    let mut app = app_with(GooeySkin::dark(), instant());
    app.insert_resource(ShortcutLegend {
        rows: vec![LegendRow::new(
            Shortcut::new(Modifiers::META, KeyCode::KeyZ),
            "Undo",
        )],
    });
    app.world_mut().resource_mut::<BindingSettings>().platform = Some(Platform::Mac);
    let save = app
        .world_mut()
        .spawn((
            button("Save"),
            Shortcut::new(Modifiers::META, KeyCode::KeyS),
        ))
        .id();
    let card = app.world_mut().spawn(panel()).add_child(save).id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(&mut app);
    down(&mut app, KeyCode::AltLeft);
    settle(&mut app);
    let symbols = chords_on_screen(&mut app);
    assert!(
        symbols.contains(&"⌘S".to_owned()) && symbols.contains(&"⌘Z".to_owned()),
        "the symbols by default, on a badge and on a legend row: {symbols:?}"
    );

    app.world_mut()
        .resource_mut::<BindingSettings>()
        .chord_glyphs = ChordGlyphs::Words;
    settle(&mut app);
    let words = chords_on_screen(&mut app);
    assert!(
        words.contains(&"Cmd+S".to_owned()) && words.contains(&"Cmd+Z".to_owned()),
        "the same overlay, in words: {words:?}"
    );
    assert!(
        words.iter().all(|chord| chord.is_ascii()),
        "Words is ASCII everywhere the overlay draws a chord: {words:?}"
    );
}
