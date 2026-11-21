# gen.py <defs.jsonl> <declared.json> <out dir>: the cut's programs (run.py beside this file). For every definition
# defs.py lists (every static and every constructor of the lean std's own source with a using or implicit clause),
# a library A whose kept body calls it, crossed with the part's list forms of its first using or implicit clause: the
# whole list written (`using summon[P]..`), named (`using p = summon[P]..`, not for a context bound's evidence),
# short (the first argument written, the rest searched, where the clause has two or more), none (the clause searched
# whole); and a program B printing Lib.value. Type parameters are Int; a plain parameter's argument is made by its
# type; a definition the table cannot make an argument for, or of a higher-kinded type parameter, is left, with
# why (left.txt), never dropped silently.
import json, os, re, shutil, sys
ENUM, DECLARED, D = sys.argv[1:4]; P = f"{D}/progs"; R = f"{D}/rows"
for d in (P, R):
    if os.path.isdir(d): shutil.rmtree(d)
    os.makedirs(d)
rows = [json.loads(l) for l in open(ENUM)]
declared = json.load(open(DECLARED))

def qualify(t, owner):
    # a type the owner object (or an object it is nested in) declares, named through that object
    parts = owner.split(".")
    for k in range(len(parts), 0, -1):
        o = ".".join(parts[:k])
        for name in declared.get(o, []):
            t = re.sub(rf"(?<![\w.]){re.escape(name)}\b", f"{o}.{name}", t)
    return t

def value(t):
    t = re.sub(r"\?\s*<:\s*", "", t.strip())
    t = re.sub(r"(?<![\w.])\?(?![\w])", "Int", t)
    if t.startswith("=>"): return value(t[2:])
    if t.endswith("*"): v = value(t[:-1]); return None if v is None else f"{v}, {v}"
    simple = {"Int": "1", "Long": "1L", "Short": "1.toShort", "Byte": "1.toByte", "Double": "1.0", "Float": "1.0f", "Char": "'c'",
              "Boolean": "true", "String": '"1"', "Any": "1", "AnyRef": '"1"', "Unit": "()", "Nothing": None}
    if t in simple: return simple[t]
    m = re.fullmatch(r"(Array|IArray)\[(.+)\]", t)
    if m: v = value(m.group(2)); return None if v is None else f"{m.group(1)}({v}, {v})"
    m = re.fullmatch(r"(?:scala\.collection\.(?:immutable\.)?)?(IterableOnce|Iterable|Seq|List|IndexedSeq|Vector)\[(.+)\]", t)
    if m: v = value(m.group(2)); return None if v is None else f"List({v}, {v})"
    m = re.fullmatch(r"\(([^()=]+)\)", t)
    if m and "," in m.group(1):
        vs = [value(x) for x in m.group(1).split(",")]
        return None if None in vs else "(" + ", ".join(vs) + ")"
    m = re.fullmatch(r"Set\[(.+)\]", t)
    if m: v = value(m.group(1)); return None if v is None else f"Set({v})"
    m = re.fullmatch(r"Map\[([^,]+),(.+)\]", t)
    if m:
        k, v = value(m.group(1)), value(m.group(2))
        return None if None in (k, v) else f"Map({k} -> {v})"
    m = re.fullmatch(r"Option\[(.+)\]", t)
    if m: v = value(m.group(1)); return None if v is None else f"Some({v})"
    m = re.fullmatch(r"\((.+)\)\s*=>\s*(.+)", t) or re.fullmatch(r"([\w.\[\]]+)\s*=>\s*(.+)", t)
    if m:
        ins = [x.strip() for x in m.group(1).split(",")] if "," in m.group(1) else [m.group(1).strip()]
        out = value(m.group(2))
        if out is None or any(i not in simple or simple[i] is None for i in ins): return None
        return "(" + ", ".join(f"x{k}: {i}" for k, i in enumerate(ins)) + f") => {out}"
    return None

def subst(t, tps, owner=""):
    for tp in tps: t = re.sub(rf"\b{re.escape(tp)}\b", "Int", t)
    return qualify(t, owner) if owner else t

