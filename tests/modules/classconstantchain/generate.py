# Module b's fillers: four files of small objects past the typer's parallel threshold (`frontend::TYPER_THRESHOLD_BYTES`,
# 512 KiB of program source, no unit over half of it), so that the whole build types its bodies on several workers while
# module a alone types on one.
import sys

for k in range(4):
    lines = ["package ccb", ""]
    for o in range(60):
        lines.append(f"object Filler{k}x{o}:")
        lines += [f"  def f{i}(x: Int): Int = x + {i}" for i in range(80)]
    with open(f"{sys.argv[1]}/b/Filler{k}.scala", "w") as out:
        out.write("\n".join(lines) + "\n")
