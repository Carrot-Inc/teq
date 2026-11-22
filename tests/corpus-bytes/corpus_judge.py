#!/usr/bin/env python3
"""corpus_judge.py <repo> <equality out dir> [--expect-trees unit,unit,..]: the judge of a corpus-bytes run
(tests/corpus-bytes/equality.py <repo> <base> <head> <out> ..., which writes <out>/equality.tsv and the trees
<out>/master and <out>/head, each build's manifest and the table's sum). corpus_judge.py --self-test: the judge on
planted records, which it must fail on each but the good ones; it runs first in every judgment.

The manifest is the run's inputs: every tests/cases/*.scala and tests/tasty/src/*.scala of <repo>, each in the two
modes (check, jvm). The table must be what its sum says (<out>/equality.sum, written after it) and hold each input
exactly once (a missing row, a duplicated row or a stray row fails), and no row may be a timeout or an exit that
differs. Each tree must be what its producer wrote: per side, mode and input the manifest the run wrote right after
the build (<out>/sums/<side>/<mode>/<name>.sum: a line naming the unit, its number of files and the build's exit, then
each file's path, bytes and sha256), one for every input on each side and none for another; a manifest of another unit
or another side, one whose count of files is not its lines', a file the tree holds and its manifest does not, a file
its manifest holds and the tree does not, and a file of other bytes or another digest than its manifest's fail (a
file cut, the same on both sides, or another unit's copied in). Each build's exit, its manifest's, is judged by a
whitelist: 0, or teq's refusal named by its line in the build's log (<out>/logs/<side>/<mode>/<name>.log, whole by
its manifest's digest): 1 with its error line (`<file>:<line>:<column>: error:`) or the TASTy writer's summary (`<n>
definitions cannot be written as TASTy`), 3 with the JVM backend's (`<n> constructs not supported by the JVM
backend`); any other exit on either side (a signal's, 137 a kill or 139 a segmentation fault, 2, a timeout, none, 1
or 3 without its line) is not judged and fails. Then every file the two trees hold is compared: a file on one side
alone fails; a .tasty file is split into its sections (the name table and each section), and may differ in its
Positions section alone (its UUID follows from it); a .class file may differ in its TASTY attribute's UUID alone (each
.tasty's base UUID replaced by its head UUID); an empty or unreadable file fails; any other file that differs fails.
--expect-trees names the units whose trees (and names) may differ too, each named in the output: a unit listed there
whose trees do not differ fails as well, so the list is exact. Exits 1 on any failure."""
import glob
import hashlib
import os
import re
import shutil
import sys

MAGIC = b"\x5c\xa1\xab\x1f"


def nat(b, i):
    x = 0
    while True:
        c = b[i]
        i += 1
        x = (x << 7) | (c & 0x7F)
        if c & 0x80:
            return x, i


def tasty_sections(b):
    """(uuid, names, {section: bytes}); raises ValueError on a file that is not a whole TASTy file."""
    try:
        if b[:4] != MAGIC:
            raise ValueError("not a TASTy file")
        i = 4
        for _ in range(3):
            _, i = nat(b, i)
        n, i = nat(b, i)
        i += n
        uuid = b[i:i + 16]
        if len(uuid) < 16:
            raise ValueError("truncated header")
        i += 16
        size, i = nat(b, i)
        end = i + size
        if end > len(b):
            raise ValueError("truncated name table")
        names = []
        while i < end:
            tag = b[i]
            i += 1
            length, i = nat(b, i)
            names.append((tag, b[i:i + length]))
            i += length
        sections = {}
        while i < len(b):
            ref, i = nat(b, i)
            length, i = nat(b, i)
            if i + length > len(b):
                raise ValueError("truncated section")
            tag, payload = names[ref]
            sections[payload.decode("utf-8", "replace") if tag == 1 else str(ref)] = b[i:i + length]
            i += length
        return uuid, names, sections
    except IndexError:
        raise ValueError("truncated")


def unit_name(src):
    """The name of an input's unit: its file's name without `.scala`, `tasty-` before it for tests/tasty/src's."""
    return ("tasty-" if "/tests/tasty/" in src else "") + os.path.basename(src)[:-6]


def sha(data):
    return hashlib.sha256(data).hexdigest()


