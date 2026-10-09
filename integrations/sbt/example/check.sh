#!/bin/bash
# Generates the corpus into src/, builds both sides with `sbt teqBuild` and runs them against scalac's
# output recorded in bench/app/expected: the frontend under node, the API side's class files under java
# with the jars of its description. Then `teqLinkJS` and `teqFullLinkJS` (the frontend project, and
# browserdemo, a small hand-written page under browserdemo-src/ for the checks below; an API side is a
# JVM build, which has no Scala.js-style dev loop), one sbt session, both served directories' main.js run
# under node against the same output: the dev build's stub over its modules, the full build's one file.
# Then the frontend through the vite plugin (integrations/vite): `vite build` run under node against the
# same output, and a dev server's first build serving the entry. Then the page under headless Chrome
# through sbt's own link tasks (check-scalajs-dev.mjs, which lists what it checks and how its time
# is bounded): the stock Scala.js vite plugin over `browserdemo` with a link per edit, the toggle
# turned off and on again in one directory, and the build shaped like the reference application's
# (`mirrored`) under `sbt ~mirrored/fastLinkJS`; and with no sbt, `teq dev browserdemo` over the
# export (check-task-dev.mjs). Then browserdemo's images generator, a Scala script the build's own
# binary runs (`teq interp`), under sbt's compile, `teq compile` and `teq dev`, a `teq` and a
# `scala-cli` first on the PATH that must not run (below). Needs sbt, sbt-teq (the release project/plugins.sbt pins, or a
# branch's published locally under a version of its own that TEQ_PLUGIN_VERSION names: `sbt --batch
# 'set version := "0.1.1-<branch>-SNAPSHOT"; publishLocal'` in integrations/sbt), npm and Chrome
# (CHROME overrides its path); TEQ names the binary (default: the repository's release build);
# VITE_PORT the vite-plugin-teq dev server's port (default 5391), VITE_SCALAJS_PORT the first of the
# five ports the pages take (default 5393), TASK_DEV_PORT the dev loop's page's (default 5399), GENERATOR_DEV_PORT the generator's dev loop's (default
# 5400).
cd "$(dirname "$0")" || exit 1
repo=../../..
export TEQ=${TEQ:-$(cd $repo && pwd)/target/release/teq}
rm -rf src && timeout 120 python3 $repo/bench/app/gen.py src > /dev/null || { echo "FAIL generation"; exit 1; }
status=0
# COMPILER_ONLY=1 runs the compiler section alone, RELEASE_ONLY=1 the release one, EXPORT_ONLY=1
# the export's (check-export.sh), GENERATOR_ONLY=1 the generator's.
if [ -z "$COMPILER_ONLY$RELEASE_ONLY$EXPORT_ONLY$GENERATOR_ONLY" ]; then
timeout 340 sbt --server --batch 'teqBuild; frontend/teqLinkJS; frontend/teqFullLinkJS; browserdemo/teqLinkJS; browserdemo/teqFullLinkJS' > target-sbt.log 2>&1 || { echo "FAIL sbt teqBuild/teqLinkJS/teqFullLinkJS (see target-sbt.log)"; exit 1; }
# api's runtime jars, as the export names them, in coursier's cache, whose layout follows the URL.
jars=$(python3 - <<'PY'
import os, sys
import importlib.util
spec = importlib.util.spec_from_file_location("c", "check-export.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
export = c.load("teq.lock")
cache = os.environ.get("COURSIER_CACHE") or os.path.expanduser("~/Library/Caches/Coursier/v1" if sys.platform == "darwin" else "~/.cache/coursier/v1")
urls = {r["id"]: r["url"].rstrip("/") for r in export["repositories"]}
# A jar's key names its record in the table, whose path is the Maven layout of the key unless it names another.
def place(entry):
    if isinstance(entry, dict):
        return entry["file"]
    repository, _, _, path = c.fields(export, entry)
    scheme, rest = (urls[repository] + "/" + path).split("://", 1)
    # coursier's CachePath.escape: a port's `:`, a version's `+`, ... as `%` and two digits of base 16.
    return os.path.join(cache, scheme, c.coursier_escape(rest))
print(":".join(place(e) for e in export["projects"]["api"]["configurations"]["runtime"]["classpath"] if isinstance(e, str) or "file" in e))
PY
)
for side in frontend api; do
  case $side in
    frontend) run=(node frontend/target/teq/out/main.mjs) ;;
    api) run=(java -cp "api/target/teq/out:$jars" TeqMain) ;;
  esac
  if timeout 120 "${run[@]}" 2>&1 | diff -q $repo/bench/app/expected/$side.txt - > /dev/null; then
    echo "$side: matches scalac's output"
  else
    echo "FAIL $side differs from scalac's output"
    status=1
  fi
done

for task in teqLinkJS teqFullLinkJS; do
  dir=served
  what="the stub main.js over main.mjs"
  [ "$task" = teqFullLinkJS ] && dir=served-full && what="the one file main.js"
  if timeout 20 node frontend/target/teq/$dir/main.js 2>&1 | diff -q $repo/bench/app/expected/frontend.txt - > /dev/null; then
    echo "$task: served frontend ($what) matches scalac's output"
  else
    echo "FAIL $task: served frontend differs from scalac's output"
    status=1
  fi
done

# `ThisBuild / teqThreads` reaches every project's description in the export (and with it the
# residents' and the one-shot builds' `--threads`); `reload` drops the setting again. The committed
# export is put back after.
threads_of() { python3 -c 'import importlib.util; spec = importlib.util.spec_from_file_location("c", "check-export.py"); c = importlib.util.module_from_spec(spec); spec.loader.exec_module(c); print(c.load("teq.lock")["projects"]["frontend"]["description"].get("threads", "none"))'; }
cp teq.lock target-threads-fixture.lock
timeout 340 sbt --server --batch 'set ThisBuild / teqThreads := Some(3); teqExportAll' > target-threads.log 2>&1
if [ "$(threads_of)" = 3 ]; then
  echo "ThisBuild / teqThreads: the frontend's description has threads 3"
else
  echo "FAIL ThisBuild / teqThreads did not reach the frontend's description (see target-threads.log)"
  status=1
fi
timeout 340 sbt --server --batch 'reload; teqExportAll' >> target-threads.log 2>&1
[ "$(threads_of)" = none ] || { echo "FAIL the description kept threads after reload"; status=1; }
cp target-threads-fixture.lock teq.lock

if [ ! -d node_modules ] && ! timeout 180 npm install --prefer-offline --no-audit --no-fund > target-npm.log 2>&1; then
  echo "FAIL npm install (see target-npm.log)"
  exit 1
fi
rm -rf dist
if ! timeout 180 ./node_modules/.bin/vite build > target-vite-build.log 2>&1; then
  echo "FAIL vite build (see target-vite-build.log)"
  status=1
elif timeout 120 node dist/frontend.js 2>&1 | diff -q $repo/bench/app/expected/frontend.txt - > /dev/null; then
  echo "vite build: frontend matches scalac's output"
else
  echo "FAIL vite build: frontend differs from scalac's output"
  status=1
fi

port=${VITE_PORT:-5391}
timeout 120 ./node_modules/.bin/vite --port "$port" --strictPort --host 127.0.0.1 > target-vite-dev.log 2>&1 &
dev=$!
entry=
for _ in $(seq 1 120); do
  entry=$(curl -s --max-time 2 "http://127.0.0.1:$port/main.js") && break
  sleep 0.5
done
module=$(echo "$entry" | grep -o '"/[^"]*/main\.mjs"' | head -1 | tr -d '"')
if [ -z "$module" ]; then
  echo "FAIL vite dev: /main.js does not import the output's main.mjs (see target-vite-dev.log)"
  status=1
elif curl -s --max-time 30 "http://127.0.0.1:$port$module" | grep -q '^\$main('; then
  echo "vite dev: serves /main.js and $module"
else
  echo "FAIL vite dev: $module is not the program's entry (see target-vite-dev.log)"
  status=1
fi
runtime=$(curl -s --max-time 5 "http://127.0.0.1:$port/" | grep -o 'import "[^"]*hot-refresh\.mjs"' | head -1 | cut -d'"' -f2)
if [ -n "$runtime" ] && curl -s --max-time 30 "http://127.0.0.1:$port$runtime" | grep -q teqHotObject; then
  echo "vite dev: injects the refresh runtime as $runtime and serves it"
else
  echo "FAIL vite dev: the refresh runtime is not injected into the page or not served (see target-vite-dev.log)"
  status=1
fi
kill "$dev" 2> /dev/null
wait "$dev" 2> /dev/null

# The page under headless Chrome over teq's output (TEQ_COMPILER=1: sbt-teq answers a Scala.js
# project's fastLinkJS and fullLinkJS): through the stock Scala.js vite plugin, with the toggle
# turned off and on again, and through the build shaped like the reference application's. A mode
# ends itself after 300 s, saying so (exit 3); the bound here is for one that cannot.
for mode in stock toggle mirrored; do
  VITE_SCALAJS_PORT=${VITE_SCALAJS_PORT:-5393} timeout 340 node check-scalajs-dev.mjs $mode
  case $? in
    0) ;;
    3 | 124) echo "TIMEOUT check-scalajs-dev.mjs $mode: out of its time on this machine, which is no verdict on the behaviour; run the mode again with the machine idle"; status=1 ;;
    *) echo "FAIL check-scalajs-dev.mjs $mode"; status=1 ;;
  esac
