#!/bin/bash
# The export's fixture (docs/TARGETS.md, "The export and the project verbs"): `sbt teqExportAll` over the
# example, which takes teq as its build tool by a bare `teqBuildTool` in build.sbt (every project's setting in
# sbt 2, which the export reads as the build's; the root files written anew), gives the same bytes
# twice, and the committed teq.lock once its header (`teq` and
# `binaries`) is the fixture's (the header names the release the example pins and its binaries, which
# the release's pin fills once it is published: the scenario of the fixture's release compares it); a dependency added to one project changes that project's three classpaths
# and the jar table's one record of it, and nothing else, or the classpaths alone when another
# project has the jar; one repository that two projects' resolvers name differently is one, its jar
# one record; a build whose one resolver is a mirror of Maven Central exports that repository
# alone (written under its target/teq/, a build without the driver), sbt's own scala-library its jar, and teq's reader compiles it with the mirror gone; every
# configuration's jars, at the paths the table gives, are sbt's externalDependencyClasspath; the
# compiler toggle (TEQ_COMPILER=1) changes nothing, since the export compiles nothing; the release
# the fixture pins gives its header, an empty table with the release absent until it is published; a
# release from a mirror of its GitHub release is pinned by its manifest's canonical URLs, SHA-1s and
# sizes, no binary downloaded; with no repository in reach and the mirror stopped the export names no
# binary and warns; a release before 0.1.7 is refused by name, the export and the resolution; a build's own
# dockerGroupLayers that reads paths alone gives the stage's layers; a `Compile / run / mainClass`
# beside `Compile / mainClass` is the run block's alone, the stage's and the manifest's staying
# native-packager's; a build leaving `Compile / mainClass` alone gets the stage block and the product
# jar's manifest without the class, which `teq stage` implies from the products, and one setting
# it to None a block that says so, the script's class implied and the manifest left alone, a session's
# `set` of the key counting as the declaration; what a committed lock cannot hold is refused under the driver, all of it in one
# message, and the file kept, a repository out of reach warned about, while without the driver the path outside the root, the artifact of
# no Maven repository and the empty binaries table are written as this machine's; what teq
# cannot reproduce is recorded, the same bytes from a copy of the build elsewhere, the file
# written, and teq refuses the project naming it while it runs the others, a verb over every
# project exiting 2; a CRLF checkout of the build's files (Git's) reads the committed lock as
# stale with a note on the line ends and the remedy, the export there warns of the CRLF, and the
# remedy, the export again included, makes the lock current; without the driver the same bytes go to the build root's target/teq/teq.lock
# whatever the root project's `target`, nothing to the root, and teq finds them there. Needs api/lib/util.jar, an unmanaged jar
# the export lists, as check.sh makes it, made here when missing, and sbt-teq, the release
# project/plugins.sbt pins or a branch's published locally under a version of its own that
# TEQ_PLUGIN_VERSION names, either defining teqBuildTool (below); TEQ names the binary (default: the
# repository's release build); EXPORT_MIRROR_PORT the mirror's port (default 5402); EXPORT_LINUX=1 adds
# the export on Linux in Docker. The committed file is put back at the end.
cd "$(dirname "$0")" || exit 1
teq=${TEQ:-$(cd ../../.. && pwd)/target/release/teq}
# The plugin the example builds with (project/plugins.sbt): a branch's, or the release it pins.
plugin=${TEQ_PLUGIN_VERSION:-$(sed -n 's/.*getOrElse("TEQ_PLUGIN_VERSION", "\([^"]*\)").*/\1/p' project/plugins.sbt)}
# The group a SNAPSHOT of the compiler published locally is resolved under (TeqPlugin.SnapshotGroup), the
# refusals' teqVersion, 0.0.1-refused-SNAPSHOT, among them.
group=build.teq
status=0
if [ ! -f api/lib/util.jar ]; then
  timeout 200 scala-cli --power package lib-src --library -o api/lib/util.jar -f -S 3.8.4 --server=false > target-util-jar.log 2>&1 || { echo "FAIL export: packaging util.jar (see target-util-jar.log)"; exit 1; }
fi
cp teq.lock target-export-fixture.lock
cp build.sbt target-export-build-sbt.txt
trap 'releases_stop; cp target-export-fixture.lock teq.lock; cp target-export-build-sbt.txt build.sbt; rm -f target-export-build-sbt.txt export-check.sbt target-export-records.txt target-export-releases.port; [ -f target-export-launchers/teq ] && cp -p target-export-launchers/teq target-export-launchers/teq.cmd .' EXIT

# export <copy> [sbt commands before teqExportAll...]
export_to() {
  local copy=$1
  shift
  timeout 340 sbt --server --batch "$@" teqExportAll > target-export.log 2>&1 && cp teq.lock "$copy"
}
compare() { python3 check-export.py "$@"; }
# releases_start <releases dir>: a mirror of GitHub's releases over the directory (bench/release-mirror.py, a port of
# its own), its base in releases_base; releases_stop ends it.
releases_pid=
releases_start() {
  rm -f target-export-releases.port
  timeout 680 python3 -B ../../../bench/release-mirror.py target-export-releases.port "$1" "$1/maven" > /dev/null 2>&1 &
  releases_pid=$!
  for _ in $(seq 100); do [ -s target-export-releases.port ] && break; sleep 0.1; done
  releases_base=http://127.0.0.1:$(cat target-export-releases.port 2> /dev/null)/releases/download
  [ -s target-export-releases.port ]
}
releases_stop() { [ -z "$releases_pid" ] || { kill "$releases_pid" 2> /dev/null; wait "$releases_pid" 2> /dev/null; }; releases_pid=; }

