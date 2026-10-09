#!/bin/bash
# The JVM build under watch (`teq compiler watch --target jvm -o dir`): a session is driven over stdin
# and the class files after every build are compared byte for byte with a fresh
# `teq compiler build --target jvm --all-mains` of the same sources, every build linked against
# scala-library's jar, pinned on the class path (which the suite skips without). First the
# scripted scenarios on a copy of tests/split/classpath: a session whose every second build takes the full path for the session's memory
# (the analysis of every owned file in its answer), the first build's analysis, a body edit
# (one class rewritten), a build that changed
# nothing (nothing rewritten, a deleted class file put back), a signature change, a class added
# and removed, a type error kept across an unchanged retry, a parse error holding the output
# back, a `text` overlay, an object gaining a `main`; a first build with a file that fails to
# parse (its recovered tree typed, its parse errors the only diagnostics) and one with a jar
# missing until the next build; the representative shapes of a syntax error broken and fixed, in
# both files in either order and under compaction, the class files a fresh build's after the
# fix. Then the inline program
# (tests/split/inline): an edit of an inline body types the files that expanded it again. Then
# the owned program (tests/split/owned): under `--own` the class files of one root are those of the whole
# program's build. Then every program of tests/jvm-passing.txt, and the few that build without
# printing scalac's output (`WATCH_ONLY`), gets a comment inserted and its first string literal
# changed, each followed by an incremental build and the comparison; a program's `// jars:`
# line puts those jars on its class path, and without one of them in the coursier cache it
# counts as passed. Last the sessions of tests/support/retype.sh, in which one file is typed
# again after another. Every session types its full builds at the automatic count and every fresh
# build it is compared with by one worker (tests/support/sessions.sh); the last line says which
# sessions forked.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
. tests/support/jars.sh
. tests/support/sessions.sh
SL=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
expect() {
  if [ "$2" = "$3" ]; then ok; else bad "$1: expected $3, found $2"; fi
}

work=$(mktemp -d "${TMPDIR:-/tmp}/teq-jvm-watch.XXXXXX")
trap 'stop; rm -rf "$work"' EXIT
sessions_log "$work/workers.log"
WPID=
# Starts a session on the given teq arguments (the inputs and flags; the target and the output
# directory are added); commands go to fd 3, answers come from fd 4.
start() {
  rm -f "$work/cmd" "$work/ans"
  mkfifo "$work/cmd" "$work/ans"
  "$TEQ" compiler watch "$@" --target jvm -o "$work/out" < "$work/cmd" > "$work/ans" 2> "$work/err" &
  WPID=$!
  session_started "$WPID" "$1"
  exec 3> "$work/cmd" 4< "$work/ans"
  answer
}
stop() {
  if [ -n "$WPID" ]; then
    echo quit >&3 2> /dev/null
    exec 3>&- 4<&-
    wait "$WPID" 2> /dev/null
    WPID=
  fi
}
RESULT=
# Reads one JSON answer (bounded: the session is killed after 30 s of silence).
answer() {
  RESULT=
  if ! read -r -t 30 RESULT <&4; then
    bad "no answer from teq compiler watch (stderr: $(head -c 300 "$work/err"))"
    kill "$WPID" 2> /dev/null
    RESULT='{"ok":false,"timeout":true}'
  fi
}
build() {
  if [ $# = 0 ]; then
    echo build >&3
  else
    echo "build $1" >&3
    shift
    for p in "$@"; do echo "$p" >&3; done
    echo >&3
  fi
  answer
}
text() {
  echo "text $1 $(wc -c < "$2" | tr -d ' ')" >&3
  cat "$2" >&3
}
withdraw() { echo "text $1 0" >&3; }
field() { echo "$RESULT" | grep -o "\"$1\":\(\[[^]]*\]\|[^,}]*\)" | head -1 | sed "s/\"$1\"://"; }
has() { case "$RESULT" in *"$1"*) return 0 ;; *) return 1 ;; esac; }
# Compares the session's output directory with a fresh build of the same sources: the same set
# of class files, byte for byte. A successful build deletes the class files it has no class for
# (a class gone, an anonymous class named by a position that moved), so the sets agree.
same_as_fresh() {
  local what=$1
  shift
  rm -rf "$work/fresh"
  if ! "$TEQ" compiler build "$@" $ONE --target jvm -o "$work/fresh" --all-mains > "$work/fresh.log" 2>&1; then
    bad "$what: the fresh build failed: $(head -3 "$work/fresh.log")"
    return
  fi
  local class differs=0
  for class in $(cd "$work/fresh" && find . -name '*.class'); do
    if ! cmp -s "$work/fresh/$class" "$work/out/$class"; then
      [ $differs = 0 ] && bad "$what: watch output differs from a fresh build"
      differs=$((differs + 1))
      [ $differs -le ${DIFF_LINES:-10} ] && echo "  $class"
    fi
  done
  for class in $(cd "$work/out" && find . -name '*.class'); do
    if [ ! -f "$work/fresh/$class" ]; then
      [ $differs = 0 ] && bad "$what: watch output holds class files a fresh build has not"
      differs=$((differs + 1))
      [ $differs -le ${DIFF_LINES:-10} ] && echo "  $class"
    fi
  done
  [ $differs = 0 ] && ok
}
run_out() { timeout 30 java -Xss512m -XX:+UseSerialGC -XX:TieredStopAtLevel=1 -XX:-UsePerfData -cp "$work/out:$SL" "$@" 2>&1; }

