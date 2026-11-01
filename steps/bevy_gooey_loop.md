# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 08:41Z.

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

## The last 12 passes

```text
12 pass(es), 4481 steps, 17.4 h of pass time

category                       hours  share  steps  mean s
model                           6.29  36.2%   1952    11.6
wait: gate                      5.88  33.8%     27   783.9
wait: tests                     2.15  12.3%     18   429.1
shell                           0.71   4.1%    614     4.1
wait: other                     0.68   3.9%     23   106.5
cargo test                      0.47   2.7%    103    16.4
wait: clip                      0.29   1.7%     13    80.9
cargo build                     0.22   1.3%     45    17.7
eval                            0.16   0.9%      6    95.7
phase mark                      0.10   0.6%     43     8.5
remote wait                     0.06   0.4%      8    28.6
clip record/render              0.06   0.4%     31     7.2
edit files                      0.06   0.3%    473     0.5
read files                      0.05   0.3%    768     0.2
github                          0.04   0.2%     24     6.1

phase                          hours  share  steps  mean s
orient                         13.06  75.1%   2803    16.8
gate                            1.38   8.0%    156    31.9
prove                           1.32   7.6%    520     9.1
build                           0.69   3.9%    422     5.9
choose                          0.32   1.8%    139     8.2
research                        0.25   1.5%    160     5.7
release                         0.12   0.7%    116     3.8
ship                            0.10   0.6%     43     8.3
bugs                            0.08   0.5%     34     8.9
log                             0.08   0.4%     58     4.7

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
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   10    1312s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
```

## Every pass recorded

```text
18 pass(es), 5874 steps, 23.0 h of pass time

category                       hours  share  steps  mean s
model                           7.55  32.8%   2568    10.6
wait: gate                      7.54  32.7%     41   662.3
wait: tests                     2.73  11.8%     27   363.5
shell                           1.03   4.5%    782     4.7
wait: other                     0.94   4.1%     32   106.0
wait: clip                      0.76   3.3%     28    98.0
cargo test                      0.60   2.6%    141    15.4
wait: build                     0.60   2.6%     10   215.6
cargo build                     0.37   1.6%     72    18.6
eval                            0.17   0.7%      9    68.4
remote wait                     0.14   0.6%     13    39.5
phase mark                      0.10   0.4%     43     8.5
edit files                      0.09   0.4%    605     0.6
read files                      0.08   0.4%    930     0.3
clip record/render              0.06   0.3%     31     7.2

phase                          hours  share  steps  mean s
orient                         18.71  81.2%   4187    16.1
gate                            1.38   6.0%    156    31.9
prove                           1.32   5.7%    520     9.1
build                           0.69   3.0%    422     5.9
choose                          0.32   1.4%    139     8.2
research                        0.25   1.1%    160     5.7
release                         0.12   0.5%    116     3.8
ship                            0.10   0.4%     43     8.3
bugs                            0.08   0.4%     34     8.9
log                             0.08   0.3%     58     4.7

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

