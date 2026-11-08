//! The selection indicator: one accent node behind a toggle group's or a tab strip's options that
//! springs from the option that was lit to the one that is, hidden at rest. On a 60 Hz clock the
//! test sets.

use std::time::Duration;

use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::window::WindowResolution;
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

/// One frame of the test clock, seconds.
const FRAME: f32 = 1.0 / 60.0;

fn app(motion: MotionSettings) -> App {
    let mut app = App::new();
    app.insert_resource(motion);
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
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        FRAME,
    )));
    app.world_mut().spawn(Camera2d);
    app.update();
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

/// A toggle group of four text options in a panel on a screen. Returns the group.
fn group_of_four(app: &mut App) -> Entity {
    let group = app
        .world_mut()
        .spawn(toggle_group(
            ["Walk", "Fly", "Orbit", "Free"].map(GroupOption::text),
            0,
        ))
        .id();
    let card = app.world_mut().spawn(panel()).add_child(group).id();
    app.world_mut().spawn(screen()).add_child(card);
    frames(app, 12);
    group
}

/// The widget's options, in order: a group's options or a strip's tabs.
fn options_of(app: &App, holder: Entity) -> Vec<Entity> {
    app.world()
        .get::<Children>(holder)
        .expect("children")
        .iter()
        .filter(|&kid| {
            matches!(
                app.world().get::<Role>(kid),
                Some(Role::GroupOption | Role::Tab)
            )
        })
        .collect()
}

/// Where `entity` is laid out, in logical px.
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

fn indicator_of(app: &mut App, host: Entity) -> Entity {
    app.world_mut()
        .query::<(Entity, &SelectionIndicator, &ChildOf)>()
        .iter(app.world())
        .find(|(_, _, parent)| parent.parent() == host)
        .map(|(entity, ..)| entity)
        .expect("the widget's indicator")
}

/// The puck's drawn rect relative to the strip, and whether the indicator shows.
fn drawn(app: &App, indicator: Entity) -> (Vec4, bool) {
    let puck = app
        .world()
        .get::<Children>(indicator)
        .expect("the indicator's puck")
        .iter()
        .find(|&kid| app.world().get::<IndicatorPuck>(kid).is_some())
        .expect("a puck");
    let node = app.world().get::<Node>(puck).expect("a node");
    let px = |val: Val| match val {
        Val::Px(px) => px,
        Val::Auto => 0.0,
        other => panic!("a pixel value, not {other:?}"),
    };
    let shown = app.world().get::<Node>(indicator).expect("a node").display == Display::Flex;
    (
        Vec4::new(px(node.left), px(node.top), px(node.width), px(node.height)),
        shown,
    )
}

fn fill(app: &App, entity: Entity) -> Color {
    app.world()
        .get::<BackgroundColor>(entity)
        .expect("a fill")
        .0
}

fn relative(app: &App, option: Entity, host: Entity) -> Vec4 {
    let (option, host) = (rect(app, option), rect(app, host));
    Vec4::new(
        option.min.x - host.min.x,
        option.min.y - host.min.y,
        option.width(),
        option.height(),
    )
}

#[test]
fn stepping_a_four_option_group_travels_continuously_and_settles_on_the_third() {
    let mut app = app(MotionSettings::default());
    let group = group_of_four(&mut app);
    let options = options_of(&app, group);
    let indicator = indicator_of(&mut app, group);
    let skin = app.world().resource::<GooeySkin>().clone();
    assert!(!drawn(&app, indicator).1, "hidden at rest");
    assert_eq!(
        fill(&app, options[0]),
        skin.accent,
        "the lit option is the accent at rest"
    );
    assert_eq!(fill(&app, options[2]), skin.surface_raised);

    let start = relative(&app, options[0], group);
    let end = relative(&app, options[2], group);
    assert!(
        end.x > start.x + 20.0,
        "the third stands to the right of the first"
    );

    app.world_mut()
        .get_mut::<ToggleGroupValue>(group)
        .expect("value")
        .0 = 2;
    let mut lefts = Vec::new();
    let mut seen_clear = false;
    for _ in 0..90 {
        app.update();
        let (rect, shown) = drawn(&app, indicator);
        if shown {
            lefts.push(rect.x);
            // While it travels the options are clear over the indicator's raised backdrop.
            seen_clear |= fill(&app, options[1]).alpha() == 0.0
                && fill(&app, indicator) == skin.surface_raised;
        }
    }
    assert!(
        lefts.len() > 6,
        "it shows while it travels: {} frames",
        lefts.len()
    );
    assert!(seen_clear, "the options draw clear while it travels");
    assert!(
        lefts.windows(2).all(|w| w[1] >= w[0] - 1e-3),
        "it only moves toward the third: {lefts:?}"
    );
    assert!(
        lefts[0] < end.x - 5.0,
        "it starts well short of the third: {}",
        lefts[0]
    );
    let steps: Vec<f32> = lefts.windows(2).map(|w| w[1] - w[0]).collect();
    let biggest = steps.iter().cloned().fold(0.0_f32, f32::max);
    assert!(
        biggest < (end.x - start.x) * 0.5,
        "no frame jumps half the way: {biggest}"
    );

    // It lands on the third's rect, the group's fills are as at rest, and nothing shows.
    let (rect, shown) = drawn(&app, indicator);
    assert!(!shown, "hidden again once it has landed");
    for (got, want) in [
        (rect.x, end.x),
        (rect.y, end.y),
        (rect.z, end.z),
        (rect.w, end.w),
    ] {
        assert!(
            (got - want).abs() <= 0.5,
            "settled on the third's rect: {rect:?} vs {end:?}"
        );
    }
    assert_eq!(fill(&app, options[2]), skin.accent);
    assert_eq!(fill(&app, options[0]), skin.surface_raised);
    assert_eq!(
        fill(&app, group),
        Color::NONE,
        "the strip itself is never filled"
    );
}