if [ ! -f "$SL" ]; then
  echo "skip: scala-library 3.8.4 is not in the coursier cache"
  echo "0 passed, 0 failed"
  exit 0
fi
PIN="--classpath $SL"
retype_linked=1
retype_start() { start "$@"; }
after_failed_full=false
as_fresh() { same_as_fresh "$@"; }
retype_runs() { expect "$1" "$(run_out "$3")" "$2"; }
retype_queries() { :; }
. tests/support/retype.sh

# --- The scripted scenarios ---------------------------------------------------------------------
# A session that takes the full path of its own accord (`TEQ_COMPACT_EVERY`, src/watch.rs), at
# every second build here, answers with the analysis of every file it owns, as a build tool
# that deletes what a full answer does not list has to be told, and writes the class that
# changed alone.
src=$work/own
cp -r tests/split/classpath "$src"
TEQ_COMPACT_EVERY=2 start "$src" $PIN
sed -i.bak 's/mkString("\[", ",", "\]")/mkString("<", ",", ">")/' "$src/util.scala"
build "$src/util.scala"
expect "own full build: the build before" "$(field ok),$(field incremental),$(field changed)" 'true,true,["util/Fmt$.class"]'
sed -i.bak 's/mkString("<", ",", ">")/mkString("(", ",", ")")/' "$src/util.scala"
build "$src/util.scala"
expect "own full build: the second build" "$(field ok),$(field incremental),$(field fallback),$(field changed)" 'true,false,"the session'"'"'s memory",["util/Fmt$.class"]'
if has '"file":"'"$src"'/main.scala"' && has '"file":"'"$src"'/util.scala"'; then ok; else bad "own full build: the analysis of every file: $RESULT"; fi
same_as_fresh "own full build" "$src" $PIN
expect "own full build runs" "$(run_out app.Main)" "(1,2,3)"
stop
rm -rf "$work/out"

# The other side of what a retype adds outside its units (tests/split-watch.sh has the first): a
# retype of inline_fold_std folds through scala-library's bodies, which adds template calls the
# walk calls wherever they stand, so the reach is walked from the roots again (src/emit/kept.rs).
src=$work/fold
mkdir -p "$src"
cp tests/cases/inline_fold_std.scala "$src/"
TEQ_SESSION_PARTS=1 TEQ_KEPT_SETTLE=wait start "$src" $PIN
expect "inline_fold_std: first build" "$(field ok)" 'true'
printf '// a comment\n' | cat - "$src/inline_fold_std.scala" > "$src/fold.tmp" && mv "$src/fold.tmp" "$src/inline_fold_std.scala"
build "$src/inline_fold_std.scala"
expect "inline_fold_std: a comment inserted" "$(field ok),$(field incremental),$(field kept)" 'true,true,false'
if has 'a template call of'; then ok; else bad "inline_fold_std: walked for another reason: $(field walked)"; fi
same_as_fresh "inline_fold_std: a comment inserted" "$src" $PIN
stop
rm -rf "$work/out"

# The kept class files (src/jvm/kept.rs): a retype takes the class files of the units it did not
# type again from the last build where the facts their emission reads are the same, and emits
# every unit again where they are not. Each scenario below changes, by a body edit in one file,
# the class files of a class in another, untouched file; the session's output is compared with a
# fresh build's, and the parts say whether the store gave units and why not. The assertion build
# emits every kept unit again and compares (`TEQ_KEPT_EMIT_CHECK`).
kept_start() {
  rm -rf "$work/out"
  TEQ_SESSION_PARTS=1 TEQ_KEPT_SETTLE=wait start "$1" $PIN
  expect "$2: first build" "$(field ok)" 'true'
}
kept_edit() {
  local what=$1 file=$2 from=$3 to=$4 why=$5
  sed -i.bak "s|$from|$to|" "$file" && rm -f "$file.bak"
  build "$file"
  expect "$what" "$(field ok),$(field incremental)" 'true,true'
  if [ -z "$why" ]; then
    local n
    n=$(echo "$RESULT" | grep -o '"units kept":\[[0-9]*' | grep -o '[0-9]*$')
    if [ "${n:-0}" -gt 0 ]; then ok; else bad "$what: no unit kept: $RESULT"; fi
  elif has "\"$why"; then ok; else bad "$what: kept, or not for '$why': $RESULT"; fi
}
src=$work/kept1
mkdir -p "$src"
cat > "$src/base.scala" <<'EOF'
package k
trait Describe:
  def describe: String = "base"
trait Loud extends Describe:
  override def describe: String = "loud"
EOF
cat > "$src/impl.scala" <<'EOF'
package k
class Impl extends Loud
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def main(args: Array[String]): Unit = println(Impl().describe + " " + Other.n)
object Other:
  def n: Int = 1
