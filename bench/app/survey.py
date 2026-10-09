#!/usr/bin/env python3
"""Counts the shape of a Scala code base without recording its content:

  python3 bench/app/survey.py <root>... [--json out.json] [--name=<root>:<label>]... [--names]

Every root is a module. The report is numbers only: files and lines, case classes and their
field-count distribution, enums and their case counts, sealed hierarchies and their depth, derives
clauses by type-class set, givens by kind, extension methods, opaque types, inline defs, macro
definitions and their call sites (each macro a row of counts, not a name), match expressions and
their case counts, for-comprehensions by generator count, lambdas, class/trait/object counts,
inheritance depth, imports per file, package depth, string interpolations and pattern kinds.
No identifier is kept: the derives clauses are reported as rows of (set size, count), and the
macros as rows of (kind, sites); `--names` prints the type-class names of the derives clauses
to stdout, for reading the rows.
The counting is lexical (regular expressions over comment-stripped, string-blanked text), so
the figures are approximate, and comparable between two code bases surveyed by the same tool,
which is what they are for: bench/app/shape.json holds the survey of an application and
bench/app/gen.py takes it as its parameter set."""
import json
import os
import re
import sys
from collections import Counter, defaultdict

FIELD_BUCKETS = [(0, "0"), (1, "1"), (2, "2"), (3, "3"), (4, "4"), (5, "5-8"), (9, "9-16"), (17, "17-32"), (33, "33+")]
CASE_BUCKETS = [(1, "1"), (2, "2"), (3, "3"), (4, "4-6"), (7, "7-12"), (13, "13-24"), (25, "25-48"), (49, "49+")]
SMALL_BUCKETS = [(0, "0"), (1, "1"), (2, "2"), (3, "3"), (4, "4-5"), (6, "6-9"), (10, "10+")]
IMPORT_BUCKETS = [(0, "0"), (1, "1-2"), (3, "3-5"), (6, "6-10"), (11, "11-20"), (21, "21+")]
LINE_BUCKETS = [(0, "1-50"), (51, "51-100"), (101, "101-200"), (201, "201-400"), (401, "401-800"), (801, "801+")]


def bucket(n, buckets):
    label = buckets[0][1]
    for lo, name in buckets:
        if n >= lo:
            label = name
    return label


def histogram(values, buckets):
    h = Counter(bucket(v, buckets) for v in values)
    return {name: h.get(name, 0) for _, name in buckets if h.get(name, 0)}


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    out = []
    for line in text.split("\n"):
        i = 0
        in_str = None
        while i < len(line):
            c = line[i]
            if in_str:
                if c == "\\":
                    i += 1
                elif c == in_str:
                    in_str = None
            elif c in "\"'" and (c == '"' or (i + 2 < len(line) and line[i + 2] == "'")):
                in_str = c
            elif c == "/" and line[i + 1 : i + 2] == "/":
                line = line[:i]
                break
            i += 1
        out.append(line)
    return "\n".join(out)


TRIPLE = re.compile(r'"""[\s\S]*?"""')
STRING = re.compile(r'"(?:\\.|[^"\\\n])*"')
CHAR = re.compile(r"'(?:\\.|[^'\\])'")


def blank_strings(text):
    text = TRIPLE.sub('""', text)
    text = STRING.sub('""', text)
    return CHAR.sub("' '", text)


def paren_group(text, start):
    """The text of the balanced parenthesised group that opens at text[start]."""
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i]
    return text[start + 1 :]


def bracket_group(text, start):
    depth = 0
    for i in range(start, len(text)):
        if text[i] == "[":
            depth += 1
        elif text[i] == "]":
            depth -= 1
            if depth == 0:
                return text[start + 1 : i]
    return text[start + 1 :]


def split_top(text, sep=","):
    parts, depth, cur = [], 0, []
    for c in text:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == sep and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(c)
    parts.append("".join(cur))
    return [p for p in parts if p.strip()]


def param_count(text, start):
    """Number of parameters in the first non-using parameter list at text[start] ('(')."""
    group = paren_group(text, start)
    if group.lstrip().startswith("using "):
        return 0
    return len(split_top(group))


