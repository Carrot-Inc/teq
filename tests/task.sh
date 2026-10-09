#!/bin/bash
# `teq` over a small hand-written export (docs/TARGETS.md, "The export and the project verbs"): a JVM project `app`
# with a test configuration over a project `core` and a command generator, its libraries
# scala-library and scala3-library pinned from the coursier cache. The first compile starts the
# daemon and the resident of app/test, whose class files hold core's; a warm compile writes
# nothing; an error fails it with the diagnostic and a fix passes it; an export changed in a
# project's flags is read again by the same daemon, while one whose `teq` line changed (a branch
# switch) has the next client start a new daemon and the old one end; two clients starting at
# once start one daemon; the generator runs again when its input changes; a changed build
# definition warns, and refuses under --strict, one that differs by line ends alone (a CRLF
# checkout of an export of LF, or the reverse) with a note naming it and the remedy; a request
# without the token of `daemon.json` is refused; `stop` ends the daemon. Then `test` over app's
# munit suites (the jars of the example
# fixture's jvmapp tests, from the coursier cache): the suites sbt's discovery would define, the
# exclusion, the patterns, an edit between two warm runs and a resource's, `--changed`, a jar
# rewritten in place restarting the runner, a runner killed and one whose client left, a JVM older
# than the export's classes refused, `stop` ending the runner. Then `run` (an alias and a main
# class, the run block's options, variables and directory, a resource of the runtime classpath,
# the arguments and the exit code, the refusals, the main class implied without a word as sbt's
# `run` implies it: the declared one, the single one among the products, a package object's
# `@main` among them, the refusals naming several or none; SIGINT passed on, the bound on a
# program that does not end, an old JVM) and `stage` (the artifacts copied, the products' jars from their own
# class files and resources, a linked directory's among them, the start script
# run from the staged tree, what it held before removed, a jar left alone when unchanged, the
# refusals; the main class implied as `run` implies it when the block declares none: the single
# product main for the script and the manifest, the manifest left alone where the build set the
# key to None, several or none refused in the stage's words, the block's class winning over
# several). Then an export under the root's target/teq/ (a build without the native driver)
# recording generators sbt alone runs: the verbs that need them refused, the others run, a compile of
# every project naming those it left out and exiting 2. Then an artifact fetched
# over loopback http into the shared cache, verified, and one whose pinned sha1 is wrong refused.
# Then a Scala.js project's `build` and `dev` over an export of their own (needs node), and
# vite-plugin-teq's own test. Without scala-library or munit's jars in the coursier cache it fails
# as incomplete validation.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
. tests/support/jars.sh
work=$(cd "$(mktemp -d)" && pwd -P)
export TEQ_CACHE_DIR=$work/cache/teq
# The lock's writer and reader, the example's check-export.py's.
export LOCK_PY=$PWD/integrations/sbt/example/check-export.py
# edit_lock <file> <statements>: the lock read as `e`, the statements run on it, written again.
edit_lock() {
  python3 - "$1" "$2" <<'PY'
import importlib.util, os, sys
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
file, statements = sys.argv[1:]
e = lock.parse(open(file, encoding="utf-8").read())
exec(statements)
open(file, "w", encoding="utf-8").write(lock.canonical(e))
PY
}
b=$work/b
status=0
pass() { echo "task: $1"; }
fail() { echo "FAIL task: $1"; status=1; }
daemons() { grep -c '^daemon ' "$b/target/teq/task.log" 2> /dev/null || echo 0; }
last_daemon() { sed -nE 's/^daemon ([0-9]+) .*/\1/p' "$b/target/teq/task.log" | tail -1; }
alive() { kill -0 "$1" 2> /dev/null; }
cleanup() {
  [ -f "$b/teq.lock" ] && (cd "$b" && timeout 30 "$TEQ" stop > /dev/null 2>&1)
  [ -f "$work/r/target/teq/teq.lock" ] && (cd "$work/r" && timeout 30 "$TEQ" stop > /dev/null 2>&1)
  [ -n "${server:-}" ] && kill "$server" 2> /dev/null
  rm -rf "$work"
}
trap cleanup EXIT

lib=$(jar_of scala-library)
lib3=$M2/org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar
if [ ! -f "$lib" ] || [ ! -f "$lib3" ]; then
  echo "FAIL: scala-library or scala3-library 3.8.4 is not in the coursier cache (incomplete validation)"
  exit 1
fi
mkdir -p "$b/core/src/core" "$b/app/src/app" "$b/app/test/app" "$b/solo/src/solo" "$b/pair/src/pair" "$b/util/src/util" "$b/pkg/src/pkg" "$b/project"
cat > "$b/core/src/core/Core.scala" <<'EOF'
package core

object Core:
  def answer: Int = 42
EOF
# A trait whose `main` an object inherits, as ZIOAppDefault's objects do: the JVM's entry point
# is the object's static forwarder.
cat > "$b/core/src/core/Entry.scala" <<'EOF'
package core

trait Entry:
  def run(args: Array[String]): Unit
  final def main(args: Array[String]): Unit = run(args)

object CoreMain extends Entry:
  def run(args: Array[String]): Unit = println("core" + args.map(" " + _).mkString)
EOF
cat > "$b/app/src/app/Main.scala" <<'EOF'
package app

object Main:
  def main(args: Array[String]): Unit = println((core.Core.answer + Labels.count).toString + args.map(" " + _).mkString)
EOF
# Projects declaring no main class: solo's products hold one (inherited through core's trait),
# pair's two (an object's own `main` and a `@main` method's class), util's none (a `@main` of an
# object, whose class is not written, aside), pkg's one, a `@main` of a package object (a block
# the analysis answers under the file's path, as the resident's other readers expect).
cat > "$b/solo/src/solo/Solo.scala" <<'EOF'
package solo

object Solo extends core.Entry:
  def run(args: Array[String]): Unit = println("solo" + args.map(" " + _).mkString)
EOF
cat > "$b/pair/src/pair/Pair.scala" <<'EOF'
package pair

object A extends core.Entry:
  def run(args: Array[String]): Unit = println("a")

@main def b(): Unit = println("b")
EOF
cat > "$b/pkg/src/pkg/package.scala" <<'EOF'
package pkg

package object q:
  @main def hi(args: String*): Unit = println("hi" + args.map(" " + _).mkString)
EOF
cat > "$b/util/src/util/Util.scala" <<'EOF'
package util

object Util:
  val n: Int = 1

// A `@main` method of an object: sbt runs the class `util.inner`, which the backend does not
// write yet, so neither it nor Holder (which has no static main) counts among util's products;
// when the backend writes it, util's single main class is `util.inner`.
object Holder:
  @main def inner(): Unit = println("inner")
EOF
cat > "$b/app/src/app/Exit.scala" <<'EOF'
package app

object Exit:
  def main(args: Array[String]): Unit =
    val stream = getClass.getResourceAsStream("/app.txt")
    val resource = if stream == null then "none" else new String(stream.readAllBytes()).trim
    val dir = java.nio.file.Paths.get("").toAbsolutePath.getFileName
    println(System.getProperty("run.option") + " " + System.getenv("RUN_VAR") + " " + dir + " " + resource + " " + args.mkString(","))
    if args.nonEmpty then System.exit(args(0).toInt)

object Sleep:
  def main(args: Array[String]): Unit =
    println("sleeping")
    Thread.sleep(60000)

object Stubborn:
  def main(args: Array[String]): Unit =
    Runtime.getRuntime.addShutdownHook(new Thread(() => Thread.sleep(60000)))
    println("stubborn")
    Thread.sleep(60000)
EOF
mkdir -p "$b/app/resources" "$b/core/resources" "$b/shared-assets"
echo packaged > "$b/app/resources/app.txt"
echo shared > "$b/shared-assets/message.txt"
ln -s ../../shared-assets "$b/app/resources/assets"
echo core > "$b/core/resources/core.txt"
cat > "$b/app/test/app/MainTest.scala" <<'EOF'
package app

class MainTest extends munit.FunSuite:
  test("the answer") { assertEquals(core.Core.answer, 42) }
EOF
cat > "$b/app/test/app/Suites.scala" <<'EOF'
package app

abstract class Base extends munit.FunSuite

class ViaBase extends Base:
  test("a suite through its base") { assertEquals(1 + 1, 2) }

class Excluded extends munit.FunSuite:
  test("excluded") { fail("an excluded suite ran") }

class Helper:
  def help: Int = 1

class Resources extends munit.FunSuite:
  test("a resource of the test configuration") {
    assertEquals(scala.io.Source.fromURL(getClass.getResource("/greeting.txt")).mkString.trim, "hello")
  }
  test("a resource of a jar on the class path") {
    assertEquals(scala.io.Source.fromURL(getClass.getResource("/extra.txt")).mkString.trim, "one")
  }

class Sleeper extends munit.FunSuite:
  override def munitTimeout = scala.concurrent.duration.Duration(120, "s")
  test("sleeps while target/sleep exists") {
    if java.nio.file.Files.exists(java.nio.file.Paths.get("target/sleep")) then Thread.sleep(60000)
  }
EOF
# A `@main` of the test sources, which `run` must not count among app's products.
cat > "$b/app/test/app/Tool.scala" <<'EOF'
package app

