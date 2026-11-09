# The strip: a puck that never crosses the gap between rows

Researched 2026-11-09 (iteration 80). No issue is filed. The pass's own pick: the ranked backlog's
second item, "wrapped tab strips stepping aside".

The gap. A **dock's** tab strip already wraps (`tabs_in(.., fill: true)` is a flex row with
`FlexWrap::Wrap`), and at 200% text in a 360 px group it puts each long title on a row of its own
(measured: rows at 77, 158, 239 and 319 px). The selection indicator is one absolute puck that springs
between two options' laid-out rectangles, so a hop from the last tab of one row to the first of the
next slid the lit shape across the gap between the rows, where it sits on nothing.

## Sources

1. *Good Game Feel* — Hicks et al., 2018, DiGRA, doi:10.26503/dl.v2018i1.936 (craft). "You should be
   able to estimate from the juiciness of each action the utility of that action", and its
   Consistency category asks for "a consistent, believable game world". A lit shape drawn where no
   tab is gives an action feedback that does not match the action.
2. *Promoting Hotkey use through rehearsal with ExposeHK* — 2013, CHI, doi:10.1145/2470654.2470735
   (technical). "Studies suggest that similar methods of parallel presentation can improve
   pointer-based selection performance [30] and reduce visual search times because rapid eye
   saccades can replace comparatively slow pointer-based manipulation". A wrapped strip is parallel
   presentation: every option stays on screen instead of scrolling out of it.
3. Rune, `crates/rune-kit/src/segmented.rs:18-140` (the owner's open reference, read and not copied):
   "SegmentedControl (`@opencode/ui` navigation/segmented-control): equal segments over a layer with
   a sliding raised indicator"; the indicator is the lit segment's own cell ("the indicator shares
   the selected segment's cell and slides between cells"). One shape, one option, never between.
4. Bevy 0.20, `bevy_ui/src/ui_node.rs:2107` (`GridTrackRepetition`) and `:2046` (`GridTrack::minmax`),
   and `bevy_ui_widgets` `ControlOrientation`: the pieces this note tried and measured (below).

## Craft principle

Feedback must name the thing it belongs to. A lit shape in transit reads as "this is moving to
there"; a shape parked in the gutter between two rows reads as "something is lit" and names nothing.
When the distance cannot be drawn honestly, land it.

## How shipping games do it

Rune's segmented control puts the puck in the lit segment's cell and lets CSS grid move it, so it is
always inside an option; when the control has to fit a narrow parent it keeps every label visible.
Bevy's own `TabList` does no layout at all: it is a behaviour over whatever its parent builds, which
is why wrapping is this crate's to get right.

## Model adopted

- **A hop between rows lands.** `update_selection_indicators` reads the two options' rows (their
  laid-out `UiGlobalTransform` y, in physical px) and, when they differ by more than a pixel on a
  **horizontal** strip, sets `moving = false` and places the lanes on the target — the same rule
  `lands_travel()` gives a reduced-motion player. `Indicating(false)` is written at once, so
  `apply_skin` draws the target's own accent fill in that frame rather than clearing it behind a
  puck.
- **A vertical strip is untouched.** `side_tabs` puts every option on a row of its own, so a row test
  there would land every step; the rule is gated on `TabList::orientation == Horizontal`.
- **A strip keeps its rows.** `Role::TabList` now carries `flex_shrink: 0`: a dock group's card is a
  column whose page area grows, and a strip that could shrink was cut at the card's edge (measured at
  200% text in a 760 px window: three rows, the second and third outside the card).
- **Within a row nothing changes**: the puck springs on the snappy spring, as before.

## What great looks like

A dock group at 200% text whose four long tabs stand one to a row: pressing Down the strip moves the
lit accent from row to row with no shape left hanging in the gutter, each row's tab lit in its own
frame; and a three-tab strip one row wide still slides its puck between neighbours.

## Parameters

| Name | Value | Where from |
|---|---|---|
| "another row" | `abs(dy) > 1.0` physical px | one pixel of layout rounding is not a row |
| travel across rows | none (lands) | this crate's reduced-motion rule, `lands_travel` |
| within a row | snappy spring, unchanged | `MotionToken::Snappy` |

## Validation targets

- A dock group at 200% text wraps its strip (the failing measurement: rows 77/158/239/319 px).
- A hop to the second row leaves the puck undrawn and `SelectionIndicator::is_moving()` false in the
  frame the lit tab changes, with that tab carrying `Indicating(false)` and Bevy's `Selected`.
- A `side_tabs` step still travels (`tests/selection_indicator.rs::tabs_and_side_tabs_travel_too`).
- A strip keeps every row it wraps to inside its group's card at 200% text.
- The strip's own layout is unchanged otherwise: `tests/tabs.rs` and `tests/side_tabs.rs` stay green.

## Deviations

- **A non-dock strip still does not wrap.** Three attempts were measured and reverted this pass:
  (1) a flex row with `flex_grow: 1, flex_basis: 0, FlexWrap::Wrap` wraps but gives unequal cells in a
  wrapped row (`[312, 284]` px at 200% text), because each cell's least width is its own label's;
  (2) `repeat(auto-fit, minmax(min-content, 1fr))` collapses to a single column in taffy, so a
  four-tab strip became four rows; (3) `repeat(auto-fit, minmax(min-content, min-content))` stops the
  strip growing to its labels. Per-row equal cells are a follow-up (breakpointed `Repeat` tracks, or
  a measured two-grid swap), not this pass's.
- **`GooeyFrames::strip` (art under a strip) was drafted and reverted.** A strip's tabs have no fill
  of their own to tint: rest tabs are `surface_raised` and the lit one the accent, so art under them
  shows only in the gaps. Rune's rail works because its rest segments are transparent and the lit one
  is raised — a restyle of the strip, recorded as the frame work's next slice.

## Pending index

- Nothing new to index: the corpus carried a craft stem (*Good Game Feel*), a technical stem
  (ExposeHK) and the owner's reference (Rune) for this note.
