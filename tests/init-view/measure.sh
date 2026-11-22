#!/bin/bash
# tests/init-view/measure.sh <binary> <out dir> <corpus dir> <view binary> <init-view dir>: each unit of a corpus run's
# outputs (its <set>/<unit>/ directories, as tests/init-view-corpus.sh leaves them) compiled by <binary> as
# tests/init-view.sh compiles, printed by <view binary> (a teq with the initialization view), then poskinds.py against
# the view of scalac's own pickles, and <init-view dir>/regressions.py against the corpus run's own products (view.dump):
# the lines scalac's view has there and not here. A unit scalac or the base refuses, by the exit the corpus run
# recorded (tests/init-view/records.py), is named and counted. A unit whose corpus record is missing, empty, cut or past
# its bound, whose build passes its bound (120 s) or fails where the base compiled it, whose view fails or passes its
# bound (60 s), or whose comparison is not judged (a dump missing, empty, not what its sum says or holding its unit
# twice) or passes its bound (60 s) is NOT JUDGED, named in its set's count, never counted equal. Beside the head's view
# its exit and its sum (`view.exit`, `view.sum`, the head's side named in its id), beside the comparison's output its
# exit and its sum (`compare.exit`, `lost.sum`: tests/init-view/records.sh); the count is read from that output only
# where it is what its sum says and its count line, its lines and its exit agree. Exits 1 when a unit loses a line, when
# a unit is NOT JUDGED, when the out dir holds a unit the corpus does not (a fresh out dir, or this corpus's own), or
# when a self-test fails: regressions.py's and this script's own (`measure.sh --self-test`), both run first.
# Everything it writes, and the only thing it deletes, is under <out dir>. Env: COURSIER (default `cs`), JOBS (4).
#
# measure.sh --self-test: the measure on planted units of a made-up corpus, with a stand-in for the binaries (no teq, no
# scalac, no coursier), its records sealed as tests/init-view-corpus.sh seals them: a good corpus (two judged units,
# one scalac refuses, one the base refuses) passes; each record of the judged unit missing, empty or cut on scalac's,
# the base's or the head's side, a step past its bound or failed, a line the head places otherwise, a dump holding its
# unit twice, a unit with no record of the corpus run, and a unit of the out dir the corpus does not hold fail; each of
# the corpus run's dumps and the comparison's output changed after its producer sealed it (its last line gone with its
# line break and exit kept, a line gone from its middle, a byte changed, its sum gone, another unit's copied in), the
# comparison's exit gone or other than its output says, are NOT JUDGED; so is an exit other than 0 or a refusal named
# by its line on its record (137, 2, 124, 139, none, 1 with no error line, 3 without the JVM backend's line, scalac's 3)
# of scalac's, the base's check, the head's build or the comparison; teq's 1 with the TASTy writer's summary and its 3
# with the JVM backend's are refusals. Prints `measure.sh self-test: N cases, N as expected`.
set -u
here=$(cd "$(dirname "$0")" && pwd)
. "$here/records.sh"

