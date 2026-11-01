# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 20:09Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 43 | 2026-11-01T06:52Z | 6 | 0 | 6 | v0.7.1 | 10 | 2 | - |
| 44 | 2026-11-01T08:41Z | 8 | 0 | 6 | v0.8.0 | 13 | 4 | - |
| 50 | 2026-11-01T13:40Z | 9 | 0 | 3 | v0.9.0 | 7 | 1 | - |
| 51 | 2026-11-01T14:03Z | 7 | 0 | 4 | v0.10.0 | 10 | 2 | - |
| 52 | 2026-11-01T14:34Z | 8 | 0 | 5 | v0.11.0 | 7 | 1 | - |
| 53 | 2026-11-01T15:21Z | 8 | 0 | 6 | v0.12.0 | 9 | 1 | - |
| 54 | 2026-11-01T15:54Z | 9 | 0 | 8 | v0.13.0 | 10 | 1 | - |
| 55 | 2026-11-01T16:34Z | 7 | 0 | 8 | v0.14.0 | 10 | 1 | - |
| 56 | 2026-11-01T17:07Z | 7 | 0 | 6 | v0.15.0 | 10 | 1 | - |
| 57 | 2026-11-01T18:21Z | 7 | 0 | 7 | v0.16.0 | 9 | 1 | - |
| 58 | 2026-11-01T18:58Z | 7 | 0 | 8 | v0.17.0 | 9 | 1 | - |
| 59 | 2026-11-01T20:09Z | 8 | 0 | 9 | v0.18.0 | 9 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            10      40.7        34.7
```

## The last 12 passes

```text
12 pass(es), 4452 steps, 9.9 h of pass time

category                       hours  share  steps  mean s
model                           4.53  45.7%   1996     8.2
wait: gate                      2.03  20.5%     18   405.8
wait: other                     0.93   9.3%     32   104.1
cargo test                      0.61   6.2%    149    14.8
shell                           0.45   4.5%    702     2.3
cargo build                     0.37   3.7%     80    16.6
phase mark                      0.25   2.5%    116     7.8
wait: tests                     0.18   1.9%      7    94.7
wait: clip                      0.11   1.1%      4   100.7
read files                      0.07   0.7%    586     0.5
github                          0.06   0.6%     56     4.1
remote wait                     0.05   0.5%      9    19.3
push gate                       0.05   0.5%     34     5.1
cargo publish                   0.04   0.4%     13    11.2
clip record/render              0.04   0.4%     32     4.1

phase                          hours  share  steps  mean s
prove                           3.01  30.4%   1527     7.1
gate                            2.54  25.6%    344    26.6
build                           2.24  22.6%   1294     6.2
research                        0.66   6.6%    389     6.1
ship                            0.43   4.3%    226     6.8
choose                          0.41   4.1%    187     7.9
release                         0.23   2.4%    184     4.6
orient                          0.23   2.3%    162     5.0
log                             0.14   1.4%    112     4.5
bugs                            0.03   0.3%     12    10.0

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   43     309s  tool  wait: clip           Waiting for pixel-identity captures
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   44     279s  tool  wait: tests          Waiting for test binaries
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   44     252s  tool  wait: other          Waiting for benchmark
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   44     221s  tool  wait: other          Waiting for repo clones
  pass   53     220s  tool  wait: other          Waiting for third perf run
  pass   50     216s  tool  wait: other          Waiting for perf runs
  pass   56     203s  tool  wait: other          {}
```

## Every pass recorded

```text
28 pass(es), 9371 steps, 29.9 h of pass time

category                       hours  share  steps  mean s
model                          11.09  37.1%   4154     9.6
wait: gate                      8.48  28.4%     55   555.4
wait: tests                     2.78   9.3%     29   344.6
wait: other                     1.59   5.3%     56   102.4
shell                           1.35   4.5%   1372     3.6
cargo test                      1.07   3.6%    266    14.4
wait: clip                      0.78   2.6%     29    97.0
cargo build                     0.71   2.4%    148    17.3
wait: build                     0.63   2.1%     13   174.1
phase mark                      0.26   0.9%    134     7.0
remote wait                     0.19   0.6%     20    34.0
eval                            0.17   0.6%      9    68.4
read files                      0.15   0.5%   1344     0.4
edit files                      0.10   0.3%    952     0.4
github                          0.10   0.3%     94     3.7

phase                          hours  share  steps  mean s
orient                         18.82  62.9%   4270    15.9
prove                           3.42  11.4%   1799     6.8
gate                            3.02  10.1%    472    23.0
build                           2.42   8.1%   1438     6.1
research                        0.66   2.2%    389     6.1
choose                          0.53   1.8%    263     7.3
ship                            0.45   1.5%    244     6.6
release                         0.31   1.0%    255     4.4
log                             0.20   0.7%    156     4.6
bugs                            0.08   0.3%     34     8.9

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

