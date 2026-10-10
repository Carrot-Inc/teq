#!/bin/bash
# mill-teq's export of a Mill build (docs/TOOLING.md, "The export of a Mill build") over the fixture
# tests/mill-export/fixture, under each Mill of MILL_VERSIONS (default "1.0.6 1.1.10", the oldest Mill 1.x
# the plugin serves and the newest it is checked on): the plugin built from integrations/mill by Mill and
# published under a version of the test's own into a Maven directory of its own, imported by the build's
# command line (`mill --import mvn:build.teq::mill-teq::<version> dev.teq.mill.Teq/teqExportAll`, no edit of
# the build). The export leaves out what teq does not build, each module named with its reason (custom, whose
# local class path the build computes, and its dependent among them), holds a test module to the same (tested's,
# with Java sources; overridden's, whose resolvedMvnDeps is the build's, before anything of it runs), and runs
# none of the build's computations of paths (computed's sources and resources, custom's class path: none of their
# task directories written); its lock is tests/mill-export/expected-<mill>.lock
# byte for byte (`--record` writes it), and a second export leaves the file alone. Then teq over it ($TEQ,
# default target/release/teq): the lock is current (`--strict`, the exporter's inputs the reader's); a compile
# of every project refuses c, which a compile-only dependency of its dependency would show what Mill hides, gen,
# whose generator is the build's own, and computed, whose sources the build computes; harness's suites run in a
# sandbox of their own, emptied before each run, with Mill's variables and harness's run class path; core's
# test, whose class path adds munit that core's sources would see typed inside it, and tested's are refused, and so
# is downstream's compile, whose class path lacks upstream's compileMvnDeps; a test of every project runs harness's
# and names the others left out; tool's generator is the overlay's command; d, which runs on r's products alone, is
# refused its run; an edited package.mill, a package added (a package.mill and a package.mill.yaml) and a declared
# input removed make the lock stale; under Mill 1.1 a build allowing nested build files (its header YAML, with a
# comment) is refused. Then the driver's mode (`"buildTool": true`): the lock at the root with the launchers, an
# overlay naming a module the build lacks and, under the driver, an input outside the root refused. Last, the
# language server over a lock declaring an input outside the root (lsp-declared.py): its watcher registered from
# the input's directory, its edit warning of the stale lock. mill-teq's own suites run first. Needs java, curl and
# python3; the Mill launcher is fetched from Maven Central once (its sha1 checked) into TEQ_MILL_CACHE (default
# ~/.cache/teq/mill), and Mill and the fixture's libraries through coursier's cache: a first run on a machine
# downloads Mill's two distributions and the fixture's libraries, which can exceed a step's bound (340 s) and read
# as that step's failure; a second run starts warm.
cd "$(dirname "$0")/.." || exit 1
root=$PWD
TEQ=${TEQ:-$root/target/release/teq}
TEQ=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
record=false
[ "${1:-}" = "--record" ] && record=true
versions=${MILL_VERSIONS:-"1.0.6 1.1.10"}
launcher_version=1.0.6
status=0
pass() { echo "mill-export: $1"; }
fail() { echo "FAIL mill-export: $1"; status=1; }
if [ ! -x "$TEQ" ]; then echo "FAIL mill-export: no teq at $TEQ"; exit 1; fi
for tool in java curl; do
  command -v $tool > /dev/null || { echo "FAIL mill-export: no $tool on the PATH (incomplete validation)"; exit 1; }
done
work=$(cd "$(mktemp -d)" && pwd -P)
trap 'rm -rf "$work"' EXIT

