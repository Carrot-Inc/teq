#!/usr/bin/env python3
"""The layout programs: small generated programs that tell one reading of a layout from another by what they print.

  layout.py [--teq BIN] [--jobs N]            every program under `BIN interp`, held to tests/parser/layout.tsv:
                                              a program is accepted with its line or refused, as recorded
  layout.py --record [--teq BIN]              writes the binary's verdicts into tests/parser/layout.tsv (scalac's
                                              column kept)
  layout.py --generate DIR                    writes every program to DIR/<name>.scala
  layout.py --diff A B [--out FILE]           two binaries' verdicts: (1) both accept and print otherwise, (2) B
                                              accepts where A refuses, (3) B refuses where A accepts
  layout.py --scalac [--only FILE]            scalac's verdicts (scala-cli, the version below; the programs of FILE,
                                              one name a line, or all) into tests/parser/layout.tsv's third column

A program is a function of the tables below and SEED alone: a frame (the construct that opens an indented block: a
right side, an `if`'s branch, an `inline if`'s, a `try`'s body, a `catch`'s cases, a `yield`, a lambda's body, an
argument after a colon, a named argument, a case's body under cases indented below their `match`, at the `match`'s own
width and in braces, a function of cases), the block's form (one expression, two statements, braces on the opener's
line, braces on a line of their own), what follows the block (nothing, a selection, an operator that leads its line, a
`match` with its cases indented or at its own width, a parenthesis) and the width of the line that follows (the
opener's, one column in, between the opener's and the block's, the block's, left of the opener's, left of that). Each
such program is written once plain and once with a variation a splitmix64 generator seeded with SEED and the
program's name picks: a blank line, a comment line or both before the line that follows, or that line indented with
tabs. Then the extension headers (one line and several, after a line that ends in an operand, in a postfix operator,
after a self type's arrow; their methods indented at each width, in braces on the header's line and on the next) and a
few programs of a brace after a selected operator. Nothing here depends on teq or on Python's own random numbers.

Every program defines the same small world (a value `V` whose every operation returns a `V` that names the operation,
the same operations on any value, `tag` for what a value is, `safe` for a call that may throw: `!` and the
exception's class in the place of its value) so that each reading of a layout types and prints another line. A
verdict is `=<line>` for a program that runs, `refused` for one the compiler rejects with an error, `exit<n>` for a
run that ends otherwise.

tests/parser/layout.tsv: `name<TAB>teq<TAB>scalac`, scalac's verdict `-` where it was not asked. The test holds teq's
verdicts; where the two columns differ the line is a departure from scalac that the suite records as it is.
"""
import concurrent.futures
import os
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RECORD = os.path.join(ROOT, "tests", "parser", "layout.tsv")
SEED = 0x7E9_2026_1116
SCALA = "3.8.4"
MASK = (1 << 64) - 1


def splitmix64(x):
    x = (x + 0x9E3779B97F4A7C15) & MASK
    z = x
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
    return z ^ (z >> 31)


def pick(name, n):
    """A number below n, from SEED and the name."""
    x = SEED
    for b in name.encode():
        x = splitmix64(x ^ b)
    return splitmix64(x) % n


WORLD = '''  final class V(val s: String):
    def sel: V = V("sel(" + s + ")")
    def post: V = V("post(" + s + ")")
    infix def op(o: V): V = V("(" + s + " op " + o.s + ")")
    def |+|(o: V): V = V("(" + s + "+" + o.s + ")")
    def apply(o: V): V = V(s + "(" + o.s + ")")
  def tag(a: Any): String = a match
    case v: V => v.s
    case l: List[?] => l.map(tag).mkString("[", ",", "]")
    case _: Function1[?, ?] => "fn"
    case u: Unit => "unit"
    case other => other.toString
  extension (a: Any)
    def sel: V = V("sel(" + tag(a) + ")")
    def |+|(o: V): V = V("(" + tag(a) + "+" + o.s + ")")
  def id(a: Any): Any = a
  def safe(a: => Any): String = try { tag(a) } catch { case e: Throwable => "!" + e.getClass.getName }
'''

