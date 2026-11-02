# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 02:36Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 56 | 2026-11-01T17:07Z | 7 | 0 | 6 | v0.15.0 | 10 | 1 | - |
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

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             7      47.2        47.1
```

## The last 12 passes

```text
12 pass(es), 4368 steps, 10.0 h of pass time

category                       hours  share  steps  mean s
model                           4.97  49.7%   2005     8.9
wait: gate                      1.63  16.3%     18   326.0
wait: other                     0.80   8.0%     38    75.9
cargo test                      0.72   7.2%    150    17.3
cargo build                     0.45   4.5%    113    14.2
shell                           0.35   3.5%    783     1.6
wait: build                     0.29   2.9%      6   172.0
phase mark                      0.27   2.7%    112     8.8
remote wait                     0.14   1.3%     14    34.7
wait: tests                     0.08   0.8%      4    70.0
read files                      0.07   0.7%    498     0.5
cargo clippy                    0.06   0.6%     31     7.0
push gate                       0.06   0.6%     30     6.7
github                          0.04   0.4%     49     3.0
web                             0.03   0.3%     12     8.2

phase                          hours  share  steps  mean s
prove                           3.10  31.0%   1492     7.5
gate                            2.82  28.2%    532    19.1
build                           2.38  23.8%   1256     6.8
choose                          0.45   4.5%    276     5.8
ship                            0.37   3.7%    240     5.6
research                        0.31   3.1%    170     6.7
release                         0.31   3.0%    191     5.8
orient                          0.14   1.4%    108     4.8
log                             0.12   1.2%     79     5.4
bugs                            0.01   0.1%      8     3.5

slowest single steps
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   63    1427s  tool  wait: gate           Waiting for the gate
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
  pass   67     221s  tool  wait: build          Waiting for example build
```

## Every pass recorded

```text
36 pass(es), 12038 steps, 36.3 h of pass time

category                       hours  share  steps  mean s
model                          14.24  39.2%   5395     9.5
wait: gate                      9.34  25.7%     64   525.2
wait: tests                     2.83   7.8%     32   318.3
wait: other                     2.26   6.2%     87    93.5
shell                           1.55   4.3%   1848     3.0
cargo test                      1.50   4.1%    350    15.5
cargo build                     0.98   2.7%    218    16.2
wait: build                     0.90   2.5%     18   180.8
wait: clip                      0.78   2.2%     29    97.0
phase mark                      0.47   1.3%    209     8.2
remote wait                     0.29   0.8%     32    32.7
read files                      0.20   0.5%   1646     0.4
eval                            0.17   0.5%      9    68.4
github                          0.13   0.3%    124     3.7
push gate                       0.11   0.3%     90     4.4

phase                          hours  share  steps  mean s
orient                         18.90  52.0%   4346    15.7
prove                           5.61  15.4%   2713     7.4
gate                            4.54  12.5%    761    21.5
build                           4.11  11.3%   2277     6.5
research                        0.84   2.3%    474     6.4
choose                          0.83   2.3%    455     6.6
ship                            0.62   1.7%    337     6.6
release                         0.53   1.5%    379     5.1
log                             0.25   0.7%    193     4.7
bugs                            0.09   0.3%     42     7.8

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

