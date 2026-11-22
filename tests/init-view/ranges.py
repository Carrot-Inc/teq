#!/usr/bin/env python3
"""ranges.py: the language server's ranges at the constructs of tests/init-view/places.py whose read of the source's
text the parser is to record (tests/init-view/ranges.sh runs it). A `teq compiler watch --check --index` session (the
one `teq lsp` runs per project) is asked `hover` and `definition` at each identifier of a cell, twice: over the cell
as its own program, whose answers come from the index of its own parse (typer/index.rs), and over a client of the
cell's products, asked at the cell's source, a document of those products whose answers come from their TASTy
positions (typer/loader/occurrences.rs, places.rs), which the writer's reads place.

  ranges.py cells [<pattern>]        the cells, by name, a shell pattern over the names choosing among them
  ranges.py queries <file> <path>    the session's queries at the file's identifiers, by byte offset, `path` naming it
  ranges.py client <file>            a program that names each top-level definition of the file (in its package), so
                                     that the loader opens the TASTy files the file's products hold
  ranges.py judge <run dir> [<places table>]
                                     per cell of the run's `cells.txt` the answers of the base's and the head's own
                                     sessions (`own-base.jsonl`, `own-head.jsonl`) and of the base's sessions over the
                                     base's and the head's products (`products-base.jsonl`, `products-head.jsonl`),
                                     each line after the build's the answer to a query of `queries.txt`, each beside
                                     its exit (`<leg>-<side>.exit`, the builds' `build-<side>.exit`)
  ranges.py --self-test              the judge on planted records (`self_test`)

`judge` prints a line per cell: `same`, the queries whose answers moved, a refusal both binaries make (their builds'
exits), or NOT JUDGED and why (a record missing, empty, not what its sum says or past its bound,
tests/init-view/records.py; answers
fewer or more than the queries or no JSON; a step's own `not-judged-own` or `not-judged-products`; one binary refusing
what the other compiles; the products' document answering none of the queries). Then each moved answer, the base's and
the head's. A move of the own session's is a span the parse moved, which a record of the parser
must never do; a move of the products' is the head's TASTy placing that tree elsewhere, told by the cell's row of the
places table (tests/init-view/places.sh's `table.md`: the lines scalac's on the base the head lost, those it gained,
those it moved and are not scalac's). Exits 1 when an own answer moved, or a products' answer moved in a cell whose
places row lost a line or moved one away from scalac's, when a leg is not judged, when the places table it is given
is not what its sum says (`table.sum`: a row cut out of it would leave its cell's moves unaccounted), and when the
run's records do not match its cells (a cell listed twice, a listed cell with no record, a record of no listed cell).
A cell of EXPECTED is a
row of master's whose products' answers move as pinned there: its moves must be exactly the pinned ones, each with the
pinned answers of the base and of the head (paths written as `normal` writes them), or the cell fails."""
import fnmatch
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

# The reads of places.py's READS whose token the parser is to record as an offset (a type parameter's sign, a
# package clause, a brace, a type selection, `match`, a selection, an argument clause's opening, a lambda's arrow,
# `case`); 6, 7 and 8 keep a read or read the AST, 15 and 16 are quotes.py's.
RECORDED = {1, 2, 3, 4, 5, 9, 10, 11, 12, 13, 14}

KEYWORDS = set("""abstract case catch class def do else enum export extends false final finally for given if implicit
import lazy match new null object override package private protected return sealed super then this throw trait true try
type val var while with yield""".split())
OP = set("!#%&*+-/:<=>?@\\^|~")
RESERVED_OPS = {"=", "=>", "?=>", "=>>", "<-", ":", "@", "#", "<:", ">:", "|", "<%"}


def cells(pattern="*"):
    import places
    out = []
    for name, _ in places.cells():
        construct = name.split("--")[0]
        if RECORDED & set(places.reads_of(construct)) and fnmatch.fnmatchcase(name, pattern):
            out.append(name)
    return out


