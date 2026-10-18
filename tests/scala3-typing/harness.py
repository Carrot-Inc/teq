#!/usr/bin/env python3
"""Runs Scala 3's compile-only tests (tests/neg, tests/pos of a scala3 checkout) through
`teq compiler check` and classifies the result per test.

  SCALA3=/path/to/scala3 TEQ=/path/to/teq python3 harness.py [options]

  --suite neg|pos|warn|all  which suites to run (default all)
  --filter REGEX          only tests whose name matches
  --update                rewrite expected.txt from this run
  --jobs N                parallel teq processes (default: cpu count)
  --list CATEGORY         print the tests of one category (e.g. accepts, rejects, same)
  --show NAME             print teq's output for one test
  --verbose               print every test with its category
  --slowest N             print the N slowest tests with teq's wall time (with a low --jobs
                          the times are closer to a lone run)

teq's output is kept up to HEAD_CAP (1 MiB) from the start and TAIL_CAP (4 KiB) from the end,
the rest read and dropped: the first errors and the closing `N errors found` classify a test.

Without --update the run is compared with expected.txt and exits 1 on any change of category,
which makes the script a regression check. expected.txt names the scala3 commit it was recorded on and,
per test, the flags its options gave teq.

A test's options are read as dotty's test framework reads them (`scalac_options`, the reader
tests/scala3/run.py shares): `-source` and `-language` reach teq as `--source` and `--language`, the
rest are scalac's alone; a group whose files name two source versions, or one the tracked scalac
refuses (teq refuses it with scalac's message), is `options`.

Categories, neg suite (scalac rejects; lines with `// error` are the expected error lines):
  same        teq rejects on exactly the lines scalac reports
  subset      teq rejects on a strict subset of those lines
  different   teq rejects, at least one line scalac does not report
  accepts     teq accepts the program: a soundness or strictness gap unless the construct is
              outside the subset
  oos         teq fails on a construct left out on purpose (detail: which one), or the test needs a file
              compiled by an older scalac (`_c3.0.0`)
  syntax      teq fails while parsing (detail: message); syntax outside the subset or a parser gap
  std         teq cannot find a library name the test relies on (detail: name)
  crash/hang  teq exits with another code, panics or exceeds the time limit
Categories, pos suite (scalac accepts):
  accepts     teq accepts
  rejects     teq rejects for a reason inside the subset (detail: first message, normalised)
  oos/syntax/std/crash/hang as above
Categories, warn suite (scalac accepts with warnings), held to scalac 3.8.4's diagnostics of the test
(warn-oracle.jsonl, `--capture-warn` over the test's files with the shared reader's options in their order, its
exit status and scalac's notes outside its diagnostics kept; the corpus's .check files are its own commit's
compiler's): multisets of severity (a migration warning apart), file, line, column
and whole message
  same        teq gives exactly scalac's diagnostics
  wording     at scalac's severities and places, with a message of its own (detail: teq's first differing)
  subset      at a strict sub-multiset of those places
  superset    at more places
  different   at other places
  silent      teq accepts without a warning: the test's warnings are absent
  accepts     scalac 3.8.4 rejects the test (a test of a later compiler), teq does not
  options     scalac 3.8.4 refuses the test's options (detail: its refusal, `invalid choice(s) for -language`)
  unpinned    the oracle has no capture of the test, or one that timed out or failed with no diagnostic:
              no expectation
  rejects/oos/syntax/std/crash/hang as in the pos suite

  --capture-warn          run scalac 3.8.4 (scala-cli) over the warn tests and write warn-oracle.jsonl

The detail column of expected.txt is informative; only the category is compared.
"""
import argparse
import collections
import json
import os
import re
import shlex
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))


def default_scala3():
    """`teq-ref/scala3` beside the main working tree, which a linked worktree is not in: the parent
    of the git common directory, as tests/all.sh resolves it."""
    try:
        common = subprocess.run(["git", "-C", HERE, "rev-parse", "--path-format=absolute", "--git-common-dir"],
                                capture_output=True, text=True, timeout=10).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        common = ""
    main = os.path.dirname(common) if common else os.path.join(HERE, "..", "..")
    return os.path.join(main, "..", "teq-ref", "scala3")


