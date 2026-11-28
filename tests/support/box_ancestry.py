"""Generate and judge primitive type-pattern cells against the local JDK."""

import csv
import hashlib
import os
import select
import json
import re
import subprocess
import sys
from collections import Counter
from functools import cache
from pathlib import Path
from tempfile import TemporaryDirectory

BOXES = {
    "Integer": ("Int", "1"), "Long": ("Long", "1L"), "Short": ("Short", "1.toShort"),
    "Byte": ("Byte", "1.toByte"), "Character": ("Char", "'a'"), "Boolean": ("Boolean", "true"),
    "Float": ("Float", "1.0f"), "Double": ("Double", "1.0"),
}
CONTROLS = {"java.lang.Runnable", "java.lang.Thread"}
LEGS = ("jvm", "js", "interp")
FIELDS = ("cell", "box", "target", "condition", "conformance", "jdk")
VERDICTS = ("as scalac", "refuses", "throws", "wrong value", "timeout", "not valid")


@cache
def declaration(name):
    result = subprocess.check_output(["javap", "-public", name], text=True, timeout=60)
    header = next(line for line in result.splitlines() if line.endswith(" {"))
    # Remove type arguments before splitting the extends and implements lists.
    erased = header
    while "<" in erased:
        erased = re.sub(r"<[^<>]*>", "", erased)
    parents = re.split(r" extends | implements ", erased.removesuffix(" {"))[1:]
    generic = name + "<" in header.split(" extends ")[0].split(" implements ")[0]
    return generic, [p for group in parents for p in group.split(", ")]


def ancestors(box):
    pending = list(declaration("java.lang." + box)[1])
    found = set()
    while pending:
        name = pending.pop()
        if name not in found:
            found.add(name)
            pending.extend(declaration(name)[1])
    return found | {"java.lang.Object"}


def cells():
    version = subprocess.check_output(["java", "-version"], stderr=subprocess.STDOUT, text=True, timeout=60)
    jdk = next(line for line in version.splitlines() if ' version "' in line)
    for box in BOXES:
        parents = ancestors(box)
        if parents & CONTROLS:
            raise ValueError(f"{box} has a control among its ancestors")
        for target in sorted(parents | CONTROLS):
            for condition in ("alone", "after_test", "other_file"):
                conformance = "-"
                if condition == "other_file":
                    conformance = (f"java.lang.Comparable[java.lang.{box}]" if box in ("Character", "Boolean")
                                   else "java.lang.Number")
                name = f"Cell_{box}_{target.replace('.', '_')}_{condition}"
                yield (name, box, target, condition, conformance, jdk)
        for target in sorted({"java.io.Serializable", "java.lang.Runnable"} | (parents & {"java.lang.constant.Constable"})):
            for condition in ("inline_alone", "inline_after_test"):
                name = f"Cell_{box}_{target.replace('.', '_')}_{condition}"
                yield (name, box, target, condition, "-", jdk)


def jvm_runs(row):
    name, box, _, condition, _, _ = row
    if not condition.startswith("inline_"):
        yield name, "whole", "-", "-"
        return
    schedules = ([(t, s) for t in (1, 8) for s in (1, 2, 3)] if box in ("Integer", "Character") else [(1, 1)])
    for mode in ("whole", "split"):
        for threads, seed in schedules:
            yield f"{name}_{mode}_{threads}_{seed}", mode, str(threads), str(seed)


