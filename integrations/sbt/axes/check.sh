#!/bin/bash
# The plugin's two sbt axes (docs/TOOLING.md, "The plugin"): sbt-teq for sbt 1 (`sbt-teq_2.12_1.0`) and for sbt 2
# (`sbt-teq_sbt2_3`), built from one source tree, over the two portable fixtures beside this script, each exported
# under sbt 1.13.0 and sbt 2.0.8 from copies that differ in project/build.properties alone.
#
# equal/ is a build whose facts the two sbts resolve alike (no project with a Test configuration depends on
# another, the Docker stage's and the generated sources' directories set): each axis's export is the committed
# equal/teq.lock but for the digest of project/build.properties and the hash over the build's files
# (`check-export.py axes`). order/ holds the facts each sbt resolves its own way, which each axis's export carries
# as its sbt has them: a Test class path of a project that depends on another holds the two projects in sbt 1's
# order on sbt 1 and sbt 2's on sbt 2 (ClasspathImpl.interSort: sbt 1 visits the project's configurations before
# its dependencies, sbt 2 the dependencies of each configuration as it goes), the Docker stage's default directory
# is under each sbt's target, BuildInfo's sbtVersion the sbt that runs; each export is its committed lock
# (order/teq.lock sbt 2's, order/teq.sbt1.lock sbt 1's).
#
# On each axis: the export runs (at the build's level, whose streams sbt 1 has under no ThisBuild task); the projects
# on every configuration's classpath are, in order, those of sbt's own fullClasspath, and its jars sbt's
# externalDependencyClasspath (`check-export.py internal`, `classpaths`); the launchers written beside the lock are
# teq's; the generators sbt alone runs are recorded and `teq compile` refuses gen, naming them, and exits 2 over
# every project while it compiles the others; `teq compile`, `teq test` (lib's munit suites; sjs's refused, as teq
# runs no Scala.js suite yet) and `teq run` (app's alias, the arguments passed) over each axis's export of equal
# give the same answers; `teq dev sjs` runs the dev script of sjs's package.json after npm's install, until SIGINT;
# sbt's own tasks with teq as the compiler (`TEQ_COMPILER=1`) run lib's suites (`test` twice runs them twice on
# sbt 1, whose `test` is the full run; sbt 2's is testQuick), app's main and sjs's suites through sbt-scalajs's
# test adapter, and `++3.8.4!` and `+lib/testOnly`; the release the build pins is resolved from a mirror of its GitHub release
# (teqResolvedBinary), a SNAPSHOT of the compiler from the build's Maven repository (teqVersion overridden), whose
# launcher runs it through the lock's binaries table; a release the mirror does not serve is named in a warning and
# the lock names no binary.
#
# The plugin is the checkout's, published on both axes under a SNAPSHOT of its own into a Maven repository of the
# run's (`^publish`, nothing reaching ~/.ivy2/local), or TEQ_PLUGIN_VERSION's, published locally on both axes
# (`sbt --batch 'set version := "1.0.1-<branch>-SNAPSHOT"; ^publishLocal'`). TEQ names the binary (default: the
# repository's release build); AXES the sbt versions (default "1.13.0 2.0.8"; a version starting 2 is the sbt 2
# axis); AXES_RECORD=1 writes each export over its committed lock instead of comparing, its binaries table empty,
# which names the run's mirror (the comparisons take the header from the committed lock). Needs sbt (its launcher
# runs either sbt), node, python3 and the network for the fixtures' libraries. Every sbt run is bounded.
cd "$(dirname "$0")" || exit 1
here=$PWD
repo=$(cd ../../.. && pwd)
teq=${TEQ:-$repo/target/release/teq}
teq=$(cd "$(dirname "$teq")" && pwd)/$(basename "$teq")
axes=${AXES:-1.13.0 2.0.8}
compare() { python3 "$repo/integrations/sbt/example/check-export.py" "$@"; }
work=$(cd "$(mktemp -d)" && pwd -P)
status=0
pass() { echo "axes: $1"; }
fail() { echo "FAIL axes: $1"; status=1; }
mirror_pid=
# The run's directory is kept when a case fails, its path printed.
trap '[ -z "$mirror_pid" ] || kill "$mirror_pid" 2> /dev/null; if [ $status = 0 ]; then rm -rf "$work"; else echo "axes: the run is kept in $work"; fi' EXIT

