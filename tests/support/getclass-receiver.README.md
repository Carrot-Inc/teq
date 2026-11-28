# Primitive receivers of `getClass`

Run the table from the repository root with an empty output directory. The optional third argument adds a second teq binary and lists every base pass it loses on each leg.

```sh
bash tests/support/getclass-receiver.sh target/fast/teq out/gcr
bash tests/support/getclass-receiver.sh /path/to/base out/gcr-compare /path/to/head
```

`getclass_receiver.py` generates a package per cell and a manifest in `cells.tsv`. The place records where the input is supplied. Each primitive has a local cell for every applicable kind. Plain Int, Char, Double and Unit also exercise parameters, fields and default arguments. Int and Unit exercise each kind and place with all three call forms. Char, Boolean and Long also have literal receivers. Value, String, Array, Module and Case identify the fence subjects on the primitive axis.

`getclass-receiver.sh` compiles the scalac 3.8.4 batch with the sbt boot compiler, retrying the remaining sources after an attributed error. It runs each cell with JVM verification. It also builds and runs each teq JVM and JavaScript program and invokes the interpreter. A split JVM build publishes the declarations in `split/Lib.scala` with `--products` and compiles `split/Main.scala` against them. For the plain Int control the published inline method supplies the local val's initializer. For signature cells, teq builds `M` into a JVM jar or publishes its products. Scalac compiles the reflection caller against that output and Java runs it. These cells use the two JVM legs.

The route cells cover direct typing, substitution of an inline argument, copying a generic inline body, macro expansion, importing an inline body from products, a chain of 33 bounds and the array intersection's `ClassTag`. The resident cell edits the caller in a watch session and checks that only the caller was retyped. It compares the full JVM or JavaScript product with a fresh build of the edited source before running it against scalac's output. The JVM comparison uses the same entry points as watch (`--all-mains`).

`runs/` keeps each build and run's stdout, stderr and exit status. `table.tsv` records both full outputs as JSON strings (including newlines), the comparison oracle and the first diagnostic or exception line. `summary.txt` counts verdicts per leg and axis. The separate JS representation column compares marked boxed receivers to the successful base JS output. The `held` column in `cells.tsv` names the row for each applicable leg. In `table.tsv` it names the row for that record's leg. All places and call forms follow the same rules.

| Held row | Receivers and legs |
| --- | --- |
| `unit-alias-representation` | An alias of Unit or `Unit & Matchable`, on each applicable leg. Both resolve to plain Unit. `Unit & AnyVal` retains its intersection and uses scalac as its oracle. |
| `unit-intersection-result-descriptor` | The reflected method result of `Unit & AnyVal` or `Unit & Matchable` on both JVM legs. |
| `js-representation` | A Unit `Serializable` pattern binder or the `ClassTag` of `Array[Unit] & Array[Any]` on JavaScript. `Array[Int](1)` on JavaScript or the interpreter. |

When a marked cell's base run differs from scalac, either run's status and full output earns the verdict `held`. A cell already matching scalac on the base must keep that result. The summary lists each held cell and leg under its row. The judge runs its planted-record self-test first and exits 1 for invalid records, a fence loss, or a difference outside the accepted outputs on the head (on the base in a single-binary run).

```sh
python3 -I tests/support/getclass_receiver.py gen out/gcr-sources
python3 -I tests/support/getclass_receiver.py selftest out/gcr-selftest
python3 -I tests/support/getclass_receiver.py judge out/gcr
```