SCALA3 = os.environ.get("SCALA3") or default_scala3()
TEQ = os.environ.get("TEQ", os.path.join(HERE, "..", "..", "target", "release", "teq"))
EXPECTED = os.path.join(HERE, "expected.txt")
ORACLE = os.path.join(HERE, "warn-oracle.jsonl")
# scalac 3.8.4's diagnostics of the warn tests (`load_oracle`), read by main.
ORACLE_DIAGS = {}
TIMEOUT = 15
HEAD_CAP = 1 << 20
TAIL_CAP = 4096

DIAG = re.compile(r"^(.*?):(\d+):(\d+): (error|warning|migration warning): (.*)$")
ERROR_MARK = re.compile(r"//\s*(?:nopos-|anypos-)?error\b")
SCALAC_BOX = re.compile(r"^-- (?:\[E\d+\] )?(?:[A-Za-z]+ )*?(Migration Warning|Warning|Error|Info): (.*?):(\d+):(\d+) -*\s*$")
SCALAC_BARE = re.compile(r"^-- (?:\[E\d+\] )?(?:[A-Za-z]+ )*?(Migration Warning|Warning|Error|Info): -*\s*$")
FRAME = re.compile(r"^(\d+ |\s+)\|(.*)$")
CARET = re.compile(r"^\s*\|(\s*)\^+\s*$")
TEQ_HEAD = re.compile(r"^(.*?):(\d+):(\d+): (migration warning|warning|error|info): (.*)$")
TEQ_UNPLACED = re.compile(r"^(migration warning|warning|error|info): (.*)$")
TEQ_CARET = re.compile(r"^\s*\^+\s*$")
SUMMARY = re.compile(r"^\d+ (warning|error)s? found")
DEF_NAME = re.compile(r"\b(?:class|trait|object|enum|def|val|var|type|given|case|lazy val|package)\s+(?:object\s+)?`?(\w+)")

OOS_MESSAGES = [
    ("inheritance", "a parent has to be a class or a trait"),
    ("inheritance", "parent constructor arguments are not supported"),
    ("inheritance", "this type cannot be extended"),
    ("inheritance", "an enum cannot extend a class"),
    ("inheritance", "a class that extends a function type cannot extend a class as well"),
    ("inheritance", "only classes can be instantiated with new"),
    ("overloading-values", "overloading a value with a method is not supported"),
    ("overloading-values", "only methods can be overloaded"),
    ("implicits", "Scala 2 implicits are not supported"),
    ("local-types", "local classes and types are not supported"),
    ("local-types", "are not supported; move them to the top level"),
    ("local-types", "local type aliases are not supported"),
    ("local-types", "nested classes are only supported inside objects"),
    ("local-types", "an object nested in a class or trait"),
    ("local-types", "a class nested in a class, made outside that class"),
    ("local-givens", "local givens cannot take parameters"),
    ("local-givens", "local structural givens are not supported"),
    ("local-extensions", "local extension methods are not supported"),
    ("context-functions", "context function types are not supported"),
    ("extractors", "is not a case class, so it cannot be used as a pattern"),
    ("extractors", "this is not a case class or enum case"),
    ("arity", "tuples of this size are not supported"),
    ("arity", "functions with this many parameters are not supported"),
    ("pattern-defs", "pattern definitions are only supported inside blocks"),
    ("exports", "exports are only supported in the body"),
    ("exports", "export clauses are not supported in a given"),
    ("package-object", "a package object"),
    ("partial-application", "partial application over several parameter lists"),
    ("for-guard", "a guard directly after a value definition"),
    ("derives", "derives on an object is not supported"),
    ("jsinterop", "@js"),
    ("abstract-def", "an abstract def needs a result type"),
    ("local-val", "a local val needs an initializer"),
    ("val-type", "a val needs a type or an initializer"),
    ("enum-extends", "an enum case of an invariant enum needs an explicit extends clause"),
]

SYNTAX_PREFIXES = (
    "expected ", "unexpected ", "unterminated ", "unclosed ", "unmatched ", "malformed ", "number too large",
    "character literal", "is not part of the supported Scala subset",
)