def identifiers(text):
    """The start of each identifier of the text (a backquoted one at its quote, an operator one), by character, past
    comments, string and character literals and keywords."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
        elif text.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if text.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif text.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        elif text.startswith('"""', i):
            j = text.find('"""', i + 3)
            i = n if j < 0 else j + 3
        elif c == '"':
            i += 1
            while i < n and text[i] != '"' and text[i] != "\n":
                i += 2 if text[i] == "\\" else 1
            i += 1
        elif c == "'" and i + 2 < n and (text[i + 2] == "'" or (text[i + 1] == "\\" and text.find("'", i + 2) > 0)):
            i = text.find("'", i + 2) + 1
        elif c == "`":
            j = text.find("`", i + 1)
            out.append(i)
            i = n if j < 0 else j + 1
        elif c.isalpha() or c in "_$":
            j = i + 1
            while j < n and (text[j].isalnum() or text[j] in "_$"):
                j += 1
            # `a_+` ends in an operator part.
            if j < n and text[j - 1] == "_" and text[j] in OP:
                while j < n and text[j] in OP:
                    j += 1
            if text[i:j] not in KEYWORDS and text[i:j] != "_":
                out.append(i)
            i = j
        elif c in OP:
            j = i
            while j < n and text[j] in OP and not text.startswith("//", j) and not text.startswith("/*", j):
                j += 1
            if text[i:j] not in RESERVED_OPS:
                out.append(i)
            i = max(j, i + 1)
        else:
            i += 1
    return out


def queries(file, path):
    text = open(file, encoding="utf-8", newline="").read()
    for at in identifiers(text):
        offset = len(text[:at].encode("utf-8"))
        print(f"hover {path} {offset}")
        print(f"definition {path} {offset}")


TOP = re.compile(r"^(?:@\S+\s+)*(?:(?:private|protected|final|case|sealed|abstract|implicit|lazy|inline|opaque|"
                 r"transparent|open|infix)\s+)*(object|class|trait|enum|def|val|var|type|given)\s+(`[^`]+`|[^\s\[\(:=,{]+)")
# A package clause's path, its names and dots apart from each other as a cell's trivia may put them (blanks, line
# breaks; comments are blanked first).
NAME = r"(?:`[^`\n]+`|[^\s.:{;`]+)"
PACKAGE = re.compile(rf"(?m)^[ \t]*package\s+(?!object\b)({NAME}(?:\s*\.\s*{NAME})*)")
PACKAGE_OBJECT = re.compile(r"^package\s+object\s+(\S+)")


def uncommented(text):
    """The text with each comment's characters blanked (its line breaks kept), strings and the rest as they are."""
    out, i, n = list(text), 0, len(text)
    while i < n:
        if text.startswith("//", i):
            while i < n and text[i] != "\n":
                out[i], i = " ", i + 1
        elif text.startswith("/*", i):
            depth = 0
            while i < n:
                if text.startswith("/*", i):
                    depth, out[i], out[i + 1], i = depth + 1, " ", " ", i + 2
                elif text.startswith("*/", i):
                    depth, out[i], out[i + 1], i = depth - 1, " ", " ", i + 2
                    if depth == 0:
                        break
                else:
                    out[i] = out[i] if text[i] == "\n" else " "
                    i += 1
        elif text[i] == '"':
            j = text.find('"""', i) if text.startswith('"""', i) else -1
            if j == i:
                k = text.find('"""', i + 3)
                i = n if k < 0 else k + 3
            else:
                i += 1
                while i < n and text[i] not in '"\n':
                    i += 2 if text[i] == "\\" else 1
                i += 1
        else:
            i += 1
    return "".join(out)


def client(file):
    """A program that names the file's definitions in the file's packages: each term in one sequence, each type in an
    alias. A package block's definitions are indented, so every line's is named; a member's name that resolves to
    nothing at the top level, and whatever the program does not type, is no matter: the names make the loader open
    the files they resolve to."""
    text = uncommented(open(file, encoding="utf-8").read().replace("\r\n", "\n"))
    packages = [re.sub(r"\s+", "", m.group(1)) for m in PACKAGE.finditer(text)]
    terms, types = [], []
    for line in text.split("\n"):
        if PACKAGE.match(line):
            continue
        m = PACKAGE_OBJECT.match(line.lstrip())
        if m:
            terms.append(m.group(1))
            continue
        m = TOP.match(line.lstrip())
        if not m:
            continue
        kind, name = m.groups()
        if kind in ("class", "trait", "type"):
            types.append(name)
        elif kind == "enum":
            types.append(name)
            terms.append(name)
        else:
            terms.append(name)
    lines = []
    if packages:
        lines.append(f"import {'.'.join(packages)}.*")
    lines.append("object RangesClient:")
    if terms:
        lines.append(f"  def terms: Any = Seq[Any]({', '.join(terms)})")
    for k, t in enumerate(types):
        lines.append(f"  type T{k} = {t}")
    if not terms and not types:
        lines.append("  def nothing = 0")
    print("\n".join(lines))


