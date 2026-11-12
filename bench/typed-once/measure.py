#!/usr/bin/env python3
"""bench/typed-once/measure.py --teq <binary> --work <fresh dir> [--runs 3] [--only <scenario>,...]: what the verbs
of one checkout hold in memory, alone and beside the dev resident, and what a `teq fix` run's time and peak are made of.

The build is bench/typed-once/setup.py's: the tree's own corpora (bench/app's generated shared, frontend and api,
out/budget/app) as two projects, `web` (JavaScript) and `api` (the JVM, with one munit suite), a teq.lock beside them.
Each run of a scenario is made in a copy of its own (<work>/<scenario>-<n>), its daemon stopped at the end.

A figure is a sum over the processes of the scenario: the ones this script starts and their descendants, and the
daemon the build's `target/teq/daemon.json` names with its descendants (the residents, the test runner's JVM); never a
process found by its name. Per process /proc/<pid>/smaps_rollup gives Rss, Pss and Pss_Anon (kB): Rss counts a page
shared by two processes twice, Pss its share, Pss_Anon the private memory a second copy of a typed program is made of.
`steady` is the sum once the verb has answered (a client that exits leaves what stays resident), `peak` the largest
sum a 50 ms sampler saw while it ran. Wall time is the client's, from its start to its exit (or to the answer, for a
resident and for the language server's hover).

Scenarios:
  resident       `teq watch web` (what `teq dev` runs through its plugin): after its first build, then after an edit
                 of one file and a `build` line: a body's, then a definition appended
  <verb>         the verb alone in a fresh copy: fix-syntactic (`teq fix --check web`, DisableSyntax, NoUnusedImports,
                 EmptyCollectionsUnified), fix-semantic (the same with ForbiddenSymbol's eight collection entries),
                 compile (`teq compile web`), test (`teq test api`), lsp (`teq lsp`: one document opened, one hover)
  <verb>+resident the same verb started while `teq watch web` of the same copy is up after its first build
  fix-syntactic-rules1, -rules4, -rules8
                 the syntactic run with one, four and eight rule workers (TEQ_FIX_RULE_THREADS; by default
                 the processors) and the typing's workers as they are (at most eight)
  fix-syntactic-threads1, -threads4
                 the syntactic run under `--threads 1` and `--threads 4`, which set the typing's workers and the rules' alike
  fix-lint, fix-lint-semantic
                 a run whose rules only lint (DisableSyntax; with ForbiddenSymbol): one round, where the two above have
                 two (their rules edit, and what they left is typed again)
  fix-syntactic-phases, fix-syntactic-phases1, fix-lint-phases
                 a run with its record: the largest anon sum inside each round's typing and inside its rules' time
                 (the default rule workers; one; the lint-only run)
  fix-asking-projects
                 both projects under the three rules that ask the SemanticDB service (OptionGetWarn, ForbiddenSymbol,
                 TypelevelHttp4sLiteralsSyntax)
  check          the session's arguments as a one-shot check: a typing's own peak, nothing kept
  session, session-index, session-index-forms
                 the check session the daemon keeps for web's compile configuration, started by hand from its
                 `resident.args`: as it is, with `--index` (the language server's records), and with `--index
                 --semantic-types` (the retained forms the SemanticDB dump's typing keeps, one worker)
Prints a line a run and, per scenario and figure, the median of the runs; --json <file> keeps every run.
Run from the repository's root. Nothing is deleted: name a fresh --work each time."""
import argparse, json, os, shutil, statistics, subprocess, sys, threading, time

SYNTACTIC = 'rules = [DisableSyntax, NoUnusedImports, EmptyCollectionsUnified]\nDisableSyntax.noVars = true\nDisableSyntax.noFinalize = true\n'
SEMANTIC = SYNTACTIC.replace("EmptyCollectionsUnified]", "EmptyCollectionsUnified, ForbiddenSymbol]") + '''ForbiddenSymbol.symbols = [
  "scala.collection.IterableOps.init", "scala.collection.IterableOps.tail", "scala.collection.IterableOps.last",
  "scala.collection.ArrayOps.init", "scala.collection.ArrayOps.head", "scala.collection.StringOps.init", "scala.collection.StringOps.last",
  "scala.collection.IterableOnce.min"
]
'''
# Rules that only lint: a run of one round (no rule edits, so nothing is typed a second time for the edits).
LINT = 'rules = [DisableSyntax]\nDisableSyntax.noVars = true\nDisableSyntax.noFinalize = true\n'
LINT_SEMANTIC = LINT.replace("[DisableSyntax]", "[DisableSyntax, ForbiddenSymbol]") + SEMANTIC[SEMANTIC.index("ForbiddenSymbol.symbols"):]
# The three rules that ask the SemanticDB service, alone.
ASKING = "rules = [OptionGetWarn, ForbiddenSymbol, TypelevelHttp4sLiteralsSyntax]\n" + SEMANTIC[SEMANTIC.index("ForbiddenSymbol.symbols"):]


