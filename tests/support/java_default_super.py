"""Generate and judge the cells run by java-default-super.sh."""
import csv
import itertools
import re
import sys
import tempfile
from collections import Counter
from pathlib import Path

# Interface, Java parent, and the members used by its defaults.
LIST_DATA = '''val data = new java.util.ArrayList[String]()
data.add("b")
data.add("a")
def size(): Int = data.size()'''
PARENTS = {
    "java.util.Iterator": ("java.util.Iterator[String]", '''def hasNext(): Boolean = false
def next(): String = "x"'''),
    "java.lang.Iterable": ("java.lang.Iterable[String]", '''def iterator(): java.util.Iterator[String] =
  java.util.Collections.singletonList("x").iterator()
var seen = ""'''),
    "java.util.Collection": ("java.util.AbstractCollection[String]", LIST_DATA + '''
def iterator(): java.util.Iterator[String] = data.iterator()'''),
    "java.util.List": ("java.util.AbstractList[String]", LIST_DATA + '''
def get(i: Int): String = data.get(i)
override def set(i: Int, s: String): String = data.set(i, s)
override def add(i: Int, s: String): Unit = data.add(i, s)'''),
    "java.util.Map": ("java.util.AbstractMap[String, String]", '''val data = new java.util.HashMap[String, String]()
data.put("k", "v")
def entrySet(): java.util.Set[java.util.Map.Entry[String, String]] = data.entrySet()
override def put(k: String, v: String): String = data.put(k, v)
override def size(): Int = data.size()
override def clear(): Unit = data.clear()
override def get(k: Any): String = data.get(k)
override def containsKey(k: Any): Boolean = data.containsKey(k)
override def keySet(): java.util.Set[String] = data.keySet()
override def values(): java.util.Collection[String] = data.values()
var seen = ""'''),
    "java.util.Comparator": ("java.util.Comparator[String]", 'def compare(a: String, b: String): Int = a.length - b.length'),
    "java.lang.CharSequence": ("java.lang.CharSequence", '''def length(): Int = 1
def charAt(i: Int): Char = 'x'
def subSequence(start: Int, end: Int): CharSequence = "x".substring(start, end)'''),
    "java.util.function.Function": ("java.util.function.Function[String, String]", 'def apply(s: String): String = s + "f"'),
    "java.util.function.Predicate": ("java.util.function.Predicate[String]", 'def test(s: String): Boolean = s.length > 0'),
}


def sam(tpe, method, params, result, body):
    return (f"({params}) => {body}", f"new {tpe} {{ def {method}({params}): {result} = {body} }}")


def function(tpe, method, params, result, body):
    return sam("java.util.function." + tpe, method, params, result, body)


