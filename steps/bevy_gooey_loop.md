# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 05:30Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 61 | 2026-11-01T21:47Z | 10 | 0 | 8 | v0.20.0 | 9 | 1 | - |
| 62 | 2026-11-01T21:59Z | 4 | 0 | 1 | v0.20.1 | 1 | 0 | demo? |
| 63 | 2026-11-01T23:06Z | 7 | 0 | 7 | v0.21.0 | 10 | 1 | - |
| 64 | 2026-11-01T23:53Z | 7 | 0 | 7 | - | 8 | 1 | release |
| 65 | 2026-11-02T00:33Z | 6 | 0 | 7 | - | 12 | 1 | release |
| 66 | 2026-11-02T01:34Z | 6 | 0 | 10 | - | 11 | 1 | release |
| 67 | 2026-11-02T02:36Z | 14 | 0 | 10 | - | 13 | 3 | release |
| 68 | 2026-11-02T03:45Z | 9 | 0 | 11 | v0.23.0 | 17 | 3 | - |
| 69 | 2026-11-02T04:18Z | 11 | 0 | 6 | v0.24.0 | 12 | 1 | - |
| 70 | 2026-11-02T04:46Z | 8 | 0 | 4 | v0.24.1 | 5 | 0 | demo? |
| 71 | 2026-11-02T04:59Z | 4 | 0 | 2 | v0.24.2 | 5 | 1 | - |
| 72 | 2026-11-02T05:30Z | 11 | 0 | 6 | v0.25.0,v0.25.1 | 9 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            12      41.7        41.0
```

## The last 12 passes

```text
12 pass(es), 3650 steps, 8.4 h of pass time

category                       hours  share  steps  mean s
model                           4.04  48.2%   1700     8.6
wait: gate                      1.16  13.8%     14   298.2
wait: other                     0.77   9.2%     35    79.7
cargo test                      0.74   8.9%    131    20.4
cargo build                     0.41   4.9%    106    13.9
shell                           0.33   3.9%    665     1.8
phase mark                      0.31   3.7%    123     9.2
wait: build                     0.17   2.0%      5   119.0
remote wait                     0.10   1.2%     14    26.2
read files                      0.07   0.8%    382     0.6
wait: tests                     0.06   0.7%      4    49.8
cargo clippy                    0.05   0.5%     22     7.4
github                          0.04   0.5%     42     3.6
cargo publish                   0.04   0.5%     14    10.2
push gate                       0.04   0.4%     27     5.0

phase                          hours  share  steps  mean s
build                           2.51  29.9%   1282     7.0
prove                           2.38  28.4%   1022     8.4
gate                            2.06  24.5%    368    20.1
choose                          0.42   5.0%    278     5.4
research                        0.29   3.4%    169     6.1
release                         0.28   3.3%    172     5.8
ship                            0.24   2.8%    157     5.5
orient                          0.10   1.2%    101     3.6
log                             0.09   1.0%     61     5.1
bugs                            0.03   0.4%     29     3.9

slowest single steps
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   66     766s  tool  wait: other          {}
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   62     282s  tool  wait: other          {}
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   67     226s  tool  wait: build          Waiting for notifications example build
  pass   67     221s  tool  wait: build          Waiting for example build
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   65     160s  tool  wait: other          {}
  pass   66     155s  tool  wait: other          {}
  pass   64     153s  model model                
  pass   61     150s  model model                
```

## Every pass recorded

```text
41 pass(es), 13374 steps, 39.2 h of pass time

category                       hours  share  steps  mean s
model                          15.63  39.9%   6017     9.3
wait: gate                      9.74  24.9%     70   501.0
wait: tests                     2.83   7.2%     33   308.8
wait: other                     2.39   6.1%     92    93.6
cargo test                      1.83   4.7%    405    16.2
shell                           1.72   4.4%   2109     2.9
cargo build                     1.17   3.0%    262    16.0
wait: build                     0.91   2.3%     19   172.5
wait: clip                      0.79   2.0%     30    94.9
phase mark                      0.59   1.5%    265     8.0
remote wait                     0.29   0.7%     35    29.9
read files                      0.21   0.5%   1763     0.4
eval                            0.18   0.5%     10    64.6
github                          0.14   0.4%    141     3.6
push gate                       0.12   0.3%    103     4.3

phase                          hours  share  steps  mean s
orient                         18.92  48.3%   4375    15.6
prove                           6.16  15.7%   2962     7.5
build                           5.22  13.3%   2860     6.6
gate                            5.19  13.2%    853    21.9
research                        1.00   2.5%    583     6.1
choose                          0.95   2.4%    541     6.3
ship                            0.70   1.8%    409     6.1
release                         0.63   1.6%    447     5.1
log                             0.28   0.7%    217     4.7
bugs                            0.12   0.3%     63     6.6

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

