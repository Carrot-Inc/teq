#!/bin/bash
# tests/tasty/representation/check.sh <teq> <work dir> [--update]: the representation oracle (README.md). The fixtures'
# products, scalac 3.8.4's and teq's, each read by the other compiler's reflection through observe.scala:
#   S·S  scalac reading scalac's products, the reference;  S·T  scalac reading teq's;  T·S  teq reading scalac's.
# Prints a line per check, `ok` or `FAIL` with the difference; exits 1 on a failure. --update rewrites the recorded
# diffs (expected/) and identity.txt, and still checks the pins against them.
teq=$1 work=$(realpath -m "$2") mode=$3  # absolute: scala-cli runs from its own project root
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../../.."
. tests/support/jars.sh
R=tests/tasty/representation W=$work
sjs=$(jar_of scalajs-library)
sl=$(jar_of scala-library)
if [ ! -f "$sjs" ] || [ ! -f "$sl" ]; then
  echo "FAIL representation: no scalajs-library or scala-library 3.8.4 in the coursier cache (incomplete validation)"
  exit 1
fi
rm -rf "$W" && mkdir -p "$W/sc" "$W/tq" "$W/cell" "$W/ts" "$W/gs" "$W/api8" "$W/rec8"
status=0
fail() { echo "FAIL representation $*"; status=1; }

