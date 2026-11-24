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
#   pin      tests/gate.sh's app-fix pin (app_fix_pin) over a repository of a migration commit, a later one and a side
#            branch's: the migration an ancestor of HEAD holds it, the side branch's commit or no commit predates it,
#            and a depth-one clone without the migration commit fails with git's error, never a skip.
#   appfix   bench/app/fix.sh over a stand-in checkout and teq: its check `teq fix --check` alone with the checkout's
#            HEAD as SOURCE_VERSION, a check, the coverage or the fixtures failing failing the line (a check's exit 1
#            among them), a check that writes no record or an empty one failing it though an earlier run's record is
#            there, the copy removed after each run and the checkout untouched.
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
#            and a migration over unchanged inputs refused, one that moves an accepted error or words it otherwise
#            at its place admitted, one that adds an error, or words one otherwise at another line, under another
#            configuration or twice at one place, refused; new generated inputs failing the run until a migration,
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
#            lines, which fail there without the checkout, and fail as not run without such a machine; a file whose
#            name git quotes mirrored with the head's tree; the eighteen
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
#   outcomes the board counts no check that did not run (docs/DEVELOPING.md): tests/support/pending.py over a copy of
#            its lists, a skipped listed check kept on the list, no pass and `cannot run`; one outcome a test (a
#            failure, then not run, then a pass), a `known` one a failure on the board held by its own list; another
#            system's test not run, no earlier machine's pass kept; tests/board/board.py's project rows over a
#            made-up record, an ignored test not run; tests/modules.sh's known list held both ways (a listed check that
#            passes fails the run, under --update it leaves the list); the split watch's opening check failing on a
#            type error and not run without the jar; tests/run_jvm.sh's checks without javap or their jars a skip
#            line each, not run; an empty selection of tests/scala3/run.py and of tests/board/dotty.py failing; an
#            empty results file taking no entry out of a fails-only list (a line naming the check passing does);
#            tests/run_stdlib.sh --update without its jar writing every case cannot run; tests/support/identity.sh
#            counting no program identical whose runs were killed on both sides (a jar's build too), its self-test's
#            line, a run of every build failing alike void by its canary, a program failed alike that the lists pass
#            failing the run and one they fail counted, a program not
#            run for want of its jars failing the run unless IDENTITY_SKIPS lists it, a FAIL line under TEQ_GATE; tests/workers.sh's checks whose
#            command was killed failing with that reason.
#   words    the first words' readers: master's code on the head's output rendered back whole, deciding as on master's
#            (run.py, the typing harness, tests/parser/run.py, bench/app/diagnostics.py; a term named `type X` the
#            pinned row); tests/run_errors.sh
#            holding a placed expectation by a diagnostic's header, never by its text inside a quoted value.
#   board    the conformance board's decision on a neg or warn test (tests/board/dotty.py, neg_or_warn) over made-up
#            sources, records of teq's and oracles of scalac's, no compiler run but teq's where said: diagnostics
#            equal to scalac's passing; one word of a message, a later line of it, a line past its end, a column,
#            a severity, a file, one more, one fewer and another order each failing with its first line; white
#            space inside a line, a
#            full stop, a letter's case, a column of 0 and a message of no line each compared; no entry, an entry of
#            another digest or without one, one not valid and one not stable (or silent on either) `marks` where the
#            marks hold and a failure where they fail, the digest that of every file's bytes; a summary of unprinted
#            warnings equal by count and kind under either compiler's flag, failing by another count or kind, on one
#            side alone, out of its place, as an error or with a second line; the -Werror line on both sides passing,
#            on one side standing for nothing of the other's, and compared where it has a place or more lines; another
#            diagnostic without a place compared whole, its severity too; an info and a build's outcome record changing nothing, an
#            unknown severity on either side compared; a box the reader of scalac's did not read `marks` where all it
#            read is equal (the reader cannot decide), a failure where a read one differs, its own severity, file
#            or line does, or teq gives nothing against it; an identity hash (after a word character, of eight digits at most), a colour sequence and
#            the corpus' own path no part of a message, anything else of it compared; a warn test, its build's failure
#            without a record and an entry of no diagnostic against a warning failing, one without a mark passing on
#            no diagnostic, and one without a mark under an option whose warnings teq does not give failing before
#            the oracle is asked, a .check beside it or not; an oracle of another corpus commit or scalac, without a
#            head, with a test twice, another row's entry or a broken line ending the run; the summary teq itself
#            prints read as one by the judge's pattern and as the build's line of tests/support/jars.sh; and the
#            oracle's own validation (tests/board/oracle.py validate) over a made-up corpus: a row's file of its
#            test validating, an entry twice, one silent on its validity, its stability or its digest and one of
#            another scalac than the head's each failing, a run taking those failures, and the runner itself (a
#            copy of it, with --update --add) ending on such a file with exit 2 and the cause, no list written.
#            And the record the runner keeps of scalac's -Werror error, by teq run as a test's groups are: added
#            once a compilation that warned under --werror, an error beside the warning or not (a test marking both
#            its error and that one holds its marks), a summary of unprinted warnings and a migration warning being
#            warnings; none for an error alone, for an info, without --werror, or where teq's build did not fail
#            (a stand-in teq that takes the flag out); two groups, one a compilation, and none for the group that
#            tells an error alone after one that warned.
#   tools    bench/actions/tools.sh, the gate's cs and scala-cli at pinned versions by their digests, the workflow taking
#            them from it and never from coursier/setup-action (whose apps channel was resolved on Maven Central in every
#            job), its three tries a download 10 s apart and each bounded at 120 s within the step's 900 s: against a
#            stand-in curl, two 429s and then the archives served (both unpacked and runnable, the pinned releases asked
#            again after each refusal, the directory added to GITHUB_PATH), three 429s the bound, cs of another digest
#            refused, scala-cli of another digest refused after a good cs, a try that hangs given up at its bound and tried
#            again, the tries a delay apart, another system refused, and a directory where a tool goes refused (each
#            refusal exit 1, said, nothing of it installed and nothing added to GITHUB_PATH).
# Every run is bounded; the script exits 1 on a failure. Needs the release binary (TEQ), scala-cli and the coursier
# cache the suites read.
cd "$(dirname "$0")/.." || exit 1
. tests/support/pieces.sh
# The units, in the order this script runs them: a group of cells, or one run of the gate's driver that its cells read.
# A piece of them (--piece <k>/<n>; --pieces <n> lists them) runs its units alone, each building the trees it reads
# (need_stand, need_driver); the gate deals the pieces as lines of their own (tests/gate.sh's `pieced`).
suite_units() { printf '%s\n' early pieces-lists pieces-stamps pieces-merge records setup pieces-mutants driver-ship driver-unheld driver-record driver-wait driver-cpus driver-steps driver-kind driver-off driver-guest-compare driver-holders driver-held-later driver-unnamed driver-without driver-two-holders driver-record-step driver-compare driver-pieces board board-cap outcomes tools tail; }
! pieces_option gate-scripts "$1" "$2" || shift 2
pieces_refuse gate-scripts "$@"
[ $# = 0 ] || { echo "tests/gate-scripts.sh: no argument $1" >&2; pieces_usage; }
# a piece's own stamps: the suites this script runs keep theirs apart
export -n PIECES_RAN
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
# need_stand: the stand-in binaries of the long suites' pieces (a teq whose sessions answer at once and whose builds
# fail, a node that runs the language server's driver as a list of its scenarios, a java and a scala-cli that fail),
# once a run.
need_stand() {
  [ -z "$stand" ] || return 0
  stand=$work/stand
  mkdir -p "$stand"
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
}
stand=
# the gate's want-of rule (`lacking`), which the skips' and the JVM runner's cells read
source <(sed -n '/^skips=/,/^lacking()/p' tests/gate.sh)

if piece_has early; then
piece_ran early
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
refused|not run, a jar of theirs not in the coursier cache: x
admitted|2954 passed, 10 backend, 108 compile, 10 verify, 11 exception, 81 output, 0 timeout, 0 not run for a jar, 0 checks not run, 71 skipped
admitted|cases: 2879 pass, 188 fail as tests/pending/cases.txt lists, 178 not run
admitted|modules: 367 pass, 31 fail as tests/pending/modules.txt lists, 57 fail as the suite's own lists of checks hold them
admitted|workers: 756 passed, 0 failed, 4 known
refused|skip forwarder_order: no javap
refused|skip the corpus's determinism: not in the coursier cache: cats-core
refused|skip the classpath session: not in the coursier cache: scala-library
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

# The app-fix line's pin: git's answer alone skips it, a lookup git cannot answer fails it.
source <(sed -n '/^app_fix_pin() {/,/^}/p' tests/gate.sh)
pin=$work/pin
commit_pin() { git -C "$pin/origin" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q --allow-empty -m "$1" && git -C "$pin/origin" rev-parse HEAD; }
{ git init -q -b main "$pin/origin" && migration=$(commit_pin migration) && git -C "$pin/origin" checkout -q -b side &&
  side=$(commit_pin side) && git -C "$pin/origin" checkout -q main && later=$(commit_pin later) &&
  git clone -q "$pin/origin" "$pin/full" && git clone -q --depth 1 "file://$pin/origin" "$pin/shallow"; } > "$work/pin.log" 2>&1
for row in "full|$migration|0|holds" "full|$side|0|predates" "full||0|predates" "shallow|$later|0|holds" "shallow|$migration|1|the migration's ancestry not known (git exit 128): "; do
  IFS='|' read -r clone since want_code want <<< "$row"
  got=$(app_fix_pin "$pin/$clone" "$since")
  code=$?
  [ "$code" = "$want_code" ] && [[ $got == "$want"* ]] || echo "$clone ${since:-(none)}: exit $code, $got; not exit $want_code, $want" >> "$work/pin.log"
done
check "pin: an ancestor holds the migration, another commit or none predates it, a depth-one clone's lookup fails" "$work/pin.log" \
  eval '[ -n "$later" ] && ! grep -q ": exit " "$work/pin.log"'

# bench/app/fix.sh over a stand-in checkout and teq: the check `teq fix --check` alone with the checkout's HEAD as
# SOURCE_VERSION (the stand-in teq exits 3 otherwise), each step's failure the line's, a record left by an earlier run
# never counted (none written, or the empty one of a run of no configuration, fails the coverage), a failing check with no exception though the comparator would
# accept anything asked of it; the copy removed after and the checkout untouched.
appfix=$work/appfix
mkdir -p "$appfix/repo/fix/fixtures" "$appfix/repo/fix/reference" "$appfix/bin"
printf '#!/bin/bash\necho "fixtures: 1 passed"\nexit "${FIXTURE_STATUS:-0}"\n' > "$appfix/repo/fix/fixtures/run.sh"
echo '// a stand-in' > "$appfix/repo/fix/Compare.scala"
echo stand-in > "$appfix/repo/fix/reference/manifest.tsv"
cat > "$appfix/bin/teq" << 'STANDIN'
#!/bin/bash
# `fix --check` with the checkout's HEAD as SOURCE_VERSION writes a record of one document, an empty one under SKIP_ALL
# (every configuration skipped), none under NO_RECORD; the coverage counts the record's documents against one source.
if [ "$*" = "fix --check" ] && [ "$SOURCE_VERSION" = "$(cat "$APPFIX_HEAD")" ]; then
  if [ -n "${SKIP_ALL:-}" ]; then : > "$TEQ_FIX_RECORD"; elif [ -z "${NO_RECORD:-}" ]; then echo "doc path=a.scala" > "$TEQ_FIX_RECORD"; fi
  touch made-in-copy
  exit "${CHECK_STATUS:-0}"
fi
[ "$1 $2" = "interp fix/Compare.scala" ] || exit 3
if [ "$4" = coverage ]; then
  documents=$(grep -c "^doc " "$6")
  echo "coverage: 1 sources of sbt's tasks, $documents documents of teq's"
  [ "$documents" = 1 ] || exit 1
  exit "${COVERAGE_STATUS:-0}"
fi
echo "compare $4: stand-in"
STANDIN
chmod +x "$appfix/repo/fix/fixtures/run.sh" "$appfix/bin/teq"
{ git -C "$appfix/repo" init -q && git -C "$appfix/repo" add -A &&
  git -C "$appfix/repo" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m stand-in; } > "$work/appfix.log" 2>&1
git -C "$appfix/repo" rev-parse HEAD > "$appfix/head"
# Each row after the clean one finds the record a passing run left: a check that writes none, or an empty one, fails.
for row in "|0|1 documents" "NO_RECORD=1|1|wrote no record" "|0|1 documents" "SKIP_ALL=1|1|0 documents" "CHECK_STATUS=1|1|" "CHECK_STATUS=2|1|" "COVERAGE_STATUS=1|1|" "FIXTURE_STATUS=1|1|"; do
  IFS='|' read -r setting want says <<< "$row"
  env $setting APPFIX_HEAD="$appfix/head" timeout 60 bench/app/fix.sh "$appfix/repo" "$appfix/bin/teq" "$appfix/out" > "$appfix/run.log" 2>&1
  code=$?
  [ $code = "$want" ] && [[ $(tail -1 "$appfix/run.log") == *"$says"* ]] && [ ! -e "$appfix/out/copy" ] && [ -z "$(git -C "$appfix/repo" status --porcelain)" ] ||
    echo "${setting:-clean}: exit $code, not $want${says:+ saying $says}; $(tail -1 "$appfix/run.log")" >> "$work/appfix.log"
done
check "appfix: the check teq fix --check alone, a failing check, coverage or fixture failing the line, no record or an empty one failing it, the copy removed, the checkout untouched" "$work/appfix.log" \
  eval '! grep -q ": exit " "$work/appfix.log"'

fi
if piece_has pieces-lists; then
piece_ran pieces-lists
# The long suites' pieces (tests/support/pieces.sh): every unit a suite lists in exactly one piece, in the suite's
# order, for every number of pieces; the units only a whole run used to reach among them.
for suite in lsp jvm-watch modules split gate-scripts; do
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
  [ "$(units_of jvm-watch | grep -c "^passing/")" = "$(python3 tests/support/pending.py passing jvm | wc -w)" ] && [ "$(units_of jvm-watch | grep -c "^retype/")" = 9 ]'
check "pieces: lsp lists every scenario of the driver once, the idle minute, the jar fetch and the libraries' sources among them" "$work/pieces-lsp.1" eval \
  '[ "$(units_of lsp | sort)" = "$(sed -n "s/^const scenarios = { \(.*\) }$/\1/p" tests/lsp/driver.mjs | tr -d " " | tr "," "\n" | sort)" ] &&
  [ "$(units_of lsp | grep -c -x -E "idleMinute|jarFetch|librarySources|sourcesFailures|sourcesHeld|sourcesBackoff|sourcesPaths")" = 7 ]'
# A selector out of range or misspelled, or another option, is refused before any setup; a piece of no units passes
# as one.
: > "$work/selectors.log"
for suite in lsp jvm-watch modules split gate-scripts; do
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
for suite in lsp jvm-watch modules split gate-scripts; do
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
fi
if piece_has pieces-stamps; then
piece_ran pieces-stamps
# Execution identities: each unit's body stamps its name and site as it starts (PIECES_STAMPS), over stand-in
# binaries (a teq whose sessions answer at once and whose builds fail, a node that runs the language server's driver
# as a list of its scenarios, a java and a scala-cli that fail) and PIECES_STAMP_ONLY=1 (a program's body ends at its
# stamp, a section runs on): every piece of every n of 1 to 8 runs each site of each of its units once and nothing
# else, the pieces of each n together run the bodies the whole run runs, and the whole run's are 1/1's.
need_stand
mkdir -p "$work/stamps" "$work/exec"
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
fi
if piece_has pieces-merge; then
piece_ran pieces-merge
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

fi
# The project records (bench/reference/records.scala) through bench/reference/ratchet.scala's verbs, the code the
# compile verbs hold their runs with, in a repository of their own: a stand-in project p whose transcripts name the
# errors a (an absolute path under the checkout, a message of two lines), a3 (a's message one line lower), b and c.
# need_records makes the repository once a run; the helpers below read it (the driver's record step too).
rec=
need_records() {
  [ -z "$rec" ] || return 0
  rec=$work/records
  mkdir -p "$rec/bench/reference" "$rec/bench/p" "$rec/bench/q" "$rec/tools" "$rec/co" "$rec/co2" "$rec/out" &&
    cp -R tools/script "$rec/tools/" && cp bench/reference/records.scala bench/reference/ratchet.scala "$rec/bench/reference/" &&
    for p in p q; do echo '{"commit": "c0ffee"}' > "$rec/bench/$p/pin.json" && echo '{}' > "$rec/bench/$p/generated.json"; done &&
    git -C "$rec" init -q || { echo "FAIL records: no repository in $rec"; fail=$((fail + 1)); }
}
commit_records() { git -C "$rec" add -A && git -C "$rec" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "$1"; }
diag() {
  local co=${2:-$rec/co}
  case $1 in
    a) printf '%s\n' "$co/modules/a/A.scala:2:15: error: Found:    (\"bad\" : String)" 'Required: Int' '  val a: Int = "bad"' '               ^^^^^' ;;
    a3) printf '%s\n' "$co/modules/a/A.scala:3:15: error: Found:    (\"bad\" : String)" 'Required: Int' '  val a: Int = "bad"' '               ^^^^^' ;;
    a3w) printf '%s\n' "$co/modules/a/A.scala:3:9: error: type mismatch: found String, required Int" '  val a: Int = "bad"' '         ^^^' ;;
    a3v) printf '%s\n' "$co/modules/a/A.scala:3:11: error: a String where an Int is required" '  val a: Int = "bad"' '           ^' ;;
    a4w) printf '%s\n' "$co/modules/a/A.scala:4:9: error: type mismatch: found String, required Int" '  val a: Int = "bad"' '         ^^^' ;;
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
if piece_has records; then
piece_ran records
need_records
transcript_of "$work/t-ab" 1 a b
held "$work/t-ab" p --baseline --binary "teq 0.1.8 aaaaaaaaa" && held "$work/t-ab"
code=$?
check "records: a baseline from master's binary, its evidence beside it and its paths the checkout's, a rerun within it" "$work/records.log" eval '[ $code = 0 ] &&
  grep -q -x "p: compile: 2 errors, within the record.s 2" "$work/records.log" && [ "$(grep -c "^error core/compile " "$rec/bench/p/errors.txt")" = 2 ] &&
  grep -q -x "kind baseline" "$rec/bench/p/errors.txt" && grep -q "^error core/compile modules/a/A.scala:2:15: error: Found: " "$rec/bench/p/errors.txt" &&
  ! grep -q -F "$rec" "$rec/bench/p/errors.txt" && [ -s "$rec/bench/p/errors.evidence.gz" ] && ! gzip -dc "$rec/bench/p/errors.evidence.gz" | grep -q -F "$rec" && grep -q -x "key web/compile" "$rec/bench/p/errors.txt"'
# A configuration's summary of ten seconds and more reads in seconds: the same judgment.
sed 's/^web\/compile: 3 classes, 3 written in 4 ms$/web\/compile: 3 classes, 3 written in 12.3 s/' "$work/t-ab" > "$work/t-ab-seconds"
held "$work/t-ab-seconds"
code=$?
check "records: a summary in seconds is a configuration's as one in milliseconds is" "$work/records.log" eval '[ $code = 0 ] && grep -q -x "p: compile: 2 errors, within the record.s 2" "$work/records.log" && grep -q "written in 12.3 s" "$work/t-ab-seconds"'
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
  daemon:"teq crashed: the daemon ended without an answer" twice:"web/compile: covered 2 times" count:"core/compile: 3 errors reported, 2 printed" exit:"teq exits 0, where its diagnostics give 1"; do
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
# A migration that words an accepted error otherwise at its place (the same line of the same file, another message and
# column) is admitted: the place was in the debt and stays in it.
transcript_of "$work/t-a3wb" 1 a3w b
held "$work/t-a3wb" m --migrate "the error is worded otherwise" && ratchet check HEAD
code=$?
placed=$(grep -c "^records: bench/m/errors.txt: 2 errors, a migration (the error is worded otherwise) from master.s 2: 1 added, 0 paired with a lost item by the message, 1 by the place$" "$work/records.log")
check "records: a migration over unchanged inputs is admitted when it words an error otherwise at its place, the check saying so" "$work/records.log" eval '[ $code = 0 ] && [ "$placed" = 1 ] &&
  grep -q "modules/a/A.scala:3:9: error: type mismatch: found String, required Int" "$rec/bench/m/errors.txt" && ! grep -q "modules/a/A.scala:3:15: " "$rec/bench/m/errors.txt" &&
  [ "$(grep -c "^error core/compile " "$rec/bench/m/errors.txt")" = 2 ]'
