#!/bin/bash
# bench/app/app-lists.sh <checkout> <project> <out dir>: the gate's lists of the application's API side, read from its
# sbt build: sbt-teq's `teqInputs` of <project>'s Compile and Test configurations (integrations/sbt, Inputs.scala), the
# plugin added to that run alone (sbt's -addPluginSbtFile) unless the build names it, written into <out dir>, which
# stays outside the repository with the application (docs/DEVELOPING.md, "The landing gate"):
#   api-modules.txt       the main modules, upstream first, one per line: its source directories relative to the
#                         checkout (bench/app/api-check.sh's APP_MODULES); its first lines say the configuration and
#                         the source counts
#   api-classpath.txt     the main class path, one jar per line, `~` for the home directory (APP_CLASSPATH): the
#                         Compile configuration's dependencyClasspath less the products of the main modules, which
#                         are compiled from their sources
#   api-flags.txt         the teq flags the configuration's scalacOptions map onto, one line (APP_FLAGS)
#   api-scalac-options.txt  the scalacOptions themselves, one per line (APP_SCALAC_OPTIONS)
#   api-test-modules.txt, api-test-classpath.txt, api-test-flags.txt, api-test-scalac-options.txt
#                         the same of the Test configuration (APP_TEST_*), its class path the Test configuration's
#                         dependencyClasspath with each project's products a line `@<project>/<configuration>` in its
#                         place, every one of them a main module's: a check puts the products of its build of the
#                         main lists at the first and drops the others
#   lists.env             the variables above as `export` lines, for a shell to source
#   summary.txt           per list its configuration, modules, source and class path counts, the options teq
#                         ignores, and the modules whose options differ from the program's
# A module's directories are its source directories that exist and hold a source of sbt's (one inside another
# left out), and a source outside them is listed itself; the lists are refused unless those inputs expanded as teq
# expands a directory (every `.scala` file under it, hidden directories left out) give exactly sbt's sources, the
# generated ones included, and every main module maps onto the same teq flags (one program takes one set). With
# APP_MODULES and APP_CLASSPATH naming existing lists, the new main ones are compared with them (the module rows and
# the class path's entries) and the difference printed. sbt runs in the checkout (writing its target directories
# there, as any sbt run does) with SBT_OPTS (default -Xmx6g), bounded by APP_LISTS_SECONDS (default 1800); answering
# the class paths compiles and packages the main modules. The plugin is build.teq's sbt-teq at TEQ_PLUGIN_VERSION, by
# default this checkout's line (integrations/sbt/plugin-version.txt), as Maven Central serves it: before a release
# carries teqInputs, a branch's or master's plugin published locally under a SNAPSHOT of its own (docs/DEVELOPING.md,
# "Building and testing").
[ $# = 3 ] || { echo "usage: bench/app/app-lists.sh <checkout> <project> <out dir>" >&2; exit 2; }
here=$(cd "$(dirname "$0")" && pwd)
checkout=$(cd "$1" 2> /dev/null && pwd) || { echo "app-lists: no checkout $1" >&2; exit 2; }
project=$2
mkdir -p "$3" && out=$(cd "$3" && pwd) || exit 2
case $out/ in "$(cd "$here/../.." && pwd)"/*) echo "app-lists: $out is inside the repository; the lists stay outside it" >&2; exit 2 ;; esac
version=${TEQ_PLUGIN_VERSION:-$(tr -d '[:space:]' < "$here/../../integrations/sbt/plugin-version.txt")}
[ -n "$version" ] || { echo "app-lists: no plugin version: integrations/sbt/plugin-version.txt is empty" >&2; exit 2; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
add=()
if ! grep -q 'sbt-teq' "$checkout"/project/*.sbt 2> /dev/null; then
  echo "addSbtPlugin(\"build.teq\" % \"sbt-teq\" % \"$version\")" > "$tmp/plugin.sbt"
  add=("-addPluginSbtFile=$tmp/plugin.sbt")
fi
log=$out/sbt.log
echo "app-lists: sbt in $checkout, sbt-teq $version${add:+ added for the run}; its log $log"
start=$(date +%s)
(cd "$checkout" && SBT_OPTS=${SBT_OPTS:--Xmx6g} timeout "${APP_LISTS_SECONDS:-1800}" sbt --server --batch -no-colors "${add[@]}" \
  "$project/Compile/teqInputs" "$project/Test/teqInputs") > "$log" 2>&1
code=$?
[ $code = 0 ] || { tail -20 "$log"; echo "app-lists: sbt exits $code after $(($(date +%s) - start)) s"; exit 1; }
echo "app-lists: sbt took $(($(date +%s) - start)) s"
inputs=$(grep -o 'teq: wrote .*inputs-[a-z]*\.json' "$log" | sed 's/^teq: wrote //')
python3 - "$here" "$checkout" "$out" $inputs <<'PY'
import difflib, json, os, re, sys

here, checkout, out, *files = sys.argv[1:]
sys.dont_write_bytecode = True
sys.path.insert(0, here)
from diagnostics import expand  # teq's expansion of a directory, as the scalac oracle reads it
docs = {}
for f in files:
    d = json.load(open(f))
    docs[d["configuration"]] = d
if set(docs) != {"compile", "test"}:
    sys.exit(f"app-lists: sbt wrote the inputs of {sorted(docs)}, not of compile and test")
root = docs["compile"]["root"]
home = os.path.expanduser("~")

def rel(p):
    r = os.path.relpath(p, root)
    return r if not r.startswith("..") else p

def jar(p):
    return "~/" + os.path.relpath(p, home) if p.startswith(home + "/") else p

problems = []
def inputs_of(m):
    """A module's inputs: its existing source directories that hold one of its sources, one inside another left out,
    then each source outside them; refused unless their expansion is exactly sbt's sources."""
    where = f"{m['project']}/{m['configuration']}"
    sources = [os.path.normpath(s) for s in m["sources"]]
    java = [s for s in sources if not s.endswith(".scala")]
    if java:
        problems.append(f"{where}: sbt compiles {len(java)} sources teq does not ({rel(java[0])} first)")
    dirs = [os.path.normpath(d) for d in m["unmanaged"] + m["managed"] if os.path.isdir(d)]
    dirs = [d for d in dirs if not any(o != d and d.startswith(o + "/") for o in dirs)]
    dirs = [d for i, d in enumerate(dirs) if d not in dirs[:i] and any(s.startswith(d + "/") for s in sources)]
    loose = [s for s in sources if not any(s.startswith(d + "/") for d in dirs)]
    read = [f for i in dirs + loose for f in expand(i)]
    want = sorted(s for s in sources if s.endswith(".scala"))
    if sorted(read) != want or len(set(read)) != len(read):
        extra = sorted(set(read) - set(want))
        missing = sorted(set(want) - set(read))
        problems.append(f"{where}: its directories expanded give {len(read)} files where sbt compiles {len(want)}"
                        + (f"; not sbt's: {', '.join(rel(f) for f in extra[:3])}" if extra else "")
                        + (f"; missing: {', '.join(rel(f) for f in missing[:3])}" if missing else "")
                        + ("; a file twice" if len(set(read)) != len(read) else ""))
    managed = [d for d in m["managed"] if os.path.isdir(d)]
    generated = sum(1 for s in want if any(s.startswith(os.path.normpath(d) + "/") for d in managed))
    return [rel(i) for i in dirs + loose], len(want), generated

summary = []
written = {}
def write(name, lines):
    with open(os.path.join(out, name), "w") as f:
        f.write("".join(l + "\n" for l in lines))
    written[name] = lines

def options_note(modules, program):
    """The modules whose scalacOptions are not the program's, by the options one has and the other lacks."""
    mine = set(program["scalacOptions"])
    differ = []
    for m in modules:
        theirs = set(m["scalacOptions"])
        if theirs != mine:
            differ.append(f"{m['project']}/{m['configuration']} " + " ".join([f"+{o}" for o in sorted(theirs - mine)] + [f"-{o}" for o in sorted(mine - theirs)]))
    return f"modules whose scalacOptions differ from the program's: {'; '.join(differ)}" if differ else "every module has the program's scalacOptions"

# The main lists: every module of the Compile configuration's inputs, one program.
main = docs["compile"]
mods = main["modules"]
program = mods[-1]
rows, counts = [], []
for m in mods:
    ins, n, g = inputs_of(m)
    rows.append(" ".join(ins))
    counts.append((m["project"], n, g))
    if m["flags"] != program["flags"]:
        problems.append(f"{m['project']}/{m['configuration']} maps onto the teq flags {m['flags']}, the program's {program['flags']}: one program takes one set")
names = {(m["project"], m["configuration"]) for m in mods}
products = [(e["project"], e["configuration"]) for e in main["classpath"] if "project" in e]
for p in products:
    if p not in names:
        problems.append(f"the main class path holds {p[0]}/{p[1]}'s products, which no main module is")
total, gen = sum(n for _, n, _ in counts), sum(g for _, _, g in counts)
head = f"# {main['project']}/compile and its {len(mods) - 1} upstream modules, upstream first: {total} sources, {gen} generated (bench/app/app-lists.sh)"
write("api-modules.txt", [head] + rows)
write("api-classpath.txt", [jar(e["path"]) for e in main["classpath"] if "project" not in e])
write("api-flags.txt", [" ".join(program["flags"])])
write("api-scalac-options.txt", program["scalacOptions"])
summary += [f"main: {main['project']}/compile, {len(mods)} modules ({', '.join(f'{p} {n}' for p, n, _ in counts)}), {total} sources ({gen} generated), "
            f"{len(written['api-classpath.txt'])} class path entries, {len(products)} products left out",
            f"  teq flags: {' '.join(program['flags'])}; ignored: {' '.join(program['ignoredScalacOptions'])}",
            f"  {options_note(mods, program)}"]

# The test lists: the Test configuration's own sources, over the main modules' products.
test = docs["test"]
own = test["modules"][-1]
ins, n, g = inputs_of(own)
cp, seen = [], set()
for e in test["classpath"]:
    if "project" in e:
        p = (e["project"], e["configuration"])
        if p not in names:
            problems.append(f"the test class path holds {p[0]}/{p[1]}'s products, which no main module is: a check over the main lists' products would miss them")
        seen.add(p)
        cp.append(f"@{p[0]}/{p[1]}")
    else:
        cp.append(jar(e["path"]))
for p in sorted(names - seen):
    problems.append(f"the main module {p[0]}/{p[1]} is not on the test class path, which the main lists' products would put there")
at = [i for i, e in enumerate(cp) if e.startswith("@")]
if at and at[-1] - at[0] + 1 != len(at):
    problems.append(f"the test class path's products are not together (lines {at[0] + 1} to {at[-1] + 1} hold a jar): one build of the main lists in their place would change the order")
write("api-test-modules.txt", [f"# {test['project']}/test: {n} sources, {g} generated, over the products of the main lists (bench/app/app-lists.sh)", " ".join(ins)])
write("api-test-classpath.txt", cp)
write("api-test-flags.txt", [" ".join(own["flags"])])
write("api-test-scalac-options.txt", own["scalacOptions"])
summary += [f"test: {test['project']}/test, {n} sources ({g} generated), {len(cp)} class path entries of which {len(seen)} products of the main modules",
            f"  teq flags: {' '.join(own['flags'])}; ignored: {' '.join(own['ignoredScalacOptions'])}"]

env = {"APP_ROOT": checkout, "APP_MODULES": "api-modules.txt", "APP_CLASSPATH": "api-classpath.txt", "APP_FLAGS": "api-flags.txt",
       "APP_SCALAC_OPTIONS": "api-scalac-options.txt", "APP_TEST_MODULES": "api-test-modules.txt", "APP_TEST_CLASSPATH": "api-test-classpath.txt",
       "APP_TEST_FLAGS": "api-test-flags.txt", "APP_TEST_SCALAC_OPTIONS": "api-test-scalac-options.txt"}
def value(k, v):
    if k == "APP_ROOT":
        return v
    return written[v][0] if k.endswith("_FLAGS") else os.path.join(out, v)
write("lists.env", [f"export {k}='{value(k, v)}'" for k, v in env.items()])
write("summary.txt", summary + [f"refused: {p}" for p in problems])
print("\n".join(summary))

# The lists the gate has, compared with these: the module rows, the class path's entries each by its path under
# coursier's cache where it is in one (whose root is the machine's), and the flags.
def cached(entry):
    m = re.search(r"/[Cc]oursier/v1/(.*)$", os.path.expanduser(entry))
    return "<coursier>/" + m.group(1) if m else os.path.expanduser(entry)
for var, name, rows_of in (("APP_MODULES", "api-modules.txt", lambda ls: [l for l in ls if l.strip() and not l.startswith("#")]),
                           ("APP_CLASSPATH", "api-classpath.txt", lambda ls: [cached(l) for l in ls if l.strip()])):
    old = os.environ.get(var, "")
    if not os.path.isfile(old) or os.path.realpath(old) == os.path.realpath(os.path.join(out, name)):
        continue
    a, b = rows_of(open(old).read().splitlines()), rows_of(written[name])
    if a == b:
        print(f"app-lists: {name} has the {len(b)} rows of {old}")
    else:
        print(f"app-lists: {name} differs from {old}:")
        for l in list(difflib.unified_diff(a, b, old, name, lineterm="", n=0))[:40]:
            print("  " + l)
if "APP_FLAGS" in os.environ and os.environ["APP_FLAGS"].split() != program["flags"]:
    print(f"app-lists: api-flags.txt has `{' '.join(program['flags'])}`, APP_FLAGS `{os.environ['APP_FLAGS']}`")
for p in problems:
    print(f"app-lists: refused: {p}")
sys.exit(1 if problems else 0)
PY
