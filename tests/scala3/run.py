#!/usr/bin/env python3
"""Runs the executable tests of a scala/scala3 checkout (tests/run) through teq.

The checkout is taken from $SCALA3 (default ../teq-ref/scala3); its test
sources stay where they are. Every single-file test and every directory test is compiled as one
program, run with node, and compared with its .check file (a test without one has to exit 0).

Classes: pass, wrong-output, run-error, run-timeout, compile-error, compile-crash,
compile-timeout. Compile errors are bucketed by their first message into what teq leaves out on
purpose and gaps.

Writes results.json, summary.md and, with --update, the allow-list passing.txt; without --update
a listed test that no longer passes fails the run.
"""
import argparse
import collections
import concurrent.futures
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
DEFAULT_SCALA3 = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "teq-ref", "scala3")

# Compile errors are bucketed by the first error: its message and the source line it points at.
# Rules are tried in order; each is (bucket, topic, regex over the message, regex over the line),
# and a rule applies when both of its patterns match (None always matches).
JAVA_NAMES = r"java|javax|System|Thread|Console|Integer|Class|StringBuffer|ArrayList|Object|Exception|Runtime|Character|Long\b|Boolean|Double|Number|Runnable|Comparable|Cloneable|Serializable|Iterator|Void|Math\b|Throwable|Error|AnyRef"
RULES = [
    ("left out on purpose", "Java", r"test contains Java sources", None),
    ("left out on purpose", "exceptions (throw/try)", r"'(try|throw|catch|finally)' is not part|\bException\b|\bThrowable\b", None),
    ("left out on purpose", "exceptions (throw/try)", None, r"\b(try|throw|catch|finally)\b|@throws"),
    ("left out on purpose", "null", r"'null' is not part|type Null not found", None),
    ("left out on purpose", "null", None, r"\bnull\b|\bNull\b"),
    ("left out on purpose", "return", r"'return' is not part", None),
    ("left out on purpose", "return", None, r"\breturn\b"),
    ("left out on purpose", "Scala 2 implicits", r"Scala 2 implicits|Conversion not found|not found: implicitly", None),
    ("left out on purpose", "Scala 2 implicits", None, r"\bimplicit\b|implicitConversions|\bConversion\[|\bimplicitly\b"),
    ("left out on purpose", "inline/macro/compiletime", r"compiletime|quoted|Specialized|\binline\b|\bmacro\b|erased|transparent", None),
    ("left out on purpose", "inline/macro/compiletime", None, r"\binline\b|\bmacro\b|'\{|\$\{|\bquoted\b|compiletime|\berased\b|\btransparent\b|\bExpr\[|\bQuotes\b|\bspecialized\b|@spec\b|@static\b|@unroll|\bunroll\b|\bcaps\."),
    ("left out on purpose", "reflection (getClass/classOf/ClassTag/deriving)", r"getClass|classOf|ClassTag|\breflect\b|deriving|Mirror\b|TypeTest|isInstanceOf|asInstanceOf|runtime not found|not found: runtime", None),
    ("left out on purpose", "reflection (getClass/classOf/ClassTag/deriving)", None, r"getClass|classOf|ClassTag|\breflect\b|isInstanceOf|asInstanceOf|scala\.deriving|Mirror\.|runtimeChecked"),
    ("left out on purpose", "extends App", r"type App not found", None),
    ("left out on purpose", "secondary constructors, value classes, Enumeration, self types", r"a parent has to be a class or a trait|this type cannot be extended|an enum cannot extend a class|AnyVal not found|Enum not found|Enumeration not found|Object not found|Serializable not found|self types are not supported", None),
    ("left out on purpose", "secondary constructors, value classes, Enumeration, self types", None, r"\bthis\(|\bAnyVal\b|= Value\b|\bself:\s*\w"),
    ("left out on purpose", "overloading a value", r"overloading a value with a method is not supported|only methods can be overloaded", None),
    ("left out on purpose", "custom extractors (unapply)", r"is not a case class, so it cannot be used as a pattern|wrong number of patterns", None),
    ("left out on purpose", "custom extractors (unapply)", None, r"\bunapply(Seq)?\b"),
    ("left out on purpose", "Scala 2 control syntax", r"expected 'then'|expected 'do'", None),
    ("left out on purpose", "Scala 2 syntax (postfix, `f _`, XML, procedure syntax)", None, r"\w\s+\w+\s*$|\b\w+\s+_\s*$|\)\s+_\b|<[a-z]+[ >/]|\bwith\b\s*\{|\)\s*\{\s*$"),
    ("left out on purpose", "context functions", r"context function", None),
    ("left out on purpose", "context functions", None, r"\?=>"),
    ("left out on purpose", "match / dependent / structural types", r"Selectable|Dynamic not found|type mismatch: found .*#", None),
    ("left out on purpose", "match / dependent / structural types", None, r"=\s*\w+\s+match\b|\bSelectable\b|\bDynamic\b|\w+#\w+|\{\s*(type|val|def)\b.*\}|\.type\b|\bthis\.type\b|\w+\.this\b|\{\s*$.*\bdef\b"),
    ("left out on purpose", "Java", r"(type|not found:|cannot resolve import:) (" + JAVA_NAMES + r")\b|value getClass|is not a member of package java|jdk not found|synchronized", None),
    ("left out on purpose", "Java", None, r"\bjava\.|\bsynchronized\b"),
    ("planned", "anonymous classes", r"cannot be instantiated|does not implement abstract member|expected end of statement, found 'with'", None),
    ("planned", "anonymous classes", None, r"\bnew\s+[\w.]+(\[.*\])?\s*(\(.*\))?\s*(\{|with\b|:\s*$)|\bnew\s*\{|\bnew:\s*$"),
    ("planned", "tuples and functions of any arity", r"tuples of this size|functions with this many parameters|TupledFunction|Tuple not found|type \*:|not found: Tuple|Product not found|toList is not a member of \(", None),
    ("planned", "tuples and functions of any arity", None, r"\*:|\bEmptyTuple\b|\bTuple\."),
    ("planned", "partial functions", r"PartialFunction not found|collect is not a member", None),
    ("planned", "partial functions", None, r"\bPartialFunction\b"),
    ("gap", "package blocks (`package p:` / `package p {`)", None, r"^\s*package\s+[\w.]+\s*[:{]"),
    ("gap", "abstract type members", r"expected '=', found (new line|';')", r"^\s*(protected\s+|private\s+)?type\s+\w+"),
    ("gap", "abstract defs outside traits", r"expected '=', found new line", r"^\s*(def|val)\b|^\s*extension\b"),
    ("gap", "Float, Byte and Short", r"(type|not found:) (Float|Byte|Short)\b|toByte|toShort|toFloat", None),
    ("gap", "Float, Byte and Short", None, r"\d[fF]\b|\d\.\d+[fF]\b|\bFloat\b|\bByte\b|\bShort\b"),
    ("gap", "infix call with a block or tuple argument", None, r"\w+\s+\w+\s*\{|\w+\s+\w+\s*\(\s*\w+\s*,"),
    ("gap", "named tuples and named patterns", None, r"\(\s*\w+\s*=\s*\w+|\(\s*\w+\s*:\s*\w+\s*,\s*\w+\s*:"),
    ("gap", "sequence patterns (`xs*`, `_*`)", r"expected a pattern, found (\)|',')", r"[@ ]_\*|\w\s*\*\s*\)|\w\*\)|\*\s*,"),
    ("gap", "given with constructor arguments (`given C(args)`)", r"expected end of statement, found '\('|expected a definition", r"^\s*given\b.*\)\s*$|^\s*given\b.*\("),
    ("gap", "polymorphic function types", None, r"\[\w+\]\s*=>\s|\[\w+(,\s*\w+)*\]\s*=>>?\s*[\w(]|\[\[\w\]"),
    ("gap", "language imports", r"language not found", None),
    ("gap", "local and nested classes", r"nested classes are only supported inside objects|an object nested in a class or trait|a class nested in a class, made outside that class|local classes and types|local extension|local givens|local structural givens", None),
    ("gap", "multiple names in one val (`val a, b = e`)", r"a val needs a type or an initializer", r"^\s*(lazy\s+)?val\s+\w+\s*,"),
    ("gap", "top-level pattern definitions", r"pattern definitions are only supported inside blocks|a val needs a type or an initializer", None),
    ("gap", "for comprehension forms", r"a for expression needs a generator|expected end of enumerator|expected a pattern, found 'case'", None),
    ("gap", "standard library", r"not found: (LazyList|BigDecimal|BigInt|IntMap|Stream|Symbol|Predef|IArray)|type (LazyList|Integer|BigDecimal|Stream|Symbol|IArray|Matchable|IndexedSeq) not found|is not a member of the package|is a package, not a value|cannot resolve import|is not a member of (Array|List|Seq|Map|Set|Option|Vector|Iterator|String|Range|StringOps|Int|Long|Double|Boolean|Char|StringBuilder|HashMap|HashSet|Some)\b|Array cannot be instantiated|only classes can be instantiated with new|number too large|missing argument for parameter raw|not found: \w+", None),
    ("gap", "typing", r"type mismatch|missing argument|missing parameter type|no given|ambiguous|is not a member of|does not take|too many arguments|cannot be applied|expected \d+ arguments|is private|is not a case class or enum case|needs an explicit result type|recursive|given twice|type \w+ not found", None),
    ("gap", "syntax", r"expected|unexpected|unterminated|invalid|is not part of", None),
]
RULES_COMPILED = [(b, t, re.compile(m) if m else None, re.compile(l) if l else None) for b, t, m, l in RULES]