git -C "$rec" checkout -q bench/m
# The place is the line of the file under its configuration, an item once: the new words one line lower, the same line
# under the other configuration, and a second item at the place of one lost are each refused, and a step of the record
# (--record) takes no new words at all; the record stays as it was.
unplaced="adds 1 item no lost item matches by its message or by its place"
transcript_of "$work/t-a4wb" 1 a4w b
held "$work/t-a4wb" m --migrate "other words one line lower"
c1=$?; n1=$(grep -c "$unplaced" "$work/records.log")
{ echo "core/compile: ran"; diag b; echo "core/compile: 1 error"; echo "web/compile: ran"; diag a3w; echo "web/compile: 1 error"; echo "# exit 1"; } > "$work/t-web-a3w"
held "$work/t-web-a3w" m --migrate "other words under the other configuration"
c2=$?; n2=$(grep -c "$unplaced" "$work/records.log")
transcript_of "$work/t-a3wvb" 1 a3w a3v b
held "$work/t-a3wvb" m --migrate "two items at the place of one"
c3=$?; n3=$(grep -c "$unplaced" "$work/records.log")
held "$work/t-a3wb" m --record
c4=$?
check "records: a migration is refused for other words at another line, under another configuration and twice at one place, and a record step for any" "$work/records.log" eval '[ "$c1 $c2 $c3 $c4" = "1 1 1 1" ] &&
  [ "$n1 $n2 $n3" = "1 1 1" ] && git -C "$rec" diff --quiet -- bench/m && [ ! -e "$rec/bench/m/errors.txt.proposed" ]'
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

fi
if piece_has setup; then
piece_ran setup
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

fi
# The gate's driver over stand-in machines: need_driver builds them and the landing once a run.
driver_ready=
need_driver() {
  [ -z "$driver_ready" ] || return 0
  driver_ready=1
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
# resolves is the landing's. The list is NUL-separated, a name git would quote (tests/semanticdb's `unicode` group
# holds one) taken as it is; one is planted beside them, and the machines' copies of the head are to hold it.
quoted=tests/gate-agent/$(printf 'A\342\200\215B.txt')
mkdir -p "$landing" && git -C "$landing" init -q &&
  { git ls-files -z -co --exclude-standard 2> /dev/null || find . -type f ! -path './out/*' ! -path './target/*' ! -path './.git/*' -print0; } |
  tar --null -cf - -T - | tar -xf - -C "$landing" && cp tests/gate-agent/pairs.py "$landing/bench/pairs.py" &&
  echo "a name git quotes" > "$landing/$quoted" &&
  git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "the tree" &&
  echo "reference $(git -C "$landing" rev-parse --short HEAD) 2026-10-14 ceiling 1.20 the landing's own" >> "$landing/bench/reference.txt" &&
  git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "the reference" ||
  { echo "FAIL driver: no landing in $landing"; fail=$((fail + 1)); }
  times_before=$(cat "$landing/tests/gate-times.txt")
}
if piece_has pieces-mutants; then
piece_ran pieces-mutants
need_stand
need_driver
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
  [ "$(grep -c -E "^FAIL pieces: the unit passing/[^ ]+ ran 2 times, 1 due$" "$work/mutants.log")" = "$(python3 tests/support/pending.py passing jvm | wc -w)" ] &&
  [ "$(grep -c -E "^FAIL pieces: the unit watch-only/[^ ]+ never ran$" "$work/mutants.log")" = 6 ] &&
  grep -q -x "FAIL pieces: the unit owned-mirrors never ran" "$work/mutants.log"'
fi
# gate <log> <pool lines> <gate's arguments...>: the landing's gate over a fresh pool of stand-in machines.
gate() {
  local log=$1 pool=$2
  shift 2
  rm -rf "$agent/machines" "$agent/claims"
  printf '%s\n' "$pool" > "$agent/pool"
  # a stand-in is a machine of eight CPUs, whatever this one's (its look's `cpus`, which the deal and --record read),
  # but where a cell plants another (GATE_ENV_<machine>)
  for m in $(awk '{print $1}' "$agent/pool"); do mkdir -p "$agent/machines/$m"; printf 'export SCALA3=%s\nexport GATE_CPUS=8\n' "$agent/scala3" > "$agent/machines/$m/env"; done
  # a machine's own login assignments (a project's checkout), GATE_ENV_<machine>
  for m in $(awk '{print $1}' "$agent/pool"); do v=GATE_ENV_$m; [ -z "${!v}" ] || echo "${!v}" >> "$agent/machines/$m/env"; done
  # the gate's own settings and the machine contract's variables of this machine's shell are not the stand-ins'
  (cd "$landing" && for v in $(compgen -e | grep -E '^(GATE_|LILA_|APP_|CHAIN_KNOWN$|SCALAC_KNOWN$|NODE$)' | grep -v '^GATE_AGENT_'); do unset "$v"; done &&
    env -u JOBS REMOTE_AGENT="$scripts" GATE_AGENT_ROOT="$agent" GATE_AGENT_TEQ="$TEQ" GATE_POLL_SECONDS=2 \
    GATE_CLAIM_WAIT=6 GATE_CLAIM_SECONDS=6 GATE_HEAD_WAIT=120 ${suite_seconds:+GATE_SUITE_SECONDS=$suite_seconds} \
    ${compare_stall:+GATE_COMPARE_STALL=$compare_stall} timeout 300 "${driver:-.}/tests/gate.sh" "$landing" "$@") > "$log" 2>&1
}
row() { grep -E "^$1 +$2 " "$3"; }
if piece_has driver-ship; then
piece_ran driver-ship
need_driver
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
check "driver: a file whose name git quotes reaches the machines with the head's tree" "$log" eval \
  '! grep -q "rsync" "$log" && [ -f "$agent/machines/m1/rw/teq/$quoted" ] && [ -f "$agent/machines/m2/rw/teq/$quoted" ]'
fi
if piece_has driver-unheld; then
piece_ran driver-unheld
need_driver
gate "$work/driver-2.log" "$(printf 'm1 free\nm2 free')" --only fold,lila-run
code=$?
log=$work/driver-2.log
check "driver: a project line without a machine holding its checkout fails as not run" "$log" eval '[ $code = 1 ] &&
  row lila-run - "$log" | grep -q "FAIL .*not run: no free machine holds lila" && row fold "m[12]" "$log" | grep -q " ok "'
fi
if piece_has driver-record; then
piece_ran driver-record
need_driver
gate "$work/driver-3.log" "$(printf 'm1 free\nm2 free')" --only parser,fold --record
code=$?
log=$work/driver-3.log
check "driver: a passing run records its suites' times" "$log" eval '[ $code = 0 ] && grep -q "^gate: the times recorded" "$log" &&
  grep -q -x "parser $(row parser "m[12]" "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  grep -q -x "fold $(row fold "m[12]" "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  [ "$(echo "$times_before" | grep -v -E "^(parser|fold) ")" = "$(grep -v -E "^(parser|fold) " "$landing/tests/gate-times.txt")" ]'
git -C "$landing" checkout -q tests/gate-times.txt
# The same from a driver run out of a copy of the tree (a branch's driver, whose own tree is not the landing's and is
# removed after the run): the times go into the landing worktree's file, which the gate names, and the driver's own
# file is as it was.
rm -rf "${work:?}/driver-copy"
cp -r "$landing" "$work/driver-copy"
driver=$work/driver-copy gate "$work/driver-3c.log" "$(printf 'm1 free\nm2 free')" --only parser,fold --record
code=$?
log=$work/driver-3c.log
check "driver: --record by a driver out of the landing's tree writes the landing worktree's times and names the file" "$log" eval '[ $code = 0 ] &&
  grep -q -x "gate: the times recorded in $landing/tests/gate-times.txt" "$log" &&
  grep -q -x "parser $(row parser "m[12]" "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  cmp -s "$work/driver-copy/tests/gate-times.txt" <(git -C "$landing" show HEAD:tests/gate-times.txt)'
git -C "$landing" checkout -q tests/gate-times.txt
fi
if piece_has driver-wait; then
piece_ran driver-wait
need_driver
# The deal: the suites longest first, each to the shard with the least so far, the shards past the builder's starting
# from tests/gate-times.txt's head-wait. With parser 100, fold 60 and planted 50 over two machines and a head-wait of
# 60, the builder (m1) takes parser and then planted (100 against fold's 60 and the wait), m2 fold; from 0 for both,
# planted would go to m2.
base=$(git -C "$landing" rev-parse HEAD)
printf '%s\n' "parser 100" "fold 60" "planted 50" "head-wait 60" >> "$landing/tests/gate-times.txt"
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "a head-wait"
gate "$work/driver-wait.log" "$(printf 'm1 free\nm2 free')" --only parser,fold,planted
code=$?
log=$work/driver-wait.log
git -C "$landing" reset -q --hard "$base"
check "driver: the shards past the builder's start the deal from head-wait, the builder taking the longest and the next" "$log" eval '[ $code = 0 ] &&
  row parser m1 "$log" | grep -q " ok " && row planted m1 "$log" | grep -q " ok " && row fold m2 "$log" | grep -q " ok "'
fi
if piece_has driver-cpus; then
piece_ran driver-cpus
need_driver
# A line's seconds by its shard's kind of machine (its look's `cpus`, GATE_CPUS on a stand-in): parser 100 on eight
# CPUs and 20 on sixteen, fold and planted 60 and 50 on either, no head-wait, m1 of sixteen and m2 of eight. m1 takes
# parser (20 against m2's 100) and then planted (70 against 110), m2 fold; by the eights alone, planted would go to
# m2. --record writes m1's parser as the row of sixteen, m2's fold as the row of eight.
base=$(git -C "$landing" rev-parse HEAD)
printf '%s\n' "parser 100" "parser@16 20" "fold 60" "fold@16 60" "planted 50" "planted@16 50" "head-wait 0" >> "$landing/tests/gate-times.txt"
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "rows of two kinds"
GATE_ENV_m1="export GATE_CPUS=16" GATE_ENV_m2="export GATE_CPUS=8" gate "$work/driver-kinds.log" "$(printf 'm1 free\nm2 free')" --only parser,fold,planted --record
code=$?
log=$work/driver-kinds.log
check "driver: a line dealt by its seconds on its shard's kind of machine, and recorded as that kind's row" "$log" eval '[ $code = 0 ] &&
  row parser m1 "$log" | grep -q " ok " && row planted m1 "$log" | grep -q " ok " && row fold m2 "$log" | grep -q " ok " &&
  grep -q -x "parser@16 $(row parser m1 "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" &&
  grep -q -x "fold $(row fold m2 "$log" | awk "{print \$3}")" "$landing/tests/gate-times.txt" && grep -q -x "parser 100" "$landing/tests/gate-times.txt"'
git -C "$landing" reset -q --hard "$base"
fi
if piece_has driver-steps; then
piece_ran driver-steps
need_driver
# A shard's own steps are lines the gate declares (tests/gate.sh's shard_steps). Every step tests/gate-shard.sh
# writes by name is one of them, and a shard script with another fails that reading. Through the driver: over
# stand-ins of bench/zig.sh and of idna's script, the shard that takes idna writes its zig step, whose row passes; a
# step the shard writes that the gate does not declare fails the gate on each machine.
shard_steps() { python3 - "$1" "$2" <<'PY'
import re, sys
declared = re.search(r'^shard_steps="([^"]*)"$', open(sys.argv[1]).read(), re.M).group(1).split()
written = sorted(set(re.findall(r"^\s*step ([a-z][a-z0-9-]*) ", open(sys.argv[2]).read(), re.M)))
unknown = [s for s in written if s not in declared]
print(f"shard steps: {' '.join(written)}; declared: {' '.join(declared)}" + (f"; not declared: {' '.join(unknown)}" if unknown else ""))
sys.exit(1 if unknown or not written else 0)
PY
}
shard_steps "$landing/tests/gate.sh" "$landing/tests/gate-shard.sh" > "$work/shard-steps.log" 2>&1
code=$?
base=$(git -C "$landing" rev-parse HEAD)
python3 - "$landing/tests/gate-shard.sh" <<'PY'
import sys
path = sys.argv[1]
s = open(path).read()
built = "step build $code $started $gate/build.log\n"
assert s.count(built) == 1
open(path, "w").write(s.replace(built, built + "step warmup 0 $started $gate/build.log\n"))
PY
shard_steps "$landing/tests/gate.sh" "$landing/tests/gate-shard.sh" >> "$work/shard-steps.log" 2>&1
code2=$?
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "a step the gate does not declare"
gate "$work/driver-step.log" "$(printf 'm1 free\nm2 free')" --only parser,fold
code3=$?
git -C "$landing" reset -q --hard "$base"
printf '%s\n' 'zig_fetch() {' '  zig=$PWD/target/cross/zig' '  mkdir -p target/cross && printf "#!/bin/sh\\necho 0.0.0-stand-in\\n" > "$zig" && chmod +x "$zig"' '}' > "$landing/bench/zig.sh"
printf '%s\n' '@main def idna(): Unit = println("idna: 1 passed, 0 failed (a stand-in)")' > "$landing/tests/idna.scala"
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "stand-ins of zig's fetch and idna"
gate "$work/driver-zig.log" "$(printf 'm1 free\nm2 free')" --only parser,idna
code4=$?
git -C "$landing" reset -q --hard "$base"
log=$work/driver-zig.log
check "driver: a shard's zig step is a line the gate declares, a step it does not declare fails the gate" "$log" eval '[ $code = 0 ] && [ $code2 = 1 ] &&
  grep -q -x "shard steps: build zig; declared: build zig" "$work/shard-steps.log" && grep -q "; not declared: warmup$" "$work/shard-steps.log" &&
  [ $code3 = 1 ] && [ "$(grep -c -E "^warmup +m[12] .* FAIL +not a line of the gate.s" "$work/driver-step.log")" = 2 ] &&
  [ $code4 = 0 ] && [ "$(row zig "m[12]" "$log" | grep -c " ok .*zig 0.0.0-stand-in at ")" = 1 ] && row idna "m[12]" "$log" | grep -q " ok .*idna: 1 passed" &&
  ! grep -q "not a line of the gate.s" "$log"'
fi
if piece_has driver-kind; then
piece_ran driver-kind
need_driver
# A named kind (a machine whose look names it, GATE_KIND in its own login environment: another host's): m1 of the
# kind guest, m2 of eight CPUs, in one run that two cells read. m1 is never the builder while m2 took the tree, so m2
# builds from 0 and m1 starts at head-wait@guest, both 0 over the tree's own. parser 100 on eight and its row of 20 on
# guest; fold 60 and planted 50 with no guest row, the kind's speed 200 (so 30 and 25 there): m1 takes parser (20
# against 100), then fold (50 against 60); m2 takes planted (50 against 75). Each defect turns the deal by itself:
# parser charged the speed's 50 instead of its row sends fold to m2 (80 against 60), and so does fold charged 120 or
# its row of eight (140 or 80 against 60); a guest builder deals alike and is the head's line; --record writes m1's
# parser as the kind's row, the larger of the run and nine tenths of the row it replaces, 18.
base=$(git -C "$landing" rev-parse HEAD)
printf '%s\n' "parser 100" "parser@guest 20" "fold 60" "planted 50" "head-wait 0" "head-wait@guest 0" "speed@guest 200" >> "$landing/tests/gate-times.txt"
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "rows of a named kind"
GATE_ENV_m1="export GATE_KIND=guest" gate "$work/driver-named.log" "$(printf 'm1 free\nm2 free')" --only parser,fold,planted --record
code=$?
log=$work/driver-named.log
check "driver: a machine naming a kind is dealt by that kind's rows and never builds the head while another can, its line recorded as the kind's row" "$log" eval '[ $code = 0 ] &&
  row parser m1 "$log" | grep -q " ok " && row fold m1 "$log" | grep -q " ok " &&
  grep -q "m2: the head" "$log" && ! grep -q "m1: the head" "$log" && r=$(row parser m1 "$log" | awk "{print \$3}") &&
  grep -q -x "parser@guest $((r > 18 ? r : 18))" "$landing/tests/gate-times.txt" && grep -q -x "parser 100" "$landing/tests/gate-times.txt"'
check "driver: a named kind without a line's row is charged its row of eight over the kind's speed, never 120" "$log" eval '[ $code = 0 ] &&
  row fold m1 "$log" | grep -q " ok " && row planted m2 "$log" | grep -q " ok "'
git -C "$landing" reset -q --hard "$base"
fi
if piece_has driver-off; then
piece_ran driver-off
need_driver
base=$(git -C "$landing" rev-parse HEAD)
# A line kept off a machine: fold marked -1 for the kind guest (its row there to be measured first) runs on m2 though
# m1 would end it first, the gate saying, a line a machine, what it kept off and why. Then two machines of the kind
# guest: fold, marked -1 there, can run on neither and fails the gate as not run, saying why; planted runs.
printf '%s\n' "parser 100" "fold 60" "planted 50" "head-wait 0" "head-wait@guest 0" "speed@guest 400" "fold@guest -1" >> "$landing/tests/gate-times.txt"
git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -a -m "a line kept off a kind"
GATE_ENV_m1="export GATE_KIND=guest" gate "$work/driver-off.log" "$(printf 'm1 free\nm2 free')" --only parser,fold,planted
code=$?
log=$work/driver-off.log
check "driver: a line kept off a machine for its kind's mark runs on another, the gate saying which and why" "$log" eval '[ $code = 0 ] &&
  row fold m2 "$log" | grep -q " ok " && ! grep -q -E "^fold +m1 " "$log" &&
  grep -q "^gate: m1 (guest): kept off fold: marked -1 for the kind guest, its row there to be measured first$" "$log"'
GATE_ENV_m1="export GATE_KIND=guest" GATE_ENV_m2="export GATE_KIND=guest" gate "$work/driver-none.log" "$(printf 'm1 free\nm2 free')" --only fold,planted
code=$?
log=$work/driver-none.log
check "driver: a line no machine of the pool can run fails the gate as not run, saying why, and the others run" "$log" eval '[ $code != 0 ] &&
  grep -q "^fold .*not run: no machine of the pool can run it; m[12] (guest): marked -1 for the kind guest, its row there to be measured first; m[12] (guest): marked -1" "$log" &&
  grep -E "^planted +m[12] " "$log" | grep -q " ok "'
git -C "$landing" reset -q --hard "$base"
fi
if piece_has driver-guest-compare; then
piece_ran driver-guest-compare
need_driver
# The comparison never measures on a machine of a named kind: m1, the first free, is of the kind guest; the comparison
# claims it, finds its kind, releases it and measures elsewhere, the gate saying so.
GATE_ENV_m1="export GATE_KIND=guest" gate "$work/driver-guest-compare.log" "$(printf 'm1 free\nm2 free\nm3 free\nm4 free\nm5 free')" --only parser,fold,budget,runtime,sentinel --master-rev HEAD
code=$?
log=$work/driver-guest-compare.log
check "driver: a machine of a named kind is no comparison machine: claimed, found, released, the timing lines measured elsewhere" "$log" eval '[ $code = 0 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | grep -c " ok ")" = 3 ] && ! grep -q -E "^(budget|runtime|sentinel) +m1 " "$log" &&
  grep -q -E "^compare +m1 .*not a comparison machine: its kind is named" "$log"'
