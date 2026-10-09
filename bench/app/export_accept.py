#!/usr/bin/env python3
"""The export's differences from master's build against an enumerated manifest (bench/app/export-accept.sh).

export_accept.py <manifest> <master's build> <landing's build> [<repository root>]
export_accept.py --draft <master's build> <landing's build>   a manifest of the builds' differences, evidence to fill in
export_accept.py --self-test

A manifest lists every difference a landing may make to the export, each bound to the exact bytes of both sides and
tied to the correction that makes it:

    # a comment
    file <path in the export>
    evidence <path in the repository> <words>
    before <sha256 of master's file>
    after <sha256 of the landing's file>
    @@ -12,7 +12,7 @@
     <a line both have>
    -<master's line>
    +<the landing's line>

`only-master <path>` takes a `before` line and `only-landing <path>` an `after` line and the added file's lines, each
`+`, with their evidence. Every entry needs an `evidence` line whose first word is a file under tests/ or docs/ of the
repository: the case or the probe that shows scalac's answer. The hashes bind an entry to the files it approves,
byte for byte (an edit in another place, a changed line ending); the hunks, a unified diff with three lines of
context, are what a reader reads, and they have to be the builds' own. The check passes when the builds'
differences are exactly the manifest's: every differing file listed with both hashes and its hunks, and no listed
difference missing.
"""
import difflib
import hashlib
import os
import sys


def parse(text):
    """The manifest's entries by path, and what is malformed in it."""
    entries, problems = {}, []
    current = None
    for number, line in enumerate(text.splitlines(), 1):
        if current is None and (not line.strip() or line.startswith("#")):
            continue
        word, _, rest = line.partition(" ")
        if word in ("file", "only-master", "only-landing"):
            if not rest:
                problems.append(f"line {number}: {word} without a path")
                current = None
                continue
            if rest in entries:
                problems.append(f"line {number}: {rest} listed twice")
            current = entries[rest] = {"kind": word, "evidence": [], "before": None, "after": None, "hunks": [], "line": number}
        elif current is None:
            problems.append(f"line {number}: outside an entry: {line[:80]}")
        elif word == "evidence" and not current["hunks"]:
            current["evidence"].append(rest)
        elif word in ("before", "after") and not current["hunks"]:
            current[word] = rest.strip()
        elif line[:1] in ("@", " ", "-", "+") or line == "":
            current["hunks"].append(line)
        else:
            problems.append(f"line {number}: neither evidence, a hash nor a line of a hunk: {line[:80]}")
    for e in entries.values():
        while e["hunks"] and e["hunks"][-1] == "":
            e["hunks"].pop()
    return entries, problems


def files(root):
    out = set()
    for cur, _, names in os.walk(root):
        for n in names:
            out.add(os.path.relpath(os.path.join(cur, n), root))
    return out


def digest(data):
    return hashlib.sha256(data).hexdigest()


def text_lines(data):
    return data.decode("utf-8", "replace").splitlines()


def hunks(a, b):
    """The unified diff of two files' bytes with three lines of context, without its header."""
    out = [line for line in difflib.unified_diff(text_lines(a), text_lines(b), n=3, lineterm="")][2:]
    return out or ["@@ the files differ in bytes on no line: an ending or an encoding @@"]


def expected_hunks(kind, old, new):
    if kind == "file":
        return hunks(old, new)
    if kind == "only-landing":
        return ["+" + line for line in text_lines(new)]
    return []


def kind_of(old, new):
    return "file" if old is not None and new is not None else "only-master" if new is None else "only-landing"


def read(root, path, present):
    return open(os.path.join(root, path), "rb").read() if path in present else None


def check(manifest_text, master, landing, repo):
    """What does not agree between the builds' differences and the manifest: an empty list when they agree."""
    entries, problems = parse(manifest_text)
    for path, e in entries.items():
        good = [ev for ev in e["evidence"] if ev.split() and ev.split()[0].split("/")[0] in ("tests", "docs") and os.path.exists(os.path.join(repo, ev.split()[0]))]
        if not good:
            problems.append(f"{path}: no evidence naming a case or a document of the repository (line {e['line']})")
    a, b = files(master), files(landing)
    seen = set()
    for path in sorted(a | b):
        old, new = read(master, path, a), read(landing, path, b)
        if old == new:
            continue
        seen.add(path)
        kind = kind_of(old, new)
        expected = expected_hunks(kind, old, new)
        entry = entries.get(path)
        if not entry:
            problems.append(f"{path}: differs ({kind}), not listed\n" + "\n".join("    " + line[:160] for line in expected[:40]))
            continue
        if entry["kind"] != kind:
            problems.append(f"{path}: differs as {kind}, listed as {entry['kind']}")
            continue
        if old is not None and entry["before"] != digest(old):
            problems.append(f"{path}: master's file is not the one approved (before {entry['before']}, found {digest(old)})")
        if new is not None and entry["after"] != digest(new):
            problems.append(f"{path}: the landing's file is not the one approved (after {entry['after']}, found {digest(new)})")
        if (old is None and entry["before"] is not None) or (new is None and entry["after"] is not None):
            problems.append(f"{path}: a hash of a file one build does not have")
        if entry["hunks"] != expected:
            problems.append(f"{path}: the entry's hunks are not the builds' own")
    for path in sorted(set(entries) - seen):
        problems.append(f"{path}: listed ({entries[path]['kind']}), no such difference")
    return problems, len(seen)


