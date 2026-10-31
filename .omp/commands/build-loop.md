---
description: One iteration of the bevy_gooey improvement loop
argument-hint: "[optional focus, e.g. 'gamepad only']"
---

One run of this command is one iteration of a long-running loop on bevy_gooey, a Bevy 0.20 crate that makes game interfaces easy: a skin, ready-made game widgets (buttons, panels, menus, HUD meters, tooltips, modal dialogs), keyboard and gamepad navigation and UI feedback, on Bevy's own headless widgets. Do ONE improvement per iteration, the single most valuable one from a game developer's point of view, finish it completely, then stop.

This crate must be easy AND good-looking AND fast: a widget that needs more than a few lines to use is not done, one that looks merely fine next to shipping games is not done, and one that costs frame time a HUD should not cost is not done. The judgement here is a game UI designer's: legibility at couch distance, a focus that is always visible, hit targets a thumb can hit, one visual language across every screen. Its API is the family's: flat re-exports and no prelude, independent plugins in `GooeyPlugins`, systems in `GooeySystems`, settings as serde data, engine-free maths in `model`. Its design rules are in `docs/research/gooey.md`.

You do all the work yourself, including looking at the images and judging them. Do not delegate
visual review.

Extra instructions for this run (a focus such as "gamepad only"): $ARGUMENTS
They override the choice of item in section 4, not the gates.

State lives in `docs/loop-log.md`; rules and traps live in `AGENTS.md`. Follow them; do not
re-derive them.

## Your work is saved even if you are killed — read this before you start

