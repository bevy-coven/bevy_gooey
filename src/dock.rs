//! The dock: a layout kept as data ([`DockLayout`]) that [`crate::dock`] builds from [`split`](crate::split)s
//! and [`tabs`](crate::tabs) and the player rearranges by picking up a tab. A panel's page is
//! spawned once, from the [`PanelRegistry`], and moved, so what it holds survives a rearrangement.
//! Its look is [`crate::apply_skin`]'s, like every widget's.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use bevy::ecs::relationship::RelatedSpawner;
use bevy::ecs::spawn::SpawnWith;
use bevy::ecs::system::SystemParam;
use bevy::input::ButtonInput;
use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform, UiTransform};
use bevy::ui_widgets::{ValueChange, observe};

use crate::cancel::Cancel;
use crate::dock_layout::{DockDropTarget, DockLayout, DockSkip};
use crate::layout_motion::{MovesSmoothly, TravelFrom};
use crate::model::{
    DOCK_PANE_MIN_PX, DockZone, MIN_TARGET_PX, dock_drop_rect, dock_edge_zone, dock_neighbour,
    dock_tab_slot, dock_zone,
};
use crate::motion::{MotionTarget, MotionToken};
use crate::nav::show_focus;
use crate::skin::Role;
use crate::split::{SplitFraction, SplitMin};
use crate::widgets::{panel, split_in, tab_page, tabs_in};

/// How far right of and below the pointer the ghost of a dragged tab is drawn, in logical pixels.
const GHOST_OFFSET_PX: f32 = 14.0;

/// A function that fills a panel's page: it gets the page's entity, a column that fills its tab
/// group, and adds the widgets (`page.with_child(label("hi"))`, `page.insert(children![..])`).
type SpawnPanel = Arc<dyn Fn(&mut EntityCommands) + Send + Sync>;

#[derive(Clone)]
struct PanelDef {
    title: String,
    spawn: SpawnPanel,
}

/// The panels a game offers a [`crate::dock`]: an id, the title on its tab and the function that
/// fills its page. A layout names panels by id, so it can be saved and loaded as plain data; an id
/// no panel is registered for is skipped and reported ([`DockPanelSkipped`]).
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn setup(mut commands: Commands, mut panels: ResMut<PanelRegistry>) {
///     panels
///         .register("scene", "Scene", |page| {
///             page.with_child(label("The scene"));
///         })
///         .register("log", "Log", |page| {
///             page.with_child(label("Nothing yet"));
///         });
///     commands.spawn(dock(DockLayout::split(
///         SplitAxis::Vertical,
///         0.7,
///         DockLayout::group(["scene"]),
///         DockLayout::group(["log"]),
///     )));
/// }
/// ```
#[derive(Resource, Default, Clone)]
pub struct PanelRegistry {
    panels: HashMap<String, PanelDef>,
}

impl fmt::Debug for PanelRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ids: Vec<&str> = self.panels.keys().map(String::as_str).collect();
        ids.sort_unstable();
        f.debug_struct("PanelRegistry")
            .field("panels", &ids)
            .finish()
    }
}

impl PanelRegistry {
    /// Registers the panel `id`, titled `title` on its tab, whose page `spawn` fills: the function
    /// runs once, when a dock first shows the panel, and the page it filled is moved, not rebuilt,
    /// when the player drags the panel elsewhere. Registering an id again replaces it for docks
    /// that have not shown it yet.
    pub fn register(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        spawn: impl Fn(&mut EntityCommands) + Send + Sync + 'static,
    ) -> &mut Self {
        self.panels.insert(
            id.into(),
            PanelDef {
                title: title.into(),
                spawn: Arc::new(spawn),
            },
        );
        self
    }

    /// Whether a panel is registered under `id`.
    pub fn contains(&self, id: &str) -> bool {
        self.panels.contains_key(id)
    }

    /// The title of the panel `id`, or the id itself when none is registered.
    pub fn title<'a>(&'a self, id: &'a str) -> &'a str {
        self.panels.get(id).map_or(id, |panel| panel.title.as_str())
    }
}

