#!/bin/bash
# tests/windows.sh: the smoke of the Windows binary on Windows itself, TEQ_EXE=<teq.exe>, in Git for Windows' bash
# (the release workflow's Windows job, bench/actions/qualify-windows.sh; docs/DEVELOPING.md, "Releases"). The
# native counterpart of tests/wine.sh, whose checks it runs where Windows answers them (NTFS, the Windows JDK's
# paths, Windows pipes) and whose evidence the binary's manifest keeps, wine's smoke still running at the build:
# the version; a check of a type error in a directory with a space, and one under a path past 260 characters; builds
# of three cases run on node; a JVM build whose class path names scala-library by a backslash path, its classes run
# on java, and one that finds it in the coursier cache under %LOCALAPPDATA%; a macro reading a file beside the file
# it expands in, under a directory with a space and a non-ASCII character, defined in the build and loaded from a
# library's products by a class path of two entries; a macro's `Path.resolve`, against scalac's answers on Windows;
# `teq lsp` driven with Windows URIs (tests/wine/lsp.mjs, natively) and its idle minute; `teq compile` over a lock of
# one project (the build tool); a generator of that project that the build's own binary runs (`teq interp`, the sbt
# example's images script), the binary a copy under a directory with a space and sentinels first on the PATH; and the launcher teq.cmd run by cmd.exe with a lock pinning the binary at a local
# URL: a cold cache fetches it with curl.exe and runs it, a warm one runs it with the server gone, a lock pinning
# another sha1 is refused and places nothing. `teq compiler watch` over a pipe stays wine's: Git's bash makes no
# Windows pipe of a FIFO; a console's Ctrl-C stays the console's user's. Every path the binary is given is a Windows one
# (cygpath -w). Prints a line per failure and, last, `windows: passed: <n> of <n>`; exits 1 when any failed.
cd "$(dirname "$0")/.." || exit 1
case $(uname -s) in MINGW* | MSYS*) ;; *) echo "windows: this is $(uname -s), not Windows (Git's bash)"; exit 2 ;; esac
[ -f "${TEQ_EXE:-}" ] || { echo "windows: no binary at TEQ_EXE=${TEQ_EXE:-}"; exit 2; }
TEQ_EXE=$(cd "$(dirname "$TEQ_EXE")" && pwd)/$(basename "$TEQ_EXE")
py=$(command -v python3 || command -v python) || { echo "windows: no python"; exit 2; }
for tool in node java curl cygpath; do command -v $tool > /dev/null || { echo "windows: no $tool"; exit 2; }; done
export LANG=C.UTF-8 LC_ALL=C.UTF-8
# Paths go to Windows programs as Windows paths, never rewritten by MSYS.
export MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*'
m2=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}/https/repo1.maven.org/maven2
unset COURSIER_CACHE
work=$(mktemp -d)
mirror=
trap '[ -z "$mirror" ] || kill "$mirror" 2> /dev/null; rm -rf -- "$work"' EXIT
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
win() { cygpath -w "$1"; }
# teq <args...>: the binary, bounded, without HOME, its stdout and stderr in $work/out and $work/err.
teq() { env -u HOME timeout 120 "$TEQ_EXE" "$@" > "$work/out" 2> "$work/err"; }
# sha1 <file>: its SHA-1, by Windows' python, which reads a Windows path.
sha1() { "$py" -c 'import hashlib, sys; print(hashlib.sha1(open(sys.argv[1], "rb").read()).hexdigest())' "$(win "$1")"; }
# need <maven path>: the jar under $m2, fetched from Maven Central by its SHA-1 when missing; curl.exe writes
# a Windows path alone (a POSIX one, not rewritten under MSYS_NO_PATHCONV, is curl's error 23).
need() {
  [ -f "$m2/$1" ] && return 0
  mkdir -p "$(dirname "$m2/$1")" && timeout 300 curl -sSfLo "$(win "$m2/$1.part")" "https://repo1.maven.org/maven2/$1" &&
    [ "$(sha1 "$m2/$1.part")" = "$(timeout 60 curl -sSfL "https://repo1.maven.org/maven2/$1.sha1" | cut -c1-40)" ] &&
    mv -f "$m2/$1.part" "$m2/$1"
}

version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
teq --version
if [ $? -eq 0 ] && grep -qx "teq $version [0-9a-f]*" <(tr -d '\r' < "$work/out"); then ok; else bad "--version: $(cat "$work/out" "$work/err")"; fi

mkdir -p "$work/check dir"
printf 'object Main {\n  def main(args: Array[String]): Unit = {\n    val n: Int = "one"\n  }\n}\n' > "$work/check dir/Bad.scala"
teq compiler check "$(win "$work/check dir/Bad.scala")"
status=$?
if [ $status -eq 1 ] && grep -qF "$(win "$work/check dir/Bad.scala")" "$work/err" && grep -q "type mismatch: found String, required Int" "$work/err"; then ok; else bad "check of a type error (exit $status): $(head -5 "$work/err")"; fi

deep=$work/$(printf 'directory-of-forty-characters-%010d/' 1 2 3 4 5 6 7)
mkdir -p "$deep"
printf 'object Deep:\n  val n: Int = "one"\n' > "$deep/Deep.scala"
teq compiler check "$(win "$deep/Deep.scala")"
status=$?
if [ "$(win "$deep" | wc -c)" -gt 260 ] && [ $status -eq 1 ] && grep -q "type mismatch: found String, required Int" "$work/err"; then ok; else bad "a check past 260 characters of path (exit $status): $(head -3 "$work/err")"; fi

for name in collections collect_lambda_match inline_match; do
  src=tests/cases/$name.scala
  [ -f "$src" ] || continue
  teq compiler build "$(win "$src")" -o "$(win "$work/$name.js")"
  status=$?
  if [ $status -eq 0 ] && timeout 20 node "$(win "$work/$name.js")" > "$work/$name.actual" 2>&1 && diff -q --strip-trailing-cr "tests/cases/$name.expected" "$work/$name.actual" > /dev/null; then ok; else bad "build of $name (exit $status): $(head -3 "$work/err")"; fi
done

lib_path=org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
lib3_path=org/scala-lang/scala3-library_3/3.8.4/scala3-library_3-3.8.4.jar
need "$lib_path" && need "$lib3_path" || bad "scala-library 3.8.4 and scala3-library_3 3.8.4 could not be had from Maven Central"
lib=$m2/$lib_path lib3=$m2/$lib3_path
if [ -f "$lib" ]; then
  printf 'object A {\n  def f(x: Int): Int = x + 1\n  def main(args: Array[String]): Unit = println(f(1))\n}\n' > "$work/A.scala"
  mkdir -p "$work/classes"
  teq compiler build --target jvm --classpath "$(win "$lib")" "$(win "$work/A.scala")" -o "$(win "$work/classes")"
  status=$?
  if [ $status -eq 0 ] && [ "$(timeout 30 java -cp "$(win "$work/classes");$(win "$lib")" A 2>&1 | tr -d '\r')" = 2 ]; then ok; else bad "a JVM build with a backslash class path (exit $status): $(head -3 "$work/err")"; fi
  # The discovery from coursier's cache under %LOCALAPPDATA%, the file put there for the check alone.
  local_app=$(cygpath -u "$LOCALAPPDATA")
  cache=$local_app/Coursier/cache/v1/https/repo1.maven.org/maven2/org/scala-lang/scala-library/3.8.4
  placed=
  [ -f "$cache/scala-library-3.8.4.jar" ] || { mkdir -p "$cache" && cp "$lib" "$cache/" && placed=1; }
  teq compiler check --target jvm "$(win "$work/A.scala")"
  status=$?
  [ -z "$placed" ] || rm -f -- "$cache/scala-library-3.8.4.jar"
  if [ $status -eq 0 ]; then ok; else bad "scala-library found under %LOCALAPPDATA% (exit $status): $(head -3 "$work/err")"; fi
fi

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
got=$(timeout 20 node "$(win "$work/macro.js")" 2>&1 | tr -d '\r')
if [ $status -eq 0 ] && [ "$got" = "$want" ]; then ok; else bad "a macro reading a sibling file (exit $status): $got $(head -3 "$work/err"), wanted $want"; fi
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
resolved=$(timeout 20 node "$(win "$work/winmatrix.js")" 2>&1 | tr -d '\r')
scalac='C:\work\sub\data.txt|D:\other\data.txt|\\server\share\data.txt|\\server\share\data.txt|null|\\server\share\'
if [ $status -eq 0 ] && [ "$resolved" = "$scalac" ]; then ok; else bad "Path.resolve's operands (exit $status): $resolved, scalac's $scalac"; fi
mkdir -p "$work/macrolib" "$work/empty"
teq compiler check --products "$(win "$work/macrolib")" --sourceroot "$(win "$work")" "$(win "$work/Macros.scala")"
status=$?
[ $status -eq 0 ] && teq compiler build --classpath "$(win "$work/macrolib");$(win "$work/empty")" "$(win "$mdir/Use.scala")" -o "$(win "$work/libmacro.js")"
status=$?
got=$(timeout 20 node "$(win "$work/libmacro.js")" 2>&1 | tr -d '\r')
if [ $status -eq 0 ] && [ "$got" = "$want" ]; then ok; else bad "a library's macro reading a sibling file (exit $status): $got $(head -3 "$work/err"), wanted $want"; fi

lws="$work/ws dir é"
mkdir -p "$lws" && cp tests/lsp/bare/Hello.scala "$lws/"
out=$(env -u HOME timeout 300 node tests/wine/lsp.mjs "$(win "$TEQ_EXE")" "$(win "$lws")" 2>&1)
if [ $? -eq 0 ]; then ok; else bad "teq lsp with Windows URIs: $out"; fi
iws="$work/idle ws"
mkdir -p "$iws"
out=$(COURSIER_CACHE=$(win "${m2%/https/repo1.maven.org/maven2}") env -u HOME timeout 200 node tests/wine/lsp.mjs "$(win "$TEQ_EXE")" "$(win "$iws")" idle 2>&1)
if [ $? -eq 0 ]; then ok; else bad "teq lsp's idle minute: $out"; fi
echo "$out" | grep '^lsp: an idle minute'

if [ -f "$lib" ] && [ -f "$lib3" ]; then
  tdir=$work/build
  mkdir -p "$tdir/core/src/core" "$tdir/project"
  printf 'package core\n\nobject Core:\n  def answer: Int = 42\n' > "$tdir/core/src/core/Core.scala"
  echo 'lazy val core = project' > "$tdir/build.sbt"
  echo 'sbt.version=2.0.8' > "$tdir/project/build.properties"
  LOCK_PY=$(win "$PWD/integrations/sbt/example/check-export.py") "$py" - "$(win "$tdir")" "$(win "$lib")" "$(win "$lib3")" "$version" << 'PY'
import hashlib, importlib.util, os, sys
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
b, lib, lib3, version = sys.argv[1:]
def artifact(path):
    data = open(path, "rb").read()
    norm = path.replace("\\", "/")
    parts = norm[norm.index("/maven2/") + len("/maven2/"):].split("/")
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
open(os.path.join(b, "teq.lock"), "w", encoding="utf-8", newline="\n").write(lock.canonical(export))
PY
  task() { (cd "$tdir" && COURSIER_CACHE=$(win "${m2%/https/repo1.maven.org/maven2}") TEQ_CACHE_DIR=$(win "$work/teq-cache") env -u HOME timeout 180 "$TEQ_EXE" "$@") > "$work/out" 2> "$work/err"; }
  task compile
  status=$?
  classes=$(find "$tdir" -name 'Core$.class' | head -1)
  if [ $status -eq 0 ] && [ -n "$classes" ]; then ok; else bad "teq compile (exit $status): $(head -5 "$work/out" "$work/err")"; fi
  task stop

  # The same project with a generator, integrations/sbt/example's images script over its images, run as
  # `teq interp`: the word `teq` is the binary running, here a copy under a directory with a space, never a teq
  # the PATH finds through PATHEXT. First on the PATH a `teq` and a `scala-cli` as an .exe that fails (a copy
  # of Git's false.exe) and as a .cmd that leaves a line in a marker and fails, each directory first in turn;
  # each run has the outputs and the fingerprint record gone and no daemon left. The objects compiled, the
  # module beside the images written, the marker absent; then the module deleted alone is written again.
  gdir="$work/gen build"
  bin="$work/bin dir"
  mkdir -p "$gdir/core/src/core" "$gdir/project" "$gdir/scripts" "$gdir/app/assets/images" "$bin" "$work/sentinel-exe" "$work/sentinel-cmd"
  cp "$TEQ_EXE" "$bin/teq.exe"
  cp integrations/sbt/example/scripts/images.scala "$gdir/scripts/"
  cp -r integrations/sbt/example/browserdemo-images/assets/images/shapes integrations/sbt/example/browserdemo-images/assets/images/marks "$gdir/app/assets/images/"
  printf 'package core\n\nobject Core:\n  def shapes: Seq[String] = demo.images.ShapeImages.all\n' > "$gdir/core/src/core/Core.scala"
  echo 'lazy val core = project' > "$gdir/build.sbt"
  echo 'sbt.version=2.0.8' > "$gdir/project/build.properties"
  marker=$work/sentinel-ran
  # The external false, Git's false.exe, not Bash's builtin; the setup fails without both sentinels of each name.
  false_exe=$(type -P false)
  [ -f "$false_exe.exe" ] && false_exe=$false_exe.exe
  sentinels=1
  for name in teq scala-cli; do
    cp "$false_exe" "$work/sentinel-exe/$name.exe" || sentinels=
    printf '@echo %%~nx0 %%* >> "%s"\r\n@exit /b 73\r\n' "$(win "$marker")" > "$work/sentinel-cmd/$name.cmd" || sentinels=
    [ -f "$work/sentinel-exe/$name.exe" ] && [ -f "$work/sentinel-cmd/$name.cmd" ] || sentinels=
  done
  if [ -z "$sentinels" ]; then
    bad "the generator's sentinels: no $work/sentinel-exe/teq.exe and scala-cli.exe from false (${false_exe:-not found}), or no .cmd beside them"
  else
    LOCK_PY=$(win "$PWD/integrations/sbt/example/check-export.py") "$py" - "$(win "$gdir")" "$(win "$lib")" "$(win "$lib3")" "$version" << 'PY'
import hashlib, importlib.util, os, sys
spec = importlib.util.spec_from_file_location("lock", os.environ["LOCK_PY"])
lock = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lock)
b, lib, lib3, version = sys.argv[1:]
def artifact(path):
    data = open(path, "rb").read()
    norm = path.replace("\\", "/")
    parts = norm[norm.index("/maven2/") + len("/maven2/"):].split("/")
    return ":".join([".".join(parts[:-3]), parts[-3], parts[-2]]), f"maven-central {hashlib.sha1(data).hexdigest()} {len(data)}"
