#!/usr/bin/env python3
"""The representation oracle's plumbing (tests/tasty/representation/README.md, check.sh):

  oracle.py split <oracle output> <dir>     each `== <name>` job's lines into <dir>/<name>.txt
  oracle.py gen <scalac cell> <use> <out>   the downstream for a cell scalac observed through use/<x>.scala (teq's,
                                            and scalac's own to check it): its imports, then a call per line of the
                                            cell by its key, an owner's names (`<spec>:members`, `Observe.names`), its own
                                            line (`<spec>:owner`, `Observe.owner`) or one member (`<spec>.<key>`,
                                            `Observe.member`)
  oracle.py teq <diagnostics> <out.scala>   the cell teq's check of that downstream gives: each call's `OBSERVE`
                                            warning, or for a call that failed `<key> <teq refused: message>`
  oracle.py strip <cell>                    the cell with the marks of scalac's bounds classes removed
  oracle.py pins <pins> <diff>...           every member a recorded diff names has a pin and every pin names one
  oracle.py coverage [--table] <api> <coverage> <cells> <src>...
                                            every kind dotty/Api.scala lists is an arm a fixture reaches from a
                                            root (the cells' `-kinds` records), every accessor printed by a
                                            reached arm or serializer or excluded with its kind
"""
import collections
import glob
import json
import os
import re
import sys


class InjectivityFailure(Exception):
    """Two distinct inputs a serializer the comparison reads covered gave one serialized value."""


def injectivity_fail(serializer, a, b, value):
    raise InjectivityFailure(f"{serializer}: {a} and {b} serialize to {value}")


def _injectivity_hook(exc_type, exc, tb):
    if issubclass(exc_type, InjectivityFailure):
        print(f"FAIL representation injectivity: {exc}", flush=True)
        os._exit(1)
    sys.__excepthook__(exc_type, exc, tb)


sys.excepthook = _injectivity_hook

MARK = re.compile(r"\{(RealTypeBounds|TypeAlias|MatchAlias)\}")
# A quoted span of observe.scala's `quoted`: `"`, then escapes (`\` and one character; `\uXXXX` is `\u` and four plain
# characters) or characters other than `"` and `\`, then `"`. Every string and every name that is not bare is one.
QUOTED = re.compile(r'("(?:\\.|[^"\\])*")')


def lines(path):
    """A file's lines split at `\n` alone (README.md, "Transport"): no newline translation on reading, and none of the
    other boundaries `str.splitlines` takes (`\r`, U+000B, U+000C, U+001C to U+001E, U+0085, U+2028, U+2029)."""
    with open(path, encoding="utf-8", newline="") as f:
        text = f.read()
    out = text.split("\n")
    return out[:-1] if out and out[-1] == "" else out


def split(out, dest):
    name, held = None, []

    def flush():
        if name is not None:
            with open(f"{dest}/{name}.txt", "w", encoding="utf-8", newline="") as f:
                f.write("".join(l + "\n" for l in held))

    for line in lines(out):
        if line.startswith("== "):
            flush()
            name, held = line[3:], []
        elif name is not None:
            held.append(line)
    flush()


def read_key(key):
    r"""An observation key split at the delimiters outside its quoted spans (observe.scala's `quoted`): the owner spec,
    the member's spelling as it is printed (quotes kept), the overload ordinal after the name, and whether the key is
    an owner record. A quoted span starts and ends at an unescaped `"`; its escapes are `\\`, `\"`, `\n`, `\r`, `\t`
    and `\uXXXX`. A name fragment without a spec is read as a member, which `base` uses."""
    spans, i = [], 0
    while i < len(key):
        if key[i] != '"':
            i += 1
            continue
        start = i
        i += 1
        while i < len(key):
            c = key[i]
            if c == "\\":
                i += 6 if i + 1 < len(key) and key[i + 1] == "u" else 2
            elif c == '"':
                i += 1
                break
            else:
                i += 1
        spans.append((start, i))

    def outside(p):
        return not any(a <= p < b for a, b in spans)

    if key.endswith(":owner") and outside(len(key) - len(":owner")):
        return key[: -len(":owner")], None, None, True
    dots = [i for i, c in enumerate(key) if c == "." and outside(i)]
    at = dots[-1] if dots else -1
    spec = key[:at] if dots else None
    rest = key[at + 1:]
    hashes = [i for i, c in enumerate(rest) if c == "#" and outside(at + 1 + i)]
    if hashes:
        h = hashes[-1]
        suffix = rest[h + 1:]
        if suffix.isdigit():
            return spec, rest[:h], int(suffix), False
    return spec, rest, None, False


def unquote(n):
    """A name as observe.scala's `quoted` writes it, read back as the dump's names are (`json.loads`): an escaped
    surrogate pair is one code point, a lone surrogate stays as it is, and a bare name stays as it is."""
    return json.loads(n) if n.startswith('"') else n


def key_selfcheck():
    """The fallback key reader's own check: the owner spec and the overload suffix are read outside quoted spans, and
    a quoted name's escapes decode as the dump's names do."""
    cases = [
        ('object:odd.Product."f#2"', ("object:odd.Product", '"f#2"', None, False)),
        ('object:odd.Product."f:owner"', ("object:odd.Product", '"f:owner"', None, False)),
        (r'object:odd.Product."f\ud83d\ude00"#2', ("object:odd.Product", r'"f\ud83d\ude00"', 2, False)),
        ('object:odd.Product."f\U0001F600"#2', ("object:odd.Product", '"f\U0001F600"', 2, False)),
        ("object:odd.Product:owner", ("object:odd.Product", None, None, True)),
    ]
    ok = True
    for key, want in cases:
        got = read_key(key)
        if got != want:
            print(f"FAIL representation key self-check: {key} reads as {got}, not {want}")
            ok = False
    if unquote(r'"f\ud83d\ude00"') != "f\U0001F600" or unquote(r'"\ud800"') != "\ud800" or unquote("f") != "f":
        print("FAIL representation key self-check: quoted names do not decode as the dump's names do")
        ok = False
    if ok:
        print("ok representation key self-check: the owner spec and the overload suffix are read outside quoted spans,"
              " and quoted names decode as the dump's names do")
    return ok


def scala_literal(s):
    """A Scala string literal of `s` in printable ASCII: `\\` and `"` escaped, every other character outside it `\\uXXXX`."""
    return '"' + "".join("\\" + c if c in '\\"' else (c if " " <= c < "\x7f" else f"\\u{ord(c):04x}") for c in s) + '"'


# An owner's spec, `object:<path>` or `class:<path>`, its path identifiers in ASCII joined by `.`: no spec holds `:`
# past its tag and a member's key in its owner no `.` or `:` unquoted, so no record's key is another's.
SPEC = re.compile(r"^((?:object|class):[A-Za-z_$][A-Za-z0-9_$]*(?:\.[A-Za-z_$][A-Za-z0-9_$]*)*):members(?: |$)")


def calls(cell):
    """The calls of the downstream for a scalac cell, in its order: (key, Scala expression). Each line's key names its
    record and its owner (observe.scala's `quoted`): an owner's names `<spec>:members`, its own line `<spec>:owner`,
    a member's `<spec>.<key in the owner>`, the member's key as the line prints it (a quoted name quoted, an
    overload's `#<n>` kept), which `member` matches. A line no key of the cell's owners names, or a key twice, fails
    the generation."""
    specs = []
    out = []
    for n, line in enumerate(lines(cell), start=1):
        if line.split(" ")[0].endswith(":members"):
            m = SPEC.match(line)
            if not m:
                sys.exit(f"oracle.py gen: line {n}: an owner's spec is not `object:<path>` or `class:<path>`: {line[:120]}")
            specs.append(m.group(1))
            out.append((m.group(1) + ":members", f"observe.Observe.names({scala_literal(m.group(1))})"))
            continue
        key = line.split(" ")[0]
        if key.endswith(":owner") and key[: -len(":owner")] in specs:
            out.append((key, f"observe.Observe.owner({scala_literal(key[: -len(':owner')])})"))
            continue
        owner = max((o for o in specs if key.startswith(o + ".")), key=len, default=None)
        if owner is None:
            sys.exit(f"oracle.py gen: line {n}: no requested owner's key: {line[:120]}")
        out.append((key, f"observe.Observe.member({scala_literal(owner)}, {scala_literal(key[len(owner) + 1:])})"))
    keys = [k for k, _ in out]
    twice = sorted({k for k in keys if keys.count(k) > 1})
    if twice:
        sys.exit(f"oracle.py gen: keys of two records: {' '.join(twice)}")
    return out


def gen(cell, use, dest):
    imports = "".join(l + "\n" for l in lines(use) if l.startswith("import "))
    body = "".join(f"  println({expr}) // key {key}\n" for key, expr in calls(cell))
    with open(dest, "w", encoding="utf-8", newline="") as f:
        f.write("// Generated by tests/tasty/representation/oracle.py: a call per line of a scalac cell.\n")
        f.write(imports + "@main def observeAgain(): Unit =\n" + (body or "  ()\n"))


def teq(diagnostics, generated):
    keys = {}
    for n, line in enumerate(lines(generated), start=1):
        # The generated traversal's calls, and the reflection's probes' (`use/probe.scala`, a call a record).
        m = re.match(r'^  println\(observe\.(?:Observe\.(?:names|owner|member)\(.*\)|Probe\.run\[.*\]\(.*\))\) // key (.*)$', line)
        if m:
            keys[n] = m.group(1)
    src = re.escape(generated.split("/")[-1])
    head = re.compile(r"^\S*" + src + r":(\d+):\d+: (warning|error): (.*)$")
    found = {}
    log = lines(diagnostics)
    i = 0
    while i < len(log):
        m = head.match(log[i])
        if m and int(m.group(1)) in keys:
            n, kind, msg = int(m.group(1)), m.group(2), m.group(3)
            if kind == "warning" and msg == "OBSERVE":
                body = []
                i += 1
                while i < len(log) and not log[i].startswith("    ") and not head.match(log[i]):
                    body.append(log[i])
                    i += 1
                found.setdefault(n, []).extend(body)
                continue
            if kind == "error":
                found.setdefault(n, []).append(f"{keys[n]} <teq refused: {msg}>")
        i += 1
    for n in sorted(keys):
        got = [l for l in found.get(n, []) if l.strip()]
        for line in got or [f"{keys[n]} <teq lists no such member>"]:
            print(line)


# A record's outcome field (observe.scala `record`): its first ` |`, a raw space, which a quoted text never holds (the
# space is `\u0020` there), then `|`, which only a quoted text holds; then `Threw([..])`, an `Unread(<the enclosing
# definitions>, <role>, <class>, <message>)` per child the reader could not give. A teq refusal (`<teq refused: ..>`,
# teq's own message) is no observation and has none.
def outcome(line):
    """A record split into its text and its outcome field's entries, each (the definitions' names, the role, the
    exception's class, its message), in the order the walk met them."""
    i = line.find(" |")
    if i < 0 or line.split(" ", 1)[-1].startswith("<teq "):
        return line, []
    return line[:i], [tuple(e) for e in fields(line[i + 2:])]


def fields(text):
    """`Threw([Unread([a, b], role, Class, "msg"), ..])` read back: each Unread's [names], role, class, message."""
    toks = [t for t in re.findall(r'"(?:\\.|[^"\\])*"(?:#\d+)?|[\[\](),]|[^\s\[\](),"]+', text)]
    out, k = [], 0
    while k < len(toks):
        if toks[k] == "Unread":
            k += 2  # `Unread`, `(`
            names = []
            k += 1  # `[`
            while toks[k] != "]":
                if toks[k] != ",":
                    names.append(toks[k])
                k += 1
            rest = []
            k += 1
            while toks[k] != ")":
                if toks[k] != ",":
                    rest.append(toks[k])
                k += 1
            out.append([tuple(names)] + rest)
        k += 1
    return out


def entry_id(e):
    """An unread child's name in its record: its enclosing definitions, then its role (`localRef#0/rhs`)."""
    return "/".join(list(e[0]) + [e[1]])


def masked(line):
    """A record without its outcome field: what of it is evidence (the child itself stands as `Threw` in the text)."""
    return outcome(line)[0]


def detect_evidence(diff):
    """Each detection section's difference must stand outside the children a reader could not give: a pair of
    records that differs only in their outcome fields is no evidence of the variant being seen (the pass over
    334a43adf). Fails on a section whose every difference is such a pair, and names each such pair."""
    ok, section, recs = True, None, {}
    def close():
        nonlocal ok
        if section is None:
            return
        real = [k for k, (a, b) in recs.items() if a is None or b is None or masked(a) != masked(b)]
        only = [k for k, (a, b) in recs.items() if a is not None and b is not None and a != b and masked(a) == masked(b)]
        for k in only:
            print(f"FAIL representation detect-evidence: {section} {k} differs only where a reader threw")
            ok = False
        if not real:
            print(f"FAIL representation detect-evidence: {section} holds no difference outside a reader's Threw")
            ok = False
    for line in lines(diff):
        if line.startswith("## "):
            close()
            section, recs = line[3:], {}
        elif line[:2] in ("< ", "> "):
            k = line[2:].split(" ")[0]
            a, b = recs.get(k, (None, None))
            recs[k] = (line[2:], b) if line[0] == "<" else (a, line[2:])
    close()
    if ok:
        print("ok representation detect-evidence: every detection section's difference stands outside the children a reader"
              " could not give")
    return ok


