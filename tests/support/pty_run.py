#!/usr/bin/env python3
"""pty_run.py [--columns N] [--pipe] [--signal INT|TERM --when TEXT] <command...>: the command run with its three
streams on a pseudo-terminal (of N columns where said), or with --pipe on a pipe; every byte it writes there goes to
stdout as it wrote it (a terminal's line ends, CR LF, among them). With --signal the command is sent that signal as
soon as TEXT has appeared among its bytes. The exit code is the command's, 128 and the signal's number where a
signal ended it. For a test of what a tool writes where its stderr is a terminal (tests/fix.scala, `told`)."""
import fcntl
import os
import pty
import signal
import struct
import subprocess
import sys
import termios

args = sys.argv[1:]
columns, sig, when, pipe = None, None, None, False
while args and args[0].startswith("--"):
    option = args.pop(0)
    if option == "--columns":
        columns = int(args.pop(0))
    elif option == "--signal":
        sig = getattr(signal, "SIG" + args.pop(0))
    elif option == "--when":
        when = args.pop(0).encode()
    elif option == "--pipe":
        pipe = True
    else:
        sys.exit("pty_run: no option " + option)
if pipe:
    child = subprocess.Popen(args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    stream = child.stdout.fileno()
else:
    stream, terminal = pty.openpty()
    if columns is not None:
        fcntl.ioctl(terminal, termios.TIOCSWINSZ, struct.pack("HHHH", 24, columns, 0, 0))
    child = subprocess.Popen(args, stdin=terminal, stdout=terminal, stderr=terminal)
    os.close(terminal)
seen, sent = b"", False
while True:
    try:
        chunk = os.read(stream, 4096)
    except OSError:
        break
    if not chunk:
        break
    seen += chunk
    sys.stdout.buffer.write(chunk)
    sys.stdout.buffer.flush()
    if sig is not None and not sent and when in seen:
        child.send_signal(sig)
        sent = True
code = child.wait()
sys.exit(128 - code if code < 0 else code)