NORMALISE = [
    (re.compile(r"^type mismatch: found .*, required .*$"), "type mismatch"),
    (re.compile(r"^value \S+ is not a member of .*$"), "value {} is not a member of {}"),
    (re.compile(r"^not found: .*$"), "not found: {}"),
    (re.compile(r"^type \S+ not found$"), "type {} not found"),
    (re.compile(r"^no given instance of type .*$"), "no given instance"),
    (re.compile(r"^ambiguous given instances.*$"), "ambiguous given instances"),
    (re.compile(r"^missing argument for parameter .*$"), "missing argument for parameter {}"),
    (re.compile(r"^too many arguments.*$"), "too many arguments"),
    (re.compile(r"^\S+ does not implement abstract member.*$"), "{} does not implement abstract member(s)"),
    (re.compile(r"^.* needs to be abstract, since .*$"), "{} needs to be abstract"),
    (re.compile(r"^object creation impossible, since .*$"), "object creation impossible"),
    (re.compile(r"^recursive use of \S+ needs an explicit result type$"), "recursive use of {} needs an explicit result type"),
    (re.compile(r"^values of types .* cannot be compared.*$"), "values of types {} and {} cannot be compared"),
    (re.compile(r"^\S+ is private to .*$"), "{} is private to {}"),
    (re.compile(r"^the constructor of \S+ is private.*$"), "the constructor of {} is private"),
    (re.compile(r"^reassignment to val .*$"), "reassignment to val {}"),
    (re.compile(r"^cannot resolve import: .*$"), "cannot resolve import"),
    (re.compile(r"^cannot resolve export: .*$"), "cannot resolve export"),
    (re.compile(r"^\S+ is exported twice$"), "{} is exported twice"),
    (re.compile(r"^type \S+ is already defined$"), "type {} is already defined"),
    (re.compile(r"^\S+ is already defined.*$"), "{} is already defined"),
    (re.compile(r"^parameter \S+ is given twice$"), "parameter {} is given twice"),
    (re.compile(r"^there is no parameter named .*$"), "there is no parameter named {}"),
    (re.compile(r"^cannot infer type arguments.*$"), "cannot infer type arguments"),
    (re.compile(r"^operator \S+ cannot be applied.*$"), "operator {} cannot be applied"),
    (re.compile(r"^\S+ is a package, not a value$"), "{} is a package, not a value"),
    (re.compile(r"^\S+ is not a member of the package$"), "{} is not a member of the package"),
    (re.compile(r"^\S+ is only allowed on .*$"), "{} is only allowed on {}"),
    (re.compile(r"^\S+ needs a body.*$"), "{} needs a body"),
    (re.compile(r"^\S+ cannot be derived.*$"), "{} cannot be derived"),
    (re.compile(r"^cyclic export of .*$"), "cyclic export"),
    (re.compile(r"^expected end of statement, found .*$"), "expected end of statement, found {}"),
    (re.compile(r"^expected (?:an )?identifier, found .*$"), "expected identifier, found {}"),
    (re.compile(r"^expected a type, found .*$"), "expected a type, found {}"),
    (re.compile(r"^expected a pattern, found .*$"), "expected a pattern, found {}"),
    (re.compile(r"^expected an expression, found .*$"), "expected an expression, found {}"),
    (re.compile(r"^expected a definition, found .*$"), "expected a definition, found {}"),
    (re.compile(r"^expected .*, found .*$"), "expected {}, found {}"),
]


# A scalac tool argument line (`// scalac: <args>` and its block-comment forms) among a file's first ten
# lines, as dotty's test framework matches it (`toolArg`, compiler/test/dotty/tools/utils.scala).
TOOL_ARG = re.compile(r"(?://|/\*| \*) ?(?i:scalac):((?:[^*]|\*(?!/))*)")
USING = re.compile(r"^\s*//>\s*using\s+(option|options)(?:\s+(.*))?$")


def header_directives(text):
    """The `//> using option` and `//> using options` lines of a file's header: the comments and blank lines
    before its first code, as the using-directives parser reads them."""
    out = []
    pos, n = 0, len(text)
    while pos < n:
        while pos < n and text[pos] in " \t\r\n":
            pos += 1
        if text.startswith("/*", pos):
            end = text.find("*/", pos + 2)
            if end < 0:
                break
            pos = end + 2
            continue
        if not text.startswith("//", pos):
            break
        end = text.find("\n", pos)
        end = n if end < 0 else end
        m = USING.match(text[pos:end].rstrip("\r"))
        if m:
            out.append(m.group(2) or "")
        pos = end
    return out


