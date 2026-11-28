#!/usr/bin/env python3
"""getclass-folds.py MANIFEST HEAD OUT

Read one JSON object per JVM build in manifest order. Paths and commands are
relative to the working directory of the original builds. Each record has:

  name      program name
  log       complete build log from the base with TEQ_GETCLASS_FOLDS=1
  jar       jar from that build
  build     teq arguments including "compiler", "build" and "-o", "{jar}"
  run       JVM command, with {jar} in its classpath
  scalac    command that runs the same program with scalac 3.8.4

For example, run can be ["java", "-Xverify:all", "-cp", "{jar}:lib.jar", "TeqMain"].
Use a new OUT directory. Exit 0 when all sites meet a criterion, 1 for LISTED, 2 for incomplete evidence.
Run with --self-test to check the criteria.
"""
import argparse
from collections import Counter
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import quote
from zipfile import BadZipFile, ZipFile


def entries(path):
    with ZipFile(path) as jar:
        members = jar.infolist()
        if not any(m.filename.endswith(".class") for m in members):
            raise ValueError(f"{path} contains no class files")
        digests = {m.filename: hashlib.sha256(jar.read(m)).digest() for m in members}
        if len(digests) != len(members):
            raise ValueError(f"{path} contains duplicate names")
        return digests


def command(args, jar=None):
    if jar is not None:
        if not any("{jar}" in arg for arg in args):
            raise ValueError(f"missing {{jar}} in {args!r}")
        args = [arg.replace("{jar}", str(jar)) for arg in args]
    return subprocess.run(args, stdout=subprocess.PIPE, check=True, timeout=300).stdout


IDENTICAL = "FOLDED identical"
DROPPED = "FOLDED pure receiver dropped"
LISTED = "LISTED"
BOX = {"Int": "Integer", "Char": "Character", "Unit": "Void", **{
    name: name for name in ("Byte", "Short", "Long", "Boolean", "Float", "Double")}}
TYPE_READ = re.compile(r"getstatic # // Field java/lang/(\w+)\.TYPE:Ljava/lang/Class;")
BRANCH = re.compile(r"((?:if\w+|goto(?:_w)?|jsr(?:_w)?) )([0-9]+)")
INSN = re.compile(r"\s*(\d+):\s+([a-z]\w*)(.*)")


def receiver_width(op):
    if re.fullmatch(r"(?:[ilfda]load(?:_[0-3]|(?:_w)? \d+)|[ilfd]const_(?:m1|[0-5])|"
                    r"aconst_null|(?:bi|si)push -?\d+)", op):
        return 2 if op[0] in "ld" else 1
    if re.fullmatch(r"ldc(?:_w|2_w)? # // (?:int|long|float|double|String) .*", op):
        return 2 if op.startswith("ldc2_w ") else 1
    return 0


def rows(section):
    for line in section.splitlines():
        m = INSN.fullmatch(line)
        if m:
            operands, sep, comment = m[3].strip().partition("//")
            operands = re.sub(r"^#\d+", "#", operands).strip()
            op = " ".join(filter(None, (m[2], operands, sep, comment.lstrip())))
            yield int(m[1]), op
        else:
            yield None, line


def retarget(op, offsets, last_pc):
    def target(pc):
        return str(offsets[int(pc)])
    if m := BRANCH.fullmatch(op):
        return m[1] + target(m[2])
    if m := re.fullmatch(r"(\s*(?:default|-?\d+):\s+)(\d+)", op):
        return m[1] + target(m[2])
    if m := re.fullmatch(r"\s*(\d+)\s+(\d+)\s+(\d+)\s+(.+)", op):
        end = "end" if int(m[2]) > last_pc else target(m[2])
        return " ".join((target(m[1]), end, target(m[3]), m[4]))
    return op


def dropped_in_method(before, after):
    base, head = list(rows(before)), list(rows(after))
    idx = 0
    retained = []
    offsets = {}
    drops = Counter()
    for pc, op in head:
        if idx == len(base):
            return None
        if (read := TYPE_READ.fullmatch(op)) and base[idx][1] != op:
            start = idx
            stack = []
            while idx < len(base) and base[idx][1] != op:
                old_pc, old_op = base[idx]
                width = receiver_width(old_op)
                if width:
                    stack.append(width)
                elif old_op == "pop" and stack[-1:] == [1]:
                    stack.pop()
                elif old_op == "pop2" and stack[-1:] == [2]:
                    stack.pop()
                elif old_op == "pop2" and stack[-2:] == [1, 1]:
                    del stack[-2:]
                else:
                    return None
                idx += 1
            if stack or idx == len(base):
                return None
            offsets[base[start][0]] = pc
            drops[read[1]] += 1
        old_pc, old_op = base[idx]
        if (pc is None) != (old_pc is None):
            return None
        if pc is not None:
            offsets[old_pc] = pc
        retained.append((old_op, op))
        idx += 1
    if idx != len(base):
        return None
    identity = {pc: pc for pc, _ in head if pc is not None}
    try:
        last_base = max(offsets, default=-1)
        last_head = max(identity, default=-1)
        for old, new in retained:
            if retarget(old, offsets, last_base) != retarget(new, identity, last_head):
                return None
    except KeyError:
        return None
    return drops


