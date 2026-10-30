# bevy_gooey loop log

## Iteration 0 — 2026-10-30 — repository setup

By the owner's session, not the loop. The crate's first slice: `GooeySkin` (dark and parchment presets, `text_scale`, `problems()`), `GooeyFonts`, `Role` and `apply_skin`; the widgets `screen`, `panel`, `menu`, `heading`, `label`, `button`, `meter`, `Tooltip`, `dialog`; `navigate_by_input` and `gamepad_select`; `UiFeedback`; `GooeyStylePlugin`, `GooeyNavPlugin`, `GooeyPlugins`; the gallery example with its capture mode. Design rules R1–R7 are in `docs/research/gooey.md`.

### Ranked backlog

Take these in order, after any bug and any filed issue:

1. `Shortcut { key: KeyCode, pad: Option<GamepadButton> }`, a component on a button. It triggers `Activate` when pressed, and the button shows its key glyph beside the label. Source: R5, novice-to-expert rehearsal.
2. `HudDetail { Full, Minimal, Hidden }`, a resource HUD widgets read. Source: R1, *Removing the HUD* (experts want less).
3. Slider, checkbox and toggle wrappers over `bevy_ui_widgets`.
4. A settings-screen kit for the accessibility options vocabulary: text size, contrast preset, remap row. Source: R2.
5. Scroll list, tabs and toast.
6. A moderate press and focus animation. Source: R4, "medium" juice.
7. A bevy_emerge consumer of `GooeySkin`: filed as [bevy_emerge#147](https://github.com/Ladvien/bevy_emerge/issues/147), "Family F13: Game UI skins (`bevy_gooey`)". It is built in bevy_emerge; this crate's part is to answer any `consumer:bevy_emerge` issue it raises.

### Known gap, seen while writing slice 1

- An open `dialog()` keeps Tab inside it (its modal `TabGroup`), but the arrow keys and the D-pad go through `AutoDirectionalNavigator`, which considers every `AutoDirectionalNavigation` entity on screen, so they can move focus to a button behind the scrim. Not yet reproduced in a test; it is a "dialog that leaks input" defect and ranks above the backlog once confirmed.
- A tooltip opens flush against its widget's bottom edge and covers the widget below it (seen on the gallery's Options tooltip over Quit, macOS 2026-10-30): it needs a gap from the skin and a placement that keeps the next control readable.
- A meter's fill has square corners inside the track's rounded ones (`Overflow::clip` clips to the rectangle, not the radius).

### Verified at setup (laptop, macOS, Apple M5, release build)

- `GOOEY_FRAMES=120` stills: dark, parchment and `GOOEY_TEXT_SCALE=1.5`; `mean_ms` 16.66 (vsync-bound) for each.
- Driven with real key events: Down then Enter on Options switches the skin (`feedback Focus`, `feedback Activate`); Shift+T enlarges every text; Enter on Quit opens the dialog with Stay focused; Tab cycles Leave and Stay only; Stay closes it and focus returns to Quit; Leave exits. Hovering Options shows its tooltip and the hovered fill.
- The loop runs on **sib** (`wiggum@gooey`, display `:93`, `~/bevy_gooey_loop`, target `~/target/gooey-loop`), moved there from big at setup because big was at load 73 on 24 cores with its swap nearly full. A 120-frame capture there renders (`mean_ms` 23.3 under Xvfb).

## Iteration 1 — 2026-10-30 — a dialog holds focus: it takes it, keeps it and gives it back

**Triage.** No open issues on Ladvien/bevy_gooey (`gh issue list`: empty). No parked branches (`wiggum/parked-*`: none). The item is the log's own recorded bug, "an open `dialog()` leaks arrow-key and D-pad focus", which ranks above every backlog feature. Runner-up: the tooltip covering the widget below it (also a recorded bug, but it costs readability for a moment of hover; the dialog leak strands a pad player's focus under the scrim).

**Reproduced before the fix.** `GOOEY_KEYS=down,down,enter,down` (Quit → dialog → Down): the ring sat on Quit under the scrim and neither dialog button showed focus (`/tmp/g1/before/dialog-down-1280x720.png`, `-2560x1440.png`). `tests/dialog.rs` failed on it: "ArrowDown left the dialog", and "no AutoFocus: its first button".

