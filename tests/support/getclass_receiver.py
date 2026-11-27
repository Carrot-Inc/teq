"""Generate and judge the getClass receiver table. See getclass-receiver.README.md."""
import csv
import json
import re
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import NamedTuple

VALUES = dict(Int="1", Byte="(1: Byte)", Short="(1: Short)", Char="'c'", Boolean="true",
              Long="1L", Float="1f", Double="1d", Unit="()")
SUPERS = dict(Number="java.lang.Number", Comparable="java.lang.Comparable[?]",
              Serializable="java.io.Serializable")
FENCES = dict(value_class="Value", value_class_generic="Value", value_class_boxed="Value",
              string="String", array="Array", module="Module", case_class="Case")
LEGS = ("jvm", "jvm-split", "js", "interp")
VERDICTS = ("as scalac", "refuses", "throws", "wrong value", "timeout", "not valid")


class Cell(NamedTuple):
    name: str
    kind: str
    primitive: str
    place: str
    call: str
    split: str
    js_repr: str


def kinds(p):
    yield from ("plain", "primitive_ref", "ref_primitive", "matchable", "anyval", "alias",
                "bound", "singleton", "opaque_outside", "opaque_inside", "inline_generic",
                "inline_any", "inline_arg_val", "inline_arg_literal", "inline_primitive",
                "generic", "effect", "effect_boxed")
    if p in ("Char", "Boolean", "Long"):
        yield "literal"
    for sup in SUPERS:
        if (sup == "Number" and p in ("Char", "Boolean", "Unit")) or (sup == "Comparable" and p == "Unit"):
            continue
        yield "binder_" + sup
        yield "intersection_" + sup


def cells():
    for p in [*VALUES, *dict.fromkeys(FENCES.values())]:
        for kind in kinds(p) if p in VALUES else (k for k, v in FENCES.items() if v == p):
            places = ["local"]
            if kind == "plain" and p in ("Int", "Char", "Double", "Unit"):
                places += ["parameter", "field", "default"]
            for place in places:
                for call in ("direct", "explicit", "saved") if p in ("Int", "Unit") else ("direct",):
                    split = (kind.startswith(("inline_", "opaque_", "value_class")) or kind == "alias"
                             or (kind, p, place, call) == ("plain", "Int", "local", "direct"))
                    boxed = (kind in ("primitive_ref", "ref_primitive", "opaque_outside", "generic", "effect_boxed")
                             or kind.startswith(("binder_", "intersection_")))
                    js = boxed and p in ("Byte", "Short", "Float", "Double", "Char")
                    yield Cell(f"gc_{p}_{kind}_{place}_{call}", kind, p, place, call,
                               "yes" if split else "no", "yes" if js else "no")


