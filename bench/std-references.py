#!/usr/bin/env python3
"""The references of std definitions in the language server over the application corpus's frontend:
what the first such query costs,
the std files it demands, and what the session keeps.

  bench/std-references.py <teq> [<teq>...] [--out <file.json>] [--top]

The corpus is bench/app's at seed 7 and scale 1, its shared and frontend trees a workspace whose
teq.lock makes them one JavaScript project over the Scala.js builds of cats-kernel, cats-core and
sourcecode (copied from the coursier cache, which `COURSIER_CACHE` names as tests/support/jars.sh
reads it) with the application's cacheable-state object; a session whose build reports an error
stops the run. The symbols are the ten std definitions
the frontend uses most, by the declaration `definition` answers for every identifier and operator
token of its sources, ties broken by spelling (frozen below as TOP; `--top` counts again with the
first binary and prints them). For each binary:

  first   per symbol, a fresh `teq lsp`, the project's file opened and its build awaited, then the
          symbol's `references` from its first use: its milliseconds and locations, with the
          document cache empty (`cold`, every std document written by the query) and holding every
          document the ten write (`warm`, filled by an untimed session first)
  kept    one fresh server, its build awaited, the ten in order and then again: after each, the
          resident size plus swap (smaps_rollup) of the server and of its children summed, and the
          address space (VmSize) apart

Prints a line per measurement and writes every number as JSON (`--out`, default
out/std-references.json). Each child types with one worker (TEQ_SESSION_WORKERS=1).
"""
import json, os, re, shutil, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# The Scala.js builds a JavaScript project's export keeps (scala-library is the lean std's to stand
# for): tests/support/jars.sh's names, sourcecode's taken beside its JVM build.
JARS = ["cats-kernel-sjs", "cats-core-sjs", "sourcecode"]
# (uses, name, declaration in std/, its first use: path under the corpus and byte offset)
TOP = [
    (2003, "Some", "collections.scala:125:18", "shared/core/Codecs.scala", 1426),
    (1874, "List", "collections.scala:252:14", "shared/core/collections/Several.scala", 120),
    (1803, "Option", "collections.scala:49:14", "shared/core/collections/Several.scala", 744),
    (1342, "None", "collections.scala:126:13", "shared/core/collections/Several.scala", 2002),
    (1294, "List", "collections.scala:509:8", "shared/core/amount/Amount.scala", 2095),
    (747, "Nil", "collections.scala:493:13", "shared/core/collections/Several.scala", 1876),
    (557, "length", "collections.scala:269:7", "shared/core/collections/Several.scala", 405),
    (475, "Option", "collections.scala:128:8", "frontend/frontend/component/anemometer/AnemometerBits.scala", 1164),
    (474, "when", "collections.scala:132:7", "frontend/frontend/component/anemometer/AnemometerBits.scala", 1171),
    (442, "map", "collections.scala:322:7", "shared/core/Codecs.scala", 8216),
]


def jar_paths():
    script = ". tests/support/jars.sh; for n in %s; do jar_of $n; done" % " ".join(JARS)
    paths = subprocess.run(["bash", "-c", script], cwd=ROOT, capture_output=True, text=True).stdout.split()
    paths = [p.replace("/sourcecode_3/", "/sourcecode_sjs1_3/").replace("/sourcecode_3-", "/sourcecode_sjs1_3-") for p in paths]
    missing = [p for p in paths if not os.path.isfile(p)]
    if len(paths) != len(JARS) or missing:
        sys.exit(f"std-references: jars missing from the coursier cache: {missing or JARS}")
    return paths


def lock_text(tree, depth=0):
    """A tree as a teq.lock, every key and string quoted (tests/lsp/driver.mjs's lockText)."""
    q = json.dumps
    nested = lambda v: isinstance(v, (dict, list)) and len(v) > 0
    scalar = lambda v: q(v) if isinstance(v, str) else "[]" if isinstance(v, list) else "{}" if isinstance(v, dict) else str(v).lower() if isinstance(v, bool) else str(v)
    pad = "  " * depth
    out = []
    items = [(None, x) for x in tree] if isinstance(tree, list) else list(tree.items())
    for k, x in items:
        head = f"{pad}-" if k is None else f"{pad}{q(k)}:"
        if not nested(x):
            out.append(f"{head} {scalar(x)}")
        elif k is not None:
            out.append(head)
            out.extend(lock_text(x, depth + 1))
        else:
            inner = lock_text(x, depth + 1)
            out.append(f"{pad}- {inner[0][len(pad) + 2:]}")
            out.extend(inner[1:])
    return out


