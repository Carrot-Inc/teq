#!/usr/bin/env python3
"""Compare separately compiled qualified-super libraries and clients with expected-products.tsv.
Write out/qualified-super/products.tsv. A refusal or difference is a table entry.
--check compares differences with pending-products.txt (exit 1 on a mismatch).
--update removes stale records; --update --add-new also records new differences.
Setup errors exit 2.
TEQ, NODE and COURSIER_CACHE select the tools and Scala 3.8.4 jars as in run.py.
"""
from collections import Counter
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

from gen import QUALIFIERS, program
from record import JARS
from run import HERE, LIB, NODE, OUT, TEQ, check_pending, classify, command, field, pending_args

sys.path.insert(0, str(HERE.parent / "tasty/representation"))
from oracle import InjectivityFailure, MODIFIERS, enc_nodes, name_json_text, record_line, tag_children, tag_owners

LEGS = ("scalac-client-over-teq", "from-tasty-over-teq", "teq-over-scalac", "pickle-listing",
        "from-tasty-members", "accessor-entry")
REFERENCE = HERE / "expected-products.tsv"
SCALA = ["scala-cli", "compile", "-S", "3.8.4", "--server=false", "-color:never"]


def libraries():
    for qualifier in ("class", "object", "trait"):
        for mix in (("none", "A", "U") if qualifier == "class" else ("none", "A")):
            for member in ("def0", "def1"):
                yield program(qualifier, mix, "b", member)


def split_library(work, name, source):
    row = work / name
    row.mkdir()
    library, main = source.split("@main ", 1)
    (row / "Library.scala").write_text(library)
    (row / "Client.scala").write_text(source.splitlines()[0] + "\n\n@main " + main)
    return row


def scala_command(row, args, *, regenerated_main=False):
    try:
        p = subprocess.run(args, cwd=row, text=True, capture_output=True, timeout=120)
    except subprocess.TimeoutExpired:
        return "crash", f"{args[0]}: timeout"
    if p.returncode == 0:
        return "out", p.stdout.removesuffix("\n")
    log = (p.stderr + p.stdout).replace(str(row) + "/", "")
    if regenerated_main and p.returncode == 1:
        exception = re.search(r'^Exception in thread "main" .+$', p.stderr, re.M)
        return ("out", exception[0]) if exception else ("run error", log.strip())
    error = re.search(r"^-- [^\n]*Error: .*?(?=^-- |\Z)", log, re.M | re.S)
    if p.returncode == 1 and error and "while compiling:" not in log:
        return "refused", error[0].strip()
    return "crash", f"{args[0]}: exit {p.returncode}\n{log.strip()}"


def require(row, stage, result):
    kind, value = result
    if kind != "out":
        raise ValueError(f"{row.name} {stage}: {kind}\n{value}")
    return value


def scalac_library(row):
    classes = row / "scalac"
    classes.mkdir()
    require(row, "scalac library", scala_command(row, [*SCALA, "Library.scala", "-d", classes]))
    return classes


def compile_jvm_client(row, library, classes):
    classes.mkdir()
    return scala_command(row, [*SCALA, "--classpath", library, "Client.scala", "-d", classes])


def run_jvm_client(row, client, library, *, regenerated=False):
    cp = os.pathsep.join(map(str, (client, library, LIB)))
    args = ["java", "-XX:-UsePerfData", "-cp", cp, f"g_{row.name}.runCell"]
    if regenerated:
        return scala_command(row, args, regenerated_main=True)
    return command(args, row, "java")


def class_members(row, classes):
    files = sorted(classes.rglob("*.class"))
    if not files:
        return "run error", "no regenerated class files"
    kind, value = command(["javap", "-p", "-s", *files], row, "javap")
    if kind != "out":
        return kind, value
    return "out", "\n".join(line for line in value.splitlines() if not line.startswith('Compiled from "'))


def from_tasty(row, library, client):
    classes = row / "regen"
    classes.mkdir()
    files = sorted(library.rglob("*.tasty"))
    if not files:
        missing = ("run error", "no library TASTy")
        return missing, missing
    # Gate.scala's regen checks every phase, then the client sees only the regenerated library.
    args = ["java", "-XX:-UsePerfData", "-cp", os.pathsep.join(map(str, JARS)), "dotty.tools.dotc.Main",
            "-from-tasty", "-Xallow-outline-from-tasty", "-Ycheck:all", "-usejavacp",
            "-classpath", f"{library}{os.pathsep}{LIB}", "-color:never", "-d", classes, *files]
    result = scala_command(row, args)
    if result[0] != "out":
        return result, result
    return run_jvm_client(row, client, classes, regenerated=True), class_members(row, classes)


