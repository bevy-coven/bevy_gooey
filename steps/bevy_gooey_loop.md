# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 14:34Z.

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
| 52 | 2026-11-01T14:34Z | 8 | 0 | 5 | v0.11.0 | 7 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             3      24.6        22.6
```

## The last 12 passes

```text
12 pass(es), 4193 steps, 10.8 h of pass time

category                       hours  share  steps  mean s
model                           4.11  38.0%   1840     8.0
wait: gate                      3.77  34.9%     19   714.6
wait: other                     0.73   6.7%     24   109.0
shell                           0.51   4.7%    600     3.0
wait: tests                     0.41   3.8%     11   133.0
cargo test                      0.37   3.4%     90    14.9
cargo build                     0.23   2.1%     40    20.6
wait: clip                      0.18   1.7%      9    73.9
phase mark                      0.15   1.4%     71     7.7
clip record/render              0.07   0.6%     41     5.8
edit files                      0.05   0.5%    454     0.4
wait: build                     0.05   0.4%      6    28.3
github                          0.04   0.3%     35     3.8
read files                      0.03   0.3%    698     0.2
push gate                       0.03   0.3%     33     3.2

phase                          hours  share  steps  mean s
orient                          5.25  48.6%   1798    10.5
prove                           1.75  16.2%    821     7.7
gate                            1.49  13.8%    183    29.3
build                           1.08  10.0%    654     5.9
choose                          0.37   3.4%    173     7.7
research                        0.31   2.9%    201     5.6
release                         0.21   1.9%    174     4.2
ship                            0.19   1.7%     60    11.1
log                             0.09   0.9%     74     4.5
bugs                            0.08   0.8%     34     8.9

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   10    1312s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   39     959s  tool  wait: gate           Waiting for the final gate
  pass   10     836s  tool  wait: tests          Waiting for the snapping test run
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   40     653s  tool  wait: gate           Waiting for the gate to finish
  pass   12     562s  tool  wait: gate           Waiting for the gate
  pass   39     443s  tool  wait: gate           Waiting for gate to finish
  pass   43     309s  tool  wait: clip           Waiting for pixel-identity captures
  pass   42     306s  model model                
  pass   12     302s  tool  wait: gate           Waiting for the gate to finish
```

## Every pass recorded

```text
21 pass(es), 6627 steps, 24.3 h of pass time

category                       hours  share  steps  mean s
model                           8.14  33.5%   2906    10.1
wait: gate                      7.66  31.5%     45   612.8
wait: tests                     2.73  11.2%     27   363.5
wait: other                     1.18   4.9%     41   103.9
shell                           1.11   4.6%    893     4.5
wait: clip                      0.76   3.1%     28    98.0
cargo test                      0.63   2.6%    152    14.9
wait: build                     0.62   2.5%     12   185.3
cargo build                     0.43   1.8%     81    19.1
eval                            0.17   0.7%      9    68.4
phase mark                      0.15   0.6%     71     7.7
remote wait                     0.14   0.6%     13    39.5
edit files                      0.10   0.4%    685     0.5
read files                      0.09   0.4%   1036     0.3
github                          0.07   0.3%     60     4.0

phase                          hours  share  steps  mean s
orient                         18.72  77.1%   4211    16.0
prove                           1.75   7.2%    821     7.7
gate                            1.49   6.1%    183    29.3
build                           1.08   4.4%    654     5.9
choose                          0.37   1.5%    173     7.7
research                        0.31   1.3%    201     5.6
release                         0.21   0.8%    174     4.2
ship                            0.19   0.8%     60    11.1
log                             0.09   0.4%     74     4.5
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
  pass    2    1393s  tool  wait: build          Waiting for baseline build
```

