#!/usr/bin/env python3
"""tests/support/null_guards.py <reference> <default>: the listings teq's lowering writes by its own null-check rule
(`teq nir --lower`: a lowered listing, or with --ll=<directory> a directory of LLVM IR modules) judged against those
it writes by Scala Native's rule (the same with --null-guards=scala-native), which tests/native-link.scala holds to
Scala Native's own bytes: the same, differing ONLY by added null checks, or otherwise (UNCLASSIFIED, with the first line
that differs). Of a directory, each module against the reference's module of its name, one that only one side wrote
being otherwise. Each listing's added checks are counted again by its guards alone (the comparison with null and the
branch on it, in the default listing less in the reference's): a count that differs from the classifier's is
otherwise too (UNRECONCILED). `--self-test` runs the planted records alone; every run starts with them.

The reference is the control, the default the head, in what follows. An added null check is
- a guard: the three lines `%a = ine[T] %v : T, null` / `if %a : bool then %b else %c` / `%b:` (NIR) or
  `%a = icmp ne ptr %v, null` / `br i1 %a, label %b, label %c` / `b:` (LLVM), inserted before a read of %v (one of
  the next three lines names it); in NIR its unwind, if any, goes where the first of those lines that has one goes
  (the instruction the guard was made for), a handler the control has;
- the null pointer's slow path its else label names (one the control has, or one only the head has): the thrower's call and
  `unreachable` (in LLVM inside a handler: the thrower's invoke, its normal successor's label and `unreachable`),
  unwinding where every guard that jumps to it unwinds (NIR; in LLVM where the line after the guard is a call or an
  invoke, each followed through the blocks that only pass the exception on, as each instruction of a `try` unwinds to
  a block of its own that jumps to the handler).
The null pointer's slow paths are judged apart from the other lines, as the lowering writes them: one path for each
handler a check unwinds to (none outside a `try`), its label made at the path's first use, the paths written together
by their labels, so in the order of their first use. So an added check may be the first use of a path the control
uses later and bring it forward: each control path is the head's path of its label under the renaming, line for line;
the head's other paths are reached by added guards alone; each listing's paths stand in one run, in the order of their
first use in that listing, the head's where the control's stand.
Every other line is the control's in order, with the locals and labels the lowering numbers renamed consistently; an
LLVM phi may name, for a block a guard split, its last part. LLVM's landing pads are compared through what they route
to: each pad of the one shape the code generator writes (`_N.landingpad:` ... `br label %T`, seven lines) is dropped,
an invoke's `unwind label %_N.landingpad` reads `unwind label %T`, and a phi's entries `[%_N, %_N.landingpad.succ]` of
those pads are dropped; a pad the head adds (a slow path's, or an added guard's comparison's, which nothing invokes)
is then no difference, and a pad routed elsewhere is one. Prints a line per listing that differs, then the totals;
exits 0 where no listing is otherwise, 1 where one is, 2 on a usage or a listing it cannot read."""
import itertools, os, re, sys, tempfile

LOCAL = re.compile(r"%_?\d+(?:\.\d+|\.landingpad(?:\.succ)?)?|(?<![\w.$\"])_\d+(?:\.\d+|\.landingpad(?:\.succ)?)(?=:)")
LABEL = re.compile(r"^(%?_?\d+(?:\.\d+|\.landingpad(?:\.succ)?)?)(?:\(.*\))?:$")
EXC = r"@\"T19java\.lang\.Throwable\""
NIR_G0 = re.compile(r"^\s*(?:\d+: )?(%\d+) = ine\[(.+?)\] (%\d+)(?: <[^>]*>)? : (.+?), null(?: unwind %\d+ : " + EXC + r" to (%\d+)\(%\d+ : " + EXC + r"\))?$")
NIR_G1 = re.compile(r"^\s*(?:\d+: )?if (%\d+) : bool then (%\d+) else (%\d+)$")
NIR_UNWIND = re.compile(r" unwind %\d+ : " + EXC + r" to (%\d+)\(")
LL_G0 = re.compile(r"^\s*(%_\d+) = icmp ne ptr (%_\d+), null$")
LL_G1 = re.compile(r"^\s*br i1 (%_\d+), label %(_\d+\.\d+), label %(_\d+\.\d+)$")
LL_UNWIND = re.compile(r"^\s*(?:%_\d+ = )?invoke .* unwind label %(_\d+\.\d+)$")
LL_CALL = re.compile(r"^\s*(?:%_\d+ = )?call ")
NIR_THROW = re.compile(r"^\s*(?:\d+: )?%\d+ = call\[\(@\"T34scala\.scalanative\.runtime\.package\$\"\) => nothing\] @\"M34scala\.scalanative\.runtime\.package\$D16throwNullPointernEO\" : ptr\(null\)(?: unwind (%\d+) : " + EXC + r" to (%\d+)\((%\d+) : " + EXC + r"\))?$")
LL_THROW = re.compile(r"^\s*call ptr @\"_SM34scala\.scalanative\.runtime\.package\$D16throwNullPointernEO\"\(ptr null\)$")
LL_INVOKE = re.compile(r"^\s*invoke ptr @\"_SM34scala\.scalanative\.runtime\.package\$D16throwNullPointernEO\"\(ptr null\) to label %(_\d+\.\d+) unwind label %(_\d+\.\d+)$")
PAD_LABEL = re.compile(r"^(_\d+)\.landingpad:$")
PAD = [re.compile(r"^\s*(%_\d+) = landingpad \{ ptr, i32 \} catch ptr null$"),
       re.compile(r"^\s*(%_\d+) = extractvalue \{ ptr, i32 \} (%_\d+), 0$"),
       re.compile(r"^\s*br label %(_\d+)\.landingpad\.succ$"),
       re.compile(r"^(_\d+)\.landingpad\.succ:$"),
       re.compile(r"^\s*(%_\d+) = call ptr @scalanative_catch\(ptr (%_\d+)\)$"),
       re.compile(r"^\s*br label %(_\d+\.\d+)$")]
