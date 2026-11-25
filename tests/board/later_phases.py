#!/usr/bin/env python3
"""dotty 3.8.4's errors of the phases after the typing, a table generated from its sources, which tests/board/dotty.py
reads to say of a neg test whose .check names one that `LATER_PHASES` does not list "a later phase not listed":

  python3 tests/board/later_phases.py --write <scala3 repository> <its tests>   both files below, from the tag 3.8.4
  python3 tests/board/later_phases.py --check <scala3 repository> <its tests>   the committed files are its output
  python3 tests/board/later_phases.py --self-test                               the generator and the matcher over
                                                                                planted sources and .checks

The repository is a clone of scala/scala3 with the tag (its compiler sources read by `git show 3.8.4:<path>`), the
tests a checkout of the tag's tests/ (the board's SCALA3_TAG): the gate has the second alone, so the files are
generated and committed, and --check runs where both are.

tests/board/dotty/later-phases.tsv, a row per `report.error` (or errorOrMigrationWarning) site of the phases from
Erasure to GenBCode in Compiler.scala's order, with dotty's backend/jvm files: the phase, the site (file:line), its
kind, and the pattern (JSON) a .check's error matches:
  literal, interpolated  {"text": [[fragments]]}: the fixed fragments of its text in order (an interpolation's between
                         its splices), each of eight characters or more;
  value                  {"text": [[...], [...]]}: the message is a val set just above the call, its literals each an
                         alternative;
  class                  {"id": "E###"}: a message class whose errorNumber the .check's header names, only where no
                         phase before Erasure raises that class too (the typer's Checking raises E097 and E120 as well):
                         such a class is `class, also raised before` with no pattern;
  other                  null: a message the table cannot read (a value from elsewhere, an exception's text).
tests/board/dotty/later-phase-tests.tsv: the neg tests of the tag (tests/neg*) whose .check has an error matching a
row: the test, the site, and that error's lines (JSON), so that gate-scripts can hold them without the corpus.
"""
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TABLE = os.path.join(HERE, "dotty", "later-phases.tsv")
TESTS = os.path.join(HERE, "dotty", "later-phase-tests.tsv")
KNOWN = os.path.join(HERE, "dotty", "later-phases-known.tsv")
TAG = "3.8.4"
COMPILER = "compiler/src/dotty/tools/dotc/Compiler.scala"
MESSAGES = "compiler/src/dotty/tools/dotc/reporting/messages.scala"
IDS = "compiler/src/dotty/tools/dotc/reporting/ErrorMessageID.scala"
BACKEND = "compiler/src/dotty/tools/backend/jvm/"
SITE = re.compile(r"report\.(?:error|errorOrMigrationWarning)\(")
LITERAL = re.compile(r'(?:\b(?:em|i|s|e|ex)(?="))?("""|")')
SPLICE = re.compile(r"\$\{[^}]*\}|\$[A-Za-z_][A-Za-z0-9_]*")


class GitTree:
    """A clone's files at the tag."""
    def __init__(self, repo):
        self.repo = repo
        self.names = subprocess.run(["git", "-C", repo, "ls-tree", "-r", "--name-only", TAG, "--", "compiler/src"],
                                    capture_output=True, text=True, check=True).stdout.split()

    def read(self, path):
        return subprocess.run(["git", "-C", self.repo, "show", f"{TAG}:{path}"], capture_output=True, text=True, check=True).stdout


class PlantedTree:
    """Files given as a dict, for the self-test."""
    def __init__(self, files):
        self.files, self.names = files, sorted(files)

    def read(self, path):
        return self.files[path]


def literal_at(text, i):
    """The string literal starting at i (after whitespace), its body and end, or None."""
    m = LITERAL.match(text, i)
    if not m:
        return None
    q = m.group(1)
    j = m.end()
    if q == '"""':
        end = text.find('"""', j)
    else:
        end = j
        while end < len(text) and text[end] != '"':
            end += 2 if text[end] == "\\" else 1
    return (text[j:end], end + len(q)) if end >= 0 else None


