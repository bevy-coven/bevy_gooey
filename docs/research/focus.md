# Focus a keyboard or pad player can always see and always reach

Researched 2026-10-30 (iteration 5), for issues #1 (arrows, D-pad and South never set `InputFocusVisible`) and #2 (with nothing focused, arrows and the D-pad do nothing). home-still's distill answered two searches at the start of the pass ("keyboard focus indicator visible … gamepad menu navigation", "switching between mouse and controller input …"): neither returned a game-UI source on focus visibility (the nearest hits were *ExposeHK* and *Usability Testing of Video Game Controllers*, on other questions). A third search mid-pass answered `500: gpu busy` (4866 MB free < 5000 MB for bge-m3). The sources below are web copies fetched this pass; the searches to repeat are under `## Pending index`.

## Sources

1. Microsoft, *Xbox Accessibility Guideline 112: UI navigation* — https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112 (craft; its *Grounded* example is a shipped game).
2. Lars Doucet, *The Steam Controller Configurator's Untapped Power*, Game Developer, 2017-01-25 — https://www.gamedeveloper.com/design/the-steam-controller-configurator-s-untapped-power (shipping game: *Defender's Quest: Valley of the Forgotten DX*, a mouse-first game given a pad cursor that coexists with the mouse).
3. W3C CSS Working Group, *Selectors Level 4*, §9.4 *The Focus-Indicated Pseudo-class: :focus-visible* — https://drafts.csswg.org/selectors-4/#the-focus-visible-pseudo (technical: the heuristics browsers ship for when a focus indicator shows).
4. Bevy 0.20 source: `bevy_input_focus-0.20.1/src/lib.rs` (`InputFocusVisible`, `on_window_acquire_focus_clear`), `tab_navigation.rs` (`handle_tab_navigation`, `TabNavigation::navigate`), `pointer_focus.rs` (`click_to_focus`) (technical: what the engine does and leaves to the UI framework).

## Craft principle

XAG 112: "The UI is fully navigable by keyboard and controller *digital* input alone." And: "If focus can be moved to a UI element of an interface, focus can be moved away from that element by using the same input (keyboard or mouse) method." And, for players on "keyboard-only or digital, mouth-operated joysticks": they "can be hampered or blocked from gameplay if focus doesn't move in a clear, consistent way. The movement of focus should always be predictable."

Both defects break this. A pad player whose D-pad moves an invisible focus cannot predict anything: the ring the skin draws never appears (#1). A player who once touched the mouse, or a Steam Deck's screen, can leave focus on nothing, and then no D-pad press ever reaches a widget again (#2): the interface is no longer navigable by digital input alone.

## How shipping games do it

XAG 112, on *Grounded* (Obsidian Entertainment) on PC: "players can use controller, mouse, or keyboard input to fully navigate the menu UIs."

Doucet, on *Defender's Quest DX*'s menus: "Console RPG's have a well-worn standard for menu navigation: the "finger cursor" … press up/down/left/right to navigate, "A" to select, "B" to go back." With a pointer in play: "whenever the game senses "mouse" movement, it hides the fixed-position menu finger cursor and instead shows the free-floating finger cursor which indicates mouse-like input … This lets the player fully control what input style is best for them -- they can navigate the menus with up/down/left/right actions, or using analog actions to move the finger pointer directly and click on the thing they want." And back again: "If you move the analog stick back to the right, the free-floating cursor turns back into a fixed cursor that moves by jumps." The hard part, in his words: "making sure that randomly clicking on stuff with the actual mouse while you were doing all this didn't make the universe explode."

So the shipped pattern is two indicators, one per input style, and the most recent input decides which is shown: a pointer hides the menu cursor, a directional press brings it back, and a click never strands the pad.

## Model adopted

Selectors 4 gives the rule browsers use for the same question: "If the user interacts with the page via keyboard or some other non-pointing device, indicate focus." "If the user interacts with the page via a pointing device (mouse, touchscreen, etc.) and the focused element does not support keyboard input, don't indicate focus." "If a newly-displayed element automatically gains focus (such as an action button in a freshly opened dialog), that element should indicate focus." Bevy leaves this to the UI: `InputFocusVisible` "should be set by your focus navigation solution", and only its Tab handler sets it; `click_to_focus` clears it on every pointer press. For arrows and the D-pad, this crate is that solution.

- **F1. Keys and the pad show focus.** An arrow key or D-pad press (`navigate_by_input`) and the pad's South button (`gamepad_select`) set `InputFocusVisible`, whether or not focus moves: the player is now on a non-pointing device. A pointer press hides it again (Bevy's own `click_to_focus`). Selectors 4's keyboard and pointer rules; Doucet's fixed and free cursors.
- **F2. A dialog that takes focus shows it.** `focus_opened_dialogs` sets `InputFocusVisible` once focus is inside a newly opened dialog, so a dialog opened with the mouse shows which button Enter or South will press. Selectors 4's "freshly opened dialog" rule.
- **F3. The flag is written only when it changes.** `apply_skin` restyles every widget when `InputFocusVisible` changes; a press with focus already shown must cost nothing.
- **F4. A direction with nowhere to start from lands where the screen starts.** When focus is on nothing, on a despawned entity, on a hidden one, or on something that is not a tab stop (Bevy focuses the window at start-up when nothing has `AutoFocus`; a click on empty space clears focus), the first arrow or D-pad press puts focus on the active scope's start, and moves nowhere else: the topmost open dialog if one is open (any visible modal `TabGroup`), otherwise everything outside modal groups. The start is the scope's `AutoFocus` widget (the topmost, if several), or else its first widget in Tab order, as Bevy's Tab picks with nothing focused. The next press moves from there. XAG 112's digital-input-alone rule; Doucet's "didn't make the universe explode".
- **F5. A host's own focusable keeps its arrows.** Focus on a visible tab stop without `AutoDirectionalNavigation` (a text field, a slider the host drives) is left where it is: Selectors 4 treats an element that "supports keyboard input" as its own, and stealing its arrows would make a text field unusable.

