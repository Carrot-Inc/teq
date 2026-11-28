"""getter_desc.py gen|selftest|judge <dir>

getter-desc.sh records the whole and split scalac oracles and each teq leg.
table.tsv compares the run and, on the JVM, the descriptor at the call site.
The super routes also compare the invocation opcode and target.
"""
import csv
import itertools
import re
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

AXES = {
    "kind": ("class", "method", "bounded", "value", "primitive", "array"),
    "place": ("earlier", "result", "first"),
    "depth": ("0", "1", "2", "restate"),
    "overload": ("twin", "pair"),
    "route": ("class", "trait", "super", "trait-super", "mixin-super"),
}
SCALAC_LEG = {"jvm": "jvm", "jvm-split": "jvm-split", "jvm-scalac-lib": "jvm-split"}
LEGS = (*SCALAC_LEG, "js", "interp")


def cells():
    def block(depths, overloads, routes):
        return itertools.product(AXES["kind"], AXES["place"], depths, overloads, routes)

    rows = itertools.chain(block(AXES["depth"][:3], AXES["overload"], ("class",)),
                           block(("restate",), AXES["overload"], ("class",)),
                           block(("1",), ("twin",), ("trait", "super")),
                           block(("1",), ("twin",), ("trait-super",)),
                           block(("1",), ("twin",), ("mixin-super",)))
    return {f"p{i:04d}": row for i, row in enumerate(rows, 1)}


def program(kind, place, depth, overload, route):
    defs = ["trait Y"]
    if kind in ("class", "method", "bounded"):
        if kind == "bounded":
            defs.append("trait Bnd")
        defs += ["trait X" + (" extends Bnd" if kind == "bounded" else ""),
                 'object Z extends X { override def toString: String = "Z" }']
        typ, value = "X", "Z"
    elif kind == "value":
        defs.append("class V(val x: Int) extends AnyVal")
        typ, value = "V", "new V(7)"
    elif kind == "primitive":
        typ, value = "Int", "7"
    else:
        typ, value = "Array[Int]", "Array(7, 8)"

    method = kind == "method"
    generic = "[U]" if method else ""
    param = "U" if method else "T"
    parent = "A" if method else "A[T]"
    if kind == "bounded":
        parent = "A[T <: Bnd]"
    extends = "A" if method else f"A[{typ}]"
    mixin_parent = extends
    seed = "t"
    if place != "earlier":
        parent += f"(t: {typ if method else param})"
        extends += f"({value})"
        if method:
            seed = "t.asInstanceOf[U]"

    restated = "2"
    if depth == "restate" and place != "earlier":
        if typ == "X":
            defs.append('object W extends X { override def toString: String = "W" }')
        restated = {"X": "W", "V": "new V(8)", "Int": "8", "Array[Int]": "Array(8, 9)"}[typ]
        if method:
            restated += ".asInstanceOf[U]"

    def signature(t, default=""):
        d = f" = {default}" if default else ""
        if place == "earlier":
            return f"m{generic}(a: {t})(b: Int{d}): Int"
        if place == "result":
            return f"m{generic}(a: Int)(b: {t}{d}): {t}"
        return f"m{generic}(a: {t}{d}): {t}"

    body = "a" if place == "first" else "b"
    on_super = route in ("super", "trait-super", "mixin-super")
    declaration = "trait" if route in ("trait", "trait-super", "mixin-super") else "class"
    original = signature(param, "1" if place == "earlier" else seed)
    defs.append(f"{declaration} {parent} {{ def {original} = {body} }}")
    rest = []
    receiver = "C" if depth in ("2", "restate") else "B"
    override_type = "U" if method else typ
    for cls, sup, offset in [("B", extends, 10), ("C", "B", 20)]:
        members = []
        if depth != "0":
            expr = f"b + {offset}" if place == "earlier" else body
            default = restated if depth == "restate" and cls == "B" else ""
            if on_super:
                args = "()" if place == "first" else "(a)()"
                expr = f"super.m{'[U]' if method else ''}{args}"
            members.append(f"override def {signature(override_type, default)} = {expr}")
        if cls == receiver and overload == "pair":
            later = "(b: Int)" if place != "first" else ""
            members.append(f"def m{generic}(a: Y){later}: Int = -99")
        declaration = f"class {cls} extends {sup}"
        if route == "mixin-super":
            declaration = f"trait M extends {mixin_parent}"
        rest.append(declaration + (" {\n  " + "\n  ".join(members) + "\n}" if members else ""))
        if route == "mixin-super":
            rest.append("class B extends M" + (f" with {extends}" if place != "earlier" else ""))
        if cls == receiver:
            break

    args = {"earlier": f"({value})()", "result": "(0)()", "first": "()"}[place]
    if on_super:
        args = {"earlier": f"({value})(0)", "result": f"(0)({value})", "first": f"({value})"}[place]
    call = f"new {receiver}().m{'[X]' if method else ''}{args}"
    if place != "earlier" and kind == "array":
        call = f"java.util.Arrays.toString({call})"
    elif place != "earlier" and kind == "value":
        call += ".x"
    rest += ["@main def run(): Unit =", f"  try println({call})",
             "  catch { case e: Throwable => println(e.getClass.getName) }"]
    return "\n".join(defs) + "\n", "\n".join(rest) + "\n"


