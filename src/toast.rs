//! Toasts: a line of news a game spawns in one call. A stack takes it in, ages it by the message's
//! reading time and the player's setting, opens room for it, fades it in and out, and removes it.

use std::collections::VecDeque;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::ComputedNode;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy::ui_widgets::Activate;

use crate::model::{toast_alpha, toast_grow, toast_hold, toast_life};
use crate::skin::Role;

/// Where a toast's stack stands in the window. The newest toast stands nearest the screen's edge,
/// and the older ones are pushed inward as room opens for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ToastAnchor {
    /// Top left corner.
    TopLeft,
    /// Top edge, centred.
    TopCenter,
    /// Top right corner.
    TopRight,
    /// Bottom left corner.
    BottomLeft,
    /// Bottom edge, centred.
    #[default]
    BottomCenter,
    /// Bottom right corner.
    BottomRight,
}

impl ToastAnchor {
    /// The stack's flex layout: down or up from the edge, hugging it, and across.
    pub(crate) fn layout(self) -> (FlexDirection, JustifyContent, AlignItems) {
        use ToastAnchor::*;
        let direction = match self {
            TopLeft | TopCenter | TopRight => FlexDirection::ColumnReverse,
            BottomLeft | BottomCenter | BottomRight => FlexDirection::Column,
        };
        let across = match self {
            TopLeft | BottomLeft => AlignItems::FlexStart,
            TopCenter | BottomCenter => AlignItems::Center,
            TopRight | BottomRight => AlignItems::FlexEnd,
        };
        (direction, JustifyContent::FlexEnd, across)
    }
}

/// The window's edges the toast stack keeps clear of, logical px on each side: a menu bar along the
/// top, a status band along the bottom, a side panel. The stack stands in the window less these, so
/// a toast anchored to the top stands under the bar with the stack's own padding below it.
///
/// Logical px are scaled by `UiScale` like every other size, so a host sets the bar's height as it
/// laid the bar out, and rewrites [`ToastSettings::inset`] when that height changes (a skin with
/// bigger text). A side that is negative or not finite keeps no room.
///
/// ```
/// use bevy_gooey::{ToastAnchor, ToastInset, ToastSettings};
/// // Refusals at the top centre, under a 34 px menu bar.
/// let settings = ToastSettings {
///     anchor: ToastAnchor::TopCenter,
///     inset: ToastInset::top(34.0),
///     ..ToastSettings::default()
/// };
/// assert_eq!(settings.inset.bottom, 0.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
pub struct ToastInset {
    /// Room kept along the window's top edge, logical px.
    pub top: f32,
    /// Room kept along the window's right edge, logical px.
    pub right: f32,
    /// Room kept along the window's bottom edge, logical px.
    pub bottom: f32,
    /// Room kept along the window's left edge, logical px.
    pub left: f32,
}

impl ToastInset {
    /// Room along the top edge only: the height of a menu bar.
    pub fn top(px: f32) -> Self {
        Self {
            top: px,
            ..Self::default()
        }
    }

    /// Room along the bottom edge only: the height of a status band.
    pub fn bottom(px: f32) -> Self {
        Self {
            bottom: px,
            ..Self::default()
        }
    }

    /// One side as the stack's position: a negative or non-finite side keeps no room.
    fn side(px: f32) -> Val {
        Val::Px(if px.is_finite() { px.max(0.0) } else { 0.0 })
    }

    /// The stack's `(left, top, right, bottom)` at this inset: it is absolute, its four sides this
    /// far in from the window's, so it is the window less the inset.
    fn sides(self) -> [Val; 4] {
        [
            Self::side(self.left),
            Self::side(self.top),
            Self::side(self.right),
            Self::side(self.bottom),
        ]
    }

    /// Whether `node` stands at this inset.
    fn holds(self, node: &Node) -> bool {
        [node.left, node.top, node.right, node.bottom] == self.sides()
    }

    /// Puts `node` at this inset.
    fn place(self, node: &mut Node) {
        let [left, top, right, bottom] = self.sides();
        node.position_type = PositionType::Absolute;
        node.left = left;
        node.top = top;
        node.right = right;
        node.bottom = bottom;
    }
}

