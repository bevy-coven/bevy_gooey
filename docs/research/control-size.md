# Control sizes

Researched 2026-11-07 for issue #57 (G-50), asked by bevy_grimoire's feel series (bevy-coven/bevy_grimoire#330). The reference is Rune (https://github.com/anomalyco/rune at `f73f72cdc563534e4f7de9f5a79bed007ac42384`, MIT, © Anomaly), read as an open reference. No Rune code is in this crate.

## Sources

1. Rune, `crates/rune-kit/src/lib.rs:182-193`: "A control's size, named and measured as AppKit's `NSControl.ControlSize` on macOS 27. A push button is 16, 20, 24, 28 or 36 points tall; a switch's track 16, 20, 24, 28 or 36." The variants are `Mini`, `Small`, `Regular` (the default), `Large` and `ExtraLarge`.
2. WCAG 2.2 SC 2.5.8 (Target Size, Minimum): a target is at least 24 by 24 CSS px. This crate's `MIN_TARGET_PX` and `GooeySkin::target()` hold it.

## Model adopted

- **`ControlSize { Small, Regular, Large }`** is a public component, with serde. The sizes are 24, 28 and 36 logical px (`CONTROL_SMALL_PX`, `CONTROL_REGULAR_PX`, `CONTROL_LARGE_PX`), which are Rune's `Regular`, `Large` and `ExtraLarge`.
  - Rune's `Mini` (16) and `Small` (20) are under 24 and are not offered.
  - The names are the issue's, so they are one step below Rune's for the same height.
- **The height** is `size.height(skin.target()) = max(size, target)`. Under a compact skin (Desk, tool: target 24) there are three heights. Under a comfortable one (target 44) every size is 44.
- **Where it applies.** The component goes on the control's root and is read in `apply_skin` through the restyle state (`State::control`). A `Changed<ControlSize>` restyles it and its parts in its frame, and so does a removed one (`Removals`).
  - **`button`, `icon_button`:** `Role::Button`.
  - **`toggle`:** `Role::Toggle`.
  - **`choice` and `select`:** `Role::Choice`. The arrow fills the row inside its frame.
  - **`text_input` and the palette's field:** the row and its framed field. The field's inset centres the line in the size, so the framed field is the size and the row adds its label's air.
  - **`text_area`:** the size is its least height. Its lines start at its top, so its inset stays the skin's.
  - **`number_field`, `vec_field`:** the framed fields follow the root's size. A vec field's component is a part inside a part, so a part with no size of its own takes the next owner's (`owner_of`).
  - **`color_field`:** a `text_row` as well.
- **A sized control's vertical air** is what centres its content (the line of text as it is set, or for a toggle the larger of that and its switch) in the size, inside its frame, never below zero. That is how a Desk button at `Small` is 24 and not the 25 its padded label makes.
- **An unsized control** keeps the skin's own air and height exactly as before: the code path is unchanged when no `ControlSize` is present.

## Where this differs from Rune

- **There is no 16 or 20.** WCAG's 24 px floor wins over AppKit's mini and small controls.
- **The size does not change the text.** The text size follows the skin, as the issue asks. Rune's smaller AppKit sizes also use a smaller font.

## Proof

`tests/control_size.rs`, 6 tests:

- Under the Desk, a button, an icon button, a toggle, a select and a text field are 24, 28 and 36 logical px (±0.5) at `ControlSize::Small`, `Regular` and `Large`. At an interface size of 2 (`UiScale` 2) they are twice that in physical px.
- Under dark (target 44) every size is at least 44.
- With no size, every control is at least the skin's target.
- A choice and a number field grow with their size, and are never under it.
- Changing every control's size from `Small` to `Large` restyles it and its parts (the select's arrow, the text input's field) to 36 px in the next frame, and taking the size off puts each back at its unsized height.
- A vec field's components are 24, 28 and 36 px at each size.

The API is new, so the file does not compile before this change.
