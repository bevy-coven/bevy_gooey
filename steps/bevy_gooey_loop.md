# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 04:59Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 60 | 2026-11-01T21:02Z | 10 | 0 | 7 | v0.19.0 | 8 | 1 | - |
| 61 | 2026-11-01T21:47Z | 10 | 0 | 8 | v0.20.0 | 9 | 1 | - |
| 62 | 2026-11-01T21:59Z | 4 | 0 | 1 | v0.20.1 | 1 | 0 | demo? |
| 63 | 2026-11-01T23:06Z | 7 | 0 | 7 | v0.21.0 | 10 | 1 | - |
| 64 | 2026-11-01T23:53Z | 7 | 0 | 7 | - | 8 | 1 | release |
| 65 | 2026-11-02T00:33Z | 6 | 0 | 7 | - | 12 | 1 | release |
| 66 | 2026-11-02T01:34Z | 6 | 0 | 10 | - | 11 | 1 | release |
| 67 | 2026-11-02T02:36Z | 14 | 0 | 10 | - | 13 | 3 | release |
| 68 | 2026-11-02T03:45Z | 9 | 0 | 11 | v0.23.0 | 17 | 3 | - |
| 69 | 2026-11-02T04:18Z | 11 | 0 | 6 | v0.24.0 | 12 | 1 | - |
| 70 | 2026-11-02T04:46Z | 8 | 0 | 4 | v0.24.1 | 5 | 0 | demo? |
| 71 | 2026-11-02T04:59Z | 4 | 0 | 2 | v0.24.2 | 5 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            11      42.7        42.4
```

## The last 12 passes

```text
12 pass(es), 3696 steps, 8.7 h of pass time

category                       hours  share  steps  mean s
model                           4.30  49.2%   1729     9.0
wait: gate                      1.19  13.6%     13   330.3
wait: other                     0.77   8.8%     35    79.4
cargo test                      0.70   8.0%    129    19.6
cargo build                     0.44   5.0%    111    14.2
shell                           0.33   3.7%    697     1.7
wait: build                     0.28   3.2%      6   168.9
phase mark                      0.28   3.2%    118     8.5
remote wait                     0.10   1.2%     15    24.4
read files                      0.07   0.7%    373     0.6
wait: tests                     0.06   0.6%      4    49.8
github                          0.05   0.5%     44     3.8
push gate                       0.04   0.5%     28     5.7
cargo clippy                    0.04   0.5%     22     7.0
cargo publish                   0.03   0.4%     12     9.9

phase                          hours  share  steps  mean s
prove                           2.67  30.5%   1072     9.0
build                           2.67  30.5%   1346     7.1
gate                            2.00  22.9%    362    19.9
choose                          0.39   4.5%    259     5.4
release                         0.30   3.5%    174     6.3
research                        0.26   2.9%    127     7.3
ship                            0.25   2.8%    161     5.5
orient                          0.10   1.2%    100     3.6
log                             0.07   0.8%     53     5.0
bugs                            0.03   0.4%     29     3.9

slowest single steps
  pass   63    1427s  tool  wait: gate           Waiting for the gate
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   66     766s  tool  wait: other          {}
  pass   60     419s  tool  wait: build          Waiting for the baseline and after gallery builds
  pass   67     409s  tool  wait: gate           Waiting for gate
  pass   61     359s  tool  wait: gate           Waiting for gate
  pass   60     348s  tool  wait: gate           Waiting for gate
  pass   62     282s  tool  wait: other          {}
  pass   66     240s  bg    shell                bg_3 completed: for i in $(seq 1 24); do grep -q DONE /tmp/g38/t2.log && break; sleep 10; 
  pass   63     240s  tool  wait: gate           Waiting for the gate
  pass   67     226s  tool  wait: build          Waiting for notifications example build
  pass   67     221s  tool  wait: build          Waiting for example build
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   65     160s  tool  wait: other          {}
  pass   66     155s  tool  wait: other          {}
```

## Every pass recorded

```text
40 pass(es), 13067 steps, 38.7 h of pass time

category                       hours  share  steps  mean s
model                          15.39  39.8%   5883     9.4
wait: gate                      9.68  25.0%     68   512.3
wait: tests                     2.83   7.3%     33   308.8
wait: other                     2.36   6.1%     91    93.5
cargo test                      1.77   4.6%    395    16.1
shell                           1.68   4.3%   2069     2.9
cargo build                     1.15   3.0%    259    16.0
wait: build                     0.91   2.4%     19   172.5
wait: clip                      0.79   2.0%     30    94.9
phase mark                      0.54   1.4%    252     7.7
remote wait                     0.29   0.8%     35    29.9
read files                      0.21   0.6%   1717     0.4
eval                            0.18   0.5%     10    64.6
github                          0.14   0.4%    138     3.7
push gate                       0.12   0.3%    101     4.3

phase                          hours  share  steps  mean s
orient                         18.92  48.9%   4370    15.6
prove                           6.09  15.7%   2871     7.6
build                           5.10  13.2%   2784     6.6
gate                            5.02  13.0%    834    21.7
choose                          0.92   2.4%    522     6.4
research                        0.92   2.4%    516     6.4
ship                            0.69   1.8%    405     6.2
release                         0.61   1.6%    429     5.1
log                             0.27   0.7%    209     4.7
bugs                            0.12   0.3%     63     6.6

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

