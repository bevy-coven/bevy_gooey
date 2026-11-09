# The gallery wraps its panels

Researched 2026-11-09 (iteration 92). Not a library feature: a correction to the crate's own showcase,
found by looking at it rather than at the code.

The problem, measured. `examples/gallery.rs` put its two panels in a row with `align_items: Center` and no
wrap. At `GOOEY_TEXT_SCALE=2` in the 1280x720 window — one of the four sizes `tests/large_text.rs`
promises — the row needs more width than the window and the second panel ran off the right edge, its
labels cut mid-word ("Autosa…", "Meadow, 1…"). The crate's own 200% promise is about the widgets; the
showcase that demonstrates them broke it, and every GIF recorded at 200% inherited the fault.

## Sources

1. WCAG 2.2 SC 1.4.4, Resize text (technical): content must be usable at 200% without loss of
   information or function, and a showcase cut in half is neither.
2. WCAG 2.2 SC 1.4.10, Reflow (technical): content must reflow at 320 CSS px without two-dimensional
   scrolling. A row of auto-width panels is exactly the layout that has to fold.
3. *Good Game Feel* — Hicks et al., 2018, DiGRA, doi:10.26503/dl.v2018i1.936 (craft): what a player sees is
   the game, so a demonstration that only works at one text size argues against its own claim.
4. This crate: `tests/large_text.rs` already lays every widget out at each `TEXT_SIZE_STEPS` and fails if a
   child leaves its parent — the promise the gallery was breaking one level up, at the window.

## Craft principle

A showcase is an argument. If the argument's own frame clips at the size the crate promises to support,
the argument is lost regardless of how correct the widgets are.

## How shipping games do it

Rune's gallery-style pages wrap their panels and centre each row: one column when wide, two when not.

## Model adopted

- **`flex_wrap: Wrap`, `row_gap` = `column_gap`, `justify_content: Center`, `max_width: 100%`** on the
  gallery's root row. At 100% the two panels stand side by side as before (identical pixels); past that
  they stack, centred, and nothing leaves the window horizontally.

## Validation targets

- At 1280x720 with `GOOEY_TEXT_SCALE=2`, no panel crosses the window's right edge.
- At 100% the gallery is unchanged (the re-recorded GIFs are byte-identical).

## Deviations

- **A single panel wider than the window still overflows.** The panels size to their own content
  (`max-content`), as every crate panel does; at 200% on a 640 px side one panel alone is wider than the
  window. That is the panel's own behaviour, and a game that hits it caps its panel's width — the gallery
  does not, and this pass does not change panel sizing (design default, out of scope).

## Pending index

- Nothing new: this is a showcase correction, not a research claim.