def leg(d, name, qs):
    """One leg of a cell (`own` or `products`): ('ok', base answers, head answers), ('refused', why) where both
    binaries refuse the cell (each build's exit `build-<side>.exit` 1 with teq's error line in its output
    `build-<side>.txt`: records.py's `judged_exit`), or ('not judged', why): a build's exit a signal's, 2, 124, none
    or 1 with no error line, or one binary refusing what the other compiles."""
    import records as rec
    if name == "products":
        builds = [rec.judged_exit(os.path.join(d, f"build-{side}.exit"), os.path.join(d, f"build-{side}.txt"), f"the {side}'s build", rec.TEQ_REFUSALS)
                  for side in ("base", "head")]
        bad = next((b for b in builds if b[0] == "not judged"), None)
        if bad:
            return bad
        if builds[0][0] == "refused" and builds[1][0] == "refused":
            return "refused", "both binaries refuse it"
        if builds[0][0] != builds[1][0]:
            return "not judged", "one binary refuses what the other compiles (" + "; ".join(b[1] or "it compiles" for b in builds) + ")"
    elif os.path.exists(os.path.join(d, "not-judged-own")):
        return "not judged", open(os.path.join(d, "not-judged-own"), encoding="utf-8").read().strip().replace("\n", "; ")
    if name == "products" and os.path.exists(os.path.join(d, "not-judged-products")):
        return "not judged", open(os.path.join(d, "not-judged-products"), encoding="utf-8").read().strip().replace("\n", "; ")
    got = []
    for side in ("base", "head"):
        state, val = rec.step(os.path.join(d, f"{name}-{side}.jsonl"), os.path.join(d, f"{name}-{side}.exit"), f"the {side}'s {name} session")
        if state != "ok":
            return state, val
        answers = val[1:]
        if len(answers) != len(qs):
            return "not judged", f"the {side}'s {name} session answered {len(answers)} of {len(qs)} queries"
        for a in answers:
            try:
                json.loads(a)
            except ValueError:
                return "not judged", f"the {side}'s {name} session wrote an answer that is no JSON"
        got.append(answers)
    if name == "products" and all(empty(x) for x in got[0]):
        return "not judged", "the base's answers are all empty"
    return "ok", got[0], got[1]


# The products' answers that move in a cell whose places row holds a line of master's, pinned: master's row of the typed
# pattern's identifier in a catch case (its plain cell too, the identifier at 38 where scalac has 41). Master gives no
# answer where a comment stands between `case` and the pattern; the head answers as the plain cell does, shifted by the
# comment. Each query is its kind and its offset; each answer the base's and the head's, as `normal` writes them.
_NONE = '{"result":null,"text":"%s"}'
_HOVER = '{"result":{"contents":{"kind":"markdown","value":"```scala\\n%s\\n```"},"range":{"start":{"line":1,"character":%d},"end":{"line":1,"character":%d}}},"text":"%s"}'
_HERE = '{"result":[{"uri":"file://<run>/%s/src/%s.scala","range":{"start":{"line":1,"character":%d},"end":{"line":1,"character":%d}}}],"text":"%s"}'
_STD = '{"result":[{"uri":"file://<std>/exceptions.scala","range":{"start":{"line":135,"character":8},"end":{"line":135,"character":17}}}],"text":"%s"}'


def _catch(cell, text, at, name_col, type_col):
    """The four moves of a catch case's comment cell: `e` at `at` (its column `name_col`), `Exception` at `at + 3`."""
    return {
        ("hover", at): (_NONE % text, _HOVER % ("e: Exception", name_col, name_col + 1, text)),
        ("definition", at): (_NONE % text, _HERE % (cell, cell, name_col, name_col + 1, text)),
        ("hover", at + 3): (_NONE % text, _HOVER % ("class java.lang.Exception", type_col, type_col + 9, text)),
        ("definition", at + 3): (_NONE % text, _STD % text),
    }


EXPECTED = {
    "s14-catch--block-comment--between": _catch("s14-catch--block-comment--between", "82359dad3d967bdc", 38, 18, 21),
    "s14-catch--nested-comment--between": _catch("s14-catch--nested-comment--between", "3f01813ccce0829e", 48, 28, 31),
}


