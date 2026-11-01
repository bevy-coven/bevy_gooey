# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 15:21Z.

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

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             4      30.0        26.9
```

## The last 12 passes

```text
12 pass(es), 3960 steps, 9.6 h of pass time

category                       hours  share  steps  mean s
model                           3.79  39.5%   1751     7.8
wait: gate                      3.03  31.6%     18   605.4
wait: other                     0.88   9.2%     26   122.2
shell                           0.45   4.7%    568     2.8
cargo test                      0.33   3.4%     75    15.7
cargo build                     0.23   2.4%     38    21.8
wait: clip                      0.19   2.0%      8    87.5
phase mark                      0.17   1.8%     78     8.1
wait: tests                     0.16   1.7%      7    82.9
clip record/render              0.07   0.7%     43     5.5
wait: build                     0.05   0.5%      6    28.3
edit files                      0.04   0.5%    414     0.4
github                          0.04   0.4%     35     4.0
read files                      0.04   0.4%    641     0.2
cargo publish                   0.03   0.3%     15     8.0

phase                          hours  share  steps  mean s
orient                          3.26  34.0%   1191     9.9
prove                           2.12  22.1%    982     7.8
gate                            1.59  16.5%    199    28.7
build                           1.27  13.2%    785     5.8
research                        0.39   4.1%    244     5.8
choose                          0.37   3.9%    173     7.7
release                         0.23   2.4%    188     4.3
ship                            0.19   1.9%     60    11.1
log                             0.10   1.0%     86     4.2
bugs                            0.08   0.9%     34     8.9

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   21    1699s  tool  wait: gate           Waiting for merged-tree gate
  pass   21    1449s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   39     959s  tool  wait: gate           Waiting for the final gate
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   40     653s  tool  wait: gate           Waiting for the gate to finish
  pass   12     562s  tool  wait: gate           Waiting for the gate
  pass   39     443s  tool  wait: gate           Waiting for gate to finish
  pass   43     309s  tool  wait: clip           Waiting for pixel-identity captures
  pass   42     306s  model model                
  pass   12     302s  tool  wait: gate           Waiting for the gate to finish
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   42     280s  tool  wait: gate           Waiting for the gate to finish
  pass   44     279s  tool  wait: tests          Waiting for test binaries
```

## Every pass recorded

```text
22 pass(es), 7012 steps, 25.1 h of pass time

category                       hours  share  steps  mean s
model                           8.50  33.9%   3088     9.9
wait: gate                      7.71  30.8%     46   603.2
wait: tests                     2.75  11.0%     28   353.8
wait: other                     1.36   5.4%     44   111.0
shell                           1.11   4.4%    957     4.2
wait: clip                      0.78   3.1%     29    97.0
cargo test                      0.69   2.7%    166    14.9
wait: build                     0.62   2.5%     12   185.3
cargo build                     0.46   1.8%     85    19.6
phase mark                      0.17   0.7%     78     8.1
eval                            0.17   0.7%      9    68.4
remote wait                     0.15   0.6%     14    37.3
edit files                      0.10   0.4%    720     0.5
read files                      0.10   0.4%   1086     0.3
github                          0.07   0.3%     64     3.9

phase                          hours  share  steps  mean s
orient                         18.73  74.7%   4218    16.0
prove                           2.12   8.5%    982     7.8
gate                            1.59   6.3%    199    28.7
build                           1.27   5.1%    785     5.8
research                        0.39   1.6%    244     5.8
choose                          0.37   1.5%    173     7.7
release                         0.23   0.9%    188     4.3
ship                            0.19   0.7%     60    11.1
log                             0.10   0.4%     86     4.2
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