def source(cell):
    _, kind, p, place, call, split, _ = cell
    value = VALUES.get(p)

    def name(r):
        if call == "explicit":
            return f"{r}.getClass[Any].getName"
        if call == "saved":
            return f"{{ val c = {r}.getClass; c.getName }}"
        return f"{r}.getClass.getName"

    upstream = ""
    member = ""
    prefix = f"val p: {p} = {value}\n" if value else ""
    result = name("p")
    if kind == "plain":
        if split == "yes":
            upstream = f"object Lib {{ inline def initial: {p} = {value} }}"
            prefix = f"val p: {p} = Lib.initial\n"
        if place == "parameter":
            member = f"def read(p: {p}): String = {result}"
            result, prefix = f"read({value})", ""
        elif place == "field":
            member, prefix = prefix.strip(), ""
        elif place == "default":
            member = f"def read(n: String = {{ {prefix}{result} }}): String = n"
            result, prefix = "read()", ""
    elif kind == "literal":
        result, prefix = name(value), ""
    elif kind in ("primitive_ref", "ref_primitive", "matchable", "anyval"):
        r = {"primitive_ref": f"p.asInstanceOf[{p} & AnyRef]", "ref_primitive": f"p.asInstanceOf[AnyRef & {p}]",
             "matchable": f"(p: {p} & Matchable)", "anyval": f"(p: {p} & AnyVal)"}[kind]
        result = name(r)
    elif kind == "alias":
        upstream = f"object Lib {{ type A = {p} }}"
        prefix = f"val p: Lib.A = {value}\n"
    elif kind == "singleton":
        prefix += "val s: p.type = p\n"
        result = name("s")
    elif kind.startswith("opaque_"):
        inside = kind == "opaque_inside"
        method = f"\n  def read(o: O): String = {name('o')}" if inside else ""
        upstream = f"object Lib {{\n  opaque type O = {p}\n  def make(p: {p}): O = p{method}\n}}"
        result = "Lib.read(Lib.make(p))" if inside else name("Lib.make(p)")
    elif kind in ("bound", "generic") or kind.startswith("inline_"):
        sig = {"bound": f"def read[T <: {p}](t: T)", "generic": "def read[T](t: T)",
               "inline_generic": "inline def read[T](t: T)", "inline_any": "inline def read(t: Any)",
               "inline_arg_literal": "inline def read(inline t: Any)", "inline_arg_val": "inline def read(inline t: Any)",
               "inline_primitive": f"inline def read(t: {p})"}[kind]
        member = f"{sig}: String = {name('t')}"
        if split == "yes":
            upstream, member = f"object Lib {{ {member} }}", ""
        arg = value if kind == "inline_arg_literal" else "p"
        result = ("Lib." if upstream else "") + f"read({arg})"
    elif kind.startswith(("binder_", "intersection_")):
        form, sup = kind.split("_")
        ty = SUPERS[sup]
        if form == "binder":
            result = f'(p: @unchecked) match {{\n  case b: {ty} => {name("b")}\n  case _ => "next"\n}}'
        else:
            prefix += f"val b: {p} & {ty} = p.asInstanceOf[{p} & {ty}]\n"
            result = name("b")
    elif kind.startswith("effect"):
        r = '{ println("e"); p }'
        if kind == "effect_boxed":
            r += ".asInstanceOf[AnyRef]"
        result = name(r)
    elif kind.startswith("value_class"):
        upstream = "class V(val i: Int) extends AnyVal"
        result = name("(new V(1))")
        if kind == "value_class_generic":
            member = f"def read[T](t: T): String = {name('t')}"
            result = "read(new V(1))"
        elif kind == "value_class_boxed":
            result = name("(new V(1)).asInstanceOf[AnyRef]")
    elif kind in FENCES:
        if kind == "module":
            member = "object M"
        elif kind == "case_class":
            member = "case class C(i: Int)"
        result = name(dict(string='"s"', array="Array[Int](1)", module="M", case_class="C(1)")[kind])
    else:
        raise ValueError(kind)
    body = prefix + f"println({result})"
    main = ("object Main {\n" + ("  " + member + "\n" if member else "") +
            "  def main(args: Array[String]): Unit = try {\n" +
            "\n".join("    " + line for line in body.splitlines()) + "\n  } catch {\n" +
            '    case ex: Throwable => System.err.println(ex.toString); println("throw")\n  }\n}\n')
    return upstream, main


def write_tsv(path, header, rows):
    with path.open("w", newline="") as stream:
        writer = csv.writer(stream, delimiter="\t", lineterminator="\n")
        writer.writerow(header)
        writer.writerows(rows)


def gen(d):
    manifest = list(cells())
    for cell in manifest:
        folder = d / cell.name
        folder.mkdir(parents=True, exist_ok=True)
        package = f"package {cell.name}\n\n"
        upstream, main = source(cell)
        (folder / "Main.scala").write_text(package + (upstream + "\n\n" if upstream else "") + main)
        if cell.split == "yes":
            (folder / "split").mkdir(exist_ok=True)
            (folder / "split/Lib.scala").write_text(package + upstream + "\n")
            (folder / "split/Main.scala").write_text(package + main)
    write_tsv(d / "cells.tsv", Cell._fields, manifest)
    print(f"getclass-receiver: {len(manifest)} cells")


