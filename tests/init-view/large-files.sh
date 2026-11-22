#!/bin/bash
# tests/init-view/large-files.sh <master teq> <head teq> <out dir> [rounds]: the CPU a `compiler check` takes of five
# generated files, master's binary against the head's, with `--products` (the TASTy writer, which reads the parser's
# record of places) and without (the record is not kept), each side run `rounds` times (3 by default) in turn and its
# least user+sys time taken. The shapes, each one file `Large.scala`:
#   u80     80 top-level objects of 250 lines (20,000 lines, as the re-check's large3.sh writes them): each holds
#           `case class P(x: Int, y: Int)` and 248 lines cycling a selection written with comments and spaces, a lambda
#           chain and a context-bounded method;
#   u160    the same, 160 objects (40,000 lines);
#   u1      one object of 20,000 such lines;
#   small   2,000 objects of 10 lines each (many small units);
#   nested  one object holding 80 objects of 250 lines (one unit, nested objects).
# Per shape and mode: `within`, where the head's least time is at most master's plus LARGE_TOLERANCE percent (5 by
# default), else `OVER`; and on u1 with --products the head's least time below master's (the one-unit gain kept). A
# run that exits otherwise than 0, past its bound (LARGE_BOUND seconds, 300 by default), without its time on record
# or with a time that is no number is NOT JUDGED and fails the run. Prints a line per shape and mode and a last line
# `large-files: <n> lines, <w> within, <o> over, <j> not judged; one-unit gain kept|lost`; exits 1 on an OVER or a lost
# gain, 2 on a NOT JUDGED. The out dir must be absolute, new or one this script wrote (`.large-files` in it).
#
# tests/init-view/large-files.sh --self-test: the judge over stand-in binaries that burn a set CPU time, exit otherwise, outlast
# their bound or print no time: each planted bad record fails the run (`large-files self-test: N cases, M as
# expected`).
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
tol=${LARGE_TOLERANCE:-5}
bound=${LARGE_BOUND:-300}

generate() { # generate <dir>: the five shapes, each <dir>/<shape>/Large.scala
  python3 - "$1" <<'PY'
import os, sys
root = sys.argv[1]
lines = ["  def f{i}(p: P) = p /* . */ .x . toString . length match {{ case 1 => {i} case _ => p.y }}",
         "  val g{i} = List({i}).map(i => i + 1).filter((j: Int) => j > 0)",
         "  def h{i}[T: Ordering](xs: List[T]) = xs . sorted . headOption"]
def body(n, indent=""):
    out = [indent + "  case class P(x: Int, y: Int)"]
    out += [indent + lines[i % 3].format(i=i) for i in range(n)]
    return out
def write(shape, text):
    d = os.path.join(root, shape)
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, "Large.scala"), "w").write("\n".join(text) + "\n")
def objects(count, per):
    text = []
    for o in range(count):
        text.append(f"object L{o}:")
        text += body(per)
    return text
write("u80", objects(80, 248))
write("u160", objects(160, 248))
write("u1", ["object L0:"] + body(20000))
write("small", objects(2000, 9))
nested = ["object Outer:"]
for o in range(80):
    nested.append(f"  object N{o}:")
    nested += ["  " + l for l in body(248)]
write("nested", nested)
PY
}