def gen(out):
    def write(path, data):
        if path.exists():
            if path.read_text() != data:
                sys.exit(f"getter-desc: changed {path}, use a fresh output directory")
        else:
            path.write_text(data)

    manifest = []
    for cell, axes in cells().items():
        src = out / cell
        src.mkdir(parents=True, exist_ok=True)
        parent, rest = program(*axes)
        package = f"package {cell}\n"
        write(src / "A.scala", package + parent)
        write(src / "Main.scala", package + rest)
        write(src / "Whole.scala", package + parent + rest)
        manifest.append("\t".join((cell, *axes)))
    write(out / "cells.tsv", "\n".join(manifest) + "\n")
    print(f"getter-desc: {len(manifest)} cells")


VERDICTS = ("as scalac", "refuses", "throws", "wrong value", "timeout", "not valid", "unstable")
GETTERS = ("same getter", "different getter", "getter unavailable")
INVOKE = re.compile(r'^\s*\d+:\s+(invoke\w+)\s+.*?// ((?:InterfaceMethod|Method) '
                    r'(?:[^\s:]+\.)?(?:[\w$]+\$\$super\$)?m\$default\$(\d+)\$?:(\S+))', re.M)
EXCEPTION = re.compile(r'(?:[\w$]+\.)+[\w$]*(?:Error|Exception|Throwable)\b')


def diagnostic(log):
    match = re.search(r'^.*?:(\d+):(\d+): error: (.+)$', log, re.M)
    if match:
        return f"{match[1]}:{match[2]}: {match[3]}"
    match = re.search(r'^-- [^\n]*?Error: [^\n]*?:(\d+):(\d+)\s*-*\s*$', log, re.M)
    if match:
        message = re.search(r'^\s*\|\s*(\w.*)$', log[match.end():], re.M)
        return f"{match[1]}:{match[2]}: {message[1] if message else match[0].strip()}"
    return ""


@dataclass
class Observation:
    state: str
    value: str = ""
    desc: str = ""
    invocation: str = ""
    declaration_desc: str = ""


class RecordError(Exception):
    pass


def record(root, step):
    try:
        status = (root / f"{step}.exit").read_text().strip()
        out = (root / f"{step}.out").read_text()
        err = (root / f"{step}.err").read_text()
    except FileNotFoundError as e:
        raise RecordError("not valid", f"missing {Path(e.filename).name}") from e
    if not status.isdecimal() or int(status) > 255:
        raise RecordError("not valid", f"invalid {step}.exit")
    if status in ("125", "126", "127"):
        raise RecordError("not valid", f"{step} exit {status}: {err.strip()}")
    if status == "124" or (status == "137" and "timeout: sending signal" in err):
        raise RecordError("timeout", step)
    return int(status), out, err


