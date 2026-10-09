#!/usr/bin/env python3
"""Compares what zinc recompiled after each scenario's edit on scalac's analysis and on teq's
(tests/invalidation.sh; docs/TARGETS.md, "The invalidation oracle").

Each side's dump is `<scenario>.<variant>.<side>.json` as the oracle's driver writes it
(integrations/sbt/analysis/project/Invalidation.scala): per project the run's initial changes,
its cycles (the classes invalidated, the batch of sources, the classes compiled again, the API
changes zinc detected, the next invalidations, the events), the batches the compile callback
was handed with their outcome, what the analysis it leaves says each source owns and the
relations it holds; and the reference, a fresh build of the edited case: its outcome, its
diagnostics, its ownership and relations, and every file of the class directory against the
fresh build's, with the source that owns it and whether the run compiled that source again.

  invalidation.py <results dir> --scenarios <scenarios.txt> --expect <expected.txt> [--only <a,b>] [--freeze]

Per scenario and variant, the differences are kept in four classes:

- an observed difference of the raw sets between the sides: a source in one side's batch of a
  cycle and not the other's (`missing` on the side without it, `extra` on the side with it, an
  extra compilation counted as cost), a class invalidated, an API change or a modified name on
  one side only, an initial change on one side only, the project's outcome, the classes a source
  owns;
- a proven stale result on a side: a file of the class directory that differs from the fresh
  build's (or that one of them lacks) whose source the run did not compile again, or
  diagnostics of the fresh build that the run did not give (`stale`); observed beside it, a file
  that differs whose source the run did compile again (`inconsistent`), the run's own relations
  or ownership against the fresh build's (`relation`), the run's diagnostics where the fresh
  build has others (`error`); a file that differs only in its class file's constant pool order or
  its TASTy identifier is `equivalent`, counted;
- an accepted representation difference: the binary names of a package object's classes,
  scalac's `p.package` and teq's `p.<File>$package` (the package-object mapping; both name the
  class `p.package`), products and external changes compared through the mapping, counted;
- an obligation of the scenario (its `expect` lines) that a side breaks, or an outcome other
  than its `status`.

Every difference but the accepted ones fails unless expected.txt names it, scoped by scenario,
variant, project, side, kind, cycle and the exact source or class:

  inv-diff <scenario>/<variant> <project> <side> <kind> <cycle|-> <name> <reason>

and every such line must name a difference that is there (a stale line fails). expected.txt
also freezes each side's batches and outcomes:

  inv-batch <scenario>/<variant> <project> <side|both> <cycle> <source,...>
  inv-status <scenario>/<variant> <project> <side|both> <passed|failed|not-run|refused|built>

a batch or an outcome the run no longer has, or one it has that no line names, fails.
`--only` compares the scenarios named and leaves the others' lines alone; `--freeze` prints the
lines the results have instead, with `?` for each difference's reason.
Prints one line per difference (`named` with its class and reason, `FAIL` when no line names it)
and per scenario, `stale` for a line of expected.txt that names nothing; the exit code is 1 when
anything fails."""

import json
import os
import sys

SIDES = ("scalac", "teq")

# zinc's options a run takes unless its variant names others (IncOptions' defaults).
DEFAULT_OPTIONS = {"transitiveStep": 3, "recompileAllFraction": 0.5, "useOptimizedSealed": False}


class Broken(Exception):
    pass


def read_scenarios(path):
    """{name: {"variants": [...], "status": [...], "expect": [[side|None, kind, args...]]}}"""
    out = {}
    current = None
    for raw in open(path):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        words = line.split()
        if words[0] == "scenario":
            current = {"variants": [], "options": {}, "status": None, "expect": [], "modules": None}
            out[words[1]] = current
        elif current is None:
            raise Broken("%s: %s before any scenario" % (path, line))
        elif words[0] == "variant":
            current["variants"].append(words[1])
            options = dict(DEFAULT_OPTIONS)
            for opt in words[2:]:
                k, _, v = opt.partition("=")
                if k not in options:
                    raise Broken("%s: variant %s: unknown option %s" % (path, words[1], opt))
                options[k] = type(options[k])(v == "true") if isinstance(options[k], bool) else type(options[k])(v)
            current["options"][words[1]] = options
        elif words[0] == "status":
            current["status"] = words[1:]
        elif words[0] == "expect":
            rest = words[1:]
            variant = None
            if rest[0].startswith("@"):
                variant, rest = rest[0][1:], rest[1:]
            side = rest[0] if rest[0] in SIDES else None
            current["expect"].append([variant, side] + (rest[1:] if side else rest))
    for s in out.values():
        if not s["variants"]:
            s["variants"] = ["default"]
            s["options"]["default"] = dict(DEFAULT_OPTIONS)
        if s["status"] is None:
            raise Broken("a scenario without its status")
    return out


