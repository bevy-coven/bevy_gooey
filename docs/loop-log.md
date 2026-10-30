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

## Iteration 2 — 2026-10-30 — a tooltip opens on focus, beside its widget, never over the next one

**Triage.** No open issues on Ladvien/bevy_gooey (`gh issue list`: empty). No parked branches (`wiggum/parked-*`: none). The item is the log's top recorded bug, "the tooltip covers the widget below it". Seen with a pad in hand it was worse: `Tooltip` opened only on `PointerOver`, so a keyboard or pad player never saw one (`/tmp/g2/before/options-focus-1280x720.png`: focus on Options, no tooltip). Both are one widget's defect and were fixed together. Runner-up: the meter fill's square corners (a recorded visual bug, but it costs polish, not information).

**What changed** (`da5c9a8`, `8880a02`, docs `471bd3d`, research `e3f5ddd`):
- `show_tooltips` (Style set, before `apply_skin`) opens a tooltip while its widget is `Hovered` or has focus with `InputFocusVisible`, and shuts it otherwise. It replaces the two pointer observers. A non-button widget gets a `Hovered` on adding `Tooltip`.
- `apply_skin` places the box with Bevy's `Popover`, across the container's flow (Right, Left, then Below, Above in a column; Below, Above, then Right, Left in a row), centred on the widget, one `gap` outside the widget's panel, `gap` from the window's edge. It wraps it to at most 24 text sizes wide, or to the room beside the widget in a column. It draws it as a small panel: `solid_surface()` fill, `surface_raised` border, drop shadow in `background`.
- `TooltipBox` (public) marks the box and records `in_row` and `room`.
- Assumed decision: placement across the flow, over always-below (covers the next widget in a column) or a fixed description area (a layout the game owns).
- Gallery hero (`docs/media/gallery.png`) is now a 1280x720 capture with Options focused and its tooltip open; the caption says so.

**Sources.** Craft: Microsoft *XAG 112* ("All aspects of the game, including menu UI's and gameplay, are fully navigable by multiple inputs"). Shipping game: Lars Doucet on the *Defender's Quest DX* gamepad port ("Tooltips -- will players using a controller be at a disadvantage?"; the pad layout "provides all the same tooltip information"). Technical: WCAG 2.1 SC 1.4.13 *Content on Hover or Focus*, and Bevy 0.20 `bevy_ui_widgets/src/popover.rs`. home-still distill answered `500: gpu busy` (4867 MB free < 5000 MB for bge-m3) again. The corpus stem `10.1145_2659796` (*Supporting Novice to Expert Transitions*) is a 40 kB partial conversion with no tooltip content. The searches are under the note's `## Pending index`. `catalog_recent` (200): no new game-UI document, so no corpus commit.

**Rounds** (`/tmp/g2/critique.md`):
1. Beside the widget at `gap`, but it straddled the panel's border (panel edge x 752, tooltip x 743) and, in a button's `surface_raised` fill, read as a fourth button. FAIL.
2. One `gap` outside the panel and drawn as a small panel, but at `text_scale` 1.5 the 520 px line fitted neither side, and `Popover` fell back to Below over Quit. FAIL.
3. Wrapped to the room beside the widget. GREAT.

Flaws left: the wrapped box keeps its full width (about 90 px empty right of "and" at 1.5x); the dark skin's shadow is nearly invisible (background on background); the pointer crossing the 27 px gap closes the tooltip (WCAG *Hoverable* deviation, logged).

**Checklist, before → after** (1280x720 and 2560x1440, dark, parchment, 1.5x, idle and `GOOEY_KEYS=down`):
- Tooltip on keyboard or pad focus: never → shown beside Options, centred on it (`after/*-options-*.png`).
- Tooltip over the next widget: flush under Options, over Quit → no overlap with any sibling (`tests/tooltip.rs`), edge 11 px outside the panel (x 763 vs 752).
- Tooltip text contrast (pixels): dark 14.26:1, parchment 10.24:1.
- Idle captures: before/after difference only in the moving meter fill (diff bbox x 674–696, y 479–527).
- Hit targets, focus ring and `problems()` (empty for both presets): unchanged.

**Tests.** `tests/tooltip.rs`, laid out headless:
- focus opens the tooltip beside the widget, clear of Play and Quit, `gap + padding + border` away and centred; it shuts when focus leaves or focus is not visible;
- a real pointer move opens and shuts it with focus not visible;
- at the window's right edge it opens on the left;
- in a row it opens below, clear of its neighbours.