def scalac_options(files):
    """scalac's options for a test: per file, in order, the arguments of a `// scalac:` line among its first
    ten lines and of its header's `//> using option(s)` directives, each split as a command line with quotes
    grouping (dotty's `platformAndToolArgsFor` and `CommandLineParser.tokenize`), the files' joined."""
    options = []
    for f in files:
        if not f.endswith(".scala"):
            continue
        with open(f, encoding="utf-8", errors="replace") as fh:
            text = fh.read()
        args = []
        for line in text.splitlines()[:10]:
            m = TOOL_ARG.search(line)
            if m:
                args.append(m.group(1))
        args += header_directives(text)
        for a in args:
            try:
                options += shlex.split(a, posix=True)
            except ValueError:
                options += a.split()
    return options


def wconf_read(rules):
    """Whether teq reads every filter of the `-Wconf` rules (comma-separated, each `filter&...:action`): a
    message's text (`msg=`), its id (`id=`) or its category (`cat=`)."""
    for rule in rules.split(","):
        if not rule:
            continue
        filters, sep, _action = rule.rpartition(":")
        if not sep or not filters:
            return False
        if not all(f.split("=", 1)[0] in ("msg", "id", "cat") and "=" in f for f in filters.split("&")):
            return False
    return True


def teq_flags(options):
    """`-source` and `-language` among scalac's options as teq's flags, in each form scalac takes (`-source 3.3`,
    `-source:3.3`, `--source`), the language features in order, each once, and each `-Wconf:<rules>` whose
    filters teq reads (`wconf_read`) as `--wconf`: (flags, conflict), where a conflict names two source versions
    the options give."""
    source, languages, conflict, wconfs = None, [], None, []
    i = 0
    while i < len(options):
        o = options[i]
        i += 1
        if o.startswith("-Wconf:"):
            rules = o[len("-Wconf:"):]
            if wconf_read(rules):
                wconfs.append(rules)
            continue
        name = o[1:] if o.startswith("--") else o
        value = None
        if ":" in name:
            name, value = name.split(":", 1)
        if name not in ("-source", "-language"):
            continue
        if value is None:
            if i >= len(options):
                continue
            value = options[i]
            i += 1
        if name == "-source":
            if source is not None and source != value:
                conflict = f"-source {source} and {value}"
            source = value
        else:
            languages += [l for l in value.split(",") if l]
    flags = ["--source", source] if source else []
    if languages:
        flags += ["--language", ",".join(dict.fromkeys(languages))]
    for rules in wconfs:
        flags += ["--wconf", rules]
    return flags, conflict


def normalise(msg):
    for rx, out in NORMALISE:
        if rx.match(msg):
            return out
    return msg


class Test:
    def __init__(self, suite, name, files):
        self.suite = suite
        self.name = name
        self.files = files
        self.key = f"{suite}/{name}"
        self.flags, self.conflict = teq_flags(scalac_options(files))


def discover(suite):
    root = os.path.join(SCALA3, "tests", suite)
    tests = []
    for entry in sorted(os.listdir(root)):
        path = os.path.join(root, entry)
        if entry.endswith(".scala"):
            tests.append(Test(suite, entry, [path]))
        elif os.path.isdir(path):
            files = []
            for dirpath, _, names in os.walk(path):
                for n in names:
                    if n.endswith(".scala") or n.endswith(".java"):
                        files.append(os.path.join(dirpath, n))
            if files:
                tests.append(Test(suite, entry, sorted(files)))
    return tests


def expected_error_lines(files):
    lines = set()
    for f in files:
        if not f.endswith(".scala"):
            continue
        with open(f, encoding="utf-8", errors="replace") as fh:
            for i, line in enumerate(fh, 1):
                if ERROR_MARK.search(line):
                    lines.add((os.path.basename(f), i))
    return lines


def defined_names(files):
    names = set()
    for f in files:
        with open(f, encoding="utf-8", errors="replace") as fh:
            names.update(DEF_NAME.findall(fh.read()))
    return names


