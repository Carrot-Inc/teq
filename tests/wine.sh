#!/bin/bash
# tests/wine.sh: the smoke of the Windows binary under wine on a Linux machine, TEQ_EXE=<teq.exe>
# (target/cross/windows/x86_64-pc-windows-gnullvm/ship/teq.exe, bench/cross-ship.sh windows, by
# default). Every path the binary is given is a Windows one, a drive letter and backslashes
# (`winepath -w`: wine's drive Z: is the Linux root), and every program runs in a prefix of its own
# (WINEPREFIX, target/wine by default, made on the first run), with neither HOME nor COURSIER_CACHE.
# It is wine's evidence and no more: wine on a Linux file system has neither NTFS's sharing rules
# nor its case folding nor a Windows console, which the maintainer's run on Windows covers
# (docs/TARGETS.md, "Windows"). The JavaScript the binary builds runs on this machine's node, and
# the class files on its java.
#
# The checks: the version; a check of a type error in a directory with a space, and one under a
# path past 260 characters; builds of three
# cases run on node; a JVM build whose class path names scala-library by a backslash path, and one
# that finds it in the coursier cache under %LOCALAPPDATA%; a macro reading a file beside the
# file it expands in (`getJPath`, `getParent`, `resolve`, under a directory with a space and a
# non-ASCII character), defined in the build and loaded from a library's products; a macro's
# `Path.resolve` of operands spelt with `/`, against scalac's answers on Windows; `teq lsp` driven with Windows URIs (tests/wine/lsp.mjs), and its idle session's polls over a minute (its `idle` mode); `teq compiler watch` over
# a pipe: a cancel read at the commit point, a command half there at it, the input's end at it;
# `teq compile` over an export of the jars of the coursier cache. Prints a line per failure, the
# idle minute's (its polls' count, spacing and time) and, last, `wine: passed: <n> of <n>`; exits 1
# when any failed.
cd "$(dirname "$0")/.." || exit 1
TEQ_EXE=${TEQ_EXE:-target/cross/windows/x86_64-pc-windows-gnullvm/ship/teq.exe}
[ -f "$TEQ_EXE" ] || { echo "wine: no binary at $TEQ_EXE"; exit 2; }
TEQ_EXE=$(cd "$(dirname "$TEQ_EXE")" && pwd)/$(basename "$TEQ_EXE")
command -v wine > /dev/null || { echo "wine: no wine; apt-get install wine64"; exit 2; }
export WINEPREFIX=${WINEPREFIX:-$PWD/target/wine} WINEDEBUG=${WINEDEBUG:--all}
# wine spells a Linux file name in Windows by the locale's character set: UTF-8's, whatever this
# machine's locale is (an absent one is ASCII's, and `é` would come out as `C)`).
export LANG=C.UTF-8 LC_ALL=C.UTF-8
m2=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}/https/repo1.maven.org/maven2
unset COURSIER_CACHE
work=$(mktemp -d)
WPID=
trap '[ -n "$WPID" ] && kill $WPID 2> /dev/null; wineserver -k 2> /dev/null; rm -rf "$work"' EXIT
# One wine server for the run, so that each program starts in a second, not in seven.
wineserver -p 2> /dev/null
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
# A Linux path's Windows one by wine's drive Z:, the Linux root (winepath mangles a path past 260
# characters).
win() {
  local path
  path=$(realpath -m "$1")
  echo "Z:${path//\//\\}"
}
# teq <args...>: the binary under wine, bounded, without HOME, its stdout and stderr in $work/out
# and $work/err.
teq() {
  env -u HOME timeout 120 wine "$TEQ_EXE" "$@" > "$work/out" 2> "$work/err"
}

# The version, as a plain build stamps it.
version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
teq --version
if [ $? -eq 0 ] && grep -qx "teq $version [0-9a-f]*" "$work/out"; then ok; else bad "--version: $(cat "$work/out" "$work/err")"; fi

# A check of a program with a type error, its file named by a Windows path: the error names the
# file as given and the exit code is 1.
mkdir -p "$work/check dir"
printf 'object Main {\n  def main(args: Array[String]): Unit = {\n    val n: Int = "one"\n  }\n}\n' > "$work/check dir/Bad.scala"
teq compiler check "$(win "$work/check dir/Bad.scala")"
status=$?
if [ $status -eq 1 ] && grep -qF "$(win "$work/check dir/Bad.scala")" "$work/err" && grep -q "type mismatch: found String, required Int" "$work/err"; then ok; else bad "check of a type error (exit $status): $(head -5 "$work/err")"; fi