def gen(root):
    root.mkdir(parents=True, exist_ok=True)
    rows = list(cells())
    with (root / "cells.tsv").open("w") as manifest:
        for row in rows:
            name, box, target, condition, conformance, _ = row
            manifest.write("\t".join(row) + "\n")
            directory = root / "cells" / name
            directory.mkdir(parents=True, exist_ok=True)
            primitive, value = BOXES[box]
            typed = target + ("[?]" if declaration(target)[0] else "")
            lines = [f"object {name} {{", "  def main(args: Array[String]): Unit = {", f"    val x: {primitive} = {value}"]
            if condition in ("after_test", "inline_after_test"):
                lines.append('    (x: @unchecked) match { case b: java.lang.Number => println("num"); case _ => println("num next") }')
            lines += [f'    (x: @unchecked) match {{ case b: {typed} => println("hit"); case _ => println("next") }}', "  }", "}"]
            if condition.startswith("inline_"):
                (directory / "Lib.scala").write_text(
                    f'object Lib {{\n  inline def t[T](x: T): String = (x: @unchecked) match {{\n'
                    f'    case b: {typed} => "hit"\n    case _ => "next"\n  }}\n}}\n')
                lines[-3] = "    println(Lib.t(x))"
            (directory / "A.scala").write_text("\n".join(lines) + "\n")
            if condition == "other_file":
                (directory / "B.scala").write_text(
                    f"object Other {{\n  val n: {conformance} = java.lang.{box}.valueOf({value})\n}}\n")
            with (directory / "jvm.tsv").open("w") as jvm:
                for run in jvm_runs(row):
                    jvm.write("\t".join(run) + "\n")
    gen_patterns(root)
    print(f"box-ancestry: {len(rows)} programs, {sum(1 for r in rows for _ in jvm_runs(r))} JVM cells")


TARGETS = ("java.lang.Runnable", "java.io.Serializable")
CONSTABLE = "java.lang.constant.Constable"
VALUE_PATTERNS = ("value_alternative", "value_field", "cloneable_alternative", "cloneable_field")


def pattern_cells():
    for shape in ("bounded", "field", "extractor", "alternative", "guard"):
        for target in TARGETS:
            for mode in ("whole", "split"):
                for seed in ("alone", "beside"):
                    yield shape, target, mode, seed
    for shape in ("anyval", "value_class"):
        for target in TARGETS:
            yield shape, target, "whole", "alone"
    yield "bounded", CONSTABLE, "split", "alone"
    for shape in VALUE_PATTERNS:
        for mode in ("whole", "split"):
            for seed in ("alone", "beside"):
                yield shape, "java.lang.Cloneable", mode, seed


def pattern_name(row):
    shape, target, mode, seed = row
    return "_".join((shape, target.split(".")[-1], mode, seed))


def pattern_sources(row):
    shape, target, mode, seed = row
    split = mode == "split"
    prefix = ""
    method = "bounded[T <: Int](x: T)" if shape == "bounded" else "test(x: Int)"
    scrutinee, pattern, call = "x", f"_: {target}", "test(1)"
    seed_value = "1"
    caller = None
    if shape in VALUE_PATTERNS:
        nested = shape.endswith("_field")
        parent = " with java.lang.Cloneable" if shape.startswith("cloneable_") else ""
        prefix = f"class V(val value: Int) extends AnyVal{parent}\n"
        if nested:
            prefix += "case class Field(value: V)\n"
        param = "Field" if nested else "V"
        method = f"test[T](x: {param})"
        caller = f"  def test(x: {param}): String = Lib.test[{target}](x)\n"
        pattern = "Field(_: T)" if nested else "_: T | _: java.lang.Runnable"
        call = "test(Field(new V(1)))" if nested else "test(new V(1))"
        seed_value = "new V(1)"
    elif shape == "bounded":
        call = "bounded(1)"
    elif shape == "anyval":
        method = "test[T <: AnyVal](x: T)"
    elif shape == "value_class":
        prefix = "class V(val value: Int) extends AnyVal\n"
        method, call = "test(x: V)", "test(new V(1))"
    elif shape == "field":
        prefix = "case class Field(value: Int)\n"
        method, pattern, call = "test[T](x: Field)", "Field(_: T)", f"test[{target}](Field(1))"
    elif shape == "extractor":
        prefix = "object E { def unapply(x: Int): Option[Int] = Some(x) }\n"
        pattern = f"E(_: {target})"
    elif shape == "alternative":
        second = next(t for t in TARGETS if t != target)
        pattern += f" | _: {second}"
    elif shape == "guard":
        method, pattern, call = "test(x: Int, cond: Boolean)", f"b: {target} if cond", "test(1, true)"

    files = {}
    inline = split or shape == "field" or caller is not None
    body = (f'  {"inline " if inline else ""}def {method}: String = ({scrutinee}: @unchecked) match '
            f'{{ case {pattern} => "hit"; case _ => "next" }}\n')
    if inline:
        files["Lib.scala"] = prefix + "object Lib {\n" + body + "}\n"
        if caller is None:
            caller = f"  def result: String = Lib.{call}\n"
            call = "result"
        prefix, body = "", caller
    files["A.scala"] = prefix + "object A {\n" + body + "}\n"
    seed_expr = ('"unused"' if seed == "alone" else
                 f'({seed_value}: @unchecked) match {{ case _: {target} => "hit"; case _ => "next" }}')
    files["B.scala"] = (f"object B {{\n  def seed: String = {seed_expr}\n"
                        f"  def main(args: Array[String]): Unit = {{ println(A.{call}); println(seed) }}\n}}\n")
    return files


