#!/bin/bash
# Watch mode (`teq compiler watch`): a session is driven over stdin and every build it answers with is
# compared with a fresh `teq compiler build --split` of the same sources, which has to be byte-identical.
# First the scripted scenarios on a copy of tests/split/cycle: a body edit (typed incrementally,
# one module rewritten), a body edit through a plain `build` that checks modification times, two
# files in one command, a definition added and an inferred type changed (full builds), a
# signature change (full build), a compile error and a parse error (reported, the output left as
# it was) and their fixes, a file added and removed; the same kinds of edit in a session
# whose every second build takes the full path for the session's memory (`TEQ_COMPACT_EVERY`,
# src/watch.rs), which has to answer and write what the others do; and a session over
# tests/split/cycles, whose macros leave cycles of reference counts behind, which the build that
# takes the full path frees (`stats`: the blocks in use back at the first build's, and the
# interpreter's registry emptied), the modules a fresh build's throughout; a session over
# tests/split/retype_cacheable whose object with cacheable state, a cycle the registry holds, the
# retypes keep whole and that build makes anew; and a session over its limit, whose next request
# takes the full path though it changes nothing, or brings only a text that fails to parse; and one
# under `TEQ_COMPACT_EVERY=1`, whose every request takes it; a session over tests/split/capture
# that adds and removes a definition a local's scope reads, and another package's of its name; a
# session over an upstream module's products that another build rewrites between two requests,
# once keeping every file's size and time, once with a request for a file whose time was kept,
# once killed on the way (the session reads the last whole publication); and a
# session over tests/split/given_memo that adds a local given to a body and removes it, and changes
# a file's import of givens and changes it back, through the given search's kept memos.
# Every session types its full builds at the automatic count and every fresh build it is compared
# with by one worker (tests/support/sessions.sh); the last line says which sessions forked.
# Then the representative shapes of
# a syntax error broken and fixed, in two files in either order and under compaction: nothing
# written while one stands, a fresh build's modules after the fix.
# Then every program of tests/cases and
# tests/interop gets a comment inserted (which moves every position in the file) and its first
# string literal changed, each followed by an incremental build and the comparison; a program's
# `// jars:` line puts those jars on its class path (tests/support/jars.sh), and without one of
# them in the coursier cache it counts as passed. Last the sessions of tests/support/retype.sh,
# in which one file is typed again after another, and every file of the realistic frontend in
# turn.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
STUB=tests/interop/scalajs-stub
. tests/support/jars.sh
. tests/support/sessions.sh
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

work=$(mktemp -d)
trap 'stop; rm -rf "$work"' EXIT
sessions_log "$work/workers.log"
WPID=
# Starts a session on the given teq arguments; commands go to fd 3, answers come from fd 4.
start() {
  rm -f "$work/cmd" "$work/ans"
  mkfifo "$work/cmd" "$work/ans"
  "$TEQ" compiler watch "$@" < "$work/cmd" > "$work/ans" 2> "$work/err" &
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
# Reads one JSON answer (bounded: the session is killed after 20 s of silence).
answer() {
  RESULT=
  if ! read -r -t 20 RESULT <&4; then
    bad "no answer from teq compiler watch (stderr: $(head -c 300 "$work/err"))"
    kill "$WPID" 2> /dev/null
    RESULT='{"ok":false,"timeout":true}'
  fi
}
# `build` for the given paths, or for every file when none are given.
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
field() { echo "$RESULT" | grep -o "\"$1\":\(\[[^]]*\]\|[^,}]*\)" | head -1 | sed "s/\"$1\"://"; }
has() { case "$RESULT" in *"$1"*) return 0 ;; *) return 1 ;; esac; }
# Compares the session's output directory with a fresh build of the same sources. Under --hot
# a footer carries the id of the build that wrote the module, which the listing names too, and
# a fresh build writes every module where the session rewrote the edited ones alone: the
# writers are left out of the comparison, the hashes and the build's id are not.
without_ids() { sed -E -e 's/(\$hot(Ran|Booted)\(import\.meta\.url, "[0-9a-f]{16}", )"[0-9a-f]{16}"/\1"ID"/' -e 's/(\["[0-9a-f]{16}", )"[0-9a-f]{16}"\]/\1"ID"]/' "$1"; }
same_as_fresh() {
  local what=$1
  shift
  rm -rf "$work/fresh"
  if ! "$TEQ" compiler build "$@" $ONE --split "$work/fresh" > "$work/fresh.log" 2>&1; then
    bad "$what: the fresh build failed: $(head -3 "$work/fresh.log")"
    return
  fi
  local same=1
  if diff -r "$work/out" "$work/fresh" > "$work/diff" 2>&1; then :; elif [[ " $* " == *" --hot "* ]]; then
    same=1
    for f in $(cd "$work/fresh" && ls); do
      [ -f "$work/out/$f" ] || { same=0; break; }
      diff <(without_ids "$work/out/$f") <(without_ids "$work/fresh/$f") > /dev/null || { same=0; break; }
    done
    [ "$(ls "$work/out" | wc -l)" = "$(ls "$work/fresh" | wc -l)" ] || same=0
  else
    same=0
  fi
  if [ $same = 1 ]; then ok; else
    bad "$what: watch output differs from a fresh build"
    head -${DIFF_LINES:-10} "$work/diff"
  fi
}

retype_start() { start "$@" --split "$work/out"; }
after_failed_full=false
as_fresh() { same_as_fresh "$@"; }
retype_runs() { expect "$1" "$(timeout 20 node "$work/out/main.mjs")" "$2"; }
# A session numbers the classes a quote makes on where a fresh build starts at one, so after
# the sweep the two outputs are compared by what they print; by their bytes once such a class
# is named by the site of its expansion.
sweep_end() {
  local what=$1
  shift
  rm -rf "$work/fresh"
  if ! "$TEQ" compiler build "$@" $ONE --split "$work/fresh" > "$work/fresh.log" 2>&1; then
    bad "$what: the fresh build failed: $(head -3 "$work/fresh.log")"
    return
  fi
  local run
  for run in out fresh; do
    if ! timeout 60 node "$work/$run/main.mjs" > "$work/$run.printed" 2>&1; then
      bad "$what: the output in $run/ did not run to its end: $(tail -2 "$work/$run.printed" | head -c 300)"
      return
    fi
  done
  if cmp -s "$work/out.printed" "$work/fresh.printed"; then ok; else
    bad "$what: the session's output prints something else than a fresh build's"
    diff "$work/out.printed" "$work/fresh.printed" | head -${DIFF_LINES:-10}
  fi
}
retype_queries() { :; }
. tests/support/retype.sh

# --- The scripted scenarios ---------------------------------------------------------------------
# A given typed `b.type` in `val a, b = new T` makes the signature phase's tables type `b`'s copy
# of the initialiser before the walk reaches `a`; a retype walks the file in order. The two
# agree on the classes' names only if the copies are typed in their order whichever is asked
# for first.
src=$work/anon
cp -r tests/split/anon_order "$src"
# Without outlining, so that the two copies' classes are not shared into one and their names stand
# in the modules.
start "$src" --split "$work/out" --no-outline
expect "anon_order: first build" "$(field ok)" 'true'
same_as_fresh "anon_order: first build" "$src" --no-outline
sed -i.bak 's/def value: Int = 1/def value: Int = 2/' "$src/Main.scala"
build "$src/Main.scala"
expect "anon_order: retype" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "anon_order: retype after an edit of the initialiser's body" "$src" --no-outline
stop
rm -rf "$work/out"

