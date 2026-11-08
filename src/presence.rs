//! Presence: a widget that leaves stays drawn while it goes, and can change its mind.
//!
//! [`Opacity`] fades a node and everything the crate draws under it. [`Leave`] plays an exit and
//! despawns the node once it has faded out; [`Stay`] turns one round from wherever it had got to. A
//! fade that has settled writes nothing, and [`MotionSettings`](crate::MotionSettings) off makes both
//! land in the frame they are asked for.

use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::picking::Pickable;
use bevy::prelude::*;

use crate::menu::MenuPanel;
use crate::model::{Spring, SpringState, spring_settled, spring_step};
use crate::nav::Dialog;
use crate::popover::PopoverPanel;
use crate::skin::{GooeySkin, SkinScope, skin_of};

/// A fade this near 0 is gone: one step of 8-bit sRGB, so the frames an exit would draw invisible
/// are not drawn at all.
pub const PRESENCE_GONE: f32 = 1.0 / 255.0;

/// How opaque a node and everything the crate draws under it is: 1 is as the skin says, 0 is
/// invisible. Every colour [`apply_skin`](crate::apply_skin) writes on a [`Role`](crate::Role) node at
/// or below one is multiplied by the product of the `Opacity` at and above it, so a fade takes a
/// panel's fill, its frame, its words and the pictures in it together. A node without a `Role` is the
/// host's own drawing and is untouched.
///
/// A value outside 0 to 1 is held to the range, and one that is not a number reads as 1 (the node is
/// drawn as the skin says, rather than vanishing). Put it on a widget whose fade a game drives;
/// [`Leave`] writes one for you.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn ui(mut commands: Commands) {
/// // A HUD panel that thins out as the player's health does.
/// commands.spawn((panel(), Opacity(0.5), children![label("Low ammo")]));
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct Opacity(pub f32);

impl Default for Opacity {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Opacity {
    /// The factor as it is drawn: held to 0 to 1, and 1 for a value that is not a number.
    pub fn factor(&self) -> f32 {
        if self.0.is_nan() {
            1.0
        } else {
            self.0.clamp(0.0, 1.0)
        }
    }
}

/// Ask a node to leave: it keeps being drawn, fades out on the skin's
/// [`fade_seconds`](crate::MotionTokens::fade_seconds), and despawns once it has gone. In the frame
/// it is added the node and everything under it stop taking the pointer and focus, and a
/// [`Dialog`] gives its focus back then, so the screen under it works at once.
///
/// A `despawn()` still removes at once, for a node the player never saw arrive. Under
/// [`MotionSettings::off`](crate::MotionSettings::off) the node is gone in the frame it is asked.
/// The `Leave` is spent when the exit starts; a second one during the exit changes nothing, and
/// [`Stay`] reverses it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// # fn close(mut commands: Commands, dialog: Entity) {
/// // Instead of `commands.entity(dialog).despawn()`.
/// commands.entity(dialog).insert(Leave);
/// # }
/// ```
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct Leave;

/// Bring a leaving node back: its fade turns round from where it is drawn, and the pointer and focus
/// reach it again in that frame. Taken off with the exit once it has settled; a node that is not
/// leaving loses it in a frame and nothing else happens.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct Stay;

/// A node fading in where it stands: the entrance a dialog's card and a popover panel get under
/// [`MotionSettings::reduced`](crate::MotionSettings::reduced), in place of the slide that would
/// carry them. [`begin_entrances`](crate::begin_entrances) gives it with an [`Opacity`] of 0, and
/// [`update_arrivals`] raises that to 1 over [`fade_seconds`](crate::MotionTokens::fade_seconds)
/// times [`duration_scale`](crate::MotionSettings::duration_scale), on an ease-in-out, then takes
/// both off. A fixed-length fade, not a spring, as reduced motion's own fade is in Rune
/// (`REDUCED_FADE`): it lasts what it says. A [`Leave`] during it starts the exit from where the
/// fade had got to.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Arriving {
    /// Seconds of the fade drawn so far.
    elapsed: f32,
    /// The opacity the fade last wrote: one that differs was written by the host, which then owns
    /// the node's opacity.
    drawn: Opacity,
}