# teq's products. A fixture teq refuses gives the S·T cell `refused: <teq's first line>`.
product() { # <name> <teq's verb and flags>...
  local name=$1; shift
  if ! timeout 120 "$teq" "$@" > "$W/tq-$name.log" 2>&1; then
    echo "refused: $(head -1 "$W/tq-$name.log")" > "$W/tq-$name.refused"
  fi
}
product api compiler check --products "$W/tq/api" "$R/api.scala"
product js compiler check --products "$W/tq/js" "$R/js.scala"
product cc compiler check --products "$W/tq/cc" "$R/cc.scala"
product refining compiler build --target jvm --std=scala-library --products "$W/tq/refining-build" "$R/refining.scala" --classpath "$sl"
# The JVM build's own products, without the std's runtime classes the directory also holds.
mkdir -p "$W/tq/refining" && { [ -d "$W/tq/refining-build/repr" ] && cp -R "$W/tq/refining-build/repr" "$W/tq/refining/"; }
# The variants: every detection pair's other side, each an exact text of api.scala or of rec/Pickled.scala.
python3 - "$R" "$W" <<'VARIANT'
import sys
r, w = sys.argv[1], sys.argv[2]
for src, dest, pairs in [
    ("api.scala", "api8/api.scala",
     [("@mark(7)", "@mark(8)"), ("@mark(Values.a.n)", "@mark(Values.b.n)"),
      ("@tag((null: String))", "@tag((null: Object))"),
      ('@Text("left), String(middle", "right")', '@Text("left", "middle), String(right")'),
      ('@tag("left\\u0085right")', '@tag("left\\u2028right")'), ('@tag("{TypeAlias}")', '@tag("")'),
      ("@memberTag(7)", "@memberTag(8)"), ("@memberTag(17)", "@memberTag(18)"), ("@memberTag(27)", "@memberTag(28)"),
      ("@memberTag(37)", "@memberTag(38)"), ("@memberTag(57)", "@memberTag(58)"), ("@memberTag(47)", "@memberTag(48)"),
      ("@memberTag(67)", "@memberTag(68)"), ("@memberTag(77)", "@memberTag(78)"), ("@memberTag(87)", "@memberTag(88)")]),
    ("rec/Pickled.scala", "rec8/Pickled.scala",
     [("Kinds.make(true)", "Kinds.make(false)"), ("Kinds.locals(true)", "Kinds.locals(false)"),
      ("@tagged(7)", "@tagged(8)"), ("Kinds.bind(true)", "Kinds.bind(false)"),
      ("Kinds.variance(true)", "Kinds.variance(false)"), ("Kinds.within(true)", "Kinds.within(false)"),
      ("Kinds.designator(true)", "Kinds.designator(false)"), ("Kinds.returnTarget(true)", "Kinds.returnTarget(false)"),
      ("Kinds.returnTargetNamed(true)", "Kinds.returnTargetNamed(false)"), ("Kinds.floatBits(true)", "Kinds.floatBits(false)"),
      ('Kinds.returnTo("jumpS", "super$jumpS")', 'Kinds.returnTo("jumpS", "jumpS")'),
      ('Kinds.returnTo("jumpA", "jumpAI")', 'Kinds.returnTo("jumpA", "jumpAS")'),
      ('Kinds.returnTo("hop", "hopS")', 'Kinds.returnTo("hop#2", "hopLiteral")'),
      ("@tagged(97)", "@tagged(98)"), ("@tagged(107)", "@tagged(108)"), ("@tagged(109)", "@tagged(110)"),
      ("Kinds.returnTarget(first = true)", "Kinds.returnTarget(first = false)"),
      ('@text("\\n")', '@text("\\\\n")'), ('@text("a], STRINGconst[b", "c")', '@text("a", "b], STRINGconst[c")'),
      ("Kinds.paramNames(first = true)", "Kinds.paramNames(first = false)"),
      ("Kinds.signed(first = true)", "Kinds.signed(first = false)"),
      ("def andPrefix(x: AndA & AndB)", "def andPrefix(x: AndA & AndBC)"), ("def orPrefix(x: OrA | OrB)", "def orPrefix(x: OrA | OrQ)"),
      ("def refinedPrefix(x: AndA { type T = Int })", "def refinedPrefix(x: AndA { type T = String })"),
      ("def nameDesignated(x: AnyRef { type T })(y: x.T): x.T", "def nameDesignated(x: AnyRef { type U })(y: x.U): x.U"),
      ("@shape(Pickled.`sigShaped(scala.Int)scala.Int`)", "@shape(Pickled.sigShaped(1))"),
      ("def m(x: CompR.Inner): CompR.Inner", "def m(x: OtherR.Inner): OtherR.Inner"),
      ("val refinedM: RefM { def f(x: Int): String } = null", "val refinedM: RefM { def f(x: Int): CharSequence } = null")])]:
    s = open(f"{r}/{src}").read()
    # The new quoted-name and surrogate fixtures in `Odd` are the fallback's, not a detection pair's: keep them as they
    # are, so `rec8`'s unread bodies do not differ only in the reader's thrown message.
    marker = "\n// The fallback's quoted member names"
    head, tail = s.split(marker, 1) if src == "rec/Pickled.scala" and marker in s else (s, "")
    for a, b in pairs:
        assert a in head, a
        head = head.replace(a, b)
    s = head + (marker + tail if tail else "")
    open(f"{w}/{dest}", "w").write(s)
VARIANT