# An anonymous JS object edited across rebuilds: leaving an optional member out,
# giving it with an override val, and leaving it out again.
src=$work/anon_undefined
cp -r tests/split/anon_js_undefined "$src"
start "$src" --split "$work/out"
expect "anon_js_undefined: first build" "$(field ok)" 'true'
same_as_fresh "anon_js_undefined: first build" "$src"
expect "anon_js_undefined: first build runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "req {\"req\":\"hello\"} "
sed -i.bak 's|val req = "hello"|override val opt = js.undefined; val req = "hello"|' "$src/main.scala"
build "$src/main.scala"
expect "anon_js_undefined: member added" "$(field ok),$(field incremental),$(field retyped)" "true,true,[\"$src/main.scala\"]"
same_as_fresh "anon_js_undefined: member added" "$src"
expect "anon_js_undefined: member added runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "opt,req {\"req\":\"hello\"} "
cp tests/split/anon_js_undefined/main.scala "$src/main.scala"
build "$src/main.scala"
expect "anon_js_undefined: member removed" "$(field ok),$(field incremental),$(field retyped)" "true,true,[\"$src/main.scala\"]"
same_as_fresh "anon_js_undefined: member removed" "$src"
expect "anon_js_undefined: member removed runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "req {\"req\":\"hello\"} "
stop
rm -rf "$work/out"

# A session keeps its reach across retypes whose edited files reach what they reached before
# (src/emit/kept.rs): a literal changed, a comment inserted, a string of an anonymous class's
# member, a literal val's value, a broken edit's fix and the edit after it; the reach is walked
# from the roots again where they do not: a call added, the last `new Circle` removed, a literal
# val made to run statements. Each answer says which (`TEQ_SESSION_PARTS=1`), the session settles
# its reach before it reads a request (`TEQ_KEPT_SETTLE=wait`), and each build is a fresh one's.
src=$work/kept_reach
cp -r tests/split/kept_reach "$src"
TEQ_SESSION_PARTS=1 TEQ_KEPT_SETTLE=wait start "$src" --split "$work/out"
expect "kept_reach: first build" "$(field ok)" 'true'
same_as_fresh "kept_reach: first build" "$src"
kept_step() {
  build "$src/$1"
  expect "kept_reach: $2" "$(field ok),$(field incremental),$(field kept)" "true,true,$3"
  same_as_fresh "kept_reach: $2" "$src"
}
sed -i.bak 's/println("label")/println("label2")/' "$src/Main.scala"
kept_step Main.scala "a literal changed" true
printf '// a comment\n' | cat - "$src/Main.scala" > "$src/Main.tmp" && mv "$src/Main.tmp" "$src/Main.scala"
kept_step Main.scala "a comment inserted" true
sed -i.bak 's/"blob"/"blob2"/' "$src/Main.scala"
kept_step Main.scala "an anonymous class's string" true
sed -i.bak 's/val limit: Int = 3/val limit: Int = 4/' "$src/Other.scala"
kept_step Other.scala "a literal val's value" true
# A retype that fails keeps the reach with the walks before it: the fix compares with those.
sed -i.bak 's/println("label2")/println(label2)/' "$src/Main.scala"
build "$src/Main.scala"
expect "kept_reach: a broken edit" "$(field ok)" 'false'
sed -i.bak 's/println(label2)/println("label2")/' "$src/Main.scala"
kept_step Main.scala "the broken edit fixed" true
sed -i.bak 's/println("label2")/println("label3")/' "$src/Main.scala"
kept_step Main.scala "a literal changed after the fix" true
sed -i.bak 's/println("label3")/println("label3"); println(describe(limit))/' "$src/Main.scala"
kept_step Main.scala "a call added" false
expect "kept_reach: a call added runs" "$(timeout 20 node "$work/out/main.mjs" | tail -1)" "x4"
sed -i.bak 's/new Circle(1), //' "$src/Main.scala"
kept_step Main.scala "the last new Circle removed" false
sed -i.bak 's/val limit: Int = 4/val limit: Int = { println("init"); 4 }/' "$src/Other.scala"
kept_step Other.scala "a literal val made to run a statement" false
expect "kept_reach: the val's statement runs" "$(timeout 20 node "$work/out/main.mjs" | tail -2 | tr '\n' ' ')" "init x4 "
# Local definitions taken out by an edit that fails, put back, and taken out by one that types:
# the walks before a retype read the old trees, and the reach the dead local symbols.
locals='def twice(x: Int): Int = x \* 2; class Box(val n: Int) { def get: Int = twice(n) }; println(new Box(2).get)'
sed -i.bak "s/println(describe(limit))/println(describe(limit)); $locals/" "$src/Main.scala"
kept_step Main.scala "local definitions added" false
sed -i.bak "s/$locals/val broken: Int = \"no\"/" "$src/Main.scala"
build "$src/Main.scala"
expect "kept_reach: local definitions taken out by an edit that fails" "$(field ok)" 'false'
sed -i.bak "s/val broken: Int = \"no\"/$locals/" "$src/Main.scala"
kept_step Main.scala "local definitions put back" true
sed -i.bak "s/; $locals//" "$src/Main.scala"
kept_step Main.scala "local definitions taken out" false
printf '// another comment\n' | cat - "$src/Main.scala" > "$src/Main.tmp" && mv "$src/Main.tmp" "$src/Main.scala"
kept_step Main.scala "a comment inserted after they were" true
stop
rm -rf "$work/out"

# The two sides of what a retype adds outside its units (`Kept::added_outside`): a macro whose run
# takes a path the first build's did not and types std bodies (`toList`, `count`) that no walk asks
# for keeps the reach, which a walk from the roots would not meet either; tests/jvm-watch.sh has
# the other side, inline_fold_std's fold adding template calls the next walk calls.
src=$work/kept_reach_macro
cp -r tests/split/kept_reach_macro "$src"
TEQ_SESSION_PARTS=1 TEQ_KEPT_SETTLE=wait start "$src" --split "$work/out"
expect "kept_reach_macro: first build" "$(field ok)" 'true'
sed -i.bak 's/letters("abc1")/letters("ab!c1")/' "$src/Main.scala"
build "$src/Main.scala"
expect "kept_reach_macro: the macro's argument changed" "$(field ok),$(field incremental),$(field kept)" "true,true,true"
same_as_fresh "kept_reach_macro: the macro's argument changed" "$src"
stop
rm -rf "$work/out"

# A macro whose run in a retype reaches the override, in a std class the session enters only then,
# of a method the output names apart (`Collection.remove`, `remove$Any` beside `List.remove(Int)`):
# the interpreter finds the override by its erased signature, whatever output name the session's
# typer holds for it, as a fresh build's run does.
src=$work/macro_std_override
cp -r tests/split/macro_std_override "$src"
TEQ_SESSION_PARTS=1 TEQ_KEPT_SETTLE=wait start "$src" --split "$work/out"
expect "macro_std_override: first build" "$(field ok)" 'true'
sed -i.bak 's/result(false)/result(true)/' "$src/Main.scala"
build "$src/Main.scala"
expect "macro_std_override: the macro reaches the override" "$(field ok),$(field incremental)" "true,true"
retype_runs "macro_std_override: the retype runs" "true"
same_as_fresh "macro_std_override: the macro reaches the override" "$src"
stop
rm -rf "$work/out"

# A local named like a definition its scope reads is named apart from it (src/emit/scope.rs):
# a definition added that a contextual parameter's scope reads, and removed again; another
# package's definition of the name `use` reads through `lib.x`, which makes the read qualified
# and the local's name free, added and removed again. Each build is a fresh one's.
src=$work/capture
cp -r tests/split/capture "$src"
start "$src" --split "$work/out"
expect "capture: first build" "$(field ok)" 'true'
same_as_fresh "capture: first build" "$src"
expect "capture: first build runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "7 "
cat > "$src/use.scala" <<'EOF'
package use

def `contextual$1`(): Int = 8

// The local `x` reads `lib.x`, which its module writes `x` while no other package defines one.
@main def main(): Unit =
  val x = lib.x()
  println(x)
  val g: Int ?=> Int = summon[Int] + `contextual$1`()
  println(g(using 1))
EOF
build "$src/use.scala"
expect "capture: a colliding definition added" "$(field ok)" 'true'
same_as_fresh "capture: a colliding definition added" "$src"
expect "capture: a colliding definition added runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "7 9 "
cp tests/split/capture/use.scala "$src/use.scala"
build "$src/use.scala"
expect "capture: the colliding definition removed" "$(field ok)" 'true'
same_as_fresh "capture: the colliding definition removed" "$src"
printf 'package other\n\ndef x(): Int = 9\n' > "$src/other.scala"
build "$src/other.scala"
expect "capture: another package's x added" "$(field ok)" 'true'
same_as_fresh "capture: another package's x added" "$src"
expect "capture: the local keeps its name where the read is qualified" "$(grep -c 'const x = ' "$work/out/use.mjs")" '1'
rm "$src/other.scala"
build "$src/other.scala"
expect "capture: another package's x removed" "$(field ok)" 'true'
same_as_fresh "capture: another package's x removed" "$src"
expect "capture: another package's x removed runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "7 "
stop
rm -rf "$work/out"

# An upstream module's products rebuilt between two requests of a session over them (the vite
# dev server over a description whose upstream sbt compiles): the next request takes the full
# path, read through the directory's new identity, and answers as a fresh session does, also
# when the rebuild keeps every file's size and time (docs/TARGETS.md, "Watch mode").
src=$work/products
mkdir -p "$src/up" "$src/down"
printf 'package up\nobject Lib:\n  def greet: String = "one"\n' > "$src/up/Lib.scala"
printf 'package down\n@main def run(): Unit = println(up.Lib.greet)\n' > "$src/down/Main.scala"
"$TEQ" compiler check --products "$src/upP" "$src/up/Lib.scala" > /dev/null 2>&1 || bad "products: the upstream's products"
retype_start "$src/down" --classpath "$src/upP"
expect "products: first build" "$(field ok)" 'true'
same_as_fresh "products: first build" "$src/down" --classpath "$src/upP"
build
expect "products: an unchanged request is incremental" "$(field incremental)" 'true'
printf 'package up\nobject Lib:\n  def greet: String = "two"\n' > "$src/up/Lib.scala"
"$TEQ" compiler check --products "$src/upP" "$src/up/Lib.scala" > /dev/null 2>&1 || bad "products: the upstream rebuilt"
build
expect "products: the upstream rebuilt takes the full path" "$(field incremental)" 'false'
if has "the products of $src/upP changed"; then ok; else bad "products: the upstream rebuilt: fallback reason: $(field fallback)"; fi
same_as_fresh "products: the upstream rebuilt" "$src/down" --classpath "$src/upP"
expect "products: the upstream rebuilt runs" "$(timeout 20 node "$work/out/main.mjs")" "two"
cp -p "$src/upP/up/Lib.tasty" "$src/Lib.tasty.times"
cp -p "$src/upP/teq-products.json" "$src/manifest.times"
printf 'package up\nobject Lib:\n  def greet: String = "six"\n' > "$src/up/Lib.scala"
"$TEQ" compiler check --products "$src/upP" "$src/up/Lib.scala" > /dev/null 2>&1 || bad "products: the upstream rewritten"
touch -r "$src/Lib.tasty.times" "$src/upP/up/Lib.tasty"
touch -r "$src/manifest.times" "$src/upP/teq-products.json"
expect "products: the rewrite keeps the pickle's size" "$(wc -c < "$src/upP/up/Lib.tasty")" "$(wc -c < "$src/Lib.tasty.times")"
build
expect "products: a rewrite keeping sizes and times takes the full path" "$(field incremental)" 'false'
same_as_fresh "products: a rewrite keeping sizes and times" "$src/down" --classpath "$src/upP"
expect "products: a rewrite keeping sizes and times runs" "$(timeout 20 node "$work/out/main.mjs")" "six"
printf 'package up\nobject Lib:\n  def greet: String = "ten"\n' > "$src/up/Lib.scala"
"$TEQ" compiler check --products "$src/upP" "$src/up/Lib.scala" > /dev/null 2>&1 || bad "products: the upstream rebuilt again"
cp -p "$src/down/Main.scala" "$src/Main.times"
printf 'package down\n@main def run(): Unit = println(up.Lib.greet + "!")\n' > "$src/down/Main.scala"
touch -r "$src/Main.times" "$src/down/Main.scala"
build "$src/down/Main.scala"
expect "products: a request with the upstream rebuilt takes the full path" "$(field incremental)" 'false'
same_as_fresh "products: a request with the upstream rebuilt" "$src/down" --classpath "$src/upP"
expect "products: a request with the upstream rebuilt reads the requested file" "$(timeout 20 node "$work/out/main.mjs")" "ten!"
printf 'package up\nobject Lib:\n  def greet: String = "cut"\n' > "$src/up/Lib.scala"
TEQ_CUT_PUBLICATION=1 "$TEQ" compiler check --products "$src/upP" "$src/up/Lib.scala" > /dev/null 2>&1 && bad "products: the cut publication exited 0"
build
expect "products: a publication killed on the way takes the full path" "$(field incremental)" 'false'
same_as_fresh "products: a publication killed on the way" "$src/down" --classpath "$src/upP"
expect "products: a publication killed on the way reads the last whole one" "$(timeout 20 node "$work/out/main.mjs")" "ten!"
stop
rm -rf "$work/out"

# The given search's memos across retypes (src/typer/implicits.rs): a local given added to a body
# and removed again, the file's import of givens changed and changed back, with the candidates of
# its package level, the fits and the head forms kept from the builds before. Each build is a
# fresh one's and prints what scalac's does.
src=$work/given_memo
cp -r tests/split/given_memo "$src"
start "$src" --split "$work/out"
expect "given_memo: first build" "$(field ok)" 'true'
same_as_fresh "given_memo: first build" "$src"
expect "given_memo: first build runs" "$(timeout 20 node "$work/out/main.mjs")" "int string low int"
perl -pi -e 's/^    summon\[TC\[Int\]\]\.name$/    given TC[Int] = Named("local")\n    summon[TC[Int]].name/' "$src/use.scala"
build "$src/use.scala"
expect "given_memo: a local given added" "$(field ok)" 'true'
same_as_fresh "given_memo: a local given added" "$src"
expect "given_memo: a local given added runs" "$(timeout 20 node "$work/out/main.mjs")" "int string low local"
cp tests/split/given_memo/use.scala "$src/use.scala"
build "$src/use.scala"
expect "given_memo: the local given removed" "$(field ok)" 'true'
same_as_fresh "given_memo: the local given removed" "$src"
expect "given_memo: the local given removed runs" "$(timeout 20 node "$work/out/main.mjs")" "int string low int"
perl -pi -e 's/^import Instances\.given$/import Others.given/' "$src/use.scala"
build "$src/use.scala"
expect "given_memo: the import changed" "$(field ok)" 'true'
same_as_fresh "given_memo: the import changed" "$src"
expect "given_memo: the import changed runs" "$(timeout 20 node "$work/out/main.mjs")" "other int other string low other int"
cp tests/split/given_memo/use.scala "$src/use.scala"
build "$src/use.scala"
expect "given_memo: the import changed back" "$(field ok)" 'true'
same_as_fresh "given_memo: the import changed back" "$src"
expect "given_memo: the import changed back runs" "$(timeout 20 node "$work/out/main.mjs")" "int string low int"
stop
rm -rf "$work/out"

# A quote's anonymous class is copied at each run and the copy named by the site of the expansion
# that ran the quote, not by how many runs came before: a retype of the file of the sites makes
# the copies anew under the names a fresh build gives them.
src=$work/copies
cp -r tests/split/quote_copies "$src"
start "$src" --split "$work/out" --no-outline
expect "quote_copies: first build" "$(field ok)" 'true'
same_as_fresh "quote_copies: first build" "$src" --no-outline
sed -i.bak 's/Macros.make("a").show/Macros.make("a").show + "!"/' "$src/use.scala"
build "$src/use.scala"
expect "quote_copies: retype" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "quote_copies: retype of the file of the sites" "$src" --no-outline
# A file added under the root leaves the identities of the others as they were: the copies
# keep their names.
copies=$(grep -oh 'class Macros[A-Za-z0-9_$]*anon[A-Za-z0-9_$]*' "$work/out"/*.mjs | sort | tr '\n' ' ')
mkdir -p "$src/a"
printf 'package copies.use\n\nobject Unrelated:\n  def n: Int = 42\n' > "$src/a/use.scala"
build "$src/a/use.scala"
expect "quote_copies: a file added" "$(field ok)" 'true'
same_as_fresh "quote_copies: a file added under the root" "$src" --no-outline
expect "quote_copies: the copies keep their names when a file is added" "$(grep -oh 'class Macros[A-Za-z0-9_$]*anon[A-Za-z0-9_$]*' "$work/out"/*.mjs | sort | tr '\n' ' ')" "$copies"
stop
rm -rf "$work/out"

# The classes stored inline bodies make (tests/workers/inline_names), through edits before and
# after the sites that make them: the session's modules a fresh build's of this binary. (Stage 2
# compared them with the retype path's as well, which the program's methods no longer take.)
src=$work/names
cp -r tests/workers/inline_names "$src"
start "$src" --split "$work/out"
expect "inline_names: first build" "$(field ok)" 'true'
same_as_fresh "inline_names: first build" "$src"
{ printf '// A line before the sites.\n'; cat "$src/f1.scala"; } > "$work/f1.scala" && mv "$work/f1.scala" "$src/f1.scala"
build "$src/f1.scala"
expect "inline_names: retype after an edit before the sites" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "inline_names: an edit before the sites" "$src"
printf '// A line after the sites.\n' >> "$src/f1.scala"
build "$src/f1.scala"
expect "inline_names: retype after an edit after the sites" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "inline_names: an edit after the sites" "$src"
stop
rm -rf "$work/out"

# An inline method's definition that fails its check (tests/support/history/inline), its call a
# plain call that depends on it: the fix types the unchanged caller again, which expands the
# fixed body, the session's modules a fresh build's; broken again and fixed again the same.
src=$work/inline-fixed
rm -rf "$src" && cp -r tests/support/history/inline/broken-start "$src"
start "$src" --split "$work/out"
expect "inline definition: the failed start" "$(field ok)" 'false'
for text in fixed broken fixed; do
  cp "tests/support/history/inline/$text.scala" "$src/Lib.scala"
  build "$src/Lib.scala"
  if [ $text = fixed ]; then
    expect "inline definition: $text" "$(field ok)" 'true'
    same_as_fresh "inline definition: $text" "$src"
  else
    expect "inline definition: $text" "$(field ok)" 'false'
  fi
done
stop
rm -rf "$work/out"

# A session that takes the full path of its own accord, at every second build here: the
# answer says so, and the modules are a fresh build's after every build, through a type error,
# a parse error and their fixes.
src=$work/compact
cp -r tests/split/cycle "$src"
TEQ_COMPACT_EVERY=2 start "$src" --split "$work/out" --module-per-file a,b --hot
same_as_fresh "own full build: first build" "$src" --module-per-file a,b --hot
sed -i.bak 's/"tagged"/"marked1"/' "$src/b.scala"
build "$src/b.scala"
expect "own full build: the build before" "$(field ok),$(field incremental),$(field changed)" 'true,true,["b.b.mjs","hot-build.mjs"]'
same_as_fresh "own full build: the build before" "$src" --module-per-file a,b --hot
sed -i.bak 's/"marked1"/"marked2"/' "$src/b.scala"
build "$src/b.scala"
expect "own full build: the second build" "$(field ok),$(field incremental),$(field fallback),$(field changed)" 'true,false,"the session'"'"'s memory",["b.b.mjs","hot-build.mjs"]'
same_as_fresh "own full build: the second build" "$src" --module-per-file a,b --hot
sed -i.bak 's/def tag: String = "marked2"/def tag: String = 42/' "$src/b.scala"
build "$src/b.scala"
expect "own full build: a type error" "$(field ok),$(field incremental)" 'false,true'
sed -i.bak 's/"aTop sees "/"aTop saw "/' "$src/a.scala"
build "$src/a.scala"
expect "own full build: the full path with the error standing" "$(field ok),$(field incremental),$(field fallback)" 'false,false,"the session'"'"'s memory"'
if has 'b.scala","line":6'; then ok; else bad "own full build: the error is answered by the full path: $RESULT"; fi
sed -i.bak 's/def tag: String = 42/def tag: String = ("marked3"/' "$src/b.scala"
build "$src/b.scala"
# A JavaScript session keeps no program of a build that failed: the builds after it are full
# until one succeeds.
expect "own full build: a parse error" "$(field ok),$(field incremental),$(field fallback)" 'false,false,"previous build failed"'
sed -i.bak 's/def tag: String = ("marked3"/def tag: String = "marked3"/' "$src/b.scala"
build "$src/b.scala"
expect "own full build: the fix" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"previous build failed"'
same_as_fresh "own full build: the fix" "$src" --module-per-file a,b --hot
expect "own full build: the fix runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 3p)" "w/marked3/B+green"
stop
rm -rf "$work/out"

# The cycles a macro's run leaves: 19 MB of blocks per retype of main.scala that nothing
# frees, until the build that takes the full path sweeps them.
src=$work/cycles
cp -r tests/split/cycles "$src"
TEQ_COMPACT_EVERY=2 start "$src" --split "$work/out"
same_as_fresh "cycles: first build" "$src"
echo stats >&3 && answer
first=$(field live)
registered=$(field registered)
sed -i.bak 's/println("cycles")/println("cycles 1")/' "$src/main.scala"
build "$src/main.scala"
expect "cycles: a retype" "$(field ok),$(field incremental)" 'true,true'
echo stats >&3 && answer
grown=$(field live)
if [ "$grown" -gt $((first + 8 * 1024 * 1024)) ]; then ok; else bad "cycles: the retype's runs left less than expected: $first then $grown bytes in use"; fi
sed -i.bak 's/println("cycles 1")/println("cycles 2")/' "$src/main.scala"
build "$src/main.scala"
expect "cycles: the build that takes the full path" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"the session'"'"'s memory"'
same_as_fresh "cycles: after the full path" "$src"
echo stats >&3 && answer
after=$(field live)
if [ "$after" -lt $((first + 1024 * 1024)) ]; then ok; else bad "cycles: the blocks in use after the full path: $first at first, $after after (grew to $grown)"; fi
# The full build's runs registered what the first build's did; the retype's are gone.
expect "cycles: the registry after the sweep" "$(field registered),$(( $(field freed) > 0 ))" "$registered,1"
stop
rm -rf "$work/out"

# Cacheable state beside the registry's sweep (tests/split/retype_cacheable, `cacheable.Cache`
# declared): the kept object is a cycle of its own, a closure of its field over `this`, which the
# registry holds. A retype after a full build makes it anew (a full build's state is not the
# retypes', at any count), the retype after that keeps it whole, its closure reading the count it
# carries on; the build that takes the full path for the session's memory makes it anew, as every full
# build does, and its sweep frees the old one; the retypes after it make a new one and keep it.
src=$work/cacheable-swept
cp -r tests/split/retype_cacheable "$src"
TEQ_COMPACT_EVERY=3 start "$src" --split "$work/out" --cacheable-state cacheable.Cache
expect "swept: first build" "$(field ok),$(field incremental)" 'true,false'
if has '"uses 1 1"'; then ok; else bad "swept: the first count: $RESULT"; fi
swept_retype() {
  sed -i.bak "s/\"$1\"/\"$2\"/" "$src/use.scala"
  build "$src/use.scala"
  expect "swept: $3" "$(field ok),$(field incremental)" 'true,true'
  if has "\"uses $4 $4\""; then ok; else bad "swept: $3, the count $4: $RESULT"; fi
  expect "swept: $3 runs" "$(timeout 20 node "$work/out/main.mjs")" "$2 $4 true 7 1 first line of the data"
}
swept_retype use used "the retype after the first build makes the object anew" 1
swept_retype used "used up" "the next retype keeps it and its cycle" 2
sed -i.bak 's/"used up"/"use"/' "$src/use.scala"
build "$src/use.scala"
expect "swept: the build that takes the full path" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"the session'"'"'s memory"'
if has '"uses 1 1"'; then ok; else bad "swept: the object made anew by the full path: $RESULT"; fi
same_as_fresh "swept: after the full path" "$src" --cacheable-state cacheable.Cache
echo stats >&3 && answer
expect "swept: the sweep freed the old build's cycles" "$(( $(field freed) > 0 ))" 1
swept_retype use used "a retype after the full path makes a new object" 1
swept_retype used "used up" "the new object kept after the sweep" 2
stop
rm -rf "$work/out"

# A session over its limit takes the full path at its next request, whatever the request
# brings: nothing changed, or only a text that fails to parse.
src=$work/cycles-due
cp -r tests/split/cycles "$src"
start "$src" --split "$work/out"
echo stats >&3 && answer
first=$(field live)
registered=$(field registered)
edits=0
# Body edits until the session's account is over the limit of src/watch.rs (`GROWN`).
grow() {
  local i
  for i in 1 2 3 4 5 6 7 8; do
    echo stats >&3 && answer
    local account base
    account=$(field account)
    base=$(field baseline)
    if [ "$account" -gt $((base + (base / 2 > 16777216 ? base / 2 : 16777216))) ]; then return 0; fi
    edits=$((edits + 1))
    sed -i.bak "s/println(\"cycles[^\"]*\")/println(\"cycles $edits\")/" "$src/main.scala"
    build "$src/main.scala"
    if [ "$(field incremental)" != true ]; then bad "due: the edits that grow the session: $RESULT"; return 1; fi
  done
  bad "due: the account stayed under the limit after eight retypes: $RESULT"
  return 1
}
if grow; then
  build
  expect "due: a request that changes nothing" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"the session'"'"'s memory"'
  same_as_fresh "due: a request that changes nothing" "$src"
  echo stats >&3 && answer
  after=$(field live)
  if [ "$after" -lt $((first + 1024 * 1024)) ]; then ok; else bad "due: the blocks in use after the full path: $first at first, $after after"; fi
  expect "due: the registry after the sweep" "$(field registered),$(( $(field freed) > 0 ))" "$registered,1"
fi
if grow; then
  sed -i.bak 's/println(nested)/println(nested/' "$src/main.scala"
  build "$src/main.scala"
  expect "due: a request with a text that fails to parse" "$(field ok),$(field incremental),$(field fallback)" 'false,false,"the session'"'"'s memory"'
  if has 'main.scala'; then ok; else bad "due: the parse error is answered: $RESULT"; fi
  sed -i.bak 's/println(nested$/println(nested)/' "$src/main.scala"
  build "$src/main.scala"
  expect "due: the fix" "$(field ok),$(field incremental)" 'true,false'
  same_as_fresh "due: the fix" "$src"
fi
stop
rm -rf "$work/out"

# Under `TEQ_COMPACT_EVERY=1` every request takes the full path, one that changes nothing too.
src=$work/every
cp -r tests/split/cycle "$src"
TEQ_COMPACT_EVERY=1 start "$src" --split "$work/out"
for n in 1 2; do
  build
  expect "every request: unchanged request $n" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"the session'"'"'s memory"'
  same_as_fresh "every request: unchanged request $n" "$src"
done
stop
rm -rf "$work/out"

src=$work/src
cp -r tests/split/cycle "$src"
start "$src" --split "$work/out"
expect "first build" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"first build"'
expect "first build writes every module" "$(field modules)" 6
same_as_fresh "first build" "$src"

sed -i.bak 's/"tagged"/"marked"/' "$src/b.scala"
build "$src/b.scala"
expect "body edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["b.mjs"]'
same_as_fresh "body edit" "$src"
expect "body edit runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 3p)" "w/marked/B+green"

build "$src/b.scala"
expect "unchanged file" "$(field ok),$(field incremental),$(field changed)" 'true,true,[]'

sleep 1.1
sed -i.bak 's/"aTop sees "/"aTop saw "/' "$src/a.scala"
build
expect "plain build finds the edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["a.mjs"]'
same_as_fresh "plain build" "$src"

sed -i.bak 's/"marked"/"tagged"/' "$src/b.scala"
sed -i.bak 's/"aTop saw "/"aTop sees "/' "$src/a.scala"
build "$src/a.scala" "$src/b.scala"
expect "two files" "$(field ok),$(field incremental),$(field changed)" 'true,true,["a.mjs","b.mjs"]'
same_as_fresh "two files" "$src"

printf '\nval bInferred = 1\n' >> "$src/b.scala"
build "$src/b.scala"
expect "definition added" "$(field ok),$(field incremental)" 'true,false'
if has "top-level definitions added or removed"; then ok; else bad "definition added: fallback reason: $(field fallback)"; fi
same_as_fresh "definition added" "$src"

sed -i.bak 's/val bInferred = 1/val bInferred = 2/' "$src/b.scala"
build "$src/b.scala"
expect "inferred type kept" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "inferred type kept" "$src"

sed -i.bak 's/val bInferred = 2/val bInferred = "two"/' "$src/b.scala"
build "$src/b.scala"
expect "inferred type changed" "$(field ok),$(field incremental)" 'true,false'
if has "inferred type of bInferred changed"; then ok; else bad "inferred type changed: fallback reason: $(field fallback)"; fi
same_as_fresh "inferred type changed" "$src"

sed -i.bak 's/class Pair(val color: Color, val mode: Mode)/class Pair(val color: Color, val mode: Mode, val extra: Int = 0)/' "$src/b.scala"
build "$src/b.scala"
expect "signature change" "$(field ok),$(field incremental)" 'true,false'
if has "Pair: parameters changed"; then ok; else bad "signature change: fallback reason: $(field fallback)"; fi
same_as_fresh "signature change" "$src"

printf '\ntrait Sized:\n  type Size\n  def size: Size\nobject Sizes extends Sized:\n  type Size = Int\n  def size: Size = 1\n' >> "$src/b.scala"
build "$src/b.scala"
expect "type member added" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "type member added" "$src"
sed -i.bak 's/def size: Size = 1/def size: Size = 2/' "$src/b.scala"
build "$src/b.scala"
expect "type member body edit" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "type member body edit" "$src"
sed -i.bak 's/type Size = Int/type Size = Long/; s/def size: Size = 2/def size: Size = 2L/' "$src/b.scala"
build "$src/b.scala"
expect "type member changed" "$(field ok),$(field incremental)" 'true,false'
if has "type changed"; then ok; else bad "type member changed: fallback reason: $(field fallback)"; fi
same_as_fresh "type member changed" "$src"

cp -r "$work/out" "$work/before-error"
sed -i.bak 's/def tag: String = "tagged"/def tag: String = 42/' "$src/b.scala"
build "$src/b.scala"
expect "type error" "$(field ok),$(field incremental)" 'false,true'
if has '"file":"'"$src"'/b.scala","line":6,"col":21'; then ok; else bad "type error position: $RESULT"; fi
if has '"source":"  def tag: String = 42"' && has '"caret":"                    ^^"'; then ok; else bad "type error source and caret: $RESULT"; fi
if diff -r "$work/out" "$work/before-error" > /dev/null; then ok; else bad "type error: the output changed"; fi

sed -i.bak 's/def tag: String = 42/def tag: String = "tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "fix after type error" "$(field ok),$(field incremental),$(field changed)" 'true,true,[]'
same_as_fresh "fix after type error" "$src"

sed -i.bak 's/def tag: String = "tagged"/def tag: String = ("tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "parse error" "$(field ok)" 'false'
if has '"file":"'"$src"'/b.scala"'; then ok; else bad "parse error position: $RESULT"; fi
if diff -r "$work/out" "$work/before-error" > /dev/null; then ok; else bad "parse error: the output changed"; fi

sed -i.bak 's/def tag: String = ("tagged"/def tag: String = "tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "fix after parse error" "$(field ok),$(field incremental),$(field changed)" 'true,true,[]'

cat > "$src/c.scala" <<'EOF'
package c
val cTop: String = "c"
EOF
build "$src/c.scala"
expect "file added" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"files added or removed"'
same_as_fresh "file added" "$src"
rm "$src/c.scala"
build "$src/c.scala"
expect "file removed" "$(field ok),$(field incremental),$(field fallback)" 'true,false,"files added or removed"'
same_as_fresh "file removed" "$src"
stop

# The same session under --module-per-file --hot: the two packages are modules per file, and a body edit
# rewrites the module of the edited file alone.
rm -rf "$work/out"
start "$src" --split "$work/out" --module-per-file a,b --hot
# hot-build.mjs, the listing of the modules' hashes, is a file of the output like them.
expect "small: first build" "$(field ok),$(field modules)" 'true,8'
expect "small: modules" "$(ls "$work/out" | sort | tr '\n' ' ')" "a.a.mjs app.mjs b.b.mjs hot-build.mjs hot-refresh.mjs main.mjs rt.mjs std.mjs "
same_as_fresh "small: first build" "$src" --module-per-file a,b --hot
sed -i.bak 's/"tagged"/"marked"/' "$src/b.scala"
build "$src/b.scala"
expect "small: body edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["b.b.mjs","hot-build.mjs"]'
same_as_fresh "small: body edit" "$src" --module-per-file a,b --hot
expect "small: body edit runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 3p)" "w/marked/B+green"
# A build that fails writes nothing, the listing included, and the build that fixes it writes
# what a fresh build does: the hashes are the texts', whatever the session went through.
cp -r "$work/out" "$work/before-hot-error"
sed -i.bak 's/def tag: String = "marked"/def tag: String = 42/' "$src/b.scala"
build "$src/b.scala"
expect "small: type error" "$(field ok)" 'false'
if diff -r "$work/out" "$work/before-hot-error" > /dev/null; then ok; else bad "small: type error: the output changed"; fi
sed -i.bak 's/def tag: String = 42/def tag: String = "fixed"/' "$src/b.scala"
build "$src/b.scala"
expect "small: fix after type error" "$(field ok),$(field changed)" 'true,["b.b.mjs","hot-build.mjs"]'
same_as_fresh "small: fix after type error" "$src" --module-per-file a,b --hot
sed -i.bak 's/def tag: String = "fixed"/def tag: String = "marked"/' "$src/b.scala"
build "$src/b.scala"
if diff -r "$work/out" "$work/before-hot-error" > /dev/null; then ok; else bad "small: the text of before gets the hash of before"; fi
rm -rf "$work/before-hot-error"
sed -i.bak 's/class Widget(val label: String)/class Widget(val label: String, val n: Int = 0)/' "$src/a.scala"
build "$src/a.scala"
expect "small: signature change" "$(field ok),$(field incremental)" 'true,false'
if has "Widget: parameters changed"; then ok; else bad "small: signature change: fallback reason: $(field fallback)"; fi
same_as_fresh "small: signature change" "$src" --module-per-file a,b --hot
# A build that changed nothing puts back what another tool removed since: a module, the whole
# directory (a build tool's `clean`), and the build that could not be written before it.
rm "$work/out/main.mjs"
build
expect "small: a removed module is put back" "$(field ok),$(field incremental),$(field changed)" 'true,true,["main.mjs"]'
same_as_fresh "small: a removed module is put back" "$src" --module-per-file a,b --hot
rm -rf "$work/out"
build "$src/b.scala"
expect "small: a removed directory is written again" "$(field ok),$(field changed)" 'true,["a.a.mjs","app.mjs","b.b.mjs","hot-build.mjs","hot-refresh.mjs","main.mjs","rt.mjs","std.mjs"]'
same_as_fresh "small: a removed directory is written again" "$src" --module-per-file a,b --hot
expect "small: the restored directory runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 3p)" "w/marked/B+green"
rm -rf "$work/out"
: > "$work/out"
sed -i.bak 's/"marked"/"tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "small: a directory that cannot be written fails the build" "$(field ok)" 'false'
if has "cannot write"; then ok; else bad "small: a directory that cannot be written: $RESULT"; fi
rm "$work/out"
build
expect "small: the build is written once it can be" "$(field ok),$(field modules)" 'true,8'
same_as_fresh "small: the build is written once it can be" "$src" --module-per-file a,b --hot
stop
# A session writes for the process that started it. That process killed in the middle of a
# build (here: gone while the session's input stays open, as a pipe's other end would not),
# the build is not written: the directory may be another writer's by then.
rm -f "$work/cmd" "$work/ans" "$work/hold"
mkfifo "$work/cmd" "$work/ans" "$work/hold"
WORK=$work bash -c '"$0" compiler watch "$@" < "$WORK/cmd" > "$WORK/ans" 2> "$WORK/err" & echo $! > "$WORK/pid"; read -r _ < "$WORK/hold"' "$TEQ" "$src" --split "$work/out" --module-per-file a,b --hot &
owner=$!
exec 3> "$work/cmd" 4< "$work/ans"
answer
expect "orphan: first build" "$(field ok)" 'true'
session=$(cat "$work/pid")
echo go > "$work/hold"
wait "$owner"
rm -rf "$work/before-orphan"
cp -r "$work/out" "$work/before-orphan"
sed -i.bak 's/"tagged"/"orphaned"/' "$src/b.scala"
echo "build $src/b.scala" >&3
echo >&3
for _ in $(seq 1 100); do kill -0 "$session" 2> /dev/null || break; sleep 0.1; done
if kill -0 "$session" 2> /dev/null; then bad "orphan: the session is still there"; kill "$session"; else ok; fi
exec 3>&- 4<&-
if diff -r "$work/out" "$work/before-orphan" > /dev/null; then ok; else bad "orphan: the build was written"; fi
if grep -q "the process that started the session is gone" "$work/err"; then ok; else bad "orphan: no message: $(head -c 300 "$work/err")"; fi
sed -i.bak 's/"orphaned"/"tagged"/' "$src/b.scala"
rm -rf "$work/before-orphan"
# The owner gone between the two steps of a publication (docs/TARGETS.md, "Watch mode"): a
# session held at TEQ_PUBLISH_GATE after it has staged the build's files under temporary
# names finds its owner gone, removes the temporaries and leaves with nothing put in place;
# with the owner there, the held build is put in place once the gate opens.
gated() {
  rm -f "$work/cmd" "$work/ans" "$work/hold"
  mkfifo "$work/cmd" "$work/ans" "$work/hold"
  : > "$work/gate"
  WORK=$work TEQ_PUBLISH_GATE=$work/gate bash -c '"$0" compiler watch "$@" < "$WORK/cmd" > "$WORK/ans" 2> "$WORK/err" & echo $! > "$WORK/pid"; read -r _ < "$WORK/hold"' "$TEQ" "$src" --split "$work/out" --module-per-file a,b --hot &
  owner=$!
  exec 3> "$work/cmd" 4< "$work/ans"
  answer
  expect "$1: first build" "$(field ok)" 'true'
  session=$(cat "$work/pid")
  rm -f "$work/gate"
  rm -rf "$work/before-gate"
  cp -r "$work/out" "$work/before-gate"
  sed -i.bak "s/\"tagged\"/\"$1\"/" "$src/b.scala"
  echo "build $src/b.scala" >&3
  echo >&3
  for _ in $(seq 1 100); do ls "$work/out"/*.tmp > /dev/null 2>&1 && break; sleep 0.1; done
  expect "$1: the build's files are staged under the session's temporary names" "$(cd "$work/out" && ls *.tmp 2> /dev/null | sed "s/\.$session\.tmp$/.TMP/" | tr '\n' ' ')" "b.b.mjs.TMP hot-build.mjs.TMP "
  if diff -r "$work/out" "$work/before-gate" -x '*.tmp' > /dev/null; then ok; else bad "$1: something was put in place before the gate"; fi
}
gated gated-orphan
echo go > "$work/hold"
wait "$owner"
: > "$work/gate"
for _ in $(seq 1 100); do kill -0 "$session" 2> /dev/null || break; sleep 0.1; done
if kill -0 "$session" 2> /dev/null; then bad "gated orphan: the session is still there"; kill "$session"; else ok; fi
exec 3>&- 4<&-
if diff -r "$work/out" "$work/before-gate" > /dev/null; then ok; else bad "gated orphan: the build was put in place, or a temporary left: $(ls "$work/out" | tr '\n' ' ')"; fi
if grep -q "the build is not put in place" "$work/err"; then ok; else bad "gated orphan: no message: $(head -c 300 "$work/err")"; fi
sed -i.bak 's/"gated-orphan"/"tagged"/' "$src/b.scala"
gated gated-owner
: > "$work/gate"
answer
expect "gated owner: the held build is put in place once the gate opens" "$(field ok),$(field changed)" 'true,["b.b.mjs","hot-build.mjs"]'
expect "gated owner: no temporary is left" "$(cd "$work/out" && ls *.tmp 2> /dev/null | wc -l | tr -d ' ')" '0'
expect "gated owner: the build runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 3p)" "w/gated-owner/B+green"
exec 3>&- 4<&-
for _ in $(seq 1 100); do kill -0 "$session" 2> /dev/null || break; sleep 0.1; done
if kill -0 "$session" 2> /dev/null; then bad "gated owner: the session did not end with its input"; kill "$session"; else ok; fi
echo go > "$work/hold"
wait "$owner"
sed -i.bak 's/"gated-owner"/"tagged"/' "$src/b.scala"
rm -rf "$work/before-gate" "$work/gate"

# Class inheritance across modules (tests/split/inherit): what a body edit in one file changes in
# the module of another. An anonymous subclass makes its superclass keep its body apart, a `super`
# call in a trait adds an accessor to the class that mixes the trait in, and the arguments of a
# parent constructor belong to the signature.
rm -rf "$work/out" "$src"
cp -r tests/split/inherit "$src"
start "$src" --split "$work/out"
expect "inherit: first build" "$(field ok)" 'true'
same_as_fresh "inherit: first build" "$src"
sed -i.bak 's|  println(cheapest(items))|  println(new Coupon("anon") { override def describe = "anonymous " + super.describe }.describe)|' "$src/main.scala"
build "$src/main.scala"
expect "inherit: anonymous subclass" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "inherit: anonymous subclass" "$src"
expect "inherit: anonymous subclass runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 13p)" "anonymous coupon anon costs 0"
cp tests/split/inherit/main.scala "$src/main.scala"
build "$src/main.scala"
expect "inherit: anonymous subclass removed" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "inherit: anonymous subclass removed" "$src"
sed -i.bak 's|"discounted: " + name|"discounted: " + super.describe|' "$src/stock.scala"
build "$src/stock.scala"
expect "inherit: super call in a trait" "$(field ok),$(field incremental),$(field changed)" 'true,true,["shop.mjs","stock.mjs"]'
same_as_fresh "inherit: super call in a trait" "$src"
expect "inherit: super call runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 9p)" "discounted: ink costs 5"
sed -i.bak 's|extends Offer(name, 5)|extends Offer(name, 6)|' "$src/shop.scala"
build "$src/shop.scala"
expect "inherit: parent arguments" "$(field ok),$(field incremental)" 'true,false'
if has "Sale: parents changed"; then ok; else bad "inherit: parent arguments: fallback reason: $(field fallback)"; fi
same_as_fresh "inherit: parent arguments" "$src"
sed -i.bak 's|val percent: Int = 10|val percent: Int = 20|' "$src/stock.scala"
build "$src/stock.scala"
expect "inherit: trait state" "$(field ok),$(field incremental),$(field changed)" 'true,true,["stock.mjs"]'
same_as_fresh "inherit: trait state" "$src"
# Exceptions across modules (tests/split/exceptions): a class of one package extends an exception
# class of another, and edits inside `try` bodies and `catch` cases are body edits that rewrite
# the module of the edited file alone; a parameter added to an exception class is a signature
# change.
rm -rf "$src" "$work/out"
cp -r tests/split/exceptions "$src"
start "$src" --split "$work/out"
expect "exceptions: first build" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "exceptions: first build" "$src"
expect "exceptions: first build runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 4p)" "out: pen"
sed -i.bak 's|"out: " + e.item|"none left of " + e.item|' "$src/main.scala"
build "$src/main.scala"
expect "exceptions: catch case edited" "$(field ok),$(field incremental),$(field changed)" 'true,true,["app.mjs"]'
same_as_fresh "exceptions: catch case edited" "$src"
expect "exceptions: catch case runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 4p)" "none left of pen"
sed -i.bak 's|        case e: StockError => "stock: " + e.getMessage|        case e: Throwable => "any: " + e.getMessage|' "$src/main.scala"
build "$src/main.scala"
# A catch that can take any Throwable brings `js.JavaScriptException` into the std module.
expect "exceptions: catch of Throwable" "$(field ok),$(field incremental),$(field changed)" 'true,true,["app.mjs","std.mjs"]'
same_as_fresh "exceptions: catch of Throwable" "$src"
expect "exceptions: catch of Throwable runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 6p)" "any: only 3 left"
sed -i.bak 's|    println(s"ordered $item")|    println(s"handled $item")|' "$src/shop.scala"
build "$src/shop.scala"
expect "exceptions: finally edited" "$(field ok),$(field incremental),$(field changed)" 'true,true,["shop.mjs"]'
same_as_fresh "exceptions: finally edited" "$src"
expect "exceptions: finally runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 1p)" "handled pen"
sed -i.bak 's|class StockError(message: String) extends Exception(message)|class StockError(message: String, val code: Int = 0) extends Exception(message)|' "$src/stock.scala"
build "$src/stock.scala"
expect "exceptions: parameter added" "$(field ok),$(field incremental)" 'true,false'
if has "StockError: parameters changed"; then ok; else bad "exceptions: parameter added: fallback reason: $(field fallback)"; fi
same_as_fresh "exceptions: parameter added" "$src"
stop
# Overloaded methods: the body of one alternative is typed incrementally and rewrites its module
# alone; an alternative added, removed or given other parameters changes the shape of the file, and
# with it the names the alternatives of other files may have in the output.
rm -rf "$src" "$work/out"
cp -r tests/split/overloads "$src"
start "$src" --split "$work/out"
expect "overloads: first build" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "overloads: first build" "$src"
sed -i.bak 's/"rect of "/"rectangle of "/' "$src/shapes.scala"
build "$src/shapes.scala"
expect "overloads: body of an alternative" "$(field ok),$(field incremental),$(field changed)" 'true,true,["shapes.mjs"]'
same_as_fresh "overloads: body of an alternative" "$src"
expect "overloads: body edit runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 3p)" "circle of 3, rectangle of 4"
sed -i.bak 's/"#" \* size/"+" * size/' "$src/app.scala"
build "$src/app.scala"
expect "overloads: body of an inherited name" "$(field ok),$(field incremental),$(field changed)" 'true,true,["app.mjs"]'
same_as_fresh "overloads: body of an inherited name" "$src"
printf '  def area(side: Int): Int = side * side\n' >> "$src/shapes.scala"
build "$src/shapes.scala"
expect "overloads: alternative added" "$(field ok),$(field incremental)" 'true,false'
if has "Measure.members added or removed"; then ok; else bad "overloads: alternative added: fallback reason: $(field fallback)"; fi
same_as_fresh "overloads: alternative added" "$src"
sed -i.bak 's/def area(side: Int): Int = side \* side/def area(side: Long): Long = side * side/' "$src/shapes.scala"
build "$src/shapes.scala"
expect "overloads: parameters of an alternative" "$(field ok),$(field incremental)" 'true,false'
if has "area: parameters changed"; then ok; else bad "overloads: parameters: fallback reason: $(field fallback)"; fi
same_as_fresh "overloads: parameters of an alternative" "$src"
sed -i.bak '/def area(side: Long)/d' "$src/shapes.scala"
build "$src/shapes.scala"
expect "overloads: alternative removed" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "overloads: alternative removed" "$src"
# The one method of its name gets a second one: its name in the output changes in every module.
sed -i.bak 's/^  def paint(label: String): String = "label " + label/  def paint(label: String): String = "label " + label\
  def frame(width: Int): String = "frame " + width/' "$src/shapes.scala"
build "$src/shapes.scala"
expect "overloads: method added" "$(field ok),$(field incremental)" 'true,false'
printf '\ntrait Titled:\n  def frame(title: String): String = "title " + title\ntrait Poster extends Painter, Titled\n' >> "$src/app.scala"
build "$src/app.scala"
expect "overloads: name joined from two traits" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "overloads: name joined from two traits" "$src"
rm -rf "$work/before-error"
cp -r "$work/out" "$work/before-error"
sed -i.bak 's/println(p.paint(3))/println(p.paint(3.5))/' "$src/app.scala"
build "$src/app.scala"
expect "overloads: no alternative applies" "$(field ok),$(field incremental)" 'false,true'
if has "None of the overloaded alternatives of method paint"; then ok; else bad "overloads: error text: $RESULT"; fi
if diff -r "$work/out" "$work/before-error" > /dev/null; then ok; else bad "overloads: error: the output changed"; fi
sed -i.bak 's/println(p.paint(3.5))/println(p.paint(3))/' "$src/app.scala"
build "$src/app.scala"
expect "overloads: fix" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "overloads: fix" "$src"
stop

# Outlined expansions across modules (tests/split/outline): an inline method of `lib` expanded
# at two sites of `p` and one of `q`, one shape with one shared anonymous class. The function
# and the class stand in the inline method's module, so an edit that takes `q`'s site out of
# the shape rewrites `q` alone; a group of two losing a site would write the survivor inline.
rm -rf "$src" "$work/out"
cp -r tests/split/outline "$src"
start "$src" --split "$work/out"
expect "outline: first build" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "outline: first build" "$src"
sed -i.bak 's|Fields.field\["depth"\](d, 4)|"depth: " + d * 4|' "$src/q.scala"
build "$src/q.scala"
expect "outline: site taken out" "$(field ok),$(field incremental),$(field changed)" 'true,true,["q.mjs"]'
same_as_fresh "outline: site taken out" "$src"
expect "outline: site taken out runs" "$(timeout 20 node "$work/out/main.mjs" | sed -n 3p)" "depth: 120"
cp tests/split/outline/q.scala "$src/q.scala"
build "$src/q.scala"
expect "outline: site put back" "$(field ok),$(field incremental),$(field changed)" 'true,true,["q.mjs"]'
same_as_fresh "outline: site put back" "$src"
# The first of the shared classes, which stands for the others, is one of p's: taking its site
# out makes another the representative, and the expansions that name it are encoded again.
sed -i.bak 's|Fields.field\["width"\](w, 2)|"width: " + w * 2|' "$src/p.scala"
build "$src/p.scala"
expect "outline: representative's site taken out" "$(field ok),$(field incremental),$(field changed)" 'true,true,["p.mjs"]'
same_as_fresh "outline: representative's site taken out" "$src"
cp tests/split/outline/p.scala "$src/p.scala"
build "$src/p.scala"
expect "outline: representative's site put back" "$(field ok),$(field incremental),$(field changed)" 'true,true,["p.mjs"]'
same_as_fresh "outline: representative's site put back" "$src"
stop

# An inferred signature changed while the program has errors (tests/split/inferred): the body
# that infers it types clean, so the build takes the full path at once, where the dependent
# file's error appears beside the one that stood; the modules of the next successful build are
# those of a fresh build.
rm -rf "$src" "$work/out"
cp -r tests/split/inferred "$src"
start "$src" --split "$work/out"
expect "inferred: first build" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "inferred: first build" "$src"
sed -i.bak 's/val bad: Int = 2/val bad: Int = "s"/' "$src/C.scala"
build "$src/C.scala"
expect "inferred: error elsewhere" "$(field ok),$(field incremental)" 'false,true'
sed -i.bak 's/val x = 1/val x = 3/' "$src/A.scala"
build "$src/A.scala"
expect "inferred: an unchanged type stays incremental" "$(field ok),$(field incremental)" 'false,true'
if ! has "the inferred type" && ! has '"file":"'"$src"'/B.scala"'; then ok; else bad "inferred: C's error only, incrementally: $RESULT"; fi
sed -i.bak 's/val x = 3/val x = "s"/' "$src/A.scala"
build "$src/A.scala"
expect "inferred: signature changed under the error" "$(field ok),$(field incremental)" 'false,false'
if has '"file":"'"$src"'/B.scala"' && has '"file":"'"$src"'/C.scala"' && has "the inferred type of x changed"; then ok; else bad "inferred: B's and C's errors after the full path: $RESULT"; fi
sed -i.bak 's/val bad: Int = "s"/val bad: Int = 2/' "$src/C.scala"
build "$src/C.scala"
expect "inferred: the error elsewhere fixed" "$(field ok)" 'false'
if has '"file":"'"$src"'/B.scala"' && ! has '"file":"'"$src"'/C.scala"'; then ok; else bad "inferred: B's error stays: $RESULT"; fi
sed -i.bak 's/val x = "s"/val x = 1/' "$src/A.scala"
build "$src/A.scala"
expect "inferred: the signature back" "$(field ok)" 'true'
same_as_fresh "inferred: the signature back" "$src"
expect "inferred: runs" "$(timeout 20 node "$work/out/main.mjs")" "1"
# A parse failure holds the output back: a valid edit of another file is typed but not
# emitted while it stands, and emitted once the failing file is back, without an edit.
sed -i.bak 's/val bad: Int = 2/val bad: Int = (2/' "$src/C.scala"
build "$src/C.scala"
expect "parse failure: reported" "$(field ok),$(field incremental)" 'false,true'
sed -i.bak 's/val y: Int = A.x/val y: Int = A.x + 1/' "$src/B.scala"
build "$src/B.scala"
expect "parse failure: another file's edit is held back" "$(field ok),$(field incremental)" 'false,true'
if has '"file":"'"$src"'/C.scala"' && has '"retyped":["'"$src"'/B.scala"]'; then ok; else bad "parse failure: C's error with B retyped: $RESULT"; fi
expect "parse failure: the output stands" "$(timeout 20 node "$work/out/main.mjs")" "1"
sed -i.bak 's/val bad: Int = (2/val bad: Int = 2/' "$src/C.scala"
build "$src/C.scala"
expect "parse failure: cleared, the held-back build emitted" "$(field ok),$(field incremental),$(field changed)" 'true,true,["_root_.mjs"]'
same_as_fresh "parse failure: cleared" "$src"
expect "parse failure: runs" "$(timeout 20 node "$work/out/main.mjs")" "2"
# A producer and its consumer edited in one build while the producer has an error of its own
# and a parse failure elsewhere holds the output back: the consumer, typed against the
# signature that is then put back, is typed again with the next build, and the output of the
# recovery is a fresh build's.
sed -i.bak 's/val bad: Int = 2/val bad: Int = (2/' "$src/C.scala"
build "$src/C.scala"
expect "batch: parse failure elsewhere" "$(field ok),$(field incremental)" 'false,true'
sed -i.bak 's/val x = 1/val x = { val oops: Int = "b"; "s" }/' "$src/A.scala"
sed -i.bak 's/val y: Int = A.x + 1/val y: Int = A.x.length/' "$src/B.scala"
build "$src/A.scala" "$src/B.scala"
expect "batch: the producer's own error" "$(field ok),$(field incremental)" 'false,true'
if has '"file":"'"$src"'/A.scala"' && ! has "value length"; then ok; else bad "batch: A's error alone: $RESULT"; fi
sed -i.bak 's/val x = { val oops: Int = "b"; "s" }/val x = 1/' "$src/A.scala"
build "$src/A.scala"
expect "batch: the producer restored, the consumer typed again" "$(field ok),$(field incremental)" 'false,true'
if has "value length is not a member of Int"; then ok; else bad "batch: B's error against Int: $RESULT"; fi
sed -i.bak 's/val y: Int = A.x.length/val y: Int = A.x + 1/' "$src/B.scala"
build "$src/B.scala"
expect "batch: the consumer restored, output still held" "$(field ok),$(field incremental)" 'false,true'
expect "batch: the output stands" "$(timeout 20 node "$work/out/main.mjs")" "2"
sed -i.bak 's/val bad: Int = (2/val bad: Int = 2/' "$src/C.scala"
build "$src/C.scala"
expect "batch: cleared, the held-back builds emitted" "$(field ok),$(field incremental)" 'true,true'
same_as_fresh "batch: cleared" "$src"
expect "batch: runs" "$(timeout 20 node "$work/out/main.mjs")" "2"
# A type error in a method with a declared type is kept by the retype and stays the answer of
# an unchanged retry, named or plain; the fix is incremental.
sed -i.bak 's/val y: Int = A.x + 1/val y: Int = A.x + "1"/' "$src/B.scala"
build "$src/B.scala"
expect "retry: type error" "$(field ok),$(field incremental)" 'false,true'
build "$src/B.scala"
expect "retry: the error kept on an unchanged retry" "$(field ok),$(field incremental)" 'false,true'
if has '"file":"'"$src"'/B.scala"'; then ok; else bad "retry: the error's file: $RESULT"; fi
build
expect "retry: the error kept on a plain build" "$(field ok)" 'false'
sed -i.bak 's/val y: Int = A.x + "1"/val y: Int = A.x + 1/' "$src/B.scala"
build "$src/B.scala"
expect "retry: fix" "$(field ok),$(field incremental),$(field changed)" 'true,true,[]'
stop

# An edit of an inline body (tests/split/inline) types the files that expanded it again, on
# their unchanged ASTs, and rewrites their modules: the output is a fresh build's.
rm -rf "$src" "$work/out"
cp -r tests/split/inline "$src"
start "$src" --split "$work/out"
expect "inline: first build" "$(field ok),$(field incremental)" 'true,false'
same_as_fresh "inline: first build" "$src"
expect "inline: first build runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "42 1 "
sed -i.bak 's/inline def factor: Int = 2/inline def factor: Int = 3/' "$src/lib.scala"
build "$src/lib.scala"
expect "inline: body edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["use.mjs"]'
if has '"retyped":["'"$src"'/use.scala","'"$src"'/lib.scala"]'; then ok; else bad "inline: the expanding file retyped with the body: $(field retyped)"; fi
same_as_fresh "inline: body edit" "$src"
expect "inline: body edit runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "63 1 "
sed -i.bak 's/inline def flag: Boolean = true/inline def flag: Boolean = false/' "$src/lib.scala"
build "$src/lib.scala"
expect "inline: condition edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["use.mjs"]'
same_as_fresh "inline: condition edit" "$src"
expect "inline: condition edit runs" "$(timeout 20 node "$work/out/main.mjs" | tr '\n' ' ')" "63 2 "
stop

# --- Syntax errors and their fixes --------------------------------------------------------------
# For each representative shape of a syntax error, a
# session over tests/split/cycle takes the broken text, typed from its recovered tree, which
# writes nothing, and then the fix, after which every module, the unrelated files' included, is
# a fresh build's; then two files broken and fixed in either order with an edit of a third
# between, with and without a full path at every second build (`TEQ_COMPACT_EVERY=2`).
src=$work/syntax
shapes=(
  'b.scala|s/def tag: String = "tagged"/def tag: String = ("tagged"/|a body with an unclosed parenthesis'
  'a.scala|s/def describe(c: Color): String = c match/def describe(c: Color): String c match/|a def missing its ='
  'a.scala|s/class Widget(val label: String) extends Tagged:/class Widget(val label: String extends Tagged:/|a broken class header'
  'a.scala|s/case Color.Red => "red: "/case => "red: "/|a case missing its pattern'
  'b.scala|s/^object BConst:/object BConst {/|a template colon made a brace'
  'b.scala|s/"tagged"/"tagged/|an unterminated string'
)
for shape in "${shapes[@]}"; do
  IFS='|' read -r file edit what <<< "$shape"
  rm -rf "$src" "$work/out" && cp -r tests/split/cycle "$src"
  start "$src" --split "$work/out"
  rm -rf "$work/before-syntax" && cp -r "$work/out" "$work/before-syntax"
  sed "$edit" "tests/split/cycle/$file" > "$src/$file"
  build "$src/$file"
  expect "syntax, $what: broken" "$(field ok)" false
  if diff -r "$work/out" "$work/before-syntax" > /dev/null; then ok; else bad "syntax, $what: the output changed"; fi
  cp "tests/split/cycle/$file" "$src/$file"
  build "$src/$file"
  expect "syntax, $what: fixed" "$(field ok)" true
  same_as_fresh "syntax, $what: fixed" "$src"
  stop
done
broken_text() {
  case $1 in
    a.scala) sed 's/case Color.Red => "red: "/case => "red: "/' tests/split/cycle/a.scala ;;
    b.scala) sed 's/def tag: String = "tagged"/def tag: String = ("tagged"/' tests/split/cycle/b.scala ;;
  esac
}
for compact in 0 2; do
  for order in "a.scala b.scala" "b.scala a.scala"; do
    rm -rf "$src" "$work/out" && cp -r tests/split/cycle "$src"
    TEQ_COMPACT_EVERY=$compact start "$src" --split "$work/out"
    for file in $order; do
      broken_text $file > "$src/$file"
      build "$src/$file"
      expect "syntax, $order in turn (compaction $compact): $file broken" "$(field ok)" false
    done
    sed -i.bak 's/println(Widget("w").show)/println(Widget("v").show)/' "$src/main.scala" && rm -f "$src/main.scala.bak"
    build "$src/main.scala"
    expect "syntax, $order in turn (compaction $compact): an unrelated edit" "$(field ok)" false
    for file in $order; do
      cp "tests/split/cycle/$file" "$src/$file"
      build "$src/$file"
    done
    expect "syntax, $order in turn (compaction $compact): fixed" "$(field ok)" true
    same_as_fresh "syntax, $order in turn (compaction $compact): fixed" "$src"
    stop
  done
done
rm -rf "$work/out"

# --- Every test program -------------------------------------------------------------------------
# A session over scala-library's jar (tests/split/classpath, `--std=scala-library`): the jar, its
# signatures and the library bodies typed so far stay in memory across the builds. A body edit
# rewrites the module of the edited file alone, and an edit that reaches more of the library
# rewrites the library module that gained a member; it skips without the jar in the coursier
# cache.
rm -rf "$work/out" "$src"
cp -r tests/split/classpath "$src"
if "$TEQ" compiler check "$src" --std=scala-library > "$work/jar.log" 2>&1; then
  start "$src" --split "$work/out" --std=scala-library
  expect "classpath: first build" "$(field ok),$(field incremental)" 'true,false'
  same_as_fresh "classpath: first build" "$src" --std=scala-library
  expect "classpath: first build runs" "$(timeout 20 node "$work/out/main.mjs")" "[1,2,3]"
  sed -i.bak 's/mkString("\[", ",", "\]")/mkString("<", ",", ">")/' "$src/util.scala"
  build "$src/util.scala"
  expect "classpath: body edit" "$(field ok),$(field incremental),$(field changed)" 'true,true,["util.mjs"]'
  same_as_fresh "classpath: body edit" "$src" --std=scala-library
  expect "classpath: body edit runs" "$(timeout 20 node "$work/out/main.mjs")" "<1,2,3>"
  # An edit that reaches more of the library rewrites the library modules that gained a member;
  # the classes typed by the later build may stand in another order than a fresh build's, so
  # the outputs are compared by what they print.
  sed -i.bak 's/xs.sorted/xs.sorted.reverse.padTo(4, 0)/' "$src/main.scala"
  build "$src/main.scala"
  expect "classpath: more of the library reached" "$(field ok),$(field incremental)" 'true,true'
  if has '"app.mjs"' && has '"scala.collection.mjs"'; then ok; else bad "classpath: more of the library reached: changed $(field changed)"; fi
  expect "classpath: more of the library runs" "$(timeout 20 node "$work/out/main.mjs")" "<3,2,1,0>"
  rm -rf "$work/fresh"
  "$TEQ" compiler build "$src" --std=scala-library $ONE --split "$work/fresh" > "$work/fresh.log" 2>&1
  expect "classpath: more of the library as fresh" "$(timeout 20 node "$work/fresh/main.mjs")" "$(timeout 20 node "$work/out/main.mjs")"
  stop
else
  echo "skip the classpath session: $(tail -1 "$work/jar.log")"
fi

# A session over monocle's jar (tests/split/retype_bridges): a body edit of the file that uses the
# library rewrites its own module alone, and the session's output stays a fresh build's. The
# reach pass names the library's members apart as it compiles their classes, after the classes
# above them were bridged; a retype gives the retyped classes their bridges anew and leaves every
# other class's as its check made them, whatever the reach settled since. Both orders of the
# inputs, since which classes the reach compiles first, and so which names stand when a class
# is bridged, follows the order: with the library file first the retype used to bridge the jar's
# anonymous classes to `PPrism.some`, reach it and its anonymous class through the bridge, and
# rewrite monocle.mjs with a class the fresh build does not have.
rm -rf "$work/out" "$src"
cp -r tests/split/retype_bridges "$src"
jars_of "$src/lib"
if [ -n "$JARS_MISSING" ]; then
  echo "skip the retype_bridges sessions: not in the coursier cache:$JARS_MISSING"
  ok
else
  for order in "lib page" "page lib"; do
    inputs=""
    for d in $order; do inputs="$inputs $src/$d"; done
    rm -rf "$work/out"
    start $inputs --classpath "$JARS_CP" --module-per-file page --split "$work/out"
    expect "retype_bridges ($order): first build" "$(field ok),$(field incremental)" 'true,false'
    same_as_fresh "retype_bridges ($order): first build" $inputs --classpath "$JARS_CP" --module-per-file page
    expect "retype_bridges ($order): first build runs" "$(timeout 20 node "$work/out/main.mjs" | head -1)" "Greeting: Bob of Oslo, Main Bob"
    for n in 1 2; do
      sed -i.bak "s/\"Greeting[a-z]*: \"/\"Greeting$(printf 's%.0s' $(seq 1 $n)): \"/" "$src/page/Page.scala"
      build "$src/page/Page.scala"
      expect "retype_bridges ($order): body edit $n" "$(field ok),$(field incremental),$(field changed)" 'true,true,["page.Page.mjs"]'
      same_as_fresh "retype_bridges ($order): body edit $n" $inputs --classpath "$JARS_CP" --module-per-file page
    done
    stop
    cp -r tests/split/retype_bridges/page/Page.scala "$src/page/Page.scala"
  done
fi

# A session over the reader's fixtures (tests/split/reader):
# the library bodies' declarations resolved in the first build are the loader's, and each file
# typed again, the library class's expansions among them, reads them again; after every edit the
# session's output is a fresh build's and runs as one.
rm -rf "$work/out" "$src"
cp -r tests/split/reader "$src"
jars_of "$src"
if [ -n "$JARS_MISSING" ]; then
  echo "skip the reader session: not in the coursier cache:$JARS_MISSING"
  ok
else
  start "$src" --classpath "$JARS_CP" --split "$work/out"
  expect "reader: first build" "$(field ok),$(field incremental)" 'true,false'
  same_as_fresh "reader: first build" "$src" --classpath "$JARS_CP"
  expect "reader: first build runs" "$(timeout 20 node "$work/out/main.mjs" | tail -1)" "130"
  while IFS='|' read -r file edit printed; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "reader: $file edited" "$(field ok),$(field incremental)" 'true,true'
    same_as_fresh "reader: $file edited" "$src" --classpath "$JARS_CP"
    expect "reader: $file edited runs" "$(timeout 20 node "$work/out/main.mjs" | tail -1)" "$printed"
  done <<'STEPS'
ones.scala|s/base(5)/base(6)/|140
twos.scala|s/put("x")/put("xy")/|141
main.scala|s/RdBox(1)/RdBox(2)/|141
ones.scala|s/base(6)/base(7)/|151
STEPS
  stop
fi

for case in tests/cases/*.scala tests/cases/*/ tests/interop/*.scala tests/interop/*/; do
  case=${case%/}
  name=$(basename "$case" .scala)
  suite=$(dirname "$case")
  [ "$case" = "$STUB" ] && continue
  [ -f "$suite/$name.expected" ] || continue
  rm -rf "$src" "$work/out"
  mkdir -p "$src"
  if [ -d "$case" ]; then cp -r "$case"/. "$src"; else cp "$case" "$src/"; fi
  # The interop programs are written against the stub; a case takes the std's Scala.js layer.
  extra=""
  if [ "$suite" = tests/interop ] && grep -rq 'scala\.scalajs' "$src" && ! grep -rq '^package scala\.scalajs' "$src"; then
    extra=$(pwd)/$STUB
  fi
  flags=$(grep -h -o '^// teq: .*' "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $name: not in the coursier cache:$JARS_MISSING"
    ok
    continue
  fi
  [ -n "$JARS_CP" ] && flags="$flags --classpath $JARS_CP"
  file=$(ls "$src"/*.scala | head -1)
  start "$src" $extra $flags --split "$work/out"
  if [ "$(field ok)" != true ]; then
    bad "$name: first build: $(echo "$RESULT" | head -c 300)"
    stop
    continue
  fi
  # A comment on the second line moves every position in the file.
  sed -i.bak '1a\
// watch
' "$file"
  build "$file"
  # An enum case that leaves the type arguments of its enum to inference takes the full path,
  # and so does a file whose definitions a macro ran.
  if has "infers the type arguments" || has "a macro ran its definitions"; then ok; else
    expect "$name: comment inserted" "$(field ok),$(field incremental)" 'true,true'
  fi
  same_as_fresh "$name: comment inserted" "$src" $extra $flags
  # The first string literal of the file gains a character.
  perl -0pi -e 's/"([^"\\\n]*)"/"$1~"/' "$file"
  build "$file"
  if [ "$(field ok)" = true ]; then
    same_as_fresh "$name: string edited" "$src" $extra $flags
  elif "$TEQ" compiler build "$src" $extra $flags $ONE --split "$work/fresh" > /dev/null 2>&1; then
    bad "$name: string edited: watch reports an error, the fresh build does not"
  else
    ok
  fi
  stop
done

# --- One file after another ---------------------------------------------------------------------
retype_entry
retype_reads
retype_facade
retype_roots
retype_state
retype_state_pair
retype_init
retype_cacheable
retype_jar_state
retype_quotes
retype_sweep

sessions_forked split-watch
echo "$pass passed, $fail failed"
[ $fail = 0 ]
