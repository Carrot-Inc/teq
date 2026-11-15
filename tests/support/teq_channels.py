#!/usr/bin/env python3
"""The two channels a stand-in teq writes a run's transcript on, and the records of the run, as teq's verbs write
them with `--diagnostics`, for the verbs' checks (tests/lila-test-verb.sh, tests/reference-test-verb.sh,
tests/mill-test-verb.sh, tests/gate-scripts.sh): teq_channels.py [--records <file>] < <script> writes each line on the
channel teq writes it on, and with --records the run's records to <file>, a JSON line each, in the script's order.

The script is the stand-in's notation, which names a configuration by its key so that a record can carry it; what
goes on the channels is the line as teq prints it, which names the project:

- a diagnostic, its header (`<path>:<line>:<col>: error|warning: <message>`) and every line after it up to the
  script's next own line: on stderr, recorded with its severity and the lines shown (src/task/daemon.rs `shown`);
  its configuration is the one whose count comes after it, else the one the next own line names, else the one the
  own line before it names (a build whose records stop before its outcome);
- `<key>: <n> errors`: a build's count, on stderr as `<project>: <n> errors`, and the build's outcome recorded
  (`outcome`); `<key>: checked in <t>` and `<key>: <n> classes, <m> written in <t>`: a build that passed, as the
  client words it, its outcome recorded; `<n> errors found`: a one-shot build's count, as it is, and its outcome of
  no configuration's recorded (src/source.rs `errors_found`);
- `<key>: <n> suites, <p> passed, <f> failed[, ..] in ..`: a configuration's summary, on stderr under its project's
  name, its counts recorded (`tests_ended`); `<key>: no suite to run`, `.. no suite depends on what changed`,
  `.. no test framework`, `.. the run failed: <why>`: its end with nothing run or failed, the same way;
- `teq: <key>: <kind> generator <n> of <key>: ..` and every line after it up to the next own line: a generator's
  failure, on stderr under the project's name, recorded whole (`failed`, kind `generator`, its message naming the
  key) and as the build's outcome;
- `teq: <key>: the build's thread panicked`, `.. the resident of <key> ended..`, `.. the daemon is ending..`: a crash
  of teq's, on stderr as teq says it, recorded (`failed`, kind `crash`), the first two as a build's outcome too;
- `teq: <key>: every suite, ..` and the overlays' note: what teq says of a build or a run that is no failure,
  recorded as a note (`told`); any other `teq: <key>: ..`: a failure, recorded (`failed`, kind `other`);
- `teq: compile left out <project>, ..` and the indented lines after it: recorded (`left-out`);
- `teq: the daemon ended without an answer..`: the client's own failure, recorded (kind `crash`, no configuration);
- any other `teq: ..` line and a panic's message: on stderr, no record; every other line on stdout, the run's
  stream: what the suites and their frameworks print and teq's report of each event (`[success] ...`, `[fail] ...`
  with its indented lines).

A line written `1> <text>` is <text> on stdout (a suite's print, or a line of an event's name or message), `2> <text>`
<text> on stderr, `+> <text>` <text> on stderr as a line of the diagnostic or the generator's failure before it (a
line of its message or output that reads as one of teq's), `n> <text>` <text> on stderr as what a passing generator
printed (`<key>: <line>`, shown under the project's name), a record of its own that the `+> ` lines after it
continue."""
import json, re, sys

header = re.compile(r"^([^ :][^:]*?(?::[0-9]+)*): (error|warning): (.*)$")
generator = re.compile(r"^teq: ([^ :]+/[^ :]+): ([a-z]+ generator [0-9]+) of ([^ :]+/[^ :]+): (.*)$")
count = re.compile(r"^(\S+/\S+): ([0-9]+) errors?$")
passed = re.compile(r"^(\S+/\S+): (checked|[0-9]+ classes, ([0-9]+) written) in ([0-9]+ ms|[0-9]+\.[0-9] s)(, the first build)?$")
found = re.compile(r"^([0-9]+) errors? found$")
summary = re.compile(r"^(\S+/test): ([0-9]+) suites?, ([0-9]+) passed, ([0-9]+) failed((?:, [0-9]+ (?:ignored|skipped|pending|canceled))*)( in .*)?$")
idle = re.compile(r"^(\S+/test): (no suite to run|no suite depends on what changed|no test framework)$")
run_failed = re.compile(r"^(\S+/test): the run failed: (.*)$")
crash = re.compile(r"^teq: ([^ :]+/[^ :]+): (the build's thread panicked|the resident of (\S+) ended|the daemon is ending)(.*)$")
teq_line = re.compile(r"^teq: ([^ :]+/[^ :]+): (.*)$")
note = re.compile(r"^(every suite, .*|no memory mapping for the type store's overlays; one worker types the build)$")
left_out = re.compile(r"^teq: compile left out ([^ ,]+),.*$")
no_answer = re.compile(r"^teq: the daemon ended without an answer.*$")
named = re.compile(r"^(?:teq: )?(\S+/[^ :]+):")
keyed = re.compile(r"^(\S+?)/([^ :]+): (.*)$")


