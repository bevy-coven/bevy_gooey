# Motion: panels that arrive, and a switch that turns it off

Researched 2026-11-02 (iteration 47). Nothing in the crate moves on its own except a toast's room and the spinner: a dialog, a menu, a select, a colour picker and a context menu appear whole in one frame. `docs/research/gooey.md` deferred animation from slice 1 ("R4 says moderate juice"), and Sanctuary, the owner's game, hand-rolls `Pop`, `FadeIn` and `SlideIn` tokens in `src/ui/juice.rs` (decision 0130) because the crate has none. Section 4 of the loop brief ranks "looks flat next to shipping games" fourth, after every widget that was unusable or missing was built; this is the first thing on that rung.

## Sources

1. Hicks, Dickinson, Holopainen and Gerling, *Good Game Feel: An Empirically Grounded Framework for Juicy Design* (home-still, in the corpus) — craft.
2. The same paper's analysis of *Downwell* (a shipped game), and the W3C's *Understanding SC 2.3.3 Animation from Interactions* (WCAG 2.2, Level AAA), which is the accessibility rule a game's motion answers to — shipping game and craft.
3. Page Laubheimer, *Executing UX Animations: Duration and Motion Characteristics*, Nielsen Norman Group — technical (duration and easing). Bevy 0.20's `UiTransform` (translation is `Val2`, resolved against the node and the scale at layout) is the mechanism.

## Craft principle

Hicks et al. report that juice is feedback, and that developers want it where the player acts: "When the player presses input to engage the action, the juice makes the action feel impactful and meaningful". Opening a menu is such an action, and today its answer is an abrupt cut. The same paper warns about the other side in the analysis of *Downwell*: "the level of exaggeration in weapon discharge effects in rapid succession can overwhelm the player". A menu is opened far more often than a weapon is fired, so its motion is the short, small kind.

WCAG 2.2 SC 2.3.3 states the duty: "Motion animation triggered by interaction can be disabled, unless the animation is essential to the functionality or the information being conveyed." Its intent: "Some users experience distraction or nausea from animated content" and "Vestibular (inner ear) disorder reactions include dizziness, nausea and headaches." An entrance is never essential (the panel is there either way), so there must be one switch that turns every motion of the crate off.

## How shipping games do it

*Downwell* "has a simplified art style and color scheme which allows the game to easily draw the attention of the player", and "All player actions lead to immediate visual and auditory response": the response is immediate, never a wait before the thing is usable. A panel that arrives by sliding must therefore be usable from its first frame: the crate moves only the picture (`UiTransform`), never the layout, the focus or the hit target's existence, and focus lands in a dialog in the frame it opens exactly as before.

NN/g gives the numbers a shipped interface keeps: "the duration of most animations should be in the range of 100–500 ms, depending on complexity and on how far the element is traveling. As a rule of thumb, look for the shortest time that an animation can take without being jarring. It is far more common for animations to be too long than too short." A checkbox or switch is "roughly 100 ms". "Most frequently, you'll want to use an ease-out animation, that starts quickly but slows down", and "keep in mind how frequently users will encounter the animation: the more frequent the animation, the more subtle and shorter you'll want it to be", and "you should respect those users who have set their browser or device to 'reduce motion' by removing animations."

Sanctuary's `juice.rs` keeps the same three tokens (a pop, a fade, a slide), all on real time "so the wheel's slow motion and a pause do not freeze the UI", each taking itself off when spent, `Pop` touching only the scale and `SlideIn` only the translation "so the two can run on one node".

## Model adopted