def gen_patterns(root):
    for row in pattern_cells():
        directory = root / "patterns" / pattern_name(row)
        directory.mkdir(parents=True, exist_ok=True)
        for name, source in pattern_sources(row).items():
            (directory / name).write_text(source)
    (root / "patterns.tsv").write_text("".join(
        "\t".join((pattern_name(row), *row)) + "\n" for row in pattern_cells()))


def class_hashes(directory, owner=None):
    classes = sorted(directory.rglob("*.class"))
    if owner:
        classes = [p for p in classes if p.stem == owner or p.stem.startswith(owner + "$")]
    hashes = {str(p.relative_to(directory)): hashlib.sha256(p.read_bytes()).hexdigest() for p in classes}
    if "A$.class" not in hashes or "A.class" not in hashes:
        raise ValueError(f"{directory}: missing A's class files")
    return hashes


def watch(root, binary, sl):
    d = root / "watch"
    d.mkdir()
    env = dict(os.environ, TEQ_THREADS="8", TEQ_BOX_TESTS="1")
    a, b = d / "A.scala", d / "B.scala"
    sources = [pattern_sources(("bounded", TARGETS[0], "whole", seed)) for seed in ("alone", "beside")]
    a.write_text(sources[0]["A.scala"])
    b.write_text(sources[0]["B.scala"])
    args = [a, b, "--target", "jvm", "--std=scala-library", "--classpath", sl]
    with (d / "watch.err").open("w") as err, subprocess.Popen(
        [binary, "compiler", "watch", *args, "-o", d / "classes"], env=env,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=err, text=True,
    ) as proc:
        try:
            for i in range(4):
                r = d / str(i)
                r.mkdir()
                if i:
                    b.write_text(sources[i % 2]["B.scala"])
                    proc.stdin.write(f"build {b}\n\n")
                    proc.stdin.flush()
                if not select.select([proc.stdout], [], [], 60)[0]:
                    raise ValueError(f"watch step {i} timed out")
                reply = json.loads(proc.stdout.readline())
                (r / "reply.json").write_text(json.dumps(reply))
                if not reply["ok"] or (i and not reply["incremental"]):
                    raise ValueError(f"watch step {i}: {reply}")
                fresh = subprocess.run([binary, "compiler", "build", *args, "--all-mains", "-o", r / "classes"],
                                       capture_output=True, env=env, timeout=60)
                for name, data in (("build.err", fresh.stderr), ("build.out", fresh.stdout)):
                    (r / name).write_bytes(data)
                (r / "build.exit").write_text(str(fresh.returncode))
                fresh.check_returncode()
                for label, directory in (("watch", d / "classes"), ("fresh", r / "classes")):
                    (r / (label + ".json")).write_text(json.dumps(class_hashes(directory), sort_keys=True))
                run = subprocess.run(["java", "-Xverify:all", "-cp", f"{d}/classes:{sl}", "B"],
                                     capture_output=True, timeout=60)
                (r / "run.exit").write_text(str(run.returncode))
                (r / "run.out").write_bytes(run.stdout)
                (r / "run.err").write_bytes(run.stderr)
            proc.stdin.write("quit\n")
            proc.stdin.flush()
            if proc.wait(timeout=30):
                raise ValueError("watch exited unsuccessfully")
        finally:
            if proc.poll() is None:
                proc.kill()


def watch_mismatch(directory):
    return json.loads((directory / "watch.json").read_text()) != json.loads((directory / "fresh.json").read_text())


def moved_bytes(alone, beside):
    return class_hashes(alone / "classes", "A") != class_hashes(beside / "classes", "A")


