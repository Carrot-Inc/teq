#!/usr/bin/env python3
"""Each landing's closure from dotty 3.8.4's own call graph (README.md): every member of `dotty.tools.dotc` its entries
reach transitively, from the SemanticDB documents semanticdb.sh writes.

  closure.py <work dir> <entries> <out dir>
  closure.py --ready <landing> <work dir> <entries> <out dir> [<table>]
  closure.py --citations <table> <lists dir>
  closure.py --entries <table> <work dir> <out file>

The graph. A node is a term member of `dotty/tools/dotc/` (a method, a val or var, a constructor, an object), named by
its SemanticDB symbol and placed by its definition's file and line. An edge goes from the member whose definition
encloses a reference occurrence, or a synthetic one (an implicit conversion, a given argument, an inserted `apply`), to
the member referred to (a class's constructor where `new` or a creator application, `C(..)` of a class with no
companion object, builds it); the enclosing member is found from the source's definitions (a definition's extent ends
before the next line indented as deep or less). A reference to a method its subclasses override also reaches every
override, transitively (`overridden_symbols` inverted): dynamic dispatch, each such member a candidate of the method
called. A reference to an object reaches its initializer, its vals and vars. A symbol outside `dotty/tools/dotc/` (the
standard library, the JDK, tasty-core, `dotty.tools.io`) is the boundary: listed, never walked.

The entries file holds, per landing, the members its mechanisms start from: `landing <N> <name>` heads a block, then
one `<symbol>` a line (`#` comments). A member's status is the first landing whose entries reach it (a helper lands
with or before its first caller), its own lines in that landing's file; a member an `owner` line gives another part is
that part's (`ported-by:<part>(shared-helpers-<n>)`, the ledger's entry). Lines before the first landing hold for every
landing: `cut <symbol or prefix*> <cut>` stops the walk at a dependency's entry (listed with its callers' count, the
dispatch candidates of a call into it going with it), `owner <symbol or prefix*> <part> <entry>` gives a member's owner
and `flag <symbol or prefix*> <teq's flag>` writes teq's flag beside a Settings read; `split <symbol or prefix*>
<setting>` marks a member that serves both of a setting's branches, walked (its branch with the setting off is in
scope), its calls that run only with the setting on stopping at their own `cut` lines. `decline <caller> <callee>
<why>` drops one edge, a call no input of the landing selects (the input that would select it named), in a landing's
block for that landing, before the first for every one; the callee is reached through its other callers or not at all.
Every member a landing reaches carries its destination (the ruling on round 12, 1): `port <symbol or prefix*> <teq
function>[ <disposition>]` (`ports.txt`, read by entries.txt's `include` line; a `+name` a function to write), or its
`owner` or `flag` line (a `cut` or `decline` line keeps a member out of the list). The disposition `older` (ledger
78) names an existing teq function kept as the one copy, its branch comparison in the table, never a second copy
beside a port; a port to an existing function carries it, a port to a function to write (`+name`) none. Dotty's
plumbing is ledger 77's cut class, `cut <member> plumbing(ledger-77) <observable> => <teq structure>` (plumbing.txt):
the walk stops there, and the list's `contracts` section names, per entry under it, the observable it stands for and
teq's structure that realizes it; a `port` line whose symbol falls under such a cut fails the run, its row naming the
contract instead (the owner's ruling on round 13's landing-2 pass, 1). Writes `<out dir>/landing-<N>.txt` (`<symbol>
<file>:<line> <status>[ candidate-of <symbol>]`, one member a line, by file and line, then the boundary's symbols with
their callers' count), `<out dir>/unported-<N>.txt` (the members whose status is `landing:<N>` and which carry none of
those lines, the landing that ports them having no destination for them) and `<out dir>/summary.txt` (each landing's
counts, its unported among them), on every run and whatever the counts: the run never fails on them. `--ready <N>`
runs the same and fails unless landing N and every landing before it in landing order are at zero unported (a
helper lands with or before its first caller): the readiness check's verb, never the default run's.
`--citations <table> <lists dir>` resolves a table's citations of the lists, `landing-<N>.txt @ <symbol or prefix*>`
(by symbol, never by line), and fails on one that names no member, candidate, cut entry, declined edge, reader or
boundary symbol of that list, and on a citation by line; it fails too on a destination row that takes a ledger-77 cut
member's val or var as its argument or state without naming the contract (`rule a`), and on a `Retires: none` row
whose teq arm does not begin `none` or whose own destination function is an existing teq function, unless a member's
port is `older` (`rule b`; the owner's ruling on round 13's landing-2 pass, 1), and on a row's destination cell or a
note whose `+` destination ends in a ledger-77 cut field's snake-case name that means the cut member (`rule d`; the
owner's ruling on round 14's class 4), and on a destination row that withdraws a member ports.txt ports (`rule e`;
finding 53.1: the row and the closure say the same thing). It reports rule a's sentence test over every six-cell row
of any id as `rule a+`, counted and not failing yet (the owner reads the count first).
"""
import collections
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "..", "semanticdb"))
import semanticdb  # noqa: E402  (tests/semanticdb/semanticdb.py, schema 4)

DOTC = "dotty/tools/dotc/"
TERM_KINDS = {"METHOD", "FIELD", "CONSTRUCTOR", "OBJECT", "MACRO", "PACKAGE_OBJECT"}
DEF = re.compile(r"^(\s*)(?:@\w+(?:\([^)]*\))?\s+)*(?:(?:private|protected)(?:\[[\w.]+\])?\s+|final\s+|override\s+|inline\s+"
                 r"|transparent\s+|implicit\s+|lazy\s+|abstract\s+|sealed\s+|case\s+|given\s+|opaque\s+|open\s+"
                 r"|infix\s+|erased\s+)*(def|class|object|trait|val|var|enum|given|type)\b")


def documents(work):
    root = os.path.join(work, "semanticdb")
    for d, _, fs in os.walk(root):
        for f in sorted(fs):
            if f.endswith(".semanticdb"):
                with open(os.path.join(d, f), "rb") as h:
                    yield from semanticdb.decode_documents(h.read())


def extents(src_lines):
    """Each definition line's (1-based) last line, by indentation: the next non-blank, non-comment line indented as
    deep or less ends it; at its own indentation a `)` or `]` continues its signature (a parameter list closed on its
    own line), a `catch` or `finally` its `try` and an `extends`, `with` or `derives` a class's parent clause (the
    clause on a line of its own), the first line after a definition line ending in `=` is its body whatever its
    indentation (dotty's `def f(..) =` followed by `trace(..):` at the def's own indentation), and a closing `}` or
    `end` is its last line."""
    ind = []
    for l in src_lines:
        s = l.strip()
        ind.append(None if not s or s.startswith(("//", "*", "/*")) else len(l) - len(l.lstrip()))
    out = {}
    n = len(src_lines)
    for i, l in enumerate(src_lines):
        if not DEF.match(l) or ind[i] is None:
            continue
        j = i + 1
        body_next = src_lines[i].split("//")[0].rstrip().endswith("=")
        while j < n:
            if ind[j] is not None and body_next:
                body_next = False  # the definition's body begins here, at any indentation
                j += 1
                continue
            if ind[j] is not None and ind[j] <= ind[i]:
                s = src_lines[j].strip()
                # A parameter list closed at the definition's indentation, a `try`'s `catch` or `finally` there
                # (`def f = try .. \n catch case ..`, Types.scala:435), or a class's parent clause on its own line
                # (`extends`, `with` or `derives`; Types.scala:4913-4914): the definition goes on.
                if ind[j] == ind[i] and (s.startswith((")", "]"))
                                        or re.match(r"^(catch|finally|extends|with|derives)\b", s)):
                    j += 1
                    continue
                if ind[j] == ind[i] and (s.startswith("}") or s == "end" or s.startswith("end ")):
                    j += 1
                break
            j += 1
        out[i + 1] = j  # last line, 1-based inclusive
    return out


SPANS = {}  # uri -> its definitions' extents, (first line, last line, symbol), for `member_at`