# The definitions of observe.scala's `serialize`, and where each child an `attempt` reads stands among the node's
# arguments, counted from its last (the metadata `definition` gives comes before them, its length the node's own).
DEF_NODES = {"ValDef", "DefDef", "TypeDef", "ClassDef", "Bind", "TypeBind"}
ROLE_ARG = {"ValDef": {"tpt": -2, "rhs": -1}, "DefDef": {"paramss": -3, "returnTpt": -2, "rhs": -1},
            "TypeDef": {"rhs": -1}, "ClassDef": {"constructor": -4, "parents": -3, "self": -2, "body": -1}}


def nodes_of(text):
    """The serialization's nodes, `Name(..)` each as [name, start, end, [(start, end) of each argument], children]:
    quoted spans skipped, arguments split at `, ` at the node's own depth."""
    roots, stack, i = [], [], 0
    while i < len(text):
        c = text[i]
        if c == '"':
            m = QUOTED.match(text, i)
            i = m.end()
            continue
        if c in "([":
            m = re.search(r"[A-Za-z0-9_$<>]+$", text[:i]) if c == "(" else None
            node = [m.group(0) if m else "", m.start() if m else i, None, [], [], i + 1]
            (stack[-1][4] if stack else roots).append(node)
            stack.append(node)
        elif c in ")]":
            node = stack.pop()
            node[3].append((node[5], i))
            node[2] = i + 1
        elif c == "," and stack and text.startswith(", ", i):
            node = stack[-1]
            node[3].append((node[5], i))
            node[5] = i + 2
        i += 1
    return roots


def base(n):
    """A definition's name without the observation's index (`x#3`: `x`), its quoted spelling as it is (`"f#3"` keeps
    its name). `read_key` finds the suffix outside every quoted span."""
    return read_key(n)[1]


def locate(text, names, role):
    """The span of the child an `Unread` names in a record's text: the record's definition by the first name, each
    further name the one definition of it beneath the last without another between, then the role's argument; the
    whole text for `record`. None where the shape is not there or a name is not one definition."""
    if role == "record" and not names:
        return (0, len(text))
    roots = [n for n in nodes_of(text) if n[0]]
    if not roots or not names:
        return None
    node = roots[-1]
    def head(n):
        a = n[3][0] if n[3] else None
        return base(text[a[0]:a[1]]) if a else None
    if node[0] not in DEF_NODES or head(node) != base(names[0]):
        return None
    for nm in names[1:]:
        found, todo = [], list(node[4])
        while todo:
            n = todo.pop(0)
            if n[0] in DEF_NODES:
                if head(n) == base(nm):
                    found.append(n)
            else:
                todo.extend(n[4])
        if len(found) != 1:
            return None
        node = found[0]
    if role == "record":
        return (node[1], node[2])
    at = ROLE_ARG.get(node[0], {}).get(role)
    if at is None or len(node[3]) < -at:
        return None
    return node[3][at]


def unread_of(cell):
    """A reference cell's unread children by record key: [(entry, ...)]."""
    out = {}
    for l in lines(cell):
        text, entries = outcome(l)
        if entries:
            out[l.split(" ")[0]] = entries
    return out


def excise(reference, cell):
    """The cell with each child its reference could not give (the reference cell's outcome fields) replaced by
    `Threw` and dropped from its own field, so that the comparison holds what the reference observed and no more (the
    pass over 7a9ec6133): that child's evidence is the products' tags (`tags`, check 11), never this record. A child
    whose place the record does not have is left as it is, a difference as any."""
    gone = unread_of(reference)
    for l in lines(cell):
        key = l.split(" ")[0]
        text, entries = outcome(l)
        cut = [(e[0], e[1]) for e in gone.get(key, [])]
        # The record's head, its key and kind (a probe's key alone), then the text its `record` read.
        at = len(" ".join(text.split(" ")[:1 if key.startswith("probe:") else 2])) + 1
        spans = []
        for names, role in cut:
            sp = locate(text[at:], names, role)
            if sp:
                spans.append((sp[0] + at, sp[1] + at))
        for a, b in sorted(spans, reverse=True):
            text = text[:a] + "Threw" + text[b:]
        kept = [e for e in entries if (e[0], e[1]) not in cut]
        if kept:
            text += " |Threw([" + ", ".join(f"Unread([{', '.join(e[0])}], {', '.join(e[1:])})" for e in kept) + "])"
        print(text)


# The products' tags (check 11): each child a reference observation could not give, read from the pickle scalac wrote,
# record by record (`teq tasty --nodes`, src/tasty/dump.rs, the route the encoding check's references take), so that it is
# observed apart from the reflection whose reader threw. The modifiers that end a definition's node; the parameters
# that begin a method's.
MODIFIERS = {"PRIVATE", "PROTECTED", "ABSTRACT", "FINAL", "SEALED", "CASE", "IMPLICIT", "LAZY", "OVERRIDE", "INLINEPROXY",
             "INLINE", "STATIC", "OBJECT", "TRAIT", "ENUM", "LOCAL", "SYNTHETIC", "ARTIFACT", "MUTABLE", "FIELDaccessor",
             "CASEaccessor", "COVARIANT", "CONTRAVARIANT", "HASDEFAULT", "STABLE", "MACRO", "ERASED", "OPAQUE",
             "EXTENSION", "GIVEN", "PARAMsetter", "EXPORTED", "OPEN", "PARAMalias", "TRANSPARENT", "INFIX", "INVISIBLE",
             "TRACKED", "SUBMATCH", "INTO", "ANNOTATION", "PRIVATEqualified", "PROTECTEDqualified"}
PARAMS = {"TYPEPARAM", "PARAM", "EMPTYCLAUSE", "SPLITCLAUSE"}
TAG_DEFS = {"VALDEF", "DEFDEF", "TYPEDEF", "PARAM", "TYPEPARAM", "BIND"}
# The tags whose payload is an address (a definition's, or a tree's to share).
TAG_ADDR = {"TERMREFdirect", "TYPEREFdirect", "TERMREFsymbol", "TYPEREFsymbol", "RECthis", "RETURN"}
# The nodes a PARAMtype's binder or a RECthis's RECtype can be: a lambda's or a recursive type's binder.
BINDER_TAGS = {"LAMBDA", "LAMBDAtpt", "METHODtype", "POLYtype", "TYPELAMBDAtype", "RECtype"}


# Check 11's fields whose raw value is not compared as it stands (the ruling on 8aa2ab15a's pass): the identity map
# replaces an address, and a name record's `ref` is the names section's index, replaced by the identity of the entry it
# names. The records keep every field, with the identity in place of the address or index. Nothing else is left out.
LEFT_OUT = [
    ("*", "at", "the record's own address, replaced by its position in the subtree (@k)"),
    ("*", "parent", "the parent's address, replaced by the parent's position in the subtree (@k), or - for the child's root"),
    ("*", "addr", "an address, replaced by the definition's number in the subtree (#k), a shared target's position (@k) when the target is inside, or tag_identity's declaration identity outside (RECthis's by its binder's identity)"),
    ("PARAMtype", "binder", "a binder's address, replaced by its number in the subtree (^k) or tag_binder's identity outside"),
]

SHARED = {"SHAREDtype", "SHAREDterm"}


def tag_left_out(tag, key):
    """Whether LEFT_OUT leaves a record's field's raw value out of the comparison."""
    return any((kind == "*" or kind == tag) and field == key for kind, field, _ in LEFT_OUT)


def tag_order(k):
    """A record's dump order: its address, and a name record before the tree that starts at the same address."""
    return (tag_addr(k), 0 if isinstance(k, tuple) else 1)


def tag_subtree(nodes, roots):
    """The records a child's subtree holds: children in order, a shared tree's target followed (as `tag_defs` follows
    it), a tree that recurs cut where it recurs, each record once. Sorted into the dump's order."""
    seen = set()
    def walk(k):
        if k in seen:
            return
        seen.add(k)
        if nodes[k][1] in SHARED:
            t = nodes[k][4].get("addr")
            if t in nodes:
                walk(t)
            return
        for c in nodes[k][3]:
            walk(c)
    for r in roots:
        walk(r)
    return sorted(seen, key=tag_order)


def evidence_injective(raw, records, write=None):
    """Distinct raw records give distinct evidence records: the fallback's record writer's check."""
    write = write or (lambda r: json.dumps(r, ensure_ascii=True, separators=(",", ":")))
    by_value = {}
    for raw_record, record in zip(raw, records):
        raw_key = json.dumps(raw_record, ensure_ascii=True, separators=(",", ":"))
        value = write(record)
        if value in by_value and by_value[value][0] != raw_key:
            injectivity_fail("evidence", by_value[value][1], raw_key, value)
        by_value[value] = (raw_key, value)
    return len(raw)


def tag_records(nodes, kids, parents, specs, numbers, binders):
    """A child's subtree as the dump's records, compared as data: each record its tag, then its own position (`at`) and
    its parent's (`parent`), then every field in the dump's order, after the identity map. The table returns the
    identities with the records: a record's `@k`, a definition's `#k` (`tag_defs` numbers them), a binder's `^k`
    (`tag_binder_numbers`), and the outside identities `tag_identity` and `tag_binder` give. A record's `ref` is the
    identity `n<k>` of the k-th distinct name-table value the evidence meets, in record order; refs whose entries hold
    equal values share one identity, as dotty's `NameTable` permits. A ref's value comes from the product's
    ref-to-name table, the records' `(ref, name)` pairs, never from the record's own `name` field; a `name` is the
    dump's structured value, never unquoted, re-escaped or re-joined."""
    ordered = tag_subtree(nodes, kids)
    position = {k: f"@{i}" for i, k in enumerate(ordered)}
    by_addr = {tag_addr(k): position[k] for k in ordered if not isinstance(k, tuple)}
    outside = {}

    def address_identity(tag, t):
        if tag in SHARED and t in by_addr:
            return by_addr[t]
        if t in numbers:
            return f"#{numbers[t]}"
        if t in by_addr:
            return by_addr[t]
        if t in nodes:
            ident = tag_identity(nodes, parents, specs, t)
            outside[str(t)] = ident
            return ident
        return "?"

    def binder_identity(b):
        if b in binders:
            return f"^{binders[b]}"
        if b in nodes:
            ident = tag_binder(nodes, parents, specs, b)
            if ident is not None:
                outside[str(b)] = ident
                return ident
        return "?"

    raw, records = [], []
    ref_pairs = {}
    for k in ordered:
        f = nodes[k][4]
        if "ref" in f and "name" in f:
            ref_pairs.setdefault(f["ref"], []).append((k, f["name"]))
    names_seen = {}
    for k in ordered:
        tag, fields = nodes[k][1], nodes[k][4]
        p = parents.get(k)
        # The dump line's first two columns: the record's own address and its parent's.
        raw_fields = [("at", tag_addr(k)), ("parent", tag_addr(p) if p is not None else "-")]
        fields_out = [("at", position[k]), ("parent", position.get(p, "-"))]
        for key, value in fields.items():
            raw_fields.append((key, value))
            if key == "ref":
                pairs = ref_pairs.get(value, [])
                named = next((n for c, n in pairs if c != k), pairs[0][1] if pairs else ["UNKNOWN", value])
                ident = json.dumps(named, ensure_ascii=True, separators=(",", ":"))
                if ident not in names_seen:
                    names_seen[ident] = f"n{len(names_seen)}"
                value = names_seen[ident]
            elif key == "addr":
                value = binder_identity(value) if tag == "RECthis" else address_identity(tag, value)
            elif key == "binder":
                value = binder_identity(value)
            fields_out.append((key, value))
        raw.append(tuple([tag] + raw_fields))
        records.append(tuple([tag] + fields_out))
    # Each identity the map wrote, in order; the dump's raw addresses stay in the map's keys, where a variant that
    # shifts them is no difference of the records.
    table = {
        "records": [position[k] for k in ordered],
        "definitions": [f"#{n}" for _, n in sorted(numbers.items(), key=lambda x: x[1])],
        "binders": [f"^{n}" for _, n in sorted(binders.items(), key=lambda x: x[1])],
        "outside": [ident for _, ident in sorted(outside.items())],
    }
    evidence_injective(raw, records)
    return records, table, raw


def tag_key_json(k):
    """A node key as JSON: an address, or a name record's (address, line) as a two-element list."""
    return k if isinstance(k, int) else [k[0], k[1]]


def tag_key_read(k):
    """A node key read back from `tag_key_json`."""
    return k if isinstance(k, int) else tuple(k)


def address_injective(nodes, parents, specs, identity=None, binder=None):
    """The identity map's check over one product dump: two definition or binder addresses never get one identity."""
    identity = identity or tag_identity
    binder = binder or tag_binder
    seen = {}
    checked = 0
    for a in sorted((k for k in nodes if isinstance(k, int)), key=tag_addr):
        tag = nodes[a][1]
        if tag in TAG_DEFS:
            ident = identity(nodes, parents, specs, a)
        elif tag in BINDER_TAGS:
            ident = binder(nodes, parents, specs, a)
            if ident is None:
                continue
        else:
            continue
        checked += 1
        key = json.dumps(ident, ensure_ascii=True, sort_keys=True)
        if key in seen:
            injectivity_fail("addresses", seen[key], a, key)
        seen[key] = a
    return checked