def bucket_of(msg, line):
    """The first rule whose message pattern and line pattern both match (a missing one matches)."""
    for bucket, topic, m_rx, l_rx in RULES_COMPILED:
        if m_rx is not None and not m_rx.search(msg):
            continue
        if l_rx is not None and not (line and l_rx.search(line)):
            continue
        return bucket, topic
    return "gap", "other"


def collect_tests(root):
    """Single files come with their .check file; a directory is one program with dir.check."""
    tests = []
    for entry in sorted(os.listdir(root)):
        path = os.path.join(root, entry)
        if entry.endswith(".scala") and os.path.isfile(path):
            name = entry[:-6]
            tests.append((name, [path], os.path.join(root, name + ".check")))
        elif os.path.isdir(path):
            files = []
            java = False
            for dirpath, _, filenames in os.walk(path):
                for f in filenames:
                    if f.endswith(".scala"):
                        files.append(os.path.join(dirpath, f))
                    elif f.endswith(".java"):
                        java = True
            if not files:
                continue
            files.sort()
            tests.append((entry, files, os.path.join(root, entry + ".check"), java))
    return [t if len(t) == 4 else t + (False,) for t in tests]


SCALA3_ROOT = os.path.abspath(os.environ.get("SCALA3", DEFAULT_SCALA3))


