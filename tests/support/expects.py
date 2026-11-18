#!/usr/bin/env python3
"""The expectations of tests/errors (tests/run_errors.sh): their continuation lines and the writing verb.

A program states a diagnostic it must give by `// expect: <text>`, a fixed string looked for in its output (the
runner's grep). A diagnostic of several lines states its further lines by `// expect+ <line>` lines right after its
`// expect:` (`// expect+` alone for a blank line), or, where the expectation stands above the program's code (a line
added there would move the code), joined to its own line by `⏎`: the expectation then holds only where a line of the
output holds its first text and its further lines are those lines exactly, whole, in order, none more.

  expects.py continuations <record> <source>...
      the continuation lines of the sources against the output of a record (out/errors-tests/<name>.out): prints the
      first that does not hold and exits 1, else exits 0.
  expects.py write <errors dir> <base records> <head records> <pending list> [--apply]
      the writing verb (`tests/run_errors.sh --write <base binary>`). In every program, those on the pending list
      too, the HEAD's diagnostics must stand at the BASE's places and severities, as many at each, or the verb stops
      and writes nothing; the files of the programs on the list are never written. On the base, each expectation that
      names a place (`L:C: <severity>: ..`, or `<file>:L:C: ..`) is mapped to the one diagnostic at that place and
      severity (and file) that holds it, then bound to that diagnostic's file, its ordinal counted among that file's
      diagnostics at that place and severity; one no diagnostic there holds, or several, and two on one diagnostic,
      are refused, as is every expectation without a place but a summary's (`N errors found`): it is left as it is.
      A mapped expectation the head's diagnostic of the same file, place, severity and ordinal no longer holds is
      rewritten to that diagnostic's first line from its place on (the file kept where the expectation names one) and
      its further lines as `// expect+` lines; one that holds stays. After the rewrite each mapped expectation must be
      held by exactly the head's diagnostic it was written from among those at its place, no two by one, or the
      program is refused and none of its files written. A `<name>.check` the base's output was and the head's is not
      becomes the head's output. The files keep every byte but the rewritten lines' text (their line ends, a last
      line's end or its lack). THE VERB NEEDS TO CREATE FILES BESIDE THE SOURCES: each file is written beside itself (a
      symbolic link's beside its target, the link kept) and renamed over itself once all are written; a file or a
      directory it cannot write stops it before anything is written, the directory named; a failure before the renames
      changes nothing, and one among them is told as a partial write, the files replaced and those not named (exit 3
      either way). Prints what it rewrites and every refusal by kind; --apply writes the files. Run with one binary as
      both base and head it rewrites nothing.
  expects.py place <errors dir> <records> <pending list> <words> [--apply]
      every expectation without a place whose text holds <words> (a regular expression), in a program not on the
      pending list, gets the place of the one diagnostic whose first line holds it in the records: the first line
      from its place up to the end of the text it held. One held by no first line, or by several, is listed.
  expects.py test
      the verb and the continuation rule on fixtures of their own (an identity, a rewrite, two expectations at one
      place, a diagnostic moved here, in another program or in a program on the pending list, an expectation two
      diagnostics hold on the base or after the rewrite on the head, two expectations on one diagnostic, an
      expectation without a place, a summary, one file of two whose diagnostics change order, a failed write, a file
      that cannot be written, the second of three renames failing, a source that is a symbolic link, line ends of two
      bytes and a last line without one, a continuation line held only in part)."""
import collections
import io
import os
import re
import shutil
import sys
import tempfile

PLACED = re.compile(r"^(?:(?P<file>[^\s:]+\.(?:scala|sc)):)?(?P<line>\d+):(?P<col>\d+): (?P<sev>error|warning|migration warning|info): ")
FIRST = re.compile(r"^(?P<file>\S+?\.(?:scala|sc)):(?P<line>\d+):(?P<col>\d+): (?P<sev>error|warning|migration warning|info): ")
CARET = re.compile(r"^\s*\^+\s*$")
SUMMARY = re.compile(r"^\d+ (?:errors?|warnings?)(?: and \d+ warnings?)? found$")
# An expectation of the summary, which the diagnostics' places and severities, as many at each, hold.
SUMMARY_EXPECTED = re.compile(r"^\d+ (?:errors?|warnings?)(?: and \d+ warnings?)? found(?:, errors under --werror)?$")


