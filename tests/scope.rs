//! A subtree that carries its own skin: `SkinScope`, in a headless app. A widget is styled from the
//! nearest scope at or above it, else from the `GooeySkin` resource.

use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::Pressed;
use bevy_gooey::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GooeyStylePlugin));
    app
}

/// A skin that differs from the default in every colour a test reads.
fn skin(accent: Color, text: Color) -> GooeySkin {
    GooeySkin {
        accent,
        text,
        ..default()
    }
}

const RED: Color = Color::srgb(0.9, 0.1, 0.1);
const GREEN: Color = Color::srgb(0.1, 0.9, 0.1);
const BLUE: Color = Color::srgb(0.1, 0.1, 0.9);
const MAGENTA: Color = Color::srgb(1.0, 0.0, 1.0);

/// A pressed button's fill is the accent of the skin it is styled from.
fn pressed_button(app: &mut App, parent: Option<Entity>) -> Entity {
    let button = app.world_mut().spawn((button("Go"), Pressed)).id();
    if let Some(parent) = parent {
        app.world_mut().entity_mut(parent).add_child(button);
    }
    button
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .map(|b| b.0)
        .unwrap_or_else(|| panic!("{entity:?} has no fill"))
}

fn scope_node(app: &mut App, scope: GooeySkin) -> Entity {
    app.world_mut().spawn((Node::default(), SkinScope(scope))).id()
}

#[test]
fn two_scopes_give_two_buttons_their_own_accent_and_removing_one_restores_the_resource() {
    let mut app = app();
    let global = GooeySkin::default();
    let red = scope_node(&mut app, skin(RED, Color::WHITE));
    let green = scope_node(&mut app, skin(GREEN, Color::WHITE));
    let in_red = pressed_button(&mut app, Some(red));
    let in_green = pressed_button(&mut app, Some(green));
    let outside = pressed_button(&mut app, None);
    app.update();
    assert_eq!(fill(&app, in_red), RED);
    assert_eq!(fill(&app, in_green), GREEN);
    assert_eq!(fill(&app, outside), global.accent);

    app.world_mut().entity_mut(red).remove::<SkinScope>();
    app.update();
    assert_eq!(
        fill(&app, in_red),
        global.accent,
        "the frame the scope is removed in"
    );
    assert_eq!(fill(&app, in_green), GREEN, "the other scope is untouched");
}

#[test]
fn a_scope_restyles_a_widget_in_the_frame_it_is_added_and_the_nearest_wins() {
    let mut app = app();
    let outer = scope_node(&mut app, skin(RED, Color::WHITE));
    let inner = scope_node(&mut app, skin(GREEN, Color::WHITE));
    app.world_mut().entity_mut(outer).add_child(inner);
    let deep = pressed_button(&mut app, Some(inner));
    let shallow = pressed_button(&mut app, Some(outer));
    app.update();
    assert_eq!(fill(&app, deep), GREEN, "the nearest scope wins");
    assert_eq!(fill(&app, shallow), RED);

    app.world_mut().entity_mut(inner).remove::<SkinScope>();
    app.update();
    assert_eq!(fill(&app, deep), RED, "the scope above shows through");

    app.world_mut().entity_mut(inner).insert(SkinScope(skin(BLUE, Color::WHITE)));
    app.update();
    assert_eq!(fill(&app, deep), BLUE, "a scope added to a settled tree");
}

#[test]
fn a_widget_may_carry_its_own_scope() {
    let mut app = app();
    let button = pressed_button(&mut app, None);
    app.update();
    app.world_mut()
        .entity_mut(button)
        .insert(SkinScope(skin(BLUE, Color::WHITE)));
    app.update();
    assert_eq!(fill(&app, button), BLUE);
}

#[test]
fn a_changed_scope_restyles_its_subtree_and_nothing_else() {
    let mut app = app();
    let scope = scope_node(&mut app, skin(RED, Color::WHITE));
    let inside = pressed_button(&mut app, Some(scope));
    let beside = pressed_button(&mut app, None);
    app.update();
    // A marker no restyle would leave, so a widget restyled for nothing shows it.
    app.world_mut().entity_mut(beside).insert(BackgroundColor(MAGENTA));

    app.world_mut().entity_mut(scope).insert(SkinScope(skin(GREEN, Color::WHITE)));
    app.update();
    assert_eq!(fill(&app, inside), GREEN);
    assert_eq!(fill(&app, beside), MAGENTA, "a widget outside was restyled");
}

#[test]
fn a_changed_resource_restyles_the_unscoped_and_leaves_a_scoped_widget_alone() {
    let mut app = app();
    let scope = scope_node(&mut app, skin(RED, Color::WHITE));
    let inside = pressed_button(&mut app, Some(scope));
    let outside = pressed_button(&mut app, None);
    let inside_text = app.world_mut().spawn(label("x")).id();
    app.world_mut().entity_mut(scope).add_child(inside_text);
    app.update();

    app.insert_resource(skin(BLUE, Color::BLACK));
    app.update();
    assert_eq!(fill(&app, outside), BLUE);
    assert_eq!(fill(&app, inside), RED);
    assert_eq!(
        app.world().get::<TextColor>(inside_text),
        Some(&TextColor(Color::WHITE)),
        "a scoped label keeps its scope's text colour"
    );
}

#[test]
fn a_widget_moved_into_or_out_of_a_scope_takes_the_skin_it_now_stands_in() {
    let mut app = app();
    let global = GooeySkin::default();
    let scope = scope_node(&mut app, skin(RED, Color::WHITE));
    let button = pressed_button(&mut app, None);
    app.update();
    assert_eq!(fill(&app, button), global.accent);

    app.world_mut().entity_mut(scope).add_child(button);
    app.update();
    assert_eq!(fill(&app, button), RED, "into the scope");

    app.world_mut().entity_mut(button).remove::<ChildOf>();
    app.update();
    assert_eq!(fill(&app, button), global.accent, "and out again");
}

#[test]
fn a_scope_is_a_whole_skin_sizes_included() {
    let mut app = app();
    let scope = scope_node(
        &mut app,
        GooeySkin {
            text_scale: 2.0,
            min_target: 60.0,
            ..default()
        },
    );
    let big = app.world_mut().spawn(label("Scaled")).id();
    let go = app.world_mut().spawn(button("Go")).id();
    let plain = app.world_mut().spawn(label("Plain")).id();
    app.world_mut().entity_mut(scope).add_children(&[big, go]);
    app.update();
    let world = app.world();
    let size = |e: Entity| world.get::<TextFont>(e).map(|f| f.font_size);
    assert_eq!(size(big), Some(FontSize::Px(40.0)));
    assert_eq!(size(plain), Some(FontSize::Px(20.0)));
    assert_eq!(
        world.get::<Node>(go).map(|n| n.min_height),
        Some(Val::Px(60.0))
    );
}

#[test]
fn a_scoped_tree_written_by_nothing_stays_put() {
    let mut app = app();
    let scope = scope_node(&mut app, skin(RED, Color::WHITE));
    let inside = pressed_button(&mut app, Some(scope));
    app.update();
    app.world_mut().entity_mut(inside).insert(BackgroundColor(MAGENTA));
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        fill(&app, inside),
        MAGENTA,
        "an idle frame restyled a scoped widget"
    );
}
