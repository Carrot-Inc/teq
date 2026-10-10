#!/bin/bash
# The resident check (`teq compiler watch --check`): a session is driven over stdin and every answer's
# `diagnostics` is compared with what the edit should produce. The scenarios run on a copy of
# tests/split/cycle: a clean first build, a type error and its fix (typed incrementally, since the
# typed program survives an error), a parse error and its fix, a warning (and the same program
# under --werror), a file added, the UTF-16 columns of a line with a non-ASCII character, a first
# build with an error followed by its fix, a constant val's value changed where its singleton
# type is read (the full path, as a fresh check), a nested class's prefix changed under a client
# (scalac's error, as a fresh check), a first build with a file that fails to parse (its
# recovered tree typed with the rest) followed by its fix, and the `text` command: an unsaved
# text standing in for the file on disk for a named and for a plain `build`, and its withdrawal;
# and that a text of megabytes with a parse error leaves no mapping kept once two more builds
# have parsed and typed (`stats`, docs/SPEED.md "A session's memory"); the lifecycle of a parse
# error (an unchanged retry, an unrelated edit, a full rebuild, a text withdrawn, the file
# removed) and, for each representative shape of a syntax error broken and fixed (one at each of
# the parser's recovery sites), in two files in either order and under compaction, and for a
# file whose package blocks come and go and a local class whose member class comes and goes, the
# diagnostics, definition, references, hover and the accumulated analysis of a fresh session at
# every step; and that a session
# with the navigation index whose every second build takes the full path for the session's
# memory (`TEQ_COMPACT_EVERY`) answers its builds and its queries as one that never does,
# through an unsaved text, an error, its fix and a file deleted; and that a query demanding every
# std file leaves the next body edit incremental. Then a JVM
# session over tests/split/classpath (`--target jvm`, linked against scala-library), which skips without
# scala-library in the coursier cache. Last the sessions of tests/support/retype.sh, in which
# one file is typed again after another, each once more under `--index`, where a definition and
# a references query at the end answer as in a fresh session, and every file of the realistic
# frontend in turn. Before them a session under `--own`, which answers the classes of the owned
# files for a build tool's discovery. Every session types its full builds at the automatic count and
# every fresh session it is compared with by one worker (tests/support/sessions.sh); the last line
# says which sessions forked.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
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
# Starts a check session on the given teq arguments; commands go to fd 3, answers come from fd 4.
start() {
  rm -f "$work/cmd" "$work/ans"
  mkfifo "$work/cmd" "$work/ans"
  "$TEQ" compiler watch --check "$@" < "$work/cmd" > "$work/ans" 2> "$work/err" &
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
# `text <path> <bytes>` with the contents of the file `$2` as the text of `$1`.
text() {
  echo "text $1 $(wc -c < "$2" | tr -d ' ')" >&3
  cat "$2" >&3
}
withdraw() { echo "text $1 0" >&3; }
field() { echo "$RESULT" | grep -o "\"$1\":\(\[[^]]*\]\|[^,}]*\)" | head -1 | sed "s/\"$1\"://"; }
has() { case "$RESULT" in *"$1"*) return 0 ;; *) return 1 ;; esac; }
# The count of diagnostics of the answer with the given severity.
count() { echo "$RESULT" | grep -o "\"severity\":\"$1\"" | wc -l | tr -d ' '; }

retype_start() { start "$@"; }
after_failed_full=true
# The answer's diagnostics against those of a fresh session's first build.
as_fresh() {
  local what=$1 answered=$RESULT
  shift
  RESULT=$(echo quit | timeout 60 "$TEQ" compiler watch --check "$@" $ONE 2> /dev/null | head -1)
  local fresh="$(field ok),$(diagnostics)"
  RESULT=$answered
  expect "$what: the diagnostics of a fresh check" "$(field ok),$(diagnostics)" "$fresh"
}
# The whole list of the answer's diagnostics.
diagnostics() { echo "$RESULT" | sed 's/.*"diagnostics":\(\[.*\]\),"ms":.*/\1/'; }
retype_runs() { :; }
sweep_end() { as_fresh "$@"; }
# With `--index` among the flags: the definition and the references of the name at the first
# <text> of <file>, as a fresh session over the same sources answers them.
retype_queries() {
  local file=$1 text=$2 at query fresh
  shift 2
  case " $* " in *" --index "*) ;; *) return ;; esac
  at=$(grep -b -o -F -- "$text" "$file" | head -1 | cut -d: -f1)
  for query in "definition $file $at" "references $file $at 1"; do
    echo "$query" >&3
    answer
    fresh=$({ echo "$query"; echo quit; } | timeout 60 "$TEQ" compiler watch --check "$@" $ONE 2> /dev/null | sed -n 2p)
    expect "the ${query%% *} of $text in ${file##*/} as in a fresh session" "$RESULT" "$fresh"
  done
}
. tests/support/retype.sh

# --- The scripted scenarios ---------------------------------------------------------------------
src=$work/src
cp -r tests/split/cycle "$src"
start "$src"
expect "first build" "$(field ok),$(field incremental),$(field fallback),$(field diagnostics)" 'true,false,"first build",[]'
if has '"modules":0' && ! [ -e "$work/out" ]; then ok; else bad "first build: wrote something: $RESULT"; fi

sed -i.bak 's/def tag: String = "tagged"/def tag: String = 42/' "$src/b.scala"
build "$src/b.scala"
expect "type error" "$(field ok),$(field incremental)" 'false,true'
if has '"file":"'"$src"'/b.scala","line":6,"col":21,"endLine":6,"endCol":23,"severity":"error","message":"type mismatch: found Int, required String"'; then ok; else bad "type error diagnostic: $RESULT"; fi
if has '"source":"  def tag: String = 42"' && has '"caret":"                    ^^"'; then ok; else bad "type error source and caret: $RESULT"; fi
expect "type error is also an error of the build answer" "$(echo "$RESULT" | grep -o '"errors":\[{' | wc -l | tr -d ' ')" 1

sed -i.bak 's/def tag: String = 42/def tag: String = "tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "fix after type error" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'

sed -i.bak 's/def tag: String = "tagged"/def tag: String = ("tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "parse error" "$(field ok),$(count error)" 'false,1'
if has '"file":"'"$src"'/b.scala","line":8,"col":1,"endLine":8,"endCol":5,"severity":"error"'; then ok; else bad "parse error position: $RESULT"; fi
sed -i.bak 's/def tag: String = ("tagged"/def tag: String = "tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "fix after parse error" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'

# A line with a two-byte character before the error: the columns count UTF-16 units.
sed -i.bak 's/def tag: String = "tagged"/def tag: String = "tägged" + 1.x/' "$src/b.scala"
build "$src/b.scala"
expect "utf-16 columns" "$(field ok),$(count error)" 'false,1'
if has '"line":6,"col":32,"endLine":6,"endCol":35,"severity":"error","message":"value x is not a member of Int"'; then ok; else bad "utf-16 columns: $RESULT"; fi
sed -i.bak 's/def tag: String = "tägged" + 1.x/def tag: String = "tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "fix after utf-16 columns" "$(field ok),$(field diagnostics)" 'true,[]'

# A warning: a pure expression in statement position. The definition added takes the full path.
printf '\ndef pure(): Unit =\n  1\n  ()\n' >> "$src/b.scala"
build "$src/b.scala"
expect "warning" "$(field ok),$(field incremental),$(count warning),$(count error)" 'true,false,1,0'
if has '"severity":"warning","message":"A pure expression does nothing in statement position"'; then ok; else bad "warning diagnostic: $RESULT"; fi
expect "warning is also a warning of the build answer" "$(echo "$RESULT" | grep -o '"warnings":\[{' | wc -l | tr -d ' ')" 1

stop

# The warning under --werror fails the check; the diagnostic keeps its severity.
start "$src" --werror
expect "werror: first build" "$(field ok),$(count warning),$(count error)" 'false,1,0'
expect "werror: the warning is an error of the build answer" "$(echo "$RESULT" | grep -o '"errors":\[{' | wc -l | tr -d ' ')" 1
stop

