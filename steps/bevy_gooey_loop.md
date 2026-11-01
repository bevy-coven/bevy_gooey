# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 03:31Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 0 | 2026-11-01T00:31Z | 16 | 0 | 5 | - | 12 | 2 | release |
| 39 | 2026-11-01T03:31Z | 10 | 0 | 7 | v0.5.0 | 10 | 2 | - |

## The last 12 passes

```text
12 pass(es), 3797 steps, 16.2 h of pass time

category                       hours  share  steps  mean s
wait: gate                      5.59  34.5%     29   693.9
model                           5.48  33.8%   1654    11.9
wait: tests                     2.05  12.6%     16   460.2
shell                           0.74   4.6%    498     5.4
wait: other                     0.50   3.1%     19    95.0
wait: clip                      0.49   3.0%     20    89.0
cargo test                      0.43   2.6%    101    15.3
cargo build                     0.28   1.7%     52    19.2
eval                            0.16   1.0%      2   283.9
remote wait                     0.14   0.9%     11    46.0
read files                      0.07   0.4%    602     0.4
edit files                      0.06   0.3%    383     0.5
homestill                       0.04   0.3%     18     8.4
web                             0.04   0.3%     15     9.7
cargo clippy                    0.03   0.2%     12     8.9

phase                          hours  share  steps  mean s
orient                         16.16  99.7%   3725    15.6
release                         0.03   0.2%     26     3.8
log                             0.02   0.1%     16     4.0

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
  pass   10    1312s  tool  wait: gate           Waiting for the gate
  pass    7    1113s  model model                
```

## Every pass recorded

```text
14 pass(es), 4140 steps, 18.6 h of pass time

category                       hours  share  steps  mean s
wait: gate                      6.18  33.2%     34   654.8
model                           5.76  31.0%   1812    11.4
wait: tests                     2.59  13.9%     22   424.1
shell                           0.82   4.4%    543     5.4
wait: clip                      0.67   3.6%     25    96.4
wait: other                     0.60   3.2%     22    97.9
wait: build                     0.57   3.1%      8   256.7
cargo test                      0.43   2.3%    111    13.9
cargo build                     0.31   1.7%     61    18.2
eval                            0.17   0.9%      5   121.9
remote wait                     0.14   0.8%     11    46.0
edit files                      0.07   0.4%    408     0.6
read files                      0.07   0.4%    629     0.4
homestill                       0.04   0.2%     19     8.0
web                             0.04   0.2%     15     9.7

phase                          hours  share  steps  mean s
orient                         18.56  99.8%   4068    16.4
release                         0.03   0.1%     26     3.8
log                             0.02   0.1%     16     4.0

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

