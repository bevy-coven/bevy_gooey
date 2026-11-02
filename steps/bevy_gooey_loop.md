# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 00:33Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 54 | 2026-11-01T15:54Z | 9 | 0 | 8 | v0.13.0 | 10 | 1 | - |
| 55 | 2026-11-01T16:34Z | 7 | 0 | 8 | v0.14.0 | 10 | 1 | - |
| 56 | 2026-11-01T17:07Z | 7 | 0 | 6 | v0.15.0 | 10 | 1 | - |
| 57 | 2026-11-01T18:21Z | 7 | 0 | 7 | v0.16.0 | 9 | 1 | - |
| 58 | 2026-11-01T18:58Z | 7 | 0 | 8 | v0.17.0 | 9 | 1 | - |
| 59 | 2026-11-01T20:09Z | 8 | 0 | 9 | v0.18.0 | 9 | 1 | - |
| 60 | 2026-11-01T21:02Z | 10 | 0 | 7 | v0.19.0 | 8 | 1 | - |
| 61 | 2026-11-01T21:47Z | 10 | 0 | 8 | v0.20.0 | 9 | 1 | - |
| 62 | 2026-11-01T21:59Z | 4 | 0 | 1 | v0.20.1 | 1 | 0 | demo? |
| 63 | 2026-11-01T23:06Z | 7 | 0 | 7 | v0.21.0 | 10 | 1 | - |
| 64 | 2026-11-01T23:53Z | 7 | 0 | 7 | - | 8 | 1 | release |
| 65 | 2026-11-02T00:33Z | 6 | 0 | 7 | - | 12 | 1 | release |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             5      41.5        42.4
```

## The last 12 passes

```text
12 pass(es), 4289 steps, 9.2 h of pass time

category                       hours  share  steps  mean s
model                           4.83  52.5%   1956     8.9
wait: gate                      1.52  16.5%     16   341.1
cargo test                      0.67   7.3%    154    15.6
wait: other                     0.50   5.5%     24    75.5
cargo build                     0.44   4.8%    101    15.6
shell                           0.36   3.9%    755     1.7
phase mark                      0.25   2.8%    112     8.1
wait: build                     0.16   1.7%      3   186.7
read files                      0.10   1.0%    494     0.7
remote wait                     0.09   1.0%     11    30.9
push gate                       0.06   0.7%     34     6.5
cargo clippy                    0.06   0.6%     30     7.1
github                          0.05   0.5%     52     3.2
wait: tests                     0.03   0.3%      2    57.8
cargo publish                   0.03   0.3%     10    10.9

phase                          hours  share  steps  mean s
prove                           2.52  27.3%   1377     6.6
gate                            2.49  27.1%    524    17.1
build                           2.44  26.5%   1304     6.7
research                        0.43   4.7%    224     7.0
ship                            0.41   4.4%    263     5.6
choose                          0.33   3.6%    184     6.4
release                         0.28   3.0%    179     5.6
orient                          0.16   1.7%    116     5.0
log                             0.14   1.5%     99     5.2
bugs                            0.01   0.1%      8     3.5

slowest single steps
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   62     282s  tool  wait: other          {}
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   56     203s  tool  wait: other          {}
  pass   59     201s  tool  wait: gate           Waiting for the full gate
  pass   59     185s  tool  wait: gate           Waiting for the gate
  pass   55     177s  model model                
  pass   59     168s  model model                
```

## Every pass recorded

```text
34 pass(es), 11301 steps, 34.3 h of pass time

category                       hours  share  steps  mean s
model                          13.33  38.9%   5044     9.5
wait: gate                      9.22  26.9%     62   535.6
wait: tests                     2.78   8.1%     30   334.0
wait: other                     1.86   5.4%     68    98.5
shell                           1.47   4.3%   1712     3.1
cargo test                      1.35   4.0%    320    15.2
cargo build                     0.90   2.6%    186    17.4
wait: clip                      0.78   2.3%     29    97.0
wait: build                     0.77   2.3%     15   185.6
phase mark                      0.43   1.2%    190     8.1
remote wait                     0.24   0.7%     25    34.5
read files                      0.19   0.6%   1580     0.4
eval                            0.17   0.5%      9    68.4
github                          0.12   0.3%    116     3.6
push gate                       0.11   0.3%     88     4.4

phase                          hours  share  steps  mean s
orient                         18.89  55.1%   4334    15.7
prove                           4.64  13.5%   2359     7.1
gate                            4.08  11.9%    723    20.3
build                           3.70  10.8%   2089     6.4
research                        0.83   2.4%    468     6.4
choose                          0.70   2.0%    357     7.0
ship                            0.59   1.7%    323     6.6
release                         0.51   1.5%    367     5.0
log                             0.24   0.7%    185     4.7
bugs                            0.09   0.3%     42     7.8

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

