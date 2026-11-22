#!/usr/bin/env python3
"""quotes.py: the table of the `Quotes` a macro's pickled trees select the reflection API on, where the TASTy writer
reads the source's text to choose it (tests/init-view/quotes.sh runs it): the first import of the reflection API in a
definition's text (pickle/reflect.rs `body_reflect_import`), and the `<name>.reflect` of a parameter's written type or
of an import (pickle/reflect.rs `declared_quotes`). scalac's typer has the import's or the type's prefix as a tree.

  quotes.py generate <dir>   writes per cell `<cell>_1.scala`, the macro, and `<cell>_2.scala`, its client
  quotes.py judge <corpus work dir> <run dir>
  quotes.py --self-test      the judge on planted records (`self_test`)

Each cell's macro implementation takes two `Quotes` parameters, `a` the expansion's own and `b` null, and selects the
reflection API on `a`, written plain and with what stands around the choice varied: a comment or a string holding
`import b.reflect` or `b.reflect` before the real one, an unrelated import before it, a blank, a line break or a
backquote in the import or the type, the varied import second after a clean one, `a` a given and `b` not, a
lambda's parameter typed on `a` in a body that imports nothing. One cell selects it on `b`, which scalac's
expansion fails on (a null `Quotes`): the control of what a run can tell.

`judge` reads per cell the view of scalac's own pickle of the macro file alone and the view of teq's
(tests/init-view-corpus.sh's `scalac-view.dump` and `view.dump`): the parameters `a`, `b` and `q` the identifiers in
the trees name, in order, are the cell's verdict (equal, or the two lists). The view prints no type tree's insides, so
the `Quotes` the parameters' types name (`t: a.reflect.Term`) are read from the two pickles' signatures as `teq tasty
--full` prints them. Then the run: each compiler compiled the macro, then its client against it, and ran `Test`;
scalac's output or first error against teq's, the control's declared (EXPECTED_RUN). Prints the table, a line per
cell, and the counts; exits 1 on a cell that differs or is not judged (a record missing, empty, not what its sum says
or past its bound, tests/init-view/records.py) and on run records that do not match the cells."""
import os
import re
import sys

HEADER = "import scala.quoted.*\n\nobject Macro:\n  inline def m: String = ${ outer }\n"
CLIENT = "object Test:\n  def main(args: Array[String]): Unit =\n    println(Macro.m)\n"

# The body of an implementation that selects the reflection API by an import (`{i}` the lines that hold it), and of
# one that takes a term whose type names the `Quotes` (`{p}` its parameters after the two).
IMPORTED = ("  def outer(using q: Quotes): Expr[String] = pick(q, null)\n"
            "  def pick(a: Quotes, b: Quotes): Expr[String] =\n"
            "    {i}\n"
            "    given Quotes = a\n"
            "    Literal(StringConstant(\"owner \" + Symbol.spliceOwner.name)).asExprOf[String]\n")
GIVEN = ("  def outer(using q: Quotes): Expr[String] = pick(null)(using q)\n"
         "  def pick(b: Quotes)(using a: Quotes): Expr[String] =\n"
         "    {i}\n"
         "    Literal(StringConstant(\"owner \" + Symbol.spliceOwner.name)).asExprOf[String]\n")
NESTED = ("  def outer(using q: Quotes): Expr[String] = pick(q, null)\n"
          "  def pick(a: Quotes, b: Quotes): Expr[String] =\n"
          "    given Quotes = a\n"
          "    val r =\n"
          "      {i}\n"
          "      Literal(StringConstant(\"owner \" + Symbol.spliceOwner.name)).asExprOf[String]\n"
          "    r\n")
TYPED = ("  def outer(using q: Quotes): Expr[String] =\n"
         "    import q.reflect.*\n"
         "    pick(q, null)(Literal(StringConstant(\"lit\")))\n"
         "  def pick(a: Quotes, b: Quotes)({p}): Expr[String] =\n"
         "    given Quotes = a\n"
         "    t.asExprOf[String]\n")