@main def tool(): Unit = println("tool")
EOF
mkdir -p "$b/app/test-resources" "$b/lib"
echo hello > "$b/app/test-resources/greeting.txt"
extra_jar() { python3 -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1], "w"); z.writestr("extra.txt", sys.argv[2] + "\n"); z.close()' "$b/lib/extra.jar" "$1"; }
extra_jar one
echo 'lazy val app = project' > "$b/build.sbt"
echo 'sbt.version=2.0.8' > "$b/project/build.properties"
echo 'one two three' > "$b/labels.txt"
cat > "$b/gen.sh" <<'EOF'
mkdir -p "$2/app"
printf 'package app\n\nobject Labels:\n  val count: Int = %s\n' "$(wc -w < "$1" | tr -d ' ')" > "$2/app/Labels.scala.new"
cmp -s "$2/app/Labels.scala.new" "$2/app/Labels.scala" && rm "$2/app/Labels.scala.new" || mv "$2/app/Labels.scala.new" "$2/app/Labels.scala"
EOF
# The export, canonical; `$1` the compiler's version, `$2` app's maxInlines, `$3` a root only app's
# description names (teqExtraSources), when given; OUTPUT_VERSION the export's java.outputVersion,
# EXTRA_FRAMEWORK a test framework app's tests name besides munit's, JAVA_OPTIONS their JVM's one
# option, STAGE_DIRECTORY the directory of app's Docker stage.
# app also declares `app/more`, which does not exist at first. The main classes are the plugin's
# shape: app's `mainClasses` the sorted union of the declared class and the aliases' targets, its
# run block's `mainClass` the declared one; core, solo, pair and util have a block without one.
# pkg is one more. solo, pair and util are packaged too, with a stage block without `mainClass`
# and without `Main-Class` in their own jar's manifest, the plugin's shape for a build that leaves
# `Compile / mainClass` alone (an inference fixture: no plugin wrote a stage without the field
# before): their products decide (one, two, none). STAGE_NONE gives solo's block `mainClassNone`,
# the shape for a build that sets the key to None. OLD_SHAPE writes the export as a plugin before
# them did: no `mainClass` in the run block (the stage block kept its own, which such a plugin
# required), and no block for core.
write_export() {
  python3 - "$b" "$lib" "$lib3" "$1" "$2" "${3:-}" "${OUTPUT_VERSION:-17}" "$PWD/integrations/sbt/example/teq.lock" "${EXTRA_FRAMEWORK:-}" "${JAVA_OPTIONS:-}" "${STAGE_DIRECTORY:-app/target/docker/stage}" <<'PY'
import hashlib, sys
import importlib.util, os
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
b, lib, lib3, version, inlines, extra, output, fixture, framework, option, stage = sys.argv[1:]
m2 = lib[: lib.index("/maven2/") + len("/maven2/")]
# A jar of the coursier cache by its key in the table, its record the Maven layout's.
def maven_path(key):
    organization, name, version = key.split(":")[:3]
    return f"{organization.replace('.', '/')}/{name}/{version}/{name}-{version}{''.join('-' + c for c in key.split(':')[3:])}.jar"
def artifact(path):
    data = open(path, "rb").read()
    parts = path[len(m2):].split("/")
    return ":".join([".".join(parts[:-3]), parts[-3], parts[-2]]), f"maven-central {hashlib.sha1(data).hexdigest()} {len(data)}"
table = dict([artifact(lib3), artifact(lib)])
jars = list(table)
example = lock.parse(open(fixture, encoding="utf-8").read())
munit = [e for e in example["projects"]["jvmapp"]["configurations"]["test"]["classpath"] if isinstance(e, str) and e not in table]
table.update({key: example["jars"][key] for key in munit})
path_of = lambda key: (example["jars"][key].split(" ")[3:] or [maven_path(key)])[0]
missing = [maven_path(key) for key in munit if not os.path.isfile(m2 + path_of(key))]
if missing:
    sys.exit("missing from the coursier cache: " + ", ".join(missing))
flags = lambda n: {"ignoredScalacOptions": [], "kindProjector": False, "maxInlines": n, "strictEquality": False, "werror": False}
gen = "target/teq/app/compile/src_managed"
def description(p, sources):
    return {"cacheableState": [], "excludes": [], "hot": False, "keys": {}, "macroState": "ordered", "modulePerFile": [], "out": p + "/target/teq/out", "production": {"excludes": [], "sources": []}, "release": False, "sources": sources}
inputs = {f: hashlib.sha256(open(os.path.join(b, f), "rb").read()).hexdigest() for f in ["build.sbt", "project/build.properties"]}
lib_name = lambda key: "lib/" + key.split(":")[0] + "." + key.split(":")[1] + "-" + key.split(":")[2] + ".jar"
manifest = lambda p, main: {"Implementation-Title": p, "Implementation-Version": "1.0", **({"Main-Class": main} if main else {})}
product = lambda p: {"configuration": "compile", "project": p}
staged = [("app", "lib/app.app-1.0.jar"), ("core", "lib/core.core-1.0.jar"), ("extra", "lib/extra.jar")] + [(a, lib_name(a)) for a in jars]
old = bool(os.environ.get("OLD_SHAPE"))
def stage_of(p):
    paths = [(p, f"lib/{p}.{p}-1.0.jar"), ("core", "lib/core.core-1.0.jar")] + [(a, lib_name(a)) for a in jars]
    return {"directory": p + "/target/docker/stage", "exposedPorts": [], "kind": "docker", "name": p, **({"mainClassNone": True} if p == "solo" and os.environ.get("STAGE_NONE") else {}), "layers": {
        "2": [{"from": a, "to": "opt/docker/" + lib_name(a)} for a in jars],
        "4": [{"from": product(p), "manifest": manifest(p, None), "to": "opt/docker/" + paths[0][1]},
              {"from": product("core"), "manifest": manifest("core", None), "to": "opt/docker/lib/core.core-1.0.jar"},
              {"classpath": [path for _, path in paths], "script": "opt/docker/bin/" + p}]}}
def runnable(p, staged=False):
    return {"base": p, "platform": "jvm", "scalaVersion": "3.8.4", "description": description(p, ["core/src", p + "/src"]),
            "configurations": {"compile": {"classpath": [product("core")] + jars, "flags": flags(80), "generators": [], "mainClasses": [], "resources": [], "sources": [p + "/src"]},
                               "runtime": {"classpath": [product(p), product("core")] + jars, "resources": [], "sources": []}},
            "run": {"aliases": {}, "baseDirectory": ".", "envVars": {}, "javaOptions": []}, **({"stage": stage_of(p)} if staged else {})}
export = {
    "teq": version,
    "format": 1,
    "binaries": {},
    "inputs": {"files": inputs, "sha256": hashlib.sha256("".join(f"{k}\0{v}\n" for k, v in sorted(inputs.items())).encode()).hexdigest()},
    "java": {"outputVersion": int(output)},
    "projects": {
        "core": {"base": "core", "platform": "jvm", "scalaVersion": "3.8.4", "description": description("core", ["core/src"]),
                 "configurations": {"compile": {"classpath": jars, "flags": flags(80), "generators": [], "mainClasses": [], "resources": ["core/resources"], "sources": ["core/src"]},
                                    "runtime": {"classpath": [product("core")] + jars, "resources": [], "sources": []}},
                 **({} if old else {"run": {"aliases": {}, "baseDirectory": ".", "envVars": {}, "javaOptions": []}})},
        "app": {"base": "app", "platform": "jvm", "scalaVersion": "3.8.4", "description": description("app", ["core/src", "app/src", "app/more", gen] + ([extra] if extra else [])),
                "configurations": {
                    "compile": {"classpath": [{"configuration": "compile", "project": "core"}] + jars, "flags": flags(int(inlines)), "mainClasses": ["app.Exit", "app.Main", "app.Sleep", "app.Stubborn"], "resources": ["app/resources"], "sources": ["app/src", "app/more", gen],
                                "generators": [{"cwd": ".", "inputs": ["labels.txt"], "kind": "command", "outputs": [gen], "run": ["sh", "gen.sh", "labels.txt", gen]}]},
                    "test": {"classpath": [{"configuration": "compile", "project": "app"}, {"configuration": "compile", "project": "core"}, {"file": "lib/extra.jar"}] + jars + munit, "flags": flags(int(inlines)), "resources": ["app/test-resources"], "sources": ["app/test"],
                             "baseDirectory": ".", "envVars": {}, "exclude": ["app.Excluded"], "fork": False, "javaOptions": [option] if option else [],
                             "frameworks": [{"arguments": [], "class": "munit.Framework"}] + ([{"arguments": [], "class": framework}] if framework else [])},
                    "runtime": {"classpath": [product("app"), product("core"), {"file": "lib/extra.jar"}] + jars, "resources": [], "sources": []}},
                "run": {"aliases": {"exit": "app.Exit", "main": "app.Main", "sleep": "app.Sleep", "stubborn": "app.Stubborn"}, "baseDirectory": "app", "envVars": {"RUN_VAR": "var"}, "javaOptions": ["-Drun.option=opt"], **({} if old else {"mainClass": "app.Main"})},
                "stage": {"directory": stage, "exposedPorts": [9000], "kind": "docker", "mainClass": "app.Main", "name": "app", "layers": {
                    "2": [{"from": {"file": "lib/extra.jar"}, "to": "opt/docker/lib/extra.jar"}] + [{"from": a, "to": "opt/docker/" + lib_name(a)} for a in jars],
                    "4": [{"from": product("app"), "manifest": manifest("app", "app.Main"), "to": "opt/docker/lib/app.app-1.0.jar"},
                          {"from": product("core"), "manifest": manifest("core", None), "to": "opt/docker/lib/core.core-1.0.jar"},
                          {"classpath": [path for _, path in staged], "script": "opt/docker/bin/app"}]}}},
        "solo": runnable("solo", staged=True),
        "pair": runnable("pair", staged=True),
        "util": runnable("util", staged=True),
        "pkg": runnable("pkg"),
    },
    "jars": table,
    "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}],
}
open(os.path.join(b, "teq.lock"), "w", encoding="utf-8").write(lock.canonical(export))
PY
}
write_export 0.1.7-check.1 80 || { echo "FAIL: munit's jars are not in the coursier cache (incomplete validation)"; exit 1; }
task() { (cd "$b" && timeout 150 "$TEQ" "$@" 2>&1); }

out=$(task)
case "$out" in *"projects of $b"*"app (jvm), core (jvm)"*) pass "without a verb, the verbs and the projects" ;; *) fail "the verbs and projects: $out" ;; esac
out=$(task compile app)
if [ $? = 0 ] && [[ "$out" == *"app/test: "*" classes, "*"the first build"* ]] && [ -f "$b/target/teq/app/test/classes/app/Main.class" ] && [ -f "$b/target/teq/app/test/classes/core/Core\$.class" ] && [ -f "$b/target/teq/app/test/classes/app/MainTest.class" ]; then
  pass "the first compile: the resident of app/test writes the closure's class files, core's among them"
else
  fail "the first compile: $out"
fi
# The resident starts as `teq @<file>` (docs/TARGETS.md, "Argument files"): its class path, past
# Windows' cap for a few hundred jars, is in target/teq/app/test/resident.args, not on its command line.
args_file=$b/target/teq/app/test/resident.args
cmdline_of() { if [ -r "/proc/$1/cmdline" ]; then tr '\0' '\n' < "/proc/$1/cmdline"; else ps -ww -o args= -p "$1"; fi; }
resident_pids() { pgrep -f "@$args_file" || true; }
# The daemon's task.log names the file and what it holds; under load the line can trail the
# resident's start, so it is waited for, ten seconds at most.
logged() { grep -qF "@$args_file, which holds compiler watch " "$b/target/teq/task.log" 2> /dev/null; }
for _ in $(seq 100); do logged && break; sleep 0.1; done
pid=$(resident_pids | head -1)
if [ -n "$pid" ] && [ "$(head -2 "$args_file" | tr '\n' ' ')" = "compiler watch " ] && grep -A1 -x -- --classpath "$args_file" | grep -qF "$lib3" && cmdline_of "$pid" | grep -qF -- "@$args_file" && ! cmdline_of "$pid" | grep -q -- --classpath && logged; then
  pass "the resident starts as teq @target/teq/app/test/resident.args, which holds its arguments, its class path among them; its command line holds no --classpath"
else
  fail "the resident's argument file ($pid, $(logged && echo "task.log names it" || echo "no task.log line")): $(head -c 300 "$args_file" 2> /dev/null) / $( [ -n "$pid" ] && cmdline_of "$pid" | head -c 300)"
fi
first=$(last_daemon)
out=$(task compile app)
if [ $? = 0 ] && [[ "$out" == *"0 written"* ]] && [ "$(last_daemon)" = "$first" ]; then pass "a warm compile writes nothing, through the same daemon"; else fail "a warm compile: $out"; fi
out=$(task compile core)
if [[ "$out" == "app/test: "* ]]; then pass "core is compiled by the resident whose closure holds it"; else fail "core's compile: $out"; fi
refused=$(python3 - "$b/target/teq/daemon.json" <<'PY'
import json, socket, sys
info = json.load(open(sys.argv[1]))
s = socket.create_connection(("127.0.0.1", info["port"]), timeout=10)
s.sendall(b'{"token": "not the token", "verb": "stop"}\n')
print(s.makefile().read())
PY
)
if [[ "$refused" == *"refused: the request does not carry the daemon's token"* ]] && alive "$first" && [ "$(stat -c %a "$b/target/teq/daemon.json" 2> /dev/null || stat -f %Lp "$b/target/teq/daemon.json")" = 600 ]; then
  pass "a request without the daemon's token is refused; daemon.json is its owner's alone"
else
  fail "the token: $refused"
fi
out=$(printf 'quit\n' | task watch app | head -1)
if [[ "$out" == '{"ok":true'* ]]; then pass "watch: the project's resident on stdin and stdout, its first answer"; else fail "watch: $out"; fi
# `teq watch` becomes the same resident through a file of its own, named by what it holds: the
# resident's arguments and its options, here from an argument file of the client's, where a literal
# `@absent` is an argument and no file, and stays one in the resident's file; the daemon's file is
# left as it was.
before=$(cat "$args_file")
printf '%s\n' watch app --threads 1 --exclude @absent > "$work/watch.call"
mkfifo "$work/watch.in"
(cd "$b" && timeout 150 "$TEQ" "@$work/watch.call" < "$work/watch.in" > "$work/watch.out" 2>&1) &
watching=$!
exec 7> "$work/watch.in"
for _ in $(seq 300); do [ -s "$work/watch.out" ] && break; sleep 0.1; done
watch_pid=$(pgrep -f "@$b/target/teq/app/test/watch-" | head -1)
watch_file=$(cmdline_of "$watch_pid" | grep -o "$b/target/teq/app/test/watch-[0-9a-f]*\.args" | head -1)
line=$(cmdline_of "$watch_pid")
echo quit >&7
exec 7>&-
wait $watching
if [[ "$(head -1 "$work/watch.out")" == '{"ok":true'* ]] && [ -n "$watch_file" ] && ! grep -q -- --classpath <<< "$line" && [ "$(cat "$watch_file")" = "$(printf '%s\n' "$before" --threads 1 --exclude @absent)" ] && [ "$(cat "$args_file")" = "$before" ]; then
  pass "watch starts as teq @target/teq/app/test/watch-<digest>.args, the resident's arguments and its options, a literal @absent among them; the daemon's file is left as it was"
else
  fail "watch's argument file ($watch_pid, $watch_file): $line / $(tail -4 "$watch_file" 2> /dev/null | tr '\n' ' ') / $(head -c 300 "$work/watch.out")"
fi
cp "$b/app/src/app/Main.scala" "$work/Main.scala"
echo '  val broken: Int = "no"' >> "$b/app/src/app/Main.scala"
out=$(task compile app)
code=$?
if [ $code = 1 ] && [[ "$out" == *"app/src/app/Main.scala:5:"*"error: "* ]] && [[ "$out" == *"app/test: 1 error"* ]]; then pass "an error fails the compile with its diagnostic"; else fail "the error ($code): $out"; fi
cp "$work/Main.scala" "$b/app/src/app/Main.scala"
out=$(task compile app)
if [ $? = 0 ]; then pass "the fix passes it"; else fail "the fix: $out"; fi
mkdir -p "$b/app/more"
echo 'object More:
  val wrong: Int = "more"' > "$b/app/more/More.scala"
out=$(task compile app)
code=$?
rm -rf "$b/app/more"
if [ $code = 1 ] && [[ "$out" == *"app/more/More.scala:2:"*"error: "* ]] && task compile app > /dev/null; then pass "a declared root created after the resident started is typed by the next compile, and its removal too"; else fail "a root created later ($code): $out"; fi
write_export 0.1.7-check.1 80 app/extra
mkdir -p "$b/app/extra"
echo 'object Extra:
  val wrong: Int = "extra"' > "$b/app/extra/Extra.scala"
