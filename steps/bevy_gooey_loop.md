# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 18:21Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 56 | 2026-11-01T17:07Z | 7 | 0 | 6 | v0.15.0 | 10 | 1 | - |
| 57 | 2026-11-01T18:21Z | 7 | 0 | 7 | v0.16.0 | 9 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8             8      37.4        33.0
```

## The last 12 passes

```text
12 pass(es), 4386 steps, 9.5 h of pass time

category                       hours  share  steps  mean s
model                           4.37  46.1%   1954     8.1
wait: gate                      1.99  21.0%     16   448.7
wait: other                     0.96  10.1%     30   115.1
cargo test                      0.45   4.8%    111    14.6
shell                           0.44   4.6%    677     2.3
cargo build                     0.30   3.2%     61    17.7
phase mark                      0.23   2.4%    116     7.0
wait: tests                     0.16   1.7%      6    96.1
wait: clip                      0.11   1.2%      4   100.7
read files                      0.08   0.8%    617     0.5
clip record/render              0.06   0.7%     37     6.1
github                          0.06   0.6%     51     4.3
remote wait                     0.05   0.5%      9    19.3
wait: build                     0.05   0.5%      4    42.4
cargo publish                   0.04   0.4%     14    10.3

phase                          hours  share  steps  mean s
prove                           3.02  31.8%   1561     7.0
gate                            2.39  25.2%    329    26.2
build                           2.06  21.7%   1261     5.9
research                        0.60   6.4%    350     6.2
choose                          0.48   5.0%    222     7.8
release                         0.27   2.8%    216     4.4
ship                            0.25   2.7%    103     8.9
orient                          0.20   2.1%    183     3.9
log                             0.13   1.4%    107     4.5
bugs                            0.08   0.9%     34     8.9

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   43     792s  tool  wait: gate           Waiting for the gate to finish
  pass   40     653s  tool  wait: gate           Waiting for the gate to finish
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
```

## Every pass recorded

```text
26 pass(es), 8526 steps, 28.1 h of pass time

category                       hours  share  steps  mean s
model                          10.14  36.1%   3766     9.7
wait: gate                      8.18  29.1%     50   588.8
wait: tests                     2.75   9.8%     28   353.8
wait: other                     1.56   5.5%     52   107.8
shell                           1.26   4.5%   1220     3.7
cargo test                      0.88   3.1%    222    14.3
wait: clip                      0.78   2.8%     29    97.0
wait: build                     0.62   2.2%     12   185.3
cargo build                     0.61   2.2%    122    17.9
phase mark                      0.23   0.8%    118     6.9
remote wait                     0.19   0.7%     20    34.0
eval                            0.17   0.6%      9    68.4
read files                      0.15   0.5%   1246     0.4
edit files                      0.10   0.3%    885     0.4
github                          0.09   0.3%     86     3.8

phase                          hours  share  steps  mean s
orient                         18.76  66.8%   4251    15.9
prove                           3.02  10.8%   1561     7.0
gate                            2.39   8.5%    329    26.2
build                           2.06   7.3%   1261     5.9
research                        0.60   2.1%    350     6.2
choose                          0.48   1.7%    222     7.8
release                         0.29   1.0%    242     4.4
ship                            0.25   0.9%    103     8.9
log                             0.15   0.5%    123     4.5
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

