//! The shortcut overlay: hold a modifier and every shortcut in view shows itself, on the widget
//! it belongs to (ExposeHK, Malacria et al. 2013). The words are [`Shortcut::label`]; the crate
//! draws and never runs a chord.

use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::window::PrimaryWindow;

use crate::binding::{BindingSettings, Capturing};
use crate::shortcut::{ChordText, Modifiers, Shortcut};
use crate::skin::Role;
use crate::text::TextEditing;
use crate::tooltip::Tooltip;

/// Above a popover's panel (150), under a tooltip (200); the legend card is one above.
const BADGE_Z: i32 = 160;

/// What reveals the shortcut overlay: a modifier, a plain key, or nothing.
///
/// A `Key` never reveals while any modifier is down (so `Ctrl+K` stays the game's own chord), while
/// a text field has focus (the player is typing a "k") or while a [`crate::binding`] row waits for
/// a press. All of them drop the overlay the frame they come true, and it is gone the frame the
/// key goes up wherever focus is.
#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Default, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Reveal {
    /// Exactly these modifiers held, either side of the keyboard counting, and no other modifier
    /// down. An empty [`Modifiers`] never reveals, like [`Reveal::Never`].
    Modifier(Modifiers),
    /// This key held, with no modifier down.
    Key(KeyCode),
    /// The overlay never shows and holds no time: a game that wants no overlay, or one that
    /// switches it off in a menu.
    Never,
}

impl Default for Reveal {
    /// Alt: Ctrl is held for multi-select and Command for switching windows, so a default on
    /// either would flash on every chord.
    fn default() -> Self {
        Self::Modifier(Modifiers::ALT)
    }
}

impl From<Modifiers> for Reveal {
    fn from(modifiers: Modifiers) -> Self {
        Self::Modifier(modifiers)
    }
}

impl Reveal {
    /// Whether the keyboard is holding this reveal down right now (before any delay).
    fn is_down(self, keys: &ButtonInput<KeyCode>) -> bool {
        match self {
            Self::Modifier(modifiers) => {
                !modifiers.is_empty() && Modifiers::held(keys) == modifiers
            }
            Self::Key(key) => keys.pressed(key) && Modifiers::held(keys).is_empty(),
            Self::Never => false,
        }
    }
}

/// The shortcut overlay's settings: what reveals the badges and how long it is held
/// first. While [`ShortcutBadges::hold`] is held for
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
/// commands.insert_resource(ShortcutBadges { hold: Reveal::Key(KeyCode::KeyK), ..default() });
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
    /// What reveals the badges: Alt by default, a plain key, or [`Reveal::Never`].
    pub hold: Reveal,
    /// How long `hold` is held before the badges show, in seconds: a chord that starts with the
    /// modifier is pressed and released in a tenth of a second and must not flash the screen.
    /// Zero shows them the frame the modifier goes down.
    pub delay_seconds: f32,
}

impl Default for ShortcutBadges {
    fn default() -> Self {
        Self {
            hold: Reveal::default(),
            delay_seconds: 0.3,
        }
    }
}

/// Whether the overlay is up, for a host that has to react to it: dim the ground under the
/// badges, leave the shortcuts out of an ink census, re-place on a tab change. `true` from the
/// frame [`update_shortcut_badges`] has the reveal held for [`ShortcutBadges::delay_seconds`] until
/// the frame it is let go, switched off or blocked, whether or not any widget has a chord to show.
/// The crate writes it, only when it changes; a host reads it (`Res<BadgesShown>`) and never
/// writes it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
pub struct BadgesShown(pub bool);

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
type Badgeable = (
    Or<(With<Shortcut>, With<Tooltip>)>,
    Without<InteractionDisabled>,
);

/// What the overlay reads of a widget: its chord (either source), whether it is shown, its box.
type BadgeItem = (
    Entity,
    Option<&'static Shortcut>,
    Option<&'static Tooltip>,
    &'static InheritedVisibility,
    &'static ComputedNode,
    &'static UiGlobalTransform,
);

