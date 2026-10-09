#!/usr/bin/env python3
"""bench/ptyper-waits.py <trace> [--json]: who blocked whom in a parallel type phase.

Reads the bounded trace `TEQ_WORKERS_TRACE=<file> teq compiler check --time --threads N ...` writes
(src/measure.rs; one row per event: the thread's number, its worker, what, detail, arg, and the
event's interval in ns) and attributes every wait to what the thread it waited for was doing
meanwhile:

- a wait for the loader's lock, and a wait for a record staged under it (its publication at the
  lock's release), to the categories of the holds that overlap the wait on the other threads
  (the lock has one holder at a time; the stretch of a hold is charged to the innermost category
  under way, src/measure.rs `Hold`), the part of the wait no other thread held the lock for being
  the hand-over's;
- a wait on a completion cell to its claimant (the row's `arg`, a thread's number): the
  claimant's holds of the loader's lock by category, its own waits by kind outside those holds (a
  wait inside a hold is the hold's time already), and the rest of the interval, when the claimant
  was neither holding the lock nor waiting as far as the trace says, as unattributed: the trace
  records no scheduling, so that time may be computing or the claimant descheduled;
- the waits for the type store's, the interner's and the shared maps' locks are counted, their
  holders' intervals not traced (an insert's hold is a hash probe and a push).

It also says how busy the loader's lock was while the workers ran: the union of its holds over
the body phase's span (the first worker's start to the last one's end).

And the lock's hand-overs, from the lock's own events: each release
with the threads waiting then, followed to the next acquisition by any thread, the gap between
them classed by how the next holder took it: woken from a park (the wake's latency from the
release, then the time to the acquisition), on its spin, by the releaser itself or by another
thread arriving (no wait); the releases that found no waiter; the holds' count and lengths; the
parks whose waiter woke and parked again; each wait's latency, from its start to its
acquisition, over every wait and over those that parked, against the longest hold (the fairness
of the spin: a parked waiter overtaken by spinners waits longer than any hold); and, on Linux, the
workers' time runnable and not running (the scheduler's run delay).
"""
import bisect
import json
import sys
from collections import defaultdict

WAITS = ["the loader's lock", "the type store's lock", "the interner's lock", "a shared map's lock", "a signature's cell",
         "a body's cell", "a class completion's cell", "an alias's cell", "a class check's cell", "a wait refused (a cycle)",
         "a staged record's publication"]
HOLDS = ["taking it", "a std class checked", "a jar class converted", "a jar class checked", "a class completed (std or jar)",
         "an alias completed (std or jar)", "a signature completed (std or jar)", "a std or library body typed",
         "a name asked of the std's files", "a std file entered otherwise", "a name asked of a jar's package",
         "a jar's class entered otherwise", "a program result published", "releasing it", "other"]
EV_WAIT, EV_HELD, EV_HOLD, EV_ITEM, EV_WORK, EV_PARK, EV_TAKEN, EV_RELEASE, EV_SCHED = 1, 2, 3, 4, 5, 6, 7, 8, 9


def read(path):
    global WAITS, HOLDS
    by_thread = defaultdict(lambda: defaultdict(list))
    worker_of = {}
    with open(path) as f:
        for line in f:
            if line.startswith("# waits: "):
                WAITS = line[len("# waits: "):].rstrip("\n").split(";")
                continue
            if line.startswith("# holds: "):
                HOLDS = line[len("# holds: "):].rstrip("\n").split(";")
                continue
            if line.startswith("thread "):
                continue
            thread, worker, what, detail, arg, t0, t1 = map(int, line.split())
            worker_of[thread] = worker
            by_thread[thread][what].append((t0, t1, detail, arg))
    for t in by_thread.values():
        for evs in t.values():
            evs.sort()
    return by_thread, worker_of


class Intervals:
    """A thread's intervals of one kind, sorted and not overlapping, for overlap sums."""

    def __init__(self, evs):
        self.evs = evs
        self.starts = [e[0] for e in evs]

    def overlaps(self, a, b):
        i = max(bisect.bisect_right(self.starts, a) - 1, 0)
        while i < len(self.evs) and self.evs[i][0] < b:
            t0, t1, detail, arg = self.evs[i]
            lo, hi = max(a, t0), min(b, t1)
            if hi > lo:
                yield lo, hi, detail, arg
            i += 1


def union_length(intervals, a, b):
    total, end = 0, a
    for t0, t1 in sorted(intervals):
        t0, t1 = max(t0, end), min(t1, b)
        if t1 > t0:
            total += t1 - t0
            end = t1
    return total


