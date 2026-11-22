#!/usr/bin/env python3
"""bench/release-bound.py <seconds> <prefix> <what> <bound's name> <then> <command> [<argument>...]: the command
held to a bound over every process it starts, whatever their process group or session (bench/github-release.sh's
verbs run under it, their bounds bench/ship-release.sh's; docs/DEVELOPING.md, "Releases").

The command runs in a session of its own, its environment TMPDIR a directory made for it and TEQ_RELEASE_BOUND
`<this pid>:<seconds>`, by which it knows it runs under its bound. This process is the child subreaper of everything
the command starts, so that whatever is orphaned on the way stays among its descendants. At the bound, on TERM, INT
or HUP, and once the command has exited, every descendant still alive is killed (SIGKILL, again until none is left),
then the directory removed. Prints `<prefix><what> ran out of its bound, <seconds> s (<bound's name>): stopped with
every command it ran; <then>` at the bound and exits 124; on a signal, `<prefix><what> received <signal>: ...` and
exits 128 and the signal's number; otherwise exits as the command did. Linux alone (/proc, PR_SET_CHILD_SUBREAPER):
elsewhere it runs nothing and exits 1, as it does without a directory under TMPDIR. Whatever its caller left of
SIGCHLD's disposition and the signal mask, it waits with SIGCHLD at its default and nothing blocked. A SIGKILL of
this process stops nothing of the command, which has a session of its own: a caller's limit ends it by TERM, as
timeout(1) does.
"""
import ctypes
import os
import shutil
import signal
import sys
import tempfile
import time

PR_SET_CHILD_SUBREAPER = 36
ENDS = (signal.SIGTERM, signal.SIGINT, signal.SIGHUP)


def descendants():
    """The live processes under this one, by their parents in /proc."""
    children = {}
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        try:
            with open("/proc/%s/stat" % entry) as f:
                stat = f.read()
        except OSError:
            continue
        state, ppid = stat[stat.rindex(")") + 2:].split()[:2]
        children.setdefault(int(ppid), []).append((int(entry), state))
    found, todo = [], [os.getpid()]
    while todo:
        for pid, state in children.get(todo.pop(), []):
            todo.append(pid)
            if state != "Z":
                found.append(pid)
    return found


def reap():
    """Every exited child reaped, the statuses by pid."""
    done = {}
    while True:
        try:
            pid, status = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return done
        if pid == 0:
            return done
        done[pid] = status


def main():
    if len(sys.argv) < 7:
        sys.exit("usage: release-bound.py <seconds> <prefix> <what> <bound's name> <then> <command> [<argument>...]")
    seconds, prefix, what, name, then = sys.argv[1:6]
    command = sys.argv[6:]

    def say(line):
        sys.stdout.write(prefix + line + "\n")
        sys.stdout.flush()

    try:
        bound = float(seconds)
    except ValueError:
        bound = 0
    if not bound > 0:
        say("%s has no bound ('%s', %s): nothing is run" % (what, seconds, name))
        return 1
    libc = ctypes.CDLL(None, use_errno=True)
    if not os.path.isdir("/proc/self/task") or libc.prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
        say("%s cannot be held to its bound here (it needs Linux's /proc and child subreaper): nothing is run" % what)
        return 1
    # What a caller leaves of its signals would change how this waits: SIGCHLD ignored has the kernel reap the command
    # unseen, and an end signal blocked keeps the caller's own limit from reaching this.
    signal.signal(signal.SIGCHLD, signal.SIG_DFL)
    signal.pthread_sigmask(signal.SIG_SETMASK, set())
    received = []
    for s in ENDS:
        signal.signal(s, lambda number, frame: received.append(number))
    deadline = time.monotonic() + bound
    base = os.environ.get("TMPDIR") or "/tmp"
    try:
        scratch = tempfile.mkdtemp(prefix="release-bound.", dir=base)
    except OSError as e:
        say("%s has no directory of its own under %s (%s): nothing is run" % (what, base, e.strerror))
        return 1
    env = dict(os.environ, TMPDIR=scratch, TEQ_RELEASE_BOUND="%d:%s" % (os.getpid(), seconds))
    try:
        body = os.posix_spawnp(command[0], command, env, setsid=True,
                               setsigdef=(signal.SIGPIPE, signal.SIGXFSZ) + ENDS)
    except OSError as e:
        shutil.rmtree(scratch, ignore_errors=True)
        say("%s could not be run: %s" % (what, e))
        return 1
    status = None
    while status is None and not received and time.monotonic() < deadline:
        status = reap().get(body)
        if status is None:
            time.sleep(min(0.05, max(deadline - time.monotonic(), 0)))
    exited, ending = status is not None, list(received)
    stopped = set()
    give_up = time.monotonic() + 30
    while True:
        reap()
        live = descendants()
        if not live or time.monotonic() > give_up:
            break
        for pid in live:
            try:
                os.kill(pid, signal.SIGKILL)
                stopped.add(pid)
            except ProcessLookupError:
                pass
        time.sleep(0.01)
    shutil.rmtree(scratch, ignore_errors=True)
    if live:
        say("%s: %d of its commands could not be stopped (pids %s)" % (what, len(live), " ".join(map(str, live))))
        return 125
    if ending:
        say("%s received %s: stopped with every command it ran; %s" % (what, signal.Signals(ending[0]).name, then))
        return 128 + ending[0]
    if not exited:
        say("%s ran out of its bound, %s s (%s): stopped with every command it ran; %s" % (what, seconds, name, then))
        return 124
    if stopped:
        say("%s left %d command(s) running at its end, stopped" % (what, len(stopped)))
    code = os.waitstatus_to_exitcode(status)
    return code if code >= 0 else 128 - code


if __name__ == "__main__":
    sys.exit(main())
