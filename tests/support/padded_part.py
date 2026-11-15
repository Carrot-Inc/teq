#!/usr/bin/env python3
"""padded_part.py <dir> [declared]: the two large sources of a part-build fixture of tests/workers.sh, written
into <dir> beside the fixture's own.

Helper.scala is an object of 120 methods of 25 vals each, their results inferred, or declared with the word;
Use.scala holds `read` and 1800 small methods. The fixture's initialiser calls every method of Helper, so that
the thread that types it under the loader's lock asks for 120 signatures while the other workers, with Use's
methods between them, are typing Helper's bodies, and meets one of them at work in nearly every schedule."""
import sys

out = sys.argv[1]
result = ": Int" if sys.argv[2:] == ["declared"] else ""
helper = ["package inc", "object Helper:"]
for i in range(1, 121):
    helper.append(f"  def c{i}(n: Int){result} =")
    for j in range(1, 26):
        helper.append(f"    val v{j} = List(n + {j}, {i}).map(k => k * {j % 7 + 1}).sum")
    helper.append("    v1 + v25")
use = ["package inc", "def read = new Child"]
for i in range(1, 1801):
    use.append(f"def pad{i}(n: Int): Int = {{ val xs = List(n, {i}).map(k => k * {i % 5 + 2}); xs.sum + xs.length }}")
for name, lines in (("Helper.scala", helper), ("Use.scala", use)):
    with open(f"{out}/{name}", "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