# Mill's launcher script, from Central, its sha1 checked; it runs the Mill a build names (.mill-version).
cache=${TEQ_MILL_CACHE:-$HOME/.cache/teq/mill}
mill=$cache/mill-dist-$launcher_version-mill.sh
if [ ! -x "$mill" ]; then
  mkdir -p "$cache"
  url=https://repo1.maven.org/maven2/com/lihaoyi/mill-dist/$launcher_version/mill-dist-$launcher_version-mill.sh
  if ! timeout 120 curl -fsSL -o "$mill.part" "$url" || ! sha1=$(timeout 60 curl -fsSL "$url.sha1"); then
    echo "FAIL mill-export: the Mill launcher could not be fetched from $url"; exit 1
  fi
  if [ "$(LC_ALL=C shasum "$mill.part" | cut -d' ' -f1)" != "$(echo "$sha1" | cut -c1-40)" ]; then echo "FAIL mill-export: $url is not its sha1's"; exit 1; fi
  chmod +x "$mill.part" && mv "$mill.part" "$mill"
fi

# The plugin, published under the test's own version into a Maven directory of the test's.
plugin=0.0.0-mill-export-test-SNAPSHOT
maven=$work/maven
if ! (cd integrations/mill && TEQ_MILL_PLUGIN_VERSION=$plugin timeout 340 "$mill" --no-server --ticker false publishM2Local --m2RepoPath "$maven") > "$work/plugin.log" 2>&1; then
  echo "FAIL mill-export: the plugin did not build:"; tail -30 "$work/plugin.log"; exit 1
fi
if [ ! -f "$maven/build/teq/mill-teq_mill1_3/$plugin/mill-teq_mill1_3-$plugin.jar" ]; then echo "FAIL mill-export: the plugin was not published under $plugin"; tail -5 "$work/plugin.log"; exit 1; fi
pass "mill-teq builds and publishes under $plugin"
if (cd integrations/mill && timeout 340 "$mill" --no-server --ticker false test) > "$work/plugin-test.log" 2>&1; then
  pass "mill-teq's own suites pass (the overlay, the inputs)"
else
  fail "mill-teq's own suites: $(tail -20 "$work/plugin-test.log")"
fi
export COURSIER_REPOSITORIES="ivy2Local|central|file://$maven"
export_build() { (cd "$1" && timeout 340 "$mill" --no-server --ticker false --import "mvn:build.teq::mill-teq::$plugin" dev.teq.mill.Teq/teqExportAll); }
teq_in() { (cd "$1" && shift && timeout 300 "$TEQ" "$@" 2>&1); }

