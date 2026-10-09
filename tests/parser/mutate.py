#!/usr/bin/env python3
"""The mutation corpus of the parser's recovery.

  mutate.py manifest            writes tests/parser/manifest.tsv from tests/cases (done once; the
                                manifest is frozen, a case added later stays out of it, and its
                                files are copied to tests/parser/corpus, which the corpus reads,
                                so that an edit of a case changes nothing here)
  mutate.py generate <dir>      writes every mutant of the corpus to <dir>/<id>.scala, the index
                                to <dir>/mutants.tsv and prints the corpus's digest

The corpus is a function of the manifest, the vocabulary below, SEED and PER_OPERATOR alone: the
sites of an operator are the matches of its rule in every file of the manifest, in manifest order
and then by offset, from which a splitmix64 generator seeded with SEED and the operator's name
picks PER_OPERATOR, kept in that order. Nothing here depends on teq or on Python's own random
numbers. The digest is the SHA-256 of every mutant's id, operator and text; tests/parser.sh
refuses a corpus whose digest is not the one the baseline was recorded for.

A mutant is one edit of one file: the bytes [start, end) replaced by a text. Each operator is
classified: valid (the program stays well formed, a type error at most), lexical (a token the
scanner rejects) or syntactic.
"""
import hashlib
import os
import re
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
MANIFEST = os.path.join(ROOT, "tests", "parser", "manifest.tsv")
# The manifest's files as they were when it was frozen, so that an edit of a case leaves the
# corpus alone; a manifest path names its file here by its base name.
CORPUS = os.path.join("tests", "parser", "corpus")


def frozen(path):
    return os.path.join(CORPUS, os.path.basename(path))
SEED = 0x7E9_2026_0930
PER_OPERATOR = 50

KEYWORDS = set("""abstract case catch class def do else enum export extends false final finally for given if implicit
import lazy match new null object override package private protected return sealed super then throw trait true try type
val var while with yield""".split())
DEF_KEYWORDS = {"val", "var", "def", "class", "trait", "object", "enum", "type", "given"}
OP_CHARS = set("!#%&*+-/:<=>?@\\^|~")


class Tok:
    __slots__ = ("kind", "text", "start", "end", "line", "col", "first")

    def __init__(self, kind, text, start, end, line, col, first):
        self.kind, self.text, self.start, self.end = kind, text, start, end
        self.line, self.col, self.first = line, col, first

    def __repr__(self):
        return f"{self.kind}:{self.text!r}@{self.start}"