# A check under a directory whose path is past 260 characters, Windows' old bound, which std's
# file operations pass in the extended-length form.
deep=$work/$(printf 'directory-of-forty-characters-%010d/' 1 2 3 4 5 6 7)
mkdir -p "$deep"
printf 'object Deep:\n  val n: Int = "one"\n' > "$deep/Deep.scala"
teq compiler check "$(win "$deep/Deep.scala")"
status=$?
if [ ${#deep} -gt 260 ] && [ $status -eq 1 ] && grep -q "type mismatch: found String, required Int" "$work/err"; then ok; else bad "a check past 260 characters of path (${#deep}, exit $status): $(head -3 "$work/err")"; fi

# A build of a case to one JavaScript file, run on node: the case's expected output.
for name in collections collect_lambda_match inline_match; do
  src=tests/cases/$name.scala
  [ -f "$src" ] || continue
  teq compiler build "$(win "$src")" -o "$(win "$work/$name.js")"
  status=$?
  if [ $status -eq 0 ] && timeout 20 node "$work/$name.js" > "$work/$name.actual" 2>&1 && diff -q "tests/cases/$name.expected" "$work/$name.actual" > /dev/null; then ok; else bad "build of $name (exit $status): $(head -3 "$work/err")"; fi
done

# A JDK as a Windows machine has one: bin\java.exe on the PATH (wine's: WINEPATH), lib\ct.sym the
# class files teq reads (this machine's JDK's; nothing runs java.exe).
jdk=$work/jdk
mkdir -p "$jdk/bin" "$jdk/lib"
cp "$(dirname "$(dirname "$(readlink -f "$(command -v java)")")")/lib/ct.sym" "$jdk/lib/" && : > "$jdk/bin/java.exe"
export WINEPATH
WINEPATH=$(win "$jdk/bin")

# A JVM build with scala-library named by its Windows path, which the driver must see as the
# library (no discovery: neither HOME nor a cache), its class files run on java; then the
# discovery from the coursier cache under %LOCALAPPDATA%, the prefix's.
lib=$m2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
lib3=$m2/org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar
if [ -f "$lib" ]; then
  printf 'object A {\n  def f(x: Int): Int = x + 1\n  def main(args: Array[String]): Unit = println(f(1))\n}\n' > "$work/A.scala"
  mkdir -p "$work/classes"
  teq compiler build --target jvm --classpath "$(win "$lib")" "$(win "$work/A.scala")" -o "$(win "$work/classes")"
  status=$?
  if [ $status -eq 0 ] && [ "$(timeout 30 java -cp "$work/classes:$lib" A 2>&1)" = 2 ]; then ok; else bad "a JVM build with a backslash class path (exit $status): $(head -3 "$work/err")"; fi
  local_app=$(winepath -u "$(env -u HOME wine cmd /c echo %LOCALAPPDATA% 2> /dev/null | tr -d '\r')" 2> /dev/null)
  cache=$local_app/Coursier/cache/v1/https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4
  mkdir -p "$cache" && cp "$lib" "$cache/"
  teq compiler check --target jvm "$(win "$work/A.scala")"
  status=$?
  rm -rf "$local_app/Coursier"
  if [ $status -eq 0 ]; then ok; else bad "scala-library found under %LOCALAPPDATA% (exit $status): $(head -3 "$work/err")"; fi
else
  bad "no scala-library 3.8.4 in the coursier cache ($lib)"
fi

# A macro defined in the build reading data.txt beside the file it expands in, and saying what the
# path's operations give: the answers of the JDK's WindowsPath.
mdir="$work/src dir é"
mkdir -p "$mdir"
cat > "$work/Macros.scala" << 'EOF'
import scala.quoted.*

object DataMacro:
  inline def data: String = ${ dataImpl }
  def dataImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val source = Position.ofMacroExpansion.sourceFile.getJPath.get
    val text = java.nio.file.Files.readString(source.getParent.resolve("data.txt")).trim
    Expr(text + " | " + source.getFileName + " | " + source.isAbsolute + " | " + source.getParent.getFileName + " | " + source.getParent.resolve("data.txt"))
EOF
printf 'object Use:\n  def main(args: Array[String]): Unit = println(DataMacro.data)\n' > "$mdir/Use.scala"
echo "the sibling's data" > "$mdir/data.txt"
teq compiler build "$(win "$work/Macros.scala")" "$(win "$mdir/Use.scala")" -o "$(win "$work/macro.js")"
status=$?
want="the sibling's data | Use.scala | true | src dir é | $(win "$mdir")\\data.txt"
got=$(timeout 20 node "$work/macro.js" 2>&1)
if [ $status -eq 0 ] && [ "$got" = "$want" ]; then ok; else bad "a macro reading a sibling file (exit $status): $got $(head -3 "$work/err"), wanted $want"; fi
# Path.resolve's operands written as WindowsPathParser writes them, and a share's root under runs of
# separators: what scalac 3.8.4's macro prints on a Windows JDK under wine.
cat > "$work/WinMatrix.scala" << 'EOF'
import scala.quoted.*

object WinMatrix:
  inline def result: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    val base = java.nio.file.Path.of("C:/work")
    val share = java.nio.file.Path.of("//server/share")
    val deep = java.nio.file.Path.of("////server/share/dir").getParent.getFileName
    Expr(base.resolve("sub/data.txt").toString + "|" + base.resolve("D:/other/data.txt").toString + "|" +
      base.resolve("//server/share/data.txt").toString + "|" + share.resolve("/data.txt").toString + "|" +
      (if deep == null then "null" else deep.toString) + "|" + java.nio.file.Path.of("//server//share").toString)
EOF
printf 'object WinUse:\n  def main(args: Array[String]): Unit = println(WinMatrix.result)\n' > "$work/WinUse.scala"
teq compiler build "$(win "$work/WinMatrix.scala")" "$(win "$work/WinUse.scala")" -o "$(win "$work/winmatrix.js")"
status=$?
resolved=$(timeout 20 node "$work/winmatrix.js" 2>&1)
scalac='C:\work\sub\data.txt|D:\other\data.txt|\\server\share\data.txt|\\server\share\data.txt|null|\\server\share\'
if [ $status -eq 0 ] && [ "$resolved" = "$scalac" ]; then ok; else bad "Path.resolve's operands (exit $status): $resolved, scalac's $scalac"; fi

# The same macro from a library: checked into a module's products (its TASTy), then loaded from
# them by the build of the use, whose class path, two entries, takes `;` (a JVM library calling
# getJPath fails on every platform: its std's java.nio.file.Path is not found, reported apart).
mkdir -p "$work/macrolib" "$work/empty"
teq compiler check --products "$(win "$work/macrolib")" --sourceroot "$(win "$work")" "$(win "$work/Macros.scala")"
status=$?
[ $status -eq 0 ] && teq compiler build --classpath "$(win "$work/macrolib");$(win "$work/empty")" "$(win "$mdir/Use.scala")" -o "$(win "$work/libmacro.js")"
status=$?
got=$(timeout 20 node "$work/libmacro.js" 2>&1)
if [ $status -eq 0 ] && [ "$got" = "$want" ]; then ok; else bad "a library's macro reading a sibling file (exit $status): $got $(head -3 "$work/err"), wanted $want"; fi

# teq lsp, driven as a Windows editor drives it.
lws="$work/ws dir é"
mkdir -p "$lws" && cp tests/lsp/bare/Hello.scala "$lws/"
out=$(env -u HOME timeout 300 node tests/wine/lsp.mjs "$TEQ_EXE" "$lws" 2>&1)
if [ $? -eq 0 ]; then ok; else bad "teq lsp with Windows URIs: $out"; fi
# Its idle session over a minute, the polls read from its trace.
iws="$work/idle ws"
mkdir -p "$iws"
out=$(COURSIER_CACHE=$(win "${m2%/https/repo1.maven.org/maven2}") env -u HOME timeout 200 node tests/wine/lsp.mjs "$TEQ_EXE" "$iws" idle 2>&1)
if [ $? -eq 0 ]; then ok; else bad "teq lsp's idle minute: $out"; fi
echo "$out" | grep '^lsp: an idle minute'

# teq compiler watch over a pipe, a build held at its commit point (TEQ_LSP_TEST_BARRIER) while the test
# writes what the session must find there. The pipe is this shell's, a Unix pipe, which wine gives
# the program as one it cannot peek (ERROR_NOT_SUPPORTED), so the session relays it into a pipe of
# its own (the pipe a Windows process makes is tests/wine/lsp.mjs's): a cancel of the build, which
# it answers cancelled; a command half written, which waits for its end and lets the build answer;
# the input's end, after which the build answers and the session ends.
wdir=$work/watch
mkdir -p "$wdir"
edit() { printf 'object W {\n  def f: Int = %s\n  def main(args: Array[String]): Unit = println(f)\n}\n' "$1" > "$wdir/W.scala"; }
edit 1
mkfifo "$work/cmd" "$work/ans"
barrier=$work/barrier
TEQ_LSP_TEST_BARRIER=$(win "$barrier") TEQ_LSP_TRACE_FILE=$(win "$work/trace") env -u HOME timeout 300 wine "$TEQ_EXE" compiler watch --check "$(win "$wdir/W.scala")" < "$work/cmd" > "$work/ans" 2> "$work/watch.err" &
WPID=$!
exec 3> "$work/cmd" 4< "$work/ans"
read -r -t 120 answer <&4
[[ $answer == *'"ok":true'* ]] || bad "watch: the first build: $answer $(head -3 "$work/watch.err")"
held() {
  local until=$((SECONDS + 60))
  until grep -q "held $1 commit" "$work/trace" 2> /dev/null; do
    [ $SECONDS -lt $until ] || return 1
    sleep 0.1
  done
}
w=$(win "$wdir/W.scala")
: > "$barrier.commit"
edit 2
printf 'build #1 %s\n\n' "$w" >&3
held 1 && printf 'cancel 1\n' >&3
sleep 0.3
rm -f "$barrier.commit"
read -r -t 120 answer <&4
if [[ $answer == *'"cancelled":true'*'"build":1'* ]]; then ok; else bad "watch: a cancel read at the commit point: $answer"; fi
: > "$barrier.commit"
edit 3
printf 'build #2 %s\n\n' "$w" >&3
held 2 && printf 'canc' >&3
sleep 0.3
rm -f "$barrier.commit"
read -r -t 120 answer <&4
if [[ $answer == *'"build":2'* && $answer != *cancelled* ]]; then ok; else bad "watch: a command half there at the commit point: $answer"; fi
# Its end: `cancel 2`, which names no build under way and answers nothing; then a build.
printf 'el 2\n' >&3
edit 4
printf 'build #3 %s\n\n' "$w" >&3
read -r -t 120 answer <&4
if [[ $answer == *'"build":3'* && $answer == *'"ok":true'* ]]; then ok; else bad "watch: the build after the command's end: $answer"; fi
: > "$barrier.commit"
edit 5
printf 'build #4 %s\n\n' "$w" >&3
held 4 && exec 3>&-
sleep 0.3
rm -f "$barrier.commit"
read -r -t 120 answer <&4
wait $WPID
status=$?
WPID=
if [[ $answer == *'"build":4'* ]] && [ $status -eq 0 ]; then ok; else bad "watch: the input's end at the commit point (exit $status): $answer"; fi
exec 4<&-

# teq compile over a lock of one project (teq.lock, written by the example's check-export.py
# as tests/task.sh writes one), its jars from the coursier cache (no sbt).
if [ -f "$lib" ] && [ -f "$lib3" ]; then
  tdir=$work/build
  mkdir -p "$tdir/core/src/core" "$tdir/project"
  printf 'package core\n\nobject Core:\n  def answer: Int = 42\n' > "$tdir/core/src/core/Core.scala"
  echo 'lazy val core = project' > "$tdir/build.sbt"
  echo 'sbt.version=2.0.8' > "$tdir/project/build.properties"
  LOCK_PY=$PWD/integrations/sbt/example/check-export.py python3 - "$tdir" "$lib" "$lib3" "$version" << 'PY'
import hashlib, importlib.util, os, sys
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
b, lib, lib3, version = sys.argv[1:]
def artifact(path):
    data = open(path, "rb").read()
    parts = path[path.index("/maven2/") + len("/maven2/"):].split("/")
    return ":".join([".".join(parts[:-3]), parts[-3], parts[-2]]), f"maven-central {hashlib.sha1(data).hexdigest()} {len(data)}"
table = dict([artifact(lib3), artifact(lib)])
inputs = {f: hashlib.sha256(open(os.path.join(b, f), "rb").read()).hexdigest() for f in ["build.sbt", "project/build.properties"]}
flags = {"ignoredScalacOptions": [], "kindProjector": False, "maxInlines": 32, "strictEquality": False, "werror": False}
export = {
    "teq": version,
    "format": 1,
    "binaries": {},
    "inputs": {"files": inputs, "sha256": hashlib.sha256("".join(f"{k}\0{v}\n" for k, v in sorted(inputs.items())).encode()).hexdigest()},
    "java": {"outputVersion": 17},
    "projects": {"core": {"base": "core", "platform": "jvm", "scalaVersion": "3.8.4",
                          "configurations": {"compile": {"classpath": list(table), "flags": flags, "generators": [], "mainClasses": [], "resources": [], "sources": ["core/src"]}}}},
    "jars": table,
    "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}],
}
open(os.path.join(b, "teq.lock"), "w", encoding="utf-8").write(lock.canonical(export))
PY
  task() { (cd "$tdir" && COURSIER_CACHE=$(win "${m2%/https/repo1.maven.org/maven2}") TEQ_CACHE_DIR=$(win "$work/teq-cache") env -u HOME timeout 180 wine "$TEQ_EXE" "$@") > "$work/out" 2> "$work/err"; }
  task compile
  status=$?
  classes=$(find "$tdir" -name 'Core$.class' | head -1)
  if [ $status -eq 0 ] && [ -n "$classes" ]; then ok; else bad "teq compile (exit $status): $(head -5 "$work/out" "$work/err")"; fi
  task stop
fi

echo "wine: passed: $pass of $((pass + fail))"
[ $fail = 0 ]
