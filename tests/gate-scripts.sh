#!/bin/bash
# Checks of the gate's harnesses' own decisions, each against a stand-in that fails as a loaded or broken machine
# fails (docs/DEVELOPING.md, "The landing gate"):
#   oracle   tests/modules.sh over its misuse case with scala-cli a stand-in that exits as a timeout (124), is
#            killed (137) or exits 1 before scalac reports (a dependency not resolved offline): each fails the
#            case's negative check with the status in its detail; scalac 3.8.4 itself refuses it (the control).
#   workers  tests/modules.sh over two cases at two at once, the first case's worker killed by its first build: the
#            case fails as one whose worker ended without its counts, the other's lines and counts still printed in
#            the cases' order, and the run exits 1.
#   parts    tests/tasty-exec.scala over six programs of its closure with a java whose first part of the
#            regenerations is killed at once: the suite fails, naming the part, its status and its stderr, and keeps
#            its work directory.
#   nodes    the runtime line's node and java are bench/runtime-machine.txt's by their recorded paths: with another
#            node first on the PATH, bench/pairs.py's preflight takes the pinned one; a pinned path with nothing
#            there refuses the line (exit 2), and --unpinned takes the PATH's; tests/asymptotics.sh runs the node
#            NODE names and says which.
#   skips    tests/gate.sh's want-of rule over each runner's wording: the by-design skips admitted (Scala.js's
#            expectations, another system's, another std's, a budget of one platform, the JVM's platform, a split
#            watch case of another std or of the JVM alone), the want-of ones refused (a jar, a tool, a note of a
#            check a tool's absence leaves undone).
#   pieces   the long suites' pieces (tests/support/pieces.sh): each of lsp, jvm-watch, modules and split deals every
#            unit it lists to exactly one piece of 1 to 8, in its order, its listing of three the units dealt; the
#            module-only checks and split programs, the programs on both of jvm-watch's lists and the language
#            server's idle minute, jar fetch and libraries' sources among the units; selectors out of range or misspelled refused before
#            any setup, another option refused there too, a piece of no units passed; every listed unit asked for once
#            by the suite's places that run units (PIECES_DRY=1); a unit asked for twice, never or unlisted, a body
#            run twice, never or unasked failing a stand-in suite's piece; the execution identities, each unit's body
#            stamping its name and site as it starts over stand-in binaries (PIECES_STAMP_ONLY=1): every piece of
#            every n of 1 to 8 running each site of each of its units once, the pieces of each n together the whole
#            run's bodies and the whole run's 1/1's (tests/lsp.sh's scenarios each at the automatic count, two and
#            sixteen workers, the idle minute, the jar fetch and the libraries' sources once); on a copy of tests/jvm-watch.sh, a program's body
#            run twice behind its guard, another's dropped behind it, a section's dropped, a retype session called
#            twice and another's call deleted each failing the piece;
#            tests/support/pieces.py's merge summing counters, counting a whole suite's verdict and a failure any
#            piece finds once, summing evidence before its check, and failing on a piece missing, doubled, unexpected,
#            of another number, without its summary or failed; tests/all.sh running the four whole.
#   records  the reference projects' records (bench/reference/records.scala) through bench/reference/ratchet.scala's
#            verbs over a stand-in project in a repository of its own: a baseline from a transcript, its paths the
#            checkout's and its evidence beside it, a rerun within it; an error outside the record failing, printed in
#            full; one gone passing, listed; one twice where the record holds it once, and one fixed beside a new one
#            of its message a line lower, failing; the same errors from another checkout's root passing; an empty or
#            cut transcript, a timeout, a configuration not covered or covered twice, a panic, a daemon gone, a count
#            or an exit not the diagnostics' each failing; a failed run's --record leaving the record as it was; a
#            passing improvement recorded, staged and installed, the check against master's counting it; after it
#            landed, the error reintroduced failing the run and the record widened back failing the check; a record
#            widened beside the run that needs it passing the run and failing the check; a baseline over a record
#            and a migration over unchanged inputs refused; new generated inputs failing the run until a migration,
#            admitted with its reason and evidence; a record removed failing the check and its line, a deliberately
#            empty one holding a clean run to none; a configuration whose build failed outright one item of its whole
#            block (a generator's run under teq interp, its diagnostics among it), the next configuration's summary
#            claiming its own, another explanation of the same first line failing; a crash, a panic and an error
#            counted without its diagnostic recorded by no baseline or migration and failing a judged run; a baseline
#            over a configuration covered twice recording nothing; a project's directory removed whole, its pin with
#            its records, failing the check; a head forked before master stepped a record down refused against master
#            until rebased onto it.
#   setup    bench/gate-projects/setup.scala over a stand-in project in a copy of the tree: a machine made ready, its
#            npm fixture deleted and installed again by a second run before the machine is ready again, a run whose
#            checkout fails removing the ready mark.
#   driver   tests/gate.sh over stand-in machines (tests/gate-agent; cargo and rustc stand-ins whose build writes a
#            binary naming the revision asked for and running TEQ), on a copy of this tree in a repository of its
#            own: the first shard's machine builds the head and the others take its binary, one whose
#            copy is spoilt refuses it and builds; the machine whose pool line holds lila is left to the project
#            lines, which fail there without the checkout, and fail as not run without such a machine; the eighteen
#            project lines run unnamed on the holder of the five projects, --without leaving one out; --record
#            leaves tests/gate-times.txt as it was after a failing run and records the suites' times after a passing
#            one; the last line is the gate's total; and with five machines the comparison (bench/pairs.py a stand-in
#            in the landing) prepares three, each over the builder's binary, and measures its lines after the shards'
#            suites end on one of them, on three when the recorded times put one machine's end past thirty minutes; a
#            run whose only line not passed is a timing line inconclusive records the times; and with tests/lsp.sh a
#            stand-in of six units, its four pieces dealt as lines of their own under the suite's merged row, their
#            times recorded and the suite's left, then read again by the dealer and the comparison's planner beside a
#            planted failure, a killed piece, one past the suite's bound and a skip for want of a jar, each failing
#            the suite, nothing recorded.
# Every run is bounded; the script exits 1 on a failure. Needs the release binary (TEQ), scala-cli and the coursier
# cache the suites read.
cd "$(dirname "$0")/.." || exit 1
export TEQ=${TEQ:-$PWD/target/release/teq}
work=$(mktemp -d "${TMPDIR:-/tmp}/gate-scripts.XXXXXX") || exit 1
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
# check <name> <log> <command...>: the command's success counts; a failure prints the log's last lines.
check() {
  local name=$1 log=$2
  shift 2
  if "$@"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL $name"
    [ -f "$log" ] && tail -15 "$log" | sed 's/^/  /'
  fi
}
for tool in scala-cli java node python3; do
  command -v $tool > /dev/null || { echo "gate-scripts: no $tool (incomplete validation)"; echo "gate-scripts: 0 passed, 1 failed"; exit 1; }
done
[ -x "$TEQ" ] || { echo "gate-scripts: no binary at $TEQ"; echo "gate-scripts: 0 passed, 1 failed"; exit 1; }

# stand_in <dir> <body>: a scala-cli in <dir> that runs <body> instead of scala-cli.
stand_in() {
  mkdir -p "$1"
  printf '#!/bin/bash\n%s\n' "$2" > "$1/scala-cli"
  chmod +x "$1/scala-cli"
}

# The module oracle: a negative check passes on scalac's refusal alone.
stand_in "$work/timeout" 'echo "stand-in: the oracle ran out of time" >&2; exit 124'
stand_in "$work/killed" 'kill -9 $$'
stand_in "$work/resolution" 'echo "Error: Could not resolve org.scala-lang:scala3-compiler_3:3.8.4 offline" >&2; exit 1'
for kind in timeout:124 killed:137 resolution:1; do
  k=${kind%%:*}
  PATH=$work/$k:$PATH timeout 300 tests/modules.sh misuse > "$work/oracle-$k.log" 2>&1
  code=$?
  check "oracle: a stand-in scala-cli's $k (exit ${kind#*:}) refuses the misuse case" "$work/oracle-$k.log" \
    eval '[ $code = 1 ] && grep -q "^FAIL misuse scalac-b$" "$work/oracle-$k.log" && grep -q "^the oracle did not refuse b as scalac: scalac exit ${kind#*:}" "$work/oracle-$k.log" && grep -q "refused 0 misuses" "$work/oracle-$k.log"'
done
timeout 300 tests/modules.sh misuse > "$work/oracle-scalac.log" 2>&1
code=$?
check "oracle: scalac 3.8.4 refuses the misuse case" "$work/oracle-scalac.log" \
  eval '[ $code = 0 ] && grep -q "refused 1 misuses as expected" "$work/oracle-scalac.log"'

# A worker that dies is a failure of its case, never a case dropped.
mkdir -p "$work/killer"
printf '#!/bin/bash\ncase " $* " in *" tests/modules/enumorder/"*) kill -9 $PPID; exit 1 ;; esac\nexec "%s" "$@"\n' "$TEQ" > "$work/killer/teq"
chmod +x "$work/killer/teq"
JOBS=2 TEQ=$work/killer/teq timeout 300 tests/modules.sh enumorder misuse > "$work/workers.log" 2>&1
code=$?
check "workers: a killed worker fails its case and the run" "$work/workers.log" \
  eval '[ $code = 1 ] && [ "$(head -1 "$work/workers.log")" = "FAIL enumorder worker: the case'"'"'s worker ended without its counts (exit 137)" ] && grep -q "^modules: 2 passed, 1 failed, .* refused 1 misuses" "$work/workers.log"'

# A part of tasty-exec's regenerations that dies is reported with its status and stderr.
mkdir -p "$work/parts"
printf '#!/bin/bash\ncase " $* " in *" gate part.0 "*) echo "stand-in: part.0 killed" >&2; kill -9 $$ ;; esac\nexec "%s" "$@"\n' "$(command -v java)" > "$work/parts/java"
chmod +x "$work/parts/java"
PATH=$work/parts:$PATH timeout 600 ./teq interp tests/tasty-exec.scala -- $(grep -v '^#' tests/tasty/exec/closure.txt | head -6) > "$work/parts.log" 2>&1
code=$?
check "parts: a killed part of the regenerations fails the suite with its status and stderr" "$work/parts.log" eval '[ $code = 1 ] &&
  grep -q "^FAIL part 0 of [0-9]*: exit 137, killed by signal 9, after [0-9]* s, 0 of [0-9]* jobs answered; kept in " "$work/parts.log" &&
  grep -q "^  stderr: stand-in: part.0 killed$" "$work/parts.log" && grep -q "no answer from part 0" "$work/parts.log"'
kept=$(sed -n 's/^FAIL part 0 of [0-9]*: .*; kept in //p' "$work/parts.log")
case $kept in */tasty-exec.*) rm -rf "$kept" ;; esac

