# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-01 23:53Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 53 | 2026-11-01T15:21Z | 8 | 0 | 6 | v0.12.0 | 9 | 1 | - |
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

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4             4      42.0        44.8
```

## The last 12 passes

```text
12 pass(es), 4307 steps, 9.3 h of pass time

category                       hours  share  steps  mean s
model                           4.79  51.4%   1958     8.8
wait: gate                      1.56  16.8%     17   331.2
cargo test                      0.70   7.5%    160    15.7
wait: other                     0.59   6.4%     22    97.3
cargo build                     0.44   4.7%     99    15.9
shell                           0.35   3.8%    745     1.7
phase mark                      0.23   2.5%    105     8.0
wait: build                     0.16   1.7%      3   186.7
read files                      0.10   1.0%    519     0.7
remote wait                     0.06   0.7%      9    25.1
push gate                       0.06   0.6%     33     6.5
wait: tests                     0.06   0.6%      3    68.7
cargo clippy                    0.05   0.6%     28     7.0
github                          0.04   0.4%     53     2.8
cargo publish                   0.03   0.4%     10    12.3

phase                          hours  share  steps  mean s
prove                           2.73  29.3%   1474     6.7
gate                            2.50  26.9%    514    17.5
build                           2.40  25.8%   1302     6.6
research                        0.48   5.2%    252     6.9
ship                            0.36   3.9%    214     6.1
choose                          0.32   3.4%    180     6.4
release                         0.28   3.0%    173     5.8
log                             0.12   1.3%     93     4.6
orient                          0.11   1.2%     86     4.8
bugs                            0.01   0.1%      8     3.5

slowest single steps
  pass   57    1434s  tool  wait: gate           Waiting for the gate
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   59     302s  tool  wait: gate           Waiting for merged-tree gate
  pass   53     294s  tool  wait: other          Waiting for second perf run
  pass   62     282s  tool  wait: other          {}
  pass   57     253s  tool  wait: gate           Waiting for the gate
  pass   59     250s  tool  wait: gate           Waiting for the full gate
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   53     220s  tool  wait: other          Waiting for third perf run
  pass   56     203s  tool  wait: other          {}
  pass   59     201s  tool  wait: gate           Waiting for the full gate
  pass   59     185s  tool  wait: gate           Waiting for the gate
```

## Every pass recorded

```text
33 pass(es), 10934 steps, 33.6 h of pass time

category                       hours  share  steps  mean s
model                          12.93  38.5%   4864     9.6
wait: gate                      9.22  27.4%     62   535.6
wait: tests                     2.78   8.3%     30   334.0
wait: other                     1.78   5.3%     63   101.6
shell                           1.46   4.3%   1638     3.2
cargo test                      1.33   3.9%    312    15.3
cargo build                     0.87   2.6%    180    17.3
wait: clip                      0.78   2.3%     29    97.0
wait: build                     0.77   2.3%     15   185.6
phase mark                      0.38   1.1%    176     7.9
remote wait                     0.21   0.6%     22    33.6
read files                      0.19   0.6%   1555     0.4
eval                            0.17   0.5%      9    68.4
github                          0.11   0.3%    113     3.4
edit files                      0.11   0.3%   1113     0.3

phase                          hours  share  steps  mean s
orient                         18.84  56.1%   4297    15.8
prove                           4.48  13.3%   2295     7.0
gate                            3.99  11.9%    697    20.6
build                           3.48  10.3%   1956     6.4
research                        0.79   2.4%    453     6.3
choose                          0.69   2.1%    353     7.0
ship                            0.55   1.6%    274     7.2
release                         0.48   1.4%    347     5.0
log                             0.21   0.6%    167     4.6
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