UNWIND_PAD = re.compile(r"unwind label %(_\d+)\.landingpad(?![.\w])")
PHI = re.compile(r"^(\s*%_\d+ = phi [^\[]+ )(.*)$")
ENTRY = re.compile(r"\[([^\[\]]+?), (%_\d+(?:\.\d+|\.landingpad\.succ))\]")
UNKNOWN = object()

def defs(text):
    """The listing as (header, body lines): a definition with a body runs to its closing `}` at column 0."""
    out, cur = [], None
    for line in text.split("\n"):
        if cur is None:
            if line.endswith("{") and not line.startswith(" "):
                cur = (line, [])
            elif line:
                out.append((line, []))
        elif line == "}":
            out.append(cur); cur = None
        else:
            cur[1].append(line)
    if cur:
        out.append((cur[0] + " (its closing brace missing)", cur[1]))
    return out

def routed_pads(lines):
    """LLVM body lines with each landing pad dropped and named by the block it routes to (see the module's text)."""
    route, kept, k = {}, [], 0
    while k < len(lines):
        m = PAD_LABEL.match(lines[k])
        if m and k + 7 <= len(lines):
            n = m.group(1)
            ms = [p.match(lines[k + 1 + x]) for x, p in enumerate(PAD)]
            if all(ms) and ms[1].group(2) == ms[0].group(1) and ms[2].group(1) == n and ms[3].group(1) == n \
                    and ms[4].group(1) == "%" + n and ms[4].group(2) == ms[1].group(1):
                route[n] = ms[5].group(1); k += 7; continue
        kept.append(lines[k]); k += 1
    def reroute(line):
        line = UNWIND_PAD.sub(lambda m: f"unwind label %{route[m.group(1)]}" if m.group(1) in route else m.group(0), line)
        m = PHI.match(line)
        if m:
            es = ENTRY.findall(m.group(2))
            if ", ".join(f"[{v}, {l}]" for v, l in es) == m.group(2):
                line = m.group(1) + ", ".join(f"[{v}, {l}]" for v, l in es if not (v[1:] in route and l == v + ".landingpad.succ"))
        return line
    return [reroute(l) for l in kept]

def toks(line):
    parts, last = [], 0
    for m in LOCAL.finditer(line):
        parts.append(("t", line[last:m.start()])); parts.append(("n", m.group(0).lstrip("%"))); last = m.end()
    parts.append(("t", line[last:]))
    return parts

def match(lc, lh, fwd, inv, alias):
    """The renaming that makes control line lc head line lh, given the names bound so far; None where none does. In
    an LLVM phi, a block a guard split may be named by its last part, `alias`."""
    a, b = toks(lc), toks(lh)
    if len(a) != len(b):
        return None
    is_phi, new = " = phi " in lc, {}
    for (ka, va), (kb, vb) in zip(a, b):
        if ka != kb:
            return None
        if ka == "t":
            if va != vb:
                return None
        else:
            want = fwd.get(va, new.get(va))
            if want is None:
                if vb in inv or vb in new.values():
                    return None
                new[va] = vb
            elif want != vb and not (is_phi and alias.get(va) == vb):
                return None
    return new

def guard(lines, j):
    """The guard starting at head line j: (else label, its comparison's local, its continuation label, the handler it
    unwinds to: None, a label, or UNKNOWN in LLVM where the line after it neither calls nor invokes), or None."""
    if j + 2 >= len(lines):
        return None
    for g0, g1 in ((NIR_G0, NIR_G1), (LL_G0, LL_G1)):
        m0 = g0.match(lines[j]); m1 = g1.match(lines[j + 1])
        if not (m0 and m1 and m1.group(1) == m0.group(1) and lines[j + 2].strip() in (m1.group(2) + ":", m1.group(2).lstrip("%") + ":")):
            continue
        value = m0.group(3) if g0 is NIR_G0 else m0.group(2)
        after = lines[j + 3:j + 6]
        if not any(re.search(re.escape(value) + r"(?![\d.])", l) for l in after):
            return None
        if g0 is NIR_G0:
            handler = m0.group(5)
            u = next((NIR_UNWIND.search(l) for l in after if NIR_UNWIND.search(l)), None)
            if u and u.group(1) != handler:
                return None
            handler = handler and handler.lstrip("%")
        else:
            nxt = lines[j + 3] if j + 3 < len(lines) else ""
            u = LL_UNWIND.match(nxt)
            handler = u.group(1) if u else None if LL_CALL.match(nxt) else UNKNOWN
        return m1.group(3).lstrip("%"), m0.group(1).lstrip("%"), m1.group(2).lstrip("%"), handler
    return None

def slow_path(lines, k):
    """The null pointer's slow path block at head line k: (its label, its length, the handler it unwinds to or None,
    its fresh name), or None."""
    if not lines[k].endswith(":") or k + 2 >= len(lines):
        return None
    lab = lines[k].strip().rstrip(":").split("(")[0].lstrip("%")
    m = NIR_THROW.match(lines[k + 1])
    if m and lines[k + 2].strip() == "unreachable":
        if m.group(1) and m.group(1) != m.group(3):
            return None
        return lab, 3, m.group(2) and m.group(2).lstrip("%"), (m.group(1) or "").lstrip("%")
    if LL_THROW.match(lines[k + 1]) and lines[k + 2].strip() == "unreachable":
        return lab, 3, None, ""
    m = LL_INVOKE.match(lines[k + 1])
    if m and k + 3 < len(lines) and lines[k + 2] == m.group(1) + ":" and lines[k + 3].strip() == "unreachable":
        return lab, 4, m.group(2), m.group(1)
    return None

