# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 21:02Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 59 | 2026-11-01T20:09Z | 8 | 0 | 9 | v0.18.0 | 9 | 1 | - |
| 60 | 2026-11-01T21:02Z | 10 | 0 | 7 | v0.19.0 | 8 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
```

## The last 12 passes

```text
12 pass(es), 4309 steps, 9.5 h of pass time

category                       hours  share  steps  mean s
model                           4.50  47.2%   1945     8.3
wait: gate                      1.91  20.0%     18   381.1
wait: other                     0.83   8.7%     29   103.5
cargo test                      0.49   5.1%    142    12.4
shell                           0.44   4.6%    717     2.2
cargo build                     0.40   4.2%     86    16.9
phase mark                      0.19   2.0%    109     6.2
wait: tests                     0.18   1.9%      6   109.1
wait: build                     0.15   1.6%      5   106.5
read files                      0.07   0.7%    534     0.5
github                          0.07   0.7%     60     4.2
push gate                       0.05   0.6%     34     5.6
remote wait                     0.05   0.5%      8    20.7
cargo publish                   0.04   0.4%     12    12.2
cargo clippy                    0.04   0.4%     21     6.7

phase                          hours  share  steps  mean s
gate                            2.65  27.7%    355    26.8
prove                           2.55  26.7%   1424     6.4
build                           2.39  25.0%   1367     6.3
research                        0.60   6.3%    346     6.3
ship                            0.43   4.6%    230     6.8
choose                          0.39   4.1%    179     7.9
release                         0.24   2.5%    175     5.0
log                             0.13   1.4%    108     4.4
orient                          0.12   1.2%     97     4.3
bugs                            0.03   0.3%     12    10.0

slowest single steps
  pass   44    1800s  tool  wait: gate           Waiting for the gate
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   44    1154s  tool  wait: gate           Waiting for the gate to finish
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   44     279s  tool  wait: tests          Waiting for test binaries
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   44     252s  tool  wait: other          Waiting for benchmark
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   44     221s  tool  wait: other          Waiting for repo clones
  pass   53     220s  tool  wait: other          Waiting for third perf run
  pass   50     216s  tool  wait: other          Waiting for perf runs
  pass   56     203s  tool  wait: other          {}
```

## Every pass recorded

```text
29 pass(es), 9724 steps, 30.8 h of pass time

category                       hours  share  steps  mean s
model                          11.59  37.7%   4317     9.7
wait: gate                      8.58  27.9%     56   551.7
wait: tests                     2.78   9.0%     29   344.6
wait: other                     1.62   5.3%     57   102.1
shell                           1.39   4.5%   1444     3.5
cargo test                      1.08   3.5%    274    14.2
wait: clip                      0.78   2.5%     29    97.0
cargo build                     0.76   2.5%    156    17.5
wait: build                     0.74   2.4%     14   191.6
phase mark                      0.27   0.9%    142     6.9
remote wait                     0.19   0.6%     21    32.4
eval                            0.17   0.6%      9    68.4
read files                      0.15   0.5%   1381     0.4
github                          0.10   0.3%     99     3.7
edit files                      0.10   0.3%    986     0.4

phase                          hours  share  steps  mean s
orient                         18.82  61.1%   4274    15.9
prove                           3.77  12.3%   1940     7.0
gate                            3.13  10.2%    485    23.2
build                           2.72   8.8%   1578     6.2
research                        0.71   2.3%    414     6.2
choose                          0.53   1.7%    263     7.3
ship                            0.46   1.5%    252     6.6
release                         0.35   1.1%    275     4.6
log                             0.20   0.6%    156     4.6
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

