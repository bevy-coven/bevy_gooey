//! The shortcut overlay: hold a modifier and every shortcut in view shows itself, on the widget
//! it belongs to (ExposeHK, Malacria et al. 2013). The words are [`Shortcut::label`]; the crate
//! draws and never runs a chord.

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::window::PrimaryWindow;

use crate::skin::Role;
use crate::shortcut::{Modifiers, Shortcut};
use crate::tooltip::Tooltip;

/// Above a popover's panel (150), under a tooltip (200).
const BADGE_Z: i32 = 160;

/// The shortcut overlay's settings: which modifier reveals the badges and how long it is held
/// first. While exactly [`ShortcutBadges::hold`] is held (no other modifier down) for
/// [`ShortcutBadges::delay_seconds`], every visible widget on screen that has a [`Shortcut`]
/// (the component, or the chord of its [`Tooltip`]) gets a badge with the chord's words on its
/// top-right corner, inside its frame so the focus ring is never covered; releasing the modifier
/// removes them in the same frame. A disabled widget shows none, its chord does nothing.
///
/// Add [`crate::GooeyStylePlugin`] (it is in [`crate::GooeyPlugins`]) and put a [`Shortcut`] on
/// the widgets:
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.insert_resource(ShortcutBadges { hold: Modifiers::ALT, ..default() });
/// commands.spawn((button("Save"), Shortcut::primary(KeyCode::KeyS)));
/// # }
/// ```
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct ShortcutBadges {
    /// The modifiers that reveal the badges, held exactly (either side of the keyboard counts):
    /// Alt by default, [`Modifiers::default`] (none) turns the overlay off.
    pub hold: Modifiers,
    /// How long `hold` is held before the badges show, in seconds: a chord that starts with the
    /// modifier is pressed and released in a tenth of a second and must not flash the screen.
    /// Zero shows them the frame the modifier goes down.
    pub delay_seconds: f32,
}

impl Default for ShortcutBadges {
    fn default() -> Self {
        Self {
            hold: Modifiers::ALT,
            delay_seconds: 0.3,
        }
    }
}

/// A badge, a child of the widget it names: query for it to read the words on show.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ShortcutBadge;

/// The badge's bundle: one line of the chord's words, absolute on its widget (the corner and the
/// frame are [`crate::apply_skin`]'s), never taking the pointer or focus.
fn badge(label: String) -> impl Bundle {
    (
        Text::new(label),
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        Role::ShortcutBadge,
        ShortcutBadge,
        Pickable::IGNORE,
        GlobalZIndex(BADGE_Z),
    )
}

/// A widget that can carry a badge: laid out, shown, on screen and not disabled.
type Badgeable = (Or<(With<Shortcut>, With<Tooltip>)>, Without<InteractionDisabled>);

/// Shows and hides the badges (see [`ShortcutBadges`]). Writes nothing while the modifier is not
/// held and no badge stands. Runs in [`crate::GooeySystems::Style`].
#[allow(clippy::too_many_arguments)]
pub fn update_shortcut_badges(
    settings: Option<Res<ShortcutBadges>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    time: Option<Res<Time<Real>>>,
    mut held_for: Local<f32>,
    windows: Query<&Window, With<PrimaryWindow>>,
    widgets: Query<
        (
            Entity,
            Option<&Shortcut>,
            Option<&Tooltip>,
            &InheritedVisibility,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        Badgeable,
    >,
    badges: Query<(Entity, &ChildOf, &Text), With<ShortcutBadge>>,
    mut commands: Commands,
) {
    let hold = settings.as_deref().copied().unwrap_or_default();
    let active = !hold.hold.is_empty()
        && keys
            .as_deref()
            .is_some_and(|keys| Modifiers::held(keys) == hold.hold);
    *held_for = if active {
        *held_for + time.as_deref().map_or(0.0, |t| t.delta_secs())
    } else {
        0.0
    };
    if !active || *held_for < hold.delay_seconds {
        for (badge, ..) in &badges {
            commands.entity(badge).despawn();
        }
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let screen = Rect::from_corners(Vec2::ZERO, window.physical_size().as_vec2());

    let mut wanted: Vec<(Entity, String)> = Vec::new();
    for (entity, shortcut, tooltip, shown, node, at) in &widgets {
        let Some(shortcut) = shortcut.copied().or_else(|| tooltip.and_then(|t| t.shortcut)) else {
            continue;
        };
        let size = node.size();
        if !shown.get() || size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let rect = Rect::from_center_size(at.translation, size);
        if rect.intersect(screen).is_empty() {
            continue;
        }
        wanted.push((entity, shortcut.label()));
    }

    // Reconcile: keep a badge whose widget still wants one, fix its words, drop the rest.
    let mut kept: Vec<Entity> = Vec::new();
    for (badge, parent, text) in &badges {
        let widget = parent.parent();
        match wanted.iter().find(|(w, _)| *w == widget) {
            Some((_, label)) if !kept.contains(&widget) => {
                kept.push(widget);
                if text.0 != *label {
                    commands.entity(badge).insert(Text::new(label.clone()));
                }
            }
            _ => commands.entity(badge).despawn(),
        }
    }
    for (widget, label) in wanted {
        if !kept.contains(&widget) {
            commands.spawn((badge(label), ChildOf(widget)));
        }
    }
}
