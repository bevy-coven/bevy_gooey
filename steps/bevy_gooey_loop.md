# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 04:18Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 69 | 2026-11-02T04:18Z | 11 | 0 | 6 | v0.24.0 | 12 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             9      47.8        47.1
```

## The last 12 passes

```text
12 pass(es), 4193 steps, 9.9 h of pass time

category                       hours  share  steps  mean s
model                           4.93  49.9%   1954     9.1
wait: gate                      1.45  14.7%     17   307.7
cargo test                      0.81   8.2%    159    18.4
wait: other                     0.76   7.7%     37    73.8
cargo build                     0.50   5.1%    129    13.9
shell                           0.35   3.6%    765     1.7
wait: build                     0.29   3.0%      7   150.5
phase mark                      0.29   2.9%    117     8.9
remote wait                     0.10   1.0%     14    26.2
wait: tests                     0.08   0.8%      4    70.0
cargo clippy                    0.06   0.6%     30     7.0
read files                      0.06   0.6%    449     0.5
push gate                       0.05   0.5%     29     6.4
github                          0.05   0.5%     47     3.8
cargo publish                   0.03   0.4%     12    10.4

phase                          hours  share  steps  mean s
prove                           2.95  29.8%   1273     8.3
build                           2.77  28.1%   1392     7.2
gate                            2.49  25.2%    465    19.3
ship                            0.41   4.2%    260     5.7
choose                          0.38   3.9%    256     5.4
release                         0.29   2.9%    169     6.1
research                        0.29   2.9%    149     6.9
orient                          0.15   1.5%    107     5.0
log                             0.11   1.2%     80     5.2
bugs                            0.03   0.3%     29     3.9

slowest single steps
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   66     766s  tool  wait: other          {}
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   62     282s  tool  wait: other          {}
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   67     226s  tool  wait: build          Waiting for notifications example build
  pass   67     221s  tool  wait: build          Waiting for example build
  pass   59     201s  tool  wait: gate           Waiting for the full gate
```

## Every pass recorded

```text
38 pass(es), 12719 steps, 38.0 h of pass time

category                       hours  share  steps  mean s
model                          15.06  39.7%   5720     9.5
wait: gate                      9.63  25.4%     67   517.5
wait: tests                     2.83   7.5%     32   318.3
wait: other                     2.32   6.1%     89    93.7
cargo test                      1.69   4.5%    381    16.0
shell                           1.61   4.2%   1985     2.9
cargo build                     1.11   2.9%    251    15.9
wait: build                     0.91   2.4%     19   172.5
wait: clip                      0.79   2.1%     30    94.9
phase mark                      0.52   1.4%    235     7.9
remote wait                     0.29   0.8%     34    30.8
read files                      0.20   0.5%   1695     0.4
eval                            0.17   0.5%      9    68.4
github                          0.14   0.4%    133     3.8
push gate                       0.11   0.3%     96     4.3

phase                          hours  share  steps  mean s
orient                         18.91  49.8%   4358    15.6
prove                           5.97  15.7%   2834     7.6
gate                            4.88  12.9%    794    22.1
build                           4.83  12.7%   2653     6.6
research                        0.89   2.3%    499     6.4
choose                          0.86   2.3%    478     6.5
ship                            0.67   1.8%    363     6.6
release                         0.58   1.5%    411     5.1
log                             0.27   0.7%    203     4.7
bugs                            0.12   0.3%     63     6.6

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