# The release the fixtures pin, 0.1.8, as a mirror of its GitHub release serves one (bench/release-mirror.py), its
# binaries scripts printing `teq 0.1.8 axes`; and a SNAPSHOT of the compiler, 0.1.8-SNAPSHOT, in the Maven repository
# the mirror serves too (TeqPlugin.SnapshotGroup's group, each file with its .sha1), printing `teq 0.1.8 snapshot`.
python3 "$repo/integrations/sbt/example/release-fixture.py" "$work/releases" 0.1.8 axes > /dev/null
classifier=$(python3 -c 'import platform; s, m = platform.system().lower(), platform.machine().lower(); print({"darwin": "osx"}.get(s, s) + "-" + {"amd64": "x86_64", "arm64": "aarch_64", "aarch64": "aarch_64"}.get(m, m))')
snapshot=$work/releases/maven/build/teq/teq/0.1.8-SNAPSHOT
mkdir -p "$snapshot"
printf '<project><modelVersion>4.0.0</modelVersion><groupId>build.teq</groupId><artifactId>teq</artifactId><version>0.1.8-SNAPSHOT</version></project>\n' > "$snapshot/teq-0.1.8-SNAPSHOT.pom"
printf '#!/bin/sh\necho "teq 0.1.8 snapshot"\n' > "$snapshot/teq-0.1.8-SNAPSHOT-$classifier.exe"
for f in "$snapshot"/*; do sha1sum "$f" | cut -d' ' -f1 | tr -d '\n' > "$f.sha1"; done
timeout 1800 python3 -B "$repo/bench/release-mirror.py" "$work/releases.port" "$work/releases" "$work/releases/maven" > /dev/null 2>&1 &
mirror_pid=$!
for _ in $(seq 100); do [ -s "$work/releases.port" ] && break; sleep 0.1; done
export AXES_RELEASES=http://127.0.0.1:$(cat "$work/releases.port")/releases/download
mavenbase=http://127.0.0.1:$(cat "$work/releases.port")/maven

# The plugin.
if [ -n "$TEQ_PLUGIN_VERSION" ]; then
  export AXES_PLUGIN_REPOSITORY=file://$work/maven
else
  export TEQ_PLUGIN_VERSION=$(cat ../plugin-version.txt)-axes-SNAPSHOT AXES_PLUGIN_REPOSITORY=file://$work/maven
  if (cd .. && timeout 340 sbt --server --batch "set version := \"$TEQ_PLUGIN_VERSION\"; set publishTo := Some(\"axes\" at \"$AXES_PLUGIN_REPOSITORY\"); ^publish") > "$work/publish.log" 2>&1 &&
    [ -f "$work/maven/build/teq/sbt-teq_2.12_1.0/$TEQ_PLUGIN_VERSION/sbt-teq_2.12_1.0-$TEQ_PLUGIN_VERSION.jar" ] &&
    [ -f "$work/maven/build/teq/sbt-teq_sbt2_3/$TEQ_PLUGIN_VERSION/sbt-teq_sbt2_3-$TEQ_PLUGIN_VERSION.jar" ]; then
    pass "the checkout's plugin published on both axes, sbt-teq_2.12_1.0 and sbt-teq_sbt2_3, under $TEQ_PLUGIN_VERSION"
  else
    fail "the publication of both axes: $(tail -20 "$work/publish.log")"
    exit 1
  fi
fi

# sbt <dir> <log> <commands>: one bounded sbt run in the copy.
sbt_in() {
  local dir=$1 log=$2
  shift 2
  (cd "$dir" && timeout 340 sbt --server --batch "$@") > "$log" 2>&1
}
# copy <fixture> <sbt version> [<case>]: a copy of the fixture under the run's directory whose build.properties names
# the sbt.
copy() {
  local dir=$work/$1-$2${3:+-$3}
  rm -rf "$dir" && cp -R "$here/$1" "$dir" && rm -f "$dir/teq.lock" "$dir/teq.sbt1.lock"
  printf 'sbt.version=%s\n' "$2" > "$dir/project/build.properties"
  echo "$dir"
}
# committed <fixture> <sbt version>: the lock the fixture commits for the axis.
committed() {
  if [ "$1" = order ] && [[ "$2" != 2* ]]; then echo "$here/order/teq.sbt1.lock"; else echo "$here/$1/teq.lock"; fi
}

declare -A locks
for version in $axes; do
  for fixture in equal order; do
    dir=$(copy $fixture "$version")
    # The projects' classpaths as sbt has them, in the order of the export's configurations, after the export.
    if ! sbt_in "$dir" "$work/$fixture-$version.log" teqExportAll; then
      fail "$fixture on sbt $version: teqExportAll (see below)"
      tail -30 "$work/$fixture-$version.log"
      continue
    fi
    locks[$fixture-$version]=$dir/teq.lock
    full=() external=()
    for c in $(compare configurations "$dir/teq.lock"); do
      full+=("export $c/fullClasspath")
      external+=("export $c/externalDependencyClasspath")
    done
    if sbt_in "$dir" "$work/$fixture-$version-classpaths.log" "${full[@]}" "${external[@]}" &&
      python3 -c 'import sys; lines = open(sys.argv[1], encoding="utf-8").read().splitlines(); half = sum(1 for l in lines if l.startswith("List(") or l.startswith("/")) // 2; out = [l for l in lines if l.startswith("List(") or l.startswith("/")]; open(sys.argv[2], "w").write("\n".join(out[:half]) + "\n"); open(sys.argv[3], "w").write("\n".join(out[half:]) + "\n")' "$work/$fixture-$version-classpaths.log" "$work/$fixture-$version-full.log" "$work/$fixture-$version-external.log" &&
      internal=$(compare internal "$dir/teq.lock" "$work/$fixture-$version-full.log") &&
      external=$(compare classpaths "$dir/teq.lock" "$work/$fixture-$version-external.log"); then
      pass "$fixture on sbt $version: every configuration's projects are sbt's fullClasspath's in its order ($internal), its jars sbt's externalDependencyClasspath ($external)"
    else
      fail "$fixture on sbt $version: the classpaths against sbt's: $internal $external (see $work/$fixture-$version-classpaths.log)"
    fi
    if cmp -s "$dir/teq" "$repo/tools/launcher/teq" && cmp -s "$dir/teq.cmd" "$repo/tools/launcher/teq.cmd" && [ -x "$dir/teq" ]; then
      pass "$fixture on sbt $version: the launchers written beside the lock are teq's"
    else
      fail "$fixture on sbt $version: the launchers $(ls -l "$dir/teq" "$dir/teq.cmd" 2>&1)"
    fi
    wanted=$(committed $fixture "$version")
    if [ "${AXES_RECORD:-}" = 1 ] && { [ $fixture = order ] || [[ "$version" == 2* ]]; }; then
      compare headerless "$dir/teq.lock" "$wanted"
      pass "$fixture on sbt $version: recorded as $(basename "$wanted")"
    elif [ $fixture = equal ] && [[ "$version" != 2* ]]; then
      if out=$(compare axes "$dir/teq.lock" "$wanted" header); then
        pass "equal on sbt $version: the committed lock (sbt 2's) but for the digest of project/build.properties and the hash over the build's files"
      else
        fail "equal on sbt $version against the committed lock: $out"
      fi
    elif out=$(compare fixture "$dir/teq.lock" "$wanted"); then
      pass "$fixture on sbt $version: the committed $(basename "$wanted"), its header the run's mirror's"
    else
      fail "$fixture on sbt $version: not the committed $(basename "$wanted"): $out"
    fi
  done
done
# The two axes' exports of equal in one run, the mirror's URLs alike: the same bytes but for the build's files.
first=
for version in $axes; do
  [ -n "${locks[equal-$version]:-}" ] || continue
  if [ -z "$first" ]; then first=$version
  elif out=$(compare axes "${locks[equal-$first]}" "${locks[equal-$version]}"); then
    pass "equal: the exports of sbt $first and sbt $version are the same bytes but for project/build.properties' digest and the hash over the build's files"
  else
    fail "equal: the exports of sbt $first and sbt $version: $out"
  fi
done

# teq over each axis's export of equal: the same answers.
for version in $axes; do
  dir=$work/equal-$version
  [ -n "${locks[equal-$version]:-}" ] || continue
  (
    cd "$dir" || exit 1
    export TEQ=$teq
    out=$(timeout 300 ./teq compile gen 2>&1); code=$?
    if [ $code = 2 ] && [[ "$out" == *"gen"* ]]; then echo "compile gen: refused ($code)"; else echo "compile gen: $code $out"; fi
    out=$(timeout 300 ./teq compile 2>&1); code=$?
    if [ $code = 2 ]; then echo "compile: exit 2"; else echo "compile: $code $out"; fi
    out=$(timeout 300 ./teq compile lib app sjs 2>&1); echo "compile lib app sjs: $? $(grep -c error <<< "$out")"
    out=$(timeout 300 ./teq test lib 2>&1); echo "test lib: $? $(grep -E '^lib/test: ' <<< "$out" | sed 's/ in [0-9].*//')"
    out=$(timeout 300 ./teq test sjs 2>&1); echo "test sjs: $? $out"
    out=$(timeout 300 ./teq run app hello -- one two 2> /dev/null); echo "run app hello: $? $out"
    timeout 60 ./teq stop > /dev/null 2>&1
  ) > "$work/verbs-$version.txt" 2>&1