/// How long toasts stand, how many, how they fade and where: the timing a player may change.
///
/// A toast stands `(base_seconds + characters ÷ chars_per_second) × duration_scale` fully shown
/// ([`crate::toast_hold`]), after fading in and before fading out over `fade_seconds` each. Offer
/// `duration_scale` in your settings screen: Xbox Accessibility Guideline 116 asks that a player can
/// stretch the time notifications stay up to at least ten times the default, and any value above
/// zero works. `fade_seconds` of zero removes the fade and the opening of room, for a player who
/// wants no motion: a toast then appears and goes.
///
/// With the `serde` feature the settings are (de)serialisable, and a partial file fills every
/// missing field from the default.
///
/// ```
/// use bevy_gooey::ToastSettings;
/// // A player's "notification time: long" option.
/// let long = ToastSettings { duration_scale: 3.0, ..ToastSettings::default() };
/// assert_eq!(long.max_visible, 3);
/// ```
#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct ToastSettings {
    /// Where the stack stands. Default: bottom centre.
    pub anchor: ToastAnchor,
    /// The window's edges the stack keeps clear of, such as an app's menu bar under a top anchor.
    /// Default: none, the stack is the whole window.
    pub inset: ToastInset,
    /// How many toasts stand at once, at least one: a spawn beyond it sends the oldest into its fade
    /// out at once. Default: 3.
    pub max_visible: usize,
    /// Seconds to notice a toast and look at it, before any reading. Default: 2.
    pub base_seconds: f32,
    /// The reading speed a message's length is timed at, characters a second. Default: 20, the most
    /// Netflix lets timed text ask of an adult.
    pub chars_per_second: f32,
    /// The player's multiplier on how long a toast stands. Default: 1.
    pub duration_scale: f32,
    /// Seconds a toast takes to fade in, opening its room, and to fade out, closing it. Default:
    /// 0.3.
    pub fade_seconds: f32,
    /// The least a toast with an action stands fully shown, before `duration_scale`: a player must
    /// have time to reach a button (WCAG 2.2.1, timing adjustable). Default: 6.
    pub action_seconds: f32,
    /// A toast identical to one that arrived within this many seconds is merged into it, which then
    /// reads "×2", "×3". Zero turns merging off. Default: 2.
    pub merge_seconds: f32,
}

impl Default for ToastSettings {
    fn default() -> Self {
        Self {
            anchor: ToastAnchor::default(),
            inset: ToastInset::default(),
            max_visible: 3,
            base_seconds: 2.0,
            chars_per_second: 20.0,
            duration_scale: 1.0,
            fade_seconds: 0.3,
            action_seconds: 6.0,
            merge_seconds: 2.0,
        }
    }
}

/// What a toast tells the player; the mark down its left edge shows it (the words carry it too).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, PartialEq, Clone)]
pub enum ToastKind {
    /// Information, news: the mark is [`crate::GooeySkin::accent`].
    #[default]
    Info,
    /// A refusal or a failure: the mark is [`crate::GooeySkin::danger`].
    Alert,
}

/// A line of news at the edge of the screen, from [`crate::toast`] or [`crate::toast_alert`]. It
/// stands for as long as its message takes to read ([`ToastSettings`]), fades out, and is
/// despawned; despawn it yourself to take it back sooner.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn save(mut commands: Commands) {
///     commands.spawn(toast("Game saved"));
/// }
/// ```
///
/// Spawning one makes it a child of the one [`ToastStack`], which the first toast creates, in the
/// frame it is spawned: a game never lays one out, and spawns from any system. A toast takes no
/// input: every node of it is [`Pickable::IGNORE`], it is no tab stop and focus never goes to it,
/// so a press, a key or a D-pad direction reaches what is under it. It is drawn above a [`crate::dialog`]'s
/// scrim and below a tooltip. Its time runs on `Time<Real>`, so a paused game's menu still ages its
/// toasts.
///
/// This component is its clock and its look: read it, do not write it. It is not announced to
/// screen readers.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Debug, Clone)]
#[component(on_add = queue_join)]
pub struct Toast {
    kind: ToastKind,
    /// The message.
    text: String,
    /// The message's length in characters, which its reading time is made of.
    characters: usize,
    /// How many identical toasts this stands for, itself included: "×3".
    count: u32,
    /// Seconds since the last identical toast arrived, on real time: the merge window.
    since_last: f32,
    /// Stands until the player dismisses it.
    sticky: bool,
    /// Its buttons: a label and the id its [`ToastAction`] carries.
    actions: Vec<(String, String)>,
    /// A button was pressed: it fades out now.
    dismissed: bool,
    /// Seconds since it was spawned, on real time.
    age: f32,
    /// The longest it may stand fully shown: infinite until the stack is over `max_visible`.
    hold_cap: f32,
    alpha: f32,
    /// How much of its room is open, from 0 to 1: [`toast_grow`] of its age.
    grow: f32,
    /// How tall its body stands in full, in logical px, measured while its room opens or closes.
    body_px: f32,
}

