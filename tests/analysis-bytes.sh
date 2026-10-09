#!/bin/bash
# What `--analysis-version 2` and `3` must leave alone (docs/TARGETS.md, "The analysis graph"):
# the products of a build and the class files of a session are the same bytes with the request
# and without it, and the graph and the dependencies are the same whatever the typer's threads
# and a session's history.
#   products  every module of every case of tests/modules (but misuse, and a case whose build
#             tests/modules/deferred.txt defers, which has no products) and of tests/analysis/cases
#             built into its products against the upstream products, without the request, with
#             each version and with each under --threads 4: the class files, the .tasty files and
#             the manifest are the same bytes, and the answers under four threads are the same
#             as under one but for their phases' times (`ms`); the Scala.js module's check build (`teq compiler check --products`) the
#             same with and without each version;
#   session   three JVM sessions over a copy of tests/modules/basic/a, two with the request (one
#             version each), taken through the same edits (a body, a signature, a type error, its
#             fix, a file added and removed): after every build the class directories are the
#             same bytes, and the graph and the dependencies of a file whose text came back are
#             the ones its first build answered;
#   deps      a session with version 3 over two files, one of whose methods gains a dependency in
#             its body and loses it again while another method's signature keeps one: each
#             retype is incremental and answers the dependencies a fresh build of the same text
#             answers.
#   answers   what the request is answered with where it is easy to miss: a check session
#             without --own answers the graph and the analysis of every file; a build whose jar
#             cannot be opened, or whose products cannot be written, answers its failure; the
#             marker of an extension method's later type parameters changes with a bound and
#             not with a class besides it (tests/analysis/cases/tparams_*); a type the TASTy
#             reader cannot read (nested past its depth) fails the build, the member named;
#             version 3 against a product directory of TASTy files alone (a check's) answers
#             the graph, each of the directory's classes depended on by its `.tasty` (docs/TARGETS.md,
#             "The contract between the plugin and the compiler"), version 2 the graph; and so is
#             each of the own directory's, an alias-only `$package` and an unlisted class file
#             beside them; a `package object`'s members are the class `p.package`, their class
#             files teq's `p.<File>$package`, and one with parents names no class twice.
# Needs scala-library's jar in the coursier cache (tests/support/jars.sh), without which it
# counts as passed.
cd "$(dirname "$0")/.."
root=$PWD
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
. tests/support/jars.sh
SL=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
if [ ! -f "$SL" ]; then
  echo "analysis-bytes: skipped, no scala-library jar"
  echo "analysis-bytes: 0 passed, 0 failed"
  exit 0
fi
work=$(mktemp -d "${TMPDIR:-/tmp}/teq-analysis-bytes.XXXXXX")
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
check() {
  if [ "$2" = 0 ]; then pass=$((pass + 1)); else echo "FAIL $1"; fail=$((fail + 1)); fi
}
# same_answer <a.json> <b.json>: the two answers are the same but for their phases' times (`ms`).
same_answer() {
  python3 -c 'import json, sys
a, b = (json.load(open(f)) for f in sys.argv[1:])
for x in (a, b): x.pop("ms", None)
sys.exit(0 if a == b else 1)' "$1" "$2" 2> /dev/null
}
# failed_answer <answer> <version>: the answer of a build that failed, under the version asked.
failed_answer() {
  python3 -c 'import json, sys; a = json.loads(sys.argv[1]); sys.exit(0 if a.get("ok") is False and a.get("analysisVersion") == int(sys.argv[2]) and "api" not in a else 1)' "$1" "$2" 2> /dev/null
}