def compare_patterns(root, sides, rows, lines):
    manifest = read_manifest(root / "patterns.tsv")
    errors = manifest_errors(manifest, [(pattern_name(row), *row) for row in rows])
    if errors:
        raise ValueError("\n".join(errors))
    runs = root / "runs/patterns"
    names = {pattern_name(row) for row in rows}
    for side in ["scalac", *sides]:
        if {p.name for p in (runs / side).iterdir()} != names:
            raise ValueError(f"{runs / side}: incomplete or stray pattern cells")

    bad = False
    counts = Counter()
    with (root / "patterns-results.tsv").open("w") as table:
        print("cell", "binary", "verdict", "scalac", "teq", "bytes moved", "missing", sep="\t", file=table)
        for row in rows:
            shape, target, mode, seed = row
            name = pattern_name(row)
            reference = result(runs / "scalac" / name, "jvm", mode == "split")
            if reference[0] != "ok":
                raise ValueError(f"{name}: scalac {reference}")
            for side in sides:
                d = runs / side / name
                actual = result(d, "jvm", mode == "split")
                v = verdict(reference, actual)
                moved = "-"
                if seed == "beside":
                    alone = runs / side / pattern_name((shape, target, mode, "alone"))
                    moved = int(moved_bytes(alone, d))
                    counts[side, "moved"] += moved
                missing = missing_records(d, mode) if side == "head" else "-"
                if side == "head":
                    counts[side, "missing"] += missing
                if target == CONSTABLE:
                    base = result(runs / "base" / name, "jvm", True)
                    bad |= actual[0] != "ok" or actual != base
                    v += " (split placeholder)"
                else:
                    bad |= v != "as scalac"
                counts[side, v] += 1
                print(name, side, v, json.dumps(reference[1]), json.dumps(actual[1]), moved, missing, sep="\t", file=table)
    lines.append(f"{len(rows)} pattern cells, {sum(row[-1] == 'beside' for row in rows)} unchanged-class comparisons per binary")
    for side in sides:
        lines.append(f"{side} pattern verdicts: " + ", ".join(
            f"{v} {n}" for (s, v), n in counts.items() if s == side and v not in ("moved", "missing")))
        lines.append(f"{side} pattern cells whose unchanged class bytes moved: {counts[side, 'moved']}")
    if "head" in sides:
        lines.append(f"pattern test sites the writer asked with no record: {counts['head', 'missing']}")
        bad |= bool(counts["head", "missing"] or counts["head", "moved"])
        d = root / "watch"
        reported = count_lines((d / "watch.err").read_text())
        if len(reported) != 4:
            raise ValueError(f"watch: expected four count lines, found {len(reported)}")
        for i in range(4):
            r = d / str(i)
            reply = json.loads((r / "reply.json").read_text())
            seed = ("alone", "beside")[i % 2]
            name = pattern_name(("bounded", TARGETS[0], "whole", seed))
            v = verdict(result(runs / "scalac" / name, "jvm"), result(r, "jvm"))
            moved = watch_mismatch(r)
            n = reported[i] + missing_records(r, "whole")
            lines.append(f"watch step {i + 1} {seed}: {v}, class files differ from fresh: {int(moved)}, missing: {n}")
            bad |= v != "as scalac" or moved or n != 0 or not reply["ok"] or (i > 0 and not reply["incremental"])
    return bad


def read_manifest(path):
    return [tuple(line.split("\t")) for line in path.read_text().splitlines()]


def manifest_errors(rows, expected):
    counts = Counter(row[0] for row in rows)
    actual = set(rows)
    errors = [f"duplicated cell {name}" for name, n in counts.items() if n > 1]
    errors.extend(f"missing cell {row[0]}" for row in sorted(set(expected) - actual))
    errors.extend(f"stray cell {row[0]}" for row in sorted(actual - set(expected)))
    return errors


def record(directory, stage):
    status = int((directory / (stage + ".exit")).read_text().strip())
    stdout, stderr = [(directory / (stage + suffix)).read_text() for suffix in (".out", ".err")]
    return status, stdout, stderr


