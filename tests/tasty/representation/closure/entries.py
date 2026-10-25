#!/usr/bin/env python3
"""The landings' entries from the table's rows (README.md, "The entries"; the ruling on round 12, 2): per landing,
every dotty member a row of that landing cites, an entry where it is in `core/` or in the landing's own mechanism
files, else a reader, generated rather than kept by hand, so that a row cannot be added without its entry.

  closure.py --entries <table> <work dir> <out file>

A row is a line of a table in the table's section 3 (the node/accessor table) or 16 (the mechanism table and its
rounds), its landings its last cell's lead (`2`, `2, 3`, `op`, `opaque-proxies`; a row with none is no landing's). A
row's own citations are its landing cell's citations by symbol where it has any (they place the members, each at the
landing of the list that holds it; the dotty column then names the row's callers and context), else its dotty
column's (the column its table's header names `dotty`; a block with no header line takes the last header with as
many columns): by dotty's source there, `<file>.scala:<lines>`
or an abbreviation the table defines (`T:4054–4067`, `TU:427`), each line resolved to the member whose definition's
extent holds it (the graph's own `member_at`); and by symbol in the landing cell, `landing-<N>.txt @ <symbol>` (a
list's member, candidate, cut entry or reader; a prefix citation `@ <prefix>*` names none). The other cells name the
row's callers, its teq code and its context, which the row does not place. The out file holds a block per landing, `landing <N> <name>` then `<symbol>` an entry or
`reader <symbol>` a reader, one a line, sorted; entries.txt includes it after its own lines.
"""
import collections
import re

# A landing's mechanism files beside core/ (README.md): a member cited elsewhere is a reader of the mechanism.
MECH = {"2": ["typer/ProtoTypes.scala", "typer/Inferencing.scala"],
        "3": ["typer/ProtoTypes.scala", "typer/Inferencing.scala"],
        "4": [],
        # The printers are above their cut at opaque-proxies for compiletime.codeOf's printed tree (round 11's ruling).
        "op": ["inlines/", "ast/TreeTypeMap.scala", "ast/Trees.scala", "ast/tpd.scala", "printing/"],
        "5": ["ast/TreeTypeMap.scala", "ast/Trees.scala", "ast/tpd.scala", "cc/", "sbt/ExtractAPI.scala"],
        # Scala.js's checks and closures on a JS type, ported at 6 (round 11's ruling, landing 6's rows).
        "6": ["transform/sjs/"], "7": []}
# A landing's mechanism members beside core/, by symbol prefix: the implicit scope's computation, ImplicitRunInfo's
# members (implicitScope, computeIScope with its locals liftToAnchors and collectCompanions, the anchors' helpers
# isAnchor and isExcluded), is landing 4's; the rest of Implicits.scala is read (the ruling on 2026-10-25).
MECH_MEMBERS = {"4": ["dotty/tools/dotc/typer/ImplicitRunInfo#"]}
NAMES = {"2": "foundation", "3": "binders", "4": "references", "op": "opaque-proxies", "5": "annotations",
         "6": "js-hierarchy", "7": "remainder"}
DOTC = "dotty/tools/dotc/"

