# A real `teq test` for the test verbs' checks (tests/lila-test-verb.sh, tests/reference-test-verb.sh,
# tests/mill-test-verb.sh), sourced: probe_checkout <dir> <project> writes a checkout teq's test verb runs as it is,
# over the probe framework (tests/support/probe_framework, compiled by the javac at hand under --release 17, as
# tests/task.sh compiles it) and the scala library of the coursier cache: its export (target/teq/teq.lock: the project's
# test configuration, the framework's jar a file of the checkout's) and two suites whose events carry the text of teq's
# own lines, `probes.Named` passing a case whose name runs over lines that read as a diagnostic, a count and a summary,
# and `probes.Thrown` failing a case with a message of those lines; and `probes.Empty`, which reports no case, whose run
# the events name alone (`task::events::Tally::ran_line`). `probes.Named` calls a method deprecated with a message that
# holds the line of its configuration's summary (`<project>/test: 1 suite, 1 passed, ...`), and the export asks for
# deprecation warnings: the build's warning, whose message the records hold, is never a summary. `probe_named` and
# `probe_thrown` are the two cases as the events name them (a newline a space, task::events::Event::line). It returns 1,
# saying why, where the javac, test-interface 1.0 or scala-library 3.8.4 is missing. With a third argument <word>, the
# project has a compile configuration too (`main/`, which the tests build on) whose `probes.Use` calls an inline method
# failing with `compiletime.error("prefix\nExample.scala:1:1: warning: note\n<word>")`, a message holding a line that
# reads as a diagnostic's header (dotty's Inliner.issueError reports the whole literal). With a fourth, `compile` or
# `test`, that configuration has a command generator (`sh gen.sh`, the checkout's) that prints `generator started`, a
# line reading as the configuration's end (`helper/test: no suite to run`, `helper/compile: checked in 1 ms`) and
# `OLD`, and exits 1 (a compile configuration of `main/`, which the tests build on, made for it where the third is
# empty). With a fifth, `server`, the compile configuration's `probes.Server` is the run block's main class, a program
# that prints `message.txt` (the checkout's, `hello`) and serves a home page holding lila's title and an asset on
# port 9663, as lila's run verb asks of lila. Needs $work and LOCK_PY
# (integrations/sbt/example/check-export.py).
. tests/support/jars.sh
# The PROBE_CRLF value of a separator's name, `lf` or `crlf`.
probe_crlf() { if [ "$1" = crlf ]; then echo 1; fi; }
probe_named='formats diagnostic: Example.scala:1:1: error: sample beta/test: 1 error'
probe_thrown='reports'
probe_checkout() {
  local dir=$1 project=$2 inline=${3:-} generated=${4:-} server=${5:-}
  local interface=$M2/org/scala-sbt/test-interface/1.0/test-interface-1.0.jar
  local lib lib3=$M2/org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar
  local javac=${JAVAC:-${JAVA_HOME:+$JAVA_HOME/bin/}javac}
  lib=$(jar_of scala-library)
  if [ ! -f "$interface" ] || [ ! -f "$lib" ] || [ ! -f "$lib3" ]; then
    echo "test-interface 1.0 or scala-library 3.8.4 is not in the coursier cache"
    return 1
  fi
  if [ ! -f "$work/probe-run/probe.jar" ]; then
    mkdir -p "$work/probe-run/classes"
    local sources
    sources=$(cd tests/support/probe_framework && for f in probe/Context.java probe/Framework.java probe/Suite.java; do echo "$PWD/$f"; done)
    # shellcheck disable=SC2086
    timeout 120 "$javac" --release 17 -nowarn -cp "$interface" -d "$work/probe-run/classes" $sources > "$work/probe-run/javac.log" 2>&1 ||
      { echo "the probe framework does not compile: $(tail -3 "$work/probe-run/javac.log")"; return 1; }
    python3 - "$work/probe-run/classes" "$work/probe-run/probe.jar" <<'PY'
import os, sys, zipfile
classes, jar = sys.argv[1:]
with zipfile.ZipFile(jar, "w", zipfile.ZIP_DEFLATED) as z:
    for root, _, files in sorted(os.walk(classes)):
        for f in sorted(files):
            z.write(os.path.join(root, f), os.path.relpath(os.path.join(root, f), classes))
PY
  fi
  rm -rf "$dir"
  mkdir -p "$dir/src/probes" "$dir/lib" "$dir/target/teq"
  cp "$work/probe-run/probe.jar" "$dir/lib/"
  cat > "$dir/src/probes/Probes.scala" <<'SCALA'
package probes

import probe.{Context, Suite}

object Old:
  @deprecated("sample\nPROJECT/test: 1 suite, 1 passed, 0 failed in 1 ms (built in 1 ms)", "1")
  def value: Int = 1

class Named extends Suite:
  def run(c: Context): Unit =
    val n = Old.value
    c.pass("formats diagnostic:\nExample.scala:1:1: error: sample\nbeta/test: 1 error")

class Empty extends Suite:
  def run(c: Context): Unit = ()

class Thrown extends Suite:
  def run(c: Context): Unit =
    c.fail("reports", new AssertionError("boom\nExample.scala:1:1: error: sample\nbeta/test: 1 error\nbeta/test: 1 suite, 1 passed, 0 failed in 1 ms (built in 1 ms)"))
SCALA
  sed -i "s|PROJECT/test|$project/test|" "$dir/src/probes/Probes.scala"
  # PROBE_CRLF=1: the messages' lines separated by CR LF, the deprecation's and the inline error's alike.
  local nl='\n'
  if [ "${PROBE_CRLF:-}" = 1 ]; then nl='\r\n'; sed -i 's|("sample\\nPROJECT|("sample\\r\\nPROJECT|; s|("sample\\n\([a-z]*\)/test|("sample\\r\\n\1/test|' "$dir/src/probes/Probes.scala"; fi
  if [ -n "$inline" ]; then
    mkdir -p "$dir/main/probes"
    printf '%s\n' 'package probes' '' 'object Message:' "  inline def broken: Unit = scala.compiletime.error(\"prefix${nl}Example.scala:1:1: warning: note${nl}$inline\")" '' 'object Use:' '  def run(): Unit = Message.broken' > "$dir/main/probes/Message.scala"
  elif [ "$generated" = compile ]; then
    mkdir -p "$dir/main/probes"
    printf '%s\n' 'package probes' '' 'object Plain:' '  def value: Int = 1' > "$dir/main/probes/Plain.scala"
  fi
  if [ -n "$server" ]; then
    mkdir -p "$dir/main/probes"
    echo hello > "$dir/message.txt"
    cat > "$dir/main/probes/Server.scala" <<'SCALA'
package probes

import java.net.ServerSocket
import java.nio.file.{Files, Paths}

object Server:
  def main(args: Array[String]): Unit =
    val server = new ServerSocket(9663)
    print(Files.readString(Paths.get("message.txt")))
    System.out.flush()
    while true do
      val c = server.accept()
      try
        val body = "<title>lichess</title><script src=\"/assets/app.js\"></script>".getBytes("UTF-8")
        val out = c.getOutputStream()
        out.write(("HTTP/1.1 200 OK\r\nContent-Length: " + body.length + "\r\nConnection: close\r\n\r\n").getBytes("UTF-8"))
        out.write(body)
        out.flush()
      catch case _: java.io.IOException => ()
      c.close()
SCALA
  fi
  if [ -n "$generated" ]; then
    local ended='helper/test: no suite to run'
    [ "$generated" = test ] || ended='helper/compile: checked in 1 ms'
    printf '%s\n' '#!/bin/sh' 'echo generator started' "echo '$ended'" 'echo OLD' 'exit 1' > "$dir/gen.sh"
  fi
  python3 - "$dir" "$project" "$lib" "$lib3" "$interface" "$inline" "$generated" "$server" <<'PY'
import hashlib, importlib.util, os, sys
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
root, project, lib, lib3, interface, inline, generated, server = sys.argv[1:]
m2 = lib[: lib.index("/maven2/") + len("/maven2/")]
def artifact(path):
    data = open(path, "rb").read()
    parts = path[len(m2):].split("/")
    return ":".join([".".join(parts[:-3]), parts[-3], parts[-2]]), f"maven-central {hashlib.sha1(data).hexdigest()} {len(data)}"
table = dict([artifact(lib3), artifact(lib), artifact(interface)])
flags = {"deprecation": True, "ignoredScalacOptions": [], "kindProjector": False, "strictEquality": False, "werror": False}
compile = {"classpath": [k for k in table if "test-interface" not in k], "flags": {"ignoredScalacOptions": [], "kindProjector": False, "strictEquality": False, "werror": False},
           "generators": [], "mainClasses": [], "resources": [], "sources": ["main"]}
compiled = inline or generated == "compile" or server
if server:
    compile["mainClasses"] = ["probes.Server"]
test = {"classpath": ([{"configuration": "compile", "project": project}] if compiled else []) + list(table) + [{"file": "lib/probe.jar"}], "flags": flags, "resources": [], "sources": ["src"], "baseDirectory": ".",
        "envVars": {}, "exclude": [], "fork": True, "javaOptions": [], "frameworks": [{"arguments": ["record.txt"], "class": "probe.Framework"}]}
# The generator failing with the lines that read as its configuration's end (src/task/generators.rs `command`).
if generated:
    {"compile": compile, "test": test}[generated]["generators"] = [{"cwd": ".", "inputs": ["gen.sh"], "kind": "command", "outputs": ["target/generated"], "run": ["sh", "gen.sh"]}]
export = {"teq": "0.1.8", "format": 1, "binaries": {}, "inputs": {"files": {}, "sha256": hashlib.sha256(b"").hexdigest()}, "java": {"outputVersion": 17},
          "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}], "jars": table,
          "projects": {project: {"base": ".", "platform": "jvm", "scalaVersion": "3.8.4", "configurations": {**({"compile": compile} if compiled else {}), "test": test,
                                 **({"runtime": {"classpath": [{"configuration": "compile", "project": project}] + [k for k in table if "test-interface" not in k], "resources": [], "sources": []}} if server else {})},
                                 **({"run": {"aliases": {}, "baseDirectory": ".", "envVars": {}, "javaOptions": [], **({"mainClass": "probes.Server"} if server else {})}} if inline or server else {})}}}
open(os.path.join(root, "target/teq/teq.lock"), "w", encoding="utf-8").write(lock.canonical(export))
PY
}