def tag_evidence(nodes, kids, owners):
    """A child's records and their identity table, as `tags` and `records_selfcheck` build them: the definitions the
    subtree holds numbered, its binders numbered, every record's parent, the owners' specs, then `tag_records`."""
    numbers = {}
    for k in kids:
        tag_defs(nodes, k, numbers)
    parents = {k: p for p, n in nodes.items() for k in n[3]}
    specs = {a: spec for spec, a in owners.items()}
    address_injective(nodes, parents, specs)
    binders = tag_binder_numbers(nodes, kids)
    return tag_records(nodes, kids, parents, specs, numbers, binders)


def name_simple(value):
    """A name's UTF-8 text where it is a simple name (`["UTF8", <text>]`), else None."""
    return value[1] if isinstance(value, list) and len(value) == 2 and value[0] == "UTF8" else None


def name_display(value):
    """A name's text for the paths and owner specs a dump's keys are built from: a simple name's text, a compound
    name's fields read recursively. Never used for a comparison's evidence; the records keep the structure."""
    if isinstance(value, str):
        return value
    if not value:
        return ""
    tag = value[0]
    if tag == "UTF8":
        return value[1]
    if tag in ("QUALIFIED", "EXPANDED", "EXPANDPREFIX"):
        return name_display(value[1]) + {"QUALIFIED": ".", "EXPANDED": "$$", "EXPANDPREFIX": "$"}[tag] + name_display(value[2])
    if tag == "UNIQUE":
        return (name_display(value[3]) if value[3] is not None else "") + name_display(value[1]) + str(value[2])
    if tag == "DEFAULTGETTER":
        return name_display(value[1]) + "$default$" + str(value[2] + 1)
    if tag in ("SUPERACCESSOR", "INLINEACCESSOR", "OBJECTCLASS", "BODYRETAINER"):
        return {"SUPERACCESSOR": "super$", "INLINEACCESSOR": "inline$", "OBJECTCLASS": "", "BODYRETAINER": ""}[tag] + name_display(value[1]) + ("$" if tag == "OBJECTCLASS" else "$retainedBody" if tag == "BODYRETAINER" else "")
    if tag in ("SIGNED", "TARGETSIGNED"):
        return name_display(value[1])
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"))


def tag_text(nodes, a):
    """A tag's name, read from its record's structured `name` (or its constant's `value`), for the lookups that match
    definitions by name: the UTF-8 text of a simple name, the structure for any other kind. The records themselves
    keep the structure (`tag_records`)."""
    fields = nodes[a][4]
    if "name" in fields:
        n = fields["name"]
        return name_simple(n) if name_simple(n) is not None else n
    return str(fields["value"]) if "value" in fields else ""


def tag_children(nodes, a, role):
    """A definition node's children for a role of observe.scala's: a ValDef's type and right-hand side, a DefDef's
    clauses, result type and right-hand side, a TypeDef's right-hand side, a class's constructor, parents, self and
    body (TEMPLATE: its parameters, parents, SELFDEF, statements). None where the node has no such child."""
    tag, kids = nodes[a][1], nodes[a][3]
    if role == "record":
        return [a]
    own = [k for k in kids if nodes[k][1] not in MODIFIERS]
    if tag == "VALDEF":
        return {"tpt": own[:1], "rhs": own[1:2]}.get(role)
    if tag == "DEFDEF":
        ps = [k for k in own if nodes[k][1] in PARAMS]
        rest = [k for k in own if nodes[k][1] not in PARAMS]
        return {"paramss": ps, "returnTpt": rest[:1], "rhs": rest[1:2]}.get(role)
    if tag == "TYPEDEF":
        if role == "rhs":
            return own[:1]
        tmpl = [k for k in own if nodes[k][1] == "TEMPLATE"]
        if not tmpl:
            return None
        # TreePickler's Template (TreePickler.scala 678-711): its parameters, its parents, its SELFDEF, then its
        # statements, the constructor first.
        tk = [k for k in nodes[tmpl[0]][3] if nodes[k][1] not in PARAMS]
        at = next((i for i, k in enumerate(tk) if nodes[k][1] == "DEFDEF" and tag_text(nodes, k) == "<init>"), len(tk))
        head, stats = tk[:at], tk[at:]
        return {"constructor": stats[:1], "parents": [k for k in head if nodes[k][1] != "SELFDEF"],
                "self": [k for k in head if nodes[k][1] == "SELFDEF"], "body": stats[1:]}.get(role)
    return None


def tag_owners(nodes, roots):
    """Every class of a dump by the owner spec observe.scala reads (`object:<full name>` for a module class,
    `class:<full name>`), each its TYPEDEF's address."""
    out = {}
    def walk(a, pkg, outer):
        tag = nodes[a][1]
        if tag == "PACKAGE":
            kids = nodes[a][3]
            p = name_display(tag_text(nodes, kids[0])) if kids else ""
            for k in kids[1:]:
                walk(k, p, [])
            return
        if tag == "TYPEDEF" and any(nodes[k][1] == "TEMPLATE" for k in nodes[a][3]):
            n = tag_text(nodes, a)
            obj = isinstance(n, list) and n and n[0] == "OBJECTCLASS"
            text = name_display(n)
            obj = obj or text.endswith("$")
            full = ".".join(([pkg] if pkg and pkg != "<empty>" else []) + outer + [text[:-1] if obj else text])
            out[("object:" if obj else "class:") + full] = a
            tmpl = [k for k in nodes[a][3] if nodes[k][1] == "TEMPLATE"][0]
            for k in nodes[tmpl][3]:
                walk(k, pkg, outer + [text[:-1] if obj else text])
    for r in roots:
        walk(r, "", [])
    return out


def tag_addr(k):
    """A node's address, a name record's key being its address and its line."""
    return k[0] if isinstance(k, tuple) else k


def tag_descendants(nodes, a):
    """A node's descendants, shared trees followed, in address order at the caller's sort."""
    out, seen = [], set()
    def walk(k):
        if k in seen:
            return
        seen.add(k)
        out.append(k)
        tag = nodes[k][1]
        if tag in ("SHAREDtype", "SHAREDterm"):
            t = nodes[k][4].get("addr")
            if t in nodes:
                walk(t)
        else:
            for c in nodes[k][3]:
                walk(c)
    for k in nodes[a][3]:
        walk(k)
    return out


def tag_enclosing(nodes, parents, a):
    """The nearest definition that holds `a`, or None: a definition's nesting level is its nearest enclosing
    definition (the owner's TYPEDEF, or another definition), not the non-definition trees between them."""
    d = parents.get(a)
    while d is not None and nodes[d][1] not in TAG_DEFS:
        d = parents.get(d)
    return d


def tag_identity(nodes, parents, specs, a):
    """A definition's identity as a structured value, never a joined string (the ruling on 0324edfab's pass): the
    owner's spec (`object:<full name>` or `class:<full name>`), then one `[name, occurrence]` component per nesting
    level from the owner down, the name the record's own text (unquoted, never re-joined) and the occurrence 1-based
    among the same-named definitions at that level. A local's components continue its enclosing definition's; a
    parameter's continue its method's."""
    name = tag_text(nodes, a)
    p = parents.get(a)
    if p is not None and nodes[p][1] == "TEMPLATE":
        c = parents.get(p)
        if c is not None and nodes[c][1] == "TYPEDEF" and c in specs:
            same = [k for k in nodes[p][3] if nodes[k][1] in TAG_DEFS and tag_text(nodes, k) == name]
            n = same.index(a) + 1 if a in same else 1
            return [specs[c], [name, n]]
    d = parents.get(a)
    while d is not None and nodes[d][1] not in TAG_DEFS:
        d = parents.get(d)
    if d is not None:
        same = sorted((k for k in tag_descendants(nodes, d)
                       if nodes[k][1] in TAG_DEFS and tag_text(nodes, k) == name
                       and tag_enclosing(nodes, parents, k) == d), key=tag_addr)
        n = same.index(a) + 1 if a in same else 1
        return tag_identity(nodes, parents, specs, d) + [[name, n]]
    return [specs[a]] if a in specs else [name]


def tag_binder(nodes, parents, specs, b):
    """A binder type outside the subtree by its defining definition's identity, its tag and its position among that
    definition's lambda or RECtype nodes, in address order: the identity list, then `["binder", <tag>, <position>]`."""
    d = tag_enclosing(nodes, parents, b)
    if d is None:
        return None
    same = sorted((k for k in tag_descendants(nodes, d) if nodes[k][1] in BINDER_TAGS), key=tag_addr)
    if b not in same:
        return None
    return tag_identity(nodes, parents, specs, d) + [["binder", nodes[b][1], same.index(b) + 1]]


def tag_binder_numbers(nodes, roots):
    """The lambda and RECtype nodes the subtree holds, numbered in order (the `^k` `tag_records` writes)."""
    seen = set()
    def walk(a):
        if a in seen:
            return
        seen.add(a)
        tag = nodes[a][1]
        if tag in ("SHAREDtype", "SHAREDterm"):
            t = nodes[a][4].get("addr")
            if t in nodes:
                walk(t)
            return
        for k in nodes[a][3]:
            walk(k)
    for a in roots:
        walk(a)
    return {b: k + 1 for k, b in enumerate(sorted((a for a in seen if nodes[a][1] in BINDER_TAGS), key=tag_addr))}


def tag_defs(nodes, a, out, path=frozenset()):
    """The definitions a subtree holds, shared trees followed, in order (the numbers `tag_records` writes)."""
    ind, tag, _, kids, fields = nodes[a]
    if tag in ("SHAREDtype", "SHAREDterm"):
        t = fields["addr"]
        if t not in path and t in nodes:
            tag_defs(nodes, t, out, path | {t})
        return
    if tag in TAG_DEFS and a not in out:
        out[a] = len(out)
    for k in kids:
        tag_defs(nodes, k, out, path | {a})


def tag_locate(nodes, owners, key, names, role):
    """The tag subtrees an `Unread` names: the record's owner (its spec), its member by key (a `name#n` key the n-th
    definition of that name in the template) or the owner's own TYPEDEF, each further name the one definition of it
    beneath the last without another between, then the role's children. A reason where it is not there."""
    spec, member, nth, owner = read_key(key)
    if owner:
        if spec not in owners:
            return None, f"no class {spec} in the product"
        a = owners[spec]
    else:
        if spec not in owners:
            return None, f"no owner of {key} in the product"
        nm = unquote(member)
        tmpl = [c for c in nodes[owners[spec]][3] if nodes[c][1] == "TEMPLATE"][0]
        same = [c for c in nodes[tmpl][3] if nodes[c][1] in TAG_DEFS and tag_text(nodes, c) == nm]
        n = nth or 1
        if len(same) < n:
            return None, f"no definition {member}{f'#{nth}' if nth else ''} in {spec}"
        a = same[n - 1]
    if names:
        def text_or_structure(n):
            # A simple name is its UTF-8 text; any other kind stays the structure it is.
            return name_simple(n) if name_simple(n) is not None else n
        def unstar(n):
            t = text_or_structure(n)
            return t[:-1] if isinstance(t, str) and t.endswith("$") else t
        if unquote(base(names[0])) != unstar(nodes[a][4].get("name", tag_text(nodes, a))) and unquote(base(names[0])) != text_or_structure(tag_text(nodes, a)):
            return None, f"the record's definition is {tag_text(nodes, a)}, not {names[0]}"
        for nm in names[1:]:
            found, todo = [], list(nodes[a][3])
            while todo:
                c = todo.pop(0)
                if nodes[c][1] in TAG_DEFS:
                    if text_or_structure(tag_text(nodes, c)) == unquote(base(nm)):
                        found.append(c)
                else:
                    todo.extend(nodes[c][3])
            if len(found) != 1:
                return None, f"{len(found)} definitions {nm} beneath {tag_text(nodes, a)}"
            a = found[0]
    kids = tag_children(nodes, a, role)
    if kids is None:
        return None, f"{nodes[a][1]} {tag_text(nodes, a)} has no {role}"
    return kids, None


