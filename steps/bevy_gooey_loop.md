# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 01:34Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 55 | 2026-11-01T16:34Z | 7 | 0 | 8 | v0.14.0 | 10 | 1 | - |
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

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             6      44.7        44.8
```

## The last 12 passes

```text
12 pass(es), 4293 steps, 9.6 h of pass time

category                       hours  share  steps  mean s
model                           4.97  51.6%   1966     9.1
wait: gate                      1.52  15.7%     16   341.1
wait: other                     0.83   8.6%     40    75.0
cargo test                      0.66   6.9%    156    15.3
cargo build                     0.40   4.1%    105    13.7
shell                           0.35   3.6%    763     1.7
phase mark                      0.25   2.6%    110     8.2
wait: build                     0.16   1.6%      3   186.7
remote wait                     0.13   1.4%     15    31.7
read files                      0.07   0.8%    490     0.5
push gate                       0.06   0.6%     32     6.6
cargo clippy                    0.06   0.6%     31     6.7
github                          0.05   0.5%     50     3.4
wait: tests                     0.03   0.3%      2    57.8
cargo publish                   0.03   0.3%     10     9.4

phase                          hours  share  steps  mean s
gate                            2.70  28.0%    527    18.5
build                           2.61  27.1%   1329     7.1
prove                           2.55  26.5%   1339     6.9
research                        0.42   4.4%    215     7.1
choose                          0.40   4.2%    244     6.0
ship                            0.39   4.1%    248     5.7
release                         0.28   2.9%    181     5.6
orient                          0.14   1.5%    107     4.9
log                             0.12   1.3%     81     5.4
bugs                            0.01   0.1%      8     3.5

slowest single steps
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   66     766s  tool  wait: other          {}
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   62     282s  tool  wait: other          {}
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   56     203s  tool  wait: other          {}
  pass   59     201s  tool  wait: gate           Waiting for the full gate
  pass   59     185s  tool  wait: gate           Waiting for the gate
```

## Every pass recorded

```text
35 pass(es), 11683 steps, 35.3 h of pass time

category                       hours  share  steps  mean s
model                          13.82  39.2%   5223     9.5
wait: gate                      9.22  26.1%     62   535.6
wait: tests                     2.78   7.9%     30   334.0
wait: other                     2.24   6.3%     86    93.6
shell                           1.52   4.3%   1768     3.1
cargo test                      1.38   3.9%    332    14.9
cargo build                     0.91   2.6%    199    16.4
wait: clip                      0.78   2.2%     29    97.0
wait: build                     0.77   2.2%     15   185.6
phase mark                      0.43   1.2%    199     7.9
remote wait                     0.28   0.8%     30    33.9
read files                      0.19   0.5%   1622     0.4
eval                            0.17   0.5%      9    68.4
github                          0.12   0.3%    120     3.7
push gate                       0.11   0.3%     89     4.4

phase                          hours  share  steps  mean s
orient                         18.89  53.6%   4340    15.7
prove                           4.94  14.0%   2499     7.1
gate                            4.36  12.4%    744    21.1
build                           4.01  11.4%   2213     6.5
research                        0.84   2.4%    474     6.4
choose                          0.79   2.2%    423     6.7
ship                            0.61   1.7%    329     6.6
release                         0.51   1.4%    369     5.0
log                             0.25   0.7%    191     4.7
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