/// Shows and hides the badges (see [`ShortcutBadges`]) and keeps [`BadgesShown`]. Writes nothing
/// while the reveal is not held and no badge stands. Runs in [`crate::GooeySystems::Style`].
#[allow(clippy::too_many_arguments)]
pub fn update_shortcut_badges(
    settings: Option<Res<ShortcutBadges>>,
    binding: Res<BindingSettings>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    time: Option<Res<Time<Real>>>,
    mut held_for: Local<f32>,
    mut up: Option<ResMut<BadgesShown>>,
    editing: Query<(), With<TextEditing>>,
    capturing: Query<(), With<Capturing>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    widgets: Query<BadgeItem, Badgeable>,
    badges: Query<(Entity, &ChildOf, &Text), With<ShortcutBadge>>,
    mut commands: Commands,
) {
    let hold = settings.as_deref().copied().unwrap_or_default();
    // A plain key is a letter the player may be typing or binding; a modifier is neither.
    let blocked =
        matches!(hold.hold, Reveal::Key(_)) && (!editing.is_empty() || !capturing.is_empty());
    let active = !blocked && keys.as_deref().is_some_and(|keys| hold.hold.is_down(keys));
    *held_for = if active {
        *held_for + time.as_deref().map_or(0.0, |t| t.delta_secs())
    } else {
        0.0
    };
    let now_shown = active && *held_for >= hold.delay_seconds;
    if let Some(up) = up.as_deref_mut()
        && up.0 != now_shown
    {
        up.0 = now_shown;
    }
    if !now_shown {
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
        let Some(shortcut) = shortcut
            .copied()
            .or_else(|| tooltip.and_then(|t| t.shortcut))
        else {
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
        wanted.push((entity, binding.chord(shortcut)));
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

/// One row of a [`ShortcutLegend`]: a chord and what it does.
#[derive(Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LegendRow {
    /// The chord, drawn as [`Shortcut::label`] on a keycap.
    pub chord: Shortcut,
    /// What it does, in the game's words.
    pub words: String,
}

impl LegendRow {
    /// A row: the chord and its words.
    pub fn new(chord: Shortcut, words: impl Into<String>) -> Self {
        Self {
            chord,
            words: words.into(),
        }
    }
}

/// Chords that act on nothing on screen: "undo", "save all", "toggle the grid". A shortcut with a
/// widget is badged on it; a verb with none has nowhere to stand, so the host lists it here and
/// the crate draws the rows as a card in the middle of the window for as long as the overlay is up
/// ([`BadgesShown`]), over the badges, never taking the pointer or focus. An empty list draws
/// nothing. Changing the rows while the card is up redraws it in that frame; the crate never runs
/// a chord.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// commands.insert_resource(ShortcutLegend {
///     rows: vec![
///         LegendRow::new(Shortcut::primary(KeyCode::KeyZ), "Undo"),
///         LegendRow::new(Shortcut::new(Modifiers::SHIFT, KeyCode::KeyG), "Snap to grid"),
///     ],
/// });
/// # }
/// ```
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct ShortcutLegend {
    /// The rows, top to bottom, in the host's order.
    pub rows: Vec<LegendRow>,
}

/// The legend's card, a root node standing while the overlay is up and the [`ShortcutLegend`] has
/// rows; it holds the rows it was built from, so a settled legend writes nothing and a host can
/// read what is on show.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct ShortcutLegendCard(pub Vec<LegendRow>);

/// Shows and hides the [`ShortcutLegend`] card with the overlay. Reads one resource and writes
/// nothing while the overlay is down and no card stands. Runs in [`crate::GooeySystems::Style`],
/// after [`update_shortcut_badges`].
pub fn update_shortcut_legend(
    binding: Res<BindingSettings>,
    up: Option<Res<BadgesShown>>,
    legend: Option<Res<ShortcutLegend>>,
    cards: Query<(Entity, &ShortcutLegendCard)>,
    mut commands: Commands,
) {
    let wanted: &[LegendRow] = match (&up, &legend) {
        (Some(up), Some(legend)) if up.0 => &legend.rows,
        _ => &[],
    };
    if !wanted.is_empty() && cards.iter().any(|(_, card)| card.0 == wanted) {
        return;
    }
    for (card, _) in &cards {
        commands.entity(card).despawn();
    }
    if wanted.is_empty() {
        return;
    }
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::Grid,
                grid_template_columns: vec![GridTrack::auto(), GridTrack::auto()],
                align_items: AlignItems::Center,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                ..default()
            },
            UiTransform {
                translation: Val2::percent(-50.0, -50.0),
                ..default()
            },
            Role::ShortcutLegend,
            ShortcutLegendCard(wanted.to_vec()),
            Pickable::IGNORE,
            GlobalZIndex(BADGE_Z + 1),
        ))
        .with_children(|card| {
            for row in wanted {
                card.spawn((
                    Text::new(binding.chord(row.chord)),
                    ChordText(row.chord),
                    TextLayout::new(Justify::Center, LineBreak::NoWrap),
                    Node {
                        justify_self: JustifySelf::End,
                        ..default()
                    },
                    Role::PromptKey,
                    Pickable::IGNORE,
                ));
                card.spawn((
                    Text::new(row.words.clone()),
                    TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    Role::Label,
                    Pickable::IGNORE,
                ));
            }
        });
}
