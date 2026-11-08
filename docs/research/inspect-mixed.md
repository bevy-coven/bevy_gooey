# Inspector rows that read mixed over a multi-selection

Researched 2026-11-07 for issue #77 (G-67), asked by bevy_grimoire's UI series (bevy-coven/bevy_grimoire#208, UI-50 #236). No reference implementation: the behaviour is the one every multi-select inspector has (Unity's mixed-value dash, Godot's multi-node editing), asked for here as data.

## Model adopted

- **`InspectMixed(Vec<String>)`**, a public component on the inspector's root: the reflect paths (`"yaw"`, `"position.x"`, as in `InspectorChange::path`) whose values differ across the host's selection. The inspector reads one entity; the host keeps the list.
- **The rows it marks** are the ones the crate already has a mixed state for:
  - a number row and a colour row: `MixedValue` (the field shows `MIXED_HINT`, the row's own observer takes the marker off on an edit);
  - a vector row: `VecMixed(Vec<bool>)`, a flag per component. A component is mixed when its own path (`position.y`) or the vector's (`position`) is listed. A vector at the root has no prefix (`y`).
- **`sync_inspect_mixed`** (exclusive, in `GooeySystems::Style`, chained between `build_inspectors` and `sync_inspectors`) compares the list with the one it last applied (`Inspector::mixed`, private) and moves only the rows whose membership changed. So:
  - a list that has not changed writes nothing;
  - a row the player edited stays as the player left it until the list changes for its path (the host applies the edit to every member, then takes the path out of the list, which is a no-op for the row);
  - a list that empties, or a removed component, puts every named row back to the component's own value (the number field's own `RemovedComponents<MixedValue>` rewrites its digits).
- **The value under a mixed row** is the inspected entity's. `sync_inspectors` keeps writing `NumberValue` and the others as before; the field shows the hint while `MixedValue` is there.

## Where this differs from the issue

- **Toggles, choices and text rows.** The issue shows a toggle or choice "in its indeterminate state where the crate has one, otherwise as unchecked with the `--` hint on its label". This crate has no indeterminate state for either, and rewriting a label's text would fight the row's own text; they keep the component's value and nothing is marked. Not built, said on the issue.
- **A path the type does not have** is ignored, as asked, but is not listed in a `problems()` line: `problems()` belongs to the skin, and the inspector has no report of its own to put a line in.

## Proof

`tests/inspect_mixed.rs`, 6 tests, headless window (mutation-checked on a copy of the tree: marking nothing fails three of them, re-marking every named row on any change fails the sixth):

- The issue's acceptance: a struct with `yaw` and `x`, `InspectMixed(["yaw"])`: the yaw row has `MixedValue` and the hint in its field, the x row has its number; the player's edit of yaw (90) sends one `InspectorChange` for `yaw`, writes the field, and the row no longer reads mixed or shows the hint, and stays so while the list still names it.
- Taking `yaw` out of the list shows the component's own value (12.5) with no hint; a list of two, then one, then the component removed.
- A vector marks `position.y` only (`VecMixed([false, true, false])`) and a colour row reads mixed; `position` marks all three; an empty list takes `VecMixed` off.
- A path the target does not have and a toggle are left alone.
- Twenty frames with an unchanged list write no `MixedValue` or `VecMixed`.
- A row the player edited is not marked again when the list gains another path while still naming it; taking the path out and putting it back marks it again.
