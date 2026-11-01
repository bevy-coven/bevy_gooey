# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 04:18Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 0 | 2026-11-01T00:31Z | 16 | 0 | 5 | - | 12 | 2 | release |
| 39 | 2026-11-01T03:31Z | 10 | 0 | 7 | v0.5.0 | 10 | 2 | - |
| 40 | 2026-11-01T04:18Z | 8 | 0 | 7 | v0.6.0 | 12 | 3 | - |

## The last 12 passes

```text
12 pass(es), 4113 steps, 16.6 h of pass time

category                       hours  share  steps  mean s
model                           5.81  35.0%   1789    11.7
wait: gate                      5.65  34.1%     27   753.4
wait: tests                     2.04  12.3%     15   490.8
shell                           0.69   4.2%    560     4.5
wait: other                     0.53   3.2%     19    99.6
wait: clip                      0.49   3.0%     19    93.0
cargo test                      0.43   2.6%     99    15.7
cargo build                     0.30   1.8%     55    19.7
eval                            0.16   1.0%      6    95.7
remote wait                     0.11   0.7%      9    44.5
read files                      0.06   0.4%    663     0.3
edit files                      0.06   0.3%    422     0.5
homestill                       0.04   0.2%     20     7.5
web                             0.04   0.2%     16     9.1
github                          0.03   0.2%     26     3.7

phase                          hours  share  steps  mean s
orient                         15.78  95.2%   3598    15.8
gate                            0.30   1.8%     59    18.3
build                           0.15   0.9%    133     4.1
choose                          0.12   0.7%     76     5.8
prove                           0.12   0.7%    127     3.4
release                         0.05   0.3%     53     3.7
log                             0.02   0.1%     20     4.2
ship                            0.02   0.1%     16     4.2

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
15 pass(es), 4616 steps, 19.4 h of pass time

category                       hours  share  steps  mean s
wait: gate                      6.37  32.9%     35   654.7
model                           6.21  32.1%   2018    11.1
wait: tests                     2.59  13.4%     22   424.1
shell                           0.84   4.3%    625     4.9
wait: clip                      0.67   3.5%     25    96.4
wait: other                     0.63   3.3%     23    99.0
wait: build                     0.57   2.9%      8   256.7
cargo test                      0.44   2.3%    113    14.0
cargo build                     0.34   1.7%     66    18.4
eval                            0.17   0.9%      9    68.4
remote wait                     0.14   0.7%     11    46.0
edit files                      0.07   0.4%    462     0.6
read files                      0.07   0.4%    707     0.4
homestill                       0.05   0.2%     22     7.5
web                             0.04   0.2%     17     9.5

phase                          hours  share  steps  mean s
orient                         18.58  95.9%   4098    16.3
gate                            0.30   1.5%     59    18.3
build                           0.15   0.8%    133     4.1
choose                          0.12   0.6%     76     5.8
prove                           0.12   0.6%    127     3.4
release                         0.05   0.3%     53     3.7
log                             0.02   0.1%     20     4.2
ship                            0.02   0.1%     16     4.2

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

