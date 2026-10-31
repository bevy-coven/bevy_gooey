# The list: rows that scroll, and a focus that never leaves the screen

Researched 2026-10-31 (iteration 13). home-still's corpus has nothing on scrolling lists in game interfaces (`hs distill search`, three queries on scrolling menus with a pad, scrollbar affordance and paging through long lists returned VR menu archetypes, a rendering course note and *Game Engine Architecture*; `catalog_recent` limit 200 held only biomedical, SIGGRAPH and rendering-course embeds), so the three kinds of source below are web copies and one of the owner's own projects. Every quotation below was fetched on 2026-10-31 and matches; the gap is under `## Pending index`.

## Sources

1. Nielsen Norman Group, Fessenden, T., *Scrolling and Attention* (2018) — https://www.nngroup.com/articles/scrolling-and-attention/ (craft: why a list that scrolls must say so, and with what).
2. Microsoft, *Gamepad and remote control interactions* (Windows apps on Xbox One and the 10-foot experience) — https://learn.microsoft.com/en-us/windows/apps/design/input/gamepad-and-remote-interactions, and *Xbox Accessibility Guideline 107: Input* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107 (shipping consoles and their games: focus, paging through a list, single presses).
3. W3C, *Understanding SC 2.4.11 Focus Not Obscured (Minimum)* (WCAG 2.2) — https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html, and the Bevy 0.20.0 source, `bevy_ui_widgets-0.20.0/src/scrollarea.rs` and `scrollbar.rs`, `bevy_feathers-0.20.0/src/controls/listview.rs` (technical: what Bevy gives and what it leaves to us).
4. The owner's `bevy_emerge` editor, `crates/bevy_emerge_mapper/src/chrome.rs` (`scroll_list`, `scroll_to_reveal`, `hide_idle_scrollbars`, `Follow`) and `Sanctuary`'s `src/board/mod.rs` (a `ScrollPosition` writer): four lists in the editor and one in the game each follow their selection by hand. Their doc comments record the failures this note designs out.

## Craft principle

NN/g on scrolling: "Beware of false floors, which are increasingly common with modern minimalist designs. The illusion of completeness can interfere with scrolling. Include signifiers (such as cut-off text) to tell people that there is content below the fold." A list that ends exactly on a row looks finished; one that cuts the next row in half does not, and a bar that is there only when there is somewhere to go says the same a second way.

WCAG 2.2, SC 2.4.11: "When a user interface component receives keyboard focus, the component is not entirely hidden due to author-created content." Its stated goal: "Keep the focused item visible." A clipped list is exactly such content: a list whose focus walks off its end fails the criterion, and a pad player, who has no cursor to find it again, loses their place.

## How shipping games do it

Xbox, on the 10-foot experience: "One focus visual should always be visible on the screen so that the user can pick up where they left off without searching for the focus." On long lists, Microsoft's table maps "Page up/down" to the pad's "Left/right triggers", built into `ListBox`, `ListView` and `ScrollViewer` ("Views that support vertical scrolling"), and warns what a long list does to a D-pad: "The user would need to scroll all the way down through the list/grid to reach the CommandBar." and "you must scroll through each item to navigate outside of the ScrollViewer". XAG 107: "A UI should be navigable by using single, non-simultaneous key presses."

The owner's editor, measured live on a list walked with the arrow keys (`chrome.rs`): "after forty presses the selection is glued to the bottom pixel with twenty-six rows of already-passed context above it and **zero rows of what is coming below**", reported as "I don't see a list of meshes as I press up and down arrows, but I see the mesh change". Its first rule, "enough to put the row's edge on the viewport's edge and not one pixel more", was pinned by a test as correct and was the wrong rule; it now reveals "with [`CONTEXT_ROWS`] of list either side of it". The same file records the mechanics: the scrollbar's track is a sibling of the scrolling node, so "the list gains a wrapper"; the thumb "deliberately has no `Node`" (Bevy lays it out after `ui_layout_system`); "`ScrollArea`'s wheel handler computes its maximum from `size()` while the scrollbar's own code subtracts `scrollbar_size`", so the bar overlays a padded lane rather than using `Node::scrollbar_width`; a reveal "asks for `max + margin`" and "Bevy clamps only `ComputedNode::scroll_position`", so the component must be clamped by the writer; and a bar is shown "only while there is somewhere to scroll" (`Display::None` otherwise).