impl Arriving {
    /// A fade that has not started. Insert it with `Opacity(0.0)`.
    pub(crate) fn begin() -> (Self, Opacity) {
        let start = Opacity(0.0);
        (
            Self {
                elapsed: 0.0,
                drawn: start,
            },
            start,
        )
    }
}

/// How far through a fade of `length` seconds, `elapsed` in, a fixed-length ease-in-out stands:
/// 0 at the start, 1 at the end (smoothstep). A length that is not above zero is at its end.
fn arrival_at(elapsed: f32, length: f32) -> f32 {
    if !(length.is_finite() && length > 0.0) {
        return 1.0;
    }
    let t = (elapsed / length).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// What an exit took off one node of its subtree, to be put back by [`Stay`].
#[derive(Clone, Copy, Debug)]
struct Pose {
    entity: Entity,
    pickable: Option<Pickable>,
    tab: Option<TabIndex>,
}

/// A node on its way out: the fade it is playing, which end it is heading for, and what it took off
/// its subtree. Sits on the node [`Leave`] was put, until the exit settles.
///
/// A game reads [`Leaving::opacity`] for how far through its exit a widget is, and `With<Leaving>` to
/// know one is on its way out.
#[derive(Component, Clone, Debug)]
pub struct Leaving {
    fade: SpringState,
    /// 0 while it goes, 1 while it comes back.
    toward: f32,
    /// The [`Opacity`] the node had of its own before the exit took the value over.
    own: Option<Opacity>,
    /// The dialog it stopped being, to be one again on [`Stay`].
    dialog: Option<Dialog>,
    group: Option<TabGroup>,
    poses: Vec<Pose>,
}

impl Leaving {
    /// How opaque the node is drawn now, 0 to 1.
    pub fn opacity(&self) -> f32 {
        self.fade.value.clamp(0.0, 1.0)
    }

    /// Whether it is on its way back in rather than out.
    pub fn returning(&self) -> bool {
        self.toward > 0.5
    }
}

/// The opacity a node is drawn at: the product of every [`Opacity`] at and above it. Written by
/// [`spread_opacity`] and read by [`apply_skin`](crate::apply_skin); in the crate's public API only
/// because those two systems name it. A game writes [`Opacity`].
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PresenceAlpha(pub f32);

impl Default for PresenceAlpha {
    fn default() -> Self {
        Self(1.0)
    }
}

/// The shape of the tree a fade walks.
type Tree<'w, 's> = (
    Query<'w, 's, &'static Children>,
    Query<'w, 's, &'static ChildOf>,
);

/// A node whose own opacity just appeared or changed: its subtree is re-read.
type Touched = Or<(Added<Opacity>, Changed<Opacity>)>;

/// A node that has got a second [`Leave`] while it is already on its way out.
type Already = (Entity, Has<Stay>);

/// Gives every node under an [`Opacity`] the factor it is drawn at: the product of the `Opacity`s at
/// and above it. Runs only when an `Opacity` appears, changes or goes — which is every frame of a
/// fade, because [`update_presence`] writes one — so a settled interface walks no tree and writes
/// nothing. Runs in [`crate::GooeySystems::Style`], after [`update_presence`].
pub fn spread_opacity(
    mut commands: Commands,
    mut moved: Query<Entity, Touched>,
    leaving: Query<Entity, With<Leaving>>,
    opacities: Query<&Opacity>,
    alphas: Query<&PresenceAlpha>,
    tree: Tree,
    mut removed: RemovedComponents<Opacity>,
) {
    let (children, parents) = tree;
    let mut roots: Vec<Entity> = moved.iter_mut().collect();
    roots.extend(leaving.iter());
    roots.extend(removed.read());
    roots.sort_unstable();
    roots.dedup();
    for root in roots {
        let above = parents
            .iter_ancestors(root)
            .filter_map(|ancestor| opacities.get(ancestor).ok())
            .map(Opacity::factor)
            .product::<f32>();
        set_subtree(
            &mut commands,
            &children,
            &opacities,
            &alphas,
            root,
            above,
            0,
        );
    }
}

/// How deep [`set_subtree`] walks: a UI tree this deep is already a bug elsewhere.
const SUBTREE_DEPTH_LIMIT: usize = 64;

fn set_subtree(
    commands: &mut Commands,
    children: &Query<&Children>,
    opacities: &Query<&Opacity>,
    alphas: &Query<&PresenceAlpha>,
    entity: Entity,
    above: f32,
    depth: usize,
) {
    if depth > SUBTREE_DEPTH_LIMIT {
        warn!("bevy_gooey: a subtree deeper than {SUBTREE_DEPTH_LIMIT} was not given its opacity");
        return;
    }
    let own = opacities.get(entity).map_or(1.0, Opacity::factor);
    let alpha = above * own;
    let shown = alphas.get(entity).map_or(1.0, |shown| shown.0);
    if shown != alpha {
        commands.entity(entity).insert(PresenceAlpha(alpha));
    }
    if let Ok(kids) = children.get(entity) {
        for child in kids.iter() {
            set_subtree(
                commands,
                children,
                opacities,
                alphas,
                child,
                alpha,
                depth + 1,
            );
        }
    }
}

/// Everything a new exit reads of the node it sits on.
type Starting = (
    Entity,
    Option<&'static Opacity>,
    Option<&'static Dialog>,
    Option<&'static TabGroup>,
    Has<Arriving>,
    Has<MenuPanel>,
);

/// What an exit reads of one node of its subtree.
type Part = (Entity, Option<&'static Pickable>, Option<&'static TabIndex>);

/// The systems' arguments that are not a query.
type World<'w, 's> = (
    Res<'w, GooeySkin>,
    Res<'w, crate::MotionSettings>,
    ResMut<'w, InputFocus>,
    Query<'w, 's, &'static ChildOf>,
    Query<'w, 's, &'static Children>,
);

/// Starts an exit: the node and everything under it stop taking the pointer and focus, a dialog
/// gives its focus back, and the fade begins from where the node is drawn. Under
/// [`MotionSettings::off`](crate::MotionSettings::off), or a skin with no fade, the node is gone in
/// that frame instead. Runs in [`crate::GooeySystems::Style`], before [`update_presence`].
pub fn begin_leaves(
    world: World,
    mut commands: Commands,
    starting: Query<Starting, (With<Leave>, Without<Leaving>)>,
    parts: Query<Part>,
    already: Query<Already, (With<Leave>, With<Leaving>)>,
    mut second: Commands,
) {
    let (skin, motion, mut focus, parents, children) = world;
    // A `Leave` on a node already on its way out (or already coming back) is spent: it would start
    // a second exit the frame the first settled.
    for (entity, _) in &already {
        second.entity(entity).remove::<Leave>();
    }
    let spring = Spring::new(skin.motion.fade_seconds, 0.0).ok();
    let instant = motion.is_off() || spring.is_none();
    for (entity, own, dialog, group, arriving, menu_panel) in &starting {
        if instant {
            // A menu's panel is there for the next opening: it goes out of sight, not out of the
            // world.
            if menu_panel {
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    hide_menu_panel(world, entity)
                });
            } else {
                commands.entity(entity).try_despawn();
            }
            continue;
        }
        let mut subtree: Vec<Entity> = std::iter::once(entity)
            .chain(children.iter_descendants(entity))
            .collect();
        subtree.sort_unstable();
        let mut poses = Vec::with_capacity(subtree.len());
        for node in subtree {
            let (pickable, tab) = parts
                .get(node)
                .map(|(_, pickable, tab)| (pickable.copied(), tab.copied()))
                .unwrap_or((None, None));
            poses.push(Pose {
                entity: node,
                pickable,
                tab,
            });
            commands
                .entity(node)
                .insert(Pickable::IGNORE)
                .remove::<TabIndex>();
        }
        // Nothing of the exit holds the keyboard or the pointer: it is a dialog and a popover no
        // longer, so the screen under it works from this frame. Taking a `Dialog` off is what hands
        // focus back to the opener, exactly as the despawn it replaces did.
        commands
            .entity(entity)
            .remove::<(PopoverPanel, Dialog, TabGroup, Leave)>();
        // Anything else that had focus inside the exit has nowhere to put it, so it is let go.
        if dialog.is_none() && focus.get().is_some_and(|f| under(f, entity, &parents)) {
            focus.clear();
        }
        let from = own.map_or(1.0, Opacity::factor);
        // A node still fading in leaves from where its fade had got to; the opacity is the
        // arrival's, not one of its own to give back on `Stay`.
        if arriving {
            commands.entity(entity).remove::<Arriving>();
        }
        commands.entity(entity).insert((
            Opacity(from),
            Leaving {
                fade: SpringState {
                    value: from,
                    velocity: 0.0,
                },
                toward: 0.0,
                own: if arriving { None } else { own.copied() },
                dialog: dialog.cloned(),
                group: group.copied(),
                poses,
            },
        ));
    }
}

