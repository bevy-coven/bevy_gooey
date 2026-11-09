# A list's rail, as the control that scrolls it

Researched 2026-11-09 (iteration 91). No issue is filed. The pass's own pick, and the half iteration 90 left
out: a list's rows and count are in the tree now, and the rail that scrolls them was `Spec::Hidden` —
invisible to a reader, so a screen reader user could read the first three rows and never reach row 20.

## Sources

1. WCAG 2.2 SC 2.1.1, Keyboard (technical): everything operable by pointer must be operable by keyboard,
   and by the same logic by an assistive technology that drives the keyboard model. A list scrolled
   only by wheel and drag fails it for a reader.
2. WCAG 2.2 SC 4.1.2, Name, Role, Value (technical): the rail's role is scroll bar, its name is what it
   scrolls, and its value is its place — the three the clause names.
3. *Future design of accessibility in games: A design vocabulary* — 2019, IJHCS,
   doi:10.1016/j.ijhcs.2019.06.010 (craft): a control that exists for two inputs and not the third is the
   incoherence the paper argues against.
4. AccessKit 0.24 and this crate's sliders: `Ak::ScrollBar` with `numeric_value`/`min`/`max`, and
   `Increment`/`Decrement`/`SetValue` are the actions a platform's scroll adapters send. Rune's scroll
   area (the owner's reference) registers the same three on its bar.

## Craft principle

The rail is the one part of a list a player can grab. If the tree says nothing about it, a reader has a
list they can hear and cannot move — the worse half of the gap iteration 90 closed.

## How shipping games do it

Rune's scrolling region pairs a list node (count) with a scroll bar node (value, and the actions that set
it), so a reader both hears "row 3 of 40" and can jump or step. Nothing visual changes.

## Model adopted

- **The rail is the node.** `Role::ListRail` moves from `Spec::Hidden` to `Spec::Node`, described
  `Ak::ScrollBar`, named "Scroll", with `numeric` = how far the list is scrolled (0 at the top, 1 at the
  bottom), read from Bevy's `Scrollbar.target` — the viewport it scrolls — and that viewport's
  `ComputedNode` (`scroll_position.y` over `content_size.y - size.y`).
- **`SCROLL` = Focus, Increment, Decrement, SetValue**, and each ends in the one state the wheel writes:
  `ScrollPosition`. Increment and Decrement move a viewport (`±node.size().y`, held to the ends) as
  PageUp and PageDown do; SetValue multiplies the fraction by the travel, as the rail's own drag does.
- **No new component and no new system.** The rail already carries Bevy's `Scrollbar`, whose `target` is
  the viewport; that is enough to read the value and to write the scroll.
- **The thumb stays silent.** It is drawn and moved by Bevy's scrollbar plugin from layout; it is not a
  control and takes no action.

## Parameters

| Name | Value | Where from |
|---|---|---|
| role | `Ak::ScrollBar` | WCAG 4.1.2, AccessKit |
| name | "Scroll" (a host's `AccessibleLabel` still wins) | the crate's naming rule |
| value | `scroll_position.y / (content_size.y - size.y)` | the same fraction the thumb is drawn at |
| step | one viewport | the Page keys' own step |
| set | the fraction × travel | the rail's own drag |

## Validation targets

- A list's rail node is `Ak::ScrollBar`, named "Scroll", declaring Focus, Increment, Decrement and
  SetValue.
- `SetValue` 1 scrolls the list to its bottom and `Decrement` goes back up by a viewport.
- A list whose rows fit has no travel, so its rail reports no value and its steps do nothing.
- Mutation-checked: removing the `rail` arm fails on the declared actions; removing `scroll_to` fails on
  the scroll assertion.

## Deviations

- **A viewport's `content_size` and a `ScrollPosition` are read from Bevy's own components** rather than
  from a crate-owned resource. That is where the truth already lives: the thumb is drawn from the same
  two numbers, so the value a reader hears is the value a player sees (one path, design default).
- **A step is a whole viewport, not a row.** A row-by-row step would be a second kind of movement the
  crate's own Page keys do not offer; matching them keeps one rule for every input (design default).

## Pending index

- Nothing new: the corpus's craft stem and WCAG are the bar.
- **Still open:** Bevy's `ScrollArea` itself carries no node, so a `ListView` used without this crate's
  crate-owned rail (a game wiring its own `ScrollArea`) still scrolls silently.
