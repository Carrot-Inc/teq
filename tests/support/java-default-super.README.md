# Super calls to Java interface defaults

`java_default_super.py` generates 432 independent Scala programs covering 24 defaults. The `call` axis pairs each `super.m(...)` with an ordinary selection `(this: InterfaceType).m(...)`. Each method has a method-body, constructor-statement and field-initialiser case, both with and without a Scala class between the caller and its Java parent. Each functional argument has lambda and anonymous-class forms. The larger interfaces use AbstractCollection, AbstractList and AbstractMap. Each program prints one observation. The Iterator.remove cases catch the default implementation's UnsupportedOperationException and print `unsupported`.

`java-default-super.sh` runs the table. From the repository root, give it a teq binary and a fresh output directory. An optional second binary adds a per-cell, per-target regression list.

```sh
bash tests/support/java-default-super.sh target/fast/teq out/jds
bash tests/support/java-default-super.sh /path/to/base out/jds-compare /path/to/head
```

The driver probes the default signatures with `javap -public -s`. It compiles the sources together with scalac 3.8.4 from the sbt boot, falling back to `scala-cli compile -S 3.8.4 --server=false`. On a source error it records the named cells and retries the others. Each teq binary builds each program for the JVM against scala-library 3.8.4 and for JavaScript against teq's std. The JVM runs use `java -Xverify:all`, JavaScript uses Node, and `teq interp` runs the source with teq's std.

On the JVM, `jvm-split` builds each intervening `Between` class with teq's `--products`, then builds and runs the caller against those products. For `jvm-jar`, scalac compiles the 24 intervening classes for method-body super calls (lambda where applicable) into one jar. Each caller builds and runs against that jar. Both legs use the cell's scalac output and have their own base-to-head fence.

- `cells.tsv` names each package and its axes. Its source is in `pNNNN/Main.scala`, with separate upstream and downstream sources in `pNNNN/split/` for the cells with an intervening class.
- `batches/` holds scalac's input lists, classes and build records. `javap/` holds the JDK declarations. `jar-cells` lists the cells compiled against `jar/between.jar`. `runs/scalac/<cell>/` and `runs/{base,head}/{jvm,js,interp,jvm-split,jvm-jar}/<cell>/` hold each step's stdout, stderr and exit status (`.out`, `.err`, `.exit`).
- `table.tsv` gives each cell's signature availability, verdict and first diagnostic, exception or output difference. `summary.txt` gives counts by target for each axis and the optional head fence. Exit 0 means all valid runs match scalac. Exit 1 reports a difference. Missing records and unclassified failures stop the judge with exit 2.

The Python commands also work independently.

```sh
python3 -I tests/support/java_default_super.py gen out/jds-sources
python3 -I tests/support/java_default_super.py selftest out/jds-selftest
python3 -I tests/support/java_default_super.py judge out/jds
```
