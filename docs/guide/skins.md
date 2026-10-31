# Skins

`GooeySkin::dark()` is the default; `GooeySkin::parchment()` is the light one. Edit the resource live (an options screen, or an editor) and `apply_skin` restyles every widget that frame. With the `serde` feature a skin is RON or any serde format, and a partial file fills the rest from the default, so a project saves only what it changed:

```ron
(accent: Srgba((red: 0.2, green: 0.6, blue: 0.9, alpha: 1.0)), text_scale: 1.25)
```

`GooeySkin::problems()` lists what a skin breaks, and the plugin warns once per problem when the skin changes (it never refuses one):

- text under 4.5:1 contrast against the surface, the raised surface, a hovered button, or the accent (WCAG 2.2, 1.4.3);
- a focus ring under 3:1 against the raised surface (WCAG 2.2, 1.4.11);
- a meter's fill (the accent) under 3:1 against its track, or its frame (`outline`) under 3:1 against the panel, or an off switch's knob (`text_muted`) under 3:1 against its track, or a toast's mark (`accent`, `danger`) under 3:1 against the toast's surface (WCAG 2.2, 1.4.11);
- `min_target` under 24 px (WCAG 2.2, 2.5.8; the default is 44 px);
- a meter too thin to show a fill inside its frame and gap (`meter_height × text_scale` at most 4 × `border`);
- a size that is not finite, or not positive where it must be.

Fonts are yours: set `GooeyFonts { body, heading }`. The default draws Bevy's built-in font when Bevy's `default_font` feature is on.

## Every field

The colours are `background`, `surface`, `surface_raised`, `text`, `text_muted`, `accent`,
`accent_text`, `danger`, `focus_ring`, `outline` and `scrim`; the sizes are `text_size`,
`heading_size`, `gap`, `padding`, `radius`, `border`, `meter_height` and `min_target`; `text_scale` is
the player's option and multiplies text alone. A name ends up in exactly one place: a heading's text
colour is `text` there and nowhere else, because `apply_skin` styles every widget by its `Role`.

`SkinProblem` is the type `problems()` returns; the plugin warns once per problem on a change and never
refuses a skin, so an editor can show a half-finished one.

## Loading one

`GooeySkin` and `UiScaleSettings` derive serde under the `serde` feature, and both are
`#[serde(default)]`, so a project's file holds only what it changed:

```ron
(accent: Srgba((red: 0.2, green: 0.6, blue: 0.9, alpha: 1.0)), text_scale: 1.25)
```

A game sets the resource at start-up (`app.insert_resource(GooeySkin::parchment())`); Grimoire reads a
project's `family/bevy_gooey.ron` into the same type while the editor runs.
