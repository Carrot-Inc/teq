#!/bin/bash
# The program's Java sources read for their declarations (src/java, the reference's JavaParsers). Each directory of
# tests/java-sources holds Scala and Java sources that scalac 3.8.4 accepts with the .java read as sources (javac then
# compiling them against its output) and runs as its expected.txt, or refuses with every line of its refused.txt; a
# program's old/ directory holds Java sources javac compiles onto the class path first, a class of the same name the
# program's own source stands for. teq is to check each program, check it to products (the module model's call: the
# Scala sources' TASTy written, the Java sources read beside them), then build it for the JVM to a class directory (one
# line saying the Java sources were read and not compiled), javac to compile its Java sources against that directory
# (vulpix's command), and the run of Main to print expected.txt. Besides: a JVM build to a jar of a program with Java
# sources is refused and writes nothing; a watch session's rebuild after a Java source's edit tells the mismatch its
# Scala reader meets, and one after a constant's edit its literal type; the interpreter refuses a Java source's
# method by its class and source; a match on a Java enum or sealed type is checked for its cases at one worker and at
# eight, from a source, from a class file and from the JDK. Without javac or scala-library's jar in the coursier cache nothing runs (counted not run). TEQ
# names the binary (default: the repository's release build).
cd "$(dirname "$0")/.." || exit 1
root=$PWD
TEQ=${TEQ:-./target/release/teq}
case $TEQ in /*) ;; *) TEQ=$root/$TEQ ;; esac
. tests/support/jars.sh
SL=$(jar_of scala-library)
programs=("$root"/tests/java-sources/*/)
if ! command -v javac > /dev/null 2>&1 || [ ! -f "$SL" ]; then
  echo "skip: javac or scala-library 3.8.4 is missing"
  echo "java-sources: 0 passed, 0 failed, $((${#programs[@]} + 6)) not run"
  exit 0
fi
out=$(mktemp -d "${TMPDIR:-/tmp}/teq-java-sources.XXXXXX") || exit 2
trap 'rm -rf "$out"' EXIT
pass=0
fail=0
ok() { echo "ok   $1"; pass=$((pass + 1)); }
bad() { echo "FAIL $1"; printf '%s\n' "$2" | head -40; fail=$((fail + 1)); }