#[test]
fn a_step_back_mid_flight_turns_round_with_no_jump() {
    let mut app = app(MotionSettings::default());
    let group = group_of_four(&mut app);
    let indicator = indicator_of(&mut app, group);
    app.world_mut()
        .get_mut::<ToggleGroupValue>(group)
        .expect("value")
        .0 = 3;
    let mut lefts = Vec::new();
    for frame in 0..60 {
        if frame == 8 {
            app.world_mut()
                .get_mut::<ToggleGroupValue>(group)
                .expect("value")
                .0 = 1;
        }
        app.update();
        let (rect, shown) = drawn(&app, indicator);
        if shown {
            lefts.push(rect.x);
        }
    }
    let steps: Vec<f32> = lefts.windows(2).map(|w| w[1] - w[0]).collect();
    let biggest = steps.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
    let worst_change = steps
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        worst_change < biggest * 0.75 + 0.5,
        "the speed changes smoothly through the turn: change {worst_change}, fastest step {biggest}"
    );
    assert!(
        steps.iter().any(|&s| s < -0.01) && steps.iter().any(|&s| s > 0.01),
        "it did go out and come back"
    );
    let options = options_of(&app, group);
    let want = relative(&app, options[1], group);
    let (rect, _) = drawn(&app, indicator);
    assert!(
        (rect.x - want.x).abs() <= 0.5,
        "it landed on the second: {rect:?} vs {want:?}"
    );
}

#[test]
fn tabs_and_side_tabs_travel_too() {
    let mut app = app(MotionSettings::default());
    let strip = app
        .world_mut()
        .spawn(tabs(
            ["Game", "Video", "Audio"],
            0,
            children![
                tab_page(children![label("a")]),
                tab_page(children![label("b")]),
                tab_page(children![label("c")]),
            ],
        ))
        .id();
    let side = app
        .world_mut()
        .spawn(side_tabs(
            ["Projects".into(), "Learn".into(), SideTab::new("News")],
            0,
        ))
        .id();
    let card = app
        .world_mut()
        .spawn(panel())
        .add_children(&[strip, side])
        .id();
    app.world_mut().spawn(screen()).add_child(card);
    frames(&mut app, 12);
    for (host, step) in [(strip, 2), (side, 2)] {
        let list = app
            .world()
            .get::<Children>(host)
            .expect("children")
            .iter()
            .find(|&kid| app.world().get::<Role>(kid) == Some(&Role::TabList))
            .expect("a strip");
        let options = options_of(&app, list);
        let indicator = indicator_of(&mut app, host);
        assert!(!drawn(&app, indicator).1, "hidden at rest");
        app.world_mut().get_mut::<TabsValue>(host).expect("value").0 = step;
        let mut shown_frames = 0;
        for _ in 0..60 {
            app.update();
            shown_frames += usize::from(drawn(&app, indicator).1);
        }
        assert!(
            shown_frames > 4,
            "{host}: it travelled for {shown_frames} frames"
        );
        let want = relative(&app, options[step], list);
        let (rect, shown) = drawn(&app, indicator);
        assert!(!shown);
        for (got, want) in [
            (rect.x, want.x),
            (rect.y, want.y),
            (rect.z, want.z),
            (rect.w, want.w),
        ] {
            assert!(
                (got - want).abs() <= 0.5,
                "{host}: landed on the tab: {rect:?} vs {want:?}"
            );
        }
    }
}

#[test]
fn reduced_motion_and_no_motion_never_show_the_indicator() {
    for motion in [MotionSettings::reduced(), MotionSettings::off()] {
        let mut app = app(motion.clone());
        let group = group_of_four(&mut app);
        let options = options_of(&app, group);
        let indicator = indicator_of(&mut app, group);
        app.world_mut()
            .get_mut::<ToggleGroupValue>(group)
            .expect("value")
            .0 = 2;
        app.update();
        app.update();
        assert!(
            app.world().get::<bevy::ui::Checked>(options[2]).is_some(),
            "the third is lit at once under {motion:?}"
        );
        for _ in 0..30 {
            app.update();
            assert!(!drawn(&app, indicator).1, "never shown under {motion:?}");
        }
    }
}

#[test]
fn a_group_at_rest_writes_nothing() {
    let mut app = app(MotionSettings::default());
    group_of_four(&mut app);
    #[derive(Resource, Default)]
    struct Writes(usize);
    app.init_resource::<Writes>().add_systems(
        Update,
        |nodes: Query<(), (With<SelectionIndicator>, Changed<Node>)>,
         marks: Query<(), Changed<Indicating>>,
         mut writes: ResMut<Writes>| {
            writes.0 += nodes.iter().count() + marks.iter().count();
        },
    );
    frames(&mut app, 4);
    let settled = app.world().resource::<Writes>().0;
    frames(&mut app, 30);
    assert_eq!(
        app.world().resource::<Writes>().0,
        settled,
        "a resting group writes no indicator and no mark"
    );
}