fi
if piece_has driver-holders; then
piece_ran driver-holders
need_driver
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
fi
if piece_has driver-held-later; then
piece_ran driver-held-later
need_driver
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
fi
if piece_has driver-unnamed; then
piece_ran driver-unnamed
need_driver
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
fi
if piece_has driver-without; then
piece_ran driver-without
need_driver
gate "$work/driver-p3.log" "$(printf 'm1 free\nm2 free')" --only parser,fold --without fold
code=$?
log=$work/driver-p3.log
gate "$work/driver-p3b.log" "$(printf 'm1 free')" --only fold --without nosuch
code2=$?
check "driver: --without leaves a line out after --only, and refuses a name it does not know" "$log" eval '[ $code = 0 ] && [ $code2 = 2 ] &&
  row parser "m[12]" "$log" | grep -q " ok " && ! grep -q "^fold " "$log" && grep -q "^gate: no line named nosuch" "$work/driver-p3b.log"'
fi
if piece_has driver-two-holders; then
piece_ran driver-two-holders
need_driver
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
fi
if piece_has driver-record-step; then
piece_ran driver-record-step
need_driver
need_records
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
fi
if piece_has driver-compare; then
piece_ran driver-compare
need_driver
# The comparison: three machines prepared beside the suites; with the recorded times the lines measure on one of
# them, after the shards' suites; with times that one machine would end past thirty minutes, on as many as fit.
# Its stall bound at 3 s: the jobs wait for the shards' suites, still and alive, far past it, and none is failed. Each
# machine's start (its CPUs' pressure and its free memory) is a line of the log.
compare_stall=3 gate "$work/driver-4.log" "$(printf 'm1 free\nm2 free\nm3 free\nm4 free\nm5 free')" --only parser,fold,budget,runtime,sentinel --master-rev HEAD
code=$?
log=$work/driver-4.log
check "driver: the comparison prepared on three machines, measuring on one after the shards' suites, over the builder's binary" "$log" eval '[ $code = 0 ] &&
  [ "$(grep -c -E "^[0-9:]+ m[0-9] \((comparison|8 CPUs)\): cpu pressure (some avg10=|unread)" "$log")" -ge 5 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | grep -c " ok .*the stand-in of bench/pairs.py")" = 3 ] &&
  [ "$(grep -E "^(budget|runtime|sentinel) " "$log" | awk "{print \$2}" | sort -u | wc -l)" = 1 ] &&
  [ "$(grep -c -E "^compare .* ok +head: copied from " "$log")" = 3 ] && grep -q -E "^compare .* ok +measure: the lines on 1 of 3 machines ready" "$log" &&
  [ "$(grep -c -E "^compare .* ok +quiet: measured after the suites on the other machines.*here: *$" "$log")" = 2 ] &&
  [ "$(grep -n "shard done after" "$log" | tail -1 | cut -d: -f1)" -lt "$(grep -n -E "^[0-9:]+ m[0-9] +quiet " "$log" | head -1 | cut -d: -f1)" ]'
# A comparison job killed before its line answers (bench/pairs-stand-in.txt's `die:budget`: the stand-in kills the
# job, as a kill for memory would): its status stills and its process is gone, and its line fails within the stall
# bound, not GATE_COMPARE_SECONDS' hour, naming its machine, its process gone and its last status line.
echo die:budget > "$landing/bench/pairs-stand-in.txt"
git -C "$landing" add -A && git -C "$landing" -c user.name=gate-scripts -c user.email=gate-scripts@localhost commit -q -m "a comparison job killed"
started=$(date +%s)
compare_stall=3 gate "$work/driver-dead.log" "$(printf 'm1 free\nm2 free\nm3 free')" --only parser,budget --master-rev HEAD
code=$?
took=$(($(date +%s) - started))
log=$work/driver-dead.log
git -C "$landing" reset -q --hard HEAD~1
check "driver: a comparison job killed fails its line within the stall bound, naming its machine, its process gone and its last status" "$log" eval '[ $code = 1 ] && [ $took -lt 240 ] &&
  grep -q -E "^compare +m[0-9] .* FAIL +the job on m[0-9] ended without its done line: its process is gone \(pid [0-9]+\); its last status line, unchanged for [0-9]+ s: " "$log"'
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

fi
if piece_has driver-pieces; then
piece_ran driver-pieces
need_driver
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

fi
if piece_has board; then
piece_ran board
# The conformance board's judge (tests/board/dotty.py): what it says of a neg or warn test is one function of the
# test's sources, teq's records and the row's oracle (neg_or_warn), held here over made-up ones. No compiler runs but
# teq once, on a deprecated call, for the summary it prints of the warnings a flag leaves out.
mkdir -p "$work/board-teq"
printf 'object A:\n  @deprecated("no", "1.0") def f = 1\n  val x = f\n' > "$work/board-teq/a.scala"
TEQ_CACHE_DIR="$work/board-teq/cache" timeout 120 "$TEQ" compiler check "$work/board-teq/a.scala" --diagnostics "$work/board-teq/records" > "$work/board-teq/printed" 2>&1
python3 -B - "$work/board" "$work/board-teq/printed" "$work/board-teq/records" > "$work/board.log" 2>&1 <<'PY'
import json, os, sys
sys.dont_write_bytecode = True
sys.path.insert(0, "tests/board")
import dotty as D
import rows as ROWS
root, teq_line, teq_records = sys.argv[1:]
os.makedirs(os.path.join(root, "tests", "neg"))
os.makedirs(os.path.join(root, "tests", "warn"))
ctx = D.Ctx()
ctx.root, ctx.group_args = root, {}
E = "Found:    (\"\" : String)\nRequired: Int"


def several(kind, name, files, flags=()):
    """A made-up test of a made-up row: sources of the corpus `root`, with the marks their texts carry."""
    paths = [os.path.join(root, "tests", kind, f) for f, _ in files]
    for path, (_, text) in zip(paths, files):
        with open(path, "w") as fh:
            fh.write(text)
    return D.Test(ROWS.Row(kind, kind, kind, "made up", list(flags), []), name, [paths], [paths[0][:-len(".scala")]])


def test(kind, name, text, flags=()):
    return several(kind, name, [(name, text)], flags)


def scalac(severity, line, column, message, file="a.scala", **more):
    return dict({"severity": severity, "file": file, "line": line, "column": column, "id": None, "message": message.split("\n")}, **more)


def teq(severity, line, col, message, file="a.scala", kind="neg", **more):
    return dict({"severity": severity, "file": os.path.join(root, "tests", kind, file), "line": line, "col": col, "message": message}, **more)


def loose(severity, message, **more):
    return dict({"severity": severity, "message": message}, **more)


def oracle(t, diagnostics, **entry):
    """The row's oracle as a file of the tool's form, read back by the judge's own loader: one entry, of `t`."""
    e = dict({"row": t.row.name, "test": t.name, "digest": D.digest(root, t.files), "valid": True, "stable": True, "diagnostics": diagnostics}, **entry)
    path = os.path.join(root, "oracle.jsonl")
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(json.dumps({"head": True, "scalac": D.SCALAC, "corpus": D.TAG_COMMIT}) + "\n" + json.dumps({k: v for k, v in e.items() if v is not None}) + "\n")
    return D.load_oracle(path, t.row.name)


def said(name, got, verdict, line):
    ok = got[0] == verdict and (got[1] == line if not line.endswith("...") else got[1].startswith(line[:-3]))
    print(("ok\t" + name) if ok else f"FAIL\t{name}: wanted {(verdict, line)}, the judge said {got}")


def judged(name, t, entry, records, verdict, line="", outcome="errors", built="", then=None, **more):
    """One scenario: the test, scalac's diagnostics as its entry (None: no entry), teq's records, and what the
    judge must say. `built`: the first line of teq's failed build; `then`: done once the entry is made."""
    ctx.oracle = oracle(t, entry, **more) if entry is not None else {}
    if then:
        then()
    said(name, D.neg_or_warn(ctx, t, outcome, built, records), verdict, line)


neg = test("neg", "a.scala", "object A:\n  val x: Int = \"\"  // error\n  val y: Int = \"\"  // error\n")
s2, s3 = scalac("error", 2, 16, E), scalac("error", 3, 16, E)
t2, t3 = teq("error", 2, 16, E), teq("error", 3, 16, E)
HOLD = "the marks hold; scalac's diagnostics are not compared: "
SHOWN = "error a.scala:2:16 Found:    (\"\" : String)"
judged("teq's diagnostics equal to scalac's pass", neg, [s2, s3], [t2, t3], "pass")
judged("one word of a message fails", neg, [s2, s3], [teq("error", 2, 16, E.replace("String", "Text")), t3], "fail",
       f"check: diagnostic 1 of 2: scalac's is {SHOWN}; teq's is {SHOWN.replace('String', 'Text')}")
judged("a later line of a message fails, the line named", neg, [s2, s3], [t2, teq("error", 3, 16, E.replace("Int", "Long"))], "fail",
       "check: diagnostic 2 of 2: error a.scala:3:16, line 2 of the message: scalac's is Required: Int; teq's is Required: Long")
judged("a message that ends early fails at scalac's next line that is not blank", neg, [scalac("error", 2, 16, E + "\n\nlonger"), s3], [t2, t3], "fail",
       "check: diagnostic 1 of 2: error a.scala:2:16, line 4 of the message: scalac's is longer; teq's is nothing")
judged("a column fails", neg, [s2, s3], [t2, teq("error", 3, 17, E)], "fail",
       f"check: diagnostic 2 of 2: scalac's is {SHOWN.replace(':2:', ':3:')}; teq's is {SHOWN.replace(':2:16', ':3:17')}")
judged("a diagnostic more fails", neg, [s2, s3], [t2, t3, teq("warning", 1, 8, "unused")], "fail",
       "check: diagnostic 3 of 2: scalac's is nothing; teq's is warning a.scala:1:8 unused")
judged("a diagnostic fewer fails", neg, [s2, s3, scalac("warning", 1, 8, "unused")], [t2, t3], "fail",
       "check: diagnostic 3 of 3: scalac's is warning a.scala:1:8 unused; teq's is nothing")
judged("another order fails", neg, [s2, s3], [t3, t2], "fail",
       f"check: diagnostic 1 of 2: scalac's is {SHOWN}; teq's is {SHOWN.replace(':2:', ':3:')}")
judged("a severity fails", neg, [s2, s3, scalac("migration warning", 1, 8, "old")], [t2, t3, teq("warning", 1, 8, "old")], "fail",
       "check: diagnostic 3 of 3: scalac's is migration warning a.scala:1:8 old; teq's is warning a.scala:1:8 old")
# no answer: `marks` where the marks hold, a failure where they do not, never a pass
for name, entry, more, why in [
        ("no entry", None, {}, "the oracle has no entry for the test"),
        ("an entry of another digest", [s2, s3], {"digest": "0" * 64}, "the oracle's entry was made from other sources than the test's"),
        ("an entry without a digest", [s2, s3], {"digest": None}, "the oracle's entry was made from other sources than the test's"),
        ("an entry that is not valid", [s2, s3], {"valid": False, "why": "the run threw"}, "scalac's capture of the test did not complete (the run threw)"),
        ("an entry that is not stable", [s2, s3], {"stable": False}, "scalac's answer differs from run to run"),
        ("an entry that does not say it is stable", [s2, s3], {"stable": None}, "scalac's answer was not held to a second capture"),
        ("an entry that does not say it is valid", [s2, s3], {"valid": None}, "scalac's capture of the test did not complete")]:
    judged(name + " is marks where the marks hold", neg, entry, [t2, t3], "marks", HOLD + why, **more)
    judged(name + " is a failure where the marks fail", neg, entry, [t2], "fail", "marks: 1 errors where 2 are marked", **more)
judged("equal diagnostics fail where the marks fail", neg, [s2], [t2], "fail", "marks: 1 errors where 2 are marked")
# rule 3: the diagnostics without a place
werr = test("neg", "w.scala", "object W:\n  def f = 1  // warn\n// nopos-error\n")
sw, tw = scalac("warning", 2, 7, "pure", "w.scala"), teq("warning", 2, 7, "pure", "w.scala")
s_werror, t_werror = scalac("error", None, None, D.WERROR, None), loose("error", D.WERROR, werror=True)
SUM = "there %s; re-run with %s for details"
s_sum, t_sum = scalac("warning", None, None, SUM % ("was 1 deprecation warning", "-deprecation"), None), loose("warning", SUM % ("was 1 deprecation warning", "--deprecation"))
judged("the -Werror line on both sides passes", werr, [sw, s_werror], [tw, t_werror], "pass")
judged("the -Werror line on scalac's side alone stands for no diagnostic of teq's", werr, [sw, s_werror], [tw, loose("error", "another")], "fail",
       "check: diagnostic 2 of 1: scalac's is nothing; teq's is error without a place: another")
judged("the -Werror line on teq's side alone stands for no diagnostic of scalac's", werr, [sw, scalac("error", None, None, "Cyclic macro dependencies", None)], [tw, t_werror], "fail",
       "check: diagnostic 2 of 2: scalac's is error without a place: Cyclic macro dependencies; teq's is nothing")
judged("a summary on both sides passes, each compiler's flag in it", neg, [s2, s3, s_sum], [t2, t3, t_sum], "pass")
judged("a summary of another count fails", neg, [s2, s3, s_sum], [t2, t3, loose("warning", SUM % ("were 2 deprecation warnings", "--deprecation"))], "fail",
       "check: diagnostic 3 of 3: scalac's is warning without a place: there was 1 deprecation warning; re-run with -deprecation for details; "
       "teq's is warning without a place: there were 2 deprecation warnings; re-run with --deprecation for details")
judged("a summary of another kind fails", neg, [s2, s3, s_sum], [t2, t3, loose("warning", SUM % ("was 1 feature warning", "--feature"))], "fail", "check: diagnostic 3 of 3: scalac's is warning without a place: there was 1 deprecation...")
judged("a summary on scalac's side alone fails", neg, [s2, s3, s_sum], [t2, t3], "fail",
       "check: diagnostic 3 of 3: scalac's is warning without a place: there was 1 deprecation warning; re-run with -deprecation for details; teq's is nothing")
judged("a summary on teq's side alone fails", neg, [s2, s3], [t2, t3, t_sum], "fail",
       "check: diagnostic 3 of 2: scalac's is nothing; teq's is warning without a place: there was 1 deprecation warning; re-run with --deprecation for details")
judged("a summary before the diagnostics it follows on the other side fails", neg, [s2, s_sum, s3], [t2, t3, t_sum], "fail", "check: diagnostic 2 of 3: scalac's is warning without a place: there was 1...")
judged("another diagnostic without a place is compared whole: equal passes", werr, [sw, scalac("error", None, None, "Cyclic macro dependencies in tests/neg/w.scala.\n\nTo fix this", None)],
       [tw, loose("error", f"Cyclic macro dependencies in {root}/tests/neg/w.scala.\n\nTo fix this")], "pass")
judged("another diagnostic without a place is compared whole: a line of it fails", werr, [sw, scalac("error", None, None, "Cyclic macro dependencies\n\nTo fix this", None)],
       [tw, loose("error", "Cyclic macro dependencies\n\nTo mend this")], "fail",
       "check: diagnostic 2 of 2: error without a place, line 3 of the message: scalac's is To fix this; teq's is To mend this")
judged("an info on either side changes nothing", neg, [scalac("info", None, None, "[log genBCode] a", None), s2, scalac("info", 2, 3, "noted"), s3],
       [t2, t3, teq("info", 3, 3, "a macro says"), loose("info", "and more")], "pass")
judged("the record of a build's outcome is no diagnostic", neg, [s2, s3], [t2, t3, {"severity": "outcome", "ok": False, "errors": 2}], "pass")
judged("a record of a severity the judge does not know is compared", neg, [s2, s3], [t2, t3, loose("failure", "crash")], "fail",
       "check: diagnostic 3 of 2: scalac's is nothing; teq's is failure without a place: crash")
# a box the reader did not read
blind = scalac("error", 2, 0, "", reader="blind", at=[2, 16])
blind["message"] = []
judged("a box the reader did not read is marks where all it read is equal: the reader cannot decide", neg, [blind, s3], [t2, t3], "marks", D.UNDECIDED % 1)
judged("a box the reader did not read is no excuse: a read one that differs fails", neg, [blind, s3], [t2, teq("error", 3, 17, E)], "fail",
       f"check: diagnostic 2 of 2: scalac's is {SHOWN.replace(':2:', ':3:')}; teq's is {SHOWN.replace(':2:16', ':3:17')}")
judged("a box the reader did not read is held to its line", neg, [blind, s3], [t3, t2], "fail",
       f"check: diagnostic 1 of 2: scalac's is error a.scala:2, a box the reader does not read; teq's is {SHOWN.replace(':2:', ':3:')}")
judged("a box the reader did not read is a failure where the marks fail", neg, [blind, s3], [t2], "fail", "marks: 1 errors where 2 are marked")
judged("a box read as two is one the reader did not read", neg, [dict(s2, message=[], reader="read 2 boxes in this rendering"), s3], [t2, t3], "marks", D.UNDECIDED % 1)
judged("the unread box named is the first of them", neg, [s2, dict(blind, line=3, at=[3, 16])], [t2, t3], "marks",
       "the marks hold; the reader cannot decide: teq's diagnostics equal all it reads of scalac's, and scalac's diagnostic 2 is a box it does not read")
# what is no part of a message
judged("an identity hash is masked on both sides", neg, [scalac("error", 2, 16, "MatchError: ExprImpl@<hash> (of class ExprImpl)"), scalac("error", 3, 16, "E$$anon$1@44bf653d")],
       [teq("error", 2, 16, "MatchError: ExprImpl@1562d7e8 (of class ExprImpl)"), teq("error", 3, 16, "E$$anon$1@1f66")], "pass")
judged("the mask takes no word that is no hash", neg, [scalac("error", 2, 16, "case x@abc"), s3], [teq("error", 2, 16, "case x@abz"), t3], "fail",
       "check: diagnostic 1 of 2: scalac's is error a.scala:2:16 case x@<hash>; teq's is error a.scala:2:16 case x@abz")
judged("a colour sequence is no part of a message", neg, [scalac("error", 2, 16, "expression expected but \x1b[31meof\x1b[0m found"), s3], [teq("error", 2, 16, "expression expected but eof found"), t3], "pass")
judged("a colour sequence is no part of teq's message either", neg, [scalac("error", 2, 16, "expression expected but eof found"), s3], [teq("error", 2, 16, "expression expected but \x1b[31meof\x1b[0m found"), t3], "pass")
judged("the corpus' path in teq's message is scalac's path from the corpus", neg, [scalac("error", 2, 16, "x is already defined as method x in tests/neg/a.scala"), s3],
       [teq("error", 2, 16, f"x is already defined as method x in {root}/tests/neg/a.scala"), t3], "pass")
judged("another path in teq's message stays", neg, [scalac("error", 2, 16, "x is already defined as method x in tests/neg/a.scala"), s3],
       [teq("error", 2, 16, "x is already defined as method x in /elsewhere/tests/neg/a.scala"), t3], "fail", "check: diagnostic 1 of 2: scalac's is error a.scala:2:16 x is already defined as method x in tests/neg/a.scala; teq's is ...")
# each rule once more by a test that only its breaking passes
one = test("neg", "c.scala", "object C:\n  val x: Int = \"\"  // error\n")
sc, tc = scalac("error", 2, 16, E, "c.scala"), teq("error", 2, 16, E, "c.scala")
C = SHOWN.replace("a.scala", "c.scala")
two = several("neg", "two", [("p.scala", "object P:\n  val x: Int = \"\"  // error\n"), ("q.scala", "object Q:\n  val x: Int = \"\"  // error\n")])
sp, sq = scalac("error", 2, 16, E, "p.scala"), scalac("error", 2, 16, E, "q.scala")
tp, tq = teq("error", 2, 16, E, "p.scala"), teq("error", 2, 16, E, "q.scala")
judged("the same diagnostic in another file of the test fails", two, [sp, sq], [tq, tp], "fail",
       f"check: diagnostic 1 of 2: scalac's is {SHOWN.replace('a.scala', 'p.scala')}; teq's is {SHOWN.replace('a.scala', 'q.scala')}")
judged("a line of teq's past the end of scalac's message fails", one, [sc], [teq("error", 2, 16, E + "\n  inlined from c.scala:1", "c.scala")], "fail",
       "check: diagnostic 1 of 1: error c.scala:2:16, line 3 of the message: scalac's is nothing; teq's is inlined from c.scala:1")
