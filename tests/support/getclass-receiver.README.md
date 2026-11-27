# Primitive receivers of `getClass`

Run the table from the repository root with an empty output directory. The optional third argument adds a second teq binary and lists every base pass it loses on each leg.

```sh
bash tests/support/getclass-receiver.sh target/fast/teq out/gcr
bash tests/support/getclass-receiver.sh /path/to/base out/gcr-compare /path/to/head
```

`getclass_receiver.py` generates a package per cell and a manifest in `cells.tsv`. The place records where the input is supplied. Each primitive has a local cell for every applicable kind. Plain Int, Char, Double and Unit also exercise parameters, fields and default arguments. Int and Unit exercise each kind and place with all three call forms. Char, Boolean and Long also have literal receivers. Value, String, Array, Module and Case identify the fence subjects on the primitive axis.

`getclass-receiver.sh` compiles the scalac 3.8.4 batch with the sbt boot compiler, retrying the remaining sources after an attributed error. It runs each cell with JVM verification. It also builds and runs each teq JVM and JavaScript program and invokes the interpreter. A split JVM build publishes the declarations in `split/Lib.scala` with `--products` and compiles `split/Main.scala` against them. For the plain Int control the published inline method supplies the local val's initializer.

`runs/` keeps each build and run's stdout, stderr and exit status. `table.tsv` records both full outputs as JSON strings (including newlines), the comparison oracle and the first diagnostic or exception line. `summary.txt` counts verdicts per leg and axis. The separate JS representation column compares marked boxed receivers to the successful base JS output. The judge runs its planted-record self-test first and exits 1 for a base difference or a regression.

```sh
python3 -I tests/support/getclass_receiver.py gen out/gcr-sources
python3 -I tests/support/getclass_receiver.py selftest out/gcr-selftest
python3 -I tests/support/getclass_receiver.py judge out/gcr
```