/// A [`crate::dock`]'s layout: the state. Write a [`DockLayout`] to it to load or reset the
/// layout (the dock shows it the same frame; panels already shown are kept and moved, the rest are
/// spawned and the ones no longer named despawned); read it, or observe [`DockLayoutChanged`], to
/// save what the player did.
///
/// What a dock shows is this layout without the panels it cannot show (unknown or repeated ids), a
/// group left empty collapsed ([`DockLayout::prune`]); a change the player makes is made to that
/// and written back here.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(from_reflect = false)]
#[reflect(Component)]
#[require(DockShown)]
pub struct Dock(#[reflect(ignore)] pub DockLayout);

/// The page of a panel, in the dock that shows it: put on the entity [`PanelRegistry::register`]'s
/// function fills, so a game can find a panel's widgets by its id.
#[derive(Component, Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component)]
pub struct DockPanel(pub String);

/// The player changed a [`Dock`]'s layout, by dropping a tab (in a group, beside one, at the
/// dock's edge or on a tab strip), moving a divider (when it is let go, and for each step of a key)
/// or picking a tab. It holds the layout now shown: save it.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_gooey::*;
/// fn save(changed: On<DockLayoutChanged>) {
///     info!("the layout is now {:?}", changed.layout.panels());
/// }
/// ```
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct DockLayoutChanged {
    /// The dock.
    #[event_target]
    pub entity: Entity,
    /// Its layout now.
    pub layout: DockLayout,
}

/// A panel the dock was told to show but could not: sent once for each, when the dock is built from
/// a layout that names it, so a game finds a typo or a panel it forgot to register.
#[derive(EntityEvent, Clone, Debug, PartialEq)]
pub struct DockPanelSkipped {
    /// The dock.
    #[event_target]
    pub entity: Entity,
    /// The panel's id.
    pub id: String,
    /// Why it is not shown.
    pub reason: DockSkip,
}

/// What a dock has built: the layout it shows (the [`Dock`] without what it could not show), the
/// entity at the top of the tree it spawned, the pages of its panels and the tab to give focus to
/// once a rearrangement has rebuilt the strips.
#[derive(Component, Default)]
pub(crate) struct DockShown {
    built: bool,
    layout: Option<DockLayout>,
    root: Option<Entity>,
    pages: HashMap<String, Entity>,
    /// The panel whose tab takes focus, and whether the strips have been rebuilt since it moved.
    refocus: Option<(String, bool)>,
    /// Where each tab was drawn (its centre, physical window px) when the tree was last rebuilt, so
    /// the new tab of the same panel glides from there ([`TravelFrom`]).
    departed: HashMap<String, Vec2>,
}

/// Where a split or a group stands in its dock: the dock and the way down the tree (`false` first
/// node, `true` second).
#[derive(Component, Clone, Debug)]
pub(crate) struct DockNode {
    dock: Entity,
    path: Vec<bool>,
}

/// A dock's group: the card that holds a strip of tabs and their pages.
#[derive(Component)]
pub(crate) struct DockGroup;

/// A tab of a dock's group: the panel it picks and the dock it is in.
#[derive(Component, Clone, Debug)]
pub(crate) struct DockTab {
    dock: Entity,
    panel: String,
    /// Where the tab it replaces was drawn, for [`start_dock_travel`] to turn into a
    /// [`TravelFrom`] the frame it appears.
    from: Option<Vec2>,
}

/// A tab being carried: on the dock, from the press that picked it up to the let go.
#[derive(Component, Clone, Debug)]
pub(crate) struct DockDrag {
    panel: String,
    /// The pointer, in window pixels.
    pointer: Vec2,
    /// Where the panel will go if the pointer is let go now.
    target: Option<DockDropTarget>,
    /// The strip the tab is carried out of, as laid out when it was picked up: what the tabs it
    /// passes step aside from.
    strip: Option<StripDrag>,
}

/// One tab of a strip, as laid out when a tab of it was picked up: its entity, centre and size, in
/// layout pixels.
#[derive(Clone, Debug)]
struct StripTab {
    entity: Entity,
    centre: Vec2,
    size: Vec2,
}

/// A tab being carried within its strip (G-63): the tabs of the strip in order, which one it is and
/// where on it the pointer took hold.
#[derive(Clone, Debug)]
struct StripDrag {
    tabs: Vec<StripTab>,
    from: usize,
    grab: Vec2,
}

/// The entities of an existing node adopted as the children of the one this is inserted in, in
/// order: how a dock puts a page it already spawned into a group it builds.
pub(crate) fn adopt(entities: Vec<Entity>) -> impl Bundle {
    Children::spawn(SpawnWith(move |parent: &mut RelatedSpawner<ChildOf>| {
        let target = parent.target_entity();
        for entity in entities {
            parent
                .world_mut()
                .entity_mut(entity)
                .insert(ChildOf(target));
        }
    }))
}

/// Builds each [`Dock`] from its layout: when the layout, or the registry, changed, and the layout
/// shown differs from what it would show now. Panels already shown keep their page and move; a
/// panel named for the first time is spawned from the registry. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::update_tabs`] and [`crate::update_splits`], so
/// a new dock is laid out in the frame it is built. A dock whose layout and registry did not
/// change costs nothing.
pub(crate) fn update_docks(
    registry: Res<PanelRegistry>,
    mut docks: Query<(Entity, Ref<Dock>, &mut DockShown, &Children)>,
    roles: Query<&Role>,
    tabs: Query<(&DockTab, &UiGlobalTransform)>,
    mut commands: Commands,
) {
    for (entity, dock, mut shown, kids) in &mut docks {
        if shown.built && !dock.is_changed() && !registry.is_changed() {
            continue;
        }
        let (layout, skipped) = dock.0.prune(|id| registry.contains(id));
        if shown.built && layout == shown.layout {
            continue;
        }
        let Some(body) = kids
            .iter()
            .find(|&kid| roles.get(kid) == Ok(&Role::DockBody))
        else {
            continue;
        };
        let wanted = layout.as_ref().map(DockLayout::panels).unwrap_or_default();
        let mut pages = HashMap::new();
        for (id, page) in std::mem::take(&mut shown.pages) {
            if wanted.contains(&id.as_str()) {
                commands.entity(page).remove::<ChildOf>();
                pages.insert(id, page);
            } else {
                commands.entity(page).despawn();
            }
        }
        for id in wanted {
            if !pages.contains_key(id) {
                let mut page = commands.spawn((tab_page(()), DockPanel(id.to_owned())));
                if let Some(def) = registry.panels.get(id) {
                    (def.spawn)(&mut page);
                }
                pages.insert(id.to_owned(), page.id());
            }
        }
        // Where each tab is drawn now: the tab of the same panel in the new tree glides from there.
        shown.departed = tabs
            .iter()
            .filter(|(tab, _)| tab.dock == entity)
            .map(|(tab, at)| (tab.panel.clone(), at.translation))
            .collect();
        if let Some(old) = shown.root.take() {
            commands.entity(old).despawn();
        }
        let departed = shown.departed.clone();
        let root = layout.as_ref().map(|layout| {
            spawn_node(
                &mut commands,
                entity,
                &registry,
                &pages,
                &departed,
                layout,
                Vec::new(),
            )
        });
        if let Some(root) = root {
            commands.entity(body).add_child(root);
        }
        for (id, reason) in skipped {
            warn!("dock: panel {id:?} is not shown ({reason:?})");
            commands.trigger(DockPanelSkipped { entity, id, reason });
        }
        if let Some((_, rebuilt)) = shown.refocus.as_mut() {
            *rebuilt = true;
        }
        shown.built = true;
        shown.layout = layout;
        shown.root = root;
        shown.pages = pages;
    }
}

/// Spawns the node `layout` is, and what is under it: a split of two nodes, or a group card of a
/// strip of tabs over the panels' pages. Returns the node's entity.
fn spawn_node(
    commands: &mut Commands,
    dock: Entity,
    registry: &PanelRegistry,
    pages: &HashMap<String, Entity>,
    departed: &HashMap<String, Vec2>,
    layout: &DockLayout,
    path: Vec<bool>,
) -> Entity {
    match layout {
        DockLayout::Split {
            axis,
            fraction,
            first,
            second,
        } => {
            let down = |second: bool| {
                let mut path = path.clone();
                path.push(second);
                path
            };
            let a = spawn_node(
                commands,
                dock,
                registry,
                pages,
                departed,
                first,
                down(false),
            );
            let b = spawn_node(
                commands,
                dock,
                registry,
                pages,
                departed,
                second,
                down(true),
            );
            commands
                .spawn((
                    split_in(*axis, adopt(vec![a]), adopt(vec![b])),
                    SplitFraction(*fraction),
                    SplitMin::new(DOCK_PANE_MIN_PX, DOCK_PANE_MIN_PX),
                    DockNode { dock, path },
                ))
                .id()
        }
        DockLayout::Group { panels, selected } => {
            let labels: Vec<(String, DockTab)> = panels
                .iter()
                .map(|id| {
                    (
                        registry.title(id).to_owned(),
                        DockTab {
                            dock,
                            panel: id.clone(),
                            from: departed.get(id).copied(),
                        },
                    )
                })
                .collect();
            let shown: Vec<Entity> = panels
                .iter()
                .filter_map(|id| pages.get(id))
                .copied()
                .collect();
            commands
                .spawn((
                    panel(),
                    DockGroup,
                    DockNode { dock, path },
                    children![tabs_in(
                        labels.into_iter().map(|(title, tab)| {
                            (
                                title,
                                (
                                    tab,
                                    observe(tab_on_drag_start),
                                    observe(tab_on_drag),
                                    observe(tab_on_drag_end),
                                    observe(tab_on_pointer_cancel),
                                    observe(tab_on_cancel),
                                    observe(tab_on_key),
                                ),
                            )
                        }),
                        *selected,
                        adopt(shown),
                        true,
                    )],
                ))
                .id()
        }
    }
}

/// Which tab of a group the player picked: written into the layout, and reported.
pub(crate) fn dock_on_select(
    change: On<ValueChange<usize>>,
    parents: Query<&ChildOf>,
    nodes: Query<&DockNode>,
    mut docks: Query<(&mut Dock, &mut DockShown)>,
    mut commands: Commands,
) {
    let Some(node) = parents
        .get(change.source)
        .ok()
        .and_then(|parent| nodes.get(parent.parent()).ok())
    else {
        return;
    };
    let value = change.value;
    commit(
        &mut docks,
        node.dock,
        &mut commands,
        |layout| match layout.node_mut(&node.path) {
            Some(DockLayout::Group { selected, .. }) if *selected != value => {
                *selected = value;
                true
            }
            _ => false,
        },
    );
}

/// Where a divider was let go: written into the layout, and reported. A divider still being dragged
/// changes nothing here: the layout is the player's when they let go.
pub(crate) fn dock_on_resize(
    change: On<ValueChange<f32>>,
    nodes: Query<&DockNode>,
    mut docks: Query<(&mut Dock, &mut DockShown)>,
    mut commands: Commands,
) {
    if !change.is_final {
        return;
    }
    let Ok(node) = nodes.get(change.source) else {
        return;
    };
    let value = change.value;
    commit(
        &mut docks,
        node.dock,
        &mut commands,
        |layout| match layout.node_mut(&node.path) {
            Some(DockLayout::Split { fraction, .. }) if *fraction != value => {
                *fraction = value;
                true
            }
            _ => false,
        },
    );
}

/// Applies `edit` to the layout the dock shows (it says whether it changed anything), makes it the
/// dock's layout and reports it. The structure on screen already is this layout, so it is not
/// rebuilt.
fn commit(
    docks: &mut Query<(&mut Dock, &mut DockShown)>,
    dock: Entity,
    commands: &mut Commands,
    edit: impl FnOnce(&mut DockLayout) -> bool,
) {
    let Ok((mut state, mut shown)) = docks.get_mut(dock) else {
        return;
    };
    // A move waiting to be built replaces what is on screen: an edit to it is lost with it. Bevy's
    // own handler of the arrow also picks the neighbouring tab for Ctrl+Shift and an arrow, and
    // that pick arrives here after the move has been made.
    if shown.refocus.is_some() {
        return;
    }
    let Some(layout) = shown.layout.as_mut() else {
        return;
    };
    if edit(layout) {
        let layout = layout.clone();
        state.0 = layout.clone();
        commands.trigger(DockLayoutChanged {
            entity: dock,
            layout,
        });
    }
}

/// Moves a panel by dropping it on `target`: the dock's layout becomes the one that makes, which
/// [`update_docks`] builds, and focus goes back to the panel's tab. Does nothing when the drop
/// changes nothing.
fn move_panel(
    docks: &mut Query<(&mut Dock, &mut DockShown)>,
    dock: Entity,
    panel: &str,
    target: &DockDropTarget,
    commands: &mut Commands,
) {
    let Ok((mut state, mut shown)) = docks.get_mut(dock) else {
        return;
    };
    let Some(layout) = shown.layout.as_ref().and_then(|l| l.moved(panel, target)) else {
        return;
    };
    state.0 = layout.clone();
    shown.refocus = Some((panel.to_owned(), false));
    commands.trigger(DockLayoutChanged {
        entity: dock,
        layout,
    });
}

/// What the drop target and the overlay are found from: where each of a dock's groups and tabs was
/// laid out.
#[derive(SystemParam)]
pub(crate) struct DockGeometry<'w, 's> {
    ui_scale: Option<Res<'w, UiScale>>,
    groups: Query<
        'w,
        's,
        (
            Entity,
            &'static DockNode,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static Children,
        ),
        With<DockGroup>,
    >,
    children: Query<'w, 's, &'static Children>,
    roles: Query<'w, 's, &'static Role>,
    tabs: Query<
        'w,
        's,
        (
            &'static DockTab,
            &'static ComputedNode,
            &'static UiGlobalTransform,
            &'static UiTransform,
        ),
    >,
    frames: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform)>,
    parents: Query<'w, 's, &'static ChildOf>,
}

