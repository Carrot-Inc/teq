#!/usr/bin/env python3
"""fold_analysis.py <answers.jsonl> <fresh answer>: whether the analysis a session's answers have
accumulated, folded as sbt-teq folds them (`TeqCompile.Reported`: a full answer's analysis
replaces the record, an incremental one's files and removals update it), is the analysis of a
fresh session's first answer. Prints `same` or the files that differ."""
import json, sys
record = None
for line in open(sys.argv[1]):
    a = json.loads(line)
    files = {f["file"]: f for f in a.get("analysis", [])} if "analysis" in a else None
    if not a.get("incremental"):
        record = files
    elif record is not None:
        for r in a.get("removed", []):
            record.pop(r, None)
        record.update(files or {})
fresh = json.loads(open(sys.argv[2]).read())
want = {f["file"]: f for f in fresh.get("analysis", [])}
if record == want:
    print("same")
else:
    keys = sorted(set(record or {}) | set(want))
    print("differs: " + ", ".join(k.split("/")[-1] for k in keys if (record or {}).get(k) != want.get(k)))
