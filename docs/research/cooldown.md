# A cooldown on any widget: `Cooldown`

Researched 2026-11-03 (iteration 60). home-still was unavailable (`gpu busy: yielded to bevy_grimoire@big` from the CLI, `home-still` MCP not connected, the bridge's token expired), so the corpus could not be searched; the sources below are primary documents read directly this pass and each is listed under `## Pending index` to be indexed.

## Sources

1. Accessible Game Design, *Status Effects: Cooldowns* — https://accessiblegamedesign.com/guidelines/statuseffects.html (craft: a timer is a bar or a clock; skills are available, in use or on cooldown).
2. Microsoft, *Xbox Accessibility Guideline 103: Visual and audio cues, multiple sensory methods* — https://learn.microsoft.com/en-us/xbox/accessibility/xbox-accessibility-guidelines/103 (shipping games: Sea of Thieves and Forza Horizon, a control that is not available is not only grayed out; Grounded, colour is never the only signifier). XAG 106 (Gears 5 narrates the seconds left in a countdown) for the assistive-technology side.
3. Bevy 0.20 `bevy_ui::ConicGradient` / `AngularColorStop` (`gradients.rs`, vendored), the CSS `conic-gradient()` model it follows (technical: a pie sweep with hard stops, drawn by the UI shader, clipped by the node's rounded corners, no mesh and no material).

## Craft principle

Accessible Game Design, *Cooldowns / Visual Design*: "**Any time you have a timer in a game, think about representing it visually, such as a bar or clock. This way, players get an idea of how quickly a skill will cool down without having to count or read anything.**" And, *Audio Design*: "**Skills generally have three statuses: available, in use, or on cooldown.**" The player must be able to tell which of the three a slot is from the corner of the eye, and when the third ends.

XAG 103: "**If a change of color (graying) is used to inform the player of the existence and state of a control that isn't available, another method is also used.**" A cooldown that only dims the icon fails this; the sweep is the shape and the number is the text, three cues by value, never by hue.

## How shipping games do it

XAG 103 on Sea of Thieves: "items for purchase that are currently locked aren't only "grayed out," but they also have a lock symbol on them", and on Forza Horizon: "unavailable menu elements are grayed out and have a text label that reads "Not available."" Both pair a dimmed control with a second signifier, which is what a cooldown does: the dimmed wedge, the clock-hand edge and the seconds.

Accessible Game Design on Team Fortress 2: "Each weapon in *Team Fortress 2* has unique sound effects and animations for reloading, which is a type of cooldown that lets the player know the weapon is disabled." A cooldown ending is an event the game wants to play a sound on, so the crate sends one.

XAG 106, on Gears 5: "the number of seconds left in the countdown timer is narrated each second." The crate does not narrate (it owns no speech); it puts the remaining seconds in the widget's accessible name so a screen reader that reads the focused control says them.

The conventional look (the radial sweep of every action RPG's hotbar): the whole icon is dimmed when the ability is used, a clear wedge opens clockwise from twelve o'clock as time passes, the seconds remaining stand in the middle, the dim is gone at zero.

## Model adopted

A **component on any node**, like `Tooltip`: `Cooldown { duration, remaining }`. Its `on_add` hook queues a sync that spawns one overlay child, `Role::Cooldown`, holding one `Role::CooldownText` chip; `on_remove` despawns it (exactly the `Tooltip` box pattern, idempotent).

- **K1. The game starts it, the crate runs it.** `Cooldown::default()` is ready (`remaining` 0, nothing drawn). `cooldown.start(seconds)` sets `duration` and `remaining`. `update_cooldowns` (Style) subtracts `Time::delta` (game time: a paused game pauses its cooldowns) and sends one `CooldownReady { entity }` (`EntityEvent`) in the frame `remaining` reaches 0. The game that wants its own clock writes `remaining` itself.
- **K2. The look.** The overlay covers the owner's padding box with the owner's corner radius. Its `BackgroundGradient` is a hard-stop `ConicGradient`: clear from 12 o'clock to the elapsed angle, `solid_surface` at `COOLDOWN_DIM_ALPHA` (0.78) from there round to 12 o'clock. It is hidden (`Visibility::Hidden`) at 0, so a ready widget draws nothing extra and costs no layout (an absolute child).
- **K3. The seconds.** A chip centred on the overlay, `solid_surface` at 0.92 alpha, `skin.text`, `type_scale[1]`. `model::cooldown_text`: at least 60 s is `m:ss`, at least 3 s whole seconds rounded up, below 3 s tenths rounded up ("2.4", ending "0.1"), so the last second is not a long "1". Text is written only when the string differs. The chip, not the icon, is what the text's 4.5:1 is measured on (a sweep boundary runs through the middle, so text straight on it would sit half on a clear wedge).
- **K4. Not by hue.** The dimmed wedge is the skin's surface colour, so in `high_contrast()` it is black or white and the edge between dim and clear is 7:1 against the icon's worst case by `problems()`.
- **K5. Accessible name.** A `Button` or slot with an active `Cooldown` is named "<name>, <n> seconds remaining" (`model::cooldown_words`), settled in the same `says` check as every other name. Overlay and chip are `Spec::Hidden`.
- **K6. Motion.** The sweep is the value drawn, not an animation: `MotionSettings::duration_scale` does not apply (WCAG 2.3.3 is about motion that is not essential; a timer is information). No flash: a cooldown ending is `CooldownReady` for the game's sound, and the overlay is gone.
- Input is not touched: a pressed button on a cooldown still sends `Activate`; the game decides (`Cooldown::is_ready`). The overlay and chip are `Pickable::IGNORE`.

## Parameters

`COOLDOWN_DIM_ALPHA` 0.78 (the unrecovered part's cover), `COOLDOWN_CHIP_ALPHA` 0.92, `COOLDOWN_WHOLE_SECONDS_FROM` 3 s, the chip's `type_scale[1]`. None in the serialised skin: they are the crate's look, like `SLOT_ICON_SHARE`.

## Validation targets

- `(icon_button(..), Cooldown::default())` spawns one overlay, hidden; `start(8.0)` shows it with "8" and a fully dimmed disc.
- After 2 s of game time the chip reads "6", the gradient's clear wedge is a quarter turn (`2/8 × TAU` clockwise from the top), after 8 s the overlay is hidden and one `CooldownReady` was sent.
- A paused game (`Time<Virtual>` paused) does not tick; `start` while running restarts; removing `Cooldown` removes the overlay in the same flush.
- The chip's words hold 4.5:1 in every preset against the chip over an icon at either extreme (`problems()` reports it, 7:1 in `high_contrast()`).
- The accessible name of a button reads "Fireball, 6 seconds remaining" and is back to "Fireball" when ready.
- An idle ready widget writes nothing for thirty frames; a ticking one writes the overlay's gradient, visibility and text only when they differ.
- The overlay changes no layout: a slot's size and its neighbours' positions are the same with and without a running cooldown.

## What great looks like

A hotbar of four 44 px keycapped slots: the one just used is a dim disc with a clean wedge of icon opening from twelve o'clock, the "5" on a quiet chip in the middle, readable at couch distance in dark, parchment and high-contrast. When it ends the dim is simply gone and the game's own sound plays. No colour tint to lose on a colour-blind display, no flicker, no layout shift.

## Deviations

- The sweep opens clockwise and starts at twelve o'clock (the convention of the genre; Godot's cooldown recipe offers the other direction as an option): a design default, no setting.
- Time is the game's (`Time`), where `Meter` uses real time: a cooldown is a game rule and a paused game must hold it. A design default.
- The accessible name carries the seconds; the crate sends no live-region announcement every second (XAG 106 asks the game to narrate; that is speech output, which the crate does not own).

## Pending index

- distill was unavailable. Index: Accessible Game Design's status-effects page, XAG 103 and XAG 106, and search the corpus for "cooldown indicator", "action bar", "timer representation in games" once distill answers.