def load(work):
    """The graph's data: each global definition's place and kind, each class's parents, the overrides, and per member
    the references in its extent (a call or a read) and the classes it instantiates."""
    defs, kinds, props, parents = {}, {}, {}, {}
    overrides = collections.defaultdict(set)
    occs = collections.defaultdict(list)   # uri -> [(line, col, end col, symbol, role)]
    syns = collections.defaultdict(list)   # uri -> [(line, symbol)]
    for doc in documents(work):
        uri = doc["uri"]
        for info in doc["symbols"]:
            sym = info["symbol"]
            kinds[sym] = info["kind"]
            props[sym] = info["properties"]
            for o in info["overridden_symbols"]:
                overrides[o].add(sym)
            sig = info.get("signature") or {}
            if sig.get("kind") == "ClassSignature":
                parents[sym] = [p.get("symbol") for p in sig.get("parents") or [] if p.get("symbol")]
        for occ in doc["occurrences"]:
            sym, r = occ["symbol"], occ["range"]
            if not sym or r is None or sym.startswith("local"):
                continue
            occs[uri].append((r["start_line"] + 1, r["start_character"], r["end_character"], sym, occ["role"]))
            if occ["role"] == "DEFINITION":
                defs.setdefault(sym, (uri, r["start_line"] + 1))
        for syn in doc["synthetics"]:
            if syn["range"] is not None:
                for sym in synthetic_symbols(syn["tree"]):
                    if not sym.startswith("local"):
                        syns[uri].append((syn["range"]["start_line"] + 1, sym))
    calls = collections.defaultdict(set)   # member -> symbols it refers to
    news = collections.defaultdict(set)    # member -> classes it instantiates
    companion = collections.defaultdict(list)  # object -> its apply, unapply and unapplySeq overloads in source
    for d in defs:
        m = re.match(r"^(.*\.)(apply|unapply|unapplySeq)\(", d)
        if m:
            companion[m.group(1)].append(d)
    # The constructor each construction selects, which SemanticDB does not name: semanticdb.sh's plugin phase
    # (plugin/Constructions.scala), `<kind> <source path> <line> <constructor>`, its paths under the work dir's src/.
    cons = collections.defaultdict(list)
    cpath = os.path.join(work, "constructions.txt")
    if not os.path.exists(cpath):
        sys.exit(f"closure.py: no {cpath}: run semanticdb.sh, whose plugin phase writes it")
    root = os.path.realpath(os.path.join(work, "src")) + "/"
    for l in open(cpath, encoding="utf-8"):
        kind, path, line, ctor = l.rstrip("\n").split(" ")
        cons[os.path.realpath(path).removeprefix(root)].append((int(line), kind, ctor))
    # The `apply` overloads the typer selected, by place: the plugin's `call` records.
    selected = collections.defaultdict(set)
    for uri, rows in cons.items():
        for line, kind, sym in rows:
            if kind == "call":
                selected[(uri, line)].add(sym)
    for uri, os_ in occs.items():
        try:
            src = open(os.path.join(work, "src", uri), encoding="utf-8").read().split("\n")
        except OSError:
            continue
        ext = extents(src)
        spans = sorted(((line, ext.get(line, line), sym) for line, _, _, sym, role in os_ if role == "DEFINITION"),
                       key=lambda t: (t[0], -t[1]))
        SPANS[uri] = spans
        for line, col, end, sym, role in os_:
            if role != "REFERENCE":
                continue
            text = src[line - 1] if line - 1 < len(src) else ""
            if text.lstrip().startswith(("import ", "export ")):
                continue
            owner = member_at(spans, line, kinds, defs)
            if not owner:
                continue
            k = kinds.get(sym)
            # A construction is the typed tree's (`constructions.txt`, below), not the class's occurrence here.
            if k == "OBJECT" and text[end:end + 1] in ("(", "["):
                # `C(..)`: the companion's `apply` or `unapply` in source, or a case class's synthetic `apply`; the
                # overload the typer selected where the plugin recorded the inserted `apply` at the line (`call`),
                # every overload where it did not (a pattern's `unapply`).
                cls = sym[:-1] + "#"
                applies = companion.get(sym, [])
                chosen = {a for a in applies if a in selected.get((uri, line), ())}
                before = text[:col]
                at = before.rfind("case ")
                if not chosen and at >= 0 and "=>" not in before[at:]:
                    # A pattern (`case C(x) =>`, `for case C(x) <- ..`): the extractor, never the `apply`.
                    chosen = {a for a in applies if re.search(r"\.unapply(Seq)?\(", a)}
                calls[owner] |= (chosen or set(applies)) | {sym}
                if not any(a.startswith(sym + "apply(") for a in applies) and kinds.get(cls) == "CLASS":
                    news[owner].add(cls + "`<init>`().")  # the synthetic `apply` builds by the primary constructor
            else:
                calls[owner].add(sym)
        for line, sym in syns.get(uri, ()):
            owner = member_at(spans, line, kinds, defs)
            if owner:
                calls[owner].add(sym)
        # Each construction by the constructor the typer selected (`new C(..)`, a creator application `C(..)` or
        # `C[T](..)`, an anonymous class's parent call, a secondary constructor's `this(..)`), and each named class's
        # parent constructor, its parent clause's selection, which runs at every construction of a subclass (the ruling
        # on round 12's points, 5).
        for line, kind, ctor in cons.get(uri, ()):
            owner = member_at(spans, line, kinds, defs)
            if owner and kind in ("new", "self", "parent"):
                news[owner].add(ctor)
            elif owner and kind == "call":
                calls[owner].add(ctor)  # an inserted `apply` on a value
    # A named class's parent clause selects a constructor only for a parent applied to arguments, so the plugin's
    # `parent` records name no trait without arguments (CachedType has none); dotty still runs every trait parent's
    # initializer at a construction, by SemanticDB's parents. A class parent stays the plugin's record's: it names the
    # overload the clause selects.
    for cls, ps in parents.items():
        ctor = cls + "`<init>`()."
        if kinds.get(cls) not in ("CLASS", "OBJECT") or ctor not in defs:
            continue
        for p in ps:
            pctor = p + "`<init>`()."
            if kinds.get(p) == "TRAIT" and pctor in defs:
                news[ctor].add(pctor)
    return defs, kinds, props, parents, overrides, calls, news


def member_at(spans, line, kinds, defs):
    """The innermost term member whose definition's extent holds the line; a class's template body is its
    constructor's."""
    best = None
    for start, end, sym in spans:
        if start > line:
            break
        if end >= line and kinds.get(sym) in TERM_KINDS | {"CLASS", "TRAIT"}:
            best = sym
    if best and kinds.get(best) in ("CLASS", "TRAIT"):
        ctor = best + "`<init>`()."
        best = ctor if ctor in defs else best
    return best


def ancestors(cls, parents, memo={}):
    if cls in memo:
        return memo[cls]
    memo[cls] = {cls}
    out = {cls}
    for p in parents.get(cls, ()):
        out |= ancestors(p, parents)
    memo[cls] = out
    return out


