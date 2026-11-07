# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-07 06:16Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 80 | 2026-11-02T17:23Z | 5 | 0 | 4 | v0.31.1 | 9 | 2 | - |
| 81 | 2026-11-02T17:37Z | 8 | 0 | 4 | v0.31.2 | 4 | 0 | demo? |
| 82 | 2026-11-02T17:53Z | 6 | 0 | 5 | v0.31.3 | 6 | 1 | - |
| 83 | 2026-11-02T18:55Z | 7 | 0 | 5 | v0.32.0 | 6 | 1 | - |
| 84 | 2026-11-03T11:52Z | 5 | 0 | 2 | v0.32.1 | 2 | 0 | demo? |
| 85 | 2026-11-03T12:05Z | 7 | 0 | 5 | v0.32.2 | 4 | 0 | demo? |
| 86 | 2026-11-03T12:18Z | 7 | 0 | 3 | v0.32.3 | 4 | 0 | demo? |
| 87 | 2026-11-03T12:45Z | 8 | 0 | 4 | v0.32.4 | 7 | 1 | - |
| 88 | 2026-11-03T13:17Z | 9 | 0 | 6 | v0.33.0 | 7 | 1 | - |
| 89 | 2026-11-04T12:18Z | 6 | 0 | 4 | v0.34.0 | 2 | 0 | demo? |
| 90 | 2026-11-04T13:05Z | 7 | 0 | 8 | v0.35.0 | 7 | 1 | - |
| 91 | 2026-11-07T06:16Z | 11 | 0 | 7 | v0.38.0 | 8 | 2 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            31      33.3        29.7
```

## The last 12 passes

```text
12 pass(es), 2150 steps, 5.8 h of pass time

category                       hours  share  steps  mean s
wait: other                     2.38  41.0%     95    90.1
model                           1.81  31.2%   1011     6.4
cargo test                      0.43   7.4%     61    25.4
wait: build                     0.25   4.4%      2   454.9
cargo build                     0.21   3.6%     37    20.2
phase mark                      0.17   3.0%     96     6.5
wait: gate                      0.13   2.2%      3   156.3
shell                           0.13   2.2%    422     1.1
cargo publish                   0.07   1.3%     13    20.5
cargo clippy                    0.05   0.9%     12    16.4
push gate                       0.04   0.7%     20     7.0
web                             0.03   0.6%     12    10.0
clip record/render              0.02   0.4%     12     6.6
wait: tests                     0.02   0.4%      1    78.1
remote wait                     0.02   0.3%      1    60.0

phase                          hours  share  steps  mean s
gate                            1.92  33.1%    183    37.7
prove                           1.46  25.2%    618     8.5
build                           1.37  23.6%    603     8.2
research                        0.43   7.3%    258     5.9
release                         0.21   3.7%     87     8.9
choose                          0.12   2.1%    162     2.7
orient                          0.10   1.8%    102     3.6
ship                            0.10   1.7%     68     5.3
bugs                            0.06   1.0%     37     5.6
log                             0.03   0.6%     26     4.5

slowest single steps
  pass   83    1800s  tool  wait: other          {}
  pass   90     910s  tool  wait: build          Waiting for example builds
  pass   87     732s  tool  wait: other          {}
  pass   83     537s  tool  wait: other          {}
  pass   90     417s  tool  wait: other          Waiting for mutation run
  pass   88     298s  tool  wait: other          {}
  pass   91     292s  tool  wait: other          {}
  pass   89     258s  tool  wait: other          {}
  pass   89     251s  tool  wait: other          {}
  pass   91     237s  bg    shell                bg_1 completed: cat >> tests/serde.rs <<'E' #[test] fn a_skins_motion_palette_is_data_and_
  pass   83     229s  tool  wait: other          {}
  pass   88     222s  tool  wait: other          {}
  pass   91     216s  tool  wait: other          {}
  pass   86     206s  tool  wait: other          {}
  pass   91     203s  tool  wait: other          {}
```

## Every pass recorded

```text
60 pass(es), 17128 steps, 48.1 h of pass time

category                       hours  share  steps  mean s
model                          19.04  39.6%   7775     8.8
wait: gate                      9.91  20.6%     74   482.3
wait: other                     5.30  11.0%    209    91.3
wait: tests                     2.85   5.9%     34   302.0
cargo test                      2.55   5.3%    516    17.8
shell                           1.94   4.0%   2857     2.4
cargo build                     1.59   3.3%    336    17.1
wait: build                     1.16   2.4%     21   199.4
phase mark                      0.94   1.9%    434     7.8
wait: clip                      0.79   1.6%     30    94.9
remote wait                     0.31   0.7%     37    30.4
read files                      0.26   0.5%   2092     0.4
cargo publish                   0.20   0.4%     59    12.0
github                          0.19   0.4%    177     3.9
cargo clippy                    0.19   0.4%     77     8.9

phase                          hours  share  steps  mean s
orient                         19.06  39.6%   4519    15.2
prove                           8.36  17.4%   4102     7.3
gate                            7.92  16.5%   1200    23.8
build                           7.29  15.1%   3867     6.8
research                        1.75   3.6%   1084     5.8
choose                          1.28   2.7%    784     5.9
release                         1.04   2.2%    600     6.2
ship                            0.84   1.7%    495     6.1
log                             0.39   0.8%    297     4.7
bugs                            0.18   0.4%    102     6.2

slowest single steps
  pass    7    2558s  bg    shell                bg_16 completed: /tmp/g11/gate.sh > /tmp/g11/gate.out 2>&1; tail -20 /tmp/g11/gate/summary
  pass    7    2114s  model model                
  pass    7    1826s  tool  wait: gate           Waiting for the gate to finish
  pass    7    1818s  tool  wait: tests          Waiting for toast tests rebuild
  pass    8    1800s  tool  wait: gate           Waiting for the gate to finish
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   83    1800s  tool  wait: other          {}
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass    7    1612s  tool  wait: tests          Waiting for test binary build
  pass    7    1611s  tool  wait: tests          Waiting for test binary link
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass    9    1481s  tool  wait: gate           Waiting for gate retry
  pass    7    1451s  model model                
  pass    4    1451s  tool  wait: gate           Waiting for gate step 1 retry
  pass   21    1449s  tool  wait: gate           Waiting for the gate
```