/// Whether `node` is `root` or one of its descendants.
fn under(node: Entity, root: Entity, parents: &Query<&ChildOf>) -> bool {
    node == root
        || parents
            .iter_ancestors(node)
            .any(|ancestor| ancestor == root)
}

/// Takes a node back from an exit: [`Stay`] turns the fade round from where it is drawn and gives the
/// pointer and focus back in that frame. Runs in [`crate::GooeySystems::Style`], before
/// [`update_presence`].
pub fn begin_stays(
    mut commands: Commands,
    staying: Query<Entity, With<Stay>>,
    exits: Query<(), With<Leaving>>,
    mut leaving: Query<&mut Leaving>,
) {
    for entity in &staying {
        if exits.get(entity).is_err() {
            // A `Stay` on a node that is not leaving is spent at once.
            commands.entity(entity).remove::<Stay>();
            continue;
        }
        let Ok(mut exit) = leaving.get_mut(entity) else {
            continue;
        };
        if exit.returning() {
            continue;
        }
        exit.bypass_change_detection().toward = 1.0;
        let poses = exit.poses.clone();
        let dialog = exit.dialog.take();
        let group = exit.group.take();
        for pose in poses {
            let mut node = commands.entity(pose.entity);
            match pose.pickable {
                Some(pickable) => node.insert(pickable),
                None => node.remove::<Pickable>(),
            };
            if let Some(tab) = pose.tab {
                node.insert(tab);
            }
        }
        // The dialog is a dialog again: it takes focus back on its own, as it did when it opened.
        if let Some(dialog) = dialog {
            commands.entity(entity).insert(dialog);
        }
        if let Some(group) = group {
            commands.entity(entity).insert(group);
        }
    }
}

