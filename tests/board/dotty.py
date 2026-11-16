#!/usr/bin/env python3
"""dotty's own test suites at the tag 3.8.4 through teq, each test run as dotty's runner runs it.

  SCALA3_TAG=/path/to/scala3-at-3.8.4 TEQ=/path/to/teq python3 tests/board/dotty.py [options]

  --row NAME[,NAME]   only these rows (default: every row of rows.py)
  --except NAME[,NAME] every row but these (the gate's line board-rest: a row is one line's or the rest's)
  --only REGEX        only the tests whose name matches
  --jobs N            tests at a time (default 8)
  --update            take the tests that pass now out of tests/pending/dotty-<row>.txt and write the row's list,
                      tests/board/dotty/<row>.tsv, from this run
  --add               also write the failures the pending list lacks into it (the part that brings tests in)
  --show TEST         run one test of the rows named and print what teq and the program printed
  --rows              print the rows and where dotty's sources name them

The rows are rows.py's: one per directory dotty's test sources compile or run at the tag (a fixed list, drawn
from those sources), each with dotty's flags and its kind. A test is vulpix's unit
(compiler/test/dotty/tools/vulpix/ParallelTesting.scala, compileFilesInDir): a source file directly in the
directory, or a subdirectory holding a source directly, its files grouped by their `_N` suffix and the groups
compiled in order, each against the output of those before it.

What passing means, by kind (ParallelTesting.scala's PosTest, NegTest, WarnTest, RunTest; for patmat the tag's
compiler/test/dotty/tools/dotc/transform/PatmatExhaustivityTest.scala):

  pos      teq writes the class files of every group (`teq compiler build --target jvm --all-mains`)
  neg      `teq compiler check` reports as many errors as the files carry `// error` marks, each on a marked line
           (`// nopos-error` for one without a place, `// anypos-error` for one at any place), and its diagnostics
           are scalac's own, which the row's oracle holds; marks that hold where the oracle gives no answer are no
           pass (`marks`, below)
  warn     no error, the warnings on the `// warn` lines, and scalac's diagnostics as for neg; a test without a
           mark passes on no diagnostic at all, under options whose warnings teq gives
  run      pos, then `java Test` with dotty's own arguments ends without an exception, its output the .check's
           lines where there is one; the row run-scalajs builds the program for JavaScript against scala-library
           and runs it on node, `<name>.scala-js.check` before `<name>.check`
  patmat   the diagnostics, written as the tag's simplified reporter writes them (`<line>: <kind>`, the uncovered
           cases after an exhaustivity warning), are the .check's lines; no .check, no diagnostic
  nocrash  any outcome but a crash or a limit

scalac's diagnostics of a neg or warn test are the row's oracle, tests/board/dotty/<row>.oracle.jsonl: what scalac
3.8.4 itself reported for the test, compiled as dotty's runner compiles it, a line a test, captured by
tests/board/oracle.py (its docstring is the recipe; the tests' .check files are that capture's validation, which a
run of these rows makes first, without a compiler, and ends on: `oracle.py validate`). A test
whose marks hold passes where teq's diagnostics are its entry's, in order: severity, file, line, column and the
whole message (teq prints no boxes; the entry has scalac's as the reader of its boxes reads them). Both sides are
read by one rule (`comparable`): an info is part of neither; nor is the error scalac adds under -Werror (`No
warnings can be incurred under -Werror`, which the runner records once a compilation where teq warned under
`--werror`, an error beside the warning or not, and the marks count); a summary of the warnings a flag left
unprinted (`there was 1 deprecation warning; re-run with ...`) is its count and its kind, each compiler naming
its own flag; any other diagnostic without a place is compared whole; an identity hash (`ExprImpl@1562d7e8`) and a colour sequence are no part of a message. scalac ran in the
corpus and names its files from there, so the corpus' own path is taken out of teq's messages. neg, warn, patmat
and nocrash are judged against scala-library (`--std scala-library`, the library dotty's tests are written
against).

Verdicts: pass, marks, fail, timeout (a limit of CPU seconds for teq, of wall-clock seconds for java and node),
crash (a panic, a signal, an exit status teq does not document, the memory cap), cannot run (with its reason
class: the test needs something no compiler run from the checkout gives). `marks` is a neg or warn test that
holds its marked lines where the oracle gives no answer to hold its diagnostics to: no entry for the test; an
entry whose capture did not complete (`valid`); one scalac does not repeat from run to run (`stable`); one made
from other sources than the test's (an entry carries a digest of the test's files); or one that teq's
diagnostics equal in all the reader of scalac's boxes reads, while a box of it is one that reader does not read
(of such a box the severity, the file and the line are compared, and equality in those is no pass: the reader
cannot decide the test, which may differ from scalac's in that box's column or message). Of these a run meets
the capture that did not complete, the unrepeated answer and the unread box: an oracle without a test's entry,
with one of other sources or with one silent on its stability does not validate, and the run ends on it before a
test is judged; the judge's own answer for them is `marks` all the same (tests/gate-scripts.sh holds it). teq refuses
or warns where the test says, and nothing compares its diagnostics with scalac's, so it is no pass (an error of
another cause on the marked line would hold the marks too); it is no failure either: its line is in no pending
list, and a test that leaves it for `fail` is a moved verdict as any other. A test whose marks fail fails,
whatever the oracle has. A warn test without a single mark has an entry of no diagnostic (dotty's WarnTest fails
on an error and on a count of warnings other than the marks'), and passes where teq's are none, but under an
option whose warnings teq does not give (`warns` in OPTIONS), where its silence says nothing: that test fails as
one under a check teq does not perform. A test with a Java source fails: teq reads
none. An option of the test or of its row is given to teq where teq has the flag, dropped where it is scalac's
own machinery, and fails the test (`option: <flag>`) where it changes what is accepted, warned or run and teq
has no flag for it (OPTIONS below).

Beside the verdict a list keeps what the other ways of running the test give, counted in no row: for a pos
kind `check` (the typing line's `teq compiler check`), for a run kind `interp` (`teq interp`), for run-scalajs
`lean` (the JavaScript build against teq's own std).

A list is one line per test: test, verdict, the other ways, the first line of the difference. Two runs on one
commit give the same verdicts; the first line is information. Without --update the run is held to
tests/pending/dotty-<row>.txt (tests/support/pending.py): it fails when a listed test passes and when an
unlisted test fails. And to the row's committed list, tests/board/dotty/<row>.tsv, which the board's counts are
read from: it fails where a test's verdict there is not the run's, and --update writes the list again, so a
change that moves a verdict moves the board with it.

The rows are lines of the gate (tests/all.sh, tests/gate.sh): board-run, board-run-scalajs, board-pos and
board-rest, every other row. The gate's table keeps a line's last line, so a run ends with one that names what
was not as listed, row and test, or the counts of a run that was.

The corpus is $SCALA3_TAG, else scala3-3.8.4 beside the checkout $SCALA3 names (tests/all.sh's variable and the
gate's machines'). teq runs under tests/support/teq-capped.sh (a memory cap) where the user's systemd manager
lingers and starts such a scope, bare where not, which the run's first line then says.
"""
import argparse
import collections
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.dont_write_bytecode = True
sys.path.insert(0, os.path.join(REPO, "tests", "scala3-typing"))
sys.path.insert(0, os.path.join(REPO, "tests", "support"))
sys.path.insert(0, HERE)
from harness import SUMMARY_COUNT  # noqa: E402  (a summary of unprinted warnings, as the warn line reads it)
import pending  # noqa: E402
import rows as ROWS  # noqa: E402

TAG_COMMIT = "7d4833b619d31ea9acac97fccf1e74b42b89c49f"
SCALAC = "3.8.4"  # the scalac whose diagnostics the rows' oracles hold
CAPPED = os.path.join(REPO, "tests", "support", "teq-capped.sh")
LAST_LINE = 280  # the gate's table keeps 300 characters of a line's last line
SRC = (".scala", ".java")

# Limits. teq types on one thread, so its CPU seconds are its work, whatever else the machine runs; the wall
# clock stands behind for a process that waits. A java or node child has dotty's 45 s for a run, on the wall.
CPU_LIMIT = 30
WALL_LIMIT = 150
RUN_LIMIT = 45
OUTPUT_LIMIT = 64 << 20
NEAR = 3  # a test past a third of a limit is named after the run


def die(msg):
    sys.stdout.flush()  # the rows' lines before the cause: the gate keeps a log's last line
    print(msg, file=sys.stderr)
    sys.exit(2)


def main_checkout():
    try:
        common = subprocess.run(["git", "-C", HERE, "rev-parse", "--path-format=absolute", "--git-common-dir"],
                                capture_output=True, text=True, timeout=10).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        common = ""
    return os.path.dirname(common) if common else REPO


