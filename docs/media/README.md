# `docs/media/` — every image, with the command that made it

Every file in this folder, what produced it and the release it shows. Nothing here is in Git LFS, and
every reference from a markdown file is a relative path (`docs/media/…`), so crates.io rewrites it to
this repository and the pictures show on the crate's page too.

Every picture comes from `examples/gallery`, which is the crate's own capture path: `GOOEY_FRAMES=<n>`
saves one still at frame `n` to `GOOEY_SHOT`, and `GOOEY_SHOT_DIR=<dir>` keeps every
`GOOEY_SHOT_EVERY`th frame from `GOOEY_SHOT_FROM` there as `f_%04d.png` for a GIF. `GOOEY_SIZE`
sets the window, `GOOEY_SKIN` the preset, and `GOOEY_KEYS` stages the input each shot shows.

| file | what it shows | command that regenerates it | captured | shows |
|---|---|---|---|---|
| `gallery.png` | the still: every widget on one screen — Play/Options/Quit, a health meter draining, the Subtitles switch, the Quality choice, the Volume slider, the Load game list four and a half rows tall with its rail, and two toasts at the bottom right | `GOOEY_FRAMES=200 GOOEY_SHOT=docs/media/gallery.png GOOEY_KEY_EVERY=14 GOOEY_KEYS=slots,down,down,down,pagedown,toast GOOEY_SIZE=1280x720 cargo run --release --example gallery` | 2026-10-31 | 0.3.0 |
| `gallery.gif` | the hero: focus walks down the panel onto Options, the meter drains, the Load game list scrolls to follow the ring onto `Orchard, 00:20`, and a "Game saved" toast appears | the same run with `GOOEY_SHOT_DIR=/tmp/gooey-gif GOOEY_SHOT_FROM=30 GOOEY_SHOT_EVERY=2`, then `ffmpeg -framerate 30 -i /tmp/gooey-gif/f_%04d.png -vf "fps=15,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer" -loop 0 docs/media/gallery.gif` | 2026-10-31 | 0.3.0 |

A capture is one run of the example; the `gooey: frames <n> mean_ms <x>` line it prints is the frame
time at that size, and the `gooey: kept <n> frames in <dir>` line says how many frames a GIF got.

## The skins

Both presets are one line: `GOOEY_SKIN=parchment` starts in the light one, and
`GOOEY_TEXT_SCALE=1.5` shows the player's text-size option. Neither is in the README today, so neither
has a picture here; a still of the parchment skin is the capture a docs change would add first.
