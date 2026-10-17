# Spike 2 manifest: the stdlib suite over a shared case runner

| Row | Contents |
|---|---|
| Identity | base master `0e2ef0f12`; spike `227d531cd` on `spike/reduction-case-runner`; the binary under test master's release (`teq 0.1.8 0e2ef0f12`, sha256 `b9a4d9d3…44e6a3`), the same for the old runner and the new; the new runner interpreted by that binary (`TEQ=… ./teq interp`) |
| Net diff | `netdiff.txt`: Scala +120 code, +37 comments, +9 blank (`tools/script/Cases.scala` 73 lines, `tests/run_stdlib.scala` 92); shell −71 code, −21 comments (`tests/run_stdlib.sh` 87 lines); five call sites and two documents renamed; total +79 physical lines |
| Unit tests | none touched (no Rust) |
| Warnings | n/a (no Rust); `teq compiler check tools/script tests/run_stdlib.scala`: no diagnostics |
| Malformed input | n/a |
| Instructions | n/a: the compiler is unchanged; the suite's own cost is its wall time below |
| Identity | the verdicts, below |
| Suites | old (`tests/run_stdlib.sh` of master) and new (`tests/run_stdlib.scala`) on one tree, one binary: the clean subset, the planted subset, the missing jar, `--update`, cancellation, the whole corpus (below) |
| Build and size | n/a |
| Commands | `recipe/plant.scala` (the planted phases, replayable), `../../commands.md`; the outputs beside this file: `clean-*.out`, `planted-*.out`, `nojar-*.out`, `update-*`, `cancel-summary.txt`, `nonexistent*.out`, `wrapped.out`, `wrapper-calls.log`, `full-*.out`, `full-compare.txt`, `gate-coverage.log`, `gate-scripts.txt` |

## The planted runs

The proof is replayable: `recipe/plant.scala` (run from the repository's root, `TEQ=<binary> ./teq interp
bench/survey/evidence/spikes/case-runner/recipe/plant.scala -- --teq <binary> --work <new directory>`) takes the
spike's tree by `git archive`, master's runner beside it, the subset of `recipe/subset.txt`, the plants of
`recipe/planted/` with their expectations, and runs every phase below on both runners, comparing what they print and
write; `recipe-run/` holds its run on master's release binary (`teq 0.1.8 0e2ef0f12`, sha256 `b9a4d9d3…44e6a3`), the recipe as
committed: `summary.txt` its verdicts, `<phase>-<old|new>.out` each runner's output and status, the lists, the wrapper's
calls, the processes each cancelled session left (`cancel-<side>.left`):

| Phase | Old | New | Verdict |
|---|---|---|---|
| clean | 20 passed, 0 failed, exit 0 | the same lines, `passing.txt`, `summaries.txt` and outputs | same |
| planted | 19 passed, 3 failed, exit 1 | the same, `diff`'s hunk lines aside | same |
| no scala-library | the skip line, exit 0 | identical | same |
| `--update` | exit 1, 19 names | the same list | same |
| a missing binary | 0 passed, 22 failed, exit 1 | one line naming the binary, exit 1 | both fail |
| a logging wrapper | | 22 builds through it for 22 cases selected | apart from its binary |
| SIGTERM 8 s in | exit 143; `timeout 20 bash -c …` and `node …zz_planted_loop.js` left in its session | exit 143; nothing left | the claimed difference |

The first run, before the recipe, was the same plants by hand in a scratch worktree, recorded below; the second
review found its record (`../../commands.md`) without the two planted cases' `.expected` files, which the first run
had and without which neither runner selects a case. A scratch worktree of the spike (removed since) kept 46 of `tests/cases`' entries: listed passes, a
directory case, cases with `// teq:` flags, one with `// std: lean`, one with sourcecode, one with JavaScript
interop, two unlisted passes, three with summary lines; the list and the summaries cut to those names, and
master's `run_stdlib.sh` beside the new suite.

| Run | Old | New |
|---|---|---|
| clean subset | 20 passed, 0 failed, 4 unlisted passes named, exit 0, 4 s | the same lines, the same `passing.txt`, `summaries.txt` and the 20 `.actual` files byte for byte, exit 0, 1 s |
| planted: an expectation broken, a listed pass unlisted, a summary changed, a case looping forever, a case not compiling, `STDLIB_REASONS=1` | the three reasons, five unlisted passes, `REGRESSIONS: capture_release_ctor_param`, the summaries' difference, 19 passed 3 failed, exit 1, 25 s | the same lines in the same order; the summaries' difference without `diff`'s `1c1` and `---` lines; exit 1, 20 s |
| no scala-library in the cache (`COURSIER_CACHE=/nonexistent`) | the skip line, exit 0 | identical |
| `--update` | 19 names written, the list | the same list byte for byte, the same lines but `diff`'s hunk lines |
| SIGTERM 8 s in | the script dies (status 15 through `setsid -w` in a shell; 143 as the recipe's `Sh` reads it); `timeout 20 bash -c …` and `node …zz_planted_loop.js` left running in its session | exit 143, nothing of its session left 1 s later |

## The whole corpus

Master's binary, `tests/cases` whole (2,146 cases, 1,912 files and 234 directories, each with an expectation; 2,090 the suite builds):
the old runner from a pristine master worktree, the new from the spike's.

| Run | Wall time | Result |
|---|---:|---|
| old, `tests/run_stdlib.sh` (sequential) | 378 s | 1,913 passed, 177 failed, exit 0 |
| new, `JOBS=8` | 30 s | the same stdout byte for byte, exit 0 |
| new, `JOBS=1` | 228 s | the same stdout byte for byte, exit 0 |

`passing.txt` and `summaries.txt` identical in both new runs. Of the 2,090 `.actual` files, 2,049 identical; 36
or 37 differ by the checkout's absolute path alone (a failing case's stack trace names the `.js` under the
worktree it ran in); 3 by `--time`'s figures alone (cases whose flags ask for them); 1 or 2 (`import_value_type_member`,
`decimal_format_scientific`) where node's `RangeError` of an unending recursion names another frame, failing in
every run (`full-compare.txt`). Load average 39 to 57 during the runs (other sessions).

Skips, counted: the suite leaves out by rule the 56 cases with JavaScript interop, `// std: lean` or sourcecode;
both runners select the same 2,090 (the same names of `.actual` files); scala-library was in the cache.

## The harness apart from the binary

- Under master's binary as the interpreter (`TEQ`), `--teq` naming a wrapper that logs and runs master's binary:
  the 22 builds of the planted subset each went through the wrapper (`wrapper-calls.log`, 22 lines), the verdicts
  as the planted run's.
- `--teq /nonexistent/teq`: one line, `java.io.IOException: Cannot run program "/nonexistent/teq"`, exit 1 (the
  old runner with `TEQ=/nonexistent/teq`: 0 passed, 22 failed, every listed case a regression, exit 1;
  `nonexistent-*.out`). A difference: the new one stops at the first spawn.
- `tests/gate-coverage.sh` on the spike: 45 suites of tests/all.sh, 42 run by the workflow, 3 excluded; 43 lines of
  tests/gate.sh, 2 excluded; 0 failed. `tests/gate-scripts.sh` (`gate-scripts.txt`): 33 passed, 0 failed on the spike and on master, once five lila jars missing from the coursier cache were fetched (before: 26 passed, 7 failed on both, the sync step's "not in the coursier cache")