def workspace(work):
    app = os.path.join(work, "app")
    subprocess.run([sys.executable, os.path.join(ROOT, "bench/app/gen.py"), app, "--seed", "7", "--scale", "1"], check=True, capture_output=True, timeout=120)
    os.makedirs(os.path.join(app, "lib"))
    classpath = []
    for p in jar_paths():
        shutil.copy(p, os.path.join(app, "lib", os.path.basename(p)))
        classpath.append({"file": "lib/" + os.path.basename(p)})
    lock = {
        "teq": "0.1.2", "format": 1, "binaries": {}, "inputs": {"files": {}},
        "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}], "jars": {},
        "projects": {"frontend": {"base": "frontend", "platform": "js", "configurations": {"compile": {"sources": ["shared", "frontend"], "classpath": classpath, "flags": {}, "generators": []}}, "description": {"cacheableState": ["meridian.web.css.Catalog"]}}},
    }
    with open(os.path.join(app, "teq.lock"), "w") as f:
        f.write("\n".join(lock_text(lock)) + "\n")
    return os.path.realpath(app)


class Server:
    def __init__(self, teq, root, cache):
        env = dict(os.environ, TEQ_CACHE_DIR=cache, TEQ_SESSION_WORKERS="1")
        self.p = subprocess.Popen([teq, "lsp"], cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env)
        self.next = 1
        self.notes = []
        self.call("initialize", {"processId": None, "rootUri": "file://" + root, "capabilities": {"general": {"positionEncodings": ["utf-16"]}}, "initializationOptions": {"maxSessions": 2}})
        self.notify("initialized", {})

    def send(self, msg):
        body = json.dumps(msg).encode()
        self.p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
        self.p.stdin.flush()

    def read(self):
        length = None
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise RuntimeError("the server ended")
            line = line.strip()
            if not line:
                break
            if line.lower().startswith(b"content-length:"):
                length = int(line.split(b":")[1])
        return json.loads(self.p.stdout.read(length))

    def call(self, method, params):
        i = self.next
        self.next += 1
        self.send({"jsonrpc": "2.0", "id": i, "method": method, "params": params})
        while True:
            msg = self.read()
            if msg.get("id") == i and "method" not in msg:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result")
            self.take(msg)

    def take(self, msg):
        if "id" in msg and "method" in msg:
            self.send({"jsonrpc": "2.0", "id": msg["id"], "result": None})
        else:
            self.notes.append(msg)

    def notify(self, method, params):
        self.send({"jsonrpc": "2.0", "method": method, "params": params})

    def memory(self):
        pids = [self.p.pid] + [int(x) for x in subprocess.run(["pgrep", "-P", str(self.p.pid)], capture_output=True, text=True).stdout.split()]
        out = {"server_kb": 0, "children_kb": 0, "vm_kb": 0}
        for n, pid in enumerate(pids):
            kb = 0
            for l in open(f"/proc/{pid}/smaps_rollup"):
                parts = l.split()
                if parts and parts[0] in ("Rss:", "Swap:"):
                    kb += int(parts[1])
            out["server_kb" if n == 0 else "children_kb"] += kb
            for l in open(f"/proc/{pid}/status"):
                if l.startswith("VmSize:"):
                    out["vm_kb"] += int(l.split()[1])
        return out

    def close(self):
        try:
            self.call("shutdown", None)
            self.notify("exit", None)
            self.p.wait(10)
        except Exception:
            self.p.kill()


