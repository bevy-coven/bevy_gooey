# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 18:55Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 83 | 2026-11-02T18:55Z | 7 | 0 | 5 | v0.32.0 | 6 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            23      34.6        30.4
```

## The last 12 passes

```text
12 pass(es), 2565 steps, 5.5 h of pass time

category                       hours  share  steps  mean s
model                           2.39  43.4%   1190     7.2
wait: other                     1.34  24.3%     30   160.5
cargo test                      0.50   9.2%     78    23.2
cargo build                     0.31   5.6%     52    21.4
phase mark                      0.27   4.9%    118     8.3
shell                           0.19   3.5%    504     1.4
wait: gate                      0.18   3.3%      5   131.7
cargo clippy                    0.07   1.2%     18    13.1
cargo publish                   0.06   1.0%     13    15.5
read files                      0.04   0.8%    275     0.6
github                          0.04   0.7%     22     6.4
push gate                       0.03   0.6%     17     6.5
web                             0.02   0.5%     11     8.1
remote wait                     0.02   0.4%      2    40.0
git                             0.02   0.3%     21     3.0

phase                          hours  share  steps  mean s
gate                            1.92  34.9%    238    29.0
prove                           1.17  21.4%    863     4.9
build                           1.09  19.8%    597     6.5
research                        0.51   9.3%    367     5.0
choose                          0.31   5.7%    199     5.7
release                         0.26   4.8%    110     8.7
log                             0.10   1.8%     68     5.1
ship                            0.08   1.4%     40     6.9
orient                          0.05   0.9%     70     2.6
bugs                            0.01   0.1%      4     4.8

slowest single steps
  pass   83    1800s  tool  wait: other          {}
  pass   83     537s  tool  wait: other          {}
  pass   74     308s  model model                
  pass   83     229s  tool  wait: other          {}
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
```

## Every pass recorded

```text
52 pass(es), 15632 steps, 44.2 h of pass time

category                       hours  share  steps  mean s
model                          17.78  40.3%   7073     9.0
wait: gate                      9.86  22.3%     73   486.3
wait: other                     3.70   8.4%    121   110.2
wait: tests                     2.83   6.4%     33   308.8
cargo test                      2.27   5.1%    473    17.3
shell                           1.87   4.2%   2573     2.6
cargo build                     1.46   3.3%    311    16.9
wait: build                     0.91   2.1%     19   172.5
phase mark                      0.81   1.8%    370     7.9
wait: clip                      0.79   1.8%     30    94.9
remote wait                     0.31   0.7%     37    30.4
read files                      0.26   0.6%   1992     0.5
github                          0.18   0.4%    160     4.1
eval                            0.18   0.4%     10    64.6
cargo clippy                    0.17   0.4%     73     8.6

phase                          hours  share  steps  mean s
orient                         18.97  43.0%   4440    15.4
prove                           7.26  16.4%   3734     7.0
gate                            6.94  15.7%   1072    23.3
build                           6.18  14.0%   3381     6.6
research                        1.43   3.2%    883     5.8
choose                          1.23   2.8%    721     6.2
release                         0.88   2.0%    539     5.9
ship                            0.77   1.7%    445     6.2
log                             0.37   0.8%    277     4.8
bugs                            0.12   0.3%     67     6.5

slowest single steps
  pass    7    2558s  bg    shell                bg_16 completed: /tmp/g11/gate.sh > /tmp/g11/gate.out 2>&1; tail -20 /tmp/g11/gate/summary
  pass    7    2114s  model model                
  pass    7    1826s  tool  wait: gate           Waiting for the gate to finish
  pass    7    1818s  tool  wait: tests          Waiting for toast tests rebuild
  pass    8    1800s  tool  wait: gate           Waiting for the gate to finish
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   83    1800s  tool  wait: other          {}
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass    7    1612s  tool  wait: tests          Waiting for test binary build
  pass    7    1611s  tool  wait: tests          Waiting for test binary link
  pass   10    1542s  tool  wait: gate           Waiting for the merged-tree gate
  pass    9    1481s  tool  wait: gate           Waiting for gate retry
  pass    7    1451s  model model                
  pass    4    1451s  tool  wait: gate           Waiting for gate step 1 retry
  pass   21    1449s  tool  wait: gate           Waiting for the gate
```