table = dict([artifact(lib3), artifact(lib)])
inputs = {f: hashlib.sha256(open(os.path.join(b, f), "rb").read()).hexdigest() for f in ["build.sbt", "project/build.properties"]}
flags = {"ignoredScalacOptions": [], "kindProjector": False, "maxInlines": 32, "strictEquality": False, "werror": False}
managed = "target/teq/core/compile/src_managed"
generator = {"cwd": ".", "inputs": ["app/assets/images/**/*.svg"], "kind": "command", "outputs": [managed, "app/assets/images.js"],
             "run": ["teq", "interp", "scripts/images.scala", "--", "app", managed]}
export = {
    "teq": version,
    "format": 1,
    "binaries": {},
    "inputs": {"files": inputs, "sha256": hashlib.sha256("".join(f"{k}\0{v}\n" for k, v in sorted(inputs.items())).encode()).hexdigest()},
    "java": {"outputVersion": 17},
    "projects": {"core": {"base": "core", "platform": "jvm", "scalaVersion": "3.8.4",
                          "configurations": {"compile": {"classpath": list(table), "flags": flags, "generators": [generator], "mainClasses": [], "resources": [], "sources": ["core/src", managed]}}}},
    "jars": table,
    "repositories": [{"id": "maven-central", "url": "https://repo1.maven.org/maven2/"}],
}
open(os.path.join(b, "teq.lock"), "w", encoding="utf-8", newline="\n").write(lock.canonical(export))
PY
    gen_task() { (cd "$gdir" && PATH="$1:$PATH" COURSIER_CACHE=$(win "${m2%/https/repo1.maven.org/maven2}") TEQ_CACHE_DIR=$(win "$work/teq-cache") env -u HOME -u TEQ timeout 180 "$bin/teq.exe" "${@:2}") > "$work/out" 2> "$work/err"; }
    module=$gdir/app/assets/images.js
    for first in exe cmd; do
      case $first in exe) path=$work/sentinel-exe:$work/sentinel-cmd ;; *) path=$work/sentinel-cmd:$work/sentinel-exe ;; esac
      gen_task "$path" stop
      rm -rf "$module" "$gdir/target/teq/generators" "$gdir/target/teq/core/compile/src_managed" "$marker"
      gen_task "$path" compile
      status=$?
      classes=$(find "$gdir" -name 'Core$.class' | head -1)
      if [ $status -eq 0 ] && [ -f "$module" ] && [ -f "$gdir/target/teq/core/compile/src_managed/demo/images/ShapeImages.scala" ] && [ -n "$classes" ] && [ ! -e "$marker" ]; then ok; else bad "teq compile's generator by the binary under a space, the $first sentinels first (exit $status, marker: $(cat "$marker" 2> /dev/null)): $(head -5 "$work/out" "$work/err")"; fi
    done
    rm -f "$module"
    gen_task "$path" compile
    status=$?
    if [ $status -eq 0 ] && [ -f "$module" ] && [ ! -e "$marker" ]; then ok; else bad "teq compile after the generator's module was deleted alone (exit $status): $(head -5 "$work/out" "$work/err")"; fi
    gen_task "$path" stop
  fi