claim_out() { # claim_out <dir>: an absolute out dir, new or this script's
  local d=$1
  case "$d" in /*) ;; *) echo "large-files: the out dir must be an absolute path: $d" >&2; return 2 ;; esac
  if [ -d "$d" ] && [ -n "$(ls -A "$d")" ] && [ ! -f "$d/.large-files" ]; then
    echo "large-files: $d holds files this script did not write; refused" >&2; return 2
  fi
  mkdir -p "$d" && : > "$d/.large-files"
}

one_run() { # one_run <teq> <shape dir> <mode> <record>: one check's time, `<user> <sys>` and its exit on the record
  local teq=$1 src=$2 mode=$3 rec=$4 p=()
  if [ "$mode" = products ]; then
    case "$rec" in */runs/*) rm -rf "$rec.products" ;; esac
    p=(--products "$rec.products")
  fi
  (cd "$src" && /usr/bin/time -f "%U %S" -o "$rec.time" timeout "$bound" "$teq" compiler check Large.scala --std scala-library "${p[@]}" > "$rec.log" 2>&1)
  echo $? > "$rec.exit"
  case "$rec" in */runs/*) rm -rf "$rec.products" ;; esac
}

judge_runs() { # judge_runs <runs dir> <shape> <mode> <rounds>: `<state> <master least> <head least>` or NOT JUDGED why
  local d=$1 shape=$2 mode=$3 rounds=$4 side r rec e t best why=""
  local -A least=()
  for side in master head; do
    best=""
    for r in $(seq 1 "$rounds"); do
      rec=$d/$shape-$mode-$side-$r
      e=$(cat "$rec.exit" 2> /dev/null)
      case "$e" in
        0) ;;
        124) why="$side's run $r past its bound of $bound s"; break 2 ;;
        '') why="$side's run $r left no exit"; break 2 ;;
        *) why="$side's run $r exited $e"; break 2 ;;
      esac
      t=$(tail -1 "$rec.time" 2> /dev/null | awk 'NF == 2 && $1 ~ /^[0-9.]+$/ && $2 ~ /^[0-9.]+$/ { printf "%.2f", $1 + $2 }')
      [ -n "$t" ] || { why="$side's run $r left no time on record"; break 2; }
      if [ -z "$best" ] || awk -v a="$t" -v b="$best" 'BEGIN { exit !(a < b) }'; then best=$t; fi
    done
    least[$side]=$best
  done
  if [ -n "$why" ]; then echo "NOT-JUDGED $why"; return; fi
  if awk -v h="${least[head]}" -v m="${least[master]}" -v t="$tol" 'BEGIN { exit !(h <= m * (1 + t / 100)) }'; then
    echo "within ${least[master]} ${least[head]}"
  else
    echo "OVER ${least[master]} ${least[head]}"
  fi
}

measure() { # measure <master> <head> <out> <rounds>
  local master=$1 head=$2 out=$3 rounds=$4 shape mode r side v
  claim_out "$out" || return 2
  case "$out/shapes" in /*/shapes) rm -rf "$out/shapes" "$out/runs" ;; esac
  mkdir -p "$out/runs"
  generate "$out/shapes" || { echo "large-files: the shapes could not be written"; return 2; }
  local n=0 w=0 o=0 j=0 gain=kept
  for shape in u80 u160 u1 small nested; do
    for mode in products plain; do
      for r in $(seq 1 "$rounds"); do
        for side in master head; do
          v=$master; [ $side = head ] && v=$head
          one_run "$v" "$out/shapes/$shape" "$mode" "$out/runs/$shape-$mode-$side-$r"
        done
      done
      set -- $(judge_runs "$out/runs" "$shape" "$mode" "$rounds")
      n=$((n + 1))
      case "$1" in
        within) w=$((w + 1)); echo "large-files: $shape $mode: within $tol%: master ${2} s, head ${3} s" ;;
        OVER) o=$((o + 1)); echo "large-files: $shape $mode: OVER $tol%: master ${2} s, head ${3} s" ;;
        *) j=$((j + 1)); shift; echo "large-files: $shape $mode: NOT JUDGED: $*" ;;
      esac
      if [ "$shape-$mode" = u1-products ]; then
        if [ "$1" = NOT-JUDGED ] || ! awk -v h="${3:-x}" -v m="${2:-x}" 'BEGIN { exit !(h + 0 < m + 0) }'; then gain=lost; fi
      fi
    done
  done
  echo "large-files: $n lines, $w within, $o over, $j not judged; one-unit gain $gain"
  [ $j = 0 ] || return 2
  [ $o = 0 ] && [ $gain = kept ] || return 1
  return 0
}