# One whose lambda's parameter names the `Quotes` in its written type (`{p}` the parameter) and nothing else does: the
# body imports nothing and the method's parameters are `Quotes` alone. The writer writes a closure's parameters with no
# read of their types (`local_param`), so its choice there is the innermost `Quotes` parameter (`innermost_quotes`).
LAMBDA = ("  def outer(using q: Quotes): Expr[String] = pick(q, null)\n"
          "  def pick(a: Quotes, b: Quotes): Expr[String] =\n"
          "    given Quotes = a\n"
          "    val f = ({p}) => t.asExprOf[String]\n"
          "    f(a.reflect.Literal(a.reflect.StringConstant(\"lit\")))\n")

# Each cell: its name (the read it varies first), the body, and what stands in it.
CELLS = [
    ("s16-import", IMPORTED, "import a.reflect.*"),
    ("s16-import-line-comment", IMPORTED, "// import b.reflect.*\n    import a.reflect.*"),
    ("s16-import-block-comment", IMPORTED, "/* import b.reflect.* */\n    import a.reflect.*"),
    ("s16-import-string", IMPORTED, "val note = \"import b.reflect\"\n    import a.reflect.*"),
    ("s16-import-after-another", IMPORTED, "import scala.collection.mutable\n    import a.reflect.*"),
    ("s16-import-blank", IMPORTED, "import  a.reflect.*"),
    ("s16-import-newline", IMPORTED, "import\n      a.reflect.*"),
    ("s16-import-backquoted", IMPORTED, "import `a`.reflect.*"),
    ("s16-import-selectors", IMPORTED, "import a.reflect.{Literal, StringConstant, Symbol}"),
    ("s16-import-given", GIVEN, "import a.reflect.*"),
    ("s16-import-given-line-comment", GIVEN, "// import b.reflect.*\n    import a.reflect.*"),
    ("s16-import-nested", NESTED, "import a.reflect.*"),
    ("s16-import-control-null", IMPORTED, "import b.reflect.*"),
    ("s15-import-block-comment", IMPORTED, "import /* b.reflect */ a.reflect.*"),
    ("s15-import-trailing-comment", IMPORTED, "import a.reflect.* // b.reflect"),
    ("s15-import-newline-dot", IMPORTED, "import a\n      .reflect.*"),
    # The body's first import clean, so that its read names `a`: the second import's own read alone varied.
    ("s15-import-second", IMPORTED, "import a.reflect.Symbol\n    import a.reflect.*"),
    ("s15-import-second-block-comment", IMPORTED, "import a.reflect.Symbol\n    import /* b.reflect */ a.reflect.*"),
    ("s15-import-second-newline-dot", IMPORTED, "import a.reflect.Symbol\n    import a\n      .reflect.*"),
    ("s15-import-second-backquoted", IMPORTED, "import a.reflect.Symbol\n    import `a`.reflect.*"),
    ("s15-param", TYPED, "t: a.reflect.Term"),
    ("s15-param-block-comment", TYPED, "t: /* b.reflect */ a.reflect.Term"),
    ("s15-param-line-comment", TYPED, "t: // b.reflect\n      a.reflect.Term"),
    ("s15-param-newline", TYPED, "t:\n      a.reflect.Term"),
    ("s15-param-blank", TYPED, "t: a .reflect.Term"),
    ("s15-param-backquoted", TYPED, "t: `a`.reflect.Term"),
    ("s15-param-default-string", TYPED, "t: a.reflect.Term, s: String = \"b.reflect\""),
    ("s15-lambda-param", LAMBDA, "t: a.reflect.Term"),
    ("s15-lambda-param-block-comment", LAMBDA, "t: /* b.reflect */ a.reflect.Term"),
    ("s15-lambda-param-newline", LAMBDA, "t:\n      a.reflect.Term"),
    ("s15-lambda-param-blank", LAMBDA, "t: a .reflect.Term"),
    ("s15-lambda-param-backquoted", LAMBDA, "t: `a`.reflect.Term"),
]