def dropped_in_class(before, after):
    base, head = (s.split("    Code:\n") for s in (before, after))
    if len(base) != len(head) or base[0] != head[0]:
        return None
    drops = Counter()
    for a, b in zip(base[1:], head[1:]):
        method = dropped_in_method(a, b)
        if method is None:
            return None
        drops.update(method)
    return drops or None


def disassemble(jar, name):
    member = f"jar:{jar.as_uri()}!/{quote(name)}"
    return command(["javap", "-c", "-p", member]).decode()


def criterion(original, rebuilt, observations, sites, listings):
    if original == rebuilt:
        return IDENTICAL
    if len(set(observations)) != 1 or original.keys() != rebuilt.keys():
        return LISTED
    drops = Counter()
    for name in original:
        if original[name] != rebuilt[name]:
            if not name.endswith(".class"):
                return LISTED
            found = dropped_in_class(*listings(name))
            if not found:
                return LISTED
            drops.update(found)
    return DROPPED if drops == Counter(BOX[s.rsplit(" ", 1)[1]] for s in sites) else LISTED


def self_test():
    base = """public class A {
  java.lang.Class<?> f(int);
    Code:
       0: iload_1
       1: ifeq          10
       4: iload_1
       5: pop
       6: getstatic     #42   // Field java/lang/Integer.TYPE:Ljava/lang/Class;
       9: areturn
      10: aconst_null
      11: areturn
}
"""
    head = base.replace("       4: iload_1\n       5: pop\n", "").replace("6:", "4:").replace(
        "#42", "#7").replace("9: areturn", "7: areturn").replace("10", "8").replace("11:", "9:")
    for want, digest, listing, value in (
        (IDENTICAL, 1, base, b"int"), (DROPPED, 2, head, b"int"), (LISTED, 2, head, b"long"),
        (LISTED, 2, head.replace("       4:", "       3: invokestatic #2 // Method side:()V\n       4:"), b"int"),
        (LISTED, 2, head.replace("ifeq          8", "ifeq          7"), b"int"),
    ):
        got = criterion({"A.class": 1}, {"A.class": digest}, [b"int", value, b"int"], ["a:1:1 Int"],
                        lambda _: (base, listing))
        if got != want:
            raise ValueError(f"self-test expected {want}, got {got}")
    print("getclass-folds self-test: identical and pure receiver dropped accepted, "
          "differing value, added call and changed branch target LISTED")


def compare(program, head, jar):
    sites = []
    for line in Path(program["log"]).read_text().splitlines():
        if line.startswith("getClass fold fired:"):
            match = re.fullmatch(r"getClass fold fired: (.+:[1-9]\d*:[1-9]\d* \w+)", line)
            if not match:
                raise ValueError(f"invalid fold record {line!r}")
            sites.append(match[1])
    if not sites:
        return Counter()
    base = Path(program["jar"]).resolve()
    original = entries(base)
    command([head, *program["build"]], jar)
    rebuilt = entries(jar)
    values = ""
    observations = []
    if original != rebuilt:
        observations = [command(program["run"], path) for path in (base, jar)]
        observations.append(command(program["scalac"]))
        values = " " + " ".join(f"{side}={json.dumps(data.decode())}" for side, data in
                                zip(("base", "head", "scalac"), observations))
    verdict = criterion(original, rebuilt, observations, sites,
                        lambda name: (disassemble(base, name), disassemble(jar, name)))
    for site in sites:
        label = verdict if verdict == LISTED else "FOLDED BY THE TYPER, " + verdict.removeprefix("FOLDED ").upper()
        print(f"{label} {program['name']} {site}{values}")
    return Counter({verdict: len(sites)})


def main():
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return 0
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("head", type=Path)
    parser.add_argument("out", type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True)
    programs = 0
    counts = Counter()
    for line in args.manifest.read_text().splitlines():
        program = json.loads(line)
        programs += 1
        try:
            counts.update(compare(program, str(args.head.resolve()), args.out.resolve() / f"{programs}.jar"))
        except (BadZipFile, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
            raise ValueError(f"incomplete {program['name']}: {error}") from error
    if not programs:
        raise ValueError("empty build manifest")
    print(f"getclass-folds: {programs} builds, {sum(counts.values())} firings, {counts[IDENTICAL]} {IDENTICAL}, "
          f"{counts[DROPPED]} {DROPPED}, {counts[LISTED]} LISTED")
    return int(bool(counts[LISTED]))


if __name__ == "__main__":
    try:
        status = main()
    except (OSError, ValueError, KeyError) as error:
        print(f"getclass-folds: {error}", file=sys.stderr)
        sys.exit(2)
    sys.exit(status)
