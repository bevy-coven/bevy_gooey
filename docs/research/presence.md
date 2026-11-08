# Presence: a widget that leaves stays drawn while it goes

The last slice of bevy_grimoire's feel series that every later request names (tracking
dark-incorporated/bevy_dark_factory#330): **#49 (G-42)** presence, `Opacity` multiplying down a tree, and
`Leave`/`Stay` on a widget. Today a dialog, a menu or a toast vanishes in one frame: `despawn()` is
the only way out the crate offers, and `apply_skin` has no idea what an opacity is.

## Sources

- Craft: Iacovides, Cox, Kennedy, Cairns, Jennett, *Removing the HUD: The impact of non-diegetic game
  elements and expertise on player involvement* (CHI PLAY 2015, doi:10.1145/2793107.2793120), in the
  corpus; and WCAG 2.2 SC 2.3.3 *Animation from Interactions*.
- Shipping practice: Rune (`anomalyco/rune` at `f73f72c`, MIT, © Anomaly),
  `crates/rune-motion/src/scene/presence.rs`, `crates/rune-motion/src/transition.rs`,
  `crates/rune-motion/src/tokens.rs`, `crates/rune-motion/README.md`; and Hicks et al., *Good Game
  Feel* (DiGRA 2018, doi:10.26503/dl.v2018i1.936), in the corpus.
- Technical: Bevy 0.20.1 `bevy_ui`'s `picking_backend.rs` (`Pickable::IGNORE` is `is_hoverable: false`,
  so a node "will not be marked as hovered, and consequently will not emit events"), `UiTransform`,
  and this crate's `apply_skin` (the one place colours are written).

## Craft principle

Iacovides et al. on what an interface element's *leaving* is for: "The HUD may initially provide
important information to players, but can later act as a barrier to deeper levels of involvement when
the player has gained sufficient expertise to continue without it." An element that goes is a change
of the player's attention, not a bookkeeping detail; if it happens between two frames the eye reads it
as a glitch, and if the element is a modal dialog the player has no way to tell whether it was
dismissed or replaced.

WCAG 2.2 SC 2.3.3: "Motion animation triggered by interaction can be disabled, unless the animation is
essential to the functionality or the information being conveyed." An exit is never essential — the
dialog is gone either way — so it must be one switch off.

Hicks et al. give the other half: "it is not enough to 'sprinkle' a game with elements of juicy design;
it is something that needs to be approached from a holistic perspective." An exit is one more motion
in the same palette as the entrances (`docs/research/motion-tracks.md`), not a second easing.

## How shipping games do it

Rune's `Scene::remove` is the model, quoted from its source:

> "The app removed a keyed node. It stays drawn, plays its removal effect on the exit spring, and is
> reported in `exited` only once its presence has settled at 0 (opacity 0)."

and from `crates/rune-motion/README.md`:

> "`scene.remove(NodeId(3)); // stays drawn until its exit settles`"
> "Under reduced motion, travel, scale, shape and blur change instantly, and opacity, colour and
> presence take a 0.18 s ease-in-out fade (`REDUCED_FADE`)."
> "A removed node always fades out (`Effect::kit_rules`)."

The revive rule, from the same file: a node "comes back while exiting" is restored to where it is
drawn rather than snapped — `revive()` keeps "the current look at p" and lands on identity at 1, and
`insert()` of an exiting key calls it. Rune's exits also run shorter than its entrances
(`SETTLE 0.3 s` entering against `SETTLE_EXIT 0.24 s`; `tokens.rs`).

Rune's `Transition` also carries a `pop_layout` flag, from Framer Motion's `AnimatePresence
mode="popLayout"`: "an exiting node leaves layout flow at once (siblings reflow while it fades in
place)". And a transition that changes nothing still holds removal: `Transition::identity()` — "No
visual change (presence still holds removal until its fade settles)".

## Model adopted

