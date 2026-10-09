# The fixture jars of the libraries scenario

`lib/liba-1.0.jar`, `lib/liba-2.0.jar` and `lib/libb-1.0.jar` (with their `-sources.jar`) are
built from these sources with scala-cli 1.17.1 and Scala 3.8.4, `libb` against `liba` 1.0, so that
the scenario's documents hold a reference to another jar (`B extends A`), a renamed import, a named
argument, `this` and `super`, locals and parameters, a type lambda, a refinement, an inline method
and a context bound (`Cases.scala`), and a project reads `libb` over `liba` 2.0 (`liba2`, the same
classes one line lower with a member more). To rebuild them:

```sh
cd liba && scala-cli --power package --library --server=false -S 3.8.4 -o ../../lib/liba-1.0.jar .
cd ../libb && scala-cli --power package --library --server=false -S 3.8.4 --jar ../../lib/liba-1.0.jar -o ../../lib/libb-1.0.jar .
cd ../liba && jar cfM ../../lib/liba-1.0-sources.jar liba/A.scala
cd ../libb && jar cfM ../../lib/libb-1.0-sources.jar libb/B.scala libb/Cases.scala
cd ../liba2 && scala-cli --power package --library --server=false -S 3.8.4 -o ../../lib/liba-2.0.jar . && jar cfM ../../lib/liba-2.0-sources.jar liba/A.scala
```
