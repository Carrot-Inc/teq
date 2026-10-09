#!/bin/bash
# Every tests/classpath/*.scala is checked with `--std=scala-library` against the jars its `// jars:` line
# names (tests/support/jars.sh). A program without
# `// expect:` lines has to check without an error; one with them has to be rejected with all
# of them. The suite skips, and counts as passed, when a jar is not in the cache.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
# A JVM run's class path: the jars, with scala-library 3.8.4 first where they name none.
SL=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
with_scala_library() {
  case ":$1" in *"/scala-library-"*) echo "$1" ;; *) echo "$SL${1:+:$1}" ;; esac
}
# And the fixtures' class files after the TASTy-only fixtures jar, which the JVM runs.
with_fixture_classes() {
  case ":$1:" in
    *-fixtures.jar:*) echo "$1" | sed "s|\([^:]*-fixtures\.jar\)|\1:$(jar_of fixtures-jvm)|" ;;
    *) echo "$1" ;;
  esac
}
fail=0
passed=0
skipped=0
for src in tests/classpath/*.scala; do
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $src: not in the coursier cache:$JARS_MISSING"
    skipped=$((skipped + 1))
    continue
  fi
  out=$(timeout 60 "$TEQ" compiler check "$src" --classpath "$JARS_CP" --std=scala-library 2>&1)
  code=$?
  if grep -q '^// expect: ' "$src"; then
    if [ $code -ne 1 ]; then
      echo "FAIL $src: exit code $code"
      echo "$out"
      fail=1
      continue
    fi
    ok=1
    while IFS= read -r line; do
      expected=${line#// expect: }
      if ! grep -qF -- "$expected" <<< "$out"; then
        echo "FAIL $src: missing '$expected'"
        echo "$out"
        ok=0
      fi
    done < <(grep '^// expect: ' "$src")
    [ $ok = 1 ] && passed=$((passed + 1)) || fail=1
  else
    if [ $code -ne 0 ]; then
      echo "FAIL $src: exit code $code"
      echo "$out"
      fail=1
      continue
    fi
    passed=$((passed + 1))
  fi
done
# tests/classpath/jvm/*.scala run on the JVM with the jars of their `// jars:` line on the class
# path, linked against scala-library's jar (the JVM's one mode; the jar pinned where the line
# names none): a `// std:` line names JavaScript's modes and is not read here. A missing
# .expected is produced with real Scala (scala-cli). A directory is one program of several files
# (a macro and its use).
scala_version=${SCALA_VERSION:-3.8.4}
mkdir -p out/classpath
for src in tests/classpath/jvm/*.scala tests/classpath/jvm/*/; do
  [ -e "$src" ] || continue
  src=${src%/}
  name=$(basename "$src" .scala)
  expected="tests/classpath/jvm/$name.expected"
  if [ ! -f "$expected" ]; then
    if ! timeout 120 scala-cli run -S "$scala_version" --jvm system "$src" --server=false -q > "$expected" 2> "out/classpath/$name.ref.err"; then
      echo "REF FAIL $name (see out/classpath/$name.ref.err)"
      rm -f "$expected"
      fail=1
      continue
    fi
  fi
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $src: not in the coursier cache:$JARS_MISSING"
    skipped=$((skipped + 1))
    continue
  fi
  # The JDK's own warnings on stderr (`sun.misc.Unsafe` under scala-library's lazy vals) are
  # not the program's output.
  jvm_cp=$(with_scala_library "$JARS_CP")
  run_jvm 120 "out/classpath/$name.jar" "$jvm_cp" --build "$src" --target jvm --classpath "$jvm_cp" 2>&1 \
    | { grep -v '^WARNING: ' > "out/classpath/$name.actual" || true; }
  if [ "${PIPESTATUS[0]}" = 0 ] && diff -q "$expected" "out/classpath/$name.actual" > /dev/null; then
    passed=$((passed + 1))
  else
    echo "FAIL $src (jvm run)"
    diff "$expected" "out/classpath/$name.actual" | head -10
    fail=1
  fi
