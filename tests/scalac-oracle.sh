#!/bin/bash
# The scalac oracle of the gate's app-scalac line (bench/app/diagnostics.py) over small programs, both compilers run:
# tests/errors' numeric_narrowing.scala has its Float assignment, which scalac accepts and teq rejects, as a teq-only
# difference among its others, and duplicates.scala's two definitions both compilers name alike agree; a known list
# drafted from exhaustivity.scala permits its run, and fails it with a stale line, another count or the same place's
# other witness (tests/scalac-oracle/witness-a.scala, then witness-b.scala as the same file), and a duplicate line or
# a tag without its cause is refused; two warnings on one line agree, and the same answer with one of them dropped is
# a difference (a multiset, not a set); a place is a line and a column, teq at scalac's span start agreeing
# (tests/scalac-oracle/span.scala) and at another column of the point's line an anchor difference; a wrapped required
# type is the payload whole (wrapped.scala); a string literal type naming the checkout's path is not rewritten, a
# symbol with two spaces in it is not another with one (symbol.scala, its answer renamed), nor a type whose name
# begins with a space one without it (leading.scala, the same); an answer that is missing or holds fewer diagnostics
# than teq printed, an exit 1 its answer does not explain (with a known line for its exit too), a crash and a
# scala-cli that gives no summary fail the run rather than pass it as empty; the application mode and api-check.sh's
# tests take lists whose rows are source files; and the options: a suppression (-Wconf) is left out of the
# conformance pass and kept in the production pass, -Werror against --werror agrees there, -Werror against no
# --werror is a production-exit difference, and so is -Werror turned off again (-Werror:false) against --werror.
# Needs scala-cli (SCALA_CLI names another; a skip without it); TEQ names the binary (default: the repository's
# release build).
# tests/scalac-oracle.sh [<out dir>]: every run, answer, fixture, known list, log and scratch file (api-check.sh's,
# through TMPDIR) goes under <out dir>, which must be new or empty (the gate gives one under its attempt's directory;
# a directory that holds files is refused, and what it holds stays, which the last check runs), else under a new
# directory of $TMPDIR, which the first line names.
caller=$PWD
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
command -v "${SCALA_CLI:-scala-cli}" > /dev/null || { echo "skip: scala-cli not found"; exit 0; }
# The application's known list and draft, which the gate's line passes on, are none of these runs'.
unset SCALAC_KNOWN SCALAC_DRAFT
if [ $# -gt 0 ]; then
  case $1 in /*) out=$1 ;; *) out=$caller/$1 ;; esac
  [ -z "$(ls -A "$out" 2> /dev/null)" ] || { echo "scalac oracle: $out holds files already; give a new directory" >&2; exit 2; }
  mkdir -p "$out" || exit 2
else
  out=$(mktemp -d) || exit 2
fi
echo "scalac oracle: the runs under $out"
# shown <file>: a file as a record names it, relative to the checkout or absolute outside it.
shown() { python3 -c 'import os, sys; r = os.path.relpath(sys.argv[1]); print(sys.argv[1] if r.startswith("..") else r)' "$1"; }
stand_in=$PWD/tests/scalac-oracle/stand-in.sh
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL $1 (see $out/$2.log)"; fail=$((fail + 1)); }
# oracle <name> <teq> <arguments>: the oracle's files mode into $out/<name>, its output in <name>.log, its exit in $code.
oracle() {
  local name=$1 teq=$2
  shift 2
  timeout 600 python3 bench/app/diagnostics.py files "$teq" "$out/$name" "$@" > "$out/$name.log" 2>&1
  code=$?
}
# expect <name> <exit> <regex>...: the run exited so and its output has every line.
expect() {
  local name=$1 want=$2 r
  shift 2
  [ "$code" = "$want" ] || { bad "$name: exit $code, not $want" "$name"; return; }
  for r in "$@"; do grep -q -E -- "$r" "$out/$name.log" "$out/$name/differences.txt" 2> /dev/null || { bad "$name: no line /$r/" "$name"; return; }
  done
  ok
}
hex='[0-9a-f]{16}'

oracle numeric "$TEQ" tests/errors/numeric_narrowing.scala
expect numeric 1 "^files teq-only tests/errors/numeric_narrowing.scala:9 error - $hex 1 " \
  "^files payload tests/errors/numeric_narrowing.scala:8 error $hex $hex 1 " "4 differences, 0 listed, 4 not listed"

oracle duplicates "$TEQ" tests/errors/duplicates.scala
expect duplicates 1 "2 differences"
if grep -q -E ':1[46] ' "$out/duplicates/differences.txt"; then bad "duplicates: lines 14 and 16 differ" duplicates; else ok; fi

SCALAC_DRAFT=$out/exhaustivity.draft oracle draft "$TEQ" tests/errors/exhaustivity.scala
expect draft 1 "^files payload tests/errors/exhaustivity.scala:33 warning $hex $hex 1 \\?" "1 differences, 0 listed, 1 not listed"
known=$out/exhaustivity.known
{ echo "cause witness  scalac writes a type test's witness as _: T, teq as T, tests/scalac-oracle/witness-a.scala"; sed 's/ ? / witness /' "$out/exhaustivity.draft"; } > "$known"
oracle listed "$TEQ" --known "$known" tests/errors/exhaustivity.scala
expect listed 0 "1 differences, 1 listed, 0 not listed, 0 stale, 0 counted otherwise; pass" "listed 1, witness: "
{ cat "$known"; sed 's/:33 /:34 /; s/ ? / witness /' "$out/exhaustivity.draft"; } > "$out/stale.known"
oracle stale "$TEQ" --known "$out/stale.known" tests/errors/exhaustivity.scala
expect stale 1 "take it off" "1 stale"
sed 's/ 1 witness/ 2 witness/' "$known" > "$out/count.known"
oracle count "$TEQ" --known "$out/count.known" tests/errors/exhaustivity.scala
expect count 1 "listed 2 times, seen 1"
{ cat "$known"; tail -1 "$known"; } > "$out/twice.known"
oracle duplicate-line "$TEQ" --known "$out/twice.known" tests/errors/exhaustivity.scala
expect duplicate-line 2 "the same difference as line"
tail -1 "$known" > "$out/causeless.known"
oracle causeless "$TEQ" --known "$out/causeless.known" tests/errors/exhaustivity.scala
expect causeless 2 "names the cause witness, which no .cause. line gives"

mkdir -p "$out/w"
cp tests/scalac-oracle/witness-a.scala "$out/w/Input.scala"
SCALAC_DRAFT=$out/witness.draft oracle witness-a "$TEQ" "$out/w/Input.scala"
expect witness-a 1 "^files payload $(shown "$out/w/Input.scala"):2 warning $hex $hex 1 "
{ echo "cause witness  scalac writes a type test's witness as _: T, teq as T, tests/scalac-oracle/witness-a.scala"; sed 's/ ? / witness /' "$out/witness.draft"; } > "$out/witness.known"
cp tests/scalac-oracle/witness-b.scala "$out/w/Input.scala"
oracle witness-b "$TEQ" --known "$out/witness.known" "$out/w/Input.scala"
expect witness-b 1 "^not listed: files payload $(shown "$out/w/Input.scala"):2 warning" "take it off" "1 not listed, 1 stale"

# A place is a line and a column: teq at scalac's span start, lines above its point, agrees; at the point's line
# with another column it is an anchor difference.
oracle span "$TEQ" tests/scalac-oracle/span.scala
expect span 0 "scalac 1 errors, 0 warnings" "teq 1 errors, 0 warnings" "0 differences"
python3 - "$out/span" <<'PY'
import json, sys
d = sys.argv[1]
a = json.loads(open(f"{d}/files.answer").read().splitlines()[-1])
a["diagnostics"][0]["line"] = 6
open(f"{d}/moved.answer", "w").write(json.dumps(a) + "\n")
PY
STAND_IN_OUT=$out/span/moved.answer STAND_IN_ERR=$out/span/files.teq.log STAND_IN_EXIT=1 oracle moved "$stand_in" tests/scalac-oracle/span.scala
expect moved 1 "^files anchor tests/scalac-oracle/span.scala:6 error $hex $hex 1 "
# A symbol is compared as written: the same answer naming `a b` for `a  b` is a payload difference.
oracle symbol "$TEQ" tests/scalac-oracle/symbol.scala
expect symbol 0 "scalac 1 errors" "teq 1 errors" "0 differences"
python3 - "$out/symbol" <<'PY'
import sys
d = sys.argv[1]
for name, renamed in (("files.answer", "renamed.answer"), ("files.teq.log", "renamed.log")):
    text = open(f"{d}/{name}").read()
    assert "not found: a  b" in text
    open(f"{d}/{renamed}", "w").write(text.replace("not found: a  b", "not found: a b"))
PY
STAND_IN_OUT=$out/symbol/renamed.answer STAND_IN_ERR=$out/symbol/renamed.log STAND_IN_EXIT=1 oracle renamed "$stand_in" tests/scalac-oracle/symbol.scala
expect renamed 1 "^files payload tests/scalac-oracle/symbol.scala:3 error $hex $hex 1 " '"name": "a  b"' '"name": "a b"'
# A type's name is compared as written, its leading space too: the same answer requiring `a` for ` a` differs.
oracle leading "$TEQ" tests/scalac-oracle/leading.scala
expect leading 0 "scalac 1 errors" "teq 1 errors" "0 differences"
python3 - "$out/leading" <<'PY'
import sys
d = sys.argv[1]
for name, renamed in (("files.answer", "renamed.answer"), ("files.teq.log", "renamed.log")):
    text = open(f"{d}/{name}").read()
    assert "required  a" in text
    open(f"{d}/{renamed}", "w").write(text.replace("required  a", "required a"))
PY
STAND_IN_OUT=$out/leading/renamed.answer STAND_IN_ERR=$out/leading/renamed.log STAND_IN_EXIT=1 oracle leading-renamed "$stand_in" tests/scalac-oracle/leading.scala
expect leading-renamed 1 "^files payload tests/scalac-oracle/leading.scala:5 error $hex $hex 1 " '"required": " a"' '"required": "a"'
oracle wrapped "$TEQ" tests/scalac-oracle/wrapped.scala
expect wrapped 1 '"required": "Either\[VeryLongTypeNameToForceThePrettyPrinterIntoMultilineModeOne, VeryLongTypeNameToForceThePrettyPrinterIntoMultilineModeTwo\]"'
# A string literal type naming the checkout's path is not rewritten: its pinned difference does not hold for
# another literal at the same place.
mkdir -p "$out/p"
python3 -c 'import sys; print("object P:\n  val v: \"%s/wanted\" = \"other\"" % sys.argv[1])' "$PWD" > "$out/p/Input.scala"
SCALAC_DRAFT=$out/path.draft oracle path-a "$TEQ" "$out/p/Input.scala"
{ echo "cause path  a literal, none"; sed 's/ ? / path /' "$out/path.draft"; } > "$out/path.known"
python3 -c 'import sys; print("object P:\n  val v: \"wanted\"%s = \"other\"" % (" " * (len(sys.argv[1]) + 1)))' "$PWD" > "$out/p/Input.scala"
oracle path-b "$TEQ" --known "$out/path.known" "$out/p/Input.scala"
expect path-b 1 "not listed: files payload $(shown "$out/p/Input.scala"):2 error" "take it off"

oracle twice "$TEQ" tests/scalac-oracle/twice.scala
expect twice 0 "teq 0 errors, 2 warnings" "0 differences"
# The same answer and output with the second warning dropped.
python3 - "$out/twice" <<'PY'
import json, sys
d = sys.argv[1]
a = json.loads(open(f"{d}/files.answer").read().splitlines()[-1])
a["diagnostics"] = a["diagnostics"][:1]
open(f"{d}/one.answer", "w").write(json.dumps(a) + "\n")
lines = open(f"{d}/files.teq.log").read().splitlines()
open(f"{d}/one.log", "w").write("\n".join(lines[:3]) + "\n")
PY
STAND_IN_OUT=$out/twice/one.answer STAND_IN_ERR=$out/twice/one.log oracle once "$stand_in" tests/scalac-oracle/twice.scala
expect once 1 "^files scalac-only tests/scalac-oracle/twice.scala:2 warning $hex - 1 " "1 differences"

: > "$out/empty"
STAND_IN_OUT=$out/empty oracle no-answer "$stand_in" tests/scalac-oracle/twice.scala
expect no-answer 1 "teq's files gave no answer with diagnostics" "0 differences.*FAIL"
STAND_IN_OUT=$out/twice/one.answer STAND_IN_ERR=$out/twice/files.teq.log oracle short-answer "$stand_in" tests/scalac-oracle/twice.scala
expect short-answer 1 "teq's files printed 2 diagnostics and answered 1"
STAND_IN_OUT=$out/twice/one.answer STAND_IN_ERR=$out/twice/one.log STAND_IN_EXIT=1 oracle unanswered-error "$stand_in" tests/scalac-oracle/twice.scala
expect unanswered-error 1 "teq's files exits 1 with 0 errors and 1 warnings in its answer \\(ok true\\)"
# An unexplained failure (no diagnostic, `ok` false) fails the run though a known line names its exit.
echo '{"ok":false,"diagnostics":[]}' > "$out/failed.answer"
echo 'internal build failure' > "$out/failed.log"
printf 'cause exit  a stand-in, none\nfiles production-exit - - 0 1 1 exit\n' > "$out/failed.known"
STAND_IN_OUT=$out/failed.answer STAND_IN_ERR=$out/failed.log STAND_IN_EXIT=1 oracle unexplained "$stand_in" --known "$out/failed.known" tests/scalac-oracle/span.scala
expect unexplained 1 "teq's files exits 1 with 0 errors and 0 warnings in its answer \\(ok false\\)" "FAIL"
STAND_IN_EXIT=101 oracle crash "$stand_in" tests/scalac-oracle/twice.scala
expect crash 1 "teq's files exits 101"
SCALA_CLI=$stand_in oracle no-summary "$TEQ" tests/scalac-oracle/twice.scala
expect no-summary 1 "scalac's files exits 0 after .* without its summary"

oracle suppressed "$TEQ" --scalac-options "-Wconf:any:s" tests/errors/exhaustivity.scala
expect suppressed 1 "scalac 0 errors, 6 warnings" "scalac exits 0, teq 0; left out of the conformance pass: -Wconf:any:s" "1 differences"
oracle werror "$TEQ" --scalac-options "-Werror" --flags "--werror" tests/errors/exhaustivity.scala
expect werror 1 "scalac exits 1, teq 1; left out of the conformance pass: -Werror" "1 differences"
oracle werror-unmapped "$TEQ" --scalac-options "-Werror" tests/errors/exhaustivity.scala
expect werror-unmapped 1 "^files production-exit - - 1 0 1 " "2 differences"
# A promotion turned off again is no promotion: scalac runs the production pass and exits 0.
oracle werror-off "$TEQ" --scalac-options "-Werror -Werror:false" --flags "--werror" tests/scalac-oracle/twice.scala
expect werror-off 1 "left out of the conformance pass: -Werror -Werror:false" "^files production-exit - - 0 1 1 "

# The application mode, and api-check.sh's tests over a binary's own products, over lists whose rows are source files
# (app-lists.sh lists a source outside its module's directories itself): main one file, the tests one over it.
. tests/support/jars.sh
lib=$(jar_of scala-library)
loose=$out/loose
mkdir -p "$loose/app"
echo 'object Main { val n = 1 }' > "$loose/app/Main.scala"
echo 'object Check { val m = Main.n + 1 }' > "$loose/app/Check.scala"
printf '# main\nMain.scala\n' > "$loose/modules.txt"
printf '# test\nCheck.scala\n' > "$loose/test-modules.txt"
echo "$lib" > "$loose/classpath.txt"
printf '@app/compile\n%s\n' "$lib" > "$loose/test-classpath.txt"
: > "$loose/options.txt"
loose_env=(APP_ROOT="$loose/app" APP_MODULES="$loose/modules.txt" APP_CLASSPATH="$loose/classpath.txt" APP_FLAGS=
  APP_SCALAC_OPTIONS="$loose/options.txt" APP_TEST_MODULES="$loose/test-modules.txt"
  APP_TEST_CLASSPATH="$loose/test-classpath.txt" APP_TEST_FLAGS= APP_TEST_SCALAC_OPTIONS="$loose/options.txt")
if [ ! -f "$lib" ]; then
  bad "no scala-library jar at $lib for the application mode over source files" loose-app
else
  env "${loose_env[@]}" timeout 600 python3 bench/app/diagnostics.py app "$TEQ" "$out/loose-app" > "$out/loose-app.log" 2>&1
  code=$?
  expect loose-app 0 "^main: 1 files; scalac 0 errors, 0 warnings" "^test: 1 files; scalac 0 errors, 0 warnings" "0 differences"
  mkdir -p "$out/loose-tmp"
  env "${loose_env[@]}" TMPDIR="$out/loose-tmp" timeout 600 bench/app/api-check.sh --test "$TEQ" > "$out/loose-check.log" 2>&1
  code=$?
  expect loose-check 0 "^api-check: exit 0"
fi

# This directory is the run's own: a second run into it is refused, and leaves it as it is.
TEQ=$TEQ timeout 60 tests/scalac-oracle.sh "$out" > "$out/again.log" 2>&1
again=$?
if [ $again = 2 ] && grep -q 'holds files already' "$out/again.log" && [ -f "$out/numeric/differences.txt" ]; then ok; else bad "a second run into this directory: exit $again" again; fi

echo "scalac oracle: $pass passed, $fail failed"
[ $fail = 0 ]
