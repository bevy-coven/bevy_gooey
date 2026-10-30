# bevy_gooey — notes for agents

The game-UI kit: a skin as data, ready-made game widgets (screen, panel, menu, heading, label, button, HUD meter, tooltip, modal dialog), keyboard and gamepad navigation and UI feedback, on Bevy 0.20's headless widgets (`bevy_ui_widgets`) and input focus (`bevy_input_focus`). Its design rules and their sources are in `docs/research/gooey.md`.

## Force pushes and deletions need the user's express permission

Never do any of the following unless the user has approved that specific action, by name, in the current conversation. A loop brief, a skill, a standing instruction such as "commit, push, PR, merge", or an earlier session's approval is not permission.

- Force-push in any form: `git push --force`, `--force-with-lease`, `--force-if-includes`, a `+` refspec, or pushing rewritten history. Deleting a remote branch or tag (`git push --delete`, `gh pr merge --delete-branch`, an API delete).
- Throwing away commits or uncommitted work: `git reset --hard`, `--merge` or `--keep`; `git checkout -- <path>` or `git restore` over changes you did not make; `git clean`; `git stash drop` or `clear`; `git branch -D`; `git worktree remove --force`; `git rebase` or `git commit --amend` on pushed commits; `git filter-repo` or `filter-branch`.
- Deleting files, directories, build caches, target dirs, models, datasets, database rows, issues, PRs, releases or cloud resources that you did not create in this session (`rm`, `rm -rf`, `git rm`, `cargo clean`, an API delete).

When one of these seems necessary, stop and ask. Name the exact command, what it destroys and whose work it touches. A commit that landed on the wrong branch is fixed with a new commit or a question, never by resetting a shared working tree or force-pushing. Files you created yourself in this session (scratch scripts, temp dirs, your own worktrees) you may delete.

## Source of truth

This repository, [`Ladvien/bevy_gooey`](https://github.com/Ladvien/bevy_gooey), is canonical. MIT OR Apache-2.0. A change a game or the editor (bevy_emerge) needs is made here, in its own commit with its tests here, and requested here as an issue labelled `consumer` and `consumer:<project>`; it is never patched into the consumer.

## Layout

- `src/lib.rs` — crate docs (the quick start doctest), re-exports, `GooeySystems`, `GooeyStylePlugin`, `GooeyNavPlugin`, `GooeyPlugins`.
- `src/model.rs` — engine-free maths: `relative_luminance`, `contrast_ratio`, `composite`, `meter_fraction`, the WCAG constants.
- `src/skin.rs` — `GooeySkin` (presets, `hovered`, `problems`), `SkinProblem`, `GooeyFonts`, `Role`, `apply_skin`.
- `src/widgets.rs` — `screen`, `panel`, `menu`, `heading`, `label`, `button`, `meter`, `dialog`.
- `src/meter.rs` — `Meter`, `update_meters`.
- `src/tooltip.rs` — `Tooltip` and its hook.
- `src/nav.rs` — `navigate_by_input`, `gamepad_select`.
- `src/feedback.rs` — `UiFeedback`, `UiFeedbackKind`, the activate observer, `feedback_on_focus`.
- `examples/gallery.rs` — every widget on one screen; capture mode with `GOOEY_FRAMES`, `GOOEY_SHOT`, `GOOEY_SKIN`, `GOOEY_TEXT_SCALE`.
- `tests/leaf.rs` — the boundary ratchet. `tests/skin.rs`, `tests/nav.rs` — what a game sees. `tests/serde.rs` — the skin as data. `tests/readme.rs` — the README quick start is the doctest.
- `docs/research/` — the research note and the corpus. `docs/loop-log.md` — the loop's log and backlog.

## Build and test

The gate, every item green before a push:

1. `cargo build && cargo build --release && cargo test && cargo test --features serde && cargo test --features readme-example --test readme`
2. `cargo build --examples`
3. `cargo clippy --all-targets --all-features -- -D warnings`
4. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
5. `cargo fmt --check`
6. `cargo build --lib --target wasm32-unknown-unknown`

`cargo build` takes no dev-dependencies, so it compiles exactly the trimmed Bevy features a consumer gets; the full `bevy` dev-dependency would hide a hole in that list. CI (`.github/workflows/ci.yml`, on big's self-hosted runner) runs the same, but the local gate is the gate: never wait on CI.

## The non-negotiables

- **No game vocabulary.** `tests/leaf.rs` rejects `avian`, `emerge`, `sanctuary`, `foundation_vs_slop` and `MainCamera` in the sources and the dependency list.
- **Independent plugins**, `GooeyStylePlugin` and `GooeyNavPlugin`, grouped as `GooeyPlugins`. Every system lives in `GooeySystems::{Navigate, Style}`, chained Navigate before Style.
- **Flat re-exports and no prelude**, like the rest of the family: `use bevy_gooey::*;`.
- **Every widget is a free function returning `impl Bundle`**, styled only in `apply_skin` through its `Role`. No colour or size is set anywhere else.
- **`GooeySkin` is plain serde data with `serde(default)`.** Fonts live in `GooeyFonts`, never in the skin.
- **A skin change that fails `problems()` warns and never panics.**
- **Engine-free maths in `model`**, tested without an app.
- **The host supplies the font.**
- **No `unwrap()`, no `expect()` outside tests.**
- **One path per feature.** No fallbacks, no legacy shims, no stub placeholders.
- **Never re-add plugins `DefaultPlugins` adds** (`UiWidgetsPlugins`, `InputFocusPlugin`, `InputDispatchPlugin`): a second add panics.
- **Enter and Space belong to `bevy_ui_widgets`, gamepad South to `gamepad_select`.** A second handler would activate twice.

## Rules

- **Bevy 0.20 is pinned.** Read the vendored source (`~/.cargo/registry/src/index.crates.io-*/bevy-0.20.0/`, its `examples/` and `_release-content/migration-guides/`), not bevy.org — that documents `main` and has been wrong for a pin more than once. Pointer events are flat in 0.20 (`On<PointerClick>`, not `On<Pointer<Click>>`).
- **A missing `Res<T>` panics its system.** Take `Option<Res<T>>`, or have the plugin `init_resource` it.
- **All run conditions are evaluated — there is no short-circuit.** A bare `Res<T>` in a `.run_if(..)` closure panics whenever that resource is absent.
- **`Single<..>` silently skips its system on a non-unique match.**
- In tests, `ButtonInput::clear()` drops `just_pressed` only: also `release` a key before pressing it again.