def relative_paths(text):
    """The checkout's absolute path written as $SCALA3, so the summaries hold no local path."""
    return text.replace(SCALA3_ROOT + "/", "$SCALA3/").replace(SCALA3_ROOT, "$SCALA3")


def first_error(stderr):
    """The first error message and the source line teq prints under it."""
    lines = relative_paths(stderr).splitlines()
    for i, line in enumerate(lines):
        if ": error: " in line:
            source = lines[i + 1].strip() if i + 1 < len(lines) else ""
            return line.split(": error: ", 1)[1].strip(), source
    return (lines[-1][:200] if lines else ""), ""


def normalise_output(text):
    return "\n".join(line.rstrip() for line in text.replace("\r\n", "\n").strip().splitlines())


def run_one(test, teq, out_dir, compile_timeout, run_timeout, target="js"):
    name, files, check, java = test
    js = os.path.join(out_dir, name + ".js")
    result = {"name": name, "files": len(files), "dir": len(files) > 1 or not files[0].endswith(name + ".scala")}
    if java:
        result.update(status="compile-error", bucket="left out on purpose", topic="Java", error="test contains Java sources")
        return result
    if target == "interp":
        return run_one_interp(test, teq, result, compile_timeout + run_timeout)
    cmd = [teq, "compiler", "build", *files, "-o", js]
    t0 = time.monotonic()
    try:
        build = subprocess.run(cmd, capture_output=True, text=True, timeout=compile_timeout)
        if build.returncode == 1 and "several entry points" in build.stderr:
            build = subprocess.run(cmd + ["--main", "Test"], capture_output=True, text=True, timeout=compile_timeout)
    except subprocess.TimeoutExpired:
        result.update(status="compile-timeout", seconds=compile_timeout)
        return result
    result["compile_seconds"] = round(time.monotonic() - t0, 3)
    if build.returncode != 0:
        err, source = first_error(build.stderr)
        if build.returncode != 1 or "panicked" in build.stderr or "internal error" in build.stderr:
            result.update(status="compile-crash", error=(build.stderr or "")[-600:], code=build.returncode)
            return result
        bucket, topic = bucket_of(err, source)
        result.update(status="compile-error", bucket=bucket, topic=topic, error=err, source=source)
        return result
    if not os.path.exists(js):
        alt = js[:-3] + ".mjs"
        if os.path.exists(alt):
            js = alt
    try:
        run = subprocess.run(["node", js], capture_output=True, text=True, timeout=run_timeout)
    except subprocess.TimeoutExpired:
        result.update(status="run-timeout", seconds=run_timeout)
        return result
    if run.returncode != 0:
        lines = [l for l in (run.stderr or "").strip().splitlines() if l.strip()]
        message = next((l.strip() for l in lines if re.match(r"^\s*\w*(Error|Exception)\b", l) or "Error:" in l), lines[-1] if lines else "")
        result.update(status="run-error", error=message[:300], code=run.returncode, stdout=run.stdout[-2000:])
        return result
    if os.path.exists(check):
        with open(check, errors="replace") as f:
            expected = f.read()
        if normalise_output(expected) == normalise_output(run.stdout):
            result.update(status="pass")
        else:
            result.update(status="wrong-output", expected=expected[-4000:], actual=run.stdout[-4000:])
        return result
    result.update(status="pass", checked=False)
    return result


