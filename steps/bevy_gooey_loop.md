# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-03 13:17Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 87 | 2026-11-03T12:45Z | 8 | 0 | 4 | v0.32.4 | 7 | 1 | - |
| 88 | 2026-11-03T13:17Z | 9 | 0 | 6 | v0.33.0 | 7 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            28      31.8        28.5
```

## The last 12 passes

```text
12 pass(es), 1822 steps, 4.3 h of pass time

category                       hours  share  steps  mean s
model                           1.58  36.7%    857     6.7
wait: other                     1.45  33.7%     23   227.5
cargo test                      0.41   9.6%     58    25.7
cargo build                     0.23   5.4%     38    22.1
phase mark                      0.21   5.0%    100     7.7
shell                           0.10   2.4%    355     1.0
wait: gate                      0.08   1.8%      2   139.9
cargo clippy                    0.06   1.5%     16    14.1
cargo publish                   0.06   1.4%     13    17.3
push gate                       0.03   0.7%     17     6.3
web                             0.03   0.6%     11     8.8
remote wait                     0.02   0.4%      1    60.0
clip record/render              0.01   0.3%     23     2.3
read files                      0.01   0.2%    165     0.2
github                          0.01   0.2%     22     1.1

phase                          hours  share  steps  mean s
gate                            1.58  36.7%    104    54.8
prove                           1.01  23.3%    597     6.1
build                           0.77  17.8%    455     6.1
research                        0.41   9.5%    261     5.6
choose                          0.19   4.4%    173     3.9
release                         0.17   3.8%     81     7.3
ship                            0.08   1.9%     36     8.1
orient                          0.06   1.4%     77     2.8
log                             0.05   1.1%     28     6.4
bugs                            0.00   0.1%      6     2.8

slowest single steps
  pass   83    1800s  tool  wait: other          {}
  pass   87     732s  tool  wait: other          {}
  pass   83     537s  tool  wait: other          {}
  pass   88     298s  tool  wait: other          {}
  pass   83     229s  tool  wait: other          {}
  pass   88     222s  tool  wait: other          {}
  pass   86     206s  tool  wait: other          {}
  pass   80     190s  tool  wait: other          {}
  pass   78     182s  tool  wait: other          {}
  pass   84     155s  tool  wait: other          {}
  pass   82     155s  tool  wait: gate           Waiting for gate
  pass   80     146s  bg    shell                bg_2 completed: G=$CARGO_TARGET_DIR/release/examples/inventory; cp $G /tmp/g52/inv_after &
  pass   85     139s  tool  wait: other          {}
  pass   79     138s  tool  wait: other          {}
  pass   77     129s  tool  wait: other          {}
```

## Every pass recorded

```text
57 pass(es), 16382 steps, 45.7 h of pass time

category                       hours  share  steps  mean s
model                          18.39  40.2%   7428     8.9
wait: gate                      9.86  21.6%     73   486.3
wait: other                     4.24   9.3%    132   115.6
wait: tests                     2.83   6.2%     33   308.8
cargo test                      2.40   5.2%    495    17.5
shell                           1.90   4.2%   2722     2.5
cargo build                     1.56   3.4%    327    17.2
wait: build                     0.91   2.0%     19   172.5
phase mark                      0.89   2.0%    410     7.8
wait: clip                      0.79   1.7%     30    94.9
remote wait                     0.31   0.7%     37    30.4
read files                      0.26   0.6%   2043     0.5
github                          0.18   0.4%    170     3.9
cargo clippy                    0.18   0.4%     76     8.7
eval                            0.18   0.4%     11    59.6

phase                          hours  share  steps  mean s
orient                         19.00  41.6%   4475    15.3
prove                           7.64  16.7%   3916     7.0
gate                            7.41  16.2%   1105    24.1
build                           6.46  14.1%   3599     6.5
research                        1.69   3.7%   1059     5.7
choose                          1.26   2.8%    760     6.0
release                         0.96   2.1%    580     6.0
ship                            0.79   1.7%    453     6.3
log                             0.38   0.8%    289     4.7
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