# teq's refusals by exit, each named by a line of the build's log: 1 with its error line or the TASTy writer's summary,
# 3 with the JVM backend's summary (the lines src/main.rs closes those refusals with).
TEQ_ERROR = re.compile(r"^[^\s:][^:]*:\d+:\d+: error:")
TASTY_REFUSAL = re.compile(r"^\d+ definitions? cannot be written as TASTy$")
JVM_REFUSAL = re.compile(r"^\d+ constructs? not supported by the JVM backend$")
REFUSALS = {"1": (TEQ_ERROR, TASTY_REFUSAL), "3": (JVM_REFUSAL,)}
# teq's internal error or a panic of its own: never a refusal, whatever summary follows it.
INTERNAL = re.compile(r"^internal error: |^thread '.*' panicked at ")


def log_path(out, side, mode, name):
    """Where a build's output is kept: outside the trees, which hold its products alone."""
    return os.path.join(out, "logs", side, mode, name + ".log")


def seal_unit(out, side, mode, name, code):
    """The manifest of what a build wrote, written by its producer right after it: `unit <side>/<mode>/<name>
    files=<n> exit=<exit> log=<bytes>:<sha256>` (`log=none` where it kept none), then per file
    `<side>/<mode>/<name>/<path> <bytes> <sha256>`."""
    uid = f"{side}/{mode}/{name}"
    lp = log_path(out, side, mode, name)
    log = "none"
    if os.path.isfile(lp):
        data = open(lp, "rb").read()
        log = f"{len(data)}:{sha(data)}"
    root = os.path.join(out, side, mode, name)
    lines = []
    for r, _, fs in os.walk(root):
        for f in fs:
            p = os.path.join(r, f)
            data = open(p, "rb").read()
            lines.append(f"{uid}/{os.path.relpath(p, root).replace(os.sep, '/')} {len(data)} {sha(data)}\n")
    path = os.path.join(out, "sums", side, mode, name + ".sum")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(f"unit {uid} files={len(lines)} exit={code} log={log}\n" + "".join(sorted(lines)))


def seal_table(table):
    """The table's sum, written after it: `<out's name>/equality.tsv <bytes> <sha256>` in <out>/equality.sum."""
    data = open(table, "rb").read()
    out = os.path.dirname(os.path.abspath(table))
    with open(os.path.join(out, "equality.sum"), "w", encoding="utf-8") as fh:
        fh.write(f"{os.path.basename(out)}/equality.tsv {len(data)} {sha(data)}\n")


def table_lines(out):
    """(the table's lines, None) where it is what its sum says, else (None, why)."""
    table, s = os.path.join(out, "equality.tsv"), os.path.join(out, "equality.sum")
    if not os.path.isfile(table):
        return None, "equality.tsv missing"
    if not os.path.isfile(s):
        return None, "equality.tsv with no sum of what its producer wrote"
    parts = open(s, encoding="utf-8", errors="replace").read().rstrip("\n").rsplit(" ", 2)
    want = f"{os.path.basename(os.path.abspath(out))}/equality.tsv"
    data = open(table, "rb").read()
    if len(parts) != 3 or parts[0] != want:
        return None, f"equality.tsv with another record's sum ({parts[0]!r}, where {want} is due)"
    if parts[1] != str(len(data)) or parts[2] != sha(data):
        return None, f"equality.tsv other than its producer wrote ({len(data)} bytes, its sum {parts[1]})"
    return data.decode("utf-8", "replace").splitlines(), None


def judged_exit(out, side, mode, name, code, log):
    """Why a build's exit is not judged, or None: 0, or a refusal of REFUSALS, its exit with its line in the build's log
    (whole by `log`, its manifest's `<bytes>:<sha256>`)."""
    if code == "0":
        return None
    if code not in REFUSALS:
        n = int(code) if code.isdigit() else None
        return (f"the build ended otherwise (exit {code}"
                + (f", killed by signal {n - 128}" if n and n > 128 else "") + ")")
    lp = log_path(out, side, mode, name)
    if not os.path.isfile(lp):
        return f"the build exited {code} and its log is missing"
    data = open(lp, "rb").read()
    if log != f"{len(data)}:{sha(data)}":
        return f"the build exited {code} and its log is other than its producer wrote"
    lines = data.decode("utf-8", "replace").splitlines()
    if any(INTERNAL.match(l) for l in lines):
        return f"the build exited {code} with an internal error in its log"
    if not any(p.match(l) for l in lines for p in REFUSALS[code]):
        return f"the build exited {code} without the line of its refusal in its log"
    return None


