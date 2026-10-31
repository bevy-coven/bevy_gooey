# Dialog presence: a panel over the scrim reads as a layer, and holds a title, a line of body and its choices

Researched 2026-10-31 (iteration 16). home-still's corpus has nothing on modal-dialog layout or layering (`distill_search`, "modal dialog visual hierarchy elevation shadow contrast layer separation interface" and "confirmation dialog game menu design player errors": a spoken-dialogue paper, a long-term-memory paper, SIGGRAPH shadow slides, a game-testing paper and an AR task-time paper; `catalog_recent` limit 200 held only biomedical, SIGGRAPH and rendering-course embeds), so the sources below are web copies and Bevy's layout rules. Every quotation was fetched on 2026-10-31 and matches; the gap is under `## Pending index`.

## Sources

1. Microsoft, *Dialog controls* (Windows apps) — https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/dialogs-and-flyouts/dialogs; Nielsen Norman Group, Fessenden, T., *Modal & Nonmodal Dialogs: When (& When Not) to Use Them* (2017) — https://www.nngroup.com/articles/modal-nonmodal-dialog/ (craft: what a dialog says, in what order, and why it must be the focal point).
2. Koncewicz, R., *Watch Your Game Prompts!* (Game Developer, 2012, on Resident Evil 5's confirmations) — https://www.gamedeveloper.com/design/watch-your-game-prompts-; Microsoft, *Designing for Xbox and TV* — https://learn.microsoft.com/en-us/previous-versions/windows/uwp/xbox-apps/designing-for-tv (shipping games and the console's 10-foot rules: the title relates to the choices, the focus is unmistakable, nothing essential in the outer 5%).
3. Microsoft Q&A, *Cannot change the size of a dialog* — https://learn.microsoft.com/en-us/answers/questions/1325684/cannot-change-the-size-of-a-dialog (the platform dialog's measure); W3C, *WCAG 2.2*, 1.4.11 Non-text Contrast, and Bevy 0.20.0 `bevy_ui-0.20.0` (taffy: `min_width`, `max_width` and a percentage padding on a flex container) and this crate's `apply_skin` (technical: what the layout does, what the skin already guarantees).

## Craft principle

Microsoft's guideline is the content order: "Clearly identify the issue or the user's objective in the first line of the dialog's text." "The dialog title is the main instruction", and below it "If a dialog title is used, use the content area to provide more detail or define terminology. Don't repeat the title with slightly different wording." "Make sure that the title relates directly to the button choices." And the buttons: "Use specific responses to the main instruction or content as button text." A dialog is therefore three things in a fixed order, a title, a line of detail and its choices; the crate gives a game a `heading` and a `label` already, and the panel they stand in must hold them.

NN/g says why the dialog has to be the focal point: "Modal dialogs were originally intended to alert users to an error or to some other system state that required immediate user action. In these cases, it was essential for users to be interrupted in order to fix the error. Thus, placing the dialog box in the middle of the screen as the focal point of the interface, made it very effective. The big advantage of such modal dialogs was that they attracted users' attention and allowed them to acknowledge the problem and correct it quickly." And when to use one at all: "Whenever there is a chance that users' work be lost or that an action may have destructive, irreversible consequences, interrupt the users to avoid a disaster." A "Leave the game?" is that case, and a box the size of its own heading, in the colour of the menu behind it, does not look like an interruption.

## How shipping games do it