COMPARE = sam("java.util.Comparator[String]", "compare", "a: String, b: String", "Int", "a.compareTo(b)")
PRED = function("Predicate[String]", "test", "s: String", "Boolean", 's == "a"')
FN = function("Function[String, String]", "apply", "s: String", "String", 's + "!"')
# Each observation contains one call to super. ARG is the lambda or its anonymous-class spelling.
METHODS = {
    "java.util.Iterator": [
        ("forEachRemaining", "super.forEachRemaining(ARG)", function("Consumer[String]", "accept", "s: String", "Unit", "()")),
        ("remove", '''try {
  super.remove()
  println("returned")
} catch { case _: UnsupportedOperationException => println("unsupported") }''', None)],
    "java.lang.Iterable": [
        ("forEach", "super.forEach(ARG)\nprintln(seen)", function("Consumer[String]", "accept", "s: String", "Unit", "{ seen = seen + s }")),
        ("spliterator", "println(super.spliterator().getExactSizeIfKnown())", None)],
    "java.util.Collection": [
        ("removeIf", "val removed = super.removeIf(ARG)\nprintln(removed.toString + data.toString())", PRED),
        ("stream", "println(super.stream().count())", None),
        ("toArray", 'println(super.toArray(ARG).mkString(","))', function("IntFunction[Array[String]]", "apply", "n: Int", "Array[String]", "new Array[String](n)"))],
    "java.util.List": [
        ("replaceAll", 'super.replaceAll(ARG)\nprintln(data.toString())', function("UnaryOperator[String]", "apply", "s: String", "String", 's + "!"')),
        ("sort", "super.sort(ARG)\nprintln(data.toString())", COMPARE),
        ("reversed", "println(super.reversed().get(0))", None),
        ("getFirst", "println(super.getFirst())", None),
        ("addFirst", 'super.addFirst("z")\nprintln(data.toString())', None)],
    "java.util.Map": [
        ("getOrDefault", 'println(super.getOrDefault("absent", "default"))', None),
        ("forEach", "super.forEach(ARG)\nprintln(seen)", function("BiConsumer[String, String]", "accept", "k: String, v: String", "Unit", "{ seen = seen + k + v }")),
        ("putIfAbsent", 'super.putIfAbsent("new", "added")\nprintln(data.get("new"))', None),
        ("computeIfAbsent", 'println(super.computeIfAbsent("new", ARG))', FN),
        ("merge", 'println(super.merge("k", "2", ARG))', function("BiFunction[String, String, String]", "apply", "a: String, b: String", "String", "a + b"))],
    "java.util.Comparator": [
        ("reversed", 'println(super.reversed().compare("aa", "b"))', None),
        ("thenComparing", 'println(super.thenComparing(ARG).compare("a", "b"))', COMPARE)],
    "java.lang.CharSequence": [
        ("chars", "println(super.chars().sum())", None),
        ("isEmpty", "println(super.isEmpty())", None)],
    "java.util.function.Function": [("andThen", 'println(super.andThen(ARG).apply("x"))', FN)],
    "java.util.function.Predicate": [
        ("negate", 'println(super.negate().test("x"))', None),
        ("and", 'println(super.and(ARG).test("x"))', PRED)],
}

AXES = ["cell", "interface", "method", "argument", "parent", "placement", "between", "call"]
TARGETS = ["jvm", "js", "interp"]
LEGS = {"whole": TARGETS, "split": ["jvm-split"], "jar": ["jvm-jar"]}
VERDICTS = ["as scalac", "refuses", "throws", "wrong value", "timeout", "not valid"]


def indent(s, spaces=2):
    return "\n".join(" " * spaces + line for line in s.splitlines())


def program(parent, members, body, placement, between, iterator_call):
    owner = "Between" if between else "C"
    source = f"class {owner} extends {parent} {{\n{indent(members)}\n"
    upstream = ""
    if between:
        upstream = source + "}\n"
        source = "class C extends Between {\n"
    main = "new C()"
    if placement == "method":
        method = body if "\n" not in body else "{\n" + indent(body) + "\n}"
        source += indent("def early(): Unit = " + method)
        main += ".early()"
    elif placement == "field":
        source += indent("val result: Unit = {\n" + indent(body) + "\n}")
    else:
        source += indent(body)
    if iterator_call:
        main += f'\nprintln(classOf[{owner}].getInterfaces().map(_.getName()).mkString(","))'
    source += "\n}\nobject Main {\n  def main(args: Array[String]): Unit = {\n" + indent(main, 4) + "\n  }\n}\n"
    return upstream, source


def jar_cell(row):
    return row["between"] == "yes" and row["placement"] == "method" and row["argument"] != "instance" and row["call"] == "super"