impl Toast {
    /// A toast of `kind` for `text`, hidden until [`update_toasts`] first runs.
    pub(crate) fn new(kind: ToastKind, text: &str) -> Self {
        Self {
            kind,
            text: text.to_string(),
            characters: text.chars().count(),
            count: 1,
            since_last: 0.0,
            sticky: false,
            actions: Vec::new(),
            dismissed: false,
            age: 0.0,
            hold_cap: f32::INFINITY,
            alpha: 0.0,
            grow: 0.0,
            body_px: 0.0,
        }
    }

    /// What it tells the player.
    pub fn kind(&self) -> ToastKind {
        self.kind
    }

    /// The message, without its count.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// How many identical toasts it stands for, itself included ([`ToastSettings::merge_seconds`]).
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Whether it stands until the player dismisses it ([`ToastBundle::sticky`]).
    pub fn is_sticky(&self) -> bool {
        self.sticky
    }

    /// Its buttons, as `(label, id)`.
    pub fn actions(&self) -> &[(String, String)] {
        &self.actions
    }

    /// Seconds since it was spawned, on real time.
    pub fn age(&self) -> f32 {
        self.age
    }

    /// How opaque it is now, from 0 to 1: [`crate::toast_alpha`] of its age.
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// How much of its room is open, from 0 to 1: [`crate::toast_grow`] of its age. The toasts
    /// around it make way as it opens and close up as it goes, so none of them jumps.
    pub fn grow(&self) -> f32 {
        self.grow
    }

    /// How tall its body stands in full, in logical px, as last measured while its room was
    /// opening or closing; zero until it has been laid out once.
    pub fn body_px(&self) -> f32 {
        self.body_px
    }
}

/// The one column every [`Toast`] stands in: a pointer-transparent node the first toast spawns, the
/// window less [`ToastSettings::inset`], aligned by [`ToastSettings::anchor`] and kept above a
/// dialog (`GlobalZIndex` 150).
/// Query it to hide the toasts (a cutscene) by setting its `Visibility`.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ToastStack;

/// Above a [`crate::dialog`]'s scrim (100), so a message about what its button did is readable over
/// it, and below a tooltip (200).
const STACK_Z: i32 = 150;

/// The room a sticky toast holds: far longer than a session, and finite, because the fade's maths
/// treats a hold that is not finite as none.
const STICKY_HOLD_SECS: f32 = 1.0e9;

/// A toast, with the buttons it was built with: `commands.spawn(toast("Item deleted").action("Undo",
/// "undo"))`. [`crate::toast`] and [`crate::toast_alert`] return one; it is a bundle like any other, so
/// `commands.spawn(toast("Game saved"))` and `(toast("Game saved"), MyMarker)` work as ever.
///
/// A toast with no button takes no input and no focus. **One with a button has exactly one thing
/// that does: each button** (and a sticky toast's Dismiss), a tab stop the pointer, the keys and the
/// pad reach like any [`crate::button`]; the toast, its mark and its words still let everything
/// through.
#[derive(Bundle)]
pub struct ToastBundle {
    toast: Toast,
    node: Node,
    role: Role,
    pickable: Pickable,
}

impl ToastBundle {
    pub(crate) fn new(kind: ToastKind, text: String) -> Self {
        Self {
            toast: Toast::new(kind, &text),
            node: Node {
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                max_height: Val::Px(0.0),
                ..default()
            },
            role: Role::Toast,
            pickable: Pickable::IGNORE,
        }
    }

    /// Adds a button to the toast. Pressing it (Enter, Space, South or a click) sends one
    /// [`ToastAction`] with `id` and dismisses the toast. A toast with a button stands at least
    /// [`ToastSettings::action_seconds`], so the player can reach it; offer the same command
    /// somewhere that does not time out too.
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_gooey::*;
    /// fn delete(mut commands: Commands) {
    ///     commands.spawn(toast("Item deleted").action("Undo", "undo"));
    /// }
    ///
    /// fn undo(action: On<ToastAction>) {
    ///     if action.id == "undo" {
    ///         info!("undone");
    ///     }
    /// }
    /// ```
    pub fn action(mut self, label: impl Into<String>, id: impl Into<String>) -> Self {
        self.toast.actions.push((label.into(), id.into()));
        self
    }

