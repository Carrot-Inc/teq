#!/usr/bin/env python3
"""The twin of a class's own pairs (tests/pairs-twin.sh): the pairs the record enumerates (`Worker::class_pairs`) whose
overriding member is the class's own, against what the own pairs' walks record of them, in one build of each program
with `TEQ_PAIRS_DUMP` (JavaScript and the JVM):

- the pairs: the record's own pairs of a class against those the override check finds overridden (`walk` lines,
  `check_overrides_of`), by the members' text; the check takes a member with a definition and no extension method,
  and so does the comparison (the record's own pairs of a member without one are counted apart);
- the name: of an own pair whose two members go by two names in the JavaScript output, the class answers the
  overridden one's name by a bridge to the overriding one (`name` lines, written after the names are settled);
- the accessor: an own stored member (a val, a var, a lazy given) implementing a method goes by its accessor;
- the bridge target: every bridge the JVM's per-method walk declares (`jvm-walk ... bridge`) is a pair of the record,
  and every pair of the record whose two descriptors differ is bridged by it (`jvm-walk ... same|bridge|declined`).
- the stored record: what each class keeps, dumped from the store the readers read (`stored`, `stored-none`), is
  every pair of its enumeration the record's rule keeps (`kept`), each with its bridged mark, and nothing else: a
  pair lost, added, retargeted or re-marked between the enumeration and the readers is a difference.

  pairs_twin.py --head <teq> --out <dir> [--cases] [--dir <d>]... [--jobs N] [--limit N] [--sl <jar>] [--results <file>] [--discard] [--only <program>,...]
Prints each difference and the totals; exits 1 when one is found. With --results it also writes a line a program for
tests/support/pending.py: `pass <program>` where a leg built and nothing differs; `skip <program>` where no leg
builds (nothing to hold: whether it should build is the cases suite's verdict); `fail <program>` and, after a tab,
its first difference. It exits 2 when the binary built none of the programs. --discard lets go of each program's
builds once they are read (the suite's run)."""
import collections, concurrent.futures, os, re, shutil, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def arg(name, default=None):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


HEAD, OUT = arg("--head"), arg("--out")
JOBS, LIMIT = int(arg("--jobs", "4")), int(arg("--limit", "0"))
HOME = os.path.expanduser("~")
SL = arg("--sl") or os.path.join(os.environ.get("COURSIER_CACHE", HOME + "/.cache/coursier/v1"),
                                "https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar")
RESULTS = arg("--results")
DISCARD = "--discard" in sys.argv
ENV = dict(os.environ, LC_ALL="C.UTF-8", TZ="UTC", TEQ_NO_CACHE="1", TEQ_THREADS="1")


def programs():
    import pairs_programs as pp
    out = []
    if "--cases" in sys.argv:
        out += list(pp.case_programs(ROOT, ["js", "jvm"]))
    for i, a in enumerate(sys.argv):
        if a == "--dir":
            out += list(pp.dir_programs(sys.argv[i + 1], ["js", "jvm"]))
    only = arg("--only")
    if only:
        out = [p for p in out if p[0] in only.split(",")]
    return out[:LIMIT] if LIMIT else out


def build(leg, files, flags, out, dump):
    env = dict(ENV, TEQ_PAIRS_DUMP=dump)
    cmd = [HEAD, "compiler", "build", "--no-cache"] + (["--target", "jvm", "--classpath", SL] if leg == "jvm" else []) + files + ["-o", out] + flags
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=300, env=env, cwd=os.path.dirname(out))
        if r.returncode == 1 and "several entry points" in r.stdout + r.stderr:
            r = subprocess.run(cmd + ["--main", "Test"], capture_output=True, text=True, timeout=300, env=env, cwd=os.path.dirname(out))
        return r.returncode
    except subprocess.TimeoutExpired:
        return "timeout"


def lines(path):
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8", errors="replace") as f:
        return [l.rstrip("\n").split("\t") for l in f]


