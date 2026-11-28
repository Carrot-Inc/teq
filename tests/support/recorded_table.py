"""Check generated table legs against their scalac observations and exact pending differences."""
import importlib
import json
import shutil
import sys
from collections import Counter
from pathlib import Path
from tempfile import NamedTemporaryFile, TemporaryDirectory


def check_names(found, expected, label):
    found = Counter(found)
    expected = set(expected)
    missing = expected - found.keys()
    stray = found.keys() - expected
    duplicates = [name for name, n in found.items() if n > 1]
    if missing or stray or duplicates:
        raise ValueError(f"{label}: missing {sorted(missing)}, stray {sorted(stray)}, duplicate {sorted(duplicates)}")


def load(path, columns):
    records = {}
    for line in path.read_text().splitlines():
        if line.startswith("#"):
            continue
        cell, leg, *fields = line.split("\t")
        if len(fields) != columns or (cell, leg) in records:
            raise ValueError(f"{path}: malformed or duplicate record: {line}")
        records[cell, leg] = (*fields[:-1], json.loads(fields[-1]))
    return records


def save(path, header, records):
    path.parent.mkdir(parents=True, exist_ok=True)
    with NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as stream:
        print("# cell", "leg", *header, sep="\t", file=stream)
        for key, fields in sorted(records.items()):
            print(*key, *fields[:-1], json.dumps(fields[-1]), sep="\t", file=stream)
    Path(stream.name).replace(path)


def differences(expected, pending, actual, update=False):
    errors = []
    fixed = set()
    for key in sorted(expected.keys() | pending.keys() | actual.keys()):
        if key not in actual:
            problem = "missing leg"
        elif key not in expected:
            problem = "stray leg"
        elif actual[key] == expected[key]:
            if key not in pending:
                continue
            fixed.add(key)
            if update:
                continue
            problem = "now matches scalac"
        elif key not in pending:
            problem = "unlisted difference"
        elif actual[key] != pending[key][1:]:
            problem = "changed difference"
        else:
            continue
        errors.append((key, problem))
    return errors, fixed


def selftest(table):
    one, two = ("one", "jvm"), ("two", "js")
    expected = {one: ("ok", "a"), two: ("ok", "b")}
    pending = {two: ("row", "refuses", "reason")}
    actual = {one: expected[one], two: pending[two][1:]}
    plants = [(actual, []),
              (actual | {one: ("ok", "b")}, [(one, "unlisted difference")]),
              (actual | {two: expected[two]}, [(two, "now matches scalac")]),
              (actual | {two: ("refuses", "another reason")}, [(two, "changed difference")]),
              ({one: actual[one]}, [(two, "missing leg")]),
              (actual | {("three", "js"): actual[one]}, [(("three", "js"), "stray leg")])]
    for planted, wanted in plants:
        if differences(expected, pending, planted)[0] != wanted:
            raise ValueError(f"selftest missed {wanted}")
    if differences(expected, pending, expected, update=True) != ([], {two}):
        raise ValueError("selftest missed a repaired pending leg")
    print(f"{table} recorded selftest: unlisted, repaired, changed, missing and stray legs found")


def judge(table, mode, root):
    module = importlib.import_module(table.replace("-", "_"))
    if mode == "selftest":
        selftest(table)
        with TemporaryDirectory(dir=root) as tmp:
            module.selftest(Path(tmp))
        return 0

    directory = Path(__file__).resolve().parents[1] / table
    oracle_path, pending_path = directory / "expected.tsv", directory / "pending.txt"
    recording = mode == "record"
    skipped = set()
    if recording and table == "getclass-receiver" and not shutil.which("javap"):
        for cell in module.cells():
            if cell.call == "reflection":
                print(f"skip {table} {cell.name} ({', '.join(cell.legs())}): no javap")
                skipped.update((cell.name, leg) for leg in cell.legs())

    expected = {} if recording else load(oracle_path, 2)
    pending = {} if recording else load(pending_path, 3)
    actual = {}
    for cell, leg, oracle, observed, row in module.recorded(root, recording, expected, skipped):
        key = cell, leg
        if key in actual:
            raise ValueError(f"duplicate leg {key}")
        actual[key] = tuple(s.replace(str(root), "$OUT") for s in observed)
        if not recording and key in pending and actual[key] == pending[key][1:] and pending[key][0] != row:
            raise ValueError(f"{cell} {leg}: expected pending row {row!r}, found {pending[key][0]!r}")
        if recording:
            expected[key] = tuple(s.replace(str(root), "$OUT") for s in oracle)
            if actual[key] != expected[key] and row:
                pending[key] = (row, *actual[key])

    if recording and skipped:
        # Preserve descriptors from the last recording on a JDK with javap.
        for path, columns, keep in ((oracle_path, 2, expected), (pending_path, 3, pending)):
            if path.exists():
                keep.update({k: v for k, v in load(path, columns).items() if k in skipped})
    errors, fixed = differences({k: v for k, v in expected.items() if k not in skipped},
                                {k: v for k, v in pending.items() if k not in skipped}, actual, mode == "update")
    for key, problem in errors:
        print(*key, problem, "scalac", expected.get(key), "pending", pending.get(key), "observed", actual.get(key), sep="\t")
    if not errors:
        if recording:
            save(oracle_path, ("scalac status", "output"), expected)
        if recording or (mode == "update" and fixed):
            for key in fixed:
                del pending[key]
            save(pending_path, ("row", "teq status", "output"), pending)
    checked_rows = Counter(v[0] for k, v in pending.items() if k not in skipped)
    for row, count in sorted(checked_rows.items()):
        print(f"{table}: {row}: {count}")
    print(f"{table}: {len(actual)} legs, {sum(checked_rows.values())} pending, {len(errors)} failed, {len(skipped)} skipped")
    return int(bool(errors))


if __name__ == "__main__":
    table, mode, directory = sys.argv[1:]
    try:
        sys.exit(judge(table, mode, Path(directory).resolve()))
    except (OSError, ValueError) as error:
        sys.exit(f"{table}: {error}")
