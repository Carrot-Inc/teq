#!/usr/bin/env python3
"""A stand-in daemon for the client's checks (tests/task.sh): fake_daemon.py <export root> <mode> <hold seconds> <log>
holds the export's target/teq/task.lock, advertises itself in daemon.json and answers each connection by <mode>:
`nothing` (reads the request, closes: a daemon that says nothing and is still there), `gone` (reads the request,
removes its daemon.json and exits at once, the connection closed by its end: a daemon that ended under the request),
`withdrawn` (reads the request, removes its daemon.json, closes, and keeps the lock: a daemon that is ending),
`ended` (one err line, closes), `restart` (answers restart and keeps its daemon.json: a daemon whose end is slow),
`silent` (reads the request and never answers). <hold seconds> after its first connection (0: never, until killed)
it removes its daemon.json, lets go of the lock and exits. Each request's verb and time go to <log>, a line each
(`<seconds> request <verb>`)."""
import fcntl, json, os, socket, sys, threading, time
root, mode, hold, log = sys.argv[1], sys.argv[2], float(sys.argv[3]), sys.argv[4]
d = os.path.join(root, "target/teq")
os.makedirs(d, exist_ok=True)
lock = open(os.path.join(d, "task.lock"), "a+")
fcntl.flock(lock, fcntl.LOCK_EX)
s = socket.socket(); s.bind(("127.0.0.1", 0)); s.listen(16)
info = os.path.join(d, "daemon.json")
with open(info + ".tmp", "w") as f:
    json.dump({"port": s.getsockname()[1], "token": "fake", "pid": os.getpid(), "sha256": ""}, f)
os.rename(info + ".tmp", info)
t0 = time.monotonic()
first = [None]
def say(text):
    with open(log, "a") as f:
        f.write("%.2f %s\n" % (time.monotonic() - t0, text))
say("fake %d listening (%s)" % (os.getpid(), mode))
def leave():
    time.sleep(hold)
    try:
        if json.load(open(info)).get("pid") == os.getpid():
            os.remove(info)
    except (OSError, ValueError):
        pass
    say("fake leaves")
    os._exit(0)
def serve(c):
    line = c.makefile().readline()
    try:
        verb = json.loads(line).get("verb")
    except ValueError:
        verb = "?"
    say("request %s" % verb)
    if first[0] is None and hold > 0:
        first[0] = True
        threading.Thread(target=leave, daemon=True).start()
    if mode in ("gone", "withdrawn"):
        try:
            os.remove(info)
        except OSError:
            pass
        if mode == "gone":
            say("fake leaves")
            os._exit(0)
    elif mode == "ended":
        c.sendall(b'{"err":"a line of the answer\\n"}\n')
    elif mode == "restart":
        c.sendall(b'{"restart":"the client is another teq binary"}\n')
    elif mode == "silent":
        time.sleep(3600)
    c.close()
while True:
    c, _ = s.accept()
    threading.Thread(target=serve, args=(c,), daemon=True).start()
