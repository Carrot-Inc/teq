"""The programs of tests/pairs-equality.sh and tests/pairs-twin.sh: (name, files, flags, legs, expectation per leg)."""
import os, re


def sources(src):
    if os.path.isdir(src):
        return sorted(os.path.join(src, f) for f in os.listdir(src) if f.endswith(".scala"))
    return [src]


def text_of(files):
    return "".join(open(f, encoding="utf-8", errors="replace").read() for f in files)


def lines_of(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read().splitlines()


def case_programs(root, legs):
    """tests/cases as tests/run.sh and tests/run_jvm.sh build them: the `// teq:` flags; a case naming jars is left
    out (its class path is the jar suite's), and so is the other platform's leg."""
    d = os.path.join(root, "tests/cases")
    for e in sorted(os.listdir(d)):
        src = os.path.join(d, e)
        if not (e.endswith(".scala") or os.path.isdir(src)):
            continue
        name = e[:-6] if e.endswith(".scala") else e
        files = sources(src)
        text = text_of(files)
        if "// jars:" in text:
            continue
        flags = []
        m = re.search(r"^// teq: (.*)$", text, re.M)
        if m:
            flags = m.group(1).split()
        own = [l for l in legs if not (l == "js" and "//> using platform jvm" in text) and not (l == "jvm" and "//> using platform js" in text)]
        lean = os.path.join(root, "tests/lean-expected", name + ".expected")
        expected = {"js": lean if os.path.exists(lean) else os.path.join(d, name + ".expected"), "jvm": os.path.join(d, name + ".expected")}
        yield "cases/" + name, files, flags, own, expected


def board_programs(root, scala3, legs):
    """The board's run rows that pass: run.tsv's on the JVM, run-scalajs.tsv's on JavaScript, each test's files
    with no option (the same command for both binaries), its .check (a .scala-js.check first for JavaScript)."""
    rows = {}
    for tsv, leg in (("run.tsv", "jvm"), ("run-scalajs.tsv", "js")):
        if leg not in legs:
            continue
        for line in lines_of(os.path.join(root, "tests/board/dotty", tsv)):
            if line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) > 1 and parts[1] == "pass":
                rows.setdefault(parts[0], []).append(leg)
    for test, ls in sorted(rows.items()):
        src = os.path.join(scala3, "tests/run", test)
        if not os.path.exists(src):
            continue
        base = src[:-6] if src.endswith(".scala") else src
        check = {"jvm": base + ".check", "js": base + ".scala-js.check" if os.path.exists(base + ".scala-js.check") else base + ".check"}
        yield "board/" + test, sources(src), [], ls, check


def dir_programs(d, legs):
    """Every program of a directory: a .scala alone, or a directory of them; one built over a library (a table's cell
    beside a library's build: lib.scala, scalac-*, prod-*) is the table's to hold and left out."""
    for e in sorted(os.listdir(d)):
        src = os.path.join(d, e)
        if e.endswith(".scala") or (os.path.isdir(src) and any(f.endswith(".scala") for f in os.listdir(src))):
            name = e[:-6] if e.endswith(".scala") else e
            files = sources(src)
            library = os.path.isdir(src) and any(f == "lib.scala" or f.startswith(("scalac-", "prod-")) for f in os.listdir(src))
            if library:
                continue
            yield os.path.basename(d) + "/" + name, files, [], legs, {}
