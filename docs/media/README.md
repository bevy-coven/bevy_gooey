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
| `gallery.png` | the still: Play/Options/Quit, a health meter, the Audio, Display and Controls tabs with Audio lit over the Volume slider and the Subtitles switch, the Load game list four and a half rows tall with its rail and the ring on `Chapel, 00:12`, and one toast at the bottom right | `GOOEY_FRAMES=200 GOOEY_SHOT=docs/media/gallery.png GOOEY_KEY_EVERY=14 GOOEY_KEYS=slots,down,down,down,pagedown,toast GOOEY_SIZE=1280x720 cargo run --release --example gallery` | 2026-10-31 | 0.4.0 |
| `gallery.gif` | the hero (5.9 s, 960×540): the ring walks down the save slots from Autosave to `Mill, 03:02`, Page Down moves it to `Chapel, 00:12`, the meter drains, and a "Game saved" toast appears | the same run with `GOOEY_SHOT_DIR=/tmp/gooey-gif GOOEY_SHOT_FROM=30 GOOEY_SHOT_EVERY=2`, then `ffmpeg -framerate 15 -pattern_type glob -i '/tmp/gooey-gif/f_*.png' -vf "fps=15,scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer" -loop 0 docs/media/gallery.gif` (a glob: the capture can skip a frame number) | 2026-10-31 | 0.4.0 |
| `tabs.gif` | the tabs (4.7 s, 520×540, the settings panel only): the ring moves from Quit onto the lit Audio tab; Right turns to Display, Right to Controls, Right wraps to Audio; Down enters the page on the Volume slider; Ctrl+PageDown turns to Display and to Controls with the ring landing on Quality and on Rebind keys; Ctrl+PageUp back to Display | `GOOEY_HEALTH=60 GOOEY_FRAMES=190 GOOEY_KEY_EVERY=16 GOOEY_KEYS=down,down,down,right,right,right,down,nexttab,nexttab,prevtab GOOEY_SIZE=1280x720 GOOEY_SHOT=/tmp/tabs.png GOOEY_SHOT_DIR=/tmp/gooey-tabs GOOEY_SHOT_FROM=56 GOOEY_SHOT_EVERY=2 cargo run --release --example gallery`, then `ffmpeg -framerate 15 -pattern_type glob -i '/tmp/gooey-tabs/f_*.png' -vf "crop=520:540:224:90,fps=15,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer" -loop 0 docs/media/tabs.gif` | 2026-10-31 | 0.4.0 |
| `cancel.gif` | Back (4.1 s, 848×540, both panels): the ring moves onto Options and then Quit, Enter opens "Leave the game?" over the dimmed screen with the ring on Stay, and Escape closes it with the ring back on Quit (the close a pad's B makes too) | `GOOEY_HEALTH=60 GOOEY_FRAMES=142 GOOEY_KEY_EVERY=18 GOOEY_KEYS=untip,down,down,enter,escape GOOEY_SIZE=1280x720 GOOEY_SHOT=/tmp/cancel.png GOOEY_SHOT_DIR=/tmp/gooey-cancel GOOEY_SHOT_FROM=26 GOOEY_SHOT_EVERY=2 cargo run --release --example gallery`, then `ffmpeg -framerate 15 -pattern_type glob -i '/tmp/gooey-cancel/f_*.png' -vf "crop=848:540:216:90,fps=15,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer" -loop 0 docs/media/cancel.gif` (`untip` takes Options' tooltip off first, so the first beat is a still screen) | 2026-10-31 | main after 0.4.0 (the release after it) |

A capture is one run of the example; the `gooey: frames <n> mean_ms <x>` line it prints is the frame
time at that size, and the `gooey: kept <n> frames in <dir>` line says how many frames a GIF got.

## The skins

Both presets are one line: `GOOEY_SKIN=parchment` starts in the light one, and
`GOOEY_TEXT_SCALE=1.5` shows the player's text-size option. Neither is in the README today, so neither
has a picture here; a still of the parchment skin is the capture a docs change would add first.