judged("white space inside a line of a message is compared", one, [sc], [teq("error", 2, 16, E.replace("Found:    ", "Found: "), "c.scala")], "fail",
       f"check: diagnostic 1 of 1: scalac's is {C}; teq's is {C.replace('Found:    ', 'Found: ')}")
judged("a full stop at a line's end is compared", one, [scalac("error", 2, 16, "No given instance of type Foo[String] was found", "c.scala")],
       [teq("error", 2, 16, "No given instance of type Foo[String] was found.", "c.scala")], "fail",
       "check: diagnostic 1 of 1: scalac's is error c.scala:2:16 No given instance of type Foo[String] was found; teq's is error c.scala:2:16 No given instance of type Foo[String] was found.")
judged("a letter's case is compared", one, [scalac("error", 2, 16, "Not found: infer", "c.scala")], [teq("error", 2, 16, "not found: infer", "c.scala")], "fail",
       "check: diagnostic 1 of 1: scalac's is error c.scala:2:16 Not found: infer; teq's is error c.scala:2:16 not found: infer")
judged("a column of 0 that the reader read is a column", one, [scalac("error", 2, 0, E, "c.scala")], [tc], "fail", f"check: diagnostic 1 of 1: scalac's is {C.replace(':2:16', ':2:0')}; teq's is {C}")
judged("a message of no line that the reader read is a message", one, [dict(sc, message=[])], [tc], "fail", f"check: diagnostic 1 of 1: scalac's is error c.scala:2:16 ; teq's is {C}")
judged("`@` and hex digits after no word character are no hash", one, [scalac("error", 2, 16, "annotation @abc is unknown", "c.scala")], [teq("error", 2, 16, "annotation @abd is unknown", "c.scala")], "fail",
       "check: diagnostic 1 of 1: scalac's is error c.scala:2:16 annotation @abc is unknown; teq's is error c.scala:2:16 annotation @abd is unknown")
judged("nine hex digits after `@` are no hash", one, [scalac("error", 2, 16, "x@123456789", "c.scala")], [teq("error", 2, 16, "x@223456789", "c.scala")], "fail",
       "check: diagnostic 1 of 1: scalac's is error c.scala:2:16 x@123456789; teq's is error c.scala:2:16 x@223456789")
judged("scalac's diagnostic of a severity the judge does not know is compared", one, [sc, scalac("failure", None, None, "crash", None)], [tc], "fail",
       "check: diagnostic 2 of 2: scalac's is failure without a place: crash; teq's is nothing")
judged("a path beside the corpus in teq's message stays", one, [scalac("error", 2, 16, "x is already defined as method x in other/tests/neg/c.scala", "c.scala")],
       [teq("error", 2, 16, f"x is already defined as method x in {os.path.dirname(root)}/other/tests/neg/c.scala", "c.scala")], "fail",
       "check: diagnostic 1 of 1: scalac's is error c.scala:2:16 x is already defined as method x in other/tests/neg/c.scala; teq's is ...")
judged("an error with a place whose words are the -Werror line's is compared", one, [scalac("error", 2, 7, D.WERROR, "c.scala"), scalac("error", 2, 16, "x", "c.scala")], [teq("error", 2, 16, "x", "c.scala")], "fail",
       "check: diagnostic 1 of 2: scalac's is error c.scala:2:7 No warnings can be incurred under -Werror; teq's is error c.scala:2:16 x")
judged("an error that only begins as the -Werror line is compared", werr, [sw, scalac("error", None, None, D.WERROR + "\nand more", None)], [tw, t_werror], "fail",
       "check: diagnostic 2 of 2: scalac's is error without a place: No warnings can be incurred under -Werror; teq's is nothing")
judged("a summary's words as an error are compared whole", werr, [sw, scalac("error", None, None, SUM % ("was 1 deprecation warning", "-deprecation"), None)],
       [tw, loose("error", SUM % ("was 1 deprecation warning", "--deprecation"))], "fail",
       "check: diagnostic 2 of 2: scalac's is error without a place: there was 1 deprecation warning; re-run with -deprecation for details; "
       "teq's is error without a place: there was 1 deprecation warning; re-run with --deprecation for details")
noline = test("neg", "l.scala", "object L:\n  val x = 1\n// nopos-error\n")
judged("a line is compared where one side has none", noline, [scalac("error", 2, 7, "M", "l.scala")], [{"severity": "error", "file": noline.files[0], "col": 7, "message": "M"}], "fail",
       "check: diagnostic 1 of 1: scalac's is error l.scala:2:7 M; teq's is error l.scala:0:7 M")
judged("a box the reader did not read is held to its file", two, [dict(sp, column=0, message=[], reader="blind", at=[2, 16]), dict(sq, column=0, message=[], reader="blind", at=[2, 16])], [tq, tp], "fail",
       f"check: diagnostic 1 of 2: scalac's is error p.scala:2, a box the reader does not read; teq's is {SHOWN.replace('a.scala', 'q.scala')}")
fewer = test("neg", "f.scala", "object F:\n  val x: Int = \"\"  // error\n  def f = 1\n")
judged("a box the reader did not read against no diagnostic of teq's fails", fewer,
       [scalac("error", 2, 16, E, "f.scala"), dict(scalac("warning", 3, 0, "", "f.scala"), message=[], reader="blind", at=[3, 7])], [teq("error", 2, 16, E, "f.scala")], "fail",
       "check: diagnostic 2 of 2: scalac's is warning f.scala:3, a box the reader does not read; teq's is nothing")
# the entry's digest is of every file's bytes


def rewritten(path, text):
    def write():
        with open(path, "w") as fh:
            fh.write(text)
    return write


judged("an entry of the test before a byte of it changed is no answer", one, [sc], [tc], "marks", HOLD + "the oracle's entry was made from other sources than the test's",
       then=rewritten(one.files[0], "object D:\n  val x: Int = \"\"  // error\n"))
judged("an entry of the test before its second file changed is no answer", two, [sp, sq], [tp, tq], "marks", HOLD + "the oracle's entry was made from other sources than the test's",
       then=rewritten(two.files[1], "object R:\n  val x: Int = \"\"  // error\n"))
# a warn test, and the option whose warnings teq does not give
warn = test("warn", "b.scala", "object B:\n  def f = 1  // warn\n")
sb, tb = scalac("warning", 2, 7, "pure", "b.scala"), dict(tw, file=os.path.join(root, "tests", "warn", "b.scala"))
judged("a warn test's warnings equal to scalac's pass", warn, [sb], [tb], "pass", outcome="ok")
judged("a warn test's warning in other words fails", warn, [sb], [dict(tb, message="impure")], "fail",
       "check: diagnostic 1 of 1: scalac's is warning b.scala:2:7 pure; teq's is warning b.scala:2:7 impure", outcome="ok")
none = test("warn", "c.scala", "object C\n")
judged("a test without a mark passes on no diagnostic on either side", none, [], [], "pass", outcome="ok")
judged("a test without a mark and without an entry is marks", none, None, [], "marks", HOLD + "the oracle has no entry for the test", outcome="ok")
wb = test("warn", "g.scala", "object G:\n  def f = 1  // warn\n  def g = 2\n")
judged("a box the reader did not read is held to its severity", wb, [dict(scalac("warning", 2, 0, "", "g.scala"), message=[], reader="blind", at=[2, 3])],
       [teq("migration warning", 2, 3, "x", "g.scala", "warn")], "fail",
       "check: diagnostic 1 of 1: scalac's is warning g.scala:2, a box the reader does not read; teq's is migration warning g.scala:2:3 x", outcome="ok")
judged("a migration warning of teq's is no warning of scalac's", warn, [dict(sb, message=["old"])], [dict(tb, severity="migration warning", message="old")], "fail",
       "check: diagnostic 1 of 1: scalac's is warning b.scala:2:7 old; teq's is migration warning b.scala:2:7 old", outcome="ok")
judged("an entry without a diagnostic is held to teq's: a warning of teq's fails", warn, [], [tb], "fail",
       "check: diagnostic 1 of 0: scalac's is nothing; teq's is warning b.scala:2:7 pure", outcome="ok")
nopos = test("warn", "h.scala", "object H\n// nopos-warn\n")
judged("a warning without a place that is no summary is compared whole", nopos, [scalac("warning", None, None, "bad option '-Yno-experimental' was ignored", None)],
       [loose("warning", "Option -source was updated")], "fail",
       "check: diagnostic 1 of 1: scalac's is warning without a place: bad option '-Yno-experimental' was ignored; teq's is warning without a place: Option -source was updated", outcome="ok")
judged("a migration warning without a place is no warning of scalac's without one", nopos, [scalac("warning", None, None, "bad option '-Yno-experimental' was ignored", None)],
       [loose("migration warning", "bad option '-Yno-experimental' was ignored")], "fail",
       "check: diagnostic 1 of 1: scalac's is warning without a place: bad option '-Yno-experimental' was ignored; "
       "teq's is migration warning without a place: bad option '-Yno-experimental' was ignored", outcome="ok")
judged("a warning without a place whose first line alone is a summary is compared whole", nopos, [scalac("warning", None, None, SUM % ("was 1 feature warning", "-feature") + "\nand this", None)],
       [loose("warning", SUM % ("was 1 feature warning", "--feature") + "\nand that")], "fail",
       "check: diagnostic 1 of 1: scalac's is warning without a place: there was 1 feature warning; re-run with -feature for details; "
       "teq's is warning without a place: there was 1 feature warning; re-run with --feature for details", outcome="ok")
judged("a warn test whose build failed without an error record fails", none, [], [], "fail", "marks: an error where warnings are marked: teq: exit 3", outcome="errors", built="teq: exit 3")
unperformed = test("warn", "d.scala", "object D\n", ["-Wsafe-init"])
judged("a test without a mark under an option teq does not perform fails, both sides silent", unperformed, [], [], "fail",
       "option: -Wsafe-init (a check teq does not perform: no warning says nothing)", outcome="ok")
with open(os.path.join(root, "tests", "warn", "d.check"), "w") as fh:
    fh.write("")
judged("and fails so with a .check beside it", unperformed, [], [], "fail", "option: -Wsafe-init (a check teq does not perform: no warning says nothing)", outcome="ok")
marked = test("warn", "e.scala", "object E:\n  def f = 1  // warn\n", ["-Wsafe-init"])
judged("a marked test under such an option is held to scalac's", marked, [dict(sb, file="e.scala")], [dict(tb, file=os.path.join(root, "tests", "warn", "e.scala"))], "pass", outcome="ok")
judged("a test without a mark under such an option fails before the oracle is asked: without an entry too", unperformed, None, [], "fail",
       "option: -Wsafe-init (a check teq does not perform: no warning says nothing)", outcome="ok")
# an oracle that is not this board's ends the run
entry = json.dumps({"row": "neg", "test": "a.scala", "digest": "", "valid": True, "stable": True, "diagnostics": []})
for name, text in [
        ("of another corpus commit", json.dumps({"head": True, "scalac": D.SCALAC, "corpus": "0" * 40}) + "\n" + entry),
        ("of another scalac", json.dumps({"head": True, "scalac": "3.10.0", "corpus": D.TAG_COMMIT}) + "\n" + entry),
        ("without a head line", entry),
        ("that is empty", ""),
        ("holding a test twice", json.dumps({"head": True, "scalac": D.SCALAC, "corpus": D.TAG_COMMIT}) + "\n" + entry + "\n" + entry),
        ("holding another row's entry", json.dumps({"head": True, "scalac": D.SCALAC, "corpus": D.TAG_COMMIT}) + "\n" + entry.replace('"neg"', '"warn"')),
        ("holding a line that is no entry", json.dumps({"head": True, "scalac": D.SCALAC, "corpus": D.TAG_COMMIT}) + "\n{\"row\": \"neg\"")]:
    path = os.path.join(root, "bad.jsonl")
    with open(path, "w") as fh:
        fh.write(text + "\n" if text else "")
    try:
        D.load_oracle(path, "neg")
        ended = None
    except SystemExit as e:
        ended = e.code
    print(("ok\t" if ended == 2 else "FAIL\t") + f"an oracle {name} ends the run" + ("" if ended == 2 else f": it was read ({ended})"))
print(("ok\t" if D.load_oracle(os.path.join(root, "none.jsonl"), "neg") == {} else "FAIL\t") + "a row without an oracle has no entry")
# the summary teq itself prints, as the judge reads it
with open(teq_line) as fh:
    printed = [l.rstrip("\n") for l in fh if l.startswith("warning: there ")]
with open(teq_records) as fh:
    records = [json.loads(l) for l in fh if l.startswith("{")]
theirs = D.teq_sequence(root, records)
ours = D.scalac_sequence({"diagnostics": [scalac("warning", None, None, SUM % ("was 1 deprecation warning", "-deprecation"), None)]})
ok = (len(printed) == 1 and D.SUMMARY_COUNT.match(printed[0][len("warning: "):]) and len(theirs) == 1 and theirs[0][0][4] == ("a summary", 1, "deprecation")
      and theirs[0][0] == ours[0][0])
print(("ok\t" if ok else "FAIL\t") + "the summary teq prints is one by the judge's pattern, equal to scalac's of that count and kind" + ("" if ok else f": printed {printed}, read {theirs}"))
# the line a suite reads a program's output without (tests/support/jars.sh, SUMMARY) is that printed line whole; the
# judge's pattern is its sentence, as either compiler's message has it
ok = len(printed) == 1 and bool(D.BUILD_SUMMARY.match(printed[0])) and not D.BUILD_SUMMARY.match(SUM % ("was 1 deprecation warning", "-deprecation"))
print(("ok\t" if ok else "FAIL\t") + "the summary teq prints is the build's line of tests/support/jars.sh, which scalac's message is not" + ("" if ok else f": printed {printed}"))

# scalac's -Werror error, of which teq keeps no record: the runner's record of it, by teq run as a test's groups are
# (the lean std): once a compilation that warned under --werror, an error beside the warning or not. Beside an error
# the warning is one scalac's typer tells too (a pure expression as a statement, an implicit conversion's feature)
ctx.teq, ctx.teq_env, ctx.cache, ctx.other_platform = os.environ["TEQ"], {}, os.path.join(root, "teq-cache"), lambda t: set()
SEALED = "sealed trait T%s\ncase object P%s extends T%s\ncase object Q%s extends T%s\n"
MATCH = "  def %s(t: T%s) = t match\n    case P%s => 1\n"


def sealed(n=""):
    return SEALED % (n, n, n, n, n)


def match(name="g", n=""):
    return MATCH % (name, n, n)


def compiled(t):
    scratch = os.path.join(root, "scratch", t.name)
    os.makedirs(scratch, exist_ok=True)
    outcome, line, diags, _ = D.compile_groups(ctx, t, scratch, "check", "check")
    return outcome, line, [d for d in diags if d.get("severity") != "outcome"]


def told(diags):
    return ["the -Werror record" if d.get("werror") else d.get("severity") for d in diags]


def same(name, got, want):
    print(("ok\t" + name) if got == want else f"FAIL\t{name}: wanted {want}, got {got}")


def as_scalac(diags):
    """teq's own records as an entry's diagnostics: the marks are what these scenarios ask, not the words."""
    return [scalac("error", None, None, D.WERROR, None) if d.get("werror") else
            scalac(d["severity"], d.get("line"), d.get("col"), d["message"], os.path.basename(d["file"]) if d.get("file") else None) for d in diags]


RECORD = "the -Werror record"
PURE = "  def f: Unit =\n    1\n    ()\n"
both = test("neg", "wb.scala", "object WB:\n  val x: Int = \"\"  // error\n" + PURE + "// nopos-error\n", ["-Werror"])
outcome, line, diags = compiled(both)
same("an error and a warning under --werror: the -Werror record is added, once, after them", told(diags), ["error", "warning", RECORD])
judged("a test marking its error and the -Werror error holds its marks where teq told an error and a warning", both, as_scalac(diags), diags, "pass", outcome=outcome, built=line)
judged("without the record such a test is one error short", both, as_scalac(diags), [d for d in diags if not d.get("werror")], "fail", "marks: 1 errors where 2 are marked", outcome=outcome, built=line)
alone = test("neg", "we.scala", "object WE:\n  val x: Int = \"\"  // error\n", ["-Werror"])
outcome, line, diags = compiled(alone)
same("an error alone under --werror: no record is added", told(diags), ["error"])
judged("a test marking its one error holds its marks under --werror", alone, as_scalac(diags), diags, "pass", outcome=outcome, built=line)
warned = test("neg", "ww.scala", sealed() + "object WW:\n" + match() + "// nopos-error\n", ["-Werror"])
outcome, line, diags = compiled(warned)
same("a warning alone under --werror: the record is added, the build failed by it", (outcome, told(diags)), ("errors", ["warning", RECORD]))
judged("a test marking the -Werror error alone holds its marks where teq warned", warned, as_scalac(diags), diags, "pass", outcome=outcome, built=line)
summed = test("neg", "ws.scala", "object WS:\n  val x: Int = \"\"  // error\n  implicit def conv(n: Int): String = n.toString\n// nopos-error\n", ["-Werror"])
outcome, line, diags = compiled(summed)
same("an error and a summary of unprinted warnings under --werror: the record is added", told(diags), ["error", "warning", RECORD])
os.makedirs(os.path.join(root, "tests", "neg", "wg"))
parts = [os.path.join(root, "tests", "neg", "wg", f) for f in ("A_1.scala", "B_2.scala")]
with open(parts[0], "w") as fh:
    fh.write(sealed(1) + "object A1:\n" + match("g", 1) + match("h", 1))
with open(parts[1], "w") as fh:
    fh.write(sealed(2) + "object B2:\n" + match("g", 2))
groups = D.Test(ROWS.Row("neg", "neg", "neg", "made up", ["-Werror"], []), "wg", [[parts[0]], [parts[1]]], [os.path.dirname(parts[0])])
outcome, line, diags = compiled(groups)
same("two groups under --werror, the first with two warnings: one record a compilation", told(diags), ["warning", "warning", RECORD, "warning", RECORD])
os.makedirs(os.path.join(root, "tests", "neg", "wh"))
parts = [os.path.join(root, "tests", "neg", "wh", f) for f in ("A_1.scala", "B_2.scala")]
with open(parts[0], "w") as fh:
    fh.write(sealed(1) + "object A1:\n" + match("g", 1))
with open(parts[1], "w") as fh:
    fh.write("object B2:\n  val x: Int = \"\"  // error\n// nopos-error\n")
halves = D.Test(ROWS.Row("neg", "neg", "neg", "made up", ["-Werror"], []), "wh", [[parts[0]], [parts[1]]], [os.path.dirname(parts[0])])
outcome, line, diags = compiled(halves)
same("two groups under --werror, the first warns and the second tells an error alone: one record, the first compilation's", told(diags), ["warning", RECORD, "error"])
judged("a test of two groups marking its error and the one -Werror error holds its marks", halves, as_scalac(diags), diags, "pass", outcome=outcome, built=line)
# a teq whose --werror fails no build (the binary, the flag taken out of its arguments): the record is of a build
# that failed, never an error teq did not tell
import shlex
stand_in = os.path.join(root, "teq-without-werror")
with open(stand_in, "w") as fh:
    fh.write('#!/bin/bash\nargs=()\nfor a in "$@"; do [ "$a" = "--werror" ] || args+=("$a"); done\nexec ' + shlex.quote(os.environ["TEQ"]) + ' "${args[@]}"\n')
os.chmod(stand_in, 0o755)
with_record = as_scalac(compiled(warned)[2])
ctx.teq = stand_in
outcome, line, diags = compiled(warned)
ctx.teq = os.environ["TEQ"]
same("a teq that warns under --werror and fails no build gets no record", (outcome, told(diags)), ("ok", ["warning"]))
judged("a test marking the -Werror error fails where teq's build did not fail", warned, with_record, diags, "fail", "marks: no error where 1 are marked", outcome=outcome, built=line)
plain = test("warn", "wn.scala", sealed() + "object WN:\n" + match())
outcome, line, diags = compiled(plain)
same("a warning without --werror: no record, the build passes", (outcome, told(diags)), ("ok", ["warning"]))
mixed = test("neg", "wm.scala", "object WM:\n  val x: Int = \"\"  // error\n" + PURE)
outcome, line, diags = compiled(mixed)
same("an error and a warning without --werror: no record", told(diags), ["error", "warning"])
same("a migration warning beside an error is a warning told", told(D.with_werror(["--werror"], 1, "1 error found", [{"severity": "error"}, {"severity": "migration warning"}])),
     ["error", "migration warning", RECORD])