def closure_of(entries, g, cuts=None, declines=None):
    """The members `entries` reach by static edges, walked transitively (a reference, a read, a synthetic call, a
    constructor by `new` or a case class's `apply`, a companion's `apply` or `unapply`); at a call to a method its
    subclasses override, every override is listed as a candidate of that call, not walked (dynamic dispatch); an
    object's reference reaches the methods called on it, not its initializer. A member under a cut (`cut_of`) is
    recorded with the members calling it and not walked; the candidates of a call to it go with it. A declined edge
    (`declines`, caller and callee) is not walked, recorded where its caller is reached."""
    defs, kinds, props, parents, overrides, calls, news = g
    reached, cand, objects = set(), {}, set()
    boundary = collections.Counter()
    cut_callers = collections.defaultdict(set)
    declined = set()
    stack = [(e, None) for e in entries]
    while stack:
        s, caller = stack.pop()
        if cuts and under_cut(s, cuts):
            cut_callers[s].add(caller)  # an entry of a dependency: listed under its cut with its callers, not walked
            continue
        if s in reached:
            continue
        if not s.startswith(DOTC):
            boundary[s] += 1
            continue
        if s not in defs or kinds.get(s) not in TERM_KINDS:
            continue
        if kinds.get(s) in ("OBJECT", "PACKAGE_OBJECT"):
            objects.add(s)
            continue
        reached.add(s)
        for ctor in news.get(s, ()):
            if not ctor.startswith(DOTC):
                boundary[ctor] += 1
                continue
            if declines and (s, ctor) in declines:
                declined.add((s, ctor))
                continue
            stack.append((ctor, s))
            # A class's vals and vars run at its construction, whichever constructor builds it.
            for d in members_of(owner_of(ctor), defs):
                ps = semanticdb.properties_names(props.get(d, 0))
                if kinds.get(d) == "METHOD" and {"VAL", "VAR"} & set(ps) and "LAZY" not in ps:
                    stack.append((d, s))
        for t in calls.get(s, ()):
            if not t.startswith(DOTC):
                boundary[t] += 1
                continue
            if declines and (s, t) in declines:
                declined.add((s, t))  # an edge no input of the landing selects: its candidates go with it
                continue
            stack.append((t, s))
            if cuts and under_cut(t, cuts):
                continue  # a call into a cut: its dispatch candidates go with it
            for o in dispatch(t, overrides):
                if o in defs and o.startswith(DOTC):
                    # The least of the methods a call dispatches from, whatever the walk's order.
                    cand[o] = min(cand.get(o, t), t)
    cand = {o: m for o, m in cand.items() if o not in reached and not (cuts and under_cut(o, cuts))}
    return reached, cand, boundary, dict(cut_callers), declined


_members = {}


def members_of(cls, defs):
    if not _members:
        for d in defs:
            o = owner_of(d)
            if o:
                _members.setdefault(o, []).append(d)
    return _members.get(cls, [])


def main(work, entries_path, out, ready=None):
    g = load(work)
    defs = g[0]
    landings, entries, readers, cuts = read_entries(entries_path)
    # An `older` port line names an existing teq function, at HEAD (the owner's ruling on round 13's stride samples,
    # class 1): one whose destination has no `fn` in its file fails the run, after all are read.
    stale = older_port_failures()
    if stale:
        sys.exit("\n".join(f"closure.py: an older port line names no function: {sym} {dest} (no fn {last} in {path})"
                           for sym, dest, path, last in stale))
    # Rule a's ports: a member under a ledger-77 cut carries its row's contract, never a port line (the owner's ruling
    # on round 13's landing-2 pass, 1). The cut line whose contract names the observable is the one that fails.
    ledger77 = {}
    for ls in cuts.values():
        ledger77.update({s: why for s, why in ls.items() if why.startswith("plumbing(ledger-77)")})
    violations = {}
    for ps in ports.values():
        for s in ps:
            cut = cut_of(s, ledger77)
            if cut:
                contract = ledger77[cut][len("plumbing(ledger-77)"):].strip()
                violations[s] = contract
    if violations:
        sys.exit("\n".join(f"closure.py: a port line for a member under ledger 77's cut: {s} (its contract: "
                           f"{violations[s]})" for s in sorted(violations)))
    order = [l for l, _ in landings]
    reach, cands, bounds, cutc, decl = {}, {}, {}, {}, {}
    for lab in order:
        # An entry outside dotty.tools.dotc is the boundary itself (the observer's, the Quotes API it calls).
        missing = [s for s in entries[lab] + readers[lab] if s not in defs and s.startswith(DOTC)]
        if missing:
            sys.exit(f"closure.py: landing {lab}: symbols with no definition at 3.8.4: {' '.join(missing[:10])}")
        lcuts = {**cuts[None], **cuts[lab]}
        ldecl = {**declines[None], **declines[lab]}
        active_splits.clear()
        active_splits.update({**splits, **landing_splits[lab]})
        reach[lab], cands[lab], bounds[lab], cutc[lab], decl[lab] = closure_of(entries[lab], g, lcuts or None, ldecl or None)
    status = {}
    for lab in order:
        for s in reach[lab]:
            status.setdefault(s, lab)
    for lab in order:
        for s in cands[lab]:
            status.setdefault(s, lab)
    names = dict(landings)
    os.makedirs(out, exist_ok=True)
    summary = []
    unassigned = []
    unported = {}
    for lab in order:
        lcuts = {**cuts[None], **cuts[lab]}
        rows = sorted((defs[s][0], defs[s][1], s) for s in reach[lab])
        crows = sorted((defs[s][0], defs[s][1], s) for s in cands[lab])
        rrows = sorted((defs[s][0], defs[s][1], s) for s in readers[lab])
        nb = sum(1 for t in bounds[lab] if t != "<?>" and not t.endswith("/"))
        with open(os.path.join(out, f"landing-{lab}.txt"), "w", encoding="utf-8") as f:
            f.write(f"# landing {lab}, {names[lab]}, at dotty 3.8.4 (closure.py, README.md): {len(entries[lab])} entries; "
                    f"{len(rows)} members they reach, {len(crows)} dispatch candidates, {nb} boundary symbols, "
                    f"{len(cutc[lab])} entries under cuts, {len(rrows)} readers\n")
            f.write("# members: <symbol> <file>:<line> <status>[ teq:<the flag a setting's branch reads>]\n")
            for uri, line, s in rows:
                st = status_of(s, status, uri)
                if lab in port_mode:
                    # A landing whose code ports what it reaches (the observer): a member by its `port` line alone; one
                    # with none is no status but a failure of the tool (the ruling on dc2a3dfec's pass, finding 6).
                    fn = ports.get(lab, {}).get(cut_of(s, ports.get(lab, {})) or "")
                    if not fn:
                        unassigned.append((lab, s))
                        continue
                    st = f"ported:{fn}"
                fl = flags.get(cut_of(s, flags) or "")
                lsp = {**splits, **landing_splits[lab]}
                sp = lsp.get(cut_of(s, lsp) or "")
                mark = (" " + sp if sp.startswith("above:") else " split:" + sp) if sp else ""
                f.write(f"{s} {uri.removeprefix('compiler/src/')}:{line} {st}{' teq:' + fl if fl else ''}{mark}\n")
            f.write("# candidates: candidate <override> <file>:<line> <status> of <the method its call dispatches from>\n")
            for uri, line, s in crows:
                f.write(f"candidate {s} {uri.removeprefix('compiler/src/')}:{line} {status_of(s, status, uri)} of {cands[lab][s]}\n")
            f.write("# boundary: boundary <symbol outside dotty.tools.dotc> <the count of members calling it>\n")
            for t, n in sorted(bounds[lab].items()):
                if t != "<?>" and not t.endswith("/"):  # an unresolved symbol, a package: no member
                    f.write(f"boundary {t} {n}\n")
            f.write("# cuts: cut <the entry called> <file>:<line> below <the cut> <the count of members calling it>\n")
            contracts = []
            for sym in sorted(cutc[lab], key=lambda x: (defs.get(x, ("~", 0)), x)):
                uri, line = defs.get(sym, ("*", 0))
                why = lcuts[cut_of(sym, lcuts)]
                if why.startswith("plumbing(ledger-77)"):
                    contracts.append((sym, why[len("plumbing(ledger-77)"):].strip()))
                    why = "plumbing(ledger-77)"
                f.write(f"cut {sym} {uri.removeprefix('compiler/src/')}:{line} below {why} "
                        f"{len(cutc[lab][sym] - {None})}\n")
            f.write("# contracts: contract <an entry under ledger 77's cut> <the observable it stands for> => <teq's"
                    " structure that realizes it>\n")
            for sym, c in contracts:
                f.write(f"contract {sym} {c}\n")
            f.write("# declined: decline <caller> <callee> <file>:<line> <why>, an edge no input of the landing selects,"
                    " its caller reached\n")
            ldecl = {**declines[None], **declines[lab]}
            for caller, callee in sorted(decl[lab], key=lambda e: (defs.get(e[1], ("~", 0)), e)):
                uri, line = defs.get(callee, ("*", 0))
                f.write(f"decline {caller} {callee} {uri.removeprefix('compiler/src/')}:{line} {ldecl[(caller, callee)]}\n")
            f.write("# readers: reader <symbol> <file>:<line>, a member the landing's rows name as calling its mechanism"
                    " (not walked)\n")
            for uri, line, s in rrows:
                f.write(f"reader {s} {uri.removeprefix('compiler/src/')}:{line}\n")
        # The members calling each cut entry, for the boundary rows (beside the work dir, not the list).
        with open(os.path.join(work, f"cut-callers-{lab}.txt"), "w", encoding="utf-8") as f:
            for sym in sorted(cutc[lab]):
                for caller in sorted(x for x in cutc[lab][sym] if x):
                    f.write(f"{sym} {caller}\n")
        own = sum(1 for _, _, s in rows if status.get(s) == lab)
        # The members this landing ports (its status, no owner) with no destination line.
        lports = {**ports.get(None, {}), **ports.get(lab, {})}
        unported[lab] = [(uri, line, s) for uri, line, s in rows
                         if status.get(s) == lab and not cut_of(s, owners) and not cut_of(s, flags)
                         and not cut_of(s, lports) and lab not in port_mode]
        disposed = collections.Counter(port_disposition.get(cut_of(s, lports) or "", "")
                                       for _, _, s in rows if status.get(s) == lab and cut_of(s, lports))
        plumbing = sum(1 for sym in cutc[lab] if lcuts[cut_of(sym, lcuts)].startswith("plumbing(ledger-77)"))
        with open(os.path.join(out, f"unported-{lab}.txt"), "w", encoding="utf-8") as f:
            f.write(f"# landing {lab}, {names[lab]}: the {len(unported[lab])} members of landing-{lab}.txt whose status is "
                    f"landing:{lab} and which carry no `port`, `owner` or `flag` line (closure.py, README.md)\n")
            for uri, line, s in unported[lab]:
                f.write(f"{s} {uri.removeprefix('compiler/src/')}:{line}\n")
        summary.append(f"landing {lab} ({names[lab]}): {len(entries[lab])} entries, {len(rows)} members ({own} first "
                       f"reached here, {len(unported[lab])} of them unported; {disposed['older']} kept as the older copy), "
                       f"{plumbing} entries under ledger 77's contracts, {len(crows)} candidates, {nb} boundary "
                       f"symbols, {len(cutc[lab])} entries under cuts, {len(rrows)} readers")
    with open(os.path.join(out, "summary.txt"), "w", encoding="utf-8") as f:
        f.write("\n".join(summary) + "\n")
    print("\n".join(summary))
    if unassigned:
        sys.exit(f"closure.py: {len(unassigned)} members reached above the boundary with no `port` line: "
                 + " ".join(f"{lab}:{s}" for lab, s in unassigned[:20]))
    if ready is not None:
        if ready not in order:
            sys.exit(f"closure.py --ready: no landing {ready}")
        behind = [f"landing {lab} {len(unported[lab])} unported (unported-{lab}.txt)"
                  for lab in order[:order.index(ready) + 1] if unported[lab]]
        # An entry no row places yet (entries-unbacked.txt) is owed its row (the ruling on round 12, 2).
        owed = collections.Counter(lab for lab, _ in unbacked_entries)
        behind += [f"landing {lab} {owed[lab]} entries no row places (entries-unbacked.txt)"
                   for lab in order[:order.index(ready) + 1] if owed[lab]]
        if behind:
            sys.exit(f"closure.py --ready {ready}: not ready: " + ", ".join(behind))
        print(f"closure.py --ready {ready}: landing {ready} and every landing before it at zero unported")