def normal(line, run):
    """An answer with its machine's paths written alike: the run's directory `<run>`, the attached std's `<std>`."""
    line = line.replace(os.path.abspath(run), "<run>")
    return re.sub(r'file://[^"]*?/attached/std-[0-9a-f]+/', "file://<std>/", line)


def expected_moves(run, qs, base, head, moved):
    """The cell's moved answers keyed as EXPECTED keys them."""
    out = {}
    for i in moved:
        parts = qs[i].split()
        out[(parts[0], int(parts[-1]))] = (normal(base[i], run), normal(head[i], run))
    return out


def empty(line):
    result = json.loads(line).get("result")
    return result is None or result == [] or result == {}


def places_rows(table):
    """(the places table's row per judged cell: (lost, gained, moved and not scalac's), None), (None, None) where no
    table is given, or (None, why) where the table given is not what its sum says (tests/init-view/records.py)."""
    import records as rec
    rows = {}
    if not table:
        return None, None
    lines, why = rec.text(table)
    if why:
        return None, why
    for line in lines:
        cols = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cols) == 6 and cols[1] == "judged":
            rows[cols[0]] = tuple(int(x) for x in cols[2:5])
    return rows, None


def judge(run, table=None):
    import records as rec
    rows, twhy = places_rows(table)
    listed, why = rec.text(os.path.join(run, "cells.txt"))
    present = sorted(n for n in os.listdir(run) if os.path.isdir(os.path.join(run, n))) if os.path.isdir(run) else []
    problems = ["no record of the cells the run was given (cells.txt " + why + ")"] if why else rec.manifest(listed, present)
    names = list(dict.fromkeys(listed or []))
    counts, bad, moves, pinned = {}, 0, [], set()
    if twhy:
        # A table cut by a row would leave that cell's moves unaccounted: none is judged against it.
        print(f"NOT JUDGED: the places table {twhy}: no products' move is judged against it")
        bad += 1
    print("| cell | own source | products | places (lost, gained, moved not scalac's) |")
    print("|---|---|---|---|")
    for cell in names:
        d = os.path.join(run, cell)
        qs, qwhy = rec.text(os.path.join(d, "queries.txt"))
        whole = None
        if not os.path.isdir(d):
            whole = "no record of the cell"
        elif qwhy:
            whole = f"its queries {qwhy}"
        elif len(set(qs)) != len(qs):
            whole = "its queries hold a query twice"
        if whole:
            verdicts = ["NOT JUDGED: " + whole] * 2
            bad += 1
        else:
            verdicts = []
            for name in ("own", "products"):
                state, *val = leg(d, name, qs)
                if state == "refused":
                    verdicts.append(val[0])
                    continue
                if state == "not judged":
                    verdicts.append("NOT JUDGED: " + val[0])
                    bad += 1
                    continue
                base, head = val
                moved = [i for i in range(len(qs)) if base[i] != head[i]]
                for i in moved:
                    moves.append((cell, name, qs[i], base[i], head[i]))
                verdict = "same" if not moved else f"{len(moved)} of {len(qs)} moved"
                row = rows.get(cell) if rows else None
                if name == "products" and cell in EXPECTED:
                    if expected_moves(run, qs, base, head, moved) == EXPECTED[cell]:
                        verdict += ", as pinned (master's row)"
                        pinned.add(cell)
                    else:
                        verdict += ", NOT AS PINNED"
                        bad += 1
                elif moved and (name == "own" or (row is not None and (row[0] or row[2]))):
                    bad += 1
                verdicts.append(verdict)
        row = rows.get(cell) if rows else None
        shown = "no places table" if rows is None else ("not judged there" if row is None else ", ".join(map(str, row)))
        for v in verdicts:
            key = ("moved, as pinned" if "as pinned (" in v else "moved") if v[0].isdigit() else v
            counts[key] = counts.get(key, 0) + 1
        print(f"| {cell} | {verdicts[0]} | {verdicts[1]} | {shown} |")
    for cell, name, q, b, h in moves:
        print(f"{cell} MOVED ({name}) at `{q}`: base `{b}` head `{h}`")
    # A pinned cell the run holds whose products' leg was not judged as pinned (refused, not judged) fails too.
    for cell in sorted(set(EXPECTED) & set(names) - pinned):
        print(f"{cell} NOT AS PINNED: its products' answers are not the pinned ones")
        bad += 1
    for p in problems:
        print(f"MANIFEST: {p}")
    print(f"ranges: {len(names)} cells; " + "; ".join(f"{k}: {n}" for k, n in sorted(counts.items()))
          + f"; {bad} legs not judged or moves the places table does not account for; {len(problems)} manifest problems")
    return 1 if bad or problems else 0


