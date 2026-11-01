# Dock layout as data: split panes and tab groups of panels, rearranged by dragging a tab

Researched 2026-11-01 (iteration 31). Issue #14 (G-10, `grimoire-ui`, `consumer:bevy_grimoire`; depends on #13, closed) is the pass's item: an editor's panel layout must be saved, restored and reset, so it has to be data, not spawned code. A game has the same need in a tuning screen, a mod manager or a debug overlay whose panels the player arranges. The editor is the example, never a name in the API.

## Sources

1. W3C WCAG 2.2, *Understanding SC 2.5.7 Dragging Movements* (https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html), already cited by `docs/research/split.md`, and the corpus's *Comparing the Performance of Mouse, Stylus and Finger Input* (stem `10.1016_j.ijhcs.2011.11.002`; craft: a drag is the slowest and least reliable pointer action, so a layout that can only be changed by one needs a second way).
2. Dear ImGui's docking branch, *Docking* wiki (https://github.com/ocornut/imgui/wiki/Docking), and `egui_dock` 0.21 (https://docs.rs/egui_dock/latest/egui_dock/), the two shipped docking systems in the immediate-mode GUI family that game tools are built with (shipping).
3. `egui_dock`'s data model and Bevy 0.20's own `bevy_picking` drag events (`PointerDragStart`, `PointerDrag`, `PointerDragEnd`) and `bevy_ui_widgets` `TabList` (technical), and the scikit-learn API paper already in the corpus (`docs/research/corpus.md`): one obvious way, so the layout is one value and one event.

The corpus has nothing on docking or workspace layouts (`distill_search` for "docking panels tabbed window layout drag and drop rearrange interface" and "customizable workspace layout user interface tabs panels" returned a hotkey paper, a vision-language benchmark and unrelated hits). Listed under Pending index.

## Craft principle

WCAG 2.5.7: "All functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging, unless dragging is essential." Moving a panel to another group is a drag in every docking system, so beside the drag it needs a keyboard way; the split's double click does not generalise (a move has a destination), so the keyboard's is the way, and the pointer's is the tab's own click (select) plus the layout as data a game can edit. The corpus's input study found "finger input was the fastest method for tapping tasks, but the slowest for dragging and radial selections": a drop zone must therefore be large (a half of a group, never a hairline) and show where the panel will land before the pointer is released, so the player can correct the drag in response to feedback ("users can achieve good finger accuracy rates with small targets when they can adjust their selections in response to system feedback").

## How shipping games do it

Dear ImGui's docking: "Dock by dragging windows from their title tab or tab (hold SHIFT to disable docking)", with two outcomes shown by the same drag: "Dock into an existing window or node" and "Split existing node". Its layout persists as data: "An alternative is to manually create desired settings, save them using `SaveIniSettingsXXX()` function, strip the data to whatever is necessary to you and call `LoadIniSettingsXXX()`"; the programmatic builder is "not great, it is unfinished, and not well documented", which is the failure this crate avoids by making the layout a plain value. `egui_dock`: "tabs and splits are represented with a binary tree", "`DockState` is generic (`DockState<Tab>`) so you can use any data to represent a tab", and the host says how a tab is shown (`TabViewer`): the same split as `DockLayout` (the tree, serde data) and `PanelRegistry` (what a panel id spawns). egui_dock also offers undocking into floating windows; the issue rules them out ("No floating windows").

## Technical: what Bevy 0.20 gives

`bevy_ui_widgets::TabList` and `Tab` already select, roving-focus and key a strip, and `tabs()` already stacks pages in one grid cell; `split()` already lays out two panes and a draggable, keyboard-operable divider. A dock is therefore a tree of those two, built from data, plus the one thing neither has: a tab that can be picked up. Bevy's picking sends `PointerDragStart`, `PointerDrag { distance }` and `PointerDragEnd` to the pressed entity, and `PointerLocation` is the pointer in window pixels, so the drop target is found by hit-testing the groups' `ComputedNode` and `UiGlobalTransform`. There is no native drop target in `bevy_ui`, so the zones are this crate's: pure maths in `model`.

## Model adopted