def status_of(s, status, uri):
    """`ported-by:<part>(shared-helpers-<n>)` for a member an `owner` line gives a named part (the ledger's entry
    <n>), else `landing:<N>`, the first landing that reaches the member."""
    o = cut_of(s, owners)
    if o:
        part, n = owners[o]
        return f"ported-by:{part}(shared-helpers-{n})"
    return f"landing:{status[s]}"


# A landing whose own code ports what it reaches (a `ports` line in its block): `port <symbol or prefix*> <its
# function>` assigns a member; a member reached with none fails the tool. The observer, landing 1, has the line and
# no `port`: it reaches nothing above its boundary, the Quotes API.
ports = {}
port_mode = set()
# A port line's disposition, by its symbol: `older`, an existing teq function kept as the one copy (ledger 78); none,
# a function to write.
port_disposition = {}
DISPOSITIONS = {"older"}
# Lines before the first landing hold for every landing: `cut`, `owner <symbol or prefix*> <part> <ledger entry>` and
# `flag <symbol or prefix*> <teq's flag, or none>`.
owners, flags, splits = {}, {}, {}
# A landing's own `split` lines (in its block), and the lines in force for the landing being walked.
landing_splits = collections.defaultdict(dict)
active_splits = {}
# `decline <caller> <callee> <why>` by landing (None: every landing), (caller, callee) -> why.
declines = collections.defaultdict(dict)
# The entries of entries-unbacked.txt, (landing, symbol): kept by hand before round 12, no row placing them yet.
unbacked_entries = []


def under_cut(s, cuts):
    """The cut line a member is under, None where a `split` or `above` line as specific as that cut's or more names it
    (a member of a cut prefix that serves both of a setting's branches, or that an input selects above the cut: walked,
    its other callees under their own lines)."""
    c = cut_of(s, cuts)
    if c is None:
        return None
    sp = cut_of(s, active_splits)
    if sp is not None and (sp == s or (c != s and len(sp) >= len(c))):
        return None
    return c


def cut_of(s, cuts):
    """The line a symbol falls under: its own, else the longest prefix line (`<prefix>*`) it starts with."""
    if s in cuts:
        return s
    best = None
    for c in cuts:
        if c.endswith("*") and s.startswith(c[:-1]) and (best is None or len(c) > len(best)):
            best = c
    return best


def dispatch(sym, overrides):
    out, stack, seen = [], [sym], set()
    while stack:
        s = stack.pop()
        for o in overrides.get(s, ()):
            if o not in seen:
                seen.add(o)
                out.append(o)
                stack.append(o)
    return out


def owner_of(sym):
    m = re.match(r"^(.*?[#.])[^#.]+(?:\(\+?\d*\))?[.#]?$", sym)
    return m.group(1) if m else None


def synthetic_symbols(tree):
    out = []

    def walk(t):
        if isinstance(t, dict):
            if t.get("kind") == "IdTree" and t.get("symbol"):
                out.append(t["symbol"])
            for v in t.values():
                walk(v)
        elif isinstance(t, list):
            for v in t:
                walk(v)
    walk(tree)
    return out


