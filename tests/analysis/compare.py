#!/usr/bin/env python3
"""Compares teq's analysis of a project with scalac's (tests/analysis.sh).

Each side is `{"api": <graph>, "hashes": [...]}` as the oracle build writes it
(integrations/sbt/analysis/project/Oracle.scala): the graph of every class of every source, in
the form of `src/jvm/api.rs`, and zinc's hashes of each class (`apiHash`, `extraHash` and the
name hashes). A class is compared by its name and kind (a class, a trait, an object), then
definition by definition, each definition rendered whole as text (a `Structure` shared between
two places is written out at both), the declarations and the inherited definitions as
multisets, its header, self type, sealed children and parents as they are, the source that
defines it; then zinc's hashes, the name hashes name by name, and the products.

  compare.py <scalac.json> <teq.json> [--expect <file>] [--case <name>] [--show]

Then the dependencies (`deps`), the callbacks the compile made as canonical tuples, per
referring class: `usedName` (the class, the name, its scopes), `classDependency` (the class
depended on, the referring class, the context) and `binaryDependency` (the binary entry, the
binary class name, the referring class, the source, the context), each compared exactly as a
set; and beside them the relations zinc stored from them, at their own granularity: the class
dependencies by context, internal or external, each source's libraries, the one class zinc
keeps per library, and the names each class uses. A tuple's difference is named by its part,
its referring class and its name (the name used, the class depended on, the binary class name);
a stored relation's difference is explained by an explained difference of the tuples it comes
from (a class dependency by one of its referring class, target and context, a library by one
of a binary dependency on it from that source, a used name by one of the name and scopes).

`--expect` names the documented mappings (tests/analysis/expected.txt): lines `<case> <class>
<name> <mapping>`, a difference in `<name>` of `<class>` that the mapping explains: a definition
of that name or its name hash, or `<header>`, `<self>`, `<children>`, `<parents>`,
`<products>`, `<class>` (a class or a companion one side has and the other lacks; a side
that names a class anywhere, a product among them, must have its class and its hashes); lines
`<case> <part> <class> <name> <mapping> <side> <fields>` a tuple of the dependencies' `<part>`
(`usedName`, `classDependency`, `binaryDependency`) of that referring class and name that
`<side>` (`scalac` or `teq`) has and the other lacks, `<fields>` the rest of the tuple joined by
commas (a used name's scopes; a class dependency's context; a binary dependency's entry, source
and context), so that a mapped difference explains no other: a dependency line without its side
and fields fails; `(any)` stands for every case, every name of the class or every class. A difference no line explains
fails, and so does a line that explains nothing (it is stale). `apiHash` and `extraHash` may
differ only in a class with an explained difference: where nothing else differs, they tell of
what the text does not show, the objects zinc hashes as one. Hash data or dependencies missing
on either side fail. Prints one line per class and the totals; the exit code is 1 when anything
fails."""

import json
import sys


# Stands for every project, class or name in tests/analysis/expected.txt (`*` is a method's name).
ANY = "(any)"


class Missing(Exception):
    pass


def load(path):
    d = json.load(open(path))
    if not isinstance(d.get("api"), dict) or not isinstance(d["api"].get("nodes"), list) or not isinstance(d["api"].get("files"), list):
        raise Missing("%s: no graph" % path)
    if not isinstance(d.get("hashes"), list):
        raise Missing("%s: no hashes" % path)
    deps = d.get("deps")
    arities = {"usedName": 3, "classDependency": 3, "binaryDependency": 5}
    if not isinstance(deps, dict) or any(not isinstance(deps.get(k), list) or any(not (isinstance(t, list) and len(t) == n) for t in deps[k]) for k, n in arities.items()):
        raise Missing("%s: no dependencies" % path)
    relations = deps.get("relations")
    if not isinstance(relations, dict) or any(not isinstance(relations.get(k), list) for k in RELATIONS):
        raise Missing("%s: no stored relations" % path)
    return {"nodes": d["api"]["nodes"], "files": d["api"]["files"], "hashes": d["hashes"], "deps": deps}