def gen(d):
    d.mkdir(parents=True, exist_ok=True)
    with (d / "cells.tsv").open("w") as f:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n")
        writer.writerow(AXES)
        count = 0
        jar_cells = []
        for call, (interface, methods) in itertools.product(("super", "select"), METHODS.items()):
            parent, members = PARENTS[interface]
            _, bracket, arguments = parent.partition("[")
            interface_type = interface + bracket + arguments
            shape = "skeletal" if "Abstract" in parent else "direct"
            for method, body, sam_args in methods:
                args = zip(("lambda", "instance"), sam_args) if sam_args else [("plain", "")]
                for (arg, expr), placement, between in itertools.product(args, ("method", "constructor", "field"), (False, True)):
                    count += 1
                    cell = f"p{count:04d}"
                    path = d / cell
                    path.mkdir(exist_ok=True)
                    observation = body.replace("super.", f"(this: {interface_type}).") if call == "select" else body
                    upstream, source = program(parent, members, observation.replace("ARG", expr), placement, between, method == "forEachRemaining")
                    package = f"package {cell}\n\n"
                    (path / "Main.scala").write_text(package + upstream + source)
                    if between:
                        split = path / "split"
                        split.mkdir(exist_ok=True)
                        (split / "Between.scala").write_text(package + upstream)
                        (split / "Main.scala").write_text(package + source)
                    row = [cell, interface, method, arg, shape, placement, "yes" if between else "no", call]
                    writer.writerow(row)
                    if jar_cell(dict(zip(AXES, row))):
                        jar_cells.append(cell)
    (d / "jar-cells").write_text("".join(cell + "\n" for cell in jar_cells))
    print(f"java-default-super: {count} cells")


# Erased parameter lists for javap -s, including overloaded names such as toArray and thenComparing.
PARAMS = {
    "forEachRemaining": "java.util.function.Consumer", "removeIf": "java.util.function.Predicate",
    "toArray": "java.util.function.IntFunction", "replaceAll": "java.util.function.UnaryOperator",
    "sort": "java.util.Comparator", "addFirst": "java.lang.Object",
    "getOrDefault": "java.lang.Object java.lang.Object", "putIfAbsent": "java.lang.Object java.lang.Object",
    "computeIfAbsent": "java.lang.Object java.util.function.Function",
    "merge": "java.lang.Object java.lang.Object java.util.function.BiFunction",
    "thenComparing": "java.util.Comparator", "andThen": "java.util.function.Function",
    "and": "java.util.function.Predicate",
}


def jdk_method(d, interface, method):
    javap = d / "javap" / interface
    status, output, err = record(javap)
    if status:
        raise ValueError(f"{javap}: exit {status}: {err.strip()}")
    params = PARAMS.get(method, "")
    if method == "forEach":
        params = "java.util.function." + ("BiConsumer" if interface == "java.util.Map" else "Consumer")
    descriptor = "".join("L" + p.replace(".", "/") + ";" for p in params.split())
    pattern = rf"^  public default [^\n]+ {method}\([^\n]*\);\n\s+descriptor: \({re.escape(descriptor)}\)"
    if re.search(pattern, output.decode(), re.M):
        return "present"
    return f"javap lacks default {interface}.{method}({', '.join(params.split())})"


def record(prefix):
    def read(suffix):
        return Path(str(prefix) + suffix).read_bytes()
    return int(read(".exit")), read(".out"), read(".err").decode(errors="replace")


def batch(d, attempt):
    """Attach this scalac build to its finished cells and leave the rest for the next invocation."""
    path = d / "batches" / attempt
    cells = (path / "cells").read_text().splitlines()
    status, _, err = record(path / "build")
    errors = {}
    if status == 1:
        for block in re.split(r"(?=^-- )", err, flags=re.M):
            match = re.match(r"^-- [^\n]*Error: (?:[^\n]*/)?(p\d+)/Main\.scala:\d+:\d+", block)
            if match and match[1] in cells:
                message = re.search(r"^\s*\| (\s*[^\s^|][^\n]*)", block, re.M)
                reason = match[0] + (" " + message[1].strip() if message else "")
                errors.setdefault(match[1], reason)
    if status and not errors:
        raise ValueError(f"scalac exit {status}, no source error attributed; see {path}/build.err")
    remaining = []
    for cell in cells:
        if status and cell not in errors:
            remaining.append(cell)
            continue
        r = d / "runs" / "scalac" / cell
        r.mkdir(parents=True)
        for ext in ("out", "err", "exit"):
            (r / f"build.{ext}").hardlink_to(path / f"build.{ext}")
        if cell in errors:
            (r / "reason").write_text(errors[cell] + "\n")
    (path / "remaining").write_text("".join(c + "\n" for c in remaining))