/// A group as laid out: its card, its strip and its tabs' ids and centres.
struct GroupSeen {
    entity: Entity,
    card: Rect,
    strip: Rect,
    tabs: Vec<(String, f32, f32, f32)>,
}

/// Where a drop would land: the target, and the part of the screen it takes, in layout pixels.
struct Landing {
    target: DockDropTarget,
    rect: Rect,
}

impl DockGeometry<'_, '_> {
    /// The interface's scale: one where nothing sets [`UiScale`].
    fn scale(&self) -> f32 {
        self.ui_scale.as_ref().map_or(1.0, |scale| scale.0)
    }

    fn rect(&self, entity: Entity) -> Option<Rect> {
        let (node, at) = self.frames.get(entity).ok()?;
        let scale = node.inverse_scale_factor();
        Some(Rect::from_center_size(
            at.translation * scale,
            node.size() * scale,
        ))
    }

    /// Every group of `dock`, laid out.
    fn groups_of(&self, dock: Entity) -> Vec<GroupSeen> {
        self.groups
            .iter()
            .filter(|(_, node, ..)| node.dock == dock)
            .filter_map(|(entity, _, _, _, kids)| {
                let root = kids.first().copied()?;
                let parts = self.children.get(root).ok()?;
                let strip = parts
                    .iter()
                    .find(|&part| self.roles.get(part) == Ok(&Role::TabList))?;
                let tabs = self
                    .children
                    .get(strip)
                    .ok()?
                    .iter()
                    .filter_map(|tab| {
                        let (tab, node, at, moved) = self.tabs.get(tab).ok()?;
                        let scale = node.inverse_scale_factor();
                        let half = node.size().y * scale / 2.0;
                        // Where layout put it: a tab being carried, or one stepping aside for it,
                        // is drawn elsewhere, and the slot under the pointer is the layout's.
                        let centre = at.translation * scale
                            - Vec2::new(px_of(moved.translation.x), px_of(moved.translation.y));
                        Some((
                            tab.panel.clone(),
                            centre.x,
                            centre.y - half,
                            centre.y + half,
                        ))
                    })
                    .collect();
                Some(GroupSeen {
                    entity,
                    card: self.rect(entity)?,
                    strip: self.rect(strip)?,
                    tabs,
                })
            })
            .collect()
    }

