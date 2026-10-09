#!/usr/bin/env python3
"""compare.py over recorded dumps (tests/analysis/fixtures, runs of tests/analysis.sh), as they
are and with a part of one side taken away, changed or added: every change must fail the
comparison, whatever of the rest still matches.

The API: the dumps of tests/modules/basic/a, scalac's and teq's graphs, their dependencies held
equal (scalac's on both sides), with a part of teq's graph or hashes changed. The dependencies:
scalac's dumps of tests/modules/diamond2/a (class dependencies, and binary ones of two classes of
one file on classes of one jar) and of tests/modules/sealed/b (a used name with `PatMatTarget`)
against themselves, with a tuple or a stored relation changed. A tuple moved between two classes
of one file, or a binary dependency on a class of a jar other than the one zinc keeps for it
taken away, leaves the stored relations as they were: the tuples alone tell them apart. A mapped
difference passes, and fails with the same tuple otherwise wrong beneath the mapping (another
artifact, another context, other scopes, the other side), with the relation zinc stores from it
on the other side, as does a mapping without its side and fields and one with nothing to
explain. Then the invalidation comparator (invalidation.py) over recorded dumps of its
scenarios (`invalidation_tests`).

  compare_test.py"""

import copy
import json
import os
import subprocess
import sys
import tempfile

here = os.path.dirname(os.path.abspath(__file__))


def compare(scalac, teq, case="basic_a", expect=None):
    with tempfile.TemporaryDirectory() as d:
        a, b, e = os.path.join(d, "scalac.json"), os.path.join(d, "teq.json"), os.path.join(d, "expected.txt")
        with open(a, "w") as f:
            json.dump(scalac, f)
        with open(b, "w") as f:
            json.dump(teq, f)
        extra = []
        if expect is not None:
            with open(e, "w") as f:
                f.write(expect)
            extra = ["--expect", e]
        r = subprocess.run([sys.executable, os.path.join(here, "compare.py"), a, b, "--case", case] + extra, capture_output=True, text=True, timeout=60)
        return r.returncode, r.stdout


def class_index(teq, name, kind):
    for f in teq["api"]["files"]:
        for c in f["classes"]:
            n = teq["api"]["nodes"][c]
            if n[1] == name and n[5] == kind:
                return f, c
    raise KeyError((name, kind))


def hash_entry(teq, name):
    return next(h for h in teq["hashes"] if h["name"] == name)


def drop_companion(teq):
    f, c = class_index(teq, "ma.Rect", "Module")
    f["classes"].remove(c)


def bump(field):
    def change(teq):
        hash_entry(teq, "ma.Rect")[field] += 1
    return change


def drop_hashes_of(teq):
    teq["hashes"] = [h for h in teq["hashes"] if h["name"] != "ma.Util"]


def drop_all_hashes(teq):
    del teq["hashes"]


def drop_name_hashes(teq):
    del hash_entry(teq, "ma.Base")["nameHashes"]


def change_name_hash(teq):
    hash_entry(teq, "ma.Base")["nameHashes"][0][2] += 1


def drop_product(teq):
    for f in teq["api"]["files"]:
        f["products"] = [p for p in f["products"] if p != ["ma.Rect", "ma.Rect$"]]


def add_ghost_product(teq):
    teq["api"]["files"][0]["products"].append(["ghost.C", "ghost.C"])


def move_file(teq):
    teq["api"]["files"][0]["file"] = "wrong-owner/Other.scala"


def drop_declaration(teq):
    _, c = class_index(teq, "ma.Util", "Module")
    nodes = teq["api"]["nodes"]
    structure = nodes[nodes[c][7]]
    twice = next(d for d in structure[2] if nodes[d][1] == "twice")
    structure[2] = [d for d in structure[2] if d != twice]


def deps(side, part):
    return side["deps"][part]


def drop_used_name(side):
    names = deps(side, "usedName")
    names.remove(next(t for t in names if t[0] == "d2a.Circle"))


def add_used_name(side):
    deps(side, "usedName").append(["d2a.Circle", "radius2", ["Default"]])


def narrow_scopes(side):
    t = next(t for t in deps(side, "usedName") if "PatMatTarget" in t[2])
    t[2] = ["Default"]


def drop_class_dependency(side):
    deps(side, "classDependency").pop(0)


