# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 07:48Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 65 | 2026-11-02T00:33Z | 6 | 0 | 7 | - | 12 | 1 | release |
| 66 | 2026-11-02T01:34Z | 6 | 0 | 10 | - | 11 | 1 | release |
| 67 | 2026-11-02T02:36Z | 14 | 0 | 10 | - | 13 | 3 | release |
| 68 | 2026-11-02T03:45Z | 9 | 0 | 11 | v0.23.0 | 17 | 3 | - |
| 69 | 2026-11-02T04:18Z | 11 | 0 | 6 | v0.24.0 | 12 | 1 | - |
| 70 | 2026-11-02T04:46Z | 8 | 0 | 4 | v0.24.1 | 5 | 0 | demo? |
| 71 | 2026-11-02T04:59Z | 4 | 0 | 2 | v0.24.2 | 5 | 1 | - |
| 72 | 2026-11-02T05:30Z | 11 | 0 | 6 | v0.25.0,v0.25.1 | 9 | 1 | - |
| 73 | 2026-11-02T06:09Z | 11 | 0 | 8 | v0.26.0 | 10 | 1 | - |
| 74 | 2026-11-02T06:50Z | 7 | 0 | 8 | v0.27.0 | 8 | 1 | - |
| 75 | 2026-11-02T07:22Z | 8 | 0 | 5 | v0.27.1 | 7 | 1 | - |
| 76 | 2026-11-02T07:48Z | 8 | 0 | 7 | v0.28.0 | 9 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            16      39.5        38.8
```

## The last 12 passes

```text
12 pass(es), 3626 steps, 7.8 h of pass time

category                       hours  share  steps  mean s
model                           3.88  49.8%   1707     8.2
wait: other                     1.01  12.9%     46    78.9
cargo test                      0.66   8.5%    125    19.0
wait: gate                      0.56   7.2%      9   223.6
cargo build                     0.46   5.9%    109    15.1
shell                           0.33   4.3%    729     1.6
phase mark                      0.29   3.8%    134     7.9
wait: build                     0.14   1.8%      4   123.4
remote wait                     0.09   1.2%     14    23.3
github                          0.07   0.9%     35     7.1
read files                      0.06   0.7%    323     0.6
cargo publish                   0.05   0.6%     15    11.4
wait: tests                     0.05   0.6%      3    56.7
cargo clippy                    0.04   0.5%     18     7.5
push gate                       0.03   0.4%     24     4.6

phase                          hours  share  steps  mean s
build                           2.22  28.4%   1188     6.7
prove                           2.15  27.6%   1024     7.6
gate                            1.84  23.6%    304    21.8
research                        0.48   6.2%    345     5.1
choose                          0.39   5.0%    234     6.0
release                         0.31   4.0%    152     7.4
ship                            0.16   2.1%    143     4.0
log                             0.12   1.5%     94     4.6
orient                          0.10   1.2%    101     3.4
bugs                            0.03   0.3%     23     4.1

slowest single steps
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   66     766s  tool  wait: other          {}
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   74     308s  model model                
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   67     226s  tool  wait: build          Waiting for notifications example build
  pass   67     221s  tool  wait: build          Waiting for example build
  pass   75     179s  tool  wait: other          {}
  pass   75     169s  tool  wait: other          {}
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   65     160s  tool  wait: other          {}
  pass   74     155s  tool  wait: other          {}
  pass   66     155s  tool  wait: other          {}
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   76     148s  tool  wait: other          {}
```

## Every pass recorded

```text
45 pass(es), 14560 steps, 41.4 h of pass time

category                       hours  share  steps  mean s
model                          16.81  40.6%   6571     9.2
wait: gate                      9.78  23.6%     71   496.0
wait: tests                     2.83   6.8%     33   308.8
wait: other                     2.79   6.7%    109    92.0
cargo test                      1.99   4.8%    437    16.4
shell                           1.79   4.3%   2367     2.7
cargo build                     1.33   3.2%    289    16.5
wait: build                     0.91   2.2%     19   172.5
wait: clip                      0.79   1.9%     30    94.9
phase mark                      0.68   1.6%    310     7.9
remote wait                     0.30   0.7%     36    29.6
read files                      0.25   0.6%   1878     0.5
eval                            0.18   0.4%     10    64.6
github                          0.18   0.4%    148     4.3
push gate                       0.13   0.3%    109     4.4

phase                          hours  share  steps  mean s
orient                         18.94  45.7%   4398    15.5
prove                           6.64  16.0%   3319     7.2
gate                            5.83  14.1%   1001    21.0
build                           5.69  13.7%   3144     6.5
research                        1.28   3.1%    798     5.8
choose                          1.08   2.6%    587     6.6
release                         0.80   1.9%    499     5.7
ship                            0.71   1.7%    417     6.1
log                             0.33   0.8%    261     4.6
bugs                            0.12   0.3%     65     6.5

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

