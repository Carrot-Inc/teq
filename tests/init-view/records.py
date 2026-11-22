#!/usr/bin/env python3
"""records.py: a run's records as the judges of tests/init-view read them, and the harness of their self-tests.

A record is a file a runner wrote: an output (a view's dump, a session's answers), or an exit status (`<step>.exit`,
the number the step's command exited with, 124 where `timeout` stopped it). Beside each output its producer writes,
right after the output, its sum (`<step>.sum`, the output's name without its extension): one line `<id> <bytes>
<sha256>`, the id naming the unit or cell the record is of and the record (`<unit>/<path under the unit's
directory>`, `head:` before it where the record is the head's of a unit whose base's record has the same name). An
output is whole only where it is what its sum says: a record missing, with no sum, with another id (another cell's
record copied in), other bytes or another digest (a line cut off its end or out of its middle) is not judged, and
neither is an empty one. Its last byte says nothing: a record cut at a line's end ends as a whole one does. A runner
records each step's exit beside its output, so that a judge tells a program a compiler refuses (an exit it records,
not 124) from a record that is missing, cut or past its bound, which it never counts equal. tests/init-view/records.sh
writes the sums from the shell (`record_seal`), as `seal` here does.

A step's exit is judged by a whitelist (`judged_exit`): 0, or a compiler's own refusal named by its line in the
step's log (whole by its sum): scalac's 1 with its error line; teq's 1 with its error line or the TASTy writer's
summary (`<n> definitions cannot be written as TASTy`), and teq's 3 with the JVM backend's summary (`<n> constructs not
supported by the JVM backend`), the two lines src/main.rs closes those refusals with. Any other exit, a signal's (137 a
kill, 139 a segmentation fault), 2, 124 (past its bound), none recorded or no number, 1 or 3 without its line, is NOT
JUDGED with why: a killed compiler is no refusal.

`corpus_unit` reads a unit of tests/init-view-corpus.sh's run: scalac's side (its InitDump's `oracle/exit` and
`oracle/scalac.txt`, the view of its pickles) and the base's (teq's `check.exit` and `check.txt`, the view of its
products)."""
import contextlib
import hashlib
import io
import os
import re
import shutil
import tempfile

# A compiler's error line, which its refusal holds on its record: scalac's (`-- Error: <file>:<l>:<c>`, `-- [E007] Type
# Mismatch Error: <file>:<l>:<c>`), teq's (`<file>:<line>:<column>: error: ...`).
SCALAC_ERROR = re.compile(r"^-- (\[E\d+\] [^:]*)?Error:")
TEQ_ERROR = re.compile(r"^[^\s:][^:]*:\d+:\d+: error:")
# teq's refusals closed by a summary line of their own (src/main.rs): the TASTy writer's, exit 1, and the JVM
# backend's, exit 3.
TASTY_REFUSAL = re.compile(r"^\d+ definitions? cannot be written as TASTy$")
JVM_REFUSAL = re.compile(r"^\d+ constructs? not supported by the JVM backend$")
# teq's internal error or a panic of its own: never a refusal, whatever summary follows it (a guard of the TASTy
# writer reports as one of the writer's errors, which its summary line closes).
INTERNAL = re.compile(r"^internal error: |^thread '.*' panicked at ")
# A program's own failure, which its run's exit 1 holds on its record: an exception the JVM reports uncaught.
PROGRAM_FAILURE = re.compile(r"^Exception in thread ")
# Each compiler's refusals, by exit: the lines one of which its log must hold.
SCALAC_REFUSALS = {1: (SCALAC_ERROR,)}
TEQ_REFUSALS = {1: (TEQ_ERROR, TASTY_REFUSAL), 3: (JVM_REFUSAL,)}


def sum_path(path):
    """Where the sum of the output at `path` is: its name without its extension, `.sum`."""
    return os.path.splitext(path)[0] + ".sum"


def record_id(path, unit=None, side=None):
    """The id a record's sum names: its unit's directory's name (`unit`, by default the directory holding it), the
    path under it, and `side:` before them where the producer names the side."""
    unit = unit or os.path.dirname(path)
    rid = os.path.basename(os.path.abspath(unit)) + "/" + os.path.relpath(path, unit).replace(os.sep, "/")
    return f"{side}:{rid}" if side else rid


def sum_line(path, unit=None, side=None):
    """The line a producer writes as the sum of the output at `path`."""
    data = open(path, "rb").read()
    return f"{record_id(path, unit, side)} {len(data)} {hashlib.sha256(data).hexdigest()}\n"


def seal(path, unit=None, side=None):
    """Writes the sum of the output at `path`, as its producer does right after it."""
    write(sum_path(path), sum_line(path, unit, side))