def reference(r):
    status, _, _ = record(r / "build")
    if status == 1:
        return None, (r / "reason").read_text().strip()
    if status:
        raise ValueError(f"{r}/build.exit: {status}")
    status, output, err = record(r / "run")
    if status or len(output.splitlines()) != 1:
        raise ValueError(f"{r}: expected one scalac output line, got {output!r}, exit {status}: {err}")
    return output, ""


def result(r, target, expected, reason):
    steps = {"interp": ("run",), "jvm-split": ("upstream", "build", "run")}
    for step in steps.get(target, ("build", "run")):
        status, output, err = record(r / step)
        if not status:
            continue
        if expected is None:
            break
        if status == 124 or (status == 137 and "timeout: sending signal KILL" in err):
            return "timeout", step
        diagnostic = re.search(r"^(?:.*:\d+:\d+: )?error: .+", err, re.M)
        if diagnostic and (step != "run" or target == "interp"):
            return "refuses", diagnostic[0]
        exception = re.search(r'^(?:Exception in thread "[^"]+".*|[\w.$]*(?:Exception|Error)(?::.*|$))$', err, re.M)
        if step == "run" and exception:
            return "throws", exception[0]
        raise ValueError(f"{r}/{step}: unclassified exit {status}: {err.splitlines()[:2]}")
    if expected is None:
        return "not valid", reason
    if output != expected:
        return "wrong value", f"scalac {expected!r}, got {output!r}"
    return "as scalac", ""


def selftest(d):
    d.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=d) as tmp:
        root = Path(tmp)

        def plant(cell, step, code, output="", err=""):
            r = root / cell
            r.mkdir(exist_ok=True)
            (r / (step + ".out")).write_text(output)
            (r / (step + ".err")).write_text(err)
            if code is not None:
                (r / (step + ".exit")).write_text(f"{code}\n")

        diag = "Main.scala:6:23: error: no such method"
        thrown = 'Exception in thread "main" java.lang.VerifyError: bad code'
        plant("scala", "build", 0)
        plant("scala", "run", 0, "ok\n")
        oracle = reference(root / "scala")
        for cell in ("ok", "wrong", "throws", "noexit", "timeout"):
            plant(cell, "build", 0)
        plant("ok", "run", 0, "ok\n")
        plant("wrong", "run", 0, "other\n")
        plant("throws", "run", 1, "ok\n", thrown + "\n")
        plant("refuses", "build", 1, err=diag + "\n")
        plant("interp", "run", 1, err=diag + "\n")
        plant("noexit", "run", None, "ok\n")
        plant("timeout", "run", 124, "ok\n")
        plant("invalid", "build", 1)
        plant("accepted-invalid", "build", 0)
        plant("accepted-invalid", "run", 0, "ok\n")
        (root / "invalid" / "reason").write_text("scalac diagnostic")
        want = {"ok": ("as scalac", ""), "wrong": ("wrong value", "scalac b'ok\\n', got b'other\\n'"),
                "throws": ("throws", thrown), "refuses": ("refuses", diag), "interp": ("refuses", diag),
                "timeout": ("timeout", "run"), "noexit": "run.exit", "missing": "build.exit",
                "accepted-invalid": ("not valid", "scalac diagnostic")}
        for cell, answer in want.items():
            try:
                ref = reference(root / "invalid") if cell == "accepted-invalid" else oracle
                got = result(root / cell, "interp" if cell == "interp" else "jvm", *ref)
            except FileNotFoundError as error:
                got = Path(error.filename).name
            if got != answer:
                raise ValueError(f"selftest {cell}: found {got!r}, expected {answer!r}")
    print("java-default-super selftest: wrong value, refusal, throw, missing status, timeout, missing cell and scalac refusal found; matching run accepted")


