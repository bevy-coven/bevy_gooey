//! The dock's layout as plain data: a tree of splits and tab groups of panel ids, and the edits a
//! player makes to it (moving a panel, resizing a split, picking a tab). No engine type but
//! [`SplitAxis`], which is itself plain data, so every edit is tested without an app.

use crate::model::DockZone;
use crate::split::SplitAxis;

/// The share a new split gives the panel that was dropped on a group's edge.
const HALF: f32 = 0.5;
/// The share the first pane gets when a whole dock is split at its edge: the new panel's quarter.
const EDGE_SHARE: f32 = 0.25;

/// The panels of a [`crate::dock`] and how they are arranged: a tree whose leaves are tab groups
/// and whose other nodes split their room in two. Plain data, serde with the `serde` feature: a
/// game saves it, loads it and resets it by writing it into [`crate::Dock`].
///
/// ```
/// # use bevy_gooey::*;
/// let layout = DockLayout::split(
///     SplitAxis::Horizontal,
///     0.25,
///     DockLayout::group(["scene", "assets"]),
///     DockLayout::split(
///         SplitAxis::Vertical,
///         0.75,
///         DockLayout::group(["viewport"]),
///         DockLayout::group(["log"]),
///     ),
/// );
/// assert_eq!(layout.panels(), ["scene", "assets", "viewport", "log"]);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DockLayout {
    /// Two nodes side by side or one above the other, with a divider between them.
    Split {
        /// Which way the two are laid out.
        axis: SplitAxis,
        /// The first node's share of the room the divider leaves, 0 to 1 (half when a file has
        /// none).
        #[cfg_attr(feature = "serde", serde(default = "half"))]
        fraction: f32,
        /// The left (or top) node.
        first: Box<DockLayout>,
        /// The right (or bottom) node.
        second: Box<DockLayout>,
    },
    /// A tab group: one panel shown at a time, each named by its id in the
    /// [`crate::PanelRegistry`].
    Group {
        /// The panel ids, in the order of the tabs.
        panels: Vec<String>,
        /// The tab shown (the first when a file has none).
        #[cfg_attr(feature = "serde", serde(default))]
        selected: usize,
    },
}

#[cfg(feature = "serde")]
fn half() -> f32 {
    HALF
}

/// Why [`DockLayout::prune`] left a panel out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSkip {
    /// No panel of that id is registered.
    Unknown,
    /// The id is already in the layout; a panel stands in one place.
    Duplicate,
}

/// Where a dragged panel lands: next to or into the group holding `onto`, or at an edge of the
/// whole dock.
#[derive(Clone, Debug, PartialEq)]
pub struct DropTarget {
    /// A panel of the group the drop is on (any one; the group is found by it), or `None` for the
    /// dock as a whole, which takes edge zones only.
    pub onto: Option<String>,
    /// Into the group, or against which of its edges.
    pub zone: DockZone,
    /// For [`DockZone::Into`]: the place among the group's tabs, `0` first and the group's length
    /// last. `None` is the end of a group the panel is not in, and no change in its own.
    pub index: Option<usize>,
}

impl DockLayout {
    /// A tab group of `panels`, the first one shown.
    pub fn group<S: Into<String>>(panels: impl IntoIterator<Item = S>) -> Self {
        Self::Group {
            panels: panels.into_iter().map(Into::into).collect(),
            selected: 0,
        }
    }