# The parts of the dependency callbacks: per tuple its referring class, the name a mapping
# names it by, and what else it holds.
PARTS = {
    "usedName": lambda t: (t[0], t[1], tuple(t[2])),
    "classDependency": lambda t: (t[1], t[0], (t[2],)),
    "binaryDependency": lambda t: (t[2], t[1], (t[0], t[3], t[4])),
}

RELATIONS = ("classes", "libraries", "libraryClasses", "names")


def dep_tuples(side):
    """{(part, class): {(name, rest)}} of a side's callbacks."""
    out = {}
    for part, key in PARTS.items():
        for t in side["deps"][part]:
            cls, name, rest = key(t)
            out.setdefault((part, cls), set()).add((name, rest))
    return out


def relation_items(side):
    """The stored relations as (relation, key, value) items."""
    r = side["deps"]["relations"]
    items = set()
    for f, kind, t, ctx in r["classes"]:
        items.add(("classes", f, (kind, t, ctx)))
    for src, lib in r["libraries"]:
        items.add(("libraries", src, lib))
    for lib, c in r["libraryClasses"]:
        items.add(("libraryClasses", lib, c))
    for c, n, scopes in r["names"]:
        items.add(("names", c, (n, tuple(scopes))))
    return items


class Graph:
    def __init__(self, nodes):
        self.nodes = nodes

    def ty(self, i, seen=()):
        n = self.nodes[i]
        k = n[0]
        if k == "EmptyType":
            return "<empty>"
        if k == "Projection":
            return self.ty(n[1], seen) + "#" + n[2]
        if k == "ParameterRef":
            return "<" + n[1] + ">"
        if k == "Singleton":
            return ".".join(c[1] if c[0] == "Id" else ("this" if c[0] == "This" else "super(" + str(c[1]) + ")") for c in n[1])
        if k == "Parameterized":
            return self.ty(n[1], seen) + "[" + ", ".join(self.ty(a, seen) for a in n[2]) + "]"
        if k == "Constant":
            return self.ty(n[1], seen) + "(" + json.dumps(n[2]) + ")"
        if k == "Annotated":
            return self.annots(n[2], seen) + self.ty(n[1], seen)
        if k == "Structure":
            if i in seen:
                return "{rec}"
            s = seen + (i,)
            return "{" + " with ".join(self.ty(p, s) for p in n[1]) + (" { " + "; ".join(self.defn(d, s) for d in n[2]) + " }" if n[2] else "") + "}"
        if k in ("Existential", "Polymorphic"):
            return self.ty(n[1], seen) + (" forSome " if k == "Existential" else " ") + self.tparams(n[2], seen)
        return "?" + k

    def access(self, a):
        if a == "Public":
            return ""
        kind, q = a
        qs = {"Unqualified": "", "This": "[this]"}.get(q) if isinstance(q, str) else "[" + q[1] + "]"
        return kind.lower() + qs + " "

    @staticmethod
    def mods(m):
        names = ["abstract", "override", "final", "sealed", "implicit", "lazy", "macro", "superAccessor"]
        return "".join(n + " " for i, n in enumerate(names) if m & (1 << i))

    def annots(self, a_s, seen=()):
        return "".join("@" + self.ty(a[0], seen) + ("(" + ",".join("%s=%s" % (x[0], x[1]) for x in a[1]) + ")" if a[1] else "") + " " for a in a_s)

    def tparams(self, ts, seen=()):
        if not ts:
            return ""
        v = {"Covariant": "+", "Contravariant": "-", "Invariant": ""}
        return "[" + ", ".join(self.annots(t[1], seen) + v[t[3]] + t[0] + self.tparams(t[2], seen) + " >: " + self.ty(t[4], seen) + " <: " + self.ty(t[5], seen) for t in ts) + "]"

    def params(self, pls, seen=()):
        mod = {"Plain": "", "Repeated": "*", "ByName": "=>"}
        return "".join("(" + ("implicit " if l[1] else "") + ", ".join(p[0] + ": " + mod[p[3]] + self.ty(p[1], seen) + (" = ..." if p[2] else "") for p in l[0]) + ")" for l in pls)

    def defn(self, i, seen=()):
        n = self.nodes[i]
        k = n[0]
        head = self.annots(n[4], seen) + self.access(n[2]) + self.mods(n[3])
        if k == "Def":
            return head + "def " + n[1] + self.tparams(n[5], seen) + self.params(n[6], seen) + ": " + self.ty(n[7], seen)
        if k in ("Val", "Var"):
            return head + k.lower() + " " + n[1] + ": " + self.ty(n[5], seen)
        if k == "TypeAlias":
            return head + "type " + n[1] + self.tparams(n[5], seen) + " = " + self.ty(n[6], seen)
        if k == "TypeDeclaration":
            return head + "type " + n[1] + self.tparams(n[5], seen) + " >: " + self.ty(n[6], seen) + " <: " + self.ty(n[7], seen)
        if k == "ClassLikeDef":
            return head + n[6] + " " + n[1] + self.tparams(n[5], seen)
        if k == "ClassLike":
            return head + n[5] + " " + n[1]
        return "?" + k

    def name_of(self, i):
        return self.nodes[i][1]

    def classlike(self, i):
        n = self.nodes[i]
        s = self.nodes[n[7]]
        header = self.annots(n[4]) + self.access(n[2]) + self.mods(n[3]) + n[5] + " " + n[1] + self.tparams(n[11]) + (" top" if n[10] else "")
        self_type = self.ty(n[6])
        return {
            "name": n[1],
            "kind": n[5],
            "header": header,
            "self": self_type,
            "children": [self.ty(c) for c in n[9]],
            "parents": [self.ty(p) for p in s[1]],
            "declared": [(self.nodes[d][1], self.defn(d)) for d in s[2]],
            "inherited": [(self.nodes[d][1], self.defn(d)) for d in s[3]],
        }