# scalac's side, in one JVM: the products, then each observation.
oracle_cp=$(COURSIER_MODE=offline timeout 300 scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline -q \
  --workspace "$W/oracle" --print-class-path "$PWD/$R/Oracle.scala" "$PWD/$R/dotty/Identity.scala" "$PWD/$R/dotty/Buckets.scala" "$PWD/$R/dotty/Api.scala" \
  --dep org.scala-lang:scala3-compiler_3:3.8.4 2> "$W/oracle-build.err") || {
  echo "FAIL representation: scalac's oracle does not build (incomplete validation): $(grep -v hint "$W/oracle-build.err" | head -3)"
  exit 1
}
oracle_cp=$(echo "$oracle_cp" | tail -1)
job() { local IFS=$'\t'; echo "$*" >> "$jobs"; }
jobs=$W/jobs
: > "$jobs"
job version version
job identity identity
job buckets buckets
job api quotes-api
job compile api "" "$W/sc/api" "" "$PWD/$R/api.scala"
job compile api8 "" "$W/sc/api8" "" "$W/api8/api.scala"
job compile cc38 "" "$W/sc/cc38" "-source:3.8" "$PWD/$R/cc.scala"
job compile cc39 "" "$W/sc/cc39" "-source:3.9" "$PWD/$R/cc.scala"
job compile js "$sjs" "$W/sc/js" "-scalajs" "$PWD/$R/js.scala"
job compile refining "" "$W/sc/refining" "" "$PWD/$R/refining.scala"
job compile recA "" "$W/sc/recA" "" "$PWD/$R/rec/Lib.scala"
job compile recB "$W/sc/recA" "$W/sc/recB" "" "$PWD/$R/rec/Pickled.scala"
job compile recB8 "$W/sc/recA" "$W/sc/recB8" "" "$W/rec8/Pickled.scala"
job compile nulls "" "$W/sc/nulls" "-Yexplicit-nulls" "$PWD/$R/rec/Nulls.scala"
job compile probe "" "$W/sc/probe" "" "$PWD/$R/probe/Subjects.scala"
job compile observe "" "$W/sc/observe" "-experimental" "$PWD/$R/observe.scala" "$PWD/$R/compiler/Scalac.scala" "$PWD/$R/probes.scala"
# Every downstream observation retains the trees it reads (scalac's -Yretain-trees, teq's --retain-trees), so that
# `Symbol.tree` gives a method its body as the pickle holds it, not the empty one it makes without them (the pass over
# 1c61410d5): the direct observations, the generated traversals, the cross reads and the probes alike.
look() { # <cell> <class path> <options> <downstream>
  job observe "$1" "$W/sc/observe:$2" "$W/out-$1" "-experimental -Yretain-trees $3" "$PWD/$R/use/$4.scala"
}
look ss-api "$W/sc/api" "" api
look ss-api8 "$W/sc/api8" "" api
look ss-cc38 "$W/sc/cc38" "" cc
look ss-cc39 "$W/sc/cc39" "" cc
look ss-js "$W/sc/js:$sjs" "-scalajs" js
look ss-refining "$W/sc/refining" "" refining
look ss-rec "$W/sc/recA:$W/sc/recB" "" rec
look ss-rec8 "$W/sc/recA:$W/sc/recB8" "" rec
look ss-nulls "$W/sc/nulls" "" nulls
# The reflection's probes (README.md): over scalac's product of their subjects (ss-probe, against teq's ts-probe), and
# over the subjects compiled with the probes' calls, each compiler's reflection answering its own typing (ps-probe,
# against teq's pt-probe), as the review passes ran them.
look ss-probe "$W/sc/probe" "" probe
job observe ps-probe "$W/sc/observe" "$W/out-ps-probe" "-experimental -Yretain-trees" "$PWD/$R/probe/Subjects.scala" "$PWD/$R/use/probe.scala"
for x in api js refining cc; do
  [ -f "$W/tq-$x.refused" ] && continue
  case $x in js) look st-js "$W/tq/js:$sjs" "-scalajs" js ;; *) look "st-$x" "$W/tq/$x" "" "$x" ;; esac
done
(cd "$W" && timeout 300 java -cp "$oracle_cp" oracle jobs > oracle.out 2> oracle.err)
python3 "$R/oracle.py" split "$W/oracle.out" "$W/cell"
for x in api js refining cc; do [ -f "$W/tq-$x.refused" ] && cp "$W/tq-$x.refused" "$W/cell/st-$x.txt"; done
if [ "$(cat "$W/cell/version.txt" 2> /dev/null)" != "3.8.4" ]; then
  fail "oracle: scalac's version is $(cat "$W/cell/version.txt" 2> /dev/null), not 3.8.4 (incomplete validation)"
fi
for name in api api8 cc38 cc39 js refining recA recB recB8 nulls probe observe; do
  if [ -s "$W/cell/$name.txt" ]; then fail "oracle: scalac's $name: $(head -3 "$W/cell/$name.txt")"; fi
done
for cell in ss-api ss-api8 ss-cc38 ss-cc39 ss-js ss-refining ss-rec ss-rec8 ss-nulls ss-probe ps-probe; do
  if [ ! -s "$W/cell/$cell.txt" ] || grep -q '^FAIL ' "$W/cell/$cell.txt"; then
    fail "oracle: scalac observes nothing in $cell: $(grep '^FAIL ' "$W/cell/$cell.txt" | head -2)"
  fi
