#!/usr/bin/env python3
"""Runs Scala 3's compile-only tests (tests/neg, tests/pos of a scala3 checkout) through
`teq compiler check` and classifies the result per test.

  SCALA3=/path/to/scala3 TEQ=/path/to/teq python3 harness.py [options]

  --suite neg|pos|all     which suites to run (default all)
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
which makes the script a regression check.

Categories, neg suite (scalac rejects; lines with `// error` are the expected error lines):
  same        teq rejects on exactly the lines scalac reports
  subset      teq rejects on a strict subset of those lines
  different   teq rejects, at least one line scalac does not report
  accepts     teq accepts the program: a soundness or strictness gap unless the construct is
              outside the subset
  oos         teq fails on a construct left out on purpose (detail: which one)
  syntax      teq fails while parsing (detail: message); syntax outside the subset or a parser gap
  std         teq cannot find a library name the test relies on (detail: name)
  crash/hang  teq exits with another code, panics or exceeds the time limit
Categories, pos suite (scalac accepts):
  accepts     teq accepts
  rejects     teq rejects for a reason inside the subset (detail: first message, normalised)
  oos/syntax/std/crash/hang as above

The detail column of expected.txt is informative; only the category is compared.
"""
import argparse
import os
import re
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
SCALA3 = os.environ.get("SCALA3", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "teq-ref", "scala3"))
TEQ = os.environ.get("TEQ", os.path.join(HERE, "..", "..", "target", "release", "teq"))
EXPECTED = os.path.join(HERE, "expected.txt")
TIMEOUT = 15
HEAD_CAP = 1 << 20
TAIL_CAP = 4096

DIAG = re.compile(r"^(.*?):(\d+):(\d+): (error|warning): (.*)$")
ERROR_MARK = re.compile(r"//\s*(?:nopos-|anypos-)?error\b")
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
    p = subprocess.Popen([TEQ, "compiler", "check"] + [f for f in test.files if f.endswith(".scala")],
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
    code, out, why = run_teq(test)
    if why == "hang":
        return "hang", "", out
    diags = [DIAG.match(l) for l in out.splitlines()]
    diags = [m for m in diags if m]
    errors = [m for m in diags if m.group(4) == "error"]
    if code not in (0, 1) or "panicked" in out:
        first = errors[0].group(5) if errors else out.strip().splitlines()[0] if out.strip() else ""
        return "crash", f"exit {code}: {first[:80]}", out
    if code == 0 and not errors:
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
    if test.suite == "pos":
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
            result[parts[0]] = (parts[1], parts[2] if len(parts) > 2 else "")
    return result


def load_expected():
    return read_expected(EXPECTED)


def write_expected(path, results, header):
    with open(path, "w") as fh:
        fh.write(header)
        for key in sorted(results):
            cat, detail = results[key][0], results[key][1]
            fh.write(f"{key}\t{cat}\t{detail}\n")


def save_expected(results):
    write_expected(EXPECTED, results, "# test\tcategory\tdetail   (written by harness.py --update)\n")


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
    args = ap.parse_args()

    suites = ["neg", "pos"] if args.suite == "all" else [args.suite]
    if not os.path.isdir(os.path.join(SCALA3, "tests")):
        print(f"skipping: no scala3 checkout at {SCALA3} (set SCALA3)")
        return 0
    tests = [t for s in suites for t in discover(s)]
    if args.filter:
        rx = re.compile(args.filter)
        tests = [t for t in tests if rx.search(t.key)]
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
    results = {t.key: (cat, detail) for t, (cat, detail, _) in zip(tests, outcomes)}
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
    for key, (cat, detail) in results.items():
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