def records_selfcheck(cells):
    """Check 11's records of their own (the ruling on 8aa2ab15a's pass): for every `(record kind, field)` the
    fixtures' unread children hold, one occurrence's value in the parsed dump is changed (a number plus one, a string
    one character inside its quotes), the child's evidence is rebuilt through `tag_records`, and it must change. A
    field LEFT_OUT leaves out is not mutated as a value; every such entry must name a field some record has. An
    address LEFT_OUT replaces through the identity map is changed to another definition's of a different identity, and
    a `ref` to another record's name-table entry; the evidence must change too, so that the maps write identities and
    not constants. A mutation the rebuilt evidence does not see fails its pair."""
    ok = True
    checked, seen = set(), set()

    def mutate(v):
        if isinstance(v, int):
            return v + 1
        if isinstance(v, str):
            if len(v) >= 2 and v[0] == '"' and v[-1] == '"':
                if len(v) > 2:
                    return v[0] + ("y" if v[1] != "y" else "z") + v[2:]
                return '"x"'
            return v + "x"
        if isinstance(v, list):
            # A structured name: its first string field, or a marker while it has none, changes the value.
            for i, x in enumerate(v):
                y = mutate(x)
                if y != x:
                    out = list(v)
                    out[i] = y
                    return out
            return v + ["mutated"]
        return v

    def changed_node(nodes, k, field, value):
        fields = dict(nodes[k][4])
        fields[field] = value
        out = dict(nodes)
        out[k] = [nodes[k][0], nodes[k][1], nodes[k][2], nodes[k][3], fields]
        return out

    def serialized(records, table):
        return json.dumps(table, ensure_ascii=True) + "\n" + json.dumps(records, ensure_ascii=True)

    inputfiles = sorted(glob.glob(os.path.join(cells, "*-tags.txt.input")))
    if not inputfiles:
        print("FAIL representation records self-check: no unread child's parsed dump is kept")
        return False
    for inputfile in inputfiles:
        with open(inputfile, encoding="utf-8") as f:
            contexts = json.load(f)
        for context in contexts:
            key, e = context["key"], context["entry"]
            nodes = {}
            for k, tag, kids, fields in context["nodes"]:
                nodes[tag_key_read(k)] = [0, tag, None, [tag_key_read(c) for c in kids], fields]
            kids = [tag_key_read(k) for k in context["kids"]]
            owners = {spec: tag_key_read(a) for spec, a in context["owners"]}
            records, table, _ = tag_evidence(nodes, kids, owners)
            unmutated = serialized(records, table)
            ordered = tag_subtree(nodes, kids)
            subtree = set(ordered)
            for k in ordered:
                tag = nodes[k][1]
                seen.add((tag, "at"))
                seen.add((tag, "parent"))
                for field, value in nodes[k][4].items():
                    seen.add((tag, field))
                    if field == "ref" or tag_left_out(tag, field) or (tag, field) in checked:
                        continue
                    rebuilt, rebuilt_table, _ = tag_evidence(changed_node(nodes, k, field, mutate(value)), kids, owners)
                    if serialized(rebuilt, rebuilt_table) == unmutated:
                        print(f"FAIL representation records self-check: ({tag}, {field}): its mutation is not seen")
                        ok = False
                    checked.add((tag, field))
            # LEFT_OUT's mapped addresses: the map must write another definition's identity, not a constant.
            parents = {k: p for p, n in nodes.items() for k in n[3]}
            specs = {a: spec for spec, a in owners.items()}
            for k in ordered:
                tag = nodes[k][1]
                if tag not in ("TERMREFdirect", "RETURN") or (tag, "addr") in checked:
                    continue
                target = nodes[k][4].get("addr")
                if target not in nodes or target in subtree or nodes[target][1] not in TAG_DEFS:
                    continue
                ident = tag_identity(nodes, parents, specs, target)
                definitions = [c for c in nodes
                               if c != target and c not in subtree and nodes[c][1] in TAG_DEFS]
                other = next((c for c in definitions if tag_identity(nodes, parents, specs, c) != ident), None)
                # If the identity function itself has collapsed, another definition still proves the map writes
                # something other than a constant.
                other = other if other is not None else (definitions[0] if definitions else None)
                if other is None:
                    continue
                rebuilt, rebuilt_table, _ = tag_evidence(changed_node(nodes, k, "addr", other), kids, owners)
                if serialized(rebuilt, rebuilt_table) == unmutated:
                    print(f"FAIL representation records self-check: ({tag}, addr): another definition's identity is"
                          " not seen")
                    ok = False
                checked.add((tag, "addr"))
            # `ref` must name another entry: a different index the records hold, whose value differs.
            ref_values = {}
            for c in ordered:
                f = nodes[c][4]
                if "ref" in f and "name" in f:
                    ref_values.setdefault(f["ref"], f["name"])
            for k in ordered:
                tag = nodes[k][1]
                if "ref" not in nodes[k][4] or (tag, "ref") in checked:
                    continue
                original = nodes[k][4]["ref"]
                original_value = ref_values.get(original)
                other = next((nodes[c][4]["ref"] for c in ordered
                              if c != k and "ref" in nodes[c][4]
                              and ref_values.get(nodes[c][4]["ref"]) != original_value), None)
                if other is None:
                    continue
                rebuilt, rebuilt_table, _ = tag_evidence(changed_node(nodes, k, "ref", other), kids, owners)
                if serialized(rebuilt, rebuilt_table) == unmutated:
                    print(f"FAIL representation records self-check: ({tag}, ref): another entry's reference is"
                          " not seen")
                    ok = False
                checked.add((tag, "ref"))
    for kind, field, _ in LEFT_OUT:
        # A record kind no unread child holds has no field to name here; every kind that occurs must own its entry.
        if kind != "*" and not any(tag == kind for tag, _ in seen):
            continue
        if not any((kind == "*" or tag == kind) and key == field for tag, key in seen):
            print(f"FAIL representation records self-check: LEFT_OUT names ({kind}, {field}), which no record has")
            ok = False
    if not checked:
        print("FAIL representation records self-check: no (record kind, field) pair is present")
        ok = False
    if ok:
        print(f"ok representation records self-check: each of the {len(checked)} (record kind, field) pairs the unread"
              " children hold is seen, N=1 mutation per field; every field left out is one of LEFT_OUT's")
    return ok


def identity_selfcheck(cells):
    """The identities' own check (the ruling on 0324edfab's pass): every definition address and every binder address
    a product dump holds gets `tag_identity`'s or `tag_binder`'s identity, and two different addresses of one dump must
    never get the same identity. Every parsed dump the fallback reads is one `<cell>-tags.txt.input` context; the same
    dump repeated for several children is checked once. Fails naming the two addresses and the identity."""
    ok, checked = True, 0
    inputfiles = sorted(glob.glob(os.path.join(cells, "*-tags.txt.input")))
    if not inputfiles:
        print("FAIL representation identity self-check: no product dump the fallback reads is kept")
        return False
    for inputfile in inputfiles:
        with open(inputfile, encoding="utf-8") as f:
            contexts = json.load(f)
        dumps = set()
        for context in contexts:
            signature = json.dumps(context["nodes"], ensure_ascii=True, sort_keys=True)
            if signature in dumps:
                continue
            dumps.add(signature)
            nodes = {}
            for k, tag, kids, fields in context["nodes"]:
                nodes[tag_key_read(k)] = [0, tag, None, [tag_key_read(c) for c in kids], fields]
            parents = {k: p for p, n in nodes.items() for k in n[3]}
            owners = {spec: tag_key_read(a) for spec, a in context["owners"]}
            specs = {a: spec for spec, a in owners.items()}
            seen = {}
            for a in sorted((k for k in nodes if isinstance(k, int)), key=tag_addr):
                tag = nodes[a][1]
                if tag in TAG_DEFS:
                    ident = tag_identity(nodes, parents, specs, a)
                elif tag in BINDER_TAGS:
                    ident = tag_binder(nodes, parents, specs, a)
                    if ident is None:
                        continue
                else:
                    continue
                checked += 1
                key = json.dumps(ident, ensure_ascii=True, sort_keys=True)
                if key in seen and seen[key] != a:
                    print(f"FAIL representation identity self-check: {os.path.basename(inputfile)}: addresses"
                          f" {seen[key]} and {a} have the identity {json.dumps(ident, ensure_ascii=True)}")
                    ok = False
                else:
                    seen[key] = a
    if ok:
        print(f"ok representation identity self-check: the identity map is injective over each product dump's {checked}"
              " definition and binder addresses")
    return ok


def injectivity_selfcheck(cells):
    """The injectivity checks' own check: each serializer is run on every product dump the fallback reads, its inputs
    counted, and then given one injected collision it must report. A collision that passes fails the self-check."""
    inputfiles = sorted(glob.glob(os.path.join(cells, "*-tags.txt.input")))
    if not inputfiles:
        print("FAIL representation injectivity self-check: no product dump the fallback reads is kept")
        return False
    counts = {"names": 0, "records": 0, "addresses": 0, "evidence": 0, "facts": 0, "lines": 0}
    sample = None
    sample_facts = None
    dumps = set()
    for inputfile in inputfiles:
        with open(inputfile, encoding="utf-8") as f:
            contexts = json.load(f)
        for context in contexts:
            signature = json.dumps(context["nodes"], ensure_ascii=True, sort_keys=True)
            if signature in dumps:
                continue
            dumps.add(signature)
            nodes = {}
            for k, tag, kids, fields in context["nodes"]:
                nodes[tag_key_read(k)] = [0, tag, None, [tag_key_read(c) for c in kids], fields]
            parents = {k: p for p, n in nodes.items() for k in n[3]}
            owners = {spec: tag_key_read(a) for spec, a in context["owners"]}
            specs = {a: spec for spec, a in owners.items()}
            roots = sorted((k for k in nodes if k not in parents), key=tag_order)
            if sample is None:
                sample = (nodes, parents, specs)
            entries = [(fields["ref"], fields["name"]) for _, _, _, _, fields in nodes.values()
                       if "ref" in fields and "name" in fields]
            counts["names"] += names_injective(entries)
            lines = []
            for k, n in nodes.items():
                p = parents.get(k)
                parent = tag_addr(p) if p is not None else "-"
                rec = (record_line(tag_addr(k), parent, n[1], list(n[4].items())),
                       tag_addr(k), parent, n[1], list(n[4].items()))
                lines.append(rec)
                record_roundtrip(*rec)
            counts["records"] += len(lines)
            counts["addresses"] += address_injective(nodes, parents, specs)
            kids = [tag_key_read(k) for k in context["kids"]]
            _, _, raw = tag_evidence(nodes, kids, owners)
            counts["evidence"] += len(raw)
            facts = node_facts(nodes, roots)
            if sample_facts is None and facts:
                sample_facts = facts
            counts["facts"] += facts_injective(facts)
            counts["lines"] += fact_lines(os.path.basename(inputfile), facts)
    # One injected collision per serializer: each check must report two inputs with one value.
    def signed(first):
        def path(parts):
            n = ["UTF8", parts[0]]
            for p in parts[1:]:
                n = ["QUALIFIED", n, ["UTF8", p]]
            return n
        a, b = (path(["repr", "rec", "X, repr.rec.Y"]), path(["repr", "rec", "Z"])) if first else \
               (path(["repr", "rec", "X"]), path(["repr", "rec", "Y, repr.rec.Z"]))
        return ["SIGNED", ["UTF8", "f"], [a, b], path(["scala", "Int"])]
    def flatten(n):
        return name_json_text(["SIGNED", n[1], [], n[3]])
    def drop_last(addr, parent, tag, fields):
        return record_line(addr, parent, tag, fields[:-1])
    def merge(nodes, parents, specs, a):
        return ["merged"]
    def drop_evidence(record):
        return json.dumps(record[:-1], ensure_ascii=True, separators=(",", ":"))
    def join_parts(f):
        return " ".join(str(x) for x in f)
    def joined_line(rel, f):
        return f"{rel}:/" + "/".join(name_display(n) for n in f[0]) + " " + fact_json(f[1:])
    def joined_facts():
        return [([["UTF8", "a/b"], ["UTF8", "m"]], "TYPEREF", "Int", None),
                ([["UTF8", "a"], ["UTF8", "b"], ["UTF8", "m"]], "TYPEREF", "Int", None)]
    cases = [
        ("names", lambda: names_injective([(1, signed(True)), (2, signed(False))], flatten)),
        ("records", lambda: record_roundtrip(record_line(1, "-", "VALDEF", [("name", ["UTF8", "a"]), ("ref", 1)]),
                                             1, "-", "VALDEF", [("name", ["UTF8", "a"]), ("ref", 1)], drop_last)),
        ("addresses", lambda: address_injective(sample[0], sample[1], sample[2], merge, merge)),
        ("evidence", lambda: evidence_injective([("A", 1), ("A", 2)], [("A", 1), ("A", 2)], drop_evidence)),
        ("facts", lambda: facts_injective((sample_facts or [([], "TERMREF", None, None)])[:1], join_parts)),
        ("lines", lambda: fact_lines("r", joined_facts(), joined_line)),
    ]
    injected, ok = [], True
    try:
        names_injective([(28, ["UTF8", "?"]), (30, ["UTF8", "?"])])
    except InjectivityFailure as e:
        print(f"FAIL representation injectivity self-check: equal name values are refused: {e}")
        ok = False
    for label, run in cases:
        try:
            run()
        except InjectivityFailure as e:
            injected.append(str(e))
        else:
            print(f"FAIL representation injectivity self-check: the {label} serializer's injected collision passes")
            ok = False
    if ok:
        print("ok representation injectivity self-check: " + ", ".join(
            f"{label} {counts[label]} inputs" for label in ("names", "records", "addresses", "evidence", "facts", "lines")) +
              "; each injected collision fails")
        for msg in injected:
            print(f"injected {msg}")
    return ok