def project(key):
    return key.split("/")[0]


def name_of(key):
    """A configuration as a generator's line names it: its project, the configuration where it is not `compile`."""
    p, c = key.split("/", 1)
    return p if c == "compile" else f"{p} ({c})"


records = sys.argv[sys.argv.index("--records") + 1] if "--records" in sys.argv else None
# The records in the script's order; a diagnostic's and a failure's lines grow as the script continues them.
blocks = []
# The script's own lines after which a diagnostic's configuration is looked for.
owns = []
current = None
block = False
left = None
for n, line in enumerate(sys.stdin):
    if line.startswith("1> "):
        sys.stdout.write(line[3:])
        block = False
        current = None
        left = None
        continue
    if line.startswith("2> "):
        sys.stderr.write(line[3:])
        continue
    if line.startswith("+> "):
        sys.stderr.write(line[3:])
        if current is not None:
            current["lines"].append(line[3:].rstrip("\n"))
            if "message" in current:
                current["message"].append(line[3:].rstrip("\n"))
        continue
    if line.startswith("n> "):
        text = line[3:].rstrip("\n")
        k = keyed.match(text)
        shown = f"{name_of(k.group(1) + '/' + k.group(2))}: {k.group(3)}" if k else text
        sys.stderr.write(shown + "\n")
        current = {"at": n, "severity": "note", "key": named.match(text).group(1) if named.match(text) else "", "lines": [shown]}
        blocks.append(current)
        block = False
        left = None
        continue
    text = line.rstrip("\n")
    if left is not None and text.startswith("  "):
        # A project left out: its reasons, indented under its line.
        left["lines"].append(text)
        sys.stderr.write(line)
        continue
    left = None
    own = header.match(text) or text.startswith("teq: ") or text.startswith("thread '") or count.match(text) or passed.match(text) or found.match(text) or summary.match(text) or idle.match(text) or run_failed.match(text)
    if own:
        # A diagnostic and a generator's failure run on to the script's next own line; any other own line is one.
        block = bool(header.match(text) or generator.match(text))
        current = None
        shown = text
        if header.match(text):
            current = {"at": n, "severity": header.match(text).group(2), "lines": [text]}
            blocks.append(current)
        elif generator.match(text):
            key, what, of, why = generator.match(text).groups()
            told = what if of == project(key) + "/compile" else f"{what} ({of.split('/', 1)[1]})" if project(of) == project(key) else f"{what} of {name_of(of)}"
            shown = f"teq: {project(key)}: {told}: {why}"
            current = {"at": n, "severity": "failure", "kind": "generator", "key": key, "lines": [shown], "message": [f"{what} of {of}: {why}"], "outcome": True}
            blocks.append(current)
            owns.append((n, text))
        elif crash.match(text):
            key, what, resident, rest = crash.match(text).groups()
            said = f"the compiler for {project(resident)} ended{rest}" if resident else what + rest
            shown = f"teq: {project(key)}: {said}"
            blocks.append({"at": n, "severity": "failure", "kind": "crash", "key": key, "lines": [shown], "message": [said], "outcome": not what.startswith("the daemon")})
            owns.append((n, text))
        elif teq_line.match(text):
            key, said = teq_line.match(text).groups()
            shown = f"teq: {project(key)}: {said}"
            if note.match(said):
                blocks.append({"at": n, "severity": "note", "key": key, "lines": [shown], "message": [said]})
            else:
                blocks.append({"at": n, "severity": "failure", "kind": "other", "key": key, "lines": [shown], "message": [said]})
            owns.append((n, text))
        elif left_out.match(text):
            left = {"at": n, "severity": "left-out", "project": left_out.match(text).group(1), "lines": [text]}
            blocks.append(left)
            owns.append((n, text))
        elif no_answer.match(text):
            blocks.append({"at": n, "severity": "failure", "kind": "crash", "key": None, "lines": [text], "message": [text[len("teq: "):]]})
            owns.append((n, text))
        elif found.match(text):
            blocks.append({"at": n, "severity": "outcome", "key": None, "errors": int(found.match(text).group(1))})
            owns.append((n, text))
        elif count.match(text):
            key, errors = count.match(text).groups()
            shown = f"{project(key)}: {errors} error{'' if errors == '1' else 's'}"
            blocks.append({"at": n, "severity": "outcome", "key": key, "errors": int(errors)})
            owns.append((n, text))
        elif passed.match(text):
            key, what, written, took, _ = passed.match(text).groups()
            covers = "main and test" if key.endswith("/test") else "main"
            shown = f"{project(key)}: checked {covers} in {took}" if what == "checked" else f"{project(key)}: compiled {covers}, {written} class{'' if written == '1' else 'es'} written, in {took}"
            blocks.append({"at": n, "severity": "outcome", "key": key, "errors": 0})
            owns.append((n, text))
        elif summary.match(text):
            key, suites, ok, failed, others, _ = summary.match(text).groups()
            shown = project(key) + text[len(key):]
            counts = {"suites": int(suites), "passed": int(ok), "failed": int(failed)}
            counts.update({word: int(number) for number, word in re.findall(r", ([0-9]+) (ignored|skipped|pending|canceled)", others)})
            blocks.append({"at": n, "severity": "tests", "key": key, "fields": {k: v for k, v in counts.items() if v > 0 or k == "suites"}})
            owns.append((n, text))
        elif idle.match(text):
            key, why = idle.match(text).groups()
            shown = f"{project(key)}: {why}"
            blocks.append({"at": n, "severity": "tests", "key": key, "fields": {"idle": why}})
            owns.append((n, text))
        elif run_failed.match(text):
            key, why = run_failed.match(text).groups()
            shown = f"{project(key)}: the run failed: {why}"
            blocks.append({"at": n, "severity": "tests", "key": key, "fields": {"run_failed": why}})
            owns.append((n, text))
        else:
            owns.append((n, text))
        sys.stderr.write(shown + "\n")
    elif block and not text.startswith("["):
        if current is not None:
            current["lines"].append(text)
            if "message" in current:
                current["message"].append(text)
        sys.stderr.write(line)
    else:
        block = False
        current = None
        sys.stdout.write(line)