def read_expected(path):
    diffs, batches, statuses = [], [], []
    for raw in open(path):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        words = line.split()
        if words[0] == "inv-diff":
            if len(words) != 8:
                raise Broken("expected.txt: %s: an inv-diff line has 8 fields" % line)
            diffs.append(tuple(words[1:]))
        elif words[0] == "inv-batch":
            if len(words) != 6:
                raise Broken("expected.txt: %s: an inv-batch line has 6 fields" % line)
            batches.append(tuple(words[1:]))
        elif words[0] == "inv-status":
            if len(words) != 5:
                raise Broken("expected.txt: %s: an inv-status line has 5 fields" % line)
            statuses.append(tuple(words[1:]))
    return diffs, batches, statuses


def load(path):
    if not os.path.isfile(path):
        raise Broken("%s: no dump" % path)
    d = json.load(open(path))
    if "error" in d:
        raise Broken("%s: the driver failed: %s" % (os.path.basename(path), d["error"]))
    if d.get("refused"):
        if not d.get("modules") or any(m.get("status") not in ("built", "refused", "not-run") for m in d["modules"]):
            raise Broken("%s: a refusing side without its projects' outcomes" % path)
        return d
    for m in d.get("modules", []):
        if m.get("status") not in ("passed", "failed", "not-run"):
            raise Broken("%s: project %s without its outcome" % (path, m.get("module")))
        if m["status"] != "not-run":
            for field in ("initial", "cycles", "batches", "ownership", "relations", "reference"):
                if field not in m:
                    raise Broken("%s: project %s without %s" % (path, m["module"], field))
            if m["initial"] is None and m["status"] == "passed":
                raise Broken("%s: project %s without its initial changes" % (path, m["module"]))
    if not d.get("modules"):
        raise Broken("%s: no projects" % path)
    return d


def mapping(scalac):
    """scalac's binary names in teq's: a package object's class `p.package`, owned by `<dir>/F.scala`,
    is teq's `p.F$package` by its binary name."""
    owner = {}
    for m in scalac["modules"]:
        for src, cls in m.get("ownership", {}).get("classes", []):
            owner[cls] = src
        for src, cls in m.get("reference", {}).get("ownership", {}).get("classes", []) if isinstance(m.get("reference"), dict) else []:
            owner.setdefault(cls, src)

    def to_teq(cls):
        # A binary class name, an object's with its `$`, maps as its class does.
        name, dollar = (cls[:-1], "$") if cls.endswith("$") else (cls, "")
        if name.endswith(".package") or name == "package":
            src = owner.get(name)
            if src:
                base = os.path.basename(src)[:-len(".scala")]
                return name[:-len("package")] + base + "$package" + dollar
        return cls
    return to_teq


class Diffs:
    def __init__(self, key):
        self.key = key
        self.items = []  # (project, side, kind, cycle, name)
        self.mapped = 0
        self.equivalent = 0

    def add(self, project, side, kind, cycle, name):
        self.items.append((project, side, kind, str(cycle), name))


def compare_sets(diffs, project, kind, cycle, a, b):
    """a, b: scalac's and teq's sets of class names."""
    ma = set(a)
    for x in sorted(ma - set(b)):
        diffs.add(project, "scalac", kind, cycle, x)
    for x in sorted(set(b) - ma):
        diffs.add(project, "teq", kind, cycle, x)


def changes_of(cycle):
    """{(class, kind): {(name, scopes)}} of a cycle's API changes."""
    return {(c[0], c[1]): set((n, ",".join(sc)) for n, sc in c[2]) for c in cycle["changes"]}


def product_mapping(to_teq):
    """A product `<project>:<path>` of scalac's in teq's names: its class's binary name mapped."""
    def to_teq_product(product):
        project, _, path = product.partition(":")
        if not path.endswith(".class"):
            return product
        binary = path[:-len(".class")].replace("/", ".")
        return "%s:%s.class" % (project, to_teq(binary).replace(".", "/"))
    return to_teq_product


def token(text):
    """A name as one word of an expected.txt line."""
    return "_".join(str(text).split()) or "-"