one() {
  d=${1%/}; set=$(basename "$(dirname "$d")"); u=$(basename "$d"); o=$W/$set/$u
  mkdir -p "$o/oracle"
  rm -f "$o/not-judged" "$o/refused" "$o/lost.txt" "$o/lost.sum" "$o/compare.exit" "$o/view.dump" "$o/view.sum" \
    "$o/check.exit" "$o/view.exit" "$o/scalac-view.dump" "$o/scalac-view.sum" "$o/oracle/pickler.dump" "$o/oracle/pickler.sum"
  # scalac's dumps beside the head's for the census, each with its sum.
  for f in scalac-view.dump scalac-view.sum oracle/pickler.dump oracle/pickler.sum; do cp "$d/$f" "$o/$f" 2> /dev/null; done
  # The corpus run's record of the unit: scalac's side, then the base's.
  state=$(cd "$V" && python3 -c 'import sys, records
s = records.corpus_unit(sys.argv[1])
bad = [v for v in (s["scalac"], s["base"]) if v[0] != "ok"]
print("ok" if not bad else bad[0][0] + ": " + bad[0][1])' "$d")
  case "$state" in
    ok) ;;
    refused:*) echo "${state#refused: }" > "$o/refused"; return ;;
    *) echo "${state#not judged: }" > "$o/not-judged"; return ;;
  esac
  (cd "$d/src" && timeout 120 "$BIN" compiler check ./*.scala --std scala-library --products "$o/p") > "$o/check.txt" 2>&1
  rc=$?
  echo $rc > "$o/check.exit"
  if [ $rc = 124 ]; then
    echo "the build passed 120 s" > "$o/not-judged"
  elif [ $rc != 0 ] || [ ! -d "$o/p" ]; then
    echo "the build failed (exit $rc) where the base compiled the unit" > "$o/not-judged"
  else
    TEQ_INIT_VIEW_CLASSPATH=$J TEQ_INIT_VIEW=$(cd "$d/src" && pwd) timeout 60 "$VIEW" tasty "$o/p" > "$o/view.dump" 2> /dev/null
    rc=$?
    echo $rc > "$o/view.exit"
    record_seal "$o" "$o/view.dump" head
    [ $rc = 0 ] || echo "the view failed or passed 60 s (exit $rc)" > "$o/not-judged"
  fi
  case "$o" in "$W"/*) rm -rf "${o:?}/p" ;; esac
  [ -e "$o/not-judged" ] && return
  (cd "$V" && timeout 60 python3 regressions.py "$d/scalac-view.dump" "$d/view.dump" "$o/view.dump") > "$o/lost.txt" 2>&1
  rc=$?
  echo $rc > "$o/compare.exit"
  record_seal "$o" "$o/lost.txt"
  [ $rc -le 1 ] || echo "the comparison: $(tail -1 "$o/lost.txt") (exit $rc)" > "$o/not-judged"
}

# measure_setup <binary> <out dir> <corpus dir> <view binary> <init-view dir>: the run's binaries, dirs and library.
measure_setup() {
  BIN=${1:?binary}; W=${2:?out dir}; C=${3:?corpus dir}; VIEW=${4:?view binary}; V=${5:?init-view dir}
  if [ -n "${MEASURE_LIBRARY:-}" ]; then
    J=$MEASURE_LIBRARY
  else
    J=$("${COURSIER:-cs}" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
    [ -n "$J" ] || { echo "measure: scala-library 3.8.4 not fetched"; return 1; }
  fi
  mkdir -p "$W"
  W=$(cd "$W" && pwd)
  export -f one record_line record_seal
  export W BIN V VIEW J
}

# measure_units: each unit of the corpus measured (`one`), JOBS at a time.
measure_units() {
  ls -d "$C"/*/*/ | xargs -P "${JOBS:-4}" -I{} bash -c 'one "$@"' _ {}
}

# measure_census: per set the positions' census, the lines lost, the refusals, each unit not judged, the out dir
# against the corpus; exits 1 on a lost line, a unit not judged or a manifest problem.
measure_census() {
  python3 - "$C" "$W" "$V" "$here" <<'PY'
import os, re, subprocess, sys
C, W, V, here = sys.argv[1:5]
sys.path.insert(0, V)
import records


def compared(o):
    """(lines lost, None) of a unit's comparison, or (None, why): its output whole by its sum, its count line, its
    lines and its exit (regressions.py's: 0 none lost, 1 some) agreeing."""
    e = records.exit_code(os.path.join(o, "compare.exit"))
    if e is None:
        return None, "no record of the comparison's exit"
    if e not in (0, 1):
        return None, f"the comparison not judged (exit {e})"
    lines, why = records.text(os.path.join(o, "lost.txt"))
    if why:
        return None, f"the comparison's output {why}"
    worse = sum(1 for l in lines if l.startswith("  scalac"))
    said = [int(m.group(1)) for m in (re.match(r"(\d+) aligned lines equal to scalac's", l) for l in lines) if m]
    if len(said) != 1 or said[0] != worse:
        return None, f"the comparison's output holds {worse} lost lines and its count line says {said or 'nothing'}"
    if (worse > 0) != (e == 1):
        return None, f"the comparison's exit {e} where its output holds {worse} lost lines"
    return worse, None


bad = 0
for st in sorted(x for x in os.listdir(C) if os.path.isdir(os.path.join(C, x))):
    units = sorted(x for x in os.listdir(os.path.join(C, st)) if os.path.isdir(os.path.join(C, st, x)))
    out = os.path.join(W, st)
    have = sorted(x for x in os.listdir(out) if os.path.isdir(os.path.join(out, x))) if os.path.isdir(out) else []
    problems = records.manifest(units, have)
    print(f"=== {st}")
    census = subprocess.run([sys.executable, os.path.join(here, "poskinds.py"), V] + [os.path.join(out, u) + "/" for u in units],
                            env=dict(os.environ, POS_ORACLE="scalac-view.dump"), capture_output=True, text=True, timeout=600)
    print((census.stdout.splitlines() or ["poskinds: no output"])[0])
    lost = refused = 0
    not_judged = []
    for u in units:
        o = os.path.join(out, u)
        if os.path.exists(os.path.join(o, "refused")):
            refused += 1
        elif os.path.exists(os.path.join(o, "not-judged")):
            not_judged.append((u, open(os.path.join(o, "not-judged"), encoding="utf-8").read().strip()))
        elif not os.path.isdir(o):
            not_judged.append((u, "no record of its measure"))
        else:
            n, why = compared(o)
            if why:
                not_judged.append((u, why))
            else:
                lost += n
    print(f"lost against the corpus run's products: {lost}")
    print(f"refused, by the exit the corpus run recorded: {refused}")
    for u, why in not_judged:
        print(f"NOT JUDGED {u}: {why}")
    for p in problems:
        print(f"MANIFEST {st}: {p}")
    bad += lost + len(not_judged) + len(problems)
print(f"measure: {bad} lost lines, units not judged and manifest problems")
sys.exit(1 if bad else 0)
PY
}