out=$(task compile app)
code=$?
rm -rf "$b/app/extra"
write_export 0.1.7-check.1 80
if [ $code = 1 ] && [[ "$out" == *"app/extra/Extra.scala:2:"*"error: "* ]]; then pass "a root only the project's description names (teqExtraSources) is among the resident's sources"; else fail "the description's extra root ($code): $out"; fi
echo 'one two three four' > "$b/labels.txt"
task compile app > /dev/null
if grep -q 'count: Int = 4' "$b/target/teq/app/compile/src_managed/app/Labels.scala"; then pass "the generator runs again when its input changes"; else fail "the generator: $(cat "$b/target/teq/app/compile/src_managed/app/Labels.scala")"; fi

write_export 0.1.7-check.1 81
out=$(task compile app)
if [ $? = 0 ] && [ "$(last_daemon)" = "$first" ] && grep -q '^stopping app/test: the export changed its command line' "$b/target/teq/task.log" && [[ "$out" == *"the first build"* ]]; then
  pass "an export changed in a project's flags is read again by the same daemon, the project's resident started anew"
else
  fail "the export read again: $out"
fi
write_export 0.1.7-check.9 81
out=$(task compile app)
code=$?
second=$(last_daemon)
sleep 1
if [ $code = 0 ] && [ -n "$second" ] && [ "$second" != "$first" ] && ! alive "$first" && alive "$second"; then
  pass "an export pinning another compiler has the next client start a new daemon, the old one ended"
else
  fail "the branch switch: daemons $first and $second: $out"
fi

# Without a client: an export pinning another compiler, then target/teq removed, each end the
# daemon within its watch's period.
gone_within() { for _ in $(seq 40); do alive "$1" || return 0; sleep 0.25; done; return 1; }
write_export 0.1.7-check.8 81
if gone_within "$second"; then pass "the daemon watches the export: another compiler pinned, it ends with no client asking"; else fail "the daemon outlived a branch switch"; fi
task compile app > /dev/null
third=$(last_daemon)
cp "$b/target/teq/task.log" "$work/task.log"
rm -rf "$b/target/teq"
if gone_within "$third"; then pass "target/teq removed: the daemon ends rather than outlive its daemon.json"; else fail "the daemon outlived target/teq"; fi
mkdir -p "$b/target/teq" && cp "$work/task.log" "$b/target/teq/task.log"
write_export 0.1.7-check.9 81
echo '// changed' >> "$b/build.sbt"
out=$(task compile app)
if [ $? = 0 ] && [[ "$out" == *"warning: teq.lock was written before build.sbt changed; run sbt teqExportAll"* ]]; then pass "a changed build definition warns, naming the file"; else fail "the stale warning: $out"; fi
out=$(task --strict compile app)
if [ $? = 2 ] && [[ "$out" == *"refusing a stale export"* ]]; then pass "--strict refuses a stale export"; else fail "--strict: $out"; fi
write_export 0.1.7-check.9 81
# build.sbt checked out with CRLF where the export read LF (Git for Windows' default), and the
# reverse: stale all the same, --strict refusing, with a note naming the file and the remedy in the
# place of the command (an export there would record those line ends); an edit beside the line
# ends gets the command and no note.
crlf() { python3 -c 'import sys; p = sys.argv[1]; t = open(p, "rb").read().replace(b"\r\n", b"\n"); open(p, "wb").write(t.replace(b"\n", b"\r\n") if sys.argv[2] == "crlf" else t)' "$1" "$2"; }
note="teq: note: build.sbt differs from teq.lock's record by line ends alone, LF against CRLF: .gitattributes keeps the build's files LF in every checkout with *.sbt text eol=lf, project/**/*.scala text eol=lf and project/build.properties text eol=lf: add them, delete the files and check them out again, then export again"
crlf "$b/build.sbt" crlf
out=$(task --strict compile app)
if [ $? = 2 ] && [[ "$out" == *"refusing a stale export: teq.lock was written before build.sbt changed"* ]] && [[ "$out" != *"sbt teqExportAll"* ]] && [[ "$out" == *"$note"* ]]; then
  pass "a build file checked out with CRLF where the export read LF is stale, --strict refusing it, with the note on its line ends and the remedy"
else
  fail "the CRLF checkout: $out"
fi
printf '// edited\r\n' >> "$b/build.sbt"
out=$(task --strict compile app)
if [ $? = 2 ] && [[ "$out" == *"build.sbt changed; run sbt teqExportAll"* ]] && [[ "$out" != *"line ends alone"* ]]; then pass "an edit beside the line ends: stale, the command and no note"; else fail "the edit beside the line ends: $out"; fi
crlf "$b/build.sbt" lf
sed -i '$d' "$b/build.sbt"
crlf "$b/build.sbt" crlf
write_export 0.1.7-check.9 81
crlf "$b/build.sbt" lf
out=$(task compile app)
if [ $? = 0 ] && [[ "$out" == *"warning: teq.lock was written before build.sbt changed"* ]] && [[ "$out" != *"sbt teqExportAll"* ]] && [[ "$out" == *"$note"* ]]; then
  pass "an export of a CRLF checkout read where the file is LF: the warning, with the note"
else
  fail "the export of the CRLF checkout: $out"
fi
write_export 0.1.7-check.9 81

last=$(last_daemon)
out=$(task stop)
sleep 1
if [[ "$out" == *"stopped the daemon"* ]] && ! alive "$last" && [ ! -f "$b/target/teq/daemon.json" ]; then pass "stop ends the daemon and its residents"; else fail "stop: $out"; fi
out=$(task stop)
if [[ "$out" == *"no daemon runs"* ]]; then pass "stop without a daemon says so"; else fail "the second stop: $out"; fi
before=$(daemons)
(task compile core > "$work/one.out") &
(task compile core > "$work/two.out") &
wait
if [ $(( $(daemons) - before )) = 1 ] && grep -q 'core/compile' "$work/one.out" && grep -q 'core/compile' "$work/two.out"; then pass "two clients starting at once start one daemon"; else fail "two clients: $(cat "$work/one.out" "$work/two.out") ($(( $(daemons) - before )) daemons)"; fi
task stop > /dev/null
# Where the system gives no mapping for the type store's overlays (an address-space limit here,
# which the daemon and its resident inherit), one worker types the resident's build and its answer
# says so, which the client prints: the resident's stderr is kept for its end.
out=$(ulimit -v 8000000 && TEQ_THREADS=2 task compile core)
if [ $? = 0 ] && [[ "$out" == *"teq: core/compile: no memory mapping for the type store's overlays; one worker types the build"* ]]; then pass "a refused mapping's note reaches the client"; else fail "the refused mapping's note: $out"; fi
task stop > /dev/null

# `test`: app's munit suites on the daemon's warm runner.
runner_pid() { pgrep -f "$b/target/teq/app/test/runner.args" | head -1; }
runners_started() { grep -c '^starting the test runner of app/test' "$b/target/teq/task.log"; }
out=$(task test app --list)
expected='Test app.Excluded : subclass(false, munit.Suite)
Test app.MainTest : subclass(false, munit.Suite)
Test app.Resources : subclass(false, munit.Suite)
Test app.Sleeper : subclass(false, munit.Suite)
Test app.ViaBase : subclass(false, munit.Suite)'
if [ "$out" = "$expected" ]; then pass "test --list: the suites as sbt's definedTests, a base's subclass among them, the abstract base and a plain class not"; else fail "test --list: $out"; fi
started=$(runners_started)
out=$(task test app)
if [ $? = 0 ] && [[ "$out" == *"app/test: 4 suites, 5 passed, 0 failed in "* ]] && [[ "$out" == *"app.MainTest"* ]] && [[ "$out" != *"excluded"* ]]; then pass "test runs the suites but the excluded one on the runner, the framework's output then the summary"; else fail "test: $out"; fi
pid=$(runner_pid)
cp "$b/app/test/app/MainTest.scala" "$work/MainTest.scala"
sed -i.bak 's/core.Core.answer, 42/core.Core.answer, 41/' "$b/app/test/app/MainTest.scala"
out=$(task test app app.MainTest)
code=$?
cp "$work/MainTest.scala" "$b/app/test/app/MainTest.scala"
again=$(task test app app.MainTest)
if [ $code = 1 ] && [[ "$out" == *"[fail] app.MainTest.the answer: "* ]] && [[ "$out" == *"app/test: 1 suite, 0 passed, 1 failed"* ]] && [[ "$again" == *"1 suite, 1 passed, 0 failed"* ]] && [ "$(runner_pid)" = "$pid" ] && [ "$(runners_started)" = "$started" ]; then
  pass "an edit to a suite between two runs on the same warm runner changes the outcome, and its revert"
else
  fail "the edit between runs ($code): $out // $again"
fi
echo bye > "$b/app/test-resources/greeting.txt"
out=$(task test app '*Resources')
code=$?
echo hello > "$b/app/test-resources/greeting.txt"
if [ $code = 1 ] && [[ "$out" == *"1 suite, 1 passed, 1 failed"* ]] && [ "$(runner_pid)" = "$pid" ]; then pass "a resource of the test configuration is read afresh by the next run"; else fail "the resource: $out"; fi
out=$(task test app '*Via*' -app.MainTest)
if [[ "$out" == *"app/test: 1 suite, 1 passed"* ]] && [[ "$out" == *"app.ViaBase"* ]]; then pass "testOnly's patterns select the suites"; else fail "the patterns: $out"; fi
task test app > /dev/null
out=$(task test app --changed)
if [[ "$out" == "app/test: no suite depends on what changed" ]]; then pass "--changed after a run of every suite, nothing written since: no suite"; else fail "--changed with nothing written: $out"; fi
cp "$b/core/src/core/Core.scala" "$work/Core.scala"
sed -i.bak 's/def answer: Int = 42/def answer: Int = "42".toInt/' "$b/core/src/core/Core.scala"
out=$(task test app --changed)
if [[ "$out" == *"app/test: 1 suite, 1 passed, 0 failed"* ]] && [[ "$out" == *"app.MainTest"* ]]; then pass "--changed runs the suite whose classes depend on the class an edit changed, alone"; else fail "--changed after an edit: $out"; fi
cp "$work/Core.scala" "$b/core/src/core/Core.scala"
task compile app > /dev/null
out=$(task test app --changed)
if [[ "$out" == *"app/test: 1 suite, 1 passed, 0 failed"* ]]; then pass "--changed counts what a compile wrote since the last run of every suite"; else fail "--changed after a compile: $out"; fi
extra_jar two
out=$(task test app '*Resources')
if [[ "$out" == *"1 suite, 1 passed, 1 failed"* ]] && [ "$(runner_pid)" != "$pid" ] && grep -q '^stopping the test runner of app/test: what it was started with changed' "$b/target/teq/task.log"; then
  pass "a jar of the class path rewritten in place starts the runner anew"
else
  fail "the jar rewritten: $out"
fi
extra_jar one
task test app '*Resources' > /dev/null
kill "$(runner_pid)"
out=$(task test app app.MainTest)
if [ $? = 0 ] && grep -q '^stopping the test runner of app/test: it ended' "$b/target/teq/task.log"; then pass "a runner that died is started again by the next test"; else fail "the runner died: $out"; fi
touch "$b/target/sleep"
(cd "$b" && exec timeout 150 "$TEQ" test app '*Sleeper' > "$work/sleeper.out" 2>&1) &
client=$!
for _ in $(seq 100); do grep -q 'app.Sleeper' "$work/sleeper.out" 2> /dev/null && break; sleep 0.1; done
{ kill "$client"; wait "$client"; } 2> /dev/null
rm -f "$b/target/sleep"
sleep 0.5
out=$(task test app app.MainTest)
if [ $? = 0 ] && grep -q '^killing the test runner of app/test: its client left during the run' "$b/target/teq/task.log"; then pass "a client that leaves during a run has the runner killed, and the next test starts another"; else fail "the client that left: $out"; fi
OUTPUT_VERSION=99 write_export 0.1.7-check.1 80
out=$(task test app)
code=$?
write_export 0.1.7-check.1 80
if [ $code = 2 ] && [[ "$out" == *"older than the 99 the export's classes are for (java.outputVersion)"* ]]; then pass "a JVM older than the export's java.outputVersion is refused"; else fail "the old JVM ($code): $out"; fi
EXTRA_FRAMEWORK=munit.NoSuchFramework write_export 0.1.7-check.1 80
out=$(task test app)
code=$?
again=$(task test app)
code2=$?
write_export 0.1.7-check.1 80
if [ $code = 1 ] && [ $code2 = 1 ] && [[ "$again" == *"the test framework munit.NoSuchFramework does not load"* ]] && [[ "$again" == *"4 suites, 5 passed, 0 failed"* ]]; then
  pass "a framework the export names that does not load fails every test, the other frameworks' suites run"
else
  fail "the framework that does not load ($code, $code2): $again"
