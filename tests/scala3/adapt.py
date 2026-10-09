#!/usr/bin/env python3
"""Adapts the single-file tests of tests/run that fail to compile (per results.json of the last
run.py run) by replacing incidental constructs: `extends App` becomes an object with an empty
main, Scala 2 control syntax becomes the Scala 3 spelling, `xs map { .. }` becomes `xs.map { .. }`,
language imports are dropped. Runs the adapted files through teq and prints the ones that pass;
they are candidates for tests/cases/scala3_<name>.scala (see README.md)."""
import os, re, subprocess, sys, json, concurrent.futures

ROOT = os.path.join(os.environ.get("SCALA3", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "teq-ref", "scala3")), "tests", "run")
HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
TEQ = os.environ.get("TEQ", os.path.join(REPO, "target", "release", "teq"))
OUT = os.path.join(REPO, "out", "scala3", "adapted")
os.makedirs(OUT, exist_ok=True)

results = json.load(open(os.path.join(HERE, "results.json")))
status = {r["name"]: r for r in results}


def match_paren(s, i):
    """Index of the ')' matching the '(' at s[i], or -1."""
    depth = 0
    j = i
    in_str = False
    while j < len(s):
        c = s[j]
        if in_str:
            if c == "\\":
                j += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return j
        j += 1
    return -1


def control_syntax(s, changed):
    """`if (c) e` -> `if c then e`, `while (c) e` -> `while c do e`, `for (g) e` -> `for g do e`."""
    out = []
    i = 0
    while i < len(s):
        m = re.compile(r"\b(if|while|for)\s*\(").match(s, i)
        if m and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == "_")) and not in_string_or_comment(s, i):
            kw = m.group(1)
            open_i = m.end() - 1
            close_i = match_paren(s, open_i)
            if close_i > 0:
                cond = s[open_i + 1:close_i].strip()
                rest = s[close_i + 1:]
                if kw == "if":
                    # a condition already followed by `then` is Scala 3 already
                    if re.match(r"\s*then\b", rest):
                        out.append(s[i:close_i + 1]); i = close_i + 1; continue
                    out.append(f"if {cond} then")
                elif kw == "while":
                    if re.match(r"\s*do\b", rest):
                        out.append(s[i:close_i + 1]); i = close_i + 1; continue
                    out.append(f"while {cond} do")
                else:
                    if re.match(r"\s*(yield|do)\b", rest):
                        out.append(f"for {cond}")
                    else:
                        out.append(f"for {cond} do")
                changed.append("control-" + kw)
                i = close_i + 1
                continue
        out.append(s[i])
        i += 1
    return "".join(out)


KEYWORDS = "match|else|yield|do|then|try|catch|finally|new|extends|with|for|if|while|return|throw|case|def|val|var|given|using|derives|object|class|trait|enum|type|import|export|lazy|override|final|implicit|inline|private|protected|sealed|abstract"


def infix_block(s, changed):
    """`xs map { x => .. }` -> `xs.map { x => .. }` for an alphanumeric method with a block argument."""
    rx = re.compile(r"(\w|\))[ \t]+(?!(?:" + KEYWORDS + r")\b)([a-z]\w*)[ \t]*\{(?=[ \t]*(?:case\b|\(?[\w, ()]*\)?[ \t]*=>|\n))")
    def repl(m):
        changed.append("infix-block")
        return f"{m.group(1)}.{m.group(2)} {{"
    return rx.sub(repl, s)


def in_string_or_comment(s, i):
    line_start = s.rfind("\n", 0, i) + 1
    prefix = s[line_start:i]
    if "//" in prefix:
        return True
    return prefix.count('"') % 2 == 1


def adapt(text):
    changed = []
    text = control_syntax(text, changed)
    text = infix_block(text, changed)
    lines = text.split("\n")
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        m = re.match(r"^(\s*)object\s+(\w+)\s+extends\s+App\s*(\{|:)\s*$", line)
        if m:
            indent, name, opener = m.groups()
            # find the body indentation from the next non-blank line
            j = i + 1
            while j < len(lines) and lines[j].strip() == "":
                j += 1
            body_indent = re.match(r"^(\s*)", lines[j]).group(1) if j < len(lines) else indent + "  "
            if len(body_indent) <= len(indent):
                body_indent = indent + "  "
            out.append(f"{indent}object {name} {opener}")
            out.append(f"{body_indent}def main(args: Array[String]): Unit = ()")
            changed.append("App")
            i += 1
            continue
        if re.match(r"^\s*import\s+(scala\.)?language\.[\w.`]+\s*$", line):
            changed.append("language-import")
            i += 1
            continue
        if re.match(r"^\s*//> using options", line):
            i += 1
            continue
        out.append(line)
        i += 1
    return "\n".join(out), changed


def run_one(name):
    src = os.path.join(ROOT, name + ".scala")
    text = open(src, errors="replace").read()
    adapted, changed = adapt(text)
    if not changed:
        return None
    path = os.path.join(OUT, name + ".scala")
    open(path, "w").write(adapted)
    js = os.path.join(OUT, name + ".js")
    try:
        b = subprocess.run([TEQ, "compiler", "build", path, "-o", js], capture_output=True, text=True, timeout=20)
    except subprocess.TimeoutExpired:
        return (name, changed, "compile-timeout", "")
    if b.returncode != 0:
        err = next((l.split(": error: ", 1)[1] for l in b.stderr.splitlines() if ": error: " in l), b.stderr[-200:])
        return (name, changed, "compile-error", err)
    try:
        r = subprocess.run(["node", js], capture_output=True, text=True, timeout=20)
    except subprocess.TimeoutExpired:
        return (name, changed, "run-timeout", "")
    if r.returncode != 0:
        return (name, changed, "run-error", (r.stderr or "").strip().splitlines()[-1][:200] if r.stderr.strip() else "")
    check = os.path.join(ROOT, name + ".check")
    if os.path.exists(check):
        exp = open(check, errors="replace").read()
        norm = lambda s: "\n".join(l.rstrip() for l in s.strip().splitlines())
        if norm(exp) == norm(r.stdout):
            return (name, changed, "pass", "")
        return (name, changed, "wrong-output", "")
    return (name, changed, "pass-nocheck", "")


names = sorted(f[:-6] for f in os.listdir(ROOT) if f.endswith(".scala"))
names = [n for n in names if status.get(n, {}).get("status") == "compile-error"]
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as ex:
    res = [r for r in ex.map(run_one, names) if r]
import collections
print(len(res), "adapted;", dict(collections.Counter(r[2] for r in res)))
json.dump(res, open(os.path.join(REPO, "out", "scala3", "adapted.json"), "w"), indent=1)
for r in res:
    if r[2] in ("pass", "pass-nocheck", "wrong-output", "run-error"):
        print(r[2], r[0], "|", r[3][:100])