def draft(master, landing):
    """A manifest of the builds' differences, each entry's evidence to be written."""
    out = []
    a, b = files(master), files(landing)
    for path in sorted(a | b):
        old, new = read(master, path, a), read(landing, path, b)
        if old == new:
            continue
        kind = kind_of(old, new)
        out.append(f"{kind} {path}")
        out.append("evidence <a case or a document under tests/ or docs/> <why>")
        if old is not None:
            out.append(f"before {digest(old)}")
        if new is not None:
            out.append(f"after {digest(new)}")
        out.extend(expected_hunks(kind, old, new))
        out.append("")
    return "\n".join(out)


def self_test():
    import tempfile
    failures = []
    count = [0]

    def lay_out(d, fs):
        os.makedirs(d, exist_ok=True)
        for path, data in fs.items():
            p = os.path.join(d, path)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            open(p, "wb").write(data.encode())

    def approved(master, landing):
        return draft(master, landing).replace("evidence <a case or a document under tests/ or docs/> <why>", "evidence tests/cases/fix.scala scalac's answer")

    def case(name, master_files, landing_files, manifest, want_pass):
        count[0] += 1
        with tempfile.TemporaryDirectory() as d:
            repo = os.path.join(d, "repo")
            lay_out(os.path.join(repo, "tests", "cases"), {"fix.scala": ""})
            master, landing = os.path.join(d, "master"), os.path.join(d, "landing")
            lay_out(master, master_files)
            lay_out(landing, landing_files)
            text = manifest(master, landing) if callable(manifest) else manifest
            problems, _ = check(text, master, landing, repo)
            if (not problems) != want_pass:
                failures.append(f"{name}: expected {'a pass' if want_pass else 'a failure'}, got {problems or 'a pass'}")

    base = {"main.js": "function a() {\n  return 0;\n}\nfunction b() {\n  return 0;\n}\n", "lib/x.js": "x\n"}
    fixed_a = {**base, "main.js": "function a() {\n  return 1;\n}\nfunction b() {\n  return 0;\n}\n"}
    fixed_b = {**base, "main.js": "function a() {\n  return 0;\n}\nfunction b() {\n  return 1;\n}\n"}
    with tempfile.TemporaryDirectory() as d:
        lay_out(os.path.join(d, "m"), base)
        lay_out(os.path.join(d, "l"), fixed_a)
        entry_a = approved(os.path.join(d, "m"), os.path.join(d, "l"))
    case("identical, empty manifest", base, dict(base), "", True)
    case("identical, a listed difference", base, dict(base), entry_a, False)
    case("the approved edit", base, fixed_a, entry_a, True)
    case("the same edit in another function", base, fixed_b, entry_a, False)
    case("the approved edit and line endings changed", base, {**fixed_a, "main.js": fixed_a["main.js"].replace("\n", "\r\n")}, entry_a, False)
    case("an unlisted difference", base, fixed_a, "", False)
    case("no evidence", base, fixed_a, entry_a.replace("evidence tests/cases/fix.scala scalac's answer\n", ""), False)
    case("evidence that does not exist", base, fixed_a, entry_a.replace("tests/cases/fix.scala", "tests/cases/none.scala"), False)
    case("evidence outside tests and docs", base, fixed_a, entry_a.replace("tests/cases/fix.scala", "bench/x"), False)
    case("hunks that are not the builds'", base, fixed_a, entry_a.replace("+  return 1;", "+  return 2;"), False)
    case("a second differing file", base, {**fixed_a, "lib/x.js": "y\n"}, entry_a, False)
    case("a file only in the landing's, approved", base, {**base, "new.js": "n\n"}, approved, True)
    case("a file only in the landing's, other content", base, {**base, "new.js": "m\n"}, lambda m, l: approved(m, l).replace("+m", "+n"), False)
    case("a file only in the landing's, unlisted", base, {**base, "new.js": "n\n"}, "", False)
    case("a file only in master's, approved", base, {"main.js": base["main.js"]}, approved, True)
    case("a malformed line", base, fixed_a, entry_a.replace("before ", "junk\nbefore "), False)
    for f in failures:
        print("FAIL " + f)
    print(f"export-accept self-test: {count[0] - len(failures)} of {count[0]} cases as expected")
    return 1 if failures else 0


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    if len(sys.argv) == 4 and sys.argv[1] == "--draft":
        print(draft(sys.argv[2], sys.argv[3]))
        return 0
    if len(sys.argv) not in (4, 5):
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    manifest, master, landing = sys.argv[1:4]
    repo = sys.argv[4] if len(sys.argv) == 5 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
    text = open(manifest).read()
    problems, differing = check(text, master, landing, repo)
    for p in problems:
        print(p)
    listed = len(parse(text)[0])
    if problems:
        print(f"export-accept: the export's differences are not the manifest's ({differing} files differ, {listed} listed)")
        return 1
    print(f"export-accept: the export's differences are exactly the manifest's ({differing} files, each bound to its bytes and its evidence)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