done

# The dev loop with no sbt alive: `teq dev browserdemo` over the export, its page under headless
# Chrome; it ends itself after 300 s, saying so (exit 3).
TASK_DEV_PORT=${TASK_DEV_PORT:-5399} timeout 340 node check-task-dev.mjs
case $? in
  0) ;;
  3 | 124) echo "TIMEOUT check-task-dev.mjs: out of its time on this machine, which is no verdict on the behaviour"; status=1 ;;
  *) echo "FAIL check-task-dev.mjs"; status=1 ;;
esac
fi

# --- a release's binary ----------------------------------------------------------------------------
# Without TEQ the plugin fetches teq's binary at teqVersion from its GitHub release (../README.md, "The teq
# binary"), checked against the release's manifest and SHA256SUMS before anything runs it: here from a mirror of
# releases of the check's own (bench/release-mirror.py, its port its own, an asset through a redirect as GitHub
# serves one; the releases under target-releases/, release-fixture.py's), which teqReleases names, whose
# "binaries" are shell scripts printing their own --version. The plugin's cache is TEQ_CACHE_DIR's here,
# target-teq-cache/: its release cache (releases/, each verified binary beside its receipt) and the shared copies
# (bin/<sha1>/), the real ones untouched. Six fresh sbt starts and their cases: the first verifies the one and
# shares it across both projects' target/teq/bin and the shared copies; the second, the build naming the next
# release, selects that one; then the next's asset is replaced behind its name, its manifest and SHA256SUMS
# unchanged, and a start with the release cache keeps the copy it verified, while one with the next's release cache
# cleared refuses the replacement by its digest, the projects' copies no authority; a release the mirror lacks fails,
# naming the release and the overrides; with the mirror stopped, a start finds the first's verified binary by its
# receipt in the release cache, the projects' copies of it removed so that nothing else could serve, and one naming
# the next, whose release cache is cleared, fails though the projects' copies of it remain.
if [ -z "$COMPILER_ONLY$EXPORT_ONLY$GENERATOR_ONLY" ]; then
first=0.1.7-check.1 next=0.1.7-check.2 missing=0.1.7-check.3
case "$(uname -s)" in Darwin) os=osx ;; *) os=linux ;; esac
case "$(uname -m)" in arm64 | aarch64) arch=aarch_64 ;; *) arch=x86_64 ;; esac
sha1_of() { python3 -c 'import hashlib, sys; print(hashlib.sha1(open(sys.argv[1], "rb").read()).hexdigest())' "$1"; }
asset() { echo "target-releases/v$1/teq-$1-$os-$arch"; }
# release_cache <version>: the release cache's directory of the version, under the mirror's base.
release_cache() { echo target-teq-cache/releases/*/"$1"; }
rm -rf target-releases target-replaced target-mirror.port target-teq-cache api/target/teq/bin frontend/target/teq/bin
timeout 30 python3 release-fixture.py target-releases $first one $os-$arch > /dev/null &&
  timeout 30 python3 release-fixture.py target-releases $next one $os-$arch > /dev/null || { echo "FAIL release: the releases could not be written"; exit 1; }
mkdir -p target-maven
timeout 600 python3 -B $repo/bench/release-mirror.py target-mirror.port target-releases target-maven > target-mirror.log 2>&1 &
mirror=$!
for _ in $(seq 100); do [ -s target-mirror.port ] && break; sleep 0.1; done
base=http://127.0.0.1:$(cat target-mirror.port 2> /dev/null)/releases/download
# release_start <version>: a fresh sbt start resolving the release from the mirror in both projects.
release_start() {
  env -u TEQ TEQ_CACHE_DIR="$PWD/target-teq-cache" timeout 340 sbt --server --batch "set ThisBuild / teqReleases := \"$base\"" \
    "set ThisBuild / teqVersion := \"$1\"" "show api/teqResolvedBinary frontend/teqResolvedBinary" >> target-release.log 2>&1
}
# copies_print <version> <word>: both projects' copies print what that binary prints.
copies_print() {
  local project
  for project in api frontend; do
    [ "$($project/target/teq/bin/teq-$1-$os-$arch --version 2> /dev/null)" = "teq $1 $2" ] || return 1
  done
}
# verified <version>: the release cache holds the version's asset, its bytes the release's, beside its receipt.
verified() { local d; d=$(release_cache "$1"); cmp -s "$d/$os-$arch/teq-$1-$os-$arch" "$(asset "$1")" && [ -f "$d/$os-$arch/teq-$1-$os-$arch.receipt" ]; }
shared() { cmp -s "target-teq-cache/bin/$(sha1_of "$(asset "$1")")/teq-$1-$os-$arch" "$(asset "$1")"; }
: > target-release.log
if [ -s target-mirror.port ] && release_start $first && copies_print $first one && verified $first && shared $first &&
  [ "$(cat api/target/teq/bin/teq-$first-$os-$arch.sha1)" = "$(sha1_of "$(asset $first)")" ]; then
  echo "release: the first start verifies the release by its manifest and SHA256SUMS, kept with its receipt, and shares it: both projects' target/teq/bin, stamped with its checksum, and teq's cache"
else
  echo "FAIL release: the first start (see target-release.log, target-mirror.log)"
  status=1
fi
if release_start $next && copies_print $next one && verified $next && shared $next; then echo "release: the build naming the next release, the next start selects it"; else echo "FAIL release: the start naming the next release (see target-release.log)"; status=1; fi
# The next's asset replaced by bytes of the same size, its manifest and SHA256SUMS as they were.
timeout 30 python3 release-fixture.py target-replaced $next two $os-$arch > /dev/null && cp "target-replaced/v$next/teq-$next-$os-$arch" "$(asset $next)"
warm=
release_start $next && copies_print $next one && warm=1
rm -rf "$(release_cache $next)"
if [ -n "$warm" ] && ! release_start $next && grep -qF "and the release v$next's manifest gives" target-release.log && grep -qF "nothing of it is run" target-release.log &&
  copies_print $next one && [ ! -e "$(release_cache $next)/$os-$arch/teq-$next-$os-$arch" ]; then
  echo "release: the next's asset replaced, its manifest and SHA256SUMS unchanged: the release cache keeps the copy it verified; that cache cleared, the replacement is refused by its digest"
else
  echo "FAIL release: the replaced asset (see target-release.log)"
  status=1
fi
if ! release_start $missing && grep -qF "teq: the release v$missing of teq cannot be read: the release v$missing is not at $base/v$missing/" target-release.log &&
  grep -qF "Set teqBinary (or TEQ in the environment) to a local binary, or teqVersion to a released compiler" target-release.log; then
  echo "release: a release the mirror lacks fails, naming the release and the overrides"
else
  echo "FAIL release: the start naming a release the mirror lacks (see target-release.log)"
  status=1
fi
kill $mirror 2> /dev/null
wait $mirror 2> /dev/null
rm -f api/target/teq/bin/teq-$first-* frontend/target/teq/bin/teq-$first-*
if release_start $first && copies_print $first one; then
  echo "release: with the mirror stopped, the release cache's verified binary serves by its receipt, the projects' copies of it removed"
else
  echo "FAIL release: the start without the mirror, the release cache kept (see target-release.log)"
  status=1
fi
if ! release_start $next && grep -qF "teq: the release v$next of teq cannot be read: " target-release.log && copies_print $next one; then
  echo "release: with the mirror stopped and the next's release cache cleared, its start fails, the projects' copies of it notwithstanding"
else
  echo "FAIL release: the start without the mirror and its release cache (see target-release.log)"
  status=1
fi
rm -rf target-releases target-replaced target-maven target-mirror.port target-teq-cache
fi

if [ -z "$RELEASE_ONLY$EXPORT_ONLY$GENERATOR_ONLY" ]; then
# --- teq as sbt's compiler ------------------------------------------------------------------------
# The api project through sbt's own tasks with TEQ_COMPILER=1, in one sbt server session
# (`sbt --client`): zinc's incremental compile with teq as its Scala compiler (docs/TARGETS.md,
# "The module model"), one `teq` process per batch zinc asks for, over the sources it invalidated
# against the configuration's own class directory. Compile, run against scalac's output, the three
# frameworks' suites, testOnly, a failing test reported by its framework, a body edit compiling
# its source alone against its siblings' products, a type error reported with its position and
# kept across an unchanged retry, packageBin, a deletion-only change, `clean`, an edit of a
# Compile source reaching a test, forked tests, testQuick over zinc's dependencies, an API change
# compiling its dependents alone in both configurations, an inline body's change reaching its
# caller, a removal nothing depends on (no batch, the manifest's own operation), a failed second
# batch leaving the products and the manifest as they were, the toggle turned off and on again
# (scalac compiles the same commands, then teq compiles afresh), and `teqCacheableState` under
# compile (a change after a passed compile compiling everything again), teqBuild and the
# frontend's teqFullLinkJS.
sbtlog=target-sbt-compiler.log
: > $sbtlog
client() {
  local out code
  timeout 300 sbt --client "$@" > target-client.out 2>&1
  code=$?
  # sbt 2's client ends its output with a cursor movement of its own: the codes go.
  out=$(sed 's/\x1b\[[0-9;]*[A-Za-z]//g' target-client.out)
  printf '== sbt --client %s (exit %s)\n%s\n' "$*" "$code" "$out" >> $sbtlog
  CLIENT_OUT=$out
  return $code
}
expect_ok() {
  if client "$1"; then echo "compiler: $2"; else echo "FAIL compiler: $2 (see $sbtlog)"; status=1; fi
}
expect_fail() {
  if client "$1"; then echo "FAIL compiler: $2 did not fail (see $sbtlog)"; status=1; else echo "compiler: $2"; fi
}
has_output() { case "$CLIENT_OUT" in *"$1"*) return 0 ;; *) return 1 ;; esac; }
timed_ms() { local t0; t0=$(date +%s%N); "$@"; local code=$?; TIMED_MS=$(( ($(date +%s%N) - t0) / 1000000 )); return $code; }
export TEQ_COMPILER=1
timeout 60 sbt --client shutdown > /dev/null 2>&1
classes=target/out/jvm/scala-3.8.4/api/classes
# The main sources gain an inline def making an anonymous class (InlineActionSuite), and the
# tests a jar dependency (JarSuite) packaged by scala-cli from lib-src/.
mkdir -p src/api/meridian/check api/lib
cat > src/api/meridian/check/Actions.scala <<'EOF'
package meridian.check

trait Action:
  def run(): Int

object Actions:
  inline def make(): Action = new Action:
    def run(): Int = 42
EOF
package_util() {
  sed -i.bak "s/inline def value: Int = .*/inline def value: Int = $1/" lib-src/Util.scala
  timeout 200 scala-cli --power package lib-src --library -o api/lib/util.jar -f -S 3.8.4 --server=false > target-util-jar.log 2>&1 || { echo "FAIL compiler: packaging util.jar (see target-util-jar.log)"; status=1; }
}
package_util 1
# The batches a compile ran, as the plugin logs them: `teq: <n> sources of <directory>`.
batches() { printf '%s\n' "$CLIENT_OUT" | grep -o 'teq: [0-9]* sources* of [a-z-]*' | tr '\n' ';'; }
# Every file of a class directory with its checksum (a link's target's), and whether the compiler's
# staging directory or the plugin's run directory is beside it: what a rollback must leave as it found.
contents() { (cd "$1" 2> /dev/null && find . \( -type f -o -type l \) -exec cksum {} + | sort -k3); ls -d "$1".teq-stage "$1".teq-run 2> /dev/null; }
# The edits carry a number of this run: sbt's cache holds the results of earlier runs, failures
# among them, by the digests, and the same edit would find its own.
nonce=$(date +%s)
expect_ok api/compile "cold compile, teq as zinc's compiler"
if grep -qx teq $classes.backend 2> /dev/null && [ -f $classes/meridian/server/Main.class ] && [ -f $classes/meridian/server/Main.tasty ] && [ -f $classes/teq-products.json ]; then echo "compiler: class directory written and marked by teq, the class files with their TASTy and the manifest"; else echo "FAIL compiler: class directory not written by teq"; status=1; fi
printf '%s\n' "$CLIENT_OUT" | grep -o 'teq: [0-9]* sources of classes in .*' | head -1 | sed 's/^/compiler: cold: /'
if client api/run && printf '%s\n' "$CLIENT_OUT" | grep -v '^\[' | grep -v '^$' | diff -q $repo/bench/app/expected/api.txt - > /dev/null; then
  echo "compiler: run matches scalac's output"