BR = re.compile(r"^\s*br label %(_\d+\.\d+)$")

def passing_on(lines):
    """A handler label of LLVM followed through the blocks that are phis and a branch alone: a function of labels."""
    starts = {m.group(1).lstrip("%"): k for k, l in enumerate(lines) if (m := LABEL.match(l.strip()))}
    def follow(lab):
        for _ in range(16):
            k = starts.get(lab)
            if k is None:
                return lab
            body = []
            for l in lines[k + 1:]:
                if LABEL.match(l.strip()):
                    break
                body.append(l)
            m = BR.match(body[-1]) if body else None
            if not m or not all(" = phi " in l for l in body[:-1]):
                return lab
            lab = m.group(1)
        return lab
    return follow

def null_paths(lines):
    """The null pointer's slow paths of a body, in their order: (label, handler, length, its first line's index); the
    body's lines without them; the index in those lines the paths stand before, or None where they are not one run
    (the lowering writes them together, before the other kinds' paths)."""
    paths, rest, runs, k = [], [], [], 0
    while k < len(lines):
        s = slow_path(lines, k)
        if s:
            if not paths or paths[-1][3] + paths[-1][2] != k:
                runs.append(len(rest))
            paths.append((s[0], s[2], s[1], k))
            k += s[1]
            continue
        rest.append(lines[k])
        k += 1
    return paths, rest, (runs[0] if len(runs) == 1 else None if runs else len(rest))

def first_use_order(lines, paths):
    """Whether the paths stand in the order the body's other lines first name them: the order the lowering emits
    them in (each path's label is made at its first use, and the paths are emitted by their labels)."""
    own = set()
    for label, _, length, at in paths:
        own.update(range(at, at + length))
    first = {}
    for k, line in enumerate(lines):
        if k in own:
            continue
        for kind, name in toks(line):
            if kind == "n" and name in {p[0] for p in paths} and name not in first:
                first[name] = k
    named = [p[0] for p in paths]
    return all(n in first for n in named) and sorted(named, key=lambda n: first[n]) == named

def judge(cl, hl):
    """(added guards, None) or (None, why) for one definition's body lines. The null pointer's slow paths are judged
    apart from the walk: the reference's each the default's of its label under the walk's renaming, unwinding where
    it does; the default's others only added guards reach; and each listing's in the order of their first use."""
    if any(PAD_LABEL.match(l) for l in cl + hl):
        cl, hl = routed_pads(cl), routed_pads(hl)
    follow = passing_on(hl)
    cpaths, crest, crun = null_paths(cl)
    hpaths, hrest, hrun = null_paths(hl)
    if crun is None or hrun is None:
        return None, "the null pointer's slow paths are not one run"
    fwd, inv, alias, i, j, added = {}, set(), {}, 0, 0, 0
    block, guards_of, handlers, paired = None, {}, set(), {}
    while i < len(crest) or j < len(hrest):
        if i < len(crest) and j < len(hrest):
            new = match(crest[i], hrest[j], fwd, inv, alias)
            if new is not None:
                paired[i] = j
                fwd.update(new); inv.update(new.values())
                m = LABEL.match(crest[i].strip())
                if m:
                    block = m.group(1).lstrip("%")
                i += 1; j += 1; continue
        if j < len(hrest):
            g = guard(hrest, j)
            if g:
                added += 1
                els, cmp, cont, handler = g
                guards_of.setdefault(els, []).append(handler)
                inv.update((cmp, cont))
                if handler not in (None, UNKNOWN):
                    handlers.add(handler)
                if block is not None:
                    alias[block] = cont
                j += 3; continue
        at_c = crest[i] if i < len(crest) else "(end)"; at_h = hrest[j] if j < len(hrest) else "(end)"
        return None, f"reference `{at_c.strip()[:90]}` against default `{at_h.strip()[:90]}`"
    unwinds = lambda h, s: h is UNKNOWN or h == s or (h and s and follow(h) == follow(s))
    hby = {p[0]: p for p in hpaths}
    for label, handler, length, at in cpaths:
        mine = hby.get(fwd.get(label))
        if not mine:
            return None, f"the reference's slow path {label} is none of the default's"
        for lc, lh in itertools.zip_longest(cl[at:at + length], hl[mine[3]:mine[3] + mine[2]], fillvalue=""):
            new = match(lc, lh, fwd, inv, alias)
            if new is None:
                return None, f"reference `{lc.strip()[:90]}` against default `{lh.strip()[:90]}` in a slow path"
            fwd.update(new); inv.update(new.values())
    mapped = {fwd.get(p[0]) for p in cpaths}
    for label, handler, length, at in hpaths:
        if label in mapped:
            continue
        names = {n for line in hl[at:at + length] for kind, n in toks(line) if kind == "n"} - {label} - ({handler} if handler else set())
        if names & inv:
            return None, f"the default's slow path {label} names {sorted(names & inv)[:2]}, which other lines name"
        inv.update(names)
        if handler:
            handlers.add(handler)
    if handlers - set(fwd.values()):
        return None, f"added checks unwind to {sorted(handlers - set(fwd.values()))[:3]}, labels the reference has not"
    for els, hs in guards_of.items():
        if els not in hby or not all(unwinds(h, hby[els][1]) for h in hs):
            return None, f"an added guard's else label {els} is not a slow path unwinding where the guard does"
    if cpaths and hrun != paired.get(crun, len(hrest)):
        return None, "the null pointer's slow paths stand elsewhere among the other lines than the reference's"
    if not first_use_order(cl, cpaths) or not first_use_order(hl, hpaths):
        return None, "the slow paths stand out of the order of their first use"
    return added, None

