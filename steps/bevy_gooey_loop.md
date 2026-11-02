# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 07:22Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 64 | 2026-11-01T23:53Z | 7 | 0 | 7 | - | 8 | 1 | release |
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

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            15      40.5        38.9
```

## The last 12 passes

```text
12 pass(es), 3730 steps, 8.2 h of pass time

category                       hours  share  steps  mean s
model                           4.12  50.3%   1761     8.4
wait: other                     0.93  11.4%     44    76.4
cargo test                      0.73   8.9%    132    20.0
wait: gate                      0.64   7.8%     11   209.0
cargo build                     0.49   5.9%    117    15.0
shell                           0.37   4.6%    759     1.8
phase mark                      0.29   3.5%    132     7.9
wait: build                     0.14   1.7%      4   123.4
remote wait                     0.09   1.1%     14    23.3
read files                      0.08   1.0%    332     0.9
github                          0.05   0.7%     36     5.4
wait: tests                     0.05   0.6%      3    56.7
cargo publish                   0.05   0.6%     15    11.2
cargo clippy                    0.04   0.5%     22     7.3
web                             0.03   0.4%     13     8.6

phase                          hours  share  steps  mean s
prove                           2.35  28.7%   1072     7.9
build                           2.32  28.4%   1206     6.9
gate                            1.81  22.1%    298    21.9
choose                          0.46   5.6%    276     6.0
research                        0.45   5.5%    334     4.9
release                         0.33   4.1%    166     7.2
ship                            0.23   2.8%    149     5.5
log                             0.11   1.4%     90     4.6
orient                          0.09   1.1%     96     3.4
bugs                            0.03   0.3%     25     3.9

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
  pass   64     153s  model model                
  pass   73     150s  tool  wait: gate           Waiting for the gate
```

## Every pass recorded

```text
44 pass(es), 14366 steps, 41.0 h of pass time

category                       hours  share  steps  mean s
model                          16.62  40.5%   6480     9.2
wait: gate                      9.78  23.9%     71   496.0
wait: tests                     2.83   6.9%     33   308.8
wait: other                     2.69   6.6%    105    92.1
cargo test                      1.95   4.8%    430    16.4
shell                           1.79   4.4%   2328     2.8
cargo build                     1.29   3.2%    285    16.3
wait: build                     0.91   2.2%     19   172.5
wait: clip                      0.79   1.9%     30    94.9
phase mark                      0.66   1.6%    299     7.9
remote wait                     0.30   0.7%     36    29.6
read files                      0.25   0.6%   1865     0.5
eval                            0.18   0.4%     10    64.6
github                          0.16   0.4%    145     4.0
push gate                       0.13   0.3%    107     4.4

phase                          hours  share  steps  mean s
orient                         18.93  46.2%   4390    15.5
prove                           6.50  15.9%   3240     7.2
gate                            5.73  14.0%    985    21.0
build                           5.61  13.7%   3096     6.5
research                        1.25   3.0%    785     5.7
choose                          1.05   2.6%    573     6.6
release                         0.78   1.9%    489     5.7
ship                            0.70   1.7%    415     6.1
log                             0.33   0.8%    257     4.6
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