def fragments(body):
    """A literal's fixed fragments: its margin stripped, its splices cut out, the whitespace made one space."""
    lines = [re.sub(r"^\s*\|", "", l) for l in body.replace("\\n", "\n").split("\n")]
    flat = " ".join(l.strip() for l in lines)
    out = []
    for piece in SPLICE.split(flat):
        piece = " ".join(piece.split()).strip(" .,:;")
        if len(piece) >= 8:
            out.append(piece)
    return out


def literals_in(text):
    """Every string literal of the text, its fragments, in order."""
    out, i = [], 0
    while True:
        m = LITERAL.search(text, i)
        if not m:
            return out
        lit = literal_at(text, m.start())
        if not lit:
            return out
        frags = fragments(lit[0])
        if frags:
            out.append(frags)
        i = lit[1]


def class_ids(tree):
    """Message class -> E###, from messages.scala's `extends ...(<Name>ID)` and ErrorMessageID.scala's comments."""
    numbers = {m.group(1): int(m.group(2)) for m in re.finditer(r"case (\w+ID)\b.*?// errorNumber: (\d+)", tree.read(IDS))}
    out = {}
    for m in re.finditer(r"^(?:final )?class (\w+)\([^\n]*\n?(?:[^\n]*\n)?\s*extends \w+\((\w+ID)\)", tree.read(MESSAGES), re.M):
        if m.group(2) in numbers:
            out[m.group(1)] = f"E{numbers[m.group(2)]:03d}"
    return out


def phase_files(tree):
    """The phases from Erasure to GenBCode (Compiler.scala's order) and their files, with backend/jvm's."""
    lines = tree.read(COMPILER).splitlines()
    start = next(i for i, l in enumerate(lines) if "new Erasure)" in l or "new Erasure," in l)
    end = next(i for i in range(start, len(lines)) if "new GenBCode" in lines[i])
    phases = re.findall(r"new ([A-Za-z.]+)", "\n".join(lines[start:end + 1]))
    files = {}
    for p in phases:
        base = p.split(".")[-1]
        found = [n for n in tree.names if n.endswith("/" + base + ".scala")]
        if found:
            files[found[0]] = base
    for n in tree.names:
        if n.startswith(BACKEND) and n.endswith(".scala"):
            files.setdefault(n, "GenBCode (backend/jvm)")
    return files


def generate(tree):
    """The table's rows: (phase, site, kind, pattern)."""
    files = phase_files(tree)
    ids = class_ids(tree)
    later = set(files)
    rows = []
    for path, phase in sorted(files.items(), key=lambda kv: (kv[1], kv[0])):
        text = tree.read(path)
        for m in SITE.finditer(text):
            line = text.count("\n", 0, m.start()) + 1
            site = f"{os.path.basename(path)}:{line}"
            i = m.end()
            while i < len(text) and text[i] in " \t\n":
                i += 1
            lit = literal_at(text, i)
            cls = re.match(r"([A-Z]\w*)\(", text[i:])
            var = re.match(r"([a-z]\w*)\s*[,)]", text[i:])
            if lit:
                kind = "interpolated" if text[i] != '"' else "literal"
                frags = fragments(lit[0])
                rows.append((phase, site, kind, {"text": [frags]} if frags else None))
            elif cls and cls.group(1) in ids:
                name = cls.group(1)
                elsewhere = sorted(n for n in tree.names if n not in later and n != MESSAGES and n.endswith(".scala")
                                   and re.search(r"\b" + name + r"\(", tree.read(n)))
                if elsewhere:
                    rows.append((phase, site, f"class {name} {ids[name]}, also raised before: {' '.join(os.path.basename(e) for e in elsewhere)}", None))
                else:
                    rows.append((phase, site, f"class {name}", {"id": ids[name]}))
            elif var:
                before = text[:m.start()]
                decl = list(re.finditer(r"\bval " + var.group(1) + r"\s*=", before))
                alts = literals_in(before[decl[-1].start():]) if decl and before.count("\n", decl[-1].start()) <= 30 else []
                rows.append((phase, site, "value", {"text": alts} if alts else None))
            else:
                rows.append((phase, site, "other", None))
    return rows