JOIN = "\u23ce"


def expectation_lines(lines):
    """The `// expect:` lines of a source's lines in order: [line index, text, [continuation indices], [continuation
    texts]]; a text joined by `⏎` gives its first part as the text and the rest as its continuation texts."""
    exps = []
    for i, l in enumerate(lines):
        if l.startswith("// expect: "):
            first, *rest = l[len("// expect: "):].split(JOIN)
            exps.append([i, first, [], rest])
        elif (l == "// expect+" or l.startswith("// expect+ ")) and exps and i - 1 in [exps[-1][0]] + exps[-1][2]:
            exps[-1][2].append(i)
            exps[-1][3].append(l[len("// expect+ "):])
    return exps


def expectations(text):
    """A source's lines (without their ends, a `\\r\\n` as one) and its expectations (`expectation_lines`)."""
    lines = [l[:-1] if l.endswith("\r") else l for l in text.split("\n")]
    return lines, expectation_lines(lines)


def split_lines(raw):
    """A source's bytes as its lines, each (its text, its end): b"\\r\\n", b"\\n", or b"" for a last line without one;
    their concatenation is the source, byte for byte."""
    parts = raw.split(b"\n")
    lines = []
    for i, part in enumerate(parts):
        if i == len(parts) - 1:
            if part:
                lines.append((part, b""))
        elif part.endswith(b"\r"):
            lines.append((part[:-1], b"\r\n"))
        else:
            lines.append((part, b"\n"))
    return lines


# Set by the script's own test: the first file written fails half way; the rename of this index (from 0) fails.
FAIL_WRITE = False
FAIL_RENAME_AT = None


class PartialWrite(OSError):
    """A failure among the renames, after some: the files replaced and those not are named."""


