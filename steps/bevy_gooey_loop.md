# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 06:52Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 0 | 2026-11-01T00:31Z | 16 | 0 | 5 | - | 12 | 2 | release |
| 39 | 2026-11-01T03:31Z | 10 | 0 | 7 | v0.5.0 | 10 | 2 | - |
| 40 | 2026-11-01T04:18Z | 8 | 0 | 7 | v0.6.0 | 12 | 3 | - |
| 42 | 2026-11-01T05:37Z | 8 | 0 | 8 | v0.7.0 | 15 | 6 | - |
| 43 | 2026-11-01T06:52Z | 6 | 0 | 6 | v0.7.1 | 10 | 2 | - |

## The last 12 passes

```text
12 pass(es), 4481 steps, 16.8 h of pass time

category                       hours  share  steps  mean s
model                           6.36  37.8%   1956    11.7
wait: gate                      5.20  30.9%     26   720.1
wait: tests                     2.03  12.0%     15   486.1
shell                           0.71   4.2%    602     4.2
cargo test                      0.54   3.2%    105    18.3
wait: other                     0.52   3.1%     19    98.6
wait: clip                      0.49   2.9%     18    97.8
cargo build                     0.26   1.5%     53    17.4
eval                            0.16   0.9%      6    95.7
remote wait                     0.11   0.7%     11    37.2
phase mark                      0.08   0.5%     33     9.2
read files                      0.07   0.4%    766     0.3
edit files                      0.06   0.4%    475     0.4
clip record/render              0.04   0.2%     25     6.0
web                             0.04   0.2%     14     9.3

phase                          hours  share  steps  mean s
orient                         14.31  85.0%   3250    15.8
prove                           1.23   7.3%    516     8.6
gate                            0.49   2.9%    130    13.5
build                           0.33   2.0%    211     5.6
choose                          0.14   0.8%     84     5.9
release                         0.11   0.7%    100     4.0
research                        0.10   0.6%     68     5.5
log                             0.07   0.4%     48     5.0
bugs                            0.05   0.3%     22     8.3
ship                            0.02   0.1%     22     4.0

slowest single steps
  pass    7    2558s  bg    shell                bg_16 completed: /tmp/g11/gate.sh > /tmp/g11/gate.out 2>&1; tail -20 /tmp/g11/gate/summary
  pass    7    2114s  model model                
  pass    7    1826s  tool  wait: gate           Waiting for the gate to finish
  pass    7    1818s  tool  wait: tests          Waiting for toast tests rebuild
  pass    8    1800s  tool  wait: gate           Waiting for the gate to finish
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass    7    1612s  tool  wait: tests          Waiting for test binary build
  pass    7    1611s  tool  wait: tests          Waiting for test binary link
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass    9    1481s  tool  wait: gate           Waiting for gate retry
  pass    7    1451s  model model                
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   10    1312s  tool  wait: gate           Waiting for the gate
  pass    7    1113s  model model                
  pass   39     959s  tool  wait: gate           Waiting for the final gate
```

## Every pass recorded

```text
17 pass(es), 5415 steps, 21.2 h of pass time

category                       hours  share  steps  mean s
model                           7.08  33.4%   2372    10.8
wait: gate                      6.68  31.4%     38   632.4
wait: tests                     2.59  12.2%     23   406.0
shell                           0.95   4.5%    727     4.7
wait: other                     0.78   3.7%     28   100.6
wait: clip                      0.76   3.6%     28    98.0
wait: build                     0.60   2.8%      9   238.8
cargo test                      0.59   2.8%    132    16.1
cargo build                     0.36   1.7%     70    18.3
eval                            0.17   0.8%      9    68.4
remote wait                     0.14   0.7%     13    39.5
edit files                      0.09   0.4%    550     0.6
phase mark                      0.08   0.4%     33     9.2
read files                      0.08   0.4%    847     0.3
homestill                       0.05   0.2%     22     7.5

phase                          hours  share  steps  mean s
orient                         18.70  88.1%   4177    16.1
prove                           1.23   5.8%    516     8.6
gate                            0.49   2.3%    130    13.5
build                           0.33   1.5%    211     5.6
choose                          0.14   0.6%     84     5.9
release                         0.11   0.5%    100     4.0
research                        0.10   0.5%     68     5.5
log                             0.07   0.3%     48     5.0
bugs                            0.05   0.2%     22     8.3
ship                            0.02   0.1%     22     4.0

slowest single steps
  pass    7    2558s  bg    shell                bg_16 completed: /tmp/g11/gate.sh > /tmp/g11/gate.out 2>&1; tail -20 /tmp/g11/gate/summary
  pass    7    2114s  model model                
  pass    7    1826s  tool  wait: gate           Waiting for the gate to finish
  pass    7    1818s  tool  wait: tests          Waiting for toast tests rebuild
  pass    8    1800s  tool  wait: gate           Waiting for the gate to finish
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass    7    1612s  tool  wait: tests          Waiting for test binary build
  pass    7    1611s  tool  wait: tests          Waiting for test binary link
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass    9    1481s  tool  wait: gate           Waiting for gate retry
  pass    7    1451s  model model                
  pass    4    1451s  tool  wait: gate           Waiting for gate step 1 retry
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass    2    1393s  tool  wait: build          Waiting for baseline build
  pass   10    1312s  tool  wait: gate           Waiting for the gate
```