/// What the fade system reads of a leaving node.
type Exiting = (
    Entity,
    &'static mut Leaving,
    &'static mut Opacity,
    Option<&'static Visibility>,
    Has<MenuPanel>,
);

/// Steps every [`Leaving`] fade on real time, writes its [`Opacity`], and despawns the node the frame
/// it has gone; a node brought back has everything put back and is left at rest. Writes only on a
/// difference, and an exit with nothing left to do is skipped before anything is read. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`].
pub fn update_presence(
    skin: Res<GooeySkin>,
    motion: Res<crate::MotionSettings>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
    mut exiting: Query<Exiting>,
) {
    if exiting.is_empty() {
        return;
    }
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let off = motion.is_off() || time.is_none();
    for (entity, mut exit, mut opacity, visibility, menu_panel) in &mut exiting {
        // A hidden node's fade waits: a popover is drawn a frame after it is spawned.
        let spring = Spring::new(
            skin_of(&skin, entity, &scopes, &parents)
                .motion
                .fade_seconds,
            0.0,
        )
        .map(|spring| spring.scaled(motion.duration_scale))
        .ok();
        let goal = exit.toward;
        let waiting = spring.is_none() || off || visibility == Some(&Visibility::Hidden);
        let next = match spring {
            Some(spring) if !waiting => spring_step(spring, exit.fade, goal, delta),
            _ => SpringState {
                value: goal,
                velocity: 0.0,
            },
        };
        let landed = waiting
            || spring.is_some_and(|spring| spring_settled(spring, next, goal, PRESENCE_GONE));
        if goal <= 0.0 && landed {
            if menu_panel {
                // A menu's panel stays for its next opening: everything the exit took off it is
                // put back, and it goes out of sight.
                for pose in std::mem::take(&mut exit.poses) {
                    let mut node = commands.entity(pose.entity);
                    match pose.pickable {
                        Some(pickable) => node.insert(pickable),
                        None => node.remove::<Pickable>(),
                    };
                    if let Some(tab) = pose.tab {
                        node.insert(tab);
                    }
                }
                let own = exit.own.take();
                commands.entity(entity).remove::<(Leaving, Stay)>();
                match own {
                    Some(own) => {
                        if *opacity != own {
                            *opacity = own;
                        }
                    }
                    None => {
                        commands.entity(entity).remove::<Opacity>();
                    }
                }
                commands.queue(move |world: &mut bevy::ecs::world::World| {
                    hide_menu_panel(world, entity)
                });
            } else {
                commands.entity(entity).try_despawn();
            }
            continue;
        }
        if goal > 0.0 && landed {
            // Back, at rest: the exit is over and the node keeps the opacity it had before it. A
            // node that had none loses the one the fade wrote, so it is drawn whole again and
            // `spread_opacity` re-reads its subtree.
            let own = exit.own.take();
            let mut back = commands.entity(entity);
            back.remove::<(Leaving, Stay)>();
            match own {
                Some(own) => {
                    if *opacity != own {
                        *opacity = own;
                    }
                }
                None => {
                    back.remove::<Opacity>();
                }
            }
            continue;
        }
        if exit.fade != next {
            exit.bypass_change_detection().fade = next;
        }
        let value = Opacity(next.value.clamp(PRESENCE_GONE, 1.0));
        if *opacity != value {
            *opacity = value;
        }
    }
}