def classify(ctext, htext):
    if ctext == htext:
        return "same", 0, None
    cd, hd = defs(ctext), defs(htext)
    if [h for h, _ in cd] != [h for h, _ in hd]:
        a = [h for h, _ in cd]; b = [h for h, _ in hd]
        k = next((x for x in range(min(len(a), len(b))) if a[x] != b[x]), min(len(a), len(b)))
        return "UNCLASSIFIED", 0, f"the definitions differ at {k}: {(a[k] if k < len(a) else '(end)')[:80]} / {(b[k] if k < len(b) else '(end)')[:80]}"
    total = 0
    for (h, cl), (_, hl) in zip(cd, hd):
        if cl == hl:
            continue
        added, why = judge(cl, hl)
        if why:
            return "UNCLASSIFIED", total, f"in {h[:80]}: {why}"
        total += added
    return "guards added", total, None

def self_test():
    exc = '@"T19java.lang.Throwable"'
    nthrow = 'call[(@"T34scala.scalanative.runtime.package$") => nothing] @"M34scala.scalanative.runtime.package$D16throwNullPointernEO" : ptr(null)'
    lthrow = '@"_SM34scala.scalanative.runtime.package$D16throwNullPointernEO"(ptr null)'
    nir_c = ('def @"M1A" : (@"T1A") => int {\n%1(%2 : @"T1A"):\n  %3 = copy @"M1B" : ptr\n  %4 = call[(@"T1A") => int] %3 : ptr(%2 : @"T1A")\n'
             f'  %5 = ine[@"T1A"] %2 : @"T1A", null\n  if %5 : bool then %6 else %7\n%6:\n  ret %4 : int\n%7:\n  %8 = {nthrow}\n  unreachable\n}}\n')
    nir_h = ('def @"M1A" : (@"T1A") => int {\n%1(%2 : @"T1A"):\n  %9 = ine[@"T1A"] %2 : @"T1A", null\n  if %9 : bool then %10 else %7\n%10:\n'
             '  %3 = copy @"M1B" : ptr\n  %4 = call[(@"T1A") => int] %3 : ptr(%2 : @"T1A")\n'
             f'  %5 = ine[@"T1A"] %2 : @"T1A", null\n  if %5 : bool then %6 else %7\n%6:\n  ret %4 : int\n%7:\n  %8 = {nthrow}\n  unreachable\n}}\n')
    nir_cu = ('def @"M1A" : (@"T1A") => int {\n%1(%2 : @"T1A"):\n'
              f'  9: %3 = copy @"M1B" : ptr unwind %4 : {exc} to %5(%4 : {exc})\n'
              f'  9: %6 = call[(@"T1A") => int] %3 : ptr(%2 : @"T1A") unwind %7 : {exc} to %5(%7 : {exc})\n  ret %6 : int\n'
              f'%5(%8 : {exc}):\n  ret int 0\n}}\n')
    nir_hu = ('def @"M1A" : (@"T1A") => int {\n%1(%2 : @"T1A"):\n'
              f'  9: %9 = ine[@"T1A"] %2 : @"T1A", null unwind %10 : {exc} to %5(%10 : {exc})\n  if %9 : bool then %11 else %12\n%11:\n'
              f'  9: %3 = copy @"M1B" : ptr unwind %13 : {exc} to %5(%13 : {exc})\n'
              f'  9: %6 = call[(@"T1A") => int] %3 : ptr(%2 : @"T1A") unwind %14 : {exc} to %5(%14 : {exc})\n  ret %6 : int\n'
              f'%5(%8 : {exc}):\n  ret int 0\n%12:\n  9: %15 = {nthrow} unwind %16 : {exc} to %5(%16 : {exc})\n  unreachable\n}}\n')
    nir_cs = ('def @"M1A" : (@"T1A") => int {\n%1(%2 : @"T1A"):\n'
              f'  9: %9 = ine[@"T1A"] %2 : @"T1A", null unwind %10 : {exc} to %5(%10 : {exc})\n  if %9 : bool then %11 else %12\n%11:\n'
              f'  9: %6 = call[(@"T1A") => int] %3 : ptr(%2 : @"T1A") unwind %14 : {exc} to %5(%14 : {exc})\n  ret %6 : int\n'
              f'%5(%8 : {exc}):\n  %17 = load[int] %2 : ptr\n  ret %17 : int\n%12:\n  9: %15 = {nthrow} unwind %16 : {exc} to %5(%16 : {exc})\n  unreachable\n}}\n')
    nir_hs = nir_cs.replace(f'%5(%8 : {exc}):\n', f'%5(%8 : {exc}):\n  9: %20 = ine[@"T1A"] %2 : @"T1A", null unwind %30 : {exc} to %5(%30 : {exc})\n  if %20 : bool then %21 else %12\n%21:\n')
    ll_c = 'define i32 @"_SM1A"(ptr %_2) {\n_1.0:\n  %_4 = call i32 @"_SM1B"(ptr %_2)\n  ret i32 %_4\n}\n'
    ll_h = (f'define i32 @"_SM1A"(ptr %_2) {{\n_1.0:\n  %_5 = icmp ne ptr %_2, null\n  br i1 %_5, label %_6.0, label %_7.0\n_6.0:\n'
            f'  %_4 = call i32 @"_SM1B"(ptr %_2)\n  ret i32 %_4\n_7.0:\n  call ptr {lthrow}\n  unreachable\n}}\n')
    ll_cp = ('define i32 @"_SM1C"(ptr %_2, i1 %_3) {\n_1.0:\n  br i1 %_3, label %_4.0, label %_5.0\n_4.0:\n  %_6 = call i32 @"_SM1B"(ptr %_2)\n'
             '  br label %_5.0\n_5.0:\n  %_7 = phi i32 [%_6, %_4.0], [0, %_1.0]\n  ret i32 %_7\n}\n')
    ll_hp = ('define i32 @"_SM1C"(ptr %_2, i1 %_3) {\n_1.0:\n  br i1 %_3, label %_4.0, label %_5.0\n_4.0:\n  %_8 = icmp ne ptr %_2, null\n'
             '  br i1 %_8, label %_9.0, label %_10.0\n_9.0:\n  %_6 = call i32 @"_SM1B"(ptr %_2)\n  br label %_5.0\n_5.0:\n'
             f'  %_7 = phi i32 [%_6, %_9.0], [0, %_1.0]\n  ret i32 %_7\n_10.0:\n  call ptr {lthrow}\n  unreachable\n}}\n')
    pad = lambda n, a, b, h: (f'_{n}.landingpad:\n  %_{a} = landingpad {{ ptr, i32 }} catch ptr null\n  %_{b} = extractvalue {{ ptr, i32 }} %_{a}, 0\n'
                              f'  br label %_{n}.landingpad.succ\n_{n}.landingpad.succ:\n  %_{n} = call ptr @scalanative_catch(ptr %_{b})\n  br label %{h}\n')
    ll_hc = ('define i32 @"_SM1H"(ptr %_2) personality ptr @p {\n_1.0:\n  %_4 = invoke i32 @"_SM1B"(ptr %_2) to label %_1.1 unwind label %_9.landingpad\n'
             '_1.1:\n  ret i32 %_4\n_6.0:\n  %_30 = phi ptr [%_9, %_9.landingpad.succ]\n  ret i32 0\n_7.0:\n  ret i32 1\n' + pad(9, 10, 11, "_6.0") + '}\n')
    ll_hh = ('define i32 @"_SM1H"(ptr %_2) personality ptr @p {\n_1.0:\n  %_12 = icmp ne ptr %_2, null\n  br i1 %_12, label %_13.0, label %_14.0\n_13.0:\n'
             '  %_4 = invoke i32 @"_SM1B"(ptr %_2) to label %_13.1 unwind label %_9.landingpad\n_13.1:\n  ret i32 %_4\n'
             '_6.0:\n  %_30 = phi ptr [%_9, %_9.landingpad.succ], [%_20, %_20.landingpad.succ], [%_25, %_25.landingpad.succ]\n  ret i32 0\n_7.0:\n  ret i32 1\n'
             f'_14.0:\n  invoke ptr {lthrow} to label %_14.1 unwind label %_20.landingpad\n_14.1:\n  unreachable\n'
             + pad(9, 10, 11, "_6.0") + pad(20, 21, 22, "_6.0") + pad(25, 26, 27, "_6.0") + '}\n')
    order_c = ('def @"M1A" : (@"T1A", @"T1A") => int {\n%1(%2 : @"T1A", %3 : @"T1A"):\n  %40 = copy @"M1B" : ptr\n'
               '  %41 = call[(@"T1A") => int] %40 : ptr(%2 : @"T1A")\n  %42 = copy @"M1B" : ptr\n'
               f'  9: %10 = ine[@"T1A"] %3 : @"T1A", null unwind %11 : {exc} to %5(%11 : {exc})\n  if %10 : bool then %12 else %13\n%12:\n'
               f'  9: %14 = call[(@"T1A") => int] %40 : ptr(%3 : @"T1A") unwind %15 : {exc} to %5(%15 : {exc})\n'
               '  %16 = ine[@"T1A"] %2 : @"T1A", null\n  if %16 : bool then %17 else %18\n%17:\n  %19 = call[(@"T1A") => int] %40 : ptr(%2 : @"T1A")\n'
               f'  ret %19 : int\n%13:\n  9: %22 = {nthrow} unwind %23 : {exc} to %5(%23 : {exc})\n  unreachable\n%18:\n  %24 = {nthrow}\n  unreachable\n'
               f'%60:\n  %61 = call[(@"T34scala.scalanative.runtime.package$", ptr, ptr) => nothing] @"M34scala.scalanative.runtime.package$D14throwClassCastR_R_nEO" : ptr(null, %62 : ptr, %63 : ptr)\n  unreachable\n'
               f'%5(%8 : {exc}):\n  ret int 0\n}}\n')
    order_h = ('def @"M1A" : (@"T1A", @"T1A") => int {\n%1(%2 : @"T1A", %3 : @"T1A"):\n  %50 = ine[@"T1A"] %2 : @"T1A", null\n  if %50 : bool then %51 else %52\n%51:\n'
               '  %40 = copy @"M1B" : ptr\n  %41 = call[(@"T1A") => int] %40 : ptr(%2 : @"T1A")\n  %42 = copy @"M1B" : ptr\n'
               f'  9: %10 = ine[@"T1A"] %3 : @"T1A", null unwind %11 : {exc} to %5(%11 : {exc})\n  if %10 : bool then %12 else %53\n%12:\n'
               f'  9: %14 = call[(@"T1A") => int] %40 : ptr(%3 : @"T1A") unwind %15 : {exc} to %5(%15 : {exc})\n'
               '  %16 = ine[@"T1A"] %2 : @"T1A", null\n  if %16 : bool then %17 else %52\n%17:\n  %19 = call[(@"T1A") => int] %40 : ptr(%2 : @"T1A")\n'
               f'  ret %19 : int\n%52:\n  %24 = {nthrow}\n  unreachable\n%53:\n  9: %22 = {nthrow} unwind %23 : {exc} to %5(%23 : {exc})\n  unreachable\n'
               f'%60:\n  %61 = call[(@"T34scala.scalanative.runtime.package$", ptr, ptr) => nothing] @"M34scala.scalanative.runtime.package$D14throwClassCastR_R_nEO" : ptr(null, %62 : ptr, %63 : ptr)\n  unreachable\n'
               f'%5(%8 : {exc}):\n  ret int 0\n}}\n')
    swap = lambda t, a, b: t.replace(a + b, b + a)
    c13 = f'%13:\n  9: %22 = {nthrow} unwind %23 : {exc} to %5(%23 : {exc})\n  unreachable\n'
    c18 = f'%18:\n  %24 = {nthrow}\n  unreachable\n'
    c60 = f'%60:\n  %61 = call[(@"T34scala.scalanative.runtime.package$", ptr, ptr) => nothing] @"M34scala.scalanative.runtime.package$D14throwClassCastR_R_nEO" : ptr(null, %62 : ptr, %63 : ptr)\n  unreachable\n'
    planted = [
        ("identical", nir_c, nir_c, "same"),
        ("an added guard first using its handler's path, so moving it before another handler's", order_c, order_h, "guards added"),
        ("the null pointer's paths moved with no added guard first using one", order_c, swap(order_c, c13, c18), "UNCLASSIFIED"),
        ("a path of another kind moved before the null pointer's", order_c, order_c.replace(c13 + c18 + c60, c60 + c13 + c18), "UNCLASSIFIED"),
        ("a path of another kind moved among the null pointer's", order_c, order_c.replace(c13 + c18 + c60, c13 + c60 + c18), "UNCLASSIFIED"),
        ("a reference's null path dropped, its guard sent to another", order_c, order_c.replace("else %13", "else %18").replace(c13, ""), "UNCLASSIFIED"),
        ("the null pointer's added paths in two runs, the reference having none",
         order_c.replace(f'  9: %10 = ine[@"T1A"] %3 : @"T1A", null unwind %11 : {exc} to %5(%11 : {exc})\n  if %10 : bool then %12 else %13\n%12:\n', '')
                .replace('  %16 = ine[@"T1A"] %2 : @"T1A", null\n  if %16 : bool then %17 else %18\n%17:\n', '').replace(c13, "").replace(c18, ""),
         order_h.replace(f'%53:\n  9: %22 = {nthrow} unwind %23 : {exc} to %5(%23 : {exc})\n  unreachable\n' + c60, c60 + f'%53:\n  9: %22 = {nthrow} unwind %23 : {exc} to %5(%23 : {exc})\n  unreachable\n'),
         "UNCLASSIFIED"),
        ("an added null path naming a local another line names", nir_cu, nir_hu.replace("  9: %15 = ", "  9: %3 = "), "UNCLASSIFIED"),
        ("an extra null path no line names", order_c, order_c.replace(c18, c18 + f'%70:\n  %71 = {nthrow}\n  unreachable\n'), "UNCLASSIFIED"),
        ("an extra null path no line names, before the others", order_c, order_c.replace(c13, f'%70:\n  %71 = {nthrow}\n  unreachable\n' + c13), "UNCLASSIFIED"),
        ("a reference's null path unwinding elsewhere", order_c, order_h.replace(f"%53:\n  9: %22 = {nthrow} unwind %23 : {exc} to %5(%23", f"%53:\n  9: %22 = {nthrow} unwind %23 : {exc} to %1(%23"), "UNCLASSIFIED"),
        ("a NIR guard before a read", nir_c, nir_h, "guards added"),
        ("a NIR guard with its unwind and a slow path unwinding there, the handlers renumbered", nir_cu, nir_hu, "guards added"),
        ("an LLVM guard and its slow path", ll_c, ll_h, "guards added"),
        ("an LLVM guard splitting a block a phi names", ll_cp, ll_hp, "guards added"),
        ("an LLVM slow path within a handler, its landing pad and a dead one, both in the handler's phi", ll_hc, ll_hh, "guards added"),
        ("another instruction changed", nir_c, nir_h.replace("ret %4 : int", "ret %3 : int"), "UNCLASSIFIED"),
        ("a guard of a value not read after it", nir_c, nir_h.replace('%9 = ine[@"T1A"] %2', '%9 = ine[@"T1A"] %8'), "UNCLASSIFIED"),
        ("a line dropped", nir_c, nir_h.replace("  ret %4 : int\n", ""), "UNCLASSIFIED"),
        ("a definition added", nir_c, nir_c + 'var @"M1X" : int\n', "UNCLASSIFIED"),
        ("a slow path calling another thrower", ll_c, ll_h.replace("throwNullPointer", "throwClassCast"), "UNCLASSIFIED"),
        ("a NIR slow path calling another thrower", nir_cu, nir_hu.replace("throwNullPointer", "throwClassCast"), "UNCLASSIFIED"),
        ("a slow path with another instruction in it", ll_c, ll_h.replace("_7.0:\n", "_7.0:\n  %_9 = add i32 1, 2\n"), "UNCLASSIFIED"),
        ("a local renamed inconsistently", nir_c, nir_c.replace("ret %4", "ret %8"), "UNCLASSIFIED"),
        ("a control local named as a guard's comparison", nir_c, nir_h.replace("%8 = call", "%9 = call"), "UNCLASSIFIED"),
        ("a phi naming another block than the split's", ll_cp, ll_hp.replace("[%_6, %_9.0]", "[%_6, %_10.0]"), "UNCLASSIFIED"),
        ("a branch to a split block's continuation", ll_cp.replace("br label %_5.0", "br label %_4.0", 1), ll_hp.replace("br label %_5.0", "br label %_9.0", 1), "UNCLASSIFIED"),
        ("a NIR guard unwinding elsewhere than the instruction after it", nir_cu, nir_hu.replace(f"unwind %10 : {exc} to %5(%10", f"unwind %10 : {exc} to %1(%10"), "UNCLASSIFIED"),
        ("a NIR guard jumping to a block that is not a slow path", nir_c, nir_h.replace("if %9 : bool then %10 else %7", "if %9 : bool then %10 else %6"), "UNCLASSIFIED"),
        ("a NIR guard with no unwind before an instruction that has one", nir_cu, nir_hu.replace(f" unwind %10 : {exc} to %5(%10 : {exc})", ""), "UNCLASSIFIED"),
        ("a NIR guard in a handler jumping to the control's slow path of that handler", nir_cs, nir_hs, "guards added"),
        ("a NIR guard outside a handler jumping to the control's slow path of one", nir_cs, nir_hs.replace(f" unwind %30 : {exc} to %5(%30 : {exc})", ""), "UNCLASSIFIED"),
        ("a NIR slow path unwinding elsewhere than its guard", nir_cu, nir_hu.replace(f" unwind %16 : {exc} to %5(%16 : {exc})", ""), "UNCLASSIFIED"),
        ("a NIR guard and its slow path unwinding to a handler the control has not", nir_c,
         nir_h.replace(f'  %9 = ine[@"T1A"] %2 : @"T1A", null\n  if %9 : bool then %10 else %7\n', f'  %9 = ine[@"T1A"] %2 : @"T1A", null unwind %90 : {exc} to %17(%90 : {exc})\n  if %9 : bool then %10 else %11\n')
              .replace('%7:\n', f'%11:\n  %12 = {nthrow} unwind %91 : {exc} to %17(%91 : {exc})\n  unreachable\n%7:\n'), "UNCLASSIFIED"),
        ("a NIR slow path unwinding to a handler the control has not", nir_cu, nir_hu.replace(f"unwind %16 : {exc} to %5(%16", f"unwind %16 : {exc} to %17(%16"), "UNCLASSIFIED"),
        ("an LLVM slow path through another instruction's block passing on to the same handler", ll_hc.replace("_7.0:\n  ret i32 1", "_7.0:\n  %_31 = phi ptr \n  br label %_6.0"), ll_hh.replace("_7.0:\n  ret i32 1", "_7.0:\n  %_31 = phi ptr \n  br label %_6.0").replace(pad(20, 21, 22, "_6.0"), pad(20, 21, 22, "_7.0")), "guards added"),
        ("an LLVM slow path through a block passing on to another handler", ll_hc.replace("_7.0:\n  ret i32 1", "_7.0:\n  %_31 = phi ptr \n  br label %_8.0\n_8.0:\n  ret i32 1"), ll_hh.replace("_7.0:\n  ret i32 1", "_7.0:\n  %_31 = phi ptr \n  br label %_8.0\n_8.0:\n  ret i32 1").replace(pad(20, 21, 22, "_6.0"), pad(20, 21, 22, "_7.0")), "UNCLASSIFIED"),
        ("an LLVM slow path's landing pad routed elsewhere than the guarded call's", ll_hc, ll_hh.replace(pad(20, 21, 22, "_6.0"), pad(20, 21, 22, "_7.0")), "UNCLASSIFIED"),
        ("an LLVM slow path unwinding to a landing pad there is not", ll_hc, ll_hh.replace("unwind label %_20.landingpad", "unwind label %_21.landingpad"), "UNCLASSIFIED"),
        ("a landing pad of the control routed to another block", ll_hc, ll_hh.replace(pad(9, 10, 11, "_6.0"), pad(9, 10, 11, "_7.0")), "UNCLASSIFIED"),
        ("a phi naming a landing pad there is not", ll_hc, ll_hh.replace("[%_25, %_25.landingpad.succ]", "[%_40, %_40.landingpad.succ]"), "UNCLASSIFIED"),
        ("an LLVM slow path outside the handler the guarded call unwinds to", ll_hc, ll_hh.replace(f"  invoke ptr {lthrow} to label %_14.1 unwind label %_20.landingpad\n_14.1:\n", f"  call ptr {lthrow}\n"), "UNCLASSIFIED"),
        ("a landing pad renumbered", ll_c.replace("ret i32 %_4\n", "br label %_9.landingpad\n_9.landingpad:\n  ret i32 %_4\n"), ll_h.replace("ret i32 %_4\n", "br label %_12.landingpad\n_12.landingpad:\n  ret i32 %_4\n"), "guards added"),
        ("a landing pad renumbered inconsistently", ll_c.replace("ret i32 %_4\n", "br label %_9.landingpad\n_9.landingpad:\n  ret i32 %_4\n"), ll_h.replace("ret i32 %_4\n", "br label %_12.landingpad\n_13.landingpad:\n  ret i32 %_4\n"), "UNCLASSIFIED"),
    ]
    bad = []
    for what, c, h, want in planted:
        got, _, why = classify(c, h)
        if got != want:
            bad.append(f"{what}: {got} ({why}), wanted {want}")
    def miscounting(c, h):
        kind, added, why = classify(c, h)
        return kind, added + 1, why
    summary = lambda counts: (counts["equal"], counts["added"], counts["checks"], counts["otherwise"])
    compared = [
        ("two modules, one with a guard added", {"a.ll": ll_c, "b.ll": ll_cp}, {"a.ll": ll_h, "b.ll": ll_cp}, classify, (1, 1, 1, 0)),
        ("a module the reference alone wrote", {"a.ll": ll_c, "b.ll": ll_cp}, {"a.ll": ll_h}, classify, (0, 1, 1, 1)),
        ("a module the default rule alone wrote", {"a.ll": ll_c}, {"a.ll": ll_h, "b.ll": ll_cp}, classify, (0, 1, 1, 1)),
        ("no listing on either side", {}, {}, classify, (0, 0, 0, 1)),
        ("an empty default listing", {"listing": nir_c}, {"listing": ""}, classify, (0, 0, 0, 1)),
        ("a default listing cut short", {"listing": nir_c}, {"listing": nir_h[:-3]}, classify, (0, 0, 0, 1)),
        ("a classifier that counts one check more than the guards", {"listing": nir_c}, {"listing": nir_h}, miscounting, (0, 0, 0, 1)),
    ]
    for what, ref, dflt, classifier, want in compared:
        got = summary(compare(ref, dflt, classifier)[0])
        if got != want:
            bad.append(f"{what}: equal, added, checks, otherwise {got}, wanted {want}")
    with tempfile.TemporaryDirectory() as d:
        def put(name, text):
            path = os.path.join(d, name)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
            return path
        for name, text in (("ref/a.ll", ll_c), ("ref/b.ll", ll_cp), ("default/a.ll", ll_h), ("default/b.ll", ll_cp), ("bad/a.ll", ll_h), ("bad/b.ll", ll_hp + "x\n")):
            put(name, text)
        ref_file, default_file = put("ref.txt", nir_c), put("default.txt", nir_h)
        at = lambda name: os.path.join(d, name)
        runs = [
            ("two directories", [at("ref"), at("default")], 0, "null_guards: 2 listings: 1 equal, 1 by added null checks (1 checks), 0 otherwise"),
            ("two files", [ref_file, default_file], 0, "null_guards: 1 listings: 0 equal, 1 by added null checks (1 checks), 0 otherwise"),
            ("a module otherwise", [at("ref"), at("bad")], 1, "null_guards: 2 listings: 0 equal, 1 by added null checks (1 checks), 1 otherwise"),
            ("a listing that is not there", [ref_file, at("absent.txt")], 2, None),
            ("a directory against a file", [at("ref"), default_file], 2, None),
            ("one argument", [ref_file], 2, None),
        ]
        for what, argv, want_code, want_line in runs:
            lines = []
            code = run(argv, lines.append)
            if code != want_code or (want_line is not None and (not lines or lines[-1] != want_line)):
                bad.append(f"{what}: exit {code}, last line {lines[-1] if lines else '(none)'}; wanted exit {want_code}" + (f", {want_line}" if want_line else ""))
    return bad, len(planted) + len(compared) + len(runs)

