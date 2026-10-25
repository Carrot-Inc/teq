#!/bin/bash
# tests/tasty/representation/closure/semanticdb.sh <scala3 checkout> <work dir>: dotty 3.8.4's compiler sources,
# `compiler/src` at the tag taken by `git archive` (the checkout is never moved off its own commit), compiled by scalac
# 3.8.4 itself with the compiler project's options (project/Build.scala 184-195 at the tag) and `-Xsemanticdb`, up to
# the phase that writes the SemanticDB documents (ExtractSemanticInfo, Compiler.scala 41): <work dir>/semanticdb, the
# occurrences and symbols closure.py reads (README.md); and, by the plugin phase `plugin/Constructions.scala` run
# after the typer in the same compile, <work dir>/constructions.txt, the constructor each construction selects, which
# SemanticDB does not name. Bounded: the compile is one JVM, at most 30 minutes.
set -e
checkout=$1 work=$2
if [ -z "$checkout" ] || [ -z "$work" ]; then echo "usage: semanticdb.sh <scala3 checkout> <work dir>" >&2; exit 2; fi
git -C "$checkout" rev-parse --verify -q 3.8.4^{commit} > /dev/null || { echo "semanticdb.sh: no tag 3.8.4 in $checkout" >&2; exit 1; }
rm -rf "$work/src" "$work/semanticdb" "$work/classes" "$work/cp" "$work/plugin" "$work/constructions.txt"
mkdir -p "$work/src" "$work/semanticdb" "$work/classes" "$work/cp" "$work/plugin/classes"
git -C "$checkout" archive 3.8.4 compiler/src | tar -x -C "$work/src"
# scalac 3.8.4 and the compiler's own dependencies (tasty-core, scala3-interfaces, scala-asm, compiler-interface and
# the library), resolved by scala-cli from the coursier cache.
echo '@main def cp(): Unit = ()' > "$work/cp/cp.scala"
cp=$(COURSIER_MODE=offline timeout 300 scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline -q \
  --workspace "$work/cp" --print-class-path "$work/cp/cp.scala" --dep org.scala-lang:scala3-compiler_3:3.8.4 | tail -1)
# The plugin, compiled against the same compiler, its descriptor beside its classes.
here=$(cd "$(dirname "$0")" && pwd)
COURSIER_MODE=offline timeout 300 scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline -q \
  --workspace "$work/plugin" --dep org.scala-lang:scala3-compiler_3:3.8.4 -d "$work/plugin/classes" \
  "$here/plugin/Constructions.scala" > "$work/plugin/compile.log" 2>&1 \
  || { echo "semanticdb.sh: the plugin does not build: $(grep -m3 -i error "$work/plugin/compile.log")" >&2; exit 1; }
echo "pluginClass=dotty.tools.dotc.semanticdb.Constructions" > "$work/plugin/classes/plugin.properties"
find "$work/src" \( -name '*.scala' -o -name '*.java' \) | sort > "$work/sources.txt"
timeout 1800 java -Xss16m -Xmx10g -cp "$cp" dotty.tools.dotc.Main -usejavacp -classpath "$cp" -color:never \
  -feature -deprecation -unchecked -encoding UTF8 -language:implicitConversions -Yexplicit-nulls \
  -Xsemanticdb -semanticdb-target "$work/semanticdb" -sourceroot "$work/src" \
  -Xplugin:"$work/plugin/classes" -P:constructions:"$work/constructions.txt" \
  -Ystop-after:extractSemanticDBExtractSemanticInfo -d "$work/classes" "@$work/sources.txt" > "$work/compile.log" 2>&1 \
  || { echo "semanticdb.sh: scalac failed: $(grep -m3 'Error' "$work/compile.log")" >&2; exit 1; }
echo "ok semanticdb: $(find "$work/semanticdb" -name '*.semanticdb' | wc -l) documents for $(wc -l < "$work/sources.txt") sources, $(wc -l < "$work/constructions.txt") constructions"