def tags(cell, product, teq_, dest):
    """`<cell>`'s unread children read from its product's pickles (`<product>`, scalac's class directory): a line per
    child, `<key> <entry> <its records>` (`tag_records`, the dump's records as data) and its identities,
    `identity <key> <entry> <the table>`, or `<key> <entry> <not located: why>`. Beside the output, `<dest>.input`
    keeps each located child's parsed dump nodes, its located children and its owners, for `records-selfcheck`'s
    mutations of the input."""
    import subprocess
    out, contexts, dumps = [], [], {}
    def owners_for(spec):
        full = spec.split(":", 1)[1].split(".")
        for k in range(len(full), 0, -1):
            f = os.path.join(product, *full[:k - 1], full[k - 1] + ".tasty")
            if os.path.exists(f):
                if f not in dumps:
                    r = subprocess.run([teq_, "tasty", "--nodes", f], capture_output=True, text=True, timeout=60)
                    tmp = dest + ".dump"
                    open(tmp, "w", encoding="utf-8", newline="").write(r.stdout)
                    try:
                        nodes, roots = enc_nodes(tmp)
                        dumps[f] = (nodes, tag_owners(nodes, roots), None)
                    except ValueError as e:
                        dumps[f] = (None, {}, str(e))
                    finally:
                        os.remove(tmp)
                return dumps[f]
        return None, {}, None
    for key, entries in sorted(unread_of(cell).items()):
        # The top-level class's pickle: the key's longest prefix naming a file of the product.
        nodes, owners, failure = owners_for(read_key(key)[0])
        for e in entries:
            if failure is not None:
                out.append(f"{key} {entry_id(e)} <not located: the product's dump is not read to its end: {failure}>")
                continue
            if nodes is None:
                out.append(f"{key} {entry_id(e)} <not located: no pickle of its owner in {product}>")
                continue
            kids, why = tag_locate(nodes, owners, key, e[0], e[1])
            if why:
                out.append(f"{key} {entry_id(e)} <not located: {why}>")
                continue
            records, table, _ = tag_evidence(nodes, kids, owners)
            out.append(f"identity {key} {entry_id(e)} {json.dumps(table, ensure_ascii=True)}")
            out.append(f"{key} {entry_id(e)} {json.dumps(records, ensure_ascii=True)}")
            contexts.append({
                "key": key,
                "entry": entry_id(e),
                "nodes": [[tag_key_json(k), n[1], [tag_key_json(c) for c in n[3]], n[4]] for k, n in nodes.items()],
                "kids": [tag_key_json(k) for k in kids],
                "owners": [[spec, tag_key_json(a)] for spec, a in owners.items()],
            })
    open(dest, "w", encoding="utf-8").write("".join(l + "\n" for l in out))
    open(dest + ".input", "w", encoding="utf-8").write(json.dumps(contexts, ensure_ascii=True) + "\n")


# Each reference cell's other reader over the same product (T·S, and the probes' own compilation), and its twin's S·T
# cell; a detection variant's cell (ss-api8, ss-rec8) has neither, its children the pair's other side.
OTHER = {**{f"ss-{x}": f"ts-{x}" for x in ("api", "cc38", "cc39", "js", "refining", "rec", "nulls", "probe")},
         "ps-probe": "pt-probe"}
TWIN = {"ss-api": "st-api", "ss-js": "st-js", "ss-refining": "st-refining", "ss-cc38": "st-cc"}


def unread_record(cells, dest):
    """Check 11's record: per child a reference cell's reader could not give, `## <cell> <key> <entry>`, then its
    tags in scalac's product (`tags`), in teq's product of the twin where teq writes one (`teq-product`), and teq's
    reflection's reading of it over scalac's product (`teq-reading`, the child's text in the T-S cell's record)."""
    other, twin = OTHER, TWIN
    def tagmap(path):
        out = {}
        if os.path.exists(path):
            for l in lines(path):
                k, e, rest = l.split(" ", 2)
                out[(k, e)] = rest
        return out
    out = []
    for path in sorted(glob.glob(os.path.join(cells, "ss-*.txt")) + glob.glob(os.path.join(cells, "ps-*.txt"))):
        cell = os.path.basename(path)[:-4]
        if cell.endswith("-kinds") or cell.endswith("-tags"):
            continue
        mine = tagmap(os.path.join(cells, cell + "-tags.txt"))
        theirs = tagmap(os.path.join(cells, twin.get(cell, "-") + "-tags.txt"))
        reading = {}
        op = os.path.join(cells, other.get(cell, "-") + ".txt")
        if os.path.exists(op):
            reading = {l.split(" ")[0]: l for l in lines(op)}
        for key, entries in sorted(unread_of(path).items()):
            for e in entries:
                eid = entry_id(e)
                out.append(f"## {cell} {key} {eid}")
                out.append(f"tags {mine.get((key, eid), '<no tags record>')}")
                if cell in twin:
                    out.append(f"teq-product {theirs.get((key, eid), '<no tags record>')}")
                if cell in other:
                    l = reading.get(key)
                    if l is None:
                        got = "<no record>"
                    else:
                        text, _ = outcome(l)
                        at = len(" ".join(text.split(" ")[:1 if key.startswith("probe:") else 2])) + 1
                        sp = locate(text[at:], e[0], e[1])
                        got = text[at:][sp[0]:sp[1]] if sp else "<no such child in its record>"
                    out.append(f"teq-reading {got}")
    open(dest, "w", encoding="utf-8").write("".join(l + "\n" for l in out))


def read_unread(accounts):
    """`<cell> <key> <entry> case|agrees <what>` lines (unread.txt) by (cell, key, entry), with their line numbers."""
    out, errors = {}, []
    for n, line in enumerate(lines(accounts), start=1):
        if not line.strip() or line.startswith("#"):
            continue
        f = line.split(" ", 4)
        if len(f) < 5 or f[3] not in ("case", "agrees"):
            errors.append(f"line {n} is not `<cell> <key> <entry> case|agrees <what>`: {line}")
            continue
        out[(f[0], f[1], f[2])] = n
    return out, errors


def incomplete(cells, accounts):
    """An incomplete reference observation is no evidence, subtree by subtree (the pass over 7a9ec6133): every child a
    reference cell's reader could not give (its records' outcome fields, ss- and ps- cells) is red unless it is
    observed apart, through the products' tags (`<cell>-tags.txt`, located), and, where another reader reads the same
    product (`OTHER`, `TWIN`), accounted for in unread.txt, `case` (the tags, scalac's answer, against that reading) or
    `agrees`; whatever the rest of the record holds and whether or not the other side's record differs. A detection
    variant's child needs its tags alone (the pair's evidence, check 1's `rec-tags`). An account that names no unread
    child is stale."""
    ok, n = True, 0
    accounted, errors = read_unread(accounts)
    for e in errors:
        print(f"FAIL representation incomplete: unread.txt {e}")
        ok = False
    seen = set()
    for path in sorted(glob.glob(os.path.join(cells, "ss-*.txt")) + glob.glob(os.path.join(cells, "ps-*.txt"))):
        cell = os.path.basename(path)[:-4]
        if cell.endswith("-kinds") or cell.endswith("-tags"):
            continue
        tagged = {}
        tp = os.path.join(cells, cell + "-tags.txt")
        if os.path.exists(tp):
            for l in lines(tp):
                k, e, rest = l.split(" ", 2)
                tagged[(k, e)] = rest
        for key, entries in sorted(unread_of(path).items()):
            for e in entries:
                n += 1
                eid = entry_id(e)
                seen.add((cell, key, eid))
                t = tagged.get((key, eid))
                if t is None or t.startswith("<not located"):
                    print(f"FAIL representation incomplete: {cell} {key} {eid}: scalac's reader threw ({e[2]}) and the"
                          f" products' tags do not observe it: {t or 'no tags record'}")
                    ok = False
                elif cell not in OTHER and cell not in TWIN:
                    seen.discard((cell, key, eid))  # a variant's: the tags are the pair's evidence, no reading to account
                elif (cell, key, eid) not in accounted:
                    print(f"FAIL representation incomplete: {cell} {key} {eid}: scalac's reader threw ({e[2]}); its tags"
                          " are observed, and unread.txt gives it no account (case or agrees)")
                    ok = False
    for k, line in sorted(accounted.items(), key=lambda x: x[1]):
        if k not in seen:
            print(f"FAIL representation incomplete: unread.txt line {line} names no unread child: {' '.join(k)}")
            ok = False
    if ok:
        print(f"ok representation incomplete: every child a reference reader could not give ({n}) is observed through"
              " the products' tags and accounted for (unread.txt), none taken as preserved by its record")
    return ok


def incomplete_selfcheck(cells, accounts, work):
    """Check 11's own mutations (the pass over 7a9ec6133), each with the outcome it must have: the products' tags
    removed, every unread child fails, whether teq's record of it differs from scalac's or equals it (whole-record
    inequality stands for nothing); an account removed, its child fails; a name and a string spelling the outcome field
    (`api.scala`'s `Marker`, quoted) are no unread child."""
    import contextlib
    import io
    import shutil
    ok = True
    def run(d, acc):
        with contextlib.redirect_stdout(io.StringIO()) as out:
            r = incomplete(d, acc)
        return r, out.getvalue()
    def fresh(name):
        d = os.path.join(work, name)
        shutil.rmtree(d, ignore_errors=True)
        shutil.copytree(cells, d)
        return d
    unread = {c: unread_of(os.path.join(cells, c + ".txt")) for c in OTHER if os.path.exists(os.path.join(cells, c + ".txt"))}
    total = sum(len(e) for u in unread.values() for e in u.values())
    if not total:
        print("FAIL representation incomplete self-check: no fixture holds a child scalac's reader cannot give")
        return False
    for equal in (False, True):
        d = fresh("no-tags-equal" if equal else "no-tags")
        for f in glob.glob(os.path.join(d, "*-tags.txt")):
            os.remove(f)
        if equal:  # teq's record made scalac's own, the reference's outcome and all
            for c, u in unread.items():
                tp = os.path.join(d, OTHER[c] + ".txt")
                mine = {l.split(" ")[0]: l for l in lines(os.path.join(cells, c + ".txt"))}
                if os.path.exists(tp):
                    out = [mine[l.split(" ")[0]] if l.split(" ")[0] in u else l for l in lines(tp)]
                    open(tp, "w", encoding="utf-8").write("".join(l + "\n" for l in out))
        r, out = run(d, accounts)
        n = out.count("do not observe it")
        if r or n < total:
            print(f"FAIL representation incomplete self-check: the tags removed{' and teq equal to scalac' if equal else ''},"
                  f" {n} of {total} unread children fail")
            ok = False
    d = fresh("no-account")
    acc = os.path.join(work, "unread-less.txt")
    kept = [l for l in lines(accounts) if l.startswith("#") or not l.strip()]
    first = next((l for l in lines(accounts) if l.strip() and not l.startswith("#")), None)
    rest = [l for l in lines(accounts) if l.strip() and not l.startswith("#") and l != first]
    open(acc, "w", encoding="utf-8").write("".join(l + "\n" for l in kept + rest))
    r, out = run(d, acc)
    if r or first is None or "gives it no account" not in out:
        print("FAIL representation incomplete self-check: an account removed, its child passes")
        ok = False
    marker = os.path.join(work, "marker.txt")
    open(marker, "w", encoding="utf-8").write(
        'class:repr.Marker:members "Threw(" "\\u0020|Threw(" field text\n'
        'class:repr.Marker.field val ValDef(field#0, Some(Literal(String("\\u0020|Threw([Unread([x#0],\\u0020rhs,'
        '\\u0020MatchError,\\u0020\\"129\\")])"))))\n')
    if unread_of(marker):
        print("FAIL representation incomplete self-check: a name or a string spelling the outcome field is read as one")
        ok = False
    if ok:
        print(f"ok representation incomplete self-check: the tags removed, each of the {total} unread children fails,"
              " teq's record different or equal to scalac's; an account removed fails; a name and a string spelling the"
              " outcome field are text")
    return ok


def strip_marks(line):
    """A line with the marks of scalac's bounds classes removed outside its quoted spans alone: a string or a name
    holding `{TypeAlias}` keeps it (re.split with a group keeps the quoted spans at the odd indices)."""
    parts = QUOTED.split(line)
    return "".join(p if i % 2 else MARK.sub("", p) for i, p in enumerate(parts))


def strip(cell):
    """The cell with its bounds marks stripped and its outcome fields dropped: a reference's own unread children are
    `incomplete`'s (check 11), the other side's record excised of them (`excise`)."""
    for line in lines(cell):
        print(strip_marks(outcome(line)[0]))


# A pin's landing: one of type-representation's seven (1 to 7, `op` the opaque-proxies landing), or another part's by
# the ledger entry that gives it the mechanism (`mixin-forwarders(shared-helpers-61)`, ~/reports/shared-helpers.txt).
LANDING = re.compile(r"^(?:[1-7]|op|[a-z][a-z0-9-]*\(shared-helpers-[0-9]+[a-z]?\))$")