same("teq's own word that warnings failed the build is a warning told, whatever its records hold",
     told(D.with_werror(["--werror"], 1, "1 warning found, errors under --werror", [])), [RECORD])
same("a warning's record under --werror in a build that did not fail is no -Werror error", told(D.with_werror(["--werror"], 0, "", [{"severity": "warning"}])), ["warning"])
same("an info beside an error is no warning told", told(D.with_werror(["--werror"], 1, "1 error found", [{"severity": "error"}, {"severity": "info"}])), ["error", "info"])
# the oracle's own validation (tests/board/oracle.py validate, no compiler), over a made-up corpus and a made-up file
# of a row of the board's: what the judge asks of an entry is asked of the data, and a run of the row is ended by it
import subprocess
corpus, held = os.path.join(root, "corpus"), os.path.join(root, "held")
os.makedirs(os.path.join(corpus, "tests", "init", "neg"))
os.makedirs(held)
with open(os.path.join(corpus, "tests", "init", "neg", "v.scala"), "w") as fh:
    fh.write("object V:\n  val x: Int = \"\"  // error\n")
v = D.tests_of(next(r for r in ROWS.rows() if r.name == "init-neg"), corpus)[0]
good = {"row": "init-neg", "test": "v.scala", "digest": D.digest(corpus, v.files), "scalac": D.SCALAC, "valid": True, "stable": True,
        "diagnostics": [scalac("error", 2, 16, E, "v.scala")]}


def without(key):
    return {k: x for k, x in good.items() if k != key}


def held_file(entries):
    with open(os.path.join(held, "init-neg.oracle.jsonl"), "w", encoding="utf-8") as fh:
        fh.write("".join(json.dumps(e) + "\n" for e in [{"head": True, "scalac": D.SCALAC, "corpus": D.TAG_COMMIT}] + entries))


def validation(name, entries, failure=None):
    held_file(entries)
    r = subprocess.run([sys.executable, "-B", "tests/board/oracle.py", "validate", "--dir", held, "--row", "init-neg", "--corpus", corpus],
                       capture_output=True, text=True, timeout=60)
    fails = [l for l in r.stdout.splitlines() if l.startswith("FAIL ")]
    ok = (r.returncode, fails) == ((1, ["FAIL init-neg/v.scala: " + failure]) if failure else (0, []))
    print(("ok\t" if ok else "FAIL\t") + name + ("" if ok else f": exit {r.returncode}, {fails} {r.stderr[-300:]}"))


validation("validate: a row's file that holds its corpus' test validates", [good])
validation("validate: an entry twice fails", [good, good], "an entry twice (in init-neg's file)")
validation("validate: an entry that does not say it is stable fails", [without("stable")], "the entry does not say its `stable`")
validation("validate: an entry that does not say it is valid fails", [without("valid")], "the entry does not say its `valid`")
validation("validate: an entry without a digest fails", [without("digest")], "the entry does not say its `digest`")
validation("validate: an entry of another scalac than its file's head fails", [dict(good, scalac="3.10.0")], "the entry is scalac 3.10.0's, its file's head says 3.8.4")
held_file([good])
fine = D.validated(corpus, ["init-neg"], held)
held_file([dict(good, digest="0" * 64)])
spoilt = D.validated(corpus, ["init-neg"], held)
ok = fine[0].startswith("oracle: 1 entries of 1 rows") and fine[1] == [] and spoilt[1] == ["init-neg/v.scala: the digest is not the test's files'"]
print(("ok\t" if ok else "FAIL\t") + "a run takes the validation's failures of the rows it judges" + ("" if ok else f": {fine}, {spoilt}"))

# the run itself, as a gate line runs it: the runner's `main` in a copy of the runner, the made-up corpus named a
# checkout at the tag's commit, the row's own lists beside a file of the row that does not validate
import shutil
copy = os.path.join(root, "copy")
for d in ("board/dotty", "support", "scala3-typing", "pending"):
    os.makedirs(os.path.join(copy, "tests", d))
for f in ("board/dotty.py", "board/oracle.py", "board/rows.py", "support/pending.py", "support/jars.sh", "scala3-typing/harness.py",
          "board/dotty/init-neg.tsv", "pending/dotty-init-neg.txt"):
    shutil.copy(os.path.join("tests", f), os.path.join(copy, "tests", f))
held_file([dict(good, digest="0" * 64)])
shutil.copy(os.path.join(held, "init-neg.oracle.jsonl"), os.path.join(copy, "tests", "board", "dotty"))
subprocess.run(["git", "init", "-q", corpus], capture_output=True, timeout=60)
with open(os.path.join(corpus, ".git", "HEAD"), "w") as fh:
    fh.write(D.TAG_COMMIT + "\n")


def lists():
    return {os.path.join(d, f): open(os.path.join(copy, "tests", d, f), "rb").read() for d in ("board/dotty", "pending") for f in sorted(os.listdir(os.path.join(copy, "tests", d)))}


before = lists()
r = subprocess.run([sys.executable, "-B", os.path.join(copy, "tests", "board", "dotty.py"), "--row", "init-neg", "--update", "--add", "--jobs", "2"],
                   capture_output=True, text=True, timeout=200, env=dict(os.environ, SCALA3_TAG=corpus, BOARD_OUT=os.path.join(root, "copy-out")))
cause = "board: the oracle does not validate (python3 tests/board/oracle.py validate): 1 failed, the first: init-neg/v.scala: the digest is not the test's files'"
ok = r.returncode == 2 and cause in r.stderr.splitlines() and lists() == before
print(("ok\t" if ok else "FAIL\t") + "a run with --update --add ends on a row's file that does not validate: exit 2, the cause named, no list written"
      + ("" if ok else f": exit {r.returncode}, lists {'as they were' if lists() == before else 'written'}, {(r.stdout + r.stderr)[-400:]}"))
PY
while IFS=$'\t' read -r said scenario; do
  check "board: $scenario" "$work/board.log" [ "$said" = ok ]
done < <(grep -E $'^(ok|FAIL)\t' "$work/board.log")
check "board: every scenario of the judge was held" "$work/board.log" eval '[ "$(grep -c "^ok[[:space:]]" "$work/board.log")" = 121 ]'
# A neg test whose .check names an error of a phase after the typing (tests/board/dotty.py's LATER_PHASES: the back
# end's names too long for the JVM and its parameter list past 254 slots, a clash after erasure): where the check
# leaves it failing, built for the JVM too, the marks held to both and its line saying so; never built for a .check
# naming the typer's error, for a row of Scala.js, nor where the check meets the marks; the JVM build's timeout or
# crash the verdict. The builds are a stand-in's records (compile_groups planted), so that no compiler runs.
python3 -B - "$work/board-later" > "$work/board-later.log" 2>&1 <<'PY'
import os, sys
sys.dont_write_bytecode = True
sys.path.insert(0, "tests/board")
import dotty as D
import rows as ROWS
root = sys.argv[1]
os.makedirs(os.path.join(root, "tests", "neg"))
ctx = D.Ctx()
ctx.root, ctx.group_args, ctx.oracle = root, {}, {}
LONG = "-- Error: tests/neg/{0}:1:8 ---\n1 |val x = \"AAAA\" // error\n  |        ^\n  |        String constant is too long for the JVM\n"
TYPER = "-- [E007] Type Mismatch Error: tests/neg/{0}:1:13 ---\n1 |val x: Int = \"\" // error\n  |             ^^\n"
CLASH = "-- [E120] Naming Error: tests/neg/{0}:1:7 ---\n1 |object D extends A: // error\n  |       Name clash between defined and inherited member:\n  |       have the same type (a: Object): Unit after erasure.\n"
def neg(name, check, platform="jvm"):
    path = os.path.join(root, "tests", "neg", name)
    with open(path, "w") as fh:
        fh.write('val x = "AAAA" // error\n')
    with open(path[:-len(".scala")] + ".check", "w") as fh:
        fh.write(check.format(name))
    return D.Test(ROWS.Row("neg", "neg", "neg", "made up", [], [], platform=platform), name, [[path]], [path[:-len(".scala")]])
built = []
def planted(error):
    def compile_groups(ctx, test, scratch, target, tag, extra=None):
        built.append((test.name, target))
        errors = [{"severity": "error", "file": test.files[0], "line": 1, "col": 9, "message": error}] if error else []
        return ("errors" if errors else "ok", errors[0]["message"] if errors else "", errors, None)
    return compile_groups
def said(name, got, verdict, line, builds):
    ok = got[0] == verdict and (got[1] == line if not line.endswith("...") else got[1].startswith(line[:-3])) and builds == built
    print(("ok\t" + name) if ok else f"FAIL\t{name}: wanted {(verdict, line, builds)}, the judge said {got} after the builds {built}")
    built.clear()
check_error = [{"severity": "error", "file": None, "line": None, "col": None, "message": "x"}]
D.compile_groups = planted("String constant is too long for the JVM")
t = neg("long.scala", LONG)
said("a back end's error the JVM build gives turns its test", D.neg_judged(ctx, t, root, "ok", "", []), "marks", D.MARKS_ALONE + "...", [("long.scala", "jvm")])
t = neg("clash.scala", CLASH)
said("a clash after erasure is built for the JVM too", D.neg_judged(ctx, t, root, "ok", "", []), "marks", D.MARKS_ALONE + "...", [("clash.scala", "jvm")])
D.compile_groups = planted(None)
t = neg("long2.scala", LONG)
said("a back end's error the JVM build does not give: failing, its line saying the build ran",
     D.neg_judged(ctx, t, root, "ok", "", []), "fail", "marks: no error where 1 are marked (the JVM build)", [("long2.scala", "jvm")])
t = neg("typer.scala", TYPER)
said("a .check naming the typer's error is never built", D.neg_judged(ctx, t, root, "ok", "", []), "fail", "marks: no error where 1 are marked", [])
t = neg("js.scala", LONG, platform="scala-js")
said("a row of Scala.js is never built for the JVM", D.neg_judged(ctx, t, root, "ok", "", []), "fail", "marks: no error where 1 are marked", [])
t = neg("met.scala", LONG)
met = [{"severity": "error", "file": t.files[0], "line": 1, "col": 9, "message": "String constant is too long for the JVM"}]
said("a test the check already meets is never built", D.neg_judged(ctx, t, root, "errors", "", met), "marks", D.MARKS_ALONE + "...", [])
for outcome, line in (("timeout", "a limit of 60 s"), ("crash", "teq: internal error")):
    def compile_groups(ctx, test, scratch, target, tag, extra=None, outcome=outcome, line=line):
        built.append((test.name, target))
        return (outcome, line, [], None)
    D.compile_groups = compile_groups
    t = neg(f"{outcome}.scala", LONG)
    said(f"the JVM build's {outcome} is the verdict, its line saying the build", D.neg_judged(ctx, t, root, "ok", "", []), outcome, f"{line} (the JVM build)", [(f"{outcome}.scala", "jvm")])
PY
while IFS=$'\t' read -r said scenario; do
  check "board: $scenario" "$work/board-later.log" [ "$said" = ok ]
done < <(grep -E $'^(ok|FAIL)\t' "$work/board-later.log")
check "board: every scenario of the later phases' leg was held" "$work/board-later.log" eval '[ "$(grep -c "^ok[[:space:]]" "$work/board-later.log")" = 8 ]'

fi
if piece_has board-cap; then
piece_ran board-cap
# The board's memory cap leaves no cgroup behind: a test's cap is the run's watchdog, the run in one scope of its
# own (a cgroup a test's teq wrote files in outlives it while their page cache stays charged to it). Over a made-up
# corpus of 30 pos tests, outputs on a tmpfs, the real teq behind a stand-in that notes the cgroup it runs in: the
# tests ran in at most ten cgroups (a cgroup a test, the old form, is 30); a runaway killed with its child, read as
# the memory cap; the timeout's path; a terminating signal ends every
# test; a run in a board scope already is not executed again. No lingering manager, no systemd-run or no tmpfs:
# each line is `skip` with the reason (not run, never a pass).
python3 -B - "$work/memcap" > "$work/memcap.log" 2>&1 <<'PY'
import json, os, re, shutil, signal, subprocess, sys, tempfile, time
sys.path.insert(0, "tests/board")
import dotty as D
root = sys.argv[1]
def out(word, name, why=""):
    print(f"{word}\t{name}" + (f": {why}" if why else ""), flush=True)
shm = next((m for m in ("/dev/shm", "/tmp") if any(l.split()[1] == m and l.split()[2] == "tmpfs" for l in open("/proc/mounts"))), None)
lingers = subprocess.run(["/usr/bin/loginctl", "show-user", str(os.getuid()), "--value", "-p", "Linger"], capture_output=True, text=True).stdout.strip() == "yes" if os.path.exists("/usr/bin/loginctl") else False
CHECKS = ["the tests run in at most ten cgroups", "a runaway is the memory cap, its child killed with it", "a test past the wall is a timeout, its child killed with it",
          "a terminating signal to the run ends every test", "a run started in a scope of another name runs in a board scope of its own",
          "two processes that share their pages are counted once: not killed", "a grandchild its parent left is the test's: the memory cap",
          "a test's own signal 9 is a crash, not the memory cap", "a child in a process group of its own is killed at the wall",
          "a green run writes its lists, the committed one the run's own bytes", "a test that dies by its own signal 9 past the cap is no kill of the watchdog's",
          "a signal at any boundary of the publication leaves every list old or every list new, the exit 128 + the signal",
          "a test the kernel kills in the run's scope fails the run, no public list changed",
          "a kill at every interval of the run and of its publication changes no public list (the supervisor's own kill: the marker), the control publishes all",
          "without a scope a run goes on uncapped and unread; under BOARD_REQUIRE_SCOPE it refuses",
          "every outcome (a listed test passing, an unlisted one failing, both, neither) of a plain run, --update, --add and --update --add: a nonzero exit changes no public list, its stage the ratchet's list; exit 0 is exactly the ratchet's change",
          "a kernel kill before the supervisor's first look at the scope fails the run, no public list changed; one in a sibling scope fails no run"]
why = None if shm and lingers and os.path.exists("/usr/bin/systemd-run") else ("no tmpfs" if not shm else "no lingering user manager or no systemd-run")
if why:
    for c in CHECKS:
        out("skip", c, why)
    sys.exit(0)
os.makedirs(root)
outs = tempfile.mkdtemp(prefix="gate-memcap.", dir=shm)
corpus, copy = os.path.join(root, "corpus"), os.path.join(root, "copy")
os.makedirs(os.path.join(corpus, "tests", "pos"))
for i in range(1, 31):
    with open(os.path.join(corpus, "tests", "pos", f"t{i:02d}.scala"), "w") as fh:
        fh.write(f"object T{i:02d} {{ val x: Int = {i} }}\n")
os.makedirs(os.path.join(corpus, "compiler", "test", "dotc"))
open(os.path.join(corpus, "compiler", "test", "dotc", "pos-linting.allowlist"), "w").close()
subprocess.run(["git", "init", "-q", corpus], capture_output=True, timeout=60)
with open(os.path.join(corpus, ".git", "HEAD"), "w") as fh:
    fh.write(D.TAG_COMMIT + "\n")
for d in ("board/dotty", "support", "scala3-typing", "pending"):
    os.makedirs(os.path.join(copy, "tests", d))
for f in ("board/dotty.py", "board/oracle.py", "board/rows.py", "support/pending.py", "support/jars.sh", "scala3-typing/harness.py"):
    shutil.copy(os.path.join("tests", f), os.path.join(copy, "tests", f))
standin = os.path.join(root, "teq")
with open(standin, "w") as fh:
    fh.write("""#!/bin/sh
sed -n 's/^0:://p' /proc/self/cgroup >> "$MEMCAP_NOTE"
for a in "$@"; do case $a in
  *"/$MEMCAP_TEST") sleep 300 & echo $! > "$MEMCAP_CHILD"
    case $MEMCAP_DO in
      grow) exec python3 -c 'a=[]\nwhile True: a.append(bytearray(50000000))' ;;
      hold) exec python3 -c 'import time; a=bytearray(300000000); time.sleep(20)' ;;
      share) exec python3 -c 'import os, time; a=bytearray(300000000); pid=os.fork(); time.sleep(2); os._exit(0) if pid == 0 else os.waitpid(pid, 0)' ;;
      orphan) exec python3 -c 'import os, time\nr, w = os.pipe()\nif os.fork() == 0:\n    if os.fork() == 0:\n        a = []\n        while True: a.append(bytearray(50000000))\n    os._exit(0)\nos.wait(); time.sleep(30)' ;;
      selfkill) exec sh -c 'kill -9 $$' ;;
      fail) echo "$MEMCAP_TEST:1:1: error: a made-up failure" >&2; exit 1 ;;
      pgid) exec python3 -c 'import os, time\nif os.fork() == 0:\n    os.setpgid(0, 0); open(os.environ["MEMCAP_CHILD"], "w").write(str(os.getpid())); time.sleep(300)\ntime.sleep(60)' ;;
      *) exec sleep 60 ;;
    esac ;;
esac; done
exec "$MEMCAP_TEQ" "$@"
""")
os.chmod(standin, 0o755)
def board(tag, only=None, do="", test="", jobs=4, extra=None, wrap=None, wait=True):
    note, child = os.path.join(root, tag + ".cgroups"), os.path.join(root, tag + ".child")
    env = dict(os.environ, SCALA3_TAG=corpus, BOARD_OUT=os.path.join(outs, tag), TEQ=standin, MEMCAP_TEQ=os.environ["TEQ"], MEMCAP_NOTE=note,
               MEMCAP_CHILD=child, MEMCAP_DO=do, MEMCAP_TEST=test)
    env.pop("BOARD_SCOPE", None)
    env.update(extra or {})
    cmd = (wrap or []) + [sys.executable, "-B", os.path.join(copy, "tests", "board", "dotty.py"), "--row", "pos", "--update", "--add", "--jobs", str(jobs)] + (["--only", only] if only else [])
    if not wait:
        return subprocess.Popen(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True), note, child
    r = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=300)
    return r, note, child
def verdict(tag, test):
    p = os.path.join(outs, tag, "lists", "pos.tsv")
    return next((l.rstrip("\n").split("\t") for l in open(p) if l.startswith(test + "\t")), None) if os.path.exists(p) else None
def gone(child):
    try:
        os.kill(int(open(child).read()), 0)
        return False
    except (OSError, ValueError):
        return True