def getter_call(bytecode, number, caller, owner=None):
    if owner:
        body = re.search(rf"^(?:public )?(?:abstract |final )?(?:class|interface) {re.escape(owner)}(?=[ <])[^\n]*\n(.*?)^\}}",
                         bytecode, re.M | re.S)
        bytecode = body[1] if body else ""

    # Trait forwarders and bridges can invoke getters from outside the tested method.
    calls = set()
    for block in re.split(r"(?=^  \S)", bytecode, flags=re.M):
        if re.match(rf"  [^\n]* {re.escape(caller)}\(", block):
            calls.update(INVOKE.findall(block))
    invocation = ", ".join(sorted(f"{op} {target}" for op, target, _, _ in calls))
    if len(calls) == 1:
        _, _, n, desc = next(iter(calls))
        if n == number:
            return desc, invocation
    return invocation or "absent", invocation


def observe(root, leg, scalac, getter, caller, owner=None):
    desc = invocation = declaration_desc = ""
    try:
        if leg != "interp":
            steps = ["parent", "parent-javap", "build"] if leg == "jvm-split" else ["build"]
            if leg.startswith("jvm"):
                steps.append("javap")
            for step in steps:
                code, out, err = record(root, step)
                if code:
                    log = (err + "\n" + out).strip()
                    detail = diagnostic(log) or next(iter(log.splitlines()), f"{step} exit {code}")
                    state = "not valid" if step.endswith("javap") else "refuses"
                    return Observation(state, detail)
                if step == "javap":
                    desc, invocation = getter_call(out, getter, caller, owner)
                    declaration_desc, target = desc, invocation
                    # The static trait forwarder takes this explicitly. A mixin accessor forwards in B.
                    accessor = re.search(r"(\w[\w$]*\$\$super\$m\$default\$\d+):", target)
                    if owner and accessor:
                        declaration_desc, target = getter_call(out, getter, accessor[1], f"{root.name}.B")
                    if target.startswith("invokestatic ") and f"m$default${getter}$:" in target:
                        declaration_desc = re.sub(r"^\(L[^;]+;", "(", declaration_desc)
                    if scalac and not desc.startswith("("):
                        return Observation("not valid", "unexpected scalac getter call", desc, invocation)
        code, out, err = record(root, "run")
        if scalac and record(root, "repeat")[:2] != (code, out):
            return Observation("unstable", "scalac repeat differs", desc, invocation, declaration_desc)
        if code:
            detail = diagnostic(err + "\n" + out)
            if detail and leg == "interp":
                return Observation("refuses", detail)
            thrown = EXCEPTION.search(err)
            if thrown:
                return Observation("throws", thrown[0], desc, invocation, declaration_desc)
            return Observation("wrong value", f"exit {code}: {(err or out).strip()}", desc, invocation, declaration_desc)
        return Observation("ran", out, desc, invocation, declaration_desc)
    except RecordError as e:
        return Observation(*e.args, desc, invocation, declaration_desc)


def oracle(whole, split, prefer_split=False):
    refs = (split, whole) if prefer_split else (whole, split)
    for ref in refs:
        if ref.state != "ran":
            state = ref.state if ref.state in ("unstable", "timeout") else "not valid"
            return Observation(state, f"scalac {ref.state}: {ref.value}")
    if whole.value != split.value:
        return Observation("unstable", f"scalac whole {whole.value!r}, split {split.value!r}")
    if len(whole.value.splitlines()) != 1 or whole.value.strip() == "-99":
        return Observation("not valid", f"scalac output {whole.value!r}")
    return refs[0]


def verdict(ref, run):
    if ref.state != "ran":
        return ref.state, ref.value
    if run.state != "ran":
        return run.state, run.value
    if ref.value == run.value:
        return "as scalac", ""
    if EXCEPTION.fullmatch(run.value.strip()):
        return "throws", run.value.strip()
    return "wrong value", repr(run.value)


def declarers(run, reference, inherited):
    # In a restatement both values are observable: at depth 2 the same C.m takes A's default.
    own = "B" if inherited else "A"
    value = getter = "unavailable"
    if run.state == "ran" and not EXCEPTION.fullmatch(run.value.strip()):
        if run.value == reference.value:
            value = own
        elif inherited and inherited.state == "ran" and run.value == inherited.value:
            value = "A"
    if run.declaration_desc.startswith("(") and reference.declaration_desc.startswith("("):
        if run.declaration_desc == reference.declaration_desc:
            getter = own
            if inherited and inherited.state == "ran" and inherited.declaration_desc == run.declaration_desc:
                getter = "A/B"
        elif inherited and inherited.state == "ran" and run.declaration_desc == inherited.declaration_desc:
            getter = "A"
        elif not inherited or inherited.state == "ran":
            getter = "no matching declaration"
    return value, getter


