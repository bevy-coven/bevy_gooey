# Progress and spinner

Researched 2026-11-01 (iteration 37, #21 G-17). home-still's index has nothing on response-time feedback (`distill_search` returned statistics papers), so the sources below were fetched today.

## Sources

1. Jakob Nielsen, *Response Times: The 3 Important Limits*, NN/g (from *Usability Engineering*, 1993; Miller 1968, Myers 1985), https://www.nngroup.com/articles/response-times-3-important-limits/ — craft.
2. Game Accessibility Guidelines (no information by colour alone; Xbox 103) — shipping games: a bar is a length, never a colour.
3. Bevy 0.20 `bevy_ui::UiTransform` (`rotation: Rot2`) and `bevy_ui::BorderColor` (one colour per side), and this crate's `meter.md` — technical.

## Craft principle

Nielsen: "**10 seconds** is about the limit for keeping the user's attention focused on the dialogue. For longer delays, users will want to perform other tasks while waiting for the computer to finish, so they should be given feedback indicating when the computer expects to be done." "As a rule of thumb, percent-done progress indicators should be used for operations taking more than about 10 seconds." For 2 to 10 s "a true percent-done indicator may be overkill and, in fact, putting one up would violate the principle of display inertia", and the last resort for unknown work is "a spinning ball ... that at least indicates that the system is working". Also: animations should be "timed according to a real-time clock rather than being timed as an indirect effect of the computer's execution speed".

## How shipping games do it

Loading and processing screens in shipped games pair a length with words ("Compiling shaders 41 / 120"), and a ring spins on a real-time clock so a stalled frame does not freeze it. Xbox 103: a state is carried by shape and text, not a colour: the bar is a length, the step and the time left are words.

## Model adopted

- **P1. `progress(fraction) -> impl Bundle`** is a column: a `Role::Meter` track and `Role::MeterFill` (the meter's look: one visual language) and a line of text. `Progress(f32)` is the state; the fill is its fraction the same frame; outside 0 to 1 it is clamped and reported once (the marker `ProgressClamped`, one `warn!`).
- **P2. The words are `ProgressText { step: Option<(u32, u32, String)>, eta_seconds: Option<f32> }`**, drawn "3 of 7: Measuring meshes · about 20 s left"; with neither, the line is not drawn. `model::progress_text` and `model::eta_words` are engine-free: under 10 s exact seconds, then 5 s steps to a minute, whole minutes to an hour, then "1 h 5 min".
- **P3. `spinner()` is indeterminate**: a ring of one accent quarter on the track colour turning once in 0.9 s on `Time<Real>`, so it keeps turning while the game is paused or a frame stalls. There is no static variant. A `Spinner` marker only; `update_spinners` writes the rotation each frame.
- **P4. The guide documents the thresholds** (NN/g): under 1 s nothing; 1 to 10 s a spinner; 10 s or more a progress with the time left.

## Parameters

|part|value|
|---|---|
|widgets|`progress(f32)`, `spinner()`; components `Progress(f32)`, `ProgressText`, `Spinner`; marker `ProgressClamped`|
|track|the meter's: `meter_height`, `outline` frame, `accent` fill|
|words|body text in `text` (4.5:1), one line, never wrapped|
|spinner|`1.5` body text sizes square, `border` wide ring, one turn per 0.9 s|

## Validation targets

- A progress at 0.5 draws half its track (fill width = half the track's inner width); 0.25 a quarter; 1.5 draws full, 2 is one warning not two, back in range clears the marker; NaN draws empty.
- `ProgressText` draws "3 of 7: Measuring meshes · about 20 s left"; step only, eta only, none (line hidden, no height).
- A spinner's rotation advances each frame on `Time<Real>` (at a fixed step the angle moves by the step over 0.9 s), and wraps.
- Thirty idle frames write nothing for a progress; a spinner writes its rotation only.
- Words at 4.5:1 on the panel in all four looks; ring at 3:1; whole at 200% text.

## What great looks like

A bar that is the meter's own track, the words under it in the panel's text colour, and a ring that turns smoothly at any frame rate.

## Deviations

No percent figure is drawn unless the host puts it in a step's text. Design default.

## Pending index

The NN/g page is not in home-still.
