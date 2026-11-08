//! The notification center: the rows of [`crate::notification_center`], kept in step with the
//! [`crate::ToastLog`]. Its look is the list's and the button's.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;

use crate::model::clock_text;
use crate::skin::Role;
use crate::toast::{ToastKind, ToastLog};

/// A [`crate::notification_center`]'s root. Query for it to find the center.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct NotificationCenter;

/// On a row [`update_notification_centers`] spawned: a button for a log entry, or the line that says
/// there is none.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct NotificationRow;

/// Keeps every [`crate::notification_center`] as the [`crate::ToastLog`] says: a row for each entry,
/// the newest first, as `m:ss  words` (an alert's words start with `! `: the mark is not only a
/// colour), or "No notifications yet" while the log is empty. Rebuilt when the log changes or a
/// center is added; a log that has not changed writes nothing. Runs in
/// [`crate::GooeySystems::Style`] before the list is laid out.
pub fn update_notification_centers(
    log: Option<Res<ToastLog>>,
    centers: Query<(Entity, Ref<NotificationCenter>)>,
    children: Query<&Children>,
    roles: Query<&Role>,
    rows: Query<Entity, With<NotificationRow>>,
    mut commands: Commands,
) {
    let Some(log) = log else {
        return;
    };
    for (center, marker) in &centers {
        if !log.is_changed() && !marker.is_added() {
            continue;
        }
        // The column the list holds its rows in is found by its role under the center.
        let mut stack = vec![center];
        let mut column = None;
        while let Some(at) = stack.pop() {
            if roles.get(at) == Ok(&Role::ListRows) {
                column = Some(at);
                break;
            }
            if let Ok(kids) = children.get(at) {
                stack.extend(kids.iter());
            }
        }
        let Some(column) = column else {
            // The list has not been spawned yet (the center was added this frame).
            continue;
        };
        for kid in children.get(column).into_iter().flat_map(|k| k.iter()) {
            if rows.contains(kid) {
                commands.entity(kid).despawn();
            }
        }
        if log.is_empty() {
            commands.spawn((
                Text::new("No notifications yet"),
                Role::Label,
                NotificationRow,
                ChildOf(column),
            ));
            continue;
        }
        for entry in log.entries().rev() {
            let mark = if entry.kind == ToastKind::Alert {
                "! "
            } else {
                ""
            };
            commands.spawn((
                Node {
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    ..default()
                },
                bevy::ui_widgets::Button,
                Role::Button,
                Hovered::default(),
                TabIndex(0),
                AutoDirectionalNavigation::default(),
                NotificationRow,
                ChildOf(column),
                children![(
                    Text::new(format!(
                        "{}  {mark}{}",
                        clock_text(entry.seconds),
                        entry.text
                    )),
                    Role::ButtonText,
                )],
            ));
        }
    }
}
