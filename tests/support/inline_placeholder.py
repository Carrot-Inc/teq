"""Generate the imported inline body probes and judge records from inline-placeholder.sh."""
import sys
from collections import Counter, defaultdict
from pathlib import Path
from tempfile import TemporaryDirectory

FORMS = ("isInstanceOf", "pattern", "ClassTag", "unapply", "cast", "classOf", "catch")
LEGS = ("jvm-whole", "jvm-split", "js-whole", "interp-whole")
VERDICTS = ("as scalac", "refuses", "throws", "wrong value", "timeout", "not valid")
# The positive value is constructed upstream when constructing it would reveal the tested class.
TYPES = (
    ("unmodelled", "java.lang.constant.Constable", '"s"'),
    ("unmodelled", "java.lang.constant.ConstantDesc", '"s"'),
    ("modelled", "java.io.Serializable", '"s"'),
    ("modelled", "java.util.RandomAccess", "new java.util.ArrayList[String]()"),
    ("jar", "scala.collection.immutable.ArraySeq[?]", "scala.collection.immutable.ArraySeq(1)"),
    ("module", "LibValue", "new LibValue"),
)
ERRORS = (
    ("unmodelled", "java.util.concurrent.TimeoutException", 'new java.util.concurrent.TimeoutException("x")'),
    ("modelled", "java.io.IOException", 'new java.io.IOException("x")'),
    ("jar", "scala.MatchError", 'new scala.MatchError("x")'),
    ("module", "LibError", 'new LibError("x")'),
)


def cells():
    for form in FORMS:
        for origin, typ, good in (ERRORS if form == "catch" else TYPES):
            short = typ.split(".")[-1].split("[")[0]
            for mention in ("unnamed", "named"):
                yield f"p{short}_{form}_{mention}", origin, typ, good, form, mention


def sources(name, origin, typ, good, form, mention):
    extra = ""
    arg = "Any"
    miss = "new Object"
    defs = ""
    match = 'x match { case b: %s => "hit"; case _ => "next" }'
    if origin == "module":
        extra = 'class LibError(m: String) extends Exception(m)' if form == "catch" else "class LibValue"
    if form == "catch":
        arg = "Throwable"
        miss = 'new RuntimeException("miss")'
        body = f'try {{ throw x }} catch {{ case e: {typ} => "hit"; case _: Throwable => "next" }}'
    elif form == "isInstanceOf":
        body = f'x.isInstanceOf[{typ}]'
    elif form == "pattern":
        body = match % typ
    elif form == "ClassTag":
        defs = "  inline def tagged[T](x: Any)(using scala.reflect.ClassTag[T]): String = " + match % "T" + "\n"
        body = f"tagged[{typ}](x)"
    elif form == "unapply":
        extra += f"\nobject Extract {{ def unapply(x: {typ}): Boolean = true }}"
        body = 'x match { case Extract() => "hit"; case _ => "next" }'
    elif form == "cast":
        body = (f'try {{ if (x.asInstanceOf[{typ}] == null) "null" else "hit" }} '
                'catch { case _: ClassCastException => "next" }')
    else:
        body = f"classOf[{typ}].getName"
    if good != '"s"':
        defs += f"  def instance: {arg} = {good}\n"
        good = "Lib.instance"
    result = "Boolean" if form == "isInstanceOf" else "String"
    lib = f"package {name}\n{extra}\nobject Lib {{\n{defs}  inline def t(x: {arg}): {result} = {body}\n}}\n"
    witness = f"  def mention(x: {typ}): Unit = ()\n" if mention == "named" else ""
    main = (f"package {name}\nobject Main {{\n{witness}  def main(args: Array[String]): Unit = {{\n"
            f'    val a = Lib.t({good})\n    val b = Lib.t({miss})\n    println(a.toString + " " + b.toString)\n  }}\n}}\n')
    return lib, main