# The same file through `text`, in a session over the program without the warning: the text
# stands in for the file on disk, for a named build and for a plain one, until it is withdrawn.
cp tests/split/cycle/b.scala "$src/b.scala"
start "$src"
expect "text: first build" "$(field ok),$(field diagnostics)" 'true,[]'
sed 's/def tag: String = "tagged"/def tag: String = 42/' "$src/b.scala" > "$work/edited.scala"
text "$src/b.scala" "$work/edited.scala"
build "$src/b.scala"
expect "text: named build" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"line":6,"col":21,"endLine":6,"endCol":23,"severity":"error"'; then ok; else bad "text: named build diagnostic: $RESULT"; fi
build "$src/b.scala"
expect "text: unchanged text keeps the diagnostics" "$(field ok),$(count error)" 'false,1'
withdraw "$src/b.scala"
build "$src/b.scala"
expect "text: withdrawn" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
text "$src/b.scala" "$work/edited.scala"
build
expect "text: plain build" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
withdraw "$src/b.scala"
build
expect "text: plain build after withdrawal" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
# A text of a file that is not part of the program changes nothing.
text "$work/elsewhere.scala" "$work/edited.scala"
build
expect "text: a file outside the program" "$(field ok),$(field diagnostics)" 'true,[]'
withdraw "$work/elsewhere.scala"
# A text that fails to parse is not applied; its parse error stays in every answer, the other
# files' builds included, until the file parses again.
sed 's/def greet: String = s"hello from $name"/def greet: String = (s"hello from $name"/' "$src/a.scala" > "$work/a-broken.scala"
text "$src/a.scala" "$work/a-broken.scala"
build "$src/a.scala"
expect "parse failure: reported" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$src"'/a.scala"'; then ok; else bad "parse failure: a.scala expected: $RESULT"; fi
sed 's/"tagged"/"marked"/' "$src/b.scala" > "$work/b-marked.scala"
text "$src/b.scala" "$work/b-marked.scala"
build "$src/b.scala"
expect "parse failure: stays in another file's answer" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$src"'/a.scala"' && has '"retyped":["'"$src"'/b.scala"]'; then ok; else bad "parse failure: a.scala's error with b.scala retyped: $RESULT"; fi
withdraw "$src/a.scala"
build "$src/a.scala"
expect "parse failure: gone once the file parses" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
withdraw "$src/b.scala"
build "$src/b.scala"
expect "parse failure: b back" "$(field ok),$(field diagnostics)" 'true,[]'

cat > "$src/c.scala" <<'EOS'
package c
val cTop: Int = "c"
EOS
build "$src/c.scala"
expect "file added" "$(field ok),$(field incremental),$(field fallback),$(count error)" 'false,false,"files added or removed",1'
if has '"file":"'"$src"'/c.scala","line":2,"col":17,"endLine":2,"endCol":20,"severity":"error"'; then ok; else bad "file added diagnostic: $RESULT"; fi
rm "$src/c.scala"
build "$src/c.scala"
expect "file removed" "$(field ok),$(field fallback),$(field diagnostics)" 'true,"files added or removed",[]'
# A text and a file added in one build: the full path builds with the text, not the disk.
text "$src/b.scala" "$work/edited.scala"
printf 'package d\nval dTop: Int = 1\n' > "$src/d.scala"
build "$src/b.scala" "$src/d.scala"
expect "text and a file added" "$(field ok),$(field incremental),$(field fallback),$(count error)" 'false,false,"files added or removed",1'
if has '"file":"'"$src"'/b.scala","line":6,"col":21'; then ok; else bad "text and a file added: b.scala's text expected: $RESULT"; fi
withdraw "$src/b.scala"
rm "$src/d.scala"
build
expect "text withdrawn and the file removed" "$(field ok),$(field incremental),$(field diagnostics)" 'true,false,[]'
# An unknown command answers with the diagnostics array too.
echo frobnicate >&3
answer
expect "unknown command" "$(field ok),$(count error)" 'false,1'
if has '"diagnostics":[{"severity":"error","message":"unknown command: frobnicate"}]'; then ok; else bad "unknown command: $RESULT"; fi
stop

# A constant val without a type stands for its literal type where its singleton is read (`x:
# C.one.type` returned as `1`), which its signature (`Int`) does not carry: a change of its value
# takes the full path, whose answer is a fresh check's.
cdir=$work/constant
mkdir -p "$cdir"
printf 'object C:\n  final val one = 1\n' > "$cdir/A.scala"
printf 'def f(x: C.one.type): 1 = x\n' > "$cdir/B.scala"
start "$cdir/A.scala" "$cdir/B.scala"
expect "a constant's singleton: the first build" "$(field ok)" 'true'
printf 'object C:\n  final val one = 2\n' > "$cdir/A.scala"
build "$cdir/A.scala"
expect "a constant's value changed: the full path" "$(field ok),$(field incremental),$(count error)" 'false,false,1'
as_fresh "a constant's value changed" "$cdir/A.scala" "$cdir/B.scala"
printf 'object C:\n  final val one: Int = 1\n' > "$cdir/A.scala"
build "$cdir/A.scala"
as_fresh "a constant given a wider type" "$cdir/A.scala" "$cdir/B.scala"
printf 'object C:\n  final val one = 1\n' > "$cdir/A.scala"
build "$cdir/A.scala"
expect "a constant again" "$(field ok),$(count error)" 'true,0'
as_fresh "a constant again" "$cdir/A.scala" "$cdir/B.scala"
stop

# A class nested in a class is a type of its own through each prefix: a result moved from
# `a.Item` to `b.Item` breaks a client of `a.Item` where scalac reports it (E007, `Found:
# API.b.Item, Required: API.a.Item`), as a fresh check answers, and moving it back fixes it.
pdir=$work/prefix
mkdir -p "$pdir"
printf 'class O:\n  class Item\nobject API:\n  val a = new O\n  val b = new O\n  def make: a.Item = new a.Item\n' > "$pdir/A.scala"
printf 'def use: API.a.Item = API.make\n' > "$pdir/B.scala"
start "$pdir/A.scala" "$pdir/B.scala"
expect "a nested class through a prefix: the first build" "$(field ok),$(count error)" 'true,0'
sed -i.bak 's/a\.Item = new a\.Item/b.Item = new b.Item/' "$pdir/A.scala"
build "$pdir/A.scala"
expect "a nested class's prefix changed" "$(field ok),$(count error)" 'false,1'
if has 'found b.Item, required a.Item'; then ok; else bad "a nested class's prefix changed: $RESULT"; fi
as_fresh "a nested class's prefix changed" "$pdir/A.scala" "$pdir/B.scala"
mv "$pdir/A.scala.bak" "$pdir/A.scala"
build "$pdir/A.scala"
expect "a nested class's prefix back" "$(field ok),$(count error)" 'true,0'
as_fresh "a nested class's prefix back" "$pdir/A.scala" "$pdir/B.scala"
stop

# A first build with an error keeps the typed program: the fix is typed incrementally.
sed -i.bak 's/def tag: String = "tagged"/def tag: String = 42/' "$src/b.scala"
start "$src"
expect "first build with an error" "$(field ok),$(field incremental),$(count error)" 'false,false,1'
sed -i.bak 's/def tag: String = 42/def tag: String = "tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "fix after a first build with an error" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
stop

# A first build with a file that fails to parse types the file's recovered tree with the rest:
# the parse error is the one diagnostic, a.scala's import of b's package resolving and its names
# (BConst) found; the fix is typed incrementally and clears the parse error.
sed -i.bak 's/def tag: String = "tagged"/def tag: String = ("tagged"/' "$src/b.scala"
start "$src"
expect "first build with a parse error" "$(field ok),$(field incremental),$(count error)" 'false,false,1'
if has '"file":"'"$src"'/b.scala","line":8,"col":1' && ! has '"file":"'"$src"'/a.scala"'; then ok; else bad "first build with a parse error: the parse error alone, the rest typed against the recovered file: $RESULT"; fi
sed -i.bak 's/def tag: String = ("tagged"/def tag: String = "tagged"/' "$src/b.scala"
build "$src/b.scala"
expect "fix after a first build with a parse error" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
stop