def batch(d, attempt):
    folder = d / "batches" / attempt
    pending = (folder / "cells").read_text().splitlines()
    status = (folder / "build.exit").read_text().strip()
    errors = {}
    if status != "0":
        for line in (folder / "build.err").read_text().splitlines():
            match = re.match(r"^-- .*Error: (?:.*?/)?(\w+)/Main\.scala:\d+:\d+", line)
            if match and match[1] in pending:
                errors.setdefault(match[1], line)
        if not errors or status == "124":
            sys.exit(f"scalac batch {attempt} failed (exit {status}), see {folder}/build.err")
    for cell in errors if errors else pending:
        r = d / "runs/scalac" / cell
        r.mkdir(parents=True)
        for ext in ("exit", "out", "err"):
            (r / f"build.{ext}").symlink_to(folder / f"build.{ext}")
        if cell in errors:
            (r / "diagnostic").write_text(errors[cell] + "\n")
    remaining = [c for c in pending if errors and c not in errors]
    (folder / "remaining").write_text("".join(c + "\n" for c in remaining))
    print(f"scalac batch {attempt}: {len(errors)} refused, {len(remaining)} to retry")


class Observation(NamedTuple):
    status: str
    output: str
    detail: str = ""


def observe(path, leg):
    steps = {"scalac": ("build", "run"), "jvm": ("build", "run"), "jvm-split": ("upstream", "build", "run"),
             "js": ("build", "run"), "interp": ("run",)}
    for stage in steps[leg]:
        try:
            rc = int((path / f"{stage}.exit").read_text())
            out = (path / f"{stage}.out").read_bytes().decode()
            err = (path / f"{stage}.err").read_text()
        except (OSError, ValueError) as e:
            return Observation("not valid", "", str(e))
        if rc == 124 or (rc == 137 and "timeout: sending signal" in err):
            return Observation("timeout", out, stage)
        caught = stage == "run" and out.splitlines()[-1:] == ["throw"]
        if rc or caught:
            lines = (err.strip() or out.strip()).splitlines()
            detail = next((s for s in lines if s.strip()), f"{stage} exit {rc}")
            error = next((s for s in lines if re.search(r"(?:^|: )error:", s)), "")
            if stage != "run" or (leg == "interp" and rc and error):
                detail = error or detail
                if (path / "diagnostic").exists():
                    detail = (path / "diagnostic").read_text().strip()
                return Observation("refuses", out, detail)
            if leg == "js" and not caught:
                detail = next((s for s in lines if re.match(r"^[\w.$]*(?:Error|Exception):", s)), detail)
            return Observation("throws", out, detail)
    return Observation("value", out)


class Row(NamedTuple):
    cell: Cell
    binary: str
    leg: str
    oracle: str
    verdict: str
    scalac: str
    expected: str
    actual: str
    detail: str


def compare(cell, side, leg, ref, base, actual, invalid=""):
    marked = leg == "js" and cell.js_repr == "yes"
    oracle = "base JS" if marked else "scalac"
    expected = base if marked else ref
    detail = actual.detail
    if invalid:
        verdict, detail = "not valid", invalid
    elif ref.status not in ("value", "throws"):
        verdict, detail = "not valid", f"scalac {ref.status}: {ref.detail}"
    elif marked and base.status != "value":
        if actual.status != "value":
            verdict = actual.status
        else:
            verdict, detail = "not valid", "missing successful base JS output"
    elif actual.status == expected.status and actual.output == expected.output:
        verdict = "as scalac"
    elif actual.status == "value":
        verdict = "wrong value"
    else:
        verdict = actual.status
    return Row(cell, side, leg, oracle, verdict, ref.output, expected.output, actual.output, detail)