done
# The entry points of a program as scalac's discovery finds them and `java` launches them by name: built as
# products (every entry point gets scalac's class), the classes the analysis marks `main` are the program's
# `mains` (scala-cli's `--list-main-classes`), and `java` runs each by its name with arguments as it runs
# scalac's (launched.expected); then `--main` with each name, and with its last segment, chooses it on each target
# named, the arguments passed on. tests/classpath/entry-app's `App` runs on the JVM alone: the lean std's `App` has
# no `args`; tests/classpath/entry-root's encoded `@main` has no package.
entry_points() {
  local entry=$1 targets=$2 name m expected actual target
  name=$(basename "$entry")
  rm -rf "out/classpath/$name"
  if timeout 60 "$TEQ" compiler build "$entry"/*.scala --target jvm --classpath "$SL" --products "out/classpath/$name" --analysis-version 3 > "out/classpath/$name.json" 2> "out/classpath/$name.err"; then
    python3 -c 'import json, sys; print("\n".join(sorted((c["name"] for f in json.load(open(sys.argv[1]))["analysis"] for c in f["classes"] if c["main"]), key=lambda n: n.encode())))' "out/classpath/$name.json" > "out/classpath/$name.mains"
    if diff -q "$entry/mains" "out/classpath/$name.mains" > /dev/null; then passed=$((passed + 1)); else
      echo "FAIL $entry: the analysis's main classes"
      diff "$entry/mains" "out/classpath/$name.mains" | head -10
      fail=1
    fi
    while read -r m; do
      printf '%s: %s\n' "$m" "$(timeout 30 java -cp "out/classpath/$name:$SL" "$m" a b 2>&1)"
    done < "$entry/mains" > "out/classpath/$name.launched"
    if diff -q "$entry/launched.expected" "out/classpath/$name.launched" > /dev/null; then passed=$((passed + 1)); else
      echo "FAIL $entry: launched by name"
      diff "$entry/launched.expected" "out/classpath/$name.launched" | head -10
      fail=1
    fi
  else
    echo "FAIL $entry: the products do not build (see out/classpath/$name.err)"
    fail=1
  fi
  while read -r m; do
    expected=$(grep -F "$m: " "$entry/launched.expected" | sed "s|^$m: ||")
    for chosen in $(printf '%s\n' "$m" "${m##*.}" | sort -u); do
      for target in $targets; do
        case $target in
          js) actual=$(run_js 60 "out/classpath/$name-main.js" "$entry"/*.scala --main "$chosen" -- a b 2>&1) ;;
          jvm) actual=$(run_jvm 60 "out/classpath/$name-main.jar" "$SL" --build "$entry"/*.scala --target jvm --classpath "$SL" --main "$chosen" -- a b 2>&1) ;;
          interp) actual=$(timeout 60 "$TEQ" interp "$entry"/*.scala --main "$chosen" -- a b 2>&1) ;;
        esac
        if [ "$actual" = "$expected" ]; then passed=$((passed + 1)); else
          echo "FAIL $entry: --main $chosen on $target: $actual"
          fail=1
        fi
      done
    done
  done < "$entry/mains"
}
entry_points tests/classpath/entry "js jvm interp"
entry_points tests/classpath/entry-app jvm
entry_points tests/classpath/entry-root "js jvm interp"
# tests/classpath/js/*.scala run under node with teq's std and the jars of their `// jars:` line
# on the class path: what the std lacks is compiled from the jars' TASTy bodies. A missing
# .expected is produced with real Scala (scala-cli). A `// std: lean scala-library` line runs the
# program under each std mode it names on JavaScript and in the interpreter; a `// teq: <flags>`
# line passes those flags to teq, and a `// targets: js interp` line runs it on each target it
# names (`jvm` under java, its stderr apart, once, linked against scala-library).
for src in tests/classpath/js/*.scala; do
  [ -e "$src" ] || continue
  name=$(basename "$src" .scala)
  expected="tests/classpath/js/$name.expected"
  if [ ! -f "$expected" ]; then
    if ! timeout 120 scala-cli run -S "$scala_version" --jvm system "$src" --server=false -q > "$expected" 2> "out/classpath/$name.ref.err"; then
      echo "REF FAIL $name (see out/classpath/$name.ref.err)"
      rm -f "$expected"
      fail=1
      continue
    fi
  fi
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $src: not in the coursier cache:$JARS_MISSING"
    skipped=$((skipped + 1))
    continue
  fi
  modes=$(grep -h -o '^// std: .*' "$src" | head -1 | sed 's|^// std: ||')
  flags=$(grep -h -o '^// teq: .*' "$src" | head -1 | sed 's|^// teq: ||')
  targets=$(grep -h -o '^// targets: .*' "$src" | head -1 | sed 's|^// targets: ||')
  for target in ${targets:-js}; do
  target_modes=${modes:-lean}
  [ "$target" = jvm ] && target_modes=link
  for mode in $target_modes; do
    std_flag=--std=$mode
    cp=$JARS_CP
    if [ "$target" = jvm ]; then
      std_flag=
      cp=$(with_scala_library "$(with_fixture_classes "$JARS_CP")")
    fi
    # The interpreter runs on the lean std and takes no --std (`teq interp` refuses it).
    [ "$target" = interp ] && std_flag=
    output="out/classpath/$name.js"
    [ "$target" = jvm ] && output="out/classpath/$name.jar"
    rm -f "out/classpath/$name.err"
    if [ "$target" = jvm ]; then
      # java's own messages (JDK 24 warns of scala-library's `LazyVals` reaching `sun.misc.Unsafe`)
      # are kept apart from the program's output, as tests/run_jvm.sh keeps them.
      run_jvm 120 "$output" "$cp" --build "$src" --target "$target" $std_flag --classpath "$cp" $flags > "out/classpath/$name.actual" 2> "out/classpath/$name.err"
    elif [ "$target" = interp ]; then
      timeout 120 "$TEQ" interp "$src" --classpath "$cp" $flags > "out/classpath/$name.actual" 2>&1
    else
      run_js 120 "$output" "$src" --target "$target" $std_flag --classpath "$cp" $flags > "out/classpath/$name.actual" 2>&1
    fi
    if [ $? -eq 0 ] && diff -q "$expected" "out/classpath/$name.actual" > /dev/null; then
      passed=$((passed + 1))
    else
      echo "FAIL $src ($target run${std_flag:+, $std_flag})"
      diff "$expected" "out/classpath/$name.actual" | head -10
      [ -s "out/classpath/$name.err" ] && grep -v "^WARNING" "out/classpath/$name.err" | head -10
      fail=1
    fi
  done
  done
done
# A TASTy entry the search for reflectively instantiatable classes cannot read is an error, in
# every build: its contents are not made up and kept in the jar cache.
lib=$(jar_of reflect-lib)
if [ -f "$lib" ]; then
  work=$(mktemp -d)
  python3 - "$lib" "$work/bad.jar" <<'EOF'
import sys, zipfile
src, dst = sys.argv[1:]
with zipfile.ZipFile(src) as zi, zipfile.ZipFile(dst, 'w') as zo:
    for i in zi.infolist():
        zo.writestr(i.filename, zi.read(i), zipfile.ZIP_STORED)
b = bytearray(open(dst, 'rb').read())
at = b.find(b'reflectlib/Deep.tasty') + len('reflectlib/Deep.tasty') + 40
b[at] ^= 0xff
open(dst, 'wb').write(bytes(b))
EOF
  printf 'import scala.scalajs.reflect.Reflect\n@main def lookup(): Unit = println(Reflect.lookupInstantiatableClass("reflectlib.Hidden").isDefined)\n' > "$work/lookup.scala"
  if TEQ_CACHE_DIR=$work/cache timeout 60 "$TEQ" interp "$work/lookup.scala" --classpath "$work/bad.jar" > /dev/null 2>&1; then
    echo "FAIL an interpreted run with an unreadable jar entry succeeds"
    fail=1
  else
    passed=$((passed + 1))
  fi
  for run in first second; do
    if TEQ_CACHE_DIR=$work/cache timeout 60 "$TEQ" compiler build tests/classpath/js/reflect_jar.scala --classpath "$work/bad.jar" -o "$work/out.js" 2>&1 | grep -q 'Deep.tasty: CRC-32 or size mismatch'; then
      passed=$((passed + 1))
    else
      echo "FAIL corrupt jar entry not reported ($run build)"
      fail=1
    fi
  done
  # What the interpreter's lookup read at run time goes into the cache as a build's search does.
  TEQ_CACHE_DIR=$work/interp timeout 60 "$TEQ" interp tests/classpath/js/reflect_jar.scala --classpath "$lib" > /dev/null 2>&1
  TEQ_CACHE_DIR=$work/js timeout 60 "$TEQ" compiler build tests/classpath/js/reflect_jar.scala --classpath "$lib" -o "$work/out.js" > /dev/null 2>&1
  if cmp -s "$work"/interp/*.jarcache "$work"/js/*.jarcache; then passed=$((passed + 1)); else
    echo "FAIL the interpreter's reflective search is not in the jar cache"
    fail=1
  fi
  # A build registers the classes found for reflective instantiation only when it reaches a
  # lookup, the one reader of a registration. Scala.js registers every class that carries the
  # annotation in its static initializer, which its linker runs whatever the program reaches.
  reach_cp="$lib:$(jar_of munit):$(jar_of munit-diff)"
  printf '@main def run(): Unit = println(reflectlib.Hidden().name)\n' > "$work/uses.scala"
  printf 'import scala.scalajs.reflect.Reflect\n@main def run(): Unit =\n  println(reflectlib.Hidden().name)\n  println(Reflect.lookupInstantiatableClass("reflectlib.Deep").isDefined)\n' > "$work/looks.scala"
  for prog in uses looks; do
    TEQ_CACHE_DIR=$work/reach timeout 60 "$TEQ" compiler build "$work/$prog.scala" --classpath "$reach_cp" -o "$work/$prog.js" > /dev/null 2>&1
  done
  if [ -f "$work/uses.js" ] && ! grep -q 'reflectClass(\|Fingerprint' "$work/uses.js" \
    && grep -q 'reflectClass("reflectlib.Deep"' "$work/looks.js" && grep -q 'reflectClass("munit.Framework"' "$work/looks.js"; then
    passed=$((passed + 1))
  else
    echo "FAIL reflective registrations made without a lookup, or missing with one"
    fail=1
  fi
  rm -rf "$work"
fi
# What follows `--` is the program's (docs/TARGETS.md, "Argument files"): `teq interp` passes an
# `@` there to the program as it is, a file of that name existing or not, as scalac's run does.
work=$(cd "$(mktemp -d)" && pwd -P)
teq=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
printf 'object Args:\n  def main(args: Array[String]): Unit = println(args.mkString("|"))\n' > "$work/Args.scala"
echo replacement > "$work/handle"
got=$(cd "$work" && timeout 120 "$teq" interp Args.scala -- @handle @absent 2>&1)
if [ "$got" = "@handle|@absent" ]; then passed=$((passed + 1)); else echo "FAIL the program's @ arguments after -- (teq interp): $got"; fail=1; fi
rm -rf "$work"
echo "classpath: $passed passed, $skipped skipped"
exit $fail
