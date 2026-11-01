//! Density: a compact skin for desktop tools. Half the padding and a 24 px least target, the
//! target floor still checked, and the Tool preset holding at every text size. Laid out for real in
//! a headless window as tall as a stack of every widget needs.

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::ui::UiGlobalTransform;
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 4800;

/// A headless window whose shorter side is the reference, so the interface is drawn at 1x and every
/// px below is logical.
fn app_with(skin: GooeySkin) -> App {
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
    })
    .insert_resource(skin);
    app.world_mut().spawn(Camera2d);
    app
}

fn settle(app: &mut App) {
    for _ in 0..6 {
        app.update();
    }
}

fn height(app: &App, entity: Entity) -> f32 {
    app.world()
        .get::<ComputedNode>(entity)
        .expect("laid out")
        .size()
        .y
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

/// A panel with one button and its padding, on a screen.
fn one_button(app: &mut App) -> (Entity, Entity) {
    let button = app.world_mut().spawn(button("Go")).id();
    let card = app.world_mut().spawn(panel()).add_child(button).id();
    app.world_mut().spawn(screen()).add_child(card);
    settle(app);
    (card, button)
}

fn padding_top(app: &App, entity: Entity) -> f32 {
    match app.world().get::<Node>(entity).expect("a node").padding.top {
        Val::Px(px) => px,
        other => panic!("padding is {other:?}"),
    }
}

#[test]
fn a_compact_button_is_at_least_24_px_and_shorter_than_a_comfortable_one() {
    let comfortable = GooeySkin::default();
    assert_eq!(comfortable.density, Density::Comfortable);
    let mut app = app_with(comfortable.clone());
    let (card, button) = one_button(&mut app);
    let (tall, pad) = (height(&app, button), padding_top(&app, card));
    assert!(tall >= 44.0, "a comfortable button is {tall} px");

    // Same colours and sizes, one field different: the target is capped and the padding halved.
    let mut app = app_with(GooeySkin {
        density: Density::Compact,
        ..comfortable
    });
    let (card, button) = one_button(&mut app);
    let (short, half) = (height(&app, button), padding_top(&app, card));
    assert!(short >= 24.0, "a compact button is {short} px");
    assert!(short < tall, "{short} px against {tall} px");
    assert_eq!(half, pad / 2.0, "the panel's padding is half");

    // The player's density is live: the same widgets, restyled by the resource.
    app.world_mut().resource_mut::<GooeySkin>().density = Density::Comfortable;
    settle(&mut app);
    assert_eq!(height(&app, button), tall);
    assert_eq!(padding_top(&app, card), pad);
}

#[test]
fn the_target_floor_is_still_checked_in_a_compact_skin() {
    assert_eq!(GooeySkin::tool().problems(), vec![]);
    assert_eq!(SkinPreset::Tool.skin(), GooeySkin::tool());
    let compact = |min_target| GooeySkin {
        density: Density::Compact,
        min_target,
        ..GooeySkin::tool()
    };
    // A comfortable 44 is capped to the floor, not refused; 30 too.
    assert_eq!(compact(44.0).problems(), vec![]);
    assert_eq!(compact(30.0).target(), 24.0);
    // Under 24 px stays under: WCAG 2.2 SC 2.5.8.
    assert_eq!(
        compact(20.0).problems(),
        vec![SkinProblem::TargetTooSmall { px: 20.0 }]
    );
    // And a comfortable skin is judged on its own field, as before.
    assert_eq!(
        GooeySkin {
            min_target: 20.0,
            ..GooeySkin::dark()
        }
        .problems(),
        vec![SkinProblem::TargetTooSmall { px: 20.0 }]
    );
}

#[test]
fn the_tool_look_is_its_own_preset_the_theme_row_can_show() {
    assert_eq!(SkinPreset::ALL.last(), Some(&SkinPreset::Tool));
    assert_eq!(SkinPreset::Tool.name(), "Tool");
    for preset in SkinPreset::ALL {
        assert_eq!(SkinPreset::of(&preset.skin()), preset);
        assert_eq!(preset.skin().problems(), vec![], "{}", preset.name());
    }
    assert_eq!(SkinPreset::Dark.skin().density, Density::Comfortable);
    // A theme step writes colours and no size: a player's density stays theirs.
    let players = GooeySkin {
        density: Density::Comfortable,
        text_scale: 1.5,
        ..GooeySkin::dark()
    };
    let stepped = players.with_colours_of(&GooeySkin::tool());
    assert_eq!(stepped.density, Density::Comfortable);
    assert_eq!(stepped.text_scale, 1.5);
    assert_eq!(SkinPreset::of(&stepped), SkinPreset::Tool);
}

/// Every widget a tool builds a panel from.
fn everything(app: &mut App) -> Entity {
    let card = app
        .world_mut()
        .spawn((
            panel(),
            children![
                heading("Inspector"),
                label("A line of body text, long enough to wrap inside a panel."),
                (
                    menu(),
                    children![button("Apply"), button("Revert the whole file")]
                ),
                label("Health"),
                meter(60.0, 100.0),
                toggle("Visible", true),
                choice("Mode", ["Local", "World", "Parent space"], 1),
                skin_preset("Theme"),
                slider("Opacity", 0.7, 0.0..=1.0),
                binding("Move", Binding::Key(KeyCode::KeyW)),
                text_input("Name", "Directional light"),
                tabs(
                    ["Scene", "Assets", "Console"],
                    0,
                    children![
                        tab_page(children![toggle("Snap", false)]),
                        tab_page(children![button("Import")]),
                        tab_page(children![label("Ready")]),
                    ]
                ),
                list(
                    3.5,
                    children![
                        button("Main camera"),
                        button("Sun"),
                        button("Ground plane"),
                        button("Spawn point"),
                        button("Trigger"),
                    ]
                ),
            ],
        ))
        .id();
    let root = app.world_mut().spawn(screen()).add_child(card).id();
    settle(app);
    root
}

/// Every child that leaves its parent's box by more than a pixel, below `root`.
fn escapes(app: &App, root: Entity) -> Vec<String> {
    let world = app.world();
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(parent) = stack.pop() {
        let Some(node) = world.get::<Node>(parent) else {
            continue;
        };
        if node.overflow.x != OverflowAxis::Visible || node.overflow.y != OverflowAxis::Visible {
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
            if child_node.position_type == PositionType::Absolute
                || world.get::<ComputedNode>(child).is_none()
            {
                continue;
            }
            stack.push(child);
            let inner = rect(app, child);
            let worst = (outer.min.x - inner.min.x)
                .max(inner.max.x - outer.max.x)
                .max(outer.min.y - inner.min.y)
                .max(inner.max.y - outer.max.y);
            if worst > 1.0 {
                found.push(format!(
                    "{:?} -> {:?} {child:?}: {worst:.1} px out",
                    world.get::<Role>(parent),
                    world.get::<Role>(child),
                ));
            }
        }
    }
    found
}

#[test]
fn the_tool_look_holds_every_widget_inside_its_box_and_every_control_24_px_high() {
    let mut heights = Vec::new();
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut app = app_with(GooeySkin {
            text_scale: scale,
            ..GooeySkin::tool()
        });
        let root = everything(&mut app);
        let lost = escapes(&app, root);
        assert!(lost.is_empty(), "at {scale}x text: {lost:#?}");
        let mut seen = 0;
        let mut query = app.world_mut().query::<(Entity, &Role)>();
        let controls: Vec<_> = query
            .iter(app.world())
            .filter(|(_, role)| {
                matches!(
                    role,
                    Role::Button
                        | Role::Toggle
                        | Role::Choice
                        | Role::Binding
                        | Role::Tab
                        | Role::TextInput
                )
            })
            .map(|(entity, _)| entity)
            .collect();
        for entity in controls {
            let tall = height(&app, entity);
            assert!(
                tall >= 24.0 - 0.5,
                "{:?} is {tall} px at {scale}x text",
                app.world().get::<Role>(entity)
            );
            seen += 1;
        }
        assert!(seen >= 14, "only {seen} controls were laid out");
        let card = app.world().get::<Children>(root).expect("a panel")[0];
        heights.push(height(&app, card));
    }
    assert!(
        heights.windows(2).all(|pair| pair[1] > pair[0]),
        "{heights:?}"
    );
}

#[test]
fn a_tool_panel_is_much_shorter_than_the_same_panel_in_a_game_skin() {
    let mut tool = app_with(GooeySkin::tool());
    let tool_root = everything(&mut tool);
    let mut game = app_with(GooeySkin::dark());
    let game_root = everything(&mut game);
    let card = |app: &App, root: Entity| app.world().get::<Children>(root).expect("a panel")[0];
    let (small, large) = (
        height(&tool, card(&tool, tool_root)),
        height(&game, card(&game, game_root)),
    );
    assert!(
        small < large * 0.75,
        "a tool panel {small} px against a game's {large} px"
    );
}
