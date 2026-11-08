# Button variants

Researched 2026-11-07 for issue #58 (G-51), asked by bevy_grimoire's feel series (dark-incorporated/bevy_dark_factory#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune `crates/rune-kit/src/actions.rs:27-39`: `ButtonVariant` Neutral, Danger, Warning, Outline, Contrast, Submit, Ghost, GhostMuted, GhostFaint, Loading; `IconButtonVariant` at `actions.rs:378-385`.
2. Rune `crates/rune-kit/src/kit.rs:37-68`: the submit and contrast fills carry a top sheen, a hover overlay and depth.
3. WCAG 2.2 SC 1.4.3 (4.5:1 text) and SC 1.4.6 (7:1): the checks in `GooeySkin::problems()` and `high_contrast_problems()`.
4. bevy_gooey#76: real-time test apps run with `MotionSettings::off()`.

## Model adopted

- **`ButtonVariant`** is a component on a `button` or `icon_button` (an icon button is a `Role::Button`). Absent is `Neutral`, today's button. The colours are `GooeySkin::variant_fill(variant, phase)` and `variant_ink(variant, phase)` over fields the skin already has, so no preset or skin file gains a field.
  - **Submit:** accent, `primary_hovered`, `primary_pressed`; ink `accent_text`.
  - **Contrast:** the text colour as a fill, lifted toward the panel colour under the pointer and held; the ink is the first of the panel, the ground, the text, black and white that reads at 7:1.
  - **Danger:** `danger`, lifted toward the text by one and two primary-button steps (`PRIMARY_LIFT_L`); the same ink rule.
  - **Warning:** the danger colour's lightness and chroma at an amber hue (Oklch 80°), so it is as visible as danger is; the same lift and ink rule.
  - **Outline:** no fill at rest, the `hovered()` fill under the pointer, and held a panel fill moved one lift away from the text (sunk), because a skin's `pressed()` is the panel itself unless it says otherwise, which would not show; a panel already as dark as it goes under white text moves toward the text instead. Ink is the text, and the frame is the skin's `outline`.
  - **Ghost:** the same fills as Outline, with no frame.
  - **GhostMuted:** the same fills as Outline with the muted ink at rest, the text ink once the pointer is on it.
  - **GhostFaint:** answers the pointer with half the hovered fill.
- **Checked by the skin.**
  - **Ink:** `problems()` holds every new variant's ink (the plain button and Submit keep the pairs it always had) on its fill (over the panel for a variant with no fill) at rest, hovered and pressed to 4.5:1, and `high_contrast_problems()` to 7:1.
  - **Fills:** the same pass holds each variant's rest, hover and pressed fills a lightness step (`STATE_STEP`) apart, as it holds a primary button's.
- **Loading(bool)** swaps the label for a `Spinner` (absolute, so it takes no room) and hides the label and icon with `Visibility::Hidden`, so the button's width is kept. It inserts `InteractionDisabled` so Bevy's button sends no `Activate`, and `LoadingHold` tells `apply_skin` to draw the button as itself and not dimmed. A button the host had already disabled stays disabled when the loading ends.

## Where this differs from the issue and from Rune

- **`primary_button` and `PrimaryButton` stay.** The issue asks for their removal ("clean cutover"). Both are pinned by the protected `tests/primary_button.rs`, and removing them deletes those assertions. `PrimaryButton` is now exactly `ButtonVariant::Submit` (one look, one code path in `apply_skin`). Removal is for the owner to decide with the test file.
- **No sheen.** The issue asks Submit and Contrast to draw a top sheen with `BackgroundGradient`. Bevy's `BackgroundGradient` replaces the fill's colour node, which colour motion eases, and a gradient would be a second paint under every restyle. It is left out and recorded as not built.
- **Disabled keeps no hue.** A disabled variant draws as every disabled button does (the panel fill, muted ink): that look already passes the disabled-text rule, and a hue-keeping dim would be a second check per variant.
- **Steps checked.** `problems()` holds Contrast, Danger and Warning to a lightness step between each of their fills, and a quiet variant to a step between rest and held. The plain button and the faint ghost answer the pointer with the skin's own hovered fill, whose step from the panel is the skin's (some presets have 3.4 L\*, under the 4.0 a primary button's lift keeps), so that step is not refused here.
- **Warning** is pulled into sRGB by lowering its chroma, so the colour checked is the colour drawn.
- **A busy button's spinner** is drawn in the button's ink, with the other three quarters a pale trace of it, so it reads on any fill (the skin's accent quarter is invisible on a Submit button).
- **Loading** notes a host that disables the button while it loads and keeps it disabled after; removing the `Loading` component releases the hold; the label and icon return to the visibility they had. A label or icon added while loading stays visible.
- **Gallery still** is owed: big has no display.

## Proof

`tests/button_variant.rs`, 7 tests (the three new ones: a quiet button held is seen to change on the panel in every preset; a host-disabled button stays disabled after loading and a removed `Loading` releases it; the outline frame). Every variant's three fills differ and the skin with its variants is clean in every preset, and at 7:1 in high contrast; a skin whose danger has no lift is refused by name; a running button draws its fill and ink at rest and held, and a changed variant restyles; a Submit variant is the primary button's look; a loading button keeps its width ±0.5 px, hides its label, shows one spinner, draws as itself, and takes no click, then presses again when done.