IDENT = r"[A-Za-z_][A-Za-z0-9_]*"
DEF_KINDS = re.compile(
    r"(?m)^(?P<indent>[ \t]*)(?P<mods>(?:(?:final|abstract|sealed|private|protected|open|implicit|lazy|transparent|inline|case|override)(?:\[[^\]]*\])?\s+)*)"
    r"(?P<kind>class|trait|object|enum)\s+(?P<name>" + IDENT + r"|`[^`]+`)"
)
DERIVES = re.compile(r"\bderives\s+((?:" + IDENT + r"(?:\.[A-Za-z_][A-Za-z0-9_]*)*(?:\[[^\]]*\])?\s*,\s*)*" + IDENT + r"(?:\.[A-Za-z_][A-Za-z0-9_]*)*(?:\[[^\]]*\])?)")
EXTENDS = re.compile(r"\bextends\s+([^{:=\n]+?)(?=\s*(?:derives\b|\{|:\s*$|:\s*\n|=|\n|$))")
GIVEN = re.compile(r"(?m)^[ \t]*(?:(?:private|protected|inline|transparent|final)(?:\[[^\]]*\])?\s+)*given\b(?P<rest>[^\n]*)")
EXTENSION = re.compile(r"(?m)^[ \t]*extension\s*(?P<tparams>\[[^\]]*\])?\s*\(")
INLINE_DEF = re.compile(r"(?m)^[ \t]*(?:(?:private|protected|final|override)(?:\[[^\]]*\])?\s+)*(?:transparent\s+)?inline\s+def\s+(?P<name>" + IDENT + r"|[!#%&*+\-/:<=>?@\\^|~]+)")
MACRO_DEF = re.compile(r"(?m)^[ \t]*(?:(?:private|protected|final|override)(?:\[[^\]]*\])?\s+)*(?:transparent\s+)?inline\s+def\s+(?P<name>" + IDENT + r"|[!#%&*+\-/:<=>?@\\^|~]+)(?P<sig>[^\n]*?)=\s*\$\{")
CASE_LINE = re.compile(r"(?m)^[ \t]*case\s+(?!class\b|object\b)(?P<pat>.*?)\s*=>")
FOR_START = re.compile(r"(?m)(?<![A-Za-z0-9_.])for\b(?!\s*(?:each|all|\w*\s*\())")
GENERATOR = re.compile(r"(?m)^[ \t]*(?:\(|" + IDENT + r"|_)[^\n]*?<-")
LITERAL = re.compile(r'(?<![A-Za-z0-9_])(?P<prefix>' + IDENT + r')?(?:"""[\s\S]*?"""|"(?:\\.|[^"\\\n])*")')
INTERPOLATION = re.compile(r"(?<![A-Za-z0-9_])(" + IDENT + r')(?=""")|(?<![A-Za-z0-9_])(' + IDENT + r')(?=")')
PACKAGE = re.compile(r"(?m)^package\s+([A-Za-z_][A-Za-z0-9_.]*)")
IMPORT = re.compile(r"(?m)^[ \t]*import\s+([^\n]+)")
TOP_IMPORT = re.compile(r"(?m)^import\s+")
ANNOTATION = re.compile(r"@(" + IDENT + r"(?:\.[A-Za-z_][A-Za-z0-9_]*)*)")
NEW_ANON = re.compile(r"\bnew\s+" + IDENT + r"(?:\.[A-Za-z_]\w*)*(?:\[[^\]]*\])?(?:\([^)]*\))?\s*(?::\s*$|\{)", re.M)
CONTEXT_BOUND = re.compile(r"\b" + IDENT + r"\s*:\s*(?:\{[^}]*\}|" + IDENT + r"(?:\.[A-Za-z_]\w*)*(?:\[[^\]]*\])?)(?=\s*[,\]])")
USING = re.compile(r"\(\s*using\b")
LAMBDA = re.compile(r"(?<![=!<>:?])=>")
PLACEHOLDER = re.compile(r"(?<![A-Za-z0-9_])_\.[A-Za-z_]")
UNION = re.compile(r":\s*[A-Za-z_][\w.\[\], ]*?\s+\|\s+[A-Za-z_]")
MATCH = re.compile(r"(?<![A-Za-z0-9_.])match\b")
OPAQUE = re.compile(r"\bopaque\s+type\b")
TYPE_ALIAS = re.compile(r"(?m)^[ \t]*(?:(?:private|protected|infix|opaque|final)\s+)*type\s+" + IDENT)
VAL = re.compile(r"(?m)^[ \t]*(?:(?:private|protected|final|override|implicit|lazy|inline)(?:\[[^\]]*\])?\s+)*val\s+")
LAZY_VAL = re.compile(r"\blazy\s+val\b")
VAR = re.compile(r"(?m)^[ \t]*(?:(?:private|protected|final|override)(?:\[[^\]]*\])?\s+)*var\s+")
DEF = re.compile(r"(?m)^[ \t]*(?:(?:private|protected|final|override|implicit|inline|transparent)(?:\[[^\]]*\])?\s+)*def\s+")
TOP_DEF = re.compile(r"(?m)^(?:(?:private|protected|inline|transparent)\s+)*def\s+")
IMPLICIT = re.compile(r"\bimplicit\s+(val|def|object|class|lazy val)\b")
IMPLICIT_PARAM = re.compile(r"\(\s*implicit\b")
SUMMON = re.compile(r"\bsummon\[")
CONVERSION = re.compile(r"\bConversion\[")
BY_NAME = re.compile(r":\s*=>\s*")
DEFAULT_ARG = re.compile(r"(?m)^[ \t]*" + IDENT + r"\s*:\s*[^=,\n]+=\s*[^>]")
VARARG = re.compile(r"\*\s*[,)]")
TYPE_PARAMS = re.compile(r"\b(?:class|trait|def|enum|type|given|extension)\s*" + IDENT + r"?\s*\[")
HIGHER_KINDED = re.compile(r"\[\s*" + IDENT + r"\[_\]")
TRY = re.compile(r"(?<![A-Za-z0-9_.])try\b")
THROW = re.compile(r"(?<![A-Za-z0-9_.])throw\b")
RETURN = re.compile(r"(?<![A-Za-z0-9_.])return\b")
TUPLE_VAL = re.compile(r"\bval\s*\(")
IF = re.compile(r"(?<![A-Za-z0-9_.])if\b")
WHILE = re.compile(r"(?<![A-Za-z0-9_.])while\b")
NAMED_ARG = re.compile(r"\(\s*" + IDENT + r"\s*=\s*[^=>]")
ASSIGN_OP = re.compile(r"\b:=\b|\s:=\s")


