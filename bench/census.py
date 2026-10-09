#!/usr/bin/env python3
"""How much of a library's source teq parses, and what accepting each missing construct would add.

  python3 bench/census.py <corpus dir> [teq binary]

The corpus holds one directory per library with its unpacked sources jar; bench/corpus.sh fills
it from the coursier cache. Every file is run through `teq compiler check` and its syntax errors (what the
parser rejects, and what the namer refuses as outside the language) are attributed to a
construct, by the message and by the source line, since a rejected `throw` or an old-style `if`
leaves errors behind it. A file "parses" when it has no such error; other errors, such as a
missing library, are not counted. The table then shows the share of files that parse today and,
cumulatively, when each construct is accepted in the order given.
"""
import collections
import concurrent.futures
import os
import re
import subprocess
import sys

SYNTAX = re.compile(
    r"^(expected|unexpected|unterminated|unmatched|'.*' is not part|Scala 2 implicits|imports are only"
    r"|malformed|invalid|a val needs|an extension needs|only defs|.* is not supported|.* are not supported"
    r"|a package object cannot|`inline`|inline)"
)
ORDER = [
    "implicit clauses",
    "Scala 2 control syntax",
    "throw/try/null",
    "return",
    "package object parents",
    "annotated type params",
    "self types",
    "secondary constructors",
    "capture checking syntax",
    "inline",
]


def files_of(lib):
    out = []
    for dp, _, fn in os.walk(lib):
        out += [os.path.join(dp, f) for f in fn if f.endswith(".scala")]
    return sorted(out)


def token_at(lines, line, col):
    if line > len(lines):
        return ""
    m = re.match(r"[A-Za-z_]\w*|\S", lines[line - 1][col - 1:])
    return m.group(0) if m else ""


def classify(msg, tok, text):
    if msg.startswith("Scala 2 implicits") or (msg == "expected identifier, found keyword" and tok == "implicit"):
        return "implicit clauses"
    if (
        msg.startswith("expected 'then'")
        or msg.startswith("expected 'do'")
        or (msg.startswith("expected ')'") and re.search(r"\b(if|while|for)\s*\(", text))
        or (msg == "expected an expression, found keyword" and tok in ("null", "throw", "new") and re.search(r"\bif\s*\(", text))
    ):
        return "Scala 2 control syntax"
    if tok in ("throw", "try", "null") or msg.startswith(("'throw'", "'try'", "'null'")):
        return "throw/try/null"
    if msg.startswith("a package object cannot"):
        return "package object parents"
    if msg.startswith("self types"):
        return "self types"
    if msg.startswith("'inline'") or "inline" in msg:
        return "inline"
    if msg == "expected identifier, found '@'":
        return "annotated type params"
    if msg.startswith("unexpected indentation"):
        return "brace on its own line"
    if msg == "expected identifier, found keyword" and tok == "this":
        return "secondary constructors"
    if msg == "expected identifier, found keyword":
        return f"keyword {tok}"
    if re.search(r"\b(if|while|for)\s*\(|^\s*else\b|\)\s*else\b", text):
        return "Scala 2 control syntax"
    if re.search(r"\breturn\b", text):
        return "return"
    if re.search(r"\bthrow\b|\btry\b|\bcatch\b|\bfinally\b|\bnull\b", text):
        return "throw/try/null"
    if re.search(r"\^\{|\]\^|\^\s*[,)=]|\^$|\bcap\b|CapSet|\^\s*\]", text):
        return "capture checking syntax"
    if re.search(r"\bimplicit\b", text) or (msg.startswith("expected a definition") and re.search(r"^\s*\w+\s*:\s*\w", text)):
        return "implicit clauses"
    if re.search(r"\[@", text):
        return "annotated type params"
    if msg.startswith("expected '=', found") and re.search(r"^\s*(override |final |protected |private )*(def|val|type)\b", text):
        return "abstract member elsewhere"
    if msg.startswith(("class inheritance", "a parent has to be a trait")):
        return "class inheritance"
    return "other: " + msg[:50]


def run(teq, f):
    try:
        r = subprocess.run([teq, "compiler", "check", f], capture_output=True, text=True, timeout=30)
    except subprocess.TimeoutExpired:
        return f, ["other: timeout"]
    lines = open(f, errors="replace").read().split("\n")
    cats = []
    for l in r.stderr.splitlines():
        if ": error: " not in l:
            continue
        loc, msg = l.split(": error: ", 1)
        if not SYNTAX.search(msg):
            continue
        _, line, col = loc.rsplit(":", 2)
        line, col = int(line), int(col)
        text = lines[line - 1] if line <= len(lines) else ""
        cats.append(classify(msg, token_at(lines, line, col), text))
    return f, cats


def main():
    corpus = sys.argv[1]
    teq = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "target", "release", "teq")
    libs = sorted(d for d in os.listdir(corpus) if os.path.isdir(os.path.join(corpus, d)))
    per_lib = {}
    by_cat = collections.Counter()
    with concurrent.futures.ThreadPoolExecutor(8) as ex:
        for lib in libs:
            fs = files_of(os.path.join(corpus, lib))
            if not fs:
                continue
            res = list(ex.map(lambda f: run(teq, f), fs))
            per_lib[lib] = res
            for _, cs in res:
                for c in set(cs):
                    by_cat[c] += 1
    print("files blocked, per construct:")
    for c, n in by_cat.most_common(24):
        print(f"  {n:4} {c}")
    print()
    print("share of files that parse today and when each construct is accepted in turn:")
    heads = ["today"] + ["+" + o for o in ORDER]
    print("library".ljust(28) + "".join(h[:13].rjust(14) for h in heads))
    totals = collections.Counter()
    n_all = 0
    for lib, res in per_lib.items():
        row = []
        for i in range(len(ORDER) + 1):
            ok = set(ORDER[:i])
            n = sum(1 for _, cs in res if all(c in ok for c in cs))
            row.append(n)
            totals[i] += n
        n_all += len(res)
        print(f"{lib[:27]:28}" + "".join(f"{100 * n // len(res):12}% " for n in row))
    print("all".ljust(28) + "".join(f"{100 * totals[i] // n_all:12}% " for i in range(len(ORDER) + 1)))


main()
