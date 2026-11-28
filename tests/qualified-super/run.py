#!/usr/bin/env python3
"""run.py compares each generated program with expected.tsv on js (build and node),
interp and jvm (build and run TeqMain with scala-library 3.8.4). It writes
out/qualified-super/table.tsv and prints counts per target and per axis value.
A JVM VERIFY uses its nested twin's output. The cells in expected-js.tsv
take Scala.js's value on js and interp.
The witnesses also hold scalac's PINs and selected JVM member listings.
--check compares the differences with pending.txt (exit 1 on a mismatch).
--update removes stale records; --update --add-new also records new differences.
Exit 2 for setup errors. TEQ, NODE, COURSIER_CACHE
and JOBS select the binary, node, cache and number of concurrent cells."""
import argparse
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
from witnesses import MEMBER_CLASSES, field, member_listing, unfield

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
OUT = ROOT / "out" / "qualified-super"
TEQ = os.environ.get("TEQ", ROOT / "target/release/teq")
NODE = os.environ.get("NODE", "node")
CACHE = Path(os.environ.get("COURSIER_CACHE", Path.home() / ".cache/coursier/v1"))
LIB = CACHE / "https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"
AXES = ("qualifier", "mix", "place", "member")
CLASSES = ("same", "refused", "accepted", "differs", "crash", "refused-other", "pinned")


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
    java = sorted(row.glob("*.java"))
    src = row if java else row / "Main.scala"
    js = row / "out.js"
    build = [TEQ, "compiler", "build", "--sourceroot", row, src]
    result = command([*build, "-o", js], row, "js build", compile_step=True)
    if result[0] == "out":
        result = command([NODE, js], row, "node")
    yield "js", result

    yield "interp", command([TEQ, "interp", "--sourceroot", row, src], row, "interp", compile_step=True)

    classes = row / "classes"
    result = command([*build, "--target", "jvm", "--classpath", LIB, "-o", classes], row, "jvm build", compile_step=True)
    if row.name in MEMBER_CLASSES:
        members = ("out", member_listing(classes, row.name)) if result[0] == "out" else result
        yield "jvm-members", members
    if result[0] == "out" and java:
        subprocess.run(["java", "-m", "jdk.compiler/com.sun.tools.javac.Main",
                        "-classpath", f"{classes}{os.pathsep}{LIB}", "-d", classes, *java],
                       check=True, timeout=120)
    if result[0] == "out":
        result = command(["java", "-Xss512m", "-XX:+UseSerialGC", "-XX:TieredStopAtLevel=1", "-XX:-UsePerfData",
                          "-Xshare:auto", "-cp", f"{classes}{os.pathsep}{LIB}", "TeqMain"], row, "java")
    yield "jvm", result


def classify(kind, actual, expected):
    if expected[0] == "PIN":
        return "pinned" if kind == "refused" else "accepted"
    if kind == "crash":
        return "crash"
    if kind == "refused":
        if expected[0] != "REFUSED":
            return "refused"
        return "same" if actual.split() == expected[1].split() else "refused-other"
    if expected[0] == "REFUSED":
        return "accepted"
    return "same" if kind == "out" and actual == expected[0] else "differs"


def expectations(filename, failures=()):
    answers = {}
    for line in (HERE / filename).read_text().splitlines():
        if line.startswith("#"):
            continue
        name, value, *diagnostic = line.split("\t")
        if name in answers or len(diagnostic) != int(value in failures):
            raise ValueError(f"invalid {filename} row: {name}")
        answers[name] = tuple(map(unfield, (value, *diagnostic)))
    return answers