    /// Makes the toast stand until the player dismisses it, with a Dismiss button: an error must not
    /// vanish before it is read. It does not count against [`ToastSettings::max_visible`], so a
    /// newer toast never pushes it out.
    ///
    /// ```
    /// # use bevy::prelude::*;
    /// # use bevy_gooey::*;
    /// fn failed(mut commands: Commands) {
    ///     commands.spawn(toast_alert("Could not save").sticky());
    /// }
    /// ```
    pub fn sticky(mut self) -> Self {
        self.toast.sticky = true;
        self
    }
}

/// A button of a [`Toast`] was pressed. Sent on the toast, and observable globally, once per press:
/// the toast is dismissed with it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn on_action(action: On<ToastAction>) {
///     info!("{} on {}", action.id, action.toast);
/// }
/// ```
#[derive(EntityEvent, Clone, Debug, PartialEq, Eq)]
pub struct ToastAction {
    /// The toast.
    #[event_target]
    pub toast: Entity,
    /// The id the button was added with ([`ToastBundle::action`]).
    pub id: String,
}

/// On a toast's button: which toast, and which id it sends (none for a sticky toast's Dismiss).
#[derive(Component, Clone, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct ToastButton {
    toast: Entity,
    id: Option<String>,
}

/// One line of the [`ToastLog`]: a toast that was spawned.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Debug, Clone, PartialEq)]
pub struct ToastLogEntry {
    /// Seconds on `Time<Real>` since the app started, when it was spawned.
    pub seconds: f32,
    /// What it told the player.
    pub kind: ToastKind,
    /// Its message.
    pub text: String,
}

/// The last [`ToastLog::CAPACITY`] toasts that were spawned, oldest first, whether or not they were
/// merged into another: a toast the player missed can be found here ([`crate::notification_center`]).
/// It is plain data, so a game may read it, save it or clear it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn count(log: Res<ToastLog>) -> usize {
///     log.entries().filter(|e| e.kind == ToastKind::Alert).count()
/// }
/// ```
#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Resource, Default, Debug, Clone, PartialEq)]
pub struct ToastLog {
    entries: VecDeque<ToastLogEntry>,
}

impl ToastLog {
    /// How many toasts it keeps.
    pub const CAPACITY: usize = 200;

    /// Every entry, oldest first.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &ToastLogEntry> {
        self.entries.iter()
    }

    /// How many entries it holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether it holds none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Forgets every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Adds an entry, forgetting the oldest past [`ToastLog::CAPACITY`].
    pub fn push(&mut self, entry: ToastLogEntry) {
        if self.entries.len() >= Self::CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }
}

/// A toast was just added: log it, merge it into an identical one that is standing, or make it a
/// child of the stack in this flush (creating the stack if none stands) and build its body, before
/// any frame can draw it unparented.
fn queue_join(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world.commands().queue(move |world: &mut World| {
        let Some(toast) = world.get::<Toast>(entity).cloned() else {
            return;
        };
        let seconds = world
            .get_resource::<Time<Real>>()
            .map_or(0.0, |time| time.elapsed_secs());
        if let Some(mut log) = world.get_resource_mut::<ToastLog>() {
            log.push(ToastLogEntry {
                seconds,
                kind: toast.kind,
                text: toast.text.clone(),
            });
        }
        let existing = world
            .query_filtered::<Entity, With<ToastStack>>()
            .iter(world)
            .next();
        let (window, inset) = world
            .get_resource::<ToastSettings>()
            .map_or((0.0, ToastInset::default()), |settings| {
                (settings.merge_seconds, settings.inset)
            });
        if let Some(stack) = existing
            && window > 0.0
            && merge_into_standing(world, stack, entity, &toast, window)
        {
            world.entity_mut(entity).despawn();
            return;
        }
        let stack = existing.unwrap_or_else(|| {
            let mut node = Node::default();
            inset.place(&mut node);
            world
                .spawn((
                    Name::new("ToastStack"),
                    node,
                    Role::ToastStack,
                    ToastStack,
                    Pickable::IGNORE,
                    GlobalZIndex(STACK_Z),
                ))
                .id()
        });
        world.entity_mut(entity).insert(ChildOf(stack));
        spawn_body(world, entity, &toast);
    });
}