try:
    U = "/sys/fs/cgroup/user.slice/user-%d.slice/user@%d.service/cgroup.stat" % (os.getuid(), os.getuid())
    dying = lambda: int(next((l.split()[1] for l in open(U) if l.startswith("nr_dying_descendants")), 0)) if os.path.exists(U) else -1
    before = dying()
    r, note, _ = board("all")
    cg = set(open(note).read().split()) if os.path.exists(note) else set()
    ok = r.returncode == 0 and 1 <= len(cg) <= 10
    out("ok" if ok else "FAIL", CHECKS[0], f"{len(cg)} cgroups, exit {r.returncode}; the user manager's dying cgroups {before} -> {dying()} (not judged)" + ("" if ok else " " + (r.stdout + r.stderr)[-300:]))
    run_list, committed = os.path.join(outs, "all", "lists", "pos.tsv"), os.path.join(copy, "tests", "board", "dotty", "pos.tsv")
    ok = (r.returncode == 0 and os.path.exists(run_list) and os.path.exists(committed) and open(run_list, "rb").read() == open(committed, "rb").read()
          and os.path.exists(os.path.join(copy, "tests", "pending", "dotty-pos.txt")) and os.path.exists(os.path.join(outs, "all", "peaks.tsv")))
    out("ok" if ok else "FAIL", CHECKS[9], "" if ok else f"exit {r.returncode}, {sorted(os.listdir(os.path.join(outs, 'all')))}")
    r, _, child = board("grow", only="^t01", do="grow", test="t01.scala", extra={"TEQ_CAP": "600000000"})
    v = verdict("grow", "t01.scala")
    ok = v is not None and v[1] == "crash" and v[3] == "teq: signal 9 (the memory cap)" and gone(child)
    out("ok" if ok else "FAIL", CHECKS[1], "" if ok else f"{v}, the child {'gone' if gone(child) else 'left'}")
    r, _, child = board("wall", only="^t02", do="sleep", test="t02.scala", extra={"BOARD_WALL_LIMIT": "3"})
    v = verdict("wall", "t02.scala")
    ok = v is not None and v[1] == "timeout" and gone(child)
    out("ok" if ok else "FAIL", CHECKS[2], "" if ok else f"{v}, the child {'gone' if gone(child) else 'left'}")
    p, _, child = board("signal", only="^t05", do="sleep", test="t05.scala", wait=False)
    deadline = time.time() + 120
    while not os.path.exists(child) and time.time() < deadline:
        time.sleep(0.2)
    time.sleep(0.5)
    p.send_signal(signal.SIGTERM)
    code = p.wait(timeout=60)
    deadline = time.time() + 5  # a killed process is gone once it is reaped, which a loaded machine does late
    while os.path.exists(child) and not gone(child) and time.time() < deadline:
        time.sleep(0.1)
    ok = code == 128 + signal.SIGTERM and os.path.exists(child) and gone(child)
    out("ok" if ok else "FAIL", CHECKS[3], "" if ok else f"exit {code}, the test {'gone' if gone(child) else 'left'}")
    unit = f"memcap-outer-{os.getpid()}"
    r, note, _ = board("nested", only="^t06", wrap=["/usr/bin/systemd-run", "--user", "--quiet", "--scope", f"--unit={unit}"])
    cg = set(open(note).read().split()) if os.path.exists(note) else set()
    ok = r.returncode == 0 and len(cg) == 1 and re.search(r"/board-\d+\.scope$", next(iter(cg))) is not None
    out("ok" if ok else "FAIL", CHECKS[4], "" if ok else f"{cg}, exit {r.returncode}")
    r, _, _ = board("share", only="^t07", do="share", test="t07.scala", extra={"TEQ_CAP": "450000000"})
    v = verdict("share", "t07.scala")
    ok = v is not None and v[1] == "pass"
    out("ok" if ok else "FAIL", CHECKS[5], "" if ok else f"{v}")
    r, _, _ = board("orphan", only="^t08", do="orphan", test="t08.scala", extra={"TEQ_CAP": "600000000"})
    v = verdict("orphan", "t08.scala")
    ok = v is not None and v[1] == "crash" and v[3] == "teq: signal 9 (the memory cap)"
    out("ok" if ok else "FAIL", CHECKS[6], "" if ok else f"{v}")
    r, _, _ = board("selfkill", only="^t09", do="selfkill", test="t09.scala")
    v = verdict("selfkill", "t09.scala")
    ok = v is not None and v[1] == "crash" and v[3] == "teq: signal 9"
    out("ok" if ok else "FAIL", CHECKS[7], "" if ok else f"{v}")
    r, _, child = board("pgid", only="^t10", do="pgid", test="t10.scala", extra={"BOARD_WALL_LIMIT": "3"})
    time.sleep(0.5)
    v = verdict("pgid", "t10.scala")
    ok = v is not None and v[1] == "timeout" and os.path.exists(child) and gone(child)
    out("ok" if ok else "FAIL", CHECKS[8], "" if ok else f"{v}, the child {'gone' if gone(child) else 'left'}")
    # the watchdog sees the test past the cap and reads its proportional memory; the test kills itself before the
    # kill, which then meets its zombie: the crash is the test's own
    sys.path.insert(0, os.path.join(copy, "tests", "board"))
    import importlib
    B = importlib.import_module("dotty")
    marker = os.path.join(root, "selfkill.go")
    real = B.proportional
    def proportional(pid):
        n = real(pid)
        if n > 32 << 20:
            open(marker, "w").close()
            until = time.monotonic() + 2
            while time.monotonic() < until:
                try:
                    if open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()[0] == "Z":
                        break
                except OSError:
                    break
                time.sleep(0.001)
        return n
    B.proportional = proportional
    code = f"import os, time\na = bytearray(64 << 20)\nwhile not os.path.exists({marker!r}): time.sleep(0.001)\nos.kill(os.getpid(), 9)\n"
    status = B.run([sys.executable, "-B", "-c", code], os.path.join(root, "selfkill.out"), 0, 10, cap=32 << 20)[0]
    out("ok" if status == "signal 9" else "FAIL", CHECKS[10], "" if status == "signal 9" else f"read as {status!r}")
    # the publication: four writes (two through pending's writer, two files), a signal before the first, between
    # each pair, after the last; SIGTERM, SIGINT, SIGHUP, and two signals in a row
    pub = """import os, signal, sys
sys.path.insert(0, sys.argv[1])
import dotty as B
d, sigs, k = sys.argv[2], [int(x) for x in sys.argv[3].split(",")], int(sys.argv[4])
for s in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
    signal.signal(s, B.end_sessions)
def fire():
    for s in sigs:
        os.kill(os.getpid(), s)
def write_pending(suite, listed):
    if suite == "p2" and k == 2:
        fire()
    open(os.path.join(d, suite), "w").write("new")
    if (suite, k) in (("p0", 1), ("p2", 3), ("p0", 5)):
        fire()
    if (suite, k) == ("p4", 5):
        os.kill(os.getpid(), signal.SIGINT)
staged = [("pending", "p0", None), ("file", os.path.join(d, "f1"), b"new"), ("pending", "p2", None), ("file", os.path.join(d, "f3"), b"new")]
if k == 5:  # the handoff: a SIGTERM held at the first boundary, then a SIGINT after the last write
    sigs = [signal.SIGTERM]
    staged.append(("pending", "p4", None))
if k == 0:
    fire()
B.publish(staged, write_pending)
if k == 4:
    fire()
"""
    bad = []
    cases = [(str(s), k) for s in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP) for k in range(5)] + [(f"{int(signal.SIGTERM)},{int(signal.SIGINT)}", 2), (str(int(signal.SIGTERM)), 5)]
    for sigs, k in cases:
        d = tempfile.mkdtemp(dir=root)
        for n in ("p0", "f1", "p2", "f3"):
            open(os.path.join(d, n), "w").write("old")
        r = subprocess.run([sys.executable, "-B", "-c", pub, os.path.join(copy, "tests", "board"), d, sigs, str(k)], capture_output=True, text=True, timeout=60)
        got = {open(os.path.join(d, n)).read() for n in ("p0", "f1", "p2", "f3")}
        want = {"old"} if k == 0 else {"new"}
        if got != want or r.returncode != 128 + int(sigs.split(",")[0]):
            bad.append(f"signals {sigs} at boundary {k}: exit {r.returncode}, the lists {sorted(got)} {r.stderr[-200:]}")
    out("ok" if not bad else "FAIL", CHECKS[11], f"{len(cases)} cases" + ("" if not bad else ": " + "; ".join(bad[:3])))
    # the supervisor's two forms and its twin. The public lists: the copy's committed and pending lists of pos and
    # the twin's BOARD_OUT list, read before and after each run (its peaks.tsv changes with every run: no list)
    pub = lambda tag: {f: open(f, "rb").read() for f in [os.path.join(copy, "tests", "board", "dotty", "pos.tsv"), os.path.join(copy, "tests", "pending", "dotty-pos.txt"),
                                                       os.path.join(outs, tag, "lists", "pos.tsv")] if os.path.exists(f)}  # peaks.tsv a measure, not a list
    marker = os.path.join(copy, "tests", "board", "dotty", ".publication-interrupted")
    before = pub("twin")
    r, _, _ = board("twin", do="hold", test="t03.scala", jobs=2, extra={"BOARD_BACKSTOP": "250000000"})
    last = (r.stdout.strip().splitlines() or [""])[-1]
    ok = r.returncode == 1 and "the kernel killed" in last and pub("twin") == before
    out("ok" if ok else "FAIL", CHECKS[12], "" if ok else f"exit {r.returncode}, {last!r}, the lists {'as they were' if pub('twin') == before else 'changed'}")
    # the twin: t01 failing would change pos.tsv and the pending list; a kill at each interval must leave them
    r, _, _ = board("twin")
    base = pub("twin")
    probe, _, _ = board("twin", do="fail", test="t01.scala", extra={"BOARD_KILL_AT": "end"})
    kept = re.search(r"the stage kept in (\S+?)\)", probe.stdout)
    n = len(json.load(open(os.path.join(kept.group(1), "manifest.json")))["entries"]) if kept else 0
    bad, cases = [], 0
    if probe.returncode == 0 or pub("twin") != base or n < 3:
        bad.append(f"the probe: exit {probe.returncode}, {n} staged entries")
    for at in [f"stage:{k}" for k in range(1, n + 1)] + ["end", "holder"]:
        cases += 1
        r, _, _ = board("twin", do="fail", test="t01.scala", extra={"BOARD_KILL_AT": at})
        if r.returncode == 0 or pub("twin") != base:
            bad.append(f"{at}: exit {r.returncode}, the lists {'as they were' if pub('twin') == base else 'changed'}")
    for k in range(1, n + 1):
        cases += 1
        r, _, _ = board("twin", do="fail", test="t01.scala", extra={"BOARD_KILL_AT": f"rename:{k}"})
        plain, _, _ = board("twin", only="^t02")
        cleared, _, _ = board("twin")
        if not (r.returncode != 0 and plain.returncode == 1 and "a publication was interrupted" in plain.stdout and cleared.returncode == 0
                and not os.path.exists(marker) and pub("twin") == base):
            bad.append(f"rename:{k}: exit {r.returncode}, a plain run's {plain.returncode} {(plain.stdout.strip().splitlines() or [''])[-1][:60]!r}, "
                       f"the whole --update's {cleared.returncode}, the marker {'left' if os.path.exists(marker) else 'gone'}")
    cases += 1
    r, _, _ = board("twin", do="fail", test="t01.scala", extra={"BOARD_ALLOC_AT_RENAME": "300000000"})
    changed = pub("twin")
    if r.returncode != 0 or changed == base or b"t01.scala\tfail" not in changed.get(os.path.join(copy, "tests", "board", "dotty", "pos.tsv"), b""):
        bad.append(f"the pressure at each rename: exit {r.returncode}, the lists {'as they were' if changed == base else 'changed'}")
    out("ok" if not bad else "FAIL", CHECKS[13], f"{cases} cases, {n} staged entries" + ("" if not bad else ": " + "; ".join(bad[:3])))
    bare, _, _ = board("noscope", only="^t11", extra={"BOARD_SYSTEMD_RUN": "/bin/false"})
    gated, _, _ = board("noscope2", only="^t11", extra={"BOARD_SYSTEMD_RUN": "/bin/false", "BOARD_REQUIRE_SCOPE": "1"})
    ok = (bare.returncode == 0 and "uncapped and unread" in bare.stdout and gated.returncode == 2 and "the run refuses without one" in gated.stdout
          and not os.path.exists(os.path.join(outs, "noscope2", "lists")))
    out("ok" if ok else "FAIL", CHECKS[14], "" if ok else f"bare: exit {bare.returncode}, gated: exit {gated.returncode} {(gated.stdout.strip().splitlines() or [''])[-1][:80]!r}")
    # The counters from zero: the supervisor late to its first look at the scope (BOARD_DELAY_LOOK) while the kernel
    # kills a test of the run (a kill no reading before the run may absorb), for each delay; and the control, a kill
    # in a sibling scope during a clean run, which is not this run's
    base = pub("twin")
    bad = []
    for delay in ("4", "8"):
        r, _, _ = board("twin", do="hold", test="t03.scala", jobs=2, extra={"BOARD_BACKSTOP": "250000000", "BOARD_DELAY_LOOK": delay})
        last = (r.stdout.strip().splitlines() or [""])[-1]
        if r.returncode == 0 or "the kernel killed" not in last or pub("twin") != base:
            bad.append(f"late {delay} s: exit {r.returncode}, {last[:80]!r}, the lists {'as they were' if pub('twin') == base else 'changed'}")
    sibling = subprocess.Popen(["/usr/bin/systemd-run", "--user", "--quiet", "--scope", f"--unit=memcap-sibling-{os.getpid()}", "-pMemoryMax=100M",
                                "-pMemorySwapMax=0", sys.executable, "-c", "import time; a = bytearray(300000000); time.sleep(5)"],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    r, _, _ = board("twin", jobs=2, extra={"BOARD_DELAY_LOOK": "2"})
    killed = sibling.wait(timeout=120)
    if r.returncode != 0 or killed not in (-9, 137):
        bad.append(f"a sibling's kill: the run's exit {r.returncode}, the sibling's {killed}, {(r.stdout.strip().splitlines() or [''])[-1][:80]!r}")
    out("ok" if not bad else "FAIL", CHECKS[16], "two delays and a sibling" + ("" if not bad else ": " + "; ".join(bad)))
    # The outcomes, generated: a corpus of a test that passes (was_bad) and one that fails (new_bad); a cell's pending
    # list holds was_bad (a listed test that passes now) or not, and new_bad (else an unlisted failure) or not; the
    # row's committed list is the run's own. By the ratchet (tests/support/pending.py hold): the run is as listed
    # (exit 0) when no listed test passes that --update does not take out and no test fails unlisted that --add does
    # not write in; the list after is the old one, less was_bad under --update, with new_bad under --add. Exit 0: the
    # public lists are exactly that; else they are the bytes before, and the stage's pending list, if it staged one,
    # is that list.
    oc, ocopy = os.path.join(root, "outcomes"), os.path.join(root, "ocopy")
    os.makedirs(os.path.join(oc, "tests", "pos"))
    open(os.path.join(oc, "tests", "pos", "was_bad.scala"), "w").write("object WasBad { val x: Int = 1 }\n")
    open(os.path.join(oc, "tests", "pos", "new_bad.scala"), "w").write('object NewBad { val x: Int = "wrong" }\n')
    os.makedirs(os.path.join(oc, "compiler", "test", "dotc"))
    open(os.path.join(oc, "compiler", "test", "dotc", "pos-linting.allowlist"), "w").close()
    subprocess.run(["git", "init", "-q", oc], capture_output=True, timeout=60)
    open(os.path.join(oc, ".git", "HEAD"), "w").write(D.TAG_COMMIT + "\n")
    shutil.copytree(copy, ocopy, ignore=shutil.ignore_patterns("*.tsv", "dotty-*.txt", ".publication-interrupted"))
    committed, pend = os.path.join(ocopy, "tests", "board", "dotty", "pos.tsv"), os.path.join(ocopy, "tests", "pending", "dotty-pos.txt")
    def orun(tag, flags):
        env = dict(os.environ, SCALA3_TAG=oc, BOARD_OUT=os.path.join(outs, tag))
        env.pop("BOARD_SCOPE", None)
        return subprocess.run([sys.executable, "-B", os.path.join(ocopy, "tests", "board", "dotty.py"), "--row", "pos", "--jobs", "2"] + flags,
                              env=env, capture_output=True, text=True, timeout=300)
    def names(path):
        return {l.split("\t")[0] for l in open(path, encoding="utf-8") if l.strip()} if os.path.exists(path) else set()
    # the committed list: the run's own, by a --add over an empty pending list (new_bad then listed)
    first = orun("oseed", ["--update", "--add"])
    seed = open(committed, "rb").read() if os.path.exists(committed) else b""
    bad, cells = [] if first.returncode == 0 and b"was_bad.scala\tpass" in seed and b"new_bad.scala\tfail" in seed else [f"the seed: exit {first.returncode} {first.stdout[-200:]!r}"], 0
    for listed_passes in (True, False):
        for unlisted_fails in (True, False):
            for flags in ([], ["--update"], ["--add"], ["--update", "--add"]):
                cells += 1
                old = ({"was_bad.scala": "previous failure"} if listed_passes else {}) | ({} if unlisted_fails else {"new_bad.scala": "listed failure"})
                open(pend, "w").write("".join(f"{n}\t{w}\n" for n, w in sorted(old.items())))
                open(committed, "wb").write(seed)
                before = {committed: seed, pend: open(pend, "rb").read()}
                update, add = "--update" in flags, "--add" in flags
                want_exit = 1 if (listed_passes and not update) or (unlisted_fails and not add) else 0
                want = (set(old) - ({"was_bad.scala"} if update else set())) | ({"new_bad.scala"} if add and unlisted_fails else set())
                tag = f"o{cells}"
                r = orun(tag, flags)
                after = {committed: open(committed, "rb").read(), pend: open(pend, "rb").read()}
                cell = f"{'listed passes' if listed_passes else ''}{'+' if listed_passes and unlisted_fails else ''}{'unlisted fails' if unlisted_fails else ''}{'neither' if not (listed_passes or unlisted_fails) else ''} {' '.join(flags) or 'plain'}"
                kept = re.search(r"the stage kept in (\S+?)\)", r.stdout)
                if r.returncode != want_exit:
                    bad.append(f"{cell}: exit {r.returncode}, the rule's {want_exit}")
                elif want_exit:
                    staged = D.read_stage(kept.group(1)) if kept else None
                    spend = next((dict(w) for k, s_, w in (staged or []) if k == "pending" and s_ == "dotty-pos"), None)
                    if after != before:
                        bad.append(f"{cell}: exit {want_exit} and a public list changed")
                    elif (update or add) and (staged is None or (spend is not None and set(spend) != want)):
                        bad.append(f"{cell}: the stage's list {sorted(spend) if spend is not None else staged and 'none'}, the ratchet's {sorted(want)}")
                else:
                    kept_lines = all(f"{n}\t{w}".encode() in after[pend] for n, w in old.items() if n in want)
                    run_list = open(os.path.join(outs, tag, "lists", "pos.tsv"), "rb").read() if os.path.exists(os.path.join(outs, tag, "lists", "pos.tsv")) else None
                    if names(pend) != want or not kept_lines:
                        bad.append(f"{cell}: the list {sorted(names(pend))}, the ratchet's {sorted(want)}")
                    elif (update or add) and after[committed] != run_list or not (update or add) and after[committed] != seed:
                        bad.append(f"{cell}: the row's list is not {'the run' if update or add else 'the one before'}'s")
    out("ok" if not bad else "FAIL", CHECKS[15], f"{cells} cells" + ("" if not bad else f", {len(bad)} wrong: " + "; ".join(bad[:4])))
finally:
    shutil.rmtree(outs, ignore_errors=True)
PY
while IFS=$'\t' read -r said scenario; do
  case $said in
    skip) echo "skip the board's memory cap: $scenario" ;;
    *) check "the board's memory cap: $scenario" "$work/memcap.log" [ "$said" = ok ] ;;
  esac
done < <(grep -E $'^(ok|FAIL|skip)\t' "$work/memcap.log")
memcap_reported=$(grep -c -E $'^(ok|FAIL|skip)\t' "$work/memcap.log")
check "the board's memory cap: every check reported" "$work/memcap.log" [ "$memcap_reported" = 17 ]

fi
if piece_has outcomes; then
piece_ran outcomes
# The board counts no check that did not run: a test's one outcome (a failure, then not run, then a pass) through
# tests/support/pending.py in a copy of the tree's lists, tests/board/board.py's project rows over a made-up record,
# the modules' known and deferred lists held both ways, a watch session's opening check that fails, and the JVM
# runner's checks without their tool or jar.
oc=$work/outcomes
mkdir -p "$oc/r/tests/support" "$oc/r/tests/pending" "$oc/r/tests/board/suites" "$oc/r/tests/board/dotty" "$oc/r/bench/probe"
cp tests/support/pending.py "$oc/r/tests/support/"
cp tests/board/board.py tests/board/rows.py "$oc/r/tests/board/"
pend() { (cd "$oc" && timeout 60 python3 -B r/tests/support/pending.py "$@") > "$oc/pending.log" 2>&1; }
printf 'package_reread\tearlier mismatch\n' > "$oc/r/tests/pending/w.txt"
printf 'skip package_reread\tnot in the coursier cache: sourcecode\n' > "$oc/skip.txt"
pend w skip.txt --fails-only --passed=0 --update
code=$?
check "outcomes: a skipped listed check is not run: the list keeps it, no pass, cannot run on the board" "$oc/pending.log" eval '[ $code = 0 ] &&
  [ "$(cat "$oc/r/tests/pending/w.txt")" = "$(printf "package_reread\tearlier mismatch")" ] &&
  grep -q -x "w: 0 pass, 0 fail as tests/pending/w.txt lists, 1 not run" "$oc/pending.log" &&
  grep -q -x "$(printf "package_reread\tcannot run\t-\tnot in the coursier cache: sourcecode")" "$oc/r/tests/board/suites/w.tsv"'
