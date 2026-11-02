# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 03:45Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 57 | 2026-11-01T18:21Z | 7 | 0 | 7 | v0.16.0 | 9 | 1 | - |
| 58 | 2026-11-01T18:58Z | 7 | 0 | 8 | v0.17.0 | 9 | 1 | - |
| 59 | 2026-11-01T20:09Z | 8 | 0 | 9 | v0.18.0 | 9 | 1 | - |
| 60 | 2026-11-01T21:02Z | 10 | 0 | 7 | v0.19.0 | 8 | 1 | - |
| 61 | 2026-11-01T21:47Z | 10 | 0 | 8 | v0.20.0 | 9 | 1 | - |
| 62 | 2026-11-01T21:59Z | 4 | 0 | 1 | v0.20.1 | 1 | 0 | demo? |
| 63 | 2026-11-01T23:06Z | 7 | 0 | 7 | v0.21.0 | 10 | 1 | - |
| 64 | 2026-11-01T23:53Z | 7 | 0 | 7 | - | 8 | 1 | release |
| 65 | 2026-11-02T00:33Z | 6 | 0 | 7 | - | 12 | 1 | release |
| 66 | 2026-11-02T01:34Z | 6 | 0 | 10 | - | 11 | 1 | release |
| 67 | 2026-11-02T02:36Z | 14 | 0 | 10 | - | 13 | 3 | release |
| 68 | 2026-11-02T03:45Z | 9 | 0 | 11 | v0.23.0 | 17 | 3 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             8      49.6        54.0
```

## The last 12 passes

```text
12 pass(es), 4403 steps, 10.6 h of pass time

category                       hours  share  steps  mean s
model                           5.20  49.2%   2030     9.2
wait: gate                      1.89  17.8%     20   339.7
cargo test                      0.81   7.6%    157    18.5
wait: other                     0.74   7.0%     36    73.7
cargo build                     0.50   4.7%    126    14.2
shell                           0.35   3.3%    802     1.6
wait: build                     0.29   2.8%      7   150.5
phase mark                      0.28   2.7%    115     8.8
remote wait                     0.12   1.1%     14    30.4
wait: tests                     0.08   0.7%      4    70.0
read files                      0.07   0.7%    485     0.5
cargo clippy                    0.05   0.5%     28     7.0
push gate                       0.05   0.5%     29     6.5
github                          0.04   0.4%     47     3.4
cargo publish                   0.04   0.3%     13     9.8

phase                          hours  share  steps  mean s
prove                           3.16  29.9%   1457     7.8
gate                            3.06  28.9%    545    20.2
build                           2.62  24.8%   1304     7.2
choose                          0.45   4.3%    281     5.8
ship                            0.41   3.9%    262     5.7
research                        0.31   2.9%    161     6.9
release                         0.30   2.8%    189     5.7
orient                          0.15   1.4%    107     4.9
log                             0.11   1.1%     75     5.4
bugs                            0.01   0.1%      8     3.5

slowest single steps
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   66     766s  tool  wait: other          {}
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   62     282s  tool  wait: other          {}
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   67     226s  tool  wait: build          Waiting for notifications example build
```

## Every pass recorded

```text
37 pass(es), 12378 steps, 37.4 h of pass time

category                       hours  share  steps  mean s
model                          14.77  39.5%   5563     9.6
wait: gate                      9.59  25.6%     66   523.4
wait: tests                     2.83   7.6%     32   318.3
wait: other                     2.29   6.1%     88    93.9
cargo test                      1.63   4.3%    369    15.9
shell                           1.58   4.2%   1921     3.0
cargo build                     1.05   2.8%    237    16.0
wait: build                     0.91   2.4%     19   172.5
wait: clip                      0.78   2.1%     29    97.0
phase mark                      0.50   1.3%    223     8.1
remote wait                     0.29   0.8%     33    31.7
read files                      0.20   0.5%   1663     0.4
eval                            0.17   0.5%      9    68.4
github                          0.13   0.4%    128     3.7
push gate                       0.11   0.3%     92     4.3

phase                          hours  share  steps  mean s
orient                         18.91  50.5%   4354    15.6
prove                           5.88  15.7%   2809     7.5
gate                            4.83  12.9%    779    22.3
build                           4.51  12.1%   2422     6.7
research                        0.89   2.4%    499     6.4
choose                          0.83   2.2%    460     6.5
ship                            0.66   1.8%    361     6.6
release                         0.56   1.5%    397     5.1
log                             0.25   0.7%    193     4.7
bugs                            0.09   0.2%     42     7.8

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