def scala_files(root):
    for dirpath, dirs, files in os.walk(root):
        dirs.sort()
        for f in sorted(files):
            if f.endswith(".scala"):
                yield os.path.join(dirpath, f)


def type_params_of(text, pos):
    """The type parameter list right after pos, or ''."""
    m = re.match(r"\s*\[", text[pos:])
    return bracket_group(text, pos + m.end() - 1) if m else ""


def header_of(text, pos):
    """The class header from pos: type parameters, every parameter list (over several lines), and
    the rest of the line the header ends on, plus a following line that carries only the
    extends or derives clause."""
    i = pos
    n = len(text)
    while i < n:
        m = re.match(r"\s*(?:private\s+|protected\s+)?(?:using\s+)?[\[(]", text[i:])
        if not m:
            break
        opener = text[i + m.end() - 1]
        group = bracket_group(text, i + m.end() - 1) if opener == "[" else paren_group(text, i + m.end() - 1)
        i = i + m.end() + len(group)
    line_end = text.find("\n", i)
    if line_end < 0:
        line_end = n
    header = text[pos:line_end]
    nxt = text[line_end + 1 : text.find("\n", line_end + 1) if text.find("\n", line_end + 1) > 0 else n]
    if re.match(r"\s*(?:derives|extends|with)\b", nxt):
        header += " " + nxt.strip()
    return header