def corpus():
    """The checkout of dotty at the tag: $SCALA3_TAG; else scala3-3.8.4 beside the checkout at main that $SCALA3
    names (a gate machine's mirror of the tree is no git checkout with a main working tree to look beside); else
    teq-ref/scala3-3.8.4 beside the main working tree, where tests/all.sh looks for $SCALA3 too.
    Another commit, or a checkout without the tests, is an error: a list of nothing proves nothing."""
    root = os.environ.get("SCALA3_TAG")
    if not root and os.environ.get("SCALA3"):
        root = os.path.join(os.path.dirname(os.path.abspath(os.environ["SCALA3"])), "scala3-3.8.4")
    if not root:
        root = os.path.join(main_checkout(), "..", "teq-ref", "scala3-3.8.4")
    root = os.path.abspath(root)
    if not os.path.isdir(os.path.join(root, "tests")):
        die(f"no checkout of dotty at the tag 3.8.4 at {root} (set SCALA3_TAG; "
            f"git -C <clone> worktree add --detach <dir> 3.8.4)")
    try:
        head = subprocess.run(["git", "-C", root, "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        head = ""
    if head != TAG_COMMIT:
        die(f"{root} is at {head or 'no commit'}, not at the tag 3.8.4 ({TAG_COMMIT})")
    return root


def scala_library():
    cache = os.environ.get("COURSIER_CACHE")
    candidates = [cache] if cache else [os.path.expanduser("~/.cache/coursier/v1"), os.path.expanduser("~/Library/Caches/Coursier/v1")]
    for c in candidates:
        jar = os.path.join(c, "https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar")
        if os.path.exists(jar):
            return jar
    die("scala-library-3.8.4.jar is not in the coursier cache (set COURSIER_CACHE)")


def teq_command():
    """teq under the memory cap, the tree's tests/support/teq-capped.sh (a systemd scope of the test's own), where
    this machine starts such a scope and keeps it: the user's systemd manager must be one that outlives a login
    (`loginctl enable-linger`), since a manager that ends with the login ends its scopes with it, and a gate's
    shard runs on after the login that started it; then the scope is asked for once, with --version, the way
    every test starts it. Where either fails (no systemd-run, no manager, one that goes) teq runs bare, and the
    fourth value is the line that says so; no verdict of the lists comes from the cap.
    (command, its environment, the binary, that line or nothing)."""
    teq = os.environ.get("TEQ") or os.path.join(REPO, "target", "release", "teq")
    if os.path.basename(os.path.realpath(teq)) == os.path.basename(CAPPED):
        if not os.environ.get("TEQ_REAL"):
            die("TEQ is the capped wrapper: set TEQ_REAL to the binary")
        return teq, {}, os.environ["TEQ_REAL"], ""
    if not os.path.exists(teq):
        die(f"no teq binary at {teq}; build it with cargo build --release")
    teq = os.path.abspath(teq)
    env = {"TEQ_REAL": teq}
    try:
        linger = subprocess.run(["loginctl", "show-user", str(os.getuid()), "--value", "-p", "Linger"], capture_output=True,
                                text=True, timeout=20, stdin=subprocess.DEVNULL)
        why = "" if linger.returncode == 0 and linger.stdout.strip() == "yes" else "the user's systemd manager is not one that lingers"
        if not why:
            probe = subprocess.run([CAPPED, "--version"], capture_output=True, text=True, timeout=30,
                                   env={**os.environ, **env}, stdin=subprocess.DEVNULL)
            why = "" if probe.returncode == 0 else (probe.stderr.strip().splitlines() or [f"exit {probe.returncode}"])[-1]
    except (OSError, subprocess.SubprocessError) as e:
        why = str(e)
    if not why:
        return CAPPED, env, teq, ""
    return teq, {}, teq, f"board: teq runs without its memory cap on this machine ({' '.join(why.split())[:160]})"


def major(cmd, rx):
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=20)
        m = re.search(rx, out.stdout + out.stderr)
        return int(m.group(1)) if m else 0
    except (OSError, subprocess.SubprocessError):
        return 0


# ---- a test's options -------------------------------------------------------------------------------------

# scalac's options, by what teq does with each. `map`: teq has the flag. `neutral`: scalac's own machinery, or
# a setting teq has always on, dropped. `semantic`: it changes what is accepted, warned or run and teq has no
# flag: the test fails with `option: <flag>`. `warns`: it turns warnings on that teq does not give; it fails
# the test only together with -Werror, where the absence of those warnings is what the test holds (a warn
# test fails for the missing warning itself). An option in none of these is semantic.
# A language feature that puts a whole unit under a checker teq does not have (CHECKER_FEATURES; pureFunctions is
# syntax alone and is none) is semantic too, as the row's option, a file's, or an import of the source: such a
# test holds what the checker says of it.
# (name, class, takes a value as its next word)
OPTIONS = {
    "-source": ("map", True), "--source": ("map", True), "-language": ("map", True),
    "-Werror": ("map", False), "-Xfatal-warnings": ("map", False),
    "-deprecation": ("map", False), "--deprecation": ("map", False), "-feature": ("map", False), "--feature": ("map", False),
    "-Wunused": ("map", False), "-Wconf": ("map", True), "-Wtostring-interpolated": ("map", False), "-nowarn": ("map", False),
    "-Xkind-projector": ("map", False), "-Ykind-projector": ("map", False),
    "-Xmax-inlines": ("map", True), "-Xmax-inlined-trees": ("map", True), "-Yretain-trees": ("map", False),
    # scalac's own machinery, or what teq does without a flag
    "-Ycheck": ("neutral", False), "-Ycheck-constraint-deps": ("neutral", False), "-pagewidth": ("neutral", True),
    "-color": ("neutral", False), "-Xsemanticdb": ("neutral", False), "-Ysemanticdb": ("neutral", False),
    "-Yforce-sbt-phases": ("neutral", False), "-Xverify-signatures": ("neutral", False), "-Xtarget": ("neutral", True),
    "-indent": ("neutral", False), "-Yno-deep-subtypes": ("neutral", False), "-Yno-double-bindings": ("neutral", False),
    "-explain": ("neutral", False), "--explain": ("neutral", False), "-explain-types": ("neutral", False),
    "-explain-cyclic": ("neutral", False), "-Ydebug-cyclic": ("neutral", False), "-Vprint": ("neutral", False),
    "-Xprint": ("neutral", False), "-Xprint-types": ("neutral", False), "-Ylog": ("neutral", False),
    "-Ystop-after": ("neutral", False), "-Ystop-before": ("neutral", False), "-Xcheck-macros": ("neutral", False),
    "-Ycheck-reentrant": ("neutral", False), "-opt": ("neutral", False), "-opt-inline": ("neutral", False),
    "-sourceroot": ("neutral", True), "-Yprint-pos": ("neutral", False), "-Yprint-pos-syms": ("neutral", False),
    "-Ydebug": ("neutral", False), "-Yshow-suppressed-errors": ("neutral", False), "-V": ("neutral", False),
    "-verbose": ("neutral", False), "-Ylog-classpath": ("neutral", False), "-usejavacp": ("neutral", False),
    # teq keeps no experimental restriction: every definition is usable, as under scalac's -experimental
    "-experimental": ("neutral", False),
    # warnings teq does not give
    "-Wsafe-init": ("warns", False), "-Ysafe-init-global": ("warns", False), "-Wvalue-discard": ("warns", False),
    "-Wall": ("warns", False), "-Wshadow": ("warns", False), "-Wnonunit-statement": ("warns", False),
    "-Wimplausible-patterns": ("warns", False), "-WunstableInlineAccessors": ("warns", False),
    "-Winfer-union": ("warns", False), "-Ycheck-all-patmat": ("warns", False), "-Wenum-comment-discard": ("warns", False),
    "-Xlint": ("warns", False), "-Wrecurse-with-default": ("warns", False), "-Wwrong-arrow": ("warns", False),
    # the rest, each a way scalac compiles that teq has no flag for
    "-Yexplicit-nulls": ("semantic", False), "-Yno-flexible-types": ("semantic", False), "-Yimports": ("semantic", False),
    "-Yno-imports": ("semantic", False), "-Yno-predef": ("semantic", False), "-Yno-experimental": ("semantic", False),
    "-preview": ("semantic", False), "-Xmacro-settings": ("semantic", True), "-Ycompile-scala2-library": ("semantic", False),
    "-Xmixin-force-forwarders": ("semantic", False), "-Ylegacy-lazy-vals": ("semantic", False),
    "-Ymagic-offset-header": ("semantic", False), "-Xread-docs": ("semantic", False), "-Xmax-fuel": ("semantic", True),
    "-no-indent": ("semantic", False), "-old-syntax": ("semantic", False), "-new-syntax": ("semantic", False),
    "-rewrite": ("semantic", False), "-sourcepath": ("semantic", True), "-encoding": ("semantic", True),
    "-scalajs": ("semantic", False), "-classpath": ("semantic", True), "-cp": ("semantic", True), "-d": ("semantic", True),
    "-coverage-out": ("semantic", True), "-coverage-exclude-classlikes": ("semantic", False),
    "-coverage-exclude-files": ("semantic", False), "-Yimplicit-to-given": ("semantic", False),
    "-Xplugin": ("semantic", False), "-Ybest-effort": ("semantic", False), "-Ywith-best-effort-tasty": ("semantic", False),
    "-Ytest-pickler": ("semantic", False), "-Ytest-pickler-check": ("semantic", False), "-Yrecheck-test": ("semantic", False),
    "-release": ("semantic", True), "-java-output-version": ("semantic", True), "-help": ("semantic", False),
    "-Ycc-debug": ("semantic", False), "-Ycc-verbose": ("semantic", False), "-Ycc-new": ("semantic", False),
    "-Yflexify-tasty": ("semantic", False), "-from-tasty": ("semantic", False), "-Ybackend-parallelism": ("semantic", False),
}


CHECKER_FEATURES = {"experimental.captureChecking", "experimental.separationChecking", "experimental.safe",
                    "experimental.saferExceptions"}
LANGUAGE_IMPORT = re.compile(r"\blanguage\.experimental\.(\{[^}]*\}|\w+)")


def checker_imports(lines):
    """The checker features a source imports (`import language.experimental.captureChecking`, braces or not), as
    the options that turn them on."""
    out = []
    for line in lines:
        if not line.lstrip().startswith("import "):
            continue
        for m in LANGUAGE_IMPORT.finditer(line):
            for name in re.findall(r"\w+", m.group(1)):
                if "experimental." + name in CHECKER_FEATURES:
                    out.append("-language:experimental." + name)
    return out


def option_reason(bad):
    """The first line of a test an option fails."""
    unperformed = bad.startswith("-language:") and bad[len("-language:"):] in CHECKER_FEATURES
    return f"option: {bad} ({'a check teq does not perform' if unperformed else 'teq has no flag for it'})"


def tokenize(text):
    """dotty's CommandLineParser.tokenize: words at white space, a quoted stretch (single or double) one word
    with its quotes gone."""
    out, cur, quote, any_ = [], [], None, False
    for ch in text:
        if quote:
            if ch == quote:
                quote = None
            else:
                cur.append(ch)
        elif ch in "'\"":
            quote, any_ = ch, True
        elif ch.isspace():
            if cur or any_:
                out.append("".join(cur))
            cur, any_ = [], False
        else:
            cur.append(ch)
    if cur or any_:
        out.append("".join(cur))
    return out


def teq_options(options, target):
    """scalac's options as teq's flags for `target` (check, jvm, js, interp): (flags, why not). `why not` names
    the first option that fails the test."""
    flags, languages, werror, warns, bad = [], [], False, [], None
    i = 0
    while i < len(options):
        o = options[i]
        i += 1
        name, _, value = o.partition(":")
        if name not in OPTIONS and "=" in name:
            name, _, value = o.partition("=")
        kind, takes = OPTIONS.get(name, ("semantic", False))
        if takes and not value and i < len(options):
            value = options[i]
            i += 1
        if kind == "neutral":
            continue
        if kind == "warns":
            warns.append(o)
            continue
        if kind == "semantic":
            # UTF-8 is what teq reads
            if name == "-encoding" and value.upper().replace("-", "") == "UTF8":
                continue
            bad = bad or o
            continue
        if name in ("-source", "--source"):
            flags += ["--source", value]
        elif name == "-language":
            features = [l for l in value.split(",") if l]
            unchecked = [l for l in features if l in CHECKER_FEATURES]
            if unchecked:
                bad = bad or "-language:" + unchecked[0]
            languages += features
        elif name in ("-Werror", "-Xfatal-warnings"):
            werror = True
            flags.append("--werror")
        elif name in ("-deprecation", "--deprecation"):
            flags.append("--deprecation")
        elif name in ("-feature", "--feature"):
            flags.append("--feature")
        elif name == "-Wunused":
            flags += ["--wunused", value or "all"]
        elif name == "-Wconf":
            flags += ["--wconf", value]
        elif name == "-nowarn":
            flags += ["--wconf", "any:s"]
        elif name == "-Wtostring-interpolated":
            flags.append("--wtostring-interpolated")
        elif name in ("-Xkind-projector", "-Ykind-projector"):
            if value:
                bad = bad or o
            else:
                flags.append("--kind-projector")
        elif name == "-Xmax-inlines":
            flags += ["--max-inlines", value]
        elif name == "-Xmax-inlined-trees":
            if target != "interp":
                flags += ["--max-inlined-trees", value]
        elif name == "-Yretain-trees":
            flags.append("--retain-trees")
    if languages:
        flags += ["--language", ",".join(dict.fromkeys(languages))]
    if warns and werror and not bad:
        bad = warns[0] + " with -Werror"
    if target == "interp":
        # `teq interp` takes the typer's settings and the warnings' but no --source
        flags = [f for j, f in enumerate(flags) if f != "--source" and (j == 0 or flags[j - 1] != "--source")]
    return flags, bad


TOOL_ARG = re.compile(r"(?://|/\*| \*) ?(?i:(scala|scalac|java|javac|scalajs|test|target)):((?:[^*]|\*(?!/))*)")
USING_OPTIONS = re.compile(r"//> using options? (.*)")
USING_JAVAC = re.compile(r"//> using javacOpt (.*)")
USING_TARGET = re.compile(r"//> using target\.platform (jvm|scala-js)")
USING_SCALA = re.compile(r"//> using (scala) (.*)")
USING_ANY = re.compile(r"//> using (.*)")


def tool_args(files, imports=True):
    """A test's tool arguments as the tag's runner reads them (compiler/test/dotty/tools/utils.scala,
    toolArgsParse): among each file's first ten lines, `// <tool>: args` and the `//> using` lines that are the
    whole line. (args by tool, files by platform, error). The error is dotty's own refusal of a line. With
    `imports`, scalac's hold the options a source's own imports stand for (`checker_imports`), which teq must be
    told and scalac reads itself."""
    args, platform, error = collections.defaultdict(list), collections.defaultdict(list), None
    for f in files:
        try:
            with open(f, encoding="utf-8", errors="replace", newline="") as fh:
                text = re.split(r"\r\n|\r|\n", fh.read())
        except OSError as e:
            error = error or str(e)
            continue
        lines = text[:10]
        if imports and f.endswith(".scala"):
            args["scalac"] += checker_imports(text)
        for line in lines:
            m = TOOL_ARG.search(line)
            if m:
                tool = m.group(1).lower()
                if tool in ("scalac", "javac"):
                    error = error or f"`// {tool}: args` not supported"
                else:
                    tokens = tokenize(m.group(2))
                    if tool == "target" and tokens:
                        platform[tokens[0]].append(f)
                    args[tool] += tokens
        for line in lines:
            m = USING_OPTIONS.fullmatch(line)
            if m:
                args["scalac"] += tokenize(m.group(1))
                continue
            m = USING_JAVAC.fullmatch(line)
            if m:
                args["javac"] += tokenize(m.group(1))
                continue
            m = USING_TARGET.fullmatch(line)
            if m:
                platform[m.group(1)].append(f)
                args["target"].append(m.group(1))
                continue
            if USING_SCALA.fullmatch(line):
                continue
            m = USING_ANY.fullmatch(line)
            if m:
                error = error or f"Unknown directive: `//> using {(tokenize(m.group(1)) or [''])[0]}`"
    return args, platform, error


# ---- the tests of a row -----------------------------------------------------------------------------------

class Test:
    """One test: its name in the list, the groups of files compiled in order, its flags, where its .check is
    looked for, and for a chain the steps."""

    def __init__(self, row, name, groups, check_bases, flags=None, steps=None):
        self.row, self.name, self.groups, self.check_bases = row, name, groups, check_bases
        self.flags = list(row.flags if flags is None else flags)
        self.steps = steps
        self.files = [f for g in groups for f in g]


def sources_in(d):
    return sorted(os.path.join(d, e) for e in os.listdir(d) if e.endswith(SRC) and os.path.isfile(os.path.join(d, e)))


def flatten(path):
    if os.path.isdir(path):
        return [f for e in sorted(os.listdir(path)) for f in flatten(os.path.join(path, e))]
    return [path]


GROUP_ORDINAL = re.compile(r"(\d+)")
GROUP_COMPILER = re.compile(r"c([\d.]+)")


def groups_of(d):
    """SeparateCompilationSource.compilationGroups: the directory's own source files by the suffix after their
    first `_`: its number, and the compiler it names (`_c3.0.0`), in that order."""
    by = collections.defaultdict(list)
    for f in sources_in(d):
        base = os.path.basename(f)
        suffix = base[base.index("_"):] if "_" in base else ""
        for ext in SRC:
            if suffix.endswith(ext):
                suffix = suffix[:-len(ext)]
        parts = suffix.split("_")
        ordinal = next((int(p) for p in parts if GROUP_ORDINAL.fullmatch(p)), -(1 << 31))
        compiler = next((GROUP_COMPILER.fullmatch(p).group(1) for p in parts if GROUP_COMPILER.fullmatch(p)), "")
        by[(ordinal, compiler)].append(f)
    return [(k, sorted(by[k])) for k in sorted(by)]


def load_list(root, name):
    out = set()
    with open(os.path.join(root, "compiler", "test", "dotc", name), encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if line.startswith("#"):
                continue
            line = line.split("#")[0].strip()
            if line:
                out.add(line)
    return out


def tests_of(row, root):
    """The row's tests, by how dotty's source names them."""
    base = os.path.join(root, "tests")
    out = []
    for entry in row.entries:
        how = entry[0]
        if how == "files-in-dir":
            _, rel, include, exclude, flags = entry
            d = os.path.join(base, rel)
            if not os.path.isdir(d):
                die(f"{row.name}: no directory {d}: the checkout lacks it")
            inc = load_list(root, include) if include else None
            exc = load_list(root, exclude) if exclude else set()
            for e in sorted(os.listdir(d)):
                if (inc is not None and e not in inc) or e in exc:
                    continue
                p = os.path.join(d, e)
                if os.path.isdir(p):
                    gs = groups_of(p)
                    if gs:
                        out.append(Test(row, e, [g for _, g in gs], [p], flags))
                        out[-1].compilers = [k[1] for k, _ in gs]
                elif e.endswith(SRC):
                    out.append(Test(row, e, [[p]], [re.sub(r"\.(scala|java)$", "", p)], flags))
        elif how == "patmat":
            _, rel, exclude = entry
            d = os.path.join(base, rel)
            if not os.path.isdir(d):
                die(f"{row.name}: no directory {d}: the checkout lacks it")
            exc = load_list(root, exclude)
            for e in sorted(os.listdir(d)):
                p = os.path.join(d, e)
                if e in exc:
                    continue
                if os.path.isdir(p):
                    t = Test(row, e, [sources_in(p)], [os.path.join(p, "expected")])
                    out.append(t)
                elif e.endswith(".scala"):
                    out.append(Test(row, e, [[p]], [p[:-len(".scala")]]))
        elif how == "file":
            _, rel, flags, *named = entry
            p = os.path.join(base, rel)
            if not os.path.isfile(p):
                die(f"{row.name}: no file {p}: the checkout lacks it")
            out.append(Test(row, named[0] if named else os.path.basename(rel), [[p]], [re.sub(r"\.(scala|java)$", "", p)], flags))
        elif how == "dir":
            _, rel, flags, *named = entry
            d = os.path.join(base, rel)
            if not os.path.isdir(d):
                die(f"{row.name}: no directory {d}: the checkout lacks it")
            srcs = [f for f in flatten(d) if f.endswith(SRC)]
            out.append(Test(row, named[0] if named else os.path.basename(rel), [srcs], [re.sub(r"\.(scala|java)$", "", f) for f in srcs], flags))
        elif how == "list":
            _, name, rels, flags = entry
            files = [os.path.join(base, r) for r in rels]
            for f in files:
                if not os.path.isfile(f):
                    die(f"{row.name}: no file {f}: the checkout lacks it")
            out.append(Test(row, name, [files], [re.sub(r"\.(scala|java)$", "", f) for f in files], flags))
        elif how == "chain":
            _, name, steps = entry
            for s in steps:
                for r in s["files"]:
                    if not os.path.exists(os.path.join(base, r)):
                        die(f"{row.name}: no {os.path.join(base, r)}: the checkout lacks it")
            t = Test(row, name, [[os.path.join(base, r) for r in s["files"]] for s in steps], [], None, steps)
            t.check_bases = [re.sub(r"\.(scala|java)$", "", os.path.join(base, r)) for r in steps[-1]["files"]]
            out.append(t)
        elif how == "units":
            _, rel, names = entry
            if not os.path.isdir(os.path.join(base, rel)):
                die(f"{row.name}: no directory {os.path.join(base, rel)}: the checkout lacks it")
            for n in names:
                out.append(Test(row, n, [[]], []))
        elif how == "each":
            # every source file directly in the directory, a test each (PrintingTest.testIn and the like)
            _, rel = entry
            d = os.path.join(base, rel)
            if not os.path.isdir(d):
                die(f"{row.name}: no directory {d}: the checkout lacks it")
            for f in sources_in(d):
                out.append(Test(row, os.path.basename(f), [[f]], [re.sub(r"\.(scala|java)$", "", f)]))
        elif how == "entries":
            # every entry of the directory holding a source at any depth (the cannot-run rows whose runner
            # walks them its own way)
            _, rel = entry
            d = os.path.join(base, rel)
            if not os.path.isdir(d):
                die(f"{row.name}: no directory {d}: the checkout lacks it")
            for e in sorted(os.listdir(d)):
                p = os.path.join(d, e)
                if (os.path.isdir(p) and any(f.endswith(SRC) for f in flatten(p))) or (os.path.isfile(p) and e.endswith(SRC)):
                    out.append(Test(row, e, [[f for f in flatten(p) if f.endswith(SRC)]], []))
        else:
            die(f"{row.name}: an entry of an unknown form {how}")
    if not out:
        die(f"{row.name}: no test found under {base}: a run that finds none proves nothing")
    seen = collections.Counter(t.name for t in out)
    twice = [n for n, c in seen.items() if c > 1]
    if twice:
        die(f"{row.name}: two tests of one name: {twice[:3]}")
    return out


# ---- running ----------------------------------------------------------------------------------------------

class Ctx:
    pass


def limited(cmd, cpu):
    """The command under its limits, set by the shell that becomes it: CPU seconds (the soft limit signals, the
    hard one behind it), the size of a file it writes, no core file."""
    script = "ulimit -c 0; ulimit -f %d; " % (OUTPUT_LIMIT // 512)
    if cpu:
        script += "ulimit -S -t %d; ulimit -H -t %d; " % (cpu, cpu + 5)
    return ["/bin/sh", "-c", script + 'exec "$@"', "sh"] + cmd


def run(cmd, out_path, cpu, wall, env=None, cwd=None):
    """Runs a command, its stdout and stderr together in out_path. (status, text, seconds): status an exit code,
    or `timeout`, or `signal <n>`."""
    start = time.monotonic()
    full_env = dict(os.environ)
    full_env.update(env or {})
    with open(out_path, "wb") as fh:
        try:
            p = subprocess.Popen(limited(cmd, cpu), stdout=fh, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, env=full_env,
                                 cwd=cwd, start_new_session=True)
        except OSError as e:
            return f"cannot start: {e}", "", 0.0
        try:
            code = p.wait(timeout=wall)
            status = code
        except subprocess.TimeoutExpired:
            try:
                os.killpg(p.pid, 9)
            except OSError:
                pass
            p.wait()
            status = "timeout"
    secs = time.monotonic() - start
    try:
        os.killpg(p.pid, 9)  # what the program left running
    except OSError:
        pass
    with open(out_path, "rb") as fh:
        text = fh.read(1 << 20).decode("utf-8", "replace")
    if status != "timeout" and isinstance(status, int) and status < 0:
        status = "timeout" if -status == 24 else "output over the limit" if -status == 25 else f"signal {-status}"
    return status, text, secs


def teq(ctx, args, scratch, tag, target_env=None):
    """One teq run under the limits. (status, text, diagnostics, seconds); the diagnostics are teq's own record
    (`--diagnostics`), each severity, file, line, column, message."""
    diag = os.path.join(scratch, tag + ".diagnostics")
    env = dict(ctx.teq_env)
    env.update({"TEQ_THREADS": "1", "TEQ_CACHE_DIR": ctx.cache, "LC_ALL": "C.UTF-8"})
    status, text, secs = run([ctx.teq] + args + ["--diagnostics", diag], os.path.join(scratch, tag + ".out"), CPU_LIMIT, WALL_LIMIT, env)
    diags = []
    if os.path.exists(diag):
        with open(diag, encoding="utf-8", errors="replace") as fh:
            for line in fh:
                if line.startswith("{"):
                    try:
                        diags.append(json.loads(line))
                    except ValueError:
                        pass
    return status, text, diags, secs


def warm(ctx):
    """One run of each way before the tests run side by side, so that the jar cache is there for all of them."""
    d = os.path.join(ctx.out, ".warm")
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    src = os.path.join(d, "warm.scala")
    with open(src, "w") as fh:
        fh.write("object Test { def main(args: Array[String]): Unit = println(List(1).map(_ + 1)) }\n")
    for i, cmd in enumerate([
            ["compiler", "check", src, "--std", "scala-library", "--classpath", ctx.lib],
            ["compiler", "build", src, "--target", "jvm", "--all-mains", "--products", os.path.join(d, "p"), "--classpath", ctx.lib],
            ["compiler", "build", src, "-o", os.path.join(d, "w.js"), "--std", "scala-library", "--classpath", ctx.lib]]):
        status, text, _, _ = teq(ctx, cmd, d, f"warm{i}")
        if status != 0:
            die(f"teq cannot build the warm-up program ({' '.join(cmd[:2])}): {text.strip().splitlines()[-1] if text.strip() else status}")


def crashed(status, text):
    """A crash of teq's: a signal, a panic, an exit status it does not document (0 done, 1 errors, 2 a command
    line it refuses, 3 what a back end or the interpreter does not do)."""
    if status == "timeout":
        return None
    if isinstance(status, str):
        return status + (" (the memory cap)" if status == "signal 9" else "")
    if "panicked at" in text or "internal error" in text or "has overflowed its stack" in text:
        line = next((l for l in text.splitlines() if "panicked at" in l or "internal error" in l or "overflowed its stack" in l), "")
        return line.strip()[:200]
    if status not in (0, 1, 2, 3):
        return f"exit {status}"
    return None


def first_error(text, diags):
    for d in diags:
        if d.get("severity") == "error":
            where = f"{os.path.basename(d['file'])}:{d.get('line')}:{d.get('col')}: " if d.get("file") else ""
            return where + d.get("message", "").split("\n")[0]
    # no error on record: the first line that is no warning's (its header, its source line, its caret)
    in_warning = False
    for line in text.splitlines():
        if not line.strip():
            continue
        if line.startswith("warning:") or ": warning: " in line or line.startswith("migration warning:"):
            in_warning = True
            continue
        if in_warning and line.startswith(" "):
            continue
        in_warning = False
        return line.strip()
    return ""


ADDRESS = re.compile(r"(0x[0-9a-f]{6,}|@[0-9a-f]{5,8}\b)")


def short(ctx, line):
    """The first line of a difference as a list keeps it: the checkout and the scratch directory by name, an
    address or an identity hash code (what differs from run to run) masked, cut at 240."""
    line = line.replace(ctx.root + "/", "$SCALA3/").replace(ctx.out + "/", "$OUT/")
    line = re.sub(r"[\x00-\x1f\x7f]", " ", line)
    return ADDRESS.sub(lambda m: "0x…" if m.group(1).startswith("0x") else "@…", line)[:240]


def group_args(ctx, test, i):
    """The tool arguments of one group's files: dotty reads them for each compilation of a test, so a group's
    options are its own files' (ParallelTesting.scala, compile: platformAndToolArgsFor over the files given)."""
    key = (test.row.name, test.name, i)
    if key not in ctx.group_args:
        ctx.group_args[key] = tool_args([f for f in test.groups[i] if f.endswith(SRC)])
    return ctx.group_args[key]


WERROR = "No warnings can be incurred under -Werror"


def with_werror(flags, status, text, diags):
    """A compilation's diagnostics with scalac's -Werror error where scalac tells it: an error of its own, without
    a place, once a compilation in which a warning was told under -Werror, whatever errors stand beside it
    (Reporter.werror: `hasWarnings && Werror`; a migration warning and a summary of unprinted warnings are
    warnings). teq keeps no record of it: it says "errors under --werror" in its summary where warnings alone failed
    the build, and nothing where an error stands too, so its warnings' records are asked. The record is teq's
    failed build read in scalac's words, never an error teq did not tell: a build that did not fail (exit 1) gets
    none, whatever it warned. The marks count the error (`// nopos-error`), a boxed .check does not."""
    warned = any(d.get("severity") in ("warning", "migration warning") for d in diags) or (status == 1 and "errors under --werror" in text)
    if "--werror" in flags and warned and status == 1:
        return diags + [{"severity": "error", "message": WERROR, "werror": True}]
    return diags


def compile_groups(ctx, test, scratch, target, tag, extra=None):
    """Compiles the test's groups in order for `target` (check: `teq compiler check`; jvm: class files), each
    against the products of those before. (outcome, first line, diagnostics, products directory): outcome is
    ok, errors, refused, timeout or crash."""
    products = os.path.join(scratch, tag + ".products")
    shutil.rmtree(products, ignore_errors=True)
    all_diags, outcome, line = [], "ok", ""
    steps = test.steps or [{"files": None} for _ in test.groups]
    outs = []
    lib = [ctx.lib] if target != "check" or extra else []
    for i, group in enumerate(test.groups):
        step = steps[i]
        # a file of the other platform (`//> using target.platform`) is not this run's (ParallelTesting.compile)
        files = [f for f in group if f.endswith(".scala") and f not in ctx.other_platform(test)]
        if not files:
            outs.append(None)
            continue
        cp = list(lib)
        options = group_args(ctx, test, i)[0].get("scalac", [])
        if test.steps:
            # a chain: each step into a directory of its own, against the steps it names alone
            products = os.path.join(scratch, f"{tag}.products{i}")
            shutil.rmtree(products, ignore_errors=True)
            cp = [outs[j] for j in step.get("against", []) if outs[j]] + [os.path.join(ctx.root, "tests", c) for c in step.get("classpath", [])] + cp
            flags, bad = teq_options(step.get("flags", test.row.flags) + options, target)
        else:
            cp = [products] + cp
            flags, bad = teq_options(test.flags + options, target)
        if bad:
            return "option", option_reason(bad), all_diags, None
        outs.append(products)
        # One group is one compilation, as scalac's: its class files in a directory. Several are compiled each
        # against the products (class files and TASTy) of those before, which the later ones read.
        separate = bool(test.steps) or len([g for g in test.groups if any(f.endswith(".scala") and f not in ctx.other_platform(test) for f in g)]) > 1
        if not separate:
            cp = [c for c in cp if c != products]
        classpath = ["--classpath", ":".join(cp)] if cp else []
        if target == "check":
            cmd = ["compiler", "check"] + files + (["--products", products] if separate else []) + classpath + (extra or []) + flags
        else:
            cmd = ["compiler", "build"] + files + ["--target", "jvm", "--all-mains"] + (["--products", products] if separate else ["-o", products]) + classpath + flags
        status, text, diags, _ = teq(ctx, cmd, scratch, f"{tag}.{i}")
        diags = with_werror(flags, status, text, diags)
        all_diags += diags
        why = crashed(status, text)
        if status == "timeout":
            return "timeout", f"teq: over {CPU_LIMIT} s of CPU", all_diags, products
        if why:
            return "crash", f"teq: {why}", all_diags, products
        if status == 2:
            return "refused", "teq: " + first_error(text, diags), all_diags, products
        if status != 0 and outcome == "ok":
            outcome, line = "errors", first_error(text, diags) or f"teq: exit {status}"
            if status == 3:
                line = "teq: " + (line or "exit 3")
        # dotty compiles every group whatever the one before gave
    return outcome, line, all_diags, products


def check_file(test, platform=None):
    for base in test.check_bases:
        for suffix in ([f".{platform}.check"] if platform else []) + [".check"]:
            if os.path.exists(base + suffix):
                return base + suffix
    return None


def lines_of(text):
    parts = re.split(r"\r\n|\r|\n", text)
    if parts and parts[-1] == "":
        parts.pop()
    return parts


def build_summary():
    """A build's summary of the warnings its flags left unreported, a whole line: the one pattern the suites read
    an expectation without (tests/support/jars.sh, SUMMARY), read from there."""
    with open(os.path.join(REPO, "tests", "support", "jars.sh"), encoding="utf-8") as fh:
        m = re.search(r"^SUMMARY='(.*)'$", fh.read(), re.M)
    if not m:
        die("board: tests/support/jars.sh holds no SUMMARY pattern")
    return re.compile(m.group(1))


BUILD_SUMMARY = build_summary()


def java_sources(ctx, test):
    _, platform, _ = ctx.args_of(test)
    other = set(platform.get("scala-js" if test.row.platform == "jvm" else "jvm", []))
    return [f for f in test.files if f.endswith(".java") and f not in other]


def marks(files, rx, prefixes):
    """The expected diagnostics of the files' marks: a count by `file:line`, and by each prefix (`nopos`)."""
    expected = collections.Counter()
    for f in files:
        if not f.endswith(SRC):
            continue
        with open(f, encoding="utf-8", errors="replace", newline="") as fh:
            for n, line in enumerate(lines_of(fh.read()), 1):
                for m in rx.finditer(line):
                    if "space" in m.groupdict() and m.group("space") == "":
                        continue  # `//error`: dotty skips the comment
                    prefix = (m.group("prefix") or "").rstrip("-")
                    expected[prefix if prefix in prefixes else f"{f}:{n}"] += 1
    return expected


ERROR_MARK = re.compile(r"//(?P<space> *)(?P<prefix>nopos-|anypos-)?error")
WARN_MARK = re.compile(r"//(?: *)(?P<prefix>nopos-)?warn")


def place(d, files_lines):
    """A diagnostic's `file:line` as dotty's runner keys it; one at the end of a file after its last line break
    counts on the line before (ParallelTesting.scala, adjustedAtEOF)."""
    if not d.get("file") or not d.get("line"):
        return None
    f, line = d["file"], d["line"]
    n = files_lines.get(f)
    if n is None:
        try:
            with open(f, encoding="utf-8", errors="replace", newline="") as fh:
                n = len(lines_of(fh.read()))
        except OSError:
            n = 0
        files_lines[f] = n
    if n and line > n:
        line = n
    return f"{f}:{line}"


def held_to_marks(ctx, test, diags, severity, rx, prefixes, what):
    """NegTest's and WarnTest's rule: the count, then each diagnostic on a marked line. None when held."""
    expected = marks(test.files, rx, prefixes)
    got = [d for d in diags if d.get("severity") == severity or (severity == "warning" and d.get("severity") == "migration warning")]
    exp_count = sum(expected.values())
    if severity == "error":
        if not got:
            return f"marks: no error where {exp_count} are marked"
        if exp_count == 0:
            return f"marks: {len(got)} errors and no mark: {first_error('', got)}"
    if exp_count != len(got):
        return f"marks: {len(got)} {what}s where {exp_count} are marked" + (f": {first_error('', got)}" if severity == "error" and len(got) > exp_count else "")
    unexpected, unplaced, lines = [], [], {}
    for d in got:
        key = place(d, lines)
        if key is None:
            if expected.get("nopos", 0) > 0:
                expected["nopos"] -= 1
            else:
                unplaced.append(d)
        elif expected.get(key, 0) > 0:
            expected[key] -= 1
        else:
            unexpected.append((key, d))
    if "anypos" in prefixes and expected.get("anypos", 0) == len(unexpected) and unexpected:
        expected["anypos"] = 0
        unexpected = []
    missing = sorted(k for k, n in expected.items() if n > 0)
    if unexpected or unplaced or missing:
        if unexpected:
            key, d = unexpected[0]
            return f"marks: {what} on an unmarked line {os.path.basename(key)}: {d.get('message', '').splitlines()[0] if d.get('message') else ''}"
        if unplaced:
            return f"marks: {what} without a place: {unplaced[0].get('message', '').splitlines()[0]}"
        return f"marks: no {what} on the marked line {os.path.basename(missing[0])}"
    return None


# ---- scalac's own diagnostics: the rows' oracles ------------------------------------------------------------

# An identity hash after a class name (`ExprImpl@1562d7e8`: `@` and the one to eight digits of Integer.toHexString)
# differs from run to run, scalac's and teq's alike: either is compared as `@<hash>`, and the oracle is written so.
MASK = re.compile(r"(?<=\w)@[0-9a-f]{1,8}\b")
# scalac writes a colour sequence around a word of some messages under -color:never too (`expression expected but
# <ESC>[31meof<ESC>[0m found`): it is no part of a message.
COLOUR = re.compile(r"\x1b\[[0-9;]*m")
MARKS_ALONE = "the marks hold; scalac's diagnostics are not compared: "
UNREAD = "a box the reader does not read"
UNDECIDED = "the marks hold; the reader cannot decide: teq's diagnostics equal all it reads of scalac's, and scalac's diagnostic %d is a box it does not read"


def digest(root, files):
    """What an oracle's entry says of the sources it was made from: sha256 over the test's files in the runner's
    order (`Test.files`), of each its path from the corpus (UTF-8, `/`), a NUL, its length in bytes in decimal, a
    NUL and its bytes."""
    h = hashlib.sha256()
    for f in files:
        with open(f, "rb") as fh:
            b = fh.read()
        h.update(os.path.relpath(f, root).replace(os.sep, "/").encode("utf-8") + b"\0" + str(len(b)).encode() + b"\0" + b)
    return h.hexdigest()


def oracle_path(row):
    return os.path.join(HERE, "dotty", row + ".oracle.jsonl")


def load_oracle(path, row):
    """A row's oracle, by test: after a head line, a line a test (tests/board/oracle.py writes them). A row without
    the file has no entry. The head must name the board's corpus commit and scalac, and every line must be an
    entry of this row once: anything else ends the run, an oracle that is not this board's proving nothing."""
    out = {}
    if not os.path.exists(path):
        return out
    with open(path, encoding="utf-8") as fh:
        lines = fh.read().split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    for n, line in enumerate(lines, 1):
        try:
            e = json.loads(line)
        except ValueError:
            e = None
        if not isinstance(e, dict):
            die(f"{path}:{n}: not a JSON object")
        if n == 1:
            if e.get("head") is not True:
                die(f"{path}: no head line")
            if e.get("corpus") != TAG_COMMIT or e.get("scalac") != SCALAC:
                die(f"{path}: the oracle is scalac {e.get('scalac')}'s at the corpus commit {e.get('corpus')}; the board is held "
                    f"to scalac {SCALAC} at {TAG_COMMIT} (capture it again: tests/board/oracle.py)")
            continue
        ds = e.get("diagnostics")
        formed = e.get("row") == row and isinstance(e.get("test"), str) and e["test"] not in out and isinstance(ds, list) and all(
            isinstance(d, dict) and isinstance(d.get("severity"), str) and (d.get("file") is None or isinstance(d["file"], str))
            and isinstance(d.get("message"), list) and all(isinstance(l, str) for l in d["message"]) for d in ds)
        if not formed:
            die(f"{path}:{n}: not an entry of the row {row}, an entry of a test twice, or one whose diagnostics are not the tool's")
        out[e["test"]] = e
    if not lines:
        die(f"{path}: no head line")
    return out


def validated(root, rows, held=os.path.join(HERE, "dotty")):
    """The oracles of the neg- and warn-kind rows a run is about to judge (the rows' files in `held`), held to the
    corpus by their own tool
    (`python3 tests/board/oracle.py validate`, no compiler: an entry for each test, once, saying its validity and
    its digest; every .check reproduced; scalac's diagnostics holding the test's marks): (the line it prints, its
    failures). A run ends on an oracle that does not validate, before a test is judged or a list written: this is
    where the gate's lines of these rows hold the data to the corpus."""
    try:
        r = subprocess.run([sys.executable, "-B", os.path.join(HERE, "oracle.py"), "validate", "--dir", held, "--row", ",".join(rows), "--corpus", root],
                           capture_output=True, text=True, timeout=300, stdin=subprocess.DEVNULL)
    except (OSError, subprocess.SubprocessError) as e:
        return "", [f"it did not run: {e}"]
    lines = r.stdout.strip().splitlines()
    fails = [l[len("FAIL "):] for l in lines if l.startswith("FAIL ")]
    if r.returncode != 0 and not fails:
        fails = [f"exit {r.returncode}: {(r.stderr.strip().splitlines() or ['no line'])[-1]}"]
    return (lines[-1] if lines else ""), fails


def no_answer(root, test, entry):
    """Why the oracle gives no answer for the test, or None where its entry is one to compare: only an entry that
    says its capture completed (`valid`) and that scalac repeated it (`stable`), made from the files the test has
    now, is scalac's answer."""
    if entry is None:
        return "the oracle has no entry for the test"
    if entry.get("valid") is not True:
        return "scalac's capture of the test did not complete" + (f" ({entry['why']})" if entry.get("why") else "")
    if entry.get("stable") is False:
        return "scalac's answer differs from run to run"
    if entry.get("stable") is not True:
        return "scalac's answer was not held to a second capture"
    try:
        same = entry.get("digest") == digest(root, test.files)
    except OSError:
        same = False
    return None if same else "the oracle's entry was made from other sources than the test's"


def comparable(severity, file, line, col, lines):
    """One diagnostic as the judge compares it, scalac's and teq's read by this one rule: (what is compared, what
    a first line shows of it), or None for one that is compared on neither side.
    A message's lines are compared without a colour sequence, with an identity hash masked, stripped, the blank
    ones at its end dropped. Of the diagnostics without a place: the error of -Werror is compared on neither side
    (scalac reports it after any warning, teq's is the runner's record where the build had no error of its own:
    the marks hold it); a summary of unprinted warnings is its count and its kind (the hint names each compiler's
    own flag); any other is compared whole."""
    msg = [MASK.sub("@<hash>", COLOUR.sub("", l)).strip() for l in lines]
    while msg and not msg[-1]:
        msg.pop()
    shown = (severity, file, line or 0, col or 0, tuple(msg))
    if file is None:
        if severity == "error" and msg == [WERROR]:
            return None
        summary = SUMMARY_COUNT.match(msg[0]) if severity == "warning" and len(msg) == 1 else None
        if summary:
            return (severity, None, 0, 0, ("a summary", int(summary.group(1)), summary.group(2))), shown
    return shown, shown


def scalac_sequence(entry):
    """The entry's diagnostics as compared, in order: (what is compared, what is shown, whether the reader read
    the box). An info is none of them. An entry flagged by the reader (a box whose own source line has its gutter
    set right: the reader takes no column and no message of it) is compared by severity, file and line alone."""
    out = []
    for d in entry["diagnostics"]:
        if d["severity"] == "info":
            continue
        if d.get("reader"):
            out.append(((d["severity"], d["file"], d.get("line") or 0), None, False))
            continue
        c = comparable(d["severity"], d["file"], d.get("line"), d.get("column"), d["message"])
        if c:
            out.append(c + (True,))
    return out


def teq_sequence(root, diags):
    """teq's records as compared, in order: (what is compared, what is shown). An info is none of them, nor is the
    record of a build's outcome, which is no diagnostic. The one thing done to teq's side alone: the corpus' own
    path is taken out of a message, scalac having run in the corpus and named its files from there
    (`tests/neg/a.scala`), where teq is given their absolute paths."""
    out = []
    for d in diags:
        if d.get("severity") in ("info", "outcome"):
            continue
        lines = (d.get("message") or "").replace(root + "/", "").split("\n")
        c = comparable(d.get("severity"), os.path.basename(d["file"]) if d.get("file") else None, d.get("line"), d.get("col"), lines)
        if c:
            out.append(c)
    return out


def difference(i, n, want, got):
    """The first line of a test whose diagnostic `i` (of scalac's `n`) is not teq's: both sides' severity, place
    and the message's first line; where those agree, the first line of the message that differs (where one
    message has ended, the other's next line that is not blank)."""
    def place(x):
        return f"{x[0]} {x[1]}:{x[2]}:{x[3]}" if x[1] else f"{x[0]} without a place"

    def show(x):
        return "nothing" if x is None else place(x) + (":" if not x[1] else "") + f" {x[4][0] if x[4] else ''}"
    head = f"check: diagnostic {i + 1} of {n}: "
    w = want[1] if want else None
    g = got[1] if got else None
    if want and w is None:
        return head + f"scalac's is {want[0][0]} {want[0][1]}:{want[0][2]}, {UNREAD}; teq's is {show(g)}"
    if w and g and w[:4] == g[:4] and w[4][:1] == g[4][:1]:
        a, b = w[4], g[4]
        k = next((k for k in range(max(len(a), len(b))) if a[k:k + 1] != b[k:k + 1]), 0)
        if k >= min(len(a), len(b)):
            longer = a if len(a) > len(b) else b
            k = next((j for j in range(k, len(longer)) if longer[j]), k)
        return head + (f"{place(w)}, line {k + 1} of the message: scalac's is {a[k] or 'a blank line' if k < len(a) else 'nothing'}; "
                       f"teq's is {b[k] or 'a blank line' if k < len(b) else 'nothing'}")
    return head + f"scalac's is {show(w)}; teq's is {show(g)}"


def held_to_oracle(root, entry, diags):
    """teq's diagnostics against the entry's, in order: (verdict, first line). `pass` where every one is equal;
    `fail` at the first that is not; `marks` where all that the reader read is equal and a box of scalac's is one
    it did not read: the reader cannot decide such a test, whose column or message in that box may be scalac's
    or not."""
    want, got = scalac_sequence(entry), teq_sequence(root, diags)
    unread = 0
    for i in range(max(len(want), len(got))):
        w = want[i] if i < len(want) else None
        g = got[i] if i < len(got) else None
        if w and g and not w[2] and w[0] == g[1][:3]:
            unread = unread or i + 1
        elif not (w and g and w[2] and w[0] == g[0]):
            return "fail", difference(i, len(want), w, g)
    if unread:
        return "marks", UNDECIDED % unread
    return "pass", ""


def unperformed_warnings(ctx, test):
    """The test's options whose warnings teq does not give (`warns` in OPTIONS): its row's and each group's."""
    out = []
    steps = test.steps or [{} for _ in test.groups]
    for i in range(len(test.groups)):
        base = steps[i].get("flags", test.row.flags) if test.steps else test.flags
        for o in base + group_args(ctx, test, i)[0].get("scalac", []):
            name = o.partition(":")[0]
            if name not in OPTIONS and "=" in name:
                name = o.partition("=")[0]
            if OPTIONS.get(name, ("semantic", False))[0] == "warns" and o not in out:
                out.append(o)
    return out


def held_to_scalac(ctx, test, diags, rx, prefixes):
    """A neg or warn test whose marks hold: (verdict, first line). A diagnostic on a marked line says only that
    teq refused or warned there, whatever its reason: the test passes where teq's diagnostics are scalac's (its
    entry of the row's oracle: severity, place and whole message, in order) and is `marks` where the oracle gives
    no answer. A test that carries no mark, under an option whose warnings teq does not give, fails before the
    oracle is asked: scalac gives none there, and teq's silence under a check it does not perform says nothing."""
    if sum(marks(test.files, rx, prefixes).values()) == 0:
        unperformed = unperformed_warnings(ctx, test)
        if unperformed:
            return "fail", f"option: {unperformed[0]} (a check teq does not perform: no warning says nothing)"
    entry = ctx.oracle.get(test.name)
    why = no_answer(ctx.root, test, entry)
    if why:
        return "marks", MARKS_ALONE + why
    return held_to_oracle(ctx.root, entry, diags)


def neg_or_warn(ctx, test, outcome, line, diags):
    """A neg or warn test from what teq's compilations gave (`compile_groups`: their outcome, its first line,
    their records): (verdict, first line). The whole decision, and a function of the test's files, those records
    and the row's oracle (`ctx.oracle`) alone: dotty's own rule first, the marks (NegTest, WarnTest), then
    scalac's diagnostics."""
    if test.row.kind == "neg":
        rx, prefixes = ERROR_MARK, ("nopos", "anypos")
        why = held_to_marks(ctx, test, diags, "error", rx, prefixes, "error")
    else:
        rx, prefixes = WARN_MARK, ("nopos",)
        errors = [d for d in diags if d.get("severity") == "error"]
        if errors or outcome == "errors":
            return "fail", "marks: an error where warnings are marked: " + (first_error("", errors) or line)
        why = held_to_marks(ctx, test, diags, "warning", rx, prefixes, "warning")
    return ("fail", why) if why else held_to_scalac(ctx, test, diags, rx, prefixes)


PATMAT_KINDS = [
    (re.compile(r"^match may not be exhaustive"), "Pattern Match Exhaustivity"),
    (re.compile(r"^Unreachable case except for null"), "Pattern Match"),
    (re.compile(r"^[Uu]nreachable case$"), "Match case Unreachable"),  # teq words one of its two sites in lower case
    (re.compile(r"^the type test for .* cannot be checked at runtime"), "Pattern Match"),
]
UNCOVERED = re.compile(r"It would fail on pattern case: (.*)$", re.S)


def patmat_lines(diags):
    """teq's errors and warnings as the tag's simplified reporter writes scalac's (TestReporter.simplifiedReporter):
    `<line>: <the message's kind>`, with `: <the uncovered cases>` after an exhaustivity warning. A message whose
    kind teq's text does not tell is written with its text, which no check holds."""
    out = []
    for d in diags:
        if d.get("severity") not in ("error", "warning", "migration warning"):
            continue
        msg = d.get("message") or ""
        if d.get("werror"):
            out.append("0: No Kind")
            continue
        kind = next((k for rx, k in PATMAT_KINDS if rx.search(msg)), None)
        line = f"{d.get('line') or 0}: "
        if kind is None:
            line += f"({d.get('severity')}) " + msg.split("\n")[0]
        else:
            line += kind
            if kind == "Pattern Match Exhaustivity":
                m = UNCOVERED.search(msg)
                line += ": " + (" ".join(m.group(1).split()) if m else "")
        out.append(line)
    return out


def run_program(ctx, test, scratch, products):
    """`java Test` over the class files, as dotty's child JVM runs it (RunnerOrchestration.scala at the tag:
    its arguments, stdout and stderr one stream). (ok, first line, output lines)."""
    # From JDK 24 the JVM warns on stderr of a program's own use of `sun.misc.Unsafe` (run/lazy-impl.scala): the
    # option keeps a test's verdict the same on every JDK.
    unsafe = ["--sun-misc-unsafe-memory-access=allow"] if ctx.jdk >= 24 else []
    cmd = ["java", "-Dfile.encoding=UTF-8", "-Dstdout.encoding=UTF-8", "-Dstderr.encoding=UTF-8", "-Duser.language=en",
           "-Duser.country=US", "-Xmx1g", "-XX:-UsePerfData"] + unsafe + ["-cp", products + ":" + ctx.lib, "Test"]
    status, text, secs = run(cmd, os.path.join(scratch, "java.out"), 0, RUN_LIMIT, {"LC_ALL": "C.UTF-8"}, cwd=scratch)
    return program_outcome(test, status, text, "jvm")


def program_outcome(test, status, text, platform, build_lines=None):
    """A program's run against its .check. `build_lines`: a pattern of whole lines that are the build's and not
    the program's, where one process does both; every other line is compared."""
    if status == "timeout":
        return "timeout", f"the program: over {RUN_LIMIT} s", []
    got = lines_of(text)
    if build_lines:
        got = [l for l in got if not build_lines.match(l)]
    if status != 0:
        why = next((l.strip() for l in got if re.match(r"^(Exception in thread|Caused by:|\w[\w.$]*(Error|Exception)\b)", l.strip())), "")
        if not why:
            why = next((l.strip() for l in reversed(got) if l.strip()), "")
        return "fail", f"run: exit {status}: {why}", got
    path = check_file(test, platform)
    if path:
        with open(path, encoding="utf-8", errors="replace", newline="") as fh:
            want = lines_of(fh.read())
        for i in range(max(len(want), len(got))):
            w = want[i] if i < len(want) else None
            g = got[i] if i < len(got) else None
            if w != g:
                return "fail", f"output: line {i + 1}: the .check has {'nothing' if w is None else repr(w)}, the program printed {'nothing' if g is None else repr(g)}", got
    return "pass", "", got


def run_js(ctx, test, scratch, std, tag):
    """The program for JavaScript, one build of all its files (the JavaScript target compiles no group apart),
    then node. (verdict, first line)."""
    options, _, _ = ctx.args_of(test)
    flags, bad = teq_options([f for f in test.flags if f != "-scalajs"] + options, "js")
    if bad:
        return "fail", option_reason(bad)
    files = [f for f in test.files if f.endswith(".scala") and f not in ctx.other_platform(test)]
    js = os.path.join(scratch, tag + ".js")
    cmd = ["compiler", "build"] + files + ["-o", js] + (["--std", "scala-library", "--classpath", ctx.lib] if std else []) + flags
    status, text, diags, _ = teq(ctx, cmd, scratch, tag)
    if status == 1 and "several entry points" in text:
        status, text, diags, _ = teq(ctx, cmd + ["--main", "Test"], scratch, tag)
    why = crashed(status, text)
    if status == "timeout":
        return "timeout", f"teq: over {CPU_LIMIT} s of CPU"
    if why:
        return "crash", f"teq: {why}"
    if status != 0:
        return "fail", ("teq: " if status in (2, 3) else "") + (first_error(text, diags) or f"exit {status}")
    if not os.path.exists(js) and os.path.exists(js[:-3] + ".mjs"):
        js = js[:-3] + ".mjs"
    status, text, secs = run(["node", "--max-old-space-size=1024", js], os.path.join(scratch, tag + ".node.out"), 0, RUN_LIMIT, cwd=scratch)
    verdict, line, _ = program_outcome(test, status, text, "scala-js")
    return verdict, line


def run_interp(ctx, test, scratch):
    options, _, _ = ctx.args_of(test)
    flags, bad = teq_options(test.flags + options, "interp")
    if bad:
        return "fail", f"option: {bad}"
    files = [f for f in test.files if f.endswith(".scala") and f not in ctx.other_platform(test)]
    cmd = ["interp"] + files + flags
    env = dict(ctx.teq_env)
    env.update({"TEQ_THREADS": "1", "TEQ_CACHE_DIR": ctx.cache, "LC_ALL": "C.UTF-8"})
    out = os.path.join(scratch, "interp.out")
    status, text, secs = run([ctx.teq] + cmd, out, CPU_LIMIT, WALL_LIMIT, env, cwd=scratch)
    if status == 1 and "several entry points" in text:
        status, text, secs = run([ctx.teq] + cmd + ["--main", "Test"], out, CPU_LIMIT, WALL_LIMIT, env, cwd=scratch)
    why = crashed(status, text)
    if status == "timeout":
        return "timeout", ""
    if why:
        return "crash", why
    # `teq interp` builds and runs in one process, so the build's summary line stands among the program's lines.
    # A .check holds the program's output alone, as the legs whose build is a command of its own compare it: that
    # one line is left out. A warning's own line, an error and every other line of either stream are compared.
    verdict, line, _ = program_outcome(test, status, text, "jvm", BUILD_SUMMARY)
    return verdict, line


def judge(ctx, test):
    """One test: (verdict, the other ways, first line)."""
    row = test.row
    if row.cannot:
        return "cannot run", "-", row.cannot
    scratch = os.path.join(ctx.out, row.name, test.name.replace("/", "__"))
    shutil.rmtree(scratch, ignore_errors=True)
    os.makedirs(scratch)
    # dotty's own filter of a compilation by the JVM it runs on (`// test: -jvm N+`), and its refusal of a line
    # it does not read, group by group
    for i in range(len(test.groups)):
        args, _, error = group_args(ctx, test, i)
        spec = options_jvm(args.get("test", []))
        if spec == "unknown":
            return "fail", "-", "dotty: unknown test option: " + " ".join(args.get("test", []))
        if spec and not jvm_fits(spec, ctx.jdk):
            return "cannot run", "-", f"jdk: the test is for JDK {spec[0]}{'+' if spec[1] else ''}, this is {ctx.jdk}"
        if error:
            return "fail", "-", "dotty: " + error
    if getattr(test, "compilers", None) and any(test.compilers):
        return "cannot run", "-", "an older compiler: a group is compiled by scalac " + next(c for c in test.compilers if c)
    if any(re.search(r"_c[\d.]+\.(scala|java)$", os.path.basename(f)) for f in test.files):
        return "cannot run", "-", "an older compiler: a file is compiled by another scalac"
    other = "-"
    if java_sources(ctx, test):
        return "fail", other, "java source: teq reads none"
    kind = row.kind
    if kind in ("pos", "run"):
        outcome, line, diags, products = compile_groups(ctx, test, scratch, "jvm", "jvm")
        if kind == "pos":
            c_outcome, c_line, _, _ = compile_groups(ctx, test, scratch, "check", "check")
            other = "check:" + {"ok": "pass", "errors": "fail", "refused": "fail", "option": "fail"}.get(c_outcome, c_outcome)
        else:
            i_verdict, _ = run_interp(ctx, test, scratch)
            other = "interp:" + i_verdict
        if outcome == "timeout":
            return "timeout", other, line
        if outcome == "crash":
            return "crash", other, line
        if outcome != "ok":
            return "fail", other, line
        if kind == "pos":
            return "pass", other, ""
        verdict, line, _ = run_program(ctx, test, scratch, products)
        return verdict, other, line
    if kind == "run-js":
        verdict, line = run_js(ctx, test, scratch, True, "js")
        l_verdict, _ = run_js(ctx, test, scratch, False, "lean")
        return verdict, "lean:" + l_verdict, line
    # the kinds judged on `teq compiler check` against scala-library
    outcome, line, diags, _ = compile_groups(ctx, test, scratch, "check", "check", ["--std", "scala-library"])
    if outcome == "timeout":
        return "timeout", other, line
    if outcome == "crash":
        return "crash", other, line
    if outcome in ("option", "refused"):
        return "fail", other, line
    if kind == "nocrash":
        return "pass", other, ""
    if kind in ("neg", "warn"):
        verdict, why = neg_or_warn(ctx, test, outcome, line, diags)
        return verdict, other, why
    if kind == "patmat":
        got = patmat_lines(diags)
        path = check_file(test)
        want = []
        if path:
            with open(path, encoding="utf-8", errors="replace", newline="") as fh:
                want = [l for l in lines_of(re.sub(r"\s+\n", "\n", fh.read().strip() + "\n"))]
        for i in range(max(len(want), len(got))):
            w = want[i] if i < len(want) else None
            g = got[i] if i < len(got) else None
            if w != g:
                return "fail", other, f"check: line {i + 1}: the .check has {'nothing' if w is None else repr(w)}, teq gives {'nothing' if g is None else repr(g)}"
        return "pass", other, ""
    die(f"{row.name}: a kind the driver does not run: {kind}")


def options_jvm(tokens):
    if not tokens:
        return None
    if len(tokens) == 2 and tokens[0] == "-jvm":
        m = re.fullmatch(r"(\d+)(\+)?", tokens[1])
        if m:
            return int(m.group(1)), bool(m.group(2))
    return "unknown"


def jvm_fits(spec, jdk):
    n, more = spec
    return jdk >= n if more else jdk == n


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--row")
    ap.add_argument("--except", dest="rest_of")
    ap.add_argument("--only")
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--update", action="store_true")
    ap.add_argument("--add", action="store_true")
    ap.add_argument("--show")
    ap.add_argument("--rows", action="store_true")
    args = ap.parse_args()

    table = ROWS.rows()
    if args.rows:
        for r in table:
            print(f"{r.name}\t{r.kind}\t{r.path}\t{r.source}\t{'cannot run: ' + r.cannot if r.cannot else ' '.join(r.flags)}")
        return 0
    every = [r.name for r in table]
    if args.row and args.rest_of:
        die("--row and --except exclude each other")
    named = (args.rest_of or args.row or "").split(",") if (args.rest_of or args.row) else []
    unknown = [n for n in named if n not in every]
    if unknown:
        die(f"no row {', '.join(unknown)} (--rows lists them)")
    names = [n for n in every if n not in named] if args.rest_of else (named or every)
    ctx = Ctx()
    ctx.root = corpus()
    ctx.lib = scala_library()
    ctx.teq, ctx.teq_env, ctx.binary, bare = teq_command()
    if bare:
        print(bare, flush=True)  # before whatever ends the run on stderr: the gate keeps a log's last line
    ctx.out = os.path.abspath(os.environ.get("BOARD_OUT") or os.path.join(REPO, "out", "board", "dotty"))
    ctx.cache = os.path.join(ctx.out, ".cache")
    os.makedirs(ctx.cache, exist_ok=True)
    ctx.group_args = {}
    ctx.jdk = major(["java", "-version"], r'version "(\d+)')
    ctx.node = major(["node", "--version"], r"v(\d+)")
    warm(ctx)
    status = 0
    unlisted, passing, stale_lists = [], [], []  # what was not as listed, in the order the last line names it
    oracles = [r.name for r in table if r.name in names and r.kind in ("neg", "warn") and not r.cannot]
    not_scalacs = validated(ctx.root, oracles)[1] if oracles else []
    if not_scalacs:
        for f in not_scalacs[1:]:
            print(f"board: an oracle does not validate: {f}", flush=True)
        die(f"board: the oracle does not validate (python3 tests/board/oracle.py validate): {len(not_scalacs)} failed, the first: {not_scalacs[0]}")
    if oracles:
        # not the tool's own line: its count of the marks it leaves to a capture's directory reads as a skip for
        # want of something to tests/gate.sh
        print(f"board: the oracles of {len(oracles)} rows validate against the corpus", flush=True)
    totals = collections.Counter()
    for name in names:
        row = next(r for r in table if r.name == name)
        if not row.cannot and row.kind in ("pos", "run") and not ctx.jdk:
            die("no java on the PATH: the run and pos rows need the JDK")
        if row.kind == "run-js" and not ctx.node:
            die("no node on the PATH: the row run-scalajs needs it")
        tests = tests_of(row, ctx.root)
        ctx.oracle = load_oracle(oracle_path(row.name), row.name) if row.kind in ("neg", "warn") and not row.cannot else {}
        ctx.tool_args = {t.name: tool_args(t.files) for t in tests}
        ctx.args_of = lambda t: (ctx.tool_args[t.name][0].get("scalac", []), ctx.tool_args[t.name][1], ctx.tool_args[t.name][2])
        ctx.other_platform = lambda t: set(ctx.tool_args[t.name][1].get("scala-js" if t.row.platform == "jvm" else "jvm", []))
        if row.kind == "run-js":
            # ScalaJSCompilationTests.shouldSkipTestSource at the tag: `// scalajs: --skip`
            tests = [t for t in tests if "--skip" not in ctx.tool_args[t.name][0].get("scalajs", [])]
        if args.only:
            rx = re.compile(args.only)
            tests = [t for t in tests if rx.search(t.name)]
        if args.show:
            tests = [t for t in tests if t.name == args.show]
            for t in tests:
                verdict, other, line = judge(ctx, t)
                print(f"{row.name}/{t.name}\t{verdict}\t{other}\t{short(ctx, line)}")
                scratch = os.path.join(ctx.out, row.name, t.name.replace("/", "__"))
                for f in sorted(os.listdir(scratch)) if os.path.isdir(scratch) else []:
                    if f.endswith(".out"):
                        print(f"--- {f}")
                        with open(os.path.join(scratch, f), encoding="utf-8", errors="replace") as fh:
                            print(short(ctx, "") + fh.read(1 << 16))
            continue
        start = time.monotonic()
        times = {}

        def one(t):
            s = time.monotonic()
            result = judge(ctx, t)
            times[t.name] = time.monotonic() - s
            return result
        with ThreadPoolExecutor(max_workers=args.jobs) as pool:
            results = list(pool.map(one, tests))
        elapsed = time.monotonic() - start
        rows_out = [(t.name, v, o, short(ctx, l)) for t, (v, o, l) in zip(tests, results)]
        counts = collections.Counter(v for _, v, _, _ in rows_out)
        os.makedirs(os.path.join(ctx.out, "lists"), exist_ok=True)
        header = f"# dotty 3.8.4 ({TAG_COMMIT[:8]}) {row.path}, {row.kind}; {row.source}; JDK {ctx.jdk}, node {ctx.node}\n# test\tverdict\tthe other ways\tthe first line of the difference\n"
        fresh = os.path.join(ctx.out, "lists", row.name + ".tsv")
        with open(fresh, "w", encoding="utf-8") as fh:
            fh.write(header)
            for r in sorted(rows_out):
                fh.write("\t".join(r) + "\n")
        others = collections.Counter(o for _, _, o, _ in rows_out if o != "-")
        print(f"{row.name}: {len(rows_out)} tests in {elapsed:.0f} s: " + ", ".join(f"{k} {counts[k]}" for k in ("pass", "marks", "fail", "timeout", "crash", "cannot run") if counts[k])
              + (" (" + ", ".join(f"{k} {n}" for k, n in sorted(others.items())) + ")" if others else ""))
        near = sorted(((s, n) for n, s in times.items() if s * NEAR > CPU_LIMIT), reverse=True)[:10]
        if near:
            print("  over a third of a limit: " + ", ".join(f"{n} {s:.0f} s" for s, n in near))
        if args.only:
            continue
        failing = {n: (v + ": " if v != "fail" else "") + l for n, v, _, l in rows_out if v in ("fail", "timeout", "crash")}
        ran = {n for n, v, _, _ in rows_out if v != "cannot run"}
        suite = "dotty-" + row.name
        # What the pending list's check is about to say, kept for the run's last line.
        listed = pending.read(suite)
        new = [] if args.add else sorted(n for n in failing if n not in listed)
        fixed = [] if args.update else sorted(n for n in listed if n in ran and n not in failing)
        held = pending.hold(suite, ran, failing, args.update, args.add, marks=counts["marks"])
        unlisted += [f"{row.name}/{n} fails and is not in tests/pending/{suite}.txt" for n in new]
        passing += [f"{row.name}/{n} fails no longer: its line in tests/pending/{suite}.txt must go (--update)" for n in fixed]
        stale = []
        if not (args.update or args.add):
            stale = stale_verdicts(row.name, {n: v for n, v, _, _ in rows_out}, set(new) | set(fixed))
            if stale:
                print(f"{suite}: {len(stale)} verdicts of tests/board/dotty/{row.name}.tsv are not this run's (write the list again: --update): "
                      + ", ".join(f"{n} (the list: {was}, the run: {now})" for n, was, now in stale[:40]) + (" ..." if len(stale) > 40 else ""))
            stale_lists += [f"{row.name}/{n} is {now} where tests/board/dotty/{row.name}.tsv says {was} (--update)" for n, was, now in stale]
        if not held or stale:
            status = 1
        elif args.update or args.add:
            os.makedirs(os.path.join(HERE, "dotty"), exist_ok=True)
            shutil.copyfile(fresh, os.path.join(HERE, "dotty", row.name + ".tsv"))
        totals.update(counts)
        totals["rows"] += 1
    if not (args.only or args.show):
        print(last_line(unlisted + passing + stale_lists, totals))
    return status


def stale_verdicts(row, verdicts, named):
    """Where the row's committed list (tests/board/dotty/<row>.tsv) does not have the run's verdict of a test:
    (test, the list's verdict, the run's), but for the tests the pending list's check has named already (a test
    that stopped passing is not answered by writing the list again)."""
    committed = {}
    path = os.path.join(HERE, "dotty", row + ".tsv")
    if os.path.exists(path):
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                parts = line.rstrip("\n").split("\t")
                if not line.startswith("#") and len(parts) > 1:
                    committed[parts[0]] = parts[1]
    out = [(n, committed.get(n, "no line"), v) for n, v in sorted(verdicts.items()) if n not in named and committed.get(n) != v]
    return out + [(n, v, "no such test") for n, v in sorted(committed.items()) if n not in verdicts]


def last_line(problems, totals):
    """The run's last line, which the gate's table keeps: what was not as listed, row and test, as many as fit
    (the lines above have all), the tests that fail unlisted before the others, a test that stopped passing being
    what a landing must not bring; or the counts of a run that was as listed."""
    if not problems:
        failed = totals["fail"] + totals["timeout"] + totals["crash"]
        return (f"board: {totals['rows']} row{'' if totals['rows'] == 1 else 's'} as listed: {sum(totals.values()) - totals['rows']} tests, "
                f"{totals['pass']} pass, " + (f"{totals['marks']} by marks alone, " if totals['marks'] else "") + f"{failed} fail, {totals['cannot run']} cannot run")
    line = f"board: {len(problems)} not as listed: {problems[0]}"
    shown = 1
    for p in problems[1:]:
        if len(line) + len(p) + 2 > LAST_LINE:
            break
        line += "; " + p
        shown += 1
    return line + (f"; and {len(problems) - shown} more" if shown < len(problems) else "")


if __name__ == "__main__":
    sys.exit(main())