NIR_GUARD = re.compile(r"^\s*(?:\d+: )?(%\d+) = ine\[.+\] %\d+(?: <[^>]*>)? : .+, null(?: unwind .*)?$")
NIR_IF = re.compile(r"^\s*(?:\d+: )?if (%\d+) : bool then %\d+ else %\d+$")
LL_GUARD = re.compile(r"^\s*(%_\d+) = icmp ne ptr %_\d+, null$")
LL_BR = re.compile(r"^\s*br i1 (%_\d+), label %_\d+\.\d+, label %_\d+\.\d+$")

def is_guard(line, after):
    for comparison, branch in ((NIR_GUARD, NIR_IF), (LL_GUARD, LL_BR)):
        m, n = comparison.match(line), branch.match(after)
        if m and n and m.group(1) == n.group(1):
            return True
    return False

def guards(text):
    """The listing's comparisons with null followed by a branch on their result, counted by those two lines alone."""
    lines = text.split("\n")
    return sum(1 for line, after in zip(lines, lines[1:]) if "null" in line and is_guard(line, after))

def compare(reference, default, classifier=classify):
    """Each listing of `default` (name -> text) against `reference`'s of its name: the counts, and a line for each
    listing that is not equal."""
    counts = {"equal": 0, "added": 0, "checks": 0, "otherwise": 0}
    lines = []
    if not reference and not default:
        counts["otherwise"] += 1
        lines.append("UNCLASSIFIED: no listing on either side")
    for name in sorted(set(reference) | set(default)):
        if name not in reference or name not in default:
            counts["otherwise"] += 1
            lines.append(f"UNCLASSIFIED {name}: written by the {'default rule' if name in default else 'reference'} alone")
            continue
        kind, added, why = classifier(reference[name], default[name])
        if kind == "UNCLASSIFIED":
            counts["otherwise"] += 1
            lines.append(f"UNCLASSIFIED {name}: {why}")
            continue
        counted = guards(default[name]) - guards(reference[name])
        if counted != added:
            counts["otherwise"] += 1
            lines.append(f"UNRECONCILED {name}: the classifier counts {added} added checks, the guards differ by {counted}")
        elif kind == "same":
            counts["equal"] += 1
        else:
            counts["added"] += 1
            counts["checks"] += added
            lines.append(f"added {added} {name}")
    return counts, lines