def check(dump, leg):
    """The differences of one build's dump, and the JVM walk's bridges of members a class's table does not hold
    (made by rule in the back end, a case companion's `apply`), which the record, reading the table, has no pair of."""
    rows = lines(dump)
    rule_bridges = []
    record = collections.defaultdict(set)  # class -> {(overriding text, overridden text)} of own pairs with a definition
    walk = collections.defaultdict(set)
    no_def = 0
    ids = {}  # (class, overriding id, overridden id) of own pairs
    diffs = []
    recorded_paths = set(r[0] for r in rows if len(r) >= 2 and r[1] in ("own", "inherited", "none"))
    for r in rows:
        if len(r) < 2:
            continue
        cls, kind = r[0], r[1]
        if kind == "own" and len(r) >= 7:
            # class own overriding overridden handled bridged #o #p def std
            o, p, oid, pid, has_def, std = r[2], r[3], r[6], r[7], r[8], r[9]
            if std == "std":
                continue
            ids[(cls, oid, pid)] = (o, p)
            if has_def == "def":
                record[cls].add((o, p))
            else:
                no_def += 1
        elif kind == "walk":
            walk[cls].add((r[2], r[3]))
        elif kind == "name":
            # class name #o #p name-o name-p bridged accessor
            o, p, no, np_, bridged, accessor = r[2], r[3], r[4], r[5], r[6], r[7]
            if no != np_ and bridged != "bridged":
                diffs.append(f"{leg} {cls}: name: {o} goes by {no}, {p} by {np_}, and no bridge answers {np_}")
            if accessor == "no-accessor":
                diffs.append(f"{leg} {cls}: accessor: {o} implements the method {p} and has no accessor")
    # The record's pairs of each class, either way round: the cursor pairs a base's concrete member over an abstract
    # redeclaration in the class (deferred ones are entered first), where the check pairs the class's own member
    # over the base's.
    either = collections.defaultdict(set)
    for r in rows:
        if len(r) >= 4 and r[1] in ("own", "inherited"):
            either[r[0]].add(frozenset((r[2], r[3])))
    for cls in sorted(set(record) | set(walk)):
        # A class the check does not walk (a copy, a std class), or one the program does not record (a stored
        # body's class, which no back end writes): nothing to hold one to the other.
        if cls not in walk or cls not in recorded_paths:
            continue
        walked = set(frozenset(w) for w in walk[cls])
        for pr in sorted(record[cls]):
            if frozenset(pr) not in walked:
                diffs.append(f"{leg} {cls}: pair in the record, not in the walk: {pr[0]} > {pr[1]}")
        for pr in sorted(walk[cls]):
            if frozenset(pr) not in either[cls]:
                diffs.append(f"{leg} {cls}: pair in the walk, not in the record: {pr[0]} > {pr[1]}")
    # The stored record against the enumeration: what each recorded class keeps (`stored` lines, read from
    # TClassLists::inherited_pairs as the readers read it) is every pair the record's rule keeps of the class's
    # enumeration (`kept` lines), with its bridged mark, and nothing else. A class recorded with none says so
    # (`stored-none`), so a lost pair is never a silence.
    kept, stored, text, recorded = collections.defaultdict(set), collections.defaultdict(set), {}, set()
    for r in rows:
        if len(r) >= 12 and r[1] in ("own", "inherited") and r[11] == "kept":
            kept[r[10]].add((r[6], r[7], r[5]))
            text[(r[10], r[6], r[7])] = (r[0], r[2], r[3])
        elif len(r) >= 8 and r[1] == "stored":
            stored[r[7]].add((r[5], r[6], r[4]))
            text.setdefault((r[7], r[5], r[6]), (r[0], r[2], r[3]))
            recorded.add(r[7])
        elif len(r) >= 3 and r[1] == "stored-none":
            recorded.add(r[2])
    for cid in sorted(recorded | set(kept)):
        for o, p, b in sorted(kept[cid] - stored[cid]):
            cls, ot, pt = text.get((cid, o, p), (cid, "#" + o, "#" + p))
            diffs.append(f"{leg} {cls}: stored record: {ot} > {pt} ({b}) is kept by the record's rule and not stored" + ("" if cid in recorded else " (the class was never recorded)"))
        for o, p, b in sorted(stored[cid] - kept[cid]):
            cls, ot, pt = text.get((cid, o, p), (cid, "#" + o, "#" + p))
            diffs.append(f"{leg} {cls}: stored record: {ot} > {pt} ({b}) is stored and not kept by the record's rule")
    if leg == "jvm":
        # By the class's id: the record's own pairs against the per-method walk's decisions (`emit_bridges_as`).
        own_ids = collections.defaultdict(dict)
        recorded = set()
        for r in rows:
            if len(r) >= 11 and r[1] in ("own", "inherited") and r[9] != "std":
                recorded.add(r[10])
                # every pair of the class: a class emits a mixed-in member's accessor (an object, a val) and the
                # walk bridges it there, the record holding that pair as an inherited one
                own_ids[r[10]][(r[6], r[7])] = (r[2], r[3])
            elif len(r) >= 4 and r[1] == "none" and r[2] != "std":
                recorded.add(r[3])
        for r in rows:
            if len(r) >= 6 and r[1] == "jvm-walk":
                cid, oid, pid, decision, differs = r[0], r[2], r[3], r[4], r[5]
                pair = own_ids.get(cid, {}).get((oid, pid))
                by_rule = len(r) >= 7 and r[6] == "rule"
                if decision == "bridge" and pair is None and cid in recorded and by_rule:
                    rule_bridges.append(f"jvm {cid}: #{oid} for #{pid}")
                elif decision == "bridge" and pair is None and cid in recorded:
                    diffs.append(f"jvm {cid}: the walk bridges #{oid} for #{pid}, no pair of the record")
                if pair is not None and differs == "descriptors-differ" and decision in ("not-overriding", "declined", "arity"):
                    # A finding of the walk's own: its gate on an alternative (`prog.overrides`) lacks a pair the
                    # record has, so no bridge is made there (`Ver.compareTo` over `Comparable`'s: AbstractMethodError).
                    diffs.append(f"jvm {cid}: the record's pair {pair[0]} > {pair[1]} differs in descriptor and the walk does not bridge it ({decision})")
    return diffs, no_def, rule_bridges