def compare_items(diffs, p, kind, cycle, a, b, mapped=lambda x: x):
    """a, b: scalac's and teq's sets of words, scalac's through `mapped`."""
    ma = set(mapped(x) for x in a)
    if ma != set(a) and ma == set(b):
        diffs.mapped += 1
    for x in sorted(ma - set(b)):
        diffs.add(p, "scalac", kind, cycle, token(x))
    for x in sorted(set(b) - ma):
        diffs.add(p, "teq", kind, cycle, token(x))


def compare_projects(diffs, ms, mt, to_teq):
    p = ms["module"]
    to_teq_product = product_mapping(to_teq)
    if ms["status"] != mt["status"]:
        diffs.add(p, "scalac", "status", "-", ms["status"])
        diffs.add(p, "teq", "status", "-", mt["status"])
    if "not-run" in (ms["status"], mt["status"]):
        return
    # The batches, cycle by cycle, by source.
    bs, bt = [b["sources"] for b in ms["batches"]], [b["sources"] for b in mt["batches"]]
    for i in range(max(len(bs), len(bt))):
        compare_items(diffs, p, "batch", i + 1, set(bs[i]) if i < len(bs) else set(), set(bt[i]) if i < len(bt) else set())
    # The cycles: what zinc's profiler recorded of each.
    cs, ct = ms["cycles"], mt["cycles"]
    for i in range(max(len(cs), len(ct))):
        a = cs[i] if i < len(cs) else None
        b = ct[i] if i < len(ct) else None
        for field in ("invalidated", "packageObjects", "recompiled", "next"):
            compare_sets(diffs, p, field, i + 1, a[field] if a else [], b[field] if b else [])
        for field in ("initialSources", "sources"):
            compare_items(diffs, p, "cycle-" + field, i + 1, set(a[field]) if a else set(), set(b[field]) if b else set())
        compare_items(diffs, p, "continues", i + 1, {str(a["continues"])} if a else set(), {str(b["continues"])} if b else set())
        ca, cb = changes_of(a) if a else {}, changes_of(b) if b else {}
        for k in sorted(set(ca) - set(cb)):
            diffs.add(p, "scalac", "change", i + 1, "%s:%s" % k)
        for k in sorted(set(cb) - set(ca)):
            diffs.add(p, "teq", "change", i + 1, "%s:%s" % k)
        for k in sorted(set(ca) & set(cb)):
            for n, sc in sorted(ca[k] - cb[k]):
                diffs.add(p, "scalac", "names", i + 1, "%s.%s:%s" % (k[0], n, sc))
            for n, sc in sorted(cb[k] - ca[k]):
                diffs.add(p, "teq", "names", i + 1, "%s.%s:%s" % (k[0], n, sc))
        def events(cy):
            return set("%s:%s>%s" % (e[0], ",".join(sorted(e[1])), ",".join(sorted(e[2]))) for e in cy["events"]) if cy else set()
        compare_items(diffs, p, "event", i + 1, events(a), events(b))
    # The initial changes.
    ia, ib = ms["initial"] or {}, mt["initial"] or {}
    for field in ("added", "removed", "changed", "libraryDeps"):
        compare_items(diffs, p, "initial-" + field, "-", set(ia.get(field, [])), set(ib.get(field, [])))
    compare_items(diffs, p, "initial-removedProducts", "-", set(ia.get("removedProducts", [])), set(ib.get("removedProducts", [])), to_teq_product)

    def external(changes, mapped):
        return set("%s:%s:%s" % (mapped(c[0]), c[1], ";".join("%s/%s" % (n, ",".join(sc)) for n, sc in c[2])) for c in changes)
    compare_items(diffs, p, "initial-external", "-", external(ia.get("external", []), to_teq), external(ib.get("external", []), lambda c: c))
    # What each source owns: its classes, and its products by binary name.
    compare_items(diffs, p, "owner", "-", set("%s:%s" % (s, c) for s, c in ms["ownership"]["classes"]),
                  set("%s:%s" % (s, c) for s, c in mt["ownership"]["classes"]))
    compare_items(diffs, p, "product-owner", "-", set("%s:%s" % (s, b) for s, b, _ in ms["ownership"]["products"]),
                  set("%s:%s" % (s, b) for s, b, _ in mt["ownership"]["products"]), lambda x: x.split(":", 1)[0] + ":" + to_teq(x.split(":", 1)[1]))