for d in tests/modules/*/ tests/analysis/cases/*/; do
  c=$(basename "$d")
  [ "$c" = misuse ] && continue
  grep -q "^$c build " tests/modules/deferred.txt 2> /dev/null && continue
  modules=()
  for m in a b c main; do
    [ -d "$d$m" ] && modules+=("$m")
  done
  [ ${#modules[@]} = 0 ] && modules=(top)
  plain_cp=$SL
  for m in "${modules[@]}"; do
    src=$d$m
    [ "$m" = top ] && src=${d%/}
    for v in plain api threads deps deps4; do
      case $v in
        plain) extra=() ;;
        api) extra=(--analysis-version 2) ;;
        threads) extra=(--analysis-version 2 --threads 4) ;;
        deps) extra=(--analysis-version 3) ;;
        deps4) extra=(--analysis-version 3 --threads 4) ;;
      esac
      "$TEQ" compiler build --target jvm --classpath "$plain_cp" --products "$work/$c/$v/$m" --sourceroot "$root" "${extra[@]}" "$src" > "$work/$c-$m-$v.json" 2> "$work/$c-$m-$v.err"
      echo $? > "$work/$c-$m-$v.exit"
    done
    same=0
    for v in api threads deps deps4; do
      diff -r "$work/$c/plain/$m" "$work/$c/$v/$m" > /dev/null || same=1
      cmp -s "$work/$c-$m-plain.exit" "$work/$c-$m-$v.exit" || same=1
    done
    check "$c/$m products with and without the analysis" $same
    graph=0
    same_answer "$work/$c-$m-api.json" "$work/$c-$m-threads.json" || graph=1
    [ -s "$work/$c-$m-api.json" ] || graph=1
    check "$c/$m graph under --threads 4" $graph
    deps=0
    same_answer "$work/$c-$m-deps.json" "$work/$c-$m-deps4.json" || deps=1
    grep -q '"deps"' "$work/$c-$m-deps.json" || deps=1
    check "$c/$m dependencies under --threads 4" $deps
    plain_cp="$work/$c/plain/$m:$plain_cp"
  done
  # The Scala.js module's check build of the first module.
  first=${modules[0]}
  src=$d$first
  [ "$first" = top ] && src=${d%/}
  "$TEQ" compiler check --products "$work/$c/check-plain" --sourceroot "$root" "$src" > /dev/null 2>&1
  plain_exit=$?
  same=0
  for v in 2 3; do
    "$TEQ" compiler check --products "$work/$c/check-$v" --sourceroot "$root" --analysis-version $v "$src" > /dev/null 2>&1
    [ $? = $plain_exit ] || same=1
    # A module a check refuses (the JDK's classes, which Scala.js lacks) has no products.
    [ $plain_exit != 0 ] || diff -r "$work/$c/check-plain" "$work/$c/check-$v" > /dev/null || same=1
  done
  check "$c/$first check products with and without the analysis" $same
done

