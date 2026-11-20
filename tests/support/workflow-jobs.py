"""workflow-jobs.py <workflow file> [<run times dir>...]: the GitHub gate's jobs and the suites each runs, one line a
suite: `<suite>\t<job>`, a pieced suite's piece (`modules.1/4`) as it is named. The jobs are a matrix's `include`
rows (each its `group` and its `suites`) and a job's own `tests/all.sh <suites>` step.

The workflow is read by a reader of its own that FAILS CLOSED (no YAML library is on every runner and machine): a
matrix is a block list `include:` of rows of plain `key: value` scalars, in any key order, each with a `group` and
non-empty `suites`. Any other form (an axis beside or in the place of `include`, an `exclude`, a flow list or
mapping, a row without `group` or `suites`, a key twice, an anchor or an alias, a tab) is a nonzero exit with a
line naming it and where, never a table.

With the times of earlier runs (a directory a run, each job's times.txt under logs-<job>/ as the workflow uploads
it: `<suite> <seconds> <outcome>`), each job's planned minutes after it: the slowest of the runs a suite, plus a
minute a job; a piece of a suite the runs ran in another number of pieces is the largest sum of a run's pieces
divided by the new number, plus a fifth (pieces are dealt by weight, not by time). Prints the suites, then
`# <job>\t<planned minutes>` a job.

`workflow-jobs.py --self-test`: the reader over the tree's .github/workflows/gate.yml with every matrix row's keys
in reverse order (the same workflow: the same table), and over tests/support/workflow-jobs/axis-exclude.yml (an
axis and an `exclude`: refused)."""
import collections
import os
import re
import sys


class Refused(Exception):
    pass


def indent(line):
    return len(line) - len(line.lstrip(" "))


def scalar(v, where):
    v = v.strip()
    if v[:1] in ("[", "{", "&", "*", "|", ">", "!"):
        raise Refused(f"{where}: a value the reader does not handle: {v[:40]}")
    if len(v) >= 2 and v[0] == v[-1] and v[0] in "'\"":
        v = v[1:-1]
    return v


def read(text):
    """[(job, [suite])] of a workflow, or Refused."""
    lines = text.split("\n")
    for n, l in enumerate(lines, 1):
        if "\t" in l[:indent(l) + 1]:
            raise Refused(f"line {n}: a tab in the indentation")
    # the jobs: the keys two deeper than `jobs:`
    try:
        j0 = next(n for n, l in enumerate(lines) if l.rstrip() == "jobs:")
    except StopIteration:
        raise Refused("no `jobs:`")
    out, job, n = [], None, j0 + 1
    while n < len(lines):
        l = lines[n]
        body = l.split(" #", 1)[0].rstrip() if not l.lstrip().startswith("#") else ""
        if not body.strip():
            n += 1
            continue
        d = indent(l)
        if d == 0:
            break
        if d == 2 and body.strip().endswith(":"):
            job = body.strip()[:-1]
        m = re.match(r"^\s*(?:-\s+)?run:\s*tests/all\.sh[ \t]+(.*)$", body)
        if m and not m.group(1).split()[0].startswith("${{"):
            out.append((job, m.group(1).split()))
        if body.strip() == "matrix:":
            n = matrix(lines, n, d, out)
            continue
        if re.match(r"^\s*matrix:\s*\S", body):
            raise Refused(f"line {n + 1}: a matrix on one line")
        n += 1
    return out


def matrix(lines, n, d, out):
    """The rows of the matrix at line n (indent d) into out; the line after it."""
    n += 1
    seen = False
    while n < len(lines):
        l = lines[n]
        if not l.strip() or l.lstrip().startswith("#"):
            n += 1
            continue
        if indent(l) <= d:
            break
        key = l.strip().split(":", 1)[0]
        if indent(l) != d + 2 or l.strip() != "include:":
            raise Refused(f"line {n + 1}: a matrix form the reader does not handle: `{key}` (only a block list `include:`)")
        if seen:
            raise Refused(f"line {n + 1}: `include:` twice")
        seen = True
        n += 1
        row, rd = None, None
        while n < len(lines):
            l = lines[n]
            if not l.strip() or l.lstrip().startswith("#"):
                n += 1
                continue
            if indent(l) <= d + 2:
                break
            s = l.split(" #", 1)[0].rstrip()
            item = re.match(r"^(\s*)-\s+(\S.*)$", s)
            if item:
                if row is not None:
                    add(row, rd, out)
                row, rd = {}, n + 1
                kv = item.group(2)
                ind = len(item.group(1)) + 2
            elif row is not None and indent(l) == ind:
                kv = s.strip()
            else:
                raise Refused(f"line {n + 1}: a row's line the reader does not handle")
            m = re.match(r"^([\w-]+):\s*(.*)$", kv)
            if not m:
                raise Refused(f"line {n + 1}: not a `key: value` line")
            if m.group(1) in row:
                raise Refused(f"line {n + 1}: `{m.group(1)}` twice in one row")
            row[m.group(1)] = scalar(m.group(2), f"line {n + 1}")
            n += 1
        if row is not None:
            add(row, rd, out)
        elif seen:
            raise Refused(f"line {n}: an empty `include:`")
    return n