if records is not None:
    with open(records, "w") as out:
        for b in blocks:
            if b["severity"] in ("error", "warning"):
                after = [t for n, t in owns if n > b["at"]]
                counted = [count.match(t).group(1) for t in after if count.match(t)]
                names = [named.match(t).group(1) for t in after if named.match(t)]
                before = [named.match(t).group(1) for n, t in owns if n < b["at"] and named.match(t)]
                key = counted[0] if counted else names[0] if names else before[-1] if before else ""
                out.write(json.dumps({"key": key, "severity": b["severity"], "shown": "\n".join(b["lines"]) + "\n"}) + "\n")
            elif b["severity"] == "outcome" and b["key"] is None:
                out.write(json.dumps({"severity": "outcome", "ok": False, "errors": b["errors"], "failed": None, "why": None, "ms": None}) + "\n")
            elif b["severity"] == "outcome":
                out.write(json.dumps({"key": b["key"], "severity": "outcome", "projects": [project(b["key"])], "ok": b["errors"] == 0, "errors": b["errors"], "failed": None, "why": None}) + "\n")
            elif b["severity"] == "tests":
                out.write(json.dumps({"key": b["key"], "severity": "tests", **b["fields"]}) + "\n")
            elif b["severity"] == "left-out":
                out.write(json.dumps({"severity": "left-out", "project": b["project"], "shown": "\n".join(b["lines"]) + "\n"}) + "\n")
            elif b["severity"] == "note":
                record = {"key": b["key"], "severity": "note", "shown": "\n".join(b["lines"]) + "\n"}
                if "message" in b:
                    record["message"] = "\n".join(b["message"])
                out.write(json.dumps(record) + "\n")
            else:
                message = "\n".join(b["message"])
                record = {"severity": "failure", "kind": b["kind"], "message": message, "shown": "\n".join(b["lines"]) + "\n"}
                if b["key"] is not None:
                    record = {"key": b["key"], **record}
                out.write(json.dumps(record) + "\n")
                if b.get("outcome"):
                    out.write(json.dumps({"key": b["key"], "severity": "outcome", "projects": [project(b["key"])], "ok": False, "errors": 0, "failed": b["kind"], "why": message}) + "\n")
