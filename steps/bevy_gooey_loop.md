# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 04:46Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 70 | 2026-11-02T04:46Z | 8 | 0 | 4 | v0.24.1 | 5 | 0 | demo? |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            10      45.7        44.8
```

## The last 12 passes

```text
12 pass(es), 3955 steps, 9.7 h of pass time

category                       hours  share  steps  mean s
model                           4.79  49.4%   1855     9.3
wait: gate                      1.45  15.0%     17   307.7
cargo test                      0.83   8.5%    156    19.1
wait: other                     0.76   7.8%     37    73.8
cargo build                     0.50   5.2%    124    14.7
shell                           0.33   3.4%    745     1.6
wait: build                     0.28   2.9%      6   168.9
phase mark                      0.26   2.7%    117     8.1
remote wait                     0.10   1.0%     15    24.4
wait: tests                     0.08   0.8%      5    57.1
cargo clippy                    0.06   0.6%     28     7.3
read files                      0.06   0.6%    392     0.5
push gate                       0.05   0.5%     29     6.2
github                          0.05   0.5%     44     3.9
cargo publish                   0.04   0.4%     12    11.0

phase                          hours  share  steps  mean s
prove                           2.90  29.8%   1197     8.7
build                           2.86  29.5%   1394     7.4
gate                            2.48  25.5%    464    19.2
choose                          0.38   3.9%    245     5.6
release                         0.29   3.0%    164     6.3
research                        0.28   2.9%    148     6.9
ship                            0.24   2.5%    127     6.8
orient                          0.15   1.5%    106     5.0
log                             0.10   1.0%     67     5.3
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
39 pass(es), 12925 steps, 38.4 h of pass time

category                       hours  share  steps  mean s
model                          15.30  39.8%   5821     9.5
wait: gate                      9.68  25.2%     68   512.3
wait: tests                     2.83   7.4%     33   308.8
wait: other                     2.33   6.1%     90    93.4
cargo test                      1.74   4.5%    388    16.1
shell                           1.65   4.3%   2045     2.9
cargo build                     1.14   3.0%    258    15.9
wait: build                     0.91   2.4%     19   172.5
wait: clip                      0.79   2.1%     30    94.9
phase mark                      0.52   1.4%    244     7.7
remote wait                     0.29   0.8%     35    29.9
read files                      0.20   0.5%   1697     0.4
eval                            0.18   0.5%     10    64.6
github                          0.14   0.4%    135     3.8
push gate                       0.12   0.3%     99     4.3

phase                          hours  share  steps  mean s
orient                         18.91  49.2%   4363    15.6
prove                           6.06  15.8%   2858     7.6
build                           5.03  13.1%   2749     6.6
gate                            4.95  12.9%    813    21.9
research                        0.92   2.4%    516     6.4
choose                          0.91   2.4%    508     6.5
ship                            0.67   1.7%    363     6.6
release                         0.60   1.6%    419     5.1
log                             0.27   0.7%    209     4.7
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

