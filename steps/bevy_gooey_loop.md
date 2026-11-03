# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-03 12:18Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 85 | 2026-11-03T12:05Z | 7 | 0 | 5 | v0.32.2 | 4 | 0 | demo? |
| 86 | 2026-11-03T12:18Z | 7 | 0 | 3 | v0.32.3 | 4 | 0 | demo? |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            26      32.0        28.5
```

## The last 12 passes

```text
12 pass(es), 1843 steps, 4.2 h of pass time

category                       hours  share  steps  mean s
model                           1.61  38.0%    860     6.7
wait: other                     1.33  31.4%     28   171.3
cargo test                      0.44  10.4%     63    25.2
phase mark                      0.22   5.2%    103     7.7
cargo build                     0.19   4.4%     32    21.1
shell                           0.12   2.8%    363     1.2
wait: gate                      0.08   1.8%      2   139.9
cargo clippy                    0.06   1.5%     16    14.0
cargo publish                   0.05   1.3%     12    16.4
github                          0.04   0.9%     23     6.2
push gate                       0.03   0.7%     17     6.2
web                             0.02   0.5%      9     7.9
remote wait                     0.02   0.4%      1    60.0
read files                      0.01   0.3%    169     0.3
clip record/render              0.01   0.2%     20     1.8

phase                          hours  share  steps  mean s
gate                            1.55  36.5%    125    44.5
prove                           0.87  20.6%    597     5.3
build                           0.87  20.5%    487     6.4
research                        0.34   8.1%    211     5.9
choose                          0.25   5.9%    192     4.7
release                         0.16   3.7%     81     7.0
ship                            0.08   1.8%     36     7.7
orient                          0.06   1.5%     77     2.9
log                             0.05   1.2%     28     6.7
bugs                            0.00   0.1%      6     2.8

slowest single steps
  pass   83    1800s  tool  wait: other          {}
  pass   83     537s  tool  wait: other          {}
  pass   83     229s  tool  wait: other          {}
  pass   86     206s  tool  wait: other          {}
  pass   80     190s  tool  wait: other          {}
  pass   78     182s  tool  wait: other          {}
  pass   75     179s  tool  wait: other          {}
  pass   75     169s  tool  wait: other          {}
  pass   84     155s  tool  wait: other          {}
  pass   82     155s  tool  wait: gate           Waiting for gate
  pass   76     148s  tool  wait: other          {}
  pass   80     146s  bg    shell                bg_2 completed: G=$CARGO_TARGET_DIR/release/examples/inventory; cp $G /tmp/g52/inv_after &
  pass   85     139s  tool  wait: other          {}
  pass   79     138s  tool  wait: other          {}
  pass   76     132s  tool  wait: other          {}
```

## Every pass recorded

```text
55 pass(es), 15970 steps, 44.7 h of pass time

category                       hours  share  steps  mean s
model                          18.02  40.3%   7228     9.0
wait: gate                      9.86  22.0%     73   486.3
wait: other                     3.85   8.6%    126   110.1
wait: tests                     2.83   6.3%     33   308.8
cargo test                      2.36   5.3%    486    17.5
shell                           1.88   4.2%   2630     2.6
cargo build                     1.46   3.3%    311    16.9
wait: build                     0.91   2.0%     19   172.5
phase mark                      0.86   1.9%    391     7.9
wait: clip                      0.79   1.8%     30    94.9
remote wait                     0.31   0.7%     37    30.4
read files                      0.26   0.6%   2020     0.5
github                          0.18   0.4%    166     4.0
cargo clippy                    0.18   0.4%     75     8.6
eval                            0.18   0.4%     10    64.6

phase                          hours  share  steps  mean s
orient                         18.99  42.4%   4463    15.3
prove                           7.29  16.3%   3752     7.0
gate                            7.12  15.9%   1087    23.6
build                           6.33  14.1%   3516     6.5
research                        1.56   3.5%    973     5.8
choose                          1.25   2.8%    742     6.1
release                         0.92   2.1%    562     5.9
ship                            0.78   1.7%    449     6.3
log                             0.37   0.8%    281     4.8
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