fi

# teq.cmd, run by cmd.exe, against the landed mirror of a release (bench/release-mirror.py), the binary at its direct path:
# the launcher follows a redirect to https alone.
release=$version asset=teq-$version-windows-x86_64.exe
mkdir -p "$work/root/v$release" "$work/maven" "$work/l/build" "$work/l/wrong" "$work/l/coursier" && cp "$TEQ_EXE" "$work/root/v$release/$asset" || exit 1
timeout 600 "$py" bench/release-mirror.py "$(win "$work/port")" "$(win "$work/root")" "$(win "$work/maven")" 2> "$work/mirror.log" &
mirror=$!
for _ in $(seq 1 100); do [ -s "$work/port" ] && break; sleep 0.1; done
url=http://localhost:$(cat "$work/port" 2> /dev/null)/objects/releases/download/v$release/$asset
sha1=$(sha1 "$TEQ_EXE")
size=$(wc -c < "$TEQ_EXE" | tr -d ' ')
wrong=$(printf '%040d' 0)
for d in build wrong; do
  cp tools/launcher/teq.cmd "$work/l/$d/teq.cmd"
  s=$sha1
  [ $d = build ] || s=$wrong
  printf 'teq: %s\r\nbinaries:\r\n  windows-x86_64: %s %s %s\r\n' "$release" "$url" "$s" "$size" > "$work/l/$d/teq.lock"
