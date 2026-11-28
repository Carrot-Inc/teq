# Primitive boxes and JDK type patterns

Run from the repository root with a JDK, Scala CLI, the Scala 3.8.4 library jar in the coursier cache, Node and Python 3.9 or later.

```sh
python3 -I tests/support/box_ancestry.py selftest out/box-selftest
bash tests/support/box-ancestry.sh /path/to/base/teq out/box
bash tests/support/box-ancestry.sh /path/to/base/teq out/box2 /path/to/head/teq
```

Use a fresh directory for each matrix. The runner builds cells independently, with four cells in parallel by default. Set `BOX_JOBS` to change that count.

- `box_ancestry.py gen <dir>` walks the superclass and interfaces of each of the eight primitive boxes using `javap -public`. It adds Object, Runnable and Thread. Each target occurs alone, after a Number test, and beside a conformance in a second file.
- `cells.tsv` records the cell name, box, target, condition, second file’s conformance type, and JDK version, in that order. `cells/<cell>/A.scala` contains the match. In the inline conditions, `Lib.scala` defines a type-parameterized inline method and `A.scala` calls it with the primitive, alone or after a Number test. Targets are Serializable, Runnable and Constable where the JDK names it. In the two-file conformance condition, `B.scala` assigns the boxed value to Number, or to Comparable of the box for Character and Boolean.
- `box-ancestry.sh` records the binaries under comparison in `binaries`. For each cell it runs `scala-cli compile -S 3.8.4 --server=false --jvm system` and executes the generated main class with `java -Xverify:all`. It then runs each teq binary on the JVM (`compiler build --target jvm --std=scala-library --classpath <scala-library jar>`, then `java -Xverify:all`), on JavaScript (`compiler build`, then `node`), and in the interpreter (`interp`).
- Inline JVM cells compile both files together and compile the caller against a separately built library. The split scalac oracle uses a classes directory, and teq publishes the library with `--products`. The two oracle runs are shared across the scheduling settings for that program. JavaScript and the interpreter run each whole program once.
- `cells/<cell>/jvm.tsv` lists the JVM run name, build mode, worker count and file-order seed. For Integer and Character the inline rows cross `TEQ_THREADS=1,8` with `TEQ_FILE_ORDER=seed:1,2,3`. The other inline rows use one worker and the first seed.
- The Cloneable pattern cells call an inline method with `T = java.lang.Cloneable`. They test a value class over Int, with and without Cloneable, both directly in an alternative and as a case-class field. Whole and split builds each pair an unchanged caller with a second file that either leaves the value alone or tests it against Cloneable. The judge compares the caller’s class bytes across each pair.
- Every head JVM build runs with `TEQ_BOX_TESTS=1`. The judge reads the line `test sites the writer asked with no record: N` from build stderr, including both stages of a split build. The `missing` column holds their sum. A nonzero count fails the comparison and a missing or duplicated line fails the inventory.
- `runs/scalac/<cell>/` and `runs/<base-or-head>/<jvm-js-or-interp>/<cell>/` hold each executed step’s `build.{out,err,exit}` or `run.{out,err,exit}`. Split builds also record `upstream.{out,err,exit}` and use `runs/scalac-split/<cell>/` for the oracle. Each build and execution has a 120-second timeout. A failed build ends that leg for the cell. The interpreter uses one command for typing and execution.
- `box_ancestry.py judge <dir>` runs the planted-record self-test, checks the manifest against the local JDK and checks the run inventory before comparing statuses and stdout. `table.tsv` records the oracle and observed values as JSON strings, including the exit status and diagnostics for a failed command. `summary.txt` counts verdicts by leg, box, target and condition. With two binaries it also lists each cell that regressed on each leg.

The judge exits 0 when every comparison agrees, 1 for a complete table with differences, and 2 for an incomplete or inconsistent record. To regenerate just the sources or rejudge an existing run on the same JDK, use the corresponding command below.

```sh
python3 -I tests/support/box_ancestry.py gen out/box-sources
python3 -I tests/support/box_ancestry.py judge out/box
```
