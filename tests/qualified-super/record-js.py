#!/usr/bin/env python3
"""record-js.py writes expected-js.tsv from scala-cli run --js for the qualified
trait calls, their nested twins, the pinning pair, sites h through o and the witness programs."""
from pathlib import Path
import re
import subprocess
import tempfile

from gen import js_reference_cells, main as generate
from record import HERE, OUT, refusal_messages
from witnesses import field


def observable(row):
    result = subprocess.run(
        ["scala-cli", "run", "--js", "-S", "3.8.4", "--server=false", "-color:never",
         "Main.scala", *(p.name for p in sorted(row.glob("*.java")))],
        cwd=row, capture_output=True, text=True, timeout=360)
    if result.returncode == 0:
        if row.name.startswith("qw_"):
            return field(result.stdout.removesuffix("\n"))
        lines = result.stdout.splitlines()
        if len(lines) != 1:
            raise ValueError(f"{row.name}: expected one output line, got {lines!r}")
        return lines[0]
    log = result.stderr + result.stdout
    messages = refusal_messages(log)
    if row.name in messages:
        return f"REFUSED\t{messages[row.name]}"
    log = log.replace(str(row) + "/", "")
    if row.name.startswith("qw_"):
        # scala-cli can warn about Java sources before the Scala.js linker reports its error.
        error = re.search(r'(?m)^(?:Referring to non-existent |Exception in thread "[^"]+" org\.scalajs\.linker\.|'
                          r'(?:[\w.$]+Error|[\w.$]+Exception): ).*$', log)
        if error:
            return "PIN\t" + field(error[0])
        raise ValueError(f"{row.name}: unexpected Scala.js failure\n{log}")
    first = next((line.strip() for line in log.splitlines() if line.strip()), "")
    return f"FAILED\texit {result.returncode} {first}".rstrip()


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    names = list(js_reference_cells())
    lines = [
        "# program\tscalac 3.8.4 Scala.js observable (output, REFUSED or FAILED with diagnostic)",
        "# For qs_pin_A_a_def0 and qs_pin_A_b_def0 scalac's two back ends print DIFFERENT values for one program: the JVM",
        "# Mid (qs_pin_A_a_def0 by its twin), Scala.js A. Neither is mended toward the other: teq's JVM prints Mid, its",
        "# JavaScript and its interpreter A. dotty's trees call A.m: ResolveSuper.rebindSuper (transform/ResolveSuper.scala",
        "# 85-118 at 3.8.4) implements T's super accessor in C (javap: T$$m$super$A is aload_0; invokespecial A.m), and the",
        "# JVM begins that invokespecial's lookup at C's direct superclass Mid. The scala-dev#143 check",
        "# (transform/SuperAccessors.scala 138-153) refuses such a call, but it tests the qualifier class T, not C.",
    ]
    with tempfile.TemporaryDirectory(dir=OUT) as tmp:
        work = Path(tmp)
        generate(work)
        for name in names:
            lines.append(f"{name}\t{observable(work / 'rows' / name)}")
    (HERE / "expected-js.tsv").write_text("\n".join(lines) + "\n")
    print(f"record-js.py: {len(names)} programs; expected-js.tsv written")


if __name__ == "__main__":
    main()