done
cmd_teq() { env -u TEQ TEQ_CACHE_DIR="$(win "$work/l/cache-$1")" COURSIER_CACHE="$(win "$work/l/coursier")" timeout 300 cmd.exe /d /c "$(win "$work/l/$2/teq.cmd")" --version 2>&1 | tr -d '\r'; }
out=$(cmd_teq ok build)
if [ -s "$work/port" ] && grep -qx "teq $version [0-9a-f]*" <<< "$(tail -1 <<< "$out")"; then
  if [ -f "$work/l/cache-ok/bin/$sha1/$asset" ]; then ok; else bad "teq.cmd's cold fetch placed no $asset: $out"; fi
else
  bad "teq.cmd's cold fetch from $url: $out $(tail -3 "$work/mirror.log")"
fi
refused=$(cmd_teq wrong wrong)
if [[ $refused == *"where the lock pins $wrong"*"refused"* ]] && [ -z "$(find "$work/l/cache-wrong" -type f 2> /dev/null)" ]; then ok; else bad "teq.cmd's refusal of another sha1: $refused"; fi
kill "$mirror" 2> /dev/null
wait "$mirror" 2> /dev/null
mirror=
out=$(cmd_teq ok build)
if grep -qx "teq $version [0-9a-f]*" <<< "$out"; then ok; else bad "teq.cmd's warm run, the server gone: $out"; fi

echo "windows: passed: $pass of $((pass + fail))"
[ $fail = 0 ]