def normal(text):
    """A .check's error as one line: the gutters (`7 |`, `  |`) cut, the whitespace made one space."""
    return " ".join(re.sub(r"^\s*\d*\s*\|", "", l) for l in text.splitlines()).split()


def errors_of(check):
    """A .check's errors, each its lines: a block from a `-- ` header to the next."""
    blocks, cur = [], None
    for l in check.splitlines():
        if l.startswith("-- "):
            cur = [l]
            blocks.append(cur)
        elif cur is not None:
            cur.append(l)
    return blocks


def matches(pattern, block):
    """Whether an error (its lines) matches a row's pattern."""
    if not pattern:
        return False
    if "id" in pattern:
        return f"[{pattern['id']}]" in block[0]
    flat = " ".join(normal("\n".join(block)))
    for alt in pattern["text"]:
        at = 0
        for frag in alt:
            k = flat.find(frag, at)
            if k < 0:
                break
            at = k + len(frag)
        else:
            return True
    return False


def load(path=TABLE):
    rows = []
    with open(path, encoding="utf-8") as fh:
        for l in fh:
            if l.startswith("#") or not l.strip():
                continue
            phase, site, kind, pattern = l.rstrip("\n").split("\t")
            rows.append((phase, site, kind, json.loads(pattern)))
    return rows


def matched(rows, check):
    """The (row, error lines) pairs of a .check's errors that match a row."""
    out = []
    for block in errors_of(check):
        for r in rows:
            if matches(r[3], block):
                out.append((r, block))
                break
    return out


def known(path=KNOWN):
    if not os.path.exists(path):
        return {}
    with open(path, encoding="utf-8") as fh:
        return {l.split("\t")[0]: l.rstrip("\n").split("\t", 1)[1] for l in fh if l.strip() and not l.startswith("#")}


def table_text(rows):
    head = "# phase\tsite\tkind\tpattern (JSON): tests/board/later_phases.py --write, from dotty " + TAG + "'s sources\n"
    return head + "".join(f"{p}\t{s}\t{k}\t{json.dumps(pat, ensure_ascii=False)}\n" for p, s, k, pat in rows)


def tests_text(rows, tests_dir):
    out = ["# test\tsite\tthe error's lines (JSON): tests/board/later_phases.py --write, the tag's neg .checks\n"]
    for d in sorted(os.listdir(tests_dir)):
        if not d.startswith("neg"):
            continue
        for root, _, names in sorted(os.walk(os.path.join(tests_dir, d))):
            for n in sorted(names):
                if not n.endswith(".check"):
                    continue
                path = os.path.join(root, n)
                with open(path, encoding="utf-8", errors="replace") as fh:
                    check = fh.read()
                for r, block in matched(rows, check):
                    test = os.path.relpath(path, os.path.dirname(tests_dir))
                    out.append(f"{test[:-len('.check')]}\t{r[1]}\t{json.dumps(block, ensure_ascii=False)}\n")
    return "".join(out)