def manifest(out, expected):
    seen = set()
    errors = {}
    path = out / "cells.tsv"
    if path.is_file():
        for line in path.read_text().splitlines():
            cell, *axes = line.split("\t")
            if cell in seen:
                errors[cell] = "duplicate cell"
            elif cell not in expected:
                errors[cell] = "stray cell"
            elif tuple(axes) != expected[cell]:
                errors[cell] = "incorrect axes"
            seen.add(cell)
    for cell in expected.keys() - seen:
        errors[cell] = "missing cell"
    for path in (out / "runs").glob("*/*/*"):
        if path.name not in expected:
            errors[path.name] = "stray cell"
    return errors


def judge(out, expected):
    sides = ["base"]
    if (out / "head.sha256").is_file() or (out / "runs/head").is_dir():
        sides.append("head")
    counts = Counter()
    problems = manifest(out, expected)
    moved = []
    apart = {}
    by_axes = {axes: cell for cell, axes in expected.items()}
    wrong_base = False
    with (out / "table.tsv").open("w", newline="") as f:
        table = csv.writer(f, delimiter="\t", lineterminator="\n")
        table.writerow(("cell", *AXES, "binary", "leg", "verdict", "detail", "getter call",
                        "teq descriptor", "scalac descriptor", "scalac output", "scalac refusal",
                        "teq invocation", "scalac invocation", "value declarer", "value leg", "getter declarer"))
        for cell in sorted(expected.keys() | problems.keys()):
            axes = expected.get(cell, ("-",) * len(AXES))
            getter = "1" if axes[1] == "first" else "2"

            on_super = axes[-1] in ("super", "trait-super", "mixin-super")

            def read(side, leg, source=cell):
                if source in problems:
                    return Observation("not valid", problems[source])
                return observe(out / "runs" / side / leg / source, leg, side == "scalac", getter,
                               "m" if on_super else "run", f"{source}.M" if axes[-1] == "mixin-super" else None)

            refs = {leg: read("scalac", leg) for leg in ("jvm", "jvm-split")}
            ref = oracle(*refs.values())
            mixed_ref = oracle(*refs.values(), prefer_split=True)
            inherited = {}
            if axes[2] == "restate":
                other_cell = by_axes[(*axes[:2], "2", *axes[3:])]
                for leg in refs:
                    parent = read("scalac", leg, other_cell)
                    parent.declaration_desc = parent.declaration_desc.replace(f"L{other_cell}/", f"L{cell}/")
                    inherited[leg] = parent
                control = oracle(*inherited.values())
                if control.state != "ran":
                    inherited = dict.fromkeys(refs, control)
            interpreter = {side: read(side, "interp") for side in sides}
            base_right = {}
            for side, leg in itertools.product(sides, LEGS):
                run = interpreter[side] if leg == "interp" else read(side, leg)
                oracle_leg = SCALAC_LEG.get(leg, "jvm")
                reference = refs[oracle_leg]
                state, detail = verdict(mixed_ref if leg == "jvm-scalac-lib" else ref, run)
                if cell in problems:
                    state, detail = "not valid", problems[cell]
                getter_call = ""
                getter_match = ""
                desc = reference.desc if leg in SCALAC_LEG else ""
                if leg in SCALAC_LEG:
                    if desc and run.desc == desc and (not on_super or run.invocation == reference.invocation):
                        getter_match, getter_call = "same getter", "getter call as scalac"
                    else:
                        getter_match = "different getter" if desc and run.desc else "getter unavailable"
                        actual = run.invocation if on_super and run.invocation else run.desc
                        getter_call = f"getter call {actual or 'unavailable'}"
                right = state == "as scalac" and getter_match in ("", "same getter")
                if side == "base":
                    base_right[leg] = right
                    wrong_base |= not right
                elif base_right[leg] and not right:
                    moved.append(f"{cell}\t{leg}\t{state}\t{getter_call}".rstrip())
                for axis, value in [("leg", leg), *zip(AXES, axes)]:
                    counts[side, axis, value, state] += 1
                    if getter_match:
                        counts[side, axis, value, getter_match] += 1
                value_owner = getter_owner = "unavailable"
                value_leg = leg
                if ref.state == "ran":
                    value_owner, getter_owner = declarers(run, reference, inherited.get(oracle_leg))
                    # A JVM linkage failure can leave the cell's printed default observable only in another run.
                    if leg in SCALAC_LEG and (run.state != "ran" or EXCEPTION.fullmatch(run.value.strip())):
                        value_owner, _ = declarers(interpreter[side], ref, inherited.get("jvm"))
                        value_leg = "interp"
                if (value_owner != "unavailable" and getter_owner not in ("unavailable", "A/B")
                        and value_owner != getter_owner):
                    difference = (side, leg, f"value {value_owner} ({value_leg})", f"getter {getter_owner}")
                    apart.setdefault(difference, []).append(cell)
                table.writerow((cell, *axes, side, leg, state, detail.replace("\n", r"\n"),
                                getter_call, run.desc, desc, reference.value.rstrip("\n") if reference.state == "ran" else "",
                                reference.value if reference.state == "refuses" else "",
                                run.invocation, reference.invocation if leg in SCALAC_LEG else "",
                                value_owner, value_leg if value_owner != "unavailable" else "", getter_owner if leg in SCALAC_LEG else ""))

    columns = VERDICTS + GETTERS
    summary = [f"getter-desc: {len(expected)} cells", "\t".join(("binary", "axis", "value", *columns))]
    for side in sides:
        for axis, values in [("leg", LEGS), *AXES.items()]:
            for value in values:
                summary.append("\t".join([side, axis, value, *(str(counts[side, axis, value, v]) for v in columns)]))
    for cell, issue in sorted(problems.items()):
        summary.append(f"{cell}: not valid ({issue})")
    summary.append(f"declarers apart: {sum(len(group) for group in apart.values())}")
    summary.extend("\t".join((*difference, " ".join(group))) for difference, group in apart.items())
    if "head" in sides:
        summary += [f"cells right on the base and not right on the head: {len(moved)}", *moved]
    summary.append(f"getter-desc: base {'differs from' if wrong_base else 'as'} scalac")
    (out / "summary.txt").write_text("\n".join(summary) + "\n")
    return int(wrong_base or bool(moved))