def error_counts(errors):
    out = {}
    for e in errors:
        key = (e[0], e[1], e[2])
        out[key] = out.get(key, 0) + 1
    return out


def check_reference(diffs, side, m):
    """A side's run against its fresh build."""
    p = m["module"]
    if m["status"] == "not-run":
        return
    ref = m["reference"]
    if ref.get("status") == "not-run":
        diffs.add(p, side, "stale", "-", "<fresh build not run>")
        return
    compiled = set(s for b in m["batches"] for s in b["sources"])
    # The diagnostics, each with its source, line and message, as many times as given.
    run = error_counts(e for b in m["batches"] for e in b["errors"])
    fresh = error_counts(ref["errors"])
    for key in sorted(set(run) | set(fresh)):
        f, line, message = key
        missing = fresh.get(key, 0) - run.get(key, 0)
        name = token("%s:%s:%s" % (f, line, message))
        if missing > 0:
            # An error the fresh build gives in a source the run never compiled is stale.
            for _ in range(missing):
                diffs.add(p, side, "stale" if f not in compiled else "error", "-", name)
        elif missing < 0:
            for _ in range(-missing):
                diffs.add(p, side, "error", "-", name)
    if ref["status"] == "failed":
        if m["status"] == "passed":
            diffs.add(p, side, "stale", "-", "<the_run_passed,_the_fresh_build_fails>")
        return
    if m["status"] == "failed":
        return
    for rel, state, owner, recompiled in m["reference"]["files"]:
        if state == "equivalent":
            diffs.equivalent += 1
        if state in ("same", "equivalent"):
            continue
        kind = "inconsistent" if recompiled else "stale"
        diffs.add(p, side, kind, "-", "%s:%s" % (rel, state))
    # The relations and ownership the run leaves against the fresh build's.
    for part in ("classes", "products", "local"):
        a = set(tuple(x) for x in m["ownership"][part])
        b = set(tuple(x) for x in ref["ownership"][part])
        for x in sorted(a ^ b):
            diffs.add(p, side, "relation", "-", "%s:%s:%s" % (part, "run" if x in a else "fresh", ":".join(x)))
    for part in ("classes", "libraries", "names"):
        a = set(json.dumps(x) for x in m["relations"][part])
        b = set(json.dumps(x) for x in ref["relations"][part])
        for x in sorted(a ^ b):
            diffs.add(p, side, "relation", "-", "%s:%s:%s" % (part, "run" if x in a else "fresh", ",".join(str(y) for y in json.loads(x)).replace(" ", "")))