def read_pins(pinfile):
    """`<cell> <key> <landing> <what>` lines by (cell, key), with their line numbers; a malformed line's error."""
    pinned, errors = {}, []
    for n, line in enumerate(lines(pinfile), start=1):
        if not line.strip() or line.startswith("#"):
            continue
        f = line.split(" ", 3)
        if len(f) < 4 or not LANDING.match(f[2]):
            errors.append(f"line {n} is not `<cell> <member> <landing: 1-7, op or <part>(shared-helpers-<n>)> <what>`: {line}")
            continue
        pinned[(f[0], f[1])] = n
    return pinned, errors


def read_map(mapfile):
    """The identity map (`encoding-map.txt`): `<I:twin> <identity key> <S-T:twin> <S·T key>` lines, each an identity
    record of the encoding check mapped to the observed S·T record that holds its difference, by line number."""
    out, errors = {}, []
    if not mapfile:
        return out, errors
    for n, line in enumerate(lines(mapfile), start=1):
        if not line.strip() or line.startswith("#"):
            continue
        f = line.split(" ")
        if len(f) != 4 or not f[0].startswith("I:") or not f[2].startswith("S-T:"):
            errors.append(f"map line {n} is not `<I:twin> <key> <S-T:twin> <key>`: {line}")
            continue
        out[(f[0], f[1])] = ((f[2], f[3]), n)
    return out, errors


def encoding_record(line):
    """An encoding line's file and whole fact (`<mark> <file> <fact_json>`), or None for a line that is no fact."""
    rel, sep, value = line[3:].partition(" ")
    if not sep:
        return None
    try:
        fact = json.loads(value)
    except (ValueError, TypeError):
        return None
    if not isinstance(fact, list) or len(fact) != 4 or not isinstance(fact[0], list):
        return None
    return rel, fact


def encoding_key(rel, f):
    """A record's key: its file and its path as structured names, never the names joined into one text."""
    return f"{rel}:{path_key(f[0])}"


def named_records(diffs):
    """The keys a diff's records name, by cell: the observation's (`< `/`> `), the encoding's encoding differences
    (`E< `/`E> `, cell `E:<twin>`) and its identity differences (`I< `/`I> `, cell `I:<twin>`) apart. An encoding
    line's key is its file and its path's structured names."""
    named, identity = set(), set()
    for d in diffs:
        section = None
        for line in lines(d):
            if line.startswith("## "):
                section = line[3:]
            elif line[:2] in ("< ", "> ") and section:
                named.add((section, line[2:].split(" ")[0]))
            elif line[:3] in ("E< ", "E> ") and section:
                parsed = encoding_record(line)
                named.add((section, encoding_key(*parsed) if parsed else line[3:].split(" ")[0]))
            elif line[:3] in ("I< ", "I> ") and section:
                # An identity difference's account is apart from an encoding difference's at one path: cell `I:<twin>`.
                parsed = encoding_record(line)
                identity.add(("I:" + section.split(":", 1)[1],
                              encoding_key(*parsed) if parsed else line[3:].split(" ")[0]))
    return named, identity


def pins_check(pinfile, diffs, mapfile=None, quiet=False):
    """Every difference accounted for, and every pin and mapping a difference's: an observation's or an encoding
    difference by its pin; an identity difference of the encoding check by its own pin, or by its line of the map to
    an observed S·T record that differs and has its pin (never both). A pin or a mapping that names no difference is
    stale. Returns (ok, the failures, the summary)."""
    named, identity = named_records(diffs)
    pinned, errors = read_pins(pinfile)
    mapped, map_errors = read_map(mapfile)
    fails = errors + map_errors
    for cell, key in sorted(named - pinned.keys()):
        fails.append(f"{cell} {key} differs from scalac's and has no pin")
    for cell, key in sorted(identity - pinned.keys() - mapped.keys()):
        fails.append(f"{cell} {key} is an identity difference with no pin and no mapping to an observed S-T record")
    for cell, key in sorted(identity & pinned.keys() & mapped.keys()):
        fails.append(f"{cell} {key} is both pinned (line {pinned[(cell, key)]}) and mapped (map line {mapped[(cell, key)][1]}): one account a record")
    for (cell, key), ((st_cell, st_key), n) in sorted(mapped.items()):
        if (cell, key) not in identity:
            fails.append(f"{cell} {key} is mapped (map line {n}) but is no identity difference")
        elif (st_cell, st_key) not in named:
            fails.append(f"{cell} {key} is mapped (map line {n}) to {st_cell} {st_key}, which does not differ")
        elif (st_cell, st_key) not in pinned:
            fails.append(f"{cell} {key} is mapped (map line {n}) to {st_cell} {st_key}, which has no pin")
    for cell, key in sorted(pinned.keys() - named - identity):
        fails.append(f"{cell} {key} is pinned (line {pinned[(cell, key)]}) but no longer differs")
    by_pin = len(identity & pinned.keys())
    summary = (f"each of the {len(named)} differences has its landing, and each of the {len(identity)} identity differences "
               f"of the encoding check its account ({len(identity) - by_pin} mapped to an observed S-T record and its pin, "
               f"{by_pin} pinned)")
    if not quiet:
        for f in fails:
            print(f"FAIL representation pins: {f}")
        if not fails:
            print(f"ok representation pins: {summary}")
    return not fails, fails, summary


def pins(pinfile, diffs, mapfile=None):
    return pins_check(pinfile, diffs, mapfile)[0]


def pins_selfcheck(pinfile, st_diff, ts_diff, enc_diff, mapfile, work, *more):
    """The pins check's own mutations (the pass on 3362b74bf), each against copies, each with the outcome it must
    have: every identity record deleted fails (their accounts are stale); an identity record added with no account
    fails; a mapped record pinned in place of its mapping passes (a pin of a present record is no stale one); a
    record both pinned and mapped fails; a pin whose landing names another part without its ledger entry fails."""
    os.makedirs(work, exist_ok=True)
    enc = open(enc_diff, encoding="utf-8").read().split("\n")
    maps = [l for l in open(mapfile, encoding="utf-8").read().split("\n")]
    pins_text = open(pinfile, encoding="utf-8").read()
    first = next(l for l in maps if l.strip() and not l.startswith("#"))
    e_cell, key, _, _ = first.split(" ")
    rel, _, path = key.partition(":")
    unassigned = f"I< {rel} {fact_json([json.loads(path), 'TYPEREF', 'unassigned', None])}"

    def run(name, enc_lines, map_lines, pin_text):
        e, m, pf = (os.path.join(work, f"{name}.{x}") for x in ("encoding.diff", "map.txt", "pins.txt"))
        open(e, "w", encoding="utf-8").write("\n".join(enc_lines))
        open(m, "w", encoding="utf-8").write("\n".join(map_lines))
        open(pf, "w", encoding="utf-8").write(pin_text)
        return pins_check(pf, [st_diff, ts_diff, e, *more], m, quiet=True)

    cases = [
        ("every identity record deleted", [l for l in enc if not l.startswith(("I< ", "I> "))], maps, pins_text, False),
        ("an identity record added with no account", enc + [unassigned], maps, pins_text, False),
        ("a mapped record pinned in place of its mapping", enc, [l for l in maps if l != first],
         pins_text + f"{e_cell} {key} 7 (7) the record pinned directly (the self-check's)\n", True),
        ("a record both pinned and mapped", enc, maps, pins_text + f"{e_cell} {key} 7 (7) pinned twice (the self-check's)\n", False),
        # A pin's landing names another part by its ledger entry, never by its name alone.
        ("a pin to another part without its ledger entry", enc, maps,
         pins_text + f"S-T:api object:repr.API.AA mixin-forwarders (the self-check's)\n", False),
    ]
    ok = True
    for name, e, m, p, want in cases:
        got, fails, _ = run(name.replace(" ", "-"), e, m, p)
        if got != want:
            ok = False
            print(f"FAIL representation pins self-check: {name}: the check {'passes' if got else 'fails'} ({(fails or ['no failure'])[0]})")
    if ok:
        print("ok representation pins self-check: every identity record deleted fails, an unaccounted one added fails, a "
              "mapped record pinned in place of its mapping passes, one both pinned and mapped fails, a pin to another "
              "part without its ledger entry fails")
    return ok


# The encoding (check 9, README.md "The encoding"): how each writer encodes a reference, read from the products
# through teq's tag-level dump (`teq tasty --nodes`, src/tasty/dump.rs, which names TERMREFin and a signed name's
# parts), never from the observation. A fact is a reference node's tag and its name as the dump shows it, by the path
# of the definitions enclosing it: a TERMREFin, TYPEREFin or SELECTin with its owner's name, a reference by symbol or
# address with the definition it names, a shared node read where it stands. Each twin's facts, scalac's against teq's,
# are kept as they are, nothing normalized away, and every difference classified: an encoding difference (`E`), a
# reference to the same name of the same owner written with another tag or signature, which pins.txt pins by its
# file and path (cell `E:<twin>`); an identity difference (`I`), a reference one writer writes and the other does not,
# which is the S·T cell's, observed and pinned there by member.
ENC_REF = {"TERMREF", "TERMREFin", "TERMREFdirect", "TERMREFsymbol", "TERMREFpkg", "TYPEREF", "TYPEREFin",
           "TYPEREFdirect", "TYPEREFsymbol", "TYPEREFpkg", "SELECT", "SELECTin"}
ENC_DEFS = {"VALDEF", "DEFDEF", "TYPEDEF", "PARAM", "TYPEPARAM"}


NUMERIC_FIELDS = {"len", "addr", "binder", "num", "value", "ref"}


def name_quote(s):
    """A string as `src/tasty/dump.rs`'s `quote` writes it: printable ASCII but space, `\\` and `"` escaped, every
    other UTF-16 unit `\\uXXXX`; the Python writer the record round trip and a structured name use."""
    raw = s.encode("utf-16-be", "surrogatepass")
    out = ['"']
    for i in range(0, len(raw), 2):
        unit = (raw[i] << 8) | raw[i + 1]
        if unit == 0x5c:
            out.append("\\\\")
        elif unit == 0x22:
            out.append('\\"')
        elif unit == 0x0a:
            out.append("\\n")
        elif unit == 0x0d:
            out.append("\\r")
        elif unit == 0x09:
            out.append("\\t")
        elif 0x21 <= unit <= 0x7e:
            out.append(chr(unit))
        else:
            out.append(f"\\u{unit:04x}")
    out.append('"')
    return "".join(out)


def name_json_text(n):
    """A structured name as the JSON value `src/tasty/dump.rs` writes: the same tags, fields and `quote`."""
    tag = n[0]
    if tag == "UTF8":
        return '["UTF8",' + name_quote(n[1]) + ']'
    if tag in ("QUALIFIED", "EXPANDED", "EXPANDPREFIX"):
        return '["' + tag + '",' + name_json_text(n[1]) + ',' + name_json_text(n[2]) + ']'
    if tag == "UNIQUE":
        return '["UNIQUE",' + name_json_text(n[1]) + ',' + str(n[2]) + ',' + (name_json_text(n[3]) if n[3] is not None else "null") + ']'
    if tag == "DEFAULTGETTER":
        return '["DEFAULTGETTER",' + name_json_text(n[1]) + ',' + str(n[2]) + ']'
    if tag in ("SUPERACCESSOR", "INLINEACCESSOR", "OBJECTCLASS", "BODYRETAINER"):
        return '["' + tag + '",' + name_json_text(n[1]) + ']'
    if tag in ("SIGNED", "TARGETSIGNED"):
        out = '["' + tag + '",' + name_json_text(n[1]) + ','
        if tag == "TARGETSIGNED":
            out += name_json_text(n[2]) + ','
        params = n[3] if tag == "TARGETSIGNED" else n[2]
        result = n[4] if tag == "TARGETSIGNED" else n[3]
        out += "[" + ",".join(
            ('["TYPEPARAMS",' + str(p[1]) + ']') if p[0] == "TYPEPARAMS" else name_json_text(p)
            for p in params) + "]," + name_json_text(result) + "]"
        return out
    return json.dumps(n, ensure_ascii=True, separators=(",", ":"))


def record_line(addr, parent, tag, fields):
    """A parsed record serialized back to its dump line, in the fields' order."""
    out = [str(addr), str(parent), tag]
    for key, value in fields:
        out.append(key + "=" + (name_json_text(value) if key == "name" else str(value)))
    return "\t".join(out)


def record_roundtrip(line, addr, parent, tag, fields, write=None):
    """The record reader's injectivity check: its parse of a line writes the line again."""
    got = (write or record_line)(addr, parent, tag, fields)
    if got != line:
        injectivity_fail("records", line, f"parsed-at-{addr}", got)


def names_injective(entries, serialize=None):
    """Within a product one `ref` names one value; two refs may share a serialization only when their structured
    values are equal, as dotty's name table allows repeated entries. Unequal values serialized alike fail."""
    serialize = serialize or name_json_text
    by_ref, by_name = {}, {}
    for ref, name in entries:
        text = serialize(name)
        if ref in by_ref and by_ref[ref][1] != name:
            injectivity_fail("names", by_ref[ref][1], name, text)
        if text in by_name and by_name[text][1] != name:
            injectivity_fail("names", by_name[text][1], name, text)
        by_ref[ref] = (text, name)
        by_name[text] = (ref, name)
    return len(entries)


