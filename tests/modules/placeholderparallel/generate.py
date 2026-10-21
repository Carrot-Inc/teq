# Module b's filler: definitions past the parallel parse's size (`frontend::PARALLEL_THRESHOLD_BYTES`, 256 KiB of program
# source), so that the whole build parses on several threads while module a alone parses on one.
import sys

lines = ["package pb", "", "object Filler:"] + [f"  def f{i}(x: Int): Int = x + {i}" for i in range(9000)]
with open(f"{sys.argv[1]}/b/Filler.scala", "w") as out:
    out.write("\n".join(lines) + "\n")
