# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 16:56Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
| 68 | 2026-11-02T03:45Z | 9 | 0 | 11 | v0.23.0 | 17 | 3 | - |
| 69 | 2026-11-02T04:18Z | 11 | 0 | 6 | v0.24.0 | 12 | 1 | - |
| 70 | 2026-11-02T04:46Z | 8 | 0 | 4 | v0.24.1 | 5 | 0 | demo? |
| 71 | 2026-11-02T04:59Z | 4 | 0 | 2 | v0.24.2 | 5 | 1 | - |
| 72 | 2026-11-02T05:30Z | 11 | 0 | 6 | v0.25.0,v0.25.1 | 9 | 1 | - |
| 73 | 2026-11-02T06:09Z | 11 | 0 | 8 | v0.26.0 | 10 | 1 | - |
| 74 | 2026-11-02T06:50Z | 7 | 0 | 8 | v0.27.0 | 8 | 1 | - |
| 75 | 2026-11-02T07:22Z | 8 | 0 | 5 | v0.27.1 | 7 | 1 | - |
| 76 | 2026-11-02T07:48Z | 8 | 0 | 7 | v0.28.0 | 9 | 1 | - |
| 77 | 2026-11-02T08:07Z | 6 | 0 | 6 | v0.29.0 | 6 | 1 | - |
| 78 | 2026-11-02T08:29Z | 5 | 0 | 5 | v0.30.0 | 7 | 1 | - |
| 79 | 2026-11-02T16:56Z | 5 | 0 | 5 | v0.31.0 | 7 | 1 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            19      36.1        32.8
```

## The last 12 passes

```text
12 pass(es), 2940 steps, 6.0 h of pass time

category                       hours  share  steps  mean s
model                           2.98  49.9%   1369     7.8
wait: other                     0.66  11.1%     27    88.1
cargo test                      0.61  10.3%    105    21.0
wait: gate                      0.45   7.5%      7   229.0
cargo build                     0.41   6.8%     81    18.1
phase mark                      0.29   4.9%    129     8.1
shell                           0.26   4.4%    587     1.6
github                          0.05   0.9%     30     6.4
cargo publish                   0.05   0.9%     16    11.9
read files                      0.05   0.9%    280     0.7
cargo clippy                    0.04   0.6%     14     9.7
push gate                       0.03   0.5%     22     5.2
git                             0.02   0.3%     19     3.3
web                             0.02   0.3%      7     8.3
clip record/render              0.01   0.2%     27     1.4

phase                          hours  share  steps  mean s
build                           1.81  30.3%    987     6.6
gate                            1.46  24.5%    256    20.6
prove                           1.29  21.7%    771     6.0
research                        0.48   8.1%    352     4.9
choose                          0.33   5.5%    167     7.0
release                         0.29   4.8%    134     7.8
ship                            0.12   2.0%     90     4.7
log                             0.11   1.8%     78     5.0
orient                          0.06   1.0%     71     2.9
bugs                            0.03   0.4%     23     4.1

slowest single steps
  pass   68     920s  tool  wait: gate           Waiting for gate rerun
  pass   74     308s  model model                
  pass   78     182s  tool  wait: other          {}
  pass   75     179s  tool  wait: other          {}
  pass   75     169s  tool  wait: other          {}
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   74     155s  tool  wait: other          {}
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   76     148s  tool  wait: other          {}
  pass   79     138s  tool  wait: other          {}
  pass   72     135s  tool  wait: gate           Waiting for gate 2
  pass   69     132s  tool  wait: gate           Waiting for gate
  pass   76     132s  tool  wait: other          {}
  pass   75     130s  tool  wait: other          {}
  pass   77     129s  tool  wait: other          {}
```

## Every pass recorded

```text
48 pass(es), 14978 steps, 42.3 h of pass time

category                       hours  share  steps  mean s
model                          17.23  40.7%   6764     9.2
wait: gate                      9.78  23.1%     71   496.0
wait: other                     2.92   6.9%    114    92.2
wait: tests                     2.83   6.7%     33   308.8
cargo test                      2.12   5.0%    455    16.7
shell                           1.81   4.3%   2435     2.7
cargo build                     1.39   3.3%    299    16.7
wait: build                     0.91   2.2%     19   172.5
wait: clip                      0.79   1.9%     30    94.9
phase mark                      0.76   1.8%    338     8.1
remote wait                     0.30   0.7%     36    29.6
read files                      0.25   0.6%   1926     0.5
github                          0.18   0.4%    154     4.2
eval                            0.18   0.4%     10    64.6
push gate                       0.14   0.3%    112     4.5

phase                          hours  share  steps  mean s
orient                         18.95  44.8%   4417    15.4
prove                           6.90  16.3%   3484     7.1
gate                            6.00  14.2%   1017    21.2
build                           5.92  14.0%   3264     6.5
research                        1.32   3.1%    826     5.8
choose                          1.16   2.7%    622     6.7
release                         0.82   1.9%    513     5.8
ship                            0.74   1.7%    427     6.2
log                             0.36   0.8%    271     4.8
bugs                            0.12   0.3%     65     6.5

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