measure() {
  measure_setup "$@" || return 1
  measure_units
  measure_census
}

self_test() {
  local root cases=0 good=0 wrong="" name
  root=$(mktemp -d "${TMPDIR:-/tmp}/measure-selftest-XXXXXX")
  mkdir -p "$root/bin"
  # A stand-in for teq: `compiler check .. --products <p>` does what the unit's `stand-in` file says (`ok`, `refuse`,
  # `124`, `view-124`) and leaves the unit's planted `head.dump` with the products; `tasty <p>` prints it.
  cat > "$root/bin/teq" <<'SH'
#!/bin/bash
if [ "$1" = compiler ]; then
  p=${!#}; mode=$(cat ../stand-in 2> /dev/null || echo ok)
  case "$mode" in 124 | 137 | 139 | 2) exit "$mode" ;; refuse) echo "error: refused"; exit 1 ;; esac
  mkdir -p "$p"; echo "$mode" > "$p/mode"; [ -f ../head.dump ] && cp ../head.dump "$p/dump"; exit 0
fi
[ "$(cat "$2/mode" 2> /dev/null)" = view-124 ] && exit 124
[ -f "$2/dump" ] || exit 1
cat "$2/dump"
SH
  chmod +x "$root/bin/teq"
  local dump='unit src/cell.scala
source s1 name=cell.scala path=src/cell.scala content=yes sha256=0
  PackageDef [s1 0..30@0 1:0-2:10] pid=<empty>
    DefDef [s1 0..30@4 1:0-2:10] sym=<empty>.cell$package$.f
      Ident [s1 10..11@10 1:10-1:11] sym=<empty>.cell$package$.g