**mean_ms** (sib, Xvfb, 120 frames): 1280x720, before: dark 24.24, parchment 26.28, 1.5x 26.51, focus on Options 26.70. After: dark 24.91, parchment 24.97, 1.5x 24.85, focus on Options 24.93 / 24.98 / 25.09. 2560x1440, before: 107.9–134.7; after: 94.1–99.4. No regression.

**Gate.** All six steps green (`CARGO_BUILD_JOBS=4`). The first clippy run failed on `type_complexity` in `show_tooltips`, fixed in `8880a02`.

**Release: not published.** sib still has no crates.io token (`~/.cargo/credentials.toml` absent, `CARGO_REGISTRY_TOKEN` unset). `## [Unreleased]` holds 0.1.0's notes.

**Cleanup.** Removed `/tmp/g2` (357 MB: captures, both gallery binaries, scratch scripts). `/tmp` went from 645 MB used to 288 MB. No baseline worktree was made: the before binary was a copy of the starting build. `git worktree list` shows only the clone; earlier passes left nothing.

### Ranked backlog

1. **A meter's fill has square corners inside the rounded track** (recorded visual bug).
2. **Couch-distance scale**: at 2560x1440 the UI is still 1x (20 px text on a 1440p TV). Scale `UiScale` from window height against 1080p, as a setting. Research: XAG 101.
3. **Dialog elevation**: a `BoxShadow` and a minimum width on a dialog's panel, reusing the tooltip's shadow recipe, so the dialog reads as a layer.
4. **Tooltip polish**: balanced wrapping (the box keeps its full max width when wrapped), and a shadow that shows on the dark skin.
5. `Shortcut { key, pad }` on a button (R5), then `HudDetail` (R1), slider/checkbox/toggle, settings kit, scroll list/tabs/toast, press animation.

## Iteration 3 — 2026-10-30 — a meter is one framed shape, its whole length legible, as large as the HUD

