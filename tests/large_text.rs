//! The player's largest text size: every widget holds when its text is doubled. WCAG 2.2 SC 1.4.4
//! and the Xbox Accessibility Guideline 101 ask that text resize to 200% "without loss of
//! content or functionality", and that every step on the way keeps that promise (failure F69: text
//! or controls "clipped, truncated or obscured"). Laid out for real in a headless window as tall
//! as a stack of every widget needs.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 4800;

/// A headless window 1280 wide and tall enough that nothing is cut by the window itself; its
/// shorter side is the reference, so the interface is drawn at 1x and every px below is logical.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WIDTH, HEIGHT)
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
    ))
    .insert_resource(UiScaleSettings {
        reference_short_side: WIDTH as f32,
        scale: 1.0,
    });
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

/// Where `entity` is drawn, in logical px.
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

/// Every widget a game builds a screen from, in one panel, with the longest labels a game is
/// likely to give them.
fn everything(app: &mut App) -> Entity {
    let panel = app
        .world_mut()
        .spawn((
            panel(),
            children![
                heading("Options and settings"),
                label("A line of body text, long enough to wrap inside a panel."),
                (
                    menu(),
                    children![button("Play"), button("Resume the last game")]
                ),
                label("Health"),
                meter(60.0, 100.0),
                toggle("Subtitles", true),
                choice("Quality", ["Low", "Medium", "Ultra high"], 1),
                skin_preset("Theme"),
                slider("Music volume", 0.7, 0.0..=1.0),
                binding("Jump", Binding::Key(KeyCode::Space)),
                binding("Use", Binding::Pad(GamepadButton::South)),
                text_input("Save as", "Aria of the Long Name"),
                color_field("Tint", Srgba::new(1.0, 0.53, 0.0, 0.5)),
                tabs(
                    ["Audio", "Display", "Controls"],
                    0,
                    children![
                        tab_page(children![toggle("Mono audio", false)]),
                        tab_page(children![choice("Window", ["Windowed", "Fullscreen"], 0)]),
                        tab_page(children![binding("Fire", Binding::Key(KeyCode::KeyF))]),
                    ]
                ),
                list(
                    3.5,
                    children![
                        button("Autosave"),
                        button("Meadow, 12:40"),
                        button("Harbour, 09:15"),
                        button("Mill, 03:02"),
                        button("Cellar, 01:48"),
                    ]
                ),
            ],
        ))
        .id();
    let root = app.world_mut().spawn(screen()).add_child(panel).id();
    settle(app);
    root
}

/// A part of the interface that lays out on its own, apart from its parent's content: its place is
/// the game's, so it is not checked against the parent.
fn free_standing(node: &Node) -> bool {
    node.position_type == PositionType::Absolute
}

/// Whether the node cuts what lies outside it (a list's viewport): its children may be past its edge.
fn clips(node: &Node) -> bool {
    node.overflow.x != OverflowAxis::Visible || node.overflow.y != OverflowAxis::Visible
}

/// Every child that leaves its parent's box by more than a pixel, as `"parent role -> child role"`
/// with how far, below `root`.
fn escapes(app: &App, root: Entity) -> Vec<String> {
    let world = app.world();
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(parent) = stack.pop() {
        let Some(node) = world.get::<Node>(parent) else {
            continue;
        };
        if clips(node) {
            continue;
        }
        let Some(children) = world.get::<Children>(parent) else {
            continue;
        };
        let outer = rect(app, parent);
        for child in children.iter() {
            let Some(child_node) = world.get::<Node>(child) else {
                continue;
            };
            if free_standing(child_node) || world.get::<ComputedNode>(child).is_none() {
                continue;
            }
            stack.push(child);
            let inner = rect(app, child);
            let (left, right) = (outer.min.x - inner.min.x, inner.max.x - outer.max.x);
            let (top, bottom) = (outer.min.y - inner.min.y, inner.max.y - outer.max.y);
            let worst = left.max(right).max(top).max(bottom);
            if worst > 1.0 {
                found.push(format!(
                    "{:?} {:?} -> {:?} {:?}: {worst:.1} px out (parent {outer:?}, child {inner:?})",
                    world.get::<Role>(parent),
                    parent,
                    world.get::<Role>(child),
                    child
                ));
            }
        }
    }
    found
}

fn at_scale(scale: f32) -> (App, Entity) {
    let mut app = app();
    app.world_mut().resource_mut::<GooeySkin>().text_scale = scale;
    let root = everything(&mut app);
    (app, root)
}

#[test]
fn every_widget_holds_inside_its_box_at_every_text_size_the_row_offers() {
    let mut heights = Vec::new();
    for scale in TEXT_SIZE_STEPS {
        let (app, root) = at_scale(scale);
        let lost = escapes(&app, root);
        assert!(lost.is_empty(), "at {scale}x text: {lost:#?}");
        let card = app.world().get::<Children>(root).expect("a panel")[0];
        heights.push(rect(&app, card).height());
    }
    // The size was applied: the panel grows with the text, so the check above looked at something.
    assert!(
        heights.windows(2).all(|pair| pair[1] > pair[0]),
        "{heights:?}"
    );
    // Padding, gaps and the list's rows do not follow the text, so the panel grows by less than 2x.
    assert!(heights[3] > heights[0] * 1.4, "{heights:?}");
}