def run_one_interp(test, teq, result, timeout):
    """`teq interp`: the compile and the run are one process, so a failure is told
    apart by the exit code (3 for what the interpreter does not execute) and the message."""
    name, files, check, _ = test
    cmd = [teq, "interp", *files]
    t0 = time.monotonic()
    try:
        run = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        if run.returncode == 1 and "several entry points" in run.stderr:
            run = subprocess.run(cmd + ["--main", "Test"], capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        result.update(status="run-timeout", seconds=timeout)
        return result
    result["compile_seconds"] = round(time.monotonic() - t0, 3)
    if run.returncode != 0:
        err, source = first_error(run.stderr)
        if "panicked" in run.stderr or "internal error" in run.stderr:
            result.update(status="compile-crash", error=(run.stderr or "")[-600:], code=run.returncode)
        elif run.returncode == 3:
            result.update(status="run-error", error=(run.stderr.strip().splitlines() or [""])[-1][:300], code=3, stdout=run.stdout[-2000:])
        elif ": error: " in run.stderr:
            bucket, topic = bucket_of(err, source)
            result.update(status="compile-error", bucket=bucket, topic=topic, error=err, source=source)
        else:
            lines = [l for l in (run.stderr or "").strip().splitlines() if l.strip()]
            result.update(status="run-error", error=(lines[-1] if lines else "")[:300], code=run.returncode, stdout=run.stdout[-2000:])
        return result
    if os.path.exists(check):
        with open(check, errors="replace") as f:
            expected = f.read()
        if normalise_output(expected) == normalise_output(run.stdout):
            result.update(status="pass")
        else:
            result.update(status="wrong-output", expected=expected[-4000:], actual=run.stdout[-4000:])
        return result
    result.update(status="pass", checked=False)
    return result


ORDER = ["pass", "wrong-output", "run-error", "run-timeout", "compile-crash", "compile-timeout", "compile-error"]


def write_summary(path, results):
    counts = collections.Counter(r["status"] for r in results)
    lines = ["# scala3 tests/run through teq", "", f"{len(results)} tests", ""]
    lines.append("| class | tests |")
    lines.append("|---|---|")
    for s in ORDER:
        lines.append(f"| {s} | {counts.get(s, 0)} |")
    compile_errors = [r for r in results if r["status"] == "compile-error"]
    buckets = collections.Counter((r["bucket"], r["topic"]) for r in compile_errors)
    lines += ["", "## Compile errors by bucket", "", "| bucket | topic | tests |", "|---|---|---|"]
    for (bucket, topic), n in sorted(buckets.items(), key=lambda kv: (kv[0][0], -kv[1])):
        lines.append(f"| {bucket} | {topic or '-'} | {n} |")
    gaps = collections.Counter(r["error"] for r in compile_errors if r["bucket"] == "gap")
    lines += ["", "## Most frequent gap messages", ""]
    for msg, n in gaps.most_common(40):
        sample = next(r for r in compile_errors if r["error"] == msg)
        lines.append(f"- {n} × `{msg}` ({sample['name']}: `{sample.get('source', '')[:80]}`)")
    for status in ["wrong-output", "run-error", "run-timeout", "compile-crash", "compile-timeout"]:
        names = sorted(r["name"] for r in results if r["status"] == status)
        if names:
            lines += ["", f"## {status} ({len(names)})", "", ", ".join(names)]
    passing = sorted(r["name"] for r in results if r["status"] == "pass")
    lines += ["", f"## pass ({len(passing)})", "", ", ".join(passing), ""]
    with open(path, "w") as f:
        f.write("\n".join(lines))


def report_pending(args):
    """Runs tests/scala3/pending/*.scala (adapted tests that expose a difference) and names the
    ones that pass now, so that a fixed difference is noticed and the test can move to tests/cases."""
    pending_dir = os.path.join(HERE, "pending")
    if not os.path.isdir(pending_dir):
        return
    tests = []
    for f in sorted(os.listdir(pending_dir)):
        if f.endswith(".scala"):
            name = f[:-6]
            tests.append((name, [os.path.join(pending_dir, f)], os.path.join(pending_dir, name + ".check"), False))
    if not tests:
        return
    out_dir = os.path.join(args.out, "pending")
    os.makedirs(out_dir, exist_ok=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as ex:
        results = list(ex.map(lambda t: run_one(t, args.teq, out_dir, args.compile_timeout, args.run_timeout), tests))
    fixed = sorted(r["name"] for r in results if r["status"] == "pass")
    print(f"pending: {len(results)} tests, {len(fixed)} pass now" + (": " + ", ".join(fixed) if fixed else ""))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--teq", default=os.environ.get("TEQ", os.path.join(REPO, "target", "release", "teq")))
    ap.add_argument("--out", default=os.path.join(REPO, "out", "scala3"))
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    ap.add_argument("--compile-timeout", type=float, default=20)
    ap.add_argument("--run-timeout", type=float, default=20)
    ap.add_argument("--only", help="regex over test names")
    ap.add_argument("--update", action="store_true", help="rewrite passing.txt from this run")
    ap.add_argument("--no-dirs", action="store_true", help="single-file tests only")
    ap.add_argument("--target", default="js", choices=["js", "interp"], help="interp runs the tests in the interpreter; the allow-list is then tests/scala3/interp-passing.txt")
    ap.add_argument("--listed", action="store_true", help="only the tests of passing.txt (the JavaScript allow-list)")
    ap.add_argument("--suite", default="run", choices=["run", "run-macros"], help="run-macros runs the quoted-macro tests; their allow-lists are tests/scala3/macros-passing.txt and macros-interp-passing.txt")
    args = ap.parse_args()

    root = os.path.join(os.environ.get("SCALA3", DEFAULT_SCALA3), "tests", args.suite)
    prefix = "" if args.suite == "run" else "macros-"
    if not os.path.isdir(root):
        print(f"skipping: no scala3 checkout at {root} (set SCALA3)")
        return 0
    if not os.path.exists(args.teq):
        print(f"no teq binary at {args.teq}; build it with cargo build --release", file=sys.stderr)
        return 2
    tests = collect_tests(root)
    if args.no_dirs:
        tests = [t for t in tests if not (len(t[1]) > 1 or os.path.dirname(t[1][0]) != root)]
    if args.only:
        rx = re.compile(args.only)
        tests = [t for t in tests if rx.search(t[0])]
    if args.listed:
        with open(os.path.join(HERE, prefix + "passing.txt")) as f:
            listed = {l.strip() for l in f if l.strip() and not l.startswith("#")}
        tests = [t for t in tests if t[0] in listed]
    os.makedirs(args.out, exist_ok=True)
    t0 = time.monotonic()
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as ex:
        results = list(ex.map(lambda t: run_one(t, args.teq, args.out, args.compile_timeout, args.run_timeout, args.target), tests))
    elapsed = time.monotonic() - t0
    results.sort(key=lambda r: r["name"])
    suffix = "" if args.target == "js" else "-" + args.target
    with open(os.path.join(HERE, prefix + "results" + suffix + ".json"), "w") as f:
        json.dump(results, f, indent=1)
    write_summary(os.path.join(HERE, prefix + "summary" + suffix + ".md"), results)
    counts = collections.Counter(r["status"] for r in results)
    print(f"{len(results)} tests in {elapsed:.0f}s: " + ", ".join(f"{s} {counts.get(s, 0)}" for s in ORDER if counts.get(s)))

    if args.target == "js" and args.suite == "run":
        report_pending(args)

    passing_path = os.path.join(HERE, prefix + ("passing.txt" if args.target == "js" else args.target + "-passing.txt"))
    passing_now = {r["name"] for r in results if r["status"] == "pass"}
    if args.update:
        with open(passing_path, "w") as f:
            f.write("\n".join(sorted(passing_now)) + "\n")
        print(f"wrote {len(passing_now)} names to passing.txt")
        return 0
    if not os.path.exists(passing_path):
        print("no passing.txt yet; run with --update to create it")
        return 0
    with open(passing_path) as f:
        listed = {l.strip() for l in f if l.strip() and not l.startswith("#")}
    ran = {r["name"] for r in results}
    regressed = sorted((listed & ran) - passing_now)
    new = sorted(passing_now - listed)
    if new:
        print(f"{len(new)} newly passing (add with --update): " + ", ".join(new))
    if regressed:
        by_name = {r["name"]: r for r in results}
        print(f"REGRESSION: {len(regressed)} listed tests no longer pass:")
        for n in regressed:
            r = by_name[n]
            print(f"  {n}: {r['status']} {r.get('error', '')[:120]}")
        return 1
    print(f"all {len(listed & ran)} listed tests still pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
