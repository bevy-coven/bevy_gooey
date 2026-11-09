# Measuring the interface's frame time, honestly

Researched 2026-11-09 (iteration 85). Not a feature: a correction to how this loop measures, forced by
a measurement that turned out to be vacuous. No issue is filed.

The problem. Three passes this session recorded the gallery's `mean_ms` as **33.333** and treated it as
a performance baseline. It is not one: `GOOEY_FIXED_CLOCK=1` installs
`TimeUpdateStrategy::ManualDuration(33_333 µs)`, which makes every frame's *clock* 1/30 s — a
deterministic recording of easing motion — and `capture` divides `Time<Real>`, whose delta under a manual
strategy is that same fixed step. So a fixed-clock run reports a constant, whatever the interface costs.
Measured this pass, same binary, same host, 120 frames each, twice:

| run | `mean_ms` |
|---|---|
| real clock | **30.709**, **31.179** |
| `GOOEY_FIXED_CLOCK=1` | 33.333, 33.333 |

The gate in section 7 of the loop command reads "`mean_ms` from the gallery capture before and after"
and compares it to 0.5 ms. Against a fixed-clock number that comparison can never fail, so it has been
passing vacuously.

## Sources

1. Bevy 0.20, `bevy_time/src/time.rs`, `TimeUpdateStrategy`: `ManualDuration(d)` gives `Time<Real>` the
   same delta every frame and sleeps to match, so anything derived from it under that resource is the
   constant. This crate's `docs/research/motion.md` already relies on the same fact for deterministic
   motion tests.
2. *Good Game Feel* — Hicks et al., 2018, DiGRA, doi:10.26503/dl.v2018i1.936 (craft): feel is a
   property of the delivered frame, so the number that matters is the one the player's machine
   produces, not the one a recording harness pins.
3. Rune's own capture harness (the owner's reference): it records with a pinned clock for determinism
   and takes frame time from a separate, unpinned pass — the two questions are asked separately, which
   is the model adopted here.

## Model adopted

- **Determinism and measurement are two runs.** `GOOEY_FIXED_CLOCK=1` stays exactly as it is for every
  GIF, because a recording must be reproducible. The performance number comes from a run **without**
  it, which is the only run whose `mean_ms` varies.
- **The gallery says so itself**: under the fixed clock the capture prints
  `mean_ms 33.333 (fixed clock: not a measurement)` so the next pass cannot mistake one for the other.
- **Every pass's log quotes the unpinned figure** and says which run it came from.

## Validation targets

- A fixed-clock run's line carries the warning suffix; an unpinned run's does not.
- Two unpinned runs of one binary differ (they are measurements), and two pinned runs do not (they are
  not).

## Deviations

- The gate's 0.5 ms tolerance is unchanged; only the input to it is now a real measurement.

## Pending index

- Nothing new: this is a harness correction, not a research claim.