/// Folds `new` into a toast in `stack` that says the same and arrived within `window` seconds: its
/// count goes up, its message reads "×n", and it stands again from the start. Whether one did.
fn merge_into_standing(
    world: &mut World,
    stack: Entity,
    new: Entity,
    toast: &Toast,
    window: f32,
) -> bool {
    let candidates: Vec<Entity> = world
        .get::<Children>(stack)
        .map(|kids| kids.iter().filter(|&k| k != new).collect())
        .unwrap_or_default();
    let found = candidates.into_iter().find(|&other| {
        world.get::<Toast>(other).is_some_and(|o| {
            !o.dismissed
                && o.since_last < window
                && o.kind == toast.kind
                && o.text == toast.text
                && o.sticky == toast.sticky
                && o.actions == toast.actions
        })
    });
    let Some(other) = found else {
        return false;
    };
    let fade = world
        .get_resource::<ToastSettings>()
        .map_or(0.0, |settings| settings.fade_seconds.max(0.0));
    let (text, count) = {
        let Some(mut standing) = world.get_mut::<Toast>(other) else {
            return false;
        };
        standing.count += 1;
        standing.since_last = 0.0;
        // Fully shown again, whole.
        standing.age = standing.age.min(fade);
        standing.hold_cap = f32::INFINITY;
        (standing.text.clone(), standing.count)
    };
    let mut stack_of_nodes = vec![other];
    while let Some(at) = stack_of_nodes.pop() {
        if world.get::<Role>(at) == Some(&Role::ToastText) {
            if let Some(mut words) = world.get_mut::<Text>(at) {
                words.0 = counted(&text, count);
            }
            break;
        }
        if let Some(kids) = world.get::<Children>(at) {
            stack_of_nodes.extend(kids.iter());
        }
    }
    true
}

/// A message with its count: "Saved" once, "Saved ×3" three times.
fn counted(text: &str, count: u32) -> String {
    if count > 1 {
        format!("{text} x{count}")
    } else {
        text.to_string()
    }
}

/// A toast's nodes: the body, a framed row of a mark, the message and any buttons, in the room
/// `entity` holds. Everything that is not a button starts transparent and transparent to the pointer.
fn spawn_body(world: &mut World, entity: Entity, toast: &Toast) {
    let body = world
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                ..default()
            },
            Role::ToastBody,
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Pickable::IGNORE,
            ChildOf(entity),
        ))
        .id();
    world.spawn((
        Node::default(),
        Role::ToastMark,
        BackgroundColor(Color::NONE),
        Pickable::IGNORE,
        ChildOf(body),
    ));
    world.spawn((
        Text::new(counted(&toast.text, toast.count)),
        TextColor(Color::NONE),
        Role::ToastText,
        Pickable::IGNORE,
        ChildOf(body),
    ));
    let buttons = toast
        .actions
        .iter()
        .map(|(label, id)| (label.clone(), Some(id.clone())))
        .chain(toast.sticky.then(|| ("Dismiss".to_string(), None)));
    for (label, id) in buttons {
        world
            .spawn((
                Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                bevy::ui_widgets::Button,
                Role::ToastAction,
                ToastButton { toast: entity, id },
                Hovered::default(),
                TabIndex(0),
                AutoDirectionalNavigation::default(),
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                ChildOf(body),
            ))
            .with_child((
                Text::new(label),
                TextColor(Color::NONE),
                Role::ToastActionText,
                crate::widgets::ONE_LINE,
            ));
    }
}

/// A toast's button is going: if it has focus, focus goes with it, so it never rests on nothing. A
/// global observer.
pub(crate) fn toast_button_removed(
    removed: On<Remove<ToastButton>>,
    mut focus: ResMut<InputFocus>,
) {
    if focus.get() == Some(removed.entity) {
        focus.clear();
    }
}

/// A toast's button was activated: send its action, if it has one, and dismiss the toast. A global
/// observer.
pub(crate) fn toast_on_activate(
    activate: On<Activate>,
    buttons: Query<&ToastButton>,
    mut toasts: Query<&mut Toast>,
    mut commands: Commands,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    let Ok(mut toast) = toasts.get_mut(button.toast) else {
        return;
    };
    if toast.dismissed {
        return;
    }
    toast.dismissed = true;
    if let Some(id) = &button.id {
        commands.trigger(ToastAction {
            toast: button.toast,
            id: id.clone(),
        });
    }
}