    /// Where dropping `panel` with the pointer at `pointer` (window pixels) lands in `dock`, which
    /// shows `layout`: at the dock's edge, on a strip, in a group or against its edge. `None` when
    /// the pointer is off the dock or the drop would change nothing.
    fn landing(
        &self,
        dock: Entity,
        pointer: Vec2,
        panel: &str,
        layout: &DockLayout,
    ) -> Option<Landing> {
        let at = pointer / self.scale();
        let whole = self.rect(dock)?;
        if !whole.contains(at) {
            return None;
        }
        let rel = at - whole.min;
        let groups = self.groups_of(dock);
        let group = groups.iter().find(|g| g.card.contains(at));
        // A tab strip first: the first slot of the first group is closer to the dock's edge than
        // the band is thick, and the strip is where a tab is meant to go.
        let landing = if let Some(group) = group.filter(|g| at.y <= g.strip.max.y) {
            let slots: Vec<(f32, f32, f32)> = group
                .tabs
                .iter()
                .map(|&(_, x, top, bottom)| (x, top, bottom))
                .collect();
            Landing {
                target: DockDropTarget {
                    onto: Some(group.tabs.first().map(|(id, ..)| id.clone())?),
                    zone: DockZone::Into,
                    index: Some(dock_tab_slot(&slots, at.x, at.y)),
                },
                rect: group.card,
            }
        } else if let Some(zone) =
            dock_edge_zone(whole.width(), whole.height(), rel.x, rel.y, MIN_TARGET_PX)
        {
            Landing {
                target: DockDropTarget {
                    onto: None,
                    zone,
                    index: None,
                },
                rect: part(whole, zone, true),
            }
        } else {
            let group = group?;
            let zone = dock_zone(
                group.card.width(),
                group.card.height(),
                at.x - group.card.min.x,
                at.y - group.card.min.y,
            );
            Landing {
                target: DockDropTarget {
                    onto: Some(group.tabs.first().map(|(id, ..)| id.clone())?),
                    zone,
                    index: None,
                },
                rect: part(group.card, zone, false),
            }
        };
        layout.moved(panel, &landing.target).map(|_| landing)
    }