def result(directory, leg, split=False):
    stages = ("upstream", "build", "run") if split else ("run",) if leg == "interp" else ("build", "run")
    for stage in stages:
        code, out, err = record(directory, stage)
        if code == 124:
            return "timeout", f"{stage} timed out\n{out}{err}"
        if code:
            refusal = stage in ("upstream", "build") or (leg == "interp" and re.search(r"(?m)^\d+ errors? found$", err))
            return "refuses" if refusal else "throws", f"{stage} exit {code}\n{out}{err}"
    return "ok", out


def count_lines(stderr):
    return [int(n) for n in re.findall(r"^test sites the writer asked with no record: ([0-9]+)$", stderr, re.MULTILINE)]


def missing_records(directory, mode):
    total = 0
    for stage in (("upstream", "build") if mode == "split" else ("build",)):
        _, _, err = record(directory, stage)
        lines = count_lines(err)
        if len(lines) != 1:
            raise ValueError(f"{directory}/{stage}.err: expected one missing-record count, found {len(lines)}")
        total += int(lines[0])
    return total


def verdict(reference, actual):
    sc, value = reference
    status, observed = actual
    if sc == "refuses":
        return "not valid"
    if sc != "ok":
        raise ValueError(f"scalac {sc}: {value}")
    if status != "ok":
        return status
    return "as scalac" if observed == value else "wrong value"


def selftest(root):
    root.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(dir=root) as tmp:
        d = Path(tmp)

        rows = [("one", "Integer", "java.io.Serializable", "alone", "-", "jdk")]
        manifest = d / "cells.tsv"
        line = "\t".join(rows[0]) + "\n"
        manifest.write_text(line)
        (d / "binaries").write_text("base\n")

        def plant(directory, stage, code=0, value=""):
            directory.mkdir(parents=True, exist_ok=True)
            for suffix, data in (("exit", str(code)), ("out", value), ("err", "")):
                (directory / f"{stage}.{suffix}").write_text(data)

        for leg in ("scalac", *LEGS):
            directory = (d / "runs/scalac" if leg == "scalac" else d / "runs/base" / leg) / "one"
            if leg != "interp":
                plant(directory, "build")
            plant(directory, "run", value="hit\n")

        jvm = d / "runs/base/jvm/one"
        for build, run, value, want in ((0, 0, "hit\n", "as scalac"), (0, 0, "next\n", "wrong value"),
                                        (1, 0, "", "refuses"), (0, 1, "hit\n", "throws"), (0, 124, "", "timeout")):
            plant(jvm, "build", build)
            plant(jvm, "run", run, value)
            code = compare(d, rows)
            with (d / "table.tsv").open() as table:
                got = {r["leg"]: r["verdict"] for r in csv.DictReader(table, delimiter="\t")}
            if got != {"jvm": want, "js": "as scalac", "interp": "as scalac"} or code != int(want != "as scalac"):
                raise ValueError(f"selftest missed {want}")
        (jvm / "run.exit").unlink()
        try:
            compare(d, rows)
        except FileNotFoundError as exc:
            if Path(exc.filename) != jvm / "run.exit":
                raise
        else:
            raise ValueError("selftest missed missing status")
        for content, want in (("", "missing cell one"), (line * 2, "duplicated cell one"),
                              (line + line.replace("one\t", "extra\t", 1), "stray cell extra")):
            manifest.write_text(content)
            try:
                compare(d, rows)
            except ValueError as exc:
                if want not in str(exc).splitlines():
                    raise
            else:
                raise ValueError(f"selftest missed {want}")
        manifest.write_text(line)
        plant(jvm, "build")
        plant(jvm, "run", value="hit\n")
        (d / "binaries").write_text("base\nhead\n")
        for leg in LEGS:
            directory = d / "runs/head" / leg / "one"
            if leg != "interp":
                plant(directory, "build")
            plant(directory, "run", value="hit\n")
        measure = d / "runs/head/jvm/one/build.err"
        for count in (0, 2):
            measure.write_text(f"test sites the writer asked with no record: {count}\n")
            if compare(d, rows) != int(count != 0):
                raise ValueError(f"selftest missed count {count}")
        upstream = measure.with_name("upstream")
        plant(upstream.parent, upstream.name)
        upstream.with_suffix(".err").write_text("test sites the writer asked with no record: 3\n")
        if missing_records(measure.parent, "split") != 5:
            raise ValueError("selftest missed split counts")
        measure.write_text("")
        try:
            compare(d, rows)
        except ValueError as exc:
            if "expected one missing-record count" not in str(exc):
                raise
        else:
            raise ValueError("selftest missed absent count")
        sides = ["base", "head"]
        paired = [("bounded", TARGETS[0], "whole", seed) for seed in ("alone", "beside")]
        (d / "patterns.tsv").write_text("".join("\t".join((pattern_name(row), *row)) + "\n" for row in paired))
        zero = "test sites the writer asked with no record: 0\n"
        for row in paired:
            for side in ["scalac", *sides]:
                r = d / "runs/patterns" / side / pattern_name(row)
                plant(r, "build")
                (r / "build.err").write_text(zero)
                plant(r, "run", value="hit\n")
                (r / "classes").mkdir()
                for name in ("A", "A$"):
                    (r / "classes" / (name + ".class")).write_bytes(b"same")
        for i in range(4):
            r = d / "watch" / str(i)
            plant(r, "build")
            (r / "build.err").write_text(zero)
            plant(r, "run", value="hit\n")
            (r / "reply.json").write_text(json.dumps({"ok": True, "incremental": i > 0}))
            for side in ("fresh", "watch"):
                (r / (side + ".json")).write_text('{"A$.class":"same"}')
        (d / "watch/watch.err").write_text(zero * 4)
        if compare_patterns(d, sides, paired, []):
            raise ValueError("selftest rejected equal class files")
        moved = d / "runs/patterns/head" / pattern_name(paired[1]) / "classes/A$.class"
        moved.write_bytes(b"moved")
        if not compare_patterns(d, sides, paired, []):
            raise ValueError("selftest missed moved bytes")
        moved.write_bytes(b"same")
        (d / "watch/2/watch.json").write_text('{"A$.class":"moved"}')
        if not compare_patterns(d, sides, paired, []):
            raise ValueError("selftest missed watch mismatch")
    print("box-ancestry selftest: wrong value, refusal, throw, missing status, timeout, missing, duplicated and stray cells, nonzero and absent counts, moved bytes and watch mismatch found")