def collect(d, manifest):
    with (d / "cells.tsv").open(newline="") as stream:
        reader = csv.reader(stream, delimiter="\t")
        if next(reader) != list(Cell._fields):
            raise ValueError("invalid cells.tsv header")
        found = [Cell(*row) for row in reader]
    counts = Counter(c.name for c in found)
    by_name = {c.name: c for c in found}
    expected = {c.name: c for c in manifest}
    sides = (d / "binaries").read_text().splitlines()
    if sides not in (["base"], ["base", "head"]):
        raise ValueError("invalid binaries file")
    problems = {}
    for name in expected.keys() | by_name.keys():
        if name not in expected:
            problems[name] = "stray cell"
        elif counts[name] == 0:
            problems[name] = "missing cell"
        elif counts[name] > 1:
            problems[name] = "duplicated cell"
        elif expected[name] != by_name[name]:
            problems[name] = "incorrect axes"

    runs = d / "runs"
    rows = []
    for name, cell in (by_name | expected).items():
        ref = observe(runs / "scalac" / name, "scalac")
        for leg in LEGS:
            if leg == "jvm-split" and cell.split == "no":
                continue
            base = observe(runs / "base" / leg / name, leg)
            for side in sides:
                actual = base if side == "base" else observe(runs / side / leg / name, leg)
                rows.append(compare(cell, side, leg, ref, base, actual, problems.get(name, "")))
    for side in ["scalac", *sides]:
        for leg in (["scalac"] if side == "scalac" else LEGS):
            path = runs / side if side == "scalac" else runs / side / leg
            for folder in path.glob("*/"):
                cell = expected.get(folder.name)
                if (cell is None and folder.name not in by_name) or (cell and leg == "jvm-split" and cell.split == "no"):
                    stray = cell or Cell(folder.name, "unknown", "unknown", "unknown", "unknown", "no", "no")
                    targets = ("jvm", "js", "interp") if side == "scalac" else (leg,)
                    for target in targets:
                        for binary in sides if side == "scalac" else (side,):
                            rows.append(Row(stray, binary, target, "scalac", "not valid", "", "", "", "stray run"))
    return rows


def summarize(rows, sides):
    lines = []
    for side in sides:
        records = [r for r in rows if r.binary == side]
        columns = (*LEGS[:2], "js", "js repr", "interp")
        present = {r.verdict for r in records}
        verdicts = [v for v in VERDICTS if v in present]
        lines += [f"{side}: counts in order " + " / ".join(verdicts), "axis\tcells\t" + "\t".join(columns)]
        for axis in (None, "kind", "primitive", "place", "call"):
            groups = {}
            for r in records:
                key = getattr(r.cell, axis) if axis else "all"
                groups.setdefault(key, []).append(r)
            if axis:
                lines.append(axis)
            for key, group in groups.items():
                counts = Counter(("js repr" if r.oracle == "base JS" else r.leg, r.verdict) for r in group)
                values = ["/".join(str(counts[col, v]) for v in verdicts) for col in columns]
                lines.append("\t".join([key, str(len({r.cell.name for r in group})), *values]))
    regressions = []
    if "head" in sides:
        right = {(r.cell.name, r.leg) for r in rows if r.binary == "base" and r.verdict == "as scalac"}
        for leg in LEGS:
            lost = [r for r in rows if r.binary == "head" and r.leg == leg and r.verdict != "as scalac"
                    and (r.cell.name, leg) in right]
            lines.append(f"{leg}: cells right on the base and not right on the head: {len(lost)}")
            lines.extend(f"  {r.cell.name}: {r.verdict}" for r in lost)
            regressions.extend(lost)
    bad = sum(r.binary == "base" and r.verdict != "as scalac" for r in rows)
    lines.append(f"getclass-receiver: {bad} base cell/leg differences")
    return "\n".join(lines) + "\n", bool(bad or regressions)


def judge(d):
    with tempfile.TemporaryDirectory(dir=d) as scratch:
        selftest(Path(scratch))
    rows = collect(d, list(cells()))
    write_tsv(d / "table.tsv", (*Cell._fields, *Row._fields[1:]), (
        (*r.cell, r.binary, r.leg, r.oracle, r.verdict, *(json.dumps(s) for s in r[5:8]), r.detail) for r in rows))
    summary, failed = summarize(rows, (d / "binaries").read_text().splitlines())
    (d / "summary.txt").write_text(summary)
    print(summary, end="")
    return int(failed)