def raw(path):
    """(lines, None) of a file a runner writes with no sum (an exit, a marker), else (None, why): missing or empty."""
    if not os.path.isfile(path):
        return None, "missing"
    data = open(path, "rb").read()
    if not data:
        return None, "empty"
    return data.decode("utf-8", "replace").splitlines(), None


def text(path, unit=None, side=None):
    """(lines, None) of an output that is what its producer wrote, else (None, why): missing, no sum, a sum that is no
    sum, another record's sum, other bytes or another digest than its sum's, or empty."""
    if not os.path.isfile(path):
        return None, "missing"
    s = sum_path(path)
    if not os.path.isfile(s):
        return None, "with no sum of what its producer wrote"
    parts = open(s, "rb").read().decode("utf-8", "replace").rstrip("\n").rsplit(" ", 2)
    if len(parts) != 3 or not parts[1].isdigit() or len(parts[2]) != 64 or "\n" in parts[0]:
        return None, "with a sum that is no sum"
    rid, size, digest = parts[0], int(parts[1]), parts[2]
    want = record_id(path, unit, side)
    if rid != want:
        return None, f"another record's ({rid}, where {want} is due)"
    data = open(path, "rb").read()
    if len(data) != size:
        return None, f"other than its producer wrote ({len(data)} bytes of {size})"
    if hashlib.sha256(data).hexdigest() != digest:
        return None, "other than its producer wrote (another digest)"
    if not data:
        return None, "empty"
    return data.decode("utf-8", "replace").splitlines(), None


def exit_code(path):
    """The exit status a runner recorded at `path`, or None where it recorded none (or no number)."""
    lines, why = raw(path)
    if why:
        return None
    try:
        return int(lines[0].strip())
    except (ValueError, IndexError):
        return None


def dump(path, unit=None, side=None):
    """`text` of a view's dump, which holds its unit's header once: a dump holding it twice is two runs' outputs."""
    lines, why = text(path, unit, side)
    if lines is not None and sum(1 for l in lines if l.startswith("unit ")) > 1:
        return None, "holds its unit twice"
    return lines, why


def ended(e):
    """Why an exit is no judged one: past its bound, a signal's, or another number."""
    if e == 124:
        return "past its bound (exit 124)"
    if e > 128:
        return f"killed by signal {e - 128} (exit {e})"
    return f"ended with exit {e}"


def judged_exit(exit_path, log, what, refusals, unit=None):
    """('ok', None) of a step that exited 0; ('refused', why) of one whose exit `refusals` names (`{exit: patterns}`,
    SCALAC_REFUSALS or TEQ_REFUSALS) with a line of its patterns in its log (`log`, whole by its sum); else
    ('not judged', why)."""
    e = exit_code(exit_path)
    if e is None:
        return "not judged", f"no record of {what}'s exit"
    if e == 0:
        return "ok", None
    if e not in refusals:
        return "not judged", f"{what} {ended(e)}"
    lines, why = text(log, unit)
    if why:
        return "not judged", f"{what} exited {e} and its log is {why}"
    internal = next((l for l in lines if INTERNAL.match(l)), None)
    if internal is not None:
        return "not judged", f"{what} exited {e} with an internal error on its record ({internal.strip()[:120]})"
    hit = next((l for l in lines if any(p.match(l) for p in refusals[e])), None)
    if hit is None:
        return "not judged", f"{what} exited {e} without the line of its refusal on its record"
    return "refused", f"{what} refuses (exit {e}: {hit.strip()[:120]})"


def step(out, exit_path, what, read=text):
    """('ok', lines) of a step whose exit is recorded as 0 and whose output is whole, else ('not judged', why)."""
    e = exit_code(exit_path)
    if e is None:
        return "not judged", f"no record of {what}'s exit"
    if e != 0:
        return "not judged", f"{what} {ended(e)}"
    lines, why = read(out)
    if why:
        return "not judged", f"{what}'s output {why}"
    return "ok", lines


def corpus_unit(u):
    """A unit of tests/init-view-corpus.sh's run, per side: ('ok', dump lines), ('refused', why) where the compiler
    refused the program (an exit it recorded, not 124), or ('not judged', why)."""
    if not os.path.isdir(os.path.join(u, "src")):
        missing = ("not judged", "the corpus run has no record of the unit")
        return {"scalac": missing, "base": missing}
    state, why = judged_exit(os.path.join(u, "oracle", "exit"), os.path.join(u, "oracle", "scalac.txt"), "scalac", SCALAC_REFUSALS, u)
    if state != "ok":
        scalac = (state, why)
    else:
        scalac = step(os.path.join(u, "scalac-view.dump"), os.path.join(u, "scalac-view.exit"), "the view of scalac's pickles", dump)
    # The view of the base's products ran where it made products; else its check's exit says why it made none.
    if exit_code(os.path.join(u, "view.exit")) is not None:
        base = step(os.path.join(u, "view.dump"), os.path.join(u, "view.exit"), "the view of the base's products", dump)
    else:
        state, why = judged_exit(os.path.join(u, "check.exit"), os.path.join(u, "check.txt"), "the base's check", TEQ_REFUSALS)
        if state == "refused":
            base = ("refused", f"the base makes no products: {why}")
        elif state == "ok":
            base = ("not judged", "the base's products have no view recorded")
        else:
            base = (state, why)
    return {"scalac": scalac, "base": base}


