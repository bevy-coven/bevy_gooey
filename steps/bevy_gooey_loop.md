# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-04 13:05Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 90 | 2026-11-04T13:05Z | 7 | 0 | 8 | v0.35.0 | 7 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            30      31.9        28.5
```

## The last 12 passes

```text
12 pass(es), 1875 steps, 4.8 h of pass time

category                       hours  share  steps  mean s
wait: other                     1.65  34.4%     26   229.0
model                           1.55  32.3%    880     6.4
cargo test                      0.42   8.6%     53    28.2
wait: build                     0.25   5.3%      2   454.9
cargo build                     0.21   4.4%     39    19.6
phase mark                      0.19   4.0%     95     7.3
wait: gate                      0.13   2.7%      3   156.3
shell                           0.11   2.3%    367     1.1
cargo publish                   0.07   1.5%     12    22.2
cargo clippy                    0.05   1.0%     11    16.2
push gate                       0.04   0.9%     20     7.5
web                             0.03   0.7%     12    10.0
clip record/render              0.02   0.5%     17     5.2
wait: tests                     0.02   0.5%      1    78.1
remote wait                     0.02   0.3%      1    60.0

phase                          hours  share  steps  mean s
gate                            1.64  34.0%    115    51.3
prove                           1.10  22.8%    538     7.3
build                           1.07  22.3%    524     7.4
research                        0.39   8.1%    246     5.7
release                         0.22   4.5%     89     8.8
choose                          0.18   3.7%    182     3.5
orient                          0.10   2.1%     98     3.7
ship                            0.09   1.8%     47     6.6
log                             0.03   0.7%     26     4.3
bugs                            0.00   0.1%      6     2.8

slowest single steps
  pass   83    1800s  tool  wait: other          {}
  pass   90     910s  tool  wait: build          Waiting for example builds
  pass   87     732s  tool  wait: other          {}
  pass   83     537s  tool  wait: other          {}
  pass   90     417s  tool  wait: other          Waiting for mutation run
  pass   88     298s  tool  wait: other          {}
  pass   89     258s  tool  wait: other          {}
  pass   89     251s  tool  wait: other          {}
  pass   83     229s  tool  wait: other          {}
  pass   88     222s  tool  wait: other          {}
  pass   86     206s  tool  wait: other          {}
  pass   80     190s  tool  wait: other          {}
  pass   90     189s  tool  wait: gate           Waiting for gate
  pass   84     155s  tool  wait: other          {}
  pass   82     155s  tool  wait: gate           Waiting for gate
```

## Every pass recorded

```text
59 pass(es), 16750 steps, 46.8 h of pass time

category                       hours  share  steps  mean s
model                          18.66  39.8%   7594     8.8
wait: gate                      9.91  21.2%     74   482.3
wait: other                     4.54   9.7%    139   117.5
wait: tests                     2.85   6.1%     34   302.0
cargo test                      2.49   5.3%    502    17.8
shell                           1.92   4.1%   2788     2.5
cargo build                     1.57   3.4%    332    17.1
wait: build                     1.16   2.5%     21   199.4
phase mark                      0.93   2.0%    425     7.9
wait: clip                      0.79   1.7%     30    94.9
remote wait                     0.31   0.7%     37    30.4
read files                      0.26   0.6%   2084     0.4
cargo publish                   0.20   0.4%     58    12.2
github                          0.19   0.4%    173     3.9
cargo clippy                    0.18   0.4%     76     8.7

phase                          hours  share  steps  mean s
orient                         19.05  40.7%   4509    15.2
prove                           7.98  17.0%   4016     7.2
gate                            7.58  16.2%   1128    24.2
build                           6.88  14.7%   3731     6.6
research                        1.71   3.7%   1072     5.7
choose                          1.28   2.7%    784     5.9
release                         1.04   2.2%    600     6.2
ship                            0.80   1.7%    468     6.1
log                             0.39   0.8%    295     4.7
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