def lex(src):
    """Scala tokens, approximately: enough to find the sites of the operators deterministically.
    Kinds: id, kw, op, punct, str, char, num, quote."""
    toks = []
    i, n = 0, len(src)
    line, line_start = 1, 0
    first = True

    def push(kind, start, end):
        nonlocal first
        toks.append(Tok(kind, src[start:end], start, end, line, start - line_start, first))
        first = False

    def skip_string(j):
        # j at the opening quote(s); returns the offset after the closing quote(s)
        if src.startswith('"""', j):
            k = src.find('"""', j + 3)
            if k < 0:
                return n
            k += 3
            while k < n and src[k] == '"':
                k += 1
            return k
        k = j + 1
        while k < n and src[k] not in '"\n':
            k += 2 if src[k] == '\\' else 1
        return min(k + 1, n)

    def skip_interp(j):
        triple = src.startswith('"""', j)
        k = j + (3 if triple else 1)
        while k < n:
            if triple and src.startswith('"""', k):
                k += 3
                while k < n and src[k] == '"':
                    k += 1
                return k
            if not triple and src[k] == '"':
                return k + 1
            if not triple and src[k] == '\n':
                return k
            if src.startswith("${", k):
                depth, k = 1, k + 2
                while k < n and depth:
                    if src[k] == '{':
                        depth += 1
                    elif src[k] == '}':
                        depth -= 1
                    elif src[k] == '"':
                        k = skip_string(k) - 1
                    k += 1
                continue
            if not triple and src[k] == '\\':
                k += 2
                continue
            k += 1
        return n

    while i < n:
        c = src[i]
        if c == '\n':
            i += 1
            line, line_start, first = line + 1, i, True
            continue
        if c in ' \t\r':
            i += 1
            continue
        if src.startswith("//", i):
            while i < n and src[i] != '\n':
                i += 1
            continue
        if src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    if src[j] == '\n':
                        line, line_start = line + 1, j + 1
                    j += 1
            i = j
            continue
        start = i
        if c == '"':
            i = skip_string(i)
            push("str", start, i)
            line += src.count('\n', start, i)
            if '\n' in src[start:i]:
                line_start = src.rfind('\n', start, i) + 1
            continue
        if c == "'":
            if i + 2 < n and src[i + 2] == "'" and src[i + 1] != '\\':
                i += 3
                push("char", start, i)
            elif i + 1 < n and src[i + 1] == '\\':
                j, nl = src.find("'", i + 2), src.find('\n', i)
                i = j + 1 if j > 0 and (nl < 0 or j < nl) else i + 2
                push("char", start, i)
            else:
                i += 1
                push("quote", start, i)
            continue
        if c == '`':
            j = i + 1
            while j < n and src[j] not in '`\n':
                j += 1
            i = j + 1 if j < n and src[j] == '`' else j
            push("id", start, i)
            continue
        if c.isdigit() or (c == '.' and i + 1 < n and src[i + 1].isdigit()):
            j = i + 1
            while j < n and (src[j].isalnum() or src[j] in '_.') and not (src[j] == '.' and not (j + 1 < n and src[j + 1].isdigit())):
                j += 1
            i = j
            push("num", start, i)
            continue
        if c.isalpha() or c == '_' or c == '$' or ord(c) >= 0x80:
            j = i + 1
            while j < n and (src[j].isalnum() or src[j] in '_$' or ord(src[j]) >= 0x80):
                j += 1
            if src[j - 1] == '_' and j < n and src[j] in OP_CHARS:
                while j < n and src[j] in OP_CHARS:
                    j += 1
            word = src[start:j]
            if j < n and src[j] == '"' and word not in KEYWORDS:
                i = skip_interp(j)
                push("str", start, i)
                line += src.count('\n', start, i)
                if '\n' in src[start:i]:
                    line_start = src.rfind('\n', start, i) + 1
                continue
            i = j
            push("kw" if word in KEYWORDS else "id", start, i)
            continue
        if c in OP_CHARS:
            j = i
            while j < n and src[j] in OP_CHARS and not src.startswith("//", j) and not src.startswith("/*", j):
                j += 1
            i = j
            push("op", start, i)
            continue
        i += 1
        push("punct", start, i)
    return toks


def at_depth0(toks, i, stop):
    """The index of the first token from i on at bracket depth 0 for which stop(tok) holds, or -1
    once the depth goes below 0."""
    depth = 0
    for j in range(i, len(toks)):
        t = toks[j]
        if depth == 0 and stop(t):
            return j
        if t.text in "([{" and t.kind == "punct":
            depth += 1
        elif t.text in ")]}" and t.kind == "punct":
            depth -= 1
            if depth < 0:
                return -1
    return -1


def new_statement(t):
    return t.first or (t.kind == "kw" and t.text in DEF_KEYWORDS)


# Each rule maps a file's text and tokens to its sites: (start, end, replacement).