DEF = (6, "def t: Any =")
DEFC = (6, "def t(c: Boolean): Any =")
DEFN = (6, "def t(n: Int): Any =")
THROW = (8, 'try throw new RuntimeException("x")')
RUN_T = "safe(Outer.Q.t)"
RUN_C = 'safe(Outer.Q.t(true)) + "|" + safe(Outer.Q.t(false))'
RUN_N = 'safe(Outer.Q.t(1)) + "|" + safe(Outer.Q.t(2))'


def widths(kw, blk, *left):
    w = {"kw": kw, "in1": kw + 1, "mid": (kw + blk) // 2, "blk": blk}
    for i, l in enumerate(left):
        w["left" if i == 0 else "left2"] = l
    return w


# A frame: its opening lines (width, text), the width of the block's statements, the widths of the line that follows
# by name, the lines after it, what the program prints, and the expression the block holds.
FRAMES = {
    "def": ([DEF], 10, widths(6, 10, 4), [], RUN_T, 'V("a")'),
    "val": ([(6, "val t: Any =")], 10, widths(6, 10, 4), [], RUN_T, 'V("a")'),
    "else": ([DEFC, (8, 'if c then V("t")'), (8, "else")], 12, widths(8, 12, 6, 4), [], RUN_C, 'V("a")'),
    "then": ([DEFC, (8, "if c then")], 12, widths(8, 12, 6, 4), [(8, 'else V("e")')], RUN_C, 'V("a")'),
    "inline": ([(6, "inline def t(inline c: Boolean): Any ="), (8, 'inline if c then V("t")'), (8, "else")], 12, widths(8, 12, 6, 4), [], RUN_C, 'V("a")'),
    "try": ([DEF, (8, "try")], 12, widths(8, 12, 6, 4), [(8, 'catch case e: RuntimeException => V("caught")')], RUN_T, 'V("a")'),
    "catch1": ([DEF, THROW, (8, "catch case e: RuntimeException =>")], 12, widths(8, 12, 6, 4), [], RUN_T, 'V("a")'),
    "catch": ([DEF, THROW, (8, "catch"), (10, "case e: RuntimeException =>")], 14, widths(10, 14, 8, 6), [], RUN_T, 'V("a")'),
    "catchw": ([DEF, THROW, (8, "catch"), (8, "case e: RuntimeException =>")], 12, widths(8, 12, 6, 4), [], RUN_T, 'V("a")'),
    "yield": ([DEF, (8, 'for x <- List(V("a"), V("b")) yield')], 12, widths(8, 12, 6, 4), [], RUN_T, "x"),
    "lambda": ([(6, "val t = (x: V) =>")], 10, widths(6, 10, 4), [], 'safe(Outer.Q.t(V("arg")))', "x"),
    "colon": ([(6, "def t: Any = id:")], 10, widths(6, 10, 4), [], RUN_T, 'V("a")'),
    "named": ([(6, "def t: Any = id(a =")], 10, widths(6, 10, 4), [(6, ")")], RUN_T, 'V("a")'),
    "match": ([DEFN, (8, "n match"), (10, 'case 1 => V("one")'), (10, "case _ =>")], 14, widths(10, 14, 8, 6), [], RUN_N, 'V("a")'),
    "match1": ([DEFN, (8, "n match"), (10, "case 1 =>")], 14, widths(10, 14, 8, 6), [(10, 'case _ => V("other")')], RUN_N, 'V("a")'),
    "matchw": ([DEFN, (8, "n match"), (8, 'case 1 => V("one")'), (8, "case _ =>")], 12, widths(8, 12, 6, 4), [], RUN_N, 'V("a")'),
    "matchw1": ([DEFN, (8, "n match"), (8, "case 1 =>")], 12, widths(8, 12, 6, 4), [(8, 'case _ => V("other")')], RUN_N, 'V("a")'),
    "matchb": ([DEFN, (8, "n match {"), (10, 'case 1 => V("one")'), (10, "case _ =>")], 14, widths(10, 14, 8), [(8, "}")], RUN_N, 'V("a")'),
    "matchb1": ([DEFN, (8, "n match {"), (10, "case 1 =>")], 14, widths(10, 14, 8), [(10, 'case _ => V("other")'), (8, "}")], RUN_N, 'V("a")'),
    "cases": ([(6, "def t: Any = List(1, 2).map:"), (8, "case 1 =>")], 12, widths(8, 12, 6, 4), [(8, 'case _ => V("other")')], RUN_T, 'V("a")'),
    "matchwif": ([DEFN, (8, "n match"), (8, 'case 1 => V("one")'), (8, "case _ => if true then")], 12, widths(8, 12, 6, 4), [], RUN_N, 'V("a")'),
    "matchwelse": ([DEFN, (8, "n match"), (8, 'case 1 => V("one")'), (8, 'case _ => if n > 5 then V("t") else')], 12, widths(8, 12, 6, 4), [], RUN_N, 'V("a")'),
    "matchwyield": ([DEFN, (8, "n match"), (8, 'case 1 => V("one")'), (8, 'case _ => for x <- List(V("a"), V("b")) yield')], 12, widths(8, 12, 6, 4), [], RUN_N, "x"),
}
BLOCKS = ["one", "two", "inbrace", "ownbrace"]
MATCH_CASES = ['case o if tag(o) == "x" => V("mx")', 'case o => V("m(" + tag(o) + ")")']
FOLLOWS = {
    "sel": lambda w: [(w, ".sel")],
    "plus": lambda w: [(w, '|+| V("z")')],
    "match": lambda w: [(w, "match")] + [(w + 2, c) for c in MATCH_CASES],
    "matchw": lambda w: [(w, "match")] + [(w, c) for c in MATCH_CASES],
    "paren": lambda w: [(w, '(V("p"))')],
}
VARIATIONS = ["blank", "comment", "both", "tabs"]


def line(width, text, tabs=False):
    return ("\t" * width if tabs else " " * width) + text


def program(index, body, run, postfix=False):
    """The whole source: the world, the frame under `Outer.Q`, and what is printed."""
    head = "import scala.language.postfixOps\n" if postfix else ""
    return (f"{head}object P{index}:\n{WORLD}  object Outer:\n    object Q:\n" + "\n".join(body)
            + f"\n  def run(): String = {run}\n@main def main{index}(): Unit = println(P{index}.run())\n")


def frame_program(index, frame, block, follow, width, variation):
    opening, blk, ws, after, run, value = FRAMES[frame]
    lines = [line(w, t) for w, t in opening]
    if block == "one":
        lines.append(line(blk, value))
    elif block == "two":
        lines += [line(blk, f"val s = {value}"), line(blk, "s")]
    elif block == "inbrace":
        lines[-1] += " { " + value + " }"
    else:
        lines.append(line(blk, "{ " + value + " }"))
    if follow is not None:
        w = ws[width]
        if variation in ("comment", "both"):
            lines.append(line(w, "// a comment"))
        if variation in ("blank", "both"):
            lines.append("")
        rest = FOLLOWS[follow](w)
        lines.append(line(rest[0][0], rest[0][1], tabs=variation == "tabs"))
        lines += [line(w2, t) for w2, t in rest[1:]]
    lines += [line(w, t) for w, t in after]
    return program(index, lines, run)


HEADERS = {
    "one": [(6, "extension (x: V)")],
    "lines": [(6, "extension ("), (10, "x: V"), (6, ")")],
    "margin": [(6, "extension ("), (10, "x: V"), (0, ")")],
    "bracket": [(6, "extension [T <: V]("), (10, "x: T"), (6, ")")],
}
BEFORE = {
    "first": [],
    "operand": [(6, 'val a0: Any = V("x")')],
    "postfix": [(6, 'val a0: Any = V("x") post')],
    "self": None,
}
SEPARATIONS = {"none": [], "blank": [""], "comment": [line(6, "// a comment")], "both": [line(6, "// a comment"), ""]}
METHOD = "def m: V = x.sel"
METHODS = {
    "at6": [(6, METHOD)], "at7": [(7, METHOD)], "at8": [(8, METHOD)], "at10": [(10, METHOD)], "at4": [(4, METHOD)],
    "blank8": [None, (8, METHOD)], "blank10": [None, (10, METHOD)],
    "brace": "{ " + METHOD + " }",
    "brace6": [(6, "{ " + METHOD + " }")], "brace8": [(8, "{ " + METHOD + " }")], "brace4": [(4, "{ " + METHOD + " }")],
    "blankbrace6": [None, (6, "{ " + METHOD + " }")],
}
RUN_M = '{ import Outer.Q.*; safe(V("r").m) }'


def extension_program(index, header, before, separation, methods):
    lines = []
    if before == "self":
        # The frame's object becomes a class with a self type, its extension's method read through an instance.
        lines.append(line(6, "class C:"))
        lines.append(line(8, "self =>"))
        shift = 2
    else:
        lines += [line(w, t) for w, t in BEFORE[before]]
        shift = 0
    if before != "first":
        lines += [s if s == "" else " " * shift + s for s in SEPARATIONS[separation]]
    head = [line(w + (shift if w else 0), t) for w, t in HEADERS[header]]
    m = METHODS[methods]
    if isinstance(m, str):
        head[-1] += " " + m
        lines += head
    else:
        lines += head
        lines += ["" if e is None else line(e[0] + shift, e[1]) for e in m]
    if before == "self":
        lines.append(line(8, 'def got: V = V("r").m'))
        return program(index, lines, "safe(new Outer.Q.C().got)")
    return program(index, lines, RUN_M, postfix=before == "postfix")


OTHERS = {
    "dotop_brace": ([(6, 'def t: Any = V("l").|+| { V("r") }')], RUN_T),
    "op_brace": ([(6, 'def t: Any = V("l") |+| { V("r") }')], RUN_T),
    "dotop_brace_lines": ([(6, 'def t: Any = V("l").|+| {'), (8, 'V("r")'), (6, "}")], RUN_T),
    "dotname_brace": ([(6, 'def t: Any = V("l").op { V("r") }')], RUN_T),
    "dotop_brace_two": ([(6, 'def t: Any = V("l").|+| { val r = V("r"); r }')], RUN_T),
    "dotop_brace_sel": ([(6, 'def t: Any = V("l").|+| { V("r") }.sel')], RUN_T),
    "dotop_brace_next": ([(6, 'def t: Any = V("l").|+|'), (6, '{ V("r") }')], RUN_T),
    "dotop_brace_next_in": ([(6, 'def t: Any = V("l").|+|'), (8, '{ V("r") }')], RUN_T),
    "postfix_val": ([(6, 'val t: Any = V("x") post'), (6, 'val u: Any = V("u")')], RUN_T),
    "postfix_blank_val": ([(6, 'val t: Any = V("x") post'), "", (6, 'val u: Any = V("u")')], RUN_T),
    "postfix_stmt": ([(6, 'val t: Any = V("x") post'), (6, 'V("u").sel')], RUN_T),
    "postfix_blank_stmt": ([(6, 'val t: Any = V("x") post'), "", (6, 'V("u").sel')], RUN_T),
    "postfix_comment_val": ([(6, 'val t: Any = V("x") post'), (6, "// a comment"), (6, 'val u: Any = V("u")')], RUN_T),
    "postfix_both_stmt": ([(6, 'val t: Any = V("x") post'), (6, "// a comment"), "", (6, 'V("u").sel')], RUN_T),
}


def programs():
    """Every program, in a fixed order: (name, source)."""
    out = []

    def add(name, make):
        out.append((name, make(len(out))))

    for frame, (_, _, ws, _, _, _) in FRAMES.items():
        for block in BLOCKS:
            # Braces on the opener's line stand after a token that ends the line; a colon's argument has its own form.
            if block == "inbrace" and frame in ("colon",):
                continue
            base = f"{frame}.{block}"
            add(base + ".plain", lambda i, f=frame, b=block: frame_program(i, f, b, None, None, None))
            for follow in FOLLOWS:
                for width in ws:
                    name = f"{base}.{follow}.{width}"
                    add(name, lambda i, f=frame, b=block, fo=follow, w=width: frame_program(i, f, b, fo, w, None))
                    variation = VARIATIONS[pick(name, len(VARIATIONS))]
                    add(f"{name}.{variation}", lambda i, f=frame, b=block, fo=follow, w=width, v=variation: frame_program(i, f, b, fo, w, v))
    for header in HEADERS:
        for before in BEFORE:
            for separation in SEPARATIONS:
                if before == "first" and separation != "none":
                    continue
                for methods in METHODS:
                    add(f"ext.{header}.{before}.{separation}.{methods}",
                        lambda i, h=header, b=before, s=separation, m=methods: extension_program(i, h, b, s, m))
    for name, (lines, run) in OTHERS.items():
        add("other." + name, lambda i, ls=lines, r=run, p=name.startswith("postfix"): program(i, [l if isinstance(l, str) else line(*l) for l in ls], r, postfix=p))
    return out


def verdict_of(code, text, said):
    if code == 0:
        return "=" + " ".join(text.split("\n")).strip()
    if code == 1 and ": error: " in said:
        return "refused"
    return f"exit{code}"


def run_teq(teq, jobs):
    """Each program's verdict under `teq interp`, by name."""
    work = tempfile.mkdtemp(prefix="teq-layout-")
    try:
        progs = programs()
        for name, source in progs:
            with open(os.path.join(work, name + ".scala"), "w") as f:
                f.write(source)

        def one(name):
            env = {k: v for k, v in os.environ.items() if k not in ("TEQ", "TEQ_REAL")}
            try:
                r = subprocess.run([teq, "interp", os.path.join(work, name + ".scala")], stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, timeout=60, env=env)
                return name, verdict_of(r.returncode, r.stdout.decode(errors="replace"), r.stderr.decode(errors="replace"))
            except subprocess.TimeoutExpired:
                return name, "timeout"

        with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
            return dict(pool.map(one, [n for n, _ in progs]))
    finally:
        shutil.rmtree(work, ignore_errors=True)


def read_record():
    rows = {}
    if os.path.exists(RECORD):
        for l in open(RECORD):
            if l.startswith("#") or not l.strip():
                continue
            name, teq, scalac = l.rstrip("\n").split("\t")
            rows[name] = [teq, scalac]
    return rows


def write_record(rows):
    with open(RECORD, "w") as f:
        f.write("# name\tteq\tscalac (`-`: not asked)\n")
        for name, _ in programs():
            teq, scalac = rows.get(name, ["-", "-"])
            f.write(f"{name}\t{teq}\t{scalac}\n")


ERROR_AT = re.compile(r"^-- .*Error: (?:.*/)?([A-Za-z0-9_.]+)\.scala:\d+:\d+")


def run_scalac(names):
    """scalac's verdict of each named program: batches compiled together until no file has an error, then run once."""
    sources = dict(programs())
    index = {name: i for i, (name, _) in enumerate(programs())}
    verdicts = {}
    names = list(names)
    for at in range(0, len(names), 120):
        batch = names[at:at + 120]
        work = tempfile.mkdtemp(prefix="teq-layout-scalac-")
        try:
            for name in batch:
                with open(os.path.join(work, f"L{index[name]}.scala"), "w") as f:
                    f.write(sources[name])
            by_file = {f"L{index[name]}": name for name in batch}
            alive = set(batch)
            common = ["scala-cli", "--power"]
            flags = ["-S", SCALA, "--server=false", "-q", "--jvm", "system"]
            for _ in range(8):
                if not alive:
                    break
                r = subprocess.run(common + ["compile"] + flags + [work], stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=900)
                text = re.sub(r"\x1b\[[0-9;]*m", "", r.stdout.decode(errors="replace"))
                bad = {by_file[m.group(1)] for m in map(ERROR_AT.match, text.split("\n")) if m and m.group(1) in by_file}
                if r.returncode == 0:
                    break
                if not bad:
                    raise RuntimeError("scalac failed with no error of a program:\n" + text[-2000:])
                for name in bad:
                    verdicts[name] = "refused"
                    alive.discard(name)
                    os.remove(os.path.join(work, f"L{index[name]}.scala"))
            if alive:
                calls = "\n".join(f'  try println("{index[n]}\\t" + P{index[n]}.run()) catch case e: Throwable => println("{index[n]}\\t!!" + e.getClass.getName)' for n in sorted(alive, key=index.get))
                with open(os.path.join(work, "All.scala"), "w") as f:
                    f.write(f"@main def all(): Unit =\n{calls}\n")
                r = subprocess.run(common + ["run"] + flags + ["--main-class", "all", work], stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=900)
                by_index = {index[n]: n for n in alive}
                for l in r.stdout.decode(errors="replace").split("\n"):
                    if "\t" in l and l.split("\t")[0].isdigit() and int(l.split("\t")[0]) in by_index:
                        verdicts[by_index[int(l.split("\t")[0])]] = "=" + l.split("\t", 1)[1].strip()
                for n in alive:
                    verdicts.setdefault(n, "unknown")
        finally:
            shutil.rmtree(work, ignore_errors=True)
        print(f"scalac: {min(at + 120, len(names))} of {len(names)}", flush=True)
    return verdicts


def arg(flag, default=None):
    return sys.argv[sys.argv.index(flag) + 1] if flag in sys.argv else default


def main():
    jobs = int(arg("--jobs", str(os.cpu_count() or 4)))
    teq = arg("--teq", os.environ.get("TEQ", os.path.join(ROOT, "target", "release", "teq")))
    if "--generate" in sys.argv:
        d = arg("--generate")
        os.makedirs(d, exist_ok=True)
        for name, source in programs():
            with open(os.path.join(d, name + ".scala"), "w") as f:
                f.write(source)
        print(f"layout: {len(programs())} programs")
        return
    if "--diff" in sys.argv:
        i = sys.argv.index("--diff")
        a, b = run_teq(sys.argv[i + 1], jobs), run_teq(sys.argv[i + 2], jobs)
        classes = {1: [], 2: [], 3: []}
        for name, _ in programs():
            va, vb = a[name], b[name]
            if va == vb:
                continue
            if va.startswith("=") and vb.startswith("="):
                classes[1].append(name)
            elif vb.startswith("="):
                classes[2].append(name)
            elif va.startswith("="):
                classes[3].append(name)
            else:
                classes.setdefault(0, []).append(name)
        out = arg("--out")
        if out:
            with open(out, "w") as f:
                for c, names in sorted(classes.items()):
                    for n in names:
                        f.write(f"{c}\t{n}\t{a[n]}\t{b[n]}\n")
        same = sum(1 for n, _ in programs() if a[n] == b[n])
        print(f"layout: {len(a)} programs; the same verdict on both: {same}; (1) both accept, another line: {len(classes[1])}; "
              f"(2) the second accepts where the first refuses: {len(classes[2])}; (3) the second refuses where the first accepts: {len(classes[3])}; "
              f"otherwise (neither accepts, another failure): {len(classes.get(0, []))}")
        return
    rows = read_record()
    if "--scalac" in sys.argv:
        only = arg("--only")
        names = [l.strip() for l in open(only) if l.strip()] if only else [n for n, _ in programs()]
        for name, v in run_scalac(names).items():
            rows.setdefault(name, ["-", "-"])[1] = v
        write_record(rows)
        return
    got = run_teq(teq, jobs)
    if "--record" in sys.argv:
        for name, v in got.items():
            rows.setdefault(name, ["-", "-"])[0] = v
        write_record(rows)
        print(f"layout: {len(got)} programs recorded")
        return
    wrong = [(n, rows.get(n, ["-", "-"])[0], v) for n, v in sorted(got.items()) if rows.get(n, ["-", "-"])[0] != v]
    for n, want, v in wrong[:40]:
        print(f"FAIL layout {n}: recorded {want}, now {v}")
    asked = [n for n in got if rows.get(n, ["-", "-"])[1] != "-"]
    as_scalac = sum(1 for n in asked if rows[n][1] == got[n])
    print(f"layout: {len(got) - len(wrong)} of {len(got)} programs as recorded; of the {len(asked)} scalac was asked, {as_scalac} as scalac")
    sys.exit(1 if wrong else 0)


if __name__ == "__main__":
    main()
