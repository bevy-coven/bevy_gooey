# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 17:53Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 71 | 2026-11-02T04:59Z | 4 | 0 | 2 | v0.24.2 | 5 | 1 | - |
| 72 | 2026-11-02T05:30Z | 11 | 0 | 6 | v0.25.0,v0.25.1 | 9 | 1 | - |
| 73 | 2026-11-02T06:09Z | 11 | 0 | 8 | v0.26.0 | 10 | 1 | - |
| 74 | 2026-11-02T06:50Z | 7 | 0 | 8 | v0.27.0 | 8 | 1 | - |
| 75 | 2026-11-02T07:22Z | 8 | 0 | 5 | v0.27.1 | 7 | 1 | - |
| 76 | 2026-11-02T07:48Z | 8 | 0 | 7 | v0.28.0 | 9 | 1 | - |
| 77 | 2026-11-02T08:07Z | 6 | 0 | 6 | v0.29.0 | 6 | 1 | - |
| 78 | 2026-11-02T08:29Z | 5 | 0 | 5 | v0.30.0 | 7 | 1 | - |
| 79 | 2026-11-02T16:56Z | 5 | 0 | 5 | v0.31.0 | 7 | 1 | - |
| 80 | 2026-11-02T17:23Z | 5 | 0 | 4 | v0.31.1 | 9 | 2 | - |
| 81 | 2026-11-02T17:37Z | 8 | 0 | 4 | v0.31.2 | 4 | 0 | demo? |
| 82 | 2026-11-02T17:53Z | 6 | 0 | 5 | v0.31.3 | 6 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            22      33.5        30.0
```

## The last 12 passes

```text
12 pass(es), 2539 steps, 4.7 h of pass time

category                       hours  share  steps  mean s
model                           2.28  48.4%   1170     7.0
wait: other                     0.65  13.8%     27    86.9
cargo test                      0.51  10.9%     81    22.7
cargo build                     0.30   6.3%     47    22.8
phase mark                      0.27   5.7%    119     8.1
shell                           0.22   4.6%    488     1.6
wait: gate                      0.18   3.9%      5   131.7
cargo clippy                    0.07   1.4%     18    13.1
cargo publish                   0.05   1.1%     13    14.5
read files                      0.05   1.1%    285     0.6
github                          0.04   0.8%     22     6.4
push gate                       0.03   0.6%     16     6.4
web                             0.02   0.5%     10     8.5
git                             0.02   0.4%     20     3.1
clip record/render              0.01   0.2%     24     1.7

phase                          hours  share  steps  mean s
gate                            1.25  26.6%    243    18.5
build                           1.13  24.1%    624     6.5
prove                           1.07  22.8%    797     4.8
research                        0.43   9.1%    325     4.7
choose                          0.32   6.8%    213     5.4
release                         0.25   5.3%    108     8.3
ship                            0.10   2.1%     80     4.5
log                             0.10   2.0%     68     5.1
orient                          0.05   1.1%     68     2.7
bugs                            0.01   0.1%      4     4.8

slowest single steps
  pass   74     308s  model model                
  pass   80     190s  tool  wait: other          {}
  pass   78     182s  tool  wait: other          {}
  pass   75     179s  tool  wait: other          {}
  pass   75     169s  tool  wait: other          {}
  pass   74     155s  tool  wait: other          {}
  pass   82     155s  tool  wait: gate           Waiting for gate
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   76     148s  tool  wait: other          {}
  pass   80     146s  bg    shell                bg_2 completed: G=$CARGO_TARGET_DIR/release/examples/inventory; cp $G /tmp/g52/inv_after &
  pass   79     138s  tool  wait: other          {}
  pass   72     135s  tool  wait: gate           Waiting for gate 2
  pass   76     132s  tool  wait: other          {}
  pass   75     130s  tool  wait: other          {}
  pass   77     129s  tool  wait: other          {}
```

## Every pass recorded

```text
51 pass(es), 15464 steps, 43.1 h of pass time

category                       hours  share  steps  mean s
model                          17.58  40.8%   6991     9.1
wait: gate                      9.86  22.9%     73   486.3
wait: other                     2.99   6.9%    117    91.9
wait: tests                     2.83   6.6%     33   308.8
cargo test                      2.25   5.2%    469    17.3
shell                           1.86   4.3%   2533     2.6
cargo build                     1.44   3.3%    305    17.0
wait: build                     0.91   2.1%     19   172.5
wait: clip                      0.79   1.8%     30    94.9
phase mark                      0.79   1.8%    363     7.8
remote wait                     0.30   0.7%     36    29.6
read files                      0.25   0.6%   1982     0.5
github                          0.18   0.4%    157     4.1
eval                            0.18   0.4%     10    64.6
cargo clippy                    0.17   0.4%     73     8.6

phase                          hours  share  steps  mean s
orient                         18.96  44.0%   4431    15.4
prove                           7.13  16.5%   3655     7.0
gate                            6.20  14.4%   1056    21.1
build                           6.16  14.3%   3373     6.6
research                        1.34   3.1%    841     5.8
choose                          1.23   2.9%    721     6.2
release                         0.85   2.0%    527     5.8
ship                            0.77   1.8%    443     6.2
log                             0.37   0.9%    277     4.8
bugs                            0.12   0.3%     67     6.5

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