def judge_plant(root):
    rows = list(cells())
    errors = manifest_errors(read_manifest(root / "cells.tsv"), rows)
    if errors:
        raise ValueError("\n".join(errors))
    counts = Counter()
    failures = []

    def check(run, directory, mode, asks):
        n = missing_records(directory / run, mode)
        counts["asks"] += n
        counts[n > 0] += 1
        if (n > 0) != asks:
            failures.append(run)

    for row in rows:
        _, _, target, condition, _, _ = row
        asks = target != "java.lang.Object" or condition == "after_test"
        for run, mode, *_ in jvm_runs(row):
            check(run, root / "runs/base/jvm", mode, asks)
    for row in pattern_cells():
        shape, _, mode, seed = row
        # An extractor's Option element and the AnyVal parameter arrive as references.
        asks = shape not in VALUE_PATTERNS and shape != "value_class" and (
            seed == "beside" or shape not in ("extractor", "anyval"))
        check(pattern_name(row), root / "runs/patterns/base", mode, asks)
    print(f"plant: {counts[True]} cells above zero, {counts[False]} at zero, {counts['asks']} missing asks, "
          f"{len(failures)} fence failures")
    for run in failures:
        print(run)
    return int(bool(failures))


def judge(root):
    selftest(root / "selftest")
    status = compare(root, list(cells()))
    lines = []
    sides = (root / "binaries").read_text().splitlines()
    if compare_patterns(root, sides, list(pattern_cells()), lines):
        status = 1
    with (root / "summary.txt").open("a") as summary:
        print(*lines, sep="\n", file=summary)
    print((root / "summary.txt").read_text(), end="")
    return status