# The lifecycle of a parse error (docs/TARGETS.md, "The resident check"): the text with the error
# is applied and its recovered tree typed; its diagnostics stay through an unchanged retry, an
# unrelated edit and a full rebuild, which keeps the file's tree, and go with a parse that finds
# none or with the file; a text withdrawn brings the disk text back with the next build of any
# file. No `api` graph accompanies an answer that is not `ok`, and no answer names a file held.
life=$work/life && rm -rf "$life" && mkdir -p "$life" && cp tests/split/cycle/*.scala "$life"
sed 's/def tag: String = "tagged"/def tag: String = ("tagged"/' tests/split/cycle/b.scala > "$life/b.scala"
start "$life" --index --analysis-version 2
expect "lifecycle: a first build with a parse error" "$(field ok),$(count error)" 'false,1'
if has '"file":"'"$life"'/b.scala","line":8' && ! has '"api":{' && ! has '"held"'; then ok; else bad "lifecycle: the parse error, no api, nothing held: $RESULT"; fi
as_fresh "lifecycle: a first build with a parse error" "$life" --index
build "$life/b.scala"
expect "lifecycle: an unchanged retry" "$(field ok),$(field incremental),$(field retyped),$(count error)" 'false,true,[],1'
if has '"file":"'"$life"'/b.scala","line":8' && ! has '"api":{'; then ok; else bad "lifecycle: an unchanged retry keeps the parse error: $RESULT"; fi
sed -i.bak 's/"A+" + Mode.Fast.name/"A++" + Mode.Fast.name/' "$life/a.scala" && rm "$life/a.scala.bak"
build "$life/a.scala"
expect "lifecycle: an unrelated edit" "$(field ok),$(field incremental),$(field retyped),$(count error)" 'false,true,["'"$life"'/a.scala"],1'
if has '"file":"'"$life"'/b.scala","line":8'; then ok; else bad "lifecycle: an unrelated edit keeps the parse error: $RESULT"; fi
as_fresh "lifecycle: an unrelated edit" "$life" --index
printf 'package c\n\nobject Added:\n  val n: Int = 1\n' > "$life/added.scala"
build "$life/added.scala"
expect "lifecycle: a full rebuild" "$(field ok),$(field incremental),$(field fallback),$(count error)" 'false,false,"files added or removed",1'
if has '"file":"'"$life"'/b.scala","line":8' && ! has '"api":{'; then ok; else bad "lifecycle: a full rebuild keeps the file's tree and its parse error: $RESULT"; fi
as_fresh "lifecycle: a full rebuild" "$life" --index
cp tests/split/cycle/b.scala "$work/b-fixed.scala"
text "$life/b.scala" "$work/b-fixed.scala"
build "$life/b.scala"
expect "lifecycle: a fixed text handed in" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
if has '"api":{'; then ok; else bad "lifecycle: an ok answer carries the api: $RESULT"; fi
withdraw "$life/b.scala"
build "$life/a.scala"
expect "lifecycle: the text withdrawn" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$life"'/b.scala","line":8' && ! has '"api":{'; then ok; else bad "lifecycle: the disk text and its parse error back with a build of another file: $RESULT"; fi
as_fresh "lifecycle: the text withdrawn" "$life" --index
rm "$life/b.scala"
build "$life/b.scala"
expect "lifecycle: the file removed" "$(field ok),$(field incremental)" 'false,false'
if ! has "')', found" && has 'cannot resolve import: b not found'; then ok; else bad "lifecycle: the parse error gone with its file: $RESULT"; fi
as_fresh "lifecycle: the file removed" "$life" --index
stop

# History independence (docs/TARGETS.md, "The resident check"): for each representative shape of a
# syntax error, a session over good sources takes the broken text and then the fixed one, and at
# each state its diagnostics, its answers to definition, references and hover at fixed places (in
# the broken file, from it into another and back), every place asserted to stand in the text, and
# the analysis its answers have accumulated, folded as sbt-teq folds them
# (tests/support/fold_analysis.py), are a fresh session's over the same texts. The good sources
# are tests/split/cycle with an extension, a match type, a `for`, a refinement and a block in b
# and their uses in a (tests/support/history), so that every recovery site of the parser's has a
# shape. Then two files broken and fixed in either order, with and without a full path at every
# second build (`TEQ_COMPACT_EVERY=2`); a file whose package blocks come and go, and a local class
# whose member class comes and goes, in a valid and in a broken text (tests/support/history).
hist=$work/hist
good=$work/hist-good
rm -rf "$good" && cp -r tests/split/cycle "$good"
cat tests/support/history/b-extra.scala >> "$good/b.scala"
cat tests/support/history/a-extra.scala >> "$good/a.scala"
HIST_FLAGS="$hist --index --own $hist"
CYCLE_SPOTS="b.scala|Color.Red.greet|6 a.scala|Mode.Fast.name|5 a.scala|BConst.value|0 a.scala|Mode.Fast.twice|10 a.scala|firsts(List|0"
HIST_SPOTS=$CYCLE_SPOTS
hbuild() { build "$@"; echo "$RESULT" >> "$work/hist.jsonl"; }
hstart() { start $HIST_FLAGS; echo "$RESULT" > "$work/hist.jsonl"; }
# hist_state <what>: the last answer and the queries at HIST_SPOTS (<file>|<text>|<offset into
# it>, the file under $hist, the text without spaces) against a fresh session's.
hist_state() {
  local what=$1 answered=$RESULT spot file rest text delta at q queries=() i=0 fresh want
  for spot in $HIST_SPOTS; do
    file=$hist/${spot%%|*} rest=${spot#*|}
    text=${rest%|*} delta=${rest##*|}
    at=$(grep -b -o -F -- "$text" "$file" | head -1 | cut -d: -f1)
    if [ -z "$at" ]; then
      bad "$what: no $text in ${file##*/} to ask at"
      continue
    fi
    at=$((at + delta))
    queries+=("definition $file $at" "references $file $at 1" "hover $file $at")
  done
  fresh=$({ printf '%s\n' "${queries[@]}"; echo quit; } | timeout 60 "$TEQ" compiler watch --check $HIST_FLAGS $ONE 2> /dev/null)
  echo "$fresh" | head -1 > "$work/hist-fresh.json"
  want=$(RESULT=$(head -1 "$work/hist-fresh.json"); echo "$(field ok),$(diagnostics)")
  expect "$what: the diagnostics of a fresh session" "$(field ok),$(diagnostics)" "$want"
  expect "$what: the analysis of a fresh session" "$(python3 tests/support/fold_analysis.py "$work/hist.jsonl" "$work/hist-fresh.json")" same
  for q in "${queries[@]}"; do
    i=$((i + 1))
    echo "$q" >&3
    answer
    expect "$what: the ${q%% *} at ${q#* } as in a fresh session" "$RESULT" "$(echo "$fresh" | sed -n "$((i + 1))p")"
  done
  RESULT=$answered
}
shapes=(
  'b.scala|s/def tag: String = "tagged"/def tag: String = ("tagged"/|a body with an unclosed parenthesis'
  'a.scala|s/def describe(c: Color): String = c match/def describe(c: Color): String c match/|a def missing its ='
  'b.scala|s/def other(n: Named): String =/def other(n: Named): =/|a def missing its result type'
  'a.scala|s/class Widget(val label: String) extends Tagged:/class Widget(val label: String extends Tagged:/|a broken class header'
  'a.scala|s/case Color.Red => "red: "/case => "red: "/|a case missing its pattern'
  'a.scala|s/^import b[.][*]$/import b./|an import cut after its dot'
  'b.scala|s/case Fast, Slow/case Fast Slow/|an enum missing a comma'
  'b.scala|s/^object BConst:/object BConst {/|a template colon made a brace'
  'a.scala|s/"aTop sees " + Mode.Slow.greet/"aTop sees " +/|an operator at the end of a line'
  'a.scala|s/^  def greet: String/ def greet: String/|a line indented one less'
  'b.scala|s/"tagged"/"tagged/|an unterminated string'
  'b.scala|s/^  def twice: String = m.shout + m.shout/  val twice: String = m.shout + m.shout/|an extension body with a val'
  'b.scala|s/^  val k = 1$/  export Mode.*; val k = 1/|an export in a block'
  'b.scala|s/^    x <- xs$/    x xs/|an enumerator missing its arrow'
  'b.scala|s/Named { def name: String }/Named { def name: String; 42 }/|a refinement with an expression'
  'b.scala|s/^  case Option\[t\] => t$/  case Option[t] t/|a match type case missing its arrow'
)
for shape in "${shapes[@]}"; do
  IFS='|' read -r file edit what <<< "$shape"
  rm -rf "$hist" && cp -r "$good" "$hist"
  hstart
  sed "$edit" "$good/$file" > "$hist/$file"
  if cmp -s "$hist/$file" "$good/$file"; then bad "history, $what: the edit changed nothing"; stop; continue; fi
  hbuild "$hist/$file"
  expect "history, $what: broken" "$(field ok)" false
  hist_state "history, $what: broken"
  cp "$good/$file" "$hist/$file"
  hbuild "$hist/$file"
  expect "history, $what: fixed" "$(field ok),$(field diagnostics)" 'true,[]'
  hist_state "history, $what: fixed"
  stop