for dir in "${programs[@]}"; do
  dir=${dir%/}
  name=$(basename "$dir")
  w=$out/$name
  mkdir -p "$w"
  cp=$SL
  if [ -d "$dir/old" ]; then
    if ! log=$(javac -encoding UTF-8 -d "$w/old" "$dir"/old/*.java 2>&1); then bad "$name" "javac of old/: $log"; continue; fi
    cp=$cp:$w/old
  fi
  checked=$(timeout 120 "$TEQ" compiler check "$dir" --exclude "$dir/old" --std scala-library --classpath "$cp" 2>&1)
  status=$?
  if [ -f "$dir/refused.txt" ]; then
    missing=$(while IFS= read -r line; do grep -qF -- "$line" <<< "$checked" || echo "missing: $line"; done < "$dir/refused.txt")
    if [ "$status" = 1 ] && [ -z "$missing" ]; then ok "$name"; else bad "$name" "exit $status; $missing; $checked"; fi
    continue
  fi
  [ "$status" = 0 ] || { bad "$name" "check: exit $status: $checked"; continue; }
  # The module model's call, a check to products: the Scala sources' TASTy written, the Java sources read beside them.
  products=$(timeout 120 "$TEQ" compiler check "$dir" --exclude "$dir/old" --std scala-library --classpath "$cp" --products "$w/products" 2>&1)
  status=$?
  if [ "$status" != 0 ] || [ -z "$(find "$w/products" -name '*.tasty' -print -quit 2> /dev/null)" ]; then
    bad "$name" "check to products: exit $status, TASTy written: $(find "$w/products" -name '*.tasty' 2> /dev/null | wc -l): $products"
    continue
  fi
  built=$(timeout 120 "$TEQ" compiler build "$dir" --exclude "$dir/old" --target jvm --all-mains --std scala-library --classpath "$cp" -o "$w/classes" 2>&1)
  status=$?
  [ "$status" = 0 ] || { bad "$name" "build: exit $status: $built"; continue; }
  grep -q '^teq: [0-9]* Java sources* read for typing and not compiled' <<< "$built" || { bad "$name" "build: no line for its Java sources: $built"; continue; }
  if ! log=$(javac -encoding UTF-8 -classpath "$w/classes:$cp" -d "$w/javac" "$dir"/*.java 2>&1); then bad "$name" "javac: $log"; continue; fi
  got=$(timeout 60 java -cp "$w/classes:$w/javac:$cp" Main 2>&1)
  if [ "$got" = "$(cat "$dir/expected.txt")" ]; then ok "$name"; else bad "$name" "$(diff "$dir/expected.txt" <(printf '%s\n' "$got"))"; fi
done

# A jar would lack the Java sources' classes: refused at the first Java source, nothing written.
jar=$out/app.jar
refused=$(timeout 120 "$TEQ" compiler build "$root/tests/java-sources/directory_reads_java_source" --target jvm --all-mains -o "$jar" 2>&1)
status=$?
if [ "$status" = 1 ] && [ ! -e "$jar" ] && grep -q 'J.java:1:1: error: not supported yet: a jar of a program with Java sources' <<< "$refused"; then
  ok jar-refused
else
  bad jar-refused "exit $status, $( [ -e "$jar" ] && echo written || echo 'not written'): $refused"
fi

# The interpreter has no body of a Java source's method: refused by the class's name and its source.
refused=$(timeout 120 "$TEQ" interp "$root/tests/java-sources/directory_reads_java_source" 2>&1)
status=$?
if [ "$status" = 3 ] && grep -q '^J is a class of the Java source .*directory_reads_java_source/J.java, whose body for n the interpreter does not have$' <<< "$refused"; then
  ok interp-refused
else
  bad interp-refused "exit $status: $refused"
fi

# A watch session: a Java source's member changed from Int to String, its Scala reader's mismatch told by the rebuild.
ws=$out/watch
mkdir -p "$ws/p"
printf 'public class J { public static int n() { return 7; } }\n' > "$ws/p/J.java"
printf 'object Use { val n: Int = J.n() }\n' > "$ws/p/Use.scala"
mkfifo "$ws/cmd" "$ws/ans"
(cd "$ws" && timeout 120 "$TEQ" compiler watch --check p < cmd > ans 2> err) &
wpid=$!
exec 3> "$ws/cmd" 4< "$ws/ans"
read -r -t 60 first <&4
printf 'public class J { public static String n() { return "s"; } }\n' > "$ws/p/J.java"
printf 'build\n\n' >&3
read -r -t 60 second <&4
echo quit >&3
exec 3>&- 4<&-
wait "$wpid" 2> /dev/null
case $first$second in
  '{"ok":true'*'{"ok":false'*'Found:    String'*'Required: Int'*) ok watch-java-edit ;;
  *) bad watch-java-edit "first: ${first:0:200} second: ${second:0:300}" ;;
esac

# A watch session: a Java source's constant changed from 7 to 8, its Scala reader's literal type telling the new one.
wc=$out/watch-constant
mkdir -p "$wc/p"
printf 'public class K { public static final int N = 7; }\n' > "$wc/p/K.java"
printf 'object UseK { val n: 7 = K.N }\n' > "$wc/p/UseK.scala"
mkfifo "$wc/cmd" "$wc/ans"
(cd "$wc" && timeout 120 "$TEQ" compiler watch --check p < cmd > ans 2> err) &
wpid=$!
exec 3> "$wc/cmd" 4< "$wc/ans"
read -r -t 60 first <&4
printf 'public class K { public static final int N = 8; }\n' > "$wc/p/K.java"
printf 'build\n\n' >&3
read -r -t 60 second <&4
echo quit >&3
exec 3>&- 4<&-
wait "$wpid" 2> /dev/null
case $first$second in
  '{"ok":true'*'{"ok":false'*'Found:    8'*'Required: 7'*) ok watch-java-constant-edit ;;
  *) bad watch-java-constant-edit "first: ${first:0:200} second: ${second:0:300}" ;;
esac

# A match on a Java enum or sealed type is checked for its cases, as scalac checks it, at one worker and at eight: an
# enum's constants, whether read from its source or from javac's class file (and the JDK's), and a sealed type's
# subclasses among the build's Java sources; a sealed type of the class path is not checked by its permits.
ex=$out/exhaustive
mkdir -p "$ex/src" "$ex/cp" "$ex/cpsrc" "$ex/jdk"
printf 'public enum E { A, B, C }\n' > "$ex/src/E.java"
printf 'public sealed interface S permits SA, SB {}\n' > "$ex/src/S.java"
printf 'public final class SA implements S {}\n' > "$ex/src/SA.java"
printf 'public final class SB implements S {}\n' > "$ex/src/SB.java"
printf 'object Use {\n  def f(e: E): Int = e match { case E.A => 1 }\n  def g(s: S): Int = s match { case _: SA => 1 }\n}\n' > "$ex/src/Use.scala"
printf 'public enum F { A, B { public int w() { return 2; } }, C; public int w() { return 0; } }\n' > "$ex/cpsrc/F.java"
printf 'public sealed interface T permits TA, TB {}\n' > "$ex/cpsrc/T.java"
printf 'public final class TA implements T {}\n' > "$ex/cpsrc/TA.java"
printf 'public final class TB implements T {}\n' > "$ex/cpsrc/TB.java"
printf 'object Use {\n  def f(e: F): Int = e match { case F.A => 1; case F.C => 3 }\n  def g(t: T): Int = t match { case _: TA => 1 }\n}\nclass X extends T\n' > "$ex/cp/Use.scala"
printf 'import java.time.DayOfWeek\nobject Use {\n  def f(d: DayOfWeek): Int = d match { case DayOfWeek.MONDAY => 1; case DayOfWeek.FRIDAY => 2 }\n}\n' > "$ex/jdk/Use.scala"
if javac -d "$ex/cpc" "$ex"/cpsrc/*.java > /dev/null 2>&1; then
  for w in 1 8; do
    src=$(timeout 120 "$TEQ" compiler check "$ex/src" --std scala-library --classpath "$SL" --threads "$w" 2>&1)
    cp=$(timeout 120 "$TEQ" compiler check "$ex/cp" --std scala-library --classpath "$SL:$ex/cpc" --threads "$w" 2>&1)
    jdk=$(timeout 120 "$TEQ" compiler check "$ex/jdk" --std scala-library --classpath "$SL" --threads "$w" 2>&1)
    want=("Use.scala:2:22: warning: match may not be exhaustive." "It would fail on pattern case: B, C" "Use.scala:3:22: warning: match may not be exhaustive." "It would fail on pattern case: _: SB")
    missing=$(for l in "${want[@]}"; do grep -qF -- "$l" <<< "$src" || echo "source: missing $l"; done)
    grep -qF 'It would fail on pattern case: B' <<< "$cp" && [ "$(grep -c 'match may not be exhaustive' <<< "$cp")" = 1 ] || missing="$missing class path: $cp"
    grep -qF 'It would fail on pattern case: TUESDAY, WEDNESDAY, THURSDAY, SATURDAY, SUNDAY' <<< "$jdk" || missing="$missing jdk: $jdk"
    if [ -z "$missing" ]; then ok "exhaustive-java-$w-workers"; else bad "exhaustive-java-$w-workers" "$missing"; fi
  done
else
  bad exhaustive-java "javac of the class path's sources failed"
fi

echo "java-sources: $pass passed, $fail failed"
[ "$fail" = 0 ]
