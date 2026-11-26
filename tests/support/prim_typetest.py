"""The type patterns of a primitive scrutinee, as cells: each primitive (Int, Long, Short, Byte, Char, Boolean, Double,
Float, Unit) against a Scala class, a Scala trait, a value class (which its box is never an instance of), the box's own
supertypes (Number for the numbers, Comparable, Serializable), Any, AnyVal, Matchable, and a union with the primitive.
Each cell is a program of its own: `(x: @unchecked) match { case b: T => <b used at T>; case _ => println("next") }`.

  python3 tests/support/prim_typetest.py gen <dir>
  python3 tests/support/prim_typetest.py judge <dir>
  python3 tests/support/prim_typetest.py selftest <dir>

`gen` writes the cells and cells.tsv. tests/support/prim-typetest.sh builds them by scalac 3.8.4 and by teq for the
JVM, runs each under -Xverify:all, and writes <dir>/runs/<side>/<cell>.out, .err and .exit. `judge` holds teq's run of
each cell to scalac's, by output AND exit status: it prints a line per cell that differs, a count, and exits 1 when
any cell differs or misses a run. `selftest` plants a wrong output, a verify failure, a trailing failure, a missing
cell and a missing exit, and fails unless the judge finds each."""
import os
import sys

PRIMS = [  # name, a value, the box's own supertypes beside Comparable and Serializable
    ("Int", "1", ["java.lang.Number"]), ("Long", "1L", ["java.lang.Number"]), ("Short", "1.toShort", ["java.lang.Number"]),
    ("Byte", "1.toByte", ["java.lang.Number"]), ("Double", "1.0", ["java.lang.Number"]), ("Float", "1.0f", ["java.lang.Number"]),
    ("Char", "'a'", []), ("Boolean", "true", []), ("Unit", "()", []),
]


def targets(p, supers):
    """(kind, tested type, the binder's use at it): what each cell's first case tests."""
    out = [("class", "C", "b.x"), ("trait", "T", "b.name"), ("valueclass", f"V{p}", "b.u")]
    for s in supers + ([] if p == "Unit" else ["java.lang.Comparable[?]"]) + ["java.io.Serializable"]:
        out.append(("super:" + s.split(".")[-1].split("[")[0], s, "b.getClass.getName"))
    out += [("Any", "Any", "b"), ("AnyVal", "AnyVal", "b"), ("Matchable", "Matchable", "b")]
    if p != "Unit":
        out.append(("union", f"({p} | String)", "b"))
    out.append(("String", "String", "b.length"))
    return out


def gen(d):
    os.makedirs(d, exist_ok=True)
    rows = []
    shared = ["case class C(x: Int)", "trait T { def name: String = \"t\" }"]
    for p, _, _ in PRIMS:
        if p != "Unit":
            shared.append(f"class V{p}(val u: {p}) extends AnyVal")
    with open(os.path.join(d, "Shared.scala"), "w") as f:
        f.write("\n".join(shared) + "\n")
    for p, v, supers in PRIMS:
        for kind, t, use in targets(p, supers):
            if kind == "valueclass" and p == "Unit":
                continue
            name = f"Cell_{p}_{kind.replace(':', '_')}"
            with open(os.path.join(d, name + ".scala"), "w") as f:
                f.write(f"object {name} {{\n  def main(args: Array[String]): Unit = {{\n    val x: {p} = {v}\n"
                        f"    (x: @unchecked) match {{\n      case b: {t} => println(\"hit \" + {use})\n"
                        f"      case _ => println(\"next\")\n    }}\n  }}\n}}\n")
            rows.append((name, p, kind, t))
    with open(os.path.join(d, "cells.tsv"), "w") as f:
        for r in rows:
            f.write("\t".join(r) + "\n")
    print(f"prim-typetest: {len(rows)} cells")


def read(d, side, name):
    base = os.path.join(d, "runs", side, name)
    try:
        out = open(base + ".out").read()
        code = open(base + ".exit").read().strip()
    except OSError:
        return None
    if not code:
        return None
    return out, code


def judge(d, quiet=False):
    cells = [l.split("\t") for l in open(os.path.join(d, "cells.tsv")).read().splitlines() if l]
    bad = []
    for name, p, kind, t in cells:
        sc, tq = read(d, "scalac", name), read(d, "teq", name)
        if sc is None or tq is None:
            bad.append((name, "a run missing: " + ("scalac" if sc is None else "teq")))
        elif sc[1] != tq[1]:
            bad.append((name, f"exit {tq[1]} against scalac's {sc[1]}"))
        elif sc[0] != tq[0]:
            bad.append((name, f"output {tq[0].strip()[:60]!r} against scalac's {sc[0].strip()[:60]!r}"))
    if not quiet:
        for name, why in bad:
            print(f"DIFF {name}: {why}")
        print(f"prim-typetest: {len(cells)} cells; as scalac's in {len(cells) - len(bad)}, differing in {len(bad)}")
    return bad


def selftest(d):
    import shutil
    if os.path.exists(d):
        shutil.rmtree(d)
    os.makedirs(os.path.join(d, "runs", "scalac"))
    os.makedirs(os.path.join(d, "runs", "teq"))
    cells = ["ok", "wrong", "verify", "trailing", "missing", "noexit"]
    with open(os.path.join(d, "cells.tsv"), "w") as f:
        for c in cells:
            f.write(f"{c}\tInt\tclass\tC\n")

    def put(side, name, out, code):
        base = os.path.join(d, "runs", side, name)
        open(base + ".out", "w").write(out)
        if code is not None:
            open(base + ".exit", "w").write(code + "\n")
    for c in cells:
        put("scalac", c, "next\n", "0")
    put("teq", "ok", "next\n", "0")
    put("teq", "wrong", "hit 1\n", "0")
    put("teq", "verify", "", "1")
    put("teq", "trailing", "next\n", "1")
    put("teq", "noexit", "next\n", None)
    got = {name for name, _ in judge(d, quiet=True)}
    want = {"wrong", "verify", "trailing", "missing", "noexit"}
    if got != want:
        print(f"prim-typetest selftest: FAILED, found {sorted(got)} against {sorted(want)}")
        return 1
    print("prim-typetest selftest: a wrong output, a verify failure, a trailing failure, a missing cell and a missing exit found, none other")
    return 0


if __name__ == "__main__":
    verb, d = sys.argv[1], sys.argv[2]
    if verb == "gen":
        gen(d)
    elif verb == "judge":
        sys.exit(1 if judge(d) else 0)
    elif verb == "selftest":
        sys.exit(selftest(d))
    else:
        sys.exit("usage: prim_typetest.py gen|judge|selftest <dir>")
