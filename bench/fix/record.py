# bench/fix/record.py <record> <workers|->: whether a `teq fix` run's execution record (TEQ_FIX_RECORD, docs/FIX.md) is
# the parallel run bench/fix/run.sh measures: the shipped rules (`eligible=1`) over the workers asked (`-`: the record's
# own count); every round's documents dispatched and done once each, with their slots' generations, by workers of the
# run's; no round serial where it has documents for several workers, no barrier waited for in vain, no worker evaluating
# two documents at once. A round with four documents or more a worker it could take is the parallel-execution witness
# (fix-speed's ruling 2, a corpus large enough to overlap): its overlap, read from the recorded evaluation intervals
# (`start_us`, `finish_us`), not from the workers' ids, which a serialized schedule may spread, reaches at least two at
# once and half those workers (the toy corpus's 1,000 documents at sixteen workers peak at ten). A smaller round (jzon's
# two documents, which worker 0 may finish before worker 1 has started) is a cost cell: complete, its utilization
# reported, no overlap asked of it. Prints a `utilization` line a round, then each problem; exits 1 on any problem.
import sys

def fields(line):
    return dict(kv.split("=", 1) for kv in line.split()[1:] if "=" in kv)

path, asked = sys.argv[1], sys.argv[2]
lines = open(path).read().split("\n")
problems = []
run = next((fields(l) for l in lines if l.startswith("run ")), None)
if run is None:
    problems.append("no run line")
    workers = 1
else:
    workers = int(run.get("workers", "0"))
    if run.get("eligible") != "1":
        problems.append(f"the run is not the shipped rules' (eligible={run.get('eligible')})")
    if asked != "-" and workers != int(asked):
        problems.append(f"the run took {workers} workers, {asked} were asked")
if any(l.startswith("barrier-timeout") for l in lines):
    problems.append("a worker waited for the barrier in vain")
rounds = 0
utilization = []
i = 0
while i < len(lines):
    if not lines[i].startswith("round "):
        i += 1
        continue
    rounds += 1
    head = fields(lines[i])
    n, first = int(head.get("docs", "0")), int(head.get("first_gen", "0"))
    where = f"round {head.get('job')}/{head.get('round')}"
    body = []
    i += 1
    while i < len(lines) and not lines[i].startswith("round-end"):
        body.append(lines[i])
        i += 1
    if i == len(lines):
        problems.append(f"{where}: no round-end")
    want = min(workers, n)
    for kind in ("dispatch", "done"):
        got = [fields(l) for l in body if l.startswith(kind + " ")]
        slots = sorted(int(f.get("slot", "-1")) for f in got)
        if slots != list(range(n)):
            problems.append(f"{where}: {kind} slots {slots[:8]}{'...' if len(slots) > 8 else ''}, expected 0..{n - 1} once each")
        for f in got:
            if int(f.get("gen", "-1")) != first + int(f.get("slot", "-1")):
                problems.append(f"{where}: slot {f.get('slot')} {kind} with generation {f.get('gen')}")
            if not 0 <= int(f.get("worker", "-1")) < max(workers, 1):
                problems.append(f"{where}: slot {f.get('slot')} {kind} by worker {f.get('worker')} of {workers}")
    done = [fields(l) for l in body if l.startswith("done ")]
    intervals = [(int(f.get("start_us", "0")), int(f.get("finish_us", "0")), f.get("worker")) for f in done]
    # The most evaluations at once: an end before a start at one instant, so that touching intervals do not overlap.
    events = sorted([(a, 1) for a, _, _ in intervals] + [(b, -1) for _, b, _ in intervals])
    running = peak = 0
    for _, d in events:
        running += d
        peak = max(peak, running)
    by_worker = {}
    for a, b, w in intervals:
        by_worker.setdefault(w, []).append((a, b))
    for w, spans in by_worker.items():
        spans.sort()
        if any(x[1] > y[0] for x, y in zip(spans, spans[1:])):
            problems.append(f"{where}: worker {w} evaluated two documents at once")
    witness = want > 1 and n >= 4 * want
    utilization.append(f"utilization {head.get('job')}/{head.get('round')} docs={n} workers={len(by_worker)} peak={peak} of={want} witness={'yes' if witness else 'no'}")
    if want > 1 and any(l.startswith("serial") for l in body):
        problems.append(f"{where}: serial ({next(l for l in body if l.startswith('serial'))})")
    if witness:
        if peak < 2:
            problems.append(f"{where}: its {n} documents evaluated one at a time ({len(by_worker)} worker ids, {want} workers)")
        elif 2 * peak < want:
            problems.append(f"{where}: its {n} documents at most {peak} at once, of {want} workers")
if rounds == 0:
    problems.append("no round")
elif not any(l.startswith("done ") for l in lines):
    problems.append("no document evaluated")
for u in utilization:
    print(u)
for p in problems[:20]:
    print(p)
sys.exit(1 if problems else 0)