def selftest(out):
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=out) as tmp:
        d = Path(tmp)
        names = ("ok", "refusal", "throws", "wrong", "timeout", "missing-status", "missing-cell",
                 "duplicate", "rejected", "unstable", "descriptor", "moved", "super", "trait-super",
                 "p0199", "inherited", "restated")
        axes = ("class", "earlier", "1", "twin", "class")
        expected = dict.fromkeys(names, axes)
        for route in ("super", "trait-super"):
            expected[route] = (*axes[:-1], route)
        expected["p0199"] = (*axes[:-1], "mixin-super")
        expected["inherited"] = (*axes[:2], "2", *axes[3:])
        expected["restated"] = (*axes[:2], "restate", *axes[3:])
        rows = [n for n in names if n != "missing-cell"] + ["duplicate", "stray"]
        (d / "cells.tsv").write_text("".join("\t".join((n, *expected.get(n, axes))) + "\n" for n in rows))
        call = ("  public void run();\n    Code:\n"
                "         2: invokevirtual #1 // Method A.m$default$2:(Ljava/lang/Object;)I\n")

        super_call = call.replace("void run()", "int m(Object, int)").replace("invokevirtual", "invokespecial")

        trait_call = ("  public int m(p0181.X, int);\n    Code:\n"
                      "         4: invokestatic  #21                 // InterfaceMethod p0181/A.m$default$2$:(Lp0181/A;Ljava/lang/Object;)I\n")

        accessor = "p0199$M$$super$m$default$2"
        mixin_call = ("public interface p0199.M extends p0199.A {\n"
                      "  public default int m(p0199.X, int);\n    Code:\n"
                      f"         4: invokeinterface #25, 2 // InterfaceMethod {accessor}:(Lp0199/X;)I\n}}\n"
                      "public class p0199.B implements p0199.M {\n"
                      f"  public int {accessor}(p0199.X);\n    Code:\n"
                      "         2: invokestatic #22 // InterfaceMethod p0199/A.m$default$2$:(Lp0199/A;Ljava/lang/Object;)I\n"
                      + super_call + "}\n")
        restated_call = call.replace("Ljava/lang/Object;", "Lrestated/X;")

        def plant(side, leg, cell, step, code=0, stdout="", stderr=""):
            root = d / "runs" / side / leg / cell
            root.mkdir(parents=True, exist_ok=True)
            for ext, value in (("exit", f"{code}\n"), ("out", stdout), ("err", stderr)):
                (root / f"{step}.{ext}").write_text(value)

        for n, side, leg in itertools.product(names, ("scalac", "base", "head"), LEGS):
            if side == "scalac" and leg not in ("jvm", "jvm-split"):
                continue
            if leg == "jvm-split":
                plant(side, leg, n, "parent")
                plant(side, leg, n, "parent-javap")
            if leg != "interp":
                plant(side, leg, n, "build")
            if leg in SCALAC_LEG:
                bytecode = {"super": super_call, "trait-super": trait_call, "p0199": mixin_call,
                            "restated": restated_call}.get(n, call)
                plant(side, leg, n, "javap", stdout=bytecode)
            value = {"trait-super": "1\n", "p0199": "1\n", "inherited": "21\n", "restated": "22\n"}.get(n, "11\n")
            plant(side, leg, n, "run", stdout=value)
            if side == "scalac":
                plant(side, leg, n, "repeat", stdout=value)
        bridge = call.replace("void run()", "int m$default$2(int)").replace("Ljava/lang/Object;", "I")
        plant("base", "jvm", "ok", "javap", stdout=bridge + call)
        plant("base", "jvm", "refusal", "build", 1, stderr="Whole.scala:7:3: error: missing method\n")
        plant("base", "jvm", "throws", "run", stdout="java.lang.NoSuchMethodError\n")
        plant("base", "jvm", "wrong", "run", stdout="12\n")
        plant("base", "jvm", "timeout", "build", 124)
        (d / "runs/base/jvm/missing-status/run.exit").unlink()
        plant("scalac", "jvm", "unstable", "repeat", stdout="12\n")
        plant("scalac", "jvm-split", "rejected", "build", 1, stderr=
              "-- [E007] Type Mismatch Error: Main.scala:4:9 ----\n4 | source\n  | ^\n  | Found: X\n")
        plant("base", "jvm", "descriptor", "javap", stdout=call.replace("java/lang/Object", "p0001/X"))
        plant("scalac", "jvm-split", "descriptor", "javap", stdout=call.replace("java/lang/Object", "p0001/X"))
        plant("base", "jvm-scalac-lib", "descriptor", "javap", stdout=call.replace("Ljava/lang/Object;", "I"))
        plant("base", "jvm-scalac-lib", "refusal", "build", 1, stderr="Main.scala:3:2: error: missing getter\n")
        plant("head", "jvm", "moved", "run", stdout="12\n")
        plant("head", "jvm-split", "moved", "javap", stdout=call.replace("java/lang/Object", "p0001/X"))
        plant("head", "jvm-scalac-lib", "super", "javap", stdout=super_call.replace("invokespecial", "invokevirtual"))
        plant("head", "jvm", "trait-super", "javap", stdout=trait_call.replace("p0181/A.m", "p0181/B.m"))
        plant("head", "jvm", "p0199", "javap", stdout=mixin_call.replace("invokeinterface", "invokevirtual"))
        plant("base", "jvm-split", "p0199", "javap", stdout=mixin_call.replace("Lp0199/A;Ljava/lang/Object;", "Lp0199/A;Lp0199/X;"))
        plant("base", "jvm", "restated", "javap", stdout=call)
        plant("base", "jvm-split", "restated", "run", stdout="21\n")
        if judge(d, expected) != 1:
            sys.exit("getter-desc selftest FAILED: base differences passed")
        with (d / "table.tsv").open() as f:
            rows = list(csv.DictReader(f, delimiter="\t"))
        result = {r["cell"]: r for r in rows if r["binary"] == "base" and r["leg"] == "jvm"}
        mixed = {r["cell"]: r for r in rows if r["binary"] == "base" and r["leg"] == "jvm-scalac-lib"}
        if (mixed["refusal"]["verdict"] != "refuses" or mixed["refusal"]["detail"] != "3:2: missing getter"
                or mixed["descriptor"]["getter call"] != "getter call (I)I"
                or mixed["descriptor"]["verdict"] != "as scalac"
                or mixed["descriptor"]["scalac descriptor"] != "(Lp0001/X;)I"):
            sys.exit("getter-desc selftest FAILED: scalac-lib refusal or split descriptor")
        states = dict(zip(names, ("as scalac", "refuses", "throws", "wrong value", "timeout", "not valid",
                                  "not valid", "not valid", "not valid", "unstable", "as scalac", "as scalac", "as scalac", "as scalac")))
        states.update(dict.fromkeys(("p0199", "inherited", "restated"), "as scalac"))
        states["stray"] = "not valid"
        if {n: r["verdict"] for n, r in result.items()} != states:
            sys.exit("getter-desc selftest FAILED: verdicts")
        details = {"refusal": "7:3: missing method", "missing-cell": "missing cell", "duplicate": "duplicate cell",
                   "stray": "stray cell", "missing-status": "missing run.exit", "rejected": "scalac refuses: 4:9: Found: X"}
        if any(result[n]["detail"] != why for n, why in details.items()):
            sys.exit("getter-desc selftest FAILED: diagnostics or missing records")
        fence = ("cells right on the base and not right on the head: 5\n"
                 "moved\tjvm\twrong value\tgetter call as scalac\n"
                 "moved\tjvm-split\tas scalac\tgetter call (Lp0001/X;)I\n"
                 f"p0199\tjvm\tas scalac\tgetter call invokevirtual InterfaceMethod {accessor}:(Lp0199/X;)I\n"
                 "super\tjvm-scalac-lib\tas scalac\tgetter call invokevirtual Method A.m$default$2:(Ljava/lang/Object;)I\n"
                 "trait-super\tjvm\tas scalac\tgetter call invokestatic InterfaceMethod "
                 "p0181/B.m$default$2$:(Lp0181/A;Ljava/lang/Object;)I\n")
        summary = (d / "summary.txt").read_text()
        if ("base\tjvm\tvalue B (jvm)\tgetter A\trestated\n" not in summary
                or "base\tjvm-split\tvalue A (jvm-split)\tgetter B\trestated\n" not in summary
                or (result["throws"]["value declarer"], result["throws"]["value leg"]) != ("A", "interp")
                or result["p0199"]["getter declarer"] != "A"
                or not any(r["binary"] == "base" and r["cell"] == "p0199" and r["leg"] == "jvm-split"
                           and r["getter declarer"] == "no matching declaration" for r in rows)):
            sys.exit("getter-desc selftest FAILED: declarers apart")
        if (result["ok"]["getter call"] != "getter call as scalac"
                or result["descriptor"]["getter call"] != "getter call (Lp0001/X;)I"
                or fence not in summary):
            sys.exit("getter-desc selftest FAILED: getter calls or per-line fence")
    print("getter-desc selftest: all seven verdicts, missing status and cell, duplicate, stray, scalac refusal, getter calls, trait-super owner, mixin-super accessor, declarers apart, scalac-lib refusal and descriptor, and fence found")


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] not in ("gen", "judge", "selftest"):
        sys.exit("usage: getter_desc.py gen|judge|selftest <dir>")
    verb, directory = sys.argv[1], Path(sys.argv[2])
    if verb == "gen":
        gen(directory)
    else:
        selftest(directory)
        if verb == "judge":
            status = judge(directory, cells())
            print((directory / "summary.txt").read_text(), end="")
            sys.exit(status)
