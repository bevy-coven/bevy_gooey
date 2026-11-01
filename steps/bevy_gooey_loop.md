# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 13:40Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 0 | 2026-11-01T00:31Z | 16 | 0 | 5 | - | 12 | 2 | release |
| 39 | 2026-11-01T03:31Z | 10 | 0 | 7 | v0.5.0 | 10 | 2 | - |
| 40 | 2026-11-01T04:18Z | 8 | 0 | 7 | v0.6.0 | 12 | 3 | - |
| 42 | 2026-11-01T05:37Z | 8 | 0 | 8 | v0.7.0 | 15 | 6 | - |
| 43 | 2026-11-01T06:52Z | 6 | 0 | 6 | v0.7.1 | 10 | 2 | - |
| 44 | 2026-11-01T08:41Z | 8 | 0 | 6 | v0.8.0 | 13 | 4 | - |
| 50 | 2026-11-01T13:40Z | 9 | 0 | 3 | v0.9.0 | 7 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             1      20.1        20.1
```

## The last 12 passes

```text
12 pass(es), 4256 steps, 12.8 h of pass time

category                       hours  share  steps  mean s
wait: gate                      5.40  42.1%     27   720.5
model                           4.37  34.1%   1864     8.4
wait: other                     0.71   5.5%     21   121.8
shell                           0.56   4.4%    599     3.4
wait: tests                     0.41   3.2%     12   122.3
cargo test                      0.40   3.1%     97    14.9
wait: clip                      0.26   2.0%     12    77.1
cargo build                     0.22   1.7%     39    20.1
phase mark                      0.11   0.8%     52     7.4
clip record/render              0.06   0.5%     32     7.0
edit files                      0.05   0.4%    447     0.4
wait: build                     0.05   0.4%      7    24.2
remote wait                     0.04   0.3%      6    24.8
read files                      0.03   0.3%    710     0.2
push gate                       0.03   0.2%     36     2.8

phase                          hours  share  steps  mean s
orient                          8.15  63.5%   2394    12.3
prove                           1.53  11.9%    641     8.6
gate                            1.38  10.8%    156    31.9
build                           0.71   5.6%    447     5.7
choose                          0.32   2.5%    139     8.2
research                        0.27   2.1%    173     5.5
release                         0.16   1.2%    137     4.2
ship                            0.15   1.1%     53     9.9
bugs                            0.08   0.7%     34     8.9
log                             0.08   0.6%     60     4.9

slowest single steps
  pass    8    1800s  tool  wait: gate           Waiting for the gate to finish
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass    9    1481s  tool  wait: gate           Waiting for gate retry
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   10    1312s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   39     959s  tool  wait: gate           Waiting for the final gate
  pass    8     930s  tool  wait: gate           Waiting for merged-tree gate
  pass   10     836s  tool  wait: tests          Waiting for the snapping test run
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   40     653s  tool  wait: gate           Waiting for the gate to finish
  pass    8     586s  tool  wait: gate           Waiting for the gate to finish
  pass   12     562s  tool  wait: gate           Waiting for the gate
```

## Every pass recorded

```text
19 pass(es), 6074 steps, 23.4 h of pass time

category                       hours  share  steps  mean s
model                           7.69  32.9%   2657    10.4
wait: gate                      7.59  32.4%     43   635.2
wait: tests                     2.73  11.7%     27   363.5
shell                           1.05   4.5%    807     4.7
wait: other                     1.00   4.3%     33   109.4
wait: clip                      0.76   3.3%     28    98.0
cargo test                      0.62   2.7%    146    15.3
wait: build                     0.62   2.6%     12   185.3
cargo build                     0.39   1.7%     73    19.2
eval                            0.17   0.7%      9    68.4
remote wait                     0.14   0.6%     13    39.5
phase mark                      0.11   0.5%     52     7.4
edit files                      0.10   0.4%    627     0.6
read files                      0.09   0.4%    957     0.3
clip record/render              0.06   0.3%     32     7.0

phase                          hours  share  steps  mean s
orient                         18.71  80.0%   4195    16.1
prove                           1.53   6.5%    641     8.6
gate                            1.38   5.9%    156    31.9
build                           0.71   3.0%    447     5.7
choose                          0.32   1.4%    139     8.2
research                        0.27   1.1%    173     5.5
release                         0.16   0.7%    137     4.2
ship                            0.15   0.6%     53     9.9
bugs                            0.08   0.4%     34     8.9
log                             0.08   0.3%     60     4.9

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
  pass    2    1393s  tool  wait: build          Waiting for baseline build
```