# The example's build.sbt sets teqBuildTool bare, which sbt 2 makes every project's setting, the
# build's scope left at its default: the export takes teq as the build tool all the same, writing
# teq.lock and the launchers at the root (set aside first, so that what the checks below read is this
# export's) and nothing under target/teq/.
mkdir -p target-export-launchers && cp -p teq teq.cmd target-export-launchers/ && rm -f teq teq.cmd teq.lock target/teq/teq.lock
if export_to target-export-1.lock && export_to target-export-2.lock; then
  if [ -x teq ] && [ -f teq.cmd ] && [ ! -e target/teq/teq.lock ]; then
    echo "export: the bare teqBuildTool of build.sbt takes teq as the build tool: teq.lock and the launchers written at the root, nothing under target/teq/"
  else
    echo "FAIL export: the bare teqBuildTool: $(ls teq teq.cmd target/teq/teq.lock 2>&1)"
    status=1
  fi
  if cmp -s target-export-1.lock target-export-2.lock; then echo "export: two exports give the same bytes"; else echo "FAIL export: two exports differ"; status=1; fi
  if compare fixture target-export-1.lock target-export-fixture.lock; then echo "export: the committed fixture, but for its header"; else echo "FAIL export: the export is not the committed fixture"; status=1; fi
  # The launchers the export wrote beside the lock, the plugin's own (and the repository's copies at its
  # root, its scripts' launchers, the same), and the compile of api through them over that export (TEQ
  # naming the branch's binary, which no repository holds; the corpus's sources generated when missing
  # and the main source api's tests use besides, as check.sh and check-stage.sh make them).
  out=
  if { [ -d src ] || timeout 120 python3 ../../../bench/app/gen.py src > /dev/null; } && mkdir -p src/api/meridian/check &&
    printf 'package meridian.check\n\ntrait Action:\n  def run(): Int\n\nobject Actions:\n  inline def make(): Action = new Action:\n    def run(): Int = 42\n' > src/api/meridian/check/Actions.scala &&
    cmp -s teq ../../../tools/launcher/teq && cmp -s teq.cmd ../../../tools/launcher/teq.cmd && [ -x teq ] &&
    cmp -s ../../../teq ../../../tools/launcher/teq && cmp -s ../../../teq.cmd ../../../tools/launcher/teq.cmd && [ -x ../../../teq ] &&
    out=$(TEQ=$teq timeout 300 ./teq compile api 2>&1); then
    echo "export: the launchers written beside the lock are teq's, and ./teq compile api runs through them"
  else
    echo "FAIL export: the launchers or the compile through them: $out"
    status=1
  fi
  TEQ=$teq timeout 30 ./teq stop > /dev/null 2>&1
  # The classpaths against sbt's own: every configuration's externalDependencyClasspath, as sbt's
  # `export` prints it, is the export's jars and files in order, each key's file at the path its
  # record gives (the Maven layout of the key, but where the record names another).
  commands=()
  for c in $(compare configurations target-export-1.lock); do
    commands+=("export $c/externalDependencyClasspath")
  done
  if timeout 340 sbt --server --batch "${commands[@]}" > target-export-classpaths.log 2>&1 && out=$(compare classpaths target-export-1.lock target-export-classpaths.log); then
    echo "export: the classpaths are sbt's externalDependencyClasspath, the paths derived from the jar table ($out)"
  else
    echo "FAIL export: the classpaths against sbt's (see target-export-classpaths.log): $out"
    status=1
  fi
  if export_to target-export-dependency.lock 'set jvmapp / libraryDependencies += "com.lihaoyi" %% "sourcecode" % "0.4.4"' &&
    compare dependency target-export-1.lock target-export-dependency.lock jvmapp com.lihaoyi:sourcecode_3:0.4.4; then
    echo "export: a dependency added to jvmapp changes its three classpaths and the jar table's one record alone"
  else
    echo "FAIL export: the dependency added to jvmapp (see target-export.log)"
    status=1
  fi
  if export_to target-export-dependency.lock 'set jvmapp / libraryDependencies += "com.lihaoyi" %% "sourcecode" % "0.4.2"' &&
    compare dependency target-export-1.lock target-export-dependency.lock jvmapp com.lihaoyi:sourcecode_3:0.4.2; then
    echo "export: a dependency api already has, added to jvmapp, changes jvmapp's three classpaths alone"
  else
    echo "FAIL export: the dependency api has added to jvmapp (see target-export.log)"
    status=1
  fi
  # One repository under two names: api's resolvers name a loopback repository "first", jvmapp's
  # "second", and both resolve a jar of it that no other repository has (a version of its own, so
  # that the resolution runs): the export lists the repository once, by api's name, the first
  # project's, and the jar's one record names it.
  alias=$(mktemp -d)
  mkdir -p "$alias/org/example/probe/1.0-alias"
  printf '<project><modelVersion>4.0.0</modelVersion><groupId>org.example</groupId><artifactId>probe</artifactId><version>1.0-alias</version></project>\n' > "$alias/org/example/probe/1.0-alias/probe-1.0-alias.pom"
  python3 -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1], "w"); z.writestr("probe.txt", "probe\n"); z.close()' "$alias/org/example/probe/1.0-alias/probe-1.0-alias.jar"
  port=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
  (cd "$alias" && exec timeout 340 python3 -m http.server "$port" --bind 127.0.0.1 > /dev/null 2>&1) &
  server=$!
  for _ in $(seq 50); do curl -fs "http://127.0.0.1:$port/" > /dev/null && break; sleep 0.1; done
  export_to target-export-alias.lock "set api / resolvers += \"first\" at \"http://127.0.0.1:$port/\"" "set jvmapp / resolvers += \"second\" at \"http://127.0.0.1:$port/\"" \
    'set api / libraryDependencies += "org.example" % "probe" % "1.0-alias"' 'set jvmapp / libraryDependencies += "org.example" % "probe" % "1.0-alias"'
  written=$?
  kill "$server" 2> /dev/null
  rm -rf "$alias"
  out=
  if [ $written = 0 ] && out=$(compare alias target-export-alias.lock "http://127.0.0.1:$port/" first org.example:probe:1.0-alias api jvmapp); then
    echo "export: one repository two projects' resolvers name differently is listed once, its jar's one record naming it"
  else
    echo "FAIL export: one repository under two names (see target-export.log): $out"
    status=1
  fi
  # A mirror alone: a build of its own whose one resolver is a loopback mirror of Maven Central
  # (coursier's cache of it), named maven-central and then mirror, sbt's own scala-library among
  # its jars. The export lists that repository alone and both Scala libraries' records name it,
  # never Central; then teq's reader compiles the build from coursier's copies, the mirror gone,
  # fetching nothing. The port is fixed: sbt keeps a resolution in its action cache across a change
  # of resolvers, so that a port of the run's own would meet the files of the run before. The
  # mirror is this run's own or the scenario fails: a port another process holds fails the setup,
  # the server answers a marker of this run before the exports, and it is gone (its process ended,
  # the marker no longer answered) before the compile that stands for the mirror's absence.
  mirror_port=${EXPORT_MIRROR_PORT:-5402}
  central=$(python3 -c 'import os, sys; print(os.path.join(os.environ.get("COURSIER_CACHE") or os.path.expanduser("~/Library/Caches/Coursier/v1" if sys.platform == "darwin" else "~/.cache/coursier/v1"), "https/repo1.maven.org/maven2"))')
  mirror=$(mktemp -d)
  mkdir -p "$mirror/project" "$mirror/src/main/scala"
  echo 'sbt.version=2.0.8' > "$mirror/project/build.properties"
  # The example's sbt-teq, from the same place: project/plugins.sbt's line of it; and its compiler, a release the
  # plugin serves whatever compiler the plugin was released with.
  compiler=$(sed -n 's/^ThisBuild \/ teqVersion := "\(.*\)"$/\1/p' target-export-build-sbt.txt)
  grep '^addSbtPlugin("build.teq" % "sbt-teq" % ' project/plugins.sbt > "$mirror/project/plugins.sbt"
  echo 'object P { val x = 1 }' > "$mirror/src/main/scala/P.scala"
  marker=$(python3 -c 'import uuid; print(uuid.uuid4().hex)')
  server_log=$PWD/target-export-mirror-server.log
  timeout 340 python3 -c '