printf 'b\tolder\nc\tolder\n' > "$oc/r/tests/pending/v.txt"
printf 'pass a\nskip a\tno jar\nknown b\tknown classes: a reason\nfail c\twhy\nskip c\tno jar\npass d\n' > "$oc/one.txt"
pend v one.txt --update
code=$?
check "outcomes: one outcome a test, a failure before not run before a pass; a known one fails on the board, held by its own list" "$oc/pending.log" eval '[ $code = 0 ] &&
  [ "$(cat "$oc/r/tests/pending/v.txt")" = "$(printf "c\tolder")" ] &&
  grep -q -x "v: 1 pass, 1 fail as tests/pending/v.txt lists, 1 fail as the suite.s own lists of checks hold them, 1 not run" "$oc/pending.log" &&
  [ "$(grep -v "^#" "$oc/r/tests/board/suites/v.tsv" | cut -f1,2 | tr "\t\n" ": ")" = "a:cannot run b:fail c:fail d:pass " ]'
printf '# test\tverdict\tthe other ways\tthe first line of the difference\nwin\tpass\t-\t\n' > "$oc/r/tests/board/suites/u.tsv"
printf "skip win\tanother system's\n" > "$oc/os.txt"
pend u os.txt --update
check "outcomes: another system's test is not run on this one, no earlier machine's pass kept" "$oc/pending.log" \
  grep -q -x "$(printf "win\tcannot run\t-\tanother system's")" "$oc/r/tests/board/suites/u.tsv"
# a project's test the oracle's run ignored: not run, no pass
printf '{}\n' > "$oc/r/bench/probe/pin.json"
printf '{"cases_count":2,"modules":{"probe.js":{"cases":{"Success":["p.Ran.a"],"Ignored":["p.Unrun.b"]}}}}\n' > "$oc/r/bench/probe/tests.json"
printf '{"projects":["probe.js"],"deviations":[]}\n' > "$oc/r/bench/probe/deviations.json"
(cd "$oc/r" && timeout 60 python3 -B tests/board/board.py --tsv) > "$oc/board.log" 2>&1
check "outcomes: a project's test its oracle ignored is not run: 2 tests, 1 pass, 1 cannot run" "$oc/board.log" \
  grep -q -x "$(printf "probe test\t2\t1\t0\t0\t1\t")" "$oc/board.log"
# the modules' lists, both ways: a listed check that passes fails the run, or under --update leaves its list
(
  out=$oc/m; mkdir -p "$out/results"; pass=0; fail=0; stale=0; known_count=0; deferred_count=0
  known=$oc/known.txt deferred=$oc/deferred.txt
  printf 'ledgerlocalclass classes  a reason\nother subset  another\n' > "$known"; : > "$deferred"
  source <(sed -n '/^is_known() {/p; /^is_deferred() {/p; /^listed_passes() {/,/^}/p; /^result() {/,/^}/p' tests/modules.sh)
  result ledgerlocalclass classes 1 ""
  echo "counts: pass=$pass fail=$fail stale=$stale known=$known_count"
  result ledgerlocalclass classes 0 ""
  echo "counts: pass=$pass fail=$fail stale=$stale known=$known_count"
  MODULES_LISTS_UPDATE=1 result ledgerlocalclass classes 1 ""
  echo "counts: pass=$pass fail=$fail stale=$stale known=$known_count"
  cat "$out/results/lists-off.txt"
) > "$oc/modules.log" 2>&1
check "outcomes: a known check that passes fails the run; under --update it leaves the list as a pass" "$oc/modules.log" eval '
  grep -q -x "FAIL ledgerlocalclass classes: listed in $oc/known.txt and passes now (take it off: --update)" "$oc/modules.log" &&
  [ "$(grep "^counts: " "$oc/modules.log" | tr "\n" " ")" = "counts: pass=0 fail=0 stale=1 known=0 counts: pass=0 fail=0 stale=1 known=1 counts: pass=1 fail=0 stale=1 known=1 " ] &&
  grep -q -x "$oc/known.txt ledgerlocalclass classes" "$oc/modules.log"'
# a watch session's opening check: the jar there, a type error in the source is a failure, never a skip
(
  pass=0; fail=0; skipped=0
  bad() { echo "FAIL $1"; fail=$((fail + 1)); }
  not_run() { echo "skip $1: $2"; skipped=$((skipped + 1)); }
  start() { echo "started"; }
  src=$oc/bad; work=$oc; mkdir -p "$src"; printf 'object Bad { val n: Int = "wrong" }\n' > "$src/Bad.scala"
  . tests/support/jars.sh
  # the jar there whatever the machine's cache holds: the check that opens the session runs and fails
  : > "$oc/stand-in.jar"
  jar_of() { echo "$oc/stand-in.jar"; }
  eval "$(sed -n '/^# The session needs the scala-library jar/,/^  start "\$src" --split/p' tests/split-watch.sh | sed '$d'; echo '  :'; echo 'fi')"
  echo "counts: pass=$pass fail=$fail skipped=$skipped"
  jar_of() { echo "$oc/no-such.jar"; }
  eval "$(sed -n '/^# The session needs the scala-library jar/,/^  start "\$src" --split/p' tests/split-watch.sh | sed '$d'; echo '  :'; echo 'fi')"
  echo "counts: pass=$pass fail=$fail skipped=$skipped"
) > "$oc/watch.log" 2>&1
check "outcomes: a watch session's opening check that fails is a failure; without the jar it is not run" "$oc/watch.log" eval '
  grep -q "^FAIL classpath: the check over scala-library that opens the session failed: " "$oc/watch.log" &&
  grep -q -x "skip the classpath session: not in the coursier cache: scala-library" "$oc/watch.log" &&
  [ "$(grep "^counts: " "$oc/watch.log" | tr "\n" " ")" = "counts: pass=0 fail=1 skipped=0 counts: pass=0 fail=1 skipped=1 " ] &&
  ! grep -q "^started" "$oc/watch.log"'
# the JVM runner's own checks without javap, and its determinism check without its jars: a skip line each, not run
(
  out=$oc/jvm; mkdir -p "$out"
  source <(sed -n '/^# aux_not_run <check> <why>/,/^}/p; /^same_jars() {/,/^}/p; /^forwarder_order() {/,/^}/p' tests/run_jvm.sh)
  PATH=$oc/empty-path forwarder_order; echo "forwarder_order exit $?"
  COURSIER_CACHE=$oc/empty-cache same_jars; echo "same_jars exit $?"
  echo "not run: $aux_not_run"
) > "$oc/jvm.log" 2>&1
check "outcomes: the JVM runner's checks without javap or their jars print a skip line each and are not run" "$oc/jvm.log" eval '
  grep -q -x "skip forwarder_order: no javap" "$oc/jvm.log" && grep -q -x "skip the corpus.s determinism: not in the coursier cache: scala-library" "$oc/jvm.log" &&
  grep -q -x "not run: 2" "$oc/jvm.log" && [ -z "$(lacking "$oc/jvm.log" | grep -v "^skip")" ] && [ -n "$(lacking "$oc/jvm.log")" ]'

# The fix round's five: a check that ran nothing passes nowhere.
# an empty selection fails, the dotty run suites' and the board's
mkdir -p "$oc/s3/tests/run" && printf 'object Test { def main(a: Array[String]): Unit = println(1) }\n' > "$oc/s3/tests/run/one.scala" && printf '1\n' > "$oc/s3/tests/run/one.check"
SCALA3=$oc/s3 timeout 120 python3 -B tests/scala3/run.py --only '^no_such_test$' --out "$oc/s3out" > "$oc/s3-empty.log" 2>&1
code=$?
check "outcomes: the dotty run suite's empty selection fails, never 0 pass" "$oc/s3-empty.log" eval '[ $code = 2 ] && grep -q "^no test of the selection: nothing ran$" "$oc/s3-empty.log"'
mkdir -p "$oc/corpus/tests/pos" "$oc/corpus/compiler/test/dotc" && printf 'object P\n' > "$oc/corpus/tests/pos/p.scala"
: > "$oc/corpus/compiler/test/dotc/pos-linting.allowlist"
: > "$oc/corpus/compiler/test/dotc/scala2-library-tasty.excludelist"
timeout 60 git init -q "$oc/corpus" && printf '%s\n' "$(python3 -B -c 'import sys; sys.path.insert(0, "tests/board"); import dotty; print(dotty.TAG_COMMIT)')" > "$oc/corpus/.git/HEAD"
SCALA3_TAG=$oc/corpus BOARD_OUT=$oc/board-out timeout 200 python3 -B tests/board/dotty.py --row pos --only '^no_such_test$' > "$oc/dotty-empty.log" 2>&1
code=$?
check "outcomes: the board's empty selection fails, never 0 tests" "$oc/dotty-empty.log" eval '[ $code = 2 ] && grep -q "^board: no test of the rows pos matches --only .*: nothing ran$" "$oc/dotty-empty.log"'
# an empty results file takes no entry out of a fails-only list
printf 'never_executed\told failure\n' > "$oc/r/tests/pending/e.txt"
: > "$oc/empty.txt"
pend e empty.txt --fails-only --passed=0 --update
code=$?
check "outcomes: an empty results file takes no entry out of a fails-only list, the run fails on it" "$oc/pending.log" eval '[ $code = 1 ] &&
  [ "$(cat "$oc/r/tests/pending/e.txt")" = "$(printf "never_executed\told failure")" ] && grep -q "named neither failing, skipped nor passing (kept" "$oc/pending.log"'
printf 'pass never_executed\n' > "$oc/named.txt"
pend e named.txt --fails-only --passed=1 --update
code=$?
check "outcomes: a fails-only entry leaves its list by a line naming it passing" "$oc/pending.log" eval '[ $code = 0 ] && [ ! -s "$oc/r/tests/pending/e.txt" ]'
# the stdlib suite without the jar: every case cannot run on the board under --update, no earlier pass kept
st=$oc/st; mkdir -p "$st/tests/support" "$st/tests/cases" "$st/tests/pending" "$st/tests/board/suites"
cp tests/run_stdlib.sh "$st/tests/" && cp tests/support/jars.sh tests/support/pending.py "$st/tests/support/"
printf '@main def m() = println(1)\n' > "$st/tests/cases/one.scala"; printf '1\n' > "$st/tests/cases/one.expected"; : > "$st/tests/stdlib-summaries.txt"
printf '# test\tverdict\tthe other ways\tthe first line of the difference\none\tpass\t-\t\n' > "$st/tests/board/suites/stdlib.tsv"
(cd "$st" && COURSIER_CACHE=$oc/empty-cache timeout 120 bash tests/run_stdlib.sh --update) > "$oc/stdlib.log" 2>&1
code=$?
check "outcomes: the stdlib suite without its jar writes every case cannot run, no earlier pass kept" "$oc/stdlib.log" eval '[ $code = 0 ] &&
  grep -q "^stdlib: skipped, no scala-library 3.x jar in the coursier cache$" "$oc/stdlib.log" &&
  grep -q -x "$(printf "one\tcannot run\t-\tno scala-library 3.x jar in the coursier cache")" "$st/tests/board/suites/stdlib.tsv"'
# identity: a stand-in compiler writing what it is asked (a jar a zip), STANDIN_KILL=<program> a run that dies of a
# signal, STANDIN_FAIL=<programs, or all> a build that fails alike on both sides; the fixture's lists pass base (the
# canary) and hello
mkdir -p "$oc/standin-teq" && cat > "$oc/standin-teq/teq" << 'EOF'
#!/bin/bash
[ "$1" = compiler ] && shift
[ "$1" = --help ] && exit 0
verb=$1 src=$2 out=
shift 2
while [ $# -gt 0 ]; do [ "$1" = -o ] && out=$2; shift; done
name=$(basename "$src" .scala)
[ "$name" != "${STANDIN_KILL:-}" ] || kill -9 $$
case " ${STANDIN_FAIL:-} " in *" $name "* | *" all "*) echo "error: cannot open scala-library.jar"; exit 1 ;; esac
case $out in
  *.jar) python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1], "w").writestr("A.class", sys.argv[2])' "$out" "$name" ;;
  ?*) echo "// $name" > "$out" ;;
esac
echo "$verb $src"
EOF
chmod +x "$oc/standin-teq/teq"
standin=$oc/standin-teq/teq
id=$oc/id; mkdir -p "$id/tests/support" "$id/tests/cases" "$id/tests/board/suites"
cp tests/support/identity.sh tests/support/identity-across.sh tests/support/jars.sh tests/support/compiler-words.sh "$id/tests/support/" 2> /dev/null
for p in base hello; do printf 'object Main { def main(args: Array[String]): Unit = println("%s") }\n' "$p" > "$id/tests/cases/$p.scala"; done
lists() { printf '# test\tverdict\tthe other ways\tthe first line of the difference\nbase\tpass\t-\t\nhello\t%s\t-\t\n' "$1" | tee "$id/tests/board/suites/cases.tsv" > "$id/tests/board/suites/jvm.tsv"; }
lists pass
# a launch that did not run on both sides is no identical program, its differs line its one result, a jar's build too
STANDIN_KILL=hello timeout 200 bash "$id/tests/support/identity.sh" "$standin" "$standin" "$oc/id-out" js,jvm > "$oc/identity.log" 2>&1
code=$?
check "outcomes: identity counts no program identical whose runs did not run on both sides, a jar's build too" "$oc/identity.log" eval '[ $code = 1 ] &&
  grep -q "^differs: js hello (did not run: ref:137 new:137)$" "$oc/identity.log" &&
  grep -q "^differs: jvm hello (did not run: ref:137 new:137)$" "$oc/identity.log" &&
  grep -q "^identity js: 1 of 2 programs identical (1 built on both sides, 0 failed alike of 0 the lists allow), 1 differing, 0 not run$" "$oc/identity.log" &&
  grep -q "^identity jvm: 1 of 2 programs identical (1 built on both sides, 0 failed alike of 0 the lists allow), 1 differing, 0 not run$" "$oc/identity.log" &&
  grep -q "^identity self-test: [0-9]* of [0-9]* planted bad records failed$" "$oc/identity.log" &&
  ! grep -q "^FAIL" "$oc/identity.log"'
# a run whose every build fails alike is void: its canary does not build with the reference
STANDIN_FAIL=all timeout 200 bash "$id/tests/support/identity.sh" "$standin" "$standin" "$oc/id-out" js,jvm > "$oc/identity-void.log" 2>&1
code=$?
check "outcomes: identity of every build failing alike is void, its kinds' programs not run" "$oc/identity-void.log" eval '[ $code = 1 ] &&
  grep -q -x "VOID identity js: the canary base does not build with the reference binary: error: cannot open scala-library.jar" "$oc/identity-void.log" &&
  grep -q -x "VOID identity jvm: the canary base does not build with the reference binary: error: cannot open scala-library.jar" "$oc/identity-void.log" &&
  ! grep -q -E "^(identity (js|jvm):|differs)" "$oc/identity-void.log" && [ ! -e "$oc/id-out/js.results" ]'
# a program the lists pass failed alike fails the run, named; one they fail is counted
STANDIN_FAIL=hello timeout 200 bash "$id/tests/support/identity.sh" "$standin" "$standin" "$oc/id-out" js > "$oc/identity-beyond.log" 2>&1
code=$?
check "outcomes: identity fails on a program failed alike that the lists say builds, naming it" "$oc/identity-beyond.log" eval '[ $code = 1 ] &&
  grep -q -x "identity js: 2 of 2 programs identical (1 built on both sides, 1 failed alike of 0 the lists allow), 0 differing, 0 not run" "$oc/identity-beyond.log" &&
  grep -q -x "FAIL identity js: 1 programs failed alike that the lists say build: hello" "$oc/identity-beyond.log"'
lists fail
STANDIN_FAIL=hello timeout 200 bash "$id/tests/support/identity.sh" "$standin" "$standin" "$oc/id-out" js > "$oc/identity-allowed.log" 2>&1
code=$?
check "outcomes: identity passes a program failed alike that the lists fail, counting it" "$oc/identity-allowed.log" eval '[ $code = 0 ] &&
  grep -q -x "identity js: 2 of 2 programs identical (1 built on both sides, 1 failed alike of 1 the lists allow), 0 differing, 0 not run" "$oc/identity-allowed.log" &&
  ! grep -q "^FAIL" "$oc/identity-allowed.log"'
lists pass
# identity: a program not run for want of its jars fails the run, unless IDENTITY_SKIPS lists it; a FAIL under TEQ_GATE
printf '// jars: nosuch\nobject Lonely\n' > "$id/tests/cases/lonely.scala"
timeout 200 bash "$id/tests/support/identity.sh" "$standin" "$standin" "$oc/id-out" js > "$oc/identity-jars.log" 2>&1
code=$?
check "outcomes: identity fails on a program not run for want of its jars, naming it" "$oc/identity-jars.log" eval '[ $code = 1 ] &&
  grep -q "^not run js lonely: jars missing: nosuch$" "$oc/identity-jars.log" &&
  grep -q "^identity js: 2 of 3 programs identical (2 built on both sides, 0 failed alike of 1 the lists allow), 0 differing, 1 not run (jars missing)$" "$oc/identity-jars.log" &&
  ! grep -q "^FAIL" "$oc/identity-jars.log"'
printf 'js lonely\n' > "$oc/identity-skips"
IDENTITY_SKIPS=$oc/identity-skips timeout 200 bash "$id/tests/support/identity.sh" "$standin" "$standin" "$oc/id-out" js > "$oc/identity-skips.log" 2>&1
code=$?
check "outcomes: identity passes a program not run that IDENTITY_SKIPS lists" "$oc/identity-skips.log" eval '[ $code = 0 ] &&
  grep -q "^not run js lonely: jars missing: nosuch (listed in IDENTITY_SKIPS)$" "$oc/identity-skips.log"'
TEQ_GATE=1 timeout 200 bash "$id/tests/support/identity.sh" "$standin" "$standin" "$oc/id-out" js > "$oc/identity-gate.log" 2>&1
code=$?
check "outcomes: identity under TEQ_GATE fails a program not run with a FAIL line" "$oc/identity-gate.log" eval '[ $code = 1 ] &&
  grep -q "^FAIL identity js: 1 programs not run for want of their jars, a gate.s want of what it needs$" "$oc/identity-gate.log"'
# workers: a check whose command did not run fails it, never an absent pattern's pass
mkdir -p "$oc/killed-teq" && printf '#!/bin/bash\n[ "$1" = --help ] && exit 0\nkill -9 $$\n' > "$oc/killed-teq/teq" && chmod +x "$oc/killed-teq/teq"
(
  pass=0; fail=0
  said_fail() { echo "FAIL $1"; }
  source <(sed -n '/^bad() {/,/^}/p; /^# not_ran <code>/,/^not_ran/p; /^said_why=$/p; /^saw() {/,/^}/p' tests/workers.sh)
  TEQ=$oc/killed-teq/teq
  source <(sed -n '/^for p in tests\/cases\/charsequence_release.scala/,/^done/p' tests/workers.sh)
  echo "counts: pass=$pass fail=$fail"
) > "$oc/workers.log" 2>&1
check "outcomes: a workers check whose command did not run fails, with its reason" "$oc/workers.log" eval '
  grep -q -x "counts: pass=0 fail=6" "$oc/workers.log" && [ "$(grep -c "(its command did not run: exit 137)$" "$oc/workers.log")" = 6 ]'

