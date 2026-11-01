# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 14:03Z.

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
| 51 | 2026-11-01T14:03Z | 7 | 0 | 4 | v0.10.0 | 10 | 2 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             2      21.4        21.4
```

## The last 12 passes

```text
12 pass(es), 4281 steps, 11.5 h of pass time

category                       hours  share  steps  mean s
wait: gate                      4.35  37.7%     23   681.6
model                           4.27  37.0%   1881     8.2
wait: other                     0.69   5.9%     22   112.1
shell                           0.52   4.5%    610     3.1
wait: tests                     0.41   3.5%     11   133.0
cargo test                      0.38   3.3%     94    14.7
cargo build                     0.23   2.0%     39    21.2
wait: clip                      0.18   1.6%      9    73.9
phase mark                      0.13   1.1%     62     7.5
clip record/render              0.06   0.6%     37     6.2
edit files                      0.05   0.5%    466     0.4
wait: build                     0.05   0.4%      6    28.3
remote wait                     0.04   0.4%      6    24.8
read files                      0.03   0.3%    717     0.2
push gate                       0.03   0.3%     34     3.1

phase                          hours  share  steps  mean s
orient                          6.49  56.2%   2146    10.9
prove                           1.72  14.9%    817     7.6
gate                            1.44  12.5%    170    30.5
build                           0.77   6.7%    479     5.8
choose                          0.34   2.9%    157     7.7
research                        0.28   2.4%    185     5.4
release                         0.18   1.6%    155     4.3
ship                            0.15   1.3%     55     9.7
bugs                            0.08   0.7%     34     8.9
log                             0.08   0.7%     62     4.8

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass    9    1481s  tool  wait: gate           Waiting for gate retry
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   10    1312s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   39     959s  tool  wait: gate           Waiting for the final gate
  pass   10     836s  tool  wait: tests          Waiting for the snapping test run
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   40     653s  tool  wait: gate           Waiting for the gate to finish
  pass   12     562s  tool  wait: gate           Waiting for the gate
  pass    9     450s  tool  wait: gate           Waiting for gate to finish
  pass   39     443s  tool  wait: gate           Waiting for gate to finish
  pass    9     333s  tool  wait: gate           Waiting for the merged-tree gate
```

## Every pass recorded

```text
20 pass(es), 6357 steps, 23.8 h of pass time

category                       hours  share  steps  mean s
model                           7.91  33.3%   2789    10.2
wait: gate                      7.59  31.9%     43   635.2
wait: tests                     2.73  11.5%     27   363.5
wait: other                     1.10   4.6%     38   103.8
shell                           1.06   4.5%    855     4.5
wait: clip                      0.76   3.2%     28    98.0
cargo test                      0.62   2.6%    149    15.0
wait: build                     0.62   2.6%     12   185.3
cargo build                     0.41   1.7%     75    19.5
eval                            0.17   0.7%      9    68.4
remote wait                     0.14   0.6%     13    39.5
phase mark                      0.13   0.5%     62     7.5
edit files                      0.10   0.4%    650     0.5
read files                      0.09   0.4%    999     0.3
clip record/render              0.06   0.3%     37     6.2

phase                          hours  share  steps  mean s
orient                         18.72  78.8%   4203    16.0
prove                           1.72   7.2%    817     7.6
gate                            1.44   6.1%    170    30.5
build                           0.77   3.3%    479     5.8
choose                          0.34   1.4%    157     7.7
research                        0.28   1.2%    185     5.4
release                         0.18   0.8%    155     4.3
ship                            0.15   0.6%     55     9.7
bugs                            0.08   0.4%     34     8.9
log                             0.08   0.3%     62     4.8

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