Koncewicz on Resident Evil 5's chain of confirmations: "The "Ready" confirmation is horribly labeled as it's an ambiguous descriptor. Is the player exiting the inventory screen, or the actual save game loading process (it's the first one, but it always makes me stop and think)?" and "The convention also seems to be that "No" should be the default selection." The label decides whether a dialog is read at a glance, which is why the title and one line of body belong in the dialog the crate draws, and why the gallery's Stay stays the default.

Microsoft on the 10-foot experience: "Make the focus clear and unmistakable." and "it's also important that interactive elements and text always avoid the screen edges to ensure that they won't be cut off on some TVs. We recommend that you draw only non-essential visuals within 5% of the screen edges." (27 epx top and bottom and 48 epx left and right on a 960 x 540 epx screen). Its body text is "15 epx minimum" on that screen, which is this crate's 20 px at 1280 x 720 (20 × 540 / 720 = 15 epx): the crate's logical pixel is a 10-foot epx scaled by 4/3, so a platform dialog's measure converts straight into text sizes.

## Technical

The platform's dialog is a measure, not a box that fits its content: "The default maximum width of `ContentDialog` is 548." (epx; at 960 x 540 that is 548 / 0.75 = 731 logical px at 1280 x 720, 36 body text sizes of 20 px). Its minimum is a theme resource of the same family, `ContentDialogMinWidth`, reported by search as 320 epx, which is 427 px, 21 text sizes (the primary file was not reachable: see `## Pending index`).

WCAG 2.2 1.4.11 Non-text Contrast: "The visual presentation of the following have a contrast ratio of at least 3:1 against adjacent color(s): User Interface Components: Visual information required to identify user interface components and states". The crate already has the colour whose job is that: `GooeySkin::outline`, "the edge of a widget whose fill does not show its size", checked against the panel at 3:1 (the meter's frame). A dialog is the widget whose fill does not show its size when it stands on a dimmed copy of the screen in a nearly equal colour.

Bevy 0.20 layout (taffy, flex): a `Node` with `min_width` and `max_width` is clamped between them whatever its content, text wraps at the width it is given, and a flex item shrinks to its container's inner width unless its `min_width` forbids it. A percentage padding resolves against the container's width, so `Val::Percent(5.0)` on the full-window scrim is the 5% edge on every side and a long body can never push the panel out of the safe area.

Measured in the gallery before this pass (`/tmp/g16/before/quit-1280x720.png`): the panel is 212 px wide round a 180 px "Leave the game?", 16 px of air each side, filled (29,32,40) as the dimmed menu behind it is (16,17,23) and framed in `surface_raised` (51,56,69), the colour of every button. It has no heading and no body, because the gallery's title is a `label`.

## Model adopted

A panel that is a direct child of a `dialog()` becomes a dialog panel, in `apply_skin`, from the skin's own values. No new field and no new widget: a game's existing `dialog()` + `panel()` code is better the day it updates, and a `heading` and a `label` in it are the title and the body.

- **P1. A measure.** `min_width` 21 body text sizes (420 px at the default size, scaling with the text), `max_width` 36 (720 px). A short title fills the minimum and the buttons are as wide as the panel; a long body wraps at the maximum and the buttons stay the panel's width.
- **P2. A room.** Padding one and a half paddings (21 px) where a screen's panel has one, and a gap and a half between the title, the body and the choices (15 px): the dialog is read in order, and nothing is closer to the edge than to the next line.
- **P3. An edge.** The frame is `outline`, one line weight, where a screen's panel's is `surface_raised`: 3:1 against the panel by the skin's own check, and against the dimmed screen behind it far more, so the extent of the dialog is visible at a glance in both presets and at both sizes.
- **P4. The safe area.** The scrim pads itself 5% of the window on every side (Microsoft), so a dialog taller or wider than the screen less its edge is made to fit and never reaches the bezel.
- **P5. Nothing else moves.** A panel that is not in a dialog, and every other widget, is pixel-identical before and after; `GooeySkin` has no new field; no system runs at rest (the change is in `apply_skin`, which restyles on a change only).

## Parameters

|part|value|
|---|---|
|dialog panel `min_width`|21 × `text_size` × `text_scale`|
|dialog panel `max_width`|36 × `text_size` × `text_scale`|
|dialog panel `padding`|1.5 × `padding`|
|dialog panel `row_gap`|1.5 × `gap`|
|dialog panel frame|`outline`, one line weight (`snap_line_width`)|
|scrim `padding`|`Val::Percent(5.0)` on every side|

## Validation targets

- The Quit dialog's panel is at least 21 text sizes wide at 1280x720, 1440p and 1.5x text, its heading and body inside it with at least the padding on every side, and the buttons the panel's inner width.
- A body of 160 characters wraps at the maximum (the panel is 36 text sizes wide and every line inside it), the dialog is at least 5% of the window from every edge, and the buttons are still inside the panel.
- The frame is `outline` and a panel not in a dialog keeps `surface_raised`; both presets, frame against the dialog's fill at 3:1 or more and against the dimmed screen behind it more than before.
- `GooeySkin::problems()` is empty for both presets and the text colours checked are the colours drawn (the fill is unchanged).
- Eight existing stills (dark, parchment and 1.5x text at 1280x720, 2560x1440, no dialog open) are pixel-identical before and after.
- Focus, Tab, arrows, Back and the dialog's focus-in and focus-back are unchanged (the existing `tests/dialog.rs` and `tests/cancel.rs` pass as they are).
- Frame time of the gallery within 0.5 ms of before.

## What great looks like

Down, Down, Enter: the menu dims and a framed dialog stands in the middle of the screen, wide enough to read as a room and not a tooltip: "Leave the game?" as a title, one line under it saying what is lost, two choices as wide as the box, the ring on Stay. Its edge is the lightest thing on the screen after the text, so even at couch distance the eye lands in it and nowhere else. In parchment the same dialog is a cream card with a brown frame on a dark-brown screen. It is the same box at 1.5x text and at 2560x1440, twice the size, with the same air round its words.

## Deviations

- **The 21-text-size minimum is a design default**: the platform's 320 epx, scaled by this crate's 4/3 and rounded to a whole number of text sizes, taken from a search result and not from the theme file (404). The maximum, 548 epx, is from Microsoft's answer.
- **The frame is `outline`, not a new colour**: a skin that sets `outline` for the meter's sake changes the dialog's edge with it, and that edge is already held to 3:1 against the panel. The fill is not lifted: a lighter fill would bring the panel closer to its buttons' `surface_raised`, whose 1.39:1 against the panel is already the narrowest pair on the screen. An assumed decision.
- **No drop shadow.** A shadow in the background colour on a scrim that is 60% black is not visible in the dark preset, and the toast's note explains why a shadow does not belong to a clipped room; the edge and the scrim carry the separation. An assumed decision; revisit if a skin with a light scrim is added.
- **A panel is a dialog panel only when it is a direct child of `dialog()`**, the rule that already selects its opaque fill (`State::scrim`). A game that wraps it in another node gets a screen's panel inside the dialog, as before.
- **No new API for the body**: `heading` and `label` in the panel are the title and the body (Microsoft: a title, then detail); `dialog()` keeps its signature, and a builder that takes the strings is the next step if games write the same four lines each time.
- Assumed decisions (owner may override), all above: the measure's two numbers, the padding and gap factors, the frame colour, no lift, no shadow, 5% for the safe area.

## Pending index

- Nothing relevant in home-still (two queries on 2026-10-31 and `catalog_recent` limit 200). Index Microsoft's *Dialog controls*, NN/g's *Modal & Nonmodal Dialogs*, Koncewicz's *Watch Your Game Prompts!* and Microsoft's *Designing for Xbox and TV*. Search later for: "modal dialog visual hierarchy", "dialog box size and spacing", "game confirmation prompt". The WinUI `ContentDialog_themeresources.xaml` (the min and max widths and the padding) was not reachable at the path tried (HTTP 404) and the minimum width rests on a search result. No GDC talk or studio write-up of a game's own confirmation dialog was found; the shipping-game evidence is one developer's critique of Resident Evil 5 and Microsoft's console guidance. The Game Accessibility Guidelines and *The Gamer's Brain* remain pending from `docs/research/gooey.md`.