/// Ages every [`Toast`], fades it in and out, sends the oldest into its fade when more than
/// [`ToastSettings::max_visible`] stand, despawns the spent, and lays the stack out for
/// [`ToastSettings::anchor`]. Runs in [`crate::GooeySystems::Style`], before [`crate::apply_skin`],
/// which draws the result.
///
/// A toast is written only when its opacity or room changes, so one that is holding costs a clock
/// tick and nothing else, and nothing restyles it.
pub fn update_toasts(
    time: Res<Time<Real>>,
    settings: Res<ToastSettings>,
    mut commands: Commands,
    mut stacks: Query<(&mut Node, &Children), With<ToastStack>>,
    mut toasts: Query<(Entity, &mut Toast)>,
    kids: Query<&Children, With<Toast>>,
    bodies: Query<&ComputedNode, Without<Toast>>,
) {
    let delta = time.delta_secs();
    let usable = |x: f32| x.is_finite() && x > 0.0;
    let fade = if usable(settings.fade_seconds) {
        settings.fade_seconds
    } else {
        0.0
    };
    let action_hold = if usable(settings.action_seconds) && usable(settings.duration_scale) {
        settings.action_seconds * settings.duration_scale
    } else {
        0.0
    };
    let hold_of = |toast: &Toast| {
        let reading = toast_hold(
            toast.characters,
            settings.base_seconds,
            settings.chars_per_second,
            settings.duration_scale,
        );
        let hold = if toast.sticky {
            STICKY_HOLD_SECS
        } else if toast.actions.is_empty() {
            reading
        } else {
            reading.max(action_hold)
        };
        hold.min(toast.hold_cap)
    };
    let (direction, justify, across) = settings.anchor.layout();
    let max = settings.max_visible.max(1);

    for (mut node, children) in &mut stacks {
        if node.flex_direction != direction {
            node.flex_direction = direction;
        }
        if node.justify_content != justify {
            node.justify_content = justify;
        }
        if node.align_items != across {
            node.align_items = across;
        }
        if !settings.inset.holds(&node) || node.position_type != PositionType::Absolute {
            settings.inset.place(&mut node);
        }

        // The clock, and who is still fully shown (not yet fading out). A node under the stack
        // that is not a toast is skipped.
        let mut standing = 0usize;
        let mut live = toasts.iter_many_mut(children.iter());
        while let Some(item) = live.fetch_next() {
            let Ok((entity, mut toast)) = item else {
                continue;
            };
            // The clock is not a change worth a restyle: only the look below is.
            {
                let quiet = toast.bypass_change_detection();
                quiet.age += delta;
                quiet.since_last += delta;
                // A button was pressed: it begins to fade now, whole as it is.
                if quiet.dismissed && quiet.hold_cap.is_infinite() {
                    quiet.hold_cap = (quiet.age - fade).max(0.0);
                }
            }
            let hold = hold_of(&toast);
            if toast.age >= toast_life(hold, fade) {
                commands.entity(entity).despawn();
            } else if toast.age < fade + hold && !toast.sticky {
                standing += 1;
            }
        }

        // Too many: the oldest begin to fade now, whole as they are.
        let mut excess = standing.saturating_sub(max);
        let mut live = toasts.iter_many_mut(children.iter());
        while let Some(item) = live.fetch_next() {
            if excess == 0 {
                break;
            }
            let Ok((_, mut toast)) = item else {
                continue;
            };
            if !toast.sticky && toast.age < fade + hold_of(&toast) {
                let cut = (toast.age - fade).max(0.0);
                toast.bypass_change_detection().hold_cap = cut;
                excess -= 1;
            }
        }

        // How open its room is. While it opens or closes, its body is measured: laid out whole in
        // a toast whose height is held down, so the room it opens is the body's height.
        let mut live = toasts.iter_many_mut(children.iter());
        while let Some(item) = live.fetch_next() {
            let Ok((entity, mut toast)) = item else {
                continue;
            };
            let hold = hold_of(&toast);
            let alpha = toast_alpha(toast.age, hold, fade);
            let grow = toast_grow(toast.age, hold, fade);
            let body_px = if grow < 1.0 {
                kids.get(entity)
                    .ok()
                    .and_then(|kids| kids.first())
                    .and_then(|&body| bodies.get(body).ok())
                    .map_or(toast.body_px, |body| {
                        body.size().y * body.inverse_scale_factor()
                    })
            } else {
                toast.body_px
            };
            if toast.alpha != alpha || toast.grow != grow || toast.body_px != body_px {
                toast.alpha = alpha;
                toast.grow = grow;
                toast.body_px = body_px;
            }
        }
    }
}