done
first=
for version in $axes; do
  [ -f "$work/verbs-$version.txt" ] || continue
  if grep -q "^compile gen: refused" "$work/verbs-$version.txt" && grep -q "^compile: exit 2" "$work/verbs-$version.txt" &&
    grep -q "^compile lib app sjs: 0 0" "$work/verbs-$version.txt" && grep -q "^test lib: 0 lib/test: 1 suite, 2 passed, 0 failed$" "$work/verbs-$version.txt" &&
    grep -q "^test sjs: 2 teq: sjs is a Scala.js project, whose suites teq does not run yet$" "$work/verbs-$version.txt" &&
    grep -q "^run app hello: 0 app one two$" "$work/verbs-$version.txt"; then
    pass "teq over equal's export on sbt $version: gen refused, a bare compile exiting 2, lib, app and sjs compiled, lib's suites passing (sjs's refused, as Scala.js suites are), app's alias run"
  else
    fail "teq over equal's export on sbt $version: $(cat "$work/verbs-$version.txt")"
  fi
  if [ -z "$first" ]; then first=$version
  elif cmp -s "$work/verbs-$first.txt" "$work/verbs-$version.txt"; then pass "teq answers alike over the exports of sbt $first and sbt $version"
  else fail "teq answers differently over the exports of sbt $first and sbt $version: $(diff "$work/verbs-$first.txt" "$work/verbs-$version.txt")"
  fi