# The suites that need one node name it, whatever the PATH's order.
mkdir -p "$work/nodes/path" "$work/nodes/pinned"
printf '#!/bin/sh\necho v22.0.0-stand-in\n' > "$work/nodes/path/node"
printf '#!/bin/sh\necho v25.9.0-stand-in\n' > "$work/nodes/pinned/node"
chmod +x "$work/nodes/path/node" "$work/nodes/pinned/node"
PATH=$work/nodes/path:$PATH python3 - "$work/nodes" > "$work/nodes.log" 2>&1 <<'PY'
import os, subprocess, sys
sys.path.insert(0, "bench")
import pairs
d = sys.argv[1]
java = pairs.tool_identity("java")
pinned = os.path.join(d, "pinned", "node")
open(os.path.join(d, "pin"), "w").write(f"node {pinned} v25.9.0-stand-in\n{java}\n")
open(os.path.join(d, "gone"), "w").write(f"node {os.path.join(d, 'gone', 'node')} v25.9.0-stand-in\n{java}\n")
got = pairs.check_runtimes({"unpinned": False}, os.path.join(d, "pin"))
assert got == [f"node {pinned} v25.9.0-stand-in", java], got
got = pairs.check_runtimes({"unpinned": True}, os.path.join(d, "gone"))
assert got[0] == f"node {os.path.join(d, 'path', 'node')} v22.0.0-stand-in", got
try:
    pairs.check_runtimes({"unpinned": False}, os.path.join(d, "gone"))
    raise AssertionError("a pinned node with nothing at its path was taken")
except SystemExit as e:
    assert e.code == 2, e.code
print("pin: ok")
PY
check "nodes: the runtime line runs the pinned node and java by their paths" "$work/nodes.log" grep -q "^pin: ok$" "$work/nodes.log"
printf '#!/bin/sh\n[ "$1" = --version ] && { echo v0-stand-in; exit 0; }\ntouch "%s"\nexit 1\n' "$work/nodes/ran" > "$work/nodes/node0"
chmod +x "$work/nodes/node0"
NODE=$work/nodes/node0 ASYMPTOTICS_BASE=100 timeout 300 tests/asymptotics.sh > "$work/asymptotics.log" 2>&1
check "nodes: tests/asymptotics.sh runs the node NODE names" "$work/asymptotics.log" eval '[ -e "$work/nodes/ran" ] && tail -1 "$work/asymptotics.log" | grep -q "(node v0-stand-in)$"'

# The API's type phase (bench/pairs.py api, the gate's app-api-time) over stand-ins of the two binaries: one command
# line for both sides; within, over and inconclusive by the paired judge; a run that times out, prints no type phase
# or a zero one, or changes the diagnostics fails the line; an input that is not there leaves it not run.
mkdir -p "$work/api"
python3 - "$work/api" > "$work/api.log" 2>&1 <<'PY'
import os, sys
sys.path.insert(0, "bench")
import pairs
d = sys.argv[1]
os.makedirs(os.path.join(d, "app", "src"), exist_ok=True)
open(os.path.join(d, "lib.jar"), "w").close()
open(os.path.join(d, "modules.txt"), "w").write("# the stand-in's module\nsrc\n")
open(os.path.join(d, "classpath.txt"), "w").write(os.path.join(d, "lib.jar") + "\n")
os.environ.update(APP_ROOT=os.path.join(d, "app"), APP_MODULES=os.path.join(d, "modules.txt"),
                  APP_CLASSPATH=os.path.join(d, "classpath.txt"), APP_FLAGS="--werror -Wall")
def standin(name, ms, warning="w.scala:1:2: warning: a stand-in's", sleep=0, phases=True):
    path = os.path.join(d, name)
    report = f"phases\n  lines  1\n  read  1.00 ms\n  parse  2.00 ms\n  type  {ms} ms\n  total  9.00 ms\n" if phases else ""
    open(path + ".err", "w").write(f"{warning}\n  x\n  ^\n1 warning found\nworkers\n  phase: fork  {sleep}.{ms} ms\n{report}")
    open(path, "w").write("#!/bin/bash\n"
        "[ \"$1\" = --version ] && { echo teq 0.1.8 stand-in; exit 0; }\n"
        "[ \"$1 $2\" = 'compiler --help' ] && exit 0\n"
        f"echo \"$*\" >> {path}.args\nsleep {sleep}\ncat {path}.err >&2\n")
    os.chmod(path, 0o755)
    return path
def line(master, head, *options):
    opts, _ = pairs.parse_options(["--out", os.path.join(d, "out"), "--cycles", "3", *options])
    try:
        return pairs.run_line("api", master, head, opts)
    except SystemExit as e:
        return f"exit {e.code}"
m = standin("master", "100.00")
results = {
    "within": line(m, standin("same", "100.00")),
    "over": line(m, standin("slow", "150.00")),
    "inconclusive": line(m, standin("near", "108.00")),
    "diagnostics": line(m, standin("other", "100.00", warning="w.scala:1:2: warning: another")),
    "no phase": line(m, standin("bare", "100.00", phases=False)),
    "zero": line(m, standin("zero", "0.00")),
}
pairs.API_TIMEOUT = 1
results["timeout"] = line(m, standin("late", "100.00", sleep=3))
pairs.API_TIMEOUT = 170
os.environ["APP_MODULES"] = os.path.join(d, "nowhere.txt")
results["no input"] = line(m, m)
os.environ["APP_MODULES"] = os.path.join(d, "modules.txt")
open(os.path.join(d, "classpath.txt"), "w").write(os.path.join(d, "gone.jar") + "\n")
results["no jar"] = line(m, m)
want = {"within": 0, "over": 1, "inconclusive": 3, "diagnostics": 1, "no phase": 1, "zero": 1, "timeout": 1, "no input": "exit 2", "no jar": "exit 2"}
assert results == want, results
args = {open(os.path.join(d, n + ".args")).read().splitlines()[0] for n in ("master", "same", "slow")}
assert len(args) == 1, args
words = args.pop().split()
assert words[:2] == ["compiler", "check"] and "--werror" not in words and "-Wall" in words, words
assert words[-8:] == ["--std", "scala-library", "--target", "jvm", "--all-mains", "--threads", "8", "--time"], words
print("api: ok")
PY
check "api: the API's type phase is judged in pairs and fails closed" "$work/api.log" grep -q "^api: ok$" "$work/api.log"

# The gate's want-of rule: what is skipped by design passes, what is skipped for want of something fails.
source <(sed -n '/^skips=/,/^lacking()/p' tests/gate.sh)
: > "$work/skips.log"
while IFS='|' read -r verdict line; do
  [ -n "$line" ] || continue
  printf '%s\n' "$line" > "$work/skip.txt"
  if [ -n "$(lacking "$work/skip.txt")" ]; then got=refused; else got=admitted; fi
  [ "$got" = "$verdict" ] || echo "$line: $got, not $verdict" >> "$work/skips.log"
