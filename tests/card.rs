//! The card grid: cards of a picture over a name and quiet lines, in a row that wraps to fill its
//! width, one tab stop the arrows walk, a chosen card drawn lit and said once, a dimmed card no
//! one chooses, laid out in a headless window and driven by real key and pointer input.

use bevy::camera::NormalizedRenderTarget;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButton;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::{InteractionDisabled, Selected, UiGlobalTransform};
use bevy::window::{PrimaryWindow, WindowRef, WindowResolution};
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
    for _ in 0..4 {
        app.update();
    }
}

/// Every `ValueChange<usize>` the grid sent: `(value, is_final)`.
#[derive(Resource, Default)]
struct Heard(Vec<(usize, bool)>);

/// A panel `width` px wide holding a grid of `n` cards, the third one dimmed: `(grid, cards)`.
fn grid_of(app: &mut App, n: usize, width: f32) -> (Entity, Vec<Entity>) {
    let cards = (0..n).map(|i| {
        let card = Card::new(format!("Map {i}"))
            .line("10 x 10 m")
            .line("13 pieces");
        if i == 2 { card.disabled() } else { card }
    });
    let grid = app
        .world_mut()
        .spawn(card_grid(cards))
        .observe(|change: On<ValueChange<usize>>, mut heard: ResMut<Heard>| {
            heard.0.push((change.value, change.is_final));
        })
        .id();
    let holder = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(width),
            ..default()
        })
        .add_child(grid)
        .id();
    app.world_mut().spawn(screen()).add_child(holder);
    settle(app);
    let cards = app
        .world()
        .get::<Children>(grid)
        .expect("cards")
        .iter()
        .collect();
    (grid, cards)
}

/// A node's rectangle in the window, logical px: `(left, top, right, bottom)`.
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
    app.update();
}

fn click(app: &mut App, entity: Entity) {
    let (l, t, r, b) = rect(app, entity);
    let window = window(app);
    let location = Location {
        target: NormalizedRenderTarget::Window(
            WindowRef::Entity(window)
                .normalize(None)
                .expect("a window ref"),
        ),
        position: Vec2::new((l + r) / 2.0, (t + b) / 2.0),
    };
    for action in [
        PointerAction::Move { delta: Vec2::X },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        app.update();
    }
    let _ = MouseButton::Left;
    settle(app);
}

fn focused(app: &App) -> Option<Entity> {
    app.world().resource::<InputFocus>().get()
}

#[test]
fn cards_stand_in_a_row_that_wraps_to_fill_its_width() {
    let mut app = app();
    let (grid, cards) = grid_of(&mut app, 7, 900.0);
    let rects: Vec<_> = cards.iter().map(|&c| rect(&app, c)).collect();
    let width = rects[0].2 - rects[0].0;
    let per_row = rects
        .iter()
        .filter(|r| (r.1 - rects[0].1).abs() < 1.0)
        .count();
    assert!(per_row >= 2, "{per_row} cards in the first row");
    assert!(
        per_row < 7,
        "the row wraps inside 900 px: {per_row} of 7 in the first"
    );
    // Every row holds as many as fit: the next card would cross the grid's edge.
    let gap = rects[1].0 - rects[0].2;
    let grid_rect = rect(&app, grid);
    assert!(
        (grid_rect.2 - grid_rect.0 - 900.0).abs() <= 1.0,
        "the grid fills its width"
    );
    assert!(
        rects[per_row - 1].2 + gap + width > grid_rect.2 - 0.5,
        "no room was left for one more card"
    );
    // Every card is as wide as the first, and the second row starts under the first.
    for r in &rects {
        assert!(((r.2 - r.0) - width).abs() <= 0.5);
    }
    assert!(rects[per_row].1 > rects[0].3, "a second row below");
    assert!(
        (rects[per_row].0 - rects[0].0).abs() <= 0.5,
        "from the start"
    );
    // The picture stands above the name, across the card, at 16:9.
    let parts = app.world().get::<Children>(cards[0]).expect("parts");
    let picture = rect(&app, parts[0]);
    let name = rect(&app, parts[1]);
    assert!(picture.3 <= name.1 + 0.5, "the picture above the name");
    let ratio = (picture.2 - picture.0) / (picture.3 - picture.1);
    assert!((ratio - 16.0 / 9.0).abs() < 0.05, "16:9, got {ratio}");
}

#[test]
fn the_grid_is_one_tab_stop_and_the_arrows_walk_it_with_the_cursor_said_once_a_move() {
    let mut app = app();
    let (grid, cards) = grid_of(&mut app, 6, 900.0);
    let stops = |app: &App| {
        cards
            .iter()
            .filter(|&&c| app.world().get::<TabIndex>(c).is_some_and(|t| t.0 >= 0))
            .count()
    };
    assert_eq!(stops(&app), 1, "one stop");
    tap(&mut app, KeyCode::Tab);
    assert_eq!(focused(&app), Some(cards[0]));
    tap(&mut app, KeyCode::ArrowRight);
    assert_eq!(focused(&app), Some(cards[1]));
    assert_eq!(app.world().get::<CardCursor>(grid), Some(&CardCursor(1)));
    assert_eq!(app.world().resource::<Heard>().0, vec![(1, false)]);
    assert_eq!(app.world().get::<TabIndex>(cards[1]).map(|t| t.0), Some(0));
    assert_eq!(app.world().get::<TabIndex>(cards[0]).map(|t| t.0), Some(-1));
    assert_eq!(stops(&app), 1, "still one stop");
    // Down goes to the card under it in the next row.
    let first_row = cards
        .iter()
        .filter(|&&c| (rect(&app, c).1 - rect(&app, cards[0]).1).abs() < 1.0)
        .count();
    tap(&mut app, KeyCode::ArrowDown);
    assert_eq!(focused(&app), Some(cards[1 + first_row]), "the card below");
    assert!(app.world().resource::<InputFocusVisible>().0);
}