**What changed** (`cae7a4d`, `7d0a52f`, docs `ef77d89`, research `1c8a309`):
- `navigate_by_input` scores candidates with Bevy's own `find_best_candidate` and `AutoNavigationConfig`, but only within the focused widget's scope, the set Tab cycles: inside a modal `TabGroup`, that group; elsewhere, everything outside modal groups. Manual `DirectionalNavigationMap` edges are still followed first.
- `Dialog` (part of `dialog()`): an `Add` observer remembers the opener before the children's `AutoFocus` fires; `focus_opened_dialogs` (Navigate set) moves focus to the dialog's first tab stop unless focus is already inside, retrying until the dialog is visible (a fresh dialog's `InheritedVisibility` is hidden for a frame, so Tab order cannot see it yet); a `Remove` observer puts focus back on the opener if focus was inside the dialog or went with it.
- A panel whose parent is a dialog's scrim is opaque, `GooeySkin::solid_surface()`, because the dark preset's 0.92-alpha surface let "Play" ghost through "Leave the game?". `problems()` checks text against the same colour. Assumed decision: opaque dialog panels (kept: HUD panels stay translucent) over opaque surfaces everywhere, or a darker scrim (still ghosts).
- Gallery: `GOOEY_SIZE=<w>x<h>` and `GOOEY_KEYS=…` (real `KeyboardInput` from frame 30, one key every 10 frames). Its Quit dialog lost its `QuitButton` marker and the hand-written "focus Quit again": 7 lines fewer.

**Sources.** Craft and shipped games: Microsoft *XAG 112: UI navigation* ("The movement of focus should always be predictable"; Forza Horizon 4, Sea of Thieves, Halo Infinite). Technical: W3C APG *Dialog (Modal) Pattern* (focus moves in, Tab contained, focus returns to the invoker) and the Bevy 0.20 source. home-still distill answered `500: gpu busy` (bge-m3 needs 5000 MB, 4867 MB free beside a 14.6 GB vLLM engine) at the start and again mid-pass, so no corpus stem is cited; the searches are under the note's `## Pending index`. `catalog_recent` is not reachable from this session (no home-still MCP bridge mounted), so the corpus was not re-scanned.

**Checklist, before → after** (`/tmp/g1/findings.md`):
- Focus after Down in the dialog: on Quit under the scrim → on Stay; ring against panel 8.80:1 dark, 4.38:1 parchment.
- Closing: the gallery restored focus by hand → the crate returns it to Quit (`after/dialog-closed-1280x720.png`).
- Ghosting through the dialog panel (luminance spread over "Play"): 0.027 → 0.000.
- Dialog title on its panel: 14.06:1 → 14.26:1 dark; 9.81:1 parchment.
- Stay hit target ~46 px at 1280x720. `problems()` empty for both presets.

**Critique and verdict** (`/tmp/g1/critique.md`): dialog focus GREAT; opaque dialog panel GREAT. Flaws seen and left for later passes: the dialog has no elevation (no shadow; its edge sits about 6 px inside the menu panel behind, so it reads as a nested card); in parchment the dialog and the scrimmed panel behind are close in tone; and at 2560x1440 the whole UI is 1x logical size (20 px text on a 1440p screen).

**mean_ms** (sib, Xvfb, same session, back to back, 1280x720, two runs each): idle 23.15 / 23.17 before, 23.04 / 23.31 after; dialog staged 23.35 / 24.07 before, 23.35 / 23.20 after. No change. The 2560x1440 captures ran at about 113 ms before and about 101 ms after, under different load.

**Gate.** All six steps green; the first try lost `ld` to the OOM killer (15 GB RAM, parallel links of Bevy test binaries) and was rerun with `CARGO_BUILD_JOBS=4`.

**Testing note.** `tests/dialog.rs` lays the UI out in a headless window: `DefaultPlugins` with `RenderPlugin { backends: None }` and no `WinitPlugin`, a 1280x720 `Window`, `Camera2d`, keys sent as `KeyboardInput` messages. That is enough for `ComputedNode`, `UiGlobalTransform` and `ComputedUiTargetCamera`, so directional navigation can be tested.

### Ranked backlog

1. **The tooltip covers the widget below it** (recorded bug): it needs a gap from the skin and a placement that keeps the next control readable.
2. **A meter's fill has square corners inside the rounded track** (recorded visual bug).
3. **Couch-distance scale.** At 2560x1440 the UI is 1x (`after/dark-2560x1440.png`), and a 20 px label on a 1440p TV is too small. Make it a skin or settings option that scales `UiScale` from window height against a reference (1080p), off by default if the crate doc's "UiScale is the host's" stays. Research: XAG 101 (text size).
4. **Dialog elevation.** A `BoxShadow` on a dialog's panel from the skin, and a minimum dialog width, so the dialog reads as a layer above the screen and not a card nested in it.
5. `Shortcut { key, pad }` on a button (R5), then `HudDetail` (R1), slider/checkbox/toggle, settings kit, scroll list/tabs/toast, press animation, as in iteration 0.

**Release: not published.** bevy_gooey is not on crates.io yet (`crates.io/api/v1/crates/bevy_gooey`: 404), and `cargo publish --dry-run` of `1ac93bb` passes, but sib has no crates.io token (`~/.cargo` holds no `credentials.toml`, `CARGO_REGISTRY_TOKEN` is unset). Publishing needs `cargo login` on sib with the owner's token, and the loop does not handle credentials. No `release:` commit and no tag were made, so `## [Unreleased]` still holds 0.1.0's notes. The next pass with a token publishes 0.1.0.

**Cleanup.** Removed `/tmp/g1` (179 MB of captures, the before binary and scratch scripts); `/tmp` went from 354 MB used to 175 MB. No baseline worktree was made: the before binary was a copy of the pre-fix gallery build. `git worktree list` shows only the clone; no earlier pass left anything behind.