SYMBOL = re.compile(r"(?:^|[\s(,;`])landing-(\w+)\.txt @ (\S+)")
TOKEN = re.compile(r"((?:[a-z0-9_]+/)*[A-Za-z0-9_]+\.scala)|(?<![\w`/.])([A-Z][A-Za-z0-9]{0,4}|tpd)(?=:\s*\d)")
NUMS = re.compile(r"(?<![\w.])(\d{1,5})(?:\s*[–-]\s*(\d{1,5}))?(?![\w.%])")
OTHER_ABBR = re.compile(r"(?<![\w`])([A-Z][A-Za-z0-9]{0,5}) = `?[\w./-]*\.(?:java|rs|js|sjs|py|md)`?")
ABBR = re.compile(r"`?([A-Za-z][A-Za-z0-9]{0,5})`?\s*(?:=|\()?\s*`?((?:[a-z0-9_]+/)*[A-Za-z0-9_]+\.scala)`?")
# The abbreviations the table uses without defining them in its text (its fold files did).
DEFAULT_ABBR = {"T": "core/Types.scala", "TC": "core/TypeComparer.scala", "TO": "core/TypeOps.scala",
                "TAp": "core/TypeApplications.scala", "Sub": "core/Substituters.scala", "U": "core/Uniques.scala",
                "SD": "core/SymDenotations.scala", "TP": "core/tasty/TreePickler.scala",
                "TU": "core/tasty/TreeUnpickler.scala", "TUp": "core/tasty/TreeUnpickler.scala",
                "Ann": "core/Annotations.scala", "CH": "core/ConstraintHandling.scala",
                "OC": "core/OrderingConstraint.scala", "TTM": "ast/TreeTypeMap.scala", "Den": "core/Denotations.scala",
                "Sy": "core/Symbols.scala", "Def": "core/Definitions.scala", "In": "inlines/Inliner.scala",
                "Ty": "typer/Typer.scala"}


def landings(cell):
    """A landing cell's lead: the landings before its first parenthesis or break (`2 (landing-2.txt @ ..)`)."""
    lead = re.split(r"<br>|\*\*(?!\s*$)(?=[A-Za-z])", re.sub(r"^\*\*([^*]+)\*\*", r"\1", cell.strip()))[0]
    s = re.sub(r"\(.*?\)", "", lead)
    s = re.sub(r"\bmoved from\b.*", "", s)
    out = sorted({x for x in re.findall(r"(?<![\w.:-])([2-7])(?![\w.:])", s)})
    if re.search(r"(?<![\w-])op(?![\w-])|opaque-proxies|opaque proxies", s):
        out.append("op")
    return out


def resolve(path, files):
    """A cited file to its SemanticDB uri (`compiler/src/dotty/tools/dotc/<path>`), or None."""
    p = path.strip("`()")
    for cand in (p, DOTC + p, "compiler/src/" + p, "compiler/src/" + DOTC + p):
        if cand in files:
            return cand
    # A file name two packages share (`TypeOps.scala`, core's and semanticdb's): core's, then the shortest path, in a
    # fixed order whatever the set's.
    tail = sorted((f for f in files if f.endswith("/" + p)), key=lambda f: ("/dotc/core/" not in f, DOTC not in f, len(f), f))
    return (tail or [None])[0]


ORDER = ["2", "3", "4", "op", "5", "6", "7"]
# The files a section-3 row's bare lines and names are read against, in order, where it names no file before them
# (`` `iso` 2938–2944 `` under the node table's heading for `Types.scala`): the file whose member at the lines the row
# names in code.
FALLBACK = ["core/Types.scala", "core/TypeComparer.scala", "core/TypeOps.scala", "core/Annotations.scala",
            "core/TypeApplications.scala", "ast/TreeTypeMap.scala", "core/Substituters.scala",
            "core/SymDenotations.scala", "core/tasty/TreePickler.scala", "core/tasty/TreeUnpickler.scala",
            "core/Uniques.scala", "core/ConstraintHandling.scala", "inlines/Inliner.scala", "core/Denotations.scala",
            "typer/Typer.scala"]
NAMED = FALLBACK + ["core/Symbols.scala", "core/Definitions.scala", "typer/Applications.scala", "typer/ProtoTypes.scala",
                    "typer/Inferencing.scala", "cc/CaptureOps.scala", "cc/CaptureSet.scala", "core/MatchTypes.scala",
                    "core/TypeErasure.scala", "typer/Implicits.scala", "typer/Namer.scala", "typer/Checking.scala"]


def short(sym):
    s = re.sub(r"\(\+?\d*\)\.?$", "", sym.rstrip(".#"))
    return re.split(r"[./#]", s)[-1].strip("`")