def write_files(changed):
    """`changed`, {path: bytes}: each file written where its path leads (a symbolic link's target, the link kept as it
    is), to a temporary file beside it, flushed and closed with every error checked; only when all are written is each
    renamed over its file. The verb needs to create files beside the sources: a file or a directory that cannot be
    written stops it before anything is written, as does one file given two contents by two paths. A failure before
    the renames changes nothing; a failure among them removes the temporary files left and raises PartialWrite, which
    names the files replaced and those not."""
    targets = {}
    for path, data in changed.items():
        real = os.path.realpath(path)
        if real in targets and targets[real][1] != data:
            raise OSError(f"{path} and {targets[real][0]} are one file, given two contents")
        targets.setdefault(real, (path, data))
    for real, (path, _) in targets.items():
        d = os.path.dirname(real) or "."
        if os.path.exists(real) and not os.access(real, os.W_OK):
            raise OSError(f"{path} cannot be written")
        if not os.access(d, os.W_OK | os.X_OK):
            raise OSError(f"no file can be made in {d}, where {path} is: the verb writes each file beside itself and renames it over it")
    temps = []
    try:
        for real, (path, data) in targets.items():
            fd, tmp = tempfile.mkstemp(prefix=".expects.", dir=os.path.dirname(real) or ".")
            temps.append((tmp, real, path))
            with os.fdopen(fd, "wb") as f:
                if FAIL_WRITE:
                    f.write(data[:len(data) // 2])
                    raise OSError("a write failed half way (the script's own test)")
                if f.write(data) != len(data):
                    raise OSError(f"{path}: a short write")
                f.flush()
                os.fsync(f.fileno())
            if os.path.exists(real):
                shutil.copymode(real, tmp)
    except BaseException:
        for tmp, _, _ in temps:
            if os.path.exists(tmp):
                os.remove(tmp)
        raise
    replaced = []
    try:
        for i, (tmp, real, path) in enumerate(temps):
            if FAIL_RENAME_AT == i:
                raise OSError("a rename failed (the script's own test)")
            os.replace(tmp, real)
            replaced.append(path)
    except OSError as e:
        for tmp, _, _ in temps:
            if os.path.exists(tmp):
                os.remove(tmp)
        if replaced:
            rest = [path for _, _, path in temps if path not in replaced]
            raise PartialWrite(f"{e}; the write is partial: replaced {', '.join(replaced)}; not replaced {', '.join(rest)}") from e
        raise


def output(record):
    raw = open(record, encoding="utf-8", errors="replace").read()
    first, _, out = raw.partition("\n")
    return first, out


def diagnostics(out):
    """The diagnostics of an output: (file, line, col, severity, first line from the place on, [further lines])."""
    found, cur, block = [], None, []

    def close():
        if cur is None:
            return
        body = list(block)
        # the source excerpt (a line of the source, two spaces in) and its caret end the message's further lines
        carets = [j for j, l in enumerate(body) if CARET.match(l) and j > 0 and body[j - 1].startswith("  ")]
        if carets:
            body = body[:carets[0] - 1]
        else:
            # without a position, the summaries after the message are no part of it
            while body and (SUMMARY.match(body[-1]) or body[-1].startswith("warning: there w")):
                body.pop()
        while body and body[-1] == "":
            body.pop()
        found.append(cur + (body,))

    for l in out.split("\n"):
        m = FIRST.match(l)
        if m:
            close()
            cur = (os.path.basename(m.group("file")), int(m.group("line")), int(m.group("col")), m.group("sev"), l[m.start("line"):])
            block = []
        elif SUMMARY.match(l):
            close()
            cur, block = None, []
        elif cur is not None:
            block.append(l)
    close()
    return found


def held_whole(out, text, conts):
    """Whether a diagnostic of the output holds `text` on its first line and its further lines are `conts`, exactly:
    no line more, no line less (the source excerpt and its caret are no part of a message)."""
    m = PLACED.match(text)
    body = text[m.start("line"):] if m and m.group("file") else text
    return any(body in d[4] and d[5] == conts for d in diagnostics(out))


def continuations(record, sources):
    _, out = output(record)
    for src in sources:
        _, exps = expectations(open(src, encoding="utf-8").read())
        for _, text, _, conts in exps:
            if conts and not held_whole(out, text, conts):
                return f"continuation of '{text}' not held whole"
    return None


def holds(body, conts, d):
    return body in d[4] and (not conts or d[5] == conts)


def programs(errors_dir):
    for n in sorted(os.listdir(errors_dir)):
        p = os.path.join(errors_dir, n)
        if n.endswith(".scala"):
            yield n[:-6], [p]
        elif os.path.isdir(p):
            yield n, sorted(os.path.join(p, f) for f in os.listdir(p) if f.endswith((".scala", ".sc")))


def read_source(path):
    """A source as its lines (`split_lines`) and their texts."""
    lines = split_lines(open(path, "rb").read())
    return lines, [c.decode("utf-8", "surrogateescape") for c, _ in lines]


def rebuilt(lines, replaced):
    """The bytes of `lines` with each line of `replaced` ({index: [texts]}, an empty list removing it) in its place:
    the new lines take the replaced line's end, and the source keeps its last line's end, or its lack of one."""
    newline = b"\r\n" if any(e == b"\r\n" for _, e in lines) else b"\n"
    out = []
    for i, (c, e) in enumerate(lines):
        if i not in replaced:
            out.append((c, e))
            continue
        texts = [t.encode("utf-8", "surrogateescape") for t in replaced[i]]
        for j, t in enumerate(texts):
            out.append((t, e if j == len(texts) - 1 or e else newline))
    if lines and out and lines[-1][1] == b"" and out[-1][1] != b"":
        out[-1] = (out[-1][0], b"")
    return b"".join(c + e for c, e in out)


def write(errors_dir, base_dir, head_dir, pending_path, apply):
    pending = {l.split("\t")[0].strip() for l in open(pending_path) if l.strip()} if os.path.exists(pending_path) else set()
    rewrites, refused, changed = [], collections.defaultdict(list), {}
    places = lambda ds: collections.Counter((d[0], d[1], d[2], d[3]) for d in ds)
    runs, stopped = [], []
    for name, srcs in programs(errors_dir):
        bp, hp = os.path.join(base_dir, name + ".out"), os.path.join(head_dir, name + ".out")
        if not (os.path.exists(bp) and os.path.exists(hp)):
            continue
        (_, bout), (_, hout) = output(bp), output(hp)
        base, head = diagnostics(bout), diagnostics(hout)
        # Every program, those on the pending list too: their files are never written.
        if places(base) != places(head):
            stopped.append(name + (" (on the pending list)" if name in pending else ""))
        if name not in pending:
            runs.append((name, srcs, bout, hout, base, head))
    # The head's diagnostics stand where the base's do, as many at each place and severity, in every program, or
    # nothing is written: a rewrite is of words alone.
    if stopped:
        print(f"the writing verb stops: in {len(stopped)} programs the head's diagnostics are not the base's places and severities, as many at each; nothing is written")
        for name in stopped:
            print(f"    {name}")
        return 2
    for name, srcs, bout, hout, base, head in runs:
        program_changed, program_rewrites = {}, []
        check = os.path.join(errors_dir, name + ".check")
        if os.path.exists(check) and name + ".check" not in pending:
            text = open(check, encoding="utf-8").read()
            if text.rstrip("\n") == bout.rstrip("\n") and text.rstrip("\n") != hout.rstrip("\n"):
                program_changed[check] = (hout.rstrip("\n") + "\n").encode("utf-8")
                program_rewrites.append((name + ".check", "the whole output", ["the head's output"]))
        sources = []
        for src in srcs:
            lines, texts = read_source(src)
            sources.append((src, lines, texts, expectation_lines(texts)))
        # The map, on the base: each expectation that names a place to the one diagnostic at that place and severity
        # (and file, where it names one) that holds it; then bound to that diagnostic's own file, its ordinal counted
        # among the diagnostics of that file, place and severity.
        mapped, owners = [], collections.defaultdict(list)
        for src, lines, texts, exps in sources:
            for idx, text, cont_idx, conts in exps:
                m = PLACED.match(text)
                if not m:
                    if not SUMMARY_EXPECTED.match(text):
                        refused["an expectation without a place (a fixed string, left as it is)"].append(f"{name}: {text}")
                    continue
                file, body = m.group("file"), text[m.start("line"):]
                key = (int(m.group("line")), int(m.group("col")), m.group("sev"))
                holders = [j for j, d in enumerate(base) if (d[1], d[2], d[3]) == key and (file is None or d[0] == file) and holds(body, conts, d)]
                if len(holders) != 1:
                    kind = "no diagnostic of the base at its place holds it" if not holders else "more than one diagnostic of the base at its place holds it"
                    refused[kind].append(f"{name}: {text}")
                    continue
                k = holders[0]
                bound = (base[k][0],) + key
                at = lambda ds, bound=bound: [j for j, d in enumerate(ds) if (d[0], d[1], d[2], d[3]) == bound]
                owners[k].append(f"{name}: {text}")
                mapped.append((src, idx, text, cont_idx, conts, file, body, key, k, at))
        for k, held in owners.items():
            if len(held) > 1:
                refused["two expectations on one diagnostic"].extend(held)
        mapped = [x for x in mapped if len(owners[x[8]]) == 1]
        # The rewrite, on the head: from the diagnostic of the same file, place, severity and ordinal.
        after = []
        for src, lines, texts, _ in sources:
            replaced = {}
            code = [j for j, l in enumerate(texts) if l.strip() and not l.lstrip().startswith("//")]
            last_code = code[-1] if code else -1
            for _, idx, text, cont_idx, conts, file, body, key, k, at in [x for x in mapped if x[0] == src]:
                j = at(head)[at(base).index(k)]
                hd = head[j]
                if holds(body, conts, hd):
                    after.append((text, file, body, conts, key, j))
                    continue
                head_text = (f"{file}:" if file else "") + hd[4]
                if idx > last_code:
                    block = ["// expect: " + head_text] + ["// expect+ " + c if c else "// expect+" for c in hd[5]]
                elif cont_idx:
                    refused["continuation lines above the code"].append(f"{name}: {text}")
                    continue
                else:
                    # above the code a line added would move it: the further lines join the expectation's own line
                    block = ["// expect: " + JOIN.join([head_text] + hd[5])]
                after.append((block[0], file, hd[4], hd[5], key, j))
                program_rewrites.append((name, text, block))
                replaced[idx] = block
                for ci in cont_idx:
                    replaced[ci] = []
            if replaced:
                program_changed[src] = rebuilt(lines, replaced)
        # After the rewrite, each expectation the map took is held by exactly the head's diagnostic it was written from,
        # among the head's diagnostics at its place (of its file, where it names one), and no two by one; else the
        # program is refused and none of its files is written.
        unsure = [t for t, file, body, conts, key, j in after
                  if [i for i, d in enumerate(head) if (d[1], d[2], d[3]) == key and (file is None or d[0] == file) and holds(body, conts, d)] != [j]]
        by = collections.Counter(j for *_, j in after)
        unsure += [t for t, *_, j in after if by[j] > 1]
        if unsure:
            refused["after the rewrite, held by none or several of the head's diagnostics, or two on one: the program is left"].extend(f"{name}: {t}" for t in unsure)
            continue
        changed.update(program_changed)
        rewrites.extend(program_rewrites)
    for name, old, block in rewrites:
        print(f"rewrite {name}: {old}")
        for b in block:
            print(f"    {b}")
    for kind, items in refused.items():
        print(f"refused, {kind}: {len(items)}")
        for it in items:
            print(f"    {it}")
    print(f"the writing verb: {len(rewrites)} rewritten in {len(changed)} files, {sum(len(v) for v in refused.values())} refused")
    if apply and changed:
        try:
            write_files(changed)
        except PartialWrite as e:
            print(f"the writing verb stops: {e}")
            return 3
        except OSError as e:
            print(f"the writing verb stops: {e}; nothing is written")
            return 3
    return 0 if not refused else 1


def place(errors_dir, records, pending_path, words, apply):
    pending = {l.split("\t")[0].strip() for l in open(pending_path) if l.strip()} if os.path.exists(pending_path) else set()
    pattern = re.compile(words)
    placed, left, changed = [], [], {}
    for name, srcs in programs(errors_dir):
        rec = os.path.join(records, name + ".out")
        if name in pending or not os.path.exists(rec):
            continue
        _, out = output(rec)
        ds = diagnostics(out)
        for src in srcs:
            lines, texts = read_source(src)
            replaced = {}
            for idx, text, _, _ in expectation_lines(texts):
                if PLACED.match(text) or not pattern.search(text):
                    continue
                holders = [d for d in ds if text in d[4]]
                if len(holders) != 1:
                    left.append(f"{name}: {text} ({len(holders)} first lines hold it)")
                    continue
                line = holders[0][4]
                replaced[idx] = ["// expect: " + line[:line.index(text) + len(text)]]
                placed.append((name, text, replaced[idx][0]))
            if replaced:
                changed[src] = rebuilt(lines, replaced)
    for name, old, line in placed:
        print(f"place {name}: {old}\n    {line}")
    for it in left:
        print(f"left: {it}")
    print(f"placed {len(placed)} in {len(changed)} files; left {len(left)}")
    if apply and changed:
        try:
            write_files(changed)
        except PartialWrite as e:
            print(f"the placing verb stops: {e}")
            return 3
        except OSError as e:
            print(f"the placing verb stops: {e}; nothing is written")
            return 3
    return 0


def test():
    """The verb and the rule on fixtures; exits 1 at the first that does not hold."""
    def run(src_text, base_out, head_out, pending="", other=None):
        d = tempfile.mkdtemp(prefix="expects-test.")
        for sub in ("errors", "base", "head"):
            os.makedirs(os.path.join(d, sub))
        for name, (s, b, h) in [("p", (src_text, base_out, head_out))] + ([("q", other)] if other else []):
            open(os.path.join(d, "errors", name + ".scala"), "w").write(s)
            open(os.path.join(d, "base", name + ".out"), "w").write("exit 1\n" + b)
            open(os.path.join(d, "head", name + ".out"), "w").write("exit 1\n" + h)
        open(os.path.join(d, "pending.txt"), "w").write(pending)
        devnull = open(os.devnull, "w")
        saved, sys.stdout = sys.stdout, devnull
        try:
            code = write(os.path.join(d, "errors"), os.path.join(d, "base"), os.path.join(d, "head"), os.path.join(d, "pending.txt"), True)
        finally:
            sys.stdout = saved
        result = open(os.path.join(d, "errors", "p.scala")).read()
        for root, dirs, files in os.walk(d, topdown=False):
            for f in files:
                os.remove(os.path.join(root, f))
            for x in dirs:
                os.rmdir(os.path.join(root, x))
        os.rmdir(d)
        return code, result

    def run_files(progs, pending="", capture=False):
        """progs, {name: ({path in tests/errors: bytes}, base output, head output)}: the verb's exit and every file's
        bytes after it, the temporary files of a failed write among them if any were left."""
        d = tempfile.mkdtemp(prefix="expects-test.")
        for sub in ("errors", "base", "head"):
            os.makedirs(os.path.join(d, sub))
        for name, (files, b, h) in progs.items():
            for rel, data in files.items():
                path = os.path.join(d, "errors", rel)
                os.makedirs(os.path.dirname(path), exist_ok=True)
                open(path, "wb").write(data)
            open(os.path.join(d, "base", name + ".out"), "w").write("exit 1\n" + b)
            open(os.path.join(d, "head", name + ".out"), "w").write("exit 1\n" + h)
        open(os.path.join(d, "pending.txt"), "w").write(pending)
        out = io.StringIO()
        saved, sys.stdout = sys.stdout, out
        try:
            code = write(os.path.join(d, "errors"), os.path.join(d, "base"), os.path.join(d, "head"), os.path.join(d, "pending.txt"), True)
        finally:
            sys.stdout = saved
        result = {}
        for root, _, fs in os.walk(os.path.join(d, "errors")):
            for f in fs:
                result[os.path.relpath(os.path.join(root, f), os.path.join(d, "errors"))] = open(os.path.join(root, f), "rb").read()
        for root, dirs, files in os.walk(d, topdown=False):
            for f in files:
                os.remove(os.path.join(root, f))
            for x in dirs:
                os.rmdir(os.path.join(root, x))
        os.rmdir(d)
        return (code, result, out.getvalue()) if capture else (code, result)

    diag = lambda line, col, msg, more="": f"tests/errors/p.scala:{line}:{col}: error: {msg}\n{more}  val x = 1\n  ^\n"
    cases = []
    src = "val x: String = 1\n// expect: 1:17: error: type mismatch: found 1, required String\n"
    old = diag(1, 17, "type mismatch: found 1, required String") + "1 error found\n"
    new = diag(1, 17, "Found:    (1 : Int)", "Required: String\n") + "1 error found\n"
    cases.append(("the identity", run(src, old, old), (0, src)))
    cases.append(("a rewrite with its further line", run(src, old, new),
                  (0, "val x: String = 1\n// expect: 1:17: error: Found:    (1 : Int)\n// expect+ Required: String\n")))
    two = "val (a: Int, b: Boolean) = p\n// expect: 1:26: error: type mismatch: found String, required Int\n// expect: 1:26: error: type mismatch: found String, required Boolean\n"
    two_old = diag(1, 26, "type mismatch: found String, required Int") + diag(1, 26, "type mismatch: found String, required Boolean") + "2 errors found\n"
    two_new = diag(1, 26, "Found: String, Required: Int") + diag(1, 26, "Found: String, Required: Boolean") + "2 errors found\n"
    cases.append(("two expectations at one place, each its own diagnostic", run(two, two_old, two_new),
                  (0, "val (a: Int, b: Boolean) = p\n// expect: 1:26: error: Found: String, Required: Int\n// expect: 1:26: error: Found: String, Required: Boolean\n")))
    moved = diag(2, 17, "type mismatch: found 1, required String") + "1 error found\n"
    cases.append(("a diagnostic moved: the verb stops", run(src, old, moved), (2, src)))
    cases.append(("a diagnostic moved in another program: nothing written", run(src, old, new, other=(src, old, moved)), (2, src)))
    two_errors = diag(1, 26, "type mismatch: found String, required Int") + diag(1, 26, "type mismatch: found String, required Int | Boolean") + "2 errors found\n"
    two_errors_new = diag(1, 26, "Found: String") + diag(1, 26, "Found: String, again") + "2 errors found\n"
    prefix = "val (a: Int, b: Boolean) = p\n// expect: 1:26: error: type mismatch: found String, required Int\n"
    cases.append(("an expectation two diagnostics at its place hold: refused", run(prefix, two_errors, two_errors_new), (1, prefix)))
    one_two = "val x: String = 1\n// expect: 1:17: error: type mismatch\n// expect: 1:17: error: type mismatch: found 1\n"
    cases.append(("two expectations on one diagnostic: refused", run(one_two, old, new), (1, one_two)))
    bare = "val x: String = 1\n// expect: type mismatch: found 1, required String\n"
    cases.append(("an expectation without a place: refused, left", run(bare, old, old), (1, bare)))
    summary = "val x: String = 1\n// expect: 1:17: error: type mismatch: found 1, required String\n// expect: 1 error found\n"
    cases.append(("a summary: no place, no refusal", run(summary, old, old), (0, summary)))
    cases.append(("a pending program: untouched", run(src, old, new, "p\tmissing\n"), (0, src)))
    above = "// expect: 2:17: error: type mismatch: found 1, required String\nval x: String = 1\n"
    above_old = diag(2, 17, "type mismatch: found 1, required String") + "1 error found\n"
    above_new = diag(2, 17, "Found:    (1 : Int)", "Required: String\n") + "warning: there was 1 deprecation warning; re-run with --deprecation for details\n1 error found\n"
    cases.append(("above the code: the further lines joined, no line added", run(above, above_old, above_new),
                  (0, "// expect: 2:17: error: Found:    (1 : Int)" + JOIN + "Required: String\nval x: String = 1\n")))
    # A write that fails half way: nothing is changed, no temporary file is left, the exit is not 0.
    global FAIL_WRITE
    FAIL_WRITE = True
    try:
        cases.append(("a failed write: nothing changed", run_files({"p": ({"p.scala": src.encode()}, old, new)}), (3, {"p.scala": src.encode()})))
    finally:
        FAIL_WRITE = False
    # A file that cannot be written beside one that can: nothing is changed.
    def unwritable():
        d = tempfile.mkdtemp(prefix="expects-test.")
        path, other = os.path.join(d, "a.scala"), os.path.join(d, "b.scala")
        for x in (path, other):
            open(x, "wb").write(b"old")
        os.chmod(other, 0o444)
        try:
            write_files({path: b"new", other: b"new"})
            code = 0
        except OSError:
            code = 3
        got = (code, open(path, "rb").read(), open(other, "rb").read(), sorted(os.listdir(d)))
        os.chmod(other, 0o644)
        for x in os.listdir(d):
            os.remove(os.path.join(d, x))
        os.rmdir(d)
        return got
    cases.append(("a file that cannot be written: nothing changed", unwritable(), (3, b"old", b"old", ["a.scala", "b.scala"])))
    # The second of three renames fails: the first file is replaced, the other two are not, no temporary file is left,
    # and the verb says so.
    global FAIL_RENAME_AT
    rewritten = b"val x: String = 1\n// expect: 1:17: error: Found:    (1 : Int)\n// expect+ Required: String\n"
    FAIL_RENAME_AT = 1
    try:
        code, files, out = run_files({n: ({n + ".scala": src.encode()}, old, new) for n in ("a", "b", "c")}, capture=True)
    finally:
        FAIL_RENAME_AT = None
    told = "the write is partial: replaced" in out and "a.scala" in out.split("not replaced")[0] and "b.scala" in out.split("not replaced")[-1] and "nothing is written" not in out
    cases.append(("the second of three renames fails: the write said partial", (code, files, told),
                  (3, {"a.scala": rewritten, "b.scala": src.encode(), "c.scala": src.encode()}, True)))
    # A source that is a symbolic link: its target is rewritten, the link stays a link.
    def symlinked():
        d = tempfile.mkdtemp(prefix="expects-test.")
        for sub in ("errors", "targets", "base", "head"):
            os.makedirs(os.path.join(d, sub))
        target, link = os.path.join(d, "targets", "p.scala"), os.path.join(d, "errors", "p.scala")
        open(target, "wb").write(src.encode())
        os.symlink(os.path.join("..", "targets", "p.scala"), link)
        open(os.path.join(d, "base", "p.out"), "w").write("exit 1\n" + old)
        open(os.path.join(d, "head", "p.out"), "w").write("exit 1\n" + new)
        open(os.path.join(d, "pending.txt"), "w").write("")
        saved, sys.stdout = sys.stdout, io.StringIO()
        try:
            code = write(os.path.join(d, "errors"), os.path.join(d, "base"), os.path.join(d, "head"), os.path.join(d, "pending.txt"), True)
        finally:
            sys.stdout = saved
        got = (code, os.path.islink(link), os.readlink(link) if os.path.islink(link) else None, open(target, "rb").read(), sorted(os.listdir(os.path.join(d, "targets"))))
        for root, dirs, files in os.walk(d, topdown=False):
            for f in files:
                os.remove(os.path.join(root, f))
            for x in dirs:
                os.rmdir(os.path.join(root, x))
        os.rmdir(d)
        return got
    cases.append(("a source that is a symbolic link: its target rewritten, the link kept", symlinked(),
                  (0, True, os.path.join("..", "targets", "p.scala"), rewritten, ["p.scala"])))
    # Two expectations rewritten at one place, the first's text held by the second's diagnostic too: the program left.
    pair = "val (a, b) = p\n// expect: 1:26: error: type mismatch: found A\n// expect: 1:26: error: type mismatch: found B\n"
    pair_old = diag(1, 26, "type mismatch: found A") + diag(1, 26, "type mismatch: found B") + "2 errors found\n"
    pair_new = diag(1, 26, "Found: new") + diag(1, 26, "Found: new suffix") + "2 errors found\n"
    cases.append(("a rewrite two of the head's diagnostics hold: the program left", run(pair, pair_old, pair_new), (1, pair)))
    # An expectation without a file in a program of two files, their diagnostics at one place in another order on the
    # head: rewritten from its own file's.
    at_a = lambda msg: f"tests/errors/q/A.scala:1:1: error: {msg}\n  val a = 1\n  ^\n"
    at_b = lambda msg: f"tests/errors/q/B.scala:1:1: error: {msg}\n  val b = 1\n  ^\n"
    two_files = {"q/A.scala": b"val a = 1\n// expect: 1:1: error: a-old\n", "q/B.scala": b"val b = 1\n"}
    cases.append(("an expectation of one file of two, the head's order changed: its own file's diagnostic",
                  run_files({"q": (two_files, at_a("a-old") + at_b("b-old") + "2 errors found\n", at_b("b-new") + at_a("a-new") + "2 errors found\n")}),
                  (0, {"q/A.scala": b"val a = 1\n// expect: 1:1: error: a-new\n", "q/B.scala": b"val b = 1\n"})))
    # A program on the pending list whose diagnostic moved: the verb stops, the other program is not written.
    cases.append(("a pending program's diagnostic moved: the verb stops",
                  run_files({"p": ({"p.scala": src.encode()}, old, moved), "q": ({"q.scala": src.encode()}, old, new)}, "p\tmissing\n"),
                  (2, {"p.scala": src.encode(), "q.scala": src.encode()})))
    # Line ends written back as read: a file of \\r\\n lines, and a last line without its end.
    crlf = b"val x: String = 1\r\n// expect: 1:17: error: type mismatch: found 1, required String\r\n// expect: 1 error found\r\n"
    cases.append(("\\r\\n kept on every line", run_files({"p": ({"p.scala": crlf}, old, new)}),
                  (0, {"p.scala": b"val x: String = 1\r\n// expect: 1:17: error: Found:    (1 : Int)\r\n// expect+ Required: String\r\n// expect: 1 error found\r\n"})))
    no_end = b"val x: String = 1\n// expect: 1:17: error: type mismatch: found 1, required String"
    cases.append(("a last line without its end kept so", run_files({"p": ({"p.scala": no_end}, old, new)}),
                  (0, {"p.scala": b"val x: String = 1\n// expect: 1:17: error: Found:    (1 : Int)\n// expect+ Required: String"})))
    failed = [n for n, got, want in cases if got != want]
    rule = "Required: Int | Boolean\n"
    part = not held_whole("x.scala:1:1: error: Found:    String\n" + rule, "Found:    String", ["Required: Int"])
    whole = held_whole("x.scala:1:1: error: Found:    String\n" + rule, "Found:    String", ["Required: Int | Boolean"])
    longer = not held_whole("x.scala:1:1: error: Found:    String\n" + rule + "Note: one more line\n  val x = 1\n  ^\n", "Found:    String", ["Required: Int | Boolean"])
    if not part:
        failed.append("a continuation line held only in part")
    if not whole:
        failed.append("a continuation line held whole")
    if not longer:
        failed.append("a message with a line more than its continuation lines")
    for n in failed:
        print(f"FAIL expects.py test: {n}")
    print(f"expects.py test: {len(cases) + 3 - len(failed)} of {len(cases) + 3} hold")
    return 1 if failed else 0


if __name__ == "__main__":
    verb = sys.argv[1] if len(sys.argv) > 1 else ""
    if verb == "continuations":
        why = continuations(sys.argv[2], sys.argv[3:])
        if why:
            print(why)
        sys.exit(1 if why else 0)
    if verb == "write":
        sys.exit(write(*sys.argv[2:6], "--apply" in sys.argv))
    if verb == "place":
        sys.exit(place(*sys.argv[2:6], "--apply" in sys.argv))
    if verb == "test":
        sys.exit(test())
    print(__doc__)
    sys.exit(2)
