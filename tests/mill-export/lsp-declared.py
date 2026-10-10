#!/usr/bin/env python3
# The language server over a Mill build's lock that declares an input outside the build's root
# (`../external.txt`, as an overlay may without the driver): `teq lsp` registers a watcher of it from its own
# directory (a relative pattern, the client taking them), and its edit, told by the client, publishes the
# stale warning on the build's file. Usage: lsp-declared.py <teq> <work directory>; exits 1 naming what failed.
import hashlib, json, queue, subprocess, sys, threading, time
from pathlib import Path

teq, work = str(Path(sys.argv[1]).resolve()), Path(sys.argv[2]).resolve()
app = work / "app"
app.mkdir(parents=True)
(app / "build.mill").write_text("package build\n")
(app / "inside.txt").write_text("one\n")
(work / "external.txt").write_text("one\n")
declared = ["../external.txt", "inside.txt"]
digest = lambda f: hashlib.sha256((app / f).read_bytes()).hexdigest()
version = subprocess.run([teq, "--version"], capture_output=True, text=True).stdout.split()[1]
(app / "teq.lock").write_text(
    f"teq: {version}\nformat: 1\nexporter: mill\nbinaries: {{}}\nprojects: {{}}\ninputs:\n  declared:\n"
    + "".join(f"    - {json.dumps(d)}\n" for d in declared)
    + "  files:\n" + "".join(f"    {json.dumps(f)}: {digest(f)}\n" for f in ["build.mill", *declared]))

server = subprocess.Popen([teq, "lsp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, cwd=app)
messages = queue.Queue()

def send(message):
    data = json.dumps(message).encode()
    server.stdin.write(f"Content-Length: {len(data)}\r\n\r\n".encode() + data)
    server.stdin.flush()

def read():
    while True:
        header = server.stdout.readline()
        if not header:
            return
        length = int(header.decode().split(":")[1])
        server.stdout.readline()
        messages.put(json.loads(server.stdout.read(length)))

threading.Thread(target=read, daemon=True).start()

def until(found, seconds=20):
    """The first message `found` takes, the server's requests answered meanwhile; None after `seconds`."""
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        try:
            m = messages.get(timeout=max(0.01, deadline - time.monotonic()))
        except queue.Empty:
            return None
        if "method" in m and "id" in m:
            send({"jsonrpc": "2.0", "id": m["id"], "result": None})
        if found(m):
            return m

def watches_external(m):
    if m.get("method") != "client/registerCapability":
        return False
    for r in m["params"]["registrations"]:
        for w in r.get("registerOptions", {}).get("watchers", []):
            g = w["globPattern"]
            if isinstance(g, dict) and g.get("baseUri") == work.resolve().as_uri() and g.get("pattern") == "external.txt":
                return True
    return False

def stale(m):
    return m.get("method") == "textDocument/publishDiagnostics" and any(
        "../external.txt changed" in d["message"] for d in m["params"]["diagnostics"])

failures = []
send({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"processId": None, "rootUri": app.resolve().as_uri(),
      "capabilities": {"workspace": {"didChangeWatchedFiles": {"dynamicRegistration": True, "relativePatternSupport": True}}}}})
send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
if not until(watches_external):
    failures.append("no watcher of ../external.txt from its directory was registered")
(work / "external.txt").write_text("two\n")
send({"jsonrpc": "2.0", "method": "workspace/didChangeWatchedFiles",
      "params": {"changes": [{"uri": (work / "external.txt").resolve().as_uri(), "type": 2}]}})
if not until(stale):
    failures.append("the edit of ../external.txt published no stale warning")
send({"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": None})
send({"jsonrpc": "2.0", "method": "exit", "params": None})
try:
    server.wait(timeout=10)
except subprocess.TimeoutExpired:
    server.kill()
if failures:
    print("; ".join(failures))
    sys.exit(1)
