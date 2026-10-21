# The toy module of fix-verb's design pass: 1,000 files of 4 lines each, no jars, nothing a rule reports.
import os, sys
d = sys.argv[1]
os.makedirs(d, exist_ok=True)
for i in range(1000):
    with open(os.path.join(d, f"M{i:04d}.scala"), "w") as f:
        f.write(f"package toy\n\nobject M{i:04d}:\n  def value: Int = {i}\n")
