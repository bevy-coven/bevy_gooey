# A screen panel never grows past the window

Researched 2026-11-09 (iteration 93). No issue is filed. The pass's own pick, and the limit iteration 92
measured and left open.

The problem, measured. A `panel()` sizes to its own widest row (`max-content`), which is right: a panel
should be as wide as what it holds. But a *screen* panel had no ceiling, so at `GOOEY_TEXT_SCALE=2` in a
1280x720 window one panel alone is wider than the window and its right edge, its border and its last
column of text leave the screen — measured in iteration 92's captures, and the reason that pass wrapped
the gallery's row rather than fixing the panel.

## Sources

1. WCAG 2.2 SC 1.4.10, Reflow (technical): "content can be presented without loss of information or
   functionality, and without requiring scrolling in two dimensions" at a width equivalent to 320 CSS px.
   A panel whose right half is off-screen loses both.
2. WCAG 2.2 SC 1.4.4, Resize text (technical): up to 200% without loss of content. Every crate widget keeps
   this promise inside its panel; the panel itself was the last thing that did not.
3. *Future design of integrated user interfaces* practice and Rune's layout (shipping reference): a panel
   is a column of rows, and a row wider than the panel wraps inside it. Nothing in the reference lets a
   card exceed its window — a card that did would be unreadable, which is the same argument.

## Craft principle

A panel is a container, and a container's job is to keep its contents visible. "As wide as my widest row"
is a good rule until the row is wider than the room; then the row must wrap, which means the panel needs a
ceiling it can pass down to its children.

## How shipping games do it

Rune's cards carry a width or a fraction of their parent, and their text truncates or wraps inside that
width. No shipping interface lets a settings panel grow past its window because one label is long.

## Model adopted

- **Attempted and reverted: a screen panel's `max_width` = the window less two gaps**, read from the layout
  target as a popover's is. It fixed the 200% overflow — `/tmp/g93/t.png` shows the panel inside the window —
  and it **broke a dock drag**: `tests/dock_reorder.rs::the_tab_follows_the_pointer_and_the_one_it_passes_steps_aside`
  failed with a tab landing at y=97.5 (a second row) before settling at 40.5. A dock group's card is a
  `panel()`, so the cap reached it; a cap in `apply_skin` cannot see the panel's own offset or the room its
  parent gives it, so a docked group measured against the whole window wraps its strip mid-drag. Reverted
  rather than patched around: the cap belongs where the panel's laid-out position is known, which is a
  system and a frame of latency, not `apply_skin`.
- **The next slice** is that system: cap a panel only when its own right edge would pass the window's, read
  from the last frame's layout — the same shape as `flow_strips`, and it must not touch a panel inside a
  sized container (a dock group, a split pane, a dialog).
- **Nothing else changes.** The panel still sizes to its content below the cap, so at 100% the layout is
  identical (the re-recorded GIFs are the evidence); above the cap the rows wrap, which is what a panel
  with a long label at 200% should do.
- **The cap is the window, not a text-size measure.** A measure in text sizes would be a second rule for
  panel width; the window is the room the panel actually has (design default, matching the popover).

## Parameters

| Name | Value | Where from |
|---|---|---|
| a screen panel's max width | `window.logical_size().x - 2 * skin.gap` | the popover's own rule |
| a dialog panel's | `text_px * DIALOG_MAX_TEXT_SIZES` | unchanged |

## Validation targets

- Measured before and after the reverted attempt: at 200% text in 1280x720 the panel's right edge moved
  inside the window (`/tmp/g93/t.png`), and at 100% the layout did not change (`/tmp/g93/n.png`).
- The attempt failed its second target: `dock_reorder`'s drag broke. Any future cap must keep
  `tests/dock_reorder.rs` and `tests/split.rs` green **and** fix the 200% overflow.

## Deviations

- **A screen panel does not centre itself.** It is the game's to place (`screen()` centres its children by
  default), so only its width is the crate's business (design default).
- **A nested panel is capped by the window too**, not by its parent. Capping by the parent would need the
  parent's laid-out width, which is a second source of truth for layout; the window is the one room a
  panel can never exceed (design default).

## Pending index

- Nothing new: WCAG and the crate's own layout rules are the bar.
- **Measured, not fixed:** a panel's *rows* still do not wrap their own text (a wide label is cut by
  `ONE_LINE`). A long row inside a capped panel keeps its width and is clipped; making every control's
  label wrap is a separate pass and would change `tests/large_text.rs`'s promise.