done
broken_text() {
  case $1 in
    a.scala) sed 's/case Color.Red => "red: "/case => "red: "/' "$good/a.scala" ;;
    b.scala) sed 's/def tag: String = "tagged"/def tag: String = ("tagged"/' "$good/b.scala" ;;
  esac
}
for compact in 0 2; do
  for order in "a.scala b.scala" "b.scala a.scala"; do
    rm -rf "$hist" && cp -r "$good" "$hist"
    TEQ_COMPACT_EVERY=$compact start $HIST_FLAGS
    echo "$RESULT" > "$work/hist.jsonl"
    for file in $order; do
      broken_text $file > "$hist/$file"
      hbuild "$hist/$file"
      hist_state "history, $order in turn (compaction $compact): $file broken"
    done
    for file in $order; do
      cp "$good/$file" "$hist/$file"
      hbuild "$hist/$file"
      hist_state "history, $order in turn (compaction $compact): $file fixed"
    done
    expect "history, $order in turn (compaction $compact): fixed" "$(field ok),$(field diagnostics)" 'true,[]'
    stop
  done
done
# hist_steps <what> <good sources> <spots> <file> <text>...: a session over the good sources takes
# each text of <file> in turn, every state against a fresh session's.
hist_steps() {
  local what=$1 from=$2 file=$4 text step=0
  HIST_SPOTS=$3
  shift 4
  rm -rf "$hist" && cp -r "$from" "$hist"
  hstart
  for text in "$@"; do
    step=$((step + 1))
    cp "$text" "$hist/$file"
    hbuild "$hist/$file"
    hist_state "history, $what: step $step, ${text##*/}"
  done
  stop
  HIST_SPOTS=$CYCLE_SPOTS
}
blocks=tests/support/history/blocks
hist_steps "package blocks" "$blocks/good" "b.scala|A.x|0 c.scala|q.C|2 c.scala|p.A.x|2 a.scala|x:|0" a.scala \
  "$blocks/two.scala" "$blocks/two-broken.scala" "$blocks/two.scala" "$blocks/one.scala" "$blocks/two-broken.scala" "$blocks/one.scala"
local_class=tests/support/history/local
hist_steps "a local class's member" "$local_class/good" "b.scala|A.f|2 a.scala|f:|0" a.scala \
  "$local_class/ghost.scala" "$local_class/good.scala" "$local_class/ghost-broken.scala" "$local_class/good.scala"
for compact in 2; do
  TEQ_COMPACT_EVERY=$compact hist_steps "package blocks under compaction" "$blocks/good" "b.scala|A.x|0 c.scala|q.C|2 a.scala|x:|0" a.scala \
    "$blocks/two.scala" "$blocks/one.scala" "$blocks/two-broken.scala" "$blocks/one.scala"
done