done

# The generated traversal, run by scalac's own reflection (check 8): a call per line of each of scalac's cells, by the
# line's key, so that what teq's reflection is asked is what scalac observed, record by record.
jobs=$W/jobs2
: > "$jobs"
again() { # <scalac cell> <class path> <options> <downstream>
  python3 "$R/oracle.py" gen "$W/cell/$1.txt" "$R/use/$4.scala" "$W/gs/$1.scala" || fail "generation: $1's calls"
  job observe "gs-${1#ss-}" "$W/sc/observe:$2" "$W/out-gs-${1#ss-}" "-experimental -Yretain-trees $3" "$W/gs/$1.scala"
}
again ss-api "$W/sc/api" "" api
again ss-api8 "$W/sc/api8" "" api
again ss-cc38 "$W/sc/cc38" "" cc
again ss-cc39 "$W/sc/cc39" "" cc
again ss-js "$W/sc/js:$sjs" "-scalajs" js
again ss-refining "$W/sc/refining" "" refining
again ss-rec "$W/sc/recA:$W/sc/recB" "" rec
again ss-rec8 "$W/sc/recA:$W/sc/recB8" "" rec
again ss-nulls "$W/sc/nulls" "" nulls
(cd "$W" && timeout 300 java -cp "$oracle_cp" oracle jobs2 > oracle2.out 2> oracle2.err)
python3 "$R/oracle.py" split "$W/oracle2.out" "$W/cell"

# teq's side: a call per line scalac observed, so that a shape teq's reflection refuses is that line's alone.
see() { # <cell> <scalac cell> <downstream> <teq's flags>...
  local cell=$1 of=$2 use=$3; shift 3
  python3 "$R/oracle.py" gen "$W/cell/$of.txt" "$R/use/$use.scala" "$W/ts/$cell.scala" || fail "generation: $of's calls"
  timeout 120 "$teq" compiler check --retain-trees "$@" "$R/observe.scala" "$R/compiler/Teq.scala" "$W/ts/$cell.scala" > "$W/ts/$cell.log" 2>&1
  python3 "$R/oracle.py" teq "$W/ts/$cell.log" "$W/ts/$cell.scala" > "$W/cell/$cell.txt"
}
# Under scala-library, which scalac compiled the fixtures against, not the lean std's own names for its classes.
see ts-api ss-api api --std=scala-library --classpath "$sl:$W/sc/api"
see ts-cc38 ss-cc38 cc --std=scala-library --classpath "$sl:$W/sc/cc38"
see ts-cc39 ss-cc39 cc --std=scala-library --classpath "$sl:$W/sc/cc39"
see ts-js ss-js js --std=scala-library --classpath "$sl:$W/sc/js"
see ts-refining ss-refining refining --std=scala-library --classpath "$sl:$W/sc/refining"
see ts-rec ss-rec rec --std=scala-library --classpath "$sl:$W/sc/recA:$W/sc/recB"
see ts-nulls ss-nulls nulls --std=scala-library --classpath "$sl:$W/sc/nulls"
# The probes: their downstream is a call a record already, so it is teq's calls as it stands.
probe() { # <cell> <the subjects' source, if compiled with the calls> <teq's flags>...
  local cell=$1 subjects=$2; shift 2
  timeout 120 "$teq" compiler check --retain-trees "$@" "$R/observe.scala" "$R/compiler/Teq.scala" "$R/probes.scala" $subjects \
    "$R/use/probe.scala" > "$W/ts/$cell.log" 2>&1
  python3 "$R/oracle.py" teq "$W/ts/$cell.log" "$R/use/probe.scala" > "$W/cell/$cell.txt"
}
probe ts-probe "" --std=scala-library --classpath "$sl:$W/sc/probe"
probe pt-probe "$R/probe/Subjects.scala" --std=scala-library --classpath "$sl"

