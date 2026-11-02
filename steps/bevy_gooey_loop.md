# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 08:07Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 77 | 2026-11-02T08:07Z | 6 | 0 | 6 | v0.29.0 | 6 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            17      38.1        38.8
```

## The last 12 passes

```text
12 pass(es), 3430 steps, 7.4 h of pass time

category                       hours  share  steps  mean s
model                           3.62  48.8%   1602     8.1
wait: other                     0.96  13.0%     43    80.6
cargo test                      0.66   9.0%    120    19.9
wait: gate                      0.56   7.5%      9   223.6
cargo build                     0.44   6.0%    104    15.3
shell                           0.34   4.5%    689     1.8
phase mark                      0.28   3.7%    130     7.7
wait: build                     0.14   1.9%      4   123.4
github                          0.06   0.8%     33     6.8
remote wait                     0.06   0.8%     11    18.6
read files                      0.06   0.7%    325     0.6
cargo publish                   0.05   0.7%     15    12.6
wait: tests                     0.05   0.6%      3    56.7
cargo clippy                    0.04   0.5%     16     8.3
push gate                       0.02   0.3%     22     4.1

phase                          hours  share  steps  mean s
prove                           2.12  28.6%   1049     7.3
build                           2.03  27.4%   1094     6.7
gate                            1.81  24.4%    282    23.0
research                        0.47   6.4%    343     4.9
choose                          0.39   5.2%    237     5.8
release                         0.31   4.1%    140     7.9
ship                            0.12   1.6%     96     4.3
log                             0.10   1.4%     80     4.5
orient                          0.05   0.7%     69     2.7
bugs                            0.03   0.4%     23     4.1

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
  pass   74     155s  tool  wait: other          {}
  pass   66     155s  tool  wait: other          {}
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   76     148s  tool  wait: other          {}
  pass   66     140s  tool  wait: other          {}
```

## Every pass recorded

```text
46 pass(es), 14731 steps, 41.7 h of pass time

category                       hours  share  steps  mean s
model                          16.94  40.7%   6646     9.2
wait: gate                      9.78  23.5%     71   496.0
wait: tests                     2.83   6.8%     33   308.8
wait: other                     2.82   6.8%    111    91.6
cargo test                      2.02   4.8%    440    16.5
shell                           1.80   4.3%   2401     2.7
cargo build                     1.34   3.2%    290    16.7
wait: build                     0.91   2.2%     19   172.5
wait: clip                      0.79   1.9%     30    94.9
phase mark                      0.71   1.7%    320     7.9
remote wait                     0.30   0.7%     36    29.6
read files                      0.25   0.6%   1905     0.5
eval                            0.18   0.4%     10    64.6
github                          0.18   0.4%    149     4.3
push gate                       0.13   0.3%    110     4.4

phase                          hours  share  steps  mean s
orient                         18.94  45.5%   4403    15.5
prove                           6.75  16.2%   3408     7.1
gate                            5.89  14.1%   1005    21.1
build                           5.73  13.8%   3183     6.5
research                        1.30   3.1%    811     5.8
choose                          1.08   2.6%    594     6.6
release                         0.81   2.0%    507     5.8
ship                            0.71   1.7%    419     6.1
log                             0.34   0.8%    265     4.7
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