def read_entries(path):
    """`landing <N> <name>` heads a block; then `<symbol>` an entry or `reader <symbol>` a reader, one a line."""
    landings, entries, readers, cur = [], collections.defaultdict(list), collections.defaultdict(list), None
    cuts = collections.defaultdict(dict)
    ports.clear()
    port_mode.clear()
    port_disposition.clear()
    owners.clear()
    flags.clear()
    splits.clear()
    landing_splits.clear()
    declines.clear()
    unbacked_entries.clear()

    def read(p):
        for line in open(p, encoding="utf-8"):
            if line.startswith("include "):
                yield from ((p2, l2) for p2, l2 in read(os.path.join(os.path.dirname(p), line.split()[1])))
            else:
                yield p, line
    for src, line in read(path):
        if os.path.basename(src) == "entries-unbacked.txt" and line.strip() and not line.startswith(("#", "landing ")):
            unbacked_entries.append((None, line.strip()))
    for src, line in read(path):
        line = "" if line.lstrip().startswith("#") else line.strip()
        if not line:
            continue
        m = re.match(r"^landing (\S+) (.+)$", line)
        if m:
            cur = m.group(1)
            if cur not in [l for l, _ in landings]:  # a block opened again (the generated entries') continues it
                landings.append((cur, m.group(2)))
        elif line.startswith("reader "):
            readers[cur].append(line[len("reader "):])
        elif line == "ports":
            port_mode.add(cur)
        elif line.startswith("port "):
            sym, _, rest = line[len("port "):].partition(" ")
            fn, _, disp = rest.partition(" ")
            disp = disp.split(" ")[0]
            if disp and disp not in DISPOSITIONS:
                sys.exit(f"closure.py: a port line's disposition is one of {sorted(DISPOSITIONS)}: {line}")
            if (disp == "older") != fn.startswith("src/"):
                sys.exit(f"closure.py: an existing teq function (`src/..`) is kept as the one copy, `older`, and a function"
                         f" to write (`+name`) carries no disposition (ledger 78): {line}")
            ports.setdefault(cur, {})[sym] = fn
            if disp:
                port_disposition[sym] = disp
        elif line.startswith("cut "):
            sym, _, why = line[len("cut "):].partition(" ")
            if why.startswith("plumbing(ledger-77)") and " => " not in why:
                sys.exit(f"closure.py: a ledger-77 cut names its contract, `<observable> => <teq structure>`: {line}")
            cuts[cur][sym] = why
        elif line.startswith("owner ") and cur is None:
            sym, part, n = line[len("owner "):].split(" ", 2)
            owners[sym] = (part, n.split()[0])
        elif line.startswith("flag ") and cur is None:
            sym, _, fl = line[len("flag "):].partition(" ")
            flags[sym] = fl.split(" ")[0]
        elif line.startswith(("split ", "above ")):
            # `above <member or prefix*> <why>`: a member a cut takes that an input of the landing selects, walked
            # above the boundary (the ruling on round 11); exempt from its cut as a split member is.
            kw, rest = line.split(" ", 1)
            sym, _, why = rest.partition(" ")
            tok = why.split(" ")[0]
            (splits if cur is None else landing_splits[cur])[sym] = ("above:" + tok) if kw == "above" else tok
        elif line.startswith("decline "):
            caller, callee, why = line[len("decline "):].split(" ", 2)
            declines[cur][(caller, callee)] = why
        else:
            entries[cur].append(line)
            if os.path.basename(src) == "entries-unbacked.txt":
                unbacked_entries.append((cur, line))
    unbacked_entries[:] = [e for e in unbacked_entries if e[0] is not None]
    return landings, entries, readers, cuts


CITATION = re.compile(r"((?:[\w./-]*/)?)landing-(\w+)\.txt @ (\S+)")
LINE_CITATION = re.compile(r"landing-(\w+)\.txt:\d+")
# A destination row of the table: its number's landing then a dash (`R12.2-...`, `R12.op-...`). Six cells, the last
# the landing cell; `# | dotty | branches | teq arm | contract (destination) | landing`.
DESTINATION_ROW = re.compile(r"^\| R12\.(?:2|3|4|op|5|6|7)-")


def row_cells(line):
    """The cells of a table row, split on unescaped `|` (the first and last empty, the line's two edges)."""
    return [c.strip() for c in re.split(r"(?<!\\)\|", line.rstrip("\n"))]


def simple_name(symbol):
    """A symbol's last component without `()` or backticks (`myHash`, `lastDenotation`, `sames`, `owningState`)."""
    tail = re.split(r"[#.]", symbol.rstrip("."))[-1]
    return re.sub(r"\([^)]*\)$", "", tail).strip("`")


def class_name(symbol):
    """A class's simple name: its last component after a `/`, a `#` or a `.` (`CachedProxyType`, `SourceFile`)."""
    return re.split(r"[/#.]", symbol.rstrip("#."))[-1]


def work_dir():
    """The graph's work dir: `CLOSURE_WORK`, the work dir's usual place under the table's repository (`out/closure/work`,
    the command's current directory), or the work dir itself when the current directory is one."""
    if os.environ.get("CLOSURE_WORK"):
        return os.environ["CLOSURE_WORK"]
    for w in (os.path.join(os.getcwd(), "out", "closure", "work"), os.getcwd(),
              os.path.join(os.path.dirname(os.getcwd()), "out", "closure", "work"),
              os.path.join(HERE, "..", "..", "..", "..", "out", "closure", "work")):
        if os.path.exists(os.path.join(w, "constructions.txt")) and os.path.isdir(os.path.join(w, "semanticdb")):
            return w
    return os.path.join(os.getcwd(), "out", "closure", "work")


def citation_candidates(raw):
    """A citation's symbol read with the prose's punctuation possibly on its end, and a constructor written without
    SemanticDB's backticks read as the same symbol: every reading from the raw text down to its punctuation."""
    cand = raw.replace("\\`", "`")
    seen = []
    while cand:
        for c in (cand, re.sub(r"([#.])<init>\(", r"\1`<init>`(", cand)):
            if c not in seen:
                seen.append(c)
        if cand[-1] not in ".,;:)]`'":
            break
        cand = cand[:-1]
    return seen


def existing_teq_functions(root):
    """The `fn <name>` names under the work dir's `src/` (`grep -rhoE "fn [a-z_0-9]+" src`)."""
    try:
        out = subprocess.run(["grep", "-rhoE", r"fn [a-z_0-9]+", "src"], cwd=root, capture_output=True, text=True,
                             timeout=120).stdout
    except (OSError, subprocess.SubprocessError):
        out = ""
    return {m.group(1) for m in re.finditer(r"^fn ([a-z_0-9]+)$", out, re.M)}


def older_port_failures():
    """Each `older` port line whose destination names no function at HEAD (the owner's ruling on round 13's stride
    samples, class 1): `src/<path>.rs:<name>` must exist in the worktree and hold `fn <name's last `::` component>`."""
    bad = []
    for ps in ports.values():
        for sym, destination in ps.items():
            if port_disposition.get(sym) != "older":
                continue
            path, _, name = destination.partition(":")
            last = name.split("::")[-1]
            try:
                text = open(os.path.join(os.getcwd(), path), encoding="utf-8").read()
            except OSError:
                text = ""
            if not re.search(r"\bfn " + re.escape(last) + r"\b[<(]?", text):
                bad.append((sym, destination, path, last))
    return bad


def row_destination(destination):
    """The row's own function: the first `+snake_name` written in bold (`**`+name(...)`**` or `**+name**`) in its
    destination cell, or, with none in bold, the first `+snake_name` of the cell."""
    for bold in re.findall(r"\*\*(.+?)\*\*", destination):
        m = re.search(r"\+([a-z_0-9]+)", bold)
        if m:
            return m.group(1)
    m = re.search(r"\+([a-z_0-9]+)", destination)
    return m.group(1) if m else None


def snake_case(name):
    """A member's name in the snake case a teq destination writes (`owningState` -> `owning_state`, `myHash` ->
    `my_hash`)."""
    n = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    n = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", n)
    return n.lower()


# A `+` destination's name: `+owning_state`, `+TVarInfo.owning_state`, `+TVarInfo::owning_state`, the name ending the
# destination and a non-name character after it (`+owning_state(`, `+owning_state:`).
DESTINATION_NAME = re.compile(r"\+([A-Za-z0-9_$]+(?:(?:::|[\.])[A-Za-z0-9_$]+)*)(?![A-Za-z0-9_$])")
# A destination a line says is not made: the owner's words for a withdrawn destination, in the sentence naming it.
WITHDRAWN = re.compile(r"\b(?:withdrawn|retired|removed)\b|no field|not a field", re.I)
# A row's `unport <symbol>` directive: the keyword opens its code span, and the symbol after it is read with the
# portdir grammar (`out/tr/r13/portdir.py`), not by the code span's closing backtick (a symbol may hold backticks).
UNPORT = re.compile(r"(?<=`)unport\s+")
UNPORT_NAME = r"(?:`[^`\s]+`|<init>|[\w$]+)"
UNPORT_SYMBOL = re.compile(rf"(?:[\w$]+/)*(?:{UNPORT_NAME}(?:\(\+?\d*\))?[#.])*"
                           rf"{UNPORT_NAME}?(?:\(\+?\d*\)?)?[.#]?")