def add_class_dependency(side):
    deps(side, "classDependency").append(["d2a.Rect", "d2a.Circle", "DependencyByMemberRef"])


def change_context(side):
    t = deps(side, "classDependency")[0]
    t[2] = "DependencyByMemberRef" if t[2] != "DependencyByMemberRef" else "DependencyByInheritance"


def jar_tuples(side):
    """The binary dependencies on a jar's classes, by source and jar: per referring class the
    binary classes."""
    out = {}
    for entry, binary, cls, source, ctx in deps(side, "binaryDependency"):
        if entry.startswith("jar:"):
            out.setdefault((source, entry), {}).setdefault(cls, set()).add(binary)
    return out


def drop_binary_dependency(side):
    deps(side, "binaryDependency").pop(0)


def add_binary_dependency(side):
    deps(side, "binaryDependency").append(["jar:scala-library", "scala.Option", "d2a.Circle", "Shapes.scala", "DependencyByMemberRef"])


def move_binary_dependency(side):
    """A binary dependency of one class given to another class of its file instead."""
    for (source, entry), by_class in sorted(jar_tuples(side).items()):
        for cls in sorted(by_class):
            for other in sorted(by_class):
                moved = sorted(by_class[cls] - by_class[other])
                if other != cls and moved:
                    t = next(t for t in deps(side, "binaryDependency") if t[0] == entry and t[1] == moved[0] and t[2] == cls)
                    t[2] = other
                    return
    raise AssertionError("no binary dependency to move")


def drop_non_representative(side):
    """A binary dependency on a class of a jar that zinc does not keep as the jar's class, of a
    class whose file depends on the jar through other classes still."""
    for (source, entry), by_class in sorted(jar_tuples(side).items()):
        every = set().union(*by_class.values())
        kept = min(every)
        for cls in sorted(by_class):
            for binary in sorted(by_class[cls] - {kept}):
                if len(every) > 1:
                    tuples = deps(side, "binaryDependency")
                    tuples.remove(next(t for t in tuples if t[0] == entry and t[1] == binary and t[2] == cls))
                    return
    raise AssertionError("no binary dependency to drop")


def drop_stored_class_relation(side):
    deps(side, "relations")["classes"].pop(0)


def change_library_class(side):
    deps(side, "relations")["libraryClasses"][0][1] = "scala.Zzz"


def drop_dependencies(side):
    del side["deps"]


INVALIDATION = os.path.join(here, "fixtures", "invalidation")


def invalidation(dumps, scenarios, expect):
    """invalidation.py over `dumps` ({file name: dump}) with the scenarios and the expected
    lines given as text."""
    with tempfile.TemporaryDirectory() as d:
        for name, dump in dumps.items():
            with open(os.path.join(d, name), "w") as f:
                json.dump(dump, f)
        sc, ex = os.path.join(d, "scenarios.txt"), os.path.join(d, "expected.txt")
        with open(sc, "w") as f:
            f.write(scenarios)
        with open(ex, "w") as f:
            f.write(expect)
        r = subprocess.run([sys.executable, os.path.join(here, "invalidation.py"), d, "--scenarios", sc, "--expect", ex], capture_output=True, text=True, timeout=60)
        return r.returncode, r.stdout


def module(dump, name):
    return next(m for m in dump["modules"] if m["module"] == name)


