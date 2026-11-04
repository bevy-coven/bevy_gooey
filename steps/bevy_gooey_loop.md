# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-04 12:18Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 78 | 2026-11-02T08:29Z | 5 | 0 | 5 | v0.30.0 | 7 | 1 | - |
| 79 | 2026-11-02T16:56Z | 5 | 0 | 5 | v0.31.0 | 7 | 1 | - |
| 80 | 2026-11-02T17:23Z | 5 | 0 | 4 | v0.31.1 | 9 | 2 | - |
| 81 | 2026-11-02T17:37Z | 8 | 0 | 4 | v0.31.2 | 4 | 0 | demo? |
| 82 | 2026-11-02T17:53Z | 6 | 0 | 5 | v0.31.3 | 6 | 1 | - |
| 83 | 2026-11-02T18:55Z | 7 | 0 | 5 | v0.32.0 | 6 | 1 | - |
| 84 | 2026-11-03T11:52Z | 5 | 0 | 2 | v0.32.1 | 2 | 0 | demo? |
| 85 | 2026-11-03T12:05Z | 7 | 0 | 5 | v0.32.2 | 4 | 0 | demo? |
| 86 | 2026-11-03T12:18Z | 7 | 0 | 3 | v0.32.3 | 4 | 0 | demo? |
| 87 | 2026-11-03T12:45Z | 8 | 0 | 4 | v0.32.4 | 7 | 1 | - |
| 88 | 2026-11-03T13:17Z | 9 | 0 | 6 | v0.33.0 | 7 | 1 | - |
| 89 | 2026-11-04T12:18Z | 6 | 0 | 4 | v0.34.0 | 2 | 0 | demo? |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            29      31.4        27.2
```

## The last 12 passes

```text
12 pass(es), 1744 steps, 4.4 h of pass time

category                       hours  share  steps  mean s
wait: other                     1.60  36.6%     27   212.8
model                           1.48  34.0%    826     6.5
cargo test                      0.43   9.9%     59    26.3
phase mark                      0.22   5.1%     97     8.3
cargo build                     0.22   5.0%     37    21.1
shell                           0.10   2.2%    329     1.0
wait: gate                      0.08   1.8%      2   139.9
cargo publish                   0.08   1.7%     14    19.5
cargo clippy                    0.06   1.3%     14    14.5
push gate                       0.03   0.7%     17     6.5
web                             0.03   0.6%     11     8.8
remote wait                     0.02   0.4%      1    60.0
clip record/render              0.01   0.3%     23     2.3
read files                      0.01   0.3%    146     0.3
github                          0.01   0.2%     22     1.2

phase                          hours  share  steps  mean s
gate                            1.64  37.6%    113    52.2
prove                           0.89  20.4%    508     6.3
build                           0.86  19.7%    448     6.9
research                        0.39   8.9%    248     5.7
release                         0.21   4.8%     87     8.7
choose                          0.19   4.4%    190     3.6
ship                            0.08   1.8%     36     8.0
orient                          0.06   1.4%     78     2.7
log                             0.04   0.9%     26     5.6
bugs                            0.00   0.1%      6     2.8

slowest single steps
  pass   83    1800s  tool  wait: other          {}
  pass   87     732s  tool  wait: other          {}
  pass   83     537s  tool  wait: other          {}
  pass   88     298s  tool  wait: other          {}
  pass   89     258s  tool  wait: other          {}
  pass   89     251s  tool  wait: other          {}
  pass   83     229s  tool  wait: other          {}
  pass   88     222s  tool  wait: other          {}
  pass   86     206s  tool  wait: other          {}
  pass   80     190s  tool  wait: other          {}
  pass   78     182s  tool  wait: other          {}
  pass   84     155s  tool  wait: other          {}
  pass   82     155s  tool  wait: gate           Waiting for gate
  pass   80     146s  bg    shell                bg_2 completed: G=$CARGO_TARGET_DIR/release/examples/inventory; cp $G /tmp/g52/inv_after &
  pass   85     139s  tool  wait: other          {}
```

## Every pass recorded

```text
58 pass(es), 16475 steps, 46.0 h of pass time

category                       hours  share  steps  mean s
model                          18.43  40.0%   7472     8.9
wait: gate                      9.86  21.4%     73   486.3
wait: other                     4.42   9.6%    138   115.3
wait: tests                     2.83   6.1%     33   308.8
cargo test                      2.45   5.3%    499    17.7
shell                           1.90   4.1%   2730     2.5
cargo build                     1.56   3.4%    327    17.2
phase mark                      0.93   2.0%    417     8.0
wait: build                     0.91   2.0%     19   172.5
wait: clip                      0.79   1.7%     30    94.9
remote wait                     0.31   0.7%     37    30.4
read files                      0.26   0.6%   2051     0.5
cargo publish                   0.20   0.4%     58    12.2
github                          0.19   0.4%    171     3.9
cargo clippy                    0.18   0.4%     76     8.7

phase                          hours  share  steps  mean s
orient                         19.00  41.3%   4481    15.3
prove                           7.64  16.6%   3916     7.0
gate                            7.53  16.3%   1118    24.2
build                           6.59  14.3%   3631     6.5
research                        1.69   3.7%   1059     5.7
choose                          1.28   2.8%    784     5.9
release                         1.02   2.2%    594     6.2
ship                            0.79   1.7%    455     6.2
log                             0.38   0.8%    291     4.7
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

