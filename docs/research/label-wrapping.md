# A control's label inside a bounded panel

A control row — a toggle, a slider, a choice, a binding — puts its words beside its control. The
row's width is normally decided by those words: the panel it sits in is as wide as its widest child.
A dialog is the exception: its panel has a `max_width` (21 to 36 body text sizes, `DIALOG_*` in
`src/skin.rs`), so a label longer than the panel has nowhere to go.

## Sources

1. **Craft** — *Good Game Feel: An Empirically Grounded Framework for Juicy Design* (10.26503/dl.v2018i1.936),
   in the corpus.
2. **Technical** — WCAG 2.2 SC 1.4.4 *Resize text* and SC 1.4.10 *Reflow* (W3C Recommendation,
   2023-10-05), plus failure F69 *"text or controls clipped, truncated or obscured"*.
3. **Shipping-game feel** — Rune (`https://github.com/anomalyco/rune`), the open reference for this
   crate's look: its panels size to content and its dialogs clamp, which is the arrangement this
   crate adopts.
4. **Technical, layout** — the CSS Flexbox specification's *automatic minimum size* of a flex item
   (`min-width: auto` computes to the content's minimum), the mechanism that decides whether a label
   can shrink at all.

## Craft principle

*Good Game Feel* is about feedback a player can read, and its consistent finding is that the player
must be able to tell what the game did:

> "You should be able to estimate from the juiciness of each action the utility of that action."

A label that leaves its frame is the opposite: the player cannot even tell what the row says, so the
row's whole message — "this is the setting that stops the prompt" — is lost. Readability is the floor
under juiciness, not a refinement of it.

## How shipping games do it

Rune's settings panels are built from its kit's `panel` and its rows size to their content; a modal
is clamped to a measure and its contents wrap inside it. The general rule every toolkit arrives at is
the same: **a container that decides its own width sizes to its content, and a container that is
bounded wraps its content.** The bug here was a row that could not shrink inside a container that was
bounded — the one combination that has no answer.

## Model adopted

- A control's label is `LineBreak::NoWrap` by default (`ONE_LINE` in `src/widgets.rs`), because its
  row decides its own width. This is the existing, protected promise: a panel sized by its widest
  child must not wrap by a rounding error.
- A label with a `Role::Scrim` at or above it (a dialog, a modal popover — anything on the veil) takes
  `LineBreak::WordBoundary`. `wrap_bounded_labels` (`src/widgets.rs`) walks up the `ChildOf` chain,
  bounded to 16 hops, and writes the one field that differs; it runs in `GooeySystems::Style` right
  after `apply_skin`, which writes the rest of the text.
- The switch is a *break opportunity*, not a size: no colour, gap, target or font changes, so the
  rest of the skin's promises are untouched.

## Parameters

|What|Value|Why|
|---|---|---|
|Break rule outside a dialog|`LineBreak::NoWrap`|The row decides its width; `WordBoundary` would let a flex item's automatic minimum fall to its longest word|
|Break rule inside a dialog|`LineBreak::WordBoundary`|The panel has a `max_width`; the label must be able to shrink|
|Ancestry walked|`Role::Scrim` at or above, ≤ 16 hops|A dialog's panel is the direct child of the scrim; 16 is the deepest a game's own nesting goes in this crate|
|Written|`TextLayout::linebreak` only|`TextLayout` is not `PartialEq`; comparing the one field keeps an idle frame writing nothing|

## Validation targets

1. A control's label under a dialog panel draws **inside** the panel's rect at 1280×720: no pixel of
   it is more than 1 px outside. (Before: 60 px outside.)
2. It does break when it must: the same row's label is more than one line high.
3. The control beside it stays centred on the row (`AlignItems::Center`), so a two-line label does not
   push its knob or thumb off the row's middle.
4. **The protected promise holds**: outside a dialog, at every 5% from 100% to 200% text, a control's
   text is one line high across every row in the test (`tests/large_text.rs`).
5. Nothing is written while the interface is idle.

## What great looks like

- A dialog's words end where its frame ends, on every side, and a two-line label reads as one thought
  broken at a word boundary — never as a line running under the panel's edge.
- The control a long label belongs to (`the switch`, `the rail`) is still on the row's centre line, so
  the row reads as one control with a caption, not as a caption with a stray switch.
- Beside Rune: a dialog that clamps its measure and wraps what is inside, with no difference in the
  rhythm of its padding or gaps.

## Deviations

None. No default colour, size or gap changed; the one changed behaviour is a break opportunity, and
it is scoped to a container that is bounded.

## Pending index

- *The Gamer's Brain* (Nacke) and the Game Accessibility Guidelines: neither is in the corpus, and
  both would be the natural craft source for "text a player can read at couch distance". Listed, not
  yet downloaded.
- No corpus stem covers *CSS flexbox* itself; the mechanism is cited from the specification and
  measured in the crate's own tests (`tests/large_text.rs`), which is the stronger evidence here.
