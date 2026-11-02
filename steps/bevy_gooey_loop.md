# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 06:50Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 63 | 2026-11-01T23:06Z | 7 | 0 | 7 | v0.21.0 | 10 | 1 | - |
| 64 | 2026-11-01T23:53Z | 7 | 0 | 7 | - | 8 | 1 | release |
| 65 | 2026-11-02T00:33Z | 6 | 0 | 7 | - | 12 | 1 | release |
| 66 | 2026-11-02T01:34Z | 6 | 0 | 10 | - | 11 | 1 | release |
| 67 | 2026-11-02T02:36Z | 14 | 0 | 10 | - | 13 | 3 | release |
| 68 | 2026-11-02T03:45Z | 9 | 0 | 11 | v0.23.0 | 17 | 3 | - |
| 69 | 2026-11-02T04:18Z | 11 | 0 | 6 | v0.24.0 | 12 | 1 | - |
| 70 | 2026-11-02T04:46Z | 8 | 0 | 4 | v0.24.1 | 5 | 0 | demo? |
| 71 | 2026-11-02T04:59Z | 4 | 0 | 2 | v0.24.2 | 5 | 1 | - |
| 72 | 2026-11-02T05:30Z | 11 | 0 | 6 | v0.25.0,v0.25.1 | 9 | 1 | - |
| 73 | 2026-11-02T06:09Z | 11 | 0 | 8 | v0.26.0 | 10 | 1 | - |
| 74 | 2026-11-02T06:50Z | 7 | 0 | 8 | v0.27.0 | 8 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            14      41.3        39.2
```

## The last 12 passes

```text
12 pass(es), 3976 steps, 8.8 h of pass time

category                       hours  share  steps  mean s
model                           4.38  49.7%   1862     8.5
wait: gate                      1.10  12.5%     14   283.3
wait: other                     0.77   8.7%     38    72.9
cargo test                      0.77   8.7%    136    20.3
cargo build                     0.49   5.6%    118    15.1
shell                           0.36   4.1%    774     1.7
phase mark                      0.31   3.6%    131     8.6
wait: build                     0.14   1.6%      4   123.4
remote wait                     0.11   1.2%     15    25.8
read files                      0.08   0.9%    401     0.7
cargo publish                   0.05   0.5%     15    11.4
wait: tests                     0.05   0.5%      3    56.7
cargo clippy                    0.04   0.5%     22     6.7
github                          0.04   0.4%     38     3.6
push gate                       0.03   0.4%     25     4.5

phase                          hours  share  steps  mean s
build                           2.45  27.8%   1322     6.7
prove                           2.34  26.6%   1032     8.2
gate                            2.29  26.1%    461    17.9
choose                          0.47   5.3%    287     5.9
research                        0.44   5.0%    317     5.0
release                         0.35   4.0%    178     7.1
ship                            0.23   2.6%    149     5.5
log                             0.11   1.2%     86     4.5
orient                          0.09   1.0%     95     3.5
bugs                            0.03   0.4%     31     3.9

slowest single steps
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   66     766s  tool  wait: other          {}
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   74     308s  model model                
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   67     226s  tool  wait: build          Waiting for notifications example build
  pass   67     221s  tool  wait: build          Waiting for example build
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   65     160s  tool  wait: other          {}
  pass   74     155s  tool  wait: other          {}
  pass   66     155s  tool  wait: other          {}
  pass   64     153s  model model                
  pass   73     150s  tool  wait: gate           Waiting for the gate
```

## Every pass recorded

```text
43 pass(es), 14127 steps, 40.5 h of pass time

category                       hours  share  steps  mean s
model                          16.41  40.5%   6368     9.3
wait: gate                      9.78  24.2%     71   496.0
wait: tests                     2.83   7.0%     33   308.8
wait: other                     2.52   6.2%     98    92.6
cargo test                      1.92   4.7%    423    16.3
shell                           1.77   4.4%   2267     2.8
cargo build                     1.27   3.1%    279    16.4
wait: build                     0.91   2.2%     19   172.5
wait: clip                      0.79   2.0%     30    94.9
phase mark                      0.64   1.6%    288     8.0
remote wait                     0.30   0.7%     36    29.6
read files                      0.24   0.6%   1851     0.5
eval                            0.18   0.4%     10    64.6
github                          0.14   0.4%    143     3.6
push gate                       0.13   0.3%    106     4.3

phase                          hours  share  steps  mean s
orient                         18.93  46.7%   4386    15.5
prove                           6.42  15.8%   3155     7.3
gate                            5.58  13.8%    962    20.9
build                           5.46  13.5%   3029     6.5
research                        1.21   3.0%    762     5.7
choose                          1.00   2.5%    550     6.5
release                         0.76   1.9%    481     5.7
ship                            0.70   1.7%    413     6.1
log                             0.32   0.8%    253     4.6
bugs                            0.12   0.3%     65     6.5

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

