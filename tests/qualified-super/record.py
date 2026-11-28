#!/usr/bin/env python3
"""record.py writes expected.tsv from the Scala 3.8.4 compiler jars in COURSIER_CACHE
(default ~/.cache/coursier/v1). Compile the generated packages together, dropping
refusals batch by batch. Run each remaining @main on the JVM. A refusal records
the first line of the scalac message. A JVM verification failure records VERIFY and
the first VerifyError line. Compile the witnesses individually with scala-cli and
record their stdout, refusal or runtime failure, and selected javap listings.
JOBS controls concurrent mains (default 8)."""
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

from witnesses import MEMBER_CLASSES, field, member_listing, witness_names

HERE = Path(__file__).resolve().parent
OUT = HERE.parents[1] / "out" / "qualified-super"
CACHE = Path(os.environ.get("COURSIER_CACHE", Path.home() / ".cache/coursier/v1"))
M2 = CACHE / "https/repo1.maven.org/maven2"
LIB = M2 / "org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"
JARS = [LIB] + [M2 / f"org/scala-lang/{jar}/3.8.4/{jar}-3.8.4.jar" for jar in
                ("scala3-compiler_3", "scala3-interfaces", "tasty-core_3", "scala3-library_3")]
JARS += [M2 / "org/scala-lang/modules/scala-asm/9.9.0-scala-1/scala-asm-9.9.0-scala-1.jar",
         M2 / "org/scala-sbt/compiler-interface/1.10.7/compiler-interface-1.10.7.jar"]


def first_message(block):
    lines = iter(block.splitlines())
    for line in lines:
        if re.fullmatch(r"\s*\|\s*\^+\s*", line):
            break
    for line in lines:
        message = line.lstrip()
        if not message.startswith("|"):
            break
        text = message[1:].strip()
        if text:
            return text
    sys.exit(f"missing scalac message in error block\n-- {block}")


def refusal_messages(log):
    bad = {}
    for block in re.split(r"(?m)^-- ", log)[1:]:
        header = block.partition("\n")[0]
        match = re.match(r".*?Error: .*?/rows/([^/]+)/Main\.scala:\d+:\d+", header)
        if match:
            bad.setdefault(match[1], first_message(block))
    return bad


def compile_rows(work, names):
    classes = work / "classes"
    remaining = list(names)
    refused = {}
    while remaining:
        classes.mkdir()
        files = [work / "rows" / n / "Main.scala" for n in remaining]
        result = subprocess.run(
            ["java", "-XX:-UsePerfData", "-Xss16m", "-Xmx3g", "-cp", os.pathsep.join(map(str, JARS)),
             "dotty.tools.dotc.Main", "-color:never", "-classpath", LIB, "-d", classes, *files],
            capture_output=True, text=True, timeout=360)
        if result.returncode == 0:
            break
        log = result.stdout + result.stderr
        if result.returncode != 1 or "while compiling:" in log:
            sys.exit(f"record.py: scalac exited {result.returncode}\n{log}")
        bad = refusal_messages(log)
        if not bad or not bad.keys() <= set(remaining):
            sys.exit(f"record.py: could not attribute scalac errors\n{log}")
        refused.update(bad)
        remaining = [n for n in remaining if n not in bad]
        shutil.rmtree(classes)
    return remaining, refused


def record_witness(row):
    classes = row / "classes"
    java = sorted(row.glob("*.java"))
    result = subprocess.run(["scala-cli", "compile", "-S", "3.8.4", "--server=false", "-color:never",
                             "-d", classes, "Main.scala", *java],
                            cwd=row, capture_output=True, text=True, timeout=360)
    if result.returncode:
        log = result.stderr + result.stdout
        messages = refusal_messages(log)
        if row.name not in messages:
            raise ValueError(f"{row.name}: unexpected scalac failure\n{log}")
        return "REFUSED\t" + field(messages[row.name]), None
    cp = f"{classes}{os.pathsep}{LIB}"
    if java:
        subprocess.run(["javac", "-classpath", cp, "-d", classes, *java], check=True, timeout=120)
    listing = member_listing(classes, row.name) if row.name in MEMBER_CLASSES else None
    main, = classes.rglob("run.class")
    main_class = ".".join(main.relative_to(classes).with_suffix("").parts)
    result = subprocess.run(["java", "-XX:-UsePerfData", "-cp", cp, main_class],
                            capture_output=True, text=True, timeout=120)
    if result.returncode:
        error = re.search(r'(?m)^Exception in thread "[^"]+" .*$', result.stderr)
        if not error:
            raise ValueError(f"{row.name}: unexpected JVM failure\n{result.stderr}")
        return "PIN\t" + field(error[0]), listing
    return field(result.stdout.removesuffix("\n")), listing


def main():
    for jar in JARS:
        if not jar.is_file():
            sys.exit(f"record.py: missing {jar}")
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=OUT) as tmp:
        work = Path(tmp)
        subprocess.run([sys.executable, HERE / "gen.py", work], check=True, stdout=subprocess.DEVNULL, timeout=60)
        names = (work / "rows.txt").read_text().splitlines()
        witnesses = witness_names()
        remaining, refused = compile_rows(work, [n for n in names if n not in witnesses])

        def run(name):
            result = subprocess.run(
                ["java", "-XX:-UsePerfData", "-Xss32m", "-cp", f"{work / 'classes'}{os.pathsep}{LIB}",
                 f"g_{name}.runCell"], capture_output=True, text=True, timeout=120)
            if result.returncode:
                for line in result.stderr.splitlines():
                    if re.match(r'^(?:Exception in thread "[^"]+" |Caused by: )?java\.lang\.VerifyError(?:$|:)', line):
                        return f"VERIFY\t{line}"
            if result.returncode or len(result.stdout.splitlines()) != 1:
                sys.exit(f"record.py: {name} JVM exit {result.returncode}\n{result.stdout}{result.stderr}")
            return result.stdout.splitlines()[0]

        with ThreadPoolExecutor(int(os.environ.get("JOBS", "8"))) as pool:
            answers = dict(zip(remaining, pool.map(run, remaining)))
        answers.update((n, f"REFUSED\t{message}") for n, message in refused.items())
        listings = {}
        for name in witnesses:
            answer, listing = record_witness(work / "rows" / name)
            answers[name] = answer
            if listing is not None:
                listings[name] = field(listing)
    if listings.keys() != MEMBER_CLASSES.keys():
        raise ValueError("missing scalac JVM member listings")
    member_lines = ["# program\tscalac 3.8.4 JVM members (javap -p -s)"]
    member_lines.extend(f"{n}\t{listing}" for n, listing in listings.items())
    (HERE / "expected-members.tsv").write_text("\n".join(member_lines) + "\n")
    lines = ["# program\tscalac 3.8.4 observable (output, REFUSED or VERIFY with the first error line)"]
    lines.extend(f"{n}\t{answers[n]}" for n in names)
    (HERE / "expected.tsv").write_text("\n".join(lines) + "\n")
    refusals = sum(answer.startswith("REFUSED\t") for answer in answers.values())
    print(f"record.py: {len(names)} programs, scalac compiles {len(names) - refusals} and refuses {refusals}; "
          f"expected.tsv and {len(listings)} listings in expected-members.tsv written")


if __name__ == "__main__":
    main()