fi
JAVA_OPTIONS=-agentlib:jdwp=transport=dt_socket,server=y,suspend=y,address=127.0.0.1:0 write_export 0.1.7-check.1 80
(cd "$b" && exec timeout 150 "$TEQ" test app > "$work/suspended.out" 2>&1) &
client=$!
for _ in $(seq 100); do pgrep -f "$b/target/teq/app/test/runner.args" > /dev/null && break; sleep 0.1; done
sleep 1
{ kill "$client"; wait "$client"; } 2> /dev/null
write_export 0.1.7-check.1 80
out=$(task test app app.MainTest)
if [ $? = 0 ] && [ "$(grep -c '^killing the test runner of app/test' "$b/target/teq/task.log")" = 2 ]; then pass "a client that leaves while its runner's JVM has not started has it killed, and the next test runs"; else fail "the runner killed at its start: $out"; fi
task test app > /dev/null
mkdir -p "$b/app/more"
echo 'object Added { val n: Int = "bad" }' > "$b/app/more/Added.scala"
out=$(task test app --changed)
code=$?
echo 'object Added { val n: Int = 1 }' > "$b/app/more/Added.scala"
again=$(task test app --changed)
rm -rf "$b/app/more"
if [ $code = 1 ] && [[ "$again" == *"every suite, its resident started anew since the last run"* ]] && [[ "$again" == *"4 suites, 5 passed"* ]]; then pass "--changed after a resident started anew runs every suite, its first build failing or not"; else fail "--changed after a restart ($code): $again"; fi
task test app > /dev/null
pid=$(runner_pid)
task stop > /dev/null
sleep 1
if [ -n "$pid" ] && ! alive "$pid"; then pass "stop ends the test runner"; else fail "stop left the runner $pid"; fi

# `run`: app's main classes on a JVM of their own, through the daemon's compile.
run_app() { (cd "$b" && timeout 150 "$TEQ" run app "$@" 2> "$work/run.err"); }
out=$(run_app main)
if [ $? = 0 ] && [[ "$out" =~ ^[0-9]+$ ]] && grep -q '^app/test: ' "$work/run.err"; then pass "run: an alias's main class, its output alone on stdout, the compile's summary on stderr"; else fail "run main: $out // $(cat "$work/run.err")"; fi
main_out=$out
out=$(run_app exit -- 3)
code=$?
if [ $code = 3 ] && [ "$out" = "opt var app packaged 3" ]; then pass "run: the run block's options, variables and directory, a product's resource on the classpath, the arguments after --, the program's exit code"; else fail "run exit ($code): $out // $(cat "$work/run.err")"; fi
out=$(run_app app.Exit)
if [ $? = 0 ] && [ "$out" = "opt var app packaged " ]; then pass "run: a main class by its name"; else fail "run app.Exit: $out // $(cat "$work/run.err")"; fi
out=$(run_app nope)
code=$?
err=$(cat "$work/run.err")
out2=$(run_app app.Nope)
code2=$?
err2=$(cat "$work/run.err")
if [ $code = 2 ] && [[ "$err" == *"nope is neither an alias of app nor a main class (aliases: exit (app.Exit), main (app.Main), sleep (app.Sleep), stubborn (app.Stubborn); main classes: app.Exit, app.Main, app.Sleep, app.Stubborn); arguments for the program follow --"* ]] && [[ "$err" != *"app/test: "* ]] &&
  [ $code2 = 2 ] && [[ "$err2" == *"app.Nope is no class of app's build"* ]]; then
  pass "run refuses a word that is neither an alias nor a main class, with the -- hint, and a class the build lacks, naming what there is"
else
  fail "run's refusals ($code, $code2): $err // $err2"
fi
# Without a word (or with `--` right after the project) the main class is implied as sbt's `run`
# implies it: the declared one (app's, among its four product mains), else the single one among
# the project's own products (core's CoreMain, inherited through its trait; solo's, through core's
# trait from another module), else a refusal naming them after the build (pair's own `main` and
# `@main` method) or none (util's).
run_in() { local p=$1; shift; (cd "$b" && timeout 150 "$TEQ" run "$p" "$@" 2> "$work/run.err"); }
out=$(run_app -- 3)
code=$?
out2=$(run_app)
code2=$?
if [ $code = 0 ] && [[ "$out" =~ ^[0-9]+\ 3$ ]] && [ $code2 = 0 ] && [[ "$out2" =~ ^[0-9]+$ ]]; then pass "run: without a word, the declared main class, the arguments after -- reaching it, and without arguments"; else fail "run app -- 3 / run app ($code, $code2): $out / $out2 // $(cat "$work/run.err")"; fi
out=$(run_in core -- 1 2)
if [ $? = 0 ] && [ "$out" = "core 1 2" ]; then pass "run: a project declaring no main class runs the single one among its own products, inherited through its trait"; else fail "run core: $out // $(cat "$work/run.err")"; fi
# What follows -- is the program's: an @ there is no argument file (docs/TARGETS.md, "Argument
# files"), a file of that name existing (labels.txt) or not.
out=$(run_in core -- @absent @labels.txt)
if [ $? = 0 ] && [ "$out" = "core @absent @labels.txt" ]; then pass "run: an @ after -- reaches the program as it is, a file of that name or none"; else fail "run core -- @absent @labels.txt: $out // $(cat "$work/run.err")"; fi
out=$(run_in pkg -- p q)
if [ $? = 0 ] && [ "$out" = "hi p q" ]; then pass "run: a @main method of a package object is the single product main, its block answered under the file"; else fail "run pkg: $out // $(cat "$work/run.err")"; fi
out=$(run_in solo -- x y)
code=$?
out2=$(run_in solo)
code2=$?
if [ $code = 0 ] && [ "$out" = "solo x y" ] && [ $code2 = 0 ] && [ "$out2" = "solo" ] && grep -q '^solo/compile: ' "$work/run.err"; then pass "run: the single product main inherited through another module's trait, with and without arguments"; else fail "run solo ($code, $code2): $out / $out2 // $(cat "$work/run.err")"; fi
out=$(run_in pair --)
code=$?
err=$(cat "$work/run.err")
out2=$(run_in util)
code2=$?
err2=$(cat "$work/run.err")
if [ $code = 2 ] && [ -z "$out" ] && [[ "$err" == *"pair declares no main class and its products hold 2, which sbt's run would ask to choose among: pair.A, pair.b; name one"* ]] && [[ "$err" == *"pair/compile: "* ]] &&
  [ $code2 = 2 ] && [[ "$err2" == *"no main class in util's products"* ]]; then
  pass "run refuses a project with two main classes among its products and none declared, naming both after the build, and one whose products hold none (an object's @main, whose class is not written, not counted)"
else
  fail "run's implied refusals ($code, $code2): $err // $err2"
fi
# The old shape, an export of a plugin before the block's mainClass and the block for every JVM
# project: app, four product mains and app.Main declared, is refused naming its own four alone (not
# core's CoreMain, not the test sources' app.tool); core, without a block, as predating it.
OLD_SHAPE=1 write_export 0.1.7-check.1 80
out=$(run_app)
code=$?
err=$(cat "$work/run.err")
out3=$( (cd "$b" && timeout 30 "$TEQ" run core x 2>&1) )
code3=$?
# The stage reads the stage block's own `mainClass`, which such a plugin always wrote: app.Main
# is staged where `run` refuses.
out4=$(task stage app)
code4=$?
write_export 0.1.7-check.1 80
if [ $code = 2 ] && [[ "$err" == *"app declares no main class and its products hold 4, which sbt's run would ask to choose among: app.Exit, app.Main, app.Sleep, app.Stubborn; name one"* ]] && [ $code3 = 2 ] && [[ "$out3" == *"core has no run block: its export predates the block"*"(app, pair, pkg, solo, util have one)"* ]] &&
  [ $code4 = 0 ] && grep -q "'app.Main'" "$b/app/target/docker/stage/4/opt/docker/bin/app"; then
  pass "run: an export of an older plugin declares none, so app's own products decide, core's and the test sources' mains left out; a project without a block is refused as predating it; the stage block's own class is staged"
else
  fail "run over the old shape ($code, $code3, stage $code4): $err // $out3 // $out4"
fi
(cd "$b" && exec timeout 150 "$TEQ" run app sleep > "$work/sleep.out" 2>&1) &
client=$!
for _ in $(seq 100); do grep -q '^sleeping' "$work/sleep.out" 2> /dev/null && break; sleep 0.1; done
java_pid=$(pgrep -f "$b/target/teq/app/runtime/run.args" | head -1)
kill -INT "$client"
wait "$client"
code=$?
sleep 0.5
if [ $code = 130 ] && [ -n "$java_pid" ] && ! alive "$java_pid"; then pass "run passes SIGINT on to the program's group and exits as it does"; else fail "run's SIGINT ($code, java $java_pid): $(cat "$work/sleep.out")"; fi
(cd "$b" && exec timeout 150 "$TEQ" run app stubborn > "$work/stubborn.out" 2>&1) &
client=$!
for _ in $(seq 100); do grep -q '^stubborn' "$work/stubborn.out" 2> /dev/null && break; sleep 0.1; done
java_pid=$(pgrep -f "$b/target/teq/app/runtime/run.args" | head -1)
started=$(date +%s)
kill -TERM "$client"
wait "$client"
code=$?
took=$(( $(date +%s) - started ))
sleep 0.5
if [ $code = 143 ] && [ "$took" -le 8 ] && [ -n "$java_pid" ] && ! alive "$java_pid"; then pass "run gives a program whose shutdown hangs 5 s after a signal passed on, then kills it"; else fail "run's bound ($code in $took s, java $java_pid): $(cat "$work/stubborn.out")"; fi
OUTPUT_VERSION=99 write_export 0.1.7-check.1 80
out=$( (cd "$b" && timeout 30 "$TEQ" run app main 2>&1) )
code=$?
write_export 0.1.7-check.1 80
if [ $code = 2 ] && [[ "$out" == *"older than the 99 the export's classes are for"* ]]; then pass "run refuses a JVM older than the export's classes"; else fail "run's old JVM ($code): $out"; fi

# `stage`: app's Docker stage from the export's mappings.
stage_dir=$b/app/target/docker/stage
entries() { python3 -c 'import sys, zipfile; print("\n".join(i.filename for i in zipfile.ZipFile(sys.argv[1]).infolist()))' "$1"; }
manifest() { python3 -c 'import sys, zipfile; print(zipfile.ZipFile(sys.argv[1]).read("META-INF/MANIFEST.MF").decode())' "$1"; }
out=$(task stage app)
app_jar=$stage_dir/4/opt/docker/lib/app.app-1.0.jar
core_jar=$stage_dir/4/opt/docker/lib/core.core-1.0.jar
if [ $? = 0 ] && [[ "$out" == *"app: staged 6 files in layers 2, 4 under app/target/docker/stage"* ]] &&
  cmp -s "$lib" "$stage_dir/2/opt/docker/lib/org.scala-lang.scala-library-3.8.4.jar" && cmp -s "$lib3" "$stage_dir/2/opt/docker/lib/org.scala-lang.scala3-library_3-3.8.4.jar" &&
  cmp -s "$b/lib/extra.jar" "$stage_dir/2/opt/docker/lib/extra.jar" && [ ! -L "$stage_dir/2/opt/docker/lib/extra.jar" ] && [ -x "$stage_dir/4/opt/docker/bin/app" ] &&
  grep -q "'app.Main'" "$stage_dir/4/opt/docker/bin/app"; then
  pass "stage: the artifacts and the repository's jar copied into their layers, the script executable and running the block's declared class over app's four product mains"
else
  fail "stage: $out"
fi
app_entries=$(entries "$app_jar")
core_entries=$(entries "$core_jar")
if [ "$(echo "$app_entries" | head -1)" = META-INF/MANIFEST.MF ] && echo "$app_entries" | grep -qx 'app/Main.class' && echo "$app_entries" | grep -qx 'app/Labels.class' && echo "$app_entries" | grep -qx 'app.txt' && echo "$app_entries" | grep -qx 'assets/message.txt' &&
  ! echo "$app_entries" | grep -q '^core/\|MainTest\|core.txt' && echo "$core_entries" | grep -qx 'core/Core.class' && echo "$core_entries" | grep -qx 'core.txt' && ! echo "$core_entries" | grep -q '^app/' &&
  manifest "$app_jar" | grep -q '^Main-Class: app.Main' && ! manifest "$core_jar" | grep -q 'Main-Class'; then
  pass "stage: each product's jar holds the class files of its own sources (a generated one's too) and its resources (through a linked directory too), no test class, the manifest's attributes"
else
  fail "stage's jars: $app_entries // $core_entries"
fi
rm -rf "$work/image" && mkdir -p "$work/image" && cp -R "$stage_dir/2/opt/docker/." "$stage_dir/4/opt/docker/." "$work/image/"
out=$(cd "$work" && timeout 60 sh image/bin/app 2>&1)
if [ "$out" = "$main_out" ]; then pass "stage: the start script runs the main class over the staged jars, from any directory"; else fail "the staged script: $out (run gave $main_out)"; fi
# A jar written again is renamed into place: another inode.
inode() { python3 -c 'import os, sys; print(os.stat(sys.argv[1]).st_ino)' "$1"; }
stamp=$(inode "$app_jar")
touch "$stage_dir/2/opt/docker/lib/stale.jar"
mkdir -p "$stage_dir/9/opt"
out=$(task stage app)
if [ $? = 0 ] && [ ! -e "$stage_dir/2/opt/docker/lib/stale.jar" ] && [ ! -e "$stage_dir/9" ] && [ "$(inode "$app_jar")" = "$stamp" ]; then
  pass "stage again: what the mappings no longer name removed, a jar whose bytes are the same left alone"
else
  fail "the second stage: $out"