## Parameters

None: no setting is added. Visibility is Bevy's own resource; the start of a scope is the host's own `AutoFocus` or Bevy's own Tab order; "topmost" is Bevy's `ComputedStackIndex`.

## Validation targets

- After a pointer press (`InputFocusVisible(false)`), ArrowDown moves focus from Play to Options, `InputFocusVisible` is `true`, Options is drawn with `focus_ring`, and its tooltip opens (issue #1's test).
- The same with D-pad Down from a real gamepad, and South alone brings the ring back.
- A second press with focus already shown leaves `InputFocusVisible`'s change tick unchanged (F3).
- With focus cleared, the first ArrowDown lands on the screen's `AutoFocus` widget, the next moves on; focus on a despawned widget recovers the same way; with no `AutoFocus`, the first press lands on the first widget in Tab order (issue #2's test).
- With a dialog open and focus cleared, the first press lands on the dialog's `AutoFocus` button (Stay), never behind the scrim; without `AutoFocus`, on its first button.
- A focused tab stop without directional navigation keeps focus on every arrow.
- A dialog opened with focus hidden shows focus on its default button.
- Gallery stills at 1280x720 and 2560x1440: `GOOEY_KEYS=down` shows the ring on Options and its tooltip; `click,down` shows the ring on Play; `down,down,enter,click,down` shows the ring on Stay inside the dialog. Before: no ring in any of them.
- No frame-time cost: all of it runs only on a frame with a directional or South press.

## What great looks like

The ring is there the moment a thumb touches the D-pad, on the widget the player expects: where they were, or where the screen starts if they were nowhere. A mouse player never sees a stray ring; a pad player never presses into the void. Switching between the two is invisible: the indicator follows the hand. The focus ring is the same 3 px `focus_ring` stroke as before; nothing about its look changes, only that it is never missing.

## Deviations

- Issue #2 asks for "the first tab stop of the active scope". Adopted with one refinement: a scope's `AutoFocus` widget wins over its first stop, because `AutoFocus` is how the host names a screen's default (the README's advice) and how a dialog names its least destructive button (APG, `docs/research/dialog.md`). Without it, a click inside the gallery's Quit dialog followed by Down would land on Leave. Assumed decision (owner may override): `AutoFocus` first over first-in-Tab-order (lands on the destructive button) or the last focused widget (needs a memory of every scope; a later candidate).
- F2 goes beyond issue #1's fix direction (directions and South only): Selectors 4's "freshly opened dialog" heuristic. A design default.
- Not adopted: showing focus from the first frame for every game (Bevy's doc: "a console-style UI … may always have the focus indicator visible"). The resource stays `false` until a key or pad press, as Bevy ships it; a pad-first game inserts `InputFocusVisible(true)` itself. A design default.
- Not adopted: Enter and Space showing focus. They belong to `bevy_ui_widgets` (AGENTS.md), and they act on a widget that already has focus; a later candidate.

## Pending index

- distill was busy mid-pass (`gpu busy: 4866 MB free < 5000 MB required to load bge-m3`). Search the corpus for "focus indicator visibility keyboard gamepad", "hybrid mouse and controller input in game menus", "menu cursor console RPG", and cite *Future design of accessibility in games* (2019) if it names input-switching.
- Index XAG 112, Selectors 4 §9.4 and Doucet's article into home-still.
