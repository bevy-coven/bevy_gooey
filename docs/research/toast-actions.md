# Toast actions, sticky errors and a notification log

Researched 2026-11-02 (iteration 39, #22 G-18). home-still's index has nothing on notification patterns, so the sources below were fetched today.

## Sources

1. W3C WAI, *Understanding SC 2.2.1 Timing Adjustable*, https://www.w3.org/WAI/WCAG22/Understanding/timing-adjustable.html — craft.
2. IBM Carbon Design System, *Notification* (toast, actionable, notification center), https://carbondesignsystem.com/components/notification/usage/ — a shipping design system. (No shipped *game* source: Xbox Accessibility Guideline 116, cited in `docs/research/toast.md`, is the game-side twin of 2.2.1 and is already in this crate's `ToastSettings`.)
3. Bevy 0.20 `On<Remove<T>>`, `EntityEvent`, `bevy_ui_widgets::Button`/`Activate`, and this crate's `toast.md`, `lists.md` — technical.

## Craft principle

WCAG 2.2.1: "Users have adequate time to complete tasks … Let users turn off, adjust, or extend time limits." and the exception that fits a toast: "Content that operates on a timer does not need to be time adjustable if there is an alternative that does not rely on a timer. For example, … a temporary message (such as a 'toast' message) … disappears after 5 seconds. Users are able to identify the arrival of email through other means … If the user has no other means of discovering the same information (or performing the same function), then each message would need to meet this success criterion". A toast that carries the only way to undo, or the only report of an error, is therefore not allowed to be only a timer: it needs a way to stay (sticky) and a place to be found again (the log).

## How shipping games do it

Carbon, on toasts: "Because toast notifications can dismiss automatically, users should be able to access them elsewhere after the toast notification disappears if they need more time to read the notification or who want to refer to it later", and "Consider using a notification center where a user can revisit and act on past notifications." On actions: "Actionable notifications are inline or toast notifications that have interactive elements. Because actionable notifications require user interaction, actionable notifications come into focus when triggered and, therefore, they disrupt screen readers and keyboard users. Only one action is allowed for each notification", and "Actionable notifications persist until a user dismisses them … Because the notification remains open, the user has enough time to interact with the button without the toast closing too soon." On errors: "Inform users of an error or critical failure"; "User actions are mandatory for error messages."

## Model adopted

- **N1. `toast(..).action(label, id)`**: `toast` and `toast_alert` return a `ToastBundle` (a `Bundle`, so `commands.spawn(toast("x"))` and tuples work as ever) with `.action(label, id)` and `.sticky()`. A button in a toast is the toast's one interactive part: a `Role::ToastAction` button (a button's look, fading with its toast), a tab stop reached by the pointer, the keys and the pad. Pressing it sends **one `ToastAction { toast, id }`** (`EntityEvent` on the toast, `id: String`) and dismisses the toast: it fades out now. Every other node of a toast, and every node of a toast with no button, keeps `Pickable::IGNORE` and no tab stop, so the HUD rule (`AGENTS.md`: a toast takes no input and no focus) holds for what it was written for.
- **N2. An action toast stands at least `ToastSettings::action_seconds`** (6, times the player's `duration_scale`), so a button can be reached; Carbon would hold it until dismissed, and `.sticky()` does that.
- **N3. `.sticky()`** (any kind; `toast_alert(..).sticky()` is the error form): stands until dismissed, with a **Dismiss** button; it is not counted against `max_visible`, so a newer toast never pushes it out (an error must not vanish before it is read).
- **N4. Repeats merge**: a toast identical (kind, words, buttons, stickiness) to one that arrived less than `ToastSettings::merge_seconds` (2, 0 turns it off) ago folds into it: the standing one's count goes up, its words read "Saved x3" (ASCII `x`: the host supplies the font and `×` is not in every one), it stands again from the start of its hold, and the new entity is despawned.
- **N5. `ToastLog`** (resource, `Reflect`, plain data) keeps the last `ToastLog::CAPACITY` (200) toasts spawned, merged or not: seconds on `Time<Real>`, kind, words. `clear()` empties it.
- **N6. `notification_center()`** is a `list` of the log, newest first, a row a `Role::Button` (a tab stop, so the keys and the pad scroll it like any list) reading `m:ss  words` (`model::clock_text`), an alert's words start with `! ` (the mark is never only a colour), "No notifications yet" while empty. `NotificationRow` marks a row; the center rebuilds only when the log changes.
- **N7. Focus on a toast's button that goes with its toast is cleared** (`On<Remove<ToastButton>>`), so focus never rests on nothing.

## Parameters

|part|value|
|---|---|
|settings|`ToastSettings::{action_seconds: 6, merge_seconds: 2}`|
|event|`ToastAction { toast, id }`|
|components|`ToastButton`, `ToastLog`, `ToastLogEntry`, `NotificationCenter`, `NotificationRow`|
|roles|`ToastAction`, `ToastActionText`, `NotificationCenter`|
|log|200 entries, `m:ss`, newest first, 5.5 rows tall|

## Validation targets

- The issue's acceptance: clicking the action sends one `ToastAction`; three identical toasts show one reading "x3"; the log holds all three.
- Enter on the focused button does the same; a second press while it fades sends nothing; the toast goes.
- A merge is only inside the window and only for the same words, kind and buttons; it restarts the hold; `merge_seconds: 0` turns it off.
- A sticky toast is still whole after 20 s and five newer toasts; Dismiss sends no `ToastAction` and the toast goes.
- An action toast is whole at 4.5 s (a plain one is gone) and gone by 7.5 s.
- A plain toast has no button, tab stop or pickable node; an action toast has exactly its button; focus on it is cleared when it goes.
- The log keeps 200 with their time, kind and words, forgetting the oldest; the center lists newest first, marks alerts, follows the log, says "No notifications yet" when empty, scrolls 100 entries in a list a few rows tall.
- The button's words are 4.5:1 on its fill in all four looks; the button is `skin.target()` high; an action toast holding writes nothing.

## What great looks like

A toast with one quiet button at its end ("Item deleted" [Undo]), an error that stays with its Dismiss, three "Game saved" presses becoming one "Game saved x3", and a panel of the missed ones a player can scroll back through.

## Deviations

- Carbon holds an actionable toast until dismissed; here it stands `action_seconds` and `.sticky()` is the opt-in, so a game's HUD is not filled by actions nobody pressed (design default; WCAG 2.2.1 is met by the log and the sticky form).
- Carbon allows one action a toast; the API accepts several and the guide says one (design default).
- Carbon says an actionable notification "comes into focus when triggered"; here it never takes focus by itself (a game's focus must not jump), so a keyboard player reaches it with Tab (design default).
- The issue writes "×3"; ASCII "x3" is used (the host supplies the font).
- The issue's `ToastKind::Error`: the crate's kinds are `Info` and `Alert`; `.sticky()` applies to either.

## Pending index

The WCAG and Carbon pages are web pages not in home-still.