def children_of(pids):
    """The descendants of `pids`, themselves included, by /proc's parent links."""
    parent = {}
    for name in os.listdir("/proc"):
        if name.isdigit():
            try:
                stat = open("/proc/%s/stat" % name).read()
                parent[int(name)] = int(stat[stat.rindex(")") + 2:].split()[1])
            except (OSError, ValueError):
                pass
    out, grew = set(pids), True
    while grew:
        grew = False
        for pid, up in parent.items():
            if up in out and pid not in out:
                out.add(pid)
                grew = True
    return out


def rollup(pid):
    fields = {"Rss": 0, "Pss": 0, "Pss_Anon": 0}
    try:
        for line in open("/proc/%d/smaps_rollup" % pid):
            key = line.split(":")[0]
            if key in fields:
                fields[key] = int(line.split()[1])
    except OSError:
        pass
    return fields


class Scenario:
    def __init__(self, teq, root):
        self.teq, self.root, self.started, self.peak, self.stop = teq, root, [], {"Rss": 0, "Pss": 0, "Pss_Anon": 0}, False
        self.series = []
        self.env = dict(os.environ, LC_ALL="C.UTF-8", TEQ_CACHE_DIR=os.path.join(os.path.dirname(root), "jar-cache"))
        self.sampler = threading.Thread(target=self.sample, daemon=True)
        self.sampler.start()

    def pids(self):
        roots = {p.pid for p in self.started if p.poll() is None}
        try:
            roots.add(json.load(open(os.path.join(self.root, "target/teq/daemon.json")))["pid"])
        except (OSError, ValueError, KeyError):
            pass
        return {p for p in children_of(roots) if os.path.exists("/proc/%d" % p)}

    def total(self):
        out = {"Rss": 0, "Pss": 0, "Pss_Anon": 0, "processes": 0}
        for pid in self.pids():
            r = rollup(pid)
            if r["Rss"]:
                out["processes"] += 1
                for k in r:
                    out[k] += r[k]
        return out

    def sample(self):
        while not self.stop:
            t = self.total()
            self.series.append((time.monotonic(), t["Pss_Anon"]))
            for k in self.peak:
                self.peak[k] = max(self.peak[k], t[k])
            time.sleep(0.05)

    def take_peak(self):
        peak, self.peak = self.peak, {"Rss": 0, "Pss": 0, "Pss_Anon": 0}
        return peak

    def spawn(self, args, env=None, **kw):
        p = subprocess.Popen([self.teq] + args, cwd=self.root, env=dict(self.env, **(env or {})), **kw)
        self.started.append(p)
        return p

    def run(self, args, bound=300, env=None):
        """A client from its start to its exit: wall seconds, exit code, the peak while it ran, what stays after."""
        self.take_peak()
        start = time.monotonic()
        p = self.spawn(args, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        chunks = []
        reader = threading.Thread(target=lambda: chunks.append(p.stdout.read()), daemon=True)
        reader.start()
        # The client's own high-water mark, which a sampler of sums can miss: the kernel's, read as it exits.
        killer = threading.Timer(bound, p.kill)
        killer.start()
        _, status, usage = os.wait4(p.pid, 0)
        killer.cancel()
        p.returncode = os.waitstatus_to_exitcode(status)
        reader.join(timeout=bound)
        out = chunks[0] if chunks else b""
        wall = time.monotonic() - start
        time.sleep(0.3)
        return {"wall": round(wall, 3), "exit": p.returncode, "client": usage.ru_maxrss, "peak": self.take_peak(), "steady": self.total(), "last": out.decode(errors="replace").strip().split("\n")[-1][:160]}

    def resident(self):
        """`teq watch web` up to the answer of its first build."""
        self.take_peak()
        start = time.monotonic()
        p = self.spawn(["watch", "web"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        line = p.stdout.readline()
        wall = time.monotonic() - start
        time.sleep(0.3)
        return p, {"wall": round(wall, 3), "ok": json.loads(line).get("ok"), "peak": self.take_peak(), "steady": self.total()}

    def session(self, extra):
        """The daemon's resident of web's compile configuration started by hand (its `resident.args`, a check session)
        with `extra` flags, up to the answer of its first build: what the index and the kept forms cost a session."""
        args = open(os.path.join(os.path.dirname(self.root), "warm/target/teq/web/compile/resident.args")).read().split("\n")
        self.take_peak()
        start = time.monotonic()
        p = self.spawn([a for a in args if a] + extra, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        line = p.stdout.readline()
        wall = time.monotonic() - start
        time.sleep(0.3)
        return {"wall": round(wall, 3), "ok": json.loads(line).get("ok"), "peak": self.take_peak(), "steady": self.total()}

    def phases(self, conf, env=None):
        """A `fix` run with its record (`TEQ_FIX_RECORD`), and the largest sum sampled inside each of its windows: a
        round's typing (the record's `typing` line, from its start to its end) and the rules after it (to the next
        round's typing, or to the exit)."""
        record = os.path.join(self.root, "fix-record.txt")
        start = time.monotonic()
        out = self.run(["fix", "--check", "--config", conf, "web"], env=dict(env or {}, TEQ_FIX_RECORD=record))
        typings = []
        for line in open(record):
            if line.startswith("typing "):
                f = dict(kv.split("=", 1) for kv in line.split()[1:])
                typings.append((int(f["round"]), start + int(f["start_us"]) / 1e6, start + int(f["end_us"]) / 1e6))
        end = start + out["wall"]
        for i, (n, a, b) in enumerate(typings):
            after = typings[i + 1][1] if i + 1 < len(typings) else end
            # The rules' window without its first and last tenth of a second: the record's clock starts a little after
            # the process does, and a sample is stamped after it is summed, so one of a typing's can fall past its end.
            for name, lo, hi, edge in (("typing", a, b, 0.0), ("rules", b, after, 0.1)):
                inside = [v for t, v in self.series if lo + edge <= t <= hi - edge]
                out["round %d %s" % (n, name)] = {"seconds": round(hi - lo, 3), "anon": max(inside) if inside else 0, "least": min(inside) if inside else 0}
        return out

    def check(self):
        """The same arguments as a one-shot check: what a typing reaches by itself, with nothing kept after it."""
        args = open(os.path.join(os.path.dirname(self.root), "warm/target/teq/web/compile/resident.args")).read().split("\n")
        return self.run(["compiler", "check"] + [a for a in args[2:] if a and a != "--check"])

    def edit(self, p, n, body):
        """An edit of one file of the frontend and a build line: a body's (a logged string of the
        API client made one letter longer) or a definition appended at the top level."""
        if body:
            rel = os.path.join(self.root, "frontend/frontend/apiclient/AnemometerApi.scala")
            text = open(rel).read()
            assert 'Log.infoEff(s"anemometer' in text, "the body edit's line"
            open(rel, "w").write(text.replace('Log.infoEff(s"anemometer', 'Log.infoEff(s"anemometer' + "x" * (n + 1), 1))
        else:
            rel = sorted(os.path.join(d, f) for d, _, fs in os.walk(os.path.join(self.root, "frontend")) for f in fs if f.endswith(".scala"))[0]
            with open(rel, "a") as f:
                f.write("\nobject TypedOnceEdit%d { val x = %d }\n" % (n, n))
        self.take_peak()
        start = time.monotonic()
        # `teq watch <project>` finds what changed by itself: its `build` line takes no paths.
        p.stdin.write(b"build\n")
        p.stdin.flush()
        answer = json.loads(p.stdout.readline())
        wall = time.monotonic() - start
        time.sleep(0.3)
        return {"wall": round(wall, 3), "ok": answer.get("ok"), "incremental": answer.get("incremental"), "fallback": answer.get("fallback"), "peak": self.take_peak(), "steady": self.total()}

    def lsp(self):
        """`teq lsp` over the copy: initialize, one document opened, one hover answered; the server left up for the measure."""
        self.take_peak()
        start = time.monotonic()
        p = self.spawn(["lsp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        def send(message):
            body = json.dumps(message).encode()
            p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
            p.stdin.flush()
        def read():
            length = 0
            while True:
                header = p.stdout.readline()
                if not header:
                    return None
                if header.strip() == b"":
                    break
                if header.lower().startswith(b"content-length:"):
                    length = int(header.split(b":")[1])
            return json.loads(p.stdout.read(length))
        def answer(of):
            while True:
                message = read()
                if message is None or message.get("id") == of:
                    return message
        path = sorted(os.path.join(d, f) for d, _, fs in os.walk(os.path.join(self.root, "frontend")) for f in fs if f.endswith(".scala"))[1]
        text = open(path).read()
        lines = text.split("\n")
        # The first `val` or `def` of the file: its name is hovered.
        at = next(((i, l.index(w) + len(w)) for i, l in enumerate(lines) for w in ("val ", "def ") if w in l), (0, 0))
        uri = "file://" + path
        send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"processId": os.getpid(), "rootUri": "file://" + self.root, "capabilities": {}}})
        answer(1)
        send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
        send({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {"uri": uri, "languageId": "scala", "version": 1, "text": text}}})
        send({"jsonrpc": "2.0", "id": 2, "method": "textDocument/hover", "params": {"textDocument": {"uri": uri}, "position": {"line": at[0], "character": at[1]}}})
        hover = answer(2)
        wall = time.monotonic() - start
        time.sleep(0.3)
        out = {"wall": round(wall, 3), "hover": bool(hover and hover.get("result")), "peak": self.take_peak(), "steady": self.total()}
        send({"jsonrpc": "2.0", "id": 3, "method": "shutdown", "params": None})
        answer(3)
        send({"jsonrpc": "2.0", "method": "exit", "params": None})
        try:
            p.wait(timeout=20)
        except subprocess.TimeoutExpired:
            p.kill()
        return out

    def close(self):
        for p in self.started:
            if p.poll() is None and p.stdin:
                try:
                    p.stdin.write(b"quit\n")
                    p.stdin.flush()
                except OSError:
                    pass
        subprocess.run([self.teq, "stop"], cwd=self.root, env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=60)
        for p in self.started:
            try:
                p.wait(timeout=20)
            except subprocess.TimeoutExpired:
                p.kill()
        self.stop = True
        self.sampler.join()


VERBS = {
    "fix-syntactic": lambda s: s.run(["fix", "--check", "--config", "syntactic.conf", "web"]),
    "fix-semantic": lambda s: s.run(["fix", "--check", "--config", "semantic.conf", "web"]),
    "fix-syntactic-rules1": lambda s: s.run(["fix", "--check", "--config", "syntactic.conf", "web"], env={"TEQ_FIX_RULE_THREADS": "1"}),
    "fix-syntactic-rules4": lambda s: s.run(["fix", "--check", "--config", "syntactic.conf", "web"], env={"TEQ_FIX_RULE_THREADS": "4"}),
    "fix-syntactic-rules8": lambda s: s.run(["fix", "--check", "--config", "syntactic.conf", "web"], env={"TEQ_FIX_RULE_THREADS": "8"}),
    "fix-syntactic-threads1": lambda s: s.run(["fix", "--check", "--config", "syntactic.conf", "--threads", "1", "web"]),
    "fix-syntactic-threads4": lambda s: s.run(["fix", "--check", "--config", "syntactic.conf", "--threads", "4", "web"]),
    "fix-lint": lambda s: s.run(["fix", "--check", "--config", "lint.conf", "web"]),
    "fix-lint-semantic": lambda s: s.run(["fix", "--check", "--config", "lint-semantic.conf", "web"]),
    "fix-syntactic-phases": lambda s: s.phases("syntactic.conf"),
    "fix-syntactic-phases1": lambda s: s.phases("syntactic.conf", env={"TEQ_FIX_RULE_THREADS": "1"}),
    "fix-lint-phases": lambda s: s.phases("lint.conf"),
    "fix-asking-projects": lambda s: s.run(["fix", "--check", "--config", "asking.conf", "web", "api"]),
    "check": lambda s: s.check(),
    "compile": lambda s: s.run(["compile", "web"]),
    "test": lambda s: s.run(["test", "api"]),
    "lsp": lambda s: s.lsp(),
    "session": lambda s: s.session([]),
    "session-index": lambda s: s.session(["--index"]),
    "session-index-forms": lambda s: s.session(["--index", "--semantic-types"]),
}
# A session started by hand is measured alone.
ALONE = ("session", "session-index", "session-index-forms", "fix-syntactic-rules1", "fix-syntactic-rules4", "fix-syntactic-rules8", "fix-syntactic-threads1", "fix-syntactic-threads4", "check", "fix-syntactic-phases", "fix-syntactic-phases1", "fix-lint-phases", "fix-asking-projects")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--teq", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--only", default="")
    ap.add_argument("--json")
    args = ap.parse_args()
    teq, work = os.path.abspath(args.teq), os.path.abspath(args.work)
    here = os.path.dirname(os.path.abspath(__file__))
    template = os.path.join(work, "build")
    subprocess.run([sys.executable, os.path.join(here, "setup.py"), template, teq], check=True, stdout=subprocess.DEVNULL)
    open(os.path.join(template, "syntactic.conf"), "w").write(SYNTACTIC)
    open(os.path.join(template, "semantic.conf"), "w").write(SEMANTIC)
    open(os.path.join(template, "lint.conf"), "w").write(LINT)
    open(os.path.join(template, "lint-semantic.conf"), "w").write(LINT_SEMANTIC)
    open(os.path.join(template, "asking.conf"), "w").write(ASKING)
    wanted = [w for w in args.only.split(",") if w]
    names = ["resident"] + [v + suffix for v in VERBS for suffix in ("", "+resident") if not (suffix and v in ALONE)]
    results = {}
    # One run outside the measures warms the jar cache every scenario shares (TEQ_CACHE_DIR beside the copies).
    warm = Scenario(teq, shutil.copytree(template, os.path.join(work, "warm")))
    warm.run(["compile", "web"])
    warm.run(["compile", "api"])
    warm.close()
    for name in names:
        if wanted and name not in wanted:
            continue
        for n in range(args.runs):
            s = Scenario(teq, shutil.copytree(template, os.path.join(work, "%s-%d" % (name.replace("+", "-with-"), n))))
            try:
                if name == "resident":
                    p, first = s.resident()
                    row = {"first build": first, "after a body's edit": s.edit(p, n, True), "after a definition added": s.edit(p, n, False)}
                else:
                    verb = name.split("+")[0]
                    row = {}
                    if name.endswith("+resident"):
                        p, first = s.resident()
                        row["resident before"] = first["steady"]
                    row["verb"] = VERBS[verb](s)
            finally:
                s.close()
            results.setdefault(name, []).append(row)
            print(name, n, json.dumps(row), flush=True)
    if args.json:
        json.dump(results, open(args.json, "w"), indent=1)
    def median(rows, path):
        values = []
        for r in rows:
            for k in path:
                r = r[k]
            values.append(r)
        return statistics.median(values)
    mb = lambda kb: "%.0f MB" % (kb / 1024)
    print("\nmedians of %d runs (steady: once the verb has answered; peak: the largest sum sampled; anon: Pss_Anon)" % args.runs)
    for name, rows in results.items():
        for step in rows[0]:
            if step == "resident before":
                print("  %-26s %-14s steady Rss %s Pss %s anon %s" % (name, step, mb(median(rows, [step, "Rss"])), mb(median(rows, [step, "Pss"])), mb(median(rows, [step, "Pss_Anon"]))))
                continue
            own = "; the client's own high-water mark %s" % mb(median(rows, [step, "client"])) if "client" in rows[0][step] else ""
            print("  %-26s %-14s wall %.2f s; steady: %d processes, Rss %s, Pss %s, anon %s; peak: Rss %s, Pss %s, anon %s%s" % (
                name, step, median(rows, [step, "wall"]), median(rows, [step, "steady", "processes"]),
                mb(median(rows, [step, "steady", "Rss"])), mb(median(rows, [step, "steady", "Pss"])), mb(median(rows, [step, "steady", "Pss_Anon"])),
                mb(median(rows, [step, "peak", "Rss"])), mb(median(rows, [step, "peak", "Pss"])), mb(median(rows, [step, "peak", "Pss_Anon"])), own))
            for window in [k for k in rows[0][step] if k.startswith("round ")]:
                print("  %-26s   %-16s %.2f s; the largest anon sum inside it %s, the least %s" % ("", window, median(rows, [step, window, "seconds"]), mb(median(rows, [step, window, "anon"])), mb(median(rows, [step, window, "least"]))))


if __name__ == "__main__":
    main()