def compare(root, expected):
    rows = read_manifest(root / "cells.tsv")
    sides = (root / "binaries").read_text().splitlines()
    if sides not in (["base"], ["base", "head"]):
        raise ValueError("expected base and optional head in binaries")
    errors = manifest_errors(rows, expected)
    names = {row[0] for row in rows}
    paths = {"scalac": root / "runs/scalac"}
    paths.update({(side, leg): root / "runs" / side / leg for side in sides for leg in LEGS})
    split_names = {r[0] for r in rows if r[3].startswith("inline_")}
    if split_names:
        paths["scalac-split"] = root / "runs/scalac-split"
    jvm_names = {run for row in rows for run, *_ in jvm_runs(row)}
    for key, directory in paths.items():
        wanted = split_names if key == "scalac-split" else jvm_names if isinstance(key, tuple) and key[1] == "jvm" else names
        found = {p.name for p in directory.iterdir()}
        errors.extend(f"missing cell {directory / n}" for n in sorted(wanted - found))
        errors.extend(f"stray cell {directory / n}" for n in sorted(found - wanted))
    if errors:
        raise ValueError("\n".join(errors))

    counts = Counter()
    right = {}
    missing = Counter()
    placeholders = {}
    changed_placeholders = []
    with (root / "table.tsv").open("w") as table:
        table.write("\t".join((*FIELDS[:5], "binary", "leg", "mode", "threads", "seed", "verdict", "scalac", "teq", "missing")) + "\n")
        for row in rows:
            name, box, target, condition, conformance, _ = row
            references = {"whole": result(paths["scalac"] / name, "scalac")}
            if name in split_names:
                references["split"] = result(paths["scalac-split"] / name, "scalac", True)
            for side in sides:
                for leg in LEGS:
                    runs = jvm_runs(row) if leg == "jvm" else [(name, "whole", "-", "-")]
                    for run, mode, threads, seed in runs:
                        directory = paths[side, leg] / run
                        actual = result(directory, leg, mode == "split")
                        if target == CONSTABLE and mode == "split":
                            if side == "base":
                                placeholders[run] = actual
                            elif actual != placeholders[run]:
                                changed_placeholders.append(run)
                        reference = references[mode]
                        v = verdict(reference, actual)
                        right[side, leg, run] = v == "as scalac"
                        for group in (leg, box, target, condition):
                            counts[side, group, v] += 1
                        if name in split_names and leg == "jvm":
                            counts[side, mode, v] += 1
                        n = "-"
                        if side == "head" and leg == "jvm":
                            n = missing_records(directory, mode)
                            missing[mode] += n
                        table.write("\t".join((run, box, target, condition, conformance, side, leg, mode, threads, seed, v,
                                               json.dumps(reference[1]), json.dumps(actual[1]), str(n))) + "\n")

    lines = [f"{len(rows)} programs, {len(jvm_names)} JVM cells", "\t".join(("binary", "group", *VERDICTS))]
    for side in sides:
        for groups in (LEGS, BOXES, sorted({row[2] for row in rows}), sorted({row[3] for row in rows}), ("whole", "split")):
            for group in groups:
                lines.append("\t".join((side, group, *(str(counts[side, group, v]) for v in VERDICTS))))
    if "head" in sides:
        for leg in LEGS:
            lost = sorted(n for n in (jvm_names if leg == "jvm" else names) if right["base", leg, n] and not right["head", leg, n])
            lines.append(f"{leg}: cells right on the base and not right on the head: {len(lost)}")
            lines.extend(lost)
        lines.append(f"split Constable cells changed from the base: {len(changed_placeholders)} of {len(placeholders)}")
        lines.extend(changed_placeholders)
        lines.append(f"test sites the writer asked with no record: {sum(missing.values())}")
    summary = "\n".join(lines) + "\n"
    (root / "summary.txt").write_text(summary)
    return int(not all(right.values()) or any(missing.values()) or bool(changed_placeholders))


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "watch":
        sys.exit(watch(Path(sys.argv[2]), sys.argv[3], sys.argv[4]))
    if len(sys.argv) != 3 or sys.argv[1] not in ("gen", "judge", "plant", "selftest"):
        sys.exit("usage: box_ancestry.py gen|judge|plant|selftest <dir>")
    try:
        sys.exit({"gen": gen, "judge": judge, "plant": judge_plant, "selftest": selftest}[sys.argv[1]](Path(sys.argv[2])))
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"box-ancestry: {error}", file=sys.stderr)
        sys.exit(2)