else
  echo "FAIL compiler: run differs from scalac's output (see $sbtlog)"
  status=1
fi
if client "show api/Test/definedTests" && has_output ZioSuite && has_output MunitSuite && has_output ScalatestSuite && has_output IndirectSuite && has_output JunitStyleSuite && ! has_output ApiFunSuite && ! has_output HiddenSuite; then
  echo "compiler: definedTests finds the suites through their frameworks, a base class and a @Test method, not the abstract base or the private class"
else
  echo "FAIL compiler: definedTests (see $sbtlog)"
  status=1
fi
# A suite whose `@Test` methods are all inherited from a base another batch compiled is discovered
# after a batch of the suite alone: the base's products carry its methods' annotations, which the
# suite's API takes its inherited members' from (zinc's discovery reads `savedAnnotations`).
printf 'package meridian.apitest\n\nimport org.junit.Test\n\nabstract class JunitBase:\n  @Test def inherited(): Unit = assert(true)\n' > api-test-src/meridian/apitest/JunitBase.scala
printf 'package meridian.apitest\n\nclass JunitInheritedSuite extends JunitBase\n' > api-test-src/meridian/apitest/JunitInheritedSuite.scala
expect_ok api/Test/compile "the base and the suite of inherited @Test methods compile"
printf 'package meridian.apitest\n\nclass JunitInheritedSuite extends JunitBase:\n  def edit%s: Int = 1\n' $nonce > api-test-src/meridian/apitest/JunitInheritedSuite.scala
if client "show api/Test/definedTests" && batches | grep -q 'teq: 1 source of test-classes' && has_output JunitInheritedSuite; then
  echo "compiler: a suite whose @Test methods are inherited from another batch's base is discovered after a batch of the suite alone"
else
  echo "FAIL compiler: suite of inherited @Test methods after a batch of its own: $(batches) (see $sbtlog)"
  status=1
fi
rm api-test-src/meridian/apitest/JunitBase.scala api-test-src/meridian/apitest/JunitInheritedSuite.scala
expect_ok api/Test/compile "the inherited @Test suite and its base removed"
# sbt 2's `test` is `testQuick`: a suite whose digest has a cached success is skipped, so the
# runs here go through `testOnly`, which runs what it names.
# The JUnit-style suite is discovered (above) but filtered out of the runs (build.sbt).
if client api/testOnly && has_output "zio-test" && has_output "MunitSuite" && has_output "ScalatestSuite" && has_output "IndirectSuite" && has_output "InlineActionSuite"; then echo "compiler: testOnly runs the three frameworks' suites and the inline-made class"; else echo "FAIL compiler: testOnly (see $sbtlog)"; status=1; fi
expect_ok "api/testOnly meridian.apitest.MunitSuite" "testOnly of one suite"
cat > api-test-src/meridian/apitest/Failing.scala <<'EOF'
package meridian.apitest

import org.scalatest.funsuite.AnyFunSuite

class Failing extends AnyFunSuite:
  test("fails on purpose") {
    assert(1 + 1 == 3, "arithmetic")
  }
