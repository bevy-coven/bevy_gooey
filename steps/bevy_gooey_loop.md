# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 21:47Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 50 | 2026-11-01T13:40Z | 9 | 0 | 3 | v0.9.0 | 7 | 1 | - |
| 51 | 2026-11-01T14:03Z | 7 | 0 | 4 | v0.10.0 | 10 | 2 | - |
| 52 | 2026-11-01T14:34Z | 8 | 0 | 5 | v0.11.0 | 7 | 1 | - |
| 53 | 2026-11-01T15:21Z | 8 | 0 | 6 | v0.12.0 | 9 | 1 | - |
| 54 | 2026-11-01T15:54Z | 9 | 0 | 8 | v0.13.0 | 10 | 1 | - |
| 55 | 2026-11-01T16:34Z | 7 | 0 | 8 | v0.14.0 | 10 | 1 | - |
| 56 | 2026-11-01T17:07Z | 7 | 0 | 6 | v0.15.0 | 10 | 1 | - |
| 57 | 2026-11-01T18:21Z | 7 | 0 | 7 | v0.16.0 | 9 | 1 | - |
| 58 | 2026-11-01T18:58Z | 7 | 0 | 8 | v0.17.0 | 9 | 1 | - |
| 59 | 2026-11-01T20:09Z | 8 | 0 | 9 | v0.18.0 | 9 | 1 | - |
| 60 | 2026-11-01T21:02Z | 10 | 0 | 7 | v0.19.0 | 8 | 1 | - |
| 61 | 2026-11-01T21:47Z | 10 | 0 | 8 | v0.20.0 | 9 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             1      42.4        42.4
```

## The last 12 passes

```text
12 pass(es), 4209 steps, 8.4 h of pass time

category                       hours  share  steps  mean s
model                           4.43  52.5%   1905     8.4
wait: gate                      1.14  13.5%     16   256.1
wait: other                     0.71   8.4%     26    97.9
cargo test                      0.52   6.2%    141    13.4
cargo build                     0.40   4.8%     89    16.3
shell                           0.38   4.5%    705     1.9
phase mark                      0.21   2.4%    108     6.9
wait: build                     0.17   2.1%      5   125.6
read files                      0.08   1.0%    516     0.6
push gate                       0.06   0.7%     34     6.2
wait: tests                     0.06   0.7%      3    68.7
github                          0.06   0.7%     63     3.2
cargo clippy                    0.05   0.6%     23     7.3
remote wait                     0.05   0.5%      8    20.7
cargo publish                   0.04   0.4%     10    13.5

phase                          hours  share  steps  mean s
prove                           2.75  32.6%   1603     6.2
build                           2.20  26.0%   1247     6.3
gate                            1.86  22.0%    339    19.7
research                        0.52   6.2%    285     6.6
ship                            0.36   4.3%    211     6.2
release                         0.29   3.5%    187     5.6
choose                          0.21   2.5%    124     6.2
log                             0.13   1.5%    105     4.4
orient                          0.11   1.3%     94     4.3

slowest single steps
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   53     220s  tool  wait: other          Waiting for third perf run
  pass   50     216s  tool  wait: other          Waiting for perf runs
  pass   56     203s  tool  wait: other          {}
  pass   59     201s  tool  wait: gate           Waiting for the full gate
  pass   59     185s  tool  wait: gate           Waiting for the gate
  pass   55     177s  model model                
  pass   52     174s  tool  wait: other          Waiting for null experiment
```

## Every pass recorded

```text
30 pass(es), 10083 steps, 31.5 h of pass time

category                       hours  share  steps  mean s
model                          11.99  38.1%   4473     9.6
wait: gate                      8.68  27.6%     57   548.3
wait: tests                     2.78   8.8%     30   334.0
wait: other                     1.65   5.2%     58   102.4
shell                           1.41   4.5%   1487     3.4
cargo test                      1.13   3.6%    282    14.4
wait: clip                      0.78   2.5%     29    97.0
cargo build                     0.77   2.5%    161    17.3
wait: build                     0.77   2.5%     15   185.6
phase mark                      0.31   1.0%    151     7.3
remote wait                     0.19   0.6%     21    32.4
eval                            0.17   0.5%      9    68.4
read files                      0.16   0.5%   1446     0.4
github                          0.10   0.3%    105     3.6
edit files                      0.10   0.3%   1035     0.4

phase                          hours  share  steps  mean s
orient                         18.82  59.8%   4281    15.8
prove                           4.07  12.9%   2123     6.9
gate                            3.24  10.3%    495    23.6
build                           2.88   9.2%   1669     6.2
research                        0.78   2.5%    445     6.3
choose                          0.53   1.7%    263     7.3
ship                            0.46   1.5%    254     6.5
release                         0.42   1.3%    303     4.9
log                             0.20   0.7%    163     4.5
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