def invalidation_tests():
    """The invalidation comparator over recorded dumps (tests/analysis/fixtures/invalidation, runs
    of tests/invalidation.sh): they pass with their scenarios and expected lines, and fail with a
    difference beneath an active exception, a changed file owner, a product's owner changed on
    one side, a dropped source batch, a stale relation, a modified name's scope, a removed
    product, an error's message, line or multiplicity in a file both builds report, a run's options, a broken obligation, a frozen batch
    changed or an exception with nothing to explain; a package object's products, which the
    package-object mapping names otherwise, compare equal through it."""
    base = {}
    for f in sorted(os.listdir(INVALIDATION)):
        if f.endswith(".json"):
            base[f] = json.load(open(os.path.join(INVALIDATION, f)))
    scenarios = open(os.path.join(INVALIDATION, "scenarios.txt")).read()
    expect = open(os.path.join(INVALIDATION, "expected.txt")).read()
    passed = failed = 0

    def check(what, change, want, expect_text=None):
        nonlocal passed, failed
        dumps = copy.deepcopy(base)
        if change:
            change(dumps)
        code, out = invalidation(dumps, scenarios, expect if expect_text is None else expect_text)
        if code == want:
            passed += 1
        else:
            failed += 1
            print("FAIL    invalidation: %s: invalidation.py exited %d\n%s" % (what, code, out))

    def teq(dumps, scenario):
        return dumps["%s.default.teq.json" % scenario]

    def owner_moved(dumps):
        o = module(teq(dumps, "result-type"), "a")["ownership"]["classes"]
        i = next(i for i, (s, c) in enumerate(o) if c == "rta.UserA")
        o[i] = ["a/NonUserA.scala", "rta.UserA"]

    def batch_dropped(dumps):
        module(teq(dumps, "result-type"), "a")["batches"].pop(1)

    def stale_relation(dumps):
        m = module(teq(dumps, "result-type"), "a")
        m["relations"]["classes"].append(["rta.NonUserA", "internal", "rta.Lib", "DependencyByInheritance"])

    def another_stale(dumps):
        files = module(teq(dumps, "inline-body"), "a")["reference"]["files"]
        f = next(f for f in files if f[0].startswith("iba/PlainUser"))
        f[1] = "differs"

    def compiled_non_user(dumps):
        module(teq(dumps, "result-type"), "a")["batches"][1]["sources"].append("a/NonUserA.scala")

    def package_object_owner(dumps):
        o = module(teq(dumps, "package-object"), "a")["ownership"]["classes"]
        i = next(i for i, (s, c) in enumerate(o) if c.endswith(".package"))
        o[i] = ["a/UserA.scala", o[i][1]]

    def product_moved(dumps):
        """A product attributed to another source of the file's project on teq's side, in the run's
        analysis and the fresh build's alike."""
        m = module(teq(dumps, "inline-body"), "a")
        for table in (m["ownership"]["products"], m["reference"]["ownership"]["products"]):
            for row in table:
                if row[2] == "a:iba/Caller$.class":
                    row[0] = "a/PlainUser.scala"
        for row in m["reference"]["files"]:
            if row[0] == "iba/Caller$.class":
                row[2] = "a/PlainUser.scala"

    def scope_changed(dumps):
        change = module(teq(dumps, "result-type"), "a")["cycles"][0]["changes"][0]
        change[2] = [[n, ["PatMatTarget"]] for n, _ in change[2]]

    def removed_product(dumps):
        module(teq(dumps, "result-type"), "a")["initial"]["removedProducts"].append("a:rta/Lib$.class")

    def matching_errors(dumps):
        """The same error in a/UserA.scala given by the run's batch and by the fresh build."""
        m = module(teq(dumps, "result-type"), "a")
        m["batches"][1]["errors"].append(["a/UserA.scala", 4, "type mismatch"])
        m["reference"]["errors"].append(["a/UserA.scala", 4, "type mismatch"])
        return m["reference"]["errors"]

    def error_message(dumps):
        matching_errors(dumps)[0][2] = "value f is not a member of rta.Lib"

    def error_line(dumps):
        matching_errors(dumps)[0][1] = 5

    def error_twice(dumps):
        errors = matching_errors(dumps)
        errors.append(list(errors[0]))

    def other_options(dumps):
        teq(dumps, "result-type")["options"]["transitiveStep"] = 4

    def no_dump(dumps):
        del dumps["result-type.default.teq.json"]

    def driver_failed(dumps):
        dumps["result-type.default.teq.json"] = {"scenario": "result-type", "variant": "default", "side": "teq", "error": "boom"}

    check("the recorded dumps", None, 0)
    check("a class's file owner changed", owner_moved, 1)
    check("a source batch dropped", batch_dropped, 1)
    check("a stale relation in the run's analysis", stale_relation, 1)
    check("another stale file beneath the inline-body exception", another_stale, 1)
    check("a non-user compiled again", compiled_non_user, 1)
    check("a package object's class moved to another source", package_object_owner, 1)
    check("a side's dump missing", no_dump, 1)
    check("a product's owner changed on one side, its run and fresh build alike", product_moved, 1)
    check("a modified name's scope changed", scope_changed, 1)
    check("a removed product on one side", removed_product, 1)
    check("the same error in the run and the fresh build", matching_errors, 0)
    check("an error's message other in the fresh build, in the same file", error_message, 1)
    check("an error's line other in the fresh build, in the same file", error_line, 1)
    check("an error the fresh build gives twice and the run once", error_twice, 1)
    check("a run's options other than its variant's", other_options, 1)
    check("the driver's failure", driver_failed, 1)
    lines = expect.splitlines()
    diff_line = next(l for l in lines if l.startswith("inv-diff inline-body/default a teq stale"))
    check("the exception on the other side", None, 1, expect.replace(diff_line, diff_line.replace(" teq ", " scalac ")))
    check("the exception scoped to another cycle", None, 1, expect.replace(diff_line, diff_line.replace(" stale - ", " stale 2 ")))
    check("an exception with nothing to explain", None, 1, expect + "inv-diff result-type/default a teq batch 2 a/NonUserA.scala some-reason\n")
    batch_line = next(l for l in lines if l.startswith("inv-batch result-type/default a both 2"))
    check("a frozen batch changed", None, 1, expect.replace(batch_line, batch_line + ",a/NonUserA.scala"))
    check("a frozen batch missing", None, 1, expect.replace(batch_line + "\n", ""))
    return passed, failed