def analyse_file(path, anonymous):
    raw = open(path, encoding="utf-8", errors="replace").read()
    lines = raw.split("\n")
    s = {}
    s["lines"] = len(lines) - (1 if raw.endswith("\n") else 0)
    s["blank"] = sum(1 for l in lines if not l.strip())
    s["comment_lines"] = sum(1 for l in lines if l.strip().startswith("//") or l.strip().startswith("*") or l.strip().startswith("/*"))
    code = strip_comments(raw)
    interps = Counter()
    for m in LITERAL.finditer(code):
        if m.group("prefix"):
            interps[m.group("prefix")] += 1
    s["triple_quoted"] = len(TRIPLE.findall(code))
    text = blank_strings(code)
    s["interpolations"] = {"s": interps.get("s", 0), "f": interps.get("f", 0), "raw": interps.get("raw", 0), "custom": sum(v for k, v in interps.items() if k not in ("s", "f", "raw"))}

    pkgs = PACKAGE.findall(text)
    s["package_depth"] = sum(p.count(".") + 1 for p in pkgs) if pkgs else 0
    imports = IMPORT.findall(text)
    s["imports"] = len(imports)
    s["wildcard_imports"] = sum(1 for i in imports if re.search(r"\.\*\s*$|\{[^}]*\*", i))
    s["given_imports"] = sum(1 for i in imports if "given" in i)
    s["local_imports"] = len(imports) - len(TOP_IMPORT.findall(text))

    defs = []
    for m in DEF_KINDS.finditer(text):
        mods = m.group("mods")
        kind = m.group("kind")
        name = m.group("name")
        end = m.end()
        tps = type_params_of(text, end)
        after = text[end:]
        pm = re.match(r"\s*(?:\[[^\]]*\])?\s*(?:private\s+|protected\s+)?\(", after)
        fields = param_count(text, end + pm.end() - 1) if pm and kind != "trait" else 0
        header = header_of(text, end)
        parents = []
        em = EXTENDS.search(header)
        if em:
            parents = [re.sub(r"[\[(].*", "", p.strip()).split(".")[-1] for p in re.split(r"\bwith\b|,", em.group(1)) if p.strip()]
        dm = DERIVES.search(header)
        derives = [re.sub(r"\[.*", "", d.strip()).split(".")[-1] for d in dm.group(1).split(",")] if dm else []
        indent = len(m.group("indent").expandtabs(2))
        defs.append({"kind": kind, "name": name, "case": "case " in mods, "sealed": "sealed" in mods, "abstract": "abstract" in mods, "fields": fields, "tparams": bool(tps.strip()), "hk": "[_]" in tps, "parents": parents, "derives": derives, "indent": indent, "pos": m.start(), "end": end})
    s["defs"] = defs

    enums = [d for d in defs if d["kind"] == "enum"]
    for e in enums:
        body_start = text.find("\n", e["end"])
        body_lines = []
        for line in text[body_start + 1 :].split("\n"):
            ind = len(line) - len(line.lstrip(" \t"))
            if line.strip() and ind <= e["indent"]:
                break
            body_lines.append(line)
        cases = 0
        class_cases = 0
        extends_cases = 0
        for line in body_lines:
            cm = re.match(r"\s*case\s+(.*)", line)
            if not cm or "=>" in line:
                continue
            names = split_top(cm.group(1).split(" extends ")[0])
            cases += len(names)
            if "(" in cm.group(1).split(" extends ")[0]:
                class_cases += 1
            if " extends " in cm.group(1):
                extends_cases += 1
        e["cases"] = cases
        e["class_cases"] = class_cases
        e["extends_cases"] = extends_cases
        e["params"] = e["fields"]

    givens = []
    for m in GIVEN.finditer(text):
        rest = m.group("rest").strip()
        kind = "alias"
        if re.match(r"(?:" + IDENT + r"\s*:\s*)?(?:\[[^\]]*\]\s*(?:=>|:)\s*)?Conversion\[", rest) or "Conversion[" in rest.split("=")[0]:
            kind = "conversion"
        elif re.search(r"\bwith\s*$|\bwith\s*\{|:\s*$", rest) and "=" not in rest.split(":")[-1]:
            kind = "instance"
        elif "=" not in rest:
            kind = "instance"
        parameterised = bool(re.match(r"(?:" + IDENT + r"\s*:\s*)?\[", rest))
        with_using = "(using" in rest or "=>" in rest.split("=")[0]
        named = bool(re.match(IDENT + r"\s*:", rest)) and not rest.startswith("Conversion")
        givens.append({"kind": kind, "parameterised": parameterised, "using": with_using, "named": named, "context_bound": bool(re.match(r"(?:" + IDENT + r"\s*:\s*)?\[[^\]]*:[^\]]*\]", rest))})
    s["givens"] = givens
    s["implicits"] = Counter(m.group(1) for m in IMPLICIT.finditer(text))
    s["implicit_params"] = len(IMPLICIT_PARAM.findall(text))
    s["summons"] = len(SUMMON.findall(text))
    s["using_clauses"] = len(USING.findall(text))
    s["context_bounds"] = sum(1 for m in CONTEXT_BOUND.finditer(text) if text[max(0, m.start() - 40) : m.start()].count("[") > text[max(0, m.start() - 40) : m.start()].count("]"))

    exts = []
    for m in EXTENSION.finditer(text):
        start = m.end() - 1
        group = paren_group(text, start)
        after = text[start + len(group) + 2 :]
        first_line = after.split("\n")[0]
        methods = 1 if re.search(r"\bdef\b", first_line) else 0
        if methods == 0:
            indent = len(text[text.rfind("\n", 0, m.start()) + 1 : m.start()].expandtabs(2)) if text.rfind("\n", 0, m.start()) >= 0 else 0
            for line in after.split("\n")[1:]:
                ind = len(line) - len(line.lstrip(" \t"))
                if line.strip() and ind <= indent:
                    break
                if re.search(r"\bdef\b", line):
                    methods += 1
        exts.append({"methods": max(methods, 1), "generic": bool(m.group("tparams")), "context_bound": bool(m.group("tparams") and ":" in m.group("tparams"))})
    s["extensions"] = exts
    s["opaque_types"] = len(OPAQUE.findall(text))
    s["type_aliases"] = len(TYPE_ALIAS.findall(text)) - s["opaque_types"]
    inline_defs = INLINE_DEF.findall(text)
    s["inline_defs"] = len(inline_defs)
    s["transparent_inline_defs"] = len(re.findall(r"\btransparent\s+inline\s+def\b", text))
    s["inline_params"] = len(re.findall(r"\(\s*inline\s+" + IDENT + r"\s*:", text)) + len(re.findall(r",\s*inline\s+" + IDENT + r"\s*:", text))
    s["inline_givens"] = len(re.findall(r"\binline\s+given\b", text))
    s["quoted_files"] = 1 if "scala.quoted" in text else 0
    macros = []
    for m in MACRO_DEF.finditer(text):
        name = m.group("name")
        sig = m.group("sig")
        before = text[: m.start()]
        owner = None
        for d in reversed([d for d in defs if d["pos"] < m.start()]):
            owner = d["name"]
            break
        indent = len(m.group(0)) - len(m.group(0).lstrip(" \t"))
        on_string_context = False
        for prev in reversed(text[: m.start()].split("\n")[:-1]):
            ind = len(prev) - len(prev.lstrip(" \t"))
            if prev.strip() and ind < indent:
                on_string_context = prev.lstrip().startswith("extension") and "StringContext" in prev
                break
        macros.append({"name": name, "owner": owner, "interpolator": on_string_context, "derived": name == "derived", "sig": sig})
    s["macros"] = macros

    groups = []
    current = []
    last_indent = None
    for line in text.split("\n"):
        cm = CASE_LINE.match(line)
        if cm:
            ind = len(line) - len(line.lstrip(" \t"))
            if current and last_indent is not None and ind != last_indent:
                groups.append(current)
                current = []
            current.append(cm.group("pat"))
            last_indent = ind
        elif line.strip() and current:
            ind = len(line) - len(line.lstrip(" \t"))
            if ind <= last_indent:
                groups.append(current)
                current = []
                last_indent = None
    if current:
        groups.append(current)
    s["match_keywords"] = len(MATCH.findall(text))
    s["case_groups"] = [len(g) for g in groups]
    pats = Counter()
    for g in groups:
        for p in g:
            core = p.split(" if ")[0] if " if " in p else p
            if " if " in p:
                pats["guard"] += 1
            if "|" in core and not re.search(r"\|\s*Null", core):
                pats["alternative"] += 1
            if "@" in core:
                pats["binder"] += 1
            if re.match(r"_$", core.strip()):
                pats["wildcard"] += 1
            elif re.match(r"\(", core.strip()):
                pats["tuple"] += 1
            elif re.match(r"(?:" + IDENT + r"\.)*[A-Z]\w*\(", core.strip()):
                pats["constructor"] += 1
            elif re.match(r"[a-z_]\w*\s*:\s*", core.strip()):
                pats["typed"] += 1
            elif re.match(r'""|\d|true\b|false\b|\' \'|-\d', core.strip()):
                pats["literal"] += 1
            elif re.match(r"(?:" + IDENT + r"\.)*[A-Z]\w*$", core.strip()) or core.strip().startswith("`"):
                pats["stable"] += 1
            elif re.match(r"[a-z_]\w*$", core.strip()):
                pats["variable"] += 1
            elif "::" in core:
                pats["cons"] += 1
            else:
                pats["other"] += 1
            if re.search(r"\*\s*\)", core):
                pats["sequence"] += 1
    s["patterns"] = pats
    s["partial_function_literals"] = len(re.findall(r"(?:\{|:)\s*\n?\s*case\b", text)) - len(re.findall(r"\bmatch\s*(?:\{|:)?\s*\n?\s*case\b", text))

    fors = []
    lines_t = text.split("\n")
    for m in FOR_START.finditer(text):
        line_no = text.count("\n", 0, m.start())
        gens = 0
        kind = "do"
        j = line_no
        head = lines_t[line_no][m.start() - (len(text[: m.start()]) - len(text[: m.start()].rsplit("\n", 1)[-1])) :]
        if "<-" in head:
            gens += head.count("<-")
        while j < len(lines_t) and j < line_no + 200:
            l = lines_t[j]
            if j > line_no:
                if "<-" in l:
                    gens += 1
                if re.search(r"\byield\b", l):
                    kind = "yield"
                    break
                if re.search(r"(?<![A-Za-z0-9_.])do\b", l) or re.search(r"^\s*\}\s*\{", l):
                    kind = "do"
                    break
                if l.strip() == "" and gens > 0 and j + 1 < len(lines_t) and not re.search(r"<-|=|yield|\bdo\b", lines_t[j + 1]):
                    break
            else:
                if re.search(r"\byield\b", l):
                    kind = "yield"
                    break
            j += 1
        if gens > 0:
            fors.append({"gens": gens, "kind": kind})
    s["fors"] = fors
    lambdas = len(LAMBDA.findall(text)) - sum(len(g) for g in groups)
    s["lambdas"] = max(lambdas, 0)
    s["placeholder_lambdas"] = len(PLACEHOLDER.findall(text))
    s["anonymous_classes"] = len(NEW_ANON.findall(text))
    s["unions"] = len(UNION.findall(text))
    s["vals"] = len(VAL.findall(text))
    s["lazy_vals"] = len(LAZY_VAL.findall(text))
    s["vars"] = len(VAR.findall(text))
    s["defs_count"] = len(DEF.findall(text))
    s["top_level_defs"] = len(TOP_DEF.findall(text))
    s["by_name_params"] = len(BY_NAME.findall(text))
    s["default_args"] = len(DEFAULT_ARG.findall(text))
    s["varargs"] = len(VARARG.findall(text))
    s["higher_kinded"] = len(HIGHER_KINDED.findall(text))
    s["try"] = len(TRY.findall(text))
    s["throw"] = len(THROW.findall(text))
    s["return"] = len(RETURN.findall(text))
    s["tuple_vals"] = len(TUPLE_VAL.findall(text))
    s["ifs"] = len(IF.findall(text))
    s["whiles"] = len(WHILE.findall(text))
    s["named_args"] = len(NAMED_ARG.findall(text))
    s["assign_ops"] = len(ASSIGN_OP.findall(text))
    annotations = Counter(m.group(1).split(".")[-1] for m in ANNOTATION.finditer(text))
    s["annotations"] = sum(annotations.values())
    s["annotation_kinds"] = len(annotations)
    s["text"] = text
    return s