done <<'WORDINGS'
admitted|skipped (expectations from Scala.js): alias_expected_shapes anon_js_object_undefined
admitted|skipped (another system's): windows_paths windows_process
admitted|skipped (another std's): anyval_extractor_view java_varargs_spread
admitted|skipped (a budget of one platform): a session
admitted|skip array_index_bounds: the JVM's platform
admitted|skip anyval_extractor_view: its std is scala-library
admitted|skip jvm_only_case: the JVM's alone
refused|skip x: not in the coursier cache: cats-core
refused|counted as passed, a jar of theirs not in the coursier cache: x
refused|stdlib: skipped, no scala-library 3.x jar in the coursier cache
refused|app: skipped, not in the coursier cache: cats-core
refused|analysis-bytes: skipped, no scala-library jar
refused|skip the jvm session: error: scala-library
refused|skip javap comparison: no javap or no ct.sym
refused|skip: scala-cli not found
refused|note: esbuild not found, the minified column is skipped
refused|task: note: no wine here, teq.cmd not run
refused|task: note: no shellcheck here, the launcher not linted
refused|task: note: no npm here, the jsdom run not checked
refused|runner-classes: note: javac 24.0.2's output differs from the committed classes, which javac 17 writes (not checked: run with JAVAC naming javac 17)
refused|runner-classes: note: no javac (javac), not checked: set JAVAC or JAVA_HOME
refused|skip the corpus: no /scala3/tests/warn
WORDINGS
check "skips: the want-of rule admits the by-design skips and refuses the others" "$work/skips.log" eval '[ ! -s "$work/skips.log" ]'

# The long suites' pieces (tests/support/pieces.sh): every unit a suite lists in exactly one piece, in the suite's
# order, for every number of pieces; the units only a whole run used to reach among them.
for suite in lsp jvm-watch modules split; do
  tests/$suite.sh --pieces 1 > "$work/pieces-$suite.1" 2>&1
  tests/$suite.sh --pieces 3 > "$work/pieces-$suite.3" 2>&1
  python3 - "$suite" "$work/pieces-$suite.1" "$work/pieces-$suite.3" > "$work/pieces-$suite.log" 2>&1 <<'PY'
import collections, subprocess, sys
suite, one, three = sys.argv[1:]
units = [l.split()[1] for l in open(one) if not l.startswith("#")]
assert units and all(l.startswith("1/1 ") for l in open(one) if not l.startswith("#")), "the whole is not one piece"
for n in range(1, 9):
    listing = subprocess.run(["python3", "tests/support/pieces.py", "list", suite, str(n)], input="\n".join(units),
                             capture_output=True, text=True, check=True).stdout
    if n == 3:
        assert listing == open(three).read(), "the suite's listing of three pieces is not its units dealt"
    rows = [l.split() for l in listing.splitlines() if not l.startswith("#")]
    count = collections.Counter(u for _, u in rows)
    assert len(rows) == len(units) and set(count) == set(units) and set(count.values()) == {1}, f"{n} pieces: {count.most_common(2)}"
    for k in range(1, n + 1):
        mine = [u for p, u in rows if p == f"{k}/{n}"]
        assert mine == [u for u in units if u in set(mine)], f"piece {k}/{n} is out of the suite's order"
print("ok", len(units))
PY
  check "pieces: $suite deals each of its units to exactly one piece, in its order, of 1 to 8" "$work/pieces-$suite.log" grep -q "^ok " "$work/pieces-$suite.log"
done
units_of() { grep -v '^#' "$work/pieces-$1.1" | cut -d' ' -f2; }
# The modules suite's checks made once a run, by their list (tests/modules.sh's suite_units): each a unit once, the
# split programs the units named split-, and the rest the cases, one per directory under tests/modules.
modules_checks="own-twice own-moved own-cut own-shared quoted-member"
check "pieces: modules lists the checks once a run and the split programs as units" "$work/pieces-modules.1" eval \
  '[ "$(units_of modules | grep -x -F -f <(printf "%s\n" $modules_checks) | sort)" = "$(printf "%s\n" $modules_checks | sort)" ] &&
  [ "$(units_of modules | grep -c "^split-")" = "$(grep -c -v -E "^[[:space:]]*(#|$)" tests/modules/split-programs.txt)" ] &&
  [ "$(units_of modules | grep -v "^split-" | grep -c -v -x -F -f <(printf "%s\n" $modules_checks))" = "$(ls -d tests/modules/*/ | wc -l)" ]'
check "pieces: jvm-watch lists a program by its list, the two on both lists twice, and its nine sessions one after another" "$work/pieces-jvm-watch.1" eval \
  '[ "$(units_of jvm-watch | grep -c -x -E "(passing|watch-only)/(inline_definition_gaps|scala3_typeclass-derivation3)")" = 4 ] &&
  [ "$(units_of jvm-watch | grep -c "^passing/")" = "$(wc -w < tests/jvm-passing.txt)" ] && [ "$(units_of jvm-watch | grep -c "^retype/")" = 9 ]'
check "pieces: lsp lists every scenario of the driver once, the idle minute, the jar fetch and the libraries' sources among them" "$work/pieces-lsp.1" eval \
  '[ "$(units_of lsp | sort)" = "$(sed -n "s/^const scenarios = { \(.*\) }$/\1/p" tests/lsp/driver.mjs | tr -d " " | tr "," "\n" | sort)" ] &&
  [ "$(units_of lsp | grep -c -x -E "idleMinute|jarFetch|librarySources|sourcesFailures|sourcesHeld|sourcesBackoff|sourcesPaths")" = 7 ]'
# A selector out of range or misspelled, or another option, is refused before any setup; a piece of no units passes
# as one.
: > "$work/selectors.log"
for suite in lsp jvm-watch modules split; do
  for bad in "--piece 0/2" "--piece 3/2" "--piece 1/0" "--piece a/b" "--piece" "--pieces 0" "--peice 1/2" "--bogus"; do
    out=$(TEQ=/nonexistent COURSIER_CACHE=$work/no-cache timeout 60 tests/$suite.sh $bad 2> /dev/null)
    code=$?
    [ $code = 2 ] && [ -z "$out" ] || echo "$suite $bad: exit $code, $out" >> "$work/selectors.log"
  done
done
TEQ=/nonexistent timeout 60 tests/modules.sh --piece 1/2 misuse > /dev/null 2>&1
[ $? = 2 ] || echo "modules --piece 1/2 misuse: not refused" >> "$work/selectors.log"
# The piece past the suite's units' count is one of no units.
n=$(( $(units_of lsp | wc -l) + 1 ))
out=$(TEQ=/nonexistent timeout 60 tests/lsp.sh --piece $n/$n 2>&1)
code=$?
[ $code = 0 ] && [ "$out" = "$(printf 'pieces: lsp %s/%s: 0 units:\npieces: lsp %s/%s summary: exit=0 units=0' $n $n $n $n)" ] || echo "lsp --piece $n/$n: exit $code, $out" >> "$work/selectors.log"
check "pieces: selectors out of range or misspelled refused before setup, a piece of no units passed" "$work/selectors.log" eval '[ ! -s "$work/selectors.log" ]'
# Each suite's places that run units ask for every unit it lists once (PIECES_DRY=1: none runs).
for suite in lsp jvm-watch modules split; do
  PIECES_DRY=1 timeout 300 tests/$suite.sh --piece 1/2 > "$work/dry-$suite.log" 2>&1
  code=$?
  check "pieces: every unit of $suite's list is asked for once by the places that run it" "$work/dry-$suite.log" eval \
    '[ $code = 0 ] && ! grep -q "^FAIL" "$work/dry-$suite.log" && tail -1 "$work/dry-$suite.log" | grep -q "^pieces: $suite 1/2 summary: exit=0 "'
done
# A unit asked for twice, never, or not listed, a body run twice, never, or not asked for, each fails the piece.
printf '%s\n' '#!/bin/bash' "cd '$PWD'" '. tests/support/pieces.sh' 'suite_units() { printf "%s\n" a "b - again" c; }' \
  '! pieces_option stub "$1" "$2" || shift 2' 'for u in $STUB_ASK; do piece_has $u || continue; done' \
  'for u in $STUB_RUN; do piece_ran ${u/:/ }; done' 'pieces_end 0 passed=0 failed=0' > "$work/stub.sh"
: > "$work/stub.log"
while IFS='|' read -r ask run said; do
  STUB_ASK=$ask STUB_RUN=$run bash "$work/stub.sh" --piece 1/1 > "$work/stub.out" 2>&1
  code=$?
  if [ -z "$said" ]; then
    [ $code = 0 ] && ! grep -q "^FAIL" "$work/stub.out" || { echo "asked $ask, ran $run: $(cat "$work/stub.out")" >> "$work/stub.log"; }
  else
    [ $code = 1 ] && grep -q -x -F "FAIL pieces: $said" "$work/stub.out" || { echo "asked $ask, ran $run: not '$said': $(cat "$work/stub.out")" >> "$work/stub.log"; }
  fi
done <<'CASES'
a b c|a b b:again c|
a a b c|a b b:again c|the unit a was asked for 2 times, not once
a b|a b b:again|the unit c was asked for 0 times, not once
a b c d|a b b:again c|a unit d was asked for that the suite does not list
a b c|a a b b:again c|the unit a ran 2 times, 1 due
a b c|a b c|the unit b never ran at again
a b c|a b b:again c d|the unit d ran 1 times, 0 due
CASES
check "pieces: a unit asked for twice, never or unlisted, a body run twice, never or unasked fails the piece" "$work/stub.log" eval '[ ! -s "$work/stub.log" ]'
# Execution identities: each unit's body stamps its name and site as it starts (PIECES_STAMPS), over stand-in
# binaries (a teq whose sessions answer at once and whose builds fail, a node that runs the language server's driver
# as a list of its scenarios, a java and a scala-cli that fail) and PIECES_STAMP_ONLY=1 (a program's body ends at its
# stamp, a section runs on): every piece of every n of 1 to 8 runs each site of each of its units once and nothing
# else, the pieces of each n together run the bodies the whole run runs, and the whole run's are 1/1's.
stand=$work/stand
mkdir -p "$stand" "$work/stamps" "$work/exec"
printf '%s\n' '#!/bin/bash' 'case "$1 $2" in' \
  '  "compiler watch") echo "{\"ok\":false}"; while read -r line; do case $line in quit) exit 0 ;; build*) echo "{\"ok\":false}" ;; esac; done; exit 0 ;;' \
  '  "compiler check") exit 0 ;;' '  "--version "*) echo "teq 0.0.0 stand-in"; exit 0 ;;' 'esac' 'exit 1' > "$stand/teq"
printf '%s\n' '#!/bin/bash' 'case $1 in tests/lsp/driver.mjs) ;; *) exec "$REAL_NODE" "$@" ;; esac' \
  'names=$(sed -n "s/^const scenarios = { \(.*\) }$/\1/p" tests/lsp/driver.mjs | tr -d " " | tr "," "\n")' \
  '[ -z "$LSP_UNITS" ] || echo "$names" > "$LSP_UNITS"' \
  'for name in $names; do' \
  '  case ,${LSP_ONLY:-}, in *,$name,*) ;; ,,) case $name in idleMinute|jarFetch|librarySources|sourcesFailures|sourcesHeld|sourcesBackoff|sourcesPaths) continue ;; esac ;; *) continue ;; esac' \
  '  [ -z "$PIECES_RAN" ] || echo "$name ${TEQ_SESSION_THREADS:-auto}" >> "$PIECES_RAN"' 'done' 'echo "0 passed, 0 failed"' > "$stand/node"
for tool in java javap scala-cli; do printf '#!/bin/sh\nexit 1\n' > "$stand/$tool"; done
chmod +x "$stand"/*
real_node=$(command -v node)
for suite in split modules jvm-watch lsp; do
  echo "$suite whole -"
  for n in 1 2 3 4 5 6 7 8; do for k in $(seq 1 $n); do echo "$suite $n $k"; done; done
done | env PATH="$stand:$PATH" TEQ="$stand/teq" REAL_NODE="$real_node" JOBS=2 PIECES_STAMP_ONLY=1 \
  xargs -P "$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)" -L 1 bash -c 'if [ "$2" = whole ]; then piece=; else piece="--piece $3/$2"; fi
    PIECES_STAMPS=$0/stamps/$1.$2.$3 timeout 600 tests/$1.sh $piece > $0/exec/$1.$2.$3.log 2>&1' "$work" 2> "$work/exec/xargs.err"
python3 - "$work" > "$work/identities.log" 2>&1 <<'PY'
import collections, os, sys
work = sys.argv[1]
def stamps(name):
    path = os.path.join(work, "stamps", name)
    return collections.Counter(l.strip() for l in open(path) if l.strip()) if os.path.exists(path) else collections.Counter()
bad = []
for suite in ("lsp", "jvm-watch", "modules", "split"):
    whole, one = stamps(f"{suite}.whole.-"), stamps(f"{suite}.1.1")
    if not one or whole != one or max(one.values()) != 1:
        bad.append(f"{suite}: the whole run's bodies ({sum(whole.values())}) are not 1/1's ({sum(one.values())})")
    for n in range(1, 9):
        union = sum((stamps(f"{suite}.{n}.{k}") for k in range(1, n + 1)), collections.Counter())
        if union != one:
            bad.append(f"{suite}: the {n} pieces run {sorted((union - one).items())[:2]} more, {sorted((one - union).items())[:2]} fewer than 1/1")
        for k in range(1, n + 1):
            log = open(os.path.join(work, "exec", f"{suite}.{n}.{k}.log"), errors="replace").read()
            fails = [l for l in log.splitlines() if l.startswith("FAIL pieces:")]
            if fails or f"pieces: {suite} {k}/{n} summary:" not in log:
                bad.append(f"{suite} {k}/{n}: " + (fails[0] if fails else "no summary"))
print("\n".join(bad) or "identities: ok")
PY
for n in 1 2 3 4 5 6 7 8; do for k in $(seq 1 $n); do rm -rf "out/split.${k}of$n"; done; done
check "pieces: every unit's body runs once over the pieces of every n of 1 to 8, the whole run's bodies 1/1's" "$work/identities.log" \
  grep -q -x "identities: ok" "$work/identities.log"
# The merge of the pieces' summaries: counters summed, a verdict of every piece's counted once, a failure any piece
# finds counted once, evidence summed before its check; a piece missing, doubled, unexpected, of another number or
# without its summary fails the suite.
python3 - "$work" > "$work/merge.log" 2>&1 <<'PY'
import os, sys
sys.path.insert(0, "tests/support")
from pieces import merge
work = sys.argv[1]
def log(name, line):
    path = os.path.join(work, name)
    open(path, "w").write(f"some output\n{line}\n")
    return path
def rows(*names):
    path = os.path.join(work, f"rows.{len(os.listdir(work))}")
    open(path, "w").write("".join(f"{n}\tm1\t{s}\t{c}\tlast\n" for n, s, c in names))
    return path
a = log("s.1of2.log", "pieces: s 1/2 summary: exit=0 units=3 passed=5 failed=0 and:parse=ok pos:joins=0")
b = log("s.2of2.log", "pieces: s 2/2 summary: exit=0 units=2 passed=4 failed=0 and:parse=ok pos:joins=2")
ok = rows(("s.1/2", "10", "0"), ("s.2/2", "20", "0"))
assert merge("s", 2, [a, b], ok) == (0, 30, "s: 2 pieces, 10 passed, 0 failed"), merge("s", 2, [a, b], ok)
bad = log("s.2of2.bad", "pieces: s 2/2 summary: exit=1 units=2 passed=4 failed=0 and:parse=fail pos:joins=0")
code, _, line = merge("s", 2, [a, bad], None)
assert code == 1 and line.startswith("s: 2 pieces, 9 passed, 2 failed;") and "parse failed in 1 of 2 pieces" in line and "no joins in any piece" in line and "piece 2/2 failed" in line, line
missing = [log(f"s.{k}of2.any", f"pieces: s {k}/2 summary: exit=1 units=2 passed=3 failed=0 any:scalac={k - 1}") for k in (1, 2)]
code, _, line = merge("s", 2, missing, None)
assert code == 1 and line.startswith("s: 2 pieces, 6 passed, 1 failed;") and "scalac in 1 pieces" in line, line
whole = log("s.whole", "pieces: s whole summary: exit=0 units=5 passed=9 failed=0 and:parse=ok pos:joins=2")
assert merge("s", 1, [whole], None)[2] == merge("s", 2, [a, b], None)[2].replace("2 pieces", "1 pieces"), "the whole and its pieces differ"
for what, results, logs, said in [
    ("missing", rows(("s.1/2", "10", "0")), [a, b], "piece 2/2: no result"),
    ("doubled", rows(("s.1/2", "10", "0"), ("s.2/2", "20", "0"), ("s.2/2", "20", "0")), [a, b], "piece 2/2: 2 results"),
    ("unexpected", rows(("s.1/2", "10", "0"), ("s.2/2", "20", "0"), ("s.3/2", "5", "0")), [a, b], "an unexpected result s.3/2"),
    ("another number", rows(("s.1/2", "10", "0"), ("s.2/2", "20", "0"), ("s.1/3", "5", "0")), [a, b], "an unexpected result s.1/3"),
    ("no summary", ok, [a, log("s.2of2.quiet", "2 passed")], "piece 2/2: no summary"),
    ("a summary of another number", ok, [a, log("s.2of2.other", "pieces: s 2/3 summary: exit=0 units=1 passed=1 failed=0")], "is the summary of piece 2/3, not of one of 2"),
    ("a failed row", rows(("s.1/2", "10", "0"), ("s.2/2", "20", "124")), [a, b], "piece 2/2 failed"),
]:
    code, _, line = merge("s", 2, logs, results)
    assert code == 1 and said in line, f"{what}: {line}"
print("merge: ok")
PY
check "pieces: the merge sums counters, counts a whole suite's verdict once and fails on a missing or doubled piece" "$work/merge.log" grep -q "^merge: ok$" "$work/merge.log"
check "pieces: tests/all.sh runs the long suites whole" /dev/null eval \
  '[ "$(grep -c -E "^suite (lsp|jvm-watch|modules|split) +\./tests/(lsp|jvm-watch|modules|split)\.sh$" tests/all.sh)" = 4 ]'

# The project records (bench/reference/records.scala) through bench/reference/ratchet.scala's verbs, the code the
# compile verbs hold their runs with, in a repository of their own: a stand-in project p whose transcripts name the
# errors a (an absolute path under the checkout, a message of two lines), a3 (a's message one line lower), b and c.
rec=$work/records
mkdir -p "$rec/bench/reference" "$rec/bench/p" "$rec/bench/q" "$rec/tools" "$rec/co" "$rec/co2" "$rec/out" &&
  cp -R tools/script "$rec/tools/" && cp bench/reference/records.scala bench/reference/ratchet.scala "$rec/bench/reference/" &&
  for p in p q; do echo '{"commit": "c0ffee"}' > "$rec/bench/$p/pin.json" && echo '{}' > "$rec/bench/$p/generated.json"; done &&
  git -C "$rec" init -q || { echo "FAIL records: no repository in $rec"; fail=$((fail + 1)); }
commit_records() { git -C "$rec" add -A && git -C "$rec" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "$1"; }
diag() {
  local co=${2:-$rec/co}
  case $1 in
    a) printf '%s\n' "$co/modules/a/A.scala:2:15: error: Found:    (\"bad\" : String)" 'Required: Int' '  val a: Int = "bad"' '               ^^^^^' ;;
    a3) printf '%s\n' "$co/modules/a/A.scala:3:15: error: Found:    (\"bad\" : String)" 'Required: Int' '  val a: Int = "bad"' '               ^^^^^' ;;
    b) printf '%s\n' 'modules/b/B.scala:9:3: error: Not found: y' '  y' '  ^' ;;
    c) printf '%s\n' 'modules/c/C.scala:1:1: error: Not found: z' 'z' '^' ;;
  esac
}
# transcript <file> <exit> <errors...>: teq compile's transcript, core/compile with those errors (a warning between
# them), web/compile checked, the verb's end marker last.
transcript_of() {
  local f=$1 code=$2 d
  shift 2
  { echo "core/compile: ran"; for d in "$@"; do diag "$d" "${CO:-}"; printf '%s\n' 'modules/w/W.scala:1:1: warning: unused' 'w' '^'; done
    if [ $# = 0 ]; then echo "core/compile: checked in 5 ms"; else echo "core/compile: $# error$([ $# = 1 ] || echo s)"; fi
    echo "web/compile: 3 classes, 3 written in 4 ms"; echo "# exit $code"; } > "$f"
}
ratchet() { (cd "$rec" && timeout 120 "$TEQ" interp bench/reference/ratchet.scala -- "$@") > "$work/records.log" 2>&1; }
# held <transcript> [<project>] [options]: the transcript through the ratchet's compile verb as teq writes it, its
# builds' diagnostics recorded beside it (`<transcript>.out` and `.out.diagnostics`, tests/support/teq_channels.py).
held() {
  local t=$1 p=${2:-p}
  shift
  [ $# = 0 ] || shift
  python3 -u tests/support/teq_channels.py --records "$t.out.diagnostics" < "$t" > "$t.out" 2>&1
  ratchet compile "bench/$p" "$t.out" "${CWD:-$rec/co}" "$rec/co" "$rec/out" "$@"
}
transcript_of "$work/t-ab" 1 a b
held "$work/t-ab" p --baseline --binary "teq 0.1.8 aaaaaaaaa" && held "$work/t-ab"
code=$?
check "records: a baseline from master's binary, its evidence beside it and its paths the checkout's, a rerun within it" "$work/records.log" eval '[ $code = 0 ] &&
  grep -q -x "p: compile: 2 errors, within the record.s 2" "$work/records.log" && [ "$(grep -c "^error core/compile " "$rec/bench/p/errors.txt")" = 2 ] &&
  grep -q -x "kind baseline" "$rec/bench/p/errors.txt" && grep -q "^error core/compile modules/a/A.scala:2:15: error: Found: " "$rec/bench/p/errors.txt" &&
  ! grep -q -F "$rec" "$rec/bench/p/errors.txt" && [ -s "$rec/bench/p/errors.evidence.gz" ] && ! gzip -dc "$rec/bench/p/errors.evidence.gz" | grep -q -F "$rec" && grep -q -x "key web/compile" "$rec/bench/p/errors.txt"'
commit_records "master: the baseline"
transcript_of "$work/t-abc" 1 a b c
held "$work/t-abc"
code=$?
check "records: an error outside the record fails the line, printed in full" "$work/records.log" eval '[ $code = 1 ] &&
  grep -q -x "FAIL p: compile: 1 error new" "$work/records.log" && grep -q -x "  core/compile modules/c/C.scala:1:1: error: Not found: z" "$work/records.log" &&
  grep -q -x "  z" "$work/records.log"'
transcript_of "$work/t-b" 1 b
held "$work/t-b"
code=$?
check "records: an error gone passes, listed" "$work/records.log" eval '[ $code = 0 ] &&
  grep -q "^p: compile: gone 1 of the record.s 2" "$work/records.log" && grep -q "^  - core/compile modules/a/A.scala:2:15: " "$work/records.log" &&
  grep -q -x "p: compile: 1 error, within the record.s 2, gone 1" "$work/records.log"'
transcript_of "$work/t-aab" 1 a a b
held "$work/t-aab"
code=$?
check "records: an error twice where the record holds it once fails (the multiset's count)" "$work/records.log" eval '[ $code = 1 ] && grep -q -x "FAIL p: compile: 1 error new" "$work/records.log"'
transcript_of "$work/t-a3b" 1 a3 b
held "$work/t-a3b"
code=$?
check "records: an error fixed beside a new one of the same message one line lower fails, the fixed one gone" "$work/records.log" eval '[ $code = 1 ] &&
  grep -q -x "FAIL p: compile: 1 error new" "$work/records.log" && grep -q "^  core/compile modules/a/A.scala:3:15: " "$work/records.log" && grep -q "gone 1 of the record" "$work/records.log"'
CO=$rec/co2 transcript_of "$work/t-ab2" 1 a b
CWD=$rec/co2 held "$work/t-ab2"
code=$?
check "records: the same errors from another checkout's root are the record's" "$work/records.log" eval '[ $code = 0 ] && grep -q -x "p: compile: 2 errors, within the record.s 2" "$work/records.log"'
# Runs that are no evidence: each fails whatever its errors.
: > "$work/t-empty"
grep -v '^# exit' "$work/t-ab" > "$work/t-cut"
printf '%s\n' "core/compile: ran" "# exit 124" > "$work/t-timeout"
grep -v '^web/compile' "$work/t-ab" > "$work/t-uncovered"
{ grep -v '^web/compile\|^# exit' "$work/t-ab"; echo "teq: web/compile: the build's thread panicked"; echo "# exit 1"; } > "$work/t-panic"
{ grep -v '^# exit' "$work/t-ab"; echo "teq: the daemon ended without an answer (see x)"; echo "# exit 2"; } > "$work/t-daemon"
{ grep -v '^# exit' "$work/t-ab"; echo "web/compile: checked in 2 ms"; echo "# exit 1"; } > "$work/t-twice"
sed 's/^core\/compile: 2 errors$/core\/compile: 3 errors/' "$work/t-ab" > "$work/t-count"
sed 's/^# exit 1$/# exit 0/' "$work/t-ab" > "$work/t-exit"
missing=
for t in empty:"no end marker" cut:"no end marker" timeout:"teq timed out" uncovered:"web/compile: not covered" panic:"web/compile: teq crashed: the build's thread panicked" \
  daemon:"teq.s daemon ended without an answer" twice:"web/compile: covered 2 times" count:"core/compile: 3 errors reported, 2 printed" exit:"teq exits 0, where its diagnostics give 1"; do
  held "$work/t-${t%%:*}" && missing="$missing ${t%%:*}(passed)"
  grep -q "^FAIL p: compile: .*${t#*:}" "$work/records.log" || missing="$missing ${t%%:*}(no '${t#*:}')"
done
check "records: an empty or cut transcript, a timeout, a configuration not covered or twice, a panic, a daemon gone, a count or an exit not the diagnostics' each fail" /dev/null eval '[ -z "$missing" ] || { echo "not as expected:$missing"; false; }'
cp "$rec/bench/p/errors.txt" "$work/errors-1.txt"
held "$work/t-abc" p --record
code=$?
check "records: a failed run records nothing" "$work/records.log" eval '[ $code = 1 ] && grep -q -x "FAIL p: compile: a failed run records nothing" "$work/records.log" &&
  cmp -s "$rec/bench/p/errors.txt" "$work/errors-1.txt" && [ ! -e "$rec/bench/p/errors.txt.proposed" ]'
held "$work/t-b" p --record --binary "teq 0.1.8 bbbbbbbbb" && ratchet check HEAD
code=$?
check "records: an improvement recorded from a passing run, staged and installed, the check counting it against master's" "$work/records.log" eval '[ $code = 0 ] &&
  [ "$(grep -c "^error " "$rec/bench/p/errors.txt")" = 1 ] && grep -q -x "kind record" "$rec/bench/p/errors.txt" && [ ! -e "$rec/bench/p/errors.txt.proposed" ] &&
  grep -q -x "records: bench/p/errors.txt: 1 error, 1 fewer than master.s 2" "$work/records.log"'
commit_records "master: the improvement landed with its record"
held "$work/t-ab"
code=$?
cp "$work/errors-1.txt" "$rec/bench/p/errors.txt" && sed -i 's/^kind baseline$/kind record/' "$rec/bench/p/errors.txt" && ratchet check HEAD
code2=$?
check "records: a resolved error reintroduced after its record landed fails the run, and the record widened back fails the check" "$work/records.log" eval '[ $code = 1 ] && [ $code2 = 1 ] &&
  grep -q "^FAIL records: bench/p/errors.txt: the record widens the accepted one by 1:" "$work/records.log"'
git -C "$rec" checkout -q bench/p/errors.txt
transcript_of "$work/t-bc" 1 b c
cp "$rec/bench/p/errors.txt" "$work/errors-2.txt"
{ cat "$work/errors-2.txt"; echo "error core/compile modules/c/C.scala:1:1: error: Not found: z\\nz\\n^"; } > "$rec/bench/p/errors.txt"
held "$work/t-bc"
code=$?
ratchet check HEAD
code2=$?
git -C "$rec" checkout -q bench/p/errors.txt
check "records: a record widened beside the run that needs it passes the run and fails the check against master's" "$work/records.log" eval '[ $code = 0 ] && [ $code2 = 1 ] &&
  grep -q "^FAIL records: bench/p/errors.txt: the record widens the accepted one by 1:" "$work/records.log" && grep -q "^  + core/compile modules/c/C.scala:1:1: " "$work/records.log"'
held "$work/t-b" p --baseline
code=$?
held "$work/t-b" p --migrate "nothing moved"
code2=$?
check "records: a baseline over a record, and a migration over unchanged inputs, are refused" "$work/records.log" eval '[ $code = 1 ] && [ $code2 = 1 ] &&
  grep -q "^FAIL p: compile: the migrate is refused: a migrate over the accepted record.s inputs" "$work/records.log" && git -C "$rec" diff --quiet &&
  [ ! -e "$rec/bench/p/errors.txt.proposed" ]'
# A migration over unchanged inputs that moves an accepted error (the same message, one line lower) is admitted; one
# that adds an error no lost item matches by its message is refused (a project of its own, m, so the record stays).
mkdir -p "$rec/bench/m" && echo '{"commit": "c0ffee"}' > "$rec/bench/m/pin.json" && echo '{}' > "$rec/bench/m/generated.json"
held "$work/t-ab" m --baseline --binary "teq 0.1.8 aaaaaaaaa" && commit_records "m: the baseline"
held "$work/t-a3b" m --migrate "the error moved one line lower" && ratchet check HEAD
code=$?
moved_kind=$(sed -n "s/^kind //p" "$rec/bench/m/errors.txt")
commit_records "m: the migration"
held "$work/t-abc" m --migrate "nothing moved, c added"
code2=$?
check "records: a migration over unchanged inputs is admitted when it moves an error and refused when it adds one" "$work/records.log" eval '[ $code = 0 ] && [ "$moved_kind" = migrate ] &&
  grep -q "modules/a/A.scala:3:15: " "$rec/bench/m/errors.txt" && ! grep -q "modules/a/A.scala:2:15: " "$rec/bench/m/errors.txt" && [ $code2 = 1 ] &&
  grep -q "adds 1 item no lost item matches by its message" "$work/records.log"'
git -C "$rec" checkout -q bench/m
echo '{"target/teq/p/compile/src_managed": "1234"}' > "$rec/bench/p/generated.json"
held "$work/t-b"
code=$?
held "$work/t-ab" p --migrate "the generated sources moved" && ratchet check HEAD
code2=$?
check "records: new generated inputs fail the run until a migration, admitted with its reason and evidence" "$work/records.log" eval '[ $code = 1 ] && [ $code2 = 0 ] &&
  grep -q -x "kind migrate" "$rec/bench/p/errors.txt" && grep -q -x "reason the generated sources moved" "$rec/bench/p/errors.txt" &&
  grep -q -x "records: bench/p/errors.txt: 2 errors, a migration (the generated sources moved) from master.s 1" "$work/records.log"'
git -C "$rec" checkout -q bench/p
git -C "$rec" rm -q bench/p/errors.txt && ratchet check HEAD
code=$?
held "$work/t-b"
code2=$?
git -C "$rec" checkout -q HEAD -- bench/p/errors.txt
transcript_of "$work/t-none" 0
transcript_of "$work/t-c" 1 c
held "$work/t-none" q --baseline && held "$work/t-none" q
code3=$?
held "$work/t-c" q
code4=$?
check "records: a record removed fails the check and its line, a deliberately empty one holds a clean run to none" "$work/records.log" eval '[ $code = 1 ] && [ $code2 = 1 ] && [ $code3 = 0 ] && [ $code4 = 1 ] &&
  [ "$(grep -c "^error " "$rec/bench/q/errors.txt")" = 0 ]'
# A configuration whose build failed outright: its whole block (a generator's run under teq interp, its own
# diagnostics among the lines) is one item, the next configuration's summary claiming its own errors; a failure of the
# same first line and another explanation is another item.
mkdir -p "$rec/bench/f" && cp "$rec/bench/q/pin.json" "$rec/bench/q/generated.json" "$rec/bench/f/"
failed_outright() {
  { echo 'teq: core/compile: command generator 0 of core/compile: `teq interp gen.scala` failed (exit status: 3): package p'
    printf '%s\n' "+> gen.scala:4:7: error: the constructor of $1 is private" '  new PrintStream(o)' '      ^' 'trait Generated'
    diag b; echo "web/compile: 1 error"; echo "# exit 1"; } > "$2"
}
failed_outright PrintStream "$work/t-failed"
failed_outright FileWriter "$work/t-failed-other"
held "$work/t-failed" f --baseline && held "$work/t-failed" f
code=$?
held "$work/t-failed-other" f
code2=$?
check "records: a build failed outright is its whole block, the next configuration's summary claiming its own; another explanation fails" "$work/records.log" eval '[ $code = 0 ] && [ $code2 = 1 ] &&
  grep -q -x "error core/compile teq: command generator 0 of core/compile: \`teq interp gen.scala\` failed (exit status: 3): package p\\\\ngen.scala:4:7: error: the constructor of PrintStream is private\\\\n  new PrintStream(o)\\\\n      ^\\\\ntrait Generated" "$rec/bench/f/errors.txt" &&
  grep -q "^error web/compile modules/b/B.scala:9:3: error: Not found: y" "$rec/bench/f/errors.txt" && [ "$(grep -c "^error " "$rec/bench/f/errors.txt")" = 2 ] &&
  grep -q -x "FAIL f: compile: 1 error new" "$work/records.log" && grep -q "the constructor of FileWriter is private" "$work/records.log"'
# A crash of teq's and an error counted without its diagnostic are no outcome: neither a baseline nor a migration
# records one, and a judged run fails on it whatever the record holds.
mkdir -p "$rec/bench/k" && cp "$rec/bench/q/pin.json" "$rec/bench/q/generated.json" "$rec/bench/k/"
printf '%s\n' "teq: core/compile: the build's thread panicked" "web/compile: checked in 2 ms" "# exit 1" > "$work/t-crash"
printf '%s\n' "core/compile: 1 error" "web/compile: checked in 2 ms" "# exit 1" > "$work/t-uncounted"
printf '%s\n' "core/compile: ran" "thread 'main' panicked at src/typer/mod.rs:1:1:" "core/compile: checked in 2 ms" "web/compile: checked in 2 ms" "# exit 1" > "$work/t-panicked"
held "$work/t-crash" k --baseline; c1=$?
held "$work/t-uncounted" k --baseline; c2=$?
held "$work/t-panicked" k --baseline; c3=$?
none=$([ ! -e "$rec/bench/k/errors.txt" ] && echo none)
transcript_of "$work/t-k" 0
held "$work/t-k" k --baseline && echo '{"x": "moved"}' > "$rec/bench/k/generated.json" && held "$work/t-crash" k --migrate "a crash moved in"; c4=$?
grep -q -x "kind baseline" "$rec/bench/k/errors.txt" && migrated=no
echo '{}' > "$rec/bench/k/generated.json"
held "$work/t-uncounted" k; c5=$?
check "records: a crash, a panic and an error without its diagnostic are recorded by no baseline or migration and fail a judged run" "$work/records.log" eval '[ $c1 = 1 ] && [ $c2 = 1 ] && [ $c3 = 1 ] && [ $c4 = 1 ] && [ $c5 = 1 ] &&
  [ "$none" = none ] && [ "$migrated" = no ] && grep -q "^FAIL k: compile: core/compile: 1 errors reported without a diagnostic" "$work/records.log"'
# Duplicate coverage is refused before a baseline is published: a configuration summarised twice records nothing.
mkdir -p "$rec/bench/d" && cp "$rec/bench/q/pin.json" "$rec/bench/q/generated.json" "$rec/bench/d/"
{ grep -v '^# exit' "$work/t-none"; echo "web/compile: checked in 2 ms"; echo "# exit 0"; } > "$work/t-twice-clean"
held "$work/t-twice-clean" d --baseline
code=$?
check "records: a baseline over a configuration covered twice records nothing" "$work/records.log" eval '[ $code = 1 ] && [ ! -e "$rec/bench/d/errors.txt" ] &&
  grep -q "web/compile: covered 2 times" "$work/records.log"'
# A floor goes with neither its record nor its project: the check enumerates master's records as well as the head's,
# so a project's directory removed whole (its pin and its records) fails it.
rm -rf "$rec/bench/k" "$rec/bench/d" "$rec/bench/f"
commit_records "master: records of f and q" && rm -rf "$rec/bench/q" && ratchet check HEAD
code=$?
git -C "$rec" checkout -q HEAD -- bench/q
check "records: a project's directory removed with its pin and records fails the check" "$work/records.log" eval '[ $code = 1 ] &&
  grep -q "^FAIL records: bench/q/errors.txt: the record is removed" "$work/records.log"'

# The accepted records are master's as it is: a head forked before master stepped a record down is refused against
# master until it contains that step, and passes once rebased onto it.
git -C "$rec" branch -q -M master && git -C "$rec" checkout -q -b candidate && git -C "$rec" checkout -q master
held "$work/t-none" p --record && commit_records "master: p's record stepped to none"
code=$?
git -C "$rec" checkout -q candidate
ratchet check master
code2=$?
git -C "$rec" rebase -q master && ratchet check master
code3=$?
git -C "$rec" checkout -q master
check "records: a head forked before master stepped a record down is refused against master, and passes rebased onto it" "$work/records.log" eval '[ $code = 0 ] && [ $code2 = 1 ] && [ $code3 = 0 ]'

# The setup of the projects' checkouts (bench/gate-projects/setup.scala) over a stand-in project in a copy of the
# tree, its driver a stand-in whose checkout installs the npm fixture and whose export refuses a checkout without it:
# a run makes the machine ready; with the fixture deleted, a second run installs it again before calling the machine
# ready; a run whose checkout fails removes the ready mark and writes no environment.
st=$work/setup-tree
mkdir -p "$st/bench/gate-projects" "$st/bench/airstream" "$st/tools" "$work/setup-bin" "$work/setup-jdk/bin" &&
  cp -R tools/script "$st/tools/" && cp bench/gate-projects/setup.scala bench/gate-projects/services.json "$st/bench/gate-projects/" &&
  cp bench/airstream/pin.json "$st/bench/airstream/" || { echo "FAIL setup: no tree in $st"; fail=$((fail + 1)); }
cat > "$st/bench/airstream/airstream.scala" <<'SCALA'
//> using file ../../tools/script
// A stand-in of Airstream's script (tests/gate-scripts.sh): checkout installs the npm fixture, export needs it.
import java.nio.file.{Files, Paths}
@main def airstream(argv: String*): Unit = Script.run {
  val args = new Args(argv, "stand-in")
  args.teq("")
  val Vector(verb, checkout, _) = args.exactly(3)
  val co = Paths.get(checkout)
  verb match
    case "checkout" =>
      if Files.exists(Paths.get("bench/airstream/fail-checkout")) then Log.die("airstream: the checkout fails (a stand-in)")
      Files.createDirectories(co.resolve(".git"))
      Files.createDirectories(co.resolve("node_modules/jsdom"))
      println(s"airstream: $co at the pin, its npm fixture installed (a stand-in)")
    case "export" =>
      if !Files.isDirectory(co.resolve("node_modules/jsdom")) then Log.die(s"airstream: no npm fixture in $co: run the verb checkout")
      Files.createDirectories(co.resolve("target/teq"))
      Files.writeString(co.resolve("target/teq/teq.lock"), "projects:\n")
      println("airstream: exported (a stand-in)")
}
SCALA
for t in curl tar node npm sbt; do printf '#!/bin/sh\necho "%s stand-in"\n' "$t" > "$work/setup-bin/$t"; chmod +x "$work/setup-bin/$t"; done
java_major=$(python3 -c "import json; print(json.load(open('bench/airstream/pin.json'))['java'])")
printf '#!/bin/sh\necho %s >&2\n' "'openjdk version \"$java_major\" 2026-01-01'" > "$work/setup-jdk/bin/java"
chmod +x "$work/setup-jdk/bin/java"
setup_run() { (cd "$st" && PATH="$work/setup-bin:$PATH" JAVA_HOME="$work/setup-jdk" timeout 120 "$TEQ" interp bench/gate-projects/setup.scala -- "$work/sp" --projects airstream --teq "$TEQ") > "$1" 2>&1; }
setup_run "$work/setup-1.log"
code=$?
rm -rf "$work/sp/airstream/node_modules"
setup_run "$work/setup-2.log"
code2=$?
touch "$st/bench/airstream/fail-checkout"
setup_run "$work/setup-3.log"
code3=$?
check "setup: a machine ready, its npm fixture deleted and installed again before it is ready again, a failed checkout leaving it not ready" "$work/setup-2.log" eval '[ $code = 0 ] && [ $code2 = 0 ] && [ $code3 = 1 ] &&
  grep -q "setup: airstream-checkout: passed" "$work/setup-2.log" && grep -q "setup: airstream-export: passed" "$work/setup-2.log" &&
  [ -d "$work/sp/airstream/node_modules/jsdom" ] && grep -q "FAIL setup: not ready: airstream" "$work/setup-3.log" && [ ! -e "$work/sp/ready" ] &&
  grep -q -x "export GATE_AIRSTREAM_CHECKOUT=$work/sp/airstream" "$work/sp/gate-projects.env"'

# The gate's driver over stand-in machines.
agent=$work/agent
mkdir -p "$agent/bin" "$agent/scala3/tests"
printf '%s\n' '#!/bin/bash' 'case $1 in' \
  '  -V) echo "cargo 1.0.0 (stand-in)" ;;' \
  '  build) dir=target; prev=; for a in "$@"; do [ "$prev" != --target-dir ] || dir=$a; prev=$a; done' \
  '    mkdir -p "$dir/release" && printf "#!/bin/bash\\n[ \"\$1\" != --version ] || { echo \"teq 0.1.8 %s\"; exit 0; }\\nexec \"%s\" \"\$@\"\\n" "$TEQ_REVISION" "$GATE_AGENT_TEQ" > "$dir/release/teq" && chmod +x "$dir/release/teq"' \
  '    echo "    Finished \`release\` profile [optimized] target(s) in 0.01s (stand-in)" ;;' \
  '  *) echo "cargo stand-in: no $1" >&2; exit 1 ;;' 'esac' > "$agent/bin/cargo"
printf '%s\n' '#!/bin/bash' 'printf "rustc 1.0.0 (stand-in)\nbinary: rustc\ncommit-hash: 0\nhost: x86_64-unknown-linux-gnu\nrelease: 1.0.0\nLLVM version: 0\n"' > "$agent/bin/rustc"
chmod +x "$agent/bin/cargo" "$agent/bin/rustc"
landing=$work/landing
scripts=$PWD/tests/gate-agent
# The landing: this tree's files in a repository of their own (the tree may be a mirror without one), the
# comparison's stand-in, and bench/reference.txt naming the landing's first commit, so that every revision the gate
# resolves is the landing's.
mkdir -p "$landing" && git -C "$landing" init -q &&
  { git ls-files -co --exclude-standard 2> /dev/null || find . -type f ! -path './out/*' ! -path './target/*' ! -path './.git/*'; } |
  tar -cf - -T - | tar -xf - -C "$landing" && cp tests/gate-agent/pairs.py "$landing/bench/pairs.py" &&
  git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "the tree" &&
  echo "reference $(git -C "$landing" rev-parse --short HEAD) 2026-10-14 ceiling 1.20 the landing's own" >> "$landing/bench/reference.txt" &&
  git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "the reference" ||
  { echo "FAIL driver: no landing in $landing"; fail=$((fail + 1)); }
# A program's body run twice behind its one guard, another's guard kept and its body dropped, a section's body
# dropped behind its guard, a retype session called twice and another's call deleted: each fails the piece, on a
# copy of tests/jvm-watch.sh in the landing.
python3 - "$landing/tests/jvm-watch.sh" <<'PY'
import re, sys
path = sys.argv[1]
s = open(path).read()
twice = 'piece_has "passing/$name" && program_case "$name" "passing/$name"'
dropped = 'piece_has "watch-only/$name" && program_case "$name" "watch-only/$name"'
assert twice in s and dropped in s
s = s.replace(twice, 'piece_has "passing/$name" && { program_case "$name" "passing/$name"; program_case "$name" "passing/$name"; }')
s = s.replace(dropped, 'piece_has "watch-only/$name" && :')
retype = 'piece_has "retype/$name" && "retype_$name" $PIN'
assert retype in s
s = s.replace(retype, 'piece_has "retype/$name" && { [ "$name" = reads ] || "retype_$name" $PIN; [ "$name" != entry ] || "retype_$name" $PIN; }')
s, n = re.subn(r"(?ms)^if piece_has owned-mirrors; then\n.*?^fi\n", "if piece_has owned-mirrors; then\n  :\nfi\n", s)
assert n == 1
open(path, "w").write(s)
PY
(cd "$landing" && env PATH="$stand:$PATH" TEQ="$stand/teq" REAL_NODE="$real_node" PIECES_STAMP_ONLY=1 timeout 300 tests/jvm-watch.sh --piece 1/1) > "$work/mutants.log" 2>&1
code=$?
git -C "$landing" checkout -q tests/jvm-watch.sh
check "pieces: a body run twice behind its guard, or dropped behind it, a retype session called twice or not at all, fails the piece" "$work/mutants.log" eval '[ $code = 1 ] &&
  grep -q -x "FAIL pieces: the unit retype/entry ran 2 times, 1 due" "$work/mutants.log" &&
  grep -q -x "FAIL pieces: the unit retype/reads never ran" "$work/mutants.log" &&
  [ "$(grep -c -E "^FAIL pieces: the unit passing/[^ ]+ ran 2 times, 1 due$" "$work/mutants.log")" = "$(wc -w < tests/jvm-passing.txt)" ] &&
  [ "$(grep -c -E "^FAIL pieces: the unit watch-only/[^ ]+ never ran$" "$work/mutants.log")" = 6 ] &&
  grep -q -x "FAIL pieces: the unit owned-mirrors never ran" "$work/mutants.log"'
# gate <log> <pool lines> <gate's arguments...>: the landing's gate over a fresh pool of stand-in machines.
gate() {
  local log=$1 pool=$2
  shift 2
  rm -rf "$agent/machines" "$agent/claims"
  printf '%s\n' "$pool" > "$agent/pool"
  for m in $(awk '{print $1}' "$agent/pool"); do mkdir -p "$agent/machines/$m"; echo "export SCALA3=$agent/scala3" > "$agent/machines/$m/env"; done
  # a machine's own login assignments (a project's checkout), GATE_ENV_<machine>
  for m in $(awk '{print $1}' "$agent/pool"); do v=GATE_ENV_$m; [ -z "${!v}" ] || echo "${!v}" >> "$agent/machines/$m/env"; done
  # the gate's own settings and the machine contract's variables of this machine's shell are not the stand-ins'
  (cd "$landing" && for v in $(compgen -e | grep -E '^(GATE_|LILA_|APP_|CHAIN_KNOWN$|SCALAC_KNOWN$|NODE$)' | grep -v '^GATE_AGENT_'); do unset "$v"; done &&
    env -u JOBS REMOTE_AGENT="$scripts" GATE_AGENT_ROOT="$agent" GATE_AGENT_TEQ="$TEQ" GATE_POLL_SECONDS=2 \
    GATE_CLAIM_WAIT=6 GATE_CLAIM_SECONDS=6 GATE_HEAD_WAIT=120 ${suite_seconds:+GATE_SUITE_SECONDS=$suite_seconds} \
    timeout 300 tests/gate.sh "$landing" "$@") > "$log" 2>&1
}
row() { grep -E "^$1 +$2 " "$3"; }
times_before=$(cat "$landing/tests/gate-times.txt")
GATE_AGENT_CORRUPT=m3 gate "$work/driver-1.log" "$(printf 'm1 free\nm2 free\nm3 free\nh1 free holds=lila,airstream\nb1 busy')" --only parser,fold,planted,lila-export --record
code=$?
log=$work/driver-1.log
check "driver: the builder's head shipped, a spoilt copy refused, lila's line on its machine, nothing recorded" "$log" eval '[ $code = 1 ] &&
  row build m1 "$log" | grep -q "ok .*Finished" && row build m2 "$log" | grep -q "ok .*the head.s binary from m1 after .*s, its digest" &&
  row build m3 "$log" | grep -q "ok .*built here, the head.s binary from m1 refused: its digest is not the builder.s" && row build h1 "$log" | grep -q "ok .*the head.s binary from m1" &&
  row ship m1 "$log" | grep -q "ok .*from m1 to 2 shards" && row lila-export h1 "$log" | grep -q "FAIL .*needs GATE_LILA_CHECKOUT" &&
  row parser "m[123]" "$log" | grep -q " ok " && row fold "m[123]" "$log" | grep -q " ok " && row planted "m[123]" "$log" | grep -q " ok " &&
  ! grep -q "^claim  *h1 " "$log" && grep -q "^gate: no times recorded" "$log" && [ "$times_before" = "$(cat "$landing/tests/gate-times.txt")" ] &&
  grep -q "^gate: [0-9]*m[0-9]*s from its first command to its last, within thirty minutes" "$log"'
gate "$work/driver-2.log" "$(printf 'm1 free\nm2 free')" --only fold,lila-run
code=$?
log=$work/driver-2.log
check "driver: a project line without a machine holding its checkout fails as not run" "$log" eval '[ $code = 1 ] &&
  row lila-run - "$log" | grep -q "FAIL .*not run: no free machine holds lila" && row fold "m[12]" "$log" | grep -q " ok "'
gate "$work/driver-3.log" "$(printf 'm1 free\nm2 free')" --only parser,fold --record
code=$?
log=$work/driver-3.log
check "driver: a passing run records its suites' times" "$log" eval '[ $code = 0 ] && grep -q "^gate: the times recorded" "$log" &&
  grep -q -x "parser $(row parser "m[12]" "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  grep -q -x "fold $(row fold "m[12]" "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  [ "$(echo "$times_before" | grep -v -E "^(parser|fold) ")" = "$(grep -v -E "^(parser|fold) " "$landing/tests/gate-times.txt")" ]'
git -C "$landing" checkout -q tests/gate-times.txt
# The projects' chains dealt over the machines holding them: two holding complementary projects each run their own,
# neither taken by the shards; a project whose holder is busy and one no machine holds each fail as not run, saying
# which; a holder busy at the first ask and free at a later one is left to its project; the eighteen project lines run
# in every gate unnamed; --without leaves a line out, and a name it does not know is refused.
gate "$work/driver-p1.log" "$(printf 'm1 free\nm2 free\nh1 free holds=lila,airstream,laminar\nh2 free holds=tyrian,indigo')" --only fold,lila-export,tyrian-export
code=$?
log=$work/driver-p1.log
check "driver: complementary holders each run their projects' chains, the shards on the others" "$log" eval '[ $code = 1 ] &&
  row lila-export h1 "$log" | grep -q "FAIL .*needs GATE_LILA_CHECKOUT" && row tyrian-export h2 "$log" | grep -q "FAIL .*needs GATE_TYRIAN_CHECKOUT" &&
  row fold "m[12]" "$log" | grep -q " ok " && ! grep -q -E "^claim +h[12] " "$log" && grep -q "projects: h1 lila; h2 tyrian: by the recorded times" "$log"'
gate "$work/driver-p2.log" "$(printf 'm1 free\nh1 busy holds=lila\nh2 free holds=tyrian')" --only fold,lila-export,tyrian-export,indigo-export
code=$?
log=$work/driver-p2.log
check "driver: a project whose holder is busy, and one no machine holds, fail as not run, each saying which" "$log" eval '[ $code = 1 ] &&
  row lila-export - "$log" | grep -q "FAIL .*not run: no free machine holds lila: the machines holding it were busy (h1(busy))" &&
  row indigo-export - "$log" | grep -q "FAIL .*not run: no free machine holds indigo: no machine of the pool holds it" &&
  row tyrian-export h2 "$log" | grep -q "FAIL .*needs GATE_TYRIAN_CHECKOUT" && row fold m1 "$log" | grep -q " ok "'
# A holder busy at the projects' first ask and free at a later one stays theirs: the shards, which find no free machine
# at their first ask and ask again once both are free, take the other.
( sleep 3; printf '%s\n' "m1 free" "h1 free holds=lila" > "$agent/pool" ) &
flip=$!
gate "$work/driver-p8.log" "$(printf 'm1 busy\nh1 busy holds=lila')" --only parser,fold,lila-export
code=$?
wait $flip
log=$work/driver-p8.log
check "driver: a holder busy at the first ask and free at a later one is left to its project, the shards taking the other" "$log" eval '[ $code = 1 ] &&
  row lila-export h1 "$log" | grep -q "FAIL .*needs GATE_LILA_CHECKOUT" && row parser m1 "$log" | grep -q " ok " && row fold m1 "$log" | grep -q " ok " &&
  ! grep -q -E "^claim +h1 " "$log"'
# The project lines run in every gate unnamed: every other line left out (--without, the names read from the gate's
# own lists), the eighteen run on the machine holding the five projects; --without one of them leaves it out.
others=$(python3 - "$landing/tests/gate.sh" <<'PY'
import re, sys
g = open(sys.argv[1]).read()
names = re.findall(r'^\s*"(\S+) ', re.search(r"^machine=\(\n(.*?)^\)", g, re.M | re.S).group(1), re.M)
for var in ("compare", "local_lines"):
    names += re.search(rf'^{var}="([^"]*)"', g, re.M).group(1).split()
print(",".join(names))
PY
)
gate "$work/driver-p9.log" "h1 free holds=lila,airstream,laminar,tyrian,indigo" --without "$others"
code=$?
gate "$work/driver-p10.log" "h1 free holds=lila,airstream,laminar,tyrian,indigo" --without "$others,lila-run"
code2=$?
log=$work/driver-p9.log
declared=$(sed -n 's/^project_lines="\(.*\)"$/\1/p' "$landing/tests/gate.sh" | tr ' ' '\n' | sort | tr '\n' ' ')
check "driver: the eighteen project lines run in every gate unnamed, --without leaving one out" "$log" eval '[ $code = 1 ] && [ $code2 = 1 ] &&
  [ "$(grep -E "^[0-9:]+ h1: " "$log" | sed "s/^[0-9:]* h1: //" | tr " " "\n" | sort | tr "\n" " ")" = "$declared" ] && [ "$(echo $declared | wc -w)" = 18 ] &&
  [ "$(grep -c -E "^(lila|airstream|laminar|tyrian|indigo)-(export|compile|test|run) +h1 .* FAIL .*needs GATE_[A-Z]*_CHECKOUT" "$log")" = 18 ] &&
  ! grep -q -E "^(airstream|laminar)-run " "$log" && ! grep -q "not a line of the gate.s" "$log" &&
  [ "$(grep -c -E "^(lila|airstream|laminar|tyrian|indigo)-(export|compile|test|run) +h1 " "$work/driver-p10.log")" = 17 ] && ! grep -q "^lila-run " "$work/driver-p10.log"'
gate "$work/driver-p3.log" "$(printf 'm1 free\nm2 free')" --only parser,fold --without fold
code=$?
log=$work/driver-p3.log
gate "$work/driver-p3b.log" "$(printf 'm1 free')" --only fold --without nosuch
code2=$?
check "driver: --without leaves a line out after --only, and refuses a name it does not know" "$log" eval '[ $code = 0 ] && [ $code2 = 2 ] &&
  row parser "m[12]" "$log" | grep -q " ok " && ! grep -q "^fold " "$log" && grep -q "^gate: no line named nosuch" "$work/driver-p3b.log"'
# One holder or two, by the recorded times: chains that end after the shards on one machine are dealt over two,
# chains that end within them stay on one, the other left to the shards.
base=$(git -C "$landing" rev-parse HEAD)
printf '%s\n' "lila-export 900" "tyrian-export 900" >> "$landing/tests/gate-times.txt"
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "chains past the shards on one machine"
gate "$work/driver-p4.log" "$(printf 'm1 free\nh1 free holds=lila,tyrian\nh2 free holds=lila,tyrian')" --only fold,lila-export,tyrian-export
code=$?
log=$work/driver-p4.log
git -C "$landing" reset -q --hard "$base"
printf '%s\n' "lila-export 1" "tyrian-export 1" "fold 600" >> "$landing/tests/gate-times.txt"
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "chains within the shards on one machine"
gate "$work/driver-p5.log" "$(printf 'm1 free\nh1 free holds=lila,tyrian\nh2 free holds=lila,tyrian')" --only fold,lila-export,tyrian-export
code2=$?
git -C "$landing" reset -q --hard "$base"
check "driver: the projects over two holders when one would end them after the shards, over one when it would not" "$log" eval '[ $code = 1 ] && [ $code2 = 1 ] &&
  [ "$(grep -E "^(lila|tyrian)-export " "$log" | awk "{print \$2}" | sort -u | tr "\n" " ")" = "h1 h2 " ] &&
  [ "$(grep -E "^(lila|tyrian)-export " "$work/driver-p5.log" | awk "{print \$2}" | sort -u | tr "\n" " ")" = "h1 " ]'
# --record: on a holder whose login environment names a TEQ_PLUGIN_VERSION, the lines run without it (their exports take
# the head's sbt-teq); a chain whose compile passes within its record steps it, and the gate installs the step in the worktree
# once every line passed (bench/reference/ratchet.scala install); a run whose compile shows an error the record lacks
# fails and records nothing. lila's script is a stand-in in the landing whose compile holds a transcript of the
# landing's to the record through bench/reference/records.scala, as the verbs do.
base=$(git -C "$landing" rev-parse HEAD)
cat > "$landing/bench/lila/lila.scala" <<'SCALA'
//> using file ../../tools/script
//> using file ../reference/records.scala
// A stand-in of lila's script (tests/gate-scripts.sh): its compile holds bench/lila/stand-in.txt, its diagnostics'
// records stand-in.txt.diagnostics, to errors.txt.
import java.nio.file.{Files, Paths}
@main def lila(argv: String*): Unit = Script.run {
  val args = new Args(argv, "stand-in")
  val mode = Mode.of(args)
  args.teq("")
  val Vector(verb, checkout, out) = args.exactly(3)
  if verb == "compile" then
    val here = Paths.get("bench/lila")
    val text = Files.readString(here.resolve("stand-in.txt"))
    val roots = Roots.of(Paths.get(checkout), Paths.get(checkout), Paths.get(out))
    val records = Transcript.recorded(here.resolve("stand-in.txt.diagnostics"))
    Script.exit(if Lines.hold("lila", "compile", here, Shape.Compile, Vector("compile" -> Transcript.compile(text, records, roots)), roots(text), mode, "teq stand-in") then 0 else 1)
  println(s"lila: $verb, a stand-in, TEQ_PLUGIN_VERSION ${Option(System.getenv("TEQ_PLUGIN_VERSION")).getOrElse("unset")}")
}
SCALA
{ diag b; diag c; echo "lila/test: 2 errors"; echo "# exit 1"; } | sed "s|$rec/co/||" > "$landing/bench/lila/stand-in.txt"
python3 tests/support/teq_channels.py --records "$landing/bench/lila/stand-in.txt.diagnostics" < "$landing/bench/lila/stand-in.txt" > /dev/null 2>&1
(cd "$landing" && rm -f bench/lila/errors.txt bench/lila/errors.evidence.gz && timeout 120 "$TEQ" interp bench/lila/lila.scala -- compile "$work" "$work/lila-out" --baseline) > "$work/stand-in-baseline.log" 2>&1
{ diag b; echo "lila/test: 1 error"; echo "# exit 1"; } > "$landing/bench/lila/stand-in.txt"
python3 tests/support/teq_channels.py --records "$landing/bench/lila/stand-in.txt.diagnostics" < "$landing/bench/lila/stand-in.txt" > /dev/null 2>&1
git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "a stand-in of lila's script, its baseline of two errors and a run of one"
GATE_ENV_h1="export GATE_LILA_CHECKOUT=$work TEQ_PLUGIN_VERSION=1.0.0-gate-SNAPSHOT" gate "$work/driver-p6.log" "$(printf 'm1 free\nh1 free holds=lila')" --only fold,lila-export,lila-compile --record
code=$?
log=$work/driver-p6.log
recorded_errors=$(grep -c "^error " "$landing/bench/lila/errors.txt")
recorded_kind=$(sed -n "s/^kind //p" "$landing/bench/lila/errors.txt")
git -C "$landing" checkout -q bench/lila tests/gate-times.txt
{ diag b; diag a3; echo "lila/test: 2 errors"; echo "# exit 1"; } | sed "s|$rec/co/||" > "$landing/bench/lila/stand-in.txt"
python3 tests/support/teq_channels.py --records "$landing/bench/lila/stand-in.txt.diagnostics" < "$landing/bench/lila/stand-in.txt" > /dev/null 2>&1
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "a run of a new error"
GATE_ENV_h1="export GATE_LILA_CHECKOUT=$work" gate "$work/driver-p7.log" "$(printf 'm1 free\nh1 free holds=lila')" --only fold,lila-export,lila-compile --record
code2=$?
errors_after=$(git -C "$landing" status --porcelain bench/lila)
git -C "$landing" reset -q --hard "$base"
check "driver: --record installs a passing chain's record step in the worktree, a failing run's records nothing" "$log" eval '[ $code = 0 ] && [ $code2 = 1 ] &&
  row lila-compile h1 "$log" | grep -q " ok .*lila: compile: the record stepped to 1 error in bench/lila/errors.txt" &&
  row lila-export h1 "$log" | grep -q " ok .*lila: export, a stand-in, TEQ_PLUGIN_VERSION unset" &&
  grep -q "^gate: recorded bench/lila/errors.txt: records: bench/lila/errors.txt: 1 error installed (record)" "$log" &&
  [ "$recorded_errors" = 1 ] && [ "$recorded_kind" = record ] &&
  row lila-compile h1 "$work/driver-p7.log" | grep -q " FAIL " && grep -q "^gate: no times recorded, and no records" "$work/driver-p7.log" && [ -z "$errors_after" ]'
# The comparison: three machines prepared beside the suites; with the recorded times the lines measure on one of
# them, after the shards' suites; with times that one machine would end past thirty minutes, on as many as fit.
gate "$work/driver-4.log" "$(printf 'm1 free\nm2 free\nm3 free\nm4 free\nm5 free')" --only parser,fold,budget,runtime,sentinel --master-rev HEAD
code=$?
log=$work/driver-4.log
check "driver: the comparison prepared on three machines, measuring on one after the shards' suites, over the builder's binary" "$log" eval '[ $code = 0 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | grep -c " ok .*the stand-in of bench/pairs.py")" = 3 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | awk "{print \$2}" | sort -u | wc -l)" = 1 ] &&
  [ "$(grep -c -E "^compare .* ok +head: copied from " "$log")" = 3 ] && grep -q -E "^compare .* ok +measure: the lines on 1 of 3 machines ready" "$log" &&
  [ "$(grep -c -E "^compare .* ok +quiet: measured after the suites on the other machines.*here: *$" "$log")" = 2 ] &&
  [ "$(grep -n "shard done after" "$log" | tail -1 | cut -d: -f1)" -lt "$(grep -n -E "^[0-9:]+ m[0-9] +quiet " "$log" | head -1 | cut -d: -f1)" ]'
sed -i -E 's/^(budget|runtime|sentinel) [0-9]+$/\1 1500/' "$landing/tests/gate-times.txt"
echo budget > "$landing/bench/pairs-stand-in.txt"
git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "times past thirty minutes on one machine; budget inconclusive"
gate "$work/driver-5.log" "$(printf 'm1 free\nm2 free\nm3 free\nm4 free\nm5 free')" --only parser,fold,budget,runtime,sentinel --master-rev HEAD --record
code=$?
log=$work/driver-5.log
recorded=$(grep -E "^(parser|fold|runtime|sentinel|budget) " "$landing/tests/gate-times.txt" | sort | tr '\n' ' ')
git -C "$landing" reset -q --hard HEAD~1
check "driver: the lines on three machines when one would end the gate past thirty minutes; an inconclusive timing line records the times" "$log" eval '[ $code = 1 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | awk "{print \$2}" | sort -u | wc -l)" = 3 ] && grep -q -E "^compare .* ok +measure: the lines on 3 of 3 machines ready" "$log" &&
  grep -q -E "^budget .* INCONCLUSIVE " "$log" && grep -q "^gate: the times recorded" "$log" &&
  [ "$recorded" = "$({ echo "budget $(row budget "m[0-9]" "$log" | awk "{print \$3}")"; echo "fold $(row fold "m[0-9]" "$log" | awk "{print \$3}")"; echo "parser $(row parser "m[0-9]" "$log" | awk "{print \$3}")"; echo "runtime $(row runtime "m[0-9]" "$log" | awk "{print \$3}")"; echo "sentinel $(row sentinel "m[0-9]" "$log" | awk "{print \$3}")"; } | sort | tr "\n" " ")" ]'

# A suite run as pieces: tests/lsp.sh a stand-in over tests/support/pieces.sh with six units, each passing unless
# tests/lsp/plant names it. The pieces are dealt as lines of their own, their rows under the suite's merged row, their
# times recorded and the suite's left as it was; the comparison's planner and the dealer read them again, and a
# planted failure, a killed piece, one past the suite's bound and a skip for want of a jar each fail the suite and
# record nothing.
base=$(git -C "$landing" rev-parse HEAD)
cat > "$landing/tests/lsp.sh" <<'EOF'
#!/bin/bash
cd "$(dirname "$0")/.."
. tests/support/pieces.sh
suite_units() { printf '%s\n' u1 u2 u3 u4 u5 u6; }
! pieces_option lsp "$1" "$2" || shift 2
pass=0
fail=0
for u in $(suite_units); do
  piece_has $u || continue
  piece_ran $u
  case $(awk -v u=$u '$2 == u { print $1 }' tests/lsp/plant 2> /dev/null) in
    fail) echo "FAIL $u: planted"; fail=$((fail + 1)) ;;
    kill) kill -9 $$ ;;
    sleep) sleep 60 ;;
    skip) echo "skip $u: not in the coursier cache: planted" ;;
    *) pass=$((pass + 1)) ;;
  esac
done
echo "$pass passed, $fail failed"
pieces_end $((fail > 0)) passed=$pass failed=$fail
EOF
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "a stand-in of the suite"
times_before=$(cat "$landing/tests/gate-times.txt")
gate "$work/driver-6.log" "$(printf 'm1 free\nm2 free\nm3 free')" --only lsp,fold --record
code=$?
log=$work/driver-6.log
recorded_pieces=$(for k in 1 2 3 4; do echo "lsp.$k/4 $(row "lsp\\.$k/4" "m[123]" "$log" | awk '{print $3}')"; done | sort)
check "driver: a suite's four pieces dealt as lines, under its merged row, their times recorded and not the suite's" "$log" eval '[ $code = 0 ] &&
  row lsp "4 pieces" "$log" | grep -q " ok .*lsp: 4 pieces, 6 passed, 0 failed; total piece time [0-9]*s, not wall time" &&
  [ "$(grep -c -E "^lsp\.[1-4]/4 +m[123] .* ok +pieces: lsp [1-4]/4 summary: exit=0 units=[12] passed=[12] failed=0$" "$log")" = 4 ] &&
  [ "$(grep -E "^lsp(\.[1-4]/4)? " "$log" | cut -d" " -f1 | tr "\n" " ")" = "lsp lsp.1/4 lsp.2/4 lsp.3/4 lsp.4/4 " ] &&
  grep -q "^gate: the times recorded" "$log" &&
  [ "$(grep -E "^lsp\.[0-9]+/[0-9]+ " "$landing/tests/gate-times.txt" | sort)" = "$recorded_pieces" ] &&
  [ "$(echo "$times_before" | grep -v -E "^(fold|lsp\.[1-4]/4) ")" = "$(grep -v -E "^(fold|lsp\.[1-4]/4) " "$landing/tests/gate-times.txt")" ]'
printf '%s\n' "fail u1" "kill u2" "sleep u3" "skip u4" > "$landing/tests/lsp/plant"
# Pieces whose end falls with every machine they get: the comparison still takes one machine a line.
sed -i -E 's/^(lsp\.[1-4]\/4) [0-9]+$/\1 600/' "$landing/tests/gate-times.txt"
git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "the pieces' times; units that fail"
times_before=$(cat "$landing/tests/gate-times.txt")
suite_seconds=8 gate "$work/driver-7.log" "$(printf 'm1 free\nm2 free\nm3 free\nm4 free\nm5 free')" --only lsp,budget,runtime,sentinel --master-rev HEAD --record
code=$?
log=$work/driver-7.log
times_after=$(cat "$landing/tests/gate-times.txt")
git -C "$landing" reset -q --hard "$base"
check "driver: a failed, a killed, a timed-out and a skipping piece fail the suite and record nothing; three comparison lines claim three machines" "$log" eval '[ $code = 1 ] &&
  [ "$(grep -E "^[0-9:]+ comparison: m[1-5] m[1-5] m[1-5] prepared for budget runtime sentinel$" "$log" | wc -l)" = 1 ] &&
  [ "$(grep -c -E "^compare +m[1-5] .* ok +claim: waited " "$log")" = 3 ] &&
  row lsp "4 pieces" "$log" | grep -q " FAIL .*lsp: 4 pieces, 1 passed, 1 failed; piece 2/4: no summary; piece 3/4: no summary; pieces 1/4, 2/4, 3/4, 4/4 failed;" &&
  row "lsp\.1/4" "m[1-5]" "$log" | grep -q " FAIL .*summary: exit=1 " && row "lsp\.2/4" "m[1-5]" "$log" | grep -q " FAIL " &&
  row "lsp\.3/4" "m[1-5]" "$log" | grep -q " FAIL " &&
  row "lsp\.4/4" "m[1-5]" "$log" | grep -q " FAIL .*gate: a skip for want of what the machine should hold: skip u4" &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | grep -c " ok ")" = 3 ] &&
  grep -q "^gate: no times recorded" "$log" && [ "$times_before" = "$times_after" ]'

echo "gate-scripts: $pass passed, $fail failed"
[ $fail = 0 ]