def unport_directives(text):
    """A destination cell's `unport <member>` directives: their symbols, as the text writes them."""
    text = text.replace("\\`", "`")
    symbols = []
    for m in UNPORT.finditer(text):
        symbol = UNPORT_SYMBOL.match(text[m.end():])
        if symbol and symbol.group(0) and re.search(r"[\w$>`]", symbol.group(0)):
            symbols.append(symbol.group(0))
    return symbols


def unport_names(symbol, member):
    """Whether an `unport` directive's symbol names a row's member: itself or with SemanticDB's terminators, a short
    one by its last name and optional owner (portdir's `full` and `resolve`)."""
    s = re.sub(r"(?<!`)<init>(?!`)", "`<init>`", symbol)
    for v in (s, s + ".", s + "().", s + ").", s + "#", re.sub(r"\($", "().", s), re.sub(r"\.$", "().", s)):
        if v == member:
            return True
    if "/" in s:
        return False
    short = re.sub(r"(?<!`)<init>(?!`)", "`<init>`", s)
    last = re.split(r"[.#]", short.rstrip(".#()").rstrip("(+0123456789)"))[-1].strip("`")
    owner = re.split(r"[.#]", short)[0] if re.search(r"[.#]", short.rstrip(".#()")) else None
    mm = re.search(r"([\w$]+)[.#]`?([^.#`(]+)`?(?:\(\+?\d*\))?[.#]?$", member)
    return bool(mm and mm.group(2) == last and (owner is None or mm.group(1) == owner))


def sentence_at(text, pos):
    """The sentence of `text` holding offset `pos` (sentences end at `. ` and `; `, as rule a's do)."""
    start = 0
    for m in re.finditer(r"(?:\. |; )", text):
        if m.start() >= pos:
            return text[start:m.start()]
        start = m.end()
    return text[start:]


def older_citation(line):
    """A line citing an older list's path (`out/closure/versions/.../landing-2.txt @ ...`), a record of the round
    that read it, outside the rows (`citations()`'s `records`)."""
    return any(m.group(1) and not m.group(1).endswith("closure/lists/") and m.group(1) != "lists/"
               for m in CITATION.finditer(line))


def rule_a_taken(destination, defs, kinds, parents, cut_names, ledger_names):
    """Rule a's sentence test over a destination cell: each sentence that does not name the contract, the ledger or
    the plumbing, and each backticked name in it a ledger-77 cut covers (a bare name only where every val or var of
    that name is cut, a qualified one where it resolves to a cut val or var)."""
    taken = []
    for sentence in re.split(r"(?:\. |; )", destination):
        if re.search(r"contract|ledger[\s-]*77|plumbing", sentence, re.I):
            continue
        for name in re.findall(r"`([^`]*)`", sentence):
            if "#" in name or "." in name:
                symbol = qualified_cut_member(name, defs, kinds, parents, cut_names)
                if symbol:
                    taken.append((name, symbol))
            elif name in ledger_names:
                taken.append((name, ledger_names[name]))
    return taken


def rule_d_prescriptions(text, cut_fields, unambiguous_names):
    """The ledger-77 cut fields a row's destination cell or a note prescribes as a destination: every `+` destination
    ending in a cut field's snake-case name, unless the sentence naming it says the destination is withdrawn, retired,
    removed, no field or not a field. A name counts where every val, var or def SemanticDB gives with it is cut (it
    means that field wherever it stands), or where the destination qualifies it by the cut member's owner class, in
    the class's spelling or its snake case (`+TypeVar.owning_state`, `+typer_state::previous`)."""
    out = []
    for m in DESTINATION_NAME.finditer(text):
        parts = re.split(r"::|[\.]", m.group(1))
        name, qualifier = parts[-1], parts[-2] if len(parts) > 1 else None
        candidates = cut_fields.get(name)
        if not candidates or WITHDRAWN.search(sentence_at(text, m.start())):
            continue
        for member in candidates:
            simple = simple_name(member)
            owner = class_name(owner_of(member) or "")
            if simple in unambiguous_names or (qualifier is not None and qualifier in (owner, snake_case(owner))):
                out.append((member, "+" + m.group(1)))
                break
    return out