- **M1. One setting turns it off: `MotionSettings { duration_scale }`** (resource, serde, `Default` 1.0, `MotionSettings::off()` is 0.0). Every motion's length is its own seconds times the scale; at 0 nothing starts and a running one jumps to its end. It is the same one-number shape as `ToastSettings::duration_scale` and a game wires it to its "Reduce motion" option; WCAG 2.3.3 is the reason, and the crate cannot read an operating-system preference (Bevy has no such API).
- **M2. One token, `SlideIn { from, seconds }`**, a component on any `Node`: the node's `UiTransform::translation` eases out (`model::ease_out`, cubic) from `from` (logical px) to nothing over `seconds` of `Time<Real>`, then the component takes itself off and leaves the translation zero. Real time so a paused game's menu still moves. It writes the translation only: a game that wants a scale pop on the same node can have it. Public so a host's own panels use it (Sanctuary's `SlideIn`).
- **M3. The crate applies it where a panel arrives:** a dialog's card (a `Panel` or `Palette` whose parent is the scrim) rises 12 px over 0.18 s, a popover panel (every menu, context menu, select list and colour picker; not a tooltip) slides 6 px from the side of its anchor it opens on, over 0.12 s. The numbers are in `model` as constants. A game that does not want the entrance on one panel removes the component in the frame it appears or turns the setting off.
- **M4. Only the picture moves.** `UiTransform` is applied after layout, so focus, the hit test's existence, scroll geometry and every `ComputedNode` size are the final ones from the first frame; a slide is a translation only (no scale), so a child's rect relative to its panel never changes and `scroll_menus_to_focus` and the popover's own flip decision are unaffected. A popover is hidden for its first frame while Bevy lays out its text (`reveal_popovers`); the clock does not run while it is hidden, so the player sees all of the slide.
- **M5. No fade for now.** Bevy has no group opacity; a fade means every part of the panel multiplies its own alpha, which the toast already does through `State::alpha` for its own parts. A slide of 6 to 12 px at 120 to 180 ms reads as arrival without one (NN/g: the shortest that is not jarring). A scrim fade and a card fade are the open gap.

## Parameters

|name|value|why|
|---|---|---|
|`DIALOG_SLIDE_PX` / `DIALOG_SLIDE_SECONDS`|12 px (rising) / 0.18 s|a rare, large thing may take the longer end of "shortest that is not jarring" (NN/g 100 to 500 ms)|
|`POPOVER_SLIDE_PX` / `POPOVER_SLIDE_SECONDS`|6 px / 0.12 s|opened often, so subtle and short (NN/g)|
|easing|cubic ease-out, `1 − (1 − t)³`|"starts quickly but slows down"; the toast's room opens on the same curve (`toast_grow`)|
|`MotionSettings::duration_scale`|1.0|0 is the reduced-motion switch|

## Validation targets

- With `MotionSettings::off()` a dialog card, a menu panel and a popover have a zero translation in their first frame and no `SlideIn` component.
- With the default a dialog card's translation starts at `(0, +12)` logical px, is between 0 and 12 at half the length, is exactly zero after the length and the component is gone.
- Real time, not virtual: a paused game's menu still slides (`Time<Virtual>` paused in the test).
- A popover hidden in its first frame starts moving only when it is shown.
- Focus lands in a dialog in the same frame as before (no change to `focus_opened_dialogs`); a click on a menu item in the middle of its slide still activates it (the hit is on the moved box).
- `model::ease_out` is 0 at 0, 1 at 1, monotonic, clamped outside.
- Thirty idle frames after the slide write nothing (no `Changed<UiTransform>` once spent).
- The gallery's frame time does not move (a query that finds nothing).

## Deviations

- NN/g's "about 100 ms for a checkbox or switch": the crate adds no motion to those in this slice (design default; the switch already changes state in the frame it is pressed).
- No fade and no scale: Bevy UI has no group opacity (design default, M5).
- 12 px and 6 px, 0.18 s and 0.12 s are design defaults inside NN/g's range; no source gives a distance.

## What great looks like

A dialog that rises gently into place under a dimming scrim, a menu that drops from its title and settles, neither slow enough to be waited for nor so fast that it reads as a flicker; the same screen with "Reduce motion" on is the screen the crate shipped yesterday, frame for frame. Reference: the entrance of a menu in *Hades* or a Windows 11 flyout, which move a few pixels in a tenth of a second.

## Pending index

- Celia Hodent, *The Gamer's Brain*; the Game Accessibility Guidelines (their "Provide an option to turn off flashing and shaking effects" page could not be fetched this pass; the guideline's page is the citation a game's settings screen would carry).
- David Candland's GDC talk on *Destiny*'s UI (cursor speed and flyout menus), named by a secondary source only: not indexed, not quoted.