def program(p):
    name, files, flags, legs, _ = p
    d = os.path.join(OUT, name.replace("/", "_"))
    os.makedirs(d, exist_ok=True)
    found, no_def, built, rules = [], 0, [], []
    for leg in legs:
        dump = os.path.join(d, f"dump-{leg}.txt")
        if os.path.exists(dump):
            os.unlink(dump)
        rc = build(leg, files, flags, os.path.join(d, "out.js" if leg == "js" else "out.jar"), dump)
        if rc != 0:
            continue
        built.append(leg)
        diffs, nd, rb = check(dump, leg)
        found += diffs
        no_def += nd
        rules += rb
    if DISCARD:
        # the suite's run: a program's builds and dumps are read, then let go (a temporary directory on tmpfs)
        shutil.rmtree(d, ignore_errors=True)
    return name, found, no_def, built, rules


def main():
    os.makedirs(OUT, exist_ok=True)
    progs = programs()
    total, bad, no_def, built, by_rule = 0, 0, 0, collections.Counter(), 0
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=JOBS) as ex:
        for name, diffs, nd, legs, rules in ex.map(program, progs):
            total += 1
            no_def += nd
            by_rule += len(rules)
            for leg in legs:
                built[leg] += 1
            for x in diffs:
                bad += 1
                print(f"{name}\t{x}", flush=True)
            if diffs:
                results.append(f"fail {name}\t{diffs[0]}")
            elif legs:
                results.append(f"pass {name}")
            else:
                # no leg built: nothing for the twin to hold (whether it should build is the cases suite's verdict)
                results.append(f"skip {name}\tno leg builds")
    if RESULTS:
        with open(RESULTS, "w", encoding="utf-8") as fh:
            fh.write("".join(r.replace("\n", " ") + "\n" for r in results))
    print(f"programs {total}; built js {built['js']}, jvm {built['jvm']}; differences {bad}; own pairs of a member without a definition (the check's own, not walked) {no_def}; the JVM walk's bridges of members made by rule, no table's (the record has none of them) {by_rule}")
    if total and not built:
        print("the binary built none of the programs: the twin held nothing")
        sys.exit(2)
    sys.exit(1 if bad else 0)


main()