def self_test():
    files = {
        COMPILER: "List(new Typer) ::\nList(new Erasure) ::\nList(new Mixin, new Tail.Rec) ::\nList(new GenBCode) ::\ndef newRun = new Run\n",
        "compiler/src/dotty/tools/dotc/transform/Erasure.scala":
            'report.error(em"$sym is declared as `erased`, but is in fact used", pos)\n'
            "report.error(ErasedNotPure(t), pos)\n"
            '      val msg =\n        if inl then em"""$s is declared as `inline`, but was not inlined\n                       |Try more"""\n        else em"$s is never pure in this place"\n      report.error(msg, tree.srcPos)\n',
        "compiler/src/dotty/tools/dotc/transform/Mixin.scala":
            'report.error("parameterized trait is indirectly implemented, needs to be direct", pos)\nreport.error(DoubleDefinition(a, b), pos)\n',
        "compiler/src/dotty/tools/dotc/transform/Rec.scala": "report.error(ex.toMessage)\n",
        "compiler/src/dotty/tools/backend/jvm/GenBCode.scala": 'report.error(s"exception from future: $ex, (${Option(ex.getCause())})")\n',
        "compiler/src/dotty/tools/dotc/typer/Checking.scala": "fail(DoubleDefinition(a, b))\n",
        "compiler/src/dotty/tools/dotc/typer/Typer.scala": 'report.error("a typer error of its own text", pos)\n',
        "compiler/src/dotty/tools/dotc/Run.scala": 'report.error("not a phase of the table", pos)\n',
        MESSAGES: "class ErasedNotPure(t: Tree)(using Context)\n  extends TypeMsg(ErasedNotPureID) {\n}\n"
                  "class DoubleDefinition(a: Symbol, b: Symbol)(using Context)\nextends NamingMsg(DoubleDefinitionID):\n",
        IDS: "  case DoubleDefinitionID // errorNumber: 120\n  case ErasedNotPureID // errorNumber: 217\n",
    }
    rows = generate(PlantedTree(files))
    got = [(p, s, k, pat) for p, s, k, pat in rows]
    want = [
        ("Erasure", "Erasure.scala:1", "interpolated", {"text": [["is declared as `erased`, but is in fact used"]]}),
        ("Erasure", "Erasure.scala:2", "class ErasedNotPure", {"id": "E217"}),
        ("Erasure", "Erasure.scala:7", "value", {"text": [["is declared as `inline`, but was not inlined Try more"], ["is never pure in this place"]]}),
        ("GenBCode", "GenBCode.scala:1", "interpolated", {"text": [["exception from future"]]}),
        ("Mixin", "Mixin.scala:1", "literal", {"text": [["parameterized trait is indirectly implemented, needs to be direct"]]}),
        ("Mixin", "Mixin.scala:2", "class DoubleDefinition E120, also raised before: Checking.scala", None),
        ("Rec", "Rec.scala:1", "other", None),
    ]
    bad = []
    if got != want:
        bad.append(f"the table: {got}")
    def check(name, text, sites):
        found = [r[1] for r, _ in matched(rows, text)]
        if found != sites:
            bad.append(f"{name}: matched {found}, not {sites}")
    check("an interpolation's fragment, the gutter cut", "-- Error: t.scala:7:7 ---\n7 |  foo0(x) // error\n  |       ^\n  |       method x is declared as\n  |       `erased`, but is in fact used\n", ["Erasure.scala:1"])
    check("a class by its id alone", "-- [E217] Type Error: t.scala:8:18 ---\n8 |  foo1(x)\n  |  argument to an erased parameter fails to be a pure expression\n", ["Erasure.scala:2"])
    check("a value's second alternative", "-- Error: t.scala:1:1 ---\n  | value v is never pure in this place\n", ["Erasure.scala:7"])
    check("a class an earlier phase raises too: no match", "-- [E120] Naming Error: t.scala:1:1 ---\n  | Double definition\n", [])
    check("the typer's own text: no match", "-- Error: t.scala:1:1 ---\n  | a typer error of its own text\n", [])
    check("two errors, each its own", "-- Error: a ---\n  | parameterized trait is indirectly implemented, needs to be direct\n-- [E217] Type Error: b ---\n  | x\n", ["Mixin.scala:1", "Erasure.scala:2"])
    print("\n".join(bad) or "later_phases: the self-test's table and its matches hold")
    return 0 if not bad else 1


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if len(argv) == 3 and argv[0] in ("--write", "--check"):
        rows = generate(GitTree(argv[1]))
        table, tests = table_text(rows), tests_text(rows, argv[2])
        if argv[0] == "--write":
            open(TABLE, "w", encoding="utf-8").write(table)
            open(TESTS, "w", encoding="utf-8").write(tests)
            print(f"later_phases: {len(rows)} sites, {tests.count(chr(10)) - 1} errors of the tag's neg .checks written")
            return 0
        same = open(TABLE, encoding="utf-8").read() == table and open(TESTS, encoding="utf-8").read() == tests
        print("later_phases: the committed files are the generator's output" if same else "later_phases: the committed files differ from the generator's output (--write)")
        return 0 if same else 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