fi
echo edited > "$b/core/resources/core.txt"
task stage app > /dev/null
read_entry() { python3 -c 'import sys, zipfile; print(zipfile.ZipFile(sys.argv[1]).read(sys.argv[2]).decode().strip())' "$1" "$2"; }
edited=$(read_entry "$core_jar" core.txt)
echo core > "$b/core/resources/core.txt"
if [ "$edited" = edited ] && [ "$(inode "$app_jar")" = "$stamp" ]; then pass "stage after a resource's edit: its product's jar written anew, the other left alone"; else fail "the edited resource: $edited"; fi
out=$(task stage core)
code=$?
STAGE_DIRECTORY=app write_export 0.1.7-check.1 80
out2=$(task stage app)
code2=$?
write_export 0.1.7-check.1 80
if [ $code = 2 ] && [[ "$out" == *"core has no stage block"*"(app, pair, solo, util have one)"* ]] && [ $code2 = 1 ] && [[ "$out2" == *"the stage's directory app is no directory under a target directory of the build"* ]] && [ -f "$b/app/src/app/Main.scala" ]; then
  pass "stage refuses a project without a stage block and a directory it would empty outside a target directory"
else
  fail "stage's refusals ($code, $code2): $out // $out2"
fi
# Without `mainClass` in the block, the plugin's shape for a build leaving the key alone, the class
# is implied as `run` implies it: solo's single product main (`solo.Solo`, its `main` inherited
# through core's trait) is the script's and its jar's `Main-Class`, core's jar still without one,
# and the staged script runs it; with `mainClassNone`, the shape for a build setting the key to
# None, the script runs it and the manifest keeps no `Main-Class`, as sbt leaves it; pair's two are
# refused naming both, util's none as "no main class", in the stage's own words (native-packager
# would write a script per class, or none), and nothing staged.
solo_stage=$b/solo/target/docker/stage
solo_jar=$solo_stage/4/opt/docker/lib/solo.solo-1.0.jar
out=$(task stage solo)
code=$?
if [ $code = 0 ] && [[ "$out" == *"solo: staged 5 files in layers 2, 4 under solo/target/docker/stage"* ]] && grep -q "'solo.Solo'" "$solo_stage/4/opt/docker/bin/solo" &&
  manifest "$solo_jar" | grep -q '^Main-Class: solo.Solo' && ! manifest "$solo_stage/4/opt/docker/lib/core.core-1.0.jar" | grep -q 'Main-Class'; then
  pass "stage: without mainClass in the block, the single product main is the script's and the project's own manifest's"
else
  fail "stage solo ($code): $out // $(manifest "$solo_jar" 2>&1)"
fi
rm -rf "$work/image" && mkdir -p "$work/image" && cp -R "$solo_stage/2/opt/docker/." "$solo_stage/4/opt/docker/." "$work/image/"
out=$(cd "$work" && timeout 60 sh image/bin/solo x y 2>&1)
if [ "$out" = "solo x y" ]; then pass "stage: the staged script runs the implied class"; else fail "the staged solo script: $out"; fi
STAGE_NONE=1 write_export 0.1.7-check.1 80
out=$(task stage solo)
code=$?
write_export 0.1.7-check.1 80
if [ $code = 0 ] && grep -q "'solo.Solo'" "$solo_stage/4/opt/docker/bin/solo" && ! manifest "$solo_jar" | grep -q 'Main-Class'; then
  pass "stage: with mainClassNone in the block, the script runs the implied class and the manifest keeps no Main-Class"
else
  fail "stage solo with mainClassNone ($code): $out // $(manifest "$solo_jar" 2>&1)"
fi
out=$(task stage pair)
code=$?
out2=$(task stage util)
code2=$?
if [ $code = 2 ] && [[ "$out" == *"teq: pair declares no main class and its products hold 2, for which native-packager would write a start script each: pair.A, pair.b; set Compile / mainClass := Some(one of them) and export again"* ]] && [ ! -e "$b/pair/target/docker" ] &&
  [ $code2 = 2 ] && [[ "$out2" == *"teq: no main class in util's products, which native-packager would stage without a start script"* ]] && [ ! -e "$b/util/target/docker" ]; then
  pass "stage refuses a project with two product mains and none declared, naming both with the declaration as the remedy, and one whose products hold none, staging nothing"
else
  fail "stage's implied refusals ($code, $code2): $out // $out2"
fi
task stop > /dev/null

# The fetch: the jar served over loopback http, absent from an empty coursier cache.
mkdir -p "$work/repo/x/lib/1.0" "$work/empty"
cp "$lib3" "$work/repo/x/lib/1.0/lib-1.0.jar"
port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
(cd "$work/repo" && exec timeout 300 python3 -m http.server "$port" --bind 127.0.0.1 > /dev/null 2>&1) &
server=$!
disown "$server"
for _ in $(seq 50); do curl -fs "http://127.0.0.1:$port/" > /dev/null && break; sleep 0.1; done
sha1=$(shasum "$lib3" | cut -d' ' -f1)
size=$(wc -c < "$lib3" | tr -d ' ')
fetch_export() {
  python3 - "$b/teq.lock" "$port" "$1" "$size" <<'PY'
import sys
import importlib.util, os
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
file, port, sha1, size = sys.argv[1:]
e = lock.parse(open(file, encoding="utf-8").read())
e["repositories"].append({"id": "local", "url": f"http://127.0.0.1:{port}/"})
e["projects"]["core"]["configurations"]["compile"]["classpath"][0] = "x:lib:1.0"
e["jars"]["x:lib:1.0"] = f"local {sha1} {size}"
open(file, "w", encoding="utf-8").write(lock.canonical(e))
PY
}
fetch_export 0000000000000000000000000000000000000000
out=$( (cd "$b" && COURSIER_CACHE=$work/empty timeout 150 "$TEQ" compile core 2>&1) )
if [ $? = 1 ] && [[ "$out" == *"where the export pins"*"refused"* ]] && [ -z "$(find "$TEQ_CACHE_DIR/artifacts" -name '*lib-1.0.jar*' 2> /dev/null)" ]; then
  pass "a fetched artifact whose sha1 is not the pinned one is refused, nothing left in the cache"
else
  fail "the refused fetch: $out"
fi
(cd "$b" && timeout 30 "$TEQ" stop > /dev/null 2>&1)
write_export 0.1.7-check.9 81
fetch_export "$sha1"
out=$( (cd "$b" && COURSIER_CACHE=$work/empty timeout 150 "$TEQ" compile core 2>&1) )
if [ $? = 0 ] && [ -f "$TEQ_CACHE_DIR/artifacts/$sha1/lib-1.0.jar" ]; then pass "an artifact missing from coursier's cache is fetched into the shared cache, verified"; else fail "the fetch: $out"; fi
# Offline: a version holding an `@`, from a repository with a port that is gone, its jar in
# coursier's cache under coursier's escapes: the build takes that copy and fetches nothing.
(cd "$b" && timeout 30 "$TEQ" stop > /dev/null 2>&1)
write_export 0.1.7-check.9 81
mkdir -p "$work/coursier/http/127.0.0.1%3A9/x/lib/1.0%40b"
cp "$lib3" "$work/coursier/http/127.0.0.1%3A9/x/lib/1.0%40b/lib-1.0%40b.jar"
python3 - "$b/teq.lock" "$sha1" "$size" <<'PY'
import sys
import importlib.util, os
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
file, sha1, size = sys.argv[1:]
e = lock.parse(open(file, encoding="utf-8").read())
e["repositories"].append({"credentials": "127.0.0.1", "id": "gone", "url": "http://127.0.0.1:9/"})
e["projects"]["core"]["configurations"]["compile"]["classpath"][0] = "x:lib:1.0@b"
e["jars"]["x:lib:1.0@b"] = f"gone {sha1} {size}"
open(file, "w", encoding="utf-8").write(lock.canonical(e))
PY
out=$( (cd "$b" && COURSIER_CACHE=$work/coursier timeout 150 "$TEQ" compile core 2>&1) )
if [ $? = 0 ] && [ -z "$(find "$TEQ_CACHE_DIR/artifacts" -name 'lib-1.0@b.jar' 2> /dev/null)" ]; then
  pass "a jar whose version holds an @, its repository gone, is coursier's copy under coursier's escapes, nothing fetched"
else
  fail "the offline jar with an @: $out"
fi
(cd "$b" && timeout 30 "$TEQ" stop > /dev/null 2>&1)

# The cold fetch (fetch::resolve_all): two configurations at once, f1 and f2, whose classpaths name
# nine small jars of a loopback repository (j4 to j6 on both; jd, on f2's, j1's bytes under another
# name), each answered after a second: four transfers at most in the daemon, one request per sha1, a
# line per transfer on the client's stderr, the classpath's order kept. Then a jar held until released, whose line the client shows before its
# transfer ends; then a 404 beside a transfer that never ends: the failure reported at once, naming
# its jar, the stalled curl ended and no partial file left.
cold=$work/cold
mkdir -p "$cold/repo/x"
for n in 1 2 3 4 5 6 7 8 h; do
  mkdir -p "$cold/repo/x/j$n/1"
  python3 -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1], "w"); z.writestr(f"j{sys.argv[2]}.txt", sys.argv[2]); z.close()' "$cold/repo/x/j$n/1/j$n-1.jar" "$n"
done
mkdir -p "$cold/repo/x/jd/1"
cp "$cold/repo/x/j1/1/j1-1.jar" "$cold/repo/x/jd/1/jd-1.jar"
cport=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
timeout 900 python3 - "$cold/repo" "$cport" "$cold" <<'PY' > /dev/null 2>&1 &
import http.server, os, sys, threading, time
root, port, state = sys.argv[1], int(sys.argv[2]), sys.argv[3]
lock, running = threading.Lock(), [0, 0]
class Repository(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        with open(os.path.join(state, "requests"), "a") as f:
            f.write(self.path + "\n")
        path = os.path.join(root, self.path.lstrip("/"))
        if "/js/" in self.path:
            open(os.path.join(state, "stalled"), "w").close()
            time.sleep(600)
        if not os.path.isfile(path):
            self.send_response(404)
            self.end_headers()
            return
        with lock:
            running[0] += 1
            running[1] = max(running[1], running[0])
            open(os.path.join(state, "peak"), "w").write(str(running[1]))
        if "/jh/" in self.path:
            open(os.path.join(state, "held"), "w").close()
            while not os.path.exists(os.path.join(state, "release")):
                time.sleep(0.05)
        else:
            time.sleep(1)
        body = open(path, "rb").read()
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        with lock:
            running[0] -= 1
    def log_message(self, *args):
        pass
http.server.ThreadingHTTPServer(("127.0.0.1", port), Repository).serve_forever()
PY
cserver=$!
for _ in $(seq 50); do curl -s "http://127.0.0.1:$cport/" > /dev/null && break; sleep 0.1; done
# cold_export <f1's jars> <f2's jars>: f1 and f2 over core's sources, each with the Scala
# libraries and the jars named, all pinned from the loopback repository.
cold_export() {
  write_export 0.1.7-check.9 81
  edit_lock "$b/teq.lock" "
import hashlib
e['repositories'].append({'id': 'cold', 'url': 'http://127.0.0.1:$cport/'})
def jar(n):
    data = {'s': b'never', 'm': b'missing'}.get(n) or open(f'$cold/repo/x/j{n}/1/j{n}-1.jar', 'rb').read()
    e['jars'][f'x:j{n}:1'] = f'cold {hashlib.sha1(data).hexdigest()} {len(data)}'
    return f'x:j{n}:1'
for name, jars, sources in (('f1', '$1'.split(), 'core/src'), ('f2', '$2'.split(), 'f2/src')):
    p = dict(e['projects']['core'])
    p['base'] = name
    p['description'] = dict(p['description'], sources=[sources])
    p['configurations'] = {'compile': dict(e['projects']['core']['configurations']['compile'], sources=[sources])}
    p['configurations']['compile']['classpath'] = list(e['projects']['core']['configurations']['compile']['classpath']) + [jar(n) for n in jars]
    if jars:
        e['projects'][name] = p
"
}
# f2's own sources, so that a resident of its own types it beside f1's.
mkdir -p "$b/f2/src/f2"
printf 'package f2\n\nobject F2:\n  val x: Int = 2\n' > "$b/f2/src/f2/F2.scala"
rm -f "$cold/requests"
cold_export "1 2 3 4 5 6" "4 5 6 7 8 d"
started=$(date +%s)
out=$( (cd "$b" && COURSIER_CACHE=$work/empty timeout 150 "$TEQ" compile f1 f2 2>&1) )
code=$?
elapsed=$(($(date +%s) - started))
order=$(grep "starting f1/compile" "$b/target/teq/task.log" | tail -1 | tr ':' '\n' | grep -o 'j[0-9]-1.jar' | tr '\n' ' ')
lines=$(printf '%s\n' "$out" | grep -c '^teq: fetching x:j[0-9d]:1 (')
requests=$(sort "$cold/requests" | uniq -d | wc -l | tr -d ' ')
j1=$(shasum "$cold/repo/x/j1/1/j1-1.jar" | cut -d' ' -f1)
if [ $code = 0 ] && [ "$(cat "$cold/peak")" = 4 ] && [ "$lines" = 8 ] && [ "$(wc -l < "$cold/requests" | tr -d ' ')" = 8 ] && [ "$requests" = 0 ] &&
  [ -f "$TEQ_CACHE_DIR/artifacts/$j1/j1-1.jar" ] && [ -f "$TEQ_CACHE_DIR/artifacts/$j1/jd-1.jar" ] && [ "$order" = "j1-1.jar j2-1.jar j3-1.jar j4-1.jar j5-1.jar j6-1.jar " ] && [ "$elapsed" -lt 30 ]; then
  pass "a cold fetch of two configurations: nine jars in ${elapsed} s, four transfers at most, eight requests (one per sha1), a line each on the client's stderr, the classpath's order kept"
else
  fail "the cold fetch ($code, peak $(cat "$cold/peak" 2> /dev/null), $lines lines, requests $(tr '\n' ' ' < "$cold/requests"), order $order, ${elapsed} s): $out"
fi
(cd "$b" && timeout 30 "$TEQ" stop > /dev/null 2>&1)
cold_export "h" ""
(cd "$b" && COURSIER_CACHE=$work/empty timeout 150 "$TEQ" compile f1 > "$cold/held.out" 2>&1) &
held=$!
for _ in $(seq 200); do [ -f "$cold/held" ] && break; sleep 0.05; done
sleep 0.5
seen=$(grep -c '^teq: fetching x:jh:1 (' "$cold/held.out")
touch "$cold/release"
wait "$held"
code=$?
if [ "$seen" = 1 ] && [ $code = 0 ]; then pass "the client shows a jar's line while its transfer is under way"; else fail "the progress before the transfer's end ($seen lines seen, $code): $(cat "$cold/held.out")"; fi
(cd "$b" && timeout 30 "$TEQ" stop > /dev/null 2>&1)
cold_export "s m" ""
started=$(date +%s)
out=$( (cd "$b" && COURSIER_CACHE=$work/empty timeout 150 "$TEQ" compile f1 2>&1) )
code=$?
elapsed=$(($(date +%s) - started))
parts=$(find "$TEQ_CACHE_DIR/artifacts" -name '*.part' 2> /dev/null)
daemon=$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["pid"])' "$b/target/teq/daemon.json" 2> /dev/null)
if [ $code = 1 ] && [[ "$out" == *"x:jm:1: GET http://127.0.0.1:$cport/x/jm/1/jm-1.jar answered 404"* ]] && [ -f "$cold/stalled" ] && [ "$elapsed" -lt 20 ] && [ -z "$parts" ] && [ -n "$daemon" ] && ! pgrep -P "$daemon" -x curl > /dev/null; then
  pass "a failure beside a transfer that never ends: reported in ${elapsed} s naming its jar, the stalled curl ended, no partial file left"