# The records.
c=$W/cell
# 11. The products' tags: each child a reference cell's reader could not give (its records' outcome fields), read from
# the pickle scalac wrote record by record (`teq tasty --nodes`, the route the encoding check's references take), so that it
# is observed apart from the reader that threw (oracle.py `tags`), on both products of a detection pair.
tagged() { # <reference cell> <its product>
  python3 "$R/oracle.py" tags "$c/$1.txt" "$2" "$teq" "$c/$1-tags.txt"
}
for x in api api8 cc38 cc39 js refining nulls probe; do tagged "ss-$x" "$W/sc/$x"; done
tagged ss-rec "$W/sc/recB"
tagged ss-rec8 "$W/sc/recB8"
tagged ps-probe "$W/out-ps-probe"
# And the same children in teq's product of the twin, where teq writes one (the S·T cells'), read alike.
for x in api js refining cc; do
  [ -f "$W/tq-$x.refused" ] && continue
  python3 "$R/oracle.py" tags "$c/ss-${x/cc/cc38}.txt" "$W/tq/$x" "$teq" "$c/st-$x-tags.txt"
done
# The observer serializes every form, so no cell holds `Unserialized` (observe.scala's last arms), and no fixture holds
# a kind no fixture covers (`Unobservable(..)`: the observation fails on meeting one, README.md, "Coverage").
for f in "$c"/ss-*.txt "$c"/gs-*.txt "$c"/st-*.txt "$c"/ts-*.txt "$c"/ps-*.txt "$c"/pt-*.txt; do
  if grep -q 'Unserialized' "$f"; then fail "observe: a form the observer does not serialize in $(basename "$f" .txt): $(grep -m1 'Unserialized' "$f" | cut -c1-160)"; fi
  if grep -q 'Unobservable(' "$f"; then fail "observe: a kind no fixture covers met in $(basename "$f" .txt): add its fixture: $(grep -m1 -o 'Unobservable([A-Za-z]*' "$f")"; fi
done
{ echo "## api"; diff "$c/ss-api.txt" "$c/ss-api8.txt"; echo "## rec"; diff "$c/ss-rec.txt" "$c/ss-rec8.txt"
  # The T-S normalization keeps every pair apart: the marks are stripped outside quoted spans alone.
  echo "## api-stripped"; diff <(python3 "$R/oracle.py" strip "$c/ss-api.txt") <(python3 "$R/oracle.py" strip "$c/ss-api8.txt")
  # Through the generated traversal, scalac's reflection answering each call teq's is asked.
  echo "## api-generated"; diff "$c/gs-api.txt" "$c/gs-api8.txt"; echo "## rec-generated"; diff "$c/gs-rec.txt" "$c/gs-rec8.txt"
  # Through the products' tags, the children scalac's reflection could not give (check 11).
  echo "## rec-tags"; diff "$c/ss-rec-tags.txt" "$c/ss-rec8-tags.txt"
} > "$W/detect.diff"
# The comparisons hold what the reference observed and no more: its outcome fields dropped, the other side's record
# excised of each child the reference could not give (oracle.py `excise`), whose evidence is check 11's.
plain() { python3 "$R/oracle.py" strip "$1"; }
excised() { python3 "$R/oracle.py" excise "$1" "$2"; }
{ for x in api js refining cc; do
    echo "## S-T:$x"; diff <(python3 "$R/oracle.py" fieldless "$c/ss-${x/cc/cc38}.txt") <(excised "$c/ss-${x/cc/cc38}.txt" "$c/st-$x.txt")
  done; } > "$W/S-T.diff"
{ for x in api cc38 cc39 js refining rec nulls probe; do
    echo "## T-S:$x"; diff <(plain "$c/ss-$x.txt") <(excised "$c/ss-$x.txt" "$c/ts-$x.txt")
  done; } > "$W/T-S.diff"