    /// The group a tab is in, and every other group of its dock, laid out.
    fn group_of_tab(&self, tab: Entity) -> Option<(Entity, Vec<GroupSeen>)> {
        let dock = self.tabs.get(tab).ok()?.0.dock;
        let strip = self.parents.get(tab).ok()?.parent();
        let root = self.parents.get(strip).ok()?.parent();
        let card = self.parents.get(root).ok()?.parent();
        Some((card, self.groups_of(dock)))
    }
}

/// The part of `rect` a drop in `zone` takes ([`dock_drop_rect`]).
fn part(rect: Rect, zone: DockZone, whole: bool) -> Rect {
    let (x, y, w, h) = dock_drop_rect(zone, rect.width(), rect.height(), whole);
    Rect::from_corners(
        rect.min + Vec2::new(x, y),
        rect.min + Vec2::new(x + w, y + h),
    )
}

/// What the strip a carried tab came out of is read from.
type StripTabs<'w, 's> = Query<
    'w,
    's,
    (
        &'static DockTab,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static ChildOf,
    ),
>;

/// A press that moves picks the tab up: the dock starts carrying it.
fn tab_on_drag_start(
    mut start: On<PointerDragStart>,
    tabs: StripTabs,
    kids: Query<&Children>,
    ui_scale: Option<Res<UiScale>>,
    mut commands: Commands,
) {
    start.propagate(false);
    if start.button != PointerButton::Primary {
        return;
    }
    let Ok((tab, _, _, strip)) = tabs.get(start.entity) else {
        return;
    };
    let scale = ui_scale.map_or(1.0, |scale| scale.0);
    // The strip as it stands now, before any tab has moved: the tabs the carried one passes step
    // aside from these places.
    let seen: Vec<StripTab> = kids
        .get(strip.parent())
        .map(|kids| {
            kids.iter()
                .filter_map(|kid| {
                    let (_, node, at, _) = tabs.get(kid).ok()?;
                    let to_layout = node.inverse_scale_factor();
                    Some(StripTab {
                        entity: kid,
                        centre: at.translation * to_layout,
                        size: node.size() * to_layout,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let pointer = start.pointer.position / scale;
    let strip = seen
        .iter()
        .position(|t| t.entity == start.entity)
        .map(|from| StripDrag {
            grab: pointer - seen[from].centre,
            from,
            tabs: seen,
        });
    commands.entity(tab.dock).insert(DockDrag {
        panel: tab.panel.clone(),
        pointer: start.pointer.position,
        target: None,
        strip,
    });
}

/// The carried tab follows the pointer. The place the tab was taken hold of is where the press was,
/// not where the pointer had got to when the drag began (a drag begins a few pixels after the press).
fn tab_on_drag(
    mut drag: On<PointerDrag>,
    tabs: Query<&DockTab>,
    ui_scale: Option<Res<UiScale>>,
    mut carried: Query<&mut DockDrag>,
) {
    drag.propagate(false);
    if let Ok(tab) = tabs.get(drag.entity)
        && let Ok(mut carried) = carried.get_mut(tab.dock)
    {
        carried.pointer = drag.pointer.position;
        if let Some(strip) = carried.strip.as_mut() {
            let scale = ui_scale.map_or(1.0, |scale| scale.0);
            let pressed = (drag.pointer.position - drag.distance) / scale;
            strip.grab = pressed - strip.tabs[strip.from].centre;
        }
    }
}

/// Let go: the panel goes where the pointer is, if that is a place for it.
fn tab_on_drag_end(
    mut end: On<PointerDragEnd>,
    tabs: Query<&DockTab>,
    carried: Query<&DockDrag>,
    geometry: DockGeometry,
    mut docks: Query<(&mut Dock, &mut DockShown)>,
    mut commands: Commands,
) {
    end.propagate(false);
    let Ok(tab) = tabs.get(end.entity) else {
        return;
    };
    commands.entity(tab.dock).remove::<DockDrag>();
    // Only a tab that is being carried (a cancelled drag took the state away).
    let Ok(carried) = carried.get(tab.dock) else {
        return;
    };
    let Some(layout) = docks
        .get(tab.dock)
        .ok()
        .and_then(|(_, shown)| shown.layout.clone())
    else {
        return;
    };
    if let Some(landing) = geometry.landing(tab.dock, end.pointer.position, &carried.panel, &layout)
    {
        move_panel(
            &mut docks,
            tab.dock,
            &carried.panel,
            &landing.target,
            &mut commands,
        );
    }
}

/// A cancelled pointer drops the tab where it was.
fn tab_on_pointer_cancel(
    mut cancel: On<PointerCancel>,
    tabs: Query<&DockTab>,
    mut commands: Commands,
) {
    cancel.propagate(false);
    if let Ok(tab) = tabs.get(cancel.entity) {
        commands.entity(tab.dock).remove::<DockDrag>();
    }
}

/// Back puts a carried tab down where it was picked up; with no tab carried it is not handled
/// here.
fn tab_on_cancel(
    mut cancel: On<Cancel>,
    tabs: Query<&DockTab>,
    carried: Query<(), With<DockDrag>>,
    mut commands: Commands,
) {
    if let Ok(tab) = tabs.get(cancel.entity)
        && carried.contains(tab.dock)
    {
        cancel.propagate(false);
        commands.entity(tab.dock).remove::<DockDrag>();
    }
}

/// Ctrl+Shift and an arrow on a focused tab moves its panel to the nearest other group that way,
/// into it and shown there, or, when there is none, to that edge of the dock (WCAG 2.5.7: the way
/// that is not a drag). Focus goes with the tab.
fn tab_on_key(
    mut input: On<FocusedInput<KeyboardInput>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut shown: ResMut<InputFocusVisible>,
    tabs: Query<&DockTab>,
    geometry: DockGeometry,
    mut docks: Query<(&mut Dock, &mut DockShown)>,
    mut commands: Commands,
) {
    if input.input.state != ButtonState::Pressed {
        return;
    }
    let zone = match input.input.key_code {
        KeyCode::ArrowLeft => DockZone::Left,
        KeyCode::ArrowRight => DockZone::Right,
        KeyCode::ArrowUp => DockZone::Top,
        KeyCode::ArrowDown => DockZone::Bottom,
        _ => return,
    };
    if !moves_tabs(&keys) {
        return;
    }
    let Ok(tab) = tabs.get(input.focused_entity) else {
        return;
    };
    input.propagate(false);
    show_focus(&mut shown);
    let Some((card, groups)) = geometry.group_of_tab(input.focused_entity) else {
        return;
    };
    let Some(here) = groups.iter().find(|g| g.entity == card) else {
        return;
    };
    let others: Vec<&GroupSeen> = groups.iter().filter(|g| g.entity != card).collect();
    let centres: Vec<(f32, f32)> = others
        .iter()
        .map(|g| (g.card.center().x, g.card.center().y))
        .collect();
    let from = (here.card.center().x, here.card.center().y);
    let target = match dock_neighbour(from, &centres, zone) {
        Some(index) => DockDropTarget {
            onto: others[index].tabs.first().map(|(id, ..)| id.clone()),
            zone: DockZone::Into,
            index: None,
        },
        None => DockDropTarget {
            onto: None,
            zone,
            index: None,
        },
    };
    move_panel(&mut docks, tab.dock, &tab.panel, &target, &mut commands);
}

/// Whether Ctrl and Shift are both held: the chord that moves a dock tab, which the navigation
/// that would otherwise take the arrow leaves alone.
pub(crate) fn moves_tabs(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
        && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
}

/// Draws a carried tab: finds where it would land, shows that part of the screen as the overlay and
/// the tab's title beside the pointer, and hides both once it is put down. Runs in
/// [`crate::GooeySystems::Style`], before [`crate::apply_skin`]. A dock carrying nothing costs
/// nothing.
pub(crate) fn update_dock_drag(
    geometry: DockGeometry,
    registry: Res<PanelRegistry>,
    mut docks: Query<(Entity, &DockShown, &Children, Option<&mut DockDrag>)>,
    mut gone: RemovedComponents<DockDrag>,
    mut parts: Query<(&Role, &mut Node)>,
    mut texts: Query<&mut Text>,
) {
    let gone: Vec<Entity> = gone.read().collect();
    for (entity, shown, kids, mut carried) in &mut docks {
        if carried.is_none() && !gone.contains(&entity) {
            continue;
        }
        let landing = carried.as_ref().and_then(|drag| {
            let layout = shown.layout.as_ref()?;
            geometry.landing(entity, drag.pointer, &drag.panel, layout)
        });
        if let Some(drag) = carried.as_mut() {
            let target = landing.as_ref().map(|landing| landing.target.clone());
            if drag.target != target {
                drag.target = target;
            }
        }
        let origin = geometry.rect(entity).map_or(Vec2::ZERO, |rect| rect.min);
        for kid in kids.iter() {
            let Ok((role, mut node)) = parts.get_mut(kid) else {
                continue;
            };
            let mut next = node.clone();
            match role {
                Role::DockDrop => match &landing {
                    Some(landing) => {
                        let rect = Rect::from_corners(
                            landing.rect.min - origin,
                            landing.rect.max - origin,
                        );
                        next.display = Display::Flex;
                        next.left = Val::Px(rect.min.x);
                        next.top = Val::Px(rect.min.y);
                        next.width = Val::Px(rect.width());
                        next.height = Val::Px(rect.height());
                    }
                    None => next.display = Display::None,
                },
                Role::DockGhost => match &carried {
                    Some(drag) => {
                        let at = drag.pointer / geometry.scale() - origin;
                        next.display = Display::Flex;
                        next.left = Val::Px(at.x + GHOST_OFFSET_PX);
                        next.top = Val::Px(at.y + GHOST_OFFSET_PX);
                        if let Ok(mut text) = texts.get_mut(kid) {
                            text.set_if_neq(Text::new(registry.title(&drag.panel)));
                        }
                    }
                    None => next.display = Display::None,
                },
                _ => continue,
            }
            node.set_if_neq(next);
        }
    }
}

/// After a rearrangement, gives focus back to the tab of the panel that moved: its old tab was
/// despawned with the strip it was in. Runs in [`crate::GooeySystems::Navigate`]; focus is as
/// visible as it was.
pub(crate) fn land_dock_focus(
    mut docks: Query<(Entity, &mut DockShown)>,
    tabs: Query<(Entity, &DockTab)>,
    mut focus: ResMut<InputFocus>,
) {
    for (dock, mut shown) in &mut docks {
        let Some((panel, true)) = shown.refocus.as_ref() else {
            continue;
        };
        if let Some((tab, _)) = tabs
            .iter()
            .find(|(_, tab)| tab.dock == dock && &tab.panel == panel)
        {
            focus.set(tab, FocusCause::Navigated);
            shown.refocus = None;
        }
    }
}

/// A logical px of a `Val`, 0 for any other.
fn px_of(val: Val) -> f32 {
    match val {
        Val::Px(px) => px,
        _ => 0.0,
    }
}

/// Turns the place the tab it replaces was drawn at into a [`TravelFrom`], the frame a tab appears,
/// so a rebuilt dock's tabs glide from where they were instead of jumping. Runs in
/// [`crate::GooeySystems::Style`], after [`update_docks`].
pub(crate) fn start_dock_travel(
    mut tabs: Query<(Entity, &mut DockTab), Added<DockTab>>,
    mut commands: Commands,
) {
    for (entity, mut tab) in &mut tabs {
        if let Some(from) = tab.bypass_change_detection().from.take() {
            // Marked here, not left to `mark_moving_parts`, which sees the new role a frame later:
            // the first look in layout is the one that starts the glide.
            commands
                .entity(entity)
                .try_insert((MovesSmoothly, TravelFrom(from)));
        }
    }
}

/// While a tab is carried within its strip (G-63): the tab follows the pointer and the tabs it
/// passes step aside on the snappy spring, so a gap stands where it would land; when it is put
/// down, taken back (Escape) or dropped elsewhere, everything goes home. Runs in
/// [`crate::GooeySystems::Style`], after [`update_dock_drag`]. A dock carrying nothing costs
/// nothing.
///
/// Only the picture moves: layout keeps every tab where it was, so the slot under the pointer is
/// found from the strip as it stood when the tab was picked up, and nothing feeds back.
pub(crate) fn update_tab_reorder(
    ui_scale: Option<Res<UiScale>>,
    docks: Query<&DockDrag>,
    mut tabs: Query<(&mut UiTransform, Option<&MotionTarget>), With<DockTab>>,
    all_tabs: Query<(Entity, &DockTab)>,
    mut gone: RemovedComponents<DockDrag>,
    mut commands: Commands,
) {
    let scale = ui_scale.map_or(1.0, |scale| scale.0);
    for drag in &docks {
        let Some(strip) = &drag.strip else {
            continue;
        };
        let pointer = drag.pointer / scale;
        let carried = &strip.tabs[strip.from];
        // The pointer is over the strip's row, or within a tab's height of it.
        let inside = (pointer.y - carried.centre.y).abs() <= carried.size.y * 1.5;
        // One row only: a wrapped strip steps nothing aside.
        let one_row = strip
            .tabs
            .iter()
            .all(|t| (t.centre.y - carried.centre.y).abs() < 1.0);
        let to = if inside && one_row {
            strip
                .tabs
                .iter()
                .enumerate()
                .filter(|&(i, t)| i != strip.from && t.centre.x < pointer.x)
                .count()
        } else {
            strip.from
        };
        // The carried tab follows the pointer along the strip, and all the way off it.
        let want = Vec2::new(
            pointer.x - strip.grab.x - carried.centre.x,
            if inside {
                0.0
            } else {
                pointer.y - strip.grab.y - carried.centre.y
            },
        );
        if let Ok((mut transform, track)) = tabs.get_mut(carried.entity) {
            let next = Val2::px(want.x, want.y);
            if transform.translation != next {
                transform.translation = next;
            }
            if track.is_some() {
                commands.entity(carried.entity).remove::<MotionTarget>();
            }
        }
        // How far a tab it passes steps aside: its own width and the gap to its neighbour.
        let gap = if strip.from + 1 < strip.tabs.len() {
            strip.tabs[strip.from + 1].centre.x
                - strip.tabs[strip.from + 1].size.x / 2.0
                - (carried.centre.x + carried.size.x / 2.0)
        } else if strip.from > 0 {
            carried.centre.x
                - carried.size.x / 2.0
                - (strip.tabs[strip.from - 1].centre.x + strip.tabs[strip.from - 1].size.x / 2.0)
        } else {
            0.0
        };
        let step = carried.size.x + gap.max(0.0);
        for (index, other) in strip.tabs.iter().enumerate() {
            if index == strip.from {
                continue;
            }
            let shift = if to > strip.from && index > strip.from && index <= to {
                -step
            } else if to < strip.from && index >= to && index < strip.from {
                step
            } else {
                0.0
            };
            let Ok((_, track)) = tabs.get(other.entity) else {
                continue;
            };
            let now = track.map_or(0.0, |t| t.translation.x);
            if now != shift {
                commands.entity(other.entity).insert(MotionTarget {
                    translation: Vec2::new(shift, 0.0),
                    spring: MotionToken::Snappy,
                    ..default()
                });
            }
        }
    }
    // A drag that ended (dropped, taken back or let go elsewhere): the carried tab springs home from
    // where it was drawn and the tabs that stepped aside go back. A drop that changed the layout
    // rebuilds the strip, which makes these tabs gone and the new ones glide in
    // ([`start_dock_travel`]).
    for dock in gone.read() {
        for (entity, tab) in &all_tabs {
            if tab.dock != dock {
                continue;
            }
            let Ok((transform, track)) = tabs.get(entity) else {
                continue;
            };
            let at = Vec2::new(
                px_of(transform.translation.x),
                px_of(transform.translation.y),
            );
            if at != Vec2::ZERO && track.is_none() {
                commands
                    .entity(entity)
                    .try_insert(MotionTarget::default().entering_from(at));
            } else if track.is_some_and(|t| t.translation != Vec2::ZERO) {
                commands.entity(entity).try_insert(MotionTarget {
                    spring: MotionToken::Snappy,
                    ..default()
                });
            }
        }
    }
}
