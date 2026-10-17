#!/usr/bin/env python3
"""The two channels a stand-in teq writes a test run's transcript on, and the records of its builds' diagnostics, as
`teq test --diagnostics` writes them, for the verbs' checks (tests/lila-test-verb.sh, tests/reference-test-verb.sh,
tests/mill-test-verb.sh, tests/gate-scripts.sh): teq_channels.py [--records <file>] < <transcript> writes each line of
the transcript on the channel teq writes it on, and with --records each diagnostic's record to <file>, a JSON line
each (src/task/daemon.rs `shown`): its configuration, its severity and the lines shown for it, and each generator's
failure (`failed`, severity `failure`: its line and every line after it up to teq's next own line) and what each
passing generator printed (`noted`, severity `note`, one record a generator) the same way. On stderr teq's own
lines (src/task/daemon.rs `built`, `test_configuration`): a diagnostic, its header
(`<path>:<line>:<col>: error|warning: <message>`) and every line after it up to teq's next own line (`render`'s
message, source and caret), a line of teq's (`teq: ...`; a generator's failure with the lines after it), a build's
count (`<key>: <n> errors`, a one-shot build's `<n> errors found`), a configuration's summary or end with nothing run,
a panic's message; on stdout every other line, the run's stream: what the suites and their frameworks print and teq's
report of each event (`[success] ...`, `[fail] ...` with its indented lines). A diagnostic's configuration is the one
whose count comes after it, else the one the next line of teq's own names. A line written `1> <text>` is <text> on
stdout (a suite's print, or a line of an event's name or message), `2> <text>` <text> on stderr, `+> <text>` <text>
on stderr as a line of the diagnostic or the generator's failure before it (a line of its message or output that reads
as one of teq's), `n> <text>` <text> on stderr as what a passing generator printed, a record of its own that the
`+> ` lines after it continue."""
import json, re, sys

header = re.compile(r"^([^ :][^:]*?(?::[0-9]+)*): (error|warning): (.*)$")
generator = re.compile(r"^teq: [^ :]+/[^ :]+: [a-z]+ generator [0-9]+ of ")
count = re.compile(r"^(\S+/\S+): [0-9]+ errors?$")
# teq's own lines that are no diagnostic and no line of teq's: a count, a summary, an end with nothing run.
own = re.compile(r"^(\S+/\S+: [0-9]+ errors?|[0-9]+ errors? found|\S+/test: [0-9]+ suites?, .*|\S+/test: (no suite to run|no suite depends on what changed|no test framework|the run failed: .*))$")
named = re.compile(r"^(?:teq: )?(\S+/[^ :]+):")

records = sys.argv[sys.argv.index("--records") + 1] if "--records" in sys.argv else None
blocks = []
owns = []
current = None
block = False
for n, line in enumerate(sys.stdin):
    if line.startswith("1> "):
        sys.stdout.write(line[3:])
        block = False
        current = None
        continue
    if line.startswith("2> "):
        sys.stderr.write(line[3:])
        continue
    if line.startswith("+> "):
        sys.stderr.write(line[3:])
        if current is not None:
            current["lines"].append(line[3:].rstrip("\n"))
        continue
    if line.startswith("n> "):
        sys.stderr.write(line[3:])
        text = line[3:].rstrip("\n")
        current = {"at": n, "severity": "note", "key": named.match(text).group(1) if named.match(text) else "", "lines": [text]}
        blocks.append(current)
        block = False
        continue
    text = line.rstrip("\n")
    if header.match(text) or text.startswith("teq: ") or text.startswith("thread '") or own.match(text):
        # A diagnostic and a generator's failure run on to teq's next own line; any other line of teq's is one.
        block = bool(header.match(text) or generator.match(text))
        current = None
        if header.match(text):
            current = {"at": n, "severity": header.match(text).group(2), "lines": [text]}
            blocks.append(current)
        elif generator.match(text):
            # A generator's failure, recorded whole (src/task/daemon.rs `failed`) in its own configuration's name.
            current = {"at": n, "severity": "failure", "key": named.match(text).group(1), "lines": [text]}
            blocks.append(current)
            owns.append((n, text))
        else:
            owns.append((n, text))
        sys.stderr.write(line)
    elif block and not text.startswith("["):
        if current is not None:
            current["lines"].append(text)
        sys.stderr.write(line)
    else:
        block = False
        current = None
        sys.stdout.write(line)
if records is not None:
    with open(records, "w") as out:
        for b in blocks:
            after = [t for n, t in owns if n > b["at"]]
            counted = [count.match(t).group(1) for t in after if count.match(t)]
            names = [named.match(t).group(1) for t in after if named.match(t)]
            key = b["key"] if "key" in b else counted[0] if counted else names[0] if names else ""
            out.write(json.dumps({"key": key, "severity": b["severity"], "shown": "\n".join(b["lines"]) + "\n"}) + "\n")
