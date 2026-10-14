#!/bin/bash
# Runs every tests/cases program through `teq compiler build --target jvm` and `java`, linked against
# scala-library's jar (the JVM's one mode, the jar pinned on the class path and beside the
# program under java), and compares the output with the same .expected files
# as tests/run.sh: real Scala on the JVM produced them, so
# they are the oracle as they are. The tests whose expectations come from Scala.js (a
# `//> using platform js` directive: doubles print as JS prints them) are compared with
# tests/jvm-expected/<name>.expected, which real Scala on the JVM produced from the same source
# without that directive (`scala-cli run -S 3.8.4`); without such a file the test is skipped.
# The expectations are the output on JDK 24, the JDK the JVM target's references run on; a case
# that prints a message of the JDK's own which another major words otherwise (an exception's text)
# has that major's expectation beside, tests/jvm-expected/<name>.jdk<major>.expected, which
# scalac 3.8.4's run on that JDK wrote (`scala-cli run -S 3.8.4 --jvm system`, that JDK's java
# first on the PATH, from the repository's root under TZ=UTC), and which a run on that major compares.
#
# tests/jvm-passing.txt lists the tests that pass; a listed test that fails is a regression and
# fails the script, an unlisted one that passes is reported so that it can be added
# (`--update` rewrites the list). Failures are classified:
#   backend   the JVM backend reported a construct it does not support (exit code 3)
#   compile   any other compile failure
#   verify    the JVM rejected a class (VerifyError, ClassFormatError, linkage errors)
#   exception the program threw
#   output    the program ran and printed something else
#   timeout   the program ran too long
# A `// jars: <names>` line puts those jars on the class path (tests/support/jars.sh); a test with
# one of them missing from the coursier cache counts as passed.
# Several tests run at once (JOBS, by default the number of cores).
# `--java-output-version=N` writes the class files for that Java release in the place of the
# default, against the same list.
# At the default version the class files are also checked to be a function of the sources: the
# API side of the application corpus (bench/app), large enough for the parser's threads, is built
# DETERMINISM_BUILDS times (5 by default) and the jars have to agree byte for byte; without the
# corpus's jars in the coursier cache the check is left out. And the static forwarders of a
# mirror class have to stand in scalac's order: javap's listing of
# tests/support/forwarders/forwarders.scala against forwarders.expected, javap's of scalac's
# class file (left out without javap). And a class's constructor calls the `$init$` of the traits it mixes in anew in
# scalac's order: tests/support/trait_inits/list.sh's listing of trait_inits.scala against trait_inits.expected,
# listed from scalac's class files (left out without javap). And an anonymous class's methods have the flags
# scalac gives them, a `final def` final, the class's own and a quote's copy at an expansion's site:
# tests/support/anon_final/list.sh's listing against anon_final.expected, listed from scalac's class files. And each program of tests/support/abi, built as a module's
# products, has scalac's layout of the members its `// abi:` line names: the names, descriptors and
# flags tests/support/abi/listing.py reads from its class files against <name>.expected, which
# tests/support/abi/expected.sh writes from scalac 3.8.4's.
cd "$(dirname "$0")/.."
# The lists are sorted bytewise: `sort -u` and `comm` under a locale's collation would take
# `tail_calls` and `tailcalls` for one name. The character set stays the locale's, which java's
# output encoding follows.
export LC_COLLATE=C
TEQ=${TEQ:-./target/release/teq}
export TZ=UTC  # an archive entry's DOS time is local time: the recorded bytes are UTC's
JDK_MAJOR=$(java -version 2>&1 | sed -nE '1s/^[^"]*"([0-9]+).*/\1/p')
JOBS=${JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)}
LIST=tests/jvm-passing.txt
out=out/jvm-tests
update=0
VERSION=
for arg in "$@"; do
  case $arg in
    --update) update=1 ;;
    --java-output-version=*) VERSION=${arg#*=} ;;
  esac
done
[ -n "$VERSION" ] && out=$out-$VERSION
rm -rf "$out"
mkdir -p "$out"

# A `// os: windows` or `// os: unix` line keeps a case to that system (Git's bash is Windows): on the other
# it is skipped, and its line in the list stands as it is.
case $(uname -s) in MINGW* | MSYS* | CYGWIN*) HERE_OS=windows ;; *) HERE_OS=unix ;; esac
other_os() {
  local os
  os=$(grep -h -o '^// os: [a-z]*' "$1" "$1"/*.scala 2> /dev/null | head -1)
  [ -n "$os" ] && [ "${os#// os: }" != "$HERE_OS" ]
}
export HERE_OS
export -f other_os

run_one() {
  src=$1
  name=$(basename "$src" .scala)
  expected="tests/cases/$name.expected"
  if other_os "$src"; then
    echo "skip $name" > "$out/$name.result"
    echo "$name" > "$out/$name.otheros"
    return
  fi
  if grep -q -h '^//> using platform js' "$src" "$src"/*.scala 2>/dev/null; then
    expected="tests/jvm-expected/$name.expected"
    if [ ! -f "$expected" ]; then
      echo "skip $name" > "$out/$name.result"
      return
    fi
  fi
  [ -f "tests/jvm-expected/$name.jdk$JDK_MAJOR.expected" ] && expected="tests/jvm-expected/$name.jdk$JDK_MAJOR.expected"
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  . tests/support/jars.sh
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "pass $name" > "$out/$name.result"
    echo "$name" > "$out/$name.nojar"
    return
  fi
  cp=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar${JARS_CP:+:$JARS_CP}
  flags="$flags --classpath $cp"
  [ -n "$VERSION" ] && flags="$flags --java-output-version $VERSION"
  timeout 20 "$TEQ" compiler build "$src" $flags --target jvm -o "$out/$name.jar" > "$out/$name.compile" 2>&1
  code=$?
  if [ $code -eq 3 ]; then
    echo "backend $name" > "$out/$name.result"
    return
  elif [ $code -ne 0 ]; then
    echo "compile $name" > "$out/$name.result"
    return
  fi
  # no perf data file: on Linux a JVM whose pid's file another process holds warns about it on stdout
  timeout 30 java -Xss512m -XX:+UseSerialGC -XX:TieredStopAtLevel=1 -XX:-UsePerfData -Xshare:auto -cp "$out/$name.jar${cp:+:$cp}" TeqMain > "$out/$name.actual" 2> "$out/$name.err"
  code=$?
  if [ $code -eq 0 ] && diff -q "$expected" "$out/$name.actual" > /dev/null; then
    echo "pass $name" > "$out/$name.result"
  elif [ $code -eq 124 ]; then
    echo "timeout $name" > "$out/$name.result"
  elif grep -q -E 'VerifyError|ClassFormatError|IncompatibleClassChangeError|NoSuchMethodError|NoSuchFieldError|AbstractMethodError|NoClassDefFoundError|IllegalAccessError|BootstrapMethodError|LambdaConversionException' "$out/$name.err"; then
    echo "verify $name" > "$out/$name.result"
  elif [ $code -ne 0 ]; then
    echo "exception $name" > "$out/$name.result"
  else
    echo "output $name" > "$out/$name.result"
  fi
}
export -f run_one
export TEQ VERSION out JDK_MAJOR

for src in tests/cases/*.scala tests/cases/*/; do echo "${src%/}"; done | xargs -P "$JOBS" -I{} bash -c 'run_one {}'

cat "$out"/*.result | sort > "$out/results.txt"
status=0
for kind in backend compile verify exception output timeout; do
  names=$(grep "^$kind " "$out/results.txt" | cut -d' ' -f2 | tr '\n' ' ')
  [ -n "$names" ] && echo "$kind: $names"
done
nojar=$(cat "$out"/*.nojar 2> /dev/null | tr '\n' ' ')
[ -n "$nojar" ] && echo "counted as passed, a jar of theirs not in the coursier cache: $nojar"
cat "$out"/*.otheros 2> /dev/null | sort > "$out/otheros.txt"
skipped=$(grep '^skip ' "$out/results.txt" | cut -d' ' -f2 | sort | comm -23 - "$out/otheros.txt" | tr '\n' ' ')
[ -n "$skipped" ] && echo "skipped (expectations from Scala.js): $skipped"
otheros=$(tr '\n' ' ' < "$out/otheros.txt")
[ -n "$otheros" ] && echo "skipped (another system's): $otheros"
grep '^pass ' "$out/results.txt" | cut -d' ' -f2 | sort > "$out/passing.txt"
touch "$LIST"
if [ $update = 1 ]; then
  sort -u "$out/passing.txt" <(sort "$LIST" | comm -12 - "$out/otheros.txt") > "$LIST.new" && mv "$LIST.new" "$LIST"
fi
regressions=$(comm -23 <(sort "$LIST") <(sort -u "$out/passing.txt" "$out/otheros.txt") | tr '\n' ' ')
new=$(comm -13 <(sort "$LIST") "$out/passing.txt" | tr '\n' ' ')
[ -n "$new" ] && echo "passing but not in $LIST: $new"
if [ -n "$regressions" ]; then
  echo "REGRESSIONS: $regressions"
  status=1
fi

same_jars() {
  . tests/support/jars.sh
  local cp="" name path
  for name in scala-library cats-kernel cats-core sourcecode; do
    path=$(jar_of "$name")
    [ -e "$path" ] || return 0
    cp="$cp:$path"
  done
  cp=${cp#:}
  local work=$out/corpus
  if ! timeout 120 python3 bench/app/gen.py "$work/src" > "$work.gen.log" 2>&1; then
    echo "DIFFERENT BUILDS: the corpus was not generated (see $work.gen.log)"
    return 1
  fi
  local i sums=""
  for i in $(seq 1 "${DETERMINISM_BUILDS:-5}"); do
    if ! timeout 120 "$TEQ" compiler build "$work/src/shared" "$work/src/api" --classpath "$cp" --target jvm -o "$work/api-$i.jar" > "$work/api-$i.log" 2>&1; then
      echo "DIFFERENT BUILDS: build $i of the corpus failed (see $work/api-$i.log)"
      return 1
    fi
    sums="$sums $(cksum < "$work/api-$i.jar" | tr ' ' '-')"
  done
  local distinct
  distinct=$(printf '%s\n' $sums | sort -u | wc -l | tr -d ' ')
  if [ "$distinct" != 1 ]; then
    echo "DIFFERENT BUILDS: $distinct distinct jars of the api corpus in ${DETERMINISM_BUILDS:-5} builds ($work/api-*.jar)"
    return 1
  fi
}
forwarder_order() {
  command -v javap > /dev/null || return 0
  local work=$out/forwarders
  if ! timeout 60 "$TEQ" compiler build tests/support/forwarders/forwarders.scala --target jvm --classpath "$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar" -o "$work" > "$work.log" 2>&1; then
    echo "FORWARDERS: the fixture does not build (see $work.log)"
    return 1
  fi
  javap -p -s "$work/p/statusCode.class" | awk '/^  public static/ { name = $0; sub(/\(.*/, "", name); sub(/.* /, "", name) } /descriptor:/ { print name $2 }' > "$work.txt"
  if ! diff tests/support/forwarders/forwarders.expected "$work.txt" > "$work.diff"; then
    echo "FORWARDERS: the mirror class's forwarders are not in scalac's order"
    head -10 "$work.diff"
    return 1
  fi
}
anon_final() {
  command -v python3 > /dev/null || return 0
  local work=$out/anon_final
  rm -rf "$work"
  if ! timeout 60 "$TEQ" compiler build tests/support/anon_final/Mac.scala tests/support/anon_final/Use.scala --target jvm --products "$work" --classpath "$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar" > "$work.log" 2>&1; then
    echo "ANON FINAL: the fixture does not build (see $work.log)"
    return 1
  fi
  bash tests/support/anon_final/list.sh "$work" > "$work.txt"
  if ! diff tests/support/anon_final/anon_final.expected "$work.txt" > "$work.diff"; then
    echo "ANON FINAL: an anonymous class's methods, plain or a quote's copy, do not have scalac's flags"
    head -10 "$work.diff"
    return 1
  fi
}
trait_inits() {
  command -v javap > /dev/null || return 0
  local work=$out/trait_inits
  rm -rf "$work"
  if ! timeout 60 "$TEQ" compiler build tests/support/trait_inits/trait_inits.scala --target jvm --products "$work" > "$work.log" 2>&1; then
    echo "TRAIT INITS: the fixture does not build (see $work.log)"
    return 1
  fi
  bash tests/support/trait_inits/list.sh "$work" > "$work.txt"
  if ! diff tests/support/trait_inits/trait_inits.expected "$work.txt" > "$work.diff"; then
    echo "TRAIT INITS: a constructor's trait initialisers are not scalac's"
    head -10 "$work.diff"
    return 1
  fi
}
abi_layout() {
  command -v python3 > /dev/null || return 0
  local f name work fails=0
  mkdir -p "$out/abi"
  for f in tests/support/abi/*.scala; do
    name=$(basename "$f" .scala)
    work=$out/abi/$name
    if ! timeout 60 "$TEQ" compiler build "$f" --target jvm --products "$work" --classpath "$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar" > "$work.log" 2>&1; then
      echo "ABI: tests/support/abi/$name.scala does not build (see $work.log)"
      fails=1
      continue
    fi
    python3 tests/support/abi/listing.py "$work" $(sed -n 's|^// abi: ||p' "$f") > "$work.txt"
    if ! diff "tests/support/abi/$name.expected" "$work.txt" > "$work.diff"; then
      echo "ABI: the class files of tests/support/abi/$name.scala are not in scalac's layout"
      head -10 "$work.diff"
      fails=1
    fi
  done
  return $fails
}
if [ -z "$VERSION" ]; then
  same_jars || status=1
  forwarder_order || status=1
  trait_inits || status=1
  anon_final || status=1
  abi_layout || status=1
fi
count() { grep -c "^$1 " "$out/results.txt"; }
echo "$(count pass) passed, $(count backend) backend, $(count compile) compile, $(count verify) verify, $(count exception) exception, $(count output) output, $(count timeout) timeout, $(count skip) skipped"
exit $status