EOF
kept_start "$src" "kept: super accessor"
kept_edit "kept: a literal elsewhere" "$src/main.scala" 'n: Int = 1' 'n: Int = 2' ''
same_as_fresh "kept: a literal elsewhere" "$src" $PIN
kept_edit "kept: super.describe added" "$src/base.scala" '"loud"' 'super.describe + "!"' "not kept: the classes' facts differ"
same_as_fresh "kept: super.describe added" "$src" $PIN
expect "kept: super.describe added runs" "$(run_out k.Main)" "base! 2"
stop

src=$work/kept2
mkdir -p "$src"
cat > "$src/c.scala" <<'EOF'
package k
class C:
  val x: Int = 1
  def get: Int = x
trait Tr:
  def m: Int = 1
class Impl extends Tr
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def main(args: Array[String]): Unit = println(C().get.toString + " " + Impl().m)
EOF
kept_start "$src" "kept: facts of another file"
kept_edit "kept: an anonymous subclass made elsewhere" "$src/main.scala" 'C().get' '(new C { override def toString = "c" }).get' "not kept"
if has '"k/C.class"'; then ok; else bad "kept: an anonymous subclass made elsewhere: C rewritten: $(field changed)"; fi
same_as_fresh "kept: an anonymous subclass made elsewhere" "$src" $PIN
expect "kept: an anonymous subclass made elsewhere runs" "$(run_out k.Main)" "1 1"
stop

# A private member's name in the class files: prefixed with its class's when a class above or
# below defines the name, which an anonymous subclass made in another file can begin to do.
src=$work/kept5
mkdir -p "$src"
cat > "$src/base.scala" <<'EOF'
package k
class Base:
  private def n: Int = 1
  def get: Int = n
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def make: Base = new Base { def other: Int = 2 }
  def main(args: Array[String]): Unit = println(make.get)
EOF
kept_start "$src" "kept: a private name"
kept_edit "kept: a private name clashes" "$src/main.scala" 'def other' 'def n' "not kept"
same_as_fresh "kept: a private name clashes" "$src" $PIN
expect "kept: a private name clashes runs" "$(run_out k.Main)" "1"
stop

# The same for a private extension method, which the class lists apart from its members.
src=$work/kept5x
mkdir -p "$src"
cat > "$src/base.scala" <<'EOF'
package k
class Base:
  extension (x: Int)
    private def n: Int = x + 1
  def get: Int = 0.n
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def make: Base = new Base { def other(x: Int): Int = 2 }
  def main(args: Array[String]): Unit = println(make.get)
EOF
kept_start "$src" "kept: a private extension's name"
kept_edit "kept: a private extension's name clashes" "$src/main.scala" 'def other' 'def n' "not kept"
same_as_fresh "kept: a private extension's name clashes" "$src" $PIN
expect "kept: a private extension's name clashes runs" "$(run_out k.Main)" "1"
stop

# A super accessor the binding of mixins writes into an anonymous class of an untouched file.
src=$work/kept6
mkdir -p "$src"
cat > "$src/traits.scala" <<'EOF'
package k
trait Parent:
  def m: Int = 1
trait Tr extends Parent:
  override def m: Int = 2
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def plain: Parent = new Parent {}
  def make: Tr = new Tr {}
  def main(args: Array[String]): Unit = println(plain.m + make.m)
EOF
kept_start "$src" "kept: an anonymous class's super accessor"
kept_edit "kept: an anonymous class's super accessor" "$src/traits.scala" 'override def m: Int = 2' 'override def m: Int = super.m + 1' "not kept"
same_as_fresh "kept: an anonymous class's super accessor" "$src" $PIN
expect "kept: an anonymous class's super accessor runs" "$(run_out k.Main)" "3"
stop

# A trait's super accessors are declared from the records of the classes that mix it in: the first
# anonymous class to do so, made in another file, gives the untouched trait an accessor to declare.
src=$work/kept7
mkdir -p "$src"
cat > "$src/t.scala" <<'EOF'
package k
trait Base:
  def m: Int = 1
trait T extends Base:
  override def m: Int = super.m + 1
class D extends Base
object Use:
  def call(t: T): Int = if t == null then 0 else t.m
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def main(args: Array[String]): Unit = println(Use.call(null) + D().m)
EOF
kept_start "$src" "kept: a trait's first anonymous mixin"
kept_edit "kept: a trait's first anonymous mixin" "$src/main.scala" 'Use.call(null)' 'Use.call(new T {})' "not kept"
if has '"k/T.class"'; then ok; else bad "kept: a trait's first anonymous mixin: T rewritten: $(field changed)"; fi
same_as_fresh "kept: a trait's first anonymous mixin" "$src" $PIN
expect "kept: a trait's first anonymous mixin runs" "$(run_out k.Main)" "3"
stop

# A member's name in the class files: its `@targetName` once an overload meets it, which an
# anonymous subclass made in another file can begin to do.
src=$work/kept8
mkdir -p "$src"
cat > "$src/base.scala" <<'EOF'
package k
import scala.annotation.targetName
class Base:
  @targetName("first") def m(x: String): Int = 1
  def call: Int = m("a")
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def make: Base = new Base { def other(x: Int): Int = 2 }
  def main(args: Array[String]): Unit = println(make.call)
