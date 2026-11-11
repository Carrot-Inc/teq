# Compare accessor used names against recorded scalac 3.8.4 -Ydump-sbt-inc output.
import json, re, sys
ops = {"tilde": "~", "eq": "=", "less": "<", "greater": ">", "bang": "!", "hash": "#", "percent": "%", "up": "^",
       "amp": "&", "bar": "|", "times": "*", "div": "/", "plus": "+", "minus": "-", "colon": ":", "bslash": "\\",
       "qmark": "?", "at": "@"}
def decode(n):
    n = re.sub(r"\$u([0-9A-F]{4})", lambda m: chr(int(m.group(1), 16)), n)
    return re.sub(r"\$(" + "|".join(sorted(ops, key=len, reverse=True)) + r")", lambda m: ops[m.group(1)], n)
def recorded_names(dump, pkg):
    result = {}
    for line in open(dump).read().split("Dependencies:")[0].splitlines():
        m = re.match(r"^(?:module )?class (\S+): (.*)$", line)
        if m:
            names = re.findall(r"(\S+) in \[", m.group(2))
            result[pkg + "." + m.group(1).removesuffix("$")] = sorted(decode(n) for n in names if "inline$" in n)
    return result

def used_names(answer):
    return {c: sorted(n for n in d["names"] if "inline$" in n)
            for f in answer["api"]["files"] for c, d in f["deps"].items()}

if sys.argv[1] == "--naming":
    answer = json.load(open(sys.argv[2]))
    expected = recorded_names(sys.argv[3], "namesprobe")
    # The review's local-class regression is in its pickled name. The unused local inline body's
    # dependency traversal is independent; hold its emitted accessor to the recorded spelling.
    pickled = set()
    for file in sys.argv[4:]:
        for line in open(file):
            m = re.match(r"\s*\d+ inlineaccessor (inline\$.*)$", line)
            if m:
                underlying = m.group(1).removeprefix("inline$")
                qualifier, sep, member = underlying.rpartition("$$")
                pickled.add(qualifier + sep + "inline$" + member)
    all_expected = {n for names in expected.values() for n in names}
    user = "namesprobe.User$anonClass"
    ok = answer.get("ok") and pickled == all_expected and used_names(answer).get(user) == expected[user]
    if not ok:
        print("accessor naming mismatch:", pickled, all_expected, used_names(answer), file=sys.stderr)
    sys.exit(0 if ok else 1)

ok = True
for answer_file, dump, pkg in zip(sys.argv[1::3], sys.argv[2::3], sys.argv[3::3]):
    answer = json.load(open(answer_file))
    teq, scalac = used_names(answer), recorded_names(dump, pkg)
    matches = answer.get("ok") and scalac and all(teq.get(c, []) == ns for c, ns in scalac.items()) \
        and all(c in scalac for c, ns in teq.items() if ns)
    if not matches:
        print("accessor used-name mismatch:", answer_file, teq, scalac, file=sys.stderr)
    ok = ok and matches
sys.exit(0 if ok else 1)