def sealed(out, side, want):
    """({path under the side's tree: (bytes, sha256)}, problems) of a side's manifests, one due per unit of `want`; a
    build whose exit the whitelist does not take (`judged_exit`) is one of the problems."""
    files, bad = {}, []
    root = os.path.join(out, "sums", side)
    have = set()
    for r, _, fs in os.walk(root):
        for f in fs:
            rel = os.path.relpath(os.path.join(r, f), root).replace(os.sep, "/")
            if rel.endswith(".sum") and rel.count("/") == 1:
                have.add(tuple(rel[:-4].split("/")))
            else:
                bad.append(f"{side}: a manifest of no unit ({rel})")
    for mode, name in sorted(have - want):
        bad.append(f"{side}/{mode}/{name}: a manifest of no input")
    for mode, name in sorted(want):
        uid = f"{side}/{mode}/{name}"
        path = os.path.join(root, mode, name + ".sum")
        if (mode, name) not in have:
            bad.append(f"{uid}: no manifest of the build")
            continue
        lines = open(path, encoding="utf-8", errors="replace").read().splitlines()
        head = lines[0].split() if lines else []
        if len(head) != 5 or head[0] != "unit" or not head[2].startswith("files=") or not head[3].startswith("exit=") or not head[4].startswith("log="):
            bad.append(f"{uid}: a manifest that is no manifest (no files, exit or log)")
            continue
        if head[1] != uid:
            bad.append(f"{uid}: the manifest of another unit ({head[1]})")
            continue
        entries = lines[1:]
        why = judged_exit(out, side, mode, name, head[3][5:], head[4][4:])
        if why:
            bad.append(f"{uid}: not judged: {why}")
        if head[2] != f"files={len(entries)}":
            bad.append(f"{uid}: a manifest of {head[2][6:]} files holding {len(entries)}")
        for e in entries:
            parts = e.rsplit(" ", 2)
            if len(parts) != 3 or not parts[0].startswith(uid + "/"):
                bad.append(f"{uid}: a manifest line of another unit ({parts[0]})")
                continue
            files[f"{mode}/{name}/{parts[0][len(uid) + 1:]}"] = (parts[1], parts[2])
    return files, bad


def manifest(repo):
    names = [os.path.basename(p)[:-6] for p in sorted(glob.glob(f"{repo}/tests/cases/*.scala"))]
    names += ["tasty-" + os.path.basename(p)[:-6] for p in sorted(glob.glob(f"{repo}/tests/tasty/src/*.scala"))]
    return {(m, n) for n in names for m in ("check", "jvm")}


