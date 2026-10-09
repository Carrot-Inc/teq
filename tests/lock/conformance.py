# python3 tests/lock/conformance.py: check-export.py's reader reads the corpus to corpus.json's tree
# and refuses every document of refused.txt on its line, its writer writes the corpus's tree and the
# example's lock to their bytes, and PyYAML (YAML 1.1), when installed, reads both to the same trees.
import importlib.util, json, os, sys

here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("c", os.path.join(here, "../../integrations/sbt/example/check-export.py"))
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
failed = 0


def check(ok, what, found=None):
    global failed
    if ok:
        print(f"lock: {what}")
    else:
        print(f"FAIL lock: {what}" + ("" if found is None else f": {found}"), file=sys.stderr)
        failed += 1


def read(path):
    with open(os.path.join(here, path), encoding="utf-8", newline="") as f:
        return f.read()


corpus, tree = read("corpus.lock"), json.loads(read("corpus.json"))
example = read("../../integrations/sbt/example/teq.lock")
check(c.parse(corpus) == tree, "check-export.py reads the corpus to corpus.json's tree")
check(c.canonical(tree) == corpus, "check-export.py writes corpus.json's tree to the corpus's bytes")
check(c.canonical(c.parse(example)) == example, "check-export.py writes the example's lock to its bytes")
cases = read("refused.txt").split("\n=== ")[1:]
refused = 0
for case in cases:
    head, _, text = case.partition("\n")
    line = head.split(" ")[0]
    try:
        c.parse(text if text.endswith("\n") else text + "\n")
        message = "it read"
    except c.LockError as e:
        message = str(e)
    if message.startswith(f"line {line}: "):
        refused += 1
    else:
        check(False, f"check-export.py refuses {head[len(line) + 1:]} on line {line}", message)
check(refused == len(cases), f"check-export.py refuses the {len(cases)} documents of refused.txt, each on its line")
try:
    import yaml
except ImportError:
    yaml = None
if yaml is None:
    print("lock: note: no PyYAML here, the YAML 1.1 reading not checked")
else:
    check(yaml.safe_load(corpus) == tree and yaml.safe_load(example) == c.parse(example), f"PyYAML {yaml.__version__} (YAML 1.1) reads the corpus and the example's lock to the same trees")
print(f"lock: {failed} failed" if failed else "lock: all passed")
sys.exit(1 if failed else 0)