# The probes over their own compilation: scalac's reflection's answer, marks stripped, against teq's.
{ echo "## R:probe"; diff <(plain "$c/ps-probe.txt") <(excised "$c/ps-probe.txt" "$c/pt-probe.txt"); } > "$W/probes.diff"
# Check 11's record: each unread child's tags, scalac's answer, beside the other reader's reading of it (teq's, from
# the T-S cell), so that a change of either reopens its account (unread.txt).
python3 "$R/oracle.py" unread-record "$c" "$W/unread-tags.txt"
# The encoding (check 9): each twin's pickles, scalac's and teq's, dumped as one escaped record per node
# (`teq tasty --nodes`), their reference facts compared as they are and each difference classified, encoding or
# identity (oracle.py `encoding`).
rm -rf "$W/enc"
for x in api js refining; do
  [ -f "$W/tq-$x.refused" ] && continue
  (cd "$W/sc/$x" && find . -name '*.tasty' | LC_ALL=C sort) | while read -r f; do
    d=$W/enc/$x/${f%.tasty}
    mkdir -p "$(dirname "$d")"
    "$teq" tasty --nodes "$W/sc/$x/$f" > "$d.sc"
    [ -f "$W/tq/$x/$f" ] && "$teq" tasty --nodes "$W/tq/$x/$f" > "$d.tq"
  done
done
if enc_counts=$(python3 "$R/oracle.py" encoding "$W/enc" "$W/encoding.diff"); then enc_ok=1; else enc_ok=0; fi
if [ "$mode" = "--update" ]; then
  cp "$W/detect.diff" "$W/S-T.diff" "$W/T-S.diff" "$W/probes.diff" "$W/encoding.diff" "$W/unread-tags.txt" "$R/expected/"
  cp "$c/identity.txt" "$R/dotty/identity.txt"
  cp "$c/buckets.txt" "$R/dotty/buckets.txt"
fi
# 1. Detection: each variant's argument is seen, on the annotated members alone, through the recorded cells.
if ! awk '/^## /{s=$2} /^[<>] /{n[s]++} END{exit !(n["api"] > 0 && n["rec"] > 0 && n["api-stripped"] > 0 && n["api-generated"] > 0 && n["rec-generated"] > 0 && n["rec-tags"] > 0)}' "$W/detect.diff"; then
  fail "detect: a variant's observations are its original's (a section of detect.diff holds no difference)"
elif ! awk '/^## /{s=$2} /^[<>] /{if (s == "rec-tags" && $0 ~ /Pickled\.blindNewline/) n++} END{exit !(n > 0)}' "$W/detect.diff"; then
  fail "detect: the newline pair (@tagged(109) against @tagged(110)) is not seen apart through the products' tags"
elif ! awk '/^## /{s=$2} /^[<>] /{if (s == "rec-tags" && $0 ~ /Pickled\.blindReturn/) n++} END{exit !(n > 0)}' "$W/detect.diff"; then
  fail "detect: the return pair (jump(Int) against jump(String)) is not seen apart through the products' tags"
elif cmp -s "$W/detect.diff" "$R/expected/detect.diff"; then
  echo "ok representation detect: the oracle sees each pair's two sides apart (@mark(7) against @mark(8), Byte against Int constants, U+0085 against U+2028, the outer x against the inner, @tagged(7) against @tagged(8) on a local and on a binder's type, a string holding a bounds mark against the empty one, raw and stripped, F[+X] against F[-X], a boundary Outer against Outer.Product, a root member's, a root parameter's, a root parent's and a requested owner's @memberTag against the next, a member named <owner> in a class and its companion), raw, stripped and through the generated traversal, and @tagged(107) against @tagged(108) and @tagged(109) against @tagged(110) on a local of a body scalac's reflection cannot read, and the return pair (jump(Int) against jump(String)) and the identity collision pair (the second hop against the literal hop#2) there, and the signed-name pair (the two f overloads whose parameter signatures flatten alike) there, and a newline against backslash-n, a], STRINGconst[b/c against a/b], STRINGconst[c, and a lambda parameter x against z, through the products' tags, which the twin's views do not"
else
  fail "detect (< recorded, > now):"; diff "$R/expected/detect.diff" "$W/detect.diff" | head -10
