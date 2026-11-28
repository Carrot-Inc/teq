#!/usr/bin/env python3
"""run.py compares each generated program with expected.tsv on js (build and node),
interp and jvm (build and run TeqMain with scala-library 3.8.4). It writes
out/qualified-super/table.tsv and prints counts per target and per axis value.
A JVM VERIFY uses its nested twin's output. The ten cells in expected-js.tsv
take Scala.js's value on js and interp.
Exit 0 after writing the table, 2 for setup errors. TEQ, NODE, COURSIER_CACHE
and JOBS select the binary, node, cache and number of concurrent cells."""
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from gen import js_reference_cells

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
OUT = ROOT / "out" / "qualified-super"
TEQ = os.environ.get("TEQ", ROOT / "target/release/teq")
NODE = os.environ.get("NODE", "node")
CACHE = Path(os.environ.get("COURSIER_CACHE", Path.home() / ".cache/coursier/v1"))
LIB = CACHE / "https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"
TARGETS = ("js", "interp", "jvm")
AXES = ("qualifier", "mix", "place", "member")
CLASSES = ("same", "refused", "accepted", "differs", "crash", "refused-other")


def command(cmd, row, stage, *, compile_step=False):
    try:
        p = subprocess.run(cmd, cwd=row, capture_output=True, text=True, timeout=120 if compile_step else 60,
                           env={**os.environ, "LC_ALL": "C" if stage == "interp" else "C.UTF-8"})
    except subprocess.TimeoutExpired:
        return "crash", f"{stage}: timeout"
    log = (p.stderr + p.stdout).replace(str(row) + "/", "")
    panic = re.search(r"(?m)^.*\bpanicked at\b.*$", log)
    if panic:
        return "crash", panic[0]
    first = next((line.strip() for line in log.splitlines() if line.strip()), "")
    if stage == "jvm build" and p.returncode == 3:
        message = re.search(r"(?m)^jvm backend: [^\n]*", log)
        return "refused", message[0] if message else ""
    if p.returncode not in (0, 1):
        return "crash", f"{stage}: exit {p.returncode} {first}".rstrip()
    if p.returncode == 1:
        if compile_step and (stage != "interp" or "error" in p.stderr):
            message = re.search(r"(?m)(?:^|[ \t])error: ([^\n]*)", log)
            return "refused", message[1] if message else ""
        return "run error", f"{stage}: exit 1 {first}".rstrip()
    return "out", p.stdout.removesuffix("\n")


def legs(row):
    src = row / "Main.scala"
    js = row / "out.js"
    build = [TEQ, "compiler", "build", "--sourceroot", row, src]
    result = command([*build, "-o", js], row, "js build", compile_step=True)
    if result[0] == "out":
        result = command([NODE, js], row, "node")
    yield "js", result

    yield "interp", command([TEQ, "interp", "--sourceroot", row, src], row, "interp", compile_step=True)

    classes = row / "classes"
    result = command([*build, "--target", "jvm", "--classpath", LIB, "-o", classes], row, "jvm build", compile_step=True)
    if result[0] == "out":
        result = command(["java", "-Xss512m", "-XX:+UseSerialGC", "-XX:TieredStopAtLevel=1", "-XX:-UsePerfData",
                          "-Xshare:auto", "-cp", f"{classes}{os.pathsep}{LIB}", "TeqMain"], row, "java")
    yield "jvm", result


def classify(kind, actual, expected):
    if kind == "crash":
        return "crash"
    if kind == "refused":
        if expected[0] != "REFUSED":
            return "refused"
        return "same" if actual.split() == expected[1].split() else "refused-other"
    if expected[0] == "REFUSED":
        return "accepted"
    return "same" if kind == "out" and actual == expected[0] else "differs"


def field(text):
    return text.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r")


def expectations(filename, failure):
    answers = {}
    for line in (HERE / filename).read_text().splitlines():
        if line.startswith("#"):
            continue
        name, value, *diagnostic = line.split("\t")
        if name in answers or len(diagnostic) != int(value in ("REFUSED", failure)):
            raise ValueError(f"invalid {filename} row: {name}")
        answers[name] = (value, *diagnostic)
    return answers


def main():
    for tool in (TEQ, NODE, "java"):
        if not shutil.which(tool):
            raise ValueError(f"missing executable: {tool}")
    if not LIB.is_file():
        raise ValueError(f"missing {LIB}")
    expected = expectations("expected.tsv", "VERIFY")
    js = expectations("expected-js.tsv", "FAILED")
    if set(js_reference_cells()) != js.keys():
        raise ValueError("Scala.js cells and expected-js.tsv disagree")
    for name, value in js.items():
        if value[0] == "FAILED":
            raise ValueError(f"{name}: Scala.js reference failed: {value[1]}")
    OUT.mkdir(parents=True, exist_ok=True)
    counts = Counter()
    differences = []
    groups = {axis: {} for axis in ("target", *AXES)}
    with tempfile.TemporaryDirectory(dir=OUT) as tmp:
        work = Path(tmp)
        subprocess.run([sys.executable, HERE / "gen.py", work], check=True, stdout=subprocess.DEVNULL, timeout=60)
        names = (work / "rows.txt").read_text().splitlines()
        if len(names) != len(expected) or set(names) != expected.keys():
            raise ValueError("rows.txt and expected.tsv disagree; run record.py after changing gen.py")

        def one(name):
            return list(legs(work / "rows" / name))

        table = work / "table.tsv"
        with table.open("w") as fh, ThreadPoolExecutor(int(os.environ.get("JOBS", "6"))) as pool:
            print("cell", *AXES, "target", "class", "reference", "teq", "scalac", "twin", "scalac-js", sep="\t", file=fh)
            for name, results in zip(names, pool.map(one, names)):
                _, *axes = name.split("_")
                jvm = expected[name]
                jvm_source = "scalac"
                twin = ""
                if jvm[0] == "VERIFY":
                    twin_name = name.replace("_a_", "_b_")
                    jvm = expected.get(twin_name)
                    if not jvm or len(jvm) != 1:
                        raise ValueError(f"{name}: expected output from twin {twin_name}")
                    twin = f"{twin_name}={jvm[0]}"
                    jvm_source = f"twin {twin_name}"
                if name in js and js[name] != jvm:
                    differences.append(f"{name} (JVM {' '.join(jvm)}, Scala.js {' '.join(js[name])})")
                for target, (kind, actual) in results:
                    if target in ("js", "interp") and name in js:
                        want, reference = js[name], "scalac-js"
                    else:
                        want, reference = jvm, jvm_source
                    verdict = classify(kind, actual, want)
                    print(name, *axes, target, verdict, reference, field(actual), field("\t".join(expected[name])),
                          twin, field("\t".join(js.get(name, ()))), sep="\t", file=fh)
                    for axis, value in zip(groups, (target, *axes)):
                        groups[axis][value] = None
                        counts[axis, value, verdict] += 1
        table.replace(OUT / "table.tsv")
    print(f"qualified-super: {len(names)} cells, {len(names) * len(TARGETS)} target observations")
    print("axis", "value", *CLASSES, sep="\t")
    for axis, values in groups.items():
        for value in values:
            print(axis, value, *(counts[axis, value, v] for v in CLASSES), sep="\t")
    print("scalac's back ends differ:", ", ".join(differences) or "none")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"run.py: {error}", file=sys.stderr)
        sys.exit(2)