def cells():
    out = []
    for name, body, text in CELLS:
        out.append((name, HEADER + body.replace("{i}", text).replace("{p}", text)))
    return out


def generate(d):
    os.makedirs(d, exist_ok=True)
    for f in os.listdir(d):
        if f.endswith(".scala"):
            os.remove(os.path.join(d, f))
    for name, src in cells():
        with open(os.path.join(d, name + "_1.scala"), "w", encoding="utf-8") as f:
            f.write(src)
        with open(os.path.join(d, name + "_2.scala"), "w", encoding="utf-8") as f:
            f.write(CLIENT)
    print(f"quotes: {len(cells())} cells in {d}")


IDENT = re.compile(r"^\s*Ident \[[^\]]*\] sym=(\S+)")

# A cell whose run is to differ, and why: what scalac's run must do there instead of equalling teq's.
EXPECTED_RUN = {
    "s16-import-control-null": "scalac's expansion fails on the null `Quotes` its pickle selects the reflection API on, "
                               "where teq's expansion runs its own code: the control of what a run can tell",
}


def quotes_named(lines):
    """The parameters `a`, `b`, `q` the identifiers of a view's lines name, in order."""
    out = []
    for line in lines:
        m = IDENT.match(line)
        if m and re.search(r"[{,]param[,}]", line):
            last = m.group(1).rsplit(".", 1)[-1]
            if last in ("a", "b", "q"):
                out.append(last)
    return out


REFLECT = re.compile(r"(\w+)\.reflect\.")


def param_quotes(lines):
    """The `Quotes` the parameter types of the implementation `pick` select the reflection API on (`t: a.reflect.Term`),
    in order, from a pickle's signatures as `teq tasty --full` prints them; None where no `pick` is there."""
    for line in lines:
        if line.strip().startswith("def pick("):
            return REFLECT.findall(line.rsplit("): ", 1)[0])
    return None


def outcome(lines):
    """A run's output: the program's lines, or the first error of a compiler or the program, on one line."""
    lines = [l.rstrip() for l in lines if l.strip()]
    err = next((l for l in lines if re.search(r"Error|Exception|error:|^-- ", l)), None)
    if err:
        why = next((l.strip() for l in lines if re.search(r"Exception:|java\.lang\.", l)), err.strip())
        return "FAILS: " + why.lstrip("| ")[:160].replace("|", "/")
    return " / ".join(lines).replace("|", "/") if lines else "nothing"


def ran(d, side):
    """('ok', exit, outcome) of one compiler's run of a cell (`<side>.out`, its exit `<side>.exit`, the last step's), or
    ('not judged', why). The exit by the whitelist (records.py's `judged_exit`): 0, or the compiler's refusal of the macro
    or the client named by its line (scalac's 1 with its error line; teq's 1 with its error line or the TASTy writer's
    summary, its 3 with the JVM backend's), or 1 with the program's uncaught exception (the run fails); any other exit
    (a signal's, 2, 124, none, 1 or 3 without its line) is not judged."""
    import records as rec
    own = rec.SCALAC_REFUSALS if side == "scalac" else rec.TEQ_REFUSALS
    refusals = {e: ps + ((rec.PROGRAM_FAILURE,) if e == 1 else ()) for e, ps in own.items()}
    state, why = rec.judged_exit(os.path.join(d, side + ".exit"), os.path.join(d, side + ".out"), f"{side}'s run", refusals)
    if state == "not judged":
        return "not judged", why
    lines, why = rec.text(os.path.join(d, side + ".out"))
    if why:
        return "not judged", f"{side}'s run's output {why}"
    return "ok", 0 if state == "ok" else 1, outcome(lines)