def macro_rows(macros, texts):
    """One row per macro definition: its kind and the number of call sites over the texts."""
    rows = []
    for mc in macros:
        if mc["interpolator"]:
            pat = re.compile(r"(?<![A-Za-z0-9_])" + re.escape(mc["name"]) + r'"')
            kind = "interpolator"
        elif mc["derived"]:
            pat = re.compile(r"\bderives\b[^\n]*\b" + re.escape(mc["owner"] or "\x00") + r"\b|\b" + re.escape(mc["owner"] or "\x00") + r"\.derived\b")
            kind = "derivation"
        elif re.match(r"[!#%&*+\-/:<=>?@\\^|~]+$", mc["name"]):
            pat = re.compile(r"\b" + re.escape(mc["owner"] or "\x00") + r"\s*\.?\s*" + re.escape(mc["name"]) + r"(?![!#%&*+\-/:<=>?@\\^|~])")
            kind = "operator"
        else:
            pat = re.compile(r"(?<![A-Za-z0-9_.])" + re.escape(mc["name"]) + r"\s*[\[(]")
            kind = "call"
        sites = sum(len(pat.findall(t)) for t in texts)
        rows.append({"kind": kind, "sites": sites, "inline_params": len(re.findall(r"\binline\s+" + IDENT, mc["sig"]))})
    rows.sort(key=lambda r: (-r["sites"], r["kind"]))
    return rows