    /// Two nodes along `axis`, the first taking `fraction` of the room.
    pub fn split(axis: SplitAxis, fraction: f32, first: Self, second: Self) -> Self {
        Self::Split {
            axis,
            fraction,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Every panel id, in tree order (first nodes first, tabs left to right).
    pub fn panels(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Self::Split { first, second, .. } => {
                first.collect(out);
                second.collect(out);
            }
            Self::Group { panels, .. } => out.extend(panels.iter().map(String::as_str)),
        }
    }

    /// Whether a panel of this id is in the layout.
    pub fn contains(&self, panel: &str) -> bool {
        match self {
            Self::Split { first, second, .. } => first.contains(panel) || second.contains(panel),
            Self::Group { panels, .. } => panels.iter().any(|p| p == panel),
        }
    }

    /// The layout with the panels `keep` refuses and every repeat of an id left out, a group left
    /// with none collapsed into the node beside it, and what was left out with the reason. `None`
    /// when nothing is left.
    pub fn prune(
        &self,
        mut keep: impl FnMut(&str) -> bool,
    ) -> (Option<Self>, Vec<(String, DockSkip)>) {
        let mut seen = Vec::new();
        let mut skipped = Vec::new();
        let layout = self.pruned(&mut keep, &mut seen, &mut skipped);
        (layout, skipped)
    }

    fn pruned(
        &self,
        keep: &mut impl FnMut(&str) -> bool,
        seen: &mut Vec<String>,
        skipped: &mut Vec<(String, DockSkip)>,
    ) -> Option<Self> {
        match self {
            Self::Split {
                axis,
                fraction,
                first,
                second,
            } => {
                let first = first.pruned(keep, seen, skipped);
                let second = second.pruned(keep, seen, skipped);
                match (first, second) {
                    (Some(first), Some(second)) => {
                        Some(Self::split(*axis, *fraction, first, second))
                    }
                    (one, other) => one.or(other),
                }
            }
            Self::Group { panels, selected } => {
                let shown = panels.get(*selected);
                let mut kept = Vec::new();
                let mut now = 0;
                for panel in panels {
                    if seen.contains(panel) {
                        skipped.push((panel.clone(), DockSkip::Duplicate));
                    } else if !keep(panel) {
                        skipped.push((panel.clone(), DockSkip::Unknown));
                    } else {
                        if Some(panel) == shown {
                            now = kept.len();
                        }
                        seen.push(panel.clone());
                        kept.push(panel.clone());
                    }
                }
                (!kept.is_empty()).then_some(Self::Group {
                    selected: now,
                    panels: kept,
                })
            }
        }
    }

    /// The layout with `panel` taken out and the group it leaves empty collapsed; `None` when no
    /// panel is left.
    fn without(self, panel: &str) -> Option<Self> {
        match self {
            Self::Split {
                axis,
                fraction,
                first,
                second,
            } => match (first.without(panel), second.without(panel)) {
                (Some(first), Some(second)) => Some(Self::split(axis, fraction, first, second)),
                (one, other) => one.or(other),
            },
            Self::Group {
                mut panels,
                selected,
            } => {
                if let Some(at) = panels.iter().position(|p| p == panel) {
                    panels.remove(at);
                    if panels.is_empty() {
                        return None;
                    }
                    let selected = match at.cmp(&selected) {
                        std::cmp::Ordering::Less => selected - 1,
                        _ => selected.min(panels.len() - 1),
                    };
                    return Some(Self::Group { panels, selected });
                }
                Some(Self::Group { panels, selected })
            }
        }
    }

    /// The group that holds `panel`, with its tabs and place.
    fn group_of(&self, panel: &str) -> Option<&[String]> {
        match self {
            Self::Split { first, second, .. } => {
                first.group_of(panel).or_else(|| second.group_of(panel))
            }
            Self::Group { panels, .. } => panels.iter().any(|p| p == panel).then_some(panels),
        }
    }

    /// Applies `edit` to the node that is the group holding `anchor`.
    fn edit_group(&mut self, anchor: &str, edit: &mut impl FnMut(&mut Self)) -> bool {
        match self {
            Self::Split { first, second, .. } => {
                first.edit_group(anchor, edit) || second.edit_group(anchor, edit)
            }
            Self::Group { panels, .. } => {
                if panels.iter().any(|p| p == anchor) {
                    edit(self);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// The layout after dropping `panel` on `target`, or `None` when that changes nothing or
    /// cannot be done (an unknown panel or group, [`DockZone::Into`] with no group, an edge of a
    /// group that holds only the panel itself). The panel leaves its group (an emptied one
    /// collapses), then goes into the target group at its place and is shown there, or into a new
    /// group of its own beside the target (half and half; a quarter at an edge of the whole dock).
    pub fn moved(&self, panel: &str, target: &DropTarget) -> Option<Self> {
        if !self.contains(panel) {
            return None;
        }
        // The group is found by a panel that stays in it, never by the one being moved; a group
        // of that panel alone has nothing to drop it beside or into.
        let anchor = match &target.onto {
            Some(onto) => Some(
                self.group_of(onto)?
                    .iter()
                    .find(|p| p.as_str() != panel)?
                    .clone(),
            ),
            None => None,
        };
        let from = self
            .group_of(panel)
            .and_then(|g| g.iter().position(|p| p == panel))?;
        let same_group = anchor
            .as_deref()
            .and_then(|a| self.group_of(a))
            .is_some_and(|g| g.iter().any(|p| p == panel));
        let rest = self.clone().without(panel);
        let mut next = match (&anchor, target.zone) {
            (None, DockZone::Into) => return None,
            (None, zone) => {
                let own = Self::group([panel]);
                match rest {
                    Some(rest) => Self::joined(zone, EDGE_SHARE, own, rest),
                    None => own,
                }
            }
            // Into the group it is already in, with no place named, is where it is.
            (Some(_), DockZone::Into) if same_group && target.index.is_none() => return None,
            (Some(anchor), DockZone::Into) => {
                let mut next = rest?;
                let place = target
                    .index
                    .map(|i| if same_group && i > from { i - 1 } else { i });
                let id = panel.to_owned();
                next.edit_group(anchor, &mut |node| {
                    if let Self::Group { panels, selected } = node {
                        let at = place.map_or(panels.len(), |i| i.min(panels.len()));
                        panels.insert(at, id.clone());
                        *selected = at;
                    }
                });
                next
            }
            (Some(anchor), zone) => {
                let mut next = rest?;
                let id = panel.to_owned();
                next.edit_group(anchor, &mut |node| {
                    let old = std::mem::replace(node, Self::group::<&str>([]));
                    *node = Self::joined(zone, HALF, Self::group([id.as_str()]), old);
                });
                next
            }
        };
        next.clamp_selected();
        (next != *self).then_some(next)
    }

    /// `own` against `old` on the side `zone` names: `own` first for Left and Top.
    fn joined(zone: DockZone, share: f32, own: Self, old: Self) -> Self {
        match zone {
            DockZone::Left => Self::split(SplitAxis::Horizontal, share, own, old),
            DockZone::Top => Self::split(SplitAxis::Vertical, share, own, old),
            DockZone::Right => Self::split(SplitAxis::Horizontal, 1.0 - share, old, own),
            DockZone::Bottom | DockZone::Into => {
                Self::split(SplitAxis::Vertical, 1.0 - share, old, own)
            }
        }
    }

    fn clamp_selected(&mut self) {
        match self {
            Self::Split { first, second, .. } => {
                first.clamp_selected();
                second.clamp_selected();
            }
            Self::Group { panels, selected } => {
                *selected = (*selected).min(panels.len().saturating_sub(1));
            }
        }
    }

    /// The node at `path` from the root: `false` is the first node of a split, `true` the second.
    pub(crate) fn node_mut(&mut self, path: &[bool]) -> Option<&mut Self> {
        match (path.split_first(), self) {
            (None, node) => Some(node),
            (
                Some((&second, rest)),
                Self::Split {
                    first: a,
                    second: b,
                    ..
                },
            ) => (if second { b } else { a }).node_mut(rest),
            (Some(_), Self::Group { .. }) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: SplitAxis = SplitAxis::Horizontal;
    const V: SplitAxis = SplitAxis::Vertical;

    fn two() -> DockLayout {
        DockLayout::split(
            H,
            0.3,
            DockLayout::group(["a", "b"]),
            DockLayout::group(["c"]),
        )
    }

    fn onto(panel: &str, zone: DockZone) -> DropTarget {
        DropTarget {
            onto: Some(panel.to_owned()),
            zone,
            index: None,
        }
    }

    #[test]
    fn a_tab_moves_from_one_group_to_another_and_is_shown() {
        let moved = two().moved("a", &onto("c", DockZone::Into));
        assert_eq!(
            moved,
            Some(DockLayout::split(
                H,
                0.3,
                DockLayout::group(["b"]),
                DockLayout::Group {
                    panels: vec!["c".into(), "a".into()],
                    selected: 1
                },
            ))
        );
    }

    #[test]
    fn the_last_tab_leaving_a_group_collapses_it() {
        let moved = two().moved("c", &onto("a", DockZone::Into));
        assert_eq!(
            moved,
            Some(DockLayout::Group {
                panels: vec!["a".into(), "b".into(), "c".into()],
                selected: 2
            })
        );
    }

    #[test]
    fn an_edge_drop_splits_the_group_with_the_panel_on_that_side() {
        let left = two().moved("c", &onto("a", DockZone::Left)).expect("moved");
        assert_eq!(
            left,
            DockLayout::split(
                H,
                0.5,
                DockLayout::group(["c"]),
                DockLayout::group(["a", "b"])
            )
        );
        let bottom = two()
            .moved("a", &onto("c", DockZone::Bottom))
            .expect("moved");
        assert_eq!(
            bottom,
            DockLayout::split(
                H,
                0.3,
                DockLayout::group(["b"]),
                DockLayout::split(V, 0.5, DockLayout::group(["c"]), DockLayout::group(["a"])),
            )
        );
    }

    #[test]
    fn an_edge_of_the_whole_dock_takes_a_quarter() {
        let target = DropTarget {
            onto: None,
            zone: DockZone::Right,
            index: None,
        };
        let moved = two().moved("a", &target).expect("moved");
        assert_eq!(
            moved,
            DockLayout::split(
                H,
                0.75,
                DockLayout::split(H, 0.3, DockLayout::group(["b"]), DockLayout::group(["c"])),
                DockLayout::group(["a"]),
            )
        );
        // Into needs a group.
        let into = DropTarget {
            zone: DockZone::Into,
            ..target
        };
        assert_eq!(two().moved("a", &into), None);
    }

    #[test]
    fn a_tab_reorders_within_its_group_and_a_drop_in_place_is_no_change() {
        let layout = DockLayout::group(["a", "b", "c"]);
        let at = |index| DropTarget {
            onto: Some("a".into()),
            zone: DockZone::Into,
            index: Some(index),
        };
        // Dropped before itself, or just after: where it was.
        assert_eq!(layout.moved("a", &at(0)), None);
        assert_eq!(layout.moved("a", &at(1)), None);
        assert_eq!(
            layout.moved("a", &at(3)),
            Some(DockLayout::Group {
                panels: vec!["b".into(), "c".into(), "a".into()],
                selected: 2
            })
        );
        assert_eq!(
            layout.moved("c", &at(0)),
            Some(DockLayout::Group {
                panels: vec!["c".into(), "a".into(), "b".into()],
                selected: 0
            })
        );
    }

    #[test]
    fn a_group_of_one_cannot_be_split_from_itself() {
        let layout = two();
        assert_eq!(layout.moved("c", &onto("c", DockZone::Left)), None);
        assert_eq!(layout.moved("c", &onto("c", DockZone::Into)), None);
        // With company it can: the panel splits off its own group.
        let split = layout.moved("a", &onto("a", DockZone::Top)).expect("moved");
        assert_eq!(
            split,
            DockLayout::split(
                H,
                0.3,
                DockLayout::split(V, 0.5, DockLayout::group(["a"]), DockLayout::group(["b"])),
                DockLayout::group(["c"]),
            )
        );
    }

    #[test]
    fn unknown_panels_and_groups_move_nothing() {
        assert_eq!(two().moved("x", &onto("a", DockZone::Into)), None);
        assert_eq!(two().moved("a", &onto("x", DockZone::Into)), None);
    }

    #[test]
    fn the_shown_tab_stays_shown_when_an_earlier_one_leaves() {
        let layout = DockLayout::split(
            H,
            0.5,
            DockLayout::Group {
                panels: vec!["a".into(), "b".into(), "c".into()],
                selected: 2,
            },
            DockLayout::group(["d"]),
        );
        let moved = layout
            .moved("a", &onto("d", DockZone::Into))
            .expect("moved");
        let DockLayout::Split { first, .. } = moved else {
            panic!("a split")
        };
        assert_eq!(
            *first,
            DockLayout::Group {
                panels: vec!["b".into(), "c".into()],
                selected: 1
            }
        );
    }

    #[test]
    fn prune_drops_unknown_and_repeated_ids_and_collapses_empty_groups() {
        let layout = DockLayout::split(
            H,
            0.4,
            DockLayout::group(["ghost"]),
            DockLayout::split(
                V,
                0.5,
                DockLayout::group(["a", "a", "gone"]),
                DockLayout::group(["b"]),
            ),
        );
        let (pruned, skipped) = layout.prune(|id| id != "ghost" && id != "gone");
        assert_eq!(
            pruned,
            Some(DockLayout::split(
                V,
                0.5,
                DockLayout::group(["a"]),
                DockLayout::group(["b"])
            ))
        );
        assert_eq!(
            skipped,
            [
                ("ghost".to_owned(), DockSkip::Unknown),
                ("a".to_owned(), DockSkip::Duplicate),
                ("gone".to_owned(), DockSkip::Unknown)
            ]
        );
        assert_eq!(DockLayout::group(["x"]).prune(|_| false).0, None);
    }

    #[test]
    fn prune_keeps_the_same_panel_shown() {
        let layout = DockLayout::Group {
            panels: vec!["ghost".into(), "a".into(), "b".into()],
            selected: 2,
        };
        let (pruned, _) = layout.prune(|id| id != "ghost");
        assert_eq!(
            pruned,
            Some(DockLayout::Group {
                panels: vec!["a".into(), "b".into()],
                selected: 1
            })
        );
    }

    #[test]
    fn a_path_finds_a_node() {
        let mut layout = two();
        assert_eq!(
            layout.node_mut(&[true]),
            Some(&mut DockLayout::group(["c"]))
        );
        assert_eq!(layout.node_mut(&[true, false]), None);
    }
}