def classes_of(side):
    out = {}
    g = Graph(side["nodes"])
    for f in side["files"]:
        for c in f["classes"]:
            cl = g.classlike(c)
            cl["file"] = f["file"]
            out[(cl["name"], cl["kind"])] = cl
    return out


def hashes_of(side, label):
    out = {}
    for h in side["hashes"]:
        if not isinstance(h, dict) or not isinstance(h.get("name"), str):
            raise Missing("%s: a hash entry without its class" % label)
        for field in ("apiHash", "extraHash"):
            if not isinstance(h.get(field), int):
                raise Missing("%s: %s has no %s" % (label, h["name"], field))
        names = h.get("nameHashes")
        if not isinstance(names, list) or any(not (isinstance(x, list) and len(x) == 3) for x in names):
            raise Missing("%s: %s has no name hashes" % (label, h["name"]))
        out[h["name"]] = {"api": h["apiHash"], "extra": h["extraHash"], "names": {(n, s): v for n, s, v in names}}
    return out


def multiset_diff(a, b):
    a, b = list(a), list(b)
    only_a = []
    for x in a:
        if x in b:
            b.remove(x)
        else:
            only_a.append(x)
    return only_a, b


def products_of(side):
    out = {}
    for f in side["files"]:
        for name, binary in f.get("products", []):
            out.setdefault(name, set()).add(binary)
    return out


def shape_diffs(a, b):
    """The differences of two renderings of a class: (the name a mapping names, the part, scalac's
    text, teq's)."""
    diffs = []
    for field in ("header", "self", "children", "parents"):
        if a[field] != b[field]:
            diffs.append(("<%s>" % field, field, a[field], b[field]))
    for field in ("declared", "inherited"):
        only_s, only_t = multiset_diff(a[field], b[field])
        for n, d in only_s:
            diffs.append((n, field + " " + n, d, None))
        for n, d in only_t:
            diffs.append((n, field + " " + n, None, d))
    return diffs