'
  # corpus <dir>: the good corpus, set `s`: `good` and `twin` judged, `sref` scalac refuses, `bref` the base refuses;
  # each output sealed as the corpus run seals it.
  corpus() {
    local c=$1/corpus/s u
    for u in good twin sref bref; do mkdir -p "$c/$u/src" "$c/$u/oracle"; echo 'val x = 1' > "$c/$u/src/cell.scala"; done
    echo 1 > "$c/sref/oracle/exit"
    printf -- '-- Error: cell.scala:1:4 \n1 |val x = 1\n1 error found\n' > "$c/sref/oracle/scalac.txt"
    record_seal "$c/sref" "$c/sref/oracle/scalac.txt"
    for u in good twin bref; do
      echo 0 > "$c/$u/oracle/exit"; printf '%s' "$dump" > "$c/$u/scalac-view.dump"; echo 0 > "$c/$u/scalac-view.exit"
      printf '%s' "$dump" > "$c/$u/oracle/pickler.dump"
      record_seal "$c/$u" "$c/$u/scalac-view.dump"; record_seal "$c/$u" "$c/$u/oracle/pickler.dump"
    done
    echo 1 > "$c/bref/check.exit"
    printf 'cell.scala:1:9: error: new Array takes one type argument\n1 error found\n' > "$c/bref/check.txt"
    record_seal "$c/bref" "$c/bref/check.txt"
    for u in good twin; do
      echo 0 > "$c/$u/check.exit"; printf '%s' "$dump" > "$c/$u/view.dump"; echo 0 > "$c/$u/view.exit"
      record_seal "$c/$u" "$c/$u/view.dump"
      printf '%s' "$dump" > "$c/$u/head.dump"
    done
  }
  # case <name> <expected exit> <text the output must hold> <mutation of the good unit, run in its directory>
  # [<mutation of the good unit's measure, run in its directory of the out dir between the units and the census>]
  case_() {
    local name=$1 want=$2 holds=$3 mutation=$4 between=${5:-} d rc out
    d=$root/$cases
    mkdir -p "$d"
    corpus "$d"
    (cd "$d/corpus/s/good" && eval "$mutation")
    out=$(MEASURE_LIBRARY=none JOBS=2
          measure_setup "$root/bin/teq" "$d/out" "$d/corpus" "$root/bin/teq" "$here" && measure_units &&
          { [ -z "$between" ] || (cd "$d/out/s/good" && eval "$between"); } && measure_census 2>&1)
    rc=$?
    cases=$((cases + 1))
    if [ "$rc" = "$want" ] && case "$out" in *"$holds"*) true ;; *) false ;; esac; then
      good=$((good + 1))
    else
      wrong="$wrong
  NOT AS EXPECTED: $name: exit $rc, $(echo "$out" | grep -E 'NOT JUDGED|lost|MANIFEST' | head -2 | tr '\n' ' ')"
    fi
  }
  case_ "a good corpus" 0 "refused, by the exit the corpus run recorded: 2" ":"
  for f in scalac-view.dump view.dump head.dump; do
    case_ "$f missing" 1 "NOT JUDGED good" "rm $f"
    case_ "$f empty" 1 "NOT JUDGED good" ": > $f; [ $f = head.dump ] || record_seal . $f"
  done
  for f in scalac-view.dump view.dump; do
    case_ "$f cut" 1 "NOT JUDGED good" "head -c 60 $f > $f.cut && mv $f.cut $f"
  done
  # The head's view cut by its producer (the view exits 0 having printed 60 bytes): judged, its lines lost.
  case_ "head.dump cut" 1 "lost against the corpus run's products: 4" "head -c 60 head.dump > h.cut && mv h.cut head.dump"
  for f in oracle/exit scalac-view.exit view.exit; do
    case_ "$f missing" 1 "NOT JUDGED good" "rm $f"
    case_ "$f 124" 1 "NOT JUDGED good" "echo 124 > $f"
  done
  case_ "the base's check past its bound" 1 "NOT JUDGED good" "rm view.exit; echo 124 > check.exit"
  case_ "the head's build past its bound" 1 "NOT JUDGED good" "echo 124 > stand-in"
  case_ "the head's build refusing what the base compiles" 1 "NOT JUDGED good" "echo refuse > stand-in"
  case_ "the head's view past its bound" 1 "NOT JUDGED good" "echo view-124 > stand-in"
  case_ "a line the head places otherwise" 1 "lost against the corpus run's products: 1" "sed -i 's/Ident \[s1 10..11@10/Ident [s1 10..11@11/' head.dump"
  case_ "a dump holding its unit twice" 1 "NOT JUDGED good" "cat head.dump head.dump > h2 && mv h2 head.dump"
  case_ "a unit with no record of the corpus run" 1 "NOT JUDGED good" "rm -r src"
  case_ "a unit of the out dir the corpus does not hold" 1 "MANIFEST s" "mkdir -p ../../../out/s/stray"
  # The corpus run's dumps changed after their producer sealed them: NOT JUDGED for what the sum says.
  for f in scalac-view.dump view.dump; do
    s=${f%.*}.sum
    case_ "$f: its last line gone, its line break and exit kept" 1 "other than its producer wrote" "sed -i '\$d' $f"
    case_ "$f: a line gone from its middle" 1 "other than its producer wrote" "sed -i '3d' $f"
    case_ "$f: a byte of it changed, its length kept" 1 "other than its producer wrote" "sed -i '1s/^unit/xnit/' $f"
    case_ "$f: its sum gone" 1 "with no sum of what its producer wrote" "rm $s"
    case_ "$f: another unit's record copied in (its output and sum)" 1 "another record's (twin/$f" "cp ../twin/$f ../twin/$s ."
  done
  # The comparison's output changed after measure sealed it, between the units and the census. The head places two
  # lines otherwise: its output holds two lost lines and its count line.
  two="sed -i 's/Ident \[s1 10..11@10/Ident [s1 10..11@11/; s/DefDef \[s1 0..30@4/DefDef [s1 0..30@5/' head.dump"
  case_ "the comparison's output: its last line gone, its line break and exit kept" 1 "NOT JUDGED good: the comparison's output other than its producer wrote" "$two" "sed -i '\$d' lost.txt"
  case_ "the comparison's output: a line gone from its middle" 1 "NOT JUDGED good: the comparison's output other than its producer wrote" "$two" "sed -i '2d' lost.txt"
  case_ "the comparison's output: a byte of it changed, its length kept" 1 "NOT JUDGED good: the comparison's output other than its producer wrote" "$two" "sed -i '1s/scalac/xcalac/' lost.txt"
  case_ "the comparison's output: its sum gone" 1 "NOT JUDGED good: the comparison's output with no sum" "$two" "rm lost.sum"
  case_ "the comparison's output: another unit's copied in (its output and sum)" 1 "NOT JUDGED good: the comparison's output another record's" "$two" "cp ../twin/lost.txt ../twin/lost.sum ."
  # One line lost: its line gone from the comparison's output would leave none counted.
  case_ "the comparison's output: its one lost line gone" 1 "NOT JUDGED good: the comparison's output other than its producer wrote" \
    "sed -i 's/Ident \[s1 10..11@10/Ident [s1 10..11@11/' head.dump" "sed -i '1d' lost.txt"
  case_ "the comparison's exit gone" 1 "NOT JUDGED good: no record of the comparison's exit" ":" "rm compare.exit"
  case_ "the comparison's exit other than its output says" 1 "NOT JUDGED good: the comparison's exit 1 where its output holds 0 lost lines" ":" "echo 1 > compare.exit"
  # An exit by the whitelist: 0, or 1 with the compiler's error line on its record. A signal's (137 a kill, 139 a
  # segmentation fault), 2, 124, none recorded and 1 with no error line are NOT JUDGED: scalac's of the unit it refuses,
  # the base's check of the unit it refuses, the head's build, and the comparison's.
  for e in 137 2 124 139 missing; do
    put="echo $e >"; [ $e = missing ] && put="rm"
    case_ "scalac's exit of the refused unit $e" 1 "NOT JUDGED sref" "$put ../sref/oracle/exit"
    case_ "the base's check's exit of the refused unit $e" 1 "NOT JUDGED bref" "$put ../bref/check.exit"
    [ $e = missing ] || [ $e = 124 ] || case_ "the head's build $e" 1 "NOT JUDGED good" "echo $e > stand-in"
    case_ "the comparison's exit $e" 1 "NOT JUDGED good" ":" "$put compare.exit"
  done
  case_ "scalac's exit of the refused unit 1 with no error line" 1 "NOT JUDGED sref" "echo compiling > ../sref/oracle/scalac.txt; record_seal ../sref ../sref/oracle/scalac.txt"
  case_ "the base's check's exit of the refused unit 1 with no error line" 1 "NOT JUDGED bref" "echo compiling > ../bref/check.txt; record_seal ../bref ../bref/check.txt"
  # teq's two refusals closed by a summary line (src/main.rs): 1 with the TASTy writer's, 3 with the JVM backend's, a
  # refusal of the base; 3 without that line, and scalac's 3, are not.
  case_ "the base's check's exit of the refused unit 1 with an internal error before the TASTy writer's summary" 1 "NOT JUDGED bref" \
    "printf 'internal error: cell.scala: parsed without the record of the places the TASTy writer reads\\n1 definition cannot be written as TASTy\\n' > ../bref/check.txt; record_seal ../bref ../bref/check.txt"
  case_ "the base's check's exit of the refused unit 1 with the TASTy writer's summary" 0 "refused, by the exit the corpus run recorded: 2" \
    "printf 'cell.scala: cannot write TASTy for N1\\n1 definition cannot be written as TASTy\\n' > ../bref/check.txt; record_seal ../bref ../bref/check.txt"
  case_ "the base's check's exit of the refused unit 3 with the JVM backend's summary" 0 "refused, by the exit the corpus run recorded: 2" \
    "echo 3 > ../bref/check.exit; printf 'jvm backend: the local o is not in scope\\n2 constructs not supported by the JVM backend\\n' > ../bref/check.txt; record_seal ../bref ../bref/check.txt"
  case_ "the base's check's exit of the refused unit 3 without the JVM backend's line" 1 "NOT JUDGED bref" "echo 3 > ../bref/check.exit"
  case_ "scalac's exit of the refused unit 3" 1 "NOT JUDGED sref" "echo 3 > ../sref/oracle/exit"
  case "$root" in "${TMPDIR:-/tmp}"/measure-selftest-*) rm -rf "$root" ;; esac
  [ -n "$wrong" ] && echo "$wrong" | grep .
  echo "measure.sh self-test: $cases cases, $good as expected"
  [ "$cases" = "$good" ]
}

if [ "${1:-}" = --self-test ]; then
  self_test
  exit
fi
# The comparison's and the measure's self-tests on planted records first: a judge that passes a bad record judges nothing.
(cd "$here" && python3 regressions.py --self-test) || exit 1
self_test || exit 1
measure "$@"