fi
# The three pairs the fallback must keep apart on their own (the ruling on 8aa2ab15a's pass): each independently, so
# that every one whose records still collide is named.
if ! awk '/^## /{s=$2} /^[<>] /{if (s == "rec-tags" && $0 ~ /Pickled\.blindEscape/) n++} END{exit !(n > 0)}' "$W/detect.diff"; then
  fail "detect: the escape pair (a newline against backslash-n) is not seen apart through the products' tags"
fi
if ! awk '/^## /{s=$2} /^[<>] /{if (s == "rec-tags" && $0 ~ /Pickled\.blindDelimit/) n++} END{exit !(n > 0)}' "$W/detect.diff"; then
  fail "detect: the delimiter pair (a], string[b / c and a / b], string c) is not seen apart through the products' tags"
fi
if ! awk '/^## /{s=$2} /^[<>] /{if (s == "rec-tags" && $0 ~ /Pickled\.blindParams/) n++} END{exit !(n > 0)}' "$W/detect.diff"; then
  fail "detect: the parameter-name pair (x against z) is not seen apart through the products' tags"
fi
if ! awk '/^## /{s=$2} /^[<>] /{if (s == "rec-tags" && $0 ~ /Pickled\.blindCollision/) n++} END{exit !(n > 0)}' "$W/detect.diff"; then
  fail "detect: the identity collision pair (the second hop against hop#2) is not seen apart through the products' tags"
fi
if ! awk '/^## /{s=$2} /^[<>] /{if (s == "rec-tags" && $0 ~ /Pickled\.blindSigned/) n++} END{exit !(n > 0)}' "$W/detect.diff"; then
  fail "detect: the signed-name pair (the two f overloads whose parameter signatures flatten alike) is not seen apart through the products' tags"
fi
# 10. The observer's API inventory (landing 1's list, closure/entries-observer.txt): regenerated from the observer's
# own SemanticDB on every run and compared with the committed one, so that it cannot fall behind the observer.
if timeout 330 "$R/closure/observer.sh" "$W/observer-inv" "$W/entries-observer.txt" > "$W/observer-inv.log" 2>&1; then
  if cmp -s "$W/entries-observer.txt" "$R/closure/entries-observer.txt"; then
    echo "ok representation inventory: the observer's API inventory, regenerated, is the committed one ($(grep -c '^scala/quoted' "$W/entries-observer.txt") symbols)"
  else
    fail "inventory (< committed, > regenerated):"; diff "$R/closure/entries-observer.txt" "$W/entries-observer.txt" | head -10
  fi
else
  fail "inventory: observer.sh failed: $(tail -2 "$W/observer-inv.log")"
fi
# 1 and 2-3's evidence: a difference only a reader's outcome field makes is no evidence (the pass over 334a43adf); and
# 11, every child a reference reader could not give is red unless the products' tags observe it and unread.txt
# accounts for it, subtree by subtree, whatever the rest of its record holds (the pass over 7a9ec6133).
python3 "$R/oracle.py" detect-evidence "$W/detect.diff" || status=1
python3 "$R/oracle.py" incomplete "$c" "$R/unread.txt" || status=1
mkdir -p "$W/incomplete-self"
python3 "$R/oracle.py" incomplete-selfcheck "$c" "$R/unread.txt" "$W/incomplete-self" || status=1
python3 "$R/oracle.py" records-selfcheck "$c" || status=1
python3 "$R/oracle.py" identity-selfcheck "$c" || status=1
python3 "$R/oracle.py" injectivity-selfcheck "$c" || status=1
python3 "$R/oracle.py" key-selfcheck || status=1
if cmp -s "$W/unread-tags.txt" "$R/expected/unread-tags.txt"; then
  echo "ok representation unread: each unread child's tags and the other reader's reading of it are the recorded ones ($(grep -c '^## ' "$W/unread-tags.txt") children)"
else
  fail "unread (< recorded, > now; its account in unread.txt to revisit):"; diff "$R/expected/unread-tags.txt" "$W/unread-tags.txt" | head -10