def selftest(d):
    d.mkdir(parents=True, exist_ok=True)
    names = ("ok", "wrong", "refusal", "throwing", "no_status", "slow", "missing", "duplicate", "invalid", "repr")
    manifest = [Cell(n, "plain", "Int", "local", "direct", "yes" if n == "ok" else "no",
                     "yes" if n == "repr" else "no") for n in names]
    write_tsv(d / "cells.tsv", Cell._fields, [c for c in manifest if c.name != "missing"] +
              [manifest[names.index("duplicate")], manifest[0]._replace(name="stray")])
    (d / "binaries").write_text("base\nhead\n")

    def plant(directory, stage, output="", status=0, diagnostic=""):
        directory.mkdir(parents=True, exist_ok=True)
        (directory / f"{stage}.out").write_text(output)
        (directory / f"{stage}.err").write_text(diagnostic)
        exit_file = directory / f"{stage}.exit"
        if status is None:
            exit_file.unlink(missing_ok=True)
        else:
            exit_file.write_text(str(status) + "\n")

    for n in names:
        ref = d / "runs/scalac" / n
        plant(ref, "build", status=int(n == "invalid"), diagnostic="Error: bad source")
        plant(ref, "run", "reference\n")
        for leg in LEGS:
            if leg == "jvm-split" and n != "ok":
                continue
            for side in ("base", "head"):
                path = d / "runs" / side / leg / n
                if leg == "jvm-split":
                    plant(path, "upstream")
                if leg != "interp":
                    plant(path, "build", status=int(n == "refusal"), diagnostic="error: refused\nmore")
                status = {"slow": 124, "no_status": None, "throwing": 1, "refusal": 1}.get(n, 0)
                out = "different\n" if n == "wrong" or n == "repr" and leg == "js" and side == "base" else "reference\n"
                if (side, leg) == ("head", "jvm-split"):
                    out = "different\n"
                plant(path, "run", out, status, "error: refused" if n == "refusal" else "Exception: thrown\ntrace")

    rows = collect(d, manifest)
    want = dict(ok="as scalac", wrong="wrong value", refusal="refuses", throwing="throws", slow="timeout",
                no_status="not valid", missing="not valid", duplicate="not valid", stray="not valid", invalid="not valid")
    details = {"refusal": "error: refused", "throwing": "Exception: thrown", "slow": "run",
               "duplicate": "duplicated cell", "missing": "missing cell", "stray": "stray cell",
               "invalid": "scalac refuses: Error: bad source"}
    for r in rows:
        verdict = want.get(r.cell.name, "wrong value" if (r.binary, r.leg) == ("head", "js") else "as scalac")
        if (r.cell.name, r.binary, r.leg) == ("ok", "head", "jvm-split"):
            verdict = "wrong value"
        if r.verdict != verdict:
            raise ValueError(f"selftest expected {verdict}: {r}")
        if r.cell.name in details and r.detail != details[r.cell.name]:
            raise ValueError(f"selftest diagnostic: {r}")
        if r.cell.name == "wrong" and (r.expected, r.actual) != ("reference\n", "different\n"):
            raise ValueError(f"selftest outputs: {r}")
    if {r.cell.name for r in rows} != set(want) | {"repr"}:
        raise ValueError("selftest cell inventory")
    summary, failed = summarize(rows, ["base", "head"])
    if not failed:
        raise ValueError("selftest expected a failing table")
    for leg, loss in (("jvm", "0"), ("jvm-split", "1\n  ok: wrong value"),
                      ("js", "1\n  repr: wrong value"), ("interp", "0")):
        if f"{leg}: cells right on the base and not right on the head: {loss}\n" not in summary:
            raise ValueError(f"selftest fence: {leg}")
    print("getclass-receiver selftest: wrong value, refusal, throw, missing status, timeout, missing, duplicate,"
          " stray and scalac-refused cells found; base JS fence passed")


if __name__ == "__main__":
    verb, directory = sys.argv[1:3]
    path = Path(directory).resolve()
    if verb == "gen":
        gen(path)
    elif verb == "batch":
        batch(path, sys.argv[3])
    elif verb == "selftest":
        selftest(path)
    elif verb == "judge":
        sys.exit(judge(path))
    else:
        sys.exit("usage: getclass_receiver.py gen|judge|selftest <dir>")