def r_def_eq(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "kw" and t.text == "def":
            j = at_depth0(toks, i + 1, lambda u: (u.kind == "op" and u.text == "=") or (u is not toks[i + 1] and new_statement(u)))
            if j > 0 and toks[j].text == "=":
                yield toks[j].start, toks[j].end, ""


def r_def_result_type(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "kw" and t.text == "def":
            j = at_depth0(toks, i + 2, lambda u: (u.kind == "op" and u.text in (":", "=")) or new_statement(u))
            if j > 0 and toks[j].text == ":":
                k = at_depth0(toks, j + 1, lambda u: (u.kind == "op" and u.text == "=") or new_statement(u))
                if k > j + 1 and toks[k].text == "=":
                    yield toks[j + 1].start, toks[k - 1].end, ""


def punct_deleted(ch):
    def rule(src, toks):
        for t in toks:
            if t.kind == "punct" and t.text == ch:
                yield t.start, t.end, ""
    return rule


def r_case_pattern(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "kw" and t.text == "case" and i + 1 < len(toks) and toks[i + 1].text not in ("class", "object"):
            j = at_depth0(toks, i + 1, lambda u: (u.kind == "op" and u.text == "=>") or u.first or u.text == "case")
            if j > i + 1 and toks[j].text == "=>":
                yield toks[i + 1].start, toks[j - 1].end, ""


def r_case_arrow(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "kw" and t.text == "case" and i + 1 < len(toks) and toks[i + 1].text not in ("class", "object"):
            j = at_depth0(toks, i + 1, lambda u: (u.kind == "op" and u.text == "=>") or u.first or u.text == "case")
            if j > i + 1 and toks[j].text == "=>":
                yield toks[j].start, toks[j].end, ""


def class_names(toks):
    for i, t in enumerate(toks):
        if t.kind == "kw" and t.text in ("class", "trait", "object", "enum") and i + 1 < len(toks) and toks[i + 1].kind == "id":
            yield i


def r_class_name(src, toks):
    for i in class_names(toks):
        yield toks[i + 1].start, toks[i + 1].end, ""


def r_class_parent(src, toks):
    for i in class_names(toks):
        j = at_depth0(toks, i + 2, lambda u: (u.kind == "kw" and u.text == "extends") or u.first or (u.kind == "op" and u.text == ":") or u.text == "{")
        if j > 0 and toks[j].text == "extends" and j + 1 < len(toks) and toks[j + 1].kind == "id" and not toks[j + 1].first:
            k = j + 1
            while k + 2 < len(toks) and toks[k + 1].text == "." and toks[k + 2].kind == "id":
                k += 2
            yield toks[j + 1].start, toks[k].end, ""


def r_enum_comma(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "kw" and t.text == "case" and i + 2 < len(toks) and toks[i + 1].kind == "id" and toks[i + 2].text == ",":
            yield toks[i + 2].start, toks[i + 2].end, ""


def r_import_trunc(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "kw" and t.text == "import":
            j = i + 1
            last_dot = -1
            while j < len(toks) and not toks[j].first and toks[j].text != ";":
                if toks[j].text == ".":
                    last_dot = j
                j += 1
            if last_dot > 0:
                yield toks[last_dot].end, toks[j - 1].end, ""


def r_type_arg_comma(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "punct" and t.text == "]" and i > 0 and toks[i - 1].kind == "id":
            yield t.start, t.start, ","


def line_continues(toks, i):
    t, u = toks[i], toks[i + 1] if i + 1 < len(toks) else None
    return u is not None and u.first and t.kind in ("id", "num", "str", "char") and u.kind in ("kw", "id") and u.col <= t.col


def r_op_eol(src, toks):
    for i in range(len(toks) - 1):
        if line_continues(toks, i):
            yield toks[i].end, toks[i].end, " +"


def lines_in_bodies(src, toks):
    """The lines that start a statement indented past the line before them."""
    for i in range(1, len(toks)):
        t, p = toks[i], toks[i - 1]
        if t.first and t.col > 0 and not p.first and p.line < t.line:
            prev_first = next((toks[k] for k in range(i - 1, -1, -1) if toks[k].first), None)
            if prev_first is not None and prev_first.col == t.col and t.kind in ("kw", "id"):
                yield t


def r_indent_plus(src, toks):
    for t in lines_in_bodies(src, toks):
        yield t.start, t.start, " "


def r_indent_minus(src, toks):
    for t in lines_in_bodies(src, toks):
        yield t.start - 1, t.start, ""


def end_markers(src, toks):
    for i, t in enumerate(toks):
        if t.first and t.kind == "id" and t.text == "end" and i + 1 < len(toks) and not toks[i + 1].first:
            if i + 2 >= len(toks) or toks[i + 2].first:
                yield i


def r_end_rename(src, toks):
    for i in end_markers(src, toks):
        u = toks[i + 1]
        yield u.end, u.end, "x"


def r_colon_to_brace(src, toks):
    for i in class_names(toks):
        j = at_depth0(toks, i + 2, lambda u: (u.kind == "op" and u.text == ":") or u.first or u.text == "{" or u.text == "=")
        if j > 0 and toks[j].text == ":" and (j + 1 >= len(toks) or toks[j + 1].first):
            yield toks[j].start, toks[j].end, " {"


def r_brace_to_colon(src, toks):
    for i in class_names(toks):
        j = at_depth0(toks, i + 2, lambda u: u.text == "{" or u.first or (u.kind == "op" and u.text == ":") or u.text == "=")
        if j > 0 and toks[j].text == "{" and (j + 1 >= len(toks) or toks[j + 1].first):
            yield toks[j].start, toks[j].end, ":"


def r_del_token(src, toks):
    for t in toks:
        yield t.start, t.end, ""


def r_str_unterminated(src, toks):
    for t in toks:
        if t.kind == "str" and t.text.startswith('"') and not t.text.startswith('"""') and len(t.text) >= 2 and t.text.endswith('"'):
            yield t.end - 1, t.end, ""


def r_backquote(src, toks):
    for t in toks:
        if t.kind == "id" and not t.text.startswith("`") and t.text[0].isalpha():
            yield t.start, t.start, "`"


def r_bad_escape(src, toks):
    for t in toks:
        if t.kind == "str" and t.text.startswith('"') and not t.text.startswith('"""') and len(t.text) >= 2:
            yield t.start + 1, t.start + 1, "\\q"


def r_bad_char(src, toks):
    for t in toks:
        if t.first and t.kind in ("kw", "id"):
            yield t.end, t.end, " §"


def r_lit_string(src, toks):
    for t in toks:
        if t.kind == "num" and re.fullmatch(r"\d+", t.text):
            yield t.start, t.end, '"s"'


SOFT_KEYWORDS = {"inline", "using", "erased", "opaque", "transparent", "open", "infix", "derives", "extension", "as", "end"}


def r_ref_rename(src, toks):
    for i, t in enumerate(toks):
        if t.kind == "id" and i > 0 and toks[i - 1].text in ("=", "(", ",") and t.text[0].islower() and t.text not in SOFT_KEYWORDS:
            after = toks[i + 1] if i + 1 < len(toks) else None
            if after is not None and not after.first and (after.text in ("=", ":") or after.kind in ("id", "kw")):
                continue
            yield t.end, t.end, "_x"


# The vocabulary, in its frozen order: name, class, rule.
VOCABULARY = [
    ("def-eq", "syntactic", r_def_eq),
    ("def-result-type", "syntactic", r_def_result_type),
    ("rparen", "syntactic", punct_deleted(")")),
    ("rbracket", "syntactic", punct_deleted("]")),
    ("rbrace", "syntactic", punct_deleted("}")),
    ("case-pattern", "syntactic", r_case_pattern),
    ("case-arrow", "syntactic", r_case_arrow),
    ("class-name", "syntactic", r_class_name),
    ("class-parent", "syntactic", r_class_parent),
    ("enum-comma", "syntactic", r_enum_comma),
    ("import-trunc", "syntactic", r_import_trunc),
    ("type-arg-comma", "syntactic", r_type_arg_comma),
    ("op-eol", "syntactic", r_op_eol),
    ("indent-plus", "syntactic", r_indent_plus),
    ("indent-minus", "syntactic", r_indent_minus),
    ("end-rename", "syntactic", r_end_rename),
    ("colon-to-brace", "syntactic", r_colon_to_brace),
    ("brace-to-colon", "syntactic", r_brace_to_colon),
    ("del-token", "syntactic", r_del_token),
    ("str-unterminated", "lexical", r_str_unterminated),
    ("backquote", "lexical", r_backquote),
    ("bad-escape", "lexical", r_bad_escape),
    ("bad-char", "lexical", r_bad_char),
    ("lit-string", "valid", r_lit_string),
    ("ref-rename", "valid", r_ref_rename),
]

MASK = (1 << 64) - 1


class SplitMix:
    def __init__(self, seed):
        self.state = seed & MASK

    def next(self):
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        return z ^ (z >> 31)

    def below(self, n):
        return self.next() % n


def operator_seed(name):
    return SEED ^ int.from_bytes(hashlib.sha256(name.encode()).digest()[:8], "big")


def read_manifest():
    entries = []
    with open(MANIFEST) as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            path, teq_flags, scalac_flags = (line.rstrip("\n").split("\t") + ["", ""])[:3]
            entries.append((path, teq_flags, scalac_flags))
    return entries


# Cases scalac rejects on the JVM: `Map[String, Int](null -> 1)` is accepted on Scala.js alone,
# `Vector.unsafeArray` is a member of teq's std.
PLATFORM_DEPENDENT = {"map_key_contract.scala", "vector_retention.scala"}


def make_manifest():
    rows = []
    cases = os.path.join(ROOT, "tests", "cases")
    for name in sorted(os.listdir(cases)):
        if not name.endswith(".scala"):
            continue
        text = open(os.path.join(cases, name), encoding="utf-8").read()
        if re.search(r"^// jars: |^//> using dep", text, re.M) or "scalajs" in text or name in PLATFORM_DEPENDENT:
            continue
        teq = (re.findall(r"^// teq: (.*)$", text, re.M) or [""])[0].strip()
        scalac = []
        if "--strict-equality" in teq:
            scalac.append("-language:strictEquality")
        if "--kind-projector" in teq:
            scalac.append("-Xkind-projector")
        inlines = re.search(r"--max-inlines (\d+)", teq)
        if inlines:
            scalac += ["-Xmax-inlines", inlines.group(1)]
        for opts in re.findall(r"^//> using options (.*)$", text, re.M):
            for o in opts.split():
                if o not in scalac:
                    scalac.append(o)
        rows.append(f"tests/cases/{name}\t{teq}\t{' '.join(scalac)}")
    with open(MANIFEST, "w") as f:
        f.write("# The files of the mutation corpus (tests/parser/mutate.py), frozen: path, teq's flags, scalac's flags.\n")
        f.write("# The single-file cases of tests/cases with no jar, no dependency and no Scala.js that scalac accepts on the JVM, as of 2026-09-30.\n")
        f.write("\n".join(rows) + "\n")
    os.makedirs(os.path.join(ROOT, CORPUS), exist_ok=True)
    for row in rows:
        path = row.split("\t")[0]
        shutil.copyfile(os.path.join(ROOT, path), os.path.join(ROOT, frozen(path)))
    print(f"{len(rows)} files")


def mutants():
    entries = read_manifest()
    files = []
    for path, teq_flags, scalac_flags in entries:
        src = open(os.path.join(ROOT, frozen(path)), encoding="utf-8").read()
        files.append((path, teq_flags, scalac_flags, src, lex(src)))
    out = []
    for name, klass, rule in VOCABULARY:
        sites = []
        for fi, (path, _, _, src, toks) in enumerate(files):
            seen = set()
            for start, end, text in rule(src, toks):
                if (start, end, text) not in seen:
                    seen.add((start, end, text))
                    sites.append((fi, start, end, text))
        rng = SplitMix(operator_seed(name))
        k = min(PER_OPERATOR, len(sites))
        picked = list(range(len(sites)))
        for i in range(k):
            j = i + rng.below(len(sites) - i)
            picked[i], picked[j] = picked[j], picked[i]
        for idx in sorted(picked[:k], key=lambda x: sites[x][:2]):
            out.append((name, klass, sites[idx]))
    result = []
    for n, (name, klass, (fi, start, end, text)) in enumerate(out):
        path, teq_flags, scalac_flags, src, _ = files[fi]
        mutated = src[:start] + text + src[end:]
        result.append({
            "id": f"m{n:04d}", "operator": name, "class": klass, "source": path, "file": frozen(path), "start": start, "end": end,
            "text": text, "teq_flags": teq_flags, "scalac_flags": scalac_flags, "mutated": mutated,
        })
    return result


def digest(ms):
    h = hashlib.sha256()
    for m in ms:
        h.update(f"{m['id']}\t{m['operator']}\t{m['source']}\t{m['start']}\t{m['end']}\t{m['text']}\t{m['teq_flags']}\t{m['scalac_flags']}\n".encode())
        h.update(m["mutated"].encode())
    return h.hexdigest()


def generate(out_dir):
    ms = mutants()
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, "mutants.tsv"), "w") as idx:
        for m in ms:
            with open(os.path.join(out_dir, m["id"] + ".scala"), "w", encoding="utf-8") as f:
                f.write(m["mutated"])
            text = m["text"].encode("unicode_escape").decode()
            idx.write(f"{m['id']}\t{m['operator']}\t{m['class']}\t{m['source']}\t{m['start']}\t{m['end']}\t{text}\t{m['teq_flags']}\t{m['scalac_flags']}\n")
    d = digest(ms)
    with open(os.path.join(out_dir, "digest"), "w") as f:
        f.write(d + "\n")
    print(f"{len(ms)} mutants, digest {d}")


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "manifest":
        make_manifest()
    elif len(sys.argv) == 3 and sys.argv[1] == "generate":
        generate(sys.argv[2])
    else:
        print(__doc__)
        sys.exit(2)