def main():
    scalac = json.load(open(os.path.join(here, "fixtures", "basic_a.scalac.json")))
    teq = json.load(open(os.path.join(here, "fixtures", "basic_a.teq.json")))
    diamond = json.load(open(os.path.join(here, "fixtures", "diamond2_a.scalac.json")))
    sealed = json.load(open(os.path.join(here, "fixtures", "sealed_b.scalac.json")))
    passed = failed = 0
    for (a, b, case) in ((scalac, teq, "basic_a"), (diamond, diamond, "diamond2_a"), (sealed, sealed, "sealed_b")):
        code, out = compare(a, b, case)
        if code == 0:
            passed += 1
        else:
            failed += 1
            print("FAIL    the recorded dumps of %s do not compare equal:\n%s" % (case, out))
    dep_changes = [
        ("a used name taken away", drop_used_name, diamond, "diamond2_a"),
        ("a used name added", add_used_name, diamond, "diamond2_a"),
        ("a used name's PatMatTarget scope taken away", narrow_scopes, sealed, "sealed_b"),
        ("a class dependency taken away", drop_class_dependency, diamond, "diamond2_a"),
        ("a class dependency added", add_class_dependency, diamond, "diamond2_a"),
        ("a class dependency's context changed", change_context, diamond, "diamond2_a"),
        ("a binary dependency taken away", drop_binary_dependency, diamond, "diamond2_a"),
        ("a binary dependency added", add_binary_dependency, diamond, "diamond2_a"),
        ("a binary dependency moved to another class of its file", move_binary_dependency, diamond, "diamond2_a"),
        ("a jar's class other than zinc's kept one taken away", drop_non_representative, diamond, "diamond2_a"),
        ("a stored class dependency taken away", drop_stored_class_relation, diamond, "diamond2_a"),
        ("a library's kept class changed", change_library_class, diamond, "diamond2_a"),
        ("the dependencies taken away", drop_dependencies, diamond, "diamond2_a"),
    ]
    for what, change, base, case in dep_changes:
        changed = copy.deepcopy(base)
        change(changed)
        code, out = compare(base, changed, case)
        if code == 1:
            passed += 1
        else:
            failed += 1
            print("FAIL    %s: compare.py passed it\n%s" % (what, out))
    # A class dependency one side adds with the relation zinc stores from it: a mapping of the
    # tuple explains both, and the same mapping with nothing to explain is stale.
    added = copy.deepcopy(diamond)
    add_class_dependency(added)
    deps(added, "relations")["classes"].append(["d2a.Circle", "internal", "d2a.Rect", "DependencyByMemberRef"])
    mapping = "diamond2_a classDependency d2a.Circle d2a.Rect some-mapping teq DependencyByMemberRef\n"
    # A binary dependency teq lacks, mapped, and the same tuple otherwise wrong beneath the
    # mapping: with another artifact, another context, or on the other side.
    first = deps(diamond, "binaryDependency")[0]
    entry, binary, cls, source, ctx = first
    dropped = copy.deepcopy(diamond)
    deps(dropped, "binaryDependency").pop(0)
    binary_mapping = "diamond2_a binaryDependency %s %s some-mapping scalac %s,%s,%s\n" % (cls, binary, entry, source, ctx)
    wrong_entry = copy.deepcopy(diamond)
    deps(wrong_entry, "binaryDependency")[0][0] = "jar:another"
    wrong_context = copy.deepcopy(diamond)
    deps(wrong_context, "binaryDependency")[0][4] = "DependencyByInheritance"
    # A used name with its scopes, mapped, and the scopes changed beneath the mapping.
    named = next(t for t in deps(sealed, "usedName") if "PatMatTarget" in t[2])
    without = copy.deepcopy(sealed)
    deps(without, "usedName").remove(named)
    name_mapping = "sealed_b usedName %s %s some-mapping scalac %s\n" % (named[0], named[1], ",".join(named[2]))
    # A tuple teq alone has, mapped, with the relation zinc stores from it on scalac's side.
    name_teq = copy.deepcopy(diamond)
    deps(name_teq, "usedName").append(["d2a.Circle", "reviewName", ["Default"]])
    name_scalac = copy.deepcopy(diamond)
    deps(name_scalac, "relations")["names"].append(["d2a.Circle", "reviewName", ["Default"]])
    name_teq_mapping = "diamond2_a usedName d2a.Circle reviewName some-mapping teq Default\n"
    class_teq = copy.deepcopy(diamond)
    add_class_dependency(class_teq)
    class_scalac = copy.deepcopy(diamond)
    deps(class_scalac, "relations")["classes"].append(["d2a.Circle", "internal", "d2a.Rect", "DependencyByMemberRef"])
    narrowed = copy.deepcopy(sealed)
    next(t for t in deps(narrowed, "usedName") if t[0] == named[0] and t[1] == named[1] and "PatMatTarget" in t[2])[2] = ["PatMatTarget"]
    for what, base, teq_side, case, expect, want in (
        ("a mapped class dependency and its relation", diamond, added, "diamond2_a", mapping, 0),
        ("a mapped class dependency's relation unmapped", diamond, added, "diamond2_a", "", 1),
        ("a dependency mapping with nothing to explain", diamond, diamond, "diamond2_a", mapping, 1),
        ("a dependency mapping without its side and fields", diamond, added, "diamond2_a", "diamond2_a classDependency d2a.Circle d2a.Rect some-mapping\n", 1),
        ("a mapped class dependency on the other side", added, diamond, "diamond2_a", mapping, 1),
        ("a mapped binary dependency teq lacks", diamond, dropped, "diamond2_a", binary_mapping, 0),
        ("a mapped binary dependency with another artifact", diamond, wrong_entry, "diamond2_a", binary_mapping, 1),
        ("a mapped binary dependency with another context", diamond, wrong_context, "diamond2_a", binary_mapping, 1),
        ("a mapped used name teq lacks", sealed, without, "sealed_b", name_mapping, 0),
        ("a mapped used name with other scopes", sealed, narrowed, "sealed_b", name_mapping, 1),
        ("a mapped teq-only used name with its relation on scalac's side", name_scalac, name_teq, "diamond2_a", name_teq_mapping, 1),
        ("a mapped teq-only class dependency with its relation on scalac's side", class_scalac, class_teq, "diamond2_a", mapping, 1),
    ):
        code, out = compare(base, teq_side, case, expect)
        if code == want:
            passed += 1
        else:
            failed += 1
            print("FAIL    %s: compare.py exited %d\n%s" % (what, code, out))
    changes = [
        ("a companion object taken away", drop_companion),
        ("apiHash changed", bump("apiHash")),
        ("extraHash changed", bump("extraHash")),
        ("a class's hashes taken away", drop_hashes_of),
        ("every hash taken away", drop_all_hashes),
        ("a class's name hashes taken away", drop_name_hashes),
        ("a name hash changed", change_name_hash),
        ("a product taken away", drop_product),
        ("a product of a class named nowhere else", add_ghost_product),
        ("a declaration taken away", drop_declaration),
        ("a class's source changed", move_file),
    ]
    for what, change in changes:
        changed = copy.deepcopy(teq)
        change(changed)
        code, out = compare(scalac, changed)
        if code == 1:
            passed += 1
        else:
            failed += 1
            print("FAIL    %s: compare.py passed it\n%s" % (what, out))
    p, f = invalidation_tests()
    passed += p
    failed += f
    print("compare_test: %d passed, %d failed" % (passed, failed))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