## Model adopted

- **L1. One call, rows as children.** `list(rows, children![..])` returns a wrapper (`Role::List`) that holds the viewport (`Role::ListView`, a clipped column carrying Bevy's `ScrollArea`, so the wheel and a trackpad work) and a rail (`Role::ListRail`, Bevy's `Scrollbar` aimed at the viewport, its thumb `Role::ListThumb`, so a track click and a drag work). The rows are the viewport's children, so a row is any widget. `rows` is how many rows tall the list stands, in `f32`: `4.5` shows four rows and half of the fifth, the cut-off signifier. The viewport is the public component `ListView { rows }`; set its `ScrollPosition` to scroll from code.
- **L2. The height is rows, not pixels.** The viewport stands `rows × (row + gap) − gap` tall, with `row` a button's height at the skin's text size and scale (`max(min_target, 1.2 × text + 1.2 × padding + 2 × line)`), so a list shows the same number of rows at 1x, 2x and at the player's text scale, and `4.5` always cuts a row.
- **L3. The rail says there is more, only when there is.** It is `Display::None` when the content fits the viewport and shown when it does not, written only on a change (`update_lists`). Its lane (the viewport's right padding, `rail + gap`) is always reserved, so rows never change width when others come and go, and the content stops short of the bar. The thumb is `text_muted` on a `surface_raised` track (the pair `GooeySkin::problems` already holds at 3:1 for a switch's knob), `text` under the pointer and `accent` while dragged.
- **L4. Focus follows, with context.** When a key, the pad or Tab moves focus onto a widget inside a list, the list scrolls by the least that leaves one of that widget's own heights of list beyond it, clamped to the list's ends. A widget taller than the viewport scrolls to its top; a jump of more than one viewport centres the widget. Nothing happens for a focus a pointer press set (`InputFocusVisible` is off): a press that scrolled the content under the pointer would send its release to another row and lose the click (XAG 107, activation on the up event).
- **L5. A clipped row is not a neighbour from outside.** Arrows and the D-pad never land on a row the list has clipped away unless focus is already in that list; inside the list the next row is a neighbour, and moving to it brings it in (L4). So pressing Up on the button below a list lands on the last row you can see.
- **L6. A page at a time.** PageUp and PageDown, and the pad's left and right triggers (Microsoft's mapping), move focus from a row to the row a viewport away within the same list, or to the end, and L4 follows. Single presses only (XAG 107).
- **L7. Nothing runs at rest.** `update_lists` reads layout and writes the rail only when it changes; the focus follower runs only on a changed focus and writes `ScrollPosition` only when the target is more than half a pixel from it.

## Parameters

No new field in `GooeySkin`; every size derives from the existing ones.

|part|size|
|---|---|
|viewport height|`rows × (row + gap) − gap`, `row = max(min_target, 1.2 × text_px + 1.2 × padding + 2 × line)`|
|rows|`gap` apart (`row_gap`), like a `menu`|
|rail|`2 × line` wide (6 px at 720p), flush with the wrapper's right edge, full height; fill `surface_raised`, fully rounded|
|lane|viewport padding right `rail + gap` (16 px at 720p): one `gap` of air between a row and the rail|
|thumb|the rail's width, at least `min_target / 2` long (Bevy's `min_thumb_length`), fully rounded; `text_muted`, `text` hovered, `accent` dragged|
|follow margin|the focused widget's own height|
|page|the viewport's height, less one margin|

## Validation targets