def check_pending(path, label, actual, update, add_new):
    previous = set()
    if path.exists() or not update:
        for line in path.read_text().splitlines():
            if not line or line.startswith("#"):
                continue
            if line in previous or len(line.split("\t")) != 5:
                raise ValueError(f"invalid pending record: {line}")
            previous.add(line)
    removed = previous - actual
    added = actual - previous
    if not update:
        for line in sorted(removed):
            print("stale", line, sep="\t")
    if not add_new:
        for line in sorted(added):
            print("unlisted", line, sep="\t")
    if update:
        kept = actual if add_new else previous & actual
        header = ("# These qualified-super observations differ from scalac and are not yet mended. None is a mapping.\n"
                  "# cell\ttarget\tclass\tteq\tscalac\n")
        with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as tmp:
            tmp.write(header)
            tmp.writelines(line + "\n" for line in sorted(kept))
        Path(tmp.name).replace(path)
        print(f"{label} --update: records removed {len(removed)}, records added {len(kept - previous)}")
    failed = (0 if update else len(removed)) + (0 if add_new else len(added))
    print(f"{label}: {len(actual)} pending records, {failed} failed")
    return int(bool(failed))


def pending_args(description, path):
    parser = argparse.ArgumentParser(description=description)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--update", action="store_true")
    parser.add_argument("--add-new", action="store_true")
    parser.add_argument("--pending", type=Path, default=path)
    args = parser.parse_args()
    if args.add_new and not args.update:
        parser.error("--add-new requires --update")
    return args


def main():
    args = pending_args(__doc__, HERE / "pending.txt")
    for tool in (TEQ, NODE, "java"):
        if not shutil.which(tool):
            raise ValueError(f"missing executable: {tool}")
    if not LIB.is_file():
        raise ValueError(f"missing {LIB}")
    expected = expectations("expected.tsv", ("REFUSED", "VERIFY", "PIN"))
    js = expectations("expected-js.tsv", ("REFUSED", "FAILED", "PIN"))
    members = expectations("expected-members.tsv")
    if MEMBER_CLASSES.keys() != members.keys():
        raise ValueError("JVM member cells and expected-members.tsv disagree")
    if set(js_reference_cells()) != js.keys():
        raise ValueError("Scala.js cells and expected-js.tsv disagree")
    for name, value in js.items():
        if value[0] == "FAILED":
            raise ValueError(f"{name}: Scala.js reference failed: {value[1]}")
    OUT.mkdir(parents=True, exist_ok=True)
    counts = Counter()
    differences = []
    pending = set()
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
                axes = name.split("_")[1:] if name.startswith("qs_") else [""] * len(AXES)
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
                    differences.append(f"{name} (JVM {field(' '.join(jvm))}, Scala.js {field(' '.join(js[name]))})")
                for target, (kind, actual) in results:
                    scalac = expected[name]
                    if target == "jvm-members":
                        want = scalac = members[name]
                        reference = "javap"
                    elif target in ("js", "interp") and name in js:
                        want, reference = js[name], "scalac-js"
                    else:
                        want, reference = jvm, jvm_source
                    verdict = classify(kind, actual, want)
                    if verdict not in ("same", "pinned"):
                        pending.add("\t".join((name, target, verdict, field(actual), field("\t".join(want)))))
                    print(name, *axes, target, verdict, reference, field(actual), field("\t".join(scalac)),
                          twin, field("\t".join(js.get(name, ()))) if target != "jvm-members" else "", sep="\t", file=fh)
                    for axis, value in zip(groups, (target, *axes)):
                        if not value:
                            continue
                        groups[axis][value] = None
                        counts[axis, value, verdict] += 1
        table.replace(OUT / "table.tsv")
    observations = sum(n for (axis, _, _), n in counts.items() if axis == "target")
    print(f"qualified-super: {len(names)} cells, {observations} target observations")
    print("axis", "value", *CLASSES, sep="\t")
    for axis, values in groups.items():
        for value in values:
            print(axis, value, *(counts[axis, value, v] for v in CLASSES), sep="\t")
    print("scalac's back ends differ:", ", ".join(differences) or "none")
    if args.check or args.update:
        return check_pending(args.pending, "qualified-super", pending, args.update, args.add_new)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"run.py: {error}", file=sys.stderr)
        sys.exit(2)