# a receiver for a member of a class or a trait, by the class's simple name (its type parameters Int)
RECEIVERS = {
    **{c: "List(1, 2)" for c in ("List", "Seq", "Iterable", "IterableOnce", "IterableOps", "IterableOnceOps", "SeqOps", "LinearSeq",
                                 "LinearSeqOps", "StrictOptimizedSeqOps", "StrictOptimizedIterableOps", "IndexedSeqOps", "collection.Seq")},
    "Vector": "Vector(1, 2)", "IndexedSeq": "Vector(1, 2)", "Set": "Set(1, 2)", "SetOps": "Set(1, 2)", "Map": "Map(1 -> 1)",
    "MapOps": "Map(1 -> 1)", "Option": "Some(1)", "Array": "Array(1, 2)", "ArrayOps": "Array(1, 2)", "IArray": "IArray(1, 2)",
    "String": '"12"', "StringOps": '"12"', "Range": "(1 to 3)", "LazyList": "LazyList(1, 2)", "Iterator": "Iterator(1, 2)",
    "ArraySeq": "scala.collection.immutable.ArraySeq(1, 2)", "ArrayBuffer": "scala.collection.mutable.ArrayBuffer(1, 2)",
    "ListBuffer": "scala.collection.mutable.ListBuffer(1, 2)",
}
names, left = [], []
for r in rows:
    if r["left"]: left.append((r, r["left"])); continue
    tps = [re.match(r"[+-]?(\w+)", tp).group(1) for tp in r["tparams"]]
    if any(re.search(r"\w\s*\[", tp) for tp in r["tparams"]): left.append((r, "a higher-kinded type parameter")); continue
    first = next(i for i, c in enumerate(r["clauses"]) if c["kind"] != "plain")
    plain_lists, fail = [], None
    for c in r["clauses"][:first]:
        vals = []
        for prm in c["params"]:
            # a default (` = ..`) is cut off, a by-name type's arrow (`=> T`) kept
            ty = subst(re.split(r"\s=(?!>)\s", prm.split(":", 1)[1])[0], tps, r["owner"]) if ":" in prm else None
            v = value(ty) if ty else None
            if v is None: fail = f"no argument for {prm}"; break
            vals.append(v)
        if fail: break
        plain_lists.append("(" + ", ".join(vals) + ")")
    if fail: left.append((r, fail)); continue
    uc = r["clauses"][first]
    # a using parameter without a name (`using Ordering[T]`) has no named form
    ups = [(prm.split(":", 1)[0].strip(), subst(prm.split(":", 1)[1], tps, r["owner"]).strip()) if ":" in prm else (None, subst(prm, tps, r["owner"]).strip()) for prm in uc["params"]]
    targs = f"[{', '.join('Int' for _ in tps)}]" if tps else ""
    if r["what"] == "member":
        recv = RECEIVERS.get(r["owner"].split(".")[-1])
        if recv is None:
            left.append((r, f"a member of {r['owner'].split('.')[-1]}, which the receiver table has no value for")); continue
        callee = f"({recv}).{r['name']}"
    else:
        callee = f"new {r['owner']}.{r['name']}" if r["what"] == "ctor" else f"{r['owner']}.{r['name']}"
    forms = {"whole": "(using " + ", ".join(f"summon[{t}]" for _, t in ups) + ")", "none": ""}
    if not uc.get("bound") and all(nm for nm, _ in ups): forms["named"] = "(using " + ", ".join(f"{n} = summon[{t}]" for n, t in ups) + ")"
    if len(ups) >= 2: forms["short"] = f"(using summon[{ups[0][1]}])"
    # the list written with values that are not givens (local vals): what master's reading takes for a plain list
    forms["local"] = "(using " + ", ".join(f"v{i}" for i in range(len(ups))) + ")"
    for fk, fl in forms.items():
        call = f"{callee}{targs}{''.join(plain_lists)}{fl}"
        if r["what"] == "ctor" and not plain_lists and not fl: call += "()"
        n = re.sub(r"\W", "_", f"cut_{r['what']}_{r['owner']}_{r['name']}_{r['line']}_{fk}")
        locals_ = "".join(f"    val v{i}: {t} = summon[{t}]\n" for i, (_, t) in enumerate(ups)) if fk == "local" else ""
        a = f"package g_{n}\nobject Lib:\n  def value: Any =\n{locals_}    {call}\n" if locals_ else f"package g_{n}\nobject Lib:\n  def value: Any = {call}\n"
        b = (f"package g_{n}\nobject Main:\n  def show(x: Any): String = x match\n    case a: Array[?] => a.map(show).mkString(\"Array(\", \", \", \")\")\n"
             f"    case other => String.valueOf(other)\n  def main(args: Array[String]): Unit = println(show(Lib.value))\n")
        os.makedirs(f"{R}/{n}"); open(f"{R}/{n}/A.scala", "w").write(a); open(f"{R}/{n}/B.scala", "w").write(b)
        open(f"{P}/{n}.scala", "w").write(a + b.split("\n", 1)[1]); names.append(n)
open(f"{P}/rows.txt", "w").write("\n".join(names) + "\n")
open(f"{D}/left.txt", "w").write("".join(f"{r['what']} {r['owner']}.{r['name']} ({r['file']}:{r['line']}): {why}\n" for r, why in left))
print(f"{len(rows)} definitions; {len(rows) - len(left)} generated as {len(names)} programs; {len(left)} left (left.txt)")