done

# teq dev over each axis's export of equal: sjs's dev loop, the package manager's install in sjs (npm, its package
# holding no dependency) and then the dev script with the arguments after --, TEQ naming the binary, until SIGINT
# ends the loop with 130.
for version in $axes; do
  dir=$work/equal-$version
  [ -n "${locks[equal-$version]:-}" ] || continue
  rm -f "$dir/sjs/dev-started"
  (cd "$dir" && TEQ=$teq timeout --preserve-status -s INT 60 ./teq dev sjs -- one > "$work/dev-$version.out" 2>&1) &
  devpid=$!
  for _ in $(seq 550); do [ -s "$dir/sjs/dev-started" ] && break; sleep 0.1; done
  sleep 1
  pkill -INT -f "teq dev sjs" 2> /dev/null
  wait "$devpid"
  code=$?
  if [ "$(cat "$dir/sjs/dev-started" 2> /dev/null)" = "one with TEQ" ] && [ $code = 130 ] && [ -f "$dir/sjs/package-lock.json" ]; then
    pass "teq dev over equal's export on sbt $version: npm's install in sjs, the dev script with the arguments and TEQ, SIGINT ending it with 130"
  else
    fail "teq dev over equal's export on sbt $version ($code, $(cat "$dir/sjs/dev-started" 2>&1)): $(tail -15 "$work/dev-$version.out")"
  fi
done