def read_listings(path):
    """A file as the one listing `listing`, a directory as its files by name."""
    def text(p):
        with open(p, encoding="utf-8", errors="surrogateescape") as f:
            return f.read()
    if os.path.isdir(path):
        return {n: text(os.path.join(path, n)) for n in sorted(os.listdir(path))}
    return {"listing": text(path)}

def run(argv, say):
    if len(argv) != 2 or os.path.isdir(argv[0]) != os.path.isdir(argv[1]):
        say("usage: tests/support/null_guards.py <reference> <default>, two files or two directories, or --self-test")
        return 2
    try:
        reference, default = read_listings(argv[0]), read_listings(argv[1])
    except OSError as e:
        say(f"null_guards: cannot read {e.filename}: {e.strerror}")
        return 2
    counts, lines = compare(reference, default)
    for line in lines:
        say(line)
    total = len(set(reference) | set(default))
    say(f"null_guards: {total} listings: {counts['equal']} equal, {counts['added']} by added null checks ({counts['checks']} checks), {counts['otherwise']} otherwise")
    return 1 if counts["otherwise"] else 0

if __name__ == "__main__":
    bad, n = self_test()
    if bad:
        print("null_guards self-test: FAILED")
        for b in bad:
            print("  " + b)
        sys.exit(1)
    print(f"null_guards self-test: {n} planted records judged as planted")
    if sys.argv[1:] != ["--self-test"]:
        sys.exit(run(sys.argv[1:], print))