def row_rules(table):
    """The table's destination rows against the five rules (the owner's ruling on round 13's landing-2 pass, 1, and
    its stride samples, class 1, and on round 14's class 4): (a) a destination sentence that does not name the
    contract must not take a ledger-77 cut member's val or var; (b) a row whose destination says `Retires: none` must
    have a teq arm beginning `none` and its own function -- the first `+snake_name` written in bold in the destination
    cell, or with none in bold the first `+snake_name` of the cell -- must not be an existing `fn` under `src/`, unless
    one of its members is an older port; (c) each member's port line must name the row's destination: a `+name` when the
    cell has `+name`, an `older` destination when the cell has its path or its function's name; (d) no row's
    destination cell and no note may prescribe a ledger-77 cut field: a `+` destination ending in the snake-case name of
    a val or var the cut covers, unless the sentence naming it says the destination is withdrawn, retired, removed, no
    field or not a field, or the line is a record citing an older list. The name counts where every val, var or def
    SemanticDB gives with it is cut (it means that field wherever it stands: `owningState`, `myUninstVars`,
    `implicitScopeCache`), or where the destination qualifies it by the cut member's owner class, in its spelling or
    snake case (`+TypeVar.owning_state`, `+typer_state::previous`); a generic name (`name`, `info`, `hash`, `size`)
    counts only with that qualification. The helpers the destination calls (`+bounds`, `+underlying`) are not its own
    function and do not fail it. (e) a destination row that withdraws a member ports.txt ports fails: the row and the
    closure say the same thing (finding 53.1). A row withdraws every member its landing cell cites when its
    destination cell, after an optional leading `**`, begins `none` or `no destination` (case-insensitive), or holds
    the words `no port line` in its opening label; it withdraws one member when the cell holds an `unport <symbol>`
    directive naming it.
    A withdrawn member with a `port` line fails unless the port is `older`; a member with no port line never fails.
    For (a), a bare backticked name counts only where every
    val or var symbol SemanticDB gives with that simple name is under a ledger-77 cut (the name can then mean nothing
    but a cut field: `myHash`, `sames`, `lastDenotation`, `owningState`), never where a non-cut val or var has it too
    (`info`, `tree`, `name`, `symbol`, `next`, `None`, `Type`); a qualified backticked name (`Class#name`,
    `Class.name`, a class's simple name or a SemanticDB-style path) counts where it resolves to a val or var under a
    cut, a class's inherited member through SemanticDB's parents. A destination row without six cells fails too (a
    bare bar in a cell splits it; the table writes a cell's `|` as `\\|`). Rule a's sentence test is reported over the
    fifth cell of every six-cell row of any id as `rule a+` (the owner reads the count first, round 14's class 4): it
    does not fail the run yet. Returns (rule-a rows, rule-b rows, rule-c rows, rule-e rows, rows not six cells, printed
    lines, (rule-a+ lines, their count by section), lines breaking rule d), each line the failing row's."""
    g = load(work_dir())
    defs, kinds, props, parents = g[0], g[1], g[2], g[3]
    _, _, _, cuts = read_entries(os.path.join(HERE, "entries.txt"))
    ledger77 = {}
    for ls in cuts.values():
        ledger77.update({s: why for s, why in ls.items() if why.startswith("plumbing(ledger-77)")})
    # The val or var symbols each simple name has, and those of them the ledger-77 cuts cover; SemanticDB's VAL/VAR
    # properties tell a val or var from a method (a method's kind is METHOD too). A name keeps its least symbol, for a
    # stable line.
    all_names, cut_names = collections.defaultdict(list), collections.defaultdict(list)
    for s in sorted(defs):
        if {"VAL", "VAR"} & set(semanticdb.properties_names(props.get(s, 0))):
            all_names[simple_name(s)].append(s)
            if cut_of(s, ledger77):
                cut_names[simple_name(s)].append(s)
    ledger_names = {n: ss[0] for n, ss in all_names.items() if len(ss) == len(cut_names.get(n, ()))}
    # Every val, var or def SemanticDB gives under a simple name (a method's kind is METHOD; a val's or var's VAL/VAR
    # flags), and those names every member of which is under a ledger-77 cut: rule d's "the name means the cut member
    # wherever it stands" (`owningState`, `myUninstVars`, `implicitScopeCache`). Rule a's names are val-or-var only.
    all_members = collections.defaultdict(list)
    for s in sorted(defs):
        if kinds.get(s) in ("METHOD", "FIELD") or {"VAL", "VAR"} & set(semanticdb.properties_names(props.get(s, 0))):
            all_members[simple_name(s)].append(s)
    unambiguous_names = {name for name, ss in all_members.items() if ss and all(cut_of(s, ledger77) for s in ss)}
    # The ledger-77 cut fields' snake-case names: each `+` destination ending in one is a candidate, the member chosen
    # by rule d's test (the name's every member cut, or the destination's owner class).
    cut_fields = collections.defaultdict(list)
    for s in ledger77:
        name = simple_name(s)
        if not name or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
            continue  # a backticked or operator name has no field name (`<init>`, `ownedVars_=`, `+`)
        if {"VAL", "VAR"} & set(semanticdb.properties_names(props.get(s, 0))):
            cut_fields[snake_case(name)].append(s)
    cut_fields = {name: sorted(ss) for name, ss in cut_fields.items()}
    older = {s for s, disp in port_disposition.items() if disp == "older"}
    fns = existing_teq_functions(os.getcwd())
    lines, arows, brows, crows, erows, badrows = [], [], [], [], [], []
    aplus, aplus_sections, dlines = [], collections.Counter(), set()
    section = "(no section)"
    for n, line in enumerate(open(table, encoding="utf-8"), 1):
        if line.startswith("## "):
            section = line[3:].strip()
        cells = row_cells(line) if line.startswith("| ") else None
        # Rule a+ (report only): rule a's sentence test over every six-cell row's fifth cell, whatever the row's id.
        if cells is not None and len(cells) == 8:
            for name, symbol in list(dict.fromkeys(rule_a_taken(cells[5], defs, kinds, parents, cut_names,
                                                                  ledger_names))):
                lines.append(f"table:{n} {cells[1]}: rule a+: the destination takes ledger 77's {name} ({symbol}) as "
                             f"its argument or state")
                aplus.append(n)
                aplus_sections[section] += 1
        # Rule d: a row's destination cell or a note that prescribes a ledger-77 cut field as a destination. A
        # record of an older round (a citation of an older list's path) is not one of the rows.
        if (cells is not None or line.lstrip().startswith("- ")) and not older_citation(line):
            text = cells[5] if cells is not None and len(cells) == 8 else line
            for member, destination_name in list(dict.fromkeys(rule_d_prescriptions(text, cut_fields,
                                                                                     unambiguous_names))):
                lines.append(f"table:{n} rule d: {member} is ledger 77's cut, and the line prescribes "
                             f"{destination_name}")
                dlines.add(n)
        if not DESTINATION_ROW.match(line):
            continue
        if len(cells) != 8:  # six cells between the row's edges
            number = cells[1] if len(cells) > 1 else ""
            badrows.append((n, number))
            lines.append(f"table:{n} {number}: not six cells ({len(cells) - 2}): a bar in a cell is written \\|")
            continue
        number, teq_arm, destination, landing = cells[1], cells[4], cells[5], cells[6].replace("\\|", "|")
        # Rule a: each sentence that does not name the contract, the ledger or the plumbing; a bare backticked name
        # every val or var of that name is cut for, or a qualified one resolving to a cut val or var, takes that
        # member as the destination's argument or state.
        taken = rule_a_taken(destination, defs, kinds, parents, cut_names, ledger_names)
        if taken:
            arows.append((n, number))
        for name, symbol in list(dict.fromkeys(taken)):
            lines.append(f"table:{n} {number}: rule a: the destination takes ledger 77's {name} ({symbol}) as its "
                         f"argument or state")
        # Rule b: `Retires: none` with a teq arm that does not begin `none`, or a destination function that is an
        # existing fn.
        if re.search(r"retires:\s*none", destination, re.I):
            arm = teq_arm.strip()
            for _ in range(4):
                before = arm
                arm = re.sub(r"^\*\*\s*", "", arm)
                arm = re.sub(r"^teq today:\s*", "", arm, flags=re.I)
                if arm == before:
                    break
            arm = re.sub(r"^\*\*", "", arm).strip()
            found = []
            if not arm.lower().startswith("none"):
                found.append("teq arm not none")
            name = row_destination(destination)
            existing = [f"fn {name}"] if name in fns else []
            if existing:
                found.append(", ".join(existing))
            if found:
                members = []
                for m in CITATION.finditer(landing):
                    members.extend(citation_candidates(m.group(3)))
                if not any(cut_of(m, older) for m in members if m):
                    brows.append((n, number))
                    lines.append(f'table:{n} {number}: rule b: an existing teq function ({found[-1]}) and "Retires: none", '
                                 f"the disposition not older")
        # Rule c: each member's port line names the row's destination. A `+name` is the cell's text; an older
        # destination is its path or its function's name.
        failed = False
        seen = set()
        for m in CITATION.finditer(landing):
            for member in citation_candidates(m.group(3)):
                port, dest = None, None
                for ps in ports.values():
                    port = cut_of(member, ps)
                    if port:
                        dest = ps[port]
                        break
                if not port:
                    continue
                if dest.startswith("+"):
                    ok = dest in destination
                else:
                    path, _, name = dest.partition(":")
                    ok = path in destination or name.split("::")[-1] in destination
                if ok or (port, dest) in seen:
                    continue
                seen.add((port, dest))
                failed = True
                lines.append(f"table:{n} {number}: rule c: ports.txt names {dest} for {port}, the row does not")
        if failed:
            crows.append((n, number))
        # Rule e: a row that withdraws a member ports.txt ports (finding 53.1): the row and the closure say the same
        # thing. A cell beginning `none` or `no destination`, or whose opening label holds `no port line`, withdraws
        # every member its landing cell cites; an `unport <symbol>` directive withdraws the member it names. A port line
        # that is `older` is the one existing copy, not a second one, and a member with no port line never fails.
        head = re.sub(r"^\*\*\s*", "", destination.lstrip())
        label = re.match(r"(.+?)\*\*", head)
        withdraws_all = bool(re.match(r"(?:none|no destination|no port line)\b", head, re.I)
                             or (label and re.search(r"\bno port line\b", label.group(1), re.I)))
        unports = unport_directives(destination)
        failed = False
        seen = set()
        for m in CITATION.finditer(landing):
            for member in citation_candidates(m.group(3)):
                if not (withdraws_all or any(unport_names(u, member) for u in unports)):
                    continue
                port, dest = None, None
                for ps in ports.values():
                    port = cut_of(member, ps)
                    if port:
                        dest = ps[port]
                        break
                if not port or port_disposition.get(port) == "older" or (port, dest) in seen:
                    continue
                seen.add((port, dest))
                failed = True
                lines.append(f"table:{n} {number}: rule e: the row withdraws {port}, and ports.txt ports it to {dest}")
        if failed:
            erows.append((n, number))
    return arows, brows, crows, erows, badrows, lines, (len(aplus), aplus_sections), len(dlines)


def qualified_cut_member(name, defs, kinds, parents, cut_names):
    """The val or var under a ledger-77 cut a qualified backticked name resolves to (`Class#name`, `Class.name`, the
    class's simple name or a SemanticDB-style path), None where it names none. The name's owner is the class's path
    or simple name, and a member the class inherits (`CachedTermRef#myHash` for CachedProxyType's `myHash`) resolves
    through SemanticDB's parents."""
    q = re.sub(r"\([^)]*\)$", "", name.strip()).rstrip(".")
    if "#" in q:
        i = q.rfind("#")
    elif "." in q:
        i = q.rfind(".")
    else:
        return None
    owner, member = q[:i].rstrip("#."), q[i + 1:]
    if not owner or not member:
        return None
    # The classes the owner names, by their path or their simple name: the ancestors an inherited member comes from.
    classes = {c for c in defs if kinds.get(c) == "CLASS"
               and (c.rstrip("#.") == owner or class_name(c) == class_name(owner))}
    for s in cut_names.get(member, ()):
        path = (owner_of(s) or "").rstrip("#.")
        if path == owner or class_name(path) == class_name(owner) or path.endswith(("#", ".", "/") + (owner,)):
            return s
        if any(owner_of(s) in ancestors(c, parents) for c in classes):
            return s
    return None