def analyse(path):
    by_thread, worker_of = read(path)
    holds = {t: Intervals(ev.get(EV_HOLD, [])) for t, ev in by_thread.items()}
    waits_of = {t: Intervals(ev.get(EV_WAIT, [])) for t, ev in by_thread.items()}
    work = [e for ev in by_thread.values() for e in ev.get(EV_WORK, [])]
    span = (min(e[0] for e in work), max(e[1] for e in work)) if work else (0, 0)
    blocked = defaultdict(float)
    count = defaultdict(int)
    total = defaultdict(float)
    for t, ev in by_thread.items():
        for t0, t1, kind, arg in ev.get(EV_WAIT, []):
            name = WAITS[kind]
            count[name] += 1
            total[name] += t1 - t0
            # The kinds' order is `measure::Wait`'s: the loader's lock 0, a cell 4 to 8, a
            # staged record's publication 10; the trace's header names them.
            if kind in (0, 10):
                # The other threads' stretches over the wait, cut into disjoint pieces: a moment
                # two stretches overlap (a holder's release timed through its unlock while the
                # next holder has the lock) goes to the one that began first.
                pieces = sorted((lo, hi, cat) for u, h in holds.items() if u != t for lo, hi, cat, _ in h.overlaps(t0, t1))
                covered, cursor = 0, t0
                for lo, hi, cat in pieces:
                    lo = max(lo, cursor)
                    if hi > lo:
                        blocked[(name, "held for " + HOLDS[cat])] += hi - lo
                        covered += hi - lo
                        cursor = hi
                blocked[(name, "no other thread held it (the hand-over)")] += max(t1 - t0 - covered, 0)
            elif 4 <= kind <= 8 and arg in by_thread:
                covered = 0
                for lo, hi, cat, _ in holds[arg].overlaps(t0, t1):
                    blocked[(name, "its claimant held the loader's lock for " + HOLDS[cat])] += hi - lo
                    covered += hi - lo
                for lo, hi, k, _ in waits_of[arg].overlaps(t0, t1):
                    outside = (hi - lo) - sum(h - l for l, h, _, _ in holds[arg].overlaps(lo, hi))
                    blocked[(name, "its claimant waited for " + WAITS[k])] += outside
                    covered += outside
                blocked[(name, "its claimant's time outside its recorded holds and waits")] += max(t1 - t0 - covered, 0)
            elif 4 <= kind <= 8:
                blocked[(name, "its claimant unknown")] += t1 - t0
    held = [(e[0], e[1]) for ev in by_thread.values() for e in ev.get(EV_HELD, [])]
    busy = union_length(held, span[0], span[1])
    return {
        "workers": sorted(set(worker_of.values())),
        "span_ms": (span[1] - span[0]) / 1e6,
        "lock_busy_ms": busy / 1e6,
        "waits": {k: [count[k], total[k] / 1e6] for k in total},
        "blocked_by": {f"{a} <- {b}": v / 1e6 for (a, b), v in sorted(blocked.items(), key=lambda kv: -kv[1])},
    }


def quantiles(xs):
    xs = sorted(xs)
    if not xs:
        return "none"
    q = lambda f: xs[min(int(f * len(xs)), len(xs) - 1)] / 1e3
    return f"{len(xs)}, {sum(xs) / 1e6:.1f} ms summed, p50 {q(0.5):.1f} us, p90 {q(0.9):.1f}, p99 {q(0.99):.1f}, max {xs[-1] / 1e3:.0f}"


def counts(xs):
    xs = sorted(xs)
    if not xs:
        return "none"
    q = lambda f: xs[min(int(f * len(xs)), len(xs) - 1)]
    return f"{len(xs)}, p50 {q(0.5)}, p90 {q(0.9)}, p99 {q(0.99)}, max {xs[-1]}"