def enc_nodes(path):
    """A product dump's records (`teq tasty --nodes`) as nodes: [depth, tag, None, children, fields], by address. A
    name record that shares its address with the definition or tree that follows it stays a record of its own (a
    tuple key), so every record is a node; the address's own key names the tree. Raises ValueError unless every line
    is a record, every parent an earlier record, no record is incomplete and the last line counts the records."""
    nodes, roots, seen, parent_of = {}, [], {}, {}
    end = None
    for n, line in enumerate(lines(path), start=1):
        f = line.split("\t")
        if f[0] == "incomplete":
            if len(f) != 3:
                raise ValueError(f"line {n}: the incomplete record is not incomplete\\t<addr>\\t<end>: {line[:120]}")
            raise ValueError(f"an incomplete record at {f[1]}, end {f[2]}")
        if f[0] == "end":
            if len(f) != 2 or not f[1].isdigit():
                raise ValueError(f"line {n}: the end line is not `end\\t<count>`: {line[:120]}")
            if end is not None:
                raise ValueError(f"line {n}: a second end line")
            end = int(f[1])
            continue
        if end is not None:
            raise ValueError(f"line {n}: a record after the end line")
        if len(f) < 3 or not f[0].isdigit() or not f[2]:
            raise ValueError(f"line {n} is not a record")
        addr = int(f[0])
        parent_field = f[1]
        tag = f[2]
        fields = {}
        for field in f[3:]:
            if "=" not in field:
                raise ValueError(f"line {n}: {field!r} is not a field")
            key, value = field.split("=", 1)
            if key in fields:
                raise ValueError(f"line {n}: the field {key} twice")
            if key in VALID_FIELDS:
                if key in NUMERIC_FIELDS:
                    try:
                        value = int(value)
                    except ValueError:
                        raise ValueError(f"line {n}: {key} is not a number: {value}")
                elif key == "name":
                    value = name_value(value)
                fields[key] = value
            else:
                raise ValueError(f"node record at {addr} has no field {key}")
        record_roundtrip(line, addr, parent_field, tag, list(fields.items()))
        if parent_field == "-":
            parent, depth = None, 0
        else:
            if not parent_field.isdigit():
                raise ValueError(f"line {n}: the parent is not an address or -: {parent_field}")
            p = int(parent_field)
            if p not in seen:
                raise ValueError(f"line {n}: parent {p} is not an earlier record")
            parent = seen[p]
            depth = nodes[parent][0] + 1
        key = addr
        if key in nodes:
            # A name record is printed at the address the next tree starts at; keep both records.
            moved = (addr, n)
            nodes[moved] = nodes.pop(key)
            pk = parent_of[key]
            if pk is None:
                roots[roots.index(key)] = moved
            else:
                kids = nodes[pk][3]
                kids[kids.index(key)] = moved
            parent_of[moved] = pk
            del parent_of[key]
        nodes[key] = [depth, tag, None, [], fields]
        parent_of[key] = parent
        if parent is None:
            roots.append(key)
        else:
            nodes[parent][3].append(key)
        seen[addr] = key
    if end is None:
        raise ValueError("the end line is missing")
    if end != len(nodes):
        raise ValueError(f"the end line counts {end} records, not {len(nodes)}")
    names_injective([(fields["ref"], fields["name"]) for _, _, _, _, fields in nodes.values()
                     if "ref" in fields and "name" in fields])
    return nodes, roots


VALID_FIELDS = {"len", "name", "ref", "addr", "binder", "num", "value"}


def name_value(value):
    """A name record's field as the dump wrote it: a JSON value, `["UTF8", <text>]` or a compound name's tag and
    fields. A consumer that looks a definition up by name reads a simple name's text (`tag_text`, `enc_text`); the
    fallback's evidence keeps the structure."""
    try:
        n = json.loads(value)
    except (ValueError, TypeError):
        raise ValueError(f"a name is not a JSON value: {value[:120]}")
    if not isinstance(n, list) or not n or not isinstance(n[0], str):
        raise ValueError(f"a name is not a structured value: {value[:120]}")
    return n


def enc_text(node):
    fields = node[4]
    if "name" in fields:
        n = fields["name"]
        return name_simple(n) if name_simple(n) is not None else n
    return str(fields["value"]) if "value" in fields else ""


def node_facts(nodes, roots):
    """A product dump's reference facts, each a structured value: the path as the names of its enclosing definitions,
    the tag, the reference's name (or the named definition's), and for an `in` reference its owner's name. Nothing is
    joined into text for the comparison."""
    defname = {a: enc_text(n) for a, n in nodes.items() if n[1] in ENC_DEFS}

    def target(a):  # a shared node's target, followed
        while a in nodes and nodes[a][1] in ("SHAREDtype", "SHAREDterm"):
            a = nodes[a][4]["addr"]
        return a

    def named(a):  # a reference node's name: its own, or the definition its address names
        n = nodes.get(a)
        if not n:
            return None
        if n[1].endswith("symbol") or n[1].endswith("direct"):
            return defname.get(n[4]["addr"])
        return enc_text(n)

    out = []

    def walk(a, path, seen):
        ind, tag, _, kids, fields = nodes[a]
        if tag in ENC_DEFS:
            path = path + [defname[a]]
        if tag in ("SHAREDtype", "SHAREDterm"):
            t = fields["addr"]
            if t in nodes and t not in seen:
                walk(t, path, seen | {t})
            return
        if tag in ENC_REF:
            owner = named(target(kids[-1])) if tag.endswith("in") and kids else None
            out.append((path, tag, named(a), owner))
        for k in kids:
            walk(k, path, seen)

    for r in roots:
        walk(r, [], frozenset())
    return out


def enc_facts(path):
    nodes, roots = enc_nodes(path)
    facts = node_facts(nodes, roots)
    facts_injective(facts)
    return facts


def name_key(n):
    """A name's stable key for classifying facts: a simple name's text before its signature, a signed name's original,
    or any other structure as JSON."""
    if isinstance(n, str):
        return re.split(r"[(\[]", n)[0]
    if isinstance(n, list) and n:
        if n[0] in ("SIGNED", "TARGETSIGNED"):
            return name_key(n[1])
        s = name_simple(n)
        if s is not None:
            return re.split(r"[(\[]", s)[0]
    return json.dumps(n, ensure_ascii=True, separators=(",", ":"))


def fact_key(f):
    """A fact's canonical key: its path, its tag-independent base name and its owner, as JSON values."""
    path, _, name, owner = f
    return (json.dumps(path, ensure_ascii=True, separators=(",", ":")), name_key(name),
            json.dumps(owner, ensure_ascii=True, separators=(",", ":")))


def fact_json(f):
    """A fact's JSON value: its path (a list of names), its tag, its name and its owner."""
    return json.dumps(f, ensure_ascii=True, separators=(",", ":"))


def facts_injective(facts, write=None):
    """Each fact's JSON reads back to its components; the fact writer's check."""
    write = write or fact_json
    for f in facts:
        text = write(f)
        try:
            got = json.loads(text)
        except (ValueError, TypeError):
            injectivity_fail("facts", fact_json(f), "joined parts", text)
        if got != list(f):
            injectivity_fail("facts", fact_json(f), "components", text)
    return len(facts)


def path_key(path):
    """A fact's path as a pin or map key: its JSON value, its spaces escaped so a line keeps one field."""
    return json.dumps(path, ensure_ascii=True, separators=(",", ":")).replace(" ", "\\u0020")


def fact_line(rel, f):
    """A fact's line in the encoding record: its file, then the whole fact as its JSON value, its path a list of
    structured names (never a text the path's names are joined into)."""
    return f"{rel} {fact_json(f)}"


def fact_lines(rel, facts, write=None):
    """The record's line writer's check: distinct facts (as structures) give distinct lines in one record."""
    write = write or fact_line
    by_line = {}
    for f in facts:
        key = fact_json(f)
        line = write(rel, f)
        if line in by_line and by_line[line] != key:
            injectivity_fail("lines", by_line[line], key, line)
        by_line[line] = key
    return len(facts)


def enc_classify(sc, tq):
    """The two multisets' differences: pairs of one path, base name and owner (`E`), and the rest (`I`). The facts are
    structured values, compared as such."""
    def key(f):
        return fact_json(f)
    a, b = collections.Counter(key(f) for f in sc), collections.Counter(key(f) for f in tq)
    by_key = {key(f): f for f in sc + tq}
    only_a = [by_key[k] for k in sorted((a - b).elements())]
    only_b = [by_key[k] for k in sorted((b - a).elements())]
    order = {key(f): (path_key(f[0]), f[1], fact_json(f)) for f in sc + tq}
    only_a.sort(key=lambda f: order[key(f)])
    only_b.sort(key=lambda f: order[key(f)])

    pool = collections.defaultdict(list)
    for f in only_b:
        pool[fact_key(f)].append(f)
    enc, ident_a = [], []
    for f in only_a:
        if pool[fact_key(f)]:
            enc.append((f, pool[fact_key(f)].pop(0)))
        else:
            ident_a.append(f)
    ident_b = sorted((f for v in pool.values() for f in v), key=lambda f: order[key(f)])
    return enc, ident_a, ident_b


def encoding(dumps, dest):
    """`<dumps>/<twin>/<file>.sc` and `.tq`, each twin's pickles dumped; writes the classified record, sections
    `## E:<twin>`, each line `E< `/`E> ` or `I< `/`I> ` then `<file> <fact_json>` (the whole fact, its path a list of
    structured names). A dump the reader cannot consume to its end is a failed observation: a FAIL line names the file
    and the reason, and the check fails."""
    out, ne, ni, ok = [], 0, 0, True
    for twin in sorted(os.listdir(dumps)):
        out.append(f"## E:{twin}")
        for sc in sorted(glob.glob(f"{dumps}/{twin}/**/*.sc", recursive=True)):
            rel = os.path.relpath(sc, f"{dumps}/{twin}")[:-3]
            tq = sc[:-3] + ".tq"
            try:
                sc_facts = enc_facts(sc)
            except ValueError as e:
                print(f"FAIL representation encoding: {rel}.sc: {e}")
                ok = False
                continue
            fact_lines(rel, sc_facts)
            if not os.path.exists(tq):
                out.append(f"I< {rel}:/ (teq writes no such pickle)")
                ni += 1
                continue
            try:
                tq_facts = enc_facts(tq)
            except ValueError as e:
                print(f"FAIL representation encoding: {rel}.tq: {e}")
                ok = False
                continue
            fact_lines(rel, tq_facts)
            enc, ia, ib = enc_classify(sc_facts, tq_facts)
            for x, y in enc:
                out += [f"E< {fact_line(rel, x)}", f"E> {fact_line(rel, y)}"]
            out += [f"I< {fact_line(rel, x)}" for x in ia]
            out += [f"I> {fact_line(rel, y)}" for y in ib]
            ne += len(enc)
            ni += len(ia) + len(ib)
    open(dest, "w", encoding="utf-8").write("\n".join(out) + "\n")
    print(f"{ne} encoding differences, {ni} identity differences")
    return ok


def code_only(src):
    """A Scala source with its comments and the text of its string and character literals blanked, an interpolation's
    `${...}` code kept: what the observer's code reads, never what it prints."""
    out, i, n = [], 0, len(src)

    def string(i):  # at an opening quote; the index after the closing one, the interpolations' code appended
        i += 1
        while i < n:
            c = src[i]
            if c == "\\":
                i += 2
                continue
            if c == '"':
                return i + 1
            if c == "$" and src.startswith("${", i):
                i += 2
                depth, start = 1, i
                while i < n and depth:
                    if src[i] == '"':
                        out.append(" " + src[start:i] + " ")
                        i = string(i)
                        start = i
                        continue
                    depth += {"{": 1, "}": -1}.get(src[i], 0)
                    i += 1
                out.append(" " + src[start:i - 1] + " ")
                continue
            i += 1
        return i

    while i < n:
        if src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            j = src.find("*/", i)
            i = n if j < 0 else j + 2
        elif src[i] == "'" and i + 2 < n and src[i + 2] == "'":
            i += 3
        elif src.startswith("'\\", i):
            j = src.find("'", i + 3)
            i = n if j < 0 else j + 1
        elif src[i] == '"':
            i = string(i)
            out.append(" ")
        else:
            out.append(src[i])
            i += 1
    return "".join(out)


KINDS = {"constructor", "operation", "not-product"}


DISPATCHERS = set()


def functions(code):
    """Each `def` of a code-only source, by name: its body (the lines below it indented deeper, nested defs included),
    the accessors its own text reads (`.name`, an extractor `Name(`) and the functions it names."""
    lines_ = code.split("\n")
    defs = []
    for i, l in enumerate(lines_):
        m = re.match(r"^(\s*)(?:inline |private |transparent )*def (\w+)", l)
        if m:
            ind, j = len(m.group(1)), i + 1
            while j < len(lines_) and (not lines_[j].strip() or len(lines_[j]) - len(lines_[j].lstrip()) > ind):
                j += 1
            defs.append((m.group(2), "\n".join(lines_[i:j])))
    names = {n for n, _ in defs}
    out = {}
    for n, body in defs:
        # The body from the def's own line on, its name and parameter list set aside: a one-line def's code counts.
        first, _, rest = body.partition("\n")
        text = first.split("=", 1)[1] if "=" in first else ""
        text += "\n" + rest
        reads = set(re.findall(r"\.(\w+)\b", text) + re.findall(r"\s(<:<|=:=)\s", text)) | set(re.findall(r"\b([A-Z]\w*)\(", text))
        calls = {c for c in re.findall(r"\b(\w+)\b", text) if c in names and c != n}
        r, c = out.get(n, (set(), set()))
        out[n] = (r | reads, c | calls)
        if re.search(r"^\s*case ", text, re.M):
            DISPATCHERS.add(n)
    return out