#[test]
fn enter_and_a_click_choose_a_card_which_is_lit_and_said_once() {
    let mut app = app();
    let (grid, cards) = grid_of(&mut app, 4, 900.0);
    tap(&mut app, KeyCode::Tab);
    tap(&mut app, KeyCode::Enter);
    assert_eq!(
        app.world().get::<CardSelection>(grid),
        Some(&CardSelection(Some(0)))
    );
    assert!(app.world().get::<Selected>(cards[0]).is_some(), "lit");
    assert_eq!(app.world().resource::<Heard>().0, vec![(0, true)]);
    app.world_mut().resource_mut::<Heard>().0.clear();
    click(&mut app, cards[3]);
    assert_eq!(
        app.world().get::<CardSelection>(grid),
        Some(&CardSelection(Some(3)))
    );
    assert!(
        app.world().get::<Selected>(cards[0]).is_none(),
        "the old one unlit"
    );
    assert!(app.world().get::<Selected>(cards[3]).is_some());
    assert!(
        app.world().resource::<Heard>().0.contains(&(3, true)),
        "{:?}",
        app.world().resource::<Heard>().0
    );
    assert_eq!(
        app.world()
            .resource::<Heard>()
            .0
            .iter()
            .filter(|(_, f)| *f)
            .count(),
        1,
        "one final change"
    );
    // A game's choice lights the card with no event.
    app.world_mut().resource_mut::<Heard>().0.clear();
    app.world_mut()
        .entity_mut(grid)
        .insert(CardSelection(Some(1)));
    settle(&mut app);
    assert!(app.world().get::<Selected>(cards[1]).is_some());
    assert!(app.world().get::<Selected>(cards[3]).is_none());
    assert!(app.world().resource::<Heard>().0.is_empty());
    // Past the end chooses none.
    app.world_mut()
        .entity_mut(grid)
        .insert(CardSelection(Some(9)));
    settle(&mut app);
    assert_eq!(
        app.world().get::<CardSelection>(grid),
        Some(&CardSelection(None))
    );
    assert!(
        cards
            .iter()
            .all(|&c| app.world().get::<Selected>(c).is_none())
    );
}

#[test]
fn a_dimmed_card_is_reached_but_never_chosen() {
    let mut app = app();
    let (grid, cards) = grid_of(&mut app, 4, 900.0);
    assert!(app.world().get::<InteractionDisabled>(cards[2]).is_some());
    click(&mut app, cards[2]);
    assert_eq!(
        app.world().get::<CardSelection>(grid),
        Some(&CardSelection(None))
    );
    assert!(app.world().resource::<Heard>().0.iter().all(|(_, f)| !*f));
    let skin = GooeySkin::default();
    let parts = app.world().get::<Children>(cards[2]).expect("parts");
    assert_eq!(
        app.world().get::<TextColor>(parts[1]).map(|c| c.0),
        Some(skin.text_muted),
        "a dimmed name"
    );
}

#[test]
fn a_cards_words_are_readable_at_rest_and_lit_in_every_preset() {
    for preset in SkinPreset::ALL {
        let skin = preset.skin();
        let mut app = app();
        app.insert_resource(skin.clone());
        let (grid, cards) = grid_of(&mut app, 2, 900.0);
        app.world_mut()
            .entity_mut(grid)
            .insert(CardSelection(Some(1)));
        settle(&mut app);
        for (card, fill_want) in [(cards[0], skin.solid_surface()), (cards[1], skin.accent)] {
            let fill = app
                .world()
                .get::<BackgroundColor>(card)
                .map(|b| b.0)
                .expect("a fill");
            assert_eq!(fill, fill_want, "{}", preset.name());
            let parts = app.world().get::<Children>(card).expect("parts");
            for &part in &parts[1..] {
                let ink = app.world().get::<TextColor>(part).expect("ink").0;
                let ratio = contrast_ratio(rgb(ink), rgb(fill));
                assert!(
                    ratio >= TEXT_CONTRAST,
                    "{}: {ratio} on {fill:?}",
                    preset.name()
                );
            }
        }
    }
}

fn rgb(color: Color) -> [f32; 3] {
    let c = Srgba::from(color);
    [c.red, c.green, c.blue]
}

#[test]
fn a_card_says_its_name_and_its_lines_to_a_screen_reader() {
    let mut app = app();
    app.world()
        .resource::<bevy::a11y::AccessibilityRequested>()
        .set(true);
    let (grid, cards) = grid_of(&mut app, 2, 900.0);
    app.world_mut()
        .entity_mut(grid)
        .insert(CardSelection(Some(1)));
    settle(&mut app);
    let selected = |app: &App, card: Entity| {
        app.world()
            .get::<bevy::a11y::AccessibilityNode>(card)
            .and_then(|node| node.is_selected())
    };
    assert_eq!(selected(&app, cards[0]), Some(false));
    assert_eq!(selected(&app, cards[1]), Some(true));
    let node = app
        .world()
        .get::<bevy::a11y::AccessibilityNode>(cards[0])
        .expect("a node");
    assert_eq!(node.label(), Some("Map 0"));
    assert_eq!(node.value(), Some("10 x 10 m, 13 pieces"));
}
