# A toast is a live region

Researched 2026-11-09 (iteration 89). No issue is filed. The pass's own pick, from the gap the last two
passes' notes kept pointing at: what an assistive technology is *told* rather than what it can *ask*.

The problem, measured. `Role::Toast` has had a node since the tree was written — `Ak::Status` or
`Ak::Alert`, named with its words — and the node carried **no `live` region**, so a toast that appears
on its own was read only if the player happened to be exploring the tree at that moment. A sighted
player sees "Saved" slide in; a screen reader user hears nothing. Measured with
`node.live() == None` on both kinds before this pass.

## Sources

1. WCAG 2.2 SC 4.1.3, Status Messages (technical): "status messages can be programmatically determined
   through role or properties such that they can be presented to the user by assistive technologies
   without receiving focus". A toast is the textbook case: no focus, no place in the tab order, and it
   matters only when it arrives.
2. WCAG 2.2 SC 2.2.1 / ARIA's `aria-live` politeness (technical): `polite` waits for the user to pause,
   `assertive` interrupts. The crate already distinguishes `ToastKind::Info` from `ToastKind::Alert`
   for its *colour*; the politeness is the same distinction for a reader.
3. *Future design of accessibility in games: A design vocabulary* — 2019, IJHCS,
   doi:10.1016/j.ijhcs.2019.06.010 (craft): the paper's point is that an accessible design is a design,
   not an afterthought bolted onto the visuals — one kind of toast, two channels, one meaning.
4. Rune's toaster (the owner's reference, read not copied): an appearing message is registered as a live
   region with a politeness that follows its severity, and the same severity picks the colour.

## Craft principle

An interface that announces itself visually must announce itself otherwise. A toast carries no focus
and no target, so its *only* way to reach a reader who is not looking at the tree is a live region; a
node with a name is not enough.

## How shipping games do it

Rune pairs each toast's colour with a politeness: errors interrupt, confirmations wait. Nothing about
the visual changes, and a reader hears the message the player sees.

## Model adopted

- **`Ak::Alert` -> `Live::Assertive`, `Ak::Status` -> `Live::Polite`.** The crate's own `ToastKind`
  already draws that line for the fill and the mark; now one component decides both channels.
- **`Wanted` gained `live: Option<accesskit::Live>`**, `says` compares it, and `write` sets it — the same
  three places the sort direction needed last pass, and for the same reason: without the comparison a
  node that only started to be live is never rewritten (that trap cost a test run in iteration 88, so
  the comparison went in with the field).
- **No new action.** A toast's buttons already carry their own nodes and `Activate`.

## Parameters

| Name | Value | Where from |
|---|---|---|
| alert politeness | `Assertive` | WCAG 4.1.3, ARIA |
| info politeness | `Polite` | the same |
| which kind | `ToastKind`, the one the skin draws | `src/toast.rs` |

## Validation targets

- A plain toast's node has `Live::Polite` and an alert's `Live::Assertive`, beside the roles and names
  the existing test already checks.
- Mutation-checked: removing the `says` comparison fails the assertion (the node keeps `None`), and
  removing the `write` call fails it the same way.

## Deviations

- **A sticky toast is still `Polite`.** It does not go away, so interrupting for it would be louder than
  it is important; a game that wants an interruption sends `toast_alert` (design default).
- **A merged toast announces only its new text.** `merge_into_standing` bumps a count ("x3") rather than
  pushing the same message again, and a live region reads its changed label — so a reader hears "Saved
  x3", not "Saved" a third time, which is the desired economy (design default).

## Pending index

- Nothing new: the corpus's craft stem and WCAG are the bar.
- **Follow-up found, not built:** a list's rows and a tree's nodes have no nodes at all — Bevy's
  `ScrollArea` carries none — so a reader cannot read a scrolling list. Still the largest remaining gap.