**The driver saves whatever you leave uncommitted, without touching your branch.** Every ten minutes
it records the whole tree to a snapshot ref (`refs/wiggum/pass-N-...`), and again when the pass ends.
Your `git status`, your index and your branch are unchanged — so this is not a substitute for your own
commit step, which is still the only thing that puts work on `main`.
Recover a killed pass's tree with `git -C <repo> log --all --oneline --grep='WIP(pass'` then
`git show <ref>`.

1. **Commit as you go, not once at the end.** Your commit step is last, behind implement, gate and
   benchmark, so a pass killed at the ceiling never reaches it. Commit the research note alone before
   writing code, as your sections already say; then commit the implementation, then the docs and the
   log. Each commit that lands is work a killed pass cannot lose.
2. **A snapshot ref is not a commit on your branch.** `git log` will not show it. Do not look for
   one; if an earlier pass was killed, its tree is in `refs/wiggum/` and nobody's gate ran on it.
3. **Parked work is a branch, and you look for it before you pick an item.** When a pass is stopped or
   killed with its tree uncommitted, the driver's next start commits that tree to a branch
   (`wiggum/parked-pass-N-<time>`, or on the pass's own `loop/iter-N`) in a commit titled
   `WIP(pass N, parked)` and puts the clone back on `main`: you start clean and nobody tells you. So run
   `git branch --list 'wiggum/parked-*'` and `git log --all --oneline -n 5 --grep='parked'` once, after
   you have triaged the filed issues and before you commit to an item. If the newest parked tree is for
   the item you would pick, or for an issue or bug that outranks your pick, **adopt it**: it is a first
   draft that was never built, tested or gated and was cut off mid-work, so read it, bring its files into
   your own `loop/iter-N` branch (`git checkout <parked-branch> -- <paths>` or `git cherry-pick`),
   finish it, and run the real gate on the result before it ships under your name. Never merge the
   parked branch itself and never delete it. If it is stale (its files conflict with `main`, or the item
   was fixed meanwhile), say so in one line in the log and ignore it. A parked branch that exists only on
   this clone holds a lot of data: the driver keeps anything over 10 MB of new data off origin.

## 1. Orient

1. `AGENTS.md`: rules, Bevy 0.20 traps, gates.
2. `docs/loop-log.md` (create it if missing): what earlier iterations did, found and deferred, and the
   newest iteration's ranked backlog. The next iteration number is the last one plus one.
3. `git log` since the last logged iteration.
4. The `docs/research/<system>.md` note of any system you may touch, and `docs/research/corpus.md` for
   the reading list per domain.
5. `git status --short`. The driver (`wiggum.sh`) starts a pass only on `main` with no
   tracked changes. If you find tracked changes you did not make, stop and log them; do not commit or
   revert them.
6. **Re-scan the corpus for new game-UI sources.** Run `catalog_recent` (limit 200) and add any
   game UI and UX design, HUD design, menu flow, gamepad and keyboard navigation, accessibility or
   typography-for-games document that `docs/research/corpus.md` does not list, as its own commit
   (`Research: corpus`).

Preflight home-still now, so research is not blocked later:

- `system_status` (MCP) or `hs status`. If distill is down, run `systemd-run --user --scope --collect hs
  restart` once, never a bare `hs restart`: that starts Qdrant's podman monitors inside this loop's own
  systemd unit, so restarting the loop restarts Qdrant (it did, on 2026-10-28). If it stays down, skip
  `distill_search` and `distill_index` and list each unindexed note under `Pending index`. Record the
  live state in the log; do not copy an earlier iteration's outage note.
- Scribe: check `scribe_health`. `scribe_convert` may queue or fail; then rely on the corpus.
- Corpus: home-still's index; `docs/research/corpus.md` lists the stems used.

## 2. Capture the current state

Build first (`cargo build --release --examples`). Keep everything under `/tmp/gN/` (`N` the iteration),
the baseline under `/tmp/gN/before/`. Keep a worktree of the starting commit for later comparisons,
**detached, at a commit, and off `/tmp`**: `git worktree add --detach ~/gN_base <sha>`, built with
`CARGO_TARGET_DIR=~/gN_base-target`.

Section 9 says how to name, build and remove that baseline. A baseline left behind costs the next pass
a build cache's worth of disk, and one with `main` checked out stops anything else checking `main` out.

On **sib** (the loop's host: 16 cores, RTX 3060), capture on the loop's own display (the unit starts an Xvfb on `:93` and sets `DISPLAY=:93`). The unit sets `CARGO_TARGET_DIR`, so the binary is under it, not under `./target`:

```sh
G="$CARGO_TARGET_DIR/release/examples/gallery"
GOOEY_FRAMES=120 GOOEY_SHOT=/tmp/gN/before/dark.png "$G"
GOOEY_FRAMES=120 GOOEY_SKIN=parchment GOOEY_SHOT=/tmp/gN/before/parchment.png "$G"
GOOEY_FRAMES=120 GOOEY_TEXT_SCALE=1.5 GOOEY_SHOT=/tmp/gN/before/dark-large.png "$G"
```

- Capture each at two window sizes, 1280x720 and 2560x1440, which the example sets from
  `GOOEY_SIZE=<w>x<h>` (add that knob in the pass that first needs it).
- Record each run's `gooey: frames … mean_ms …` line: it is the performance baseline for section 7.
- Anything a state change shows (focus moving, a press, a dialog opening, a tooltip) needs the state
  staged before the shot: add a capture knob to the gallery for it rather than describing it unseen.
- A run that leaves no image is retried once; then check `df -h /tmp` and that the display is up
  (`xdpyinfo -display :93`), and log what you found.

## 3. Inspect closely (you judge the images yourself)

You do all the work yourself, including looking at the images and judging them. Do not delegate
visual review. Read each image with `read` (it decodes PNGs inline) and zoom in on the widgets.

- Crop and upscale (nearest-neighbour, 2x-4x): `magick in.png -crop WxH+X+Y +repage -filter point -resize 400% out.png`.
- Use OpenCV or Pillow where useful (not installed system-wide; run scripts with
  `uv run --no-project --with opencv-python-headless --with pillow python script.py`): per-frame mean
  luminance plots, difference images between consecutive frames, histograms.
- Quote pixel values when a number settles it: `magick file.png -crop 40x40+X+Y +repage -format "%[fx:mean*255]" info:`.

Always view at least one actual-resolution full frame: a crop can pass while the whole looks wrong.

Write the **game-UI checklist** to `/tmp/gN/findings.md`, each line with a measurement or a file and
region, not an impression:

- **text contrast** against its background, from pixel values: WCAG ≥ 4.5:1 for body text, ≥ 3:1 for
  large text;
- **the focus ring** visible on the focused element at both window sizes;
- **hit targets** ≥ 44 px high at 1280x720;
- **one spacing scale** (`gap`, `padding`) used everywhere, misalignments measured in px;
- **text inside its box** at the longest label;
- **both skins read as one family**, and `GooeySkin::problems()` empty for both (run it in a scratch
  test);
- **the dialog's scrim** separating it from the screen behind;
- **meter fill** proportional to its value.

## Owner input: recommend, record, proceed

**This loop runs fully automated (owner, 2026-10-29): never stop to wait for the owner.** When a pass needs the owner's input (a design choice the issue or item does not settle, such as a colour, a coverage, an API shape, a schema change or which of two behaviours; or a protected file that must change), it answers the question itself:

1. **Recommend.** Name the two or three real options and pick one, with one line of why. Prefer the conservative choice: reversible, conventional, consistent with the existing code and the issue's stated intent, and the smallest that meets the acceptance.
2. **Assume the owner chose it, and build it this pass.** A decision is never a reason to skip an item or end a pass.
3. **Record it where the owner reviews.** On the issue, a comment beginning `Assumed decision (owner may override):` with the options, the pick and why, and the label `assumed-decision` (`gh label create assumed-decision --color FBCA04 2>/dev/null; gh issue edit <N> --add-label assumed-decision`). With no issue, in the commit body and the log line, beginning `Assumed decision:`.
4. **The owner's reply wins.** An owner comment that contradicts an assumed decision overrides it: the next pass that reads it reworks the item to the owner's choice before anything else.
5. **Earlier blocks are lifted.** An issue or backlog item logged as blocked on, or waiting for, an owner decision is workable now: decide it this way.
6. **Protected checks change only additively.** An assumed decision may add to a protected test or check (a new row or entry, a new field's empty default in a literal, a count or version raised by exactly what the change adds), with the commit message `assumed owner exception: <what>`. It never removes, loosens, skips or re-pins an existing assertion, golden, eval answer or threshold; that is the one block left, so say so on the issue and go on.
7. **Never, whatever you assume.** These need the owner's word in a conversation, and no prompt can give it: yanking a release or changing crate ownership; force-pushing, rewriting history or deleting branches; deleting files or work this pass did not create; editing this command file; spending money or sharing credentials.

## Filed issues come before new features

**A filed issue is a consumer telling you what broke, which outranks anything this pass invented.**
Before you pick an item, list the repository's open issues and triage them:

```sh
gh issue list --repo bevy-coven/bevy_gooey --state open --limit 100 \
  --json number,title,labels,createdAt --jq '.[] | "#\(.number) \(.title)"'
```

(If `gh` is absent, the same list is
`https://api.github.com/repos/Ladvien/bevy_gooey/issues?state=open&per_page=100`.)

Then, in this order:

1. **A bug issue is the pass's item, ahead of any enhancement.** Anything reporting wrong behaviour —
   a crash, a panic, a visual artifact, a value that is implausible, a documented promise that is not
   kept — is a bug. Take the oldest such issue that is still reproducible, and say why it beats the
   others.
2. **An issue labelled `consumer` is the pass's item next, ahead of the series and your own candidates.**
   A project built on this crate needs it and is waiting. The `consumer:<name>` label says which project;
   `consumer:bevy_grimoire` is the owner's editor. Its own work resumes when the issue is closed as completed
   and it moves its git pin to a `main` commit containing the fix. Take the oldest one you can work this pass.
   - **Build the general form.** The requester is the example, never a name in this crate's API, a path or a
     default: this crate serves other projects too. Where an issue asks for something shaped to one project,
     build the general form and say so on the issue.
   - **Prove it here**, with a test in this repository that fails before and passes after against the issue's
     acceptance. Never build or run the requesting project to prove it.
   - **Pushing to `main` is the delivery.** Requesters pin a git revision, so a consumer fix never needs a
     publish or a version bump.
   - **One that needs a decision the issue does not settle is decided, not skipped:** recommend, record and
     build it (see "Owner input: recommend, record, proceed"). Only a change that would weaken a protected
     check cannot be worked: say exactly what is needed on the issue (`gh issue comment`), write one log
     line, and go on. A blocked consumer issue never stops the pass.
   - **Read `git log` first.** While this loop is stopped, the requester's loop may fix the issue here at the
     source, in its own commits with tests; check whether that has happened so the work is not done twice.
3. **Every other open enhancement issue is the pass's item next, ahead of your own candidates** (the
   owner's rule, 2026-10-29): a filed request was asked for; a candidate this pass invents was not. The
   order of work is: **bugs first** (a bug issue, then a bug already recorded in the backlog or the log),
   then `consumer` issues, then every other open enhancement issue, oldest first. Only when each of them
   is done, blocked or a release issue does "Pick the single most valuable item" choose a feature of your
   own. A blocked issue gets a `gh issue comment` saying exactly what it needs and is skipped; it never
   stops the pass. Name the issues you skipped, and why, in the log line, so the next pass sees the list
   was worked through.
4. **Close the loop.** An issue your pass fixes is referenced in the commit body (`Fixes #N`) **and**
   in the report, so the next pass can see it was addressed. Do not close an issue you did not fix,
   and do not close one whose fix this pass did not verify. For a `consumer` issue the close is the signal
   the requester waits for: close it only through a verified fix pushed to `main`, never as not planned
   without the owner's word.
5. **A release issue is done by the next release.** An issue asking to publish or bump a version is closed by the pass whose release (the "Release" block in the ship section) publishes it, with the crates.io URL.

**When the API fallback answers `403` or `429`** (rate limit — 60 requests/hour per IP unauthenticated,
shared by everything on this network), do not treat it as "no issues": write one line saying the list
could not be read and continue. Never spend a second call to retry it in the same pass, and never infer
from silence that nothing is filed.

**An empty issue list costs nothing:** if it is empty or `gh` is unreachable, write one line saying so
and continue — never spend a pass on the lookup.

## 4. Pick the single most valuable item

**You are here only when no bug and no open issue can be worked this pass** (see "Filed issues come
before new features"); otherwise that issue is the item. Rank candidates from your findings, the loop log's backlog and the extra instructions, always asking
what matters most to a game developer who will ship this:

1. A widget that is wrong or unusable: focus lost, a click that does not activate, text clipped, a
   dialog that leaks input.
2. Setup that takes more lines than it should.
3. A missing widget every game needs: slider, checkbox and toggle, tabs, scroll list, text input,
   inventory grid, toast and notification, radial menu, key-rebinding row. Most wrap a
   `bevy_ui_widgets` widget.
4. Looks flat next to shipping games.
5. Frame-time cost.

Ownership is a gate: if the crate doc's "What this crate does not own" gives the item to a sibling
crate (balloons and dialogue to `bevy_speech_bubbles`, sound, states and localization to the game), do
not build it here; add a hook instead.

State in one sentence why your pick beats the runner-up. If it is too big, carve off the most valuable
self-contained slice. Record the ranked candidate list in the log, not only the winner.

**A pass never ends idle** (owner, 2026-10-29: "If they're not addressing bugs or enhancing features, they need to evolve their own code base to support Bevy Emerge better or add new features. Research these things in home still, implement them. They shouldn't stay idle."). An empty issue list or a stale backlog is this step's input, never a reason to ship nothing, and a candidate is not "speculation" once it is researched. When the ranked list above gives no workable item, take the first of these that applies:
1. **What the owner's projects hand-roll.** Read Sanctuary's `src/ui/` (`gh repo clone Ladvien/sanctuary /tmp/gN/sanctuary -- --depth 1`, removed afterwards) and bevy_grimoire's `crates/bevy_grimoire_mapper/src/chrome.rs` (a shallow clone under `/tmp`), find a widget or pattern either builds by hand, and build its general form here, never named for either.
2. **A feature a shipping game in this domain expects and this crate lacks**, found by research: home-still first (`distill_search`; for a paper the corpus lacks, `paper_search`, `paper_download`, `scribe_convert`, then `distill_index`), then shipped-game practice, then section 5 before any code.
3. **Depth on the weakest existing feature**: measure it against its research note's validation targets and improve the worst gap.

Record the ranked candidates and the pick in the log. A pass that ships nothing names the concrete blocker that stopped all three; "nothing new to do" is not a blocker. **"Not researched yet" is not a blocker either: it is this pass's work.** Take the top item, do section 5 for it now (home-still preflight included), commit the note, and build its first slice. "Building it would be speculation" means the research is missing, so do the research. A log-only iteration is a failed pass, not an outcome, and a pass never copies an earlier iteration's reasons for shipping nothing (measured 2026-10-29: iterations 194 and 195 each shipped a log line saying every item was "not researched", and neither ran the research).

## 5. Research thoroughly (before any code)

Every feature gets researched, new or improved, with no exceptions. A game interface is design with
ergonomics attached, so every iteration cites **three** kinds of source:

1. a **craft** source: game UI, UX or accessibility design. `docs/research/corpus.md` lists the stems
   already in home-still: *Do Players Prefer Integrated User Interfaces?*, *Removing the HUD*, *Future
   design of accessibility in games*, *Good Game Feel*, *Supporting Novice to Expert Transitions in User
   Interfaces*. Its Pending index names *The Gamer's Brain* and the Game Accessibility Guidelines.
2. a **shipping-game** source: a GDC talk or studio write-up on a shipped game's UI;
3. a **technical** source: UI layout, input focus and navigation, text rendering, or API design
   (*API design for machine learning software: experiences from the scikit-learn project* is the
   corpus's API-design stem).

When the home-still MCP bridge answers `401 Refresh token expired`, search with the CLI,
`hs distill search "<query>" --output json`, which refreshes its own token.

If the corpus lacks one of the three, `paper_search` for it, and record the gap under the note's
`## Pending index`. Write the note with the headings `AGENTS.md` requires (`## Sources`,
`## Craft principle`, `## How shipping games do it`, `## Model adopted`, `## Parameters`,
`## Validation targets`, `## Deviations`, `## Pending index`) plus `## What great looks like`: the
visual qualities and reference images you are aiming for. `## Craft principle` and
`## How shipping games do it` must each **quote** the source they rest on, not paraphrase it from
memory.

Any changed default goes under `Deviations` with a citation, or is marked a design default. Commit the
note alone before writing code: `git add docs/research/<system>.md && git commit -m "Research: <system>"`.

## 6. Implement within the crate's rules

- Keep every rule in `AGENTS.md`. In particular: no game vocabulary in `src/` (`tests/leaf.rs`
  enforces it); no `unwrap()`/`expect()` outside tests.
- A new widget is a free function returning `impl Bundle` plus a `Role` row in `apply_skin`. No styling
  outside `apply_skin`.
- A new setting is a field on `GooeySkin` (serde data, with a `Default`) or its own `*Settings`
  resource, never a constructor argument. Anything a game must check gets a `SkinProblem` variant.
- A new system goes in a `GooeySystems` set; a new plugin is independent and joins `GooeyPlugins`.
- The API matches the family: flat re-exports from the crate root, no prelude, `#![warn(missing_docs)]`.
- Wrap Bevy's `bevy_ui_widgets` widget when one exists; never re-implement it.
- Settings are `Resource + Default + Clone + Debug + Reflect`, added with `init_resource`; messages use
  `add_message`.
- `src/model.rs` stays engine-free: if the new maths can be a plain function, it goes there, with its own
  tests.
- One path per feature, no fallbacks or shims; delete the code the change replaces.
- Tests: for a bug, a test that fails before and passes after. Permanent tests check what a game sees
  (components, colours, focus, activation); throwaway checks go in `/tmp`.

## 7. Verify and sign off: it must look GREAT

You are grading your own work, so grade it as a hostile art director would — and as a
game UI designer, who will notice the focus you cannot see before they notice anything you are
proud of.

- Rebuild the examples, re-capture the same skins, text scales and window sizes as section 2
  under `/tmp/gN/after/`, and rebuild the same zoomed evidence set.
- View before and after side by side, every capture, at full frame and at zoom.
- Fill in the game-UI checklist for the "after" captures and compare each line with the "before".
- Write the critique BEFORE the verdict (`/tmp/gN/critique.md`): list every flaw you can find at zoom
  and at full frame, against the research note's `## What great looks like` and `## Validation
  targets`. Look for what is wrong, not for confirmation that your change worked.
- Then give each feature exactly one verdict: GREAT, NOT YET or FAIL. GREAT means you would be proud to
  ship it in a showcase. "Fine", "acceptable" and "good" are NOT YET.
- On NOT YET or FAIL, write the specific changes that would move it to GREAT, apply them, rebuild,
  re-capture and re-judge. Do not lower the bar to pass.
- After 5 failed rounds on one feature, stop: log the blocker, the remaining flaws and your best
  hypothesis, and revert to the last GREAT state.
- **Performance gate**, equal in weight: `mean_ms` from the gallery capture before and after, same
  machine and load. No regression over 0.5 ms without a logged trade.
- Run the full gate (the `AGENTS.md` set):
  1. `cargo build && cargo build --release && cargo test && cargo test --features serde && cargo test --features readme-example --test readme`
  2. `cargo build --examples`
  3. `cargo clippy --all-targets --all-features -- -D warnings`
  4. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
  5. `cargo fmt --check`
  6. `cargo build --lib --target wasm32-unknown-unknown`

## 8. Ship

Start only after section 7 passes. Then release (the "Release" block at the end of this section).

1. Document: rustdoc for every changed public item, the README for user-visible changes, the research
   note's `Deviations` and `Pending index`, and an entry under `## [Unreleased]` in `CHANGELOG.md`
   (`Added`, `Changed`, `Fixed`, `Removed`).
2. Regenerate `docs/media/gallery.png` from the gallery capture when the look it shows has moved.
3. Log: append `## Iteration N — <date> — <title>` to `docs/loop-log.md`:
   - what changed and why it ranked first (and the runner-up)
   - the sources used: the craft, shipping-game and technical stems, by stem
   - your critique and verdict per capture, with evidence filenames
   - the game-UI checklist's numbers, before and after
   - `mean_ms`, before and after
   - the commit hash, and any sibling-repo commits
   - the top 5 remaining items, ranked, for the next iteration
4. Branch and commit: `git checkout -b loop/iter-N` from `main`. Commit by path with messages that
   state the user-visible change: the research note (already committed in section 5), the
   implementation, the docs and changelog, then the log (`Loop log: iteration N, <title>`).
   `git status --short` must be empty afterwards.
5. Push and merge: `git push -u origin loop/iter-N`, then `git checkout main && git pull --ff-only &&
   git merge --no-ff loop/iter-N -m "Merge loop/iter-N: <title>"`. On a conflict, `git merge --abort`,
   log it, and stop.
6. Re-run the gate on the merged tree; a failure stops the pass before the push. Then
   `git push origin main`.

**Release, every pass that changed the package** (owner, 2026-10-29: loops publish, and never wait on CI to do it). After the push, from that pushed commit, on a clean tree with the gate green: if a file `cargo package --list` ships changed since the last published version (`git diff --quiet v<last> HEAD -- <those files>` fails; with no tag `v<last>`, diff from the commit that set that version in `Cargo.toml`, found with `git log -S'version = "<last>"' -- Cargo.toml`), release:
1. Decide the version with `cargo semver-checks check-release` (`cargo install cargo-semver-checks --locked` if missing). Pre-1.0: a breaking change bumps MINOR and resets PATCH, anything else bumps PATCH; never 1.0.0. Move `## [Unreleased]` in `CHANGELOG.md` to `## [<version>] - <date>`, bump `Cargo.toml` and `Cargo.lock`, and commit `release: v<version>`.
2. `cargo publish --dry-run`, then `cargo publish`, locally, with the credentials `cargo login` stored; never print them. Never wait for, watch or re-run a GitHub workflow to release.
3. `git tag -a v<version> -m v<version>`, then push the commit and the tag (merge, never force, on a refused push).
4. `cargo info <crate>@<version>` resolves; put the crates.io URL in the log entry.

**At most one release every 6 hours** (owner, 2026-10-30: bevy_lights published v0.2.1 through v0.2.20 in one day, and then crates.io answered 429 for three passes in a row, each of which did nothing but retry). Release only when the last tag is more than 6 hours old (`git log -1 --format=%ct <last tag>`); until then the changes ride in the next release. A publish refused with 429 is not retried in this pass or the next: log it once and build, and the next release carries it. A pass that only retries a release is an idle pass. Any other failed publish is logged with its error and retried at the next release. Never yank, never reuse a version; fix a bad release forward.

**bevy_gooey has never been published.** The first pass with a green gate publishes `0.1.0` as it stands; a first publish of this crate name is allowed (owner, 2026-10-30).

### README, demos and docs stay true

A README is read before anything else, so a false line there costs more than a false line anywhere else.
An audit on 2026-10-27 found one in each of the four loop repos: a `tour.gif` captioned "at noon" over a HUD
reading 10.00 h with a running clock; a `ties.gif` captioned for rivalry colours that appear in none of its 240
frames; a hero caption promising an effect its file never shows; and a demo video captioned as the full
recording of the GIF above it when it shows different objects. Before you push a change a user can see:

1. **Put it in the section it belongs to.** Do not append to the introduction or to the end, and do not
   restructure the README in a feature pass. It reads in this order — what it is in one line, the demo, what
   you get, install, quick start, then the detail sections — and an item added in the wrong place undoes it.
2. **A caption describes the file as it exists.** Open the GIF or still (frame count, size, any HUD burned into
   it) and write only what you can read off it; a figure quoted from program output comes from running that
   program now, not from the log. Recipe: the gallery's capture mode (section 2).
3. **Media you cannot regenerate here is logged, not left lying.** With no GPU or display on this machine, name
   the stale file and its path in the report and change its caption to what it still shows.
4. **The quick-start block is the crate-root doctest in `src/lib.rs`;** `cargo test --features readme-example --test readme` fails when the README's copy differs.

## 9. Clean up when you are done

Worktrees and build directories outside the clone are not free: a `target/` for this crate is tens of GB, a
loop host's `/tmp` may be RAM, and on 2026-10-27 earlier passes had left 37 GB of baseline builds and 5 GB of scratch behind.
**After you ship and before you write the pass's last log line, whether or not you shipped,** remove what *you*
made this pass, and list what earlier passes left.

1. **A baseline is a pair that can be found.** Make it with `git worktree add --detach <path> <sha>` (never a
   branch name: a worktree with a branch checked out stops anything else checking that branch out) and build it
   into `<path>-target`. Then `git worktree list`, plus the `*-target` directory beside each worktree, is the
   whole inventory.
2. **Remove yours.** `git worktree remove <path>` (plain, never `--force`) and `rm -rf <path>-target`, then your
   scratch under `/tmp` (captures, frames, venvs, gate logs, `rustdoctest*`). Remove only what you created this
   pass, and name each path in the log. If git refuses because the worktree has a change in it, name the file
   in the log and leave it.
3. **Leftovers from earlier passes are listed, never deleted.** A killed pass cannot clean up after itself, but
   an agent does not delete what it did not create, so list them once and let a person approve the removal:

   ```sh
   git worktree list --porcelain | awk '/^worktree /{sub(/^worktree /,""); print}' | tail -n +2   # all but the clone
   du -sh <wt> <wt>-target                       # the size of each one that is not yours
   git -C <wt> status --short                    # empty: nothing is lost by removing it
   find <wt> <wt>-target -mmin -120 -print -quit # empty: untouched for two hours, so no live build is using it
   ```

   Log each with its path, size, and whether it is clean, already on `origin/main` and idle (the likeliest to be
   stale). Say so, and still leave it.
4. **Never remove** the clone itself, its own `target/` (the build cache the next pass starts from), anything you
   did not create this pass, or anything a process is using.
5. Write one log line: what you removed and the space it freed (`df -h .` before and after), and the leftovers
   you listed.

## Rules

- One improvement per iteration, fully finished: researched, implemented, GREAT-signed-off,
  benchmarked, gated, committed, documented, logged.
- No feature ships without a written critique, a GREAT verdict and a clean performance check.
- If blocked, log the blocker and pick the next item. A failed gate or a visible defect is not a
  blocker: fix it. A failed push or merge is: log the exact error and stop. Never force-push.