def survey_module(root, anonymous, all_macros=None):
    files = list(scala_files(root))
    per_file = [analyse_file(f, anonymous) for f in files]
    m = {"files": len(files)}
    m["lines"] = sum(f["lines"] for f in per_file)
    m["blank_lines"] = sum(f["blank"] for f in per_file)
    m["comment_lines"] = sum(f["comment_lines"] for f in per_file)
    m["file_lines"] = histogram([f["lines"] for f in per_file], LINE_BUCKETS)
    defs = [d for f in per_file for d in f["defs"]]
    case_classes = [d for d in defs if d["kind"] == "class" and d["case"]]
    m["case_classes"] = {
        "count": len(case_classes),
        "fields": histogram([d["fields"] for d in case_classes], FIELD_BUCKETS),
        "fields_total": sum(d["fields"] for d in case_classes),
        "generic": sum(1 for d in case_classes if d["tparams"]),
        "with_parent": sum(1 for d in case_classes if d["parents"]),
        "with_derives": sum(1 for d in case_classes if d["derives"]),
        "nested": sum(1 for d in case_classes if d["indent"] > 0),
    }
    m["case_objects"] = sum(1 for d in defs if d["kind"] == "object" and d["case"])
    enums = [d for d in defs if d["kind"] == "enum"]
    m["enums"] = {
        "count": len(enums),
        "cases": histogram([d["cases"] for d in enums], CASE_BUCKETS),
        "cases_total": sum(d["cases"] for d in enums),
        "with_class_cases": sum(1 for d in enums if d["class_cases"]),
        "class_cases_total": sum(d["class_cases"] for d in enums),
        "with_params": sum(1 for d in enums if d["params"]),
        "with_extends_cases": sum(1 for d in enums if d["extends_cases"]),
        "generic": sum(1 for d in enums if d["tparams"]),
        "with_derives": sum(1 for d in enums if d["derives"]),
        "with_parent": sum(1 for d in enums if d["parents"]),
    }
    sealed = [d for d in defs if d["sealed"]]
    names = {d["name"]: d for d in defs}
    children = defaultdict(list)
    for d in defs:
        for p in d["parents"]:
            if p in names:
                children[p].append(d["name"])

    def depth(name, seen=()):
        if name in seen:
            return 0
        return 1 + max([depth(c, seen + (name,)) for c in children.get(name, [])], default=0)

    m["sealed"] = {
        "count": len(sealed),
        "traits": sum(1 for d in sealed if d["kind"] == "trait"),
        "children": histogram([len(children.get(d["name"], [])) for d in sealed], CASE_BUCKETS),
        "depth": histogram([depth(d["name"]) for d in sealed], SMALL_BUCKETS),
    }
    m["classes"] = {
        "plain": sum(1 for d in defs if d["kind"] == "class" and not d["case"]),
        "abstract": sum(1 for d in defs if d["kind"] == "class" and d["abstract"]),
        "traits": sum(1 for d in defs if d["kind"] == "trait"),
        "objects": sum(1 for d in defs if d["kind"] == "object" and not d["case"]),
        "companions": sum(1 for d in defs if d["kind"] == "object" and any(o["name"] == d["name"] and o["kind"] != "object" for o in defs)),
        "generic_traits": sum(1 for d in defs if d["kind"] == "trait" and d["tparams"]),
        "higher_kinded_params": sum(f["higher_kinded"] for f in per_file),
        "anonymous": sum(f["anonymous_classes"] for f in per_file),
        "nested": sum(1 for d in defs if d["indent"] > 0),
    }
    program_parents = [p for d in defs for p in d["parents"] if p in names]
    external_parents = [p for d in defs for p in d["parents"] if p not in names]
    m["inheritance"] = {
        "with_parent": sum(1 for d in defs if d["parents"]),
        "depth": histogram([depth(d["name"]) for d in defs if children.get(d["name"])], SMALL_BUCKETS),
        "program_parents": len(program_parents),
        "external_parents": len(external_parents),
        "external_parent_kinds": len(set(external_parents)),
    }
    derive_sets = Counter(",".join(sorted(d["derives"])) for d in defs if d["derives"])
    derive_totals = Counter(t for d in defs for t in d["derives"])
    m["derives"] = {
        "clauses": sum(derive_sets.values()),
        "by_set": [[k.count(",") + 1, v] for k, v in sorted(derive_sets.items(), key=lambda kv: (-kv[1], kv[0]))],
        "by_type_class": sorted(derive_totals.values(), reverse=True),
        "type_classes": len(derive_totals),
        "on_generic": sum(1 for d in defs if d["derives"] and d["tparams"]),
        "_names": {"sets": dict(sorted(derive_sets.items(), key=lambda kv: (-kv[1], kv[0]))), "type_classes": dict(sorted(derive_totals.items(), key=lambda kv: (-kv[1], kv[0])))},
    }
    givens = [g for f in per_file for g in f["givens"]]
    m["givens"] = {
        "count": len(givens),
        "alias": sum(1 for g in givens if g["kind"] == "alias"),
        "instance": sum(1 for g in givens if g["kind"] == "instance"),
        "conversion": sum(1 for g in givens if g["kind"] == "conversion"),
        "parameterised": sum(1 for g in givens if g["parameterised"]),
        "with_using": sum(1 for g in givens if g["using"]),
        "named": sum(1 for g in givens if g["named"]),
        "implicit_val": sum(f["implicits"].get("val", 0) + f["implicits"].get("lazy val", 0) for f in per_file),
        "implicit_def": sum(f["implicits"].get("def", 0) for f in per_file),
        "implicit_object": sum(f["implicits"].get("object", 0) for f in per_file),
        "implicit_class": sum(f["implicits"].get("class", 0) for f in per_file),
        "implicit_params": sum(f["implicit_params"] for f in per_file),
        "using_clauses": sum(f["using_clauses"] for f in per_file),
        "context_bounds": sum(f["context_bounds"] for f in per_file),
        "summons": sum(f["summons"] for f in per_file),
        "given_imports": sum(f["given_imports"] for f in per_file),
    }
    exts = [e for f in per_file for e in f["extensions"]]
    m["extensions"] = {
        "clauses": len(exts),
        "methods": sum(e["methods"] for e in exts),
        "generic": sum(1 for e in exts if e["generic"]),
        "context_bound": sum(1 for e in exts if e["context_bound"]),
    }
    m["types"] = {
        "opaque": sum(f["opaque_types"] for f in per_file),
        "aliases": sum(f["type_aliases"] for f in per_file),
        "unions": sum(f["unions"] for f in per_file),
    }
    m["inline"] = {
        "defs": sum(f["inline_defs"] for f in per_file),
        "transparent": sum(f["transparent_inline_defs"] for f in per_file),
        "params": sum(f["inline_params"] for f in per_file),
        "givens": sum(f["inline_givens"] for f in per_file),
    }
    macros = [mc for f in per_file for mc in f["macros"]]
    texts = [f["text"] for f in per_file]
    rows = macro_rows(all_macros if all_macros is not None else macros, texts)
    m["macros"] = {"definitions": len(macros), "sites": sum(r["sites"] for r in rows), "rows": rows, "files_with_quoted": sum(f["quoted_files"] for f in per_file)}
    m["_texts"] = texts
    m["_macros"] = macros
    groups = [g for f in per_file for g in f["case_groups"]]
    pats = Counter()
    for f in per_file:
        pats.update(f["patterns"])
    m["matches"] = {
        "match_keywords": sum(f["match_keywords"] for f in per_file),
        "case_groups": len(groups),
        "cases": sum(groups),
        "cases_per_group": histogram(groups, CASE_BUCKETS),
        "partial_function_literals": max(sum(f["partial_function_literals"] for f in per_file), 0),
        "patterns": dict(sorted(pats.items(), key=lambda kv: (-kv[1], kv[0]))),
    }
    fors = [fr for f in per_file for fr in f["fors"]]
    m["fors"] = {
        "count": len(fors),
        "yield": sum(1 for fr in fors if fr["kind"] == "yield"),
        "generators": histogram([fr["gens"] for fr in fors], SMALL_BUCKETS),
        "generators_total": sum(fr["gens"] for fr in fors),
    }
    m["lambdas"] = {
        "arrows": sum(f["lambdas"] for f in per_file),
        "placeholders": sum(f["placeholder_lambdas"] for f in per_file),
    }
    m["imports"] = {
        "per_file": histogram([f["imports"] for f in per_file], IMPORT_BUCKETS),
        "total": sum(f["imports"] for f in per_file),
        "wildcard": sum(f["wildcard_imports"] for f in per_file),
        "local": sum(f["local_imports"] for f in per_file),
    }
    m["package_depth"] = histogram([f["package_depth"] for f in per_file], SMALL_BUCKETS)
    m["strings"] = {
        "s": sum(f["interpolations"]["s"] for f in per_file),
        "f": sum(f["interpolations"]["f"] for f in per_file),
        "raw": sum(f["interpolations"]["raw"] for f in per_file),
        "custom": sum(f["interpolations"]["custom"] for f in per_file),
        "triple_quoted": sum(f["triple_quoted"] for f in per_file),
    }
    m["members"] = {
        "vals": sum(f["vals"] for f in per_file),
        "lazy_vals": sum(f["lazy_vals"] for f in per_file),
        "vars": sum(f["vars"] for f in per_file),
        "defs": sum(f["defs_count"] for f in per_file),
        "top_level_defs": sum(f["top_level_defs"] for f in per_file),
        "by_name_params": sum(f["by_name_params"] for f in per_file),
        "default_args": sum(f["default_args"] for f in per_file),
        "varargs": sum(f["varargs"] for f in per_file),
        "named_args": sum(f["named_args"] for f in per_file),
        "tuple_vals": sum(f["tuple_vals"] for f in per_file),
    }
    m["control"] = {
        "ifs": sum(f["ifs"] for f in per_file),
        "whiles": sum(f["whiles"] for f in per_file),
        "try": sum(f["try"] for f in per_file),
        "throw": sum(f["throw"] for f in per_file),
        "return": sum(f["return"] for f in per_file),
        "assign_ops": sum(f["assign_ops"] for f in per_file),
    }
    m["annotations"] = {"sites": sum(f["annotations"] for f in per_file), "kinds": max((f["annotation_kinds"] for f in per_file), default=0)}
    return m


