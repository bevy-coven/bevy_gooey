//! A split's panes are not marked as interface while the split is open (bevy_grimoire's editor, whose map's viewport stands
//! in a pane: since v0.44.0 every click meant for the map was taken for a click on the interface).
//!
//! A host tells the interface from the world behind it by asking whether any node that carries [`Hovered`] is under the
//! pointer. `split_pane` carried one from v0.44.0 (the peek of #69 needed it), so a pane holding a game's viewport covered it
//! entirely. The peek now puts `Hovered` on the first pane only while the split is collapsed, and takes it off when the split
//! is not.

use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::WgpuSettings;
use bevy::time::TimeUpdateStrategy;
use bevy::window::{PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use bevy_gooey::*;

const FRAME: f32 = 0.03;

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
    app.update();
    app
}

struct Rig {
    first: Entity,
    divider: Entity,
    second: Entity,
}

/// An 800 px split of two panels, the first at 40% with a minimum of 100 px, open and settled.
fn open_split(app: &mut App) -> Rig {
    let container = app
        .world_mut()
        .spawn((
            Node {
                width: Val::Px(800.0),
                height: Val::Px(400.0),
                ..default()
            },
            children![(
                split(
                    SplitAxis::Horizontal,
                    (panel(), children![label("A"), button("Sidebar")]),
                    (panel(), children![label("B")]),
                ),
                SplitFraction(0.4),
                SplitMin::new(100.0, 100.0),
            )],
        ))
        .id();
    frames(app, 10);
    let split = app.world().get::<Children>(container).expect("kids")[0];
    let kids: Vec<Entity> = app
        .world()
        .get::<Children>(split)
        .expect("kids")
        .iter()
        .collect();
    Rig {
        first: kids[0],
        divider: kids[1],
        second: kids[2],
    }
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn focus(app: &mut App, entity: Entity) {
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(entity, FocusCause::Navigated);
    app.world_mut().insert_resource(InputFocusVisible(true));
    frames(app, 3);
}

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

fn marked(app: &App, pane: Entity) -> bool {
    app.world().get::<Hovered>(pane).is_some()
}

#[test]
fn an_open_split_marks_neither_pane_as_interface() {
    let mut app = app();
    let rig = open_split(&mut app);
    assert!(
        app.world()
            .get::<SplitSaved>(
                app.world()
                    .get::<ChildOf>(rig.first)
                    .expect("a parent")
                    .parent()
            )
            .is_none(),
        "the split is open"
    );
    assert!(
        !marked(&app, rig.first),
        "the first pane of an open split carries no Hovered"
    );
    assert!(
        !marked(&app, rig.second),
        "the second pane of an open split carries no Hovered"
    );
    assert!(
        marked(&app, rig.divider),
        "the divider is a control and still does"
    );
}

#[test]
fn a_collapsed_split_marks_its_first_pane_for_the_peek_and_a_restored_one_clears_it() {
    let mut app = app();
    let rig = open_split(&mut app);

    focus(&mut app, rig.divider);
    tap(&mut app, KeyCode::Enter);
    frames(&mut app, 40);
    let split_entity = app
        .world()
        .get::<ChildOf>(rig.first)
        .expect("a parent")
        .parent();
    assert!(
        app.world().get::<SplitSaved>(split_entity).is_some(),
        "Enter on the divider collapsed it"
    );
    assert!(
        marked(&app, rig.first),
        "the collapsed first pane carries the Hovered the peek reads"
    );
    assert!(!marked(&app, rig.second), "the second pane never does");

    tap(&mut app, KeyCode::Enter);
    frames(&mut app, 40);
    assert!(
        app.world().get::<SplitSaved>(split_entity).is_none(),
        "Enter again restored it"
    );
    assert!(
        !marked(&app, rig.first),
        "a restored split's first pane carries no Hovered again"
    );
    assert!(!marked(&app, rig.second));
}