def run_teq(test):
    p = subprocess.Popen([TEQ, "compiler", "check"] + [f for f in test.files if f.endswith(".scala")] + test.flags,
                         stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    timed_out = threading.Event()
    timer = threading.Timer(TIMEOUT, lambda: (timed_out.set(), p.kill()))
    timer.start()
    head, tail, dropped = bytearray(), b"", 0
    try:
        while chunk := p.stdout.read(1 << 16):
            room = max(0, HEAD_CAP - len(head))
            head += chunk[:room]
            tail += chunk[room:]
            if len(tail) > TAIL_CAP:
                dropped += len(tail) - TAIL_CAP
                tail = tail[-TAIL_CAP:]
        code = p.wait()
    finally:
        timer.cancel()
    if timed_out.is_set():
        return None, "", "hang"
    if dropped:
        cut = head.rfind(b"\n") + 1
        start = tail.find(b"\n") + 1
        dropped += len(head) - cut + start
        head = head[:cut] + f"[... {dropped} bytes dropped]\n".encode() + tail[start:]
    else:
        head += tail
    return code, head.decode("utf-8", "replace"), None


def classify(test):
    if any(f.endswith(".java") for f in test.files):
        return "oos", "java", ""
    # A file the corpus's framework compiles with an older scalac (`A_1_c3.0.0.scala`), whose TASTy a check
    # of the group's sources from here cannot give (`Checking.checkValidInfix`'s `methCompiledBeforeDeprecation`).
    if any(re.search(r"_c\d+\.\d+\.\d+\.scala$", f) for f in test.files):
        return "oos", "an older scalac's file", ""
    if test.conflict:
        return "options", test.conflict, ""
    code, out, why = run_teq(test)
    if why == "hang":
        return "hang", "", out
    # A version the tracked scalac does not know (`-source 3.11`), which teq refuses as scalac does.
    if code == 2 and test.flags and ("is not a valid choice for" in out or out.startswith("usage:")):
        return "options", out.strip().splitlines()[0][:80], out
    diags = [DIAG.match(l) for l in out.splitlines()]
    diags = [m for m in diags if m]
    errors = [m for m in diags if m.group(4) == "error"]
    if code not in (0, 1) or "panicked" in out:
        first = errors[0].group(5) if errors else out.strip().splitlines()[0] if out.strip() else ""
        return "crash", f"exit {code}: {first[:80]}", out
    if code == 0 and not errors:
        if test.suite == "warn":
            return classify_warnings(test, out)
        return "accepts", "", out
    if not errors:
        return "crash", "exit 1 without an error line", out
    first = errors[0].group(5)
    for bucket, needle in OOS_MESSAGES:
        if needle in first:
            return "oos", bucket, out
    # `expected a constant value` is the typer's (`requireConst`, scalac's words), no parser's.
    if first.startswith(SYNTAX_PREFIXES) and not first.startswith("expected a constant value") or "is not part of the supported Scala subset" in first:
        return "syntax", normalise(first), out
    # A member a package lacks is scalac's `value x is not a member of p`; packages are the
    # lowercase qualifiers, `<root>` among them.
    m = (
        re.match(r"^(?:not found: |type )(\w+)(?: not found)?$", first)
        or re.match(r"^(\w+) is a package, not a value$", first)
        or re.match(r"^value (\w+) is not a member of ([a-z_]\w*|<root>)$", first)
    )
    if m and all(g not in defined_names(test.files) for g in m.groups()):
        return "std", m.group(1), out
    if test.suite in ("pos", "warn"):
        return "rejects", normalise(first), out
    expected = expected_error_lines(test.files)
    got = {(os.path.basename(m.group(1)), int(m.group(2))) for m in errors}
    if got == expected:
        return "same", "", out
    if got < expected:
        return "subset", f"{len(got)}/{len(expected)} lines", out
    if got > expected:
        return "superset", f"+{len(got - expected)} lines: {normalise(first)}", out
    return "different", normalise(first), out


def scalac_diagnostics(text):
    """scalac's diagnostics in its boxed form (`-color:never`): per box its severity (a migration warning apart),
    file name, line, column (1-based, from the caret line) and message lines whole, the `longer explanation`
    trailer and trailing blank lines left out; a box of no place has no file. In the order given."""
    out = []
    lines = text.replace("\r", "").split("\n")
    i = 0
    while i < len(lines):
        placed = SCALAC_BOX.match(lines[i])
        bare = None if placed else SCALAC_BARE.match(lines[i])
        if not placed and not bare:
            i += 1
            continue
        j, col, msg, in_message = i + 1, None, [], False
        while j < len(lines) and FRAME.match(lines[j]):
            caret = CARET.match(lines[j])
            if caret and col is None and not in_message:
                col = len(caret.group(1)) + 1
            else:
                frame = FRAME.match(lines[j])
                # The source lines carry their number in the gutter; the message's lines do not.
                if frame.group(1).strip() == "" and (col is not None or not placed):
                    in_message = True
                    msg.append(frame.group(2).strip())
            j += 1
        if msg and msg[-1] == "longer explanation available when compiling with `-explain`":
            msg.pop()
        while msg and not msg[-1]:
            msg.pop()
        level = (placed or bare).group(1)
        severity = "migration warning" if level == "Migration Warning" else level.lower()
        if placed:
            out.append((severity, os.path.basename(placed.group(2)), int(placed.group(3)), col or 0, tuple(msg)))
        else:
            out.append((severity, None, 0, 0, tuple(msg)))
        i = j
    return out


def teq_diagnostics(text):
    """teq's diagnostics (`path:line:col: severity: message`, the message's further lines, then the source line and
    its caret) in scalac_diagnostics' form."""
    out = []
    lines = text.replace("\r", "").split("\n")

    def ends(k):
        return k >= len(lines) or TEQ_HEAD.match(lines[k]) or TEQ_UNPLACED.match(lines[k]) or SUMMARY.match(lines[k].strip())

    i = 0
    while i < len(lines):
        placed = TEQ_HEAD.match(lines[i])
        bare = None if placed else TEQ_UNPLACED.match(lines[i])
        if not placed and not bare:
            i += 1
            continue
        first = placed.group(5) if placed else bare.group(2)
        c = i + 1
        while not ends(c) and not (c + 1 < len(lines) and TEQ_CARET.match(lines[c + 1]) and lines[c].startswith("  ")):
            c += 1
        at_source = not ends(c)
        msg = [l.strip() for l in [first] + lines[i + 1:c]]
        for a, b in WORDING:
            msg = [l.replace(a, b) for l in msg]
        while msg and not msg[-1]:
            msg.pop()
        if placed:
            out.append((placed.group(4), os.path.basename(placed.group(1)), int(placed.group(2)), int(placed.group(3)), tuple(msg)))
        else:
            out.append((bare.group(1), None, 0, 0, tuple(msg)))
        i = c + 2 if at_source else c
    return out


# What scala-cli and scalac print beside their diagnostics that is no refusal: the build's own lines, the
# summaries, `-Werror`'s note on warnings it promotes.
CHATTER = re.compile(r"^(Compiling project|Compiled project|Error compiling project|Compilation failed|Downloading|Downloaded|"
                     r"\s*scalac -help\s+gives more information|No warnings can be incurred under -Werror|\d+ (warning|error)s? found)")


def scalac_notes(text):
    """What scalac prints outside its boxes, its build's lines and its summaries left out: its refusals of the
    command line (`invalid choice(s) for -language: …`, `X is not a valid choice for -source`) and its notes on
    options it ignores (`bad option '…' was ignored`)."""
    notes = []
    for line in text.replace("\r", "").split("\n"):
        if not line.strip() or line.startswith("-- ") or FRAME.match(line) or CHATTER.match(line):
            continue
        notes.append(line.strip())
    return notes


# scalac's summary of the warnings of a kind its options hide (`there was 1 deprecation warning; re-run with
# -deprecation for details`), an unplaced warning, which scala-cli prints outside the boxes.
SUMMARY_NOTE = re.compile(r"^there (was|were) \d+ .* warnings?; re-run with ")
# teq names its own flags where scalac names its options.
WORDING = [("--deprecation", "-deprecation"), ("--feature", "-feature"), ("--source", "-source"), ("--language", "-language")]


def load_oracle():
    """scalac 3.8.4's capture of each warn test (`--capture-warn`), by test: its exit status, its diagnostics (its
    summaries of hidden warnings among them, unplaced) and the notes outside them (`scalac_notes`), the refusals."""
    found = {}
    if os.path.exists(ORACLE):
        with open(ORACLE, encoding="utf-8") as fh:
            for line in fh:
                if line.startswith("{"):
                    entry = json.loads(line)
                    notes = entry.get("notes", [])
                    summaries = [("warning", None, 0, 0, (n,)) for n in notes if SUMMARY_NOTE.match(n)]
                    found[entry["test"]] = {
                        "exit": entry["exit"],
                        "diagnostics": [(d[0], d[1], d[2], d[3], tuple(d[4])) for d in entry["diagnostics"]] + summaries,
                        "notes": [n for n in notes if not SUMMARY_NOTE.match(n) and not re.match(r"^-+$", n)],
                    }
    return found


def places(diags):
    return collections.Counter((d[0], d[1], d[2], d[3]) for d in diags)


def classify_warnings(test, out):
    """A warn test teq accepts, held to scalac 3.8.4's diagnostics of it (warn-oracle.jsonl): the multisets of
    severity, file, line, column and whole message. A test that warns nowhere fails as `silent`. A capture that did
    not end, or ended in failure with no diagnostic, is no expectation: a refusal of the test's options is
    `options`, any other `unpinned`."""
    entry = ORACLE_DIAGS.get(test.key)
    if entry is None:
        return "unpinned", "no capture of scalac 3.8.4 (--capture-warn)", out
    want = entry["diagnostics"]
    if entry["exit"] == 124:
        return "unpinned", "the capture timed out", out
    if entry["exit"] != 0 and not any(d[0] == "error" for d in want):
        if entry["notes"]:
            return "options", entry["notes"][0][:80], out
        if not want:
            return "unpinned", f"scalac exited {entry['exit']} without a diagnostic", out
    got = [d for d in teq_diagnostics(out) if d[0] != "info"]
    if any(d[0] == "error" for d in want):
        return "accepts", "scalac rejects", out
    if not got:
        return ("silent", "", out) if want else ("same", "", out)
    if collections.Counter(got) == collections.Counter(want):
        return "same", "", out
    pg, pw = places(got), places(want)
    if pg == pw:
        differing = next(d for d in got if d not in want)
        return "wording", normalise(differing[4][0] if differing[4] else ""), out
    if pg <= pw:
        return "subset", f"{sum(pg.values())}/{sum(pw.values())} diagnostics", out
    if pg >= pw:
        return "superset", f"+{sum((pg - pw).values())} diagnostics: {normalise(got[0][4][0] if got[0][4] else '')}", out
    return "different", normalise(got[0][4][0] if got[0][4] else ""), out


def capture_warn(tests, jobs):
    """Runs scalac 3.8.4 (scala-cli) over each warn test's files, copied to a directory of its own so that nothing is
    written into the checkout, and writes warn-oracle.jsonl. The options are the shared reader's (`scalac_options`:
    a `// scalac:` line's and the directives', in its order), given on the command line, the copies' directive lines
    blanked so that scala-cli does not give them twice; each capture keeps its exit status (124 for one that did not
    end) and scalac's notes outside its diagnostics."""
    import tempfile
    import shutil

    def one(test):
        with tempfile.TemporaryDirectory(prefix="teq-warn-oracle.") as tmp:
            files = []
            for f in test.files:
                if f.endswith(".scala"):
                    dest = os.path.join(tmp, os.path.basename(f))
                    with open(f, encoding="utf-8", errors="replace") as src:
                        lines = src.read().split("\n")
                    with open(dest, "w", encoding="utf-8") as dst:
                        dst.write("\n".join("//" if USING.match(l.rstrip("\r")) else l for l in lines))
                    files.append(dest)
            options = scalac_options(test.files)
            cmd = ["scala-cli", "compile", "-S", "3.8.4", "--server=false", "--jvm", "system", "-O", "-color:never",
                   *[w for o in options for w in ("-O", o)], *files]
            try:
                r = subprocess.run(cmd, capture_output=True, text=True, timeout=300, cwd=tmp)
                text, code = r.stderr + "\n" + r.stdout, r.returncode
            except subprocess.TimeoutExpired:
                text, code = "", 124
            return {"test": test.key, "exit": code, "options": options,
                    "diagnostics": [list(d[:4]) + [list(d[4])] for d in scalac_diagnostics(text)], "notes": scalac_notes(text)}

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        entries = list(pool.map(one, tests))
    with open(ORACLE, "w", encoding="utf-8") as fh:
        fh.write(f"# scalac 3.8.4's diagnostics of tests/warn at scala3 {corpus_commit()} (harness.py --capture-warn), a test a line\n")
        for e in sorted(entries, key=lambda e: e["test"]):
            fh.write(json.dumps(e, ensure_ascii=False) + "\n")
    print(f"wrote {ORACLE}: {len(entries)} tests")


def corpus_commit():
    try:
        return subprocess.run(["git", "-C", SCALA3, "rev-parse", "HEAD"], capture_output=True, text=True, timeout=10).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return ""


def read_expected(path):
    result = {}
    if not os.path.exists(path):
        return result
    with open(path) as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            result[parts[0]] = (parts[1], parts[2] if len(parts) > 2 else "", parts[3] if len(parts) > 3 else "")
    return result


def load_expected():
    return read_expected(EXPECTED)


def write_expected(path, results, header):
    with open(path, "w") as fh:
        fh.write(header)
        for key in sorted(results):
            cat, detail = results[key][0], results[key][1]
            flags = results[key][2] if len(results[key]) > 2 else ""
            fh.write(f"{key}\t{cat}\t{detail}" + (f"\t{flags}" if flags else "") + "\n")


def save_expected(results):
    write_expected(EXPECTED, results, f"# scala3 {corpus_commit()}\n# test\tcategory\tdetail\tteq's flags from the test's options   (written by harness.py --update)\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--suite", default="all")
    ap.add_argument("--filter")
    ap.add_argument("--update", action="store_true")
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    ap.add_argument("--list")
    ap.add_argument("--show")
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--slowest", type=int, default=0)
    ap.add_argument("--capture-warn", action="store_true")
    args = ap.parse_args()
    global ORACLE_DIAGS
    ORACLE_DIAGS = load_oracle()

    suites = ["neg", "pos", "warn"] if args.suite == "all" else [args.suite]
    if not os.path.isdir(os.path.join(SCALA3, "tests")):
        print(f"no scala3 checkout at {SCALA3} (set SCALA3)", file=sys.stderr)
        return 2
    tests = [t for s in suites for t in discover(s)]
    # A run that finds no test of a suite it was asked for proves nothing: an empty or moved checkout
    # fails rather than passing with every test skipped, the negative tests counted as the positive ones.
    empty = [s for s in suites if not any(t.suite == s for t in tests)]
    if empty:
        print(f"no {' or '.join(empty)} tests under {SCALA3}/tests", file=sys.stderr)
        return 2
    if args.filter:
        rx = re.compile(args.filter)
        tests = [t for t in tests if rx.search(t.key)]
    if args.capture_warn:
        capture_warn([t for t in tests if t.suite == "warn" and not any(f.endswith(".java") for f in t.files)], args.jobs)
        return 0
    if args.show:
        tests = [t for t in tests if t.key == args.show or t.name == args.show]
        for t in tests:
            cat, detail, out = classify(t)
            print(f"{t.key}\t{cat}\t{detail}")
            print(out)
        return 0

    def timed(test):
        start = time.monotonic()
        outcome = classify(test)
        return outcome, time.monotonic() - start

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        timed_outcomes = list(pool.map(timed, tests))
    outcomes = [o for o, _ in timed_outcomes]
    results = {t.key: (cat, detail, " ".join(t.flags)) for t, (cat, detail, _) in zip(tests, outcomes)}
    if args.slowest:
        ranked = sorted(zip(tests, timed_outcomes), key=lambda p: -p[1][1])[: args.slowest]
        print(f"slowest {len(ranked)} tests:")
        for t, ((cat, _, _), secs) in ranked:
            print(f"  {secs:7.3f} s  {t.key}\t{cat}")

    if args.list:
        for key in sorted(results):
            if results[key][0] == args.list:
                print(f"{key}\t{results[key][1]}")
        return 0

    counts = {}
    for key, (cat, detail, _) in results.items():
        suite = key.split("/")[0]
        counts.setdefault(suite, {}).setdefault(cat, 0)
        counts[suite][cat] += 1
    for suite in suites:
        total = sum(counts.get(suite, {}).values())
        print(f"{suite}: {total} tests")
        for cat, n in sorted(counts.get(suite, {}).items(), key=lambda kv: -kv[1]):
            print(f"  {cat:10} {n}")
    if args.verbose:
        for key in sorted(results):
            print(f"{key}\t{results[key][0]}\t{results[key][1]}")

    if args.update:
        old = load_expected()
        old.update(results)
        save_expected(old)
        print(f"wrote {EXPECTED}")
        return 0

    old = load_expected()
    changed = []
    for key in sorted(results):
        if key in old and old[key][0] != results[key][0]:
            changed.append((key, old[key][0], results[key][0], results[key][1]))
    if changed:
        print(f"\n{len(changed)} tests changed category:")
        for key, was, now, detail in changed:
            print(f"  {key}: {was} -> {now}  {detail}")
        return 1
    if old:
        print("\nno category changed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
