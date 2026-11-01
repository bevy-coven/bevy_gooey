# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 18:58Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 42 | 2026-11-01T05:37Z | 8 | 0 | 8 | v0.7.0 | 15 | 6 | - |
| 43 | 2026-11-01T06:52Z | 6 | 0 | 6 | v0.7.1 | 10 | 2 | - |
| 44 | 2026-11-01T08:41Z | 8 | 0 | 6 | v0.8.0 | 13 | 4 | - |
| 50 | 2026-11-01T13:40Z | 9 | 0 | 3 | v0.9.0 | 7 | 1 | - |
| 51 | 2026-11-01T14:03Z | 7 | 0 | 4 | v0.10.0 | 10 | 2 | - |
| 52 | 2026-11-01T14:34Z | 8 | 0 | 5 | v0.11.0 | 7 | 1 | - |
| 53 | 2026-11-01T15:21Z | 8 | 0 | 6 | v0.12.0 | 9 | 1 | - |
| 54 | 2026-11-01T15:54Z | 9 | 0 | 8 | v0.13.0 | 10 | 1 | - |
| 55 | 2026-11-01T16:34Z | 7 | 0 | 8 | v0.14.0 | 10 | 1 | - |
| 56 | 2026-11-01T17:07Z | 7 | 0 | 6 | v0.15.0 | 10 | 1 | - |
| 57 | 2026-11-01T18:21Z | 7 | 0 | 7 | v0.16.0 | 9 | 1 | - |
| 58 | 2026-11-01T18:58Z | 7 | 0 | 8 | v0.17.0 | 9 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             9      37.3        33.0
```

## The last 12 passes

```text
12 pass(es), 4354 steps, 9.4 h of pass time

category                       hours  share  steps  mean s
model                           4.30  46.0%   1948     7.9
wait: gate                      1.86  19.9%     16   418.2
wait: other                     0.94  10.1%     30   113.3
shell                           0.48   5.1%    675     2.6
cargo test                      0.47   5.0%    119    14.3
cargo build                     0.30   3.2%     68    15.9
phase mark                      0.25   2.7%    117     7.7
wait: tests                     0.16   1.7%      6    96.1
wait: clip                      0.11   1.2%      4   100.7
read files                      0.08   0.8%    598     0.5
github                          0.06   0.7%     55     4.1
wait: build                     0.06   0.6%      5    42.0
clip record/render              0.05   0.6%     37     5.2
remote wait                     0.05   0.5%      9    19.3
push gate                       0.04   0.4%     33     4.6

phase                          hours  share  steps  mean s
prove                           3.04  32.5%   1534     7.1
gate                            2.17  23.2%    290    26.9
build                           2.02  21.6%   1222     6.0
research                        0.63   6.8%    368     6.2
ship                            0.41   4.4%    220     6.7
choose                          0.41   4.4%    187     7.9
release                         0.25   2.7%    202     4.5
orient                          0.18   2.0%    159     4.2
log                             0.15   1.6%    122     4.4
bugs                            0.08   0.9%     34     8.9

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   43     309s  tool  wait: clip           Waiting for pixel-identity captures
  pass   42     306s  model model                
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   42     280s  tool  wait: gate           Waiting for the gate to finish
  pass   44     279s  tool  wait: tests          Waiting for test binaries
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   44     252s  tool  wait: other          Waiting for benchmark
  pass   44     221s  tool  wait: other          Waiting for repo clones
  pass   53     220s  tool  wait: other          Waiting for third perf run
  pass   50     216s  tool  wait: other          Waiting for perf runs
  pass   56     203s  tool  wait: other          {}
```

## Every pass recorded

```text
27 pass(es), 8970 steps, 28.7 h of pass time

category                       hours  share  steps  mean s
model                          10.51  36.6%   3966     9.5
wait: gate                      8.22  28.6%     51   580.5
wait: tests                     2.75   9.6%     28   353.8
wait: other                     1.58   5.5%     53   107.1
shell                           1.32   4.6%   1300     3.7
cargo test                      0.91   3.2%    232    14.1
wait: clip                      0.78   2.7%     29    97.0
cargo build                     0.64   2.2%    134    17.1
wait: build                     0.63   2.2%     13   174.1
phase mark                      0.26   0.9%    127     7.3
remote wait                     0.19   0.7%     20    34.0
eval                            0.17   0.6%      9    68.4
read files                      0.15   0.5%   1305     0.4
edit files                      0.10   0.3%    935     0.4
github                          0.09   0.3%     91     3.7

phase                          hours  share  steps  mean s
orient                         18.77  65.3%   4257    15.9
prove                           3.16  11.0%   1661     6.8
gate                            2.47   8.6%    349    25.5
build                           2.17   7.6%   1355     5.8
research                        0.63   2.2%    368     6.2
choose                          0.53   1.8%    263     7.3
ship                            0.43   1.5%    236     6.5
release                         0.31   1.1%    255     4.4
log                             0.17   0.6%    142     4.4
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
  pass   57    1434s  tool  wait: gate           Waiting for the gate
```