**Triage.** No open issues on Ladvien/bevy_gooey (`gh issue list`: empty). One parked tree: `WIP(pass 3, parked)` `e9c3da6` on `loop/iter-3`, the cut-off pass 3's draft of the log's top backlog item (the meter's square corners), with its research commit `2b53257`. It was the item this pass would pick, so it was adopted: the research commit cherry-picked (`c4eb24b`), `src/skin.rs`, `src/widgets.rs` and `tests/skin.rs` taken from `e9c3da6`, then finished and gated here. The branch is `loop/iter-3b` because `loop/iter-3` is the parked branch, left as it is. Pass 3's scratch is still in `/tmp/g3` (see Cleanup). Runner-up: couch-distance scale (a 1x UI on a 1440p screen hurts every widget, but the meter's defects are a wrong widget, which ranks first).

**What was wrong** (`/tmp/g3b/findings.md`): the fill's square corners covered the track's rounded ends, and a full meter was a square slab; the track's length was 1.39:1 (dark) and 1.25:1 (parchment) against the panel, so 60 of 100 read as a short full bar; the bar stayed 14 px at text scale 1.5, under a 30 px label.

**What changed** (`cdd5fc5`, format `551b38f`, docs `a51bbe5`, research `517531f` on top of pass 3's `c4eb24b`):
- The track is framed `border` px wide in a new `GooeySkin::outline` colour; the fill sits one `border` inside the frame, on `surface_raised`, with radius `max(radius − 2 × border, 0)`, concentric with the frame (CSS's content-edge radius; Bevy clips a child to a rectangle, so the fill rounds itself).
- `GooeySkin::meter_height` (24) × `text_scale` sizes the bar; its least length is 8 text sizes (160 px at 1x).
- `problems()` checks `accent on surface_raised` (fill on track) and `outline on surface` (frame on panel) at 3:1, and reports the new `SkinProblem::MeterTooThin` when the frame and gap leave no room for a fill.
- Gallery: `GOOEY_HEALTH=<value>` holds the meter for a still. Hero `docs/media/gallery.png` regenerated (Options focused with its tooltip, health 70).
- Assumed decision: a frame in a new `outline` colour (Material 3's *Outline* role), over Material's stop dot at the track's end (easy to lose at couch distance) or a frame in `text_muted` (round 1: as bright as the focus ring).

**Sources.** Craft: corpus stem `10.26503_dl.v2011i1.514`, Llanos & Jørgensen, *Do Players Prefer Integrated User Interfaces?* ("information that is critical, or that needs to be gauged on a continuous basis"), and Hodent's GDC 2015 write-up (Gestalt similarity). Shipping game: Naughty Dog, *The Last of Us Part II* accessibility options ("HUD Scale … Changes the size of in-game HUD elements"). Technical: Material's progress indicator *Non-text contrast update* (`indicatorTrackGapSize`), Material 3 colour roles (*Outline*), WCAG 2.2 *Understanding 1.4.11*, CSS Backgrounds 3 §4.2–4.3, and Bevy 0.20's `clipping.rs` and `ui.wesl`. Every quote was checked against a fetched copy. home-still: distill healthy and idle; scribe on big `backend_unavailable`, on bmb healthy with no free slot. `catalog_recent` (200): forestry, medicine and LLM papers only, no game-UI document, so no corpus commit.

**Rounds** (`/tmp/g3b/critique.md`):
1. Frame in `text_muted` (6.6:1 in the dark skin): the extent read, but the frame was as bright and as thick as the focus ring and the meter looked like a text field (`r1/dark.png`). NOT YET.
2. Frame in `outline`, about 3.7:1 in both skins: quieter than the ring, louder than the panel border. GREAT.

Flaws left: with a 3 px gap the fill's corners are 2 px, so at 1.5x the 24 px fill reads close to a rectangle (concentric, accepted); the UI is still 1x at 2560x1440.

**Checklist, before → after** (1280x720, dark / parchment):
- Fill corners: square, over the track's rounded ends → 2 px, concentric with the frame; no fill pixel outside the track (`after/z-all.png`).
- Meter extent against the panel: 1.39 / 1.25 → 3.67 / 3.96 (pixels).
- Fill against track: 6.03 / 4.69, unchanged.
- Height at 1x / 1.5x: 14 / 14 → 24 / 36 px (fill 12 / 24).
- Fill proportion: 115 of 192 px at 60 → 108 of 180 px interior at 60, 7 at 4, 180 at 100.
- Focus ring, hit targets, text in boxes, scrim: unchanged. `problems()` empty for both presets.

**Tests.** `tests/meter.rs`, laid out in a headless window: the fill sits `2 × border` inside the frame on every side, as long as its value says, rounded and concentric with the frame; the bar is `meter_height` high and `1.5 × meter_height` at 1.5x, the fill taking the growth; a near-empty meter draws a sliver and an empty one none. `tests/skin.rs`: a fill, a frame or a meter height that hides the reading is reported. The parked draft's test that copied `Node` fields was replaced by the layout test.

**mean_ms** (sib, Xvfb, 240 frames, 1280x720, `GOOEY_HEALTH=60`). Sequential runs were swamped by other loops on the host (load 7 to 60, kswapd active): before median 26.76 (8 runs), after 29.19 (8 runs), paired differences −5.1 to +9.1 ms. Run side by side, six pairs, the paired difference is −0.06 ms mean (−0.42 to +0.24). No regression: the change adds one border to one node and nothing per frame.

**Gate.** All six steps green. The first run sat for an hour linking `--features serde` test binaries while sib thrashed (memory pressure "full" 66%, load 60: three loops linking Bevy at once) and was stopped; rerun with `CARGO_BUILD_JOBS=1` once the load fell, every step passed in under a minute.

**Release: v0.1.0 published**, the crate's first: https://crates.io/crates/bevy_gooey/0.1.0. sib now holds a crates.io login (`~/.cargo/credentials.toml`, dated 2026-10-30 10:06), so the first green-gated pass published as the owner allowed. Nothing was on crates.io before, so `cargo semver-checks` had no baseline; the version is the manifest's 0.1.0. `release: v0.1.0` (`b5cd4a8`) moved `## [Unreleased]` to `## [0.1.0] - 2026-10-30`; `cargo publish --dry-run` then `cargo publish` from the clean tree; tag `v0.1.0` pushed; `cargo info bevy_gooey@0.1.0` resolves from the registry. The next release may go out after 2026-10-31 00:50 local (six hours).

**Cleanup.** Removed `/tmp/g3b` (359 MB: both gallery binaries, captures, rounds, scratch). `/tmp` went from 2.3 GB to 2.0 GB used (other loops share it). No baseline worktree was made (the before binary was a copy of the starting build with the `GOOEY_HEALTH` knob); `git worktree list` shows only the clone. Left for a person: `/tmp/g3`, 181 MB, the killed pass 3's captures, `gallery-before` binary, `critique.md` and a `gestalt.pdf`, untouched since 10:33 except that this pass moved its own files out of it; and the branch `loop/iter-3` (local and on origin) holding `WIP(pass 3, parked)` `e9c3da6`, now superseded by this iteration.

### Ranked backlog

1. **Couch-distance scale**: at 2560x1440 the UI is still 1x (20 px text on a 1440p TV). Scale `UiScale` from window height against 1080p, as a setting. Research: XAG 101.
2. **Dialog elevation**: a `BoxShadow` and a minimum width on a dialog's panel, reusing the tooltip's shadow recipe, so the dialog reads as a layer.
3. **Slider, checkbox and toggle** over `bevy_ui_widgets`: their box and track take `GooeySkin::outline`, the colour this pass added for edges a fill does not show.
4. **Meter damage trail**: the lost part of a meter lingers in a lighter tone, then drains (League of Legends' health bar update makes "current health more visible"); needs a shipping-game write-up in the corpus first.
5. **Tooltip polish** (balanced wrapping, a shadow that shows on the dark skin), then `Shortcut { key, pad }` (R5) and `HudDetail` (R1).

## Iteration 4 — 2026-10-30 — the interface keeps its share of the screen at every resolution

**Triage.** No open issues on Ladvien/bevy_gooey (`gh issue list`: empty). No new parked branch: `git log --all --grep=parked` shows only `e9c3da6` on `loop/iter-3`, adopted and shipped by iteration 3. No bug recorded in the backlog, so the item is the backlog's first: couch-distance scale. It was measured as a defect before it was built: at 2560x1440 the tooltip line's body height (XAG 101's measure, ascender to descender) is 21 px, under XAG's 24 px PC and 34.7 px console minimums for 1440p, and the menu panel is 21.7 % of the window's height against 43.3 % at 720p (`/tmp/g4/findings.md`). Runner-up: dialog elevation; scale ranked first because it decides whether any widget can be read on a television, and every later widget inherits it.

**What changed** (`6615933`, docs `f6837a5`, research `0f0e85e`):
- `GooeyScalePlugin`, the third plugin in `GooeyPlugins`: `scale_with_window` (Style set) sets Bevy's `UiScale` to the primary window's shorter side over `UiScaleSettings::reference_short_side` (720), times the player's `UiScaleSettings::scale` (1.0), writing it only when it changes. 1920x1080 → 1.5, 2560x1440 → 2, 1280x800 → 1.11, portrait 720x1280 → 1. A minimised (0x0) window keeps the last scale; a reference or scale at or below zero warns and leaves `UiScale` alone.
- `UiScaleSettings` (serde data with `serde(default)`) and `model::window_ui_scale`, the engine-free maths with its own tests.
- Crate doc's *Units*: sizes are logical px at a 720-px short side; the README gains *Screen size* and an install from crates.io (it still said "until the first release").
- Assumed decision: on by default in `GooeyPlugins` (Unreal applies DPI scaling "to every project"; a game that never tests at 1440p still ships legible text), over opt-in. A host that drives `UiScale` itself leaves the plugin out. Reference 720, not Unreal's 1080: the skin's sizes were set and measured at 1280x720, and at 720 they meet XAG's console minimum at every resolution (31.5 px body at 1080p ≥ 26).

**Sources.** Craft: corpus stem `10.1016_j.ijhcs.2019.06.010` (*Future design of accessibility in games*: "resolution of screen could make text too small"), XAG 101 ("Console: … 26 px at 1080p, 52 px at 4K"), the Game Accessibility Guidelines' font-size page (28 px at 1080p). Shipping game: Wube, *Factorio Friday Facts #277* ("the proportions of everything stay the same, so it is just smaller, but not deformed"). Technical: Unreal's *DPI Scaling* ("Shortest Side … (Most Common Setting)", "default DPI scaling settings applied to every project"), corpus stem `W1728842521` (scikit-learn: "Sensible defaults"), CSS Values 4 ("snap a length as a border width", for the next item), Bevy 0.20 `update.rs` and `popover.rs`. home-still: distill healthy and idle; scribe on big `backend unavailable`, on bmb healthy and busy; `catalog_recent` (200): no new game-UI document.

**Checklist, before → after** (`/tmp/g4/findings.md`, `critique.md`):
- Body height of the tooltip line: 720p 21 → 21 (identical), 1080p 30 (≥ 26 console, ≥ 28 GAG), 1440p 21 → 41 (≥ 34.7), 1280x800 23 (≥ 19.3).
- Panel share of the window height: 720p 43.3 %, 1080p 42.8 %, 1440p 21.7 % → 43.2 %. 1440p halved against 720p: RMSE 1.03 %.
- Focus ring 3 px at 720p → 4 px at 1080p, 6 px at 1440p; hit targets 44 px at 720p → 88 px at 1440p (button fill 41 → 82).
- Tooltip at 2x: beside Options, centred, clear of Play and Quit, inside the window. Dialog at 1.5x and 2x: scrim, opaque panel, Stay focused.
- Text contrast, ring contrast, meter fill (108 of 180 px at 60), `problems()`: unchanged; all five 1280x720 captures pixel-identical to the baseline.

**Critique and verdict** (`/tmp/g4/critique.md`): scale with the window, GREAT. Flaw found and left as the next item: at a fractional scale Bevy's layout rounds each border and padding to physical pixels from the node's sub-pixel position, so at 1920x1080 (1.5x) the meter's gap is 5 px above the fill and 4 px below (`after/z-1080-meter.png`). Not new: a window scale factor of 1.5 (a 150 % laptop) did the same before. At 1280x800 the meter is even (3 / 3 / 14 / 3 / 3). Capture note: the Xvfb pointer sits at (960, 540) and is back there after each gallery run, so every 1920x1080 capture shows Options hovered.

**Tests.** `tests/scale.rs`, laid out headless: a button at 2560x1440 is drawn twice its 1280x720 height; a resize to 1920x1080 draws it at 1.5x the next frame, a portrait window by its width, a 0x0 window keeps the scale; the player's `scale` multiplies the window's, a 1080 reference draws 1080p at 1x, an invalid reference leaves `UiScale`; a host that leaves the plugin out keeps its own `UiScale`. `src/model.rs`: `window_ui_scale` cases. `tests/serde.rs`: a settings file with only `scale` keeps the reference.

**mean_ms** (sib, Xvfb, 1280x720, `GOOEY_HEALTH=60`): first captures, before 24.58 / 25.80 / 27.37 / 27.45 / 27.20 (dark, parchment, 1.5x, Options, dialog), after 24.73 / 24.70 / 25.75 / 24.73 / 24.09. Five side-by-side pairs, 240 frames: after − before −0.08, −0.02, −0.31, −0.07, +0.44 ms, mean −0.01. No regression. 2560x1440 (Xvfb, software-bound by window size): before 104–120, after 99–121.

**Gate.** All six steps green on `f6837a5` (100 tests passed, 0 failed). The first `cargo test` lost three test-binary links to the OOM killer (`ld terminated with signal 9`, memory pressure "full" 35–44 % with other loops linking); rerun with `CARGO_BUILD_JOBS=1`, every step passed.

**Release: not this pass.** v0.1.0 was tagged at 13:50 today; the six-hour rule allows the next release after 19:50. `## [Unreleased]` holds this change for it; that pass decides the version with `cargo semver-checks` (the API change is additive, but `UiScale` is now set by the crate, a behaviour change its log should weigh).

**Cleanup.** Removed `/tmp/g4` (358 MB: both gallery binaries, 30 captures, zooms, findings, critique, gate log, scratch); `/tmp` went from 1.4 GB to 1.1 GB used. No baseline worktree was made (the before binary was a copy of the starting build); `git worktree list` shows only the clone. Left for a person, as iteration 3 listed: `/tmp/g3` (181 MB, the killed pass 3's scratch, untouched for over two hours) and the branch `loop/iter-3` holding `WIP(pass 3, parked)` `e9c3da6`, superseded by iteration 3's merge.

### Ranked backlog

1. **Line weights snap to whole device pixels** (a measured visual defect at fractional scales, 1080p and 150 % displays): at 1.5x the meter's gap is 5 / 4 px. Snap each border and the meter's gap to whole physical pixels, as CSS's "snap a length as a border width" does (round down above 1 px), from the node's `ComputedUiRenderTargetInfo::scale_factor`, and restyle when it changes. Validation: 4 / 4 / 20 / 4 / 4 px meter at 1080p, even rings at 1.11x and 1.5x.
2. **Dialog elevation**: a `BoxShadow` and a minimum width on a dialog's panel, reusing the tooltip's shadow recipe, so the dialog reads as a layer.
3. **Slider, checkbox and toggle** over `bevy_ui_widgets`, framed in `GooeySkin::outline`.
4. **A settings-screen kit** for the options this crate now has: text size (`GooeySkin::text_scale`), interface size (`UiScaleSettings::scale`), skin preset. Source: R2, XAG 101's "provide players the option to scale larger or smaller".
5. **Tooltip polish** (balanced wrapping, a shadow that shows on the dark skin), then the meter damage trail, `Shortcut { key, pad }` (R5) and `HudDetail` (R1).
