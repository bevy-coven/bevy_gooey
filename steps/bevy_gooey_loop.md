# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 06:09Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 62 | 2026-11-01T21:59Z | 4 | 0 | 1 | v0.20.1 | 1 | 0 | demo? |
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

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            13      41.5        39.5
```

## The last 12 passes

```text
12 pass(es), 3643 steps, 8.3 h of pass time

category                       hours  share  steps  mean s
model                           4.00  47.9%   1713     8.4
wait: gate                      1.10  13.2%     14   283.3
wait: other                     0.79   9.5%     37    77.2
cargo test                      0.76   9.1%    136    20.0
cargo build                     0.46   5.5%    113    14.7
shell                           0.35   4.2%    697     1.8
phase mark                      0.29   3.5%    126     8.3
wait: build                     0.14   1.6%      4   123.4
remote wait                     0.11   1.3%     15    25.8
read files                      0.08   1.0%    351     0.8
wait: tests                     0.05   0.6%      3    56.7
cargo publish                   0.05   0.6%     15    11.0
github                          0.04   0.5%     38     3.6
cargo clippy                    0.04   0.5%     20     6.8
push gate                       0.03   0.4%     26     4.4

phase                          hours  share  steps  mean s
build                           2.45  29.4%   1243     7.1
gate                            2.28  27.4%    455    18.1
prove                           2.10  25.1%    849     8.9
choose                          0.42   5.1%    282     5.4
research                        0.36   4.3%    268     4.9
release                         0.24   2.9%    158     5.6
ship                            0.24   2.9%    157     5.5
log                             0.11   1.4%     88     4.6
orient                          0.10   1.2%    102     3.6
bugs                            0.03   0.4%     29     3.9

slowest single steps
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   66     766s  tool  wait: other          {}
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   62     282s  tool  wait: other          {}
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   67     226s  tool  wait: build          Waiting for notifications example build
  pass   67     221s  tool  wait: build          Waiting for example build
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   65     160s  tool  wait: other          {}
  pass   66     155s  tool  wait: other          {}
  pass   64     153s  model model                
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   64     147s  tool  wait: gate           Waiting for the gate job
```

## Every pass recorded

```text
42 pass(es), 13726 steps, 39.8 h of pass time

category                       hours  share  steps  mean s
model                          15.98  40.1%   6186     9.3
wait: gate                      9.78  24.6%     71   496.0
wait: tests                     2.83   7.1%     33   308.8
wait: other                     2.44   6.1%     95    92.6
cargo test                      1.88   4.7%    418    16.2
shell                           1.76   4.4%   2184     2.9
cargo build                     1.24   3.1%    274    16.3
wait: build                     0.91   2.3%     19   172.5
wait: clip                      0.79   2.0%     30    94.9
phase mark                      0.60   1.5%    277     7.8
remote wait                     0.30   0.7%     36    29.6
read files                      0.24   0.6%   1797     0.5
eval                            0.18   0.5%     10    64.6
github                          0.14   0.4%    143     3.6
push gate                       0.12   0.3%    105     4.3

phase                          hours  share  steps  mean s
orient                         18.93  47.5%   4383    15.5
prove                           6.17  15.5%   2972     7.5
gate                            5.52  13.9%    950    20.9
build                           5.33  13.4%   2912     6.6
research                        1.14   2.9%    713     5.8
choose                          0.95   2.4%    545     6.3
ship                            0.70   1.8%    411     6.1
release                         0.66   1.7%    461     5.2
log                             0.32   0.8%    251     4.6
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