# sbt's own tasks with teq as the compiler (TEQ_COMPILER=1) on each axis: zinc's incremental compile through teq and
# its analysis (TeqCompile, TeqAnalysis), lib's suites under sbt's runner (`testOnly`, then `test` twice), app's main
# run by sbt's `run`, sjs's suites under sbt-scalajs's test adapter over teq's test link (TeqScalaJSPlugin), and the
# version switch and cross command (`++3.8.4!`, `+lib/testOnly`). sbt 1's `test` runs the suites each time, five
# passing runs in all; sbt 2's is testQuick, which skips a suite its machine-wide disk cache saw pass, three at least.
for version in $axes; do
  dir=$(copy equal "$version" compiler)
  if (export TEQ_COMPILER=1 TEQ=$teq; sbt_in "$dir" "$work/compiler-$version.log" lib/testOnly lib/test lib/test 'app/run one two' sjs/testOnly '++3.8.4!' '+lib/testOnly'); then
    out=$(sed 's/\x1b\[[0-9;]*m//g' "$work/compiler-$version.log")
    passed=$(grep -c "Passed: Total 2, Failed 0, Errors 0, Passed 2" <<< "$out")
    if grep -q "^app one two$" <<< "$out" && { [[ "$version" == 2* ]] || [ "$passed" = 5 ]; } && [ "$passed" -ge 3 ] &&
      [ -n "$(find "$dir/lib/target" "$dir/target/out" -path '*classes/lib/Util.class' 2> /dev/null | head -1)" ]; then
      pass "sbt $version under teqCompiler: lib compiled by teq, its suites run by sbt ($passed passing runs), app run, sjs's suites through the test adapter, ++ and + switching"
    else
      fail "sbt $version under teqCompiler: $passed passing runs: $(grep -a "error\|Passed\|No tests\|^app" <<< "$out" | head -20)"
    fi
  else
    fail "sbt $version under teqCompiler: $(sed 's/\x1b\[[0-9;]*m//g' "$work/compiler-$version.log" | grep -a "error" | head -20)"
  fi
done

# The compiler's binary on each axis: the pinned release resolved from the mirror and verified; a SNAPSHOT from the
# build's Maven repository, which the export pins and the launcher fetches; a release the mirror does not serve
# refused by name.
for version in $axes; do
  dir=$(copy equal "$version" release)
  if (unset TEQ; export TEQ_CACHE_DIR=$work/cache-$version; sbt_in "$dir" "$work/release-$version.log" 'show app/teqResolvedBinary') &&
    binary=$(sed 's/\x1b\[[0-9;]*m//g' "$work/release-$version.log" | sed -n 's/^\[info\] \(\/.*teq-0\.1\.8-[a-z_0-9-]*\)$/\1/p' | tail -1) &&
    [ "$("$binary" --version)" = "teq 0.1.8 axes" ]; then
    pass "sbt $version: the pinned release resolved from the mirror and verified ($(basename "$binary"))"
  else
    fail "sbt $version: the pinned release's binary: $(tail -20 "$work/release-$version.log")"
  fi
  dir=$(copy equal "$version" snapshot)
  if (unset TEQ; sbt_in "$dir" "$work/snapshot-$version.log" "set ThisBuild / teqVersion := \"0.1.8-SNAPSHOT\"" "set ThisBuild / resolvers += \"axes-bin\" at \"$mavenbase\"" 'show app/teqResolvedBinary' teqExportAll) &&
    grep -q "^  $classifier: $mavenbase/build/teq/teq/0.1.8-SNAPSHOT/teq-0.1.8-SNAPSHOT-$classifier.exe " "$dir/teq.lock" &&
    [ "$(cd "$dir" && env -u TEQ TEQ_CACHE_DIR="$work/cache-launcher-$version" timeout 60 ./teq --version 2>&1)" = "teq 0.1.8 snapshot" ]; then
    pass "sbt $version: a SNAPSHOT of the compiler from the build's Maven repository, pinned by the export and run by the launcher"
  else
    fail "sbt $version: the SNAPSHOT's binary: $(grep "$classifier" "$dir/teq.lock" 2>&1) $(tail -20 "$work/snapshot-$version.log")"
  fi
  dir=$(copy equal "$version" absent)
  if sbt_in "$dir" "$work/absent-$version.log" 'set ThisBuild / teqVersion := "0.1.9"' teqExportAll &&
    grep -q "teq: no release v0.1.9 of teq is at $AXES_RELEASES/v0.1.9/" "$work/absent-$version.log" && ! grep -qE "^  (linux|osx|windows)-" "$dir/teq.lock"; then
    pass "sbt $version: a release the mirror does not serve named in a warning, the lock naming no binary"
  else
    fail "sbt $version: the absent release: $(head -12 "$dir/teq.lock") $(grep -a "error\|warn" "$work/absent-$version.log" | head -10)"
  fi
done

[ $status = 0 ] && echo "axes: passed" || echo "FAIL axes"
exit $status