EOF
kept_start "$src" "kept: an overload made elsewhere"
kept_edit "kept: an overload made elsewhere" "$src/main.scala" 'def other' 'def m' "not kept"
same_as_fresh "kept: an overload made elsewhere" "$src" $PIN
expect "kept: an overload made elsewhere runs" "$(run_out k.Main)" "1"
stop

# A body edited while another file's error stops the build: the next build, asked for the fixed
# file alone, writes the first file's classes too.
src=$work/kept3
mkdir -p "$src"
cat > "$src/a.scala" <<'EOF'
package k
object A:
  def f: Int = 1
EOF
cat > "$src/b.scala" <<'EOF'
package k
object B:
  def g: Int = 2
EOF
cat > "$src/main.scala" <<'EOF'
package k
object Main:
  def main(args: Array[String]): Unit = println(A.f + B.g)
EOF
kept_start "$src" "kept: an error between"
sed -i.bak 's/f: Int = 1/f: Int = 10/' "$src/a.scala" && sed -i.bak 's/g: Int = 2/g: Int = "two"/' "$src/b.scala" && rm -f "$src"/*.bak
build "$src/a.scala" "$src/b.scala"
expect "kept: an error between: the failed build" "$(field ok)" 'false'
sed -i.bak 's/g: Int = "two"/g: Int = 20/' "$src/b.scala" && rm -f "$src/b.scala.bak"
build "$src/b.scala"
expect "kept: an error between: the fix" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "kept: an error between: the fix" "$src" $PIN
expect "kept: an error between runs" "$(run_out k.Main)" "30"
stop

# A member a class gains from a parent trait in another file: a member added is no body's edit,
# and the build takes the full path, which keeps nothing.
src=$work/kept4
mkdir -p "$src"
cat > "$src/p.scala" <<'EOF'
package k
trait P:
  def a: Int
EOF
cat > "$src/q.scala" <<'EOF'
package k
class Q extends P:
  def a: Int = 1
  val b: Int = 2
object Main:
  def main(args: Array[String]): Unit = println(Q().a + Q().b)
EOF
kept_start "$src" "kept: a parent's member"
printf 'package k\ntrait P:\n  def a: Int\n  def b: Int\n' > "$src/p.scala"
build "$src/p.scala"
expect "kept: a parent's member added" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "kept: a parent's member added" "$src" $PIN
stop
rm -rf "$work/out"

src=$work/src
cp -r tests/split/classpath "$src"
start "$src" $PIN
expect "first build" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"first build"'
if has '"file":"'"$src"'/main.scala","classes":[{"name":"app.Main","kind":"object","top":true,"public":true,"abstract":false,"final":true,"bases":[],"annotations":[],"methodAnnotations":[],"main":true,"files":["app/Main$.class","app/Main.class"]}],"local":[]}'; then ok; else bad "first build: the analysis of main.scala: $RESULT"; fi
if has '"changed":["app/Main$.class","app/Main.class","scala/runtime/jvm$package$.class","util/Fmt$.class","util/Fmt.class"]'; then ok; else bad "first build: the classes written: $(field changed)"; fi
same_as_fresh "first build" "$src" $PIN
expect "first build runs" "$(run_out app.Main)" "[1,2,3]"

sed -i.bak 's/mkString("\[", ",", "\]")/mkString("<", ",", ">")/' "$src/util.scala"
build "$src/util.scala"
expect "body edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["util/Fmt$.class"]'
if has '"analysis":[{"file":"'"$src"'/util.scala"'; then ok; else bad "body edit: the analysis of the retyped file alone: $RESULT"; fi
same_as_fresh "body edit" "$src" $PIN
expect "body edit runs" "$(run_out app.Main)" "<1,2,3>"

build "$src/util.scala"
expect "unchanged file" "$(field ok),$(field incremental),$(field changed),$(field analysis)" 'true,true,[],'
rm "$work/out/util/Fmt$.class"
build
expect "a deleted class file is put back" "$(field ok),$(field changed)" 'true,["util/Fmt$.class"]'
same_as_fresh "a deleted class file is put back" "$src" $PIN

sed -i.bak 's/def show(xs: List\[Int\]): String/def show(xs: List[Int], sep: String = ","): String/; s/mkString("<", ",", ">")/mkString("<", sep, ">")/' "$src/util.scala"
build "$src/util.scala"
expect "signature change" "$(field ok),$(field incremental)" 'true,false'
if has "show: parameters changed"; then ok; else bad "signature change: fallback reason: $(field fallback)"; fi
if has '"file":"'"$src"'/main.scala"' && has '"file":"'"$src"'/util.scala"'; then ok; else bad "signature change: the analysis of every file: $RESULT"; fi
same_as_fresh "signature change" "$src" $PIN

cat > "$src/extra.scala" <<'EOF'
package util

class Tag(val label: String):
  def render: String = "#" + label
EOF
build "$src/extra.scala"
expect "class added" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"files added or removed"'
if has '"name":"util.Tag","kind":"class","top":true,"public":true,"abstract":false,"final":false,"bases":[],"annotations":[],"methodAnnotations":[],"main":false,"files":["util/Tag.class"]'; then ok; else bad "class added: its analysis: $RESULT"; fi
same_as_fresh "class added" "$src" $PIN
rm "$src/extra.scala"
build "$src/extra.scala"
expect "class removed" "$(field ok),$(field incremental),$(field removed),$(field deleted)" 'true,false,["'"$src"'/extra.scala"],["util/Tag.class"]'
if [ ! -f "$work/out/util/Tag.class" ]; then ok; else bad "class removed: its class file stays"; fi
same_as_fresh "class removed" "$src" $PIN

# A deletion whose build fails: the class file goes with the build that passes, and the full
# answer of that build lists every file left, the deleted one not among them.
cat > "$src/extra.scala" <<'EOF'
package util

class Tag(val label: String):
  def render: String = "#" + label
EOF
cat > "$src/tags.scala" <<'EOF'
package util

object Tags:
  val first: Tag = Tag("first")
EOF
build "$src/extra.scala" "$src/tags.scala"
expect "class and its use added" "$(field ok),$(field incremental)" 'true,false'
rm "$src/extra.scala"
build "$src/extra.scala"
expect "class removed under a use" "$(field ok),$(field incremental),$(field removed)" 'false,false,["'"$src"'/extra.scala"]'
if [ -f "$work/out/util/Tag.class" ]; then ok; else bad "class removed under a use: the failed build deleted its class file"; fi
rm "$src/tags.scala"
build "$src/tags.scala"
expect "use removed after the failed deletion" "$(field ok),$(field incremental),$(field removed),$(field deleted)" 'true,false,["'"$src"'/tags.scala"],["util/Tag.class","util/Tags$.class","util/Tags.class"]'
if ! has '"file":"'"$src"'/extra.scala"' && ! has '"file":"'"$src"'/tags.scala"' && has '"file":"'"$src"'/util.scala"'; then ok; else bad "use removed: the full answer lists the files left: $RESULT"; fi
same_as_fresh "use removed after the failed deletion" "$src" $PIN

cp -r "$work/out" "$work/before-error"
sed -i.bak 's/mkString("<", sep, ">")/mkString("<", sep, 42)/' "$src/util.scala"
build "$src/util.scala"
expect "type error" "$(field ok),$(field incremental)" 'false,true'
if has '"severity":"error"' && has '"file":"'"$src"'/util.scala","line":4'; then ok; else bad "type error diagnostic: $RESULT"; fi
build "$src/util.scala"
expect "type error kept on an unchanged retry" "$(field ok),$(field incremental)" 'false,true'
if has '"severity":"error"'; then ok; else bad "unchanged retry: no diagnostic: $RESULT"; fi
build
expect "type error kept on a plain build" "$(field ok)" 'false'
if diff -r "$work/out" "$work/before-error" > /dev/null; then ok; else bad "type error: the output changed"; fi
sed -i.bak 's/mkString("<", sep, 42)/mkString("<", sep, ">")/' "$src/util.scala"
build "$src/util.scala"
expect "fix after type error" "$(field ok),$(field incremental),$(field changed)" 'true,true,[]'
same_as_fresh "fix after type error" "$src" $PIN

sed -i.bak 's/def show(xs: List\[Int\], sep: String = ","): String =/def show(xs: List[Int], sep: String = ","): String = (/' "$src/util.scala"
build "$src/util.scala"
expect "parse error" "$(field ok)" 'false'
if has '"file":"'"$src"'/util.scala"'; then ok; else bad "parse error position: $RESULT"; fi
sed -i.bak 's/xs.mkString("<", sep, ">")/xs.mkString("<", sep, ">")/' "$src/main.scala"
sed -i.bak 's/println(util.Fmt.show(xs.sorted))/println(util.Fmt.show(xs.sorted.reverse))/' "$src/main.scala"
build "$src/main.scala"
expect "parse failure elsewhere holds the output back" "$(field ok),$(field incremental)" 'false,true'
if diff -r "$work/out" "$work/before-error" > /dev/null; then ok; else bad "parse failure: the output changed"; fi
sed -i.bak 's/): String = ( xs/): String = xs/' "$src/util.scala"
build "$src/util.scala"
expect "parse failure cleared, the held-back build emitted" "$(field ok),$(field incremental),$(field changed)" 'true,true,["app/Main$.class"]'
same_as_fresh "parse failure cleared" "$src" $PIN
expect "held-back build runs" "$(run_out app.Main)" "<3,2,1>"

# An unsaved text stands in for the file; withdrawn, the file on disk counts again.
sed 's/"<"/"{"/; s/">"/"}"/' "$src/util.scala" > "$work/util-braces.scala"
text "$src/util.scala" "$work/util-braces.scala"
build "$src/util.scala"
expect "text overlay" "$(field ok),$(field incremental),$(field changed)" 'true,true,["util/Fmt$.class"]'
expect "text overlay runs" "$(run_out app.Main)" "{3,2,1}"
withdraw "$src/util.scala"
build "$src/util.scala"
expect "text withdrawn" "$(field ok),$(field incremental),$(field changed)" 'true,true,["util/Fmt$.class"]'
same_as_fresh "text withdrawn" "$src" $PIN

# A second entry point: every object with a `main` gets its static forwarder, no launcher.
printf '\nobject Other:\n  def main(args: Array[String]): Unit = println("other")\n' >> "$src/util.scala"
build "$src/util.scala"
expect "object with main added" "$(field ok),$(field incremental)" 'true,false'
if has '"name":"util.Other","kind":"object","top":true,"public":true,"abstract":false,"final":true,"bases":[],"annotations":[],"methodAnnotations":[],"main":true,"files":["util/Other$.class","util/Other.class"]'; then ok; else bad "object with main added: its analysis: $RESULT"; fi
if [ ! -e "$work/out/TeqMain.class" ]; then ok; else bad "object with main added: a launcher was written"; fi
same_as_fresh "object with main added" "$src" $PIN
expect "the second entry point runs" "$(run_out util.Other)" "other"
stop

# --- First builds that cannot proceed -----------------------------------------------------------
# A file that fails to parse on a first build: its recovered tree is typed with the rest, the
# parse errors the answer's only diagnostics, under `diagnostics` with their positions (what the
# sbt compiler reports), and nothing is written; the same on a build that changed nothing; the
# fix builds.
rm -rf "$src" "$work/out"
cp -r tests/split/classpath "$src"
printf 'package util\n\nobject Broken:\n  def oops(: Int = 1\n' > "$src/broken.scala"
start "$src" $PIN
expect "first build with a parse error" "$(field ok)" 'false'
if has '"diagnostics":[{"file":"'"$src"'/broken.scala","line":4,"col":12,' && ! has '"file":"'"$src"'/main.scala"' && ! has '"file":"'"$src"'/util.scala"'; then ok; else bad "first build with a parse error: its diagnostics alone: $RESULT"; fi
if [ -z "$(find "$work/out" -name '*.class' 2> /dev/null)" ]; then ok; else bad "first build with a parse error: class files written"; fi
build
if has '"diagnostics":[{"file":"'"$src"'/broken.scala","line":4,"col":12,'; then ok; else bad "parse error kept on a plain build: $RESULT"; fi
printf 'package util\n\nobject Broken:\n  def oops: Int = 1\n' > "$src/broken.scala"
build "$src/broken.scala"
expect "parse error fixed" "$(field ok),$(field diagnostics)" 'true,[]'
same_as_fresh "parse error fixed" "$src" $PIN
stop

# A jar that cannot be opened fails the build and leaves the session running; the build after
# the jar appears succeeds.
extra=$(jar_of sourcecode)
if [ -f "$extra" ]; then
  rm -rf "$src" "$work/out"
  cp -r tests/split/classpath "$src"
  start "$src" --classpath "$SL:$work/extra.jar"
  expect "a missing jar" "$(field ok)" 'false'
  if has 'extra.jar' && has '"severity":"error"'; then ok; else bad "a missing jar: its error: $RESULT"; fi
  build
  expect "a missing jar, built again" "$(field ok)" 'false'
  cp "$extra" "$work/extra.jar"
  build
  expect "the jar appeared" "$(field ok),$(field incremental)" 'true,false'
  same_as_fresh "the jar appeared" "$src" --classpath "$SL:$work/extra.jar"
  stop
fi

# --- Syntax errors and their fixes --------------------------------------------------------------
# Each shape of a syntax error in a JVM session: the
# broken text is typed from its recovered tree and writes nothing; after the fix every class
# file, the unrelated file's included, is a fresh build's. Then both files broken and fixed in
# either order, with and without a full path at every second build.
shapes=(
  'util.scala|s/def show(xs: List\[Int\]): String = xs/def show(xs: List[Int]): String = (xs/|a body with an unclosed parenthesis'
  'util.scala|s/def show(xs: List\[Int\]): String =/def show(xs: List[Int]): String/|a def missing its ='
  'util.scala|s/def show(xs: List\[Int\]): String/def show(xs: List[Int]: String/|a broken parameter list'
  'main.scala|s/^object Main {/object Main/|a template brace missing'
  'main.scala|s/List(3, 1, 2)/List(3, 1, 2/|an argument list left open'
)
for shape in "${shapes[@]}"; do
  IFS='|' read -r file edit what <<< "$shape"
  rm -rf "$src" "$work/out" && cp -r tests/split/classpath "$src"
  start "$src" $PIN
  rm -rf "$work/before-syntax" && cp -r "$work/out" "$work/before-syntax"
  sed "$edit" "tests/split/classpath/$file" > "$src/$file"
  if cmp -s "$src/$file" "tests/split/classpath/$file"; then bad "syntax, $what: the edit changed nothing"; stop; continue; fi
  build "$src/$file"
  expect "syntax, $what: broken" "$(field ok)" false
  if diff -r "$work/out" "$work/before-syntax" > /dev/null; then ok; else bad "syntax, $what: the output changed"; fi
  cp "tests/split/classpath/$file" "$src/$file"
  build "$src/$file"
  expect "syntax, $what: fixed" "$(field ok)" true
  same_as_fresh "syntax, $what: fixed" "$src" $PIN
  stop
done
broken_text() {
  case $1 in
    util.scala) sed 's/def show(xs: List\[Int\]): String = xs/def show(xs: List[Int]): String = (xs/' tests/split/classpath/util.scala ;;
    main.scala) sed 's/List(3, 1, 2)/List(3, 1, 2/' tests/split/classpath/main.scala ;;
  esac
}
for compact in 0 2; do
  for order in "util.scala main.scala" "main.scala util.scala"; do
    rm -rf "$src" "$work/out" && cp -r tests/split/classpath "$src"
    TEQ_COMPACT_EVERY=$compact start "$src" $PIN
    for file in $order; do
      broken_text $file > "$src/$file"
      build "$src/$file"
      expect "syntax, $order in turn (compaction $compact): $file broken" "$(field ok)" false
    done
    for file in $order; do
      cp "tests/split/classpath/$file" "$src/$file"
      build "$src/$file"
    done
    expect "syntax, $order in turn (compaction $compact): fixed" "$(field ok)" true
    same_as_fresh "syntax, $order in turn (compaction $compact): fixed" "$src" $PIN
    expect "syntax, $order in turn (compaction $compact): runs" "$(run_out app.Main)" "[1,2,3]"
    stop
  done
done

# --- Inline bodies ------------------------------------------------------------------------------
# An edit of an inline body types the files that expanded it again, on their unchanged ASTs.
rm -rf "$work/out" "$src"
cp -r tests/split/inline "$src"
start "$src" $PIN
expect "inline: first build" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "inline: first build" "$src" $PIN
expect "inline: first build runs" "$(run_out app.Main | tr '\n' ' ')" "42 1 "
sed -i.bak 's/inline def factor: Int = 2/inline def factor: Int = 3/' "$src/lib.scala"
build "$src/lib.scala"
expect "inline: body edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["use/Sizes$.class"]'
if has '"retyped":["'"$src"'/use.scala","'"$src"'/lib.scala"]'; then ok; else bad "inline: the expanding file retyped with it: $(field retyped)"; fi
same_as_fresh "inline: body edit" "$src" $PIN
expect "inline: body edit runs" "$(run_out app.Main | tr '\n' ' ')" "63 1 "
sed -i.bak 's/inline def flag: Boolean = true/inline def flag: Boolean = false/' "$src/lib.scala"
build "$src/lib.scala"
expect "inline: condition edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["use/Sizes$.class"]'
same_as_fresh "inline: condition edit" "$src" $PIN
expect "inline: condition edit runs" "$(run_out app.Main | tr '\n' ' ')" "63 2 "
stop

# --- Owned roots --------------------------------------------------------------------------------
# Under `--own`, the class files of the owned root are those of the whole program's build, and
# the others are not written.
rm -rf "$work/out" "$src"
mkdir -p "$src"
cp -r tests/split/owned/main tests/split/owned/test "$src/"
start "$src/main" "$src/test" $PIN --own "$src/test"
expect "owned: first build" "$(field ok),$(field incremental)" 'true,false'
# The anonymous class `Actions.make()` expands to is the caller's: written with the test root.
expect "owned: the test root's classes alone, the expansion-made class among them" "$(cd "$work/out" && find . -name '*.class' | LC_ALL=C sort | tr '\n' ' ')" "./app/Actions\$\$anon\$1sa7j06_378\$lsb184_91.class ./app/Checks\$.class ./app/Checks.class ./app/Discounted.class ./scala/runtime/jvm\$package\$.class "
if has '"file":"'"$src"'/test/app/Checks.scala"' && ! has '"file":"'"$src"'/main/app/Model.scala"'; then ok; else bad "owned: the analysis of the owned files alone: $RESULT"; fi
rm -rf "$work/fresh"
"$TEQ" compiler build "$src/main" "$src/test" $PIN $ONE --target jvm -o "$work/fresh" --all-mains > "$work/fresh.log" 2>&1 || bad "owned: the fresh build failed"
for class in $(cd "$work/out" && find . -name '*.class'); do
  if cmp -s "$work/out/$class" "$work/fresh/$class"; then ok; else bad "owned: $class differs from the whole program's build"; fi
done
sed -i.bak 's/"discounted /"cheaper /' "$src/test/app/Checks.scala"
build "$src/test/app/Checks.scala"
expect "owned: body edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["app/Discounted.class"]'
sed -i.bak 's/List(Item("pen", 3), Item("ink", 5))/List(Item("pen", 3), Item("ink", 5), Item("pad", 4))/' "$src/main/app/Model.scala"
build "$src/main/app/Model.scala"
expect "owned: an edit of the other root writes nothing" "$(field ok),$(field incremental),$(field changed)" 'true,true,[]'
# The file that expanded the other root's inline def is typed again (its answer lists it, with
# nothing changed); the other root's own file is not the owned build's to report.
if has '"file":"'"$src"'/test/app/Checks.scala"' && ! has '"file":"'"$src"'/main/app/Model.scala"'; then ok; else bad "owned: the analysis after the other root's edit: $RESULT"; fi
stop

# --- Owned roots' extensions ------------------------------------------------------------------
# An extension method of an owned root that nothing of the program calls is written all the same:
# another project built over these class files (sbt's residents, one per project) calls it, as it
# calls a `private[shared]` member from its own file of the package. And a
# trait nested in the upstream's trait keeps teq's name for its outer accessor in both projects'
# class files: the downstream's inline expansion calls it on the upstream's object, the upstream's
# default method on the downstream's.
rm -rf "$work/ext-up" "$work/ext-dn"
"$TEQ" compiler build tests/support/ownedext/up --own tests/support/ownedext/up $PIN $ONE --target jvm -o "$work/ext-up" --all-mains > "$work/ext-up.log" 2>&1 || bad "owned extensions: the upstream build failed"
"$TEQ" compiler build tests/support/ownedext/up tests/support/ownedext/dn --own tests/support/ownedext/dn --classpath "$work/ext-up:$SL" $ONE --target jvm -o "$work/ext-dn" --all-mains > "$work/ext-dn.log" 2>&1 || bad "owned extensions: the downstream build failed"
expect "owned extensions: a downstream's call of an unreached extension runs" "$(timeout 30 java -Xss512m -XX:+UseSerialGC -XX:TieredStopAtLevel=1 -XX:-UsePerfData -cp "$work/ext-up:$work/ext-dn:$SL" main.Launcher 2>&1 | tr '\n' ' ')" "offset true true 5 40 8 70 41 "

# --- Owned roots' mirrors ---------------------------------------------------------------------
# The mirror of an upstream class that only the downstream summons is a val in the class's file,
# which the upstream's build writes summoned or not, with the accessor the downstream calls
# whatever type its summon names the class by.
rm -rf "$work/mir-up" "$work/mir-dn"
"$TEQ" compiler build tests/support/ownedmirror/up --own tests/support/ownedmirror/up $PIN $ONE --target jvm -o "$work/mir-up" --all-mains > "$work/mir-up.log" 2>&1 || bad "owned mirrors: the upstream build failed"
"$TEQ" compiler build tests/support/ownedmirror/up tests/support/ownedmirror/dn --own tests/support/ownedmirror/dn --classpath "$work/mir-up:$SL" $ONE --target jvm -o "$work/mir-dn" --all-mains > "$work/mir-dn.log" 2>&1 || bad "owned mirrors: the downstream build failed"
expect "owned mirrors: a downstream's summons of the upstream's mirrors run" "$(timeout 30 java -Xss512m -XX:+UseSerialGC -XX:TieredStopAtLevel=1 -XX:-UsePerfData -cp "$work/mir-up:$work/mir-dn:$SL" dmb.run 2>&1 | tr '\n' ' ')" "Point(1,2) Box(3) 1 1 Rgb(1,2,3) Red Origin Square(2.0) Circle Color Origin "

# --- Every JVM program --------------------------------------------------------------------------
program_case() {
  local name=$1
  local case=tests/cases/$name
  [ -d "$case" ] || case=$case.scala
  [ -e "$case" ] || return
  rm -rf "$src" "$work/out"
  mkdir -p "$src"
  if [ -d "$case" ]; then cp -r "$case"/. "$src"; else cp "$case" "$src/"; fi
  local flags
  flags=$(grep -h -o '^// teq: .*' "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    ok
    return
  fi
  flags="$flags --classpath $SL${JARS_CP:+:$JARS_CP}"
  local file
  file=$(ls "$src"/*.scala | head -1)
  start "$src" $flags
  if [ "$(field ok)" != true ]; then
    bad "$name: first build: $(echo "$RESULT" | head -c 300)"
    stop
    return
  fi
  sed -i.bak '1a\
// watch
' "$file"
  build "$file"
  if has "infers the type arguments" || has "a macro ran its definitions"; then ok; else
    expect "$name: comment inserted" "$(field ok),$(field incremental)" 'true,true'
  fi
  same_as_fresh "$name: comment inserted" "$src" $flags
  perl -0pi -e 's/"([^"\\\n]*)"/"$1~"/' "$file"
  build "$file"
  if [ "$(field ok)" = true ]; then
    same_as_fresh "$name: string edited" "$src" $flags
  elif "$TEQ" compiler build "$src" $flags $ONE --target jvm -o "$work/fresh" --all-mains > /dev/null 2>&1; then
    bad "$name: string edited: watch reports an error, the fresh build does not"
  else
    ok
  fi
  stop
}
# Programs that build but do not print scalac's output: a session over them is held to a fresh
# build all the same.
WATCH_ONLY="extension_then_conversion indexed_seq_defaults inline_definition_gaps map_key_contract scala3_typeclass-derivation3 string_f_targ"
for name in $(cat tests/jvm-passing.txt) $WATCH_ONLY; do
  program_case "$name"
done

# --- One file after another ---------------------------------------------------------------------
retype_entry $PIN
retype_reads $PIN
retype_facade $PIN
retype_roots $PIN
retype_state $PIN
retype_state_pair $PIN
retype_cacheable $PIN
retype_jar_state $PIN
retype_quotes $PIN

sessions_forked jvm-watch
echo "$pass passed, $fail failed"
[ $fail = 0 ]
