# A double click's interval is a wall clock; the test must not be

Researched 2026-11-09 (iteration 86). A correction to this loop's own test harness, forced by a flake it
had already logged twice. No issue is filed.

The problem. `tests/split.rs::a_double_click_collapses_the_first_pane_and_a_second_gives_it_back` ended
with the assertion that a *fresh* pair of clicks gives the collapsed pane back, after
`std::thread::sleep(600 ms)` so the pair does not continue the earlier burst. Both clocks the behaviour
rests on are wall clocks: this crate's `DoublePress` pairs on `Time<Real>` within
`PointerSettings::double_click_seconds` (0.5 s), and Bevy's picking counts on `Instant::now()` against
`PickingSettings::multi_click_interval`. On a host at load 46 the suite failed this test and its
neighbour; both pass alone and at load 25 (iterations 80 and 85 both logged the same flake). A gate that
can fail on load is not a gate.

## Sources

1. Bevy 0.20, `bevy_picking/src/events.rs:835,1056`: `let now = Instant::now();` … `now - *last_click <=
   picking_settings.multi_click_interval` — a wall clock, immune to `TimeUpdateStrategy`.
2. This crate, `src/clicks.rs`: `DoublePress` keeps its own `Presses` on `Time<Real>`, "the one place two
   presses become a pair", with `PointerSettings::double_click_seconds`. `Time<Real>` *is* steppable: it
   is a `Time` like any other and `advance_by` moves it.
3. *A Survey on Metamorphic Testing* — Segura, Fraser, Sánchez & Ruiz-Cortés, IEEE TSE 2016,
   `10.1109/TSE.2016.2532875` (the corpus's testing stem; `bevy_cauldron` cites it for the same
   reason): a test is only evidence if the relation it checks holds for the same reason under a
   different run, which a wall-clock sleep does not give.

## Craft principle

A test that measures elapsed wall time is measuring the machine, not the interface. Where the behaviour
is "two presses within the interval", the interval must be advanced by the test, not waited out.

## How shipping games do it

Rune's own interaction tests step a virtual clock rather than sleeping; the harness owns time so a
loaded CI box and a quiet laptop answer the same.

## Model adopted

- **No sleep.** The test asks for the interval (`PointerSettings::double_click_seconds`), installs a
  fresh `Time<Real>`, advances it four intervals, moves the pointer off the divider for one frame, and
  then clicks twice. The pair that follows is a fresh one on both clocks.
- **The assertion keeps its teeth.** Mutation-checked: delete the second click and the test fails
  ("and again gives it back"), which a zero `multi_click_interval` did *not* do — the first attempt set
  that to `Duration::ZERO` and the mutation passed, because it made every press a singleton and so made
  the assertion vacuous. Reverted.
- **`tests/double_press.rs` keeps its wall-clock cases**, because it tests the interval itself, including
  a press held past it; that is the one place real elapsed time is the subject.

## Validation targets

- `tests/split.rs` contains no `thread::sleep` (`grep -c thread::sleep tests/*.rs` is 0).
- Deleting the second click of the fresh pair fails the test.
- The full suite passes on a host at load 6 and at load 20.

## Deviations

- None: the test's behaviour and assertions are unchanged; only the waiting is.

## Pending index

- Nothing new; the corpus's testing stem covered it.
