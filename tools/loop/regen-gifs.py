#!/usr/bin/env python3
"""Re-record every demo GIF listed in docs/media/README.md and keep the ones that changed.

Each GIF's row in that table holds the command that made it: a capture run of an example
(`GOOEY_FRAMES=... cargo run --release --example split`) and an `ffmpeg` line that turns the kept
frames into the GIF. This script runs both with the prebuilt release examples, on the display in
`$DISPLAY` (the loop's Xvfb), into a scratch directory, then compares the new GIF with the committed
one by its frames (downscaled, grey) and replaces the committed file only when it differs by more
than noise: a different number of frames, or a mean difference above `--threshold` grey levels. A
rewrite of byte-different but identical-looking GIFs would add tens of MB to the repository every pass.

    cargo build --release --examples --all-features   # the inspector example needs `reflect`
    python3 tools/loop/regen-gifs.py            # every GIF
    python3 tools/loop/regen-gifs.py split tabs # only these
    python3 tools/loop/regen-gifs.py --dry-run  # record and compare, change nothing

Prints one line per GIF: `same`, `UPDATED`, `FAILED` with the reason. Run from the repository root.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MEDIA = ROOT / "docs" / "media"
THUMB = (96, 54)
FRAMES_MAX = 400


def rows():
    """(name, run command, encode command, crop, frame range) for each GIF row of the media list."""
    for line in (MEDIA / "README.md").read_text().splitlines():
        m = re.match(r"\| `([^`]+\.gif)` \|", line)
        if not m:
            continue
        cells = line.split(" | ")
        if len(cells) < 3:
            continue
        spans = re.findall(r"`([^`]+)`", cells[2])
        run = next((s for s in spans if "cargo run" in s), None)
        encode = next((s for s in spans if s.startswith("ffmpeg")), None)
        # A row that crops each kept frame first says so in its words: "-crop WxH+X+Y" or "crops each
        # frame to WxH+X+Y", and may use only a range of frames ("over frames 29 to 68").
        crop = re.search(r"(?:-crop |crops each frame to )(\d+[x×]\d+\+\d+\+\d+)", cells[2])
        span = re.search(r"over frames (\d+) to (\d+)", cells[2])
        if run and encode:
            yield m.group(1), run, encode, crop and crop.group(1), span and (
                int(span.group(1)), int(span.group(2)))


def target_dir():
    env = os.environ.get("CARGO_TARGET_DIR")
    if env:
        return Path(env)
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    import json
    return Path(json.loads(out)["target_directory"])


def thumbs(gif):
    """The frames of a GIF as small grey byte strings."""
    w, h = THUMB
    out = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(gif), "-vf", f"scale={w}:{h},format=gray",
         "-f", "rawvideo", "-"],
        capture_output=True, check=True,
    ).stdout
    size = w * h
    return [out[i:i + size] for i in range(0, len(out) - size + 1, size)][:FRAMES_MAX]


def difference(old, new):
    """(frame count change, mean absolute grey difference over the frames both have)."""
    if not old or not new:
        return abs(len(old) - len(new)), 255.0
    n = min(len(old), len(new))
    total = 0
    for a, b in zip(old[:n], new[:n]):
        total += sum(abs(x - y) for x, y in zip(a, b)) / len(a)
    return abs(len(old) - len(new)), total / n


def record(name, run, encode, crop, span, binaries, scratch):
    """Runs the capture and the encode into `scratch`; returns the new GIF's path."""
    example = re.search(r"--example (\w+)", run).group(1)
    exe = binaries / example
    if not exe.exists():
        raise RuntimeError(f"{exe} is not built: cargo build --release --examples")
    work = scratch / name.removesuffix(".gif")
    work.mkdir(parents=True, exist_ok=True)
    # Every /tmp path of the commands moves into this GIF's own scratch directory.
    def local(text):
        return re.sub(r"/tmp/([\w.-]+)", lambda m: str(work / m.group(1)), text)
    run_cmd = local(run).replace(f"cargo run --release --example {example}", f'"{exe}"')
    for path in set(re.findall(re.escape(str(work)) + r"/[\w.-]+", run_cmd + encode)):
        if "." not in Path(path).name:
            Path(path).mkdir(parents=True, exist_ok=True)
    result = subprocess.run(["bash", "-c", run_cmd], cwd=ROOT, capture_output=True, text=True,
                            timeout=300)
    if result.returncode != 0:
        raise RuntimeError(f"the capture failed: {result.stderr.strip().splitlines()[-1:]}")
    out = work / name
    encode_cmd = local(encode).replace(f"docs/media/{name}", str(out)).replace(
        "ffmpeg ", "ffmpeg -loglevel error -y ", 1)
    if crop:
        # The frames the capture kept, cropped (and only the range the row names) into one directory
        # the encode reads from.
        shots = re.search(r"GOOEY_SHOT_DIR=(\S+)", run_cmd).group(1)
        cropped = work / "crop"
        cropped.mkdir(exist_ok=True)
        frames = sorted(Path(shots).glob("f_*.png"))
        for number, frame in enumerate(frames, start=1):
            if span and not span[0] <= number <= span[1]:
                continue
            subprocess.run(["magick", str(frame), "-crop", crop.replace("×", "x"), "+repage",
                            str(cropped / frame.name)], check=True)
        encode_cmd = re.sub(r"-i '[^']*f_\*\.png'", f"-i '{cropped}/f_*.png'", encode_cmd)
    result = subprocess.run(["bash", "-c", encode_cmd], cwd=ROOT, capture_output=True, text=True,
                            timeout=300)
    if result.returncode != 0 or not out.exists():
        raise RuntimeError(f"ffmpeg failed: {result.stderr.strip().splitlines()[-1:]}")
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("names", nargs="*", help="GIF names without .gif; default every one")
    ap.add_argument("--threshold", type=float, default=1.5,
                    help="mean grey-level difference under which a GIF counts as unchanged")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    if "DISPLAY" not in os.environ:
        sys.exit("no DISPLAY: run it where the loop's Xvfb is (DISPLAY=:93)")
    binaries = target_dir() / "release" / "examples"
    scratch = Path(f"/tmp/regen-gifs-{os.getpid()}")
    scratch.mkdir(parents=True)
    wanted = {n.removesuffix(".gif") + ".gif" for n in args.names}
    status = {}
    try:
        for name, run, encode, crop, span in rows():
            if wanted and name not in wanted:
                continue
            try:
                new = record(name, run, encode, crop, span, binaries, scratch)
                old = MEDIA / name
                frames, mean = difference(thumbs(old) if old.exists() else [], thumbs(new))
                if frames > 2 or mean > args.threshold:
                    if not args.dry_run:
                        shutil.copyfile(new, old)
                    status[name] = f"UPDATED (frames {frames:+}, mean diff {mean:.2f})"
                else:
                    status[name] = f"same (mean diff {mean:.2f})"
            except Exception as err:  # one GIF failing never stops the rest
                status[name] = f"FAILED {err}"
            print(f"{name}: {status[name]}", flush=True)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    failed = [n for n, s in status.items() if s.startswith("FAILED")]
    print(f"{len(status)} GIFs: {sum(s.startswith('UPDATED') for s in status.values())} updated, "
          f"{len(failed)} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
