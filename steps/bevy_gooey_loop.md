# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 00:31Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 0 | 2026-11-01T00:31Z | 16 | 0 | 5 | - | 12 | 2 | release |

## The last 12 passes

```text
12 pass(es), 3776 steps, 17.1 h of pass time

category                       hours  share  steps  mean s
wait: gate                      5.79  33.8%     32   651.9
model                           5.49  32.0%   1648    12.0
wait: tests                     2.34  13.7%     20   421.6
shell                           0.77   4.5%    480     5.8
wait: clip                      0.52   3.1%     21    89.9
wait: other                     0.47   2.8%     18    94.3
cargo test                      0.43   2.5%    107    14.4
wait: build                     0.40   2.4%      6   242.0
cargo build                     0.28   1.7%     56    18.3
eval                            0.16   0.9%      2   283.9
remote wait                     0.13   0.8%     10    48.5
edit files                      0.07   0.4%    390     0.6
read files                      0.07   0.4%    594     0.4
homestill                       0.04   0.2%     18     8.4
web                             0.04   0.2%     15     9.7

phase                          hours  share  steps  mean s
orient                         17.14 100.0%   3746    16.5

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

## Every pass recorded

```text
13 pass(es), 3919 steps, 17.9 h of pass time

category                       hours  share  steps  mean s
wait: gate                      5.79  32.4%     32   651.9
model                           5.60  31.3%   1712    11.8
wait: tests                     2.59  14.5%     22   424.1
shell                           0.81   4.5%    502     5.8
wait: clip                      0.61   3.4%     24    92.1
wait: build                     0.57   3.2%      7   293.3
wait: other                     0.55   3.1%     20    99.2
cargo test                      0.43   2.4%    110    14.0
cargo build                     0.29   1.6%     60    17.5
eval                            0.17   0.9%      5   121.9
remote wait                     0.13   0.8%     10    48.5
edit files                      0.07   0.4%    397     0.6
read files                      0.07   0.4%    607     0.4
homestill                       0.04   0.2%     19     8.0
web                             0.04   0.2%     15     9.7

phase                          hours  share  steps  mean s
orient                         17.90 100.0%   3889    16.6

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

