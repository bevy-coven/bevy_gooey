# Split panes: a divider that resizes two panes, by pointer, key and pad, and never past a pane's minimum

Researched 2026-11-01 (iteration 30). Issue #13 (G-09, `grimoire-ui`, `consumer:bevy_grimoire`) is the pass's item: a docked layout is made of resizable panes, and a game has the same need in an inventory beside a map, a log under a viewport, a mod list beside its description. The editor is the example, never a name in the API.

## Sources

1. W3C WCAG 2.2, *Understanding SC 2.5.7 Dragging Movements* (https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html) (craft: accessibility of a control whose only pointer operation is a drag). The corpus has nothing on resizable panes: `distill_search` for "resizable split pane divider user interface" and "window layout docking panels editor interface usability" returned a volume-data paper, an agent-systems survey and an unrelated tool paper. Listed under Pending index.
2. Godot Engine 4.7, `SplitContainer` (https://docs.godotengine.org/en/stable/classes/class_splitcontainer.html), the split container of a shipped game engine's UI toolkit, also its editor's own panes (shipping).
3. W3C WAI-ARIA Authoring Practices, *Window Splitter Pattern* (https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/), the keyboard and value contract (technical), and Bevy 0.20's `bevy_ui_widgets` `slider.rs` and `scrollbar.rs`, which turn a `PointerDrag` into a value (`distance / UiScale`, against `ComputedNode::size() * inverse_scale_factor`).
4. The scikit-learn API paper already in the corpus (`docs/research/corpus.md`): one obvious way to do a thing, so the split is one bundle with one state component and one event.

## Craft principle

WCAG 2.5.7: "All functionality that uses a dragging movement for operation can be achieved by a single pointer without dragging, unless dragging is essential." Its understanding page: "Not all users can accurately press and hold that contact while also repositioning the pointer. An alternative method must be provided", and it says why the keyboard is not the answer by itself: "This requirement is separate from keyboard accessibility because people using a touchscreen device may not use a physical keyboard." A divider is a drag-only control, so beside the drag it needs a keyboard way (arrows, the pad's D-pad) and a pointer way that is not a drag (a double click that collapses the first pane to its minimum and restores it).

## How shipping games do it

Godot's `SplitContainer` ("A container that arranges child controls horizontally or vertically and creates grabbers between them. The grabbers can be dragged around to change the size relations between the child controls") keeps a **pixel offset** (`split_offsets`, "Offsets for each dragger in pixels") and clamps it so panes respect their minimum size ("`clamp_split_offset` ... ensure they are within valid ranges and do not overlap"; "If the split offsets cause Control nodes to overlap, the first split will take priority when resolving the positions"). Its separation is a theme constant (`separation` 12) and the grab area has a floor of its own (`minimum_grab_thickness` 6), with a touch dragger that overlaps the children because a thin grabber is hard to hit. Its grabber icon is shown or hidden (`dragger_visibility`, `autohide`): the handle is the affordance.

## Technical: the keys and the value

APG: a window splitter "is a moveable separator between two sections, or panes, of a window that enables users to change the relative size of the panes." It "has a value that represents the size of one of the panes, which, in this pattern, is called the primary pane." Keys: "Left Arrow: Moves a vertical splitter to the left. Right Arrow ... Up Arrow: Moves a horizontal splitter up. Down Arrow ... Enter: If the primary pane is not collapsed, collapses the pane. If the pane is collapsed, restores the splitter to its previous position. Home (Optional): Moves splitter to the position that gives the primary pane its smallest allowed size. End (Optional): ... largest allowed size." The separator is focusable and is a widget.

Bevy 0.20 has no splitter. Layout is taffy flexbox: two items of `flex_basis: 0` with `flex_grow` of `f` and `1 - f` share the room left after a fixed-size divider in exactly that ratio, with their `min_width` honoured by flex's own freezing (the same technique the slider's thumb gap uses), so no measurement is needed for the ratio.

## Model adopted

- **S1. The state is a fraction, on the split.** `SplitFraction(f32)` on the root is the first pane's share of the room the divider leaves (`extent`, the split's length on its axis less the divider's). It is serde data, reflected, set by the game with no event, and moved by the player with one `ValueChange<f32>` (`is_final` false while dragging, true at the end and for every key). A fraction is resolution independent (it survives a window resize and a UI scale change), which Godot's pixel offset is not; the drag converts pixels to it: `fraction + distance / extent`, so 100 px of drag changes the fraction by 100 / extent exactly, which is the issue's acceptance.
- **S2. Minimums.** `SplitMin { first, second }` in logical px on the root. A drag or key never leaves the clamp `[min_first / extent, 1 - min_second / extent]` (`model::split_clamp`). When the room is smaller than the two minimums the second side gives way first (the issue's edge case): the effective minimums are `eff_first = min(min_first, extent)` and `eff_second = min(min_second, extent - eff_first)`, so the second pane shrinks to nothing before the first goes below its own minimum. The stored fraction is the player's preference and is not rewritten by a window that shrinks.
- **S3. Layout.** The root is a flex row (`SplitAxis::Horizontal`: panes side by side, a vertical bar) or column; each pane is a clipped one-cell grid of `1fr` so an auto-sized `panel()` inside it fills the pane on both axes, and the game's content is a plain child. `update_splits` writes the two panes' `flex_grow` and effective `min_*` only when they differ.
- **S4. The divider** is a focusable, hoverable node with a grip in the middle. Its thickness is the 24 px of WCAG 2.5.8's minimum target (`MIN_TARGET_PX`), its length the split's: the grip is the affordance and the channel is the target, so a pointer lands on it without aiming at a line. Look: the rail's `surface_raised`, the grip `text_muted` (the pair `problems()` already holds at 3:1 for a switch's knob and a list's thumb), `text` under the pointer, `accent` while dragged, the ring while focused. A resize cursor (`EntityCursor`) on hover.
- **S5. Keys.** Arrows along the axis step 16 logical px (held keys repeat, like a slider's); Home and End move to the smallest and largest first pane; Enter, South and a double click collapse the first pane to its minimum and restore it. The perpendicular arrows move focus on. The D-pad steps along the axis with the slider's hold-to-repeat. A disabled divider (`InteractionDisabled`) is a fixed split: focusable, no change.
- **S6. One path.** Nothing else reads these keys: `navigate_by_input` leaves a focused divider its axis' arrows (keys and D-pad), the divider's own observer takes the keys and `split_by_pad` the D-pad and South.

## Parameters

| Parameter | Value | Source |
| --- | --- | --- |
| Divider thickness | `MIN_TARGET_PX` = 24 logical px | WCAG 2.5.8 minimum target; Godot's own touch dragger exists because 6 px is too thin |
| Key step | 16 px | design default (a slider's step is 1/100 of its range; panes are pixel things) |
| Grip | 4 px by 40 px pill, `radius` | design default |
| Default fraction | 0.5 | design default |
| Default minimums | 0, 0 | design default (a game that must keep a pane readable sets them) |
| Pad repeat | `PAD_REPEAT_DELAY`, `PAD_REPEAT_RATE` | the slider's |

## Validation targets

- A drag of 100 px along the axis changes `SplitFraction` by 100 / extent (±0.5 px) at 1x and at a UI scale of 1.5, and the panes' laid-out sizes follow it.
- A drag far past either end leaves each pane at least its minimum; a window shrunk below the two minimums shrinks the second pane first.
- Left and Right (Up and Down for a stacked split), Home, End, Enter, the D-pad and South do what S5 says and move no focus; the perpendicular arrows do move it.
- A pane's content fills it, and a window resize keeps the ratio with no write from the game.
- Grip contrast 3:1 against the rail in all four looks; the ring complete at 1x to 2x; nothing written while idle.

## What great looks like

A quiet gutter between two panels, the grip a short pill in the middle that brightens under the pointer and goes accent while held; the whole bar a comfortable target; the panels never jump, flicker or clip their text while resizing. In a dock of three panes (a tall sidebar, a viewport, a log under it) the nested dividers read as one family with the menus' and the list's rails.

## Deviations

- **A fraction, not a pixel offset** (Godot's `split_offsets`): a fraction survives a resize and a UI scale change; design default, recorded on the issue as an assumed decision.
- **A double click is the pointer alternative to the drag** (WCAG 2.5.7), not an arbitrary-size alternative: it reaches two positions. A game that needs every size by pointer alone adds step buttons beside the split (`SplitFraction` is plain data); the crate does not draw them. Assumed decision: the criterion allows "an equivalent option elsewhere", and the keyboard and the pad cover the rest.
- **"Collapse" is "to the minimum"**: APG's Enter collapses the pane; with a nonzero `SplitMin` the smallest allowed size is not zero, and the issue forbids ever violating a minimum, so Enter goes to the minimum (zero when the game says zero).
- **`SplitAxis`, not `Axis`**: Bevy's prelude already exports `Axis<T>`, so a glob import of both would make `Axis` ambiguous.
- **The 24 px channel** is wider than editors' 4 px hairline: a deliberate trade for a target and for the one-visual-language rule; recorded for review.

## Pending index

- The corpus lacks a paper on resizable panes, splitters or docking layouts; no `paper_search` hit was a fit. Cited from the web: WCAG 2.5.7, APG's window splitter, Godot's `SplitContainer`.
