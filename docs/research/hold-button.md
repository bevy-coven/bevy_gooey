# A button that is held to confirm: `hold_button`

Researched 2026-11-04 (iteration 62). home-still was unavailable (the CLI answered `gateway rejected the refresh token (401): Refresh token expired`), so the corpus could not be searched; the sources below are primary documents read directly this pass and each is listed under `## Pending index`.

## Sources

1. Microsoft, *Xbox Accessibility Guideline 107: Input* — https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/107 (craft: a hold is a duration barrier and needs a press alternative; activation on the up event, with a way to cancel).
2. Game Accessibility Guidelines, *Avoid / provide alternatives to requiring buttons to be held down* — https://gameaccessibilityguidelines.com/avoid-provide-alternatives-to-requiring-buttons-to-be-held-down/ (craft: motor, intermediate).
3. XAG 107's own examples of shipped games: *The Long Dark* ("accessible interactions") and *Gears 5* ("Button Tap Challenges": press and hold or quick taps); Capcom, *Resident Evil 4* manual, *Player Actions* — https://game.capcom.com/manual/re4/en/ps5/page/2/2 (shipping games).
4. Bevy 0.20 `bevy_ui_widgets::Button` and `bevy_picking` pointer events (vendored `button.rs`: `Activate` fires on a key's down and on a pointer's click; `Pressed` is the pointer's) (technical: why the widget cannot be a `Button`).

## Craft principle

XAG 107: "**Avoid introducing mechanics where a key or button should be held down for an extended period before the input is registered.**" and "**If inputs that require repeated button strokes, prolonged button presses, or simultaneous button presses cannot be avoided, provide an alternative input option that is either less physically demanding or allow the player to bypass game events that require the use of using these mechanisms.**" Its list of alternatives is "A single input press instead of an input hold." first. And for the pointer: "**The down click or down press (the down event) on an item doesn't automatically activate that item. Activation occurs on the up event**", with "A mechanism is available to cancel the action before completion. (For example, if a player touches down on the wrong location or item, they can move their cursor or finger away from that location before the up event or click release.)"

GAG: "**Holding requires much greater motor ability than pressing, particularly on buttons that require more strength to operate, such as L3/R3 (pushing in the sticks).**" and "**ideally offer a setting for players to choose which option they would prefer to play with.**"

A confirm-by-hold is the one hold a menu may keep: it guards a destructive act (delete a save, quit without saving) with a cost the player pays on purpose, and the cost is a motor barrier for some players. So the hold is a duration the player can set, down to a tap, and it never completes on a down event that cannot be taken back.

## How shipping games do it

XAG 107 on The Long Dark: "there is an 'accessible interactions' feature, which 'converts all press and hold actions into press actions, for people with CTS [carpal tunnel syndrome] or similar considerations.'" One setting, every hold in the game; the crate's equivalent is one resource every `hold_button` reads.

XAG 107 on Gears 5: "Players can choose to press and hold a button during these scenarios, or quickly tap buttons. Both options provide the same functionality", with the note "Quick taps, as well as prolonged button holds, can be difficult for players with mobility-related disabilities. Additional options that don't rely on either of these mechanisms ... will provide an even more accessible experience."

Resident Evil 4's manual (Player Actions): "Some interactions require the player to hold down a button. Hold until the white gauge is completely filled to perform the action." The shipped convention is a gauge that fills while the button is down, a label that stays readable, and the act at the full gauge.

The same pattern in the web's UI kits (Lessly, DinachiUI, motion.dev): a fill from the start edge over about a second, "If the person lets go before the fill is full, the fill goes back to empty", unwinding faster than it fills, and for a screen reader "cannot hold": a plain activation does it.

## Model adopted

`hold_button(label, seconds)` is a free function returning `impl Bundle`: a `Role::Button` (the button's look, size, focus ring and hover) over a column of the label and a meter-shaped track whose fill is the hold's progress. The track is always drawn, so a hold button reads as one before it is touched.

- **Not a Bevy `Button`.** `bevy_ui_widgets::Button` activates on a key's down edge and on a pointer's click: a hold button on it would send `Activate` for a tap, and a game that observed `Activate` as for every other button would confirm a delete on a tap. So the widget sends no `Activate` at all; its one event is `HoldConfirmed { entity }`. It handles its own input: Enter and Space held while focused, the pad's South held while focused, and the primary pointer button held while over it.
- **State.** `HoldToConfirm { seconds }` (the component the game writes), `HoldProgress(f32)` (0 to 1, the crate writes it, a game may read it for a sound or a rumble ramp).
- **Rise and unwind.** Held, the progress rises by `dt / seconds` on `Time<Real>` (a paused game's menu still holds). Let go or cancelled, it unwinds four times as fast (`model::hold_step`; instant when `MotionSettings` is off), and a new press resumes from where it stands: "committing should feel considered, backing out should feel free". The confirm is sent once, the frame the progress reaches 1 while held; the bar stays full until the player lets go (a hold that is still down after the confirm does not confirm again).
- **Cancel.** A pointer hold ends, without confirming, when the pointer leaves the button (XAG's "move the cursor away"); a key or pad hold ends when focus leaves it, when it is disabled, or when the button is despawned.
- **The press alternative.** `HoldSettings { duration_scale }` (a serde resource `GooeyNavPlugin` inits, 1.0): the seconds a hold takes are `seconds * duration_scale`. At 0 (`HoldSettings::tap()`) the hold is a tap: pressing fills the bar at once and **the confirm is sent on the up event** (the key, the South button or the primary button released while still on the button), so XAG's up-event rule holds for the setting that exists for it. A scale between is "a shorter hold" for a player who can hold, but not that long.
- **Screen readers.** A screen reader cannot hold, and AccessKit's Click is a deliberate act: Click on a hold button sends `HoldConfirmed` at once.
- **Feedback.** `UiFeedbackKind::Activate` is written on the confirm, so the game's sound for a button plays once, at the moment of the act and not at the press.
- **Reading its place.** The bar's fill is the existing `Role::MeterFill` with its width a percentage, written in `GooeySystems::Style` only on a difference; the label is `Role::ButtonText`, so the name a screen reader reads is the label.

## Parameters

- `seconds` is the game's (the demo uses 1.2 s); `HoldSettings::duration_scale` 1.0, clamped to 0.0 to 4.0 when read.
- Unwind rate: 4 × the rise (design default).
- No dead zone: the bar shows a press the frame it starts.

## Validation targets

1. Held for `seconds` of real time: exactly one `HoldConfirmed`; released early: none, and the progress returns to 0.
2. A key, the pad's South and the pointer each hold; releasing one of two held sources does not cancel the hold.
3. The pointer leaving ends it without a confirm; focus leaving ends a key hold.
4. Still down after the confirm: no second confirm until it is let go and held again.
5. `HoldSettings::tap()`: a press and release confirms once, a press cancelled (pointer away, focus gone) confirms nothing; a half scale halves the time.
6. A disabled hold button never starts; a button disabled mid-hold ends it.
7. The AccessKit Click action confirms at once; no `Activate` is ever sent.
8. The label and the track in all four presets read at 4.5:1 and 3:1 (the button's and the meter's own checks), the button never narrower than the track, 44 px high at 1280x720 and unchanged in height at every `TEXT_SIZE_STEPS` step; nothing written while idle.

## What great looks like

A button the same size and weight as its neighbours in the menu, with a quiet track under the label; under the finger the accent fills it left to right at an even rate, the ring (for key and pad) on the button, the label never covered; letting go drains it in a fifth of the time; at full the sound plays and the bar stays full until the player lets go. No flash, no shake. The reference is the Resident Evil 4 gauge, moved from the world prompt to a menu button.

## Deviations

- The resume-not-restart rule and the 4 x unwind are a design default from the UI kits above, not from a game source.
- The fill is a track under the label, not a sweep behind it: a fill behind the label puts half the label on the accent and half on the surface, and no one ink holds 4.5:1 on both (the same reason the cooldown's number sits on a chip).
- `HoldSettings` is its own resource, not `MotionSettings`: reduced motion and a shorter hold are different needs (XAG 107 is about duration; WCAG 2.3.3 about animation).

## Pending index

home-still was down. Not indexed: XAG 107, Game Accessibility Guidelines *Avoid / provide alternatives to requiring buttons to be held down*, the Resident Evil 4 manual page, Understanding SC 2.5.2 *Pointer Cancellation* (https://www.w3.org/WAI/WCAG21/Understanding/pointer-cancellation.html).
