# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 08:29Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 78 | 2026-11-02T08:29Z | 5 | 0 | 5 | v0.30.0 | 7 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            18      37.2        35.8
```

## The last 12 passes

```text
12 pass(es), 3192 steps, 6.7 h of pass time

category                       hours  share  steps  mean s
model                           3.28  48.7%   1491     7.9
cargo test                      0.70  10.4%    117    21.5
wait: other                     0.65   9.6%     27    86.1
wait: gate                      0.56   8.3%      9   223.6
cargo build                     0.45   6.7%     94    17.3
phase mark                      0.30   4.5%    131     8.3
shell                           0.29   4.3%    653     1.6
wait: build                     0.14   2.0%      4   123.4
read files                      0.06   0.8%    295     0.7
github                          0.06   0.8%     31     6.6
cargo publish                   0.05   0.8%     16    11.9
wait: tests                     0.05   0.7%      3    56.7
cargo clippy                    0.04   0.6%     17     9.2
push gate                       0.03   0.4%     22     4.1
web                             0.02   0.3%      8     8.3

phase                          hours  share  steps  mean s
prove                           1.95  29.0%    979     7.2
build                           1.80  26.7%    994     6.5
gate                            1.58  23.5%    269    21.2
research                        0.48   7.1%    352     4.9
choose                          0.31   4.6%    179     6.3
release                         0.31   4.6%    142     7.9
log                             0.11   1.6%     78     5.0
ship                            0.11   1.6%     92     4.1
orient                          0.06   0.8%     71     2.8
bugs                            0.03   0.4%     23     4.1

slowest single steps
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   74     308s  model model                
  pass   67     226s  tool  wait: build          Waiting for notifications example build
  pass   67     221s  tool  wait: build          Waiting for example build
  pass   78     182s  tool  wait: other          {}
  pass   75     179s  tool  wait: other          {}
  pass   75     169s  tool  wait: other          {}
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   74     155s  tool  wait: other          {}
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   76     148s  tool  wait: other          {}
  pass   72     135s  tool  wait: gate           Waiting for gate 2
  pass   69     132s  tool  wait: gate           Waiting for gate
  pass   76     132s  tool  wait: other          {}
```

## Every pass recorded

```text
47 pass(es), 14875 steps, 42.0 h of pass time

category                       hours  share  steps  mean s
model                          17.10  40.7%   6714     9.2
wait: gate                      9.78  23.3%     71   496.0
wait: other                     2.88   6.9%    113    91.8
wait: tests                     2.83   6.7%     33   308.8
cargo test                      2.07   4.9%    449    16.6
shell                           1.81   4.3%   2421     2.7
cargo build                     1.36   3.2%    293    16.7
wait: build                     0.91   2.2%     19   172.5
wait: clip                      0.79   1.9%     30    94.9
phase mark                      0.74   1.8%    330     8.0
remote wait                     0.30   0.7%     36    29.6
read files                      0.25   0.6%   1917     0.5
eval                            0.18   0.4%     10    64.6
github                          0.18   0.4%    151     4.3
cargo clippy                    0.13   0.3%     65     7.5

phase                          hours  share  steps  mean s
orient                         18.95  45.1%   4411    15.5
prove                           6.89  16.4%   3478     7.1
gate                            5.94  14.1%   1013    21.1
build                           5.81  13.8%   3207     6.5
research                        1.32   3.1%    826     5.8
choose                          1.10   2.6%    602     6.6
release                         0.82   1.9%    511     5.8
ship                            0.71   1.7%    421     6.1
log                             0.36   0.8%    269     4.8
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