self_test() {
  local tmp n=0 good=0
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/large-files-self-test.XXXXXX") || { echo "large-files self-test: no scratch directory"; return 1; }
  # A stand-in teq: burns about <loops> of CPU, then exits <code> (or sleeps past the bound when the code is `slow`).
  stand_in() { # stand_in <path> <loops> <code>
    printf '#!/bin/bash\ni=0; while [ $i -lt %s ]; do i=$((i+1)); done\n[ "%s" = slow ] && sleep 5\n[ "%s" = slow ] && exit 0\nexit %s\n' "$2" "$3" "$3" "$3" > "$1"
    chmod +x "$1"
  }
  check() { # check <label> <want exit> <want line part> <master loops> <head loops> [head code] [bound]
    local label=$1 want=$2 part=$3 got rc
    n=$((n + 1))
    stand_in "$tmp/m" "$4" 0
    stand_in "$tmp/h" "$5" "${6:-0}"
    got=$(LARGE_BOUND=${7:-60} LARGE_TOLERANCE=5 bash "$here/large-files.sh" "$tmp/m" "$tmp/h" "$tmp/out-$n" 1 2>&1)
    rc=$?
    if [ "$rc" = "$want" ] && grep -qF -- "$part" <<< "$got"; then good=$((good + 1)); else echo "large-files self-test: NOT AS EXPECTED: $label: exit $rc ($want wanted); $(tail -2 <<< "$got" | tr '\n' ' ')"; fi
  }
  check "equal work on both sides, the one-unit gain lost (the stand-ins take one time everywhere)" 1 "one-unit gain lost" 20000 20000
  check "the head three times master's work" 1 "OVER 5%" 20000 60000
  check "the head exits 1" 2 "head's run 1 exited 1" 20000 20000 1
  check "the head past its bound" 2 "head's run 1 past its bound of 2 s" 20000 20000 slow 2
  rm -f "$tmp/no-time"
  n=$((n + 1))
  # A time record cut or missing: the judge alone over planted records.
  mkdir -p "$tmp/runs"
  for side in master head; do echo 0 > "$tmp/runs/u80-products-$side-1.exit"; echo "1.00 0.10" > "$tmp/runs/u80-products-$side-1.time"; done
  : > "$tmp/runs/u80-products-head-1.time"
  if [ "$(judge_runs "$tmp/runs" u80 products 1)" = "NOT-JUDGED head's run 1 left no time on record" ]; then good=$((good + 1)); else echo "large-files self-test: NOT AS EXPECTED: an empty time record"; fi
  n=$((n + 1))
  echo "1.00 x" > "$tmp/runs/u80-products-head-1.time"
  if [ "$(judge_runs "$tmp/runs" u80 products 1)" = "NOT-JUDGED head's run 1 left no time on record" ]; then good=$((good + 1)); else echo "large-files self-test: NOT AS EXPECTED: a time that is no number"; fi
  n=$((n + 1))
  rm -f "$tmp/runs/u80-products-master-1.exit"
  if [ "$(judge_runs "$tmp/runs" u80 products 1)" = "NOT-JUDGED master's run 1 left no exit" ]; then good=$((good + 1)); else echo "large-files self-test: NOT AS EXPECTED: a missing exit"; fi
  n=$((n + 1))
  for side in master head; do echo 0 > "$tmp/runs/u80-products-$side-1.exit"; done
  echo "1.00 0.10" > "$tmp/runs/u80-products-head-1.time"
  if [ "$(judge_runs "$tmp/runs" u80 products 1)" = "within 1.10 1.10" ]; then good=$((good + 1)); else echo "large-files self-test: NOT AS EXPECTED: equal times are within"; fi
  n=$((n + 1))
  if ! claim_out relative/dir 2> /dev/null; then good=$((good + 1)); else echo "large-files self-test: NOT AS EXPECTED: a relative out dir taken"; fi
  case "$tmp" in "${TMPDIR:-/tmp}"/large-files-self-test.?*) rm -rf "$tmp" ;; esac
  echo "large-files self-test: $n cases, $good as expected"
  [ "$good" = "$n" ]
}

if [ "${1:-}" = --self-test ]; then
  self_test
  exit $?
fi
[ $# -ge 3 ] || { echo "usage: tests/init-view/large-files.sh <master teq> <head teq> <out dir> [rounds] | --self-test" >&2; exit 2; }
measure "$1" "$2" "$3" "${4:-3}"
