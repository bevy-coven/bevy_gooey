# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 16:34Z.

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
| 55 | 2026-11-01T16:34Z | 7 | 0 | 8 | v0.14.0 | 10 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             6      32.1        32.1
```

## The last 12 passes

```text
12 pass(es), 3979 steps, 8.7 h of pass time

category                       hours  share  steps  mean s
model                           3.85  44.3%   1775     7.8
wait: gate                      1.91  22.0%     14   491.9
wait: other                     0.93  10.7%     30   111.6
shell                           0.42   4.8%    599     2.5
cargo test                      0.38   4.4%     96    14.3
cargo build                     0.25   2.9%     48    18.7
phase mark                      0.20   2.3%     97     7.4
wait: clip                      0.17   1.9%      5   120.2
wait: tests                     0.16   1.8%      6    96.1
read files                      0.06   0.7%    583     0.4
clip record/render              0.06   0.7%     41     5.4
github                          0.06   0.6%     43     4.7
wait: build                     0.05   0.5%      6    28.3
edit files                      0.04   0.5%    402     0.4
push gate                       0.04   0.4%     33     3.9

phase                          hours  share  steps  mean s
prove                           2.51  28.8%   1221     7.4
build                           1.73  19.9%   1021     6.1
gate                            1.72  19.7%    229    27.0
orient                          1.15  13.2%    577     7.2
research                        0.53   6.0%    304     6.2
choose                          0.38   4.4%    179     7.7
ship                            0.25   2.8%     97     9.2
release                         0.23   2.6%    188     4.3
log                             0.13   1.5%    114     4.2
bugs                            0.08   1.0%     34     8.9

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
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
  pass   53     220s  tool  wait: other          Waiting for third perf run
  pass   50     216s  tool  wait: other          Waiting for perf runs
```

## Every pass recorded

```text
24 pass(es), 7670 steps, 26.3 h of pass time

category                       hours  share  steps  mean s
model                           9.27  35.3%   3390     9.8
wait: gate                      7.71  29.3%     46   603.2
wait: tests                     2.75  10.5%     28   353.8
wait: other                     1.46   5.5%     49   107.2
shell                           1.20   4.6%   1065     4.1
wait: clip                      0.78   3.0%     29    97.0
cargo test                      0.78   3.0%    200    14.0
wait: build                     0.62   2.3%     12   185.3
cargo build                     0.53   2.0%    105    18.2
phase mark                      0.20   0.8%     97     7.4
eval                            0.17   0.7%      9    68.4
remote wait                     0.16   0.6%     18    31.1
read files                      0.13   0.5%   1148     0.4
edit files                      0.10   0.4%    773     0.5
github                          0.09   0.3%     75     4.2

phase                          hours  share  steps  mean s
orient                         18.75  71.3%   4238    15.9
prove                           2.51   9.5%   1221     7.4
build                           1.73   6.6%   1021     6.1
gate                            1.72   6.5%    229    27.0
research                        0.53   2.0%    304     6.2
choose                          0.38   1.5%    179     7.7
ship                            0.25   0.9%     97     9.2
release                         0.23   0.9%    188     4.3
log                             0.13   0.5%    114     4.2
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

