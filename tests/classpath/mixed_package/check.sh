#!/bin/bash
# PackageClassDenotation.packageObjs (SymDenotations.scala:2514-2528): a Scala 2
# package object is a declaration beside the Scala 3 file modules. Run from the repo root.
set -euo pipefail
. tests/support/jars.sh
teq=${TEQ:-./target/release/teq}
out=${TEQ_TEST_OUT:-out/classpath/mixed-package}
mkdir -p "$out"
scala2=$(jar_of scala2-lib)
scala=$(jar_of scala-library)
case_dir=tests/classpath/mixed_package
# Keep the upstream products on a separate path from the Scala 2 library too.
(ulimit -v 4000000; timeout -k 10 120 "$teq" compiler build --target jvm --products "$out/extra" --classpath "$scala" "$case_dir/a/Extra.scala")
(ulimit -v 4000000; timeout -k 10 120 "$teq" compiler build --target jvm -o "$out/main.jar" --classpath "$scala:$scala2:$out/extra" "$case_dir/b/Main.scala")
timeout -k 10 60 java -Xmx512m -cp "$out/main.jar:$out/extra:$scala2:$scala" run > "$out/actual"
diff -u "$case_dir/expected" "$out/actual"