def statuses(lists):
    """The committed lists' statuses (`landing:<N>` of each member) and each landing's candidates that are not its
    members (listed, not walked)."""
    import os
    status, cands = {}, collections.defaultdict(set)
    members = collections.defaultdict(set)
    for lab in ORDER:
        p = os.path.join(lists, f"landing-{lab}.txt")
        if not os.path.exists(p):
            continue
        for l in open(p, encoding="utf-8"):
            f = l.split()
            if not f or f[0].startswith("#"):
                continue
            if f[0] == "candidate":
                cands[lab].add(f[1])
            elif f[0] not in ("cut", "reader", "boundary", "decline"):
                members[lab].add(f[0])
                if f[2].startswith("landing:"):
                    status.setdefault(f[0], f[2][len("landing:"):])
    return status, {lab: cands[lab] - members[lab] for lab in ORDER}


def generate(table, g, files, spans_of, member_at, kinds, defs, lists=None, above=None):
    T = open(table, encoding="utf-8").read().split("\n")
    above = above or {}
    fallback = [u for u in (resolve(f, files) for f in FALLBACK) if u]
    by_name = collections.defaultdict(list)  # a method's name -> its definitions in the files the table cites most
    named_files = {u for u in (resolve(f, files) for f in NAMED) if u}
    for sym, (uri, line) in defs.items():
        if uri in named_files and kinds.get(sym) == "METHOD":
            by_name[short(sym)].append(sym)
    status, cand_only = statuses(lists) if lists else ({}, collections.defaultdict(set))
    moved = []  # (table line, member, the landing the row places it at, its status in the committed lists)
    s3 = next(i for i, l in enumerate(T) if l.startswith("## 3. "))
    e3 = next(i for i, l in enumerate(T) if l.startswith("## 4. "))
    s16 = next(i for i, l in enumerate(T) if l.startswith("## 16. "))
    abbr = []  # (table line, abbreviation, uri or None), every definition the table writes
    for n, l in enumerate(T):
        if "Abbreviation" in l or re.search(r"`[A-Za-z]{1,5}` (?:core|typer|cc|ast|inlines|transform)/", l) \
                or re.search(r"\b[A-Z][A-Za-z]{0,4} = (?:core|typer|cc|ast|inlines|transform)/", l):
            for m in ABBR.finditer(l):
                u = resolve(m.group(2), files)
                if u:
                    abbr.append((n, m.group(1), u))
        # A definition naming another file (the JDK's `CP = .../regex/CharPredicates.java`) shadows a dotty one.
        for m in OTHER_ABBR.finditer(l):
            abbr.append((n, m.group(1), None))
    ent, rdr = collections.defaultdict(set), collections.defaultdict(set)
    stats = collections.Counter()
    headers, hdr = [], None
    def split(l):
        return [c.strip() for c in re.split(r"(?<!\\)\|", l.strip().strip("|"))]
    skip = False
    for i in list(range(s3, e3)) + list(range(s16, len(T))):
        l = T[i]
        if l.startswith("#"):
            # The destinations' tables (16.12.d and after) mirror the lists, a row a member's destination: they place
            # nothing, or every listed member would be its own entry and no decline could take one out.
            if l.startswith(("### ", "#### ")):
                skip = "(destinations)" in l
            continue
        if skip:
            continue
        if not l.startswith("|"):
            hdr = None
            continue
        if re.match(r"^\|[\s:|-]+\|?$", l):
            continue
        cells = split(l)
        if i + 1 < len(T) and re.match(r"^\|\s*:?-{3,}", T[i + 1]):
            hdr = cells
            headers.append(cells)
            continue  # a header
        here = hdr if hdr and len(hdr) == len(cells) else next((h for h in reversed(headers) if len(h) == len(cells)), None)
        lcols = [k for k, h in enumerate(here or []) if "landing" in h.lower() or "moved" in h.lower()]
        if not here or not lcols:
            continue
        lc = lcols[-1]
        labs = landings(cells[lc])
        if not labs:
            continue
        dcols = [k for k, h in enumerate(here) if "dotty" in h.lower() and k != lc]
        if not dcols:
            continue
        stats["rows"] += 1
        local = dict(DEFAULT_ABBR)
        for n, a, u in abbr:
            if n < i:
                local[a] = u
        cited, placed = set(), collections.defaultdict(set)
        # The landing cell's citations by symbol place each member at the landing of the list that holds it
        # (`4 (landing-4.txt @ FunProto#typedArgs)`); a candidate's is no placement (listed, not walked).
        for m in SYMBOL.finditer(cells[lc]):
            if re.search(r"candidate\s*`?[^`]*`?\s*$", cells[lc][max(0, m.start() - 60):m.start() + 1]):
                continue
            sym = m.group(2)
            while sym and sym not in defs and sym[-1] in ".,;:)]`'":
                sym = sym[:-1]
            if sym in defs and sym.startswith(DOTC) and m.group(1) in labs:
                placed[m.group(1)].add(sym)
                stats["by symbol"] += 1
        for c in [cells[k] for k in dcols] + [cells[lc]]:
            text = re.sub(r"landing-\w+\.txt @ \S+", " ", c)
            pos = list(TOKEN.finditer(text))
            names = set(re.findall(r"`([A-Za-z_][\w$]*)", text)) | set(re.findall(r"`[\w.#]*[.#]([A-Za-z_][\w$]*)`", text))
            head = re.sub(r"`[^`]*`", " ", text[:pos[0].start()] if pos else text)
            hn = [(int(x.group(1)), int(x.group(2) or x.group(1))) for x in NUMS.finditer(head)]
            if hn and names and c is not cells[lc]:
                # Lines before any file: the first fallback file whose members at them the row names.
                for u in fallback:
                    ms = {member_at(spans_of(u), a, kinds, defs) for a, _ in hn}
                    ms = {m_ for m_ in ms if m_ and short(m_) in names}
                    if ms:
                        cited |= ms
                        stats["by fallback"] += len(ms)
                        break
            if not pos and not hn and names and c is not cells[lc]:
                # Names alone: a method defined once across the files the table cites most.
                for n_ in names:
                    if len(by_name.get(n_, [])) == 1:
                        cited.add(by_name[n_][0])
                        stats["by name"] += 1
            for k, m in enumerate(pos):
                stop = pos[k + 1].start() if k + 1 < len(pos) else len(text)
                seg = text[m.end():stop]
                if not seg.lstrip().startswith(":"):
                    continue
                uri = resolve(m.group(1), files) if m.group(1) else local.get(m.group(2))
                uri = uri if uri is None or uri in files else resolve(uri, files)
                if not uri:
                    continue
                spans = spans_of(uri)
                # The numbers that follow the file, up to the first word that is not a number list.
                run = re.match(r"^:\s*((?:\d{1,5}(?:\s*[–-]\s*\d{1,5})?(?:,\s*|\s+and\s+)?)+)", seg)
                if not run:
                    continue
                for a, b in [(int(x.group(1)), int(x.group(2) or x.group(1))) for x in NUMS.finditer(run.group(1))]:
                    for line in ({a} if b < a or b - a > 60 else set(range(a, b + 1))):
                        s = member_at(spans, line, kinds, defs)
                        if s and s.startswith(DOTC):
                            cited.add(s)
                            stats["by line"] += 1
        # A row of one landing places every member its dotty column cites there; a row of several places the members
        # its landing cell places, the others of its dotty column at no landing (which of its landings is not said),
        # except in section 3, whose rows predate the lists and name their landings for the whole row.
        assign = collections.defaultdict(set)
        for lab in labs:
            assign[lab] |= placed[lab]
        if any(placed.values()):
            # The landing cell places its members by symbol: they are the row's; its dotty column's other lines are
            # its callers' and its context's (`GB7-3`: the callers at TC:3695, `necessarilyCompatible` placed).
            stats["context"] += len(cited - set().union(*placed.values()))
        elif len(labs) == 1:
            assign[labs[0]] |= cited
        else:
            # Several landings: each member at every one of them from its list status on (the landings that start
            # from a helper another one ports, `2, 4, 6` for a member landing 2 ports), never one before it; a member
            # in no list at each.
            for s_ in cited - set().union(*placed.values()):
                st = status.get(s_)
                # A member in no list is placed by a row of one landing alone (it says where); a row of several,
                # which of them it means unsaid, places it nowhere.
                later = [l for l in labs if st is not None and ORDER.index(l) >= ORDER.index(st)]
                for l in later:
                    assign[l].add(s_)
                if not later:
                    stats["unplaced"] += 1
        for lab in list(assign):
            # A member the landing's list holds as a dispatch candidate alone stays one (listed, not walked).
            gone = {s_ for s_ in assign[lab] if s_ in cand_only.get(lab, set())}
            stats["candidates"] += len(gone)
            assign[lab] -= gone
            for s_ in assign[lab]:
                st = status.get(s_)
                if st and ORDER.index(st) > ORDER.index(lab):
                    moved.append((i + 1, s_, lab, st))
        for lab, members in assign.items():
            for s in members:
                file = defs[s][0].removeprefix("compiler/src/" + DOTC)
                # A member an `above` or `split` line walks above its cut (entries.txt) is in scope where the line
                # holds: a row citing it makes it an entry, as a mechanism file's member.
                lifted = any(s == k or (k.endswith("*") and s.startswith(k[:-1])) for k in above.get(lab, ())) \
                    or any(s == k or (k.endswith("*") and s.startswith(k[:-1])) for k in above.get(None, ()))
                if file.startswith("core/") or lifted or any(file.startswith(m) for m in MECH.get(lab, [])) \
                        or any(s.startswith(m) for m in MECH_MEMBERS.get(lab, [])):
                    ent[lab].add(s)
                else:
                    rdr[lab].add(s)
    return ent, rdr, stats, moved


