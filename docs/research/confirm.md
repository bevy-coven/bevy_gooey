# A confirmation dialog in one line: `confirm`

Researched 2026-11-03 (iteration 58). home-still's distill answered `500: gpu busy: yielded to bevy_grimoire@big` and `hs status` a `401 Refresh token expired`, so the corpus could not be searched; the sources below are the three primary documents read directly (the Microsoft page fetched this pass), and the corpus searches are under `## Pending index`.

## Sources

1. Microsoft, *Command Buttons in Windows 7* (Win32 UX guide) — https://learn.microsoft.com/en-us/windows/win32/uxguide/ctrl-command-buttons (craft: the default button, the labels, Enter and Esc).
2. Microsoft, *Xbox Accessibility Guideline 112: UI navigation* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112 (shipping games: Forza Horizon 4, Halo Infinite; already the source of `docs/research/dialog.md`).
3. W3C WAI, *ARIA Authoring Practices Guide: Dialog (Modal) Pattern* — https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/ (technical: Escape, where focus starts, where it returns), and this crate's own `dialog()` (`docs/research/dialog.md`, `cancel.md`).

## Craft principle

Microsoft, *Default values*: "Include a default command button on every dialog box. **Select the safest (to prevent loss of data or system access) and most secure command to be the default**. … **Don't make a destructive action the default command button** unless there is an easy way to undo the command."

On the labels: "Prefer specific labels over generic ones. Ideally users shouldn't have to read anything else to understand the label." and "When asking a question, use labels that match the question. Don't use OK/Cancel to answer Yes/No questions." And on the keys: "Enter is the access key for the default button (which is usually the OK button), and Esc is the access key for Cancel."

So a confirmation has three decisions a game keeps getting wrong by hand: which choice has focus when it opens (the safe one), what Escape does (the safe choice, never a silent nothing and never the destructive one), and that the dialog closes whichever way it is answered.

## How shipping games do it

XAG 112, on Halo Infinite: "When the player selects a mission from the missions list, cursor focus is automatically brought to the area of the map in which the selected mission is located." Opening a layer moves focus into it and the player never hunts for it; the guideline's rule is "The movement of focus should always be predictable." A confirmation that opens with focus on the destructive choice, or on nothing, breaks that. [No shipped-game source in the corpus names the confirm dialog itself; its pending index below lists what to add.]

## Model adopted

`confirm(title, body, choices)` is `dialog()` plus a `panel()` of a `heading`, a `label` and a `menu` of `button`s, built once by the component's `on_add` hook (a `Toast` and a `MenuItem` do the same), and **one answer event**:

- **C1. One safe choice.** `Confirm::safe` is an index, the last choice by default (the Win32 guide's order puts the affirmative first and the dismissal after it: `["Leave", "Stay"]`); `.safe(i)` moves it. It takes Bevy's `AutoFocus` (APG: "If a dialog contains the final step in a process that is not easily reversible … it may be advisable to set focus on the least destructive action").
- **C2. Escape and the pad's East are that choice.** `Cancel` reaching the dialog (`cancel.md`) answers with the safe index and `cancelled: true`; APG "`Escape`: Closes the dialog." The crate otherwise closes nothing (`cancel.md`); a `Confirm` is the one widget whose whole job is to be answered, so it closes itself.
- **C3. One event, then gone.** Any answer (Enter, Space, South, a click, Escape) sends one `ConfirmChosen { entity, index, cancelled }` on the dialog and despawns it in the same flush, so focus returns by `dialog()`'s own rule (APG: "When a dialog closes, focus returns to the element that invoked the dialog").
- No new look: the panel is the dialog's panel, the buttons are `button()`s, so the skin, `problems()`, the 21 to 36 text size measure and the 5% safe edge are the dialog's.

## Parameters

None in the skin. `Confirm` is a component with the title, body, choices and safe index (serde-free data, `Reflect`).

## Validation targets

- `commands.spawn(confirm("Leave the game?", "Progress is lost.", ["Leave", "Stay"]))` opens a dialog with focus on **Stay**, and Tab, the arrows and the D-pad cannot leave it.
- Enter on Stay sends `ConfirmChosen { index: 1, cancelled: false }` once and the dialog is gone; a click on Leave sends `index: 0`.
- Escape sends `ConfirmChosen { index: 1, cancelled: true }` once and the dialog is gone; a pad's East the same.
- `.safe(0)` focuses and answers the first choice; no choices sends index 0 on Escape and breaks nothing; an empty body spawns no label.
- Focus is back on the opener after any answer.
- The screen it draws is the existing Quit dialog's (the gallery's Quit dialog is rewritten onto `confirm` and its capture is pixel-identical).
- No frame-time cost: observers fire on an `Activate` or `Cancel` only.

## What great looks like

The game writes one line and gets the dialog every shipping game has: a framed card over the dimmed screen, the question, what it costs, the safe choice already lit by the focus ring, and no way to lose focus or to leave it open by pressing Escape. The 12-line `dialog()` recipe stays for dialogs with their own content (a slider, a text field).

## Deviations

- Microsoft's "Esc is the access key for Cancel" assumes the dialog has a Cancel; a one-choice alert (`["OK"]`) has only that choice, so Escape answers it with `cancelled: true`. A design default.
- The default order is last-is-safe, not the platform's OK-then-Cancel: a design default (the gallery's "Leave / Stay"); `.safe(i)` covers the other order.
- `ConfirmChosen` carries an index, not the label: a game that translates labels still matches on its own constants. A design default.

## Pending index

- distill was unavailable (`gpu busy`; the bridge's token expired). Search the corpus for: "confirmation dialog default button destructive", "game menu dialog gamepad back button", and index the Win32 command-button guide and XAG 112.
