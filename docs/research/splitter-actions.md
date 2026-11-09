# A splitter a screen reader can move

Researched 2026-11-09 (iteration 87). No issue is filed. The pass's own pick, from the gap the crate's own
splits guide admits and a measurement of the accessibility tree.

The problem, measured. `Role::SplitDivider` has had an `Ak::Splitter` role, a name and a numeric value
since `access.rs` was written, so a screen reader reads "Divider, 50 percent". It declared **no
actions** — `actions_of` had no arm for it (`src/access_actions.rs`) — so a reader had no way to move
it: `Node::supports_action` was false for every action, and a platform's "adjust" gesture did nothing.
The arrows work for a keyboard, and a drag for a pointer, and a screen reader had neither.

## Sources

1. *Future design of accessibility in games: A design vocabulary* — 2019, IJHCS,
   doi:10.1016/j.ijhcs.2019.06.010 (craft): the paper argues for design vocabulary "specific enough to
   inform application to a particular game", and the crate's own rule from it is that what a control
   offers must be what it shows and what it can be operated by. A splitter that reports a value and
   refuses every attempt to change it breaks that pairing.
2. WCAG 2.2 SC 4.1.2, Name, Role, Value (technical): for "all user interface components … the name and
   role can be programmatically determined; states, properties, and values that can be set by the user
   can be programmatically set". A value a user can set is the clause; reading one is not.
3. Rune's tree (the owner's reference, read not copied): every adjustable control it registers exposes
   the set/step actions with the live value, and its splitter is no exception.

## Craft principle

A value on screen is a promise that it can be changed. `Ak::Splitter` with a numeric value and no
`SetValue` is a promise broken for exactly the players who need the alternative most.

## How shipping games do it

Rune's kit attaches the adjustable-value actions to its layout splitter the same way it attaches them
to a slider; nothing on screen changes, and every way of operating it (pointer, keys, assistive
technology) ends in one `ValueChange`.

## Model adopted

- **`SPLIT` = Focus, Increment, Decrement, SetValue** on every enabled `SplitDivider`
  (`actions_of`, one new `divider` field in `Handles`), and none while it is disabled — the same gate
  every other action takes.
- **One path**: `Increment`/`Decrement` are `Reading::moved(±SPLIT_KEY_STEP_PX)`, the arrows' own step, and
  `SetValue` is `split_clamp(value, …)`, the drag's own hold to the minimums. Both end in the
  `ValueChange<f32>` on the split that `split_self_update` already turns into `SplitFraction`, so a
  game's observer hears a screen reader's change exactly as it hears a drag.
- **`SplitGeometry` does the reading.** It is already the crate's one reader of a divider's split
  (fraction, minimums, room, disabled), so the actions reuse it rather than re-deriving layout.

## Validation targets

- A divider's node reports `Ak::Splitter` and declares Focus, Increment, Decrement and SetValue, and
  does not declare Click (a divider is not pressed).
- `Increment` sends exactly one `ValueChange<f32>` on the split and widens the first pane; `SetValue`
  with 0.25 puts it at 0.25 within a hundredth; a value under a pane's minimum is held to it.
- A disabled divider declares nothing.
- Mutation-checked: removing the `SPLIT` arm fails the test on the first `supports_action`.

## Deviations

- **No Click.** A double click collapses a divider, and a screen reader's Click could map to it; it is
  left out because collapse is already reachable by setting the value to the first pane's minimum, and
  a Click that toggles would be a second path to the same state (design default).

## Pending index

- Nothing new: the corpus had the accessibility-vocabulary stem; WCAG 4.1.2 is the technical bar.
- **Follow-up found, not built:** a table's header cell is `Spec::Group` in `access.rs`, so the sort
  control is a container in the tree. Sorting by screen reader needs it to be a `Spec::Node` with a
  Click action, which is its own pass.
