#!/bin/bash
# bench/app/chain.sh <build file> <teq> <out dir> frontend|api [<module bound>]: the application's modules built one
# against another's products, as sbt compiles them: each module with --products, its upstream modules' product
# directories on its class path, then all of them as one build; every .tasty and class file a module's manifest owns
# compared with the one build's, in both directions (a file one build owns and the other does not is a difference).
#   frontend  the Scala.js check builds of the export's sources, the build file's class path and flags as
#             bench/app/export-build.mjs takes them; each `src` directory of its `sources` a module, a generated
#             one joining the module before it
#   api       the JVM builds of the module list APP_MODULES names, with bench/app/api-check.sh's class path
#             (APP_CLASSPATH) and flags (APP_FLAGS)
# The build file is sbt-teq's teqExport (the gate's --app); its root, relative to the file, is the checkout, which is
# read and never written. With a bound, the first that many modules alone. The list CHAIN_KNOWN names (a path,
# kept outside the repository with the application; no rows when it is unset or names no file) permits a
# mismatch by the bytes it was reviewed with: `cause <tag>  <cause>, <the case that pins it>` names a cause, and
# `<side> <module> <path> <split sha256> <whole sha256> <tag>` permits that file while both builds' files have those
# digests (`-` for a file a build does not own); the same path with other bytes is a mismatch like any other. A line
# `<side> <module> *  <cause>, <case>` defers the module's build, which ends the chain before it. Every mismatch is
# written to <out dir>/mismatches.txt; with CHAIN_DRAFT=<file>, every mismatch as an entry for that list,
# its tag the listed one where the path is listed and `?` otherwise; CHAIN_FLAGS adds flags to every build.
# Exits 0 when every build succeeds and every mismatch is permitted, 1 otherwise, naming the module whose build
# failed.
[ $# = 4 ] || [ $# = 5 ] || { echo "usage: bench/app/chain.sh <build file> <teq> <out dir> frontend|api [<module bound>]" >&2; exit 2; }
case $4 in frontend | api) ;; *) echo "usage: bench/app/chain.sh <build file> <teq> <out dir> frontend|api [<module bound>]" >&2; exit 2 ;; esac
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
exec python3 - "$(absolute "$1")" "$(absolute "$2")" "$(absolute "$3")" "$4" "${5:-0}" <<'PY'
import hashlib, json, os, shutil, subprocess, sys, time

build_file, teq, out, side, bound = sys.argv[1:6]
bound = int(bound)
d = json.load(open(build_file))
root = os.path.normpath(os.path.join(os.path.dirname(build_file), d.get("root", "../../..")))
seconds = int(os.environ.get("CHAIN_BUILD_SECONDS", "300"))

if side == "frontend":
    modules = []
    for src in d["sources"]:
        if "src_managed" in src and modules:
            modules[-1].append(src)
        else:
            modules.append([src])
    cp = d["classpath"]
    flags = ["--max-inlines", str(d["maxInlines"])] if d.get("maxInlines") else []
    flags += ["--strict-equality"] if d.get("strictEquality") else []
    flags += ["--kind-projector"] if d.get("kindProjector") else []
    flags += ["--werror"] if d.get("werror") else []
    command = "check"
else:
    lists = [os.environ.get(v, "") for v in ("APP_MODULES", "APP_CLASSPATH")]
    if not all(os.path.isfile(f) for f in lists) or "APP_FLAGS" not in os.environ:
        sys.exit("chain api: set APP_MODULES, APP_CLASSPATH and APP_FLAGS to the application's module list, class path and flags")
    modules = [l.split() for l in open(lists[0]) if l.strip() and not l.startswith("#")]
    cp = [os.path.expanduser(l.strip()) for l in open(lists[1]) if l.strip()]
    # bench/app/api-check.sh's, and a JVM module's entry points as sbt compiles them.
    flags = os.environ["APP_FLAGS"].split() + ["--std", "scala-library", "--target", "jvm", "--all-mains"]
    command = "build"
if bound:
    modules = modules[:bound]
all_modules = list(modules)
# CHAIN_FLAGS adds flags to every build, the whole one's included: `--analysis-version 3` runs the
# modules as sbt-teq's compile does, each answer's `ms` the last line of its log.
flags += os.environ.get("CHAIN_FLAGS", "").split()

def defines(module, qualified):
    """Whether a source of the module defines the object `qualified` (`package p.q` and `object C`)."""
    pkg, _, obj = qualified.rpartition(".")
    for src in module:
        for cur, _, files in os.walk(os.path.join(root, src)):
            for f in files:
                if f.endswith(".scala"):
                    text = open(os.path.join(cur, f), encoding="utf-8", errors="replace").read()
                    if f"object {obj}" in text and f"package {pkg}" in text:
                        return True
    return False

# A cacheable object is named to the module that defines it and to every module after it, which a build that cannot
# see it refuses.
cacheable = [(n, next((i for i, m in enumerate(modules) if defines(m, n)), len(modules))) for n in (d.get("cacheableState") or [])] if side == "frontend" else []
def state_flags(upto):
    return [x for n, at in cacheable if at <= upto for x in ("--cacheable-state", n)]