def gen(root):
    with (root / "cells.tsv").open("w") as rows:
        for cell in cells():
            name, origin, typ, _, form, mention = cell
            src = root / name
            src.mkdir()
            lib, main = sources(*cell)
            (src / "Lib.scala").write_text(lib)
            (src / "Main.scala").write_text(main)
            print(name, origin, typ, form, mention, sep="\t", file=rows)
    print(f"inline-placeholder: {len(list(cells()))} source cells")


def read(directory, phases):
    for phase in phases:
        path = directory / phase
        try:
            code = int(path.with_suffix(".exit").read_text())
            out = path.with_suffix(".out").read_bytes()
            err = path.with_suffix(".err").read_bytes()
        except (OSError, ValueError):
            return "not valid", f"missing or invalid {phase} record"
        if not 0 <= code <= 255:
            return "not valid", f"invalid {phase} exit {code}"
        if code == 124 or (code == 137 and b"timeout:" in err):
            return "timeout", f"{phase} exit {code}, stdout {out!r}"
        if code:
            diagnostic = any(line.startswith(b"error:") or b": error:" in line for line in err.splitlines())
            refuses = phase in ("build", "upstream") or (phase == "interp" and diagnostic)
            return ("refuses" if refuses else "throws"), f"{phase} exit {code}, stdout {out!r}"
    return "ok", out


def verdict(oracle, actual):
    if oracle[0] != "ok" or len(oracle[1].splitlines()) != 1 or len(oracle[1].split()) != 2:
        return "not valid"
    if actual[0] != "ok":
        return actual[0]
    return "as scalac" if actual[1] == oracle[1] else "wrong value"


def phases(leg):
    if leg == "interp-whole":
        return ("interp",)
    if leg == "jvm-split":
        return ("upstream", "build", "run")
    return ("build", "run")


def inventory(root, names, sides):
    problems = []
    manifest = root / "cells.tsv"
    if not manifest.is_file():
        return {}, ["missing cells.tsv"]
    listed = Counter(row.split("\t")[0] for row in manifest.read_text().splitlines())
    for name in sorted(names - listed.keys()):
        problems.append(f"missing manifest cell {name}")
    for name, count in listed.items():
        if name not in names:
            problems.append(f"stray manifest cell {name}")
        if count > 1:
            problems.append(f"duplicate manifest cell {name}")

    runs = root / "runs"
    records = {}
    for side in ("scalac", *sides):
        for leg in (("jvm-whole", "jvm-split") if side == "scalac" else LEGS):
            for name in sorted(names):
                cell = Path(side, leg, name)
                records[cell] = read(runs / cell, phases(leg))
                status, detail = records[cell]
                if status == "not valid":
                    problems.append(f"{cell}: {detail}")
    for directory in sorted(runs.glob("*/*/*")):
        cell = directory.relative_to(runs)
        if directory.is_dir() and cell not in records:
            problems.append(f"stray run cell {cell}")
    return records, problems


def judge(root, sides, probes=None):
    if probes is None:
        probes = list(cells())
    records, problems = inventory(root, {c[0] for c in probes}, sides)
    if problems:
        for problem in problems:
            print(f"inline-placeholder inventory: {problem}")
        return 1
    counts = defaultdict(Counter)
    totals = defaultdict(Counter)
    fences = defaultdict(Counter)
    failed = False
    for name, origin, _, _, form, mention in probes:
        for leg in LEGS:
            refleg = "jvm-split" if leg == "jvm-split" else "jvm-whole"
            ref = records[Path("scalac", refleg, name)]
            results = []
            for side in sides:
                actual = records[Path(side, leg, name)]
                v = verdict(ref, actual)
                results.append(v)
                totals[side][v] += 1
                for axis, field in (("origin", origin), ("form", form), ("caller", mention), ("build", leg)):
                    counts[side, axis, field][v] += 1
                if v != "as scalac":
                    print(f"{side} {leg} {name}: {v}; scalac {ref[1]!r}, {side} {actual[1]!r}")
                if v == "not valid" or (side == sides[-1] and v != "as scalac"):
                    failed = True
            if len(results) == 2:
                before, after = results
                fences[leg][before, after] += 1
                if before != after:
                    print(f"fence {leg} {name}: {before} -> {after}")
    for (side, axis, field), count in sorted(counts.items()):
        print(side, axis, field, *(f"{v}={count[v]}" for v in VERDICTS), sep="\t")
    for leg, changes in fences.items():
        for (before, after), n in sorted(changes.items()):
            print(f"fence {leg}: {before} -> {after}: {n}")
    for side, count in totals.items():
        print(f"inline-placeholder {side}: {sum(count.values())} legs; " + ", ".join(f"{count[v]} {v}" for v in VERDICTS))
    return int(failed)