#[test]
fn a_dialog_a_toast_and_the_legend_hold_at_every_text_size_too() {
    for scale in TEXT_SIZE_STEPS {
        let mut app = app();
        app.world_mut().resource_mut::<GooeySkin>().text_scale = scale;
        let dialog = app
            .world_mut()
            .spawn((
                dialog(),
                children![(
                    panel(),
                    children![
                        heading("Leave the game?"),
                        label("Your progress since the last save will be lost, and this line is long enough to wrap."),
                        (menu(), children![button("Leave"), button("Stay")]),
                    ]
                )],
            ))
            .id();
        let legend = app
            .world_mut()
            .spawn((
                prompts(),
                children![
                    prompt("Select", Prompt::SELECT),
                    prompt("Back", Prompt::BACK)
                ],
            ))
            .id();
        app.world_mut().spawn(toast(
            "Game saved, and a line long enough that a player must read it twice",
        ));
        settle(&mut app);
        let bodies: Vec<Entity> = app
            .world_mut()
            .query::<(Entity, &Role)>()
            .iter(app.world())
            .filter(|(_, role)| **role == Role::ToastBody)
            .map(|(entity, _)| entity)
            .collect();
        assert_eq!(bodies.len(), 1, "one toast");
        for (name, root) in [("dialog", dialog), ("legend", legend), ("toast", bodies[0])] {
            let lost = escapes(&app, root);
            assert!(lost.is_empty(), "the {name} at {scale}x text: {lost:#?}");
        }
    }
}

#[test]
fn a_controls_text_is_one_line_at_every_text_size_from_100_to_200_percent() {
    // A label 210 px wide in a row that came out a hair narrower wrapped onto two lines at exactly
    // 125%, and its row stood twice as tall. It happens to the row that decides its panel's width,
    // so each widget stands alone in its own panel, sized by it; and every 5% from 100 to 200 is
    // looked at, not only the steps the row offers, because the player's own `text_scale` may be
    // any value.
    for step in 0..=20 {
        let scale = 1.0 + step as f32 * 0.05;
        let mut app = app();
        app.world_mut().resource_mut::<GooeySkin>().text_scale = scale;
        let column = app
            .world_mut()
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: Val::Px(20.0),
                ..default()
            })
            .id();
        let rows: Vec<Entity> = {
            let world = app.world_mut();
            vec![
                world.spawn(text_size("Text size")).id(),
                world.spawn(interface_size("Interface size")).id(),
                world.spawn(pad_style("Controller")).id(),
                world.spawn(skin_preset("Theme")).id(),
                world.spawn(toggle("Subtitles", true)).id(),
                world.spawn(slider("Music volume", 0.7, 0.0..=1.0)).id(),
                world
                    .spawn(choice("Quality", ["Low", "Medium", "Ultra high"], 1))
                    .id(),
                world
                    .spawn(binding("Jump", Binding::Key(KeyCode::Space)))
                    .id(),
                world.spawn(text_input("Save as", "Aria")).id(),
                world
                    .spawn(color_field("Tint", Srgba::new(1.0, 0.53, 0.0, 0.5)))
                    .id(),
                world.spawn(button("Resume the last game")).id(),
                world.spawn(heading("Options and settings")).id(),
                world.spawn(label("A line of body text")).id(),
                world
                    .spawn(tabs(
                        ["Audio", "Display", "Controls"],
                        0,
                        children![
                            tab_page(children![label("A line of body text")]),
                            tab_page(children![label("A line of body text")]),
                            tab_page(children![label("A line of body text")]),
                        ],
                    ))
                    .id(),
            ]
        };
        for row in rows {
            let card = app.world_mut().spawn(panel()).add_child(row).id();
            app.world_mut().entity_mut(column).add_child(card);
        }
        app.world_mut().spawn(screen()).add_child(column);
        settle(&mut app);
        for role in [
            Role::ControlText,
            Role::ChoiceOption,
            Role::SliderReadout,
            Role::BindingText,
            Role::ButtonText,
            Role::Heading,
            Role::Label,
            Role::TabText,
        ] {
            let heights: Vec<f32> = app
                .world_mut()
                .query::<(Entity, &Role)>()
                .iter(app.world())
                .filter(|(_, found)| **found == role)
                .map(|(entity, _)| rect(&app, entity).height())
                .collect();
            let (low, high) = heights
                .iter()
                .fold((f32::MAX, f32::MIN), |(l, h), &x| (l.min(x), h.max(x)));
            assert!(
                high - low <= 2.0,
                "at {scale:.2}x text the {role:?} lines are {heights:?} px high: one wrapped"
            );
        }
    }
}