def macro_built(d, side):
    """('ok', None), ('refused', why) or ('not judged', why) of one compiler's build of the macro file alone, its own
    step (`<side>-macro.exit`, `<side>-macro.out`): a refusal of a cell scalac refuses is this step's, by its line, never
    a later step's failure (the client's build, the program's exception)."""
    import records as rec
    own = rec.SCALAC_REFUSALS if side == "scalac" else rec.TEQ_REFUSALS
    return rec.judged_exit(os.path.join(d, side + "-macro.exit"), os.path.join(d, side + "-macro.out"), f"{side}'s build of the macro", own)


def judge(work, run):
    """The table (`judge <corpus work dir> <run dir>`): per cell the trees, the parameter types and the run, each
    `equal`, `DIFFERS`, a refusal both compilers make, or NOT JUDGED where a record is missing, empty, not what its sum
    says (another cell's, a line cut off its end or out of its middle) or past its bound (tests/init-view/records.py). Exits 1 on a cell that differs or is not judged, a cell whose run differs
    otherwise than EXPECTED_RUN declares, or run records that do not match the cells."""
    import records as rec
    print("| cell | scalac's trees name | teq's trees name | trees | scalac's parameter types name | teq's | parameter "
          "types | scalac's run | teq's run | run |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    problems = rec.manifest([n for n, _ in cells()], sorted(os.listdir(run)) if os.path.isdir(run) else [])
    counts, bad = {}, 0
    show = lambda xs: " ".join(xs) if xs is not None else "none"
    for name in dict.fromkeys(n for n, _ in cells()):
        u, d = os.path.join(work, name), os.path.join(run, name)
        unit = rec.corpus_unit(u)
        (ss, sv), (bs, bv) = unit["scalac"], unit["base"]
        o = t = po = pt = None
        if ss == "not judged":
            trees = params = "NOT JUDGED: " + sv
        elif ss == "refused":
            trees = params = "scalac refuses"
        elif bs == "not judged":
            trees = params = "NOT JUDGED: " + bv
        elif bs == "refused":
            trees = params = "DIFFERS: teq refuses what scalac compiles"
        else:
            o, t = quotes_named(sv), quotes_named(bv)
            trees = "equal" if o == t else "DIFFERS"
            sides = []
            for side in ("scalac", "teq"):
                state, val = rec.step(os.path.join(d, side + "-signatures.txt"), os.path.join(d, side + "-signatures.exit"), f"{side}'s signatures")
                q = param_quotes(val) if state == "ok" else None
                sides.append((state, val, q))
            (s1, v1, po), (s2, v2, pt) = sides
            if s1 != "ok" or s2 != "ok":
                params = "NOT JUDGED: " + (v1 if s1 != "ok" else v2)
            elif po is None or pt is None:
                params = "NOT JUDGED: no signature of `pick`"
            else:
                params = "equal" if po == pt else "DIFFERS"
        rs = rt = ("none",)
        if not os.path.isdir(d):
            runv = "NOT JUDGED: no record of the run"
        elif os.path.exists(os.path.join(d, "not-judged")):
            runv = "NOT JUDGED: " + open(os.path.join(d, "not-judged"), encoding="utf-8").read().strip()
        else:
            rs, rt = ran(d, "scalac"), ran(d, "teq")
            if rs[0] != "ok" or rt[0] != "ok":
                runv = "NOT JUDGED: " + (rs[1] if rs[0] != "ok" else rt[1])
            elif ss == "refused":
                (ms, mv), (mt, mtv) = macro_built(d, "scalac"), macro_built(d, "teq")
                if ms == "not judged" or mt == "not judged":
                    runv = "NOT JUDGED: " + (mv if ms == "not judged" else mtv)
                elif mt == "ok":
                    runv = "DIFFERS: teq compiles what scalac refuses"
                elif ms == "ok":
                    runv = "NOT JUDGED: scalac's build of the macro compiles the file its corpus run refused"
                else:
                    runv = "both refuse"
            elif name in EXPECTED_RUN:
                runv = "as declared" if rs[1] != 0 else "DIFFERS: the control's run by scalac does not fail"
            else:
                runv = "equal" if (rs[1] == 0) == (rt[1] == 0) and rs[2] == rt[2] else "DIFFERS"
        for k, v in (("trees", trees), ("parameter types", params), ("run", runv)):
            key = k + " " + (v if not v.startswith("NOT JUDGED") else "NOT JUDGED")
            counts[key] = counts.get(key, 0) + 1
        if any(v.startswith(("NOT JUDGED", "DIFFERS")) for v in (trees, params, runv)):
            bad += 1
        outs = [x[2] if x[0] == "ok" else "none" for x in (rs, rt)]
        print(f"| {name} | {show(o)} | {show(t)} | {trees} | {show(po)} | {show(pt)} | {params} | {outs[0]} | {outs[1]} | {runv} |")
    for p in problems:
        print(f"MANIFEST: {p}")
    print("quotes: " + str(len(cells())) + " cells; " + "; ".join(f"{k}: {n}" for k, n in sorted(counts.items()))
          + f"; {bad} cells that differ or are not judged; {len(problems)} manifest problems")
    return 1 if bad or problems else 0


# The self-test's cells: two judged, the control (its run expected to differ), one both compilers refuse.
FIXTURE = ["s16-import", "s15-param", "s16-import-control-null", "s15-lambda-param-newline"]
VIEW = ("unit Macro_1.scala\nsource s1 name=Macro_1.scala path=Macro_1.scala content=yes sha256=0\n"
        "  DefDef [s1 0..9@4 1:0-1:9] sym=Macro$.pick\n"
        "    Ident [s1 5..6@5 1:5-1:6] sym=Macro$.outer.q {param} pre=none\n"
        "    Ident [s1 7..8@7 1:7-1:8] sym=Macro$.pick.a {param} pre=none\n")
SIGNATURES = ("object Macro\n  def outer(using q: scala.quoted.Quotes): scala.quoted.Expr[String]\n"
              "  def pick(a: scala.quoted.Quotes, b: scala.quoted.Quotes)(t: a.reflect.Term): scala.quoted.Expr[String]\n")
# A run's output as a compiler and the program write it: two lines of the build, then the program's.
RUN = "Compiling Macro_1.scala\nCompiling Macro_2.scala\nowner pick\n"


def self_test():
    """The judge on planted records (`--self-test`): a good run passes; each record missing, empty or cut (scalac's or
    teq's view, signatures, run), a step past its bound or failed, a dump holding its unit twice, and a manifest cell
    with no record are NOT JUDGED; a wrong tree, parameter type or run (each sealed as its producer wrote it) DIFFERS;
    a manifest holding a cell twice and a run record of a cell the manifest does not hold fail; so does an exit other
    than 0, a refusal named by its line (teq's 1 with the TASTy writer's summary and 3 with the JVM backend's among
    them, which pass) or 1 with the program's exception on its record (137, 2, 124, 139, none, 1 with none of those
    lines, 3 without the JVM backend's, scalac's 3), on each side. Each record changed after its producer sealed it (its last line gone with its line break and exit kept, a line gone from its middle, a
    byte changed, its sum gone, another cell's record copied in) is NOT JUDGED for what its sum says."""
    import shutil
    import records as rec
    t = rec.SelfTest("quotes.py")
    real = cells

    def good(root):
        work, run = os.path.join(root, "work"), os.path.join(root, "run")
        for c in FIXTURE:
            u, d = os.path.join(work, c), os.path.join(run, c)
            rec.write(os.path.join(u, "src", "Macro.scala"), "object Macro\n")
            os.makedirs(d, exist_ok=True)
            if c == "s15-lambda-param-newline":
                rec.write(os.path.join(u, "oracle", "exit"), "1\n")
                rec.put(os.path.join(u, "oracle", "scalac.txt"), "-- Error: Macro_1.scala:9:6 \n9 |  x\n1 error found\n", unit=u)
                for side, error in (("scalac", "-- Error: Macro_1.scala:9:6\n"), ("teq", "Macro_1.scala:9:23: error: expected end of statement\n")):
                    for f in (side + "-macro", side):
                        rec.put(os.path.join(d, f + ".out"), error)
                        rec.write(os.path.join(d, f + ".exit"), "1\n")
                continue
            rec.write(os.path.join(u, "oracle", "exit"), "0\n")
            for side in ("scalac-view", "view"):
                rec.put(os.path.join(u, side + ".dump"), VIEW)
                rec.write(os.path.join(u, side + ".exit"), "0\n")
            rec.write(os.path.join(u, "check.exit"), "0\n")
            for side in ("scalac", "teq"):
                rec.put(os.path.join(d, side + "-signatures.txt"), SIGNATURES)
                rec.write(os.path.join(d, side + "-signatures.exit"), "0\n")
                rec.put(os.path.join(d, side + "-macro.out"), "")
                rec.write(os.path.join(d, side + "-macro.exit"), "0\n")
                rec.put(os.path.join(d, side + ".out"), RUN)
                rec.write(os.path.join(d, side + ".exit"), "0\n")
            if c == "s16-import-control-null":
                rec.put(os.path.join(d, "scalac.out"), "Exception in thread \"main\" java.lang.NullPointerException\n")
                rec.write(os.path.join(d, "scalac.exit"), "1\n")
        return work, run

    def case(name, mutate, names=None, want_rc=1, want=None):
        root = t.dir(name)
        work, run = good(root)
        mutate(os.path.join(work, FIXTURE[0]), os.path.join(run, FIXTURE[0]), work, run)
        globals()["cells"] = lambda: [(n, "") for n in (names or FIXTURE)]
        try:
            rc, out = t.quiet(judge, work, run)
        finally:
            globals()["cells"] = real
        rows = {r.split("|")[1].strip(): r for r in out.splitlines() if r.startswith("| ") and r.count("|") >= 10}
        ok = rc == want_rc and (want is None or want(rows))
        t.expect(name, ok, f"exit {rc}; {rows.get(FIXTURE[0], 'no row')}")

    def at(kind, path):
        return lambda u, d: os.path.join({"u": u, "d": d}[kind], path)

    def drop(p):
        return lambda u, d, w, r: os.remove(p(u, d))

    def blank(p):
        # An empty output, as its producer wrote it and sealed it.
        return lambda u, d, w, r: rec.put(p(u, d), "")

    def cut(p):
        def f(u, d, w, r):
            data = open(p(u, d), encoding="utf-8").read()
            rec.write(p(u, d), data[: len(data) // 2])
        return f

    def put(p, content):
        # A record as its producer wrote it: an output sealed, a marker or an exit as it is.
        def f(u, d, w, r):
            path = p(u, d)
            if path.endswith((".dump", ".txt", ".out")):
                rec.put(path, content)
            else:
                rec.write(path, content)
        return f

    row = lambda rows: rows.get(FIXTURE[0], "")
    named = lambda rows: "NOT JUDGED" in row(rows)
    differs = lambda rows: "DIFFERS" in row(rows) and "NOT JUDGED" not in row(rows)
    by_sum = lambda rows: named(rows) and any(w in row(rows) for w in rec.SUM_WHY)
    case("a good run", lambda *a: None, want_rc=0)
    outputs = (("scalac's view", at("u", "scalac-view.dump")), ("teq's view", at("u", "view.dump")),
               ("scalac's signatures", at("d", "scalac-signatures.txt")), ("teq's signatures", at("d", "teq-signatures.txt")),
               ("scalac's run", at("d", "scalac.out")), ("teq's run", at("d", "teq.out")))
    for what, p in outputs:
        case(f"{what} missing", drop(p), want=named)
        case(f"{what} empty", blank(p), want=named)
        case(f"{what} cut", cut(p), want=named)
    for what, p in (("scalac's exit", at("u", "oracle/exit")), ("the view of scalac's pickle's exit", at("u", "scalac-view.exit")),
                    ("the view of teq's products' exit", at("u", "view.exit")), ("scalac's signatures' exit", at("d", "scalac-signatures.exit")),
                    ("teq's signatures' exit", at("d", "teq-signatures.exit")), ("scalac's run's exit", at("d", "scalac.exit")),
                    ("teq's run's exit", at("d", "teq.exit"))):
        case(f"{what} missing", drop(p), want=named)
        case(f"{what} 124", put(p, "124\n"), want=named)
    case("a step past its bound (the NOT JUDGED marker)", put(at("d", "not-judged"), "teq.out step over 120 s\n"), want=named)
    case("a wrong tree", put(at("u", "view.dump"), VIEW.replace("sym=Macro$.pick.a {param}", "sym=Macro$.pick.b {param}")), want=differs)
    case("a wrong parameter type", put(at("d", "teq-signatures.txt"), SIGNATURES.replace("t: a.reflect.Term", "t: b.reflect.Term")), want=differs)
    case("a wrong run", put(at("d", "teq.out"), RUN.replace("owner pick", "owner outer")), want=differs)
    case("a dump holding its unit twice", put(at("u", "view.dump"), VIEW + VIEW), want=named)
    case("a manifest holding a cell twice", lambda *a: None, names=FIXTURE + [FIXTURE[1]])
    case("a run record of a cell the manifest does not hold", lambda u, d, w, r: os.makedirs(os.path.join(r, "s16-stray")))
    case("a manifest cell with no run record", lambda u, d, w, r: shutil.rmtree(d), want=named)
    case("a manifest cell with no record of the corpus run", lambda u, d, w, r: shutil.rmtree(u), want=named)
    # Each output changed after its producer sealed it, the same record of the next cell (sealed by its producer, the
    # same bytes) the other cell's.
    other = lambda path: path.replace(FIXTURE[0], FIXTURE[1])
    for what, p in outputs:
        for plant, mutation in rec.after_the_producer(other):
            case(f"{what}: {plant}", lambda u, d, w, r, p=p, mutation=mutation: mutation(p(u, d)), want=by_sum)
    # A step's exit by the whitelist: 0, or 1 with the compiler's error line (or the program's exception) on its record.
    # A signal's (137 a kill, 139 a segmentation fault), 2, 124, none recorded and 1 with neither are NOT JUDGED: on
    # each side's run of the cell both compilers refuse, on scalac's InitDump of it, and on the judged cell's runs and
    # signatures.
    refused = "s15-lambda-param-newline"

    def exit_plant(kind, cell, f, value, log=None):
        def g(u, d, w, r):
            root = os.path.join({"u": w, "d": r}[kind], cell)
            if value is None:
                os.remove(os.path.join(root, f))
            else:
                rec.write(os.path.join(root, f), value)
            if log:
                rec.put(os.path.join(root, log[0]), log[1], unit=root)
        return g
    named_cell = lambda cell: (lambda rows: "NOT JUDGED" in rows.get(cell, ""))
    bad_exits = (("137 (killed)", "137\n"), ("2", "2\n"), ("124", "124\n"), ("139", "139\n"), ("missing", None))
    for kind, f, log in (("d", "scalac.exit", "scalac.out"), ("d", "teq.exit", "teq.out"), ("u", "oracle/exit", "oracle/scalac.txt")):
        for what, value in bad_exits:
            case(f"{f} of the refused cell {what}", exit_plant(kind, refused, f, value), want=named_cell(refused))
        case(f"{f} of the refused cell 1 with no error line", exit_plant(kind, refused, f, "1\n", (log, "compiling\ndone\n")), want=named_cell(refused))
    for f in ("scalac.exit", "teq.exit", "scalac-signatures.exit", "teq-signatures.exit"):
        for what, value in bad_exits[:2] + bad_exits[3:4]:
            case(f"{f} of the judged cell {what}", exit_plant("d", FIXTURE[0], f, value), want=named)
    case("teq.exit of the judged cell 1 with no error line", exit_plant("d", FIXTURE[0], "teq.exit", "1\n"), want=named)
    # teq's two refusals closed by a summary line (src/main.rs): 1 with the TASTy writer's, 3 with the JVM backend's,
    # each teq's refusal of the cell both refuse; 3 without that line, and scalac's 3, are not.
    tasty = "Macro_1.scala: cannot write TASTy for a type that did not check in Macro\n1 definition cannot be written as TASTy\n"
    jvm = "jvm backend: Macro$.pick: the local a is not in scope of the generated method\n1 construct not supported by the JVM backend\n"
    both = lambda rows: "| both refuse |" in rows.get(refused, "")
    case("teq.exit of the refused cell 1 with the TASTy writer's summary", exit_plant("d", refused, "teq.exit", "1\n", ("teq.out", tasty)), want_rc=0, want=both)
    case("teq.exit of the refused cell 3 with the JVM backend's summary", exit_plant("d", refused, "teq.exit", "3\n", ("teq.out", jvm)), want_rc=0, want=both)
    case("teq.exit of the refused cell 3 without the JVM backend's line", exit_plant("d", refused, "teq.exit", "3\n"), want=named_cell(refused))
    internal = "internal error: Macro_1.scala: parsed without the record of the places the TASTy writer reads\n1 definition cannot be written as TASTy\n"
    case("teq.exit of the refused cell 1 with an internal error before the TASTy writer's summary", exit_plant("d", refused, "teq.exit", "1\n", ("teq.out", internal)), want=named_cell(refused))
    case("scalac.exit of the refused cell 3 with the JVM backend's line", exit_plant("d", refused, "scalac.exit", "3\n", ("scalac.out", jvm)), want=named_cell(refused))
    # A refusal of the cell scalac refuses is the macro build's own, by its own exit and log: a teq that compiles both
    # files and whose program then throws is no refusal (exit 1 with the exception, the last step's, on the run's log).
    def compiled_then_threw(u, d, w, r):
        root = os.path.join(r, refused)
        rec.put(os.path.join(root, "teq-macro.out"), "")
        rec.write(os.path.join(root, "teq-macro.exit"), "0\n")
        rec.put(os.path.join(root, "teq.out"), "Exception in thread \"main\" java.lang.RuntimeException: boom\n")
        rec.write(os.path.join(root, "teq.exit"), "1\n")
    case("teq compiles both files of the refused cell and its program throws", compiled_then_threw,
         want=lambda rows: "DIFFERS: teq compiles what scalac refuses" in rows.get(refused, ""))
    for what, value, log, want in (("1 with the TASTy writer's summary", "1\n", tasty, both), ("3 with the JVM backend's summary", "3\n", jvm, both),
                                   ("3 without the JVM backend's line", "3\n", "compiling\n", named_cell(refused)),
                                   ("1 with no error line", "1\n", "compiling\ndone\n", named_cell(refused)),
                                   ("1 with an internal error before the TASTy writer's summary", "1\n", internal, named_cell(refused))):
        case(f"teq-macro.exit of the refused cell {what}", exit_plant("d", refused, "teq-macro.exit", value, ("teq-macro.out", log)),
             want_rc=0 if want is both else 1, want=want)
    for what, value in bad_exits:
        case(f"teq-macro.exit of the refused cell {what}", exit_plant("d", refused, "teq-macro.exit", value), want=named_cell(refused))
        case(f"scalac-macro.exit of the refused cell {what}", exit_plant("d", refused, "scalac-macro.exit", value), want=named_cell(refused))
    case("teq-macro.out of the refused cell missing", lambda u, d, w, r: os.remove(os.path.join(r, refused, "teq-macro.out")), want=named_cell(refused))
    return t.finish()


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        sys.exit(self_test())
    if len(sys.argv) == 3 and sys.argv[1] == "generate":
        generate(sys.argv[2])
    elif len(sys.argv) == 4 and sys.argv[1] == "judge":
        sys.exit(judge(sys.argv[2], sys.argv[3]))
    else:
        sys.exit(__doc__)