import functools, http.server, sys
directory, port, marker = sys.argv[1:]
class Mirror(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/teq-mirror-marker":
            return super().do_GET()
        self.send_response(200)
        self.send_header("Content-Length", str(len(marker)))
        self.end_headers()
        self.wfile.write(marker.encode())
    def log_message(self, *args):
        pass
try:
    server = http.server.ThreadingHTTPServer(("127.0.0.1", int(port)), functools.partial(Mirror, directory=directory))
except OSError as e:
    sys.exit(f"the mirror cannot listen on 127.0.0.1:{port}: {e}")
server.serve_forever()' "$central" "$mirror_port" "$marker" > "$server_log" 2>&1 &
  server=$!
  answers() { [ "$(curl -fs --max-time 2 "http://127.0.0.1:$mirror_port/teq-mirror-marker" 2> /dev/null)" = "$marker" ]; }
  mirrored=
  for _ in $(seq 50); do
    if ! kill -0 "$server" 2> /dev/null; then mirrored="its server did not start: $(cat "$server_log")"; break; fi
    answers && mirrored=ok && break
    sleep 0.1
  done
  [ -z "$mirrored" ] && mirrored="its server on port $mirror_port did not answer this run's marker"
  log=$PWD/target-export-mirror.log
  # The first without the driver, its lock under target/teq/; the second takes it in the build's
  # scope, `ThisBuild / teqBuildTool`, which the export reads as it reads a bare setting.
  for name in maven-central mirror; do
    [ "$mirrored" = ok ] || break
    build_tool_line= lock=target/teq/teq.lock
    [ $name = mirror ] && build_tool_line=$'ThisBuild / teqBuildTool := true\n' lock=teq.lock
    printf 'scalaVersion := "3.8.4"\nteqVersion := "%s"\nteqExportSnapshots := true\n%sexternalResolvers := Seq("%s" at "http://127.0.0.1:%s/")\n' "$compiler" "$build_tool_line" "$name" "$mirror_port" > "$mirror/build.sbt"
    out=
    if ! (cd "$mirror" && timeout 200 sbt --server --batch teqExportAll > "$log" 2>&1) ||
      ! out=$(compare mirror "$mirror/$lock" "http://127.0.0.1:$mirror_port/" "$name" org.scala-lang:scala-library:3.8.4 org.scala-lang:scala3-library_3:3.8.4); then
      mirrored="the mirror named $name (see target-export-mirror.log): $out"
      break
    fi
  done
  kill "$server" 2> /dev/null
  for _ in $(seq 50); do kill -0 "$server" 2> /dev/null || break; sleep 0.1; done
  if kill -0 "$server" 2> /dev/null; then
    pkill -9 -P "$server" 2> /dev/null
    kill -9 "$server" 2> /dev/null
    [ "$mirrored" = ok ] && mirrored="its server did not end on SIGTERM"
  fi
  wait "$server" 2> /dev/null
  if [ "$mirrored" = ok ] && answers; then mirrored="port $mirror_port still answers this run's marker after its server ended"; fi
  if [ "$mirrored" = ok ]; then
    project=$(compare projects "$mirror/target/teq/teq.lock")
    own_cache=$(mktemp -d)
    out=$( (cd "$mirror" && TEQ_CACHE_DIR=$own_cache timeout 150 "$teq" compile "$project" 2>&1) )
    code=$?
    (cd "$mirror" && TEQ_CACHE_DIR=$own_cache timeout 30 "$teq" stop > /dev/null 2>&1)
    if [ $code != 0 ] || [ -n "$(ls -A "$own_cache/artifacts" 2> /dev/null)" ]; then mirrored="teq's compile with the mirror gone ($code): $out"; fi
    rm -rf "$own_cache"
  fi
  rm -rf "$mirror"
  if [ "$mirrored" = ok ]; then
    echo "export: a mirror alone, by either name, is the export's one repository, sbt's own scala-library its jar, and teq compiles from coursier's copies with the mirror gone"
  else
    echo "FAIL export: a mirror alone: $mirrored"
    status=1
  fi
  if (export TEQ_COMPILER=1; export_to target-export-teq.lock) && cmp -s target-export-1.lock target-export-teq.lock; then
    echo "export: with teq as the compiler the same bytes"
  else
    echo "FAIL export: with TEQ_COMPILER=1 (see target-export.log, target-export-teq.lock)"
    status=1
  fi
  # The compiler the fixture pins (bench/release.sh --pin moves it with the plugin), exported from its GitHub
  # release: the fixture's own header. Until the release is published the fixture's table is empty (the lock the
  # release's pin fills), and the export's must be too, with the warning that the release is not there (its
  # manifest answering 404), which an export that could not read the release does not give.
  released=$(sed -n 's/^teq: //p' target-export-fixture.lock)
  if export_to target-export-release.lock "set ThisBuild / teqVersion := \"$released\"" &&
    compare header target-export-release.lock target-export-fixture.lock; then
    if ! grep -qx 'binaries: {}' target-export-fixture.lock; then
      echo "export: the release the fixture pins, $released, exported from its GitHub release: the fixture's header"
    elif grep -qF "[warn] teq: no release v$released of teq is at " target-export.log; then
      echo "export: the release the fixture pins, $released, not published yet: its table empty, as the fixture's, the release absent"
    else
      echo "FAIL export: the fixture's table is empty, and the export does not say that the release $released is absent (see target-export.log)"
      status=1
    fi
  else
    echo "FAIL export: the released compiler's header (see target-export.log, target-export-release.lock)"
    status=1
  fi
  # A release from its GitHub release, a mirror's here (release-fixture.py's release of every classifier): the
  # lock pins exactly its binaries as its manifest gives them, each asset's canonical URL with the manifest's SHA-1
  # and size (check-export.py binaries, as the release's pin compares them), and nothing is downloaded, the
  # plugin's cache left empty.
  releases=$(mktemp -d)
  served=0.1.7-check.1 inconsistent=0.1.7-check.2
  records=()
  if timeout 30 python3 release-fixture.py "$releases" $served one > target-export-records.txt && releases_start "$releases"; then
    while read -r c a sha1 size; do records+=("$c $releases_base/v$served/$a $sha1 $size"); done < target-export-records.txt
  fi
  own_cache=$(mktemp -d)
  out=
  if [ ${#records[@]} = 5 ] && (export TEQ_CACHE_DIR=$own_cache; export_to target-export-served.lock "set ThisBuild / teqReleases := \"$releases_base\"" "set ThisBuild / teqVersion := \"$served\"") &&
    out=$(compare binaries target-export-served.lock $served "${records[@]}") && [ -z "$(ls -A "$own_cache")" ]; then
    echo "export: a release from its GitHub release (a mirror's) pins its binaries as its manifest gives them, each canonical URL, SHA-1 and size, none downloaded"
  else
    echo "FAIL export: the release from the mirror (see target-export.log, target-export-served.lock): $out"
    status=1
  fi
  rm -rf "$own_cache"
  # No repository in reach (the proxies on a closed port; the jars and the plugin in coursier's and ivy's caches)
  # and the release's mirror stopped: the export writes the lock all the same, the fixture but for its header,
  # which names no binary, with a warning that the release could not be read.
  releases_stop
  offline=(-J-Dhttps.proxyHost=127.0.0.1 -J-Dhttps.proxyPort=9 -J-Dhttp.proxyHost=127.0.0.1 -J-Dhttp.proxyPort=9)
  if export_to target-export-offline.lock "${offline[@]}" "set ThisBuild / teqReleases := \"$releases_base\"" "set ThisBuild / teqVersion := \"$served\"" &&
    compare fixture target-export-offline.lock target-export-fixture.lock && grep -qx "binaries: {}" target-export-offline.lock &&
    grep -qF "[warn] teq: the release v$served of teq could not be read: " target-export.log && grep -qF "; teq.lock names no binary" target-export.log; then
    echo "export: with no repository in reach and the release's mirror stopped the lock is written, no binary named, the release warned about"
  else
    echo "FAIL export: the export with no repository in reach (see target-export.log, target-export-offline.lock)"
    status=1
  fi
  # A release before 0.1.7 is refused by name before any request (its base a closed port, which a request would
  # report instead): the export under the driver, the lock left as it was, and the resolution of its binary.
  cp teq.lock target-export-before-old.lock
  old=('set ThisBuild / teqReleases := "http://127.0.0.1:9/releases/download"' 'set ThisBuild / teqVersion := "0.1.6"')
  if ! export_to target-export-old.lock "${old[@]}" && grep -qF "teqVersion is 0.1.6, and releases before 0.1.7 are not served: pin 0.1.7 or later" target-export.log &&
    cmp -s teq.lock target-export-before-old.lock &&
    ! env -u TEQ timeout 340 sbt --server --batch "${old[@]}" 'show api/teqResolvedBinary' > target-export-old.log 2>&1 &&
    grep -qF "teq: teqVersion is 0.1.6, and releases before 0.1.7 are not served: pin 0.1.7 or later. Set teqBinary" target-export-old.log; then
    echo "export: a release before 0.1.7 refused by name before any request: the export under the driver, the lock left as it was, and the resolution of its binary"
  else
    echo "FAIL export: the release before 0.1.7 (see target-export.log, target-export-old.log)"
    status=1
  fi
  # The build's own layers, a function of the paths alone, which the export evaluates.
  echo 'LocalProject("api") / dockerGroupLayers := { case (_, path) if path.contains("/lib/api.") => 3; case _ => 5 }' > export-check.sbt
  if export_to target-export-layers.lock && python3 -c '
import importlib.util, sys
spec = importlib.util.spec_from_file_location("c", "check-export.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
layers = c.load(sys.argv[1])["projects"]["api"]["stage"]["layers"]
sys.exit(sorted(layers) != ["3", "5"] or [m["to"] for m in layers["3"]] != ["opt/docker/lib/api.api-0.1.0-SNAPSHOT.jar"])' target-export-layers.lock; then
    echo "export: the build's own dockerGroupLayers, a function of the paths, gives the stage's layers"
  else
    echo "FAIL export: the build's dockerGroupLayers (see target-export.log, target-export-layers.lock)"
    status=1
  fi
  rm export-check.sbt
  # Both scopes set: the run block takes `Compile / run / mainClass`, as sbt's `run` does, while
  # `mainClasses`, the stage's class and the product jar's Main-Class keep `Compile / mainClass`, as
  # sbt-native-packager stages it.
  echo 'LocalProject("api") / Compile / run / mainClass := Some("meridian.server.Dev")' > export-check.sbt
  if export_to target-export-runmain.lock && python3 -c '
import importlib.util, sys
spec = importlib.util.spec_from_file_location("c", "check-export.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
api = c.load(sys.argv[1])["projects"]["api"]
jars = [m for layer in api["stage"]["layers"].values() for m in layer if m.get("from") == {"configuration": "compile", "project": "api"}]
sys.exit(api["run"]["mainClass"] != "meridian.server.Dev" or api["configurations"]["compile"]["mainClasses"] != ["meridian.server.Main"] or api["stage"]["mainClass"] != "meridian.server.Main" or [m["manifest"]["Main-Class"] for m in jars] != ["meridian.server.Main"])' target-export-runmain.lock; then
    echo "export: Compile / run / mainClass beside Compile / mainClass: the run block takes the run scope; mainClasses, the stage and the manifest keep Compile / mainClass"
  else
    echo "FAIL export: the run-scoped main class (see target-export.log, target-export-runmain.lock)"
    status=1
  fi
  rm export-check.sbt
  # No `Compile / mainClass` declared (api's line taken out of build.sbt for the scenario, put back
  # after): the stage block, api's jar's manifest and the run block carry no class, `mainClasses`
  # keeps the alias's target; then `teq stage api` over that lock implies the single product
  # main for the script and the manifest, as sbt's `mainClass` picks the one discovered class after
  # a compile and native-packager and `packageBin` follow it.
  # stage_field <lock> <field> [<value>]: the stage block's field of api, "absent" for none.
  stage_field() { python3 -c '
import importlib.util, sys
spec = importlib.util.spec_from_file_location("c", "check-export.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
api = c.load(sys.argv[1])["projects"]["api"]
jars = [m for layer in api["stage"]["layers"].values() for m in layer if m.get("from") == {"configuration": "compile", "project": "api"}]
block = {"run": api["run"], "manifest": jars[0]["manifest"] if len(jars) == 1 else {}, "mainClasses": api["configurations"]["compile"]["mainClasses"]}.get(sys.argv[2], api["stage"])
print(block.get(sys.argv[3], "absent") if len(sys.argv) > 3 else block)' "$@"; }
  # staged_main <expected manifest Main-Class or absent>: `teq stage api` over teq.lock, its script running meridian.server.Main.
  staged_main() {
    out=$(TEQ=$teq timeout 300 ./teq stage api 2>&1) && [[ "$out" == *"api: staged "* ]] && grep -q "'meridian.server.Main'" api/target/docker/stage/4/opt/docker/bin/api &&
      [ "$(python3 -c 'import sys, zipfile; m = dict(l.split(": ", 1) for l in zipfile.ZipFile(sys.argv[1]).read("META-INF/MANIFEST.MF").decode().split("\r\n") if ": " in l); print(m.get("Main-Class", "absent"))' api/target/docker/stage/4/opt/docker/lib/api.api-0.1.0-SNAPSHOT.jar)" = "$1" ]
  }
  out=
  if grep -q 'Compile / mainClass := Some("meridian.server.Main"),' build.sbt && sed '/Compile \/ mainClass := Some("meridian.server.Main"),/d' target-export-build-sbt.txt > build.sbt &&
    export_to target-export-undeclared.lock && cp target-export-build-sbt.txt build.sbt &&
    [ "$(stage_field target-export-undeclared.lock stage mainClass)" = absent ] && [ "$(stage_field target-export-undeclared.lock stage mainClassNone)" = absent ] &&
    [ "$(stage_field target-export-undeclared.lock run mainClass)" = absent ] && [ "$(stage_field target-export-undeclared.lock manifest Main-Class)" = absent ] &&
    [ "$(stage_field target-export-undeclared.lock mainClasses)" = "['meridian.server.Main']" ]; then
    echo "export: without Compile / mainClass the stage block, the product jar's manifest and the run block carry no class, mainClasses the alias's target"
  else
    echo "FAIL export: the undeclared main class (see target-export.log, target-export-undeclared.lock)"
    status=1
  fi
  # A session's set of a scope no project's lookup reaches (sbt: "used by no settings or tasks") sets
  # nothing: the block stays without a field, and the export reads no default of sbt's, which would
  # compile.
  if sed '/Compile \/ mainClass := Some("meridian.server.Main"),/d' target-export-build-sbt.txt > build.sbt && export_to target-export-zero.lock 'set Zero / Compile / mainClass := None' &&
    cp target-export-build-sbt.txt build.sbt && [ "$(stage_field target-export-zero.lock stage mainClass)" = absent ] && [ "$(stage_field target-export-zero.lock stage mainClassNone)" = absent ] &&
    [ "$(stage_field target-export-zero.lock manifest Main-Class)" = absent ] && ! grep -q "compiling" target-export.log; then
    echo "export: a session's set of Zero / Compile / mainClass, which reaches no project, leaves the block without a field and compiles nothing"
  else
    echo "FAIL export: the unreached Zero scope (see target-export.log, target-export-zero.lock)"
    status=1
  fi
  cp target-export-build-sbt.txt build.sbt
  if cp target-export-undeclared.lock teq.lock && staged_main meridian.server.Main; then
    echo "export: teq stage api over that lock implies meridian.server.Main, api's single product main, for the script and the manifest"
  else
    echo "FAIL export: the stage over the undeclared lock: $out"
    status=1
  fi
  # `Compile / mainClass := None` in a build file: the block says so (`mainClassNone`) and carries no
  # `mainClass`, the manifest and the run block no class; `teq stage api` over that lock runs the
  # implied class from the script, as native-packager's does, and leaves the manifest without
  # Main-Class, as `packageBin` does.
  echo 'LocalProject("api") / Compile / mainClass := None' > export-check.sbt
  if export_to target-export-none.lock && [ "$(stage_field target-export-none.lock stage mainClassNone)" = True ] && [ "$(stage_field target-export-none.lock stage mainClass)" = absent ] &&
    [ "$(stage_field target-export-none.lock run mainClass)" = absent ] && [ "$(stage_field target-export-none.lock manifest Main-Class)" = absent ]; then
    echo "export: Compile / mainClass := None: the stage block says mainClassNone, the manifest and the run block carry no class"
  else
    echo "FAIL export: the explicit None (see target-export.log, target-export-none.lock)"
    status=1
  fi
  rm export-check.sbt
  if cp target-export-none.lock teq.lock && staged_main absent; then
    echo "export: teq stage api over that lock runs meridian.server.Main from the script and leaves the manifest without Main-Class"
  else
    echo "FAIL export: the stage over the explicit-None lock: $out"
    status=1
  fi
  TEQ=$teq timeout 30 ./teq stop > /dev/null 2>&1
  # A session's `set` of the key is the build's declaration too, its setting coming last: `None` gives
  # the block's mainClassNone, a class the block's mainClass, the manifest's Main-Class, the run
  # block's class and a place among mainClasses.
  if export_to target-export-set-none.lock 'set api / Compile / mainClass := None' && [ "$(stage_field target-export-set-none.lock stage mainClassNone)" = True ] &&
    [ "$(stage_field target-export-set-none.lock stage mainClass)" = absent ] && [ "$(stage_field target-export-set-none.lock manifest Main-Class)" = absent ] && [ "$(stage_field target-export-set-none.lock run mainClass)" = absent ] &&
    export_to target-export-set-some.lock 'set api / Compile / mainClass := Some("meridian.server.Dev")' && [ "$(stage_field target-export-set-some.lock stage mainClass)" = meridian.server.Dev ] &&
    [ "$(stage_field target-export-set-some.lock stage mainClassNone)" = absent ] && [ "$(stage_field target-export-set-some.lock manifest Main-Class)" = meridian.server.Dev ] && [ "$(stage_field target-export-set-some.lock run mainClass)" = meridian.server.Dev ] &&
    [ "$(stage_field target-export-set-some.lock mainClasses)" = "['meridian.server.Dev', 'meridian.server.Main']" ] &&
    export_to target-export-every-none.lock 'set every Compile / mainClass := None' && [ "$(stage_field target-export-every-none.lock stage mainClassNone)" = True ] &&
    [ "$(stage_field target-export-every-none.lock stage mainClass)" = absent ] && [ "$(stage_field target-export-every-none.lock manifest Main-Class)" = absent ] &&
    export_to target-export-every-some.lock 'set every Compile / mainClass := Some("meridian.server.Dev")' && [ "$(stage_field target-export-every-some.lock stage mainClass)" = meridian.server.Dev ] &&
    [ "$(stage_field target-export-every-some.lock manifest Main-Class)" = meridian.server.Dev ] && [ "$(stage_field target-export-every-some.lock run mainClass)" = meridian.server.Dev ]; then
    echo "export: a session's set of Compile / mainClass is the declaration, set every too: None gives mainClassNone, a class the block's, the manifest's and the run block's"
  else
    echo "FAIL export: the session's set of the main class (see target-export.log, target-export-set-*.lock, target-export-every-*.lock)"
    status=1
  fi
  # What still refuses, since no reader could use the lock, every reason in one failure and the file
  # kept: under the driver without the exemption a snapshot compiler (a version of the scenario's
  # own), with the plugin when it is a branch's snapshot, and a dynamic version; a binary its
  # repository serves without its .sha1 (a loopback repository serving the compiler's pom and
  # binaries alone), a repository out of reach before it warned about; a path outside the build's
  # root; an artifact from none of the build's Maven repositories (a file repository); and a key
  # naming two files (org.example:probe:1.0-twofold, other bytes in each of two loopback
  # repositories, api's and jvmapp's). The file and the checksumless repositories serve the next
  # scenario too.
  cp teq.lock target-export-before.lock
  checksumless=$(mktemp -d)
  for v in 0.0.1-refused-SNAPSHOT; do
    mkdir -p "$checksumless/${group//.//}/teq/$v"
    printf '<project><modelVersion>4.0.0</modelVersion><groupId>%s</groupId><artifactId>teq</artifactId><version>%s</version></project>\n' "$group" "$v" > "$checksumless/${group//.//}/teq/$v/teq-$v.pom"
    for c in osx-aarch_64 linux-x86_64 linux-aarch_64 windows-x86_64; do echo binary > "$checksumless/${group//.//}/teq/$v/teq-$v-$c.exe"; done
  done
  checksumless_port=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
  (cd "$checksumless" && exec timeout 680 python3 -m http.server "$checksumless_port" --bind 127.0.0.1 > /dev/null 2>&1) &
  checksumless_server=$!
  for _ in $(seq 50); do curl -fs "http://127.0.0.1:$checksumless_port/" > /dev/null && break; sleep 0.1; done
  checksumless_url=http://127.0.0.1:$checksumless_port/
  local_repository=$(mktemp -d)
  mkdir -p "$local_repository/org/example/probe-local/1.0-local"
  printf '<project><modelVersion>4.0.0</modelVersion><groupId>org.example</groupId><artifactId>probe-local</artifactId><version>1.0-local</version></project>\n' > "$local_repository/org/example/probe-local/1.0-local/probe-local-1.0-local.pom"
  python3 -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1], "w"); z.writestr("probe.txt", "local\n"); z.close()' "$local_repository/org/example/probe-local/1.0-local/probe-local-1.0-local.jar"
  local_settings=("set jvmcore / resolvers += \"local\" at \"file://$local_repository/\"" 'set jvmcore / libraryDependencies += "org.example" % "probe-local" % "1.0-local"')
  twofold=$(mktemp -d)
  for r in one two; do
    mkdir -p "$twofold/$r/org/example/probe/1.0-twofold"
    printf '<project><modelVersion>4.0.0</modelVersion><groupId>org.example</groupId><artifactId>probe</artifactId><version>1.0-twofold</version></project>\n' > "$twofold/$r/org/example/probe/1.0-twofold/probe-1.0-twofold.pom"
    python3 -c 'import sys, zipfile; z = zipfile.ZipFile(sys.argv[1], "w"); z.writestr("probe.txt", sys.argv[2] + "\n"); z.close()' "$twofold/$r/org/example/probe/1.0-twofold/probe-1.0-twofold.jar" "$r"
  done
  port=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
  (cd "$twofold" && exec timeout 340 python3 -m http.server "$port" --bind 127.0.0.1 > /dev/null 2>&1) &
  server=$!
  for _ in $(seq 50); do curl -fs "http://127.0.0.1:$port/" > /dev/null && break; sleep 0.1; done
  export_to target-export-refused.lock 'set ThisBuild / teqExportSnapshots := false' 'set ThisBuild / teqVersion := "0.0.1-refused-SNAPSHOT"' \
    'set jvmcore / libraryDependencies += "com.lihaoyi" %% "sourcecode" % "0.4.+"' \
    'set ThisBuild / resolvers += "unreachable" at "http://127.0.0.1:9/"' "set ThisBuild / resolvers += \"checksumless\" at \"$checksumless_url\"" \
    'set jvmcore / Compile / unmanagedSourceDirectories += file("/teq-outside-the-root")' "${local_settings[@]}" \
    "set api / resolvers += \"one\" at \"http://127.0.0.1:$port/one/\"" "set jvmapp / resolvers += \"two\" at \"http://127.0.0.1:$port/two/\"" \
    'set api / libraryDependencies += "org.example" % "probe" % "1.0-twofold"' 'set jvmapp / libraryDependencies += "org.example" % "probe" % "1.0-twofold"'
  written=$?
  kill "$server" 2> /dev/null
  rm -rf "$twofold"
  if [ $written = 0 ]; then
    echo "FAIL export: the refusals were written (see target-export.log)"
    status=1
  elif grep -qF "teqVersion is 0.0.1-refused-SNAPSHOT," target-export.log &&
    case $plugin in *-SNAPSHOT) grep -qF "the sbt-teq plugin's version is $plugin," target-export.log ;; *) ! grep -q "sbt-teq plugin's version" target-export.log ;; esac &&
    grep -q "jvmcore: com.lihaoyi:sourcecode:0.4.+ is a dynamic version" target-export.log &&
    grep -qF "[error]   teq's binary for osx-aarch_64 cannot be pinned from the repository checksumless ($checksumless_url): ${checksumless_url}${group//.//}/teq/0.0.1-refused-SNAPSHOT/teq-0.0.1-refused-SNAPSHOT-osx-aarch_64.exe is served without its checksum" target-export.log &&
    grep -qF "[warn] teq: whether the repository unreachable (http://127.0.0.1:9/) serves $group:teq:0.0.1-refused-SNAPSHOT cannot be told" target-export.log &&
    grep -q "jvmcore: the source directory /teq-outside-the-root is outside the build's root" target-export.log &&
    grep -q "the jar org.example:probe:1.0-twofold is two files" target-export.log &&
    grep -q "(org.example:probe-local:1.0-local) comes from file:" target-export.log &&
    cmp -s teq.lock target-export-before.lock; then
    echo "export: a snapshot compiler under the driver without the exemption (and the plugin, $plugin, when it is one), a dynamic version, a binary served without its checksum, a path outside the root, an artifact of no Maven repository of the build and a key naming two files refused together, a repository out of reach warned about, the file left as it was"
  else
    echo "FAIL export: the refusals (see target-export.log)"
    status=1
  fi
  # Without the driver three of those are this machine's, the lock written under target/teq/: the
  # path outside the root absolute, the artifact of the file repository a file entry by its path,
  # and the binaries table of a release whose SHA256SUMS disagrees with its manifest (on a mirror again)
  # empty, with a warning saying why; teq compiles jvmcore over it.
  zeros=$(printf '0%.0s' $(seq 64))
  timeout 30 python3 release-fixture.py "$releases" $inconsistent one > /dev/null &&
    sed -i.orig "s/^[0-9a-f]\{64\}  teq-$inconsistent-linux-x86_64\$/$zeros  teq-$inconsistent-linux-x86_64/" "$releases/v$inconsistent/SHA256SUMS" &&
    releases_start "$releases"
  out=
  if ! timeout 340 sbt --server --batch 'set every teqBuildTool := false' "set ThisBuild / teqReleases := \"$releases_base\"" "set ThisBuild / teqVersion := \"$inconsistent\"" \
    'set jvmcore / Compile / unmanagedSourceDirectories += file("/teq-outside-the-root")' "${local_settings[@]}" teqExportAll > target-export.log 2>&1; then
    out="the export failed (see target-export.log)"
  elif ! out=$(python3 -c '
import importlib.util, sys
spec = importlib.util.spec_from_file_location("c", "check-export.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
e = c.load("target/teq/teq.lock")
compile = e["projects"]["jvmcore"]["configurations"]["compile"]
jar = sys.argv[1] + "/org/example/probe-local/1.0-local/probe-local-1.0-local.jar"
for what, ok in [("the outside source directory", "/teq-outside-the-root" in compile["sources"]), ("the file entry", {"file": jar} in compile["classpath"]), ("no binary", e["binaries"] == {})]:
    if not ok: print("not so:", what)' "$local_repository") || [ -n "$out" ]; then
    out="target/teq/teq.lock: $out"
  elif ! grep -qF "[warn] teq: the release v$inconsistent of teq could not be read: the release v$inconsistent at $releases_base/v$inconsistent/: SHA256SUMS gives teq-$inconsistent-linux-x86_64 $zeros, the manifest " target-export.log ||
    ! grep -qF "; teq.lock names no binary" target-export.log; then
    out="no warning of the binary table (see target-export.log)"
  elif ! out=$(timeout 200 "$teq" --export target/teq/teq.lock compile jvmcore 2>&1); then
    out="teq compile jvmcore: $out"
  else
    out=ok
  fi
  timeout 30 "$teq" --export target/teq/teq.lock stop > /dev/null 2>&1
  kill "$checksumless_server" 2> /dev/null
  releases_stop
  rm -rf "$local_repository" "$checksumless" "$releases" target/teq/teq.lock
  if [ "$out" = ok ]; then
    echo "export: without the driver a path outside the root, an artifact of no Maven repository of the build and a binary table that cannot be filled are this machine's, written, and teq compiles over them"
  else
    echo "FAIL export: without the driver, this machine's lock: $out"
    status=1
  fi
  # What teq cannot reproduce is recorded and the lock written: api's source generator of the
  # build's own (an unnamed Def.task), its Test generator, its resource generator, its Tests.Filter
  # and Tests.Setup, its BuildInfo with an action other than gitSha and a Long, and a Docker stage
  # teq would not reproduce (a build file: a layer function reading a task, a start script's
  # classpath and template of the build's own, a manifest attribute of its own, which `set` cannot
  # stand for, with mirrored's resource generators the build sets), a managed directory of api's
  # tests outside the root left out with a warning. Then teq refuses api's
  # compile naming the generator and compiles jvmcore, and its test over every project runs
  # jvmapp's suites and names api left out.
  printf '%s\n' 'LocalProject("mirrored") / Compile / resourceGenerators := Seq(Def.task(Seq.empty[File]).taskValue)' \
    'LocalProject("api") / dockerGroupLayers := (LocalProject("api") / dockerGroupLayers).value' 'LocalProject("api") / scriptClasspath := Seq("*")' \
    'LocalProject("api") / bashScriptTemplateLocation := file("launcher")' \
    'LocalProject("api") / Compile / packageBin / packageOptions += Package.ManifestAttributes("Sealed" -> "true")' > export-check.sbt
  # A Universal source directory outside the root, holding a file native-packager would stage.
  external_universal=$(mktemp -d)
  echo staged > "$external_universal/staged.txt"
  recorded=('set api / Compile / sourceGenerators += Def.task(Seq.empty[File]).taskValue'
    'set api / Test / sourceGenerators += Def.task(Seq.empty[File]).taskValue'
    'set api / Compile / resourceGenerators += Def.task(Seq.empty[File]).taskValue'
    'set api / Test / testOptions += Tests.Filter(_ => true)' 'set api / Test / testOptions += Tests.Setup(() => ())'
    'set api / buildInfoKeys += BuildInfoKey.map(BuildInfoKey.action("builtAt")(System.currentTimeMillis()))(identity)'
    'set api / buildInfoKeys += BuildInfoKey("count" -> 1L)' 'set api / buildInfoKeys += BuildInfoKey("base" -> (api / baseDirectory).value)'
    'set api / buildInfoKeys += BuildInfoKey("home" -> file(sys.props("user.home")))'
    'set api / Test / managedSourceDirectories += file("/teq-outside-managed")'
    "set api / Universal / sourceDirectory := file(\"$external_universal\")")
  export_to target-export-recorded.lock "${recorded[@]}"
  written=$?
  # The same export of a copy of the build in another directory, at another depth: the same bytes,
  # no reason naming a file of the machine (`base`, a directory of the build, recorded relative to
  # the root; `home` and the Universal sources, outside it, unnamed).
  relocated=$(mktemp -d)/example
  mkdir -p "$relocated" && tar cf - --exclude=./target --exclude="./*/target" --exclude=./project/project --exclude=./node_modules --exclude=./src --exclude="./dist*" --exclude="./target-*" . | tar xf - -C "$relocated"
  (cd "$relocated" && timeout 340 sbt --server --batch "${recorded[@]}" teqExportAll > target-export-relocated.log 2>&1)
  relocated_written=$?
  rm export-check.sbt
  out=
  if [ $written != 0 ]; then
    out="the export failed (see target-export.log)"
  elif ! out=$(compare recorded target-export-recorded.lock target-export-1.lock); then
    :
  elif ! grep -q "teq: api: sbt's managed source directory /teq-outside-managed of test is outside the build's root" target-export.log; then
    out="no warning of the managed directory outside the root (see target-export.log)"
  elif ! grep -q "the BuildInfo key base is the file api, which is no class directory of the build" target-export-recorded.lock ||
    ! grep -q "the BuildInfo key home is a file outside the build's root, which is no class directory of the build" target-export-recorded.lock ||
    ! grep -q "Universal / sourceDirectory, outside the build's root, holds files native-packager stages" target-export-recorded.lock; then
    out="the recorded reasons do not name api's directory relative to the root, and the home directory and the Universal sources as outside it"
  elif [ $relocated_written != 0 ] || ! cmp -s "$relocated/teq.lock" target-export-recorded.lock; then
    out="the export of the copy in $relocated ($relocated_written) is not the same bytes: $(diff "$relocated/teq.lock" target-export-recorded.lock 2>&1 | head -5)"
  else
    out=$(TEQ=$teq timeout 120 ./teq compile api 2>&1)
    code=$?
    if [ $code != 2 ] || [[ "$out" != *"api/compile has the source generator an unnamed task, which sbt runs and teq cannot"* ]] ||
      [[ "$out" != *"api/compile has the source generator buildInfo, which sbt runs and teq cannot (the BuildInfo key builtAt runs"* ]]; then
      out="teq compile api ($code): $out"
    elif ! out=$(TEQ=$teq timeout 200 ./teq compile jvmcore 2>&1); then
      out="teq compile jvmcore: $out"
    else
      out=$(TEQ=$teq timeout 300 ./teq test 2>&1)
      code=$?
      stage_out=$(TEQ=$teq timeout 120 ./teq stage 2>&1)
      stage_code=$?
      if [ $code != 2 ] || [[ "$out" != *"teq: test left out api, which needs what sbt alone runs:"* ]] ||
        [[ "$out" != *"api: Test / testOptions holds a Tests.Setup or Tests.Cleanup"* ]] || [[ "$out" != *"jvmapp/test"* ]] || [[ "$out" == *"api/test:"* ]]; then
        out="teq test ($code): $out"
      elif [ $stage_code != 2 ] || [[ "$stage_out" != *"teq: stage left out api, which needs what sbt alone runs:"* ]] || [[ "$stage_out" != *"api: the build sets scriptClasspath"* ]]; then
        out="teq stage ($stage_code): $stage_out"
      else
        out=ok
      fi
    fi
  fi
  TEQ=$teq timeout 30 ./teq stop > /dev/null 2>&1
  cp target-export-fixture.lock teq.lock
  rm -rf "$(dirname "$relocated")" "$external_universal"
  if [ "$out" = ok ]; then
    echo "export: a generator of the build's own, a Test and a resource generator, test options, BuildInfo keys and a Docker stage teq cannot reproduce recorded and the lock written, the same bytes from a copy elsewhere; teq refuses api's compile naming the generator, compiles jvmcore, and its test and stage over every project run the others, name api left out and exit 2"
  else
    echo "FAIL export: the recorded scenario: $out"
    status=1
  fi
  # A CRLF checkout, Git's (core.autocrlf=true in the clone's configuration, as Git for Windows has
  # it in its own) of a repository of the build that commits the driver's export and keeps the
  # launchers' and the lock's line ends alone: the build's four files come out CRLF. teq there reads
  # the lock as stale, --strict refusing it, with a note naming the files that differ by line ends
  # alone and the remedy in the place of the command; an edit beside the line ends gets the command
  # and is not noted. The export there warns of the CRLF with the same remedy (a Git work tree), its
  # lock the LF export's but for the inputs, which teq where the files are LF reads as stale with the
  # note. The remedy in the clone (the build's lines in .gitattributes, the files deleted and checked
  # out again) makes them LF, the lock it exported from CRLF stale with the note until it is exported
  # again, which gives the committed lock's bytes, --strict going on.
  crlf_root=$(mktemp -d)
  mkdir "$crlf_root/source"
  tar cf - --exclude=./target --exclude="./*/target" --exclude=./project/project --exclude=./node_modules --exclude=./src --exclude="./dist*" --exclude="./target-*" . | tar xf - -C "$crlf_root/source"
  cp target-export-1.lock "$crlf_root/source/teq.lock"
  grep -v -e '^\*\.sbt ' -e '^project/' .gitattributes > "$crlf_root/source/.gitattributes"
  definition=(build.sbt project/Mirror.scala project/build.properties project/plugins.sbt)
  files_note="build.sbt, project/Mirror.scala, project/build.properties, project/plugins.sbt differ from teq.lock's record by line ends alone, LF against CRLF: .gitattributes keeps the build's files LF in every checkout with *.sbt text eol=lf, project/**/*.scala text eol=lf and project/build.properties text eol=lf: add them, delete the files and check them out again, then export again"
  strict() { (cd "$1" && TEQ=$teq timeout 60 ./teq --strict stop 2>&1); }
  out=
  if ! (cd "$crlf_root/source" && git init -q && git -c core.autocrlf=false add -A && git -c core.autocrlf=false -c user.name=check -c user.email=check@teq.build commit -qm build) > "$crlf_root/git.log" 2>&1 ||
    ! git clone -q --config core.autocrlf=true "$crlf_root/source" "$crlf_root/crlf" >> "$crlf_root/git.log" 2>&1; then
    out="the repository or its CRLF clone could not be made: $(cat "$crlf_root/git.log")"
  else
    crlf="$crlf_root/crlf"
    mkdir -p "$crlf/api/lib" && cp api/lib/util.jar "$crlf/api/lib/"
    lines=$(cd "$crlf" && git ls-files --eol "${definition[@]}" teq.lock)
    strict_out=$(strict "$crlf")
    strict_code=$?
    printf '// edited\r\n' >> "$crlf/build.sbt"
    edited_out=$(strict "$crlf")
    edited_code=$?
    git -C "$crlf" checkout -q -- build.sbt
    (cd "$crlf" && timeout 340 sbt --server --batch teqExportAll > "$crlf_root/export.log" 2>&1)
    exported=$?
    cp "$crlf/teq.lock" "$crlf_root/crlf.lock"
    cp "$crlf_root/crlf.lock" "$crlf_root/source/teq.lock"
    lf_out=$(strict "$crlf_root/source")
    lf_code=$?
    cp .gitattributes "$crlf/.gitattributes"
    (cd "$crlf" && rm "${definition[@]}" && git checkout -q -- "${definition[@]}")
    lines_after=$(cd "$crlf" && git ls-files --eol "${definition[@]}")
    reverse_out=$(strict "$crlf")
    reverse_code=$?
    (cd "$crlf" && timeout 340 sbt --server --batch teqExportAll > "$crlf_root/export-again.log" 2>&1)
    again=$?
    remedied_out=$(strict "$crlf")
    remedied_code=$?
    without_inputs() { sed '/^inputs:/,/^  sha256:/d' "$1"; }
    if [ "$(grep -c 'w/crlf' <<< "$lines")" != 4 ] || ! grep -q 'w/lf.*teq.lock' <<< "$lines"; then
      out="the clone's build files are not CRLF and its lock LF: $lines"
    elif [ $strict_code != 2 ] || [[ "$strict_out" != *"refusing a stale export: teq.lock was written before build.sbt changed, project/Mirror.scala changed, project/build.properties changed, project/plugins.sbt changed"* ]] ||
      [[ "$strict_out" == *"sbt teqExportAll"* ]] || [[ "$strict_out" != *"teq: note: $files_note"* ]]; then
      out="teq --strict in the CRLF checkout ($strict_code): $strict_out"
    elif [ $edited_code != 2 ] || [[ "$edited_out" != *"build.sbt changed"*"; run sbt teqExportAll and commit the file"* ]] || [[ "$edited_out" != *"teq: note: project/Mirror.scala, project/build.properties, project/plugins.sbt differ from"* ]]; then
      out="teq --strict after an edit beside the line ends ($edited_code): $edited_out"
    elif [ $exported != 0 ]; then
      out="the export in the CRLF checkout failed (see $crlf_root/export.log)"
    elif ! grep -qF "teq: build.sbt, project/Mirror.scala, project/build.properties, project/plugins.sbt have CRLF line ends, and teq.lock records their bytes, so that a checkout with LF finds it stale: .gitattributes keeps the build's files LF in every checkout with *.sbt text eol=lf" "$crlf_root/export.log"; then
      out="the export in the CRLF checkout does not warn of the CRLF (see $crlf_root/export.log)"
    elif cmp -s "$crlf_root/crlf.lock" target-export-1.lock || [ "$(without_inputs "$crlf_root/crlf.lock")" != "$(without_inputs target-export-1.lock)" ]; then
      out="the CRLF checkout's lock is not the LF export's but for the inputs: $(diff "$crlf_root/crlf.lock" target-export-1.lock | head -12)"
    elif [ $lf_code != 2 ] || [[ "$lf_out" == *"sbt teqExportAll"* ]] || [[ "$lf_out" != *"teq: note: $files_note"* ]]; then
      out="teq --strict where the files are LF, over the CRLF checkout's lock ($lf_code): $lf_out"
    elif [ "$(grep -c 'w/lf' <<< "$lines_after")" != 4 ]; then
      out="the remedy did not make the build files LF: $lines_after"
    elif [ $reverse_code != 2 ] || [[ "$reverse_out" == *"sbt teqExportAll"* ]] || [[ "$reverse_out" != *"teq: note: $files_note"* ]]; then
      out="teq --strict after the remedy, over the lock exported from CRLF ($reverse_code): $reverse_out"
    elif [ $again != 0 ] || grep -q 'CRLF line ends' "$crlf_root/export-again.log" || ! cmp -s "$crlf/teq.lock" target-export-1.lock; then
      out="the export again after the remedy ($again) is not the committed lock's bytes, or warns of CRLF (see $crlf_root/export-again.log)"
    elif [ $remedied_code != 0 ] || [[ "$remedied_out" == *"teq.lock was written before"* ]]; then
      out="teq --strict after the remedy and the export again ($remedied_code): $remedied_out"
    else
      out=ok
    fi
  fi
  rm -rf "$crlf_root"
  if [ "$out" = ok ]; then
    echo "export: a CRLF checkout of the build's files, Git's: teq reads the committed lock as stale with a note naming each file and the remedy in the place of the command, --strict refusing, an edit beside the line ends getting the command and no note; the export there warns of the CRLF, its lock the LF export's but for the inputs, stale where the files are LF with the note; the remedy makes the files LF, that lock stale with the note until the export again, which gives the committed bytes"
  else
    echo "FAIL export: the CRLF checkout: $out"
    status=1
  fi
  # Without the driver the lock is target/teq/teq.lock, the same bytes, and nothing is written at
  # the root: the launchers set aside are not written again, and the teq.lock there, which every
  # reader takes first, is left as it was with a warning; with it gone, teq finds the other
  # from the root, and its test runs.
  mkdir -p target-export-launchers && cp -p teq teq.cmd target-export-launchers/ && rm -f teq teq.cmd target/teq/teq.lock
  out=
  # The root project's own `target` moved besides (sbt 2's is target/out/jvm/scala-3.8.4/root): the
  # lock is the build root's target/teq/teq.lock all the same.
  if ! timeout 340 sbt --server --batch 'set every teqBuildTool := false' 'set LocalRootProject / target := (LocalRootProject / baseDirectory).value / "target" / "root-elsewhere"' teqExportAll > target-export.log 2>&1; then
    out="the export failed (see target-export.log)"
  elif [ -e teq ] || [ -e teq.cmd ] || ! cmp -s teq.lock target-export-fixture.lock; then
    out="it wrote at the root: $(ls teq teq.cmd 2> /dev/null) $(cmp teq.lock target-export-fixture.lock 2>&1)"
  elif ! cmp -s target/teq/teq.lock target-export-1.lock; then
    out="target/teq/teq.lock is not the driver's export's bytes"
  elif ! grep -q "teq: wrote $PWD/target/teq/teq.lock, which teq" target-export.log || ! grep -q "teq: $PWD/teq.lock is not this export's" target-export.log; then
    out="its log does not say where it wrote, or warn of the root's teq.lock (see target-export.log)"
  else
    rm teq.lock
    out=$(timeout 300 "$teq" test 2>&1)
    code=$?
    if [ $code != 0 ] || [[ "$out" != *"jvmapp/test"* ]] || [[ "$out" != *"api/test"* ]]; then out="teq test ($code): $out"; else out=ok; fi
    timeout 30 "$teq" stop > /dev/null 2>&1
  fi
  cp -p target-export-launchers/teq target-export-launchers/teq.cmd . && cp target-export-fixture.lock teq.lock && rm -f target/teq/teq.lock
  if [ "$out" = ok ]; then
    echo "export: without the driver target/teq/teq.lock, the same bytes, nothing written at the root (its teq.lock left with a warning), and teq test from the root finds it"
  else
    echo "FAIL export: without the driver: $out"
    status=1
  fi
  # EXPORT_LINUX=1: the same export on Linux, in a container of its own caches (the volume
  # teq-export-linux), the plugin a branch's snapshot read from this machine's ivy repository, a
  # release from the repository: the same bytes.
  if [ -n "$EXPORT_LINUX" ]; then
    local_plugin=()
    case $plugin in *-SNAPSHOT) local_plugin=(-v "$HOME/.ivy2/local/$group/sbt-teq_sbt2_3/$plugin:/root/.ivy2/local/$group/sbt-teq_sbt2_3/$plugin:ro") ;; esac
    rm -rf target-export-linux && mkdir target-export-linux
    if timeout 340 docker run --rm -v teq-export-linux:/root "${local_plugin[@]}" \
      -v "$PWD:/src:ro" -v "$PWD/target-export-linux:/out" -e TEQ_PLUGIN_VERSION="$plugin" eclipse-temurin:21-jdk bash -c '
        set -e
        [ -x /root/sbt/bin/sbt ] || curl -fsSL https://github.com/sbt/sbt/releases/download/v2.0.8/sbt-2.0.8.tgz | tar xz -C /root
        mkdir /work && cd /src && tar cf - --exclude=./target --exclude="./*/target" --exclude=./node_modules --exclude=./src --exclude="./dist*" --exclude="./target-*" . | tar xf - -C /work
        cd /work && /root/sbt/bin/sbt --server --batch teqExportAll > /out/sbt.log 2>&1 && cp teq.lock /out/' &&
      cmp -s target-export-linux/teq.lock target-export-1.lock; then
      echo "export: on Linux ($(docker run --rm eclipse-temurin:21-jdk uname -m)) with caches of its own the same bytes"
    else
      echo "FAIL export: on Linux (see target-export-linux/)"
      status=1
    fi
  fi
else
  echo "FAIL export: sbt teqExportAll (see target-export.log)"
  status=1
fi
exit $status