def add(row, line, out):
    if not row.get("group"):
        raise Refused(f"line {line}: a matrix row without `group`")
    if not row.get("suites", "").split():
        raise Refused(f"line {line}: the matrix row {row['group']} without suites")
    out.append((row["group"], row["suites"].split()))


def planned(jobs, runs):
    slowest = collections.defaultdict(int)  # suite -> seconds
    pieces = collections.defaultdict(int)  # pieced suite -> the largest sum of one run's pieces
    for run in runs:
        sums = collections.defaultdict(int)
        for d in sorted(os.listdir(run)):
            f = os.path.join(run, d, "times.txt")
            if not (d.startswith("logs-") and os.path.exists(f)):
                continue
            for line in open(f, encoding="utf-8"):
                p = line.split()
                if len(p) >= 2 and p[1].isdigit():
                    slowest[p[0]] = max(slowest[p[0]], int(p[1]))
                    m = re.fullmatch(r"([a-z0-9-]+)\.\d+/\d+", p[0])
                    # a job's second attempt (logs-<job>-2 beside logs-<job>) is another run of the same pieces
                    if m and not (d.endswith("-2") and os.path.isdir(os.path.join(run, d[:-2]))):
                        sums[m.group(1)] += int(p[1])
        for k, v in sums.items():
            pieces[k] = max(pieces[k], v)

    def one(s):
        if s in slowest:
            return slowest[s]
        m = re.fullmatch(r"([a-z0-9-]+)\.\d+/(\d+)", s)
        if m and m.group(1) in pieces:
            return pieces[m.group(1)] / int(m.group(2)) * 1.2
        return None
    lines = []
    for job, suites in jobs:
        known = [one(s) for s in suites]
        missing = [s for s, t in zip(suites, known) if t is None]
        total = sum(t for t in known if t is not None) / 60 + 1
        lines.append(f"# {job}\t{total:.1f}" + (f"\t(no times: {' '.join(missing)})" if missing else ""))
    return lines


def table(jobs):
    return [f"{s}\t{job}" for job, suites in jobs for s in suites]


def self_test():
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
    gate = open(os.path.join(root, ".github", "workflows", "gate.yml"), encoding="utf-8").read()
    want = table(read(gate))
    # every matrix row's keys in reverse order: the same workflow
    lines, out, i = gate.split("\n"), [], 0
    while i < len(lines):
        m = re.match(r"^(\s*)-\s+(\S.*)$", lines[i])
        if m and re.match(r"^group:", m.group(2)):
            pad, pairs, j = m.group(1), [m.group(2)], i + 1
            while j < len(lines) and re.match(r"^" + pad + r"  [\w-]+:", lines[j]):
                pairs.append(lines[j].strip())
                j += 1
            pairs.reverse()
            out += [f"{pad}- {pairs[0]}"] + [f"{pad}  {p}" for p in pairs[1:]]
            i = j
        else:
            out.append(lines[i])
            i += 1
    got = table(read("\n".join(out)))
    fails = []
    if got != want or not any("/" in l for l in got):
        fails.append(f"the gate with its rows' keys reversed reads {len(got)} lines where it reads {len(want)}")
    try:
        read(open(os.path.join(root, "tests", "support", "workflow-jobs", "axis-exclude.yml"), encoding="utf-8").read())
        fails.append("axis-exclude.yml is read, not refused")
    except Refused as e:
        if "matrix form" not in str(e):
            fails.append(f"axis-exclude.yml refused for another reason: {e}")
    for f in fails:
        print(f"FAIL workflow-jobs: {f}")
    print(f"workflow-jobs: the self-test {'passed' if not fails else 'failed'}: {len(want)} suites by job, every row's keys reversed; "
          "an axis and an exclude refused")
    return 1 if fails else 0


def main(argv):
    if argv == ["--self-test"]:
        return self_test()
    if not argv:
        sys.exit(__doc__)
    try:
        jobs = read(open(argv[0], encoding="utf-8").read())
    except Refused as e:
        print(f"workflow-jobs: {argv[0]}: refused: {e}", file=sys.stderr)
        return 2
    print("\n".join(table(jobs)))
    if len(argv) > 1:
        print("\n".join(planned(jobs, argv[1:])))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