def position(path, offset):
    data = open(path, "rb").read()[:offset].decode("utf-8")
    line = data.count("\n")
    return {"line": line, "character": len(data[data.rfind("\n") + 1:].encode("utf-16-le")) // 2}


def started(teq, root, cache):
    s = Server(teq, root, cache)
    first = os.path.join(root, TOP[0][3])
    uri = "file://" + first
    s.notify("textDocument/didOpen", {"textDocument": {"uri": uri, "languageId": "scala", "version": 1, "text": open(first).read()}})
    # Answered once the session's first build is: the publications of its errors come before.
    s.call("textDocument/documentSymbol", {"textDocument": {"uri": uri}})
    errors = [(n["params"]["uri"], d["message"]) for n in s.notes if n.get("method") == "textDocument/publishDiagnostics" for d in n["params"]["diagnostics"] if d.get("severity") == 1]
    if errors:
        s.close()
        sys.exit(f"std-references: the project does not check: {errors[:3]}")
    return s


def references(s, root, sym):
    path = os.path.join(root, sym[3])
    t = time.perf_counter()
    locs = s.call("textDocument/references", {"textDocument": {"uri": "file://" + path}, "position": position(path, sym[4]), "context": {"includeDeclaration": False}}) or []
    return (time.perf_counter() - t) * 1000, len(locs), sum(1 for l in locs if "/attached/std-" in l["uri"])


def measure(teq, root, work):
    label = os.path.basename(os.path.dirname(os.path.dirname(teq))) + "/" + os.path.basename(teq)
    result = {"teq": teq, "version": subprocess.run([teq, "--version"], capture_output=True, text=True).stdout.strip(), "first": {}, "kept": []}
    warm = tempfile.mkdtemp(dir=work)
    for state in ("cold", "warm"):
        if state == "warm":
            # The documents every query of the ten writes, written by an untimed session first.
            s = started(teq, root, warm)
            for sym in TOP:
                references(s, root, sym)
            s.close()
        rows = []
        for sym in TOP:
            cache = tempfile.mkdtemp(dir=work) if state == "cold" else warm
            s = started(teq, root, cache)
            ms, n, in_std = references(s, root, sym)
            s.close()
            rows.append({"name": sym[1], "declaration": sym[2], "ms": round(ms, 1), "locations": n, "in_std": in_std})
            print(f"{label} first {state} {sym[1]:8s} {sym[2]:26s} {ms:8.1f} ms {n:6d} locations ({in_std} in the std)", flush=True)
        result["first"][state] = rows
    s = started(teq, root, warm)
    m = s.memory()
    result["kept"].append({"after": "the build", **m})
    print(f"{label} kept after the build: server {m['server_kb'] / 1024:.1f} MB, children {m['children_kb'] / 1024:.1f} MB, VmSize {m['vm_kb'] / 1024:.1f} MB", flush=True)
    for rnd in (1, 2):
        for sym in TOP:
            ms, n, in_std = references(s, root, sym)
            m = s.memory()
            result["kept"].append({"after": f"{sym[1]} ({sym[2]}), round {rnd}", "ms": round(ms, 1), "locations": n, "in_std": in_std, **m})
            print(f"{label} kept {rnd} {sym[1]:8s} {ms:8.1f} ms {n:6d} locations: server {m['server_kb'] / 1024:.1f} MB, children {m['children_kb'] / 1024:.1f} MB, VmSize {m['vm_kb'] / 1024:.1f} MB", flush=True)
    s.close()
    base = result["kept"][0]
    end = result["kept"][-1]
    result["retained_kb"] = end["server_kb"] + end["children_kb"] - base["server_kb"] - base["children_kb"]
    result["vm_added_kb"] = end["vm_kb"] - base["vm_kb"]
    print(f"{label} retained after the ten and their repeat: {result['retained_kb'] / 1024:.1f} MB resident plus swap, {result['vm_added_kb'] / 1024:.1f} MB of address space", flush=True)
    return result


TOKEN = re.compile(r'"(?:[^"\\\n]|\\.)*"|//[^\n]*|/\*.*?\*/|([A-Za-z_][A-Za-z0-9_]*|[!#%&*+\-/:<=>?@\\^|~]+)', re.S)


def top(teq, root, work):
    """Counts the std declarations `definition` answers for every token of the project, through a
    resident check of its own (the language server's child, asked directly)."""
    cache = tempfile.mkdtemp(dir=work)
    args = [teq, "compiler", "watch", "--check", "--index", os.path.join(root, "shared"), os.path.join(root, "frontend"), "--classpath", ":".join(os.path.join(root, "lib", os.path.basename(p)) for p in jar_paths()), "--cacheable-state", "meridian.web.css.Catalog", "--threads", "1"]
    p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, env=dict(os.environ, TEQ_CACHE_DIR=cache))
    p.stdout.readline()
    counts, first = {}, {}
    for tree in ("shared", "frontend"):
        for d, _, fs in sorted(os.walk(os.path.join(root, tree))):
            for f in sorted(fs):
                if not f.endswith(".scala"):
                    continue
                path = os.path.realpath(os.path.join(d, f))
                text = open(path, encoding="utf-8").read()
                for m in TOKEN.finditer(text):
                    if not m.group(1):
                        continue
                    offset = len(text[: m.start(1)].encode("utf-8"))
                    p.stdin.write(f"definition {path} {offset}\n")
                    p.stdin.flush()
                    for loc in json.loads(p.stdout.readline()).get("result") or []:
                        if "/attached/std-" in loc["uri"]:
                            r = loc["range"]["start"]
                            key = (loc["uri"].split("/attached/", 1)[1].split("/", 1)[1], r["line"], r["character"])
                            counts[key] = counts.get(key, 0) + 1
                            first.setdefault(key, (os.path.relpath(path, root), offset))
    p.stdin.write("quit\n")
    p.stdin.flush()
    rows = []
    for (file, line, col), n in counts.items():
        text = open(os.path.join(ROOT, "std", file), encoding="utf-8").read().split("\n")[line]
        name = re.match(r"[A-Za-z_][A-Za-z0-9_]*|[!#%&*+\-/:<=>?@\\^|~]+", text[col:]).group(0)
        rows.append((n, name, f"{file}:{line + 1}:{col + 1}", *first[(file, line, col)]))
    rows.sort(key=lambda r: (-r[0], r[1], r[2]))
    for r in rows[:10]:
        print(f"    {r},")


def main():
    args = sys.argv[1:]
    out = os.path.join(ROOT, "out/std-references.json")
    if "--out" in args:
        i = args.index("--out")
        out = args[i + 1]
        del args[i : i + 2]
    recount = "--top" in args
    teqs = [os.path.abspath(a) for a in args if a != "--top"]
    if not teqs:
        sys.exit(__doc__)
    work = tempfile.mkdtemp(prefix="std-references-")
    try:
        root = workspace(work)
        if recount:
            top(teqs[0], root, work)
            return
        results = [measure(t, root, work) for t in teqs]
        os.makedirs(os.path.dirname(out), exist_ok=True)
        json.dump({"corpus": "bench/app --seed 7 --scale 1, shared + frontend", "top": TOP, "results": results}, open(out, "w"), indent=1)
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