def judge(repo, out, expect, say=print):
    bad = []
    rows = {}
    want = manifest(repo)
    lines, why = table_lines(out)
    if why:
        bad.append(why)
    for line in lines or []:
        parts = line.split("\t")
        if len(parts) < 3:
            bad.append(f"a malformed row: {line.strip()!r}")
            continue
        key = (parts[0], parts[1])
        if key in rows:
            bad.append(f"a duplicated row: {key[0]} {key[1]}")
        rows[key] = parts[2]
        if key not in want:
            bad.append(f"a stray row (no such input): {key[0]} {key[1]}")
        if parts[2] in ("timeout", "exit-differs"):
            bad.append(f"{key[0]} {key[1]}: {parts[2]} {parts[3] if len(parts) > 3 else ''}")
    for key in sorted(want - set(rows)):
        bad.append(f"a missing row: {key[0]} {key[1]}")
    base, head = os.path.join(out, "master"), os.path.join(out, "head")

    def files(root):
        res = set()
        for r, _, fs in os.walk(root):
            for f in fs:
                res.add(os.path.relpath(os.path.join(r, f), root))
        return res
    fb, fh = files(base), files(head)
    # Each tree against its manifests: every file the producer wrote, as it wrote it, and no other.
    units = {(m, n) for m, n in want}
    for side, root, present in (("master", base, fb), ("head", head, fh)):
        sums, problems = sealed(out, side, units)
        bad += problems
        for rel in sorted(present):
            key = rel.replace(os.sep, "/")
            if key not in sums:
                bad.append(f"{side}/{key}: a file its manifest does not hold")
                continue
            data = open(os.path.join(root, rel), "rb").read()
            if sums[key] != (str(len(data)), sha(data)):
                bad.append(f"{side}/{key}: other than its producer wrote ({len(data)} bytes, its manifest {sums[key][0]})")
        for key in sorted(set(sums) - {r.replace(os.sep, "/") for r in present}):
            bad.append(f"{side}/{key}: in its manifest, not in the tree")
    for rel in sorted(fb ^ fh):
        bad.append(f"{rel}: on {'the base' if rel in fb else 'the head'} alone")
    both = sorted(fb & fh)
    uuids = {}
    for rel in both:
        if rel.endswith(".tasty"):
            try:
                ua = tasty_sections(open(os.path.join(base, rel), "rb").read())[0]
                ub = tasty_sections(open(os.path.join(head, rel), "rb").read())[0]
                if ua != ub:
                    uuids[ua] = ub
            except ValueError:
                pass
    counts, trees = {}, set()
    for rel in both:
        a, b = open(os.path.join(base, rel), "rb").read(), open(os.path.join(head, rel), "rb").read()
        if not a or not b:
            bad.append(f"{rel}: empty on {'the base' if not a else 'the head'}")
            continue
        if a == b:
            continue
        if os.path.basename(rel) == "teq-products.json":
            continue  # equality.py compares the manifests itself (the row's verdict)
        unit = rel.split(os.sep)[1] if rel.count(os.sep) >= 1 else rel
        if rel.endswith(".tasty"):
            try:
                _, na, sa = tasty_sections(a)
                _, nb, sb = tasty_sections(b)
            except ValueError as e:
                bad.append(f"{rel}: unreadable ({e})")
                continue
            differ = sorted(k for k in set(sa) | set(sb) if sa.get(k) != sb.get(k))
            if na != nb:
                differ.append("<names>")
            others = [d for d in differ if d != "Positions"]
            key = "tasty: " + (" ".join(differ) or "the UUID alone")
            counts[key] = counts.get(key, 0) + 1
            if others:
                if unit in expect:
                    trees.add(unit)
                else:
                    bad.append(f"{rel}: {' '.join(differ)}")
        elif rel.endswith(".class"):
            mapped = a
            for ua, ub in uuids.items():
                mapped = mapped.replace(ua, ub)
            same = mapped == b
            counts["class: its TASTY attribute's UUID alone" if same else "class: OTHER BYTES"] = counts.get("class: its TASTY attribute's UUID alone" if same else "class: OTHER BYTES", 0) + 1
            if not same:
                bad.append(f"{rel}: class bytes beyond the TASTY attribute's UUID")
        else:
            bad.append(f"{rel}: differs")
    for u in sorted(expect - trees):
        bad.append(f"{u}: named in --expect-trees, and its trees do not differ")
    for k, v in sorted(counts.items()):
        say(f"{k}: {v}")
    say(f"units whose trees differ, as named: {' '.join(sorted(trees)) or 'none'}")
    for x in bad[:200]:
        say("FAIL " + x)
    say(f"corpus judge: {len(rows)} rows of {len(want)}, {sum(counts.values())} files differ, {len(bad)} failures")
    return 1 if bad else 0