def closure(fns, start):
    """What `start` reads, transitively over the functions it names: (accessor tokens, functions reached)."""
    seen, todo, reads = set(), [start], set()
    while todo:
        f = todo.pop()
        if f in seen or f not in fns:
            continue
        seen.add(f)
        r, c = fns[f]
        reads |= r
        todo.extend(c)
    return reads, seen


def arm(code, fn, kind):
    """The text of `fn`'s `case` arm for the extractor or type test `kind`, up to the next `case` of that match."""
    fns_text = {}
    for m in re.finditer(r"^([ \t]*)def " + re.escape(fn) + r"\b", code, re.M):
        ind = len(m.group(1))
        rest = code[m.end():].split("\n")
        body = []
        for l in rest[1:]:
            if l.strip() and len(l) - len(l.lstrip()) <= ind:
                break
            body.append(l)
        fns_text = "\n".join(body)
        break
    if not fns_text:
        return None
    m = re.search(r"^([ \t]*)case [^\n]*\b" + re.escape(kind) + r"\b[^\n]*", fns_text, re.M)
    if not m:
        return None
    ind, out = len(m.group(1)), [m.group(0)]
    for l in fns_text[m.end():].split("\n")[1:]:
        if l.strip() and (len(l) - len(l.lstrip()) <= ind):
            break
        out.append(l)
    return "\n".join(out)


def labelled(fns_text):
    """The functions that record a label of their own (`hit("<label>")` in their body, not only in a `case` arm)."""
    return {n for n, t in fns_text.items() if re.search(r'^\s*hit\("' + re.escape(n) + r'"\)', t, re.M)}


def bodies(code):
    """Each function's body text, by name (a nested one's too), as `functions` delimits it."""
    lines_ = code.split("\n")
    out = {}
    for i, l in enumerate(lines_):
        m = re.match(r"^(\s*)(?:inline |private |transparent )*def (\w+)", l)
        if m:
            ind, j = len(m.group(1)), i + 1
            while j < len(lines_) and (not lines_[j].strip() or len(lines_[j]) - len(lines_[j].lstrip()) > ind):
                j += 1
            out[m.group(2)] = out.get(m.group(2), "") + "\n" + "\n".join(lines_[i:j])
    return out


def coverage(api, table, kinds_dir, sources, show_table=False):
    """README.md, "Coverage": every tree, type, constant, clause and selector kind 3.8.4's reflection has an extractor
    for is an arm of the observer's serializers, reached from the root path by a fixture; every accessor of the traits
    the observation reaches is read by the arm or the labelled serializer it names, itself reached, or excluded."""
    import glob
    listed, kinds = [], {}
    for l in lines(api):
        if l.startswith("kind "):
            _, root, kind = l.split(" ")
            kinds[kind] = root
        elif l and not l.startswith("=="):
            listed.append(l)
    raw = "\n".join(open(f, encoding="utf-8").read() for f in sources)
    code = "\n".join(code_only(open(f, encoding="utf-8").read()) for f in sources)
    # Reads are found in the code alone (literals and comments set aside); the labels an arm or a serializer records
    # (`hit("<label>")`, a literal) in the source itself.
    fns, texts = functions(code), bodies(code)
    labels = labelled(bodies(raw))
    hits_by = {}
    for f in sorted(glob.glob(f"{kinds_dir}/ss-*-kinds.txt")):
        cell = f.split("/")[-1][:-len("-kinds.txt")]
        for k in lines(f):
            if k:
                hits_by.setdefault(k, []).append(cell)
    rows, kind_rows, ok = {}, {}, True

    def fail(msg):
        nonlocal ok
        print(f"FAIL representation coverage: {msg}")
        ok = False

    def reach(text):
        """What a text reads, itself or through the functions it calls that record no label (a labelled one is
        another arm's or serializer's, which accounts for its own)."""
        reads = set(re.findall(r"\.(\w+)\b", text) + re.findall(r"\s(<:<|=:=)\s", text)) | set(re.findall(r"\b([A-Z]\w*)\(", text))
        seen, todo = set(), [c for c in re.findall(r"\b(\w+)\b", text) if c in fns]
        while todo:
            g = todo.pop()
            if g in seen or g in labels or g in KIND_FUNCTIONS:
                continue
            seen.add(g)
            reads |= fns[g][0]
            todo.extend(fns[g][1])
        return reads

    for n, line in enumerate(lines(table), start=1):
        if not line.strip() or line.startswith("#"):
            continue
        if line.startswith("kind "):
            m = re.match(r"^kind (\w+) (\w+)(?: (fails-on-meeting))?$", line)
            if not m:
                fail(f"line {n}: not `kind <kind> <function> [fails-on-meeting]`: {line}")
                continue
            kind_rows[m.group(1)] = (n, m.group(2), m.group(3))
            continue
        entry, _, rest = line.partition(" ")
        if entry in rows:
            fail(f"line {n}: {entry} has two lines")
        rows[entry] = (n, rest)
    KIND_FUNCTIONS.clear()
    KIND_FUNCTIONS.update(f for _, f, _ in kind_rows.values())
    # The kinds: each an arm of its function that records it and reached by a fixture, or one whose arm makes the
    # observation fail on meeting it (`Unobservable(..)`, which check.sh fails on): never an exemption.
    for kind in kinds:
        if kind not in kind_rows:
            fail(f"kind {kind} (below {kinds[kind]}) has no line in coverage.txt, so no arm")
    for kind, (n, fn, unreached) in sorted(kind_rows.items()):
        if kind not in kinds:
            fail(f"line {n}: {kind} is no kind of 3.8.4's reflection with an extractor")
            continue
        text = arm(raw, fn, kind)
        if text is None or f'hit("{kind}")' not in text:
            fail(f"line {n}: `{fn}` has no arm for {kind} that records it")
        elif unreached and "Unobservable(" not in text:
            fail(f"line {n}: {kind}'s arm does not make the observation fail on meeting it (`Unobservable(..)`)")
        reached = kind in hits_by
        if not unreached and not reached:
            fail(f"line {n}: no fixture reaches {kind} from a root (no observation of scalac's records it)")
        if unreached and reached:
            fail(f"line {n}: {kind} is met by {', '.join(hits_by[kind])}: give it its fixture's arm, not a failing one")
    # The accessors: each printed by the arm or labelled serializer it names, itself reached.
    for entry in listed:
        if entry not in rows:
            fail(f"{entry} is neither printed nor excluded (no line in coverage.txt)")
    for entry in sorted(rows.keys() - set(listed)):
        fail(f"line {rows[entry][0]}: {entry} is no accessor of 3.8.4's Quotes")
    for entry, (n, rest) in sorted(rows.items()):
        trait, _, name = entry.partition(".")
        status, _, why = rest.partition(" ")
        if status == "printed":
            label = why.strip()
            if label in kind_rows:
                text = arm(code, kind_rows[label][1], label) or ""
            elif label in labels:
                text = texts.get(label, "")
            else:
                fail(f"line {n}: {entry} is printed by `{label}`, which is no arm and no labelled serializer")
                continue
            kind = re.sub(r"(Module|Methods)$", "", trait)
            token = kind if trait.endswith("Module") and name == "unapply" else name
            if token not in reach(text):
                fail(f"line {n}: {entry} is printed by `{label}`, which never reads it")
            if label not in hits_by and not (label in kind_rows and kind_rows[label][2]):
                fail(f"line {n}: {entry} is printed by `{label}`, which no fixture reaches from a root")
        elif status == "excluded":
            kind, _, rest2 = why.partition(" ")
            if kind.endswith(":"):
                kind, rest2 = kind[:-1], ": " + rest2
            refs, _, reason = rest2.partition(": ")
            if kind not in KINDS or not reason.strip():
                fail(f"line {n}: {entry} is excluded as `{kind}`, not a constructor, an operation or a value not part of the product with what makes it so")
            elif kind == "operation":
                named = [r.strip() for r in refs.split(",") if r.strip()]
                if not named:
                    fail(f"line {n}: {entry} is an operation naming no printed accessor it derives from")
                for ref in named:
                    if ref != "self" and rows.get(ref, (0, ""))[1].split(" ")[0] != "printed":
                        fail(f"line {n}: {entry} is an operation on {ref}, which is not printed")
            elif refs.strip():
                fail(f"line {n}: {entry} names accessors for a `{kind}`, which derives from none")
        else:
            fail(f"line {n}: {entry} has no status (printed or excluded)")
    if ok:
        counts = {k: sum(1 for _, r in rows.values() if r.startswith(k)) for k in
                  ("printed", "excluded constructor", "excluded operation", "excluded not-product")}
        unreached = sum(1 for _, _, u in kind_rows.values() if u)
        print(f"ok representation coverage: each of the {len(kinds)} kinds 3.8.4's reflection extracts is an arm of the "
              f"serializers, {len(kinds) - unreached} reached from a root by a fixture and {unreached} whose arm fails "
              f"the observation on meeting it; each of the {len(listed)} accessors is printed by an arm or serializer a fixture reaches "
              f"({counts['printed']}) or excluded as a constructor ({counts['excluded constructor']}), an operation on "
              f"printed ones ({counts['excluded operation']}) or no part of the product ({counts['excluded not-product']})")
    if show_table:
        print("| kind | below | arm | reached by (scalac's observations) | accessors its arm prints |")
        print("|---|---|---|---|---|")
        for kind, (n, fn, unreached) in sorted(kind_rows.items(), key=lambda x: (kinds.get(x[0], ""), x[0])):
            acc = sorted(e for e, (_, r) in rows.items() if r == f"printed {kind}")
            where = ", ".join(hits_by.get(kind, [])) or "none: the observation fails on meeting it"
            print(f"| `{kind}` | {kinds.get(kind, '')} | `{fn}` | {where} | " +
                  (", ".join(f"`{a}`" for a in acc) or "its extractor's fields") + " |")
        print()
        print("| serializer | reached by | accessors it prints |")
        print("|---|---|---|")
        for lab in sorted(labels):
            acc = sorted(e for e, (_, r) in rows.items() if r == f"printed {lab}")
            if acc:
                fl = [a for a in acc if a.startswith("FlagsModule.")]
                rest_ = [a for a in acc if not a.startswith("FlagsModule.")]
                shown = ", ".join(f"`{a}`" for a in rest_) + (f", the {len(fl)} `FlagsModule` flags" if fl else "")
                print(f"| `{lab}` | {', '.join(hits_by.get(lab, [])) or 'none'} | {shown} |")
    return ok


KIND_FUNCTIONS = set()


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "split":
        split(*args)
    elif cmd == "gen":
        gen(*args)
    elif cmd == "teq":
        teq(*args)
    elif cmd == "strip":
        strip(*args)
    elif cmd == "encoding":
        sys.exit(0 if encoding(args[0], args[1]) else 1)
    elif cmd == "pins":
        # <pins> <diff>... [--map <encoding-map.txt>]
        mapfile = args[args.index("--map") + 1] if "--map" in args else None
        diffs = [a for k, a in enumerate(args[1:], 1) if a != "--map" and (k == 0 or args[k - 1] != "--map")]
        sys.exit(0 if pins(args[0], diffs, mapfile) else 1)
    elif cmd == "pins-selfcheck":
        sys.exit(0 if pins_selfcheck(*args) else 1)
    elif cmd == "detect-evidence":
        sys.exit(0 if detect_evidence(args[0]) else 1)
    elif cmd == "incomplete":
        sys.exit(0 if incomplete(args[0], args[1]) else 1)
    elif cmd == "unread-record":
        unread_record(*args)
    elif cmd == "fieldless":
        for line in lines(args[0]):
            print(outcome(line)[0])
    elif cmd == "incomplete-selfcheck":
        sys.exit(0 if incomplete_selfcheck(*args) else 1)
    elif cmd == "records-selfcheck":
        sys.exit(0 if records_selfcheck(*args) else 1)
    elif cmd == "identity-selfcheck":
        sys.exit(0 if identity_selfcheck(*args) else 1)
    elif cmd == "injectivity-selfcheck":
        sys.exit(0 if injectivity_selfcheck(*args) else 1)
    elif cmd == "key-selfcheck":
        sys.exit(0 if key_selfcheck() else 1)
    elif cmd == "excise":
        excise(*args)
    elif cmd == "tags":
        tags(*args)
    elif cmd == "coverage":
        table = args[0] == "--table"
        args = args[1:] if table else args
        sys.exit(0 if coverage(args[0], args[1], args[2], args[3:], table) else 1)
    else:
        sys.exit(f"oracle.py: no command {cmd}")