- **P1. One opacity, multiplied down the tree.** `Opacity(f32)` is public, `0..=1`, a component on any
  node; every colour `apply_skin` writes on a node with a `Role` at or below one is multiplied by the
  product of the `Opacity` at and above it (fills, borders, text, icons' `ImageNode::color`, the
  caret, and a floating box's `BoxShadow`). A node without a `Role` is the host's and is untouched,
  as the issue says.
- **P2. Only the subtree of a node that carries an `Opacity` is walked.** `spread_opacity` runs on
  `Changed<Opacity>` and on a removal, writes a private `Alpha` on that node and its descendants, and
  writes nothing at all once a fade has settled — so a settled interface costs one `Changed` check
  and no writes.
- **P3. An exit is a fade on the skin's `fade_seconds`.** A `Leave` on any entity gives it `Leaving`:
  the opacity springs from 1 to 0 on `Spring::new(motion.fade_seconds, 0.0)` scaled by
  `MotionSettings::duration_scale`, and the entity despawns when the value is under `1/255`
  (`PRESENCE_GONE`, an eighth of a step of 8-bit sRGB). The token named "how long an opacity change
  takes" is the one an opacity change uses; Rune's reduced-motion fade (0.18 s) and its exit tokens
  (10–25 % shorter than their entrances) agree. Measured: 0.177 s of a 0.15 s spring (bounce 0),
  inside `motion.default` (0.3 s) plus a frame at 30 and 60 Hz.
- **P4. A leaving subtree is inert.** At the frame `Leave` is applied the entity gets
  `Pickable::IGNORE` and so does every descendant, every `TabIndex` in the subtree is taken off, and a
  `Dialog` and its modal `TabGroup` are removed — so `return_focus` gives focus back then, exactly as
  it did at despawn, and a press during the exit reaches the screen under the veil. Everything
  removed is recorded on `Leaving`.
- **P5. `Stay` reverses from where it is drawn.** `Stay` retargets the same opacity spring to 1 with
  the value and velocity it has, restores the `Pickable` and `TabIndex` values it found, and takes
  off both `Stay` and `Leaving` when it settles. Two `Leave`s are one exit; a plain `despawn()` still
  removes at once (the tests that do that keep their meaning).
- **P6. Off is off.** Under `MotionSettings::off()` a `Leave` despawns in its own frame; nothing
  begins.

## Parameters

|part|value|
|---|---|
|`Opacity` range|`0..=1`, clamped on read; not a number reads as 1|
|exit spring|`Spring::new(skin.motion.fade_seconds, 0.0)`, scaled by `MotionSettings::duration_scale`|
|`PRESENCE_GONE`|`1.0 / 255.0` — the fade is over at one 8-bit step|
|despawn|the frame the value passes `PRESENCE_GONE`, or `spring_settled` at it|

## Validation targets

- Escape on the Quit dialog: the card is drawn on every frame of its exit with its opacity falling
  and strictly between 0 and 1, and it is gone within `motion.default` plus one frame at 30 Hz and at
  60 Hz.
- A press at the middle of the card during the exit sends no `Activate` to the dialog's buttons, and
  the node under the veil (the screen's button) is what the pointer finds.
- `Stay` 0.05 s into an exit: no frame's opacity step is larger than the largest step an
  uninterrupted exit makes, and it comes back to exactly 1.
- A toast's own end still fades (its `Toast::alpha`), and 30 idle frames write nothing: no
  `UiTransform`, `BackgroundColor`, `BorderColor`, `TextColor` or `UiTransform` change.
- `Opacity(0.5)` on a panel's root halves the fill, border and text of every `Role` node under it and
  leaves a host's own `Text` alone; the skin's contrast checks are unchanged (`problems()` reads the
  skin, not a drawn node).
- `MotionSettings::off()`: a `Leave` despawns in its own frame and no `Leaving` is ever added.

## Deviations

- **`Leave` and `Stay` are components, not `EntityCommand`s.** The issue proposes
  `commands.entity(e).queue(Leave)`; this crate's state is components (`Opacity`, `MotionTarget`,
  `InstantMotion`), a component can be asserted on by a test, and `commands.entity(e).insert(Leave)`
  needs no deferred indirection. Recorded on the issue as an assumed decision.
- **The toast keeps its own fade.** `Toast::alpha` already falls to 0 before `update_toasts` despawns
  it, so its exit is a fade today; routing it through `Leaving` would give it a second fade of its
  own. The cutover is the dialog (`confirm`), the popover, the palette and the context menu, whose
  `despawn()` becomes `insert(Leave)`.
- **The exit is the `fade_seconds` spring, not `motion.default`.** The acceptance asks for a despawn
  inside `motion.default` plus a frame; a `default`-length spring is still at 1.4 % opacity at 0.3 s
  and settles at 0.6 s, so the token that names an opacity change is the correct one and the numbers
  are in `## Parameters`.

## Pending index

- Nothing in the corpus covers exit animation as its own subject; the two craft stems above are the
  nearest (an interface element's presence, and the duty to switch motion off). *The Gamer's Brain*
  (Nacke) and the Game Accessibility Guidelines are still not indexed; both are named in
  `docs/research/corpus.md`.
- Rune's `rune-perf` harness would give an independent measurement of a fade's frame cost; not run.

## What great looks like

A Quit dialog that is dismissed on Escape keeps its card and its veil drawn and lets them thin out
together over about a sixth of a second, while the screen behind gets its colour and its pointer back
in the same frame; nothing under it moves, nothing is left behind, and no frame after the last one
writes a byte. A second press during the exit brings it back from exactly where it was, with no jump:
the eye reads one object that changed its mind, not two that swapped. At rest, the whole interface is
byte-for-byte as still as it was before any of this existed.