- `list(4.5, ..)` of 12 buttons at 1280x720: the fifth row is cut; the viewport is 4.5 × (row + gap) − gap high (within 1 px); the same number of rows at 2560x1440 (twice the pixels) and at text scale 1.5.
- Rail hidden for 3 rows, shown for 12; the thumb's length is the visible share of the track within 1 px; the thumb sits at the scroll's share of the free track.
- Wheel: one notch scrolls Bevy's amount; it clamps at both ends and does not chain to the panel.
- Down × 11 from the first row: every focus lands with the row fully inside the viewport and, until the end, the next row fully inside too; the ring (3 px on all sides) is whole; the list ends flush with the last row.
- Up from the last row walks back the same way; a Tab walk does the same as the arrows.
- A pointer press on a half-visible row focuses it and does not scroll.
- From the button below the list, Up lands on the last visible row, never a clipped one.
- PageDown moves the focus about one viewport and the list follows; PageUp back; at the ends it stops.
- Text and ring contrast unchanged; thumb on track at least 3:1 (4.78:1 dark, 3.89:1 parchment); the rail and thumb whole pixels at 1x to 2x.
- Frame time within 0.5 ms of the gallery before this change; no per-frame write while idle.

## What great looks like

A panel of save slots, four and a half rows tall, the fifth cut at the middle of its label. A slim rail on the right, its thumb a little under half the track, in the same muted grey as a switch's knob, brightening under the pointer. Walk down with the D-pad and the focus ring (the same blue, 3 px) never touches the bottom edge until the end; the list slides a row at a time to keep the next slot in sight. Wheel it, grab the thumb, click the track: all work. A list of three rows in the same place has no rail, no dead lane worth noticing, and looks like a menu.

## Deviations

- APG's listbox pattern is not the base: "Because of these traits of the listbox widget, it does not provide an accessible way to present a list of interactive elements, such as links, buttons, or checkboxes. To present a list of interactive elements, see the Grid Pattern." A `list` holds buttons, switches and sliders, each its own tab stop, not options with a selected state; Bevy's `ListBox` (a single-selection widget with `ActiveDescendant`) is for that and is not wrapped here. Feathers' `FeathersListView` is a `ListBox`; this is a scroll container for any rows.
- Bevy's own `ScrollIntoView` scrolls "by the minimal amount to make the target fully visible", the rule `chrome.rs` found glued the selection to the edge; this crate computes its own reveal with context (`model::scroll_to_reveal`, engine-free).
- The bar is an overlay in a padded lane, not `Node::scrollbar_width` (the wheel and the bar disagree by the gutter, as `chrome.rs` measured). The lane is reserved even when the bar is hidden, so a list that fits has a `rail + gap` of empty right edge: the price of rows that never change width. A design default.
- XAG 107 and the 10-foot guidance say paging should be on the pad's triggers; a keyboard player has PageUp and PageDown. No Home and End: a focused slider or choice keeps them, and the list would otherwise act twice.
- The rail is `2 × line` wide, a mouse target well under 44 px (WCAG 2.5.8 asks 24 px for a pointer, which it also misses). The wheel, the focus follower and a track click are the primary ways; a drag of the thumb is for a mouse player, and a list is never scrolled by a drag on its rows (that would be a path-based gesture, XAG 107).
- No scroll chaining: a list at its end does not scroll the panel behind it (Bevy's `ScrollArea` stops the event). Matches every list in the owner's editor.
- No accessibility node for the list or its rail (Bevy's `Scrollbar` carries none, by design: "scrollbars don't have an [`AccessibilityNode`] component"); a list role with position and size ("item 3 of 12") is a backlog item with the slider's and the toasts'.
- A row clicked while half-hidden is focused where it stands, not scrolled in, by L4; the next key press brings it in.

Assumed decisions (owner may override), all above: the model's `list(rows, children![..])` shape (rows as the second argument, since the wrapper, not the viewport, is the entity a game spawns and the viewport must be the rows' parent); `rows` in row-pitches rather than pixels; the lane always reserved; the margin of one widget height; the triggers for paging; Home and End left out.

## Pending index

- Nothing relevant in home-still (three queries on 2026-10-31 and `catalog_recent` limit 200). Index NN/g's *Scrolling and Attention*, Microsoft's *Gamepad and remote control interactions* and WCAG 2.2 *Understanding SC 2.4.11*. Search later for: "gamepad menu list scrolling", "ten-foot interface lists", "inventory screen design". The Game Accessibility Guidelines and *The Gamer's Brain* remain pending from `docs/research/gooey.md`. No shipping game's write-up of its own list scrolling was found; the shipping-game evidence here is Microsoft's console guidance and the owner's editor.
