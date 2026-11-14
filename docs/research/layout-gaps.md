# Seven layout gaps the editor hit retiring its raw nodes

Researched 2026-11-14 for issue #95 (G-84), asked by bevy_dark_factory (the Cauldron), #260 (UI-99, retire the editor's hand-built widgets), tracking #208. After G-82 the editor's `Node` literals went 230 to 197 using `row` and `column`; what was left needed a constructor, and the rule is that the editor never hand-builds a widget gooey lacks. Each item is a general need; the editor is the example and its name is in no API.

## Sources

1. CSS Flexible Box Layout Module Level 1, 8 (aligning: `align-items`, `justify-content`), 5.2 (`flex-wrap`), 4.5 (automatic minimum size of flex items): a flex item's `min-width`/`min-height: auto` is its content's size, so an item that is to give way, or to shrink to nothing, states `min-*: 0`.
2. Bevy 0.20 `bevy_ui::ui_node::Node` (read from the vendored 0.20.0 source): `align_items`, `flex_wrap`, `justify_content`, `flex_grow`, `flex_shrink`, `min_width`, `min_height`, `max_width`, `aspect_ratio`, `position_type`, `left`, `top`, `right`, `bottom`.
3. This crate: `src/layout.rs` and `docs/research/layout-primitives.md` (the gap is a multiple of the skin's gap), `docs/research/kit-parts.md` (G-82, the same method), `src/anchor.rs`.

## Method

A layout choice is data on the node, never a literal in the host and never a number in a skin: the constructor puts a `Role` (and, for a value, a small public component) on the node and `apply_skin` writes the `Node` from it, so the layout follows the skin's gap and the player's text size, and a host that wants a different value changes the component and the node follows in the same frame. A field a constructor does not own is left alone, so a host that patched one on a plain `row(..)` keeps it.

## What each item of the issue is

1. **Layout-aware `row` and `column`: `RowLayout { align, wrap, justify }`**, `row_with(gap, layout)`, `column_with(gap, layout)`, `button_row(gap)`. `RowLayout` is a public component (Bevy's own `AlignItems`, `FlexWrap`, `JustifyContent`; its default is each one's default), so it stands where a patched `Node` cannot: beside `row(1.0)` in a bundle, in a `Children::spawn(SpawnWith(..))` tuple. `apply_skin` writes it on a `Role::Row`, `Column` or `ButtonRow` **only when the component is on the node**, so a plain row keeps a field the host patched. A wrapping container puts the skin gap between its lines as well as between its children (`row_gap` and `column_gap` both `LayoutGap` times the skin's gap), which a bare `flex_wrap` never did. `button_row(gap)` is `Role::ButtonRow`: a row whose children are centred on one line and wrap onto the next, a gap below, when they do not fit; a `RowLayout` beside it overrides any of the three. The gap is a multiple of the skin's, so the PLAY and BUILD line passes `0.5`.
2. **A growing column that holds text and can narrow to zero: `fill_column(gap)`**, `Role::FillColumn`. A flex column (children stretched across it, `gap` skin gaps apart) with `flex_grow` 1, `flex_shrink` 1 and `min_width` 0 (CSS Flexbox 4.5: a flex item's automatic minimum is its content's, which for a long line of text is the whole line). Beside a fixed picture or a `count_text` it takes the room its row leaves, less the gap, and with less room than the picture alone it is nothing wide and the row stays as wide as its host. `fill_text` is a text that does this by itself; `fill_column` is the column that holds several parts. It takes a `RowLayout` like a row does.