name = lambda m: os.path.basename(os.path.dirname(m[0])) if m[0].endswith("/src") else m[0]

causes = {}
known = {}
deferrals = {}
known_file = os.environ.get("CHAIN_KNOWN", "")
if not os.path.isfile(known_file):
    print(f"chain {side}: no known rows ({'CHAIN_KNOWN names no file: ' + known_file if known_file else 'CHAIN_KNOWN unset'})")
for l in open(known_file) if os.path.isfile(known_file) else []:
    if not l.strip() or l.startswith("#"):
        continue
    w = l.split()
    if w[0] == "cause":
        causes[w[1]] = l.split(None, 2)[2].strip()
    elif len(w) > 2 and w[2] == "*":
        deferrals[(w[0], w[1])] = l.split(None, 3)[3].strip()
    else:
        s_, m, p, a, b, tag = w[:6]
        known[(s_, m, p)] = (a, b, tag)
for key, (_, _, tag) in known.items():
    if tag not in causes:
        sys.exit(f"{known_file}: {' '.join(key)} names the cause {tag}, which no `cause` line gives")

deferred = next(((i, deferrals[(side, name(m))]) for i, m in enumerate(modules) if (side, name(m)) in deferrals), None)
if deferred:
    modules = modules[:deferred[0]]

shutil.rmtree(out, ignore_errors=True)
os.makedirs(out)

def build(label, sources, classpath, upto):
    log = os.path.join(out, label + ".log")
    args = [teq, "compiler", command, "--products", os.path.join(out, label), *sources, "--classpath", ":".join(classpath), *flags, *state_flags(upto)]
    start = time.monotonic()
    try:
        r = subprocess.run(args, cwd=root, stdout=open(log, "w"), stderr=subprocess.STDOUT, timeout=seconds)
        code = r.returncode
    except subprocess.TimeoutExpired:
        code = "timeout"
    took = time.monotonic() - start
    if code != 0 or not os.path.exists(os.path.join(out, label, "teq-products.json")):
        lines = [l.rstrip() for l in open(log) if " error" in l or "cannot" in l][:3]
        print(f"chain {side}: {label}'s build failed ({code} after {took:.1f} s, {log}):")
        for l in lines:
            print("  " + l[:300])
        sys.exit(1)
    return took

def owned(label, sources=None):
    manifest = json.load(open(os.path.join(out, label, "teq-products.json")))
    files = set()
    for p in manifest["products"]:
        if sources is not None and not any(p["source"].startswith(s.rstrip("/") + "/") for s in sources):
            continue
        files.update(([p["tasty"]] if p.get("tasty") else []) + p["classes"])
    return files

times = []
upstream = []
for i, m in enumerate(modules):
    label = f"m{i}-{name(m)}"
    times.append((name(m), build(label, m, cp + upstream, i)))
    upstream.append(os.path.join(out, label))
whole = build("whole", [s for m in modules for s in m], cp, len(modules))

def digest(path):
    if not os.path.exists(path):
        return "-"
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()

same = differ = listed = 0
mismatches = open(os.path.join(out, "mismatches.txt"), "w")
draft = []
seen = set()
by_cause = {}
for i, m in enumerate(modules):
    label = f"m{i}-{name(m)}"
    split, one = owned(label), owned("whole", m)
    for f in sorted(split | one):
        a = digest(os.path.join(out, label, f)) if f in split else "-"
        b = digest(os.path.join(out, "whole", f)) if f in one else "-"
        key = (side, name(m), f)
        if a == b:
            same += 1
            continue
        what = "differs" if a != "-" and b != "-" else "only in the module's build" if b == "-" else "only in the whole build"
        mismatches.write(f"{what}: {name(m)} {f}\n")
        entry = known.get(key)
        draft.append(f"{side} {name(m)} {f} {a} {b} {entry[2] if entry else '?'}")
        seen.add(key)
        if entry and entry[0] == a and entry[1] == b:
            listed += 1
            by_cause[entry[2]] = by_cause.get(entry[2], 0) + 1
        elif entry:
            differ += 1
            print(f"{what} with other bytes than {known_file} permits ({entry[2]}): {name(m)} {f}")
        else:
            differ += 1
            print(f"{what}: {name(m)} {f}")
mismatches.close()
if os.environ.get("CHAIN_DRAFT"):
    with open(os.environ["CHAIN_DRAFT"], "w") as d:
        d.write("\n".join(draft) + ("\n" if draft else ""))
built = {(side, name(m)) for m in modules}
for key in sorted(k for k in known if k[:2] in built and k not in seen):
    print(f"listed {key[1]} {key[2]} is the same in both builds now: take it off {known_file}")
for tag, n in sorted(by_cause.items(), key=lambda x: -x[1]):
    print(f"listed {n} files, {tag}: {causes[tag]}")
builds = ", ".join(f"{n} {t:.1f} s" for n, t in times)
deferral = f"; deferred: {name(all_modules[deferred[0]])} and after it ({deferred[1]})" if deferred else ""
print(f"chain {side}: {len(modules)} modules over their upstreams' products ({builds}), the whole build {whole:.1f} s; "
      f"{same} files identical, {listed} listed, {differ} differ{deferral}")
sys.exit(1 if differ else 0)
PY