def manifest(names, present):
    """What is wrong with a run's records against its manifest `names`: each name the manifest holds twice, each one
    `present` (the ids a run's directory or list holds) lacks, and each id `present` holds that it does not."""
    seen, dup = set(), []
    for n in names:
        if n in seen:
            dup.append(n)
        seen.add(n)
    have = list(present)
    seen_have, dup_have = set(), []
    for n in have:
        if n in seen_have:
            dup_have.append(n)
        seen_have.add(n)
    return ([f"the manifest holds {n} twice" for n in dup] + [f"the records hold {n} twice" for n in dup_have]
            + [f"no record of {n}" for n in names if n not in seen_have]
            + [f"a record of {n}, which the manifest does not hold" for n in sorted(seen_have - seen)])


class SelfTest:
    """Planted records in a scratch directory under $TMPDIR, each judged and its verdict checked; prints one line."""

    def __init__(self, tool):
        self.tool, self.cases, self.good, self.wrong = tool, 0, 0, []
        self.root = tempfile.mkdtemp(prefix="selftest-", dir=os.environ.get("TMPDIR"))

    def dir(self, name):
        d = os.path.join(self.root, name)
        os.makedirs(d, exist_ok=True)
        return d

    def quiet(self, f, *args):
        """`f(*args)` with what it prints kept: (its result or 'raised', the printed text)."""
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            try:
                r = f(*args)
            except Exception as e:  # a judge that raises on a record has not judged it
                r = f"raised {type(e).__name__}: {e}"
        return r, out.getvalue()

    def expect(self, case, ok, detail=""):
        self.cases += 1
        if ok:
            self.good += 1
        else:
            self.wrong.append(f"{case}{': ' + detail if detail else ''}")

    def finish(self):
        shutil.rmtree(self.root, ignore_errors=True)
        for w in self.wrong:
            print(f"  NOT AS EXPECTED: {w}")
        print(f"{self.tool} self-test: {self.cases} cases, {self.good} as expected")
        return 0 if self.good == self.cases else 1


def write(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(content)


def put(path, content, unit=None, side=None):
    """A planted output as its producer writes it: the output, then its sum."""
    write(path, content)
    seal(path, unit, side)


# A record changed after its producer wrote it and its sum, each a plant a judge must not judge: what the sum says of
# it, which the judge names, is one of SUM_WHY.
SUM_WHY = ("other than its producer wrote", "another record's", "with no sum of what its producer wrote")


def _lines(path):
    return open(path, "rb").read().splitlines(keepends=True)


def cut_last_line(path):
    """The record's last line gone, the line break before it kept (it ends as a whole record does); its sum kept."""
    lines = _lines(path)
    assert len(lines) >= 2, path
    with open(path, "wb") as f:
        f.write(b"".join(lines[:-1]))


def cut_middle_line(path):
    """A line out of the record's middle gone; its sum kept."""
    lines = _lines(path)
    assert len(lines) >= 3, path
    del lines[len(lines) // 2]
    with open(path, "wb") as f:
        f.write(b"".join(lines))


def change_byte(path):
    """A byte of the record's first line changed, its length kept; its sum kept."""
    data = bytearray(open(path, "rb").read())
    i = next(i for i, b in enumerate(data) if chr(b).isalnum())
    data[i] = ord("x") if data[i] != ord("x") else ord("y")
    with open(path, "wb") as f:
        f.write(bytes(data))


def copy_record(src, dst):
    """Another cell's record copied in: its output and its sum."""
    shutil.copyfile(src, dst)
    shutil.copyfile(sum_path(src), sum_path(dst))


def after_the_producer(other):
    """The plants of a record changed after its producer sealed it, each `(name, mutation of its path)`: the last line
    gone (its line break and its exit kept), a line gone from the middle, a byte changed, the sum gone, and `other(path)`
    (the same record of another cell, sealed by its producer) copied in."""
    return [("its last line gone, its line break and exit kept", cut_last_line),
            ("a line gone from its middle", cut_middle_line),
            ("a byte of it changed, its length kept", change_byte),
            ("its sum gone", lambda p: os.remove(sum_path(p))),
            ("another cell's record copied in (its output and sum)", lambda p: copy_record(other(p), p))]
