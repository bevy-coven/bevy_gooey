# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 05:37Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 0 | 2026-11-01T00:31Z | 16 | 0 | 5 | - | 12 | 2 | release |
| 39 | 2026-11-01T03:31Z | 10 | 0 | 7 | v0.5.0 | 10 | 2 | - |
| 40 | 2026-11-01T04:18Z | 8 | 0 | 7 | v0.6.0 | 12 | 3 | - |
| 42 | 2026-11-01T05:37Z | 8 | 0 | 8 | v0.7.0 | 15 | 6 | - |

## The last 12 passes

```text
12 pass(es), 4124 steps, 16.1 h of pass time

category                       hours  share  steps  mean s
model                           5.94  36.8%   1805    11.8
wait: gate                      5.17  32.0%     27   689.5
wait: tests                     2.04  12.7%     15   490.8
shell                           0.69   4.3%    564     4.4
wait: other                     0.47   2.9%     16   106.8
wait: clip                      0.47   2.9%     17    99.2
cargo test                      0.43   2.6%     94    16.3
cargo build                     0.27   1.7%     53    18.5
eval                            0.16   1.0%      6    95.7
remote wait                     0.11   0.7%      9    44.5
read files                      0.07   0.4%    689     0.3
edit files                      0.05   0.3%    417     0.4
clip record/render              0.04   0.2%     21     6.5
web                             0.04   0.2%     14     9.3
wait: build                     0.04   0.2%      6    21.6

phase                          hours  share  steps  mean s
orient                         14.74  91.3%   3319    16.0
gate                            0.48   3.0%    128    13.6
prove                           0.40   2.5%    272     5.3
build                           0.19   1.2%    144     4.7
choose                          0.12   0.8%     76     5.8
release                         0.07   0.5%     71     3.8
log                             0.06   0.4%     44     4.7
bugs                            0.05   0.3%     22     8.3
ship                            0.02   0.1%     18     4.1

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
16 pass(es), 4919 steps, 20.0 h of pass time

category                       hours  share  steps  mean s
model                           6.56  32.8%   2158    10.9
wait: gate                      6.46  32.3%     37   628.1
wait: tests                     2.59  13.0%     22   424.1
shell                           0.91   4.5%    670     4.9
wait: clip                      0.67   3.3%     25    96.4
wait: other                     0.67   3.3%     24   100.1
wait: build                     0.60   3.0%      9   238.8
cargo test                      0.45   2.3%    117    13.9
cargo build                     0.34   1.7%     68    18.1
eval                            0.17   0.9%      9    68.4
remote wait                     0.14   0.7%     11    46.0
edit files                      0.08   0.4%    487     0.6
read files                      0.07   0.4%    758     0.4
homestill                       0.05   0.2%     22     7.5
web                             0.04   0.2%     17     9.5

phase                          hours  share  steps  mean s
orient                         18.59  93.0%   4108    16.3
gate                            0.48   2.4%    128    13.6
prove                           0.40   2.0%    272     5.3
build                           0.19   0.9%    144     4.7
choose                          0.12   0.6%     76     5.8
release                         0.07   0.4%     71     3.8
log                             0.06   0.3%     44     4.7
bugs                            0.05   0.3%     22     8.3
ship                            0.02   0.1%     18     4.1

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

