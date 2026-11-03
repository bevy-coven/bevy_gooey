# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-03 11:52Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 84 | 2026-11-03T11:52Z | 5 | 0 | 2 | v0.32.1 | 2 | 0 | demo? |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            24      33.6        30.0
```

## The last 12 passes

```text
12 pass(es), 2324 steps, 5.2 h of pass time

category                       hours  share  steps  mean s
model                           2.21  42.9%   1089     7.3
wait: other                     1.35  26.2%     30   162.5
cargo test                      0.48   9.4%     73    23.8
cargo build                     0.29   5.6%     49    21.3
phase mark                      0.25   4.8%    112     8.0
shell                           0.15   2.9%    474     1.2
wait: gate                      0.12   2.3%      3   143.3
cargo clippy                    0.06   1.2%     16    13.4
cargo publish                   0.05   1.0%     11    16.2
read files                      0.04   0.8%    232     0.6
github                          0.04   0.8%     21     6.7
push gate                       0.03   0.6%     17     7.0
remote wait                     0.02   0.4%      2    40.0
web                             0.02   0.4%      9     8.2
git                             0.02   0.3%     21     3.0

phase                          hours  share  steps  mean s
gate                            1.82  35.2%    223    29.3
prove                           1.10  21.4%    772     5.2
build                           0.96  18.7%    525     6.6
research                        0.52  10.0%    341     5.4
choose                          0.29   5.6%    180     5.7
release                         0.26   5.0%     98     9.5
log                             0.09   1.7%     62     5.0
ship                            0.07   1.4%     36     7.2
orient                          0.05   1.0%     70     2.6
bugs                            0.01   0.1%      8     3.1

slowest single steps
  pass   83    1800s  tool  wait: other          {}
  pass   83     537s  tool  wait: other          {}
  pass   74     308s  model model                
  pass   83     229s  tool  wait: other          {}
  pass   80     190s  tool  wait: other          {}
  pass   78     182s  tool  wait: other          {}
  pass   75     179s  tool  wait: other          {}
  pass   75     169s  tool  wait: other          {}
  pass   84     155s  tool  wait: other          {}
  pass   74     155s  tool  wait: other          {}
  pass   82     155s  tool  wait: gate           Waiting for gate
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   76     148s  tool  wait: other          {}
  pass   80     146s  bg    shell                bg_2 completed: G=$CARGO_TARGET_DIR/release/examples/inventory; cp $G /tmp/g52/inv_after &
  pass   79     138s  tool  wait: other          {}
```

## Every pass recorded

```text
53 pass(es), 15698 steps, 44.3 h of pass time

category                       hours  share  steps  mean s
model                          17.84  40.2%   7106     9.0
wait: gate                      9.86  22.2%     73   486.3
wait: other                     3.75   8.5%    122   110.5
wait: tests                     2.83   6.4%     33   308.8
cargo test                      2.31   5.2%    478    17.4
shell                           1.87   4.2%   2583     2.6
cargo build                     1.46   3.3%    311    16.9
wait: build                     0.91   2.1%     19   172.5
phase mark                      0.84   1.9%    377     8.0
wait: clip                      0.79   1.8%     30    94.9
remote wait                     0.31   0.7%     37    30.4
read files                      0.26   0.6%   1995     0.5
github                          0.18   0.4%    162     4.0
eval                            0.18   0.4%     10    64.6
cargo clippy                    0.17   0.4%     73     8.6

phase                          hours  share  steps  mean s
orient                         18.97  42.8%   4445    15.4
prove                           7.26  16.4%   3734     7.0
gate                            7.00  15.8%   1076    23.4
build                           6.19  14.0%   3385     6.6
research                        1.51   3.4%    924     5.9
choose                          1.23   2.8%    721     6.2
release                         0.89   2.0%    545     5.9
ship                            0.77   1.7%    445     6.2
log                             0.37   0.8%    279     4.8
bugs                            0.12   0.3%     71     6.2

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