def self_test():
    """The judge on planted runs, each sealed as equality.py seals it (each build's manifest, the table's sum): the good
    ones pass; each bad one fails. Then each run changed after its producer sealed it: the head's last section gone at a
    section boundary (its Positions: the rest still reads as "Positions alone"), the same bytes gone from the middle of
    a file on both sides, another unit's tree and manifest copied in, a manifest gone, a file put in after its manifest,
    a file and its manifest's line gone on both sides, and the table's last row gone (its line break kept), a row gone
    from its middle, a byte of a row changed, its sum gone, another run's table and sum copied in. A build's exit other
    than 0 or a refusal named by its line in its log (137, 2, a timeout, 139, none, 1 with no error line, 3 without
    the JVM backend's line) fails on either side; a refusal with its line on both sides passes: teq's error line or the
    TASTy writer's summary with exit 1, the JVM backend's summary with exit 3."""
    import tempfile

    def tasty(uuid, sections, names=(b"ASTs", b"Positions")):
        """A small well-formed TASTy file: header, a name table of `names`, the sections by their names."""
        def n(x):
            out = [x & 0x7F | 0x80]
            x >>= 7
            while x:
                out.insert(0, x & 0x7F)
                x >>= 7
            return bytes(out)
        tool = b"teq"
        table = b"".join(bytes([1]) + n(len(nm)) + nm for nm in names)
        body = b"".join(n(names.index(k)) + n(len(v)) + v for k, v in sections)
        return MAGIC + n(28) + n(8) + n(0) + n(len(tool)) + tool + uuid + n(len(table)) + table + body

    u1, u2 = b"\x01" * 16, b"\x02" * 16
    good_m = tasty(u1, [(b"ASTs", b"tree"), (b"Positions", b"p1")])
    good_h = tasty(u2, [(b"ASTs", b"tree"), (b"Positions", b"p2")])
    ast_h = tasty(u2, [(b"ASTs", b"TREE"), (b"Positions", b"p2")])
    cls_m, cls_h = b"CAFE" + u1 + b"rest", b"CAFE" + u2 + b"rest"

    def case(root, rows, base, head, after=None):
        """rows: the table's lines; base/head: {relative path: bytes}; after: a change of the run (its out dir) after
        its producer sealed it."""
        os.makedirs(os.path.join(root, "repo", "tests", "cases"))
        os.makedirs(os.path.join(root, "repo", "tests", "tasty", "src"))
        for name in ("a", "b"):
            open(os.path.join(root, "repo", "tests", "cases", name + ".scala"), "w").write("object A\n")
        out = os.path.join(root, "out")
        for side, fs in (("master", base), ("head", head)):
            for rel, data in fs.items():
                p = os.path.join(out, side, rel)
                os.makedirs(os.path.dirname(p), exist_ok=True)
                open(p, "wb").write(data)
        os.makedirs(out, exist_ok=True)
        for side in ("master", "head"):
            for mode, name in manifest(os.path.join(root, "repo")):
                lp = log_path(out, side, mode, name)
                os.makedirs(os.path.dirname(lp), exist_ok=True)
                open(lp, "w").write("")
                seal_unit(out, side, mode, name, 0)
        open(os.path.join(out, "equality.tsv"), "w").write("".join(r + "\n" for r in rows))
        seal_table(os.path.join(out, "equality.tsv"))
        if after:
            after(out)
        return os.path.join(root, "repo"), out

    full = ["check\ta\tdiffers\tx", "jvm\ta\tequal\t1 files exit 0", "check\tb\tequal\t1 files exit 0", "jvm\tb\tequal\t1 files exit 0"]
    files_ok = ({"check/a/A.tasty": good_m, "jvm/a/A.class": cls_m}, {"check/a/A.tasty": good_h, "jvm/a/A.class": cls_h})
    # A run holding a file alike on both sides, and the other unit's file, for the changes after the producer.
    same = b"CAFEBABE" + bytes(range(64))
    files_more = ({**files_ok[0], "jvm/a/B.class": same, "check/b/B.tasty": good_m},
                  {**files_ok[1], "jvm/a/B.class": same, "check/b/B.tasty": good_h})

    def write(out, rel, data):
        with open(os.path.join(out, rel), "wb") as fh:
            fh.write(data)

    def both(f):
        return lambda out: [f(out, side) for side in ("master", "head")]

    def drop_middle(out, side):
        p = os.path.join(out, side, "jvm/a/B.class")
        data = open(p, "rb").read()
        write(out, os.path.join(side, "jvm/a/B.class"), data[:20] + data[40:])

    def other_unit(out, side):
        shutil.rmtree(os.path.join(out, side, "check", "a"))
        shutil.copytree(os.path.join(out, side, "check", "b"), os.path.join(out, side, "check", "a"))
        shutil.copyfile(os.path.join(out, "sums", side, "check", "b.sum"), os.path.join(out, "sums", side, "check", "a.sum"))

    def file_and_line_gone(out, side):
        os.remove(os.path.join(out, side, "jvm/a/A.class"))
        p = os.path.join(out, "sums", side, "jvm", "a.sum")
        lines = open(p).read().splitlines(keepends=True)
        write(out, os.path.join("sums", side, "jvm", "a.sum"), "".join(l for l in lines if "/A.class " not in l).encode())

    def table_change(f):
        def g(out):
            p = os.path.join(out, "equality.tsv")
            write(out, "equality.tsv", f(open(p, "rb").read()))
        return g

    def other_run(out):
        o2 = os.path.join(os.path.dirname(out), "out2")
        os.makedirs(o2)
        open(os.path.join(o2, "equality.tsv"), "w").write("".join(r + "\n" for r in full))
        seal_table(os.path.join(o2, "equality.tsv"))
        for f in ("equality.tsv", "equality.sum"):
            shutil.copyfile(os.path.join(o2, f), os.path.join(out, f))

    cases = [
        ("positions alone (good)", full, *files_ok, set(), 0),
        ("trees named in --expect-trees (good)", full, {"check/a/A.tasty": good_m}, {"check/a/A.tasty": ast_h}, {"a"}, 0),
        ("a missing row", full[:-1], *files_ok, set(), 1),
        ("a duplicated row", full + [full[0]], *files_ok, set(), 1),
        ("a stray row", full + ["check\tzz\tequal\t1 files exit 0"], *files_ok, set(), 1),
        ("a timeout on a side", ["check\ta\ttimeout\ttimeout 0"] + full[1:], *files_ok, set(), 1),
        ("an exit that differs", ["check\ta\texit-differs\t0 1"] + full[1:], *files_ok, set(), 1),
        ("a file on the base alone", full, {"check/a/A.tasty": good_m, "check/a/B.tasty": good_m}, {"check/a/A.tasty": good_h}, set(), 1),
        ("a file on the head alone", full, {"check/a/A.tasty": good_m}, {"check/a/A.tasty": good_h, "check/a/B.tasty": good_h}, set(), 1),
        ("an empty file", full, {"check/a/A.tasty": good_m}, {"check/a/A.tasty": b""}, set(), 1),
        ("a truncated TASTy file", full, {"check/a/A.tasty": good_m}, {"check/a/A.tasty": good_h[:-3]}, set(), 1),
        ("trees that differ, not named", full, {"check/a/A.tasty": good_m}, {"check/a/A.tasty": ast_h}, set(), 1),
        ("a unit named whose trees do not differ", full, *files_ok, {"a"}, 1),
        ("a class beyond its UUID (a wrong value)", full, {"check/a/A.tasty": good_m, "jvm/a/A.class": cls_m}, {"check/a/A.tasty": good_h, "jvm/a/A.class": b"CAFE" + u2 + b"REST"}, set(), 1),
        ("more files, all sealed (good)", full, *files_more, set(), 0),
    ]
    after = [
        ("the head's last section gone at a section boundary", lambda out: write(out, "head/check/a/A.tasty", tasty(u2, [(b"ASTs", b"tree")]))),
        ("the same bytes gone from the middle of a file on both sides", both(drop_middle)),
        ("another unit's tree and manifest copied in on both sides", both(other_unit)),
        ("the head's manifest of a unit gone", lambda out: os.remove(os.path.join(out, "sums", "head", "check", "a.sum"))),
        ("a file put in on both sides after its manifest", both(lambda out, side: write(out, os.path.join(side, "jvm/a/C.class"), same))),
        ("a file and its manifest's line gone on both sides", both(file_and_line_gone)),
        ("the table's last row gone, its line break kept", table_change(lambda d: b"".join(d.splitlines(keepends=True)[:-1]))),
        ("a row gone from the table's middle", table_change(lambda d: b"".join(l for k, l in enumerate(d.splitlines(keepends=True)) if k != 2))),
        ("a byte of a row changed, its length kept", table_change(lambda d: d.replace(b"jvm\tb\tequal\t1 files exit 0", b"jvm\tb\tequal\t1 files exit 1"))),
        ("the table's sum gone", lambda out: os.remove(os.path.join(out, "equality.sum"))),
        ("another run's table and sum copied in", other_run),
    ]
    cases += [(name, full, *files_more, set(), 1, f) for name, f in after]

    # A build's exit by the whitelist: 0, or 1 with teq's error line in its log (a refusal, both sides' alike, passes).
    # A signal's (137 a kill, 139 a segmentation fault), 2, a timeout, none, and 1 with no error line fail, on each side.
    def exit_of(side, code, log=""):
        def g(out):
            lp = log_path(out, side, "check", "b")
            write(out, os.path.relpath(lp, out), log.encode())
            if code is None:
                p = os.path.join(out, "sums", side, "check", "b.sum")
                lines = open(p).read().splitlines(keepends=True)
                head = lines[0].split()
                write(out, os.path.relpath(p, out), (" ".join(head[:3] + head[4:]) + "\n" + "".join(lines[1:])).encode())
            else:
                seal_unit(out, side, "check", "b", code)
        return g
    refusal = "tests/cases/b.scala:1:8: error: Not found: type X\n1 error found\n"
    cases.append(("a refusal with teq's error line on both sides (good)", full, *files_more, set(), 0,
                  lambda out: (exit_of("master", 1, refusal)(out), exit_of("head", 1, refusal)(out))))
    for side in ("master", "head"):
        for what, code in (("137 (killed)", 137), ("2", 2), ("a timeout", "timeout"), ("139", 139), ("no exit", None)):
            cases.append((f"the {side}'s build {what}", full, *files_more, set(), 1, exit_of(side, code)))
        cases.append((f"the {side}'s build 1 with no error line", full, *files_more, set(), 1, exit_of(side, 1, "compiling\ndone\n")))
        cases.append((f"the {side}'s build 3 without the JVM backend's line", full, *files_more, set(), 1, exit_of(side, 3, "compiling\ndone\n")))
        cases.append((f"the {side}'s build 1 with an internal error before the TASTy writer's summary", full, *files_more, set(), 1,
                      exit_of(side, 1, "internal error: tests/cases/b.scala: parsed without the record of the places the TASTy writer reads\n1 definition cannot be written as TASTy\n")))
    # teq's two refusals closed by a summary line (src/main.rs), on both sides alike, pass.
    tasty_log = "tests/cases/b.scala: cannot write TASTy for a type that did not check in N1\n1 definition cannot be written as TASTy\n"
    jvm_log = "jvm backend: b$package$.run: the local o is not in scope of the generated method\n2 constructs not supported by the JVM backend\n"
    cases.append(("the TASTy writer's refusal, exit 1, on both sides (good)", full, *files_more, set(), 0,
                  lambda out: (exit_of("master", 1, tasty_log)(out), exit_of("head", 1, tasty_log)(out))))
    cases.append(("the JVM backend's refusal, exit 3, on both sides (good)", full, *files_more, set(), 0,
                  lambda out: (exit_of("master", 3, jvm_log)(out), exit_of("head", 3, jvm_log)(out))))
    tmp = tempfile.mkdtemp(prefix="corpus-judge-self-test-")
    wrong = []
    try:
        for k, (name, rows, base, head, expect, want, *f) in enumerate(cases):
            repo, out = case(os.path.join(tmp, str(k)), rows, base, head, f[0] if f else None)
            got = judge(repo, out, expect, say=lambda *_: None)
            if got != want:
                wrong.append(f"{name}: exit {got}, {want} expected")
    finally:
        if tmp.startswith(tempfile.gettempdir()):
            shutil.rmtree(tmp, ignore_errors=True)
    for w in wrong:
        print("corpus judge self-test: NOT AS EXPECTED: " + w)
    print(f"corpus judge self-test: {len(cases)} cases, {len(cases) - len(wrong)} as expected")
    return 1 if wrong else 0


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    if self_test():
        print("corpus judge: the self-test failed, nothing judged")
        return 1
    repo, out = sys.argv[1], sys.argv[2]
    expect = set()
    if len(sys.argv) > 4 and sys.argv[3] == "--expect-trees":
        expect = set(x for x in sys.argv[4].split(",") if x)
    return judge(repo, out, expect)


if __name__ == "__main__":
    sys.exit(main())