def compare_deps(scalac, teq, explained):
    """The dependencies of the two sides, per referring class and part; then the stored
    relations. Returns the failures, the exact and the mapped (part, class) pairs."""
    failures = exact = mapped = 0
    st, tt = dep_tuples(scalac), dep_tuples(teq)
    # Every explained tuple difference: its part, class, name, the rest of the tuple and the
    # side that has it.
    explained_tuples = []
    for part, cls in sorted(set(st) | set(tt)):
        a, b = st.get((part, cls), set()), tt.get((part, cls), set())
        if a == b:
            exact += 1
            continue
        diffs = [(n, r, "scalac") for n, r in a - b] + [(n, r, "teq") for n, r in b - a]
        found = [(n, r, where, explained(cls, n, part, where, r)) for n, r, where in sorted(diffs)]
        unexplained = [d for d in found if d[3] is None]
        for n, r, where, m in found:
            if m is not None:
                explained_tuples.append((part, cls, n, r, where))
        if unexplained:
            failures += 1
            print("FAIL    %s %s: %s" % (part, cls, ", ".join(sorted(set(d[0] for d in unexplained)))))
            for n, r, where, _ in unexplained:
                print("  only %s: %s %s" % (where, n, " ".join(r)))
        else:
            mapped += 1
            print("mapped  %s %s: %s" % (part, cls, ", ".join(sorted(set("%s (%s)" % (d[0], d[3]) for d in found)))))
    sr, tr = relation_items(scalac), relation_items(teq)

    # A stored relation differs where an explained tuple difference it comes from does, on the
    # same side: the same classes and context, the same name and scopes, the same library and
    # source.
    def relation_explained(item, side):
        rel, key, value = item
        tuples = [(p, c, n, r) for p, c, n, r, where in explained_tuples if where == side]
        if rel == "classes":
            to, ctx = value[1], value[2]
            return any(
                c == key and n in (to, to + "$") and r[-1] == ctx
                for p, c, n, r in tuples
                if p in ("classDependency", "binaryDependency")
            )
        if rel == "names":
            return any(p == "usedName" and c == key and n == value[0] and r == value[1] for p, c, n, r in tuples)
        if rel == "libraries":
            return any(p == "binaryDependency" and r[0] == value and r[1] == key for p, _, _, r in tuples)
        return any(p == "binaryDependency" and r[0] == key for p, _, _, r in tuples)

    for item in sorted(sr ^ tr, key=repr):
        if not relation_explained(item, "scalac" if item in sr else "teq"):
            failures += 1
            rel, key, value = item
            print("FAIL    relations %s %s: only %s: %s" % (rel, key, "scalac" if item in sr else "teq", value))
    return failures, exact, mapped