def pickle_members(row, library):
    qualifier = row.name.split("_")[1]
    name = QUALIFIERS[qualifier][0]
    package = "g_" + row.name
    tasty = library / package / (name + ".tasty")
    if not tasty.is_file():
        missing = ("run error", f"missing {package}/{tasty.name}")
        return missing, missing
    # --nodes is the record form of --trees, with the recursive name-table entry at each definition.
    result = command([TEQ, "tasty", "--nodes", tasty], row, "tasty")
    if result[0] != "out":
        return result, result
    dump = row / "nodes.tsv"
    dump.write_text(result[1] + "\n")
    try:
        nodes, roots = enc_nodes(dump)
    except (ValueError, InjectivityFailure) as error:
        failure = ("crash", f"tasty: {error}")
        return failure, failure
    owner = ("object" if qualifier == "object" else "class") + f":{package}.{name}"
    owners = tag_owners(nodes, roots)
    if owner not in owners:
        missing = ("run error", f"missing {owner}")
        return missing, missing
    template, = (k for k in nodes[owners[owner]][3] if nodes[k][1] == "TEMPLATE")
    members = []
    accessors = []
    for k in nodes[template][3]:
        _, tag, _, children, fields = nodes[k]
        if tag in ("DEFDEF", "VALDEF", "TYPEDEF", "PARAM", "TYPEPARAM"):
            parts = [tag, name_json_text(fields["name"])]
            # TreeUnpickler.normalizeFlags (3.8.4, lines 554-561) reads Method from DEFDEF
            # and an ordinary method's Deferred from its absent right-hand side.
            if tag == "DEFDEF" and fields["name"] != ["UTF8", "<init>"] and not tag_children(nodes, k, "rhs"):
                parts.append("empty-rhs")
            parts.extend(sorted(nodes[c][1] for c in children if nodes[c][1] in MODIFIERS))
            if tag == "DEFDEF" and fields["name"][0] == "SUPERACCESSOR":
                record = record_line(k, template, tag, fields.items())
                accessors.append(f"member[{len(members)}]\n{record}\n" + " ".join(parts[2:]))
            members.append(" ".join(parts))
    listing = " | ".join(members)
    entry = ("out", "\n".join(accessors)) if accessors else ("run error", f"missing super accessor: {listing}")
    return ("out", listing), entry


def teq_clients(row, library):
    args = ["--sourceroot", row, "--classpath", library, "Client.scala"]
    js = command([TEQ, "compiler", "build", *args, "-o", "client.js"], row, "js build", compile_step=True)
    if js[0] == "out":
        js = command([NODE, "client.js"], row, "node")
    interp = command([TEQ, "interp", *args], row, "interp", compile_step=True)
    if js == interp:
        return js
    kind = next((k for k in ("crash", "refused") if k in (js[0], interp[0])), "run error")
    return kind, f"js {js[0]}: {js[1]} | interp {interp[0]}: {interp[1]}"



def observations(row, expected):
    scalac = scalac_library(row)
    products = row / "teq"
    build = command([TEQ, "compiler", "build", "--target", "jvm", "--products", products,
                     "--sourceroot", row, "--classpath", LIB, "Library.scala"], row, "jvm build", compile_step=True)

    if build[0] == "out":
        teq_client = row / "client-teq"
        direct = compile_jvm_client(row, products, teq_client)
        if direct[0] == "out":
            direct = run_jvm_client(row, teq_client, products)
        scalac_client = row / "client-scalac"
        require(row, "scalac client", compile_jvm_client(row, scalac, scalac_client))
        regenerated, members = from_tasty(row, products, scalac_client)
        listing, entry = pickle_members(row, products)
    else:
        direct = regenerated = members = listing = entry = build

    results = (direct, regenerated, teq_clients(row, scalac), listing, members, entry)
    for leg, (kind, actual) in zip(LEGS, results):
        reference = expected[row.name, leg]
        actual = field(actual)
        yield row.name, leg, classify(kind, actual, (reference,)), actual, reference



def main():
    args = pending_args(__doc__, HERE / "pending-products.txt")
    expected = {}
    for line in REFERENCE.read_text().splitlines():
        if not line.startswith("#"):
            name, leg, value = line.split("\t")
            if (name, leg) in expected:
                raise ValueError(f"duplicate reference for {name} {leg}")
            expected[name, leg] = value
    if expected.keys() != {(name, leg) for name, _ in libraries() for leg in LEGS}:
        raise ValueError("libraries, legs and expected-products.tsv disagree")
    counts = Counter()
    pending = set()
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=OUT) as tmp:
        work = Path(tmp)
        table = work / "products.tsv"
        with table.open("w") as fh:
            print("library", "leg", "class", "teq", "reference", sep="\t", file=fh)
            for name, source in libraries():
                for result in observations(split_library(work, name, source), expected):
                    print(*result, sep="\t", file=fh)
                    counts[result[2]] += 1
                    if result[2] != "same":
                        pending.add("\t".join(result))
        table.replace(OUT / "products.tsv")
    totals = ", ".join(f"{counts[v]} {v}" for v in ("same", "refused", "differs", "crash"))
    print(f"products.py: {sum(counts.values())} rows, {totals}; products.tsv written")
    if args.check or args.update:
        return check_pending(args.pending, "qualified-super products", pending, args.update, args.add_new)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"products.py: {error}", file=sys.stderr)
        sys.exit(2)