# A build teq refuses, its error line on its record (`build-<side>.txt`).
REFUSAL = "c.scala:2:27: error: new Array takes one type argument\n    val a = new Array(1)\n1 error found\n"


def self_test():
    """The judge on planted records (`--self-test`): a good run passes (a cell both binaries refuse among them); each
    record missing, empty or cut (the queries, a session's answers on either leg and side), a step past its bound or
    failed (a build, a session), an answer that moved or that is no JSON, answers fewer or more than the queries, a
    query held twice (each sealed as its producer wrote it), a cell list holding a cell twice, a run record of a cell
    the list does not hold, and a listed cell with no record fail, the cell named; so does a build's exit other than 0
    or a refusal named by its line on its record (137, 2, 124, 139, none, 1 with no error line, 3 without the JVM
    backend's line), and a session's other than 0; teq's 1 with the TASTy writer's summary and its 3 with the JVM
    backend's are refusals. Each record changed after its
    producer sealed it (its last line gone with its line break and exit kept, a line gone from its middle, a byte
    changed, its sum gone, another cell's record copied in), the cell list and the places table among them, is NOT
    JUDGED for what its sum says; a products' move in a cell whose places row lost a line fails. A cell of EXPECTED
    moving as pinned passes; a pinned answer changed on either side, a pinned move missing, a move beyond the pinned
    ones and the cell refused fail."""
    import shutil
    import records as rec
    t = rec.SelfTest("ranges.py")
    # A judged cell, a cell both binaries refuse, and a judged cell like the first.
    fixture = ["s11-field--nothing--before", "s13-lambda-unicode--nothing--before", "s12-call--nothing--before"]
    queries = "hover src/c.scala 4\ndefinition src/c.scala 4\nhover src/c.scala 8\n"
    build = '{"ok":true,"diagnostics":[]}\n'
    answers = (build + '{"result":{"range":{"start":{"line":0,"character":4}}},"text":"0"}\n{"result":[{"uri":"file:///c.scala"}],"text":"0"}\n'
               '{"result":null,"text":"0"}\n')
    moved = answers.replace('"character":4', '"character":5')
    # The places table of the run: a row per cell, the first cell's in the table's middle.
    table_text = ("| cell | verdict | lost | gained | moved, not scalac's | master's, unchanged |\n|---|---|---|---|---|---|\n"
                  f"| {fixture[1]} | scalac refuses | | | | |\n| {fixture[0]} | judged | 1 | 0 | 0 | 3 |\n"
                  f"| {fixture[2]} | judged | 0 | 0 | 0 | 3 |\nplaces: 3 cells, 2 judged\n")

    def good(root):
        run = os.path.join(root, "run")
        rec.put(os.path.join(run, "cells.txt"), "".join(c + "\n" for c in fixture))
        for c in fixture:
            d = os.path.join(run, c)
            rec.put(os.path.join(d, "queries.txt"), queries)
            for leg in ("own", "products"):
                for side in ("base", "head"):
                    rec.put(os.path.join(d, f"{leg}-{side}.jsonl"), answers)
                    rec.write(os.path.join(d, f"{leg}-{side}.exit"), "0\n")
            for side in ("base", "head"):
                rec.write(os.path.join(d, f"build-{side}.exit"), "0\n")
            if c == fixture[1]:
                # Neither binary compiles it: its products' leg is no one's to judge, and says so.
                for side in ("base", "head"):
                    os.remove(os.path.join(d, f"products-{side}.jsonl"))
                    os.remove(os.path.join(d, f"products-{side}.sum"))
                    os.remove(os.path.join(d, f"products-{side}.exit"))
                    rec.write(os.path.join(d, f"build-{side}.exit"), "1\n")
                    rec.put(os.path.join(d, f"build-{side}.txt"), REFUSAL)
                rec.write(os.path.join(d, "not-judged-products"), "the base makes no products (exit 1)\nthe head makes no products (exit 1)\n")
        return run

    def case(name, mutate, want_rc=1, want=None, table=False):
        root = t.dir(name)
        run = good(root)
        tpath = None
        if table:
            tpath = os.path.join(root, "places", "table.md")
            rec.put(tpath, table_text)
        mutate(os.path.join(run, fixture[0]), run)
        rc, out = t.quiet(judge, run, tpath)
        row = next((r for r in out.splitlines() if r.startswith(f"| {fixture[0]} |")), "no row")
        t.expect(name, rc == want_rc and (want is None or want(row, out)), f"exit {rc}; {row}")

    def drop(f):
        return lambda d, r: os.remove(os.path.join(d, f))

    def blank(f):
        # An empty output, as its producer wrote it and sealed it.
        return lambda d, r: rec.put(os.path.join(d, f), "")

    def cut(f):
        def g(d, r):
            data = open(os.path.join(d, f), encoding="utf-8").read()
            rec.write(os.path.join(d, f), data[: len(data) - 7])
        return g

    def put(f, content):
        # A record as its producer wrote it: an output sealed, a marker or an exit as it is.
        def g(d, r):
            path = os.path.join(d, f)
            if f.endswith((".jsonl", ".txt")):
                rec.put(path, content)
            else:
                rec.write(path, content)
        return g

    named = lambda row, out: "NOT JUDGED" in row
    says = lambda text: (lambda row, out: text in row)
    by_sum = lambda row, out: "NOT JUDGED" in row and any(w in row for w in rec.SUM_WHY)
    outputs = ["queries.txt"] + [f"{leg}-{side}.jsonl" for leg in ("own", "products") for side in ("base", "head")]
    case("a good run", lambda d, r: None, want_rc=0)
    for f in outputs:
        case(f"{f} missing", drop(f), want=named)
        case(f"{f} empty", blank(f), want=named)
        case(f"{f} cut", cut(f), want=named)
    for f in [f"{leg}-{side}.exit" for leg in ("own", "products") for side in ("base", "head")] + ["build-base.exit", "build-head.exit"]:
        case(f"{f} missing", drop(f), want=named)
        case(f"{f} 124", put(f, "124\n"), want=named)
    case("a leg's NOT JUDGED marker", put("not-judged-own", "the base's own session: past its bound\n"), want=named)
    case("an own answer that moved", put("own-head.jsonl", moved), want=says("1 of 3 moved"))
    case("an answer that is no JSON", put("own-head.jsonl", answers.replace('{"result":[{"uri"', '{"result":[{uri')), want=says("no JSON"))
    case("answers fewer than the queries", put("own-head.jsonl", "".join(answers.splitlines(keepends=True)[:2])), want=says("answered 1 of 3"))
    case("answers more than the queries", put("own-head.jsonl", answers + answers.splitlines(keepends=True)[1]), want=says("answered 4 of 3"))
    case("a query held twice", put("queries.txt", queries + queries.splitlines(keepends=True)[0]), want=says("a query twice"))
    case("one binary refusing what the other compiles", lambda d, r: (put("build-head.exit", "1\n")(d, r), put("build-head.txt", REFUSAL)(d, r)),
         want=says("one binary refuses"))
    case("a cell list holding a cell twice", lambda d, r: rec.put(os.path.join(r, "cells.txt"), "".join(c + "\n" for c in fixture + fixture[:1])))
    case("a run record of a cell the list does not hold", lambda d, r: os.makedirs(os.path.join(r, "s11-stray--nothing--before")))
    case("a listed cell with no record", lambda d, r: shutil.rmtree(d), want=named)
    case("no cell list", lambda d, r: os.remove(os.path.join(r, "cells.txt")))
    # Each record changed after its producer sealed it, the same record of the third cell (sealed by its producer, the
    # same bytes) the other cell's.
    other = lambda path: path.replace(fixture[0], fixture[2])
    for f in outputs:
        for plant, mutation in rec.after_the_producer(other):
            case(f"{f}: {plant}", lambda d, r, f=f, mutation=mutation: mutation(os.path.join(d, f)), want=by_sum)
    # The cell list, which is of the run: the plants of a record of its own (no other cell's).
    for plant, mutation in rec.after_the_producer(None)[:4]:
        case(f"cells.txt: {plant}", lambda d, r, mutation=mutation: mutation(os.path.join(r, "cells.txt")),
             want=lambda row, out: "MANIFEST: no record of the cells the run was given" in out and any(w in out for w in rec.SUM_WHY))
    # The places table: a products' move in the first cell, whose row lost a line, fails; the table changed after its
    # producer sealed it (the first cell's row gone from its middle would leave the move unaccounted) is NOT JUDGED.
    head_moves = put("products-head.jsonl", moved)
    case("a products' move where the places row lost a line", head_moves, table=True, want=says("1 of 3 moved"))
    case("the places table missing", lambda d, r: (head_moves(d, r), os.remove(os.path.join(os.path.dirname(r), "places", "table.md"))),
         table=True, want=lambda row, out: "NOT JUDGED: the places table missing" in out)
    for plant, mutation in rec.after_the_producer(None)[:4]:
        case(f"the places table: {plant}",
             lambda d, r, mutation=mutation: (head_moves(d, r), mutation(os.path.join(os.path.dirname(r), "places", "table.md"))),
             table=True, want=lambda row, out: "NOT JUDGED: the places table" in out and any(w in out for w in rec.SUM_WHY))

    # A build's exit by the whitelist: 0, or 1 with teq's error line on its record. A signal's (137 a kill, 139 a
    # segmentation fault), 2, 124, none recorded and 1 with no error line are NOT JUDGED: on each side's build of the
    # cell both binaries refuse, and on each side's build of the judged cell; a session's exit other than 0 too.
    def exit_plant(cell, f, value, log=None):
        def g(d, r):
            root = os.path.join(r, cell)
            if value is None:
                os.remove(os.path.join(root, f))
            else:
                rec.write(os.path.join(root, f), value)
            if log:
                rec.put(os.path.join(root, log[0]), log[1])
        return g

    def named_cell(cell):
        return lambda row, out: "NOT JUDGED" in next((r for r in out.splitlines() if r.startswith(f"| {cell} |")), "")
    bad_exits = (("137 (killed)", "137\n"), ("2", "2\n"), ("124", "124\n"), ("139", "139\n"), ("missing", None))
    for side in ("base", "head"):
        for what, value in bad_exits:
            case(f"build-{side}.exit of the refused cell {what}", exit_plant(fixture[1], f"build-{side}.exit", value), want=named_cell(fixture[1]))
            case(f"build-{side}.exit of the judged cell {what}", exit_plant(fixture[0], f"build-{side}.exit", value), want=named)
        case(f"build-{side}.exit of the refused cell 1 with no error line", exit_plant(fixture[1], f"build-{side}.exit", "1\n", (f"build-{side}.txt", "compiling\ndone\n")),
             want=named_cell(fixture[1]))
    for f in [f"{leg}-{side}.exit" for leg in ("own", "products") for side in ("base", "head")]:
        for what, value in bad_exits[:2] + bad_exits[3:4]:
            case(f"{f} {what}", exit_plant(fixture[0], f, value), want=named)
    # teq's two refusals closed by a summary line (src/main.rs): 1 with the TASTy writer's, 3 with the JVM backend's, a
    # refusal of the build; 3 without that line is not.
    tasty = "c.scala: cannot write TASTy for a type that did not check in N1\n1 definition cannot be written as TASTy\n"
    jvm = "jvm backend: c$package$.run: the local o is not in scope of the generated method\n1 construct not supported by the JVM backend\n"
    refusing = lambda row, out: "both binaries refuse it" in next((r for r in out.splitlines() if r.startswith(f"| {fixture[1]} |")), "")
    for side in ("base", "head"):
        case(f"build-{side}.exit of the refused cell 1 with the TASTy writer's summary", exit_plant(fixture[1], f"build-{side}.exit", "1\n", (f"build-{side}.txt", tasty)),
             want_rc=0, want=refusing)
        case(f"build-{side}.exit of the refused cell 3 with the JVM backend's summary", exit_plant(fixture[1], f"build-{side}.exit", "3\n", (f"build-{side}.txt", jvm)),
             want_rc=0, want=refusing)
        case(f"build-{side}.exit of the refused cell 3 without the JVM backend's line", exit_plant(fixture[1], f"build-{side}.exit", "3\n"), want=named_cell(fixture[1]))
        internal = "internal error: c.scala: parsed without the record of the places the TASTy writer reads\n1 definition cannot be written as TASTy\n"
        case(f"build-{side}.exit of the refused cell 1 with an internal error before the TASTy writer's summary",
             exit_plant(fixture[1], f"build-{side}.exit", "1\n", (f"build-{side}.txt", internal)), want=named_cell(fixture[1]))

    # A cell of EXPECTED: its products' answers move as pinned (with the places row that leaves them unaccounted),
    # which passes; a pinned answer changed on either side, a pinned move missing, a move beyond the pinned ones, and
    # the cell's products' leg refused fail.
    pin = "s14-catch--block-comment--between"
    keys = sorted(EXPECTED[pin])

    def pinned_run(root, change=None):
        run = os.path.join(root, "run")
        d = os.path.join(run, pin)
        rec.put(os.path.join(run, "cells.txt"), pin + "\n")
        qs = [f"{k} {os.path.join(d, 'src', pin + '.scala')} {at}" for k, at in keys] + [f"hover {os.path.join(d, 'src', pin + '.scala')} 4"]
        rec.put(os.path.join(d, "queries.txt"), "".join(q + "\n" for q in qs))
        real = lambda a: a.replace("<run>", os.path.abspath(run)).replace("file://<std>/", "file:///cache/teq/attached/std-0a1b2c/")
        # The query at `f` answers alike on both sides, as the real run's does: the base's document answers it.
        plain = '{"result":{"contents":{"kind":"markdown","value":"def f: Int"},"range":{"start":{"line":0,"character":4},"end":{"line":0,"character":5}}},"text":"0"}'
        base = [real(EXPECTED[pin][k][0]) for k in keys] + [plain]
        head = [real(EXPECTED[pin][k][1]) for k in keys] + [plain]
        if change:
            change(base, head)
        for leg, (b, h) in (("own", (head, head)), ("products", (base, head))):
            rec.put(os.path.join(d, f"{leg}-base.jsonl"), build + "".join(x + "\n" for x in b))
            rec.put(os.path.join(d, f"{leg}-head.jsonl"), build + "".join(x + "\n" for x in h))
            for side in ("base", "head"):
                rec.write(os.path.join(d, f"{leg}-{side}.exit"), "0\n")
        for side in ("base", "head"):
            rec.write(os.path.join(d, f"build-{side}.exit"), "0\n")
        table = os.path.join(root, "table.md")
        rec.put(table, f"| {pin} | judged | 0 | 1 | 1 | 2 |\n")
        return run, table

    def pinned_case(name, change, want_rc=1, after=None, want="NOT AS PINNED"):
        root = t.dir(name)
        run, table = pinned_run(root, change)
        if after:
            after(os.path.join(run, pin))
        rc, out = t.quiet(judge, run, table)
        row = next((r for r in out.splitlines() if r.startswith(f"| {pin} |")), "no row")
        t.expect(name, rc == want_rc and want in out, f"exit {rc}; {row}")

    def at(k, side, f):
        def g(base, head):
            lst = base if side == "base" else head
            lst[keys.index(k)] = f(lst[keys.index(k)])
        return g

    pinned_case("a pinned cell moving as pinned", None, want_rc=0, want="as pinned (master's row)")
    pinned_case("a pinned head answer changed", at(keys[0], "head", lambda a: a.replace('"character":18', '"character":19')))
    pinned_case("a pinned base answer changed", at(keys[1], "base", lambda a: a.replace('"result":null', '"result":[]')))
    pinned_case("a pinned move missing", lambda b, h: h.__setitem__(2, b[2]))
    pinned_case("a move beyond the pinned ones", lambda b, h: h.__setitem__(len(h) - 1, h[-1].replace('"character":4', '"character":5')))

    def refused(d):
        for side in ("base", "head"):
            os.remove(os.path.join(d, f"products-{side}.jsonl"))
            os.remove(os.path.join(d, f"products-{side}.exit"))
            rec.write(os.path.join(d, f"build-{side}.exit"), "1\n")
            rec.put(os.path.join(d, f"build-{side}.txt"), REFUSAL)
        for side in ("base", "head"):
            os.remove(os.path.join(d, f"own-{side}.jsonl"))
            os.remove(os.path.join(d, f"own-{side}.exit"))
        rec.write(os.path.join(d, "not-judged-products"), "the base makes no products (exit 1)\n")
        rec.write(os.path.join(d, "not-judged-own"), "the base makes no products (exit 1)\n")
    pinned_case("a pinned cell that both binaries refuse", None, after=refused)
    return t.finish()


if __name__ == "__main__":
    a = sys.argv[1:]
    if a == ["--self-test"]:
        sys.exit(self_test())
    if a[:1] == ["cells"] and len(a) <= 2:
        print("\n".join(cells(*a[1:])))
    elif a[:1] == ["queries"] and len(a) == 3:
        queries(a[1], a[2])
    elif a[:1] == ["client"] and len(a) == 2:
        client(a[1])
    elif a[:1] == ["judge"] and len(a) in (2, 3):
        sys.exit(judge(*a[1:]))
    else:
        sys.exit(__doc__)