def summary(rows, sides):
    lines = ["Counts in order: " + " / ".join(VERDICTS)]
    targets = [t for leg in LEGS.values() for t in leg]
    for side in sides:
        data = [r for r in rows if r["binary"] == side]
        lines += [side, "axis\tvalue\tcells\t" + "\t".join(targets)]
        for axis in ["all", *AXES[1:]]:
            counts = {}
            for r in data:
                value = r.get(axis, "all")
                if axis == "method":
                    value = r["interface"] + "." + value
                counts.setdefault(value, Counter())[r["target"], r["verdict"]] += 1
            for value, count in counts.items():
                n = sum(count["jvm", v] for v in VERDICTS)
                totals = ["/".join(str(count[t, v]) for v in VERDICTS) for t in targets]
                lines.append("\t".join([axis, value, str(n), *totals]))
    if "head" in sides:
        base_right = {(r["cell"], r["target"]) for r in rows if r["binary"] == "base" and r["verdict"] == "as scalac"}
        losses = [r for r in rows if r["binary"] == "head" and (r["cell"], r["target"]) in base_right and r["verdict"] != "as scalac"]
        for leg, targets in LEGS.items():
            lost = [r for r in losses if r["target"] in targets]
            lines.append(f"{leg}: cells right on the base and not right on the head: {len(lost)}")
            lines.extend("\t".join(r[key] for key in ("cell", "target", "verdict", "detail")) for r in lost)
    return "\n".join(lines) + "\n"


def judge(d):
    selftest(d / "selftest")
    with (d / "cells.tsv").open() as f:
        cells = list(csv.DictReader(f, delimiter="\t"))
    sides = (d / "binaries").read_text().splitlines()
    if not cells or sides not in (["base"], ["base", "head"]):
        raise ValueError("empty cells.tsv or missing binary names")
    available = {(i, m): jdk_method(d, i, m) for i, methods in METHODS.items() for m, _, _ in methods}
    rows = []
    for cell in cells:
        name = cell["cell"]
        jdk = available[cell["interface"], cell["method"]]
        oracle = reference(d / "runs" / "scalac" / name)
        targets = list(TARGETS)
        if cell["between"] == "yes":
            targets += LEGS["split"]
        if jar_cell(cell):
            targets += LEGS["jar"]
        for side, target in itertools.product(sides, targets):
            verdict, detail = result(d / "runs" / side / target / name, target, *oracle)
            rows.append(dict(cell, jdk=jdk, binary=side, target=target, verdict=verdict, detail=detail))
    with (d / "table.tsv").open("w") as f:
        writer = csv.DictWriter(f, AXES + ["jdk", "binary", "target", "verdict", "detail"], delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    report = summary(rows, sides)
    (d / "summary.txt").write_text(report)
    print(report, end="")
    differences = sum(r["binary"] == "base" and r["verdict"] not in ("as scalac", "not valid") for r in rows)
    print(f"java-default-super: {len(cells)} cells; {differences} base cell/target results differ from scalac")
    return int(bool(differences) or any(r["verdict"] not in ("as scalac", "not valid") for r in rows if r["binary"] == "head"))


if __name__ == "__main__":
    verb, directory, *args = sys.argv[1:]
    try:
        sys.exit({"gen": gen, "judge": judge, "selftest": selftest, "batch": batch}[verb](Path(directory), *args))
    except (OSError, ValueError) as e:
        print(f"java-default-super: {e}", file=sys.stderr)
        sys.exit(2)