# A large text with a parse error, typed as a body edit: what the session mapped for it is kept
# for the next build and given back by the one after.
src=$work/large && mkdir -p "$src" && cp tests/split/cycle/*.scala "$src"
start "$src"
{ head -c 2097152 /dev/zero | tr '\0' ' '; echo; sed 's/def tag: String = "tagged"/def tag: String = ("tagged"/' tests/split/cycle/b.scala; } > "$work/large.text"
text "$src/b.scala" "$work/large.text"
build "$src/b.scala"
expect "large text: the parse failure" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
echo stats >&3 && answer
if [ "$(field mapped_kept)" -gt 0 ] 2> /dev/null; then ok; else bad "large text: nothing kept after the build that freed it: $RESULT"; fi
for turn in 1 2; do
  sed "s/def tag: String = \"tagged\"/def tag: String = (\"tagged$turn\"/" tests/split/cycle/b.scala > "$work/short.text"
  text "$src/b.scala" "$work/short.text"
  build "$src/b.scala"
  expect "large text: the short text fails to parse too ($turn)" "$(field ok),$(count error)" 'false,1'
done
echo stats >&3 && answer
expect "large text: mappings kept after two more builds" "$(field mapped),$(field mapped_kept)" '0,0'
# A query and a build that finds nothing changed are no builds to count.
text "$src/b.scala" "$work/large.text"
build "$src/b.scala"
echo stats >&3 && answer
kept=$(field mapped_kept)
for turn in 1 2 3; do
  echo stats >&3 && answer
  build "$src/b.scala"
done
echo stats >&3 && answer
expect "large text: mappings kept through queries and builds of nothing" "$(field mapped_kept)" "$kept"
stop

# A profiled retype after a build through the fork (`TEQ_FORK=1`), a local class with an opaque
# type among the files: the merge of the forked build moved the class's mark to its merged id,
# and the retype's merge, which renumbers nothing, meets no worker's id of it; the same with the
# type store's overlays.
forked=$work/forked
for overlays in off shared; do
  rm -rf "$forked" && mkdir -p "$forked"
  cp tests/split/cycle/a.scala tests/split/cycle/b.scala "$forked/"
  cp tests/workers/opaque_local/main.scala "$forked/opaque_local.scala"
  TEQ_FORK=1 TEQ_TYPE_OVERLAYS=$overlays start "$forked" --profile
  expect "forked build then profiled retype ($overlays): first build" "$(field ok),$(field incremental)" 'true,false'
  sed 's/"tagged"/"changed"/' "$forked/b.scala" > "$work/forked.text"
  text "$forked/b.scala" "$work/forked.text"
  build "$forked/b.scala"
  expect "forked build then profiled retype ($overlays): the retype" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
  stop
done

# A session that takes the full path of its own accord against one that does not: the same
# edits and queries, and every answer the same but for how the build was made.
own_session() {
  local src=$1 out=$2
  : > "$out"
  note() { echo "$1: $RESULT" | sed -E 's/"ms":\{[^}]*\},?//; s/"incremental":(true|false),?//; s/"fallback":"[^"]*",?//; s/"retyped":\[[^]]*\],?//' | sed "s|$src|SRC|g" >> "$out"; }
  ask() {
    local at=$(grep -b -o 'BConst.value' tests/split/cycle/a.scala | head -1 | cut -d: -f1)
    local tag=$(grep -b -o 'def tag' tests/split/cycle/b.scala | head -1 | cut -d: -f1)
    for query in "definition $src/a.scala $at" "hover $src/a.scala $at" "references $src/b.scala $((tag + 4)) 1" "symbols $src/b.scala" "workspace-symbols Color"; do
      echo "$query" >&3
      answer
      note "$1, ${query%% *}"
    done
  }
  start "$src" --index
  note "first build"
  ask "first build"
  sed 's/"tagged"/"marked"/' "$src/b.scala" > "$work/own.text"
  text "$src/b.scala" "$work/own.text"
  build "$src/b.scala"
  note "an unsaved text"
  ask "an unsaved text"
  sed 's/def tag: String = "tagged"/def tag: String = 42/' "$src/b.scala" > "$work/own.text"
  text "$src/b.scala" "$work/own.text"
  build "$src/b.scala"
  note "an error"
  ask "an error"
  sed -i.bak 's/"aTop sees "/"aTop saw "/' "$src/a.scala"
  build "$src/a.scala"
  note "another file under the error"
  sed 's/"tagged"/"marked again"/' "$src/b.scala" > "$work/own.text"
  text "$src/b.scala" "$work/own.text"
  build "$src/b.scala"
  note "the fix"
  ask "the fix"
  rm "$src/main.scala"
  build "$src/main.scala"
  note "a file deleted"
  ask "a file deleted"
  withdraw "$src/b.scala"
  build "$src/b.scala"
  note "the text withdrawn"
  sed -i.bak 's/"aTop saw "/"aTop sees again "/' "$src/a.scala"
  build "$src/a.scala"
  note "a body on disk"
  ask "a body on disk"
  stop
}
for kind in plain own; do
  mkdir -p "$work/$kind" && cp tests/split/cycle/*.scala "$work/$kind"
done
own_session "$work/plain" "$work/plain.answers"
TEQ_COMPACT_EVERY=2 own_session "$work/own" "$work/own.answers"
expect "own full build: the answers of a session that never takes it" "$(cksum < "$work/own.answers")" "$(cksum < "$work/plain.answers")"
if [ "$(grep -c . "$work/own.answers")" -ge 30 ]; then ok; else bad "own full build: the answers compared: $(grep -c . "$work/own.answers")"; fi
if ! cmp -s "$work/own.answers" "$work/plain.answers"; then diff "$work/plain.answers" "$work/own.answers" | head -6 | cut -c1-300; fi

# --- A query that demands every std file -------------------------------------------------------
# The references of the std's `List`, which demand every std file,
# keep what they typed as the session's and not as dead records: the next body edit
# is typed incrementally, where the demand's growth over a small program's last full build would
# take the full path for the session's memory ("Watch mode"). Its documents go under a cache of
# its own.
lean=$work/lean-demand
mkdir -p "$lean"
printf 'object T:\n  val xs = List(1, 2)\n  def size: Int = xs.length\n' > "$lean/T.scala"
export TEQ_CACHE_DIR=$work/cache
start "$lean" --index
at=$(grep -b -o -F 'List(' "$lean/T.scala" | head -1 | cut -d: -f1)
echo "references $lean/T.scala $at 0" >&3
answer
case $RESULT in
  *'/attached/std-'*'/collections.scala"'*) ok ;;
  *) bad "the references of the std's List hold no use in collections.scala: ${RESULT:0:200}" ;;
esac
printf 'object T:\n  val xs = List(1, 2)\n  def size: Int = xs.length + 1\n' > "$lean/T.scala"
build "$lean/T.scala"
expect "a body edit after a demand of every std file: typed incrementally" "$(field ok),$(field incremental)" "true,true"
stop
unset TEQ_CACHE_DIR

# --- An inferred signature changed while the program has errors ---------------------------------
# A changed inferred signature is decided by its own definition. Where inferring it reported no
# error, the build takes the full path at once, whatever errors stand elsewhere, in another file
# or in its own: the error the change causes elsewhere appears with it. Where inferring it
# reported an error, the old signature is put back (the rest of the program was typed against
# it) and the file stays pending until the edit that fixes the body, which settles the
# signature through the comparison and the full path.
rm -rf "$src"
mkdir -p "$src"
printf 'object A { val x = 1 }\n' > "$src/A.scala"
printf 'object B { val y: Int = A.x }\n' > "$src/B.scala"
printf 'object C { val bad: Int = "s" }\n' > "$src/C.scala"
start "$src"
expect "pending: first build" "$(field ok),$(count error)" 'false,1'
# A body edit that keeps the inferred type is typed incrementally under the error elsewhere.
printf 'object A { val x = 3 }\n' > "$work/A-three.scala"
text "$src/A.scala" "$work/A-three.scala"
build "$src/A.scala"
expect "pending: an unchanged type stays incremental" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$src"'/C.scala"' && ! has '"file":"'"$src"'/B.scala"' && ! has "the inferred type"; then ok; else bad "pending: C's error only, incrementally: $RESULT"; fi
printf 'object A { val x = "s" }\n' > "$work/A-string.scala"
text "$src/A.scala" "$work/A-string.scala"
build "$src/A.scala"
expect "pending: signature changed under an error elsewhere" "$(field ok),$(field incremental),$(count error)" 'false,false,2'
if has '"file":"'"$src"'/B.scala","line":1' && has '"file":"'"$src"'/C.scala"' && has "the inferred type of x changed"; then ok; else bad "pending: B's and C's errors after the full path: $RESULT"; fi
printf 'object C { val bad: Int = 2 }\n' > "$work/C-fixed.scala"
text "$src/C.scala" "$work/C-fixed.scala"
build "$src/C.scala"
expect "pending: the error elsewhere fixed" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$src"'/B.scala","line":1'; then ok; else bad "pending: B's error stays: $RESULT"; fi
printf 'object A { val x = 1 }\n' > "$work/A-int.scala"
text "$src/A.scala" "$work/A-int.scala"
build "$src/A.scala"
expect "pending: the signature back" "$(field ok),$(field incremental),$(field diagnostics)" 'true,false,[]'
stop
rm -rf "$src"
mkdir -p "$src"
printf 'object A { val x = 1; val bad: Int = "s" }\n' > "$src/A.scala"
printf 'object B { val y: Int = A.x }\n' > "$src/B.scala"
start "$src"
expect "pending in the edited file: first build" "$(field ok),$(count error)" 'false,1'
# The error is in the body that infers the type: the signature is put back.
printf 'object A { val x = { val own: Int = "s"; "s" }; val bad: Int = "s" }\n' > "$work/A-own.scala"
text "$src/A.scala" "$work/A-own.scala"
build "$src/A.scala"
expect "pending in the edited file: signature changed under its body's error" "$(field ok),$(field incremental),$(count error)" 'false,true,2'
if has '"file":"'"$src"'/A.scala"' && ! has '"file":"'"$src"'/B.scala"'; then ok; else bad "pending in the edited file: only A's errors expected: $RESULT"; fi
printf 'object A { val x = { val own: Int = 2; "s" }; val bad: Int = "s" }\n' > "$work/A-fixed.scala"
text "$src/A.scala" "$work/A-fixed.scala"
build "$src/A.scala"
expect "pending in the edited file: the fix settles the signature" "$(field ok),$(field incremental),$(count error)" 'false,false,2'
if has '"file":"'"$src"'/B.scala","line":1' && has "inferred type of x changed"; then ok; else bad "pending in the edited file: B's error after the full build: $RESULT"; fi
stop
# The error is in another definition of the edited file: the body that infers the type is
# clean, and its new type takes the full path at once.
rm -rf "$src"
mkdir -p "$src"
printf 'object A { val x = 1; val bad: Int = "s" }\n' > "$src/A.scala"
printf 'object B { val y: Int = A.x }\n' > "$src/B.scala"
start "$src"
printf 'object A { val x = "s"; val bad: Int = "s" }\n' > "$work/A-both.scala"
text "$src/A.scala" "$work/A-both.scala"
build "$src/A.scala"
expect "beside an error of its file: a clean body's new type takes the full path" "$(field ok),$(field incremental),$(count error)" 'false,false,2'
if has '"file":"'"$src"'/B.scala","line":1' && has "inferred type of x changed"; then ok; else bad "beside an error of its file: B's error after the full build: $RESULT"; fi
stop
# A file pending under its own error does not hold back another file's changed signature.
rm -rf "$src"
mkdir -p "$src"
printf 'object A { val x = 1 }\n' > "$src/A.scala"
printf 'object B { val y: Int = A.x }\n' > "$src/B.scala"
printf 'object C { val c = 1 }\n' > "$src/C.scala"
start "$src"
expect "pending beside: first build" "$(field ok),$(count error)" 'true,0'
printf 'object C { val c = { val own: Int = "s"; "s" } }\n' > "$work/C-pending.scala"
text "$src/C.scala" "$work/C-pending.scala"
build "$src/C.scala"
expect "pending beside: C pending under its body's error" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
text "$src/A.scala" "$work/A-string.scala"
build "$src/A.scala"
expect "pending beside: A's changed signature takes the full path" "$(field ok),$(field incremental),$(count error)" 'false,false,2'
if has '"file":"'"$src"'/B.scala"' && has "the inferred type of x changed"; then ok; else bad "pending beside: B's error after the full path: $RESULT"; fi
stop
# A pending file is retried from its current text: a malformed text of it is applied, and the
# type its recovered tree gives x (String), inferred without an error, takes the full path; its
# parse error stays through the builds of other files.
rm -rf "$src"
mkdir -p "$src"
printf 'object A { val x = 1; val bad: Int = "s" }\n' > "$src/A.scala"
printf 'object B { val y: Int = A.x }\n' > "$src/B.scala"
start "$src"
text "$src/A.scala" "$work/A-own.scala"
build "$src/A.scala"
expect "pending text: A pending" "$(field ok),$(field incremental),$(count error)" 'false,true,2'
printf 'object A { val x = ("s"; val bad: Int = "s" }\n' > "$work/A-malformed.scala"
text "$src/A.scala" "$work/A-malformed.scala"
build "$src/A.scala"
expect "pending text: A malformed" "$(field ok),$(field fallback),$(count error)" 'false,"the inferred type of x changed",3'
if has "')', found" && has '"file":"'"$src"'/B.scala","line":1,"col":25' && has '"message":"type mismatch: found String, required Int"'; then ok; else bad "pending text: A's parse error, its own error and B's: $RESULT"; fi
printf 'object B { val y: Int = A.x + 0 }\n' > "$work/B-plus.scala"
text "$src/B.scala" "$work/B-plus.scala"
build "$src/B.scala"
expect "pending text: B's build keeps A's parse failure" "$(field ok),$(field incremental),$(count error)" 'false,true,3'
if has "')', found" && has '"retyped":["'"$src"'/B.scala"]'; then ok; else bad "pending text: A's parse error with B retyped: $RESULT"; fi
# A malformed text of A with a valid edit of B in one build; A's text then withdrawn to a disk
# text that is neither the program's nor the malformed one: the next build of B reads A, types
# it, and its parse error goes, x's type back at Int taking the full path.
text "$src/A.scala" "$work/A-malformed.scala"
printf 'object B { val y: Int = A.x + 1 }\n' > "$work/B-plus-one.scala"
text "$src/B.scala" "$work/B-plus-one.scala"
build "$src/A.scala" "$src/B.scala"
expect "pending text: a malformed A with a valid B edit" "$(field ok),$(field incremental),$(count error)" 'false,true,3'
if has "')', found" && has '"retyped":["'"$src"'/B.scala"]'; then ok; else bad "pending text: B applied beside A's failure: $RESULT"; fi
withdraw "$src/A.scala"
build "$src/B.scala"
expect "pending text: A's disk text read with B's build" "$(field ok),$(field fallback),$(count error)" 'false,"the inferred type of x changed",1'
if ! has "')', found" && has '"file":"'"$src"'/A.scala","line":1,"col":38' && has '"message":"type mismatch: found String, required Int"'; then ok; else bad "pending text: A read from disk, its parse error gone: $RESULT"; fi
stop
# Parse errors survive the full path with their files' trees, and the parse errors of the files
# that remain are kept when one of them is removed; a fix that comes with a file added back is
# read there.
rm -rf "$src"
mkdir -p "$src"
printf 'object A { val x = 1 }\n' > "$src/A.scala"
printf 'object B { val y: Int = A.x }\n' > "$src/B.scala"
printf 'object C { val c = 1 }\n' > "$src/C.scala"
start "$src"
expect "failures across the full path: first build" "$(field ok),$(field diagnostics)" 'true,[]'
printf 'object A { val x = (1 }\n' > "$src/A.scala"
build "$src/A.scala"
expect "failures across the full path: A fails on disk" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
printf 'object C { val c = (1 }\n' > "$src/C.scala"
build "$src/C.scala"
expect "failures across the full path: C fails too" "$(field ok),$(count error)" 'false,2'
printf 'object B { val y: Int = A.x; val z = 2 }\n' > "$src/B.scala"
build "$src/B.scala"
expect "failures across the full path: a definition added elsewhere" "$(field ok),$(field incremental),$(count error)" 'false,false,2'
if has '"file":"'"$src"'/A.scala"' && has '"file":"'"$src"'/C.scala"'; then ok; else bad "failures across the full path: A's and C's errors after the full build: $RESULT"; fi
rm "$src/A.scala"
build "$src/A.scala"
# C keeps its tree and its parse error, and B misses A.
expect "failures across the full path: A removed, C's failure kept" "$(field ok),$(field incremental),$(count error)" 'false,false,2'
if has '"file":"'"$src"'/C.scala"' && has '"file":"'"$src"'/B.scala"' && ! has '"file":"'"$src"'/A.scala"'; then ok; else bad "failures across the full path: C's parse errors and B's missing A: $RESULT"; fi
printf 'object C { val c = 1 }\n' > "$src/C.scala"
printf 'object A { val x = 1 }\n' > "$src/A.scala"
build "$src/A.scala" "$src/C.scala"
expect "failures across the full path: all fixed" "$(field ok),$(field diagnostics)" 'true,[]'
stop

# A producer and its consumer edited in one build while the producer has an error of its own:
# the consumer was typed against the signature that is then put back, so it is typed again
# with the next build, which reports its error.
rm -rf "$src"
mkdir -p "$src"
printf 'object A { val x = 1; val bad: Int = 2 }\n' > "$src/A.scala"
printf 'object B { val y: Int = A.x }\n' > "$src/B.scala"
start "$src"
expect "batch: first build" "$(field ok),$(field diagnostics)" 'true,[]'
printf 'object A { val x = { val own: Int = "bad"; "s" }; val bad: Int = 2 }\n' > "$work/A-batch.scala"
printf 'object B { val y: Int = A.x.length }\n' > "$work/B-batch.scala"
text "$src/A.scala" "$work/A-batch.scala"
text "$src/B.scala" "$work/B-batch.scala"
build "$src/A.scala" "$src/B.scala"
expect "batch: the producer's own error" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$src"'/A.scala"' && ! has '"file":"'"$src"'/B.scala"'; then ok; else bad "batch: A's error alone: $RESULT"; fi
printf 'object A { val x = 1; val bad: Int = 2 }\n' > "$work/A-orig.scala"
text "$src/A.scala" "$work/A-orig.scala"
build "$src/A.scala"
expect "batch: the producer restored, the consumer typed again" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$src"'/B.scala"' && has "value length is not a member of Int"; then ok; else bad "batch: B's error against Int: $RESULT"; fi
withdraw "$src/B.scala"
build "$src/B.scala"
expect "batch: the consumer restored" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
# The same batch, then the producer restored while the consumer's text fails to parse: the
# consumer's text is applied and typed against the restored signature, its parse error beside
# the error of its own, and once its text is back it is typed again.
text "$src/A.scala" "$work/A-batch.scala"
text "$src/B.scala" "$work/B-batch.scala"
build "$src/A.scala" "$src/B.scala"
expect "batch, malformed consumer: the producer's own error" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
printf 'object B { val y: Int = (A.x.length }\n' > "$work/B-malformed.scala"
text "$src/A.scala" "$work/A-orig.scala"
text "$src/B.scala" "$work/B-malformed.scala"
build "$src/A.scala" "$src/B.scala"
expect "batch, malformed consumer: the producer restored" "$(field ok),$(field incremental)" 'false,true'
if has "')', found" && has "value length is not a member of Int"; then ok; else bad "batch, malformed consumer: B's parse error and its own: $RESULT"; fi
text "$src/B.scala" "$work/B-batch.scala"
build "$src/B.scala"
expect "batch, malformed consumer: the consumer's text back, typed again" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has "value length is not a member of Int"; then ok; else bad "batch, malformed consumer: B's error against Int: $RESULT"; fi
stop
# --- An inline method's body under the definition check --------------------------
# An error in an inline body is reported at the definition, called or not, and once: a call of
# the failed body is not expanded again. A retype of the defining file drops the stored body and
# checks it again, and the file that expands the method is typed again with it: the error goes
# with the edit that fixes it and comes back with the edit that brings it back, as in a fresh
# session.
rm -rf "$src"
mkdir -p "$src"
printf 'object Lib:\n  inline def twice(x: Int): Int = x * 2\n  inline def unused(x: Int): Int = x + 1\n' > "$src/Lib.scala"
printf 'object Use:\n  def four: Int = Lib.twice(2)\n' > "$src/Use.scala"
start "$src"
expect "inline definition: first build" "$(field ok),$(field diagnostics)" 'true,[]'
printf 'object Lib:\n  inline def twice(x: Int): Int = x * 2\n  inline def unused(x: Int): Int = x.nosuch\n' > "$work/Lib-unused.scala"
cp "$work/Lib-unused.scala" "$src/Lib.scala"
build "$src/Lib.scala"
expect "inline definition: an uncalled body's error" "$(field ok),$(count error)" 'false,1'
if has '"file":"'"$src"'/Lib.scala","line":3' && has "value nosuch is not a member of Int"; then ok; else bad "inline definition: the error at the definition: $RESULT"; fi
as_fresh "inline definition: an uncalled body's error" "$src"
printf 'object Lib:\n  inline def twice(x: Int): Int = x * 2\n  inline def unused(x: Int): Int = x + 1\n' > "$work/Lib-fixed.scala"
cp "$work/Lib-fixed.scala" "$src/Lib.scala"
build "$src/Lib.scala"
expect "inline definition: the body fixed" "$(field ok),$(field diagnostics)" 'true,[]'
printf 'object Lib:\n  inline def twice(x: Int): Int = x.nosuch\n  inline def unused(x: Int): Int = x + 1\n' > "$work/Lib-twice.scala"
cp "$work/Lib-twice.scala" "$src/Lib.scala"
build "$src/Lib.scala"
expect "inline definition: a called body's error, at the definition alone" "$(field ok),$(count error)" 'false,1'
if has '"file":"'"$src"'/Lib.scala","line":2' && ! has '"file":"'"$src"'/Use.scala","line":2'; then ok; else bad "inline definition: the error at the definition, none at the call: $RESULT"; fi
as_fresh "inline definition: a called body's error" "$src"
cp "$work/Lib-fixed.scala" "$src/Lib.scala"
build "$src/Lib.scala"
expect "inline definition: the called body fixed" "$(field ok),$(field diagnostics)" 'true,[]'
stop
# The definition's diagnostics are its file's typing's (tests/support/history/inline): a session
# that starts from a failed definition (`Lib.f` is `x.noSuch`) beside an unchanged caller, whose
# call is the plain call yet depends on the definition, takes the fix (`x + 1`) and then
# `compiletime.error("boom")`, which a fresh session reports at the caller; an unrelated edit of
# the caller while the definition stays broken leaves the error the definition's, reported once;
# a bound the body violates comes and goes at the definition. Every state's diagnostics,
# analysis and answers against a fresh session's (`hist_steps`), and the caller typed again
# where the definition changes.
inl=tests/support/history/inline
INLINE_SPOTS="Use.scala|Lib.f(1)|4 Use.scala|m:|0 Lib.scala|f(x|0"
hist_steps "a failed inline definition fixed, then an error" "$inl/broken-start" "$INLINE_SPOTS" Lib.scala \
  "$inl/fixed.scala" "$inl/boom.scala" "$inl/fixed.scala" "$inl/broken.scala" "$inl/fixed.scala"
hist_steps "a caller edited while its inline definition stays broken" "$inl/broken-start" "$INLINE_SPOTS" Use.scala \
  "$inl/use-edit.scala" "$inl/use.scala"
hist_steps "an inline body's bound violated and kept" "$inl/good" "$INLINE_SPOTS" Lib.scala \
  "$inl/bounds.scala" "$inl/fixed.scala" "$inl/bounds.scala"
rm -rf "$src" && cp -r "$inl/broken-start" "$src"
start "$src"
expect "inline definition fixed: the failed start" "$(field ok),$(count error)" 'false,1'
cp "$inl/boom.scala" "$src/Lib.scala"
build "$src/Lib.scala"
if has '"file":"'"$src"'/Use.scala","line":2' && has 'boom'; then ok; else bad "inline definition fixed: the caller typed again with the definition, its error at the call: $RESULT"; fi
case "$(field retyped)" in *"$src/Use.scala"*) ok ;; *) bad "inline definition fixed: the caller among the files typed again: $(field retyped)" ;; esac
stop
# A local inline method's stored body is indexed once per record (`InlineState::stored_indexes`):
# each edit that types its file again makes the method a symbol of its own, whose index goes with
# the old record, so that three edits leave one.
rm -rf "$src" && mkdir -p "$src"
printf 'object Host:\n  def run(): Int =\n    inline def g(x: Int): Int = x + 1\n    g(1)\n' > "$src/Host.scala"
start "$src"
expect "a local inline method: the first build" "$(field ok),$(field diagnostics)" 'true,[]'
for n in 2 3 4; do
  printf 'object Host:\n  def run(): Int =\n    inline def g(x: Int): Int = x + %s\n    g(1)\n' "$n" > "$src/Host.scala"
  build "$src/Host.scala"
done
expect "a local inline method: the third edit" "$(field ok),$(field diagnostics)" 'true,[]'
echo stats >&3 && answer
expect "a local inline method edited three times: one index of its body" "$(field inline_indexes)" '1'
stop
# A text withdrawn after its file was deleted, then the file recreated: the disk text counts.
rm -rf "$src"
cp -r tests/split/cycle "$src"
start "$src"
printf 'package d\nval dTop: Int = 1\n' > "$src/d.scala"
build "$src/d.scala"
expect "withdraw after delete: file added" "$(field ok),$(field diagnostics)" 'true,[]'
printf 'package d\nval dTop: Int = "s"\n' > "$work/d-broken.scala"
text "$src/d.scala" "$work/d-broken.scala"
build "$src/d.scala"
expect "withdraw after delete: the text's error" "$(field ok),$(count error)" 'false,1'
rm "$src/d.scala"
build "$src/d.scala"
expect "withdraw after delete: file removed" "$(field ok),$(field fallback),$(field diagnostics)" 'true,"files added or removed",[]'
# Another text for the deleted file keeps the key its first text had, so the withdrawal finds it.
text "$src/d.scala" "$work/d-broken.scala"
withdraw "$src/d.scala"
printf 'package d\nval dTop: Int = 2\n' > "$src/d.scala"
build "$src/d.scala"
expect "withdraw after delete: the recreated file's own text" "$(field ok),$(field fallback),$(field diagnostics)" 'true,"files added or removed",[]'
stop
# Several @main methods are no error in a check session: no entry point is chosen.
rm -rf "$src"
mkdir -p "$src"
printf '@main def one(): Unit = ()\n@main def two(): Unit = ()\n' > "$src/mains.scala"
start "$src"
expect "several @main: no entry point chosen" "$(field ok),$(field diagnostics)" 'true,[]'
stop

# --- Inline bodies ------------------------------------------------------------------------------
# An edit of an inline body types the files that expanded it again: the error it makes in an
# unchanged caller's `inline if` branch is reported at once, on an incremental build, and an
# unchanged retry after a type error kept in a method with a declared type keeps the error.
# `pick` is transparent: its call is typed by the branch it expands to, as scalac types it (a
# plain inline method's call has its inferred result, `Int | String`, `Namer.inferredResultType`).
rm -rf "$src"
cp -r tests/split/inline "$src"
sed -i.bak 's/def pick: Int = if flag then 1 else 2/transparent inline def pick = inline if flag then 1 else "two"\
  val chosen: Int = pick/' "$src/use.scala"
start "$src"
expect "inline: first build" "$(field ok),$(field diagnostics)" 'true,[]'
sed -i.bak 's/inline def flag: Boolean = true/inline def flag: Boolean = false/' "$src/lib.scala"
build "$src/lib.scala"
expect "inline: the caller's branch changes type" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
if has '"file":"'"$src"'/use.scala","line":8' && has '"retyped":["'"$src"'/main.scala","'"$src"'/use.scala","'"$src"'/lib.scala"]'; then ok; else bad "inline: the error in the unchanged caller and its own caller, retyped with the body: $RESULT"; fi
build "$src/lib.scala"
expect "inline: the error kept on an unchanged retry" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
sed -i.bak 's/inline def flag: Boolean = false/inline def flag: Boolean = true/' "$src/lib.scala"
build "$src/lib.scala"
expect "inline: fix" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
stop

# --- The analysis of the owned files ------------------------------------------------------------
# Under `--own` a check session answers the classes of the owned files it typed anew (sbt-teq's
# `compile` of a Scala.js project turns them into zinc's analysis): the first build every owned
# file, a body edit the file retyped, and an edit of a file of another root none; a file of
# package blocks once, and the classes of a package object under its name, as scalac's are.
own=$work/own
mkdir -p "$own/main/p" "$own/test/p"
cat > "$own/main/p/Base.scala" <<'EOS'
package p

trait Base:
  def n: Int = 1

class Marker extends scala.annotation.StaticAnnotation

object Tool:
  def main(args: Array[String]): Unit = println("tool")
EOS
cat > "$own/test/p/Suite.scala" <<'EOS'
package p

@Marker
class Suite extends Base:
  def m: Int = n + 1

object Main:
  def main(args: Array[String]): Unit = ()
EOS
cat > "$own/test/p/Names.scala" <<'EOS'
package p

class Pair
object Pair:
  class Inner

enum Mode:
  case On
  case Level(n: Int)

object Odd$
EOS
cat > "$own/test/p/package.scala" <<'EOS'
package object p extends p.Base {
  class InObject extends Base
  object Helper
}
EOS
cat > "$own/test/p/Blocks.scala" <<'EOS'
package p {
  class InBlock extends Base:
    def k: Int = 1
}
EOS
analysis() { echo "$RESULT" | python3 -c 'import json, sys; d = json.loads(sys.stdin.read()); print(";".join(f["file"].rsplit("/", 1)[-1] + ":" + ",".join(c["name"] + "/" + c["kind"] + "/" + "+".join(c["bases"][:1]) + ("/main" if c["main"] else "") + ("/" + "+".join(c["annotations"]) if c["annotations"] else "") for c in f["classes"]) for f in d.get("analysis", [])) or "-")'; }
start "$own/main" "$own/test" --own "$own/test"
binaries() { echo "$RESULT" | python3 -c 'import json, sys; d = json.loads(sys.stdin.read()); print(" ".join(sorted(b for f in d.get("analysis", []) for c in f["classes"] for b in c["files"])))'; }
expect "own: first build" "$(field ok),$(analysis | tr ';' '\n' | grep Suite.scala)" 'true,Suite.scala:p.Suite/class/p.Base/p.Marker,p.Main/object//main'
expect "own: the binary names of the classes" "$(binaries)" 'p/InBlock p/Main p/Main$ p/Mode p/Mode$Level p/Odd$ p/Odd$$ p/Pair p/Pair$ p/Pair$Inner p/Suite p/package p/package$ p/package$Helper$ p/package$InObject'
# A class of a package object is a member of the object `p.package`, as scalac names it, and not
# top-level: discovery leaves it out. The object itself, which its parents make, is top-level.
tops() { echo "$RESULT" | python3 -c 'import json, sys; d = json.loads(sys.stdin.read()); print(" ".join(c["name"] + ("/top" if c["top"] else "") for f in d.get("analysis", []) if f["file"].endswith("package.scala") for c in f["classes"]))'; }
expect "own: the classes of a package object" "$(tops)" 'p.package/top p.package$.InObject p.package$.Helper'
expect "own: a file of package blocks answered once" "$(analysis | tr ';' '\n' | grep Blocks.scala)" 'Blocks.scala:p.InBlock/class/p.Base'
sed -i.bak 's/def k: Int = 1/def k: Int = 2/' "$own/test/p/Blocks.scala"
build "$own/test/p/Blocks.scala"
expect "own: a file of package blocks retyped, answered once" "$(field incremental),$(analysis)" 'true,Blocks.scala:p.InBlock/class/p.Base'
sed -i.bak 's/def m: Int = n + 1/def m: Int = n + 2/' "$own/test/p/Suite.scala"
build "$own/test/p/Suite.scala"
expect "own: a body edit" "$(field incremental),$(analysis)" 'true,Suite.scala:p.Suite/class/p.Base/p.Marker,p.Main/object//main'
sed -i.bak 's/def n: Int = 1/def n: Int = 3/' "$own/main/p/Base.scala"
build "$own/main/p/Base.scala"
expect "own: an edit outside the owned roots" "$(field incremental),$(analysis)" 'true,-'
stop

# --- A JVM program ------------------------------------------------------------------------------
rm -rf "$src"
cp -r tests/split/classpath "$src"
if "$TEQ" compiler check "$src" --std=scala-library > "$work/jar.log" 2>&1; then
  start "$src" --target jvm
  expect "jvm: first build" "$(field ok),$(field incremental),$(field diagnostics)" 'true,false,[]'
  sed -i.bak 's/mkString("\[", ",", "\]")/mkString("[", 1, "]")/' "$src/util.scala"
  build "$src/util.scala"
  expect "jvm: type error" "$(field ok),$(field incremental),$(count error)" 'false,true,1'
  if has '"file":"'"$src"'/util.scala","line":'; then ok; else bad "jvm: type error diagnostic: $RESULT"; fi
  sed -i.bak 's/mkString("\[", 1, "\]")/mkString("[", ",", "]")/' "$src/util.scala"
  build "$src/util.scala"
  expect "jvm: fix" "$(field ok),$(field incremental),$(field diagnostics)" 'true,true,[]'
  stop
else
  echo "skip the jvm session: $(tail -1 "$work/jar.log")"
fi

# --- Unused imports without the index -------------------------------------------------------------
# A session under --wunused imports and --werror, no --index: a body's retype keeps the use a
# signature made of an import (resolved once, its mark moved by the shape comparison), and an edit
# taking a body's only use away makes its import a warning, which --werror fails.
unused_imports_session() {
  local src=$work/unused
  rm -rf "$src"
  mkdir -p "$src"
  printf 'object Lib { class T; val u = 1 }\nimport Lib.T\nimport Lib.u\nobject Use {\n  def f(t: T): Int = 1\n  def g: Int = u\n}\n' > "$src/a.scala"
  start "$src" --wunused imports --werror
  expect "unused imports: the first build" "$(field ok),$(count warning)" 'true,0'
  sed -i.bak 's/def g: Int = u/def g: Int = u + 1/' "$src/a.scala"
  build "$src/a.scala"
  expect "unused imports: a body's retype keeps the signature's use" "$(field ok),$(field incremental),$(count warning)" 'true,true,0'
  sed -i.bak 's/def g: Int = u + 1/def g: Int = 3/' "$src/a.scala"
  build "$src/a.scala"
  expect "unused imports: the body's only use taken away" "$(field ok),$(field incremental),$(count warning)" 'false,true,1'
  stop
}
unused_imports_session

# --- One file after another ---------------------------------------------------------------------
retype_entry
retype_entry --index
retype_reads
retype_reads --index
retype_errors
retype_errors --index
retype_facade
retype_facade --index
retype_facade_errors
retype_facade_errors --index
retype_kept
retype_kept --index
retype_roots
retype_roots --index
retype_state
retype_state --index
retype_state_pair
retype_cacheable
retype_cacheable --index
retype_jar_state
retype_jar_state --index
retype_sweep

sessions_forked check-watch
echo "$pass passed, $fail failed"
[ $fail = 0 ]
