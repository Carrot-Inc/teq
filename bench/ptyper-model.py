#!/usr/bin/env python3
"""bench/ptyper-model.py <run.json> [<weights.json>] [--threads 4,8,16]: the two bounds of the
parallel type phase from what `teq compiler check --time` wrote with
`TEQ_WORKERS_JSON=<file>` (src/measure.rs).

<run.json> is a run at N workers; <weights.json>, when given, a run of the same build at one worker
through the fork (`TEQ_FORK=1`), whose items' times are the weights without contention; without it
the weights are the N-worker run's own, each item's time less the waits inside it.

- The contention-free lower bound of the body phase at n workers: the largest of the longest
  item, the items' weights spread evenly over n workers, and the loader's lock's held time at
  one worker (its holder's work is the std's and the libraries' completions, the publications
  and the lock's own taking and releasing, one holder at a time while the lock is one); plus
  the run's own phases outside the body phase (signatures, prefix, fork, merge, final). The
  weights are the items' times at one worker through the fork, where nothing waits: only the
  waits and what contention adds to the work are removed, and nothing nested is counted twice,
  since an item's time holds what it typed on demand.
- The model of the current queue (`check::Queue`): the files largest first, a worker taking whole
  files and their items from the head, a worker without a file taking the last item of the file
  with the most items left; each item as long as its weight. The makespan, plus the phases outside
  the body phase.

The dependencies between items (a wait on a cell another item claimed) are left out of both: the
run's cell waits say how much they came to.
"""
import json
import sys


def load(path):
    d = json.load(open(path))
    items = []
    for w in d["workers"]:
        for r in w.get("item_rows", []):
            worker, file, size, index, t0, t1, waited, cpu = r
            items.append({"worker": worker, "file": file, "bytes": size, "index": index, "ns": t1 - t0, "waited": waited, "cpu": cpu})
    return d, items


def phases(d):
    return {name: ns / 1e6 for name, ns in d["phases"]}


def outside_body(ph):
    return sum(v for k, v in ph.items() if not k.startswith("workers"))


def weights(items, uncontended):
    w = {}
    for it in items:
        ns = it["ns"] if uncontended else max(it["ns"] - it["waited"], 0)
        w[(it["file"], it["index"])] = (ns / 1e6, it["bytes"])
    return w


def queue_model(w, n):
    """The makespan of `check::Queue` over the items `w` ((file, index) -> (ms, bytes)) at n workers."""
    files = {}
    for (f, i), (ms, size) in w.items():
        files.setdefault(f, {"bytes": size, "items": {}})["items"][i] = ms
    order = sorted(files, key=lambda f: (-files[f]["bytes"], f))
    heads = {f: 0 for f in order}
    tails = {f: max(files[f]["items"]) + 1 if files[f]["items"] else 0 for f in order}
    import heapq
    free = [(0.0, k) for k in range(n)]
    heapq.heapify(free)
    held = {}
    next_file = 0
    end = 0.0
    while free:
        t, k = heapq.heappop(free)
        f = held.get(k)
        if f is not None and heads[f] < tails[f]:
            i = heads[f]
            heads[f] += 1
        else:
            held.pop(k, None)
            if next_file < len(order):
                f = order[next_file]
                next_file += 1
                held[k] = f
                heapq.heappush(free, (t, k))
                continue
            # The file with the most items left, the first in the queue's order on a tie, as
            # `Queue::steal` takes it.
            best = None
            for g in order:
                n_left = tails[g] - heads[g]
                if n_left > 0 and (best is None or n_left > best[0]):
                    best = (n_left, g)
            if best is None:
                end = max(end, t)
                continue
            f = best[1]
            tails[f] -= 1
            i = tails[f]
        ms = files[f]["items"].get(i, 0.0)
        heapq.heappush(free, (t + ms, k))
        end = max(end, t + ms)
    return end


def totals(d):
    workers = [w for w in d["workers"] if w["elapsed_ns"] > 0]
    wall = sum(w["elapsed_ns"] for w in workers) / 1e6
    waited = sum(sum(v[1] for v in w["waits"].values()) for w in workers) / 1e6
    held = sum(w["held"][1] for w in workers) / 1e6
    cells = sum(sum(v[1] for k, v in w["waits"].items() if "cell" in k) for w in workers) / 1e6
    return workers, wall, waited, held, cells


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    threads = None
    for a in sys.argv[1:]:
        if a.startswith("--threads"):
            threads = [int(x) for x in a.split("=", 1)[1].split(",")]
    if not args:
        print(__doc__)
        sys.exit(2)
    run, run_items = load(args[0])
    ph = phases(run)
    workers, wall, waited, held, cells = totals(run)
    work = wall - waited
    threads = threads or [len(workers)]
    if len(args) > 1:
        base, w_items = load(args[1])
        w = weights(w_items, True)
        _, base_wall, base_waited, base_held, _ = totals(base)
        source = "one worker through the fork"
    else:
        w = weights(run_items, False)
        base_held = held
        source = "this run, each item's time less its waits (contended)"
    longest = max((ms for ms, _ in w.values()), default=0.0)
    total = sum(ms for ms, _ in w.values())
    outside = outside_body(ph)
    body = sum(v for k, v in ph.items() if k.startswith("workers"))
    print(f"run: {len(workers)} workers; type phase {outside + body:.0f} ms: body phase {body:.0f}, outside it {outside:.0f} ({', '.join(f'{k} {v:.0f}' for k, v in ph.items() if not k.startswith('workers'))})")
    print(f"  workers' wall summed {wall:.0f} ms: waits {waited:.0f} (cells {cells:.0f}), the rest {work:.0f} ({work / max(total, 1e-9):.2f} times the weights), the loader's lock held {held:.0f} of it")
    print(f"weights from {source}: {len(w)} items, {total:.0f} ms, the longest {longest:.0f} ms, the loader's lock held {base_held:.0f} ms")
    for n in threads:
        lb_body = max(longest, total / n, base_held)
        model = queue_model(w, n)
        print(f"  {n:2} workers: contention-free lower bound {outside + lb_body:.0f} ms (body {lb_body:.0f}: the longest item {longest:.0f}, the weights over n {total / n:.0f}, the lock's serial work {base_held:.0f}); the queue's model {outside + model:.0f} ms (body {model:.0f}); measured {outside + body:.0f} ms")


if __name__ == "__main__":
    main()