else
  fail "the failure beside a stalled transfer ($code, ${elapsed} s, parts: $parts): $out"
fi
(cd "$b" && timeout 30 "$TEQ" stop > /dev/null 2>&1)
kill "$cserver" 2> /dev/null

# A Scala.js project's one build and its dev loop, over an export of its own: `web`, whose
# generator writes its labels, and `side`, a project whose block the loop does not read. The
# package manager and the dev command are scripts: the one records its runs and makes
# node_modules, the other records its start, its arguments and TEQ, and waits.
w=$work/w
mkdir -p "$w/web/src/web" "$w/side/src" "$w/bin" "$w/project"
cat > "$w/web/src/web/Main.scala" <<'EOF'
package web

object Main:
  def main(args: Array[String]): Unit = println("hello " + Labels.text)
EOF
echo 'lazy val web = project' > "$w/build.sbt"
echo 'sbt.version=2.0.8' > "$w/project/build.properties"
echo one > "$w/labels.txt"
cat > "$w/gen.sh" <<'EOF'
mkdir -p "$2/web"
printf 'package web\n\nobject Labels:\n  val text: String = "%s"\n' "$(cat "$1")" > "$2/web/Labels.scala"
EOF
cat > "$w/bin/fakepm" <<'EOF'
#!/bin/sh
echo "$PWD $*" >> "$FAKEPM_LOG"
mkdir -p node_modules
[ -f fake.lock ] || echo 'lock 1' > fake.lock
EOF
chmod +x "$w/bin/fakepm"
cat > "$w/web/dev.sh" <<'EOF'
echo "started in $PWD with $* TEQ=$TEQ" >> "$DEV_LOG"
[ -n "$DEV_EXIT" ] && exit "$DEV_EXIT"
if [ -n "$DEV_STUBBORN" ]; then
  trap '' TERM
elif [ -n "$DEV_SLOW_INT" ]; then
  trap 'sleep 1; echo cleaned >> "$DEV_LOG"; exit 0' INT
else
  trap 'echo ended >> "$DEV_LOG"; exit 0' TERM
fi
sleep 120 &
echo $! > "$DEV_LOG.sleep"
wait
EOF
# The web export; `$1` the compiler's version, `$2` a key of web's description, `$3` one of side's,
# `$4` web's `release` (false when not given), WEB_JAR_SIZE and SIDE_JAR_SIZE the pinned sizes of a
# jar each project's classpath names (1 when not given; no Scala.js library, so never resolved).
# web's production sources are web/prod.
web_export() {
  python3 - "$w" "$1" "$2" "$3" "${4:-false}" "${WEB_JAR_SIZE:-1}" "${SIDE_JAR_SIZE:-1}" <<'PY'
import hashlib, sys
import importlib.util, os
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
w, version, web_key, side_key, release, web_jar, side_jar = sys.argv[1:]
gen = "target/teq/web/compile/src_managed"
flags = {"ignoredScalacOptions": [], "kindProjector": False, "maxInlines": 80, "strictEquality": False, "werror": False}
def description(p, sources, key):
    production = ["web/prod"] if p == "web" else []
    return {"cacheableState": [], "excludes": [], "hot": False, "keys": {"k": key}, "macroState": "ordered", "modulePerFile": [], "out": p + "/target/teq/out", "production": {"excludes": [], "sources": production}, "release": p == "web" and release == "true", "sources": sources}
def compile(sources, generators, jar):
    return {"compile": {"classpath": [jar], "flags": flags, "generators": generators, "mainClasses": [], "resources": [], "sources": sources}}
inputs = {f: hashlib.sha256(open(os.path.join(w, f), "rb").read()).hexdigest() for f in ["build.sbt", "project/build.properties"]}
export = {
    "teq": version,
    "format": 1,
    "binaries": {},
    "inputs": {"files": inputs, "sha256": hashlib.sha256("".join(f"{k}\0{v}\n" for k, v in sorted(inputs.items())).encode()).hexdigest()},
    "java": {"outputVersion": 17},
    "projects": {
        "web": {"base": "web", "platform": "js", "scalaVersion": "3.8.4", "description": description("web", ["web/src", gen], web_key),
                "dev": {"command": ["sh", "dev.sh"], "lockfile": "web/fake.lock", "packageManager": "fakepm"},
                "configurations": compile(["web/src", gen], [{"cwd": ".", "inputs": ["labels.txt"], "kind": "command", "outputs": [gen], "run": ["sh", "gen.sh", "labels.txt", gen]}], "x:web-lib:1")},
        "side": {"base": "side", "platform": "js", "scalaVersion": "3.8.4", "description": description("side", ["side/src"], side_key), "configurations": compile(["side/src"], [], "x:side-lib:1")},
    },
    "jars": {"x:web-lib:1": f"maven-central {'0' * 40} {web_jar}", "x:side-lib:1": f"maven-central {'0' * 40} {side_jar}"},
    "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}],
}
open(os.path.join(w, "teq.lock"), "w", encoding="utf-8").write(lock.canonical(export))
PY
}
web_export 0.1.7-check.1 a a
out=$( (cd "$w" && timeout 30 "$TEQ" run web 2>&1) )
if [ $? = 2 ] && [[ "$out" == *"web is a Scala.js project, which teq dev runs"*"(none has one)"* ]]; then pass "run refuses a Scala.js project, naming dev"; else fail "run of a Scala.js project: $out"; fi
export FAKEPM_LOG=$work/fakepm.log DEV_LOG=$work/dev.log
out=$( (cd "$w" && timeout 60 "$TEQ" build web -o "$work/web.js" 2>&1) )
if [ $? = 0 ] && [ "$(timeout 20 node "$work/web.js")" = "hello one" ]; then pass "build: one build of a Scala.js project, its generator run first"; else fail "build: $out"; fi
out=$( (cd "$b" && timeout 60 "$TEQ" build app 2>&1) )
if [ $? = 2 ] && [[ "$out" == *"build is a Scala.js project's; app is a JVM project"* ]]; then pass "build refuses a JVM project"; else fail "build of a JVM project: $out"; fi
mkdir -p "$w/web/prod"
printf 'object Bad:\n  val x: Int = "no"\n' > "$w/web/prod/Bad.scala"
web_export 0.1.7-check.1 a a true
out=$( (cd "$w" && timeout 60 "$TEQ" build web -o "$work/web-release.js" 2>&1) )
code=$?
web_export 0.1.7-check.1 a a
(cd "$w" && timeout 60 "$TEQ" build web -o "$work/web-dev.js" > /dev/null 2>&1)
code2=$?
rm -rf "$w/web/prod"
if [ $code != 0 ] && [[ "$out" == *"web/prod/Bad.scala"* ]] && [ $code2 = 0 ]; then pass "build: a description that sets release is the production build, its sources included, whatever output the options name"; else fail "build of a release description ($code, $code2): $out"; fi