fi
if piece_has tools; then
piece_ran tools
# The gate's cs and scala-cli (bench/actions/tools.sh), never coursier/setup-action, which resolved its apps channel on
# Maven Central in every job: copies of the script whose digests are stand-in archives' (both, or cs's alone), against a
# curl answering 429 for its first tries (or hanging on its first) and then serving the archive its URL names, uname's
# stand-in an x86-64 Linux (or another system).
check "tools: the workflow without coursier/setup-action" /dev/null eval '! grep -q coursier/setup-action .github/workflows/gate.yml'
check "tools: three tries a download, 10 s apart, each bounded at 120 s, within the workflow's 900 s" /dev/null \
  eval 'grep -qx "attempts=3" bench/actions/tools.sh && grep -qxF "delay=\${TOOLS_RETRY_DELAY:-10}" bench/actions/tools.sh &&
    grep -qxF "try=\${TOOLS_TRY_SECONDS:-120}" bench/actions/tools.sh && [ "$(grep -c "run: timeout 900 bench/actions/tools.sh" .github/workflows/gate.yml)" = 2 ]'
t=$work/tools
mkdir -p "$t/bin" "$t/other" "$t/files"
printf '#!/bin/sh\necho cs stand-in\n' | gzip > "$t/files/cs.gz"
printf '#!/bin/sh\necho scala-cli stand-in\n' | gzip > "$t/files/scala-cli.gz"
digest() { python3 -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "$1"; }
cs_stand_in="s/^cs_sha256=.*/cs_sha256=$(digest "$t/files/cs.gz")/"
sed -e "$cs_stand_in" -e "s/^scala_cli_sha256=.*/scala_cli_sha256=$(digest "$t/files/scala-cli.gz")/" bench/actions/tools.sh > "$t/tools.sh"
sed -e "$cs_stand_in" bench/actions/tools.sh > "$t/cs-only.sh"
cp bench/actions/tools.sh "$t/pinned.sh"
check "tools: the copies differ from the script by their stand-in digests alone (two, one)" /dev/null \
  eval '[ "$(diff bench/actions/tools.sh "$t/tools.sh" | grep -c "^> ") $(diff bench/actions/tools.sh "$t/cs-only.sh" | grep -c "^> ")" = "2 1" ]'
printf '#!/bin/sh\ncase "$1" in -m) echo x86_64 ;; -s) echo Linux ;; *) exec %s "$@" ;; esac\n' "$(command -v uname)" > "$t/bin/uname"
printf '#!/bin/sh\ncase "$1" in -m) echo arm64 ;; -s) echo Darwin ;; *) exec %s "$@" ;; esac\n' "$(command -v uname)" > "$t/other/uname"
cat > "$t/bin/curl" << 'EOF'
#!/bin/bash
out= url=
while [ $# -gt 0 ]; do case $1 in -*o) out=$2; shift 2 ;; -*) shift ;; *) url=$1; shift ;; esac; done
echo "$url" >> "$TOOLS_T/urls"
python3 -c 'import time; print(time.time())' >> "$TOOLS_T/times"
if [ -e "$TOOLS_T/hang" ]; then rm -f "$TOOLS_T/hang"; exec sleep 300; fi
n=$(cat "$TOOLS_T/fails")
if [ "$n" -gt 0 ]; then echo $((n - 1)) > "$TOOLS_T/fails"; echo "curl: (22) The requested URL returned error: 429" >&2; exit 22; fi
case $url in
  */cs-x86_64-pc-linux.gz) cp "$TOOLS_T/files/cs.gz" "$out" ;;
  */scala-cli-x86_64-pc-linux.gz) cp "$TOOLS_T/files/scala-cli.gz" "$out" ;;
  *) exit 22 ;;
esac
EOF
cp "$t/bin/curl" "$t/other/curl"
chmod +x "$t/bin/uname" "$t/bin/curl" "$t/other/uname" "$t/other/curl" "$t/tools.sh" "$t/cs-only.sh" "$t/pinned.sh"
# tools_run <script> <429s before a serve> <dir>: the script's exit in $code, its stderr in $t/err, the URLs asked in
# $t/urls and the times of the asks in $t/times, the lines added to GITHUB_PATH in $t/path; the delay between tries
# $run_delay (0), each try's bound $run_try (30), the stand-ins of $run_bin ($t/bin).
tools_run() {
  echo "$2" > "$t/fails"
  : > "$t/urls"
  : > "$t/times"
  : > "$t/path"
  PATH="${run_bin:-$t/bin}:$PATH" TOOLS_T=$t TOOLS_RETRY_DELAY=${run_delay:-0} TOOLS_TRY_SECONDS=${run_try:-30} GITHUB_PATH=$t/path \
    timeout 60 "$1" "$3" > /dev/null 2> "$t/err"
  code=$?
}
pinned_urls="https://github.com/VirtusLab/scala-cli/releases/download/v$(sed -n 's/^scala_cli_version=//p' bench/actions/tools.sh)/scala-cli-x86_64-pc-linux.gz https://github.com/coursier/coursier/releases/download/v$(sed -n 's/^cs_version=//p' bench/actions/tools.sh)/cs-x86_64-pc-linux.gz "
tools_run "$t/tools.sh" 2 "$t/d1"
check "tools: two 429s, then both served and unpacked, runnable" "$t/err" \
  eval '[ $code = 0 ] && [ "$("$t/d1/cs") / $("$t/d1/scala-cli")" = "cs stand-in / scala-cli stand-in" ]'
check "tools: the pinned releases asked, four times (the two refused asked again)" "$t/urls" \
  eval '[ "$(sort -u "$t/urls" | tr "\n" " ")" = "$pinned_urls" ] && [ "$(wc -l < "$t/urls" | tr -d " ")" = 4 ]'
check "tools: the directory added to GITHUB_PATH" "$t/path" eval '[ "$(cat "$t/path")" = "$(cd "$t/d1" && pwd)" ]'
tools_run "$t/tools.sh" 3 "$t/d2"
check "tools: three 429s, the bound: exit 1, said, nothing installed nor added to GITHUB_PATH" "$t/err" \
  eval '[ $code = 1 ] && grep -q "cs-x86_64-pc-linux.gz could not be fetched in 3 attempts" "$t/err" && [ -z "$(ls "$t/d2")" ] && [ ! -s "$t/path" ]'
tools_run "$t/pinned.sh" 0 "$t/d3"
check "tools: cs of another digest refused, said, nothing installed nor added" "$t/err" \
  eval '[ $code = 1 ] && grep -qF "cs-x86_64-pc-linux.gz'"'"'s digest is not the pinned $(sed -n "s/^cs_sha256=//p" bench/actions/tools.sh)" "$t/err" && [ -z "$(ls "$t/d3")" ] && [ ! -s "$t/path" ]'
tools_run "$t/cs-only.sh" 0 "$t/d4"
check "tools: cs served, scala-cli of another digest refused, said, nothing of it installed nor added" "$t/err" \
  eval '[ $code = 1 ] && grep -qF "scala-cli-x86_64-pc-linux.gz'"'"'s digest is not the pinned $(sed -n "s/^scala_cli_sha256=//p" bench/actions/tools.sh)" "$t/err" && [ "$(ls "$t/d4")" = cs ] && [ ! -s "$t/path" ]'
touch "$t/hang"
run_try=2 tools_run "$t/tools.sh" 0 "$t/d5"
check "tools: a try that hangs is given up at its bound and tried again" "$t/err" \
  eval '[ $code = 0 ] && grep -q "cs-x86_64-pc-linux.gz not fetched (attempt 1 of 3)" "$t/err" && [ "$(wc -l < "$t/urls" | tr -d " ")" = 3 ] && [ -x "$t/d5/scala-cli" ]'
run_delay=1 tools_run "$t/tools.sh" 2 "$t/d6"
check "tools: the tries a delay apart" "$t/times" \
  eval '[ $code = 0 ] && python3 -c "import sys; t = [float(l) for l in open(sys.argv[1])]; sys.exit(0 if len(t) == 4 and t[1] - t[0] >= 0.9 and t[2] - t[1] >= 0.9 else 1)" "$t/times"'
run_bin=$t/other tools_run "$t/tools.sh" 0 "$t/d7"
check "tools: another system refused, said, nothing asked, installed nor added" "$t/err" \
  eval '[ $code = 1 ] && grep -q "for x86-64 Linux alone, not Darwin arm64" "$t/err" && [ ! -s "$t/urls" ] && [ ! -e "$t/d7" ] && [ ! -s "$t/path" ]'
mkdir -p "$t/d8/scala-cli"
tools_run "$t/tools.sh" 0 "$t/d8"
check "tools: a directory where a tool goes refused, said, nothing asked, installed nor added" "$t/err" \
  eval '[ $code = 1 ] && grep -q "d8/scala-cli is a directory, where the scala-cli binary goes" "$t/err" && [ ! -s "$t/urls" ] && [ "$(ls "$t/d8")" = scala-cli ] && [ -z "$(ls "$t/d8/scala-cli")" ] && [ ! -s "$t/path" ]'

fi
if piece_has tail; then
piece_ran tail
# The first words' readers are master's code, fed the head's output rendered back whole (rewording.render_stream): the
# rendered stream is master's byte for byte, and each reader decides on it as on master's (run.py's bucket and its
# source line, the typing harness's category, the parser's normalised lines, the app oracle's record); a backticked
# term named `type Widget` reads as the missing type (a row, pinned). tests/run_errors.sh holds a placed expectation
# by a diagnostic's header alone.
wd=$work/words
mkdir -p "$wd"
cat > "$wd/readers.py" <<'PY'
import importlib.util, json, sys
sys.argv = ['run.py']
for d in ('tests/scala3-typing', 'tests/scala3', 'tests/support'):
    sys.path.insert(0, d)
import run as R, harness as H, rewording as W
def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m
P, A = load('parser_run', 'tests/parser/run.py'), load('app_diagnostics', 'bench/app/diagnostics.py')
def out(msg, src, col):
    return f'p.scala:1:{col}: error: {msg}\n  {src}\n  {" " * (col - 1)}^\n1 error found\n'
def decisions(o):
    first = next(m for m in (H.DIAG.match(l) for l in o.splitlines()) if m)
    return (R.bucket_of(*R.first_error(o)), R.first_error(o)[1], H.normalise(first.group(5)),
            sorted(P.normalise(m['path'], 1, 1, m['severity'], m['message']) for m in map(P.DIAG.match, o.splitlines()) if m))
def app(msg):
    r = A.record('teq', 'files', '.', '.', '/nowhere', {'file': 'p.scala', 'line': 1, 'col': 1, 'severity': 'error', 'message': msg})
    return r['category'], r['payload'], r['message']
cases = (('a missing type of two words', 'type Two Words not found', 'Not found: type Two Words', 'val b: `Two Words` = ???', 8),
         ('a backticked term', 'not found: two words', 'Not found: two words', 'val a = `two words`', 9),
         ('a mismatch whose literal holds the delimiter', 'type mismatch: found "a, required B", required Int', 'Found:    "a, required B"\nRequired: Int', 'val x: Int = "a, required B"', 14),
         ('a mismatch over a null source line', 'type mismatch: found "oops", required Int', 'Found:    "oops"\nRequired: Int', 'val x: Int = { val a = null; "oops" }', 30),
         ('a message of several lines, the same words', 'None of the overloaded alternatives of method area with types\n (r: Int): Int\n (w: Int, h: Int): Int\nmatch arguments ()', 'None of the overloaded alternatives of method area with types\n (r: Int): Int\n (w: Int, h: Int): Int\nmatch arguments ()', 'area()', 3))
for name, old, new, src, col in cases:
    o, n = out(old, src, col), out(new, src, col)
    rendered = W.render_stream(n)
    ok = rendered == o and decisions(rendered) == decisions(o) and app(new) == app(old)
    print(('ok ' if ok else 'FAIL ') + name)
o, n = out('not found: type Widget', 'val x = `type Widget`', 9), out('Not found: type Widget', 'val x = `type Widget`', 9)
d_old, d_new = decisions(o), decisions(W.render_stream(n))
print(('ok ' if (d_old[0], d_new[0], d_old[2], d_new[2]) == (('gap', 'standard library'), ('gap', 'typing'), 'not found: {}', 'type {} not found') else 'FAIL ')
      + 'a term named `type Widget`, read as the missing type (a row, pinned)')
# A row, pinned: two context bounds of one type and place (`def m[A: Need, B: Need]`, `m[Int, Int]`): master named the
# second `evidence$2`, which the new line does not hold; a reader renders both `evidence$1`. run.py and the harness
# read the first error alike, the parser's lines differ.
two = lambda a, b: f'p.scala:3:20: error: {a}\n  val x = m[Int, Int]\n                   ^\np.scala:3:20: error: {b}\n  val x = m[Int, Int]\n                   ^\n2 errors found\n'
o = two('no given instance of type Need[Int] was found for parameter evidence$1', 'no given instance of type Need[Int] was found for parameter evidence$2')
nw = two('No given instance of type Need[Int] was found for a context parameter of method m', 'No given instance of type Need[Int] was found for a context parameter of method m')
d_old, d_new = decisions(o), decisions(W.render_stream(nw))
print(('ok ' if (d_old[0], d_old[2]) == (d_new[0], d_new[2]) and d_old[3] != d_new[3] else 'FAIL ')
      + 'two context bounds of one type and place: the first error read alike, the second\'s evidence$2 a row (pinned)')
PY
timeout 60 python3 -B "$wd/readers.py" > "$wd/readers.log" 2>&1
check "words: master's readers on the head's output rendered back decide as on master's, a term named \`type Widget\` the pinned row" "$wd/readers.log" eval '
  [ "$(grep -c "^ok " "$wd/readers.log")" = 7 ] && ! grep -q -v "^ok " "$wd/readers.log"'
held_of() {
  local expected=$1 out=$2 held
  eval "$(sed -n '/^    if \[\[ \$expected =~ /,/^    fi$/p' tests/run_errors.sh)"
  echo "${held:-none}"
}
out='tests/errors/p.scala:1:14: error: Found:    "1:14: error: type mismatch"
Required: Int
  val x: Int = "1:14: error: type mismatch"
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^'
check "words: a placed expectation is held by a diagnostic's header, never by its text inside a quoted value" /dev/null eval '
  [ "$(held_of "1:14: error: type mismatch" "$out")" = none ] && [ "$(held_of "1:14: error: Found:    \"1:14" "$out")" = held ] &&
  [ "$(held_of "p.scala:1:14: error: Found:" "$out")" = held ] && [ "$(held_of "Required: Int" "$out")" = held ]'

# The workflow's jobs as tests/support/workflow-jobs.py reads them, failing closed: the gate's matrix with every
# row's keys reversed reads the same suites by job, and a matrix of an axis and an `exclude` is refused.
check "workflow-jobs: the gate's matrix read in any key order, a form the reader does not handle refused" "$work/workflow-jobs.log" \
  eval 'python3 tests/support/workflow-jobs.py --self-test > "$work/workflow-jobs.log" 2>&1'
# The judge of tests/workers.sh's refused reads (tests/support/refused_reads.py) on planted records: the property at
# every read a build reaches, a read no build reaches said and not failed; red for a build not one worker's, a refused
# read not typed again, a count past the read with no refusal, five builds without a count, nothing reached.
check "refused reads: the judge's planted records judged as they must be" "$work/refused-reads.log" \
  eval 'python3 tests/support/refused_reads.py --self-test > "$work/refused-reads.log" 2>&1'
# Every line the gate deals to the shards has its row in tests/gate-times.txt (tests/support/gate_rows.py): a line
# without one is dealt as 120 s, whatever it takes. Its check on planted lists first, then the tree's own.
check "gate's rows: every line dealt to the shards has its row in tests/gate-times.txt" "$work/gate-rows.log" \
  eval 'python3 tests/support/gate_rows.py --self-test > "$work/gate-rows.log" 2>&1 && python3 tests/support/gate_rows.py tests/gate.sh tests/gate-times.txt >> "$work/gate-rows.log" 2>&1'
# A comparison job left running by an aborted gate, on the machine a later gate's job starts on (tests/gate.sh's
# earlier_job): ended with its children, a process its pid file names whose command is not a job's left alone (a pid
# the system gave another process since); and a job's end (tests/gate-compare.sh's finish) removes the pid file while
# it is its own, and leaves a later job's.
ej=$work/earlier-job
mkdir -p "$ej/out/compare"
eval "$(sed -n '/^earlier_job=/p' tests/gate.sh)"
(exec -a "bash out/compare/gate-compare.sh" sleep 300) &
fake=$!
sleep 1
echo $fake > "$ej/out/compare/pid"
ej_said=$(cd "$ej" && bash -c "$earlier_job")
sleep 1
if kill -0 $fake 2> /dev/null; then ej_fake=alive; kill $fake; else ej_fake=ended; fi
sleep 300 &
other=$!
echo $other > "$ej/out/compare/pid"
ej_other_said=$(cd "$ej" && bash -c "$earlier_job")
if kill -0 $other 2> /dev/null; then ej_other=alive; else ej_other=ended; fi
kill $other 2> /dev/null
eval "$(sed -n '/^finish() {/p' tests/gate-compare.sh)"
echo 1 > "$ej/out/compare/pid"
(out=$ej/out/compare status=$ej/out/compare/status; finish 0) > /dev/null 2>&1
ej_later=$([ -s "$ej/out/compare/pid" ] && echo kept || echo removed)
(out=$ej/out/compare status=$ej/out/compare/status; echo $$ > "$out/pid"; finish 0) > /dev/null 2>&1
ej_own=$([ -e "$ej/out/compare/pid" ] && echo kept || echo removed)
echo "fake $ej_fake ($ej_said); unrelated $ej_other ($ej_other_said); a later job's pid file $ej_later; its own $ej_own" > "$work/earlier-job.log"
check "comparison: an aborted gate's job ended at a later gate's start, another process left, a job's end removing its own pid file alone" "$work/earlier-job.log" \
  eval '[ $ej_fake = ended ] && [ "$ej_said" = "an earlier gate'"'"'s job ended: pid $fake" ] && [ $ej_other = alive ] && [ -z "$ej_other_said" ] &&
    [ $ej_later = kept ] && [ $ej_own = removed ]'

# The table of inline methods called through an export from a third module (tests/support/export_inline.py, run
# by tests/support/export-inline.sh): its templates and its judge over planted tables.
check "export-inline: the table's templates and its judge's planted tables judged as they must be" "$work/export-inline.log" \
  eval 'python3 tests/support/export_inline.py --self-test > "$work/export-inline.log" 2>&1'

# The table of an omitted default argument called through an export (tests/support/export_default.py, run by
# tests/support/export-inline.sh): its templates, the cells its change turns, and its judge over planted tables.
check "export-default: the table's templates, its part and its judge's planted tables judged as they must be" "$work/export-default.log" \
  eval 'python3 tests/support/export_default.py --self-test > "$work/export-default.log" 2>&1'

# The table of an inline definition's ordinary default (tests/support/inline_getters.py, run by
# tests/support/inline-getters.sh): its templates and its judge over planted tables.
check "inline-getters: the table's templates and its judge's planted tables judged as they must be" "$work/inline-getters.log" \
  eval 'python3 tests/support/inline_getters.py --self-test > "$work/inline-getters.log" 2>&1'

# The table of a local given that takes parameters (tests/support/local_given_params.py, run by
# tests/support/local-given-params.sh): its templates and its judge over planted tables.
check "local-given-params: the table's templates and its judge's planted tables judged as they must be" "$work/local-given-params.log" \
  eval 'python3 tests/support/local_given_params.py --self-test > "$work/local-given-params.log" 2>&1'

# The table of an export in a given instance's body (tests/support/export_given_body.py, run by
# tests/support/local-given-params.sh): its templates.
check "export-given-body: the table's templates" "$work/export-given-body.log" \
  eval 'python3 tests/support/export_given_body.py --self-test > "$work/export-given-body.log" 2>&1'

fi
echo "gate-scripts: $pass passed, $fail failed"
pieces_end $((fail > 0)) passed=$pass failed=$fail
[ $fail = 0 ]