for v in $versions; do
  b=$work/build-$v
  cp -R tests/mill-export/fixture "$b"
  echo "$v" > "$b/.mill-version"
  if ! out=$(export_build "$b" 2>&1); then fail "Mill $v: the export failed: $(tail -20 <<< "$out")"; continue; fi
  lock=$b/target/teq/teq.lock
  left=$(grep -c 'is left out of teq.lock' <<< "$out")
  if [ "$left" = 9 ] && [[ "$out" == *"teq: native is left out of teq.lock: a Scala Native module, which teq does not build"* ]] &&
    [[ "$out" == *"teq: javaonly is left out of teq.lock: a Java module, whose sources teq does not compile"* ]] &&
    [[ "$out" == *"teq: mixed is left out of teq.lock: its sources hold a Java file, which teq does not compile"* ]] &&
    [[ "$out" == *"teq: usesjava is left out of teq.lock: it depends on javaonly, which is left out"* ]] &&
    [[ "$out" == *"teq: core.integration is left out of teq.lock: a test module that is no module's test configuration"* ]] &&
    [[ "$out" == *"teq: custom is left out of teq.lock: its local class path, custom.localCompileClasspath, is the build's own computation, which the export does not run"* ]] &&
    [[ "$out" == *"teq: usescustom is left out of teq.lock: it depends on custom, which is left out"* ]] &&
    [[ "$out" == *"teq: tested's test configuration is left out of teq.lock: its test module tested.test: its sources hold a Java file, which teq does not compile"* ]] &&
    [[ "$out" == *"teq: overridden's test configuration is left out of teq.lock: its test module overridden.test: its resolvedMvnDeps is the build's own task, which the export does not run"* ]] &&
    [ ! -e "$b/out/overridden/test/generatedSources.dest/ran" ] &&
    [ ! -e "$b/out/computed/sources.dest/ran" ] && [ ! -e "$b/out/computed/resources.dest/ran" ] && [ ! -e "$b/out/custom/unmanagedClasspath.dest/ran" ]; then
    pass "Mill $v: the export leaves out what teq does not build, each with its reason, a test module held to the same before anything of it is read, and runs none of the build's computations of paths"
  else
    fail "Mill $v: the modules left out ($left): $out"
  fi
  expected=tests/mill-export/expected-$v.lock
  if $record; then
    cp "$lock" "$expected"
    pass "Mill $v: recorded $expected"
  elif cmp -s "$lock" "$expected"; then
    pass "Mill $v: the lock is $expected byte for byte"
  else
    fail "Mill $v: the lock differs from $expected: $(diff "$expected" "$lock" | head -20)"
  fi
  cp "$lock" "$work/first.lock"
  if export_build "$b" > "$work/second.log" 2>&1 && cmp -s "$lock" "$work/first.lock" && [ ! "$lock" -nt "$work/first.lock" ]; then
    pass "Mill $v: a second export gives the same bytes and leaves the file alone"
  else
    fail "Mill $v: the second export: $(tail -5 "$work/second.log")"
  fi
  out=$(teq_in "$b" --strict compile b)
  if [ $? = 0 ] && [[ "$out" != *"written before"* ]]; then pass "Mill $v: the lock is current, the reader's inputs the exporter's"; else fail "Mill $v: --strict over the fresh export: $out"; fi
  out=$(teq_in "$b" compile)
  code=$?
  if [ $code = 2 ] && [[ "$out" == *"teq: compile left out c, which needs what mill alone runs:
  c/compile does not see b/compile, which a/compile builds on"* ]] && [[ "$out" == *"teq: compile left out gen, which needs what mill alone runs:
  gen/compile has the source generator gen.generatedSources, which mill runs and teq cannot"* ]] &&
    [[ "$out" == *"teq: compile left out computed, which needs what mill alone runs:
  computed/compile has the source generator computed.sources, which mill runs and teq cannot (the build computes its sources, which the export does not run)"* ]] &&
    [[ "$out" == *"core.extras/compile: "* ]] && [[ "$out" == *"pkg/compile: "* ]] && [[ "$out" == *"web/compile: checked"* ]] && [[ "$out" != *"error"* ]]; then
    pass "Mill $v: a compile of every project builds the others and names c (a dependency it does not see), gen (Mill's generator) and computed (sources the build computes) left out"
  else
    fail "Mill $v: the compile of every project ($code): $out"
  fi
  downstream=$(teq_in "$b" compile downstream)
  downstream_code=$?
  upstream=$(teq_in "$b" compile upstream)
  upstream_code=$?
  if [ $downstream_code = 2 ] && [[ "$downstream" == *"upstream/compile is typed inside downstream/compile, whose class path lacks com.lihaoyi:sourcecode_3:0.4.2, which upstream/compile's names"* ]] &&
    [ $upstream_code = 0 ] && [[ "$upstream" == *"upstream/compile: "* ]]; then
    pass "Mill $v: downstream's compile, which would type upstream's sources without upstream's compileMvnDeps, refused; upstream's compiles"
  else
    fail "Mill $v: the compile-time dependency: downstream ($downstream_code) $downstream // upstream ($upstream_code) $upstream"
  fi
  first=$(teq_in "$b" test harness)
  first_code=$?
  second=$(teq_in "$b" test harness)
  second_code=$?
  if [ $first_code = 0 ] && [ $second_code = 0 ] && [[ "$second" == *"harness/test: 1 suite, 2 passed, 0 failed"* ]] && [ -f "$b/target/teq/harness/test/sandbox/written-by-a-run.txt" ]; then
    pass "Mill $v: harness's suites run in a sandbox emptied before each run, with Mill's variables and harness's run class path"
  else
    fail "Mill $v: harness's suites ($first_code, $second_code): $first // $second"
  fi
  core=$(teq_in "$b" test core)
  core_code=$?
  tested=$(teq_in "$b" test tested)
  tested_code=$?
  if [ $core_code = 2 ] && [[ "$core" == *"core/compile is typed inside core/test, whose class path holds "*"which core/compile's does not name"* ]] &&
    [ $tested_code = 2 ] && [[ "$tested" == *"tested: its test module tested.test: its sources hold a Java file, which teq does not compile"* ]]; then
    pass "Mill $v: core's test, which would show core's sources munit, and tested's, whose test module holds Java sources, refused"
  else
    fail "Mill $v: the refused tests: core ($core_code) $core // tested ($tested_code) $tested"
  fi
  out=$(teq_in "$b" test)
  code=$?
  if [ $code = 2 ] && [[ "$out" == *"harness/test: 1 suite, 2 passed"* ]] && [[ "$out" == *"teq: test left out core, which needs"* ]] && [[ "$out" == *"teq: test left out matrix[3.8.4], which needs"* ]] &&
    [[ "$out" == *"teq: test left out tested, which needs"* ]] &&
    [[ "$out" == *"teq: test left out web is a Scala.js project, whose suites (linked as CommonJSModule, run under node with jsdom) teq does not run yet"* ]]; then
    pass "Mill $v: a test of every project runs the JVM suites it may and names the others left out, exiting 2"
  else
    fail "Mill $v: the test of every project ($code): $out"
  fi
  out=$(teq_in "$b" run tool main)
  if [ $? = 0 ] && [[ "$out" == *"labels: 3"* ]]; then pass "Mill $v: tool's generator is the overlay's command, its program run by its alias"; else fail "Mill $v: tool's run: $out"; fi
  out=$(teq_in "$b" run d)
  if [ $? = 2 ] && [[ "$out" == *"d/runtime runs on r/compile's products, which d/compile's closure does not build (a dependency at run time alone): run d with mill"* ]]; then
    pass "Mill $v: d, which runs on r's products alone, is refused its run"
  else
    fail "Mill $v: d's run: $out"
  fi
  teq_in "$b" stop > /dev/null
  printf '\n// edited\n' >> "$b/pkg/package.mill"
  mkdir -p "$b/more" && printf 'package build.more\n' > "$b/more/package.mill"
  mkdir -p "$b/yaml" && printf 'extends: mill.scalalib.ScalaModule\n' > "$b/yaml/package.mill.yaml"
  rm "$b/.fixture-version"
  out=$(teq_in "$b" --strict compile b)
  if [ $? = 2 ] && [[ "$out" == *"teq.lock was written before pkg/package.mill changed, more/package.mill added, yaml/package.mill.yaml added, .fixture-version removed; run mill --import mvn:build.teq::mill-teq::"*" dev.teq.mill.Teq/teqExportAll"* ]]; then
    pass "Mill $v: an edited package.mill, a package added (Mill 1.0's and 1.1's YAML one) and a declared input removed make the lock stale, --strict refusing"
  else
    fail "Mill $v: the stale lock: $out"
  fi
  # Mill 1.1's nested build files, which teq's readers do not find among the inputs: the export refuses them.
  if [[ "$v" == 1.0.* ]]; then continue; fi
  n=$work/nested-$v
  cp -R tests/mill-export/fixture "$n"
  echo "$v" > "$n/.mill-version"
  # The header as Mill reads it, YAML: a comment after the value.
  printf '//| mill-allow-nested-build-mill: true # nested packages\n' | cat - "$n/build.mill" > "$n/build.mill.new" && mv "$n/build.mill.new" "$n/build.mill"
  mkdir -p "$n/sub" && printf 'package build.sub\nimport mill.*, scalalib.*\nobject nested extends ScalaModule { def scalaVersion = "3.8.4" }\n' > "$n/sub/build.mill"
  out=$(export_build "$n" 2>&1)
  if [ $? != 0 ] && [[ "$out" == *"the build allows nested build files (mill-allow-nested-build-mill), whose scripts teq's readers do not find among its inputs"* ]] && [ ! -e "$n/target/teq/teq.lock" ]; then
    pass "Mill $v: a build that allows nested build files, its header YAML with a comment, is refused"
  else
    fail "Mill $v: the nested build files: $(tail -5 <<< "$out")"
  fi
done

# The driver's mode: the lock at the root, the launchers beside it, the build's files LF.
v=${versions%% *}
d=$work/driver
cp -R tests/mill-export/fixture "$d"
echo "$v" > "$d/.mill-version"
python3 - "$d/teq.mill.json" <<'PY'
import json, sys
overlay = json.load(open(sys.argv[1]))
overlay["buildTool"] = True
# The plugin the test publishes is a SNAPSHOT, which the driver's mode allows when the overlay says so.
overlay["snapshots"] = True
open(sys.argv[1], "w").write(json.dumps(overlay, indent=2) + "\n")
PY
if out=$(export_build "$d" 2>&1) && [ -f "$d/teq.lock" ] && [ ! -e "$d/target/teq/teq.lock" ] && [ -x "$d/teq" ] && [ -f "$d/teq.cmd" ] &&
  cmp -s "$d/teq" tools/launcher/teq && [[ "$out" == *"wrote $d/teq, launcher "* ]]; then
  out=$(cd "$d" && TEQ=$TEQ timeout 300 ./teq --strict compile b 2>&1)
  if [ $? = 0 ] && [[ "$out" == "b/compile: "* ]]; then pass "the driver's mode: the lock at the root with the launchers, ./teq running it"; else fail "the driver's mode, ./teq: $out"; fi
  (cd "$d" && TEQ=$TEQ timeout 60 ./teq stop > /dev/null 2>&1)
else
  fail "the driver's mode: $out"
fi
# An overlay naming a module the build does not have is refused, the lock left alone.
printf '{"modules": {"nowhere": {"mainClasses": ["x.Main"]}}}\n' > "$d/teq.mill.json"
cp "$d/teq.lock" "$work/driver.lock"
out=$(export_build "$d" 2>&1)
if [ $? != 0 ] && [[ "$out" == *"teq.mill.json declares the module nowhere, which the build does not have"* ]] && cmp -s "$d/teq.lock" "$work/driver.lock"; then
  pass "an overlay naming a module the build does not have is refused"
else
  fail "the overlay's unknown module: $out"
fi
# Under the driver a declared input outside the build's root is refused, which the lock without it would name.
printf '{"buildTool": true, "snapshots": true, "inputs": ["../outside.txt"]}\n' > "$d/teq.mill.json"
out=$(export_build "$d" 2>&1)
if [ $? != 0 ] && [[ "$out" == *"teq.mill.json declares the input ../outside.txt, outside the build's root, which a lock the repository commits cannot name"* ]] && cmp -s "$d/teq.lock" "$work/driver.lock"; then
  pass "under the driver a declared input outside the build's root is refused"
else
  fail "the declared input outside the root: $out"
fi
# The language server over a lock declaring an input outside the build's root, as an overlay may without the
# driver: it watches the input from its own directory and refreshes the stale warning when it changes.
mkdir -p "$work/lsp"
if out=$(timeout 120 python3 tests/mill-export/lsp-declared.py "$TEQ" "$work/lsp" 2>&1); then
  pass "the language server watches a declared input outside the build's root and warns of the stale lock when it changes"
else
  fail "the language server's declared input outside the root: $out"
fi
exit $status