lines() { if [ -f "$1" ]; then grep -c "${2:-.}" "$1"; else echo 0; fi; }
until_true() { for _ in $(seq 60); do eval "$1" && return 0; sleep 0.2; done; return 1; }
dev_pid=
start_dev() {
  (cd "$w" && PATH="$w/bin:$PATH" exec "$TEQ" dev web "$@") > "$work/dev.out" 2>&1 < /dev/null &
  dev_pid=$!
}
# A process that has ended or is a zombie, which kill -0 would still find.
over() { local s; s=$(ps -o stat= -p "$1" 2> /dev/null); [ -z "$s" ] || [[ "$s" == Z* ]]; }
ended() { over "$dev_pid"; }
sleep_gone() { [ ! -f "$DEV_LOG.sleep" ] || over "$(cat "$DEV_LOG.sleep")"; }
canonical_teq=$(python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$TEQ")
start_dev -- --flag
if until_true '[ "$(lines "$DEV_LOG" started)" = 1 ]' && grep -q "started in $w/web with --flag TEQ=$canonical_teq" "$DEV_LOG" && [ "$(lines "$FAKEPM_LOG")" = 1 ] && grep -q "^$w/web install$" "$FAKEPM_LOG"; then
  pass "dev: the install in the package's directory, then the dev command there with the args after -- and TEQ naming the binary"
else
  fail "dev's start: $(cat "$work/dev.out" "$DEV_LOG" "$FAKEPM_LOG" 2> /dev/null)"
fi
echo two > "$w/labels.txt"
if until_true 'grep -q "\"two\"" "$w/target/teq/web/compile/src_managed/web/Labels.scala"' && [ "$(lines "$DEV_LOG" started)" = 1 ]; then pass "dev: a generator's input changed runs it again, the dev command left running"; else fail "dev's generator: $(cat "$w/target/teq/web/compile/src_managed/web/Labels.scala")"; fi
web_export 0.1.7-check.1 a b
sleep 2.5
if [ "$(lines "$DEV_LOG" started)" = 1 ]; then pass "dev: a change of another project's block leaves the dev command running"; else fail "dev restarted for side: $(cat "$work/dev.out")"; fi
web_export 0.1.7-check.1 b b
if until_true '[ "$(lines "$DEV_LOG" started)" = 2 ]' && [ "$(lines "$DEV_LOG" ended)" = 1 ] && [ "$(lines "$FAKEPM_LOG")" = 1 ] && grep -q "changed what web reads: restarting sh dev.sh" "$work/dev.out"; then
  pass "dev: a change of the project's block restarts the dev command, no install"
else
  fail "dev's restart: $(cat "$work/dev.out")"
fi
echo 'lock 2' > "$w/web/fake.lock"
if until_true '[ "$(lines "$DEV_LOG" started)" = 3 ]' && [ "$(lines "$FAKEPM_LOG")" = 2 ]; then pass "dev: a changed lockfile is installed and the dev command restarted"; else fail "dev's lockfile: $(cat "$work/dev.out" "$FAKEPM_LOG")"; fi
web_export 0.1.7-check.9 b b
until_true ended
wait "$dev_pid"
code=$?
if [ $code = 75 ] && [ "$(lines "$DEV_LOG" ended)" = 3 ] && sleep_gone && grep -q "pins another compiler now" "$work/dev.out"; then pass "dev: another compiler pinned ends the loop with code 75, the dev command's group with it"; else fail "dev's end on a pin ($code): $(cat "$work/dev.out")"; fi
start_dev
until_true '[ "$(lines "$DEV_LOG" started)" = 4 ]'
kill -INT "$dev_pid"
until_true ended
wait "$dev_pid"
code=$?
if [ $code = 130 ] && [ "$(lines "$FAKEPM_LOG")" = 2 ] && sleep_gone; then pass "dev: SIGINT ends the loop with code 130 and the dev command's group; no install when the lockfile is the one installed"; else fail "dev's SIGINT ($code, $(lines "$FAKEPM_LOG") installs): $(cat "$work/dev.out")"; fi
# Another loop over the same package installs a new lockfile: its record first, then the file.
start_dev
until_true '[ "$(lines "$DEV_LOG" started)" = 5 ]'
printf '%s\n' "$(printf 'lock 3\n' | shasum -a 256 | cut -d' ' -f1)" > "$w/target/teq/dev/web/fake.lock.sha256"
echo 'lock 3' > "$w/web/fake.lock"
if until_true '[ "$(lines "$DEV_LOG" started)" = 6 ]' && [ "$(lines "$FAKEPM_LOG")" = 2 ]; then pass "dev: a lockfile another loop installed restarts the dev command, with no install of its own"; else fail "dev after another loop's install ($(lines "$FAKEPM_LOG") installs): $(cat "$work/dev.out")"; fi
kill -INT "$dev_pid"
until_true ended
wait "$dev_pid"
DEV_STUBBORN=1 start_dev
until_true '[ "$(lines "$DEV_LOG" started)" = 7 ]'
kill -TERM "$dev_pid"
until_true ended
wait "$dev_pid"
code=$?
if [ $code = 143 ] && sleep_gone; then pass "dev: SIGTERM ends the loop with 143 when the dev command ignores it, its group killed after the bound"; else fail "dev's SIGTERM to a command that ignores it ($code): $(cat "$work/dev.out")"; fi
DEV_SLOW_INT=1 start_dev
until_true '[ "$(lines "$DEV_LOG" started)" = 8 ]'
kill -INT "$dev_pid"
until_true ended
wait "$dev_pid"
code=$?
if [ $code = 130 ] && [ "$(lines "$DEV_LOG" cleaned)" = 1 ] && sleep_gone; then pass "dev: SIGINT passed on leaves the dev command its cleanup, with no SIGTERM after it"; else fail "dev's SIGINT to a command that cleans up ($code, $(lines "$DEV_LOG" cleaned) cleanups): $(cat "$work/dev.out")"; fi
# One write of the export that pins another compiler and drops the project; then one that drops it alone.
start_dev
until_true '[ "$(lines "$DEV_LOG" started)" = 9 ]'
edit_lock "$w/teq.lock" 'e["teq"] = "0.1.7-check.7"; del e["projects"]["web"]'
until_true ended
wait "$dev_pid"
code=$?
web_export 0.1.7-check.9 b b
start_dev
until_true '[ "$(lines "$DEV_LOG" started)" = 10 ]'
edit_lock "$w/teq.lock" 'del e["projects"]["web"]["dev"]'
until_true ended
wait "$dev_pid"
code2=$?
web_export 0.1.7-check.9 b b
if [ $code = 75 ] && [ $code2 = 2 ] && sleep_gone && grep -q "web has no dev loop" "$work/dev.out"; then pass "dev: a pin changed with the project dropped ends the loop with 75; the project's dev loop dropped ends it with 2"; else fail "dev's export without the project ($code, $code2): $(cat "$work/dev.out")"; fi
DEV_EXIT=3 start_dev
until_true ended
wait "$dev_pid"
code=$?
if [ $code = 3 ]; then pass "dev: a dev command that ends ends the loop with its code"; else fail "dev's own end ($code): $(cat "$work/dev.out")"; fi
# The jar table: a record only side's classpath names re-pinned leaves the dev command running; one
# web's names restarts it.
n=$(lines "$DEV_LOG" started)
start_dev
until_true '[ "$(lines "$DEV_LOG" started)" = $((n + 1)) ]'
SIDE_JAR_SIZE=2 web_export 0.1.7-check.9 b b
sleep 2.5
if [ "$(lines "$DEV_LOG" started)" = $((n + 1)) ]; then pass "dev: a jar re-pinned that another project's classpath names leaves the dev command running"; else fail "dev restarted for side's jar: $(cat "$work/dev.out")"; fi
SIDE_JAR_SIZE=2 WEB_JAR_SIZE=2 web_export 0.1.7-check.9 b b
if until_true '[ "$(lines "$DEV_LOG" started)" = $((n + 2)) ]' && grep -q "changed what web reads: restarting sh dev.sh" "$work/dev.out"; then
  pass "dev: a jar re-pinned under the key web's classpath names restarts the dev command"
else
  fail "dev's restart on web's jar: $(cat "$work/dev.out")"
fi
kill -INT "$dev_pid"
until_true ended
wait "$dev_pid"
web_export 0.1.7-check.9 b b
out=$( (cd "$w" && timeout 20 "$TEQ" dev side 2>&1) )
if [ $? = 2 ] && [[ "$out" == *"side has no dev loop"* ]]; then pass "dev refuses a project without a dev block"; else fail "dev without a dev block: $out"; fi


# An export under the root's target/teq/ (a build without the native driver), found from below the
# root, the root the build's: a lock recording a generator sbt alone runs (kind sbt) of `gen`, whose
# compile is refused naming it, of `uses`'s too, whose closure holds gen's; `other` compiles; a
# compile of every project compiles other and names gen and uses left out, exiting 2, as it does
# when every project is left out. `tested` has a Test generator sbt runs: its compile runs (the
# resident types its compile configuration), its test is refused; `kit`'s Test generator refuses
# the compile of `withkit`, whose compile classpath holds kit's test products (compile->test).
r=$work/r
mkdir -p "$r/gen/src/gen" "$r/uses/src/uses" "$r/other/src/other" "$r/tested/src/tested" "$r/tested/test/tested" "$r/kit/test/kit" "$r/withkit/src/withkit" "$r/target/teq" "$r/project"
printf 'package gen\n\nobject Gen:\n  val n: Int = 1\n' > "$r/gen/src/gen/Gen.scala"
printf 'package uses\n\nobject Uses:\n  val n: Int = gen.Gen.n\n' > "$r/uses/src/uses/Uses.scala"
printf 'package other\n\nobject Other:\n  val n: Int = 2\n' > "$r/other/src/other/Other.scala"
printf 'package tested\n\nobject Tested:\n  val n: Int = 3\n' > "$r/tested/src/tested/Tested.scala"
# Its test sources name what the Test generator writes, which is not there: typed, they fail.
printf 'package tested\n\nobject TestedTest:\n  val n: Int = Generated.n\n' > "$r/tested/test/tested/TestedTest.scala"
printf 'package kit\n\nobject Kit:\n  val n: Int = 4\n' > "$r/kit/test/kit/Kit.scala"
printf 'package withkit\n\nobject WithKit:\n  val n: Int = kit.Kit.n\n' > "$r/withkit/src/withkit/WithKit.scala"
echo 'lazy val gen = project' > "$r/build.sbt"
echo 'sbt.version=2.0.8' > "$r/project/build.properties"
python3 - "$r" "$lib" "$lib3" <<'PY'
import hashlib, importlib.util, os, sys
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
r, lib, lib3 = sys.argv[1:]
m2 = lib[: lib.index("/maven2/") + len("/maven2/")]
def artifact(path):
    data = open(path, "rb").read()
    parts = path[len(m2):].split("/")
    return ":".join([".".join(parts[:-3]), parts[-3], parts[-2]]), f"maven-central {hashlib.sha1(data).hexdigest()} {len(data)}"
table = dict([artifact(lib3), artifact(lib)])
flags = {"ignoredScalacOptions": [], "kindProjector": False, "strictEquality": False, "werror": False}
by_sbt = [{"kind": "sbt", "task": "an unnamed task"}]
def project(name, generators=(), depends=(), test_generators=None, test_of=()):
    products = [{"configuration": "compile", "project": d} for d in depends] + [{"configuration": "test", "project": d} for d in test_of]
    configurations = {"compile": {"classpath": products + list(table), "flags": flags, "generators": list(generators), "mainClasses": [], "resources": [], "sources": [name + "/src"] + ([f"target/out/jvm/scala-3.8.4/{name}/src_managed/main"] if generators else [])}}
    if test_generators is not None:
        configurations["test"] = {"classpath": [{"configuration": "compile", "project": name}] + list(table), "flags": flags, "frameworks": [], "generators": test_generators, "resources": [], "sources": [name + "/test"] + ([f"target/out/jvm/scala-3.8.4/{name}/src_managed/test"] if test_generators else [])}
    return {"base": name, "platform": "jvm", "scalaVersion": "3.8.4", "configurations": configurations}
inputs = {f: hashlib.sha256(open(os.path.join(r, f), "rb").read()).hexdigest() for f in ["build.sbt", "project/build.properties"]}
digest = hashlib.sha256("".join(f"{f}\0{d}\n" for f, d in sorted(inputs.items())).encode()).hexdigest()
export = {"teq": "0.1.7-check.1", "format": 1, "binaries": {}, "inputs": {"files": inputs, "sha256": digest}, "java": {"outputVersion": 17},
          "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}], "jars": table,
          "projects": {"gen": project("gen", by_sbt), "uses": project("uses", depends=["gen"]), "other": project("other"),
                       "tested": project("tested", test_generators=by_sbt), "kit": project("kit", test_generators=by_sbt), "withkit": project("withkit", test_of=["kit"])}}
open(os.path.join(r, "target/teq/teq.lock"), "w", encoding="utf-8").write(lock.canonical(export))
# Every project left out: gen and uses alone.
export["projects"] = {k: v for k, v in export["projects"].items() if k in ("gen", "uses")}
open(os.path.join(r, "target/teq/blocked.lock"), "w", encoding="utf-8").write(lock.canonical(export))
PY
generated="gen/compile has the source generator an unnamed task, which sbt runs and teq cannot: make it a TeqCommand of Compile / teqGenerators, or build gen with sbt"
out=$(cd "$r/other/src" && timeout 120 "$TEQ" compile gen uses 2>&1)
code=$?
if [ $code = 2 ] && [ "$out" = "teq: cannot compile gen uses, which need what sbt alone runs:
  $generated" ]; then pass "target/teq: a project whose closure holds a generator sbt alone runs is refused, found from below the root, every reason once"; else fail "target/teq, the refused compile ($code): $out"; fi
out=$(cd "$r/other/src" && timeout 120 "$TEQ" compile other 2>&1)
if [ $? = 0 ] && [[ "$out" == "other/compile: "*" classes, "* ]] && [ -f "$r/target/teq/other/compile/classes/other/Other.class" ]; then pass "target/teq: the other project compiles, its classes under the build's root"; else fail "target/teq, other's compile: $out"; fi
out=$(cd "$r" && timeout 120 "$TEQ" compile 2>&1)
code=$?
if [ $code = 2 ] && [[ "$out" == *"other/compile: "* ]] && [[ "$out" == *"tested/compile: "* ]] && [[ "$out" != *"gen/compile: "* ]] && [[ "$out" == *"teq: compile left out gen, which needs what sbt alone runs:
  $generated"* ]] && [[ "$out" == *"teq: compile left out uses, which needs"* ]] && [[ "$out" == *"teq: compile left out withkit, which needs"* ]]; then
  pass "target/teq: a compile of every project compiles the others and names the refused ones left out after, exiting 2"
else
  fail "target/teq, the compile of every project ($code): $out"
fi
out=$(cd "$r" && timeout 120 "$TEQ" compile tested 2>&1)
code=$?
test_out=$(cd "$r" && timeout 120 "$TEQ" test tested 2>&1)
test_code=$?
if [ $code = 0 ] && [[ "$out" == "tested/compile: "* ]] && [ $test_code = 2 ] && [ "$test_out" = "teq: cannot test tested, which needs what sbt alone runs:
  tested/test has the source generator an unnamed task, which sbt runs and teq cannot: build tested with sbt" ]; then
  pass "target/teq: a Test generator sbt runs leaves the compile running, typed without the test configuration, and refuses the test"
else
  fail "target/teq, the Test generator: compile ($code) $out; test ($test_code) $test_out"
fi
out=$(cd "$r" && timeout 120 "$TEQ" compile withkit 2>&1)
if [ $? = 2 ] && [[ "$out" == *"kit/test has the source generator an unnamed task, which sbt runs and teq cannot: build kit with sbt"* ]]; then
  pass "target/teq: a compile whose classpath holds a test configuration with a generator sbt runs is refused (compile->test)"
else
  fail "target/teq, compile->test: $out"
fi
out=$(cd "$r" && timeout 120 "$TEQ" --export target/teq/blocked.lock compile 2>&1)
if [ $? = 2 ] && [[ "$out" == *"teq: nothing to compile"* ]] && [[ "$out" == *"teq: compile left out gen"* ]] && [[ "$out" == *"teq: compile left out uses"* ]]; then
  pass "target/teq: a compile of every project with every one left out compiles nothing and exits 2"
else
  fail "target/teq, every project left out: $out"
fi
(cd "$r" && timeout 30 "$TEQ" stop > /dev/null 2>&1)

# The launcher tools/launcher/teq (docs/TARGETS.md, "The launchers") under each POSIX shell
# present, over a repository on the loopback interface that serves a stand-in for the binary, which
# prints its arguments and exits with STAND_IN_EXIT: a cold cache fetched into, the arguments and
# the exit code passed through, the cached copy run with the repository gone; then TEQ, coursier's
# copy, two launchers fetching at once, a CRLF lock and a quoted binaries line, and the refusals.
l=$work/launch
mkdir -p "$l/build"
printf '#!/bin/sh\necho "stand-in $*"\nexit "${STAND_IN_EXIT:-0}"\n' > "$l/stand-in"
chmod +x "$l/stand-in"
case $(uname -s) in Darwin) los=osx ;; Linux) los=linux ;; *) los=other ;; esac
case $(uname -m) in arm64 | aarch64) larch=aarch_64 ;; *) larch=$(uname -m) ;; esac
lc=$los-$larch
lsha1=$(shasum "$l/stand-in" | cut -d' ' -f1)
lsize=$(wc -c < "$l/stand-in" | tr -d ' ')
lport=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
timeout 600 python3 - "$l/stand-in" "$lport" "$l/requests" <<'PY' > /dev/null 2>&1 &
import http.server, sys, time
binary, port, log = open(sys.argv[1], "rb").read(), int(sys.argv[2]), sys.argv[3]
class Repository(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        with open(log, "a") as f:
            f.write(self.path + "\n")
        if self.path.startswith("/redirect/"):
            self.send_response(302)
            self.send_header("Location", f"http://127.0.0.1:{port}/teq/teq.exe")
            self.end_headers()
            return
        if not self.path.startswith(("/teq/", "/slow/", "/big/")):
            self.send_response(404)
            self.end_headers()
            return
        if self.path.startswith("/slow/"):
            time.sleep(1)
        body = binary + (b"#" * 64 if self.path.startswith("/big/") else b"")
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def log_message(self, *args):
        pass
http.server.ThreadingHTTPServer(("127.0.0.1", port), Repository).serve_forever()
PY
lserver=$!
for _ in $(seq 50); do curl -s "http://127.0.0.1:$lport/" > /dev/null && break; sleep 0.1; done
lurl=http://127.0.0.1:$lport/teq/teq.exe
cp tools/launcher/teq "$l/build/teq"
# lock_of <binaries line's fields>: the build's teq.lock pinning the stand-in for this machine.
lock_of() { printf 'teq: 9.9.9\nformat: 1\nbinaries:\n  %s: %s\nprojects: {}\n' "$lc" "$*" > "$l/build/teq.lock"; }
# launch <shell> <cache> <args...>: the launcher run from the build by the shell, TEQ unset.
launch() {
  local shell=$1 cache=$2
  shift 2
  # shellcheck disable=SC2086
  (cd "$l/build" && env -u TEQ TEQ_CACHE_DIR="$l/$cache" COURSIER_CACHE="$l/coursier" timeout 60 $shell ./teq "$@" 2>&1)
}
requests() { wc -l < "$l/requests" 2> /dev/null | tr -d ' '; }
shells=()
for s in sh dash bash zsh busybox; do command -v "$s" > /dev/null && shells+=("$s"); done
for s in "${shells[@]}"; do
  shell=$s
  [ "$s" = busybox ] && shell="busybox sh"
  syntax=$( (cd "$l/build" && $shell -n ./teq 2>&1) )
  [ -z "$syntax" ] || fail "launcher: $s -n: $syntax"
  lock_of "$lurl $lsha1 $lsize"
  before=$(requests)
  out=$(launch "$shell" "cache-$s" --version)
  bin=$l/cache-$s/bin/$lsha1/teq-9.9.9-$lc
  if [ $? = 0 ] && [[ "$out" == *"teq: fetching teq 9.9.9 for $lc from $lurl"*"stand-in --version" ]] && [ -x "$bin" ] && [ "$(requests)" = $((before + 1)) ] && [ "$(ls -A "$l/cache-$s/bin/$lsha1")" = "teq-9.9.9-$lc" ]; then
    pass "launcher ($s): a cold cache fetches the pinned binary once, verified and executable, and runs it"
  else
    fail "launcher ($s), a cold cache: $out"
  fi
  out=$(STAND_IN_EXIT=7 launch "$shell" "cache-$s" task test 'a b' '$x' "")
  code=$?
  lock_of "http://127.0.0.1:9/gone/teq.exe $lsha1 $lsize"
  gone=$(launch "$shell" "cache-$s" lsp)
  if [ $code = 7 ] && [ "$out" = 'stand-in task test a b $x ' ] && [ "$gone" = "stand-in lsp" ]; then
    pass "launcher ($s): the arguments as given and the exit code passed through; the cached copy runs with the repository gone"
  else
    fail "launcher ($s), the pass-through ($code: $out) and the cached copy ($gone)"
  fi
done
sh=${shells[0]}
lock_of "$lurl $lsha1 $lsize"
out=$(cd "$l/build" && TEQ="$l/stand-in" TEQ_CACHE_DIR="$l/cache-teq" timeout 30 "$sh" ./teq run x)
if [ "$out" = "stand-in run x" ] && [ ! -d "$l/cache-teq" ]; then pass "launcher: TEQ runs the binary it names, the lock unread"; else fail "launcher, TEQ: $out"; fi
lock_of "http://127.0.0.1:$lport/coursier-only/teq.exe $lsha1 $lsize"
mkdir -p "$l/coursier/http/127.0.0.1%3A$lport/coursier-only"
cp "$l/stand-in" "$l/coursier/http/127.0.0.1%3A$lport/coursier-only/teq.exe"
before=$(requests)
out=$(launch "$sh" cache-coursier compile)
if [ "$out" = "stand-in compile" ] && [ "$(requests)" = "$before" ] && [ -x "$l/cache-coursier/bin/$lsha1/teq-9.9.9-$lc" ]; then pass "launcher: coursier's copy of the URL is copied into teq's cache, nothing fetched"; else fail "launcher, coursier's copy: $out"; fi
lock_of "http://127.0.0.1:$lport/slow/teq.exe $lsha1 $lsize"
launch "$sh" cache-both one > "$l/both-1" &
first=$!
launch "$sh" cache-both two > "$l/both-2"
wait "$first"
if grep -q 'stand-in one' "$l/both-1" && grep -q 'stand-in two' "$l/both-2" && [ "$(ls -A "$l/cache-both/bin/$lsha1")" = "teq-9.9.9-$lc" ]; then
  pass "launcher: two launchers fetching at once both run the binary, one file left"
else
  fail "launcher, two at once: $(cat "$l/both-1" "$l/both-2")"
fi
printf 'teq: "9.9.9"\r\nformat: 1\r\nbinaries:\r\n  %s: "%s %s %s"\r\nprojects: {}\r\n' "$lc" "$lurl" "$lsha1" "$lsize" > "$l/build/teq.lock"
out=$(launch "$sh" cache-crlf x)
if [ "$out" = "$(printf 'teq: fetching teq 9.9.9 for %s from %s\nstand-in x' "$lc" "$lurl")" ]; then pass "launcher: a lock with CRLF line ends and the binaries line in quotes"; else fail "launcher, CRLF and quotes: $out"; fi
# refused <what> <message> <fields...>: the launcher over a lock of those fields fails, saying so, and leaves no file.
refused() {
  local what=$1 message=$2
  shift 2
  lock_of "$*"
  out=$(launch "$sh" cache-refused x)
  if [ $? = 1 ] && [[ "$out" == *"$message"* ]] && [ -z "$(find "$l/cache-refused" -type f 2> /dev/null)" ]; then pass "launcher: $what"; else fail "launcher, $what: $out"; fi
}
refused "a wrong sha1 is refused, nothing left in the cache" "has sha1 $lsha1 and $lsize bytes where the lock pins $(printf '0%.0s' $(seq 40)) and $lsize bytes: refused" "$lurl $(printf '0%.0s' $(seq 40)) $lsize"
refused "a body past the pinned size is refused" "has more than the $lsize bytes the lock pins: refused" "http://127.0.0.1:$lport/big/teq.exe $lsha1 $lsize"
refused "a redirect to plain http is refused" "GET http://127.0.0.1:$lport/redirect/teq.exe failed: curl: (1) Protocol \"http\" disabled" "http://127.0.0.1:$lport/redirect/teq.exe $lsha1 $lsize"
refused "a binary the repository lacks: its answer" "GET http://127.0.0.1:$lport/missing/teq.exe answered 404" "http://127.0.0.1:$lport/missing/teq.exe $lsha1 $lsize"
refused "plain http to another host is refused" "is not https, which teq fetches over from every host but this one" "http://repo.invalid/teq.exe $lsha1 $lsize"
refused "a binaries line of other fields is refused" "binaries line for $lc is not <url> <sha1> <size>" "$lurl $lsha1"
refused "a size that is no number is refused" "has no size in bytes" "$lurl $lsha1 12a"
printf 'teq: 9.9.9\nformat: 1\nbinaries:\n  other-classifier: %s %s %s\nprojects: {}\n' "$lurl" "$lsha1" "$lsize" > "$l/build/teq.lock"
out=$(launch "$sh" cache-refused x)
if [ $? = 1 ] && [[ "$out" == *"pins no binary of teq 9.9.9 for $lc: set TEQ to a teq 9.9.9 binary, or export the build (sbt teqExportAll) once the release 9.9.9 serves one"* ]]; then pass "launcher: a lock without this platform's binary is refused"; else fail "launcher, no binary: $out"; fi
# The table of a release not published yet, in the three shapes a lock may give it: empty (the export's), absent,
# and bare (no reader's lock, which teq's own reader refuses); each refused naming TEQ and the release to pin, the
# stand-in run all the same through TEQ.
for shape in 'binaries: {}\n' '' 'binaries:\n'; do
  printf "teq: 0.1.7\\nformat: 1\\n${shape}projects: {}\\n" > "$l/build/teq.lock"
  out=$(launch "$sh" cache-refused x)
  code=$?
  named=$(cd "$l/build" && TEQ="$l/stand-in" timeout 30 "$sh" ./teq x)
  if [ $code = 1 ] && [[ "$out" == *"pins no binary of teq 0.1.7 for $lc: set TEQ to a teq 0.1.7 binary, or export the build (sbt teqExportAll) once the release 0.1.7 serves one"* ]] &&
    [ "$named" = "stand-in x" ] && [ -z "$(find "$l/cache-refused" -type f 2> /dev/null)" ]; then
    pass "launcher: the table ${shape:-absent}, refused naming TEQ and 0.1.7; TEQ runs"
  else
    fail "launcher, the table '$shape': $out / $named"
  fi
done
# A lock of a release before 0.1.7 is refused before anything is read of its binaries, its copy in the cache
# never run; a SNAPSHOT is no release, and 0.1.60 none before it.
for old in 0.1.6 0.1.0-pre.1 0.0.1-check; do
  mkdir -p "$l/cache-old/bin/$lsha1" && cp "$l/stand-in" "$l/cache-old/bin/$lsha1/teq-$old-$lc"
  printf 'teq: %s\nformat: 1\nbinaries:\n  %s: %s %s %s\nprojects: {}\n' "$old" "$lc" "$lurl" "$lsha1" "$lsize" > "$l/build/teq.lock"
  out=$(launch "$sh" cache-old x)
  if [ $? = 1 ] && [ "$out" = "teq: ./teq.lock pins teq $old, and releases before 0.1.7 are not served: pin 0.1.7 or later (sbt teqExportAll), or set TEQ to a local binary" ]; then
    pass "launcher: a lock of $old, a release before 0.1.7, refused, its cached copy not run"
  else
    fail "launcher, the lock of $old: $out"
  fi
done
for later in 0.1.60 0.1.1-branch-SNAPSHOT; do
  mkdir -p "$l/cache-old/bin/$lsha1" && cp "$l/stand-in" "$l/cache-old/bin/$lsha1/teq-$later-$lc"
  printf 'teq: %s\nformat: 1\nbinaries:\n  %s: %s %s %s\nprojects: {}\n' "$later" "$lc" "$lurl" "$lsha1" "$lsize" > "$l/build/teq.lock"
  out=$(launch "$sh" cache-old x)
  if [ $? = 0 ] && [ "$out" = "stand-in x" ]; then pass "launcher: a lock of $later runs its copy"; else fail "launcher, the lock of $later: $out"; fi
done
printf '{\n  "schema": 2\n}\n' > "$l/build/teq.lock"
out=$(launch "$sh" cache-refused x)
if [ $? = 1 ] && [[ "$out" == *"first line is not teq: <version>: {"* ]]; then pass "launcher: a lock whose first line is not teq: is refused"; else fail "launcher, the first line: $out"; fi
rm "$l/build/teq.lock"
out=$(launch "$sh" cache-refused x)
if [ $? = 1 ] && [[ "$out" == *"no teq.lock beside ./teq: export the build with sbt teqExportAll"* ]]; then pass "launcher: no lock beside it is refused"; else fail "launcher, no lock: $out"; fi
if command -v shellcheck > /dev/null; then
  if out=$(shellcheck -s sh tools/launcher/teq 2>&1); then pass "launcher: shellcheck finds nothing"; else fail "launcher, shellcheck: $out"; fi
else
  echo "task: note: no shellcheck here, the launcher not linted"
fi
kill "$lserver" 2> /dev/null

if timeout 60 node integrations/vite/test.mjs > "$work/vite.out" 2>&1; then pass "vite-plugin-teq reads the export and finds the binary it pins (integrations/vite/test.mjs)"; else fail "vite-plugin-teq: $(grep FAIL "$work/vite.out" || tail -5 "$work/vite.out")"; fi

[ $status = 0 ] && echo "task: all passed"
exit $status