def unbacked(path, ent):
    """entries-unbacked.txt less the entries the rows now give: the hand-kept entries of before round 12 that no row
    places yet, each owed a row (`--ready` fails on a landing's while any remains)."""
    import os
    if not os.path.exists(path):
        return {}
    keep, cur, out, left = [], None, [], collections.defaultdict(list)
    for l in open(path, encoding="utf-8"):
        t = l.strip()
        m = re.match(r"^landing (\S+) ", t)
        if m:
            cur = m.group(1)
            out.append(l)
            continue
        if t and not t.startswith("#") and cur and t in ent.get(cur, set()):
            continue  # a row gives it now
        out.append(l)
        if t and not t.startswith("#") and cur:
            left[cur].append(t)
    open(path, "w", encoding="utf-8").write("".join(out))
    return left


def write(out, ent, rdr, table):
    with open(out, "w", encoding="utf-8") as f:
        f.write("# Generated by closure.py --entries from the type-representation table's rows (entries.py, README.md\n"
                "# \"The entries\"): per landing, every dotty member a row of that landing cites, an entry in core/ or\n"
                "# the landing's mechanism files, else a reader. Not edited by hand.\n")
        for lab in ["2", "3", "4", "op", "5", "6", "7"]:
            f.write(f"landing {lab} {NAMES[lab]}\n")
            f.write(f"# mechanism files beside core/: {', '.join(MECH[lab]) or 'none'}\n")
            f.write(f"# mechanism members beside core/: {', '.join(MECH_MEMBERS.get(lab, [])) or 'none'}\n")
            for s in sorted(ent[lab]):
                f.write(s + "\n")
            for s in sorted(rdr[lab] - ent[lab]):
                f.write("reader " + s + "\n")
