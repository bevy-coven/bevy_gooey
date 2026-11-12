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
- **A plain `side_tabs` ignores `icon`** and draws the label and the badge, so one list of
  `SideTab`s serves either strip.
- **`SideTab::count(usize)` and a zero badge.** `badge("0")` stores `None`, and `count(0)` is `None`:
  a count of nothing draws nothing, which is the rule the issue states, in the one place a badge is
  built rather than in the drawing code. `badge("")` is also no badge.
- **`side_tabs_icons(tabs, selected)`** is `side_tabs` with pictures: the same `TabsValue` /
  `ValueChange<usize>` state, the same `TabList` with `ControlOrientation::Vertical` (Up and Down and
  the D-pad step it, Left and Right leave it), the same `side_tab_page` for its pages anywhere, and
  the same `SelectionIndicator`, so nothing about focus or selection is a second path.
- **Each tab is `Role::IconTab`**: a square target the size of a button (`GooeySkin::target()`), its
  picture centred in it, `Tooltip`ed with the label and an `AccessibleLabel` of the label **and its
  badge's count** ("Debug (4)"), so a screen reader hears the name and what still needs the player
  while the picture is decoration. The badge is `Role::CornerBadge`: the `Role::TabBadge` pill, made
  absolute and placed at the tab's top-right corner — placed by `apply_skin` only, so a host's own
  skin can move it.
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
- **Many tabs scroll** (built in the issue's remainder, after the first landing left it open). A strip
  taller than the room its host gives it scrolls inside that room, and both side strips do it, since a
  `side_tabs` taller than a sidebar is the same defect. The strip's own `TabList` is the viewport
  (`TabScroll`: `Overflow::scroll_y()` and Bevy's `ScrollArea` for the wheel), not a wrapper inside the
  root: the sliding indicator reads the list's rect and its tabs' rects, and a scrolled list moves the
  tabs' rects with it, so the indicator needed no change, and every test that reads the root's first
  child as the list still reads it. The root is `min_height: 0` and `max_height: 100%` so a host with a
  height caps it and one without does not; the list keeps `flex_shrink: 0` (the horizontal strip's
  rule, which `apply_skin` writes for `Role::TabList`) except while it scrolls. Each tab is
  `flex_shrink: 0`: the list's own comment on a squeezed button (a row at its `min_height`, 44 against a
  natural 46.8, clips its text) is the reason. The rail is the `list`'s (`list_rail`, `Role::ListRail`,
  shown by `update_lists` only while the content does not fit), so its look, its drag and its
  accessibility are not a second path. Reveal follows `scroll_to_focus`'s rule with the strip's own
  geometry (`scroll_tabs_to_reveal`: `scroll_to_reveal`, a tab and a half of context, `ScrollGlide`
  for the motion) for the focused tab, and the selected tab is revealed once when it changes (a click,
  a key, `TabsValue` written by the game, the first layout), so a strip opens on its selected tab and a
  wheel after it is left alone. A tooltip's box is `OverrideClip`, so the list's clip does not cut the
  name beside a scrolled tab.

## Proof

`tests/side_tab_icons.rs`, laid out for real in a headless window: the acceptance (five icon tabs, the
third selected — `TabsValue`, `Selected`, one `ValueChange` when Down selects the fourth — every tab
drawing its icon and naming itself in a tooltip, one badge on the fourth with the text "4", the strip
narrow); the edge cases (no icon shows the label's initial "F" with the whole name in its tooltip; a
badge of "0" shows nothing while "7" shows); the shape (square targets in one column, the badge at the
tab's top-right, the strip's selection indicator present); and the announced name ("Debug (4)" for a
badged tab, the bare label for one without).

`tests/side_tabs_scroll.rs` holds the scrolling: a strip of twelve in a host of 220 px takes the host's
height with a rail and square tabs; one of three has no rail and nothing to scroll; the wheel reaches
both ends; Down and Up through all twelve keep each focused tab whole in view; focus leaves the next tab
in sight; a `TabsValue` written by the game and a strip opened on its tenth tab are revealed and the
wheel after is not undone; the labelled strip scrolls too; and with motion on a reveal glides and lands
on its goal.