- **K1. The layout is a tree of data.** `DockLayout` is `Split { axis, fraction, first, second }` or `Group { panels, selected }` (a tab group of panel ids, `String`s). Serde with `default` on every field: a file without `selected` shows the first tab, one without `fraction` splits in half. It round-trips through RON (the acceptance). `DockLayout` holds no engine type beyond `SplitAxis`, which is already serde; its edits are plain methods, tested without an app.
- **K2. The widget.** `dock(layout)` is a bundle: a node that fills its parent, with `Dock(DockLayout)` as the state. `PanelRegistry` (a resource, `init_resource`d by `GooeyStylePlugin`) maps an id to a title and a spawn function `Fn(&mut EntityCommands)` that receives the panel's page and fills it, so a game registers `("scene", "Scene", |page| { page.with_child(label("hi")); })` and the dock does the rest. A panel's page is spawned once and *moved* when the player rearranges, never respawned, so what a panel holds (a scroll position, a text field) survives a drag.
- **K3. One tree, built from the two widgets.** A `Split` node is `split()`, a `Group` is `tabs()`; the dock reconciles the tree when the layout the game set differs from the one shown. Dragging a divider writes the fraction and picking a tab writes `selected` into the dock's layout and sends the same event, so what the player did is always what a save would hold.
- **K4. Drag, drop zones and the event.** A drag picks up a tab: a ghost with its title follows the pointer and an overlay shows the landing. Over a group's tab strip the tab goes into the group at the place under the pointer; over the middle half of a group, to the end of its tabs; over an outer quarter (the nearest edge) the group is split on that side and the panel gets a group of its own (half each); in a thin band along the dock's own edge, the whole layout is split (the panel takes a quarter). Letting go emits `DockLayoutChanged { entity, layout }` with the new layout; Escape, or letting go over no zone, changes nothing. A group left with no panels collapses (its parent split becomes the other side), so a layout never holds an empty group.
- **K5. Unknown and duplicate ids are skipped and reported** (the issue's edge case): the dock builds the layout without them (a group left empty collapses), warns, and sends one `DockPanelSkipped { entity, id, reason }` each. A panel registered later appears the frame the registry changes.
- **K6. The way that is not a drag (WCAG 2.5.7).** On a focused dock tab, Ctrl+Shift with an arrow key moves its panel to the nearest other group in that direction (into it, selected), or, when there is none, to that edge of the dock; focus follows the tab. Bevy's `TabList` would also take the arrow (move focus), so the tab's observer stops it.
- **K7. Minimums.** Every dock split has `SplitMin::new(DOCK_PANE_MIN_PX, DOCK_PANE_MIN_PX)` (96 logical px): a panel too narrow to read is worse than none, and a drop that makes one cannot be undone by dragging.

## Parameters

| Parameter | Value | Source |
| --- | --- | --- |
| Drop into a group | centre half of both sides, and the strip | `egui_dock` and ImGui: the middle docks, an edge splits |
| Edge zone | the outer quarter, nearest side | design default (a half-size target for an inaccurate drag) |
| Dock-edge band | 24 logical px (`MIN_TARGET_PX`) | WCAG 2.5.8 minimum target |
| New split from a group edge | 0.5 | ImGui's default for a split drop |
| New split from the dock edge | 0.25 for the new panel | design default |
| Pane minimum | 96 px | design default |
| Move by key | Ctrl+Shift+arrow | design default (the tab's own arrows and Enter are Bevy's) |

## Validation targets

- A layout of splits and groups round-trips through RON, and a file with only `panels` reads.
- Dragging a tab from the left group to the right group emits one `DockLayoutChanged` whose layout has the tab there and not in the left group; the left group, left empty, collapses; the page entity is the same one.
- Dropping on a group's edge splits it; on the dock's edge splits the whole; on its strip reorders.
- An unknown panel id is skipped and reported once and the other panels show; a duplicate is reported.
- Ctrl+Shift+Left/Right moves a tab with no pointer, focus follows it.
- Dock tab, ghost and overlay readable in all four looks (overlay edge 3:1 against the dimmed panels); no write while idle.

## What great looks like

A workbench: groups are quiet panels with a strip of tabs on top, split by the same rails as everywhere in the crate. Picking up a tab is felt at once: the group under the pointer lights the half the panel will take, in the accent, with a frame, and the ghost carries the tab's own title; letting go slides nothing and flickers nothing, the panels simply are where the overlay said.

## Deviations

- **No floating windows** (the issue; `egui_dock` has them).
- **Ids are strings**, not a generic `Tab` type as in `egui_dock`: the layout must be plain data a file holds. A game with its own enum maps it to strings in one `register`.
- **A new split is half and half** whatever the old fraction was; ImGui keeps the old node's size. Design default.
- **The keyboard's way moves a panel by direction**, not to any chosen place: it reaches every group and the edges (and repeats), which is what the criterion asks. Assumed decision (on the issue).
- **No pad way**: a dock is an editor's widget; the D-pad already moves focus and Enter on a tab selects it. Open.

## Pending index

- The corpus lacks any paper on docking, tabbed workspaces or layout persistence. Cited from the web: WCAG 2.5.7, the ImGui docking wiki, `egui_dock`.