def citations(table, lists):
    """Every `landing-<N>.txt @ <symbol or prefix*>` of the table resolved against lists/landing-<N>.txt: a symbol
    any section of the list names (a member, a candidate, a cut entry, a declined edge's caller or callee, a reader, a
    boundary symbol), a prefix one at least. A citation by line fails (the ruling on round 12, 2). An older list's
    citation (a path prefix other than `closure/lists/` or `lists/`) is a record of the round that read it outside the
    rows, and fails on a row: a row's evidence cites today's lists (the ruling on round 12's points, 6)."""
    names = {}
    for p in os.listdir(lists):
        m = re.match(r"^landing-(\w+)\.txt$", p)
        if not m:
            continue
        syms = set()
        for l in open(os.path.join(lists, p), encoding="utf-8"):
            f = l.split()
            if not f or f[0].startswith("#"):
                continue
            if f[0] == "decline":
                syms.update(f[1:3])
            elif f[0] in ("candidate", "cut", "reader", "boundary"):
                syms.add(f[1])
            else:
                syms.add(f[0])
        names[m.group(1)] = syms
    bad, n, by_line, records = [], 0, [], 0
    for k, line in enumerate(open(table, encoding="utf-8"), 1):
        for m in CITATION.finditer(line):
            prefix, lab, raw = m.group(1), m.group(2), m.group(3)
            if prefix and not prefix.endswith("closure/lists/") and prefix != "lists/":
                if line.startswith("|"):
                    bad.append(f"table:{k} {prefix}landing-{lab}.txt @ {raw}: a row's evidence cites today's lists")
                else:
                    records += 1  # an older list's, a record of the round that read it: outside the rows
                continue
            raw = raw.replace("\\`", "`").replace("\\|", "|")  # a backtick or a bar escaped inside a code span
            if raw.lstrip("`").startswith("<") or "/" not in raw:
                continue  # a placeholder or a word in prose (`landing-<N>.txt @ <symbol>`, `@ that`), no citation
            n += 1
            syms = names.get(lab)
            if syms is None:
                bad.append(f"table:{k} landing-{lab}.txt: no such list")
                continue
            # The citation's own text runs into the prose's punctuation (`().).`): the longest reading that names
            # a symbol of the list, the punctuation dropped from its end one character at a time.
            cand, ok = raw, False
            while cand:
                if cand.endswith("*") and any(s.startswith(cand[:-1]) for s in syms):
                    ok = True
                    break
                # A term's symbol with its final `.` read as the prose's punctuation, a constructor's name written
                # without SemanticDB's backticks (`#<init>()`): the same symbol.
                bare = re.sub(r"([#.])<init>\(", r"\1`<init>`(", cand)
                if cand in syms or cand + "." in syms or bare in syms or bare + "." in syms:
                    ok = True
                    break
                if cand[-1] not in ".,;:)]`'":
                    break
                cand = cand[:-1]
            if not ok:
                bad.append(f"table:{k} landing-{lab}.txt @ {raw}: no such symbol in the list")
        for m in LINE_CITATION.finditer(line):
            by_line.append(f"table:{k} {m.group(0)}: a citation by line")
    arows, brows, crows, erows, badrows, row_lines, aplus, dlines = row_rules(table)
    for b in bad + by_line + row_lines:
        print(b)
    print("closure.py --citations: rule a+ by section: "
          + (", ".join(f"{s} {k}" for s, k in aplus[1].items()) or "none"))
    print(f"closure.py --citations: {n} citations by symbol, {len(bad)} naming nothing in their list, {len(by_line)} by line"
          f" ({records} of older lists outside the rows, records of the rounds that read them),"
          f" {len(arows)} rows breaking rule a, {aplus[0]} rule a+ lines (report only),"
          f" {len(brows)} breaking rule b, {len(crows)} rows breaking rule c,"
          f" {dlines} lines breaking rule d, {len(erows)} rows breaking rule e, {len(badrows)} rows not six cells")
    return not bad and not by_line and not arows and not brows and not crows and not erows and not badrows and not dlines


if __name__ == "__main__":
    if sys.argv[1] == "--ready":
        # closure.py --ready <N> <work> <entries> <out> [<table>]: with the table, its rows' entries regenerated and
        # compared with entries-generated.txt, and its citations resolved, before the counts.
        if len(sys.argv) > 6:
            import entries as gen
            import tempfile
            g = load(sys.argv[3])
            read_entries(os.path.join(HERE, "entries.txt"))
            above = {None: set(splits), **{lab: set(v) for lab, v in landing_splits.items()}}
            ent, rdr, _, _ = gen.generate(sys.argv[6], g, set(SPANS), lambda u: SPANS.get(u, []), member_at, g[1], g[0],
                                          os.path.join(HERE, "lists"), above)
            tmp = tempfile.mktemp(suffix=".txt")
            gen.write(tmp, ent, rdr, sys.argv[6])
            if open(tmp, encoding="utf-8").read() != open(os.path.join(HERE, "entries-generated.txt"), encoding="utf-8").read():
                sys.exit(f"closure.py --ready {sys.argv[2]}: entries-generated.txt is not the table's rows' (closure.py --entries)")
            if not citations(sys.argv[6], os.path.join(HERE, "lists")):
                sys.exit(f"closure.py --ready {sys.argv[2]}: the table's citations of the lists do not all resolve")
        main(sys.argv[3], sys.argv[4], sys.argv[5], ready=sys.argv[2])
    elif sys.argv[1] == "--citations":
        sys.exit(0 if citations(sys.argv[2], sys.argv[3]) else 1)
    elif sys.argv[1] == "--entries":
        # The landings' entries from the table's rows (entries.py; the ruling on round 12, 2).
        import entries as gen
        g = load(sys.argv[3])
        lists = os.path.join(HERE, "lists")
        read_entries(os.path.join(HERE, "entries.txt"))  # its `above` and `split` lines: the members walked above a cut
        above = {None: set(splits), **{lab: set(v) for lab, v in landing_splits.items()}}
        ent, rdr, stats, moved = gen.generate(sys.argv[2], g, set(SPANS), lambda u: SPANS.get(u, []), member_at, g[1],
                                              g[0], lists, above)
        gen.write(sys.argv[4], ent, rdr, sys.argv[2])
        left = gen.unbacked(os.path.join(os.path.dirname(sys.argv[4]), "entries-unbacked.txt"), ent)
        print("closure.py --entries: unbacked entries left (entries-unbacked.txt): "
              + ", ".join(f"landing {l} {len(v)}" for l, v in left.items() if v) or "none")
        with open(sys.argv[4] + ".moved", "w", encoding="utf-8") as f:
            for n, s, lab, st in sorted(moved):
                f.write(f"table:{n} {s} placed at {lab}, landing:{st} in the lists\n")
        print(f"closure.py --entries: {len(moved)} placements earlier than the lists' status ({sys.argv[4]}.moved); "
              f"{stats['unplaced']} members of several-landing rows placed by no status, {stats['candidates']} "
              f"candidates kept listed")
        print(f"closure.py --entries: {stats['rows']} rows with a landing, {stats['by symbol']} citations by symbol and "
              f"{stats['by line']} lines resolved; " + ", ".join(f"landing {l} {len(ent[l])} entries, {len(rdr[l] - ent[l])} "
                                                                   f"readers" for l in gen.NAMES))
    else:
        main(sys.argv[1], sys.argv[2], sys.argv[3])
