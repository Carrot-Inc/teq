# bench/fix/inventory.py <inventory> <inputs> <dir>...: a corpus's identity from the inputs the engine resolved for its
# jobs (`TEQ_FIX_INPUTS`'s `<job>\t<path>` lines: the compiler's own input set, its roots, the links it follows, its
# exclusions; no walk of the directories), each path named under the corpus directory it lies in (that directory's name,
# then the path below it, a linked directory's name among it) and digested with its bytes: printed `<count>:<digest>`,
# the count of distinct files; the inventory, a name and its bytes' digest a line in the names' order, written to
# <inventory> (bench/fix/run.sh keeps it by the digest, so that a run over another corpus names the files it lacks
# against the baseline's).
import hashlib, os, sys

out, inputs, dirs = sys.argv[1], sys.argv[2], sys.argv[3:]
roots = [(os.path.normpath(d), os.path.basename(os.path.normpath(d))) for d in dirs]
names = {}
for line in open(inputs):
    line = line.rstrip("\n")
    if not line:
        continue
    _, _, path = line.partition("\t")
    norm = os.path.normpath(path)
    name = norm
    for root, base in roots:
        if norm == root or norm.startswith(root + os.sep):
            name = base + "/" + os.path.relpath(norm, root).replace(os.sep, "/")
            break
    if name not in names:
        with open(path, "rb") as fh:
            names[name] = hashlib.sha256(fh.read()).hexdigest()
text = "".join(f"{n} {d}\n" for n, d in sorted(names.items()))
with open(out, "w") as fh:
    fh.write(text)
print(f"{len(names)}:{hashlib.sha256(text.encode()).hexdigest()[:16]}")
