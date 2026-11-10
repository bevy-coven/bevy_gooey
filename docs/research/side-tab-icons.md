# An icon-only tab strip: an activity bar's workspaces

Researched 2026-11-10 for issue #87 (G-76), asked by bevy_dark_factory#380 (NV-03: a vertical strip
of workspaces, each with an icon, a label on hover and a badge for what needs the person). At
`d1ec9ee` `side_tabs`/`SideTab` carried a label and a badge and nothing else: no picture, and a strip
as wide as its labels.

## Sources

1. W3C, *WAI-ARIA Authoring Practices Guide*, "Tabs" and "Tooltip" — craft: each tab is one tab stop
   with a name and a selected state; a picture-only control carries its name as a tooltip and an
   accessible name (WCAG 1.1.1, non-text content).
2. W3C, *Understanding SC 1.4.11 (Non-text Contrast)* and *2.4.13* (focus appearance) — technical: the
   selected tab, its focus ring and its hover are three distinguishable states without hue alone
   (`GooeySkin::problems()` already holds the fills and the ring).
3. `docs/research/icons.md` (#25, G-21) and `docs/research/selection-indicator.md` (G-61) — craft, the
   crate's own icon tinting and its sliding indicator, which this strip keeps untouched.
4. Rune (`https://github.com/anomalyco/rune`, MIT), its activity bar — craft, read as an open
   reference: a narrow column of square targets, an unread marker in a corner, the name on hover.

## Model adopted

- **`SideTab` gains `icon: Option<IconSource>`** (`IconSource` is the crate's own, from G-21), so the
  data a tab carries is the same for both strips: a label that is always its name, an optional picture,
  an optional badge.
- **`SideTab::count(usize)` and a zero badge.** `badge("0")` stores `None`, and `count(0)` is `None`:
  a count of nothing draws nothing, which is the rule the issue states, in the one place a badge is
  built rather than in the drawing code. `badge("")` is also no badge.
- **`side_tabs_icons(tabs, selected)`** is `side_tabs` with pictures: the same `TabsValue` /
  `ValueChange<usize>` state, the same `TabList` with `ControlOrientation::Vertical` (Up and Down and
  the D-pad step it, Left and Right leave it), the same `side_tab_page` for its pages anywhere, and
  the same `SelectionIndicator`, so nothing about focus or selection is a second path.
- **Each tab is `Role::IconTab`**: a square target the size of a button (`GooeySkin::target()`), its
  picture centred in it, `Tooltip`ed with the label and an `AccessibleLabel` of the same text, so a
  screen reader hears the name and the picture is decoration. The badge is `Role::CornerBadge`: the
  `Role::TabBadge` pill, made absolute and placed at the tab's top-right corner.
- **No icon falls back to the label's first letter** (upper-cased), in `Role::TabText`, so a strip is
  never a column of empty squares.

## A bundle cannot hold two `Node`s, and a role is the look

The first build marked an icon tab with a component and asked `apply_skin` to look it up. That needs a
query in `apply_skin` — which is at Bevy's system-parameter limit, and a `State` flag would have been
the second path for one shape. The look is the `Role` instead: `Role::IconTab` is a `Role::Tab` that
is square, and `Role::CornerBadge` is a `Role::TabBadge` in a corner, so `apply_skin` reads a node's
role exactly as it always has. `access.rs`'s exhaustive `match` refused to compile until both roles
were classified, which is the check that keeps a new role from going silent.

## Where this differs from the issue

- **Not a variant flag on `side_tabs`** but a second free function. The two strips differ in a tab's
  shape, its padding and its badge's place — a `bool` on `side_tabs` would put every one of those
  behind an `if` in the middle of a bundle literal; a second builder of the same state costs one
  function and keeps both readable.
- **The badge is a pill, not a bare dot.** A dot says "something"; the issue allows either and a count
  says more, so `count(4)` is a pill in the corner and the API accepts any short text.
- **Many tabs scroll** is not built: like `side_tabs` before it, the strip is a column that grows. A
  strip taller than the window is the host's to put in a `list`; the issue lists it as an edge case,
  not the acceptance.

## Proof

`tests/side_tab_icons.rs`, laid out for real in a headless window: the acceptance (five icon tabs, the
third selected — `TabsValue`, `Selected`, one `ValueChange` when Down selects the fourth — every tab
drawing its icon and naming itself in a tooltip, one badge on the fourth with the text "4", the strip
narrow); the edge cases (no icon shows the label's initial "F" with the whole name in its tooltip; a
badge of "0" shows nothing while "7" shows); and the shape (square targets in one column, the badge at
the tab's top-right, the strip's selection indicator present).