# Two sessions through the same edits.
python3 - "$TEQ" "$SL" "$work" "$root/tests/modules/basic/a/A.scala" <<'EOF' > "$work/session.txt"
import filecmp, json, os, shutil, subprocess, sys, time
teq, lib, work, source = sys.argv[1:5]
dirs = {}
procs = {}
for v in ("plain", "api", "deps"):
    d = os.path.join(work, "session-" + v)
    os.makedirs(os.path.join(d, "src"))
    os.makedirs(os.path.join(d, "out"))
    shutil.copy(source, os.path.join(d, "src", "A.scala"))
    dirs[v] = d
    extra = {"plain": [], "api": ["--analysis-version", "2"], "deps": ["--analysis-version", "3"]}[v]
    procs[v] = subprocess.Popen([teq, "compiler", "watch", "--target", "jvm", "--classpath", lib, "-o", "out"] + extra + ["src"],
                                cwd=d, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
text = open(source).read()
def answers():
    return {v: json.loads(p.stdout.readline()) for v, p in procs.items()}
def same_out():
    def clean(c):
        if c.left_only or c.right_only or c.diff_files or c.funny_files:
            return False
        return all(clean(s) for s in c.subdirs.values())
    for other in ("api", "deps"):
        if not clean(filecmp.dircmp(os.path.join(dirs["plain"], "out"), os.path.join(dirs[other], "out"))):
            return False
        for root_dir, _, files in os.walk(os.path.join(dirs["plain"], "out")):
            for f in files:
                a = os.path.join(root_dir, f)
                b = a.replace(dirs["plain"], dirs[other], 1)
                if open(a, "rb").read() != open(b, "rb").read():
                    return False
    return True
def step(name, edit):
    time.sleep(1.1)
    for v in procs:
        edit(dirs[v])
    for p in procs.values():
        p.stdin.write("build\n")
        p.stdin.flush()
    a = answers()
    ok = same_out() and all(a["plain"]["ok"] == a[v]["ok"] and a["plain"].get("changed") == a[v].get("changed") and a["plain"].get("deleted") == a[v].get("deleted") for v in ("api", "deps"))
    print(("ok " if ok else "FAIL ") + name)
    return a
first = answers()
print(("ok " if same_out() else "FAIL ") + "first build")
first_api = first["api"].get("api")
def deps_of(answer):
    api = answer.get("api") or {}
    entries = api.get("entries", [])
    out = {}
    for f in api.get("files", []):
        out[f["file"]] = {c: dict(d, binaries=[[entries[e], b, ctx] for e, b, ctx in d["binaries"]]) for c, d in f.get("deps", {}).items()}
    return out
first_deps = deps_of(first["deps"])
def write(rel, t):
    return lambda d: open(os.path.join(d, "src", rel), "w").write(t)
step("body edit", write("A.scala", text.replace("x * 2", "x * 3")))
step("signature edit", write("A.scala", text.replace("def show: String", "def show: Any")))
step("type error", write("A.scala", text.replace("x * 2", "x * \"no\"")))
a = step("fixed", write("A.scala", text))
# The session's one file typed again from the text it started with: the same graph.
print(("ok " if first_api is not None and a["api"].get("api") == first_api else "FAIL ") + "the graph of the text come back")
print(("ok " if first_deps and deps_of(a["deps"]) == first_deps else "FAIL ") + "the dependencies of the text come back")
step("file added", write("B.scala", "package ma\nclass Extra(val n: Int)\n"))
step("file removed", lambda d: os.remove(os.path.join(d, "src", "B.scala")))
for p in procs.values():
    p.stdin.write("quit\n")
    p.stdin.flush()
    p.wait(timeout=30)
EOF
session_status=$?
while read -r status name; do
  if [ "$status" = ok ]; then pass=$((pass + 1)); else echo "FAIL session $name"; fail=$((fail + 1)); fi
done < "$work/session.txt"
[ $session_status = 0 ] || { echo "FAIL session: the sessions' driver ended with $session_status"; fail=$((fail + 1)); }
[ -s "$work/session.txt" ] || { echo "FAIL session: no answers"; fail=$((fail + 1)); }
echo "session: $(grep -c '^ok' "$work/session.txt") of $(wc -l < "$work/session.txt" | tr -d ' ') steps the same"

# A body dependency added and removed while a signature's stays: a retype answers what a
# fresh build of the same text does.
python3 - "$TEQ" "$SL" "$work" <<'EOF' > "$work/deps-session.txt"
import json, os, shutil, subprocess, sys, time
teq, lib, work = sys.argv[1:4]
a_src = "package rt\n\nclass Dep1:\n  def a: Int = 1\nclass Dep2:\n  def b: String = \"b\"\nclass SigDep\n"
b_v1 = "package rt\n\nclass User:\n  def sig(x: SigDep): Int = 1\n  def body: Int = new Dep1().a\n"
b_v2 = "package rt\n\nclass User:\n  def sig(x: SigDep): Int = 1\n  def body: Int = new Dep1().a + new Dep2().b.length\n"
def by_file(answer, base):
    api = answer.get("api") or {}
    entries = [e.replace(base, "<build>") for e in api.get("entries", [])]
    return {os.path.basename(f["file"]): {c: dict(d, binaries=sorted([entries[e], b, ctx] for e, b, ctx in d["binaries"])) for c, d in f["deps"].items()} for f in api.get("files", [])}
def fresh(b_text):
    d = os.path.join(work, "deps-fresh")
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(os.path.join(d, "src"))
    open(os.path.join(d, "src", "A.scala"), "w").write(a_src)
    open(os.path.join(d, "src", "B.scala"), "w").write(b_text)
    r = subprocess.run([teq, "compiler", "build", "--target", "jvm", "--classpath", lib, "--products", "out", "--analysis-version", "3", "src"], cwd=d, capture_output=True, text=True, timeout=60)
    return by_file(json.loads(r.stdout), d)
d = os.path.join(work, "deps-session")
os.makedirs(os.path.join(d, "src"))
os.makedirs(os.path.join(d, "out"))
open(os.path.join(d, "src", "A.scala"), "w").write(a_src)
open(os.path.join(d, "src", "B.scala"), "w").write(b_v1)
p = subprocess.Popen([teq, "compiler", "watch", "--target", "jvm", "--classpath", lib, "-o", "out", "--analysis-version", "3", "src"], cwd=d, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
state = {}
def fold():
    a = json.loads(p.stdout.readline())
    state.update(by_file(a, d))
    return a
fold()
print(("ok " if state and state == fresh(b_v1) else "FAIL ") + "first build")
for name, text in (("body dependency added", b_v2), ("body dependency removed", b_v1)):
    time.sleep(1.1)
    open(os.path.join(d, "src", "B.scala"), "w").write(text)
    p.stdin.write("build\n")
    p.stdin.flush()
    a = fold()
    print(("ok " if a.get("incremental") else "FAIL ") + name + ", retyped")
    print(("ok " if state == fresh(text) else "FAIL ") + name + ", as a fresh build")
p.stdin.write("quit\n")
p.stdin.flush()
p.wait(timeout=30)
EOF
deps_status=$?
while read -r status name; do
  if [ "$status" = ok ]; then pass=$((pass + 1)); else echo "FAIL deps session $name"; fail=$((fail + 1)); fi
done < "$work/deps-session.txt"
[ $deps_status = 0 ] || { echo "FAIL deps session: the driver ended with $deps_status"; fail=$((fail + 1)); }
[ -s "$work/deps-session.txt" ] || { echo "FAIL deps session: no answers"; fail=$((fail + 1)); }
echo "deps session: $(grep -c '^ok' "$work/deps-session.txt") of $(wc -l < "$work/deps-session.txt" | tr -d ' ') checks"

# What the request is answered with.
answer=$(printf 'quit\n' | timeout 60 "$TEQ" compiler watch --check --analysis-version 2 tests/modules/basic/a 2> /dev/null | head -1)
python3 -c 'import json, sys; a = json.loads(sys.argv[1]); sys.exit(0 if a.get("ok") and a.get("analysisVersion") == 2 and sorted(f["file"] for f in a["api"]["files"]) == sorted(f["file"] for f in a["analysis"]) and a["analysis"] else 1)' "$answer" 2> /dev/null
check "a check session without --own answers the graph of every file" $?
out=$("$TEQ" compiler check --analysis-version 2 --classpath "$work/no-such.jar" tests/modules/basic/a 2> /dev/null)
failed_answer "$out" 2
check "a jar that cannot be opened answers the failure" $?
touch "$work/a-file"
out=$("$TEQ" compiler build --target jvm --classpath "$SL" --products "$work/a-file/products" --analysis-version 2 tests/modules/basic/a 2> /dev/null)
failed_answer "$out" 2
check "products that cannot be written answer the failure" $?
out=$("$TEQ" compiler check --products "$work/a-file/products" --analysis-version 2 tests/modules/basic/a 2> /dev/null)
failed_answer "$out" 2
check "a check build's products that cannot be written answer the failure" $?
for v in string runnable extra; do
  "$TEQ" compiler build --target jvm --classpath "$SL" --products "$work/tparams-$v" --sourceroot "$root" --analysis-version 2 tests/analysis/cases/tparams_$v > "$work/tparams-$v.json" 2> /dev/null
done
marker() {
  python3 -c 'import json, sys; n = json.load(open(sys.argv[1]))["api"]["nodes"]; print([x[2] for x in n if x[0] == "Constant" and str(x[2]).startswith("tparamsExtra:")])' "$1" 2> /dev/null
}
[ -n "$(marker "$work/tparams-string.json")" ] && [ "$(marker "$work/tparams-string.json")" != "[]" ] && [ "$(marker "$work/tparams-string.json")" != "$(marker "$work/tparams-runnable.json")" ]
check "a later type parameter's bound changes its marker" $?
[ -n "$(marker "$work/tparams-string.json")" ] && [ "$(marker "$work/tparams-string.json")" = "$(marker "$work/tparams-extra.json")" ]
check "a class besides leaves the marker" $?
"$TEQ" compiler check --products "$work/tasty-only" --sourceroot "$root" tests/modules/basic/a > /dev/null 2>&1
# named_by_pickles <answer> <dir> <file...>: the answer is a success, and every path of the directory
# it names is a `.tasty` that exists, the files given among them.
named_by_pickles() {
  python3 - "$@" << 'EOF' > /dev/null 2>&1
import json, os, re, sys
answer = json.load(open(sys.argv[1]))
entries = set(re.findall(r'"(' + re.escape(sys.argv[2]) + r'/[^"]*)"', json.dumps(answer)))
named = all(e.endswith(".tasty") and os.path.exists(e) for e in entries) and all(sys.argv[2] + "/" + f in entries for f in sys.argv[3:])
sys.exit(0 if answer.get("ok") and entries and named else 1)
EOF
}
"$TEQ" compiler check --classpath "$work/tasty-only" --analysis-version 3 tests/modules/basic/b > "$work/tasty-only.json" 2> /dev/null
named_by_pickles "$work/tasty-only.json" "$work/tasty-only"
check "version 3 against a directory of TASTy files alone names its classes by their pickles" $?
# The own directory's classes are named by their pickles too, whatever else the directory holds: a
# JVM module's alias-only `$package`, and a class file its manifest does not list.
own=$work/own-deps
mkdir -p "$own"
printf 'package od\ntype Id = Long\n' > "$own/Alias.scala"
printf 'package od\nclass A\n' > "$own/A.scala"
printf 'package od\nclass B(val a: A, val id: Id)\n' > "$own/B.scala"
"$TEQ" compiler build --target jvm --sourceroot "$own" --classpath "$own/out:$SL" --products "$own/out" "$own/Alias.scala" "$own/A.scala" "$own/B.scala" > /dev/null 2>&1
cp "$own/out/od/A.class" "$own/out/Unlisted.class"
"$TEQ" compiler build --target jvm --sourceroot "$own" --classpath "$own/out:$SL" --products "$own/out" --analysis-version 3 "$own/B.scala" > "$own/answer.json" 2> /dev/null
named_by_pickles "$own/answer.json" "$own/out" od/A.tasty 'od/Alias$package.tasty'
check "version 3 names the own directory's classes by their pickles" $?
# A `package object`'s members are the class `p.package` of the answer, as scalac's, their class
# files teq's `p.<File>$package`; with parents, whose `object package` is `p.package`, the members'
# object keeps its file's name, and no class is named twice.
"$TEQ" compiler build --target jvm --sourceroot "$root/tests/modules/pkgobj" --classpath "$work/pkgobj:$SL" --products "$work/pkgobj" --analysis-version 3 tests/modules/pkgobj/a > "$work/pkgobj.json" 2> /dev/null
"$TEQ" compiler build --target jvm --sourceroot "$root/tests/cases/package_object_parents" --classpath "$work/pkgparents:$SL" --products "$work/pkgparents" --analysis-version 3 tests/cases/package_object_parents > "$work/pkgparents.json" 2> /dev/null
python3 - "$work/pkgobj.json" "$work/pkgparents.json" << 'EOF' > /dev/null 2>&1
import json, sys
holder, parents = (json.load(open(f)) for f in sys.argv[1:])
products = [p for f in holder["api"]["files"] for p in f["products"] if p[0] == "pa.nested.package"]
named = sorted(c["name"] for f in holder["analysis"] for c in f["classes"])
likes = [n[1] for n in parents["api"]["nodes"] if isinstance(n, list) and n and n[0] == "ClassLike"]
listed = [c["name"] for f in parents["analysis"] for c in f["classes"]]
ok = sorted(b for _, b in products) == ["pa.nested.Obj$package", "pa.nested.Obj$package$"] and named == ["pa.Top$package", "pa.nested.package"]
sys.exit(0 if ok and len(likes) == len(set(likes)) and len(listed) == len(set(listed)) else 1)
EOF
check "a package object's members are the class p.package, by their binary names" $?
out=$("$TEQ" compiler check --classpath "$work/tasty-only" --analysis-version 2 tests/modules/basic/b 2> /dev/null)
python3 -c 'import json, sys; sys.exit(0 if json.loads(sys.argv[1]).get("ok") else 1)' "$out" 2> /dev/null
check "version 2 against a directory of TASTy files alone answers the graph" $?
mkdir -p "$work/deep"
python3 -c 'import sys; t = "Int"
for _ in range(210): t = "List[" + t + "]"
open(sys.argv[1], "w").write("package deep\n\nobject Deep:\n  def f(x: %s): Int = 0\n" % t)' "$work/deep/Deep.scala"
out=$("$TEQ" compiler build --target jvm --classpath "$SL" --products "$work/deep-out" --analysis-version 2 "$work/deep" 2> "$work/deep.err")
failed_answer "$out" 2 && grep -q "deep.Deep\$.f: a type nested deeper than the TASTy reader reads" "$work/deep.err"
check "a type the reader cannot read fails the graph, its member named" $?
echo "analysis-bytes: $pass passed, $fail failed"
[ $fail = 0 ]