def selftest():
    from contextlib import redirect_stdout
    from io import StringIO

    with TemporaryDirectory(dir=sys.argv[2] if len(sys.argv) > 2 else None) as scratch:
        root = Path(scratch)

        def put(phase, code=0, stdout=b"", stderr=b"", directory=root):
            (directory / (phase + ".exit")).write_text(str(code))
            (directory / (phase + ".out")).write_bytes(stdout)
            (directory / (phase + ".err")).write_bytes(stderr)

        ref = ("ok", b"a b\n")
        for phase in phases("jvm-split"):
            put(phase, stdout=ref[1])
        for phase, code, stdout, expected in (("run", 0, ref[1], "as scalac"), ("run", 0, b"a a\n", "wrong value"),
                                               ("run", 1, ref[1], "throws"), ("build", 124, b"", "timeout"),
                                               ("upstream", 1, b"", "refuses")):
            put(phase, code, stdout)
            assert verdict(ref, read(root, phases("jvm-split"))) == expected
        assert verdict(("refuses", "build exit 1"), ref) == "not valid"
        assert verdict(("ok", b""), ("ok", b"")) == "not valid"
        assert verdict(ref, read(root, ("interp",))) == "not valid"
        put("interp", 1, stderr=b"Main.scala:1:3: error: unknown type")
        assert verdict(ref, read(root, ("interp",))) == "refuses"
        (root / "interp.exit").write_text("")
        assert verdict(ref, read(root, ("interp",))) == "not valid"

        manifest = root / "cells.tsv"
        manifest.write_text("probe\n")
        for side, legs in (("base", LEGS), ("scalac", ("jvm-whole", "jvm-split"))):
            for leg in legs:
                directory = root / "runs" / side / leg / "probe"
                directory.mkdir(parents=True)
                for phase in phases(leg):
                    put(phase, stdout=ref[1], directory=directory)

        def judge_probe():
            out = StringIO()
            with redirect_stdout(out):
                status = judge(root, ["base"], [("probe", *next(cells())[1:])])
            return status, out.getvalue()

        assert judge_probe()[0] == 0
        put("build", 1, directory=root / "runs/scalac/jvm-split/probe")
        status, report = judge_probe()
        assert status == 1 and "base jvm-split probe: not valid;" in report

        manifest.write_text("probe\nprobe\n")
        (root / "runs/base/js-whole/stray").mkdir()
        (root / "runs/base/interp-whole/probe").rename(root / "missing")
        status, report = judge_probe()
        assert status == 1 and set(report.splitlines()) == {
            "inline-placeholder inventory: duplicate manifest cell probe",
            "inline-placeholder inventory: stray run cell base/js-whole/stray",
            "inline-placeholder inventory: base/interp-whole/probe: missing or invalid interp record",
        }
    print("inline-placeholder selftest: all six verdicts, missing record, duplicate cell, stray cell and scalac refusal found")


if __name__ == "__main__":
    if sys.argv[1] == "selftest":
        selftest()
    elif sys.argv[1] == "gen":
        gen(Path(sys.argv[2]))
    elif sys.argv[1] == "judge":
        sys.exit(judge(Path(sys.argv[2]), sys.argv[3:]))
    else:
        sys.exit("usage: inline_placeholder.py selftest [scratch dir] | gen <dir> | judge <dir> <base> [head]")