fi
# 2, 3. The writer and the reader, and the reflection's probes, against the recorded differences.
for d in S-T T-S probes; do
  if cmp -s "$W/$d.diff" "$R/expected/$d.diff"; then
    echo "ok representation $d: the identity differences from scalac's observation are the recorded ones"
  else
    fail "$d (< recorded, > now):"; diff "$R/expected/$d.diff" "$W/$d.diff" | head -20
  fi
done
# 9. The encoding: how each writer encodes a reference, apart from what it refers to, against the record.
if [ "$enc_ok" = 1 ] && cmp -s "$W/encoding.diff" "$R/expected/encoding.diff"; then
  echo "ok representation encoding: the twins' reference encodings, read from the products tag by tag, are the recorded ones: $enc_counts (an encoding difference, the same name and owner written with another tag or signature, pinned; an identity difference, a reference one writer writes and the other not, pinned or mapped to the observed S-T record that holds it, encoding-map.txt)"
else
  fail "encoding (< recorded, > now):"; diff "$R/expected/encoding.diff" "$W/encoding.diff" | head -20
fi
# 4. The pins: every difference recorded has its landing, and every pin a difference; an encoding difference by its
# pin, an identity difference of the encoding check by its pin or its line of encoding-map.txt (an observed S-T
# record that differs and has its pin); then the check's own mutations, each with the outcome it must have.
python3 "$R/oracle.py" pins "$R/pins.txt" "$R/expected/S-T.diff" "$R/expected/T-S.diff" "$R/expected/encoding.diff" \
  "$R/expected/probes.diff" --map "$R/encoding-map.txt" || status=1
python3 "$R/oracle.py" pins-selfcheck "$R/pins.txt" "$R/expected/S-T.diff" "$R/expected/T-S.diff" \
  "$R/expected/encoding.diff" "$R/encoding-map.txt" "$W/pins-self" "$R/expected/probes.diff" || status=1
# 5. Identity: dotty's own contract, as scalac 3.8.4 prints it.
if cmp -s "$c/identity.txt" "$R/dotty/identity.txt"; then
  echo "ok representation identity: dotty's identity rules as recorded"
else
  fail "identity (< recorded, > now):"; diff "$R/dotty/identity.txt" "$c/identity.txt" | head -10
fi
# 6. Dotty's hash buckets: the two effects teq declares and does not reproduce, as scalac 3.8.4 shows them (a record
# of dotty, never a case teq matches; DIFFERENCES.md, "Typing").
if cmp -s "$c/buckets.txt" "$R/dotty/buckets.txt"; then
  echo "ok representation buckets: dotty's two hash-bucket effects as recorded (a record, not a case teq matches)"
else
  fail "buckets (< recorded, > now):"; diff "$R/dotty/buckets.txt" "$c/buckets.txt" | head -10
fi
# 7. Coverage: every accessor of 3.8.4's quoted API (dotty/Api.scala, from scalac's own Quotes) is printed by the
# observer, derived from printed ones, or excluded with its kind (coverage.txt).
python3 "$R/oracle.py" coverage "$c/quotes-api.txt" "$R/coverage.txt" "$c" "$R/observe.scala" "$R/compiler/Scalac.scala" || status=1
# 8. Generation: scalac's reflection answering the generated traversal gives scalac's own observation, line for line,
# so that each record's key names its record alone and the traversal asks for every one (README.md, "Generation").
gen_ok=1
for x in api api8 cc38 cc39 js refining rec rec8 nulls; do
  if ! cmp -s "$c/ss-$x.txt" "$c/gs-$x.txt"; then
    gen_ok=0; fail "generation: the generated traversal of ss-$x is not scalac's observation (< direct, > generated):"
    diff "$c/ss-$x.txt" "$c/gs-$x.txt" | head -6 | cut -c1-200
  fi
done
[ $gen_ok = 1 ] && echo "ok representation generation: scalac's reflection answering each generated traversal gives its own observation, record by record (every cell teq reads and both variants)"
exit $status
