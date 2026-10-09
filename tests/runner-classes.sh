#!/bin/bash
# Whether the test runner's committed class files (src/task/runner/dev/teq/runner/, embedded in
# teq by src/task/runner.rs) are those of src/task/runner/Runner.java: compiled again with the
# javac at hand (JAVAC, else JAVA_HOME's, else PATH's) under `--release 17` against
# test-interface 1.0, from the coursier cache or fetched from Maven Central and checked by its
# sha1. A difference fails under javac 17, which wrote the committed classes; another javac's
# bytes differ from 17's anyway, so there it is a note naming the command that writes them, as is
# a machine without a javac or without test-interface (not checked).
cd "$(dirname "$0")/.."
dir=src/task/runner
javac=${JAVAC:-${JAVA_HOME:+$JAVA_HOME/bin/}javac}
if ! version=$("$javac" -version 2>&1); then
  echo "runner-classes: note: no javac ($javac), not checked: set JAVAC or JAVA_HOME"
  exit 0
fi
major=$(printf '%s\n' "$version" | sed -nE 's/^javac ([0-9]+).*/\1/p')
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
sha1=0a3f14d010c4cb32071f863d97291df31603b521
path=org/scala-sbt/test-interface/1.0/test-interface-1.0.jar
case "$(uname -s)" in Darwin) coursier=$HOME/Library/Caches/Coursier/v1 ;; *) coursier=$HOME/.cache/coursier/v1 ;; esac
jar=${COURSIER_CACHE:-$coursier}/https/repo1.maven.org/maven2/$path
if [ ! -f "$jar" ]; then
  jar=$work/test-interface-1.0.jar
  timeout 60 curl -fsSL -o "$jar" "https://repo1.maven.org/maven2/$path" || { echo "runner-classes: note: test-interface 1.0 neither in the coursier cache nor fetched, not checked"; exit 0; }
fi
if [ "$(shasum "$jar" | cut -d' ' -f1)" != "$sha1" ]; then
  echo "FAIL runner-classes: $jar is not test-interface 1.0 (sha1)"
  exit 1
fi
timeout 120 "$javac" --release 17 -cp "$jar" -d "$work/classes" "$dir/Runner.java" || { echo "FAIL runner-classes: $version does not compile $dir/Runner.java"; exit 1; }
listed() { (cd "$1" && find . -name '*.class' | sort); }
if [ "$(listed "$work/classes")" = "$(listed "$dir")" ] && diff -rq "$work/classes/dev" "$dir/dev" > /dev/null; then
  echo "runner-classes: the committed classes are $version's output for $dir/Runner.java"
  exit 0
fi
write="javac --release 17 -cp test-interface-1.0.jar -d $dir $dir/Runner.java"
if [ "$major" = 17 ]; then
  echo "FAIL runner-classes: the committed classes are not $version's output for $dir/Runner.java: run $write with javac 17, and remove the classes it no longer writes"
  exit 1
fi
echo "runner-classes: note: $version's output differs from the committed classes, which javac 17 writes (not checked: run with JAVAC naming javac 17, or $write under 17)"