/// What the arrival system reads of a node fading in.
type Fading = (
    Entity,
    &'static mut Arriving,
    &'static mut Opacity,
    Option<&'static Visibility>,
);

/// Steps every [`Arriving`] fade on real time and writes its [`Opacity`]; at the end of the fade
/// both come off, so the node is drawn whole and [`spread_opacity`] re-reads its subtree once. A
/// hidden node's fade waits (a popover is drawn a frame after it is spawned), and with motion off
/// or no clock it lands at once. Writes only on a difference. Runs in
/// [`crate::GooeySystems::Style`], after [`update_presence`] and before [`spread_opacity`].
pub fn update_arrivals(
    skin: Res<GooeySkin>,
    motion: Res<crate::MotionSettings>,
    scopes: Query<&SkinScope>,
    parents: Query<&ChildOf>,
    time: Option<Res<Time<Real>>>,
    mut commands: Commands,
    mut fading: Query<Fading, Without<Leaving>>,
) {
    if fading.is_empty() {
        return;
    }
    let delta = time.as_ref().map_or(0.0, |time| time.delta_secs());
    let off = motion.is_off() || time.is_none();
    for (entity, mut arrival, mut opacity, visibility) in &mut fading {
        // A host that wrote the node's opacity during the fade has taken it over: the fade stops
        // and the host's value stays.
        if *opacity != arrival.drawn {
            commands.entity(entity).remove::<Arriving>();
            continue;
        }
        if visibility == Some(&Visibility::Hidden) && !off {
            continue;
        }
        let length = skin_of(&skin, entity, &scopes, &parents)
            .motion
            .fade_seconds
            * motion.duration_scale;
        let elapsed = arrival.elapsed + delta;
        // A thousandth of a frame short of the end is the end: the sum of frame times rounds. A
        // length that is not a finite number above zero has no middle.
        let done = off
            || !(length.is_finite() && length > 0.0)
            || elapsed + ARRIVAL_END_SLACK_SECONDS >= length;
        if done {
            commands.entity(entity).remove::<(Arriving, Opacity)>();
            continue;
        }
        let value = Opacity(arrival_at(elapsed, length));
        *arrival = Arriving {
            elapsed,
            drawn: value,
        };
        if *opacity != value {
            *opacity = value;
        }
    }
}

/// How far short of a fade's length its summed frame times may fall and still be at its end, in
/// seconds: `f32` noise on a sum of frames, far under a frame.
const ARRIVAL_END_SLACK_SECONDS: f32 = 1.0e-5;

/// Takes a menu's panel out of sight once its exit is over (or at once, with no motion): hidden, not
/// laid out, ready for the next opening.
pub(crate) fn hide_menu_panel(world: &mut bevy::ecs::world::World, panel: Entity) {
    if let Ok(mut entity) = world.get_entity_mut(panel) {
        entity.insert(Visibility::Hidden);
        if let Some(mut node) = entity.get_mut::<Node>()
            && node.display != Display::None
        {
            node.display = Display::None;
        }
    }
}