def check_obligations(diffs, scenario, side, d):
    """The scenario's `expect` lines and `status` on one side."""
    by_project = {m["module"]: m for m in d["modules"]}

    def project_of(path):
        return path.split("/", 1)[0] if "/" in path else "top"

    def compiled(path):
        m = by_project.get(project_of(path))
        return m is not None and m["status"] != "not-run" and any(path in b["sources"] for b in m["batches"])

    def broken(text):
        diffs.add("-", side, "obligation", "-", text.replace(" ", "_"))

    for e in scenario["expect"]:
        variant, who, kind, args = e[0], e[1], e[2], e[3:]
        if who is not None and who != side or variant is not None and variant != d["variant"]:
            continue
        text = " ".join([kind] + args)
        if d.get("refused") and kind != "refused":
            broken("refused, so not " + text)
            continue
        if kind == "refused":
            m = by_project.get(args[0])
            if not m or m["status"] != "refused" or (len(args) > 1 and args[1] not in m.get("failure", "")):
                broken(text)
        elif kind == "compiled":
            for a in args:
                if not compiled(a):
                    broken("compiled " + a)
        elif kind == "not-compiled":
            for a in args:
                if compiled(a):
                    broken("not-compiled " + a)
        elif kind == "cycle":
            n, paths = int(args[0]), args[1:]
            m = by_project.get(project_of(paths[0]))
            got = m["batches"][n - 1]["sources"] if m and m["status"] != "not-run" and len(m["batches"]) >= n else None
            if got is None or set(got) != set(paths):
                broken(text)
        elif kind == "change":
            cls, what = args
            found = False
            for m in d["modules"]:
                if m["status"] == "not-run":
                    continue
                changes = [c for cy in m["cycles"] for c in cy["changes"]] + (m["initial"] or {}).get("external", [])
                found = found or any(c[0] == cls and c[1] == what for c in changes)
            if not found:
                broken(text)
        elif kind == "full":
            m = by_project.get(args[0])
            every = set(s for s, _ in m["reference"]["ownership"]["classes"]) if m and m["status"] != "not-run" else set()
            if not m or not any(every and every <= set(b["sources"]) for b in m["batches"]):
                broken(text)
        elif kind == "transitive":
            # From the cycle at transitiveStep on, the rest of the chain is invalidated at once:
            # the next cycle's batch holds every source named.
            m = by_project.get(args[0])
            step = d["options"]["transitiveStep"]
            if not m or m["status"] == "not-run" or len(m["cycles"]) < step or not m["cycles"][step - 1]["next"] \
                    or len(m["batches"]) <= step or not set(args[1:]) <= set(m["batches"][step]["sources"]):
                broken(text)
        elif kind == "recompiled":
            for cls in args:
                if not any(cls in cy["recompiled"] for m in d["modules"] if m["status"] != "not-run" for cy in m["cycles"]):
                    broken("recompiled " + cls)
        elif kind == "collision":
            # A product a batch reports from one source that the analysis kept says another owns.
            product = args[0]
            m = by_project.get(product.split(":", 1)[0])
            reported = set(src for b in (m["batches"] if m and m["status"] != "not-run" else []) for src, path, _ in b["products"] if path == product)
            owned = set(src for src, _, path in (m["ownership"]["products"] if m and m["status"] != "not-run" else []) if path == product)
            if not reported or not owned or reported == owned:
                broken(text)
        elif kind == "event":
            what, cls = args[0].replace("_", " "), args[1]
            if not any(ev[0] == what and cls in ev[2] for m in d["modules"] if m["status"] != "not-run" for cy in m["cycles"] for ev in cy["events"]):
                broken(text)
        else:
            broken("unknown " + text)
    status = scenario["status"]
    if d.get("refused"):
        return
    fresh_failed = set(e[0] for m in d["modules"] if m["status"] != "not-run" for e in m["reference"]["errors"])
    if status[0] == "pass":
        for m in d["modules"]:
            if m["status"] != "passed" or m["reference"]["status"] != "passed":
                broken("status pass: project %s %s" % (m["module"], m["status"]))
    elif status[0] == "fail":
        if fresh_failed != set(status[1:]):
            broken("status fail %s: the fresh build fails in %s" % (" ".join(status[1:]), " ".join(sorted(fresh_failed)) or "nothing"))
    else:
        broken("status " + " ".join(status))


def compare(results, name, variant, scenario):
    key = "%s/%s" % (name, variant)
    diffs = Diffs(key)
    dumps = {side: load(os.path.join(results, "%s.%s.%s.json" % (name, variant, side))) for side in SIDES}
    for side in SIDES:
        options = dumps[side].get("options")
        if options != scenario["options"][variant]:
            raise Broken("%s: the %s run took the options %s, the variant names %s" % (key, side, options, scenario["options"][variant]))
    to_teq = mapping(dumps["scalac"])
    ps = {m["module"]: m for m in dumps["scalac"]["modules"]}
    pt = {m["module"]: m for m in dumps["teq"]["modules"]}
    if list(ps) != list(pt):
        raise Broken("%s: the sides have other projects" % key)
    # A side that refused a project before the edit is held to its obligations alone.
    if not any(dumps[side].get("refused") for side in SIDES):
        for p in ps:
            compare_projects(diffs, ps[p], pt[p], to_teq)
    for side in SIDES:
        if not dumps[side].get("refused"):
            for m in dumps[side]["modules"]:
                check_reference(diffs, side, m)
        check_obligations(diffs, scenario, side, dumps[side])
    return diffs, dumps


def frozen(dumps):
    """The batches and the outcomes of both sides, as inv-batch and inv-status lines' fields."""
    batches, statuses = set(), set()
    for side in SIDES:
        for m in dumps[side]["modules"]:
            statuses.add((m["module"], side, m["status"]))
            if m["status"] in ("passed", "failed"):
                for i, b in enumerate(m["batches"]):
                    batches.add((m["module"], side, str(i + 1), ",".join(b["sources"]) or "-"))
    return batches, statuses


def merge_sides(items, width):
    """Lines that both sides have named once with `both`."""
    out = set()
    for it in items:
        other = list(it)
        other[1] = "teq" if it[1] == "scalac" else "scalac"
        if tuple(other) in items:
            merged = list(it)
            merged[1] = "both"
            out.add(tuple(merged))
        else:
            out.add(it)
    return out


