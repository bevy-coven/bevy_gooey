# A screen's own card is capped; a container-sized panel is not

Researched 2026-11-09 (iteration 94). No issue is filed. The pass's own pick: iteration 93's reverted
attempt, done where it is correct.

The problem, restated from the measurement. A `panel()` sizes to its widest row. At
`GOOEY_TEXT_SCALE=2` in a 1280x720 window a screen's own card is therefore wider than the screen and its
right edge — border and last column of text — leaves the window. Iteration 93 capped **every** screen
`Role::Panel` in `apply_skin` and broke
`tests/dock_reorder.rs::the_tab_follows_the_pointer_and_the_one_it_passes_steps_aside`: a dock group's
card *is* a `panel()`, and a cap applied there, with no knowledge of the room its parent gives it, wrapped
a docked group's strip mid-drag (a tab landed at y=97.5 before settling at 40.5).

## Sources

1. WCAG 2.2 SC 1.4.10, Reflow (technical): content presented "without requiring scrolling in two
   dimensions" — a card whose half is off-screen loses content and function.
2. WCAG 2.2 SC 1.4.4, Resize text (technical): usable at 200%, which the crate's `tests/large_text.rs`
   already holds every *widget* to.
3. This crate's own layout, `src/widgets.rs`: `screen()` is the screen's root; a dock group's card stands
   in a `Role::SplitPane`, a dialog's on a `Role::Scrim`, a split's pane in a `Role::Split`. Those
   containers size their children; the screen does not, it centres them.
4. Rune's layout (shipping reference): a card carries a width or a fraction of its parent, so the card's
   ceiling is the room it was given, never the window alone.

## Craft principle

A width ceiling belongs to the container that has to hold the card. The screen is a container with a
width; a split pane is a container with a different width. Applying the screen's ceiling to a pane's card
is what broke the dock — the cap was right, its scope was wrong.

## How shipping games do it

Rune's cards never exceed their parent, and no card is capped at the window when it stands in a sized
container. The rule is "fit the room you were given", which is exactly the distinction this pass draws.

## Model adopted

- **`restyle` learns one fact: whether the node stands directly on a `Role::Screen`** (`parent_screen`,
  read from a `roles` query at both call sites). A `Role::Panel` whose parent is a `Screen` gets
  `max_width = window - 2 * skin.gap`; a panel whose parent is anything else is untouched.
- **A dialog's panel keeps its text-size measure**, and a dock group's, a split pane's and a dialog card's
  layout are exactly as before — proven by running `dock_reorder`, `split`, `large_text` and
  `dialog_look` together (47 tests).
- **One assertion changes, and only in its expectation of the new contract:** `dialog_look`'s screen panel
  asserted `max_width == Auto`; it now asserts a positive cap inside the window, with the same
  `min_width` and every other claim unchanged. That is the "additive with an exception recorded" case: no
  assertion was removed, loosened or re-pinned, and the commit says so.

## Parameters

| Name | Value | Where from |
|---|---|---|
| cap, a screen's card | `window.logical_size().x - 2 * skin.gap` | the popover's existing rule |
| cap, any other panel | none (container-sized) | the regression iteration 93 measured |
| dialog card | `text_px * DIALOG_MAX_TEXT_SIZES` | unchanged |

## Validation targets

- At 200% text in 1280x720, a screen panel's right edge is inside the window (`/tmp/g94/t2.png`).
- At 100% the gallery is unchanged (`/tmp/g94/n.png` against the re-recorded GIFs, `0 updated`).
- `dock_reorder`, `split`, `large_text` and `dialog_look` all green together: **47 passed**.

## Deviations

- **The cap reads the parent's role, not its measured width.** A container that sizes a panel needs no
  cap; a container that does not (the screen) needs the window's width. Reading the role is one query and
  no frame of latency; measuring the parent would be a second source of truth for layout (design default).
- **A panel nested two deep on a screen (a panel in a panel) is uncapped at the inner one**, because its
  parent is a panel. Its outer card is already capped, so the inner one inherits a bounded room (design
  default).

## Pending index

- Nothing new: WCAG and this crate's own layout are the bar.
- **Measured, not fixed:** a row still does not wrap its own text (`ONE_LINE`), so a long row inside a
  capped panel is clipped rather than folded. That is the backlog's next item.