def handovers(path):
    by_thread, worker_of = read(path)
    releases = sorted((e[0], t, e[3], e[2], e[1] - e[0]) for t, ev in by_thread.items() for e in ev.get(EV_RELEASE, []))
    if not releases:
        return None
    acquisitions = []
    began = []
    relapsed = 0
    for t, ev in by_thread.items():
        taken = ev.get(EV_TAKEN, [])
        parks = ev.get(EV_PARK, [])
        starts = [p[0] for p in parks]
        # Each acquisition after a wait is its own hold's: the last one ending before the hold
        # begins, consumed by it; a hold with none since the thread's previous hold arrived.
        ti = 0
        for t0, t1, _, _ in ev.get(EV_HELD, []):
            k = None
            while ti < len(taken) and taken[ti][1] <= t0:
                k = taken[ti]
                ti += 1
            if k is None:
                acquisitions.append((t0, t, "arrived", None))
                began.append((t0, t0))
                continue
            w0, w1, _, nparks = k
            i = bisect.bisect_right(starts, w1) - 1
            own = [p for p in parks[max(i - nparks + 1, 0):i + 1] if w0 <= p[0] <= w1] if nparks else []
            relapsed += max(len(own) - 1, 0)
            acquisitions.append((t0, t, "woken" if own else "spun", own[-1][1] if own else None))
            began.append((t0, w0))
    acquisitions.sort()
    times = [a[0] for a in acquisitions]
    classes = defaultdict(list)
    wake = []
    to_acquire = []
    weighted = 0
    idle_releases = 0
    for t_r, t, waiting, woke, wake_ns in releases:
        i = bisect.bisect_left(times, t_r)
        if i >= len(acquisitions):
            continue
        t_a, who, how, woken_at = acquisitions[i]
        gap = t_a - t_r
        if waiting == 0:
            idle_releases += 1
            continue
        weighted += gap * waiting
        if how == "arrived":
            how = "the releaser again" if who == t else "another thread arriving"
        classes[how].append(gap)
        if how == "woken" and woken_at is not None and woken_at >= t_r:
            wake.append(woken_at - t_r)
            to_acquire.append(t_a - woken_at)
    held = [e[1] - e[0] for ev in by_thread.values() for e in ev.get(EV_HELD, [])]
    # A wait overtaken: an acquisition inside it by a thread that began waiting, or arrived, after it.
    began.sort()
    at = [b[0] for b in began]
    overtaken, overtaken_parked = [], []
    for ev in by_thread.values():
        for w0, w1, _, parks in ev.get(EV_TAKEN, []):
            n = sum(1 for _, b in began[bisect.bisect_right(at, w0):bisect.bisect_left(at, w1)] if b > w0)
            overtaken.append(n)
            if parks:
                overtaken_parked.append(n)
    sched = {}
    for t, ev in by_thread.items():
        if ev.get(EV_SCHED):
            delay, run, _, slices = ev[EV_SCHED][0]
            sched[worker_of[t]] = [round(delay / 1e6, 1), round(sum(e[1] - e[0] for e in ev.get(EV_WORK, [])) / 1e6, 1), slices]
    return {
        "releases": len(releases),
        "releases_found_none_waiting": idle_releases,
        "releases_waking": sum(1 for r in releases if r[3]),
        "release_wake_call": quantiles([r[4] for r in releases if r[3]]),
        "handovers_by_next_holder": {k: quantiles(v) for k, v in sorted(classes.items())},
        "handover_gaps_weighted_by_waiters_ms": weighted / 1e6,
        "wake_latency_from_release": quantiles(wake),
        "wake_to_acquisition": quantiles(to_acquire),
        "parks": sum(len(ev.get(EV_PARK, [])) for ev in by_thread.values()),
        "parks_woken_and_parked_again": relapsed,
        "holds": quantiles(held),
        "waits": quantiles([e[1] - e[0] for ev in by_thread.values() for e in ev.get(EV_TAKEN, [])]),
        "waits_that_parked": quantiles([e[1] - e[0] for ev in by_thread.values() for e in ev.get(EV_TAKEN, []) if e[3] > 0]),
        "longest_hold_us": max(held, default=0) / 1e3,
        "overtaken": counts(overtaken),
        "overtaken_parked": counts(overtaken_parked),
        "run_delay_ms_by_worker": dict(sorted(sched.items())),
    }


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    r = analyse(sys.argv[1])
    h = handovers(sys.argv[1])
    if "--json" in sys.argv:
        r["handovers"] = h
        print(json.dumps(r, indent=1))
        return
    print(f"workers {len(r['workers'])}; body phase {r['span_ms']:.1f} ms; loader's lock held {r['lock_busy_ms']:.1f} ms of it ({100 * r['lock_busy_ms'] / max(r['span_ms'], 1e-9):.0f}%)")
    print("waits (count, ms summed over the workers):")
    for k, (n, t) in sorted(r["waits"].items(), key=lambda kv: -kv[1][1]):
        print(f"  {k}: {n}, {t:.1f}")
    print("blocked by (ms summed over the waiting workers):")
    for k, v in r["blocked_by"].items():
        if v >= 0.05:
            print(f"  {k}: {v:.1f}")
    if h:
        print(f"the lock's releases: {h['releases']}, {h['releases_found_none_waiting']} finding no thread waiting, {h['releases_waking']} waking one (the wake's call: {h['release_wake_call']})")
        print("the hand-overs, a release with threads waiting to the next acquisition, by how the next holder took it (gaps):")
        for k, v in h["handovers_by_next_holder"].items():
            print(f"  {k}: {v}")
        print(f"  the gaps times the threads waiting: {h['handover_gaps_weighted_by_waiters_ms']:.1f} ms")
        print(f"a woken holder's wake from the release: {h['wake_latency_from_release']}")
        print(f"  and from its wake to the acquisition: {h['wake_to_acquisition']}")
        print(f"parks: {h['parks']}, {h['parks_woken_and_parked_again']} of them a waiter's second or later in one wait")
        print(f"holds: {h['holds']}")
        print(f"a wait's latency, its start to its acquisition: {h['waits']}")
        print(f"  the waits that parked: {h['waits_that_parked']}; the longest hold {h['longest_hold_us']:.0f} us")
        print(f"a wait overtaken, the acquisitions inside it by threads that began waiting or arrived after it: {h['overtaken']}")
        print(f"  the waits that parked: {h['overtaken_parked']}")
        if h["run_delay_ms_by_worker"]:
            print("the scheduler's run delay by worker (ms runnable and not running, ms of work, slices):")
            print("  " + ", ".join(f"{w}: {v[0]}/{v[1]}/{v[2]}" for w, v in h["run_delay_ms_by_worker"].items()))


if __name__ == "__main__":
    main()