def main():
    args = sys.argv[1:]
    freeze = "--freeze" in args
    if freeze:
        args.remove("--freeze")

    def option(flag):
        i = args.index(flag)
        v = args[i + 1]
        del args[i:i + 2]
        return v
    scenarios_path, expect_path = option("--scenarios"), option("--expect")
    only = set(option("--only").split(",")) if "--only" in args else None
    results = args[0]
    failures = 0
    try:
        scenarios = read_scenarios(scenarios_path)
        exp_diffs, exp_batches, exp_statuses = read_expected(expect_path)
    except Broken as e:
        print("FAIL    %s" % e)
        sys.exit(1)
    used = set()
    out_lines = []
    totals = {"stale": 0, "missing": 0, "extra": 0, "mapped": 0, "observed": 0, "obligation": 0, "equivalent": 0}
    if only is not None:
        unknown = only - set(scenarios)
        if unknown:
            print("FAIL    no scenario %s" % ", ".join(sorted(unknown)))
            sys.exit(1)
        scenarios = {n: s for n, s in scenarios.items() if n in only}
    keys = set("%s/%s" % (n, v) for n, s in scenarios.items() for v in s["variants"])
    for name, sc in scenarios.items():
        for variant in sc["variants"]:
            key = "%s/%s" % (name, variant)
            try:
                diffs, dumps = compare(results, name, variant, sc)
            except Broken as e:
                failures += 1
                print("FAIL    %s: %s" % (key, e))
                continue
            totals["mapped"] += diffs.mapped
            totals["equivalent"] += diffs.equivalent
            batches, statuses = frozen(dumps)
            batches, statuses = merge_sides(batches, 4), merge_sides(statuses, 3)
            if freeze:
                out_lines += ["inv-status %s %s %s %s" % ((key,) + s) for s in sorted(statuses)]
                out_lines += ["inv-batch %s %s %s %s %s" % ((key,) + b) for b in sorted(batches, key=lambda b: (b[0], b[1], int(b[2])))]
                out_lines += ["inv-diff %s %s %s %s %s %s ?" % ((key,) + d) for d in sorted(set(diffs.items))]
                continue
            want_b = set(b[1:] for b in exp_batches if b[0] == key)
            want_s = set(s[1:] for s in exp_statuses if s[0] == key)
            bad = 0
            for b in sorted(batches ^ want_b):
                bad += 1
                print("FAIL    %s batch %s %s %d: %s %s" % (key, b[0], b[1], int(b[2]), b[3], "is not frozen" if b in batches else "is frozen and no longer compiled"))
            for s in sorted(statuses ^ want_s):
                bad += 1
                print("FAIL    %s status %s %s: %s %s" % (key, s[0], s[1], s[2], "is not frozen" if s in statuses else "is frozen and no longer the outcome"))
            for d in sorted(set(diffs.items)):
                project, side, kind, cycle, item = d
                cls = "obligation" if kind == "obligation" else "stale" if kind == "stale" else \
                    "missing" if kind == "batch" and side == "scalac" else "extra" if kind == "batch" else "observed"
                totals[cls] += 1
                match = [e for e in exp_diffs if e[0] == key and e[1:6] == d]
                if match:
                    used.add(match[0])
                    print("named   %-10s %s %s %s %s %s %s (%s)" % (cls, key, project, side, kind, cycle, item, match[0][6]))
                else:
                    bad += 1
                    print("FAIL    %s %s %s %s %s %s: %s" % (key, project, side, kind, cycle, item, cls))
            failures += bad
            if not bad:
                print("pass    %s: %d differences named, %d mapped" % (key, len(set(diffs.items)), diffs.mapped))
    if freeze:
        print("\n".join(out_lines))
        sys.exit(0)
    for e in exp_batches + exp_statuses:
        if e[0] not in keys and (only is None or e[0].split("/")[0] in only):
            failures += 1
            print("stale   %s: no scenario %s" % (" ".join(e), e[0]))
    for e in exp_diffs:
        if (e[0] in keys or only is None) and e not in used:
            failures += 1
            print("stale   inv-diff %s: nothing differs there now" % " ".join(e))
    print("summary invalidation: %d stale, %d missing, %d extra, %d observed, %d obligations; %d mapped, %d equivalent files; %d failing" % (
        totals["stale"], totals["missing"], totals["extra"], totals["observed"], totals["obligation"], totals["mapped"], totals["equivalent"], failures))
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