def merge(mods):
    """Totals over modules: sums of counts, merged histograms."""

    def add(a, b):
        if isinstance(a, dict) and isinstance(b, dict):
            out = dict(a)
            for k, v in b.items():
                out[k] = add(a[k], v) if k in a else v
            return out
        if isinstance(a, list) and isinstance(b, list):
            if a and isinstance(a[0], dict):
                return sorted(a + b, key=lambda r: -r.get("sites", 0))
            if a and isinstance(a[0], list):
                return sorted(a + b, key=lambda r: (-r[1], -r[0]))
            return sorted(a + b, reverse=True)
        if isinstance(a, (int, float)) and isinstance(b, (int, float)):
            return a + b
        return b

    total = {}
    for m in mods.values():
        total = add(total, m)
    return total


def main():
    args = sys.argv[1:]
    out = None
    anonymous = False
    show_names = False
    labels = {}
    roots = []
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--json":
            out = args[i + 1]
            i += 2
            continue
        if a.startswith("--json="):
            out = a.split("=", 1)[1]
        elif a == "--names":
            show_names = True
        elif a.startswith("--name="):
            root, label = a.split("=", 1)[1].split(":", 1)
            labels[root] = label
        else:
            roots.append(a)
        i += 1
    if not roots:
        print(__doc__)
        sys.exit(2)
    modules = {}
    for root in roots:
        label = labels.get(root, os.path.basename(os.path.normpath(root)))
        modules[label] = survey_module(root, anonymous)
    all_macros = [mc for m in modules.values() for mc in m["_macros"]]
    all_texts = [t for m in modules.values() for t in m["_texts"]]
    names = {}
    for label, m in modules.items():
        rows = macro_rows(all_macros, m["_texts"])
        m["macros"]["rows"] = rows
        m["macros"]["sites"] = sum(r["sites"] for r in rows)
        names[label] = m["derives"].pop("_names")
        del m["_texts"]
        del m["_macros"]
    total = merge(modules)
    total["macros"]["rows"] = macro_rows(all_macros, all_texts)
    total["macros"]["sites"] = sum(r["sites"] for r in total["macros"]["rows"])
    report = {"modules": modules, "total": total}
    if "_names" in total.get("derives", {}):
        del total["derives"]["_names"]
    if show_names:
        for label, n in names.items():
            print(f"{label}: derives by type class {n['type_classes']}")
            print(f"{label}: derives by set {n['sets']}")
    if out:
        with open(out, "w") as f:
            json.dump(report, f, indent=1, sort_keys=False)
            f.write("\n")
    t = report["total"]
    print(f"{'module':28} {'files':>6} {'lines':>7} {'case cl':>7} {'enums':>6} {'derives':>7} {'givens':>6} {'macros':>7} {'matches':>7} {'fors':>5}")
    for name, m in list(modules.items()) + [("total", t)]:
        print(f"{name:28} {m['files']:6} {m['lines']:7} {m['case_classes']['count']:7} {m['enums']['count']:6} {m['derives']['clauses']:7} {m['givens']['count']:6} {m['macros']['sites']:7} {m['matches']['case_groups']:7} {m['fors']['count']:5}")


if __name__ == "__main__":
    main()
