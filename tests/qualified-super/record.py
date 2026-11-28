#!/usr/bin/env python3
"""record.py writes expected.tsv from the Scala 3.8.4 compiler jars in COURSIER_CACHE
(default ~/.cache/coursier/v1). Compile the generated packages together, dropping
refusals batch by batch. Run each remaining @main on the JVM. A refusal records
the first line of the scalac message. A JVM verification failure records VERIFY and
the first VerifyError line. JOBS controls concurrent mains (default 8)."""
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

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


def main():
    for jar in JARS:
        if not jar.is_file():
            sys.exit(f"record.py: missing {jar}")
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=OUT) as tmp:
        work = Path(tmp)
        subprocess.run([sys.executable, HERE / "gen.py", work], check=True, stdout=subprocess.DEVNULL, timeout=60)
        names = (work / "rows.txt").read_text().splitlines()
        remaining, refused = compile_rows(work, names)

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
    lines = ["# program\tscalac 3.8.4 observable (output, REFUSED or VERIFY with the first error line)"]
    lines.extend(f"{n}\t{answers[n]}" for n in names)
    (HERE / "expected.tsv").write_text("\n".join(lines) + "\n")
    print(f"record.py: {len(names)} programs, scalac compiles {len(remaining)} and refuses {len(refused)}; expected.tsv written")


if __name__ == "__main__":
    main()