EOF
if ! client "api/testOnly meridian.apitest.Failing" && has_output "arithmetic"; then echo "compiler: a failing test is reported by its framework"; else echo "FAIL compiler: failing test (see $sbtlog)"; status=1; fi
rm api-test-src/meridian/apitest/Failing.scala
expect_ok api/packageBin "packageBin"
jar=$(ls target/out/jvm/scala-3.8.4/api/*.jar 2> /dev/null | grep -v -- '-tests.jar' | head -1)
if [ -n "$jar" ] && unzip -l "$jar" | grep -q "meridian/server/Main.class"; then echo "compiler: the jar holds the classes"; else echo "FAIL compiler: no jar with the classes"; status=1; fi
# A body edit of a small file: the second compile is a retype.
edited=src/api/business/ServiceError.scala
cp $edited target-edited.bak
sed -i.bak 's/case Invalid(message) => ApiFailure.Invalid(message)/case Invalid(message) => ApiFailure.Invalid(message + "")/' $edited
if timed_ms client api/compile && [ "$(batches)" = "teq: 1 source of classes;" ]; then
  echo "compiler: a body edit compiles its source alone against its siblings' products ($TIMED_MS ms wall clock through sbt; $(printf '%s\n' "$CLIENT_OUT" | grep -o 'teq: 1 source of classes in .*' | head -1))"
else
  echo "FAIL compiler: body edit (see $sbtlog)"
  status=1
fi
if client api/run && printf '%s\n' "$CLIENT_OUT" | grep -v '^\[' | grep -v '^$' | diff -q $repo/bench/app/expected/api.txt - > /dev/null; then
  echo "compiler: run after the one-source compile matches scalac's output"
else
  echo "FAIL compiler: run after the one-source compile (see $sbtlog)"
  status=1
fi
cp target-edited.bak $edited
# A type error: reported with its position, and again on an unchanged retry.
sed -i.bak 's/case Invalid(message) => ApiFailure.Invalid(message)/case Invalid(message) => ApiFailure.Invalid(42)/' $edited
if ! client api/compile && has_output "ServiceError.scala:23:" && has_output "error"; then echo "compiler: a type error is reported with its position"; else echo "FAIL compiler: type error (see $sbtlog)"; status=1; fi
expect_fail api/compile "the type error is reported again on an unchanged retry"
cp target-edited.bak $edited
expect_ok api/compile "the fix compiles"
# A deletion-only change: a file another one refers to is removed, then put back.
mv $edited target-deleted.scala
expect_fail api/compile "a deleted file's uses are errors"
mv target-deleted.scala $edited
expect_ok api/compile "the file put back compiles"
# clean: the products are written again from nothing.
expect_ok "api/clean; api/compile" "clean then compile"
if [ -f $classes/meridian/server/Main.class ]; then echo "compiler: the class files are back after clean"; else echo "FAIL compiler: no class files after clean"; status=1; fi
# An edit of a Compile source reaches the Test configuration's compile.
shared=src/shared/core/http/Endpoint.scala
cp $shared target-shared.bak
sed -i.bak 's|s"$method /${segments.mkString("/")}$q$h|s"$method //${segments.mkString("/")}$q$h|' $shared
if ! client api/testOnly && has_output "//v1/sites" ; then echo "compiler: an upstream edit reaches the tests (they fail on the changed output)"; else echo "FAIL compiler: upstream edit (see $sbtlog)"; status=1; fi
cp target-shared.bak $shared
expect_ok api/testOnly "the tests pass again"
expect_ok "set api / Test / fork := true; api/testOnly" "forked tests"
# sbt 2's `test` is `testQuick` over cached digests, which walk zinc's dependencies and the
# products' hashes: after testOnly it runs none, a changed suite and a production change re-run
# the tests, and their revert gives back digests that passed.
quick() { if client api/test && has_output "No tests to run for api / Test / testQuick"; then echo "compiler: $1"; else echo "FAIL compiler: $1 (see $sbtlog)"; status=1; fi; }
quick "test after testOnly runs none, the suites' successes recorded"
sed -i.bak 's/List("v1", "sites"), Nil, Nil, None)/List("v1", "nowhere"), Nil, Nil, None)/' api-test-src/meridian/apitest/ScalatestSuite.scala
if ! client api/test && has_output "ScalatestSuite"; then echo "compiler: test re-runs a changed suite"; else echo "FAIL compiler: test after a suite edit (see $sbtlog)"; status=1; fi
cp api-test-src/meridian/apitest/ScalatestSuite.scala.bak api-test-src/meridian/apitest/ScalatestSuite.scala && rm api-test-src/meridian/apitest/ScalatestSuite.scala.bak
quick "test after the suite is restored runs none, its digest the one that passed"
sed -i.bak 's|s"$method /${segments.mkString("/")}$q$h|s"$method //${segments.mkString("/")}$q$h|' $shared
if ! client api/test && has_output "//v1/sites"; then echo "compiler: test re-runs the suites after a production change"; else echo "FAIL compiler: test after a production edit (see $sbtlog)"; status=1; fi
cp $shared.bak $shared
quick "test after the production edit is reverted runs none, the digests the ones that passed"
# A jar replaced in place: zinc compiles again the sources that depend on it.
expect_ok "api/testOnly meridian.apitest.JarSuite" "the jar's constant"
package_util 2
if ! client "api/testOnly meridian.apitest.JarSuite" && has_output "JarSuite"; then echo "compiler: a jar rebuilt in place reaches the suite"; else echo "FAIL compiler: rebuilt jar (see $sbtlog)"; status=1; fi
package_util 1
expect_ok "api/testOnly meridian.apitest.JarSuite" "the jar's constant again"
# Java sources are refused, naming the files.
mkdir -p src/api/meridian/check
echo 'package meridian.check; public class J {}' > src/api/meridian/check/J.java
if ! client api/compile && has_output "J.java" && has_output "Java sources are not compiled"; then echo "compiler: Java sources are refused by name"; else echo "FAIL compiler: Java sources (see $sbtlog)"; status=1; fi
rm src/api/meridian/check/J.java
expect_ok api/compile "compiles again without the Java source"
# A class deleted or renamed leaves the class directory and the jar.
cat > src/api/meridian/check/Gone.scala <<'EOF'
package meridian.check

class Gone:
  def here: Boolean = true
EOF
expect_ok "api/compile; api/packageBin" "a class added is packaged"
if unzip -l "$jar" | grep -q "meridian/check/Gone.class"; then echo "compiler: the jar holds the added class"; else echo "FAIL compiler: the added class is not in the jar"; status=1; fi
rm src/api/meridian/check/Gone.scala
expect_ok "api/compile; api/packageBin" "a class deleted compiles"
if ! unzip -l "$jar" | grep -q "meridian/check/Gone.class" && [ ! -f target/out/jvm/scala-3.8.4/api/classes/meridian/check/Gone.class ]; then echo "compiler: the deleted class is gone from the class directory and the jar"; else echo "FAIL compiler: the deleted class stays (see $sbtlog)"; status=1; fi
# A deletion whose build fails is not lost: once the build succeeds, the suite is gone.
cat > api-test-src/meridian/apitest/RemovedSuite.scala <<'EOF'
package meridian.apitest

class RemovedSuite extends munit.FunSuite:
  test("removed later") { assert(true) }
EOF
cat > api-test-src/meridian/apitest/UsesRemoved.scala <<'EOF'
package meridian.apitest

object UsesRemoved:
  val suite = new RemovedSuite
EOF
if client "show api/Test/definedTests" && has_output RemovedSuite; then echo "compiler: a suite added is discovered"; else echo "FAIL compiler: added suite not discovered (see $sbtlog)"; status=1; fi
rm api-test-src/meridian/apitest/RemovedSuite.scala
expect_fail api/Test/compile "deleting the suite fails the build through its use"
rm api-test-src/meridian/apitest/UsesRemoved.scala
if client "show api/Test/definedTests" && ! has_output RemovedSuite && [ ! -f target/out/jvm/scala-3.8.4/api/test-classes/meridian/apitest/RemovedSuite.class ]; then echo "compiler: the suite deleted under a failing build is gone once the build passes"; else echo "FAIL compiler: deleted suite kept (see $sbtlog)"; status=1; fi
# The configuration's only root removed: its products go with the analysis, and come back
# with the root.
mv api-test-src target-tests.bak
expect_ok "api/Test/compile; api/Test/packageBin" "the test root removed compiles to nothing"
tjar=$(ls target/out/jvm/scala-3.8.4/api/*-tests.jar 2> /dev/null | head -1)
if [ ! -f target/out/jvm/scala-3.8.4/api/test-classes/meridian/apitest/MunitSuite.class ] && { [ -z "$tjar" ] || ! unzip -l "$tjar" | grep -q "MunitSuite.class"; }; then echo "compiler: the removed root's classes are gone from the directory and the jar"; else echo "FAIL compiler: the removed root's classes stay (see $sbtlog)"; status=1; fi
mv target-tests.bak api-test-src
expect_ok api/Test/compile "the test root put back compiles"
if [ -f target/out/jvm/scala-3.8.4/api/test-classes/meridian/apitest/MunitSuite.class ]; then echo "compiler: the root's classes are back"; else echo "FAIL compiler: the root's classes did not come back"; status=1; fi
# The toggle off: scalac compiles into a class directory teq wrote, afresh; then on again.
expect_ok "set every teqCompiler := false; api/compile" "toggle off: scalac compiles"
if [ ! -f $classes.backend ] && [ ! -f $classes/teq-products.json ]; then echo "compiler: teq's marker and manifest are gone after scalac wrote the directory"; else echo "FAIL compiler: the marker or the manifest survived scalac"; status=1; fi
expect_ok "api/run; api/testOnly" "toggle off: run and testOnly through scalac"
expect_ok "set every teqCompiler := true; api/compile" "toggle on again: teq compiles afresh"
if grep -qx teq $classes.backend 2> /dev/null && [ -f $classes/teq-products.json ]; then echo "compiler: the marker is teq's again, the manifest written"; else echo "FAIL compiler: the marker is not teq's"; status=1; fi
# zinc's invalidation over teq's analysis: an API change compiles its dependents alone, in this
# configuration and the next; an inline body's change compiles its caller; a removal nothing
# depends on runs no batch and leaves the manifest by its own operation; a failed second batch
# leaves the products and the manifest as they were (the plugin's journal and zinc's class-file
# manager), on the JVM and on Scala.js, whose stamps the journal keeps too.
check=src/api/meridian/check
manifest=$classes/teq-products.json
cat > $check/Api.scala <<'EOF'
package meridian.check

object Api:
  def f: Int = 1
  inline def k: Int = 1
EOF
cat > $check/UsesApi.scala <<'EOF'
package meridian.check

object UsesApi:
  def v: Long = Api.f + 1
EOF
cat > $check/UsesInline.scala <<'EOF'
package meridian.check

object UsesInline:
  def v: Int = Api.k
EOF
cat > $check/Unrelated.scala <<'EOF'
package meridian.check

object Unrelated:
  def v: Int = 3
EOF
cat > api-test-src/meridian/apitest/UsesApiSuite.scala <<'EOF'
package meridian.apitest

class UsesApiSuite extends munit.FunSuite:
  test("the api's number") { assertEquals(meridian.check.Api.f.toString, "1") }
EOF
expect_ok api/Test/compile "incremental: the check's sources compile"
sed -i.bak 's/def f: Int = 1/def f: Long = 1L/' $check/Api.scala
if client api/Test/compile && [ "$(batches)" = "teq: 1 source of classes;teq: 1 source of classes;teq: 1 source of test-classes;" ]; then
  echo "incremental: an API change compiles its class, its one dependent and the one suite that uses it"
else
  echo "FAIL incremental: API change ($(batches); see $sbtlog)"
  status=1
fi
cp $classes/meridian/check/UsesInline\$.class target-inline-before.class
sed -i.bak 's/inline def k: Int = 1/inline def k: Int = 2/' $check/Api.scala
if client api/Test/compile && [ "$(batches)" = "teq: 1 source of classes;teq: 1 source of classes;" ] && ! cmp -s target-inline-before.class $classes/meridian/check/UsesInline\$.class; then
  echo "incremental: an inline body's change compiles its caller, whose expansion changes"
else
  echo "FAIL incremental: inline body change ($(batches); see $sbtlog)"
  status=1
fi
rm -f target-inline-before.class
rm $check/Unrelated.scala
if client api/compile && [ -z "$(batches)" ] && [ ! -f $classes/meridian/check/Unrelated\$.class ] && ! grep -q Unrelated.scala $manifest; then
  echo "incremental: a removal nothing depends on runs no batch, its products and its manifest entry gone"
else
  echo "FAIL incremental: deletion-only run ($(batches); see $sbtlog)"
  status=1
fi
cat > $check/UsesApiLong.scala <<'EOF'
package meridian.check

object UsesApiLong:
  val x: Long = Api.f
EOF
expect_ok api/compile "incremental: a second dependent compiles"
cp $manifest target-manifest.bak
cp $classes/meridian/check/Api\$.class target-api.bak
cp $classes/meridian/check/Api.tasty target-api-tasty.bak
before=$(contents $classes)
sed -i.bak 's/def f: Long = 1L/def f: String = "1"/' $check/Api.scala
if ! client api/compile && [ "$(batches)" = "teq: 1 source of classes;" ] && has_output "UsesApiLong.scala" && cmp -s target-manifest.bak $manifest && cmp -s target-api.bak $classes/meridian/check/Api\$.class && cmp -s target-api-tasty.bak $classes/meridian/check/Api.tasty && [ "$(contents $classes)" = "$before" ]; then
  echo "incremental: a failed second batch leaves the first batch's class files, TASTy and manifest as they were, the whole directory"
else
  echo "FAIL incremental: the failed second batch's rollback ($(batches); see $sbtlog)"
  status=1
fi
sed -i.bak 's/def f: String = "1"/def f: Long = 1L/' $check/Api.scala
if client api/compile && [ -z "$(batches)" ]; then echo "incremental: the change taken back compiles nothing"; else echo "FAIL incremental: after the rollback ($(batches); see $sbtlog)"; status=1; fi
rm -f target-manifest.bak target-api.bak target-api-tasty.bak
sjsclasses=target/out/sjs1/scala-3.8.4/sjscore/classes
cp sjscore-src/core/Core.scala target-core-rollback.bak
printf 'package core\n\n// %s\nobject Dep:\n  val m: Int = Core.n\n' "$nonce" > sjscore-src/core/Dep.scala
expect_ok sjscore/compile "incremental (Scala.js): a dependent compiles"
cp $sjsclasses/teq-products.json target-manifest.bak
cp $sjsclasses/core/Core\$.teq target-stamp.bak
before=$(contents $sjsclasses)
sed -i.bak 's/val n: Int = 1/val n: String = "1"/' sjscore-src/core/Core.scala
if ! client sjscore/compile && [ "$(batches)" = "teq: 1 source of classes;" ] && cmp -s target-manifest.bak $sjsclasses/teq-products.json && cmp -s target-stamp.bak $sjsclasses/core/Core\$.teq && [ -f $sjsclasses/core/Core.tasty ] && [ "$(contents $sjsclasses)" = "$before" ]; then
  echo "incremental (Scala.js): a failed second batch leaves the first batch's stamps, TASTy and manifest as they were, the whole directory"
else
  echo "FAIL incremental (Scala.js): the failed second batch's rollback ($(batches); see $sbtlog)"
  status=1
fi
cp target-core-rollback.bak sjscore-src/core/Core.scala
rm -f sjscore-src/core/Dep.scala sjscore-src/core/Core.scala.bak target-core-rollback.bak target-manifest.bak target-stamp.bak
expect_ok sjscore/compile "incremental (Scala.js): the dependent taken out"
# A first batch that drops runtime classes no other product needs (an `Any` equality's), then a
# failed second: the runtime classes, which zinc does not own, are back with the rest.
jvmclasses=target/out/jvm/scala-3.8.4/jvmcore/classes
cp sjscore-src/core/Core.scala target-core-runtime.bak
printf '  def same(x: Any, y: Any): Boolean = x == y\n' >> sjscore-src/core/Core.scala
printf 'package core\n\n// %s\nobject Same:\n  def check: Boolean = Core.same(1, 1)\n' "$nonce" > sjscore-src/core/Same.scala
expect_ok jvmcore/compile "incremental (runtime): a source whose code needs runtime classes compiles"
before=$(contents $jvmclasses)
cp target-core-runtime.bak sjscore-src/core/Core.scala
if ! client jvmcore/compile && [ "$(batches)" = "teq: 1 source of classes;" ] && has_output "Same.scala" && [ -f $jvmclasses/scala/runtime/jvm\$package\$.class ] && [ "$(contents $jvmclasses)" = "$before" ]; then
  echo "incremental (runtime): a failed second batch puts back the runtime classes its first batch dropped, the whole directory as it was"
else
  echo "FAIL incremental (runtime): the rollback after a batch that dropped runtime classes ($(batches); see $sbtlog)"
  status=1
fi
rm -f sjscore-src/core/Same.scala target-core-runtime.bak
expect_ok "jvmcore/compile; sjscore/compile" "incremental (runtime): the caller taken out"
rm -f $check/Api.scala $check/UsesApi.scala $check/UsesInline.scala $check/UsesApiLong.scala $check/*.bak api-test-src/meridian/apitest/UsesApiSuite.scala
expect_ok api/Test/compile "incremental: the check's sources taken out"
# The setup and the compile cache carry teq's identity: a change of the cacheable state after a
# passed compile compiles everything again, as a change of the binary does.
if client 'set api / teqCacheableState := Seq("meridian.server.Main"); api/compile' && printf '%s\n' "$(batches)" | grep -Eq '^teq: [0-9]{3,} sources of classes;$'; then
  echo "compiler: a teqCacheableState change after a passed compile compiles the configuration again"
else
  echo "FAIL compiler: teqCacheableState change ($(batches); see $sbtlog)"
  status=1
fi
expect_ok 'set api / teqCacheableState := Nil; api/compile' "compiler: the cacheable state taken back"
# teqCacheableState reaches the JVM's commands and the full link: a name the program lacks is an
# error of the compile, of teqBuild and of teqFullLinkJS, a name it has is accepted.
nowhere="--cacheable-state meridian.server.Nowhere: no such object"
expect_fail 'set api / teqCacheableState := Seq("meridian.server.Nowhere"); api/compile' "cacheable state naming no object fails the compile"
if has_output "$nowhere"; then echo "compiler: the compile names the missing object"; else echo "FAIL compiler: the compile's error of the name (see $sbtlog)"; status=1; fi
expect_fail api/teqBuild "cacheable state naming no object fails the JVM teqBuild"
if has_output "$nowhere"; then echo "compiler: teqBuild names the missing object"; else echo "FAIL compiler: teqBuild's error of the name (see $sbtlog)"; status=1; fi
# The failed compile left api's class directory empty, and sbt 2 answers the compiles that put the
# state back from its cache without re-extracting a directory whose archive its link already names
# (a failed compile under a changed setup); `clean`
# first, as a user does.
expect_ok api/clean "the class directory the failed compile emptied cleaned"
expect_ok 'set api / teqCacheableState := Seq("meridian.server.Main"); api/compile; api/teqBuild' "cacheable state naming an object of the program"
expect_ok 'set api / teqCacheableState := Nil; api/compile' "the cacheable state taken back"
expect_fail 'set frontend / teqCacheableState := Seq("meridian.web.Nowhere"); frontend/teqFullLinkJS' "cacheable state naming no object fails the full link"
if has_output "--cacheable-state meridian.web.Nowhere: no such object"; then echo "compiler: the full link names the missing object"; else echo "FAIL compiler: the full link's error of the name (see $sbtlog)"; status=1; fi
expect_ok 'set frontend / teqCacheableState := Nil; frontend/teqFullLinkJS' "the full link's cacheable state taken back"
# --- The Scala.js tests ---------------------------------------------------------------------------
# browserdemo's Test configuration (browserdemo-test-src/) under TEQ_COMPILER=1: `compile` is a
# check through zinc that writes TASTy and a stamp per class and no IR, sbt's discovery finds a zio-test, a munit
# and a utest suite in its analysis, and `test` runs them under node through sbt-scalajs's test
# adapter over the link of teq (`Test/fastLinkJS`, the test bridge its entry point), testOnly
# whatever sbt's cache holds of earlier runs. Suites that
# fail on purpose (`demoFailing`) are reported by their frameworks with their positions; `test` is
# testQuick, whose digests skip a passed suite, re-run it after an edit of its source and re-run
# every suite after an edit of the main sources; a type error in a test source is reported with its
# position; and the same suites run with the toggle off, through scalac and the Scala.js linker.
timeout 60 sbt --client shutdown > /dev/null 2>&1
rm -rf target/out/sjs1/scala-3.8.4/browserdemo target/out/sjs1/scala-3.8.4/browserdemo.backend
tclasses=target/out/sjs1/scala-3.8.4/browserdemo/test-classes
if timed_ms client browserdemo/Test/compile; then echo "scalajs tests: cold Test/compile ($TIMED_MS ms wall clock through sbt, the server's start included)"; else echo "FAIL scalajs tests: cold Test/compile (see $sbtlog)"; status=1; fi
if [ -f $tclasses/demo/ThemeSuite.teq ] && [ -f $tclasses/demo/ThemeTests.teq ] && [ -f $tclasses/demo/ThemeTests\$.teq ] && [ -f $tclasses/demo/ThemeSuite.tasty ] && [ -z "$(find $tclasses -name '*.sjsir' -o -name '*.class')" ]; then
  echo "scalajs tests: the test classes are stamped, with their TASTy and no IR or class file written"
else
  echo "FAIL scalajs tests: the test class directory (see $tclasses)"
  status=1
fi
if timed_ms client browserdemo/Test/compile; then echo "scalajs tests: warm Test/compile ($TIMED_MS ms wall clock through sbt)"; else echo "FAIL scalajs tests: warm Test/compile (see $sbtlog)"; status=1; fi
# sbt-scalajs asks the test bridge for the frameworks it has: discovery links the tests.
if timed_ms client "show browserdemo/Test/definedTests" && has_output "demo.ThemeSpec : subclass(true, zio.test.ZIOSpecAbstract)" && has_output "demo.ThemeSuite : subclass(false, munit.Suite)" && has_output "demo.ThemeTests : subclass(true, utest.TestSuite)" && [ "$(printf '%s\n' "$CLIENT_OUT" | grep -c '\* Test demo\.')" = 3 ]; then
  echo "scalajs tests: definedTests finds the zio-test, munit and utest suites, each once ($TIMED_MS ms wall clock through sbt, the first link of the tests included)"
else
  echo "FAIL scalajs tests: definedTests (see $sbtlog)"
  status=1
fi
# testOnly runs what it names whatever sbt's cache holds of earlier runs; `test` after it skips.
if timed_ms client browserdemo/testOnly && has_output "Passed: Total 6, Failed 0" && has_output "+ Themes" && has_output "demo.ThemeSuite finished: 0 failed" && has_output "Tests: 2, Passed: 2, Failed: 0"; then
  echo "scalajs tests: testOnly runs the three frameworks' suites under node, all passing ($TIMED_MS ms wall clock through sbt, linked already)"
else
  echo "FAIL scalajs tests: testOnly (see $sbtlog)"
  status=1
fi
if client browserdemo/test && has_output "No tests to run for browserdemo / Test / testQuick"; then echo "scalajs tests: test skips the suites that passed"; else echo "FAIL scalajs tests: test after testOnly (see $sbtlog)"; status=1; fi
if timed_ms client "browserdemo/testOnly demo.ThemeSuite" && has_output "demo.ThemeSuite finished: 0 failed, 0 ignored, 2 total" && ! has_output "+ Themes"; then
  echo "scalajs tests: testOnly runs the suite it names ($TIMED_MS ms wall clock through sbt, warm)"
else
  echo "FAIL scalajs tests: testOnly (see $sbtlog)"
  status=1
fi
suite=browserdemo-test-src/demo/ThemeSuite.scala
cp $suite target-suite.bak
sed -i.bak "s/assertEquals(Themes.next(Themes.next(Theme.Light)), Theme.Dark(2))/assertEquals(Themes.next(Themes.next(Theme.Light)), Theme.Dark(2), \"run $nonce\")/" $suite
if client browserdemo/test && has_output "demo.ThemeSuite finished: 0 failed, 0 ignored, 2 total"; then echo "scalajs tests: test re-runs a suite after an edit of its source"; else echo "FAIL scalajs tests: test after a suite edit (see $sbtlog)"; status=1; fi
cp target-suite.bak $suite
model=browserdemo-src/demo/model/Theme.scala
cp $model target-model.bak
sed -i.bak "s/case Theme.Dark(level) => Theme.Dark(level + 1)/case Theme.Dark(level) => Theme.Dark(level + 1 + 0 * $nonce)/" $model
if client browserdemo/test && has_output "Passed: Total 6, Failed 0"; then echo "scalajs tests: test re-runs every suite after an edit of the main sources they use"; else echo "FAIL scalajs tests: test after a main edit (see $sbtlog)"; status=1; fi
cp target-model.bak $model
# A project's tests over another's class directory (exportJars off, sjsapp over sjscore): the
# stamps there are what tell the tests' digests of an edit of the other project.
core=sjscore-src/core/Core.scala
cp $core target-core.bak
expect_ok "sjsapp/testOnly" "scalajs tests: a project's suite over another's class directory runs"
if client sjsapp/test && has_output "No tests to run for sjsapp / Test / testQuick"; then echo "scalajs tests: over another's class directory: test skips the suite that passed"; else echo "FAIL scalajs tests: over another's class directory: test after testOnly (see $sbtlog)"; status=1; fi
sed -i.bak "s/val n: Int = 1/val n: Int = 1 + 0 * $nonce/" $core
if client sjsapp/test && has_output "Passed: Total 2, Failed 0"; then echo "scalajs tests: over another's class directory: test re-runs the suite after an edit of that project"; else echo "FAIL scalajs tests: over another's class directory: test after an edit of the other project (see $sbtlog)"; status=1; fi
cp target-core.bak $core
rm -f target-core.bak $core.bak
failing="browserdemo/testOnly demo.FailingThemeSpec demo.FailingThemeSuite demo.FailingThemeTests"
if ! client "set browserdemo / demoFailing := true; $failing" && has_output "FailingThemeSpec.scala:10" && has_output "Dark(1) was not equal to Light" && has_output "FailingThemeSuite.scala:7" && has_output "values are not the same" && has_output "X demo.FailingThemeTests.the light theme goes back to light" && has_output "Failed: Total 3, Failed 3" && has_output "demo.FailingThemeTests"; then
  echo "scalajs tests: failing suites are reported by their frameworks, with their positions, and fail the task"
else
  echo "FAIL scalajs tests: failing suites (see $sbtlog)"
  status=1
fi
expect_ok "set browserdemo / demoFailing := false; browserdemo/Test/compile" "the failing Scala.js suites taken out"
sed -i.bak 's/assertEquals(Themes.next(Theme.Light), Theme.Dark(1))/assertEquals(Themes.next(Theme.Light), Theme.Dark("1"))/' $suite
if ! client browserdemo/Test/compile && has_output "ThemeSuite.scala:7:" && has_output "error"; then echo "scalajs tests: a type error in a test source is reported with its position"; else echo "FAIL scalajs tests: type error in a test source (see $sbtlog)"; status=1; fi
cp target-suite.bak $suite
expect_ok browserdemo/Test/compile "the fix of the Scala.js test source compiles"
if client "set every teqCompiler := false; browserdemo/testOnly" && has_output "Passed: Total 6, Failed 0" && has_output "Fast optimizing"; then echo "scalajs tests: toggle off: scalac and the Scala.js linker run the same suites"; else echo "FAIL scalajs tests: toggle off (see $sbtlog)"; status=1; fi
if client "set every teqCompiler := true; browserdemo/testOnly" && has_output "Passed: Total 6, Failed 0" && ! has_output "Fast optimizing"; then echo "scalajs tests: toggle on again: teq compiles and links the suites afresh"; else echo "FAIL scalajs tests: toggle on again (see $sbtlog)"; status=1; fi
rm -f target-suite.bak target-model.bak $suite.bak $model.bak
# --- Failures, siblings and the toggle per project -------------------------------------------------
# A type error and its retry on both platforms, whose fix compiles nothing (the failed compile
# left the products as they were); a suite added to one file while another fails; a file deleted
# while a type error stands; sibling projects compiled at once; an upstream inline body edit
# reaching the tests on both platforms (sjsapp over sjscore, jvmapp over jvmcore); the toggle of
# one project in either dependency direction; a compile cancelled while teq runs.
timeout 60 sbt --client shutdown > /dev/null 2>&1
cp $edited target-edited.bak
sed -i.bak 's/case Invalid(message) => ApiFailure.Invalid(message)/case Invalid(message) => ApiFailure.Invalid(42)/' $edited
expect_fail api/compile "failures: a type error fails the compile"
expect_fail api/compile "failures: the unchanged retry fails again"
cp target-edited.bak $edited
if client api/compile && [ -z "$(batches)" ]; then echo "failures: the fix compiles nothing, the products as they were"; else echo "FAIL failures: the fix of a failure ($(batches); see $sbtlog)"; status=1; fi
cp $core target-core.bak
sed -i.bak 's/val n: Int = 1/val n: Int = "1"/' $core
expect_fail sjscore/compile "failures: a type error of a check"
cp target-core.bak $core
if client sjscore/compile && [ -z "$(batches)" ]; then echo "failures: its fix compiles nothing, the stamps as they were"; else echo "FAIL failures: the fix of the check's failure ($(batches); see $sbtlog)"; status=1; fi
expect_ok sjscore/clean "failures: clean"
sed -i.bak 's/val n: Int = 1/val n: Int = "1"/' $core
expect_fail sjscore/compile "failures: a type error after clean"
expect_fail sjscore/compile "failures: and on its retry"
cp target-core.bak $core
# The fix brings back sources sbt 2's compile cache knows: restored from the cache, or compiled.
if client sjscore/compile && [ -f target/out/sjs1/scala-3.8.4/sjscore/classes/core/Core.teq ]; then echo "failures: and its fix compiles, its stamps there"; else echo "FAIL failures: the fix after clean and a failure ($(batches); see $sbtlog)"; status=1; fi
munit=api-test-src/meridian/apitest/MunitSuite.scala
scalatest=api-test-src/meridian/apitest/ScalatestSuite.scala
cp $munit target-munit.bak
cp $scalatest target-scalatest.bak
expect_ok api/Test/compile "failures: the tests compile"
printf '\nclass LateSuite extends munit.FunSuite:\n  test("late") { assert(true) }\n' >> $munit
sed -i.bak 's/List("v1", "sites"), Nil, Nil, None)/List("v1", "sites"), Nil, Nil, 42)/' $scalatest
expect_fail api/Test/compile "failures: a suite added in one file while another has a type error"
cp target-scalatest.bak $scalatest
if client "show api/Test/definedTests" && has_output LateSuite; then echo "failures: the suite is found once the error is fixed"; else echo "FAIL failures: the suite added beside a type error (see $sbtlog)"; status=1; fi
cp target-munit.bak $munit
expect_ok api/Test/compile "failures: the suite taken out again"
for side in jvm sjs; do
  if [ $side = jvm ]; then project=api; broken=$edited; gone=src/api/meridian/check/Vanishing.scala; product=$classes/meridian/check/Vanishing.class
  else project=sjscore; broken=$core; gone=sjscore-src/core/Vanishing.scala; product=target/out/sjs1/scala-3.8.4/sjscore/classes/core/Vanishing.teq; fi
  printf 'package %s\n\nclass Vanishing:\n  def here: Boolean = true\n' "$([ $side = jvm ] && echo meridian.check || echo core)" > $gone
  cp $broken target-broken.bak
  expect_ok $project/compile "failures ($side): a class added"
  if [ $side = jvm ]; then sed -i.bak 's/case Invalid(message) => ApiFailure.Invalid(message)/case Invalid(message) => ApiFailure.Invalid(42)/' $broken
  else sed -i.bak 's/val n: Int = 1/val n: Int = "1"/' $broken; fi
  expect_fail $project/compile "failures ($side): a type error"
  rm $gone
  expect_fail $project/compile "failures ($side): a file deleted while the type error stands"
  cp target-broken.bak $broken
  if client $project/compile && [ ! -f $product ]; then echo "failures ($side): once the error is fixed the deleted file's products are gone"; else echo "FAIL failures ($side): the file deleted during a failure (see $sbtlog)"; status=1; fi
done
rm -f target-broken.bak $edited.bak $core.bak $scalatest.bak
stamp=target/out/sjs1/scala-3.8.4/sjscore/classes/core/Core.teq
before=$(cat $stamp 2> /dev/null)
sed -i.bak "s/val n: Int = 1/val n: Int = 1 + 0 * $nonce/" $core
if client "all sjscore/compile jvmcore/compile browserdemo/compile" && [ "$(cat $stamp)" != "$before" ] && [ -f target/out/jvm/scala-3.8.4/jvmcore/classes/core/Core.class ]; then echo "siblings: three projects compiled at once, each by its compile"; else echo "FAIL siblings: the projects compiled at once (see $sbtlog)"; status=1; fi
cp target-core.bak $core
expect_ok "sjsapp/testOnly; jvmapp/testOnly" "upstream inline: the suites over sjscore and jvmcore pass"
sed -i.bak 's/inline def twice(x: Int): Int = x \* 2/inline def twice(x: Int): Int = x * 3/' $core
printf '\n// %s\n' "$nonce" >> $core
if ! client sjsapp/test && has_output "CoreSuite.scala:8" && has_output "values are not the same"; then echo "upstream inline: an edit of an inline body upstream reaches sjsapp's test"; else echo "FAIL upstream inline: sjsapp (see $sbtlog)"; status=1; fi
if ! client jvmapp/test && has_output "CoreSuite.scala:8" && has_output "values are not the same"; then echo "upstream inline: and jvmapp's"; else echo "FAIL upstream inline: jvmapp (see $sbtlog)"; status=1; fi
cp target-core.bak $core
expect_ok "sjsapp/test; jvmapp/test" "upstream inline: the body taken back"
for side in sjs jvm; do
  if [ $side = sjs ]; then up=sjscore; down=sjsapp; dir=target/out/sjs1/scala-3.8.4/sjscore/classes/core; ir=Core.sjsir; total=3
  else up=jvmcore; down=jvmapp; dir=target/out/jvm/scala-3.8.4/jvmcore/classes/core; ir=Core.tasty; total=2; fi
  if client "set $up / teqCompiler := false; $down/testOnly" && has_output "Passed: Total $total, Failed 0" && [ -f $dir/$ir ] && [ ! -f $dir/Core.teq ]; then echo "toggle ($side): $up through scalac, $down through teq over $up's sources"; else echo "FAIL toggle ($side): $up off (see $sbtlog)"; status=1; fi
  sed -i.bak "s/val n: Int = 1/val n: Int = 1 + 0 * $nonce/" $core
  if client $down/test && has_output "Passed: Total 2, Failed 0"; then echo "toggle ($side): $down's tests re-run after an edit of the scalac-compiled $up"; else echo "FAIL toggle ($side): $down after an edit of $up (see $sbtlog)"; status=1; fi
  cp target-core.bak $core
  if client "set $up / teqCompiler := true; set $down / teqCompiler := false; $down/Test/compile"; then echo "toggle ($side): scalac compiles $down over $up's TASTy, which teq wrote"; else echo "FAIL toggle ($side): $down off over teq's products (see $sbtlog)"; status=1; fi
  expect_ok "set $down / teqCompiler := true; $down/testOnly" "toggle ($side): both on again"
done
# A compile cancelled while teq runs (an interactive client's Ctrl+C): the batch's process ends with
# the compile, nothing of it is published, and the next compile compiles the configuration. The
# client runs under job control, since a script's background commands ignore SIGINT otherwise.
# The batch starts as `teq @<file>`, its arguments in api's target/teq/compile-batch-<digest>.args.
running() { pgrep -f -- "@$PWD/api/target/teq/compile-batch-[0-9a-f]{16}\.args" | wc -l | tr -d ' '; }
expect_ok api/clean "cancel: clean"
before=$(contents $classes)
# A source of this run's own, so that the compile misses sbt 2's cache and compiles from nothing.
cp $edited target-edited.bak
printf '\n// cancel %s\n' "$nonce" >> $edited
rm -f target-fifo && mkfifo target-fifo
set -m
sbt --client -no-colors < target-fifo > target-cancel.log 2>&1 &
shell=$!
set +m
exec 7> target-fifo
for _ in $(seq 1 120); do grep -q "terminate the server with" target-cancel.log && break; sleep 0.5; done
successes=$(grep -c "success" target-cancel.log)
echo "api/compile" >&7
started=
for _ in $(seq 1 600); do [ "$(running)" != 0 ] && started=1 && break; sleep 0.1; done
interrupted=
kill -INT $shell 2> /dev/null && interrupted=1
sleep 3
exec 7>&-
kill $shell 2> /dev/null
wait $shell 2> /dev/null
# The server completes the cancelled compile's rollback after the client is gone.
restored=
for _ in $(seq 1 60); do contents $classes > target-cancel-contents.log; [ "$(cat target-cancel-contents.log)" = "$before" ] && restored=1 && break; sleep 0.5; done
if [ -n "$started" ] && [ -n "$interrupted" ] && [ "$(grep -c "success" target-cancel.log)" = "$successes" ] && [ "$(running)" = 0 ] && [ -n "$restored" ] && client api/compile && printf '%s\n' "$(batches)" | grep -Eq '^teq: [0-9]{3,} sources of classes;$'; then echo "cancel: a compile cancelled while teq runs leaves no process and the class directory as it was, and the next compile compiles the configuration"; else echo "FAIL cancel: after a cancelled compile (started '$started', interrupted '$interrupted', successes $successes then $(grep -c "success" target-cancel.log), teq processes $(running), directory restored '$restored'; see $sbtlog, target-cancel.log, target-cancel-contents.log)"; status=1; fi
cp target-edited.bak $edited
rm -f target-fifo target-core.bak target-munit.bak target-scalatest.bak $core.bak
timeout 60 sbt --client shutdown > /dev/null 2>&1
rm -f target-edited.bak target-shared.bak target-client.out $edited.bak $shared.bak
unset TEQ_COMPILER
fi

# --- a generator the build's own teq runs ----------------------------------------------------------
# browserdemo's images generator is `teq interp scripts/images.scala -- browserdemo-images` (build.sbt):
# the word `teq` is the build's own binary, sbt-teq's teqResolvedBinary under sbt and the binary running
# under `teq`, never a teq of the PATH. A `teq` and a `scala-cli` go first on the PATH, each leaving a line
# in target-sentinels/ran and failing, and each run is a fresh process with the generator's outputs (its
# objects and the module beside the images) and the generators' fingerprint records gone first, no daemon
# of the build left: sbt's compile with TEQ naming the binary by its absolute path; through the launcher
# with TEQ so, `teq compile` and `teq dev` (its loop ended by SIGINT once the outputs are there); and
# `teq compile` through the launcher with TEQ unset, a copy of the lock pinning the binary's sha1 and size
# and a cache of the check's own holding it (the lock put back after). Each writes both outputs, the
# objects among the sources the compile takes (sbt's managedSources), the sentinels untouched. Then, under
# `teq compile`, nothing changed runs nothing, and an image added, then removed, the script alone edited (the
# images compiled as they are first) and the module deleted alone each run it again, its outputs written anew;
# under sbt's compile, the module deleted alone is written again.
if [ -z "$COMPILER_ONLY$RELEASE_ONLY$EXPORT_ONLY" ]; then
teq_abs=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
sentinels=$PWD/target-sentinels
rm -rf "$sentinels" && mkdir -p "$sentinels"
for name in teq scala-cli; do
  printf '#!/bin/sh\necho "%s $*" >> "%s/ran"\nexit 73\n' "$name" "$sentinels" > "$sentinels/$name"
  chmod +x "$sentinels/$name"
done
module=browserdemo-images/assets/images.js
objects=target/teq/browserdemo/compile/src_managed/demo/images
script=scripts/images.scala
added=browserdemo-images/assets/images/shapes/oval.svg
sbt_objects() { find target/out -path '*/browserdemo/src_managed/*/demo/images/*Images.scala' 2> /dev/null; }
untouched() { [ ! -e "$sentinels/ran" ]; }
generated() { [ -f "$module" ] && [ -f "$objects/ShapeImages.scala" ] && [ -f "$objects/MarkImages.scala" ]; }
fresh() {
  TEQ=$teq_abs timeout 30 ./teq stop > /dev/null 2>&1
  rm -f "$module" target/teq/generators/browserdemo-compile-*
  rm -rf "$objects"
  sbt_objects | while read -r f; do rm -f "$f"; done
}
glog=target-generator.log
: > $glog
fresh
if TEQ=$teq_abs PATH="$sentinels:$PATH" timeout 340 sbt --server --batch browserdemo/compile 'show browserdemo/Compile/managedSources' > target-generator-sbt.log 2>&1 &&
  [ -f "$module" ] && [ "$(sbt_objects | wc -l | tr -d ' ')" = 2 ] && [ "$(grep -c 'demo/images/[A-Za-z]*Images\.scala' target-generator-sbt.log)" = 2 ] && untouched; then
  echo "generator: sbt's compile runs teq interp by the binary TEQ names, the PATH's teq and scala-cli untouched; the module written, the objects among the sources compiled"
else
  echo "FAIL generator: sbt's compile (sentinels: $(cat "$sentinels/ran" 2> /dev/null); see target-generator-sbt.log)"
  status=1
fi
rm -f "$module"
if TEQ=$teq_abs PATH="$sentinels:$PATH" timeout 340 sbt --server --batch browserdemo/compile >> $glog 2>&1 && [ -f "$module" ] && untouched; then
  echo "generator: sbt's compile writes the module again once it is deleted alone"
else
  echo "FAIL generator: sbt's compile after the module was deleted (see $glog)"
  status=1
fi
compile() { TEQ=$teq_abs PATH="$sentinels:$PATH" timeout 340 ./teq compile browserdemo >> $glog 2>&1; }
runs() { grep -c '^browserdemo/compile: images: ' $glog; }
fresh
if compile && generated && untouched; then
  echo "generator: teq compile through the launcher runs teq interp by the binary TEQ names, the PATH's teq and scala-cli untouched"
else
  echo "FAIL generator: teq compile (sentinels: $(cat "$sentinels/ran" 2> /dev/null); see $glog)"
  status=1
fi
before=$(runs)
if compile && [ "$(runs)" = "$before" ]; then echo "generator: teq compile with nothing changed runs nothing"; else echo "FAIL generator: an unchanged teq compile ran the generator (see $glog)"; status=1; fi
cp browserdemo-images/assets/images/shapes/circle.svg "$added"
if compile && grep -q 'val oval: String' "$objects/ShapeImages.scala" && grep -q '  oval: ' "$module"; then
  echo "generator: an image added, teq compile writes its object and the module anew"
else
  echo "FAIL generator: teq compile after an image was added (see $glog)"
  status=1
fi
rm -f "$added"
if compile && ! grep -q 'oval' "$objects/ShapeImages.scala" "$module"; then
  echo "generator: the image removed, teq compile writes them anew without it"
else
  echo "FAIL generator: teq compile after the image was removed (see $glog)"
  status=1
fi
# The images as they were compiled last: the script's edit alone is what runs the generator again.
cp "$script" target-images-script.bak
python3 -c 'import sys; p = sys.argv[1]; t = open(p).read(); open(p, "w").write(t.replace("Do not edit.", "Do not edit; written again."))' "$script"
if compile && grep -q 'written again' "$module" && grep -q 'written again' "$objects/MarkImages.scala"; then
  echo "generator: the script alone edited, teq compile runs it again"
else
  echo "FAIL generator: teq compile after the script was edited (see $glog)"
  status=1
fi
cp target-images-script.bak "$script" && rm -f target-images-script.bak
compile
rm -f "$module"
before=$(runs)
if compile && [ -f "$module" ] && [ "$(runs)" -gt "$before" ] && ! grep -q 'written again\|oval' "$module" && untouched; then
  echo "generator: the module deleted alone, teq compile writes it again"
else
  echo "FAIL generator: teq compile after the module was deleted alone (see $glog)"
  status=1
fi
dev_port=${GENERATOR_DEV_PORT:-5400}
fresh
TEQ=$teq_abs PATH="$sentinels:$PATH" timeout 200 ./teq dev browserdemo -- --port "$dev_port" --strictPort --host 127.0.0.1 > target-generator-dev.log 2>&1 &
dev=$!
written=
for _ in $(seq 1 240); do generated && written=1 && break; sleep 0.5; done
kill -INT $dev 2> /dev/null
for _ in $(seq 1 60); do kill -0 $dev 2> /dev/null || break; sleep 0.5; done
wait $dev 2> /dev/null
left=$(ps -eo args | grep -F -- "--port $dev_port --strictPort" | grep -v grep)
if [ -n "$written" ] && untouched && [ -z "$left" ]; then
  echo "generator: teq dev through the launcher runs teq interp by the binary TEQ names, the PATH's teq and scala-cli untouched"
else
  echo "FAIL generator: teq dev (outputs written '$written', processes left '$left'; see target-generator-dev.log)"
  status=1
fi
# The launcher with TEQ unset takes the binary the lock pins from teq's cache: a lock pinning this binary.
cp teq.lock target-generator-fixture.lock
cache=$PWD/target-launcher-cache
rm -rf "$cache"
case "$(uname -s)" in Darwin) launcher_os=osx ;; *) launcher_os=linux ;; esac
case "$(uname -m)" in arm64 | aarch64) launcher_arch=aarch_64 ;; *) launcher_arch=x86_64 ;; esac
pinned=$(python3 - "$teq_abs" "$launcher_os-$launcher_arch" << 'PY'
import hashlib, re, sys
binary, classifier = sys.argv[1:]
data = open(binary, "rb").read()
sha1 = hashlib.sha1(data).hexdigest()
lock = open("teq.lock").read()
pinned = re.sub(r"(?m)^(  %s: \S+) [0-9a-f]{40} [0-9]+$" % re.escape(classifier), lambda m: "%s %s %d" % (m.group(1), sha1, len(data)), lock)
# A table without the classifier's line (a release not published yet) gains one, at a URL nothing fetches from:
# the launcher takes the cache's copy by the pin.
version = re.match(r"teq: (\S+)\n", lock).group(1)
if pinned == lock:
    pinned = lock.replace("\nbinaries: {}\n", "\nbinaries:\n  %s: https://releases.invalid/teq-%s-%s %s %d\n" % (classifier, version, classifier, sha1, len(data)), 1)
open("teq.lock", "w").write(pinned)
print(sha1 if pinned != lock else "")
PY
)
version=$(sed -n '1s/^teq: //p' teq.lock)
fresh
if [ -n "$pinned" ] && mkdir -p "$cache/bin/$pinned" && cp "$teq_abs" "$cache/bin/$pinned/teq-$version-$launcher_os-$launcher_arch" &&
  env -u TEQ TEQ_CACHE_DIR="$cache" PATH="$sentinels:$PATH" timeout 340 ./teq compile browserdemo >> $glog 2>&1 && generated && untouched; then
  echo "generator: teq compile through the launcher with TEQ unset runs teq interp by the binary the lock pins, the PATH's teq and scala-cli untouched"
else
  echo "FAIL generator: teq compile with the lock's binary (sentinels: $(cat "$sentinels/ran" 2> /dev/null); see $glog)"
  status=1
fi
env -u TEQ TEQ_CACHE_DIR="$cache" timeout 30 ./teq stop > /dev/null 2>&1
cp target-generator-fixture.lock teq.lock
rm -rf "$cache" "$sentinels" target-generator-fixture.lock
fi

if [ -z "$COMPILER_ONLY$RELEASE_ONLY$GENERATOR_ONLY" ]; then
  ./check-export.sh || status=1
fi
exit $status
