# defs.py <std dir> <declared.json>: the cut's enumeration (run.py beside this file): every
# static method (a member of an object, nested objects included, or a top-level definition), every member of a class or
# a trait and every constructor of the lean std's own source that has a using or an implicit clause (a context bound is one), read from the source,
# never chosen. Prints one JSON line per definition: its file, the path of the object or class, the name, the type
# parameters and the clauses as written (each with its kind: plain, using, implicit; a context bound's evidence
# clause added last, as the desugaring adds it), and why a definition the generator cannot call is left (private,
# protected, inline, an extension, a class that is abstract, sealed or a trait).
import json, os, re, sys
STD = sys.argv[1]
files = sorted(os.path.join(d, f) for d, _, fs in os.walk(STD) for f in fs if f.endswith(".scala") and "/fix" not in d)

def split_clauses(sig):
    """The bracket and paren groups of a signature in order: ('[', text) or ('(', text)."""
    out, i, n = [], 0, len(sig)
    while i < n:
        c = sig[i]
        if c in "([":
            close = ")" if c == "(" else "]"
            depth, j = 0, i
            while j < n:
                if sig[j] in "([": depth += 1
                elif sig[j] in ")]":
                    depth -= 1
                    if depth == 0: break
                j += 1
            out.append((c, sig[i + 1:j])); i = j + 1
        elif c in ":=" and not out:
            i += 1
        elif c == ":" or c == "=":
            break
        else:
            i += 1
    return out

def params_of(text):
    """Top-level comma split of a clause's text."""
    out, depth, cur = [], 0, ""
    for ch in text:
        if ch in "([{": depth += 1
        elif ch in ")]}": depth -= 1
        if ch == "," and depth == 0: out.append(cur.strip()); cur = ""
        else: cur += ch
    if cur.strip(): out.append(cur.strip())
    return out

def clause_kinds(groups):
    tparams, clauses, bounds = [], [], []
    for kind, text in groups:
        if kind == "[" and not clauses:
            for tp in params_of(text):
                m = re.match(r"([+-]?\w+)", tp)
                tparams.append(tp)
                for b in re.findall(r":\s*([A-Za-z_][\w.]*)", re.sub(r"<:[^,:]*|>:[^,:]*", "", tp)):
                    bounds.append(f"{b}[{m.group(1).lstrip('+-')}]")
        elif kind == "(":
            t = text.strip()
            k = "using" if t.startswith("using ") else "implicit" if t.startswith("implicit ") else "plain"
            body = re.sub(r"^(using|implicit)\s+", "", t)
            clauses.append({"kind": k, "params": params_of(body)})
    if bounds:
        clauses.append({"kind": "using", "params": [f"evidence${i + 1}: {b}" for i, b in enumerate(bounds)], "bound": True})
    return tparams, clauses

# the types each object declares (classes, traits, objects, enums, type aliases), by the object's path: the generator
# names a parameter's type through the object that declares it
declared = {}
def owner_path(pkg, stack, kinds=("object", "class")):
    return ".".join(x for x in [pkg] + [s[2] for s in stack if s[1] in kinds] if x)
for path in files:
    rel = os.path.relpath(path, os.path.dirname(STD.rstrip("/")))
    lines = open(path, encoding="utf-8").read().split("\n")
    stack = []  # (indent, kind, name, modifiers)
    pkg = ""
    for ln, line in enumerate(lines, 1):
        if not line.strip() or line.strip().startswith("//"): continue
        ind = len(line) - len(line.lstrip(" "))
        s = line.strip()
        while stack and stack[-1][0] >= ind and not s.startswith(("end ",)):
            stack.pop()
        m = re.match(r"package\s+([\w.]+)\s*$", s)
        if m and ind == 0: pkg = (pkg + "." if pkg else "") + m.group(1); continue
        mods = re.match(r"((?:(?:private(?:\[\w+\])?|protected(?:\[\w+\])?|final|sealed|abstract|case|implicit|inline|transparent|open|override|lazy|opaque)\s+)*)", s).group(1)
        rest = s[len(mods):]
        mo = re.match(r"(object|class|trait|enum|extension|given)\b\s*([\w$]*)(.*)", rest)
        if mo:
            kind, name, tail = mo.groups()
            # a given's body (`given X: T with`) is an instance's, not a static scope; a given alias opens none
            if kind == "given" and not rest.rstrip().endswith("with"):
                continue
            if kind == "class" and not re.search(r"\b(abstract|sealed|private)\b", mods):
                groups = split_clauses(tail.split(" extends ")[0])
                ctor_private = bool(re.match(r"\s*(\[[^\]]*\])?\s*private\b", tail))
                tparams, clauses = clause_kinds(groups)
                if any(c["kind"] != "plain" for c in clauses):
                    owner = owner_path(pkg, stack)
                    print(json.dumps({"what": "ctor", "file": rel, "line": ln, "owner": owner, "name": name, "tparams": tparams,
                                      "clauses": clauses, "left": "a private constructor" if ctor_private else None}))
            elif kind == "class":
                tparams, clauses = clause_kinds(split_clauses(tail.split(" extends ")[0]))
                if any(c["kind"] != "plain" for c in clauses):
                    owner = owner_path(pkg, stack)
                    print(json.dumps({"what": "ctor", "file": rel, "line": ln, "owner": owner, "name": name, "tparams": tparams,
                                      "clauses": clauses, "left": "an abstract, sealed or private class"}))
            if stack and stack[-1][1] == "object" and kind in ("object", "class", "trait", "enum"):
                declared.setdefault(owner_path(pkg, stack, ("object",)), set()).add(name)
            stack.append((ind, kind, name, mods))
            continue
        mt = re.match(r"type\s+(\w+)", rest)
        if mt and stack and stack[-1][1] == "object":
            declared.setdefault(owner_path(pkg, stack, ("object",)), set()).add(mt.group(1))
        md = re.match(r"def\s+([^\s\[\(:=]+)(.*)", rest)
        if not md: continue
        name, tail = md.groups()
        statics = all(x[1] == "object" for x in stack)  # top-level (empty stack) or objects alone
        # a member of a class or a trait (the innermost scope one, every scope outside it an object or the top level)
        member = bool(stack) and stack[-1][1] in ("class", "trait") and all(x[1] == "object" for x in stack[:-1])
        in_extension = bool(stack) and stack[-1][1] == "extension" and all(x[1] == "object" for x in stack[:-1])
        if not (statics or member or in_extension): continue
        tparams, clauses = clause_kinds(split_clauses(tail))
        if not any(c["kind"] != "plain" for c in clauses): continue
        left = None
        if re.search(r"\b(private|protected)\b", mods): left = "private or protected"
        elif re.search(r"\binline\b", mods): left = "inline: its calls are expanded, no call is kept"
        elif in_extension: left = "an extension method: its receiver's syntax is not generated"
        what = "member" if member else "static"
        owner = owner_path(pkg, stack, ("object", "class", "trait")) if member else owner_path(pkg, stack, ("object",))
        print(json.dumps({"what": what, "file": rel, "line": ln, "owner": owner, "name": name, "tparams": tparams, "clauses": clauses, "left": left}))

json.dump({k: sorted(v) for k, v in declared.items()}, open(sys.argv[2], "w"))