def main():
    args = sys.argv[1:]
    expect_path = None
    case = ""
    show = False
    if "--expect" in args:
        i = args.index("--expect")
        expect_path = args[i + 1]
        del args[i:i + 2]
    if "--case" in args:
        i = args.index("--case")
        case = args[i + 1]
        del args[i:i + 2]
    if "--show" in args:
        args.remove("--show")
        show = True
    try:
        scalac, teq = load(args[0]), load(args[1])
        sh, th = hashes_of(scalac, "scalac"), hashes_of(teq, "teq")
    except Missing as e:
        print("FAIL    %s" % e)
        print("summary %s: 0 exact, 0 mapped, 1 failing" % (case or args[0]))
        sys.exit(1)
    expected = []
    unconstrained = []
    exact_case = False
    if expect_path:
        for line in open(expect_path):
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if parts[0] == "exact" and len(parts) == 2 and parts[1] == case:
                exact_case = True
            elif len(parts) == 4 and parts[0] in (case, ANY):
                expected.append(("api", parts[1], parts[2], parts[3], None, None))
            elif len(parts) == 7 and parts[1] in PARTS and parts[0] in (case, ANY):
                expected.append((parts[1], parts[2], parts[3], parts[4], parts[5], parts[6]))
            elif len(parts) > 4 and parts[1] in PARTS and parts[0] in (case, ANY):
                print("FAIL    %s: a dependency's mapping names its side and its tuple's other fields" % line)
                unconstrained.append(line)

    used = set()

    def explained(cls, name, part="api", side=None, rest=None):
        if exact_case:
            return None
        detail = None if rest is None else ",".join(rest)
        for line in expected:
            p, c, n, m, s, d = line
            if p != part or not (c == ANY or c == cls) or not (n == ANY or n == name):
                continue
            if p != "api" and (s != side or d != detail):
                continue
            used.add(line)
            return m
        return None

    sc, tc = classes_of(scalac), classes_of(teq)
    sp, tp = products_of(scalac), products_of(teq)
    failures = exact = mapped = 0
    for cname in sorted(set(sh) | set(th) | set(n for n, _ in sc) | set(n for n, _ in tc) | set(sp) | set(tp)):
        # A side that names the class anywhere, its products among them, has its class and its
        # hashes.
        named = {}
        broken = False
        for side, h, c, p in (("scalac", sh, sc, sp), ("teq", th, tc, tp)):
            has_class = any(n == cname for n, _ in c)
            named[side] = cname in h or has_class or cname in p
            if named[side] and (cname not in h or not has_class):
                failures += 1
                broken = True
                print("FAIL    %s: %s names it without %s" % (cname, side, "its hashes" if cname not in h else "its class"))
        if broken:
            continue
        if not named["scalac"] or not named["teq"]:
            m = explained(cname, "<class>")
            where = "teq" if named["teq"] else "scalac"
            if m:
                mapped += 1
                print("mapped  %s: only %s (%s)" % (cname, where, m))
            else:
                failures += 1
                print("FAIL    %s: only %s" % (cname, where))
            continue
        # Each difference with the name a mapping names it by.
        diffs = []
        kinds_s = set(k for n, k in sc if n == cname)
        kinds_t = set(k for n, k in tc if n == cname)
        for kind in sorted(kinds_s ^ kinds_t):
            diffs.append(("<class>", "%s %s only in %s" % (kind, cname, "scalac" if kind in kinds_s else "teq"), None, None))
        for kind in sorted(kinds_s & kinds_t):
            label = cname + ("$" if kind == "Module" else "")
            if sc[(cname, kind)]["file"] != tc[(cname, kind)]["file"]:
                diffs.append(("<file>", label + " file", sc[(cname, kind)]["file"], tc[(cname, kind)]["file"]))
            for name, part, x, y in shape_diffs(sc[(cname, kind)], tc[(cname, kind)]):
                diffs.append((name, label + " " + part, x, y))
        hs, ht = sh[cname]["names"], th[cname]["names"]
        for key in sorted(set(hs) | set(ht)):
            if hs.get(key) != ht.get(key):
                diffs.append((key[0], "name hash of %s (%s)" % key, hs.get(key), ht.get(key)))
        if sp.get(cname, set()) != tp.get(cname, set()):
            diffs.append(("<products>", "products", sorted(sp.get(cname, set())), sorted(tp.get(cname, set()))))
        found = [(n, part, x, y, explained(cname, n)) for n, part, x, y in diffs]
        unexplained = [d for d in found if d[4] is None]
        for h, label in (("api", "apiHash"), ("extra", "extraHash")):
            if sh[cname][h] != th[cname][h] and not found:
                unexplained.append(("<%s>" % label, label + " differs where nothing else does", sh[cname][h], th[cname][h], None))
        if not found and not unexplained:
            exact += 1
            print("exact   %s (%d names)" % (cname, len(hs)))
        elif not unexplained:
            mapped += 1
            print("mapped  %s: %s" % (cname, ", ".join(sorted(set("%s (%s)" % (d[0], d[4]) for d in found)))))
        else:
            failures += 1
            print("FAIL    %s: %s" % (cname, ", ".join(sorted(set(d[0] for d in unexplained)))))
        shown = (found if show else [d for d in found if d[4] is None]) + [d for d in unexplained if d not in found]
        for n, part, x, y, m in shown:
            print("  %s\n    scalac: %s\n    teq:    %s" % (part, x, y))
    f, dep_exact, dep_mapped = compare_deps(scalac, teq, explained)
    failures += f + len(unconstrained)
    for line in expected:
        p, c, n, m, s, d = line
        if c != ANY and n != ANY and line not in used:
            where = "" if p == "api" else " (only %s: %s)" % (s, d)
            print("stale   %s%s %s %s%s: nothing differs there now" % ("" if p == "api" else p + " ", c, n, m, where))
            failures += 1
    print("summary %s: %d exact, %d mapped, %d failing; dependencies of %d referring classes by part exact, %d mapped" % (case or args[0], exact, mapped, failures, dep_exact, dep_mapped))
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
