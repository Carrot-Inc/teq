# Scala 3's executable tests through teq

`run.py` runs `tests/run` of a scala/scala3 checkout (path in `$SCALA3`, default
`../teq-ref/scala3`; nothing is copied into teq) through teq and node:
every single-file test with its `.check` file, every directory test as one program (a directory
with Java sources is counted as left out). Each test is classified as pass, wrong-output,
run-error, run-timeout, compile-error, compile-crash or compile-timeout, and compile errors are
bucketed by their first error into what teq leaves out on purpose, what is planned and gaps.

```
tests/scala3/run.sh              # regression run: every test in passing.txt has to pass
tests/scala3/run.sh --update     # rewrite passing.txt from the tests that pass now
tests/scala3/run.sh --only enum  # a subset, by regex over test names
```

Results go to `results.json` and `summary.md` next to this file (both are written on every run,
so they show the state of the last run). `pending/` holds adapted tests that expose a difference
between teq and scalac (`scala3_<original>.scala`, the difference named in the second line,
the original `.check` next to it); every run reports the pending tests that pass, so that a
fixed one can move to `tests/cases/`.

`tests/cases/scala3_*.scala` are tests of the suite that failed only for a construct outside
the subset (`extends App`, Scala 2 control syntax, `xs map { .. }`), adapted mechanically, and
a few tests of `tests/run`, `tests/pos` and `tests/neg` adapted by hand (`tests/errors/scala3_*.scala`
for the rejected ones); the first line of each names what was replaced. `./tests/run.sh` and
`./tests/run_errors.sh` run them like every other case.

## NOTICE

Every file under `tests/` whose header comment names a test of the Scala 3 compiler's suite
(https://github.com/scala/scala3) is adapted, copied or taken from it, and so is the expected
output beside such a file (a pending test's `.check`), Copyright 2012-2026 EPFL and Copyright
2012-2026 Lightbend, Inc. dba Akka, licensed under the Apache License, Version 2.0
(http://www.apache.org/licenses/LICENSE-2.0); the root's `NOTICE` names them by that rule. Most
are named `scala3_*` (under `tests/cases`, `tests/errors`, `tests/parser/corpus` and
`tests/scala3/pending`). The adaptations replace constructs outside teq's subset (`extends App`,
`try`/`catch`, Java calls in printing code, ...) and keep the feature under test.
