# Where bevy_gooey_loop's time goes

Written by `tools/loop/steps.py publish` after every pass; the rows are in [`bevy_gooey_loop.tsv`](bevy_gooey_loop.tsv). Updated 2026-11-02 17:23Z.

## What the last passes shipped

From [`bevy_gooey_loop-audit.tsv`](bevy_gooey_loop-audit.tsv): commits on main, unpushed commits, package files changed, release tags, doc and demo files, and what was missing.

| pass | at | commits | unpushed | package_files | released | docs_files | demo_files | missing |
|---|---|---|---|---|---|---|---|---|
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
| 80 | 2026-11-02T17:23Z | 5 | 0 | 4 | v0.31.1 | 9 | 2 | - |

## How each configuration did

From [`bevy_gooey_loop-config.tsv`](bevy_gooey_loop-config.tsv): one row per pass with its host, wall time, model, thinking level and build settings.

```text
host       model                        think    jobs  threads  passes  mean min  median min
big        anthropic/claude-sonnet-5-5  high     8     8            11      41.7        36.4
big        anthropic/claude-sonnet-5-5  high     6     4            20      35.5        31.6
```

## The last 12 passes

```text
12 pass(es), 2821 steps, 5.3 h of pass time

category                       hours  share  steps  mean s
model                           2.62  49.8%   1304     7.2
wait: other                     0.69  13.1%     29    85.9
cargo test                      0.56  10.6%     94    21.3
cargo build                     0.36   6.9%     64    20.5
phase mark                      0.27   5.1%    124     7.8
shell                           0.25   4.7%    554     1.6
wait: gate                      0.19   3.6%      5   135.3
cargo clippy                    0.06   1.2%     18    12.7
read files                      0.05   0.9%    293     0.6
cargo publish                   0.05   0.9%     13    13.2
github                          0.05   0.9%     27     6.3
push gate                       0.03   0.7%     21     5.9
web                             0.02   0.4%     10     8.4
git                             0.02   0.3%     20     3.1
clip record/render              0.01   0.2%     25     1.7

phase                          hours  share  steps  mean s
build                           1.49  28.2%    876     6.1
gate                            1.22  23.2%    246    17.9
prove                           1.22  23.1%    797     5.5
research                        0.43   8.2%    329     4.7
choose                          0.36   6.8%    199     6.5
release                         0.26   4.9%    116     8.1
log                             0.11   2.1%     80     5.0
ship                            0.10   1.9%     74     4.9
orient                          0.05   1.0%     68     2.8
bugs                            0.03   0.6%     25     4.2

slowest single steps
  pass   74     308s  model model                
  pass   80     190s  tool  wait: other          {}
  pass   78     182s  tool  wait: other          {}
  pass   75     179s  tool  wait: other          {}
  pass   75     169s  tool  wait: other          {}
  pass   70     165s  tool  wait: gate           Waiting for gate
  pass   74     155s  tool  wait: other          {}
  pass   73     150s  tool  wait: gate           Waiting for the gate
  pass   76     148s  tool  wait: other          {}
  pass   80     146s  bg    shell                bg_2 completed: G=$CARGO_TARGET_DIR/release/examples/inventory; cp $G /tmp/g52/inv_after &
  pass   79     138s  tool  wait: other          {}
  pass   72     135s  tool  wait: gate           Waiting for gate 2
  pass   69     132s  tool  wait: gate           Waiting for gate
  pass   76     132s  tool  wait: other          {}
  pass   75     130s  tool  wait: other          {}
```

## Every pass recorded

```text
49 pass(es), 15199 steps, 42.7 h of pass time

category                       hours  share  steps  mean s
model                          17.40  40.7%   6867     9.1
wait: gate                      9.78  22.9%     71   496.0
wait: other                     2.99   7.0%    117    91.9
wait: tests                     2.83   6.6%     33   308.8
cargo test                      2.18   5.1%    463    17.0
shell                           1.83   4.3%   2475     2.7
cargo build                     1.42   3.3%    301    17.0
wait: build                     0.91   2.1%     19   172.5
wait: clip                      0.79   1.9%     30    94.9
phase mark                      0.77   1.8%    347     8.0
remote wait                     0.30   0.7%     36    29.6
read files                      0.25   0.6%   1956     0.5
github                          0.18   0.4%    155     4.2
eval                            0.18   0.4%     10    64.6
cargo clippy                    0.16   0.4%     69     8.4

phase                          hours  share  steps  mean s
orient                         18.96  44.4%   4422    15.4
prove                           7.10  16.6%   3606     7.1
gate                            6.06  14.2%   1025    21.3
build                           6.00  14.1%   3298     6.5
research                        1.32   3.1%    828     5.8
choose                          1.19   2.8%    659     6.5
release                         0.82   1.9%    513     5.8
ship                            0.76   1.8%    435     6.3
log                             0.36   0.8%    273     4.8
bugs                            0.12   0.3%     67     6.5

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

