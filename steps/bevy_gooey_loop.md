# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 15:54Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 0 | 2026-11-01T00:31Z | 16 | 0 | 5 | - | 12 | 2 | release |
| 39 | 2026-11-01T03:31Z | 10 | 0 | 7 | v0.5.0 | 10 | 2 | - |
| 40 | 2026-11-01T04:18Z | 8 | 0 | 7 | v0.6.0 | 12 | 3 | - |
| 42 | 2026-11-01T05:37Z | 8 | 0 | 8 | v0.7.0 | 15 | 6 | - |
| 43 | 2026-11-01T06:52Z | 6 | 0 | 6 | v0.7.1 | 10 | 2 | - |
| 44 | 2026-11-01T08:41Z | 8 | 0 | 6 | v0.8.0 | 13 | 4 | - |
| 50 | 2026-11-01T13:40Z | 9 | 0 | 3 | v0.9.0 | 7 | 1 | - |
| 51 | 2026-11-01T14:03Z | 7 | 0 | 4 | v0.10.0 | 10 | 2 | - |
| 52 | 2026-11-01T14:34Z | 8 | 0 | 5 | v0.11.0 | 7 | 1 | - |
| 53 | 2026-11-01T15:21Z | 8 | 0 | 6 | v0.12.0 | 9 | 1 | - |
| 54 | 2026-11-01T15:54Z | 9 | 0 | 8 | v0.13.0 | 10 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             5      30.6        31.1
```

## The last 12 passes

```text
12 pass(es), 3959 steps, 9.3 h of pass time

category                       hours  share  steps  mean s
model                           3.66  39.3%   1754     7.5
wait: gate                      2.79  29.9%     16   627.1
wait: other                     0.93  10.0%     28   119.4
shell                           0.45   4.9%    566     2.9
cargo test                      0.31   3.4%     75    15.1
cargo build                     0.23   2.5%     39    21.2
wait: clip                      0.19   2.1%      7   100.0
phase mark                      0.18   2.0%     89     7.4
wait: tests                     0.16   1.7%      7    82.9
clip record/render              0.07   0.7%     44     5.4
read files                      0.05   0.6%    604     0.3
wait: build                     0.05   0.5%      6    28.3
github                          0.05   0.5%     40     4.1
edit files                      0.04   0.5%    423     0.4
cargo publish                   0.04   0.4%     16     8.6

phase                          hours  share  steps  mean s
orient                          2.42  26.0%    830    10.5
prove                           2.38  25.6%   1160     7.4
gate                            1.66  17.8%    217    27.5
build                           1.40  15.0%    884     5.7
research                        0.42   4.5%    259     5.8
choose                          0.38   4.1%    179     7.7
release                         0.23   2.4%    188     4.3
ship                            0.21   2.3%     81     9.5
log                             0.13   1.4%    110     4.2
bugs                            0.08   0.9%     34     8.9

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   39     959s  tool  wait: gate           Waiting for the final gate
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   40     653s  tool  wait: gate           Waiting for the gate to finish
  pass   39     443s  tool  wait: gate           Waiting for gate to finish
  pass   43     309s  tool  wait: clip           Waiting for pixel-identity captures
  pass   42     306s  model model                
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   42     280s  tool  wait: gate           Waiting for the gate to finish
  pass   44     279s  tool  wait: tests          Waiting for test binaries
  pass   44     252s  tool  wait: other          Waiting for benchmark
  pass   44     221s  tool  wait: other          Waiting for repo clones
```

## Every pass recorded

```text
23 pass(es), 7390 steps, 25.6 h of pass time

category                       hours  share  steps  mean s
model                           8.85  34.5%   3257     9.8
wait: gate                      7.71  30.1%     46   603.2
wait: tests                     2.75  10.7%     28   353.8
wait: other                     1.40   5.5%     46   109.8
shell                           1.17   4.5%   1005     4.2
wait: clip                      0.78   3.0%     29    97.0
cargo test                      0.71   2.8%    176    14.6
wait: build                     0.62   2.4%     12   185.3
cargo build                     0.51   2.0%     94    19.5
phase mark                      0.18   0.7%     89     7.4
eval                            0.17   0.7%      9    68.4
remote wait                     0.15   0.6%     15    36.2
read files                      0.12   0.5%   1132     0.4
edit files                      0.10   0.4%    772     0.5
github                          0.08   0.3%     70     3.9

phase                          hours  share  steps  mean s
orient                         18.75  73.1%   4233    15.9
prove                           2.38   9.3%   1160     7.4
gate                            1.66   6.5%    217    27.5
build                           1.40   5.4%    884     5.7
research                        0.42   1.6%    259     5.8
choose                          0.38   1.5%    179     7.7
release                         0.23   0.9%    188     4.3
ship                            0.21   0.8%     81     9.5
log                             0.13   0.5%    110     4.2
bugs                            0.08   0.3%     34     8.9

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
  pass    2    1393s  tool  wait: build          Waiting for baseline build
```

