# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 23:06Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 52 | 2026-11-01T14:34Z | 8 | 0 | 5 | v0.11.0 | 7 | 1 | - |
| 53 | 2026-11-01T15:21Z | 8 | 0 | 6 | v0.12.0 | 9 | 1 | - |
| 54 | 2026-11-01T15:54Z | 9 | 0 | 8 | v0.13.0 | 10 | 1 | - |
| 55 | 2026-11-01T16:34Z | 7 | 0 | 8 | v0.14.0 | 10 | 1 | - |
| 56 | 2026-11-01T17:07Z | 7 | 0 | 6 | v0.15.0 | 10 | 1 | - |
| 57 | 2026-11-01T18:21Z | 7 | 0 | 7 | v0.16.0 | 9 | 1 | - |
| 58 | 2026-11-01T18:58Z | 7 | 0 | 8 | v0.17.0 | 9 | 1 | - |
| 59 | 2026-11-01T20:09Z | 8 | 0 | 9 | v0.18.0 | 9 | 1 | - |
| 60 | 2026-11-01T21:02Z | 10 | 0 | 7 | v0.19.0 | 8 | 1 | - |
| 61 | 2026-11-01T21:47Z | 10 | 0 | 8 | v0.20.0 | 9 | 1 | - |
| 62 | 2026-11-01T21:59Z | 4 | 0 | 1 | v0.20.1 | 1 | 0 | demo? |
| 63 | 2026-11-01T23:06Z | 7 | 0 | 7 | v0.21.0 | 10 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             3      40.2        42.4
```

## The last 12 passes

```text
12 pass(es), 4279 steps, 9.0 h of pass time

category                       hours  share  steps  mean s
model                           4.59  50.8%   1930     8.6
wait: gate                      1.56  17.3%     17   329.8
wait: other                     0.66   7.3%     23   102.8
cargo test                      0.60   6.6%    149    14.5
cargo build                     0.40   4.4%     93    15.5
shell                           0.36   4.0%    714     1.8
phase mark                      0.24   2.6%    105     8.1
wait: build                     0.16   1.7%      3   186.7
read files                      0.07   0.8%    534     0.5
remote wait                     0.06   0.7%      9    25.1
push gate                       0.06   0.7%     33     6.5
wait: tests                     0.06   0.6%      3    68.7
github                          0.05   0.6%     55     3.4
cargo clippy                    0.05   0.5%     24     7.1
cargo publish                   0.04   0.4%     10    12.7

phase                          hours  share  steps  mean s
build                           2.51  27.8%   1411     6.4
gate                            2.48  27.5%    517    17.3
prove                           2.43  26.9%   1351     6.5
research                        0.51   5.7%    266     7.0
ship                            0.33   3.7%    211     5.6
release                         0.26   2.9%    168     5.6
choose                          0.25   2.7%    140     6.4
log                             0.13   1.4%    105     4.4
orient                          0.12   1.3%     91     4.7
bugs                            0.01   0.1%      6     3.9

slowest single steps
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   62     282s  tool  wait: other          {}
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   53     220s  tool  wait: other          Waiting for third perf run
  pass   56     203s  tool  wait: other          {}
  pass   59     201s  tool  wait: gate           Waiting for the full gate
  pass   59     185s  tool  wait: gate           Waiting for the gate
```

## Every pass recorded

```text
32 pass(es), 10636 steps, 32.8 h of pass time

category                       hours  share  steps  mean s
model                          12.50  38.1%   4719     9.5
wait: gate                      9.14  27.9%     60   548.6
wait: tests                     2.78   8.5%     30   334.0
wait: other                     1.75   5.3%     61   103.4
shell                           1.42   4.3%   1569     3.2
cargo test                      1.22   3.7%    298    14.7
cargo build                     0.81   2.5%    168    17.3
wait: clip                      0.78   2.4%     29    97.0
wait: build                     0.77   2.4%     15   185.6
phase mark                      0.37   1.1%    167     7.9
remote wait                     0.21   0.6%     22    33.6
eval                            0.17   0.5%      9    68.4
read files                      0.16   0.5%   1533     0.4
github                          0.11   0.3%    109     3.5
edit files                      0.10   0.3%   1108     0.3

phase                          hours  share  steps  mean s
orient                         18.84  57.4%   4294    15.8
prove                           4.15  12.7%   2168     6.9
gate                            3.92  12.0%    687    20.6
build                           3.28  10.0%   1890     6.3
research                        0.79   2.4%    451     6.3
choose                          0.58   1.8%    297     7.1
ship                            0.48   1.5%    266     6.5
release                         0.45   1.4%    323     5.0
log                             0.21   0.6%    167     4.6
bugs                            0.09   0.3%     40     8.1

slowest single steps
  pass    7    2558s  bg    shell                bg_16 completed: /tmp/g11/gate.sh > /tmp/g11/gate.out 2>&1; tail -20 /tmp/g11/gate/summary
  pass    7    2114s  model model                
  pass    7    1826s  tool  wait: gate           Waiting for the gate to finish
  pass    7    1818s  tool  wait: tests          Waiting for toast tests rebuild
  pass    8    1800s  tool  wait: gate           Waiting for the gate to finish
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass    7    1612s  tool  wait: tests          Waiting for test binary build
  pass    7    1611s  tool  wait: tests          Waiting for test binary link
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass    9    1481s  tool  wait: gate           Waiting for gate retry
  pass    7    1451s  model model                
  pass    4    1451s  tool  wait: gate           Waiting for gate step 1 retry
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   57    1434s  tool  wait: gate           Waiting for the gate
```

