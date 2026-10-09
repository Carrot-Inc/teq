#!/bin/bash
# `teq stage` and the export's `stage` block against sbt-native-packager: native-packager's
# own `api/Docker/stage` (sbt compiling with scalac, TEQ_COMPILER unset) gives the tree the
# export's mappings name, but for the files teq does not write (native-packager's generated
# Dockerfile and the Windows script); its dependency jars are the bytes of the artifacts the
# export names, its own jar's manifest the bytes of the export's attributes, and its start
# script's classpath the order of the export's. Then `teq stage api` writes that tree with
# api's jar of teq's class files, and `docker build` from the example's Dockerfile gives an image
# whose entrypoint prints what scalac's build of api prints (bench/app/expected/api.txt), a note
# where Docker or its base image is absent. `teq run api server` prints the same. The corpus
# is generated into src/ and api's jar dependency packaged as check.sh does. Needs sbt, scala-cli
# and sbt-teq published locally (TEQ_PLUGIN_VERSION as check.sh takes it); TEQ names the binary
# (default: the repository's release build).
cd "$(dirname "$0")" || exit 1
repo=../../..
export TEQ=${TEQ:-$(cd $repo && pwd)/target/release/teq}
unset TEQ_COMPILER
status=0
pass() { echo "stage: $1"; }
fail() { echo "FAIL stage: $1"; status=1; }
image=teq-example-api:check-stage
trap 'timeout 30 "$TEQ" stop > /dev/null 2>&1; timeout 60 docker rmi -f $image > /dev/null 2>&1' EXIT

rm -rf src && timeout 120 python3 $repo/bench/app/gen.py src > /dev/null || { echo "FAIL generation"; exit 1; }
mkdir -p src/api/meridian/check api/lib
cat > src/api/meridian/check/Actions.scala <<'EOF'
package meridian.check

trait Action:
  def run(): Int

object Actions:
  inline def make(): Action = new Action:
    def run(): Int = 42
EOF
timeout 200 scala-cli --power package lib-src --library -o api/lib/util.jar -f -S 3.8.4 --server=false > target-util-jar.log 2>&1 || { echo "FAIL packaging util.jar (see target-util-jar.log)"; exit 1; }

stage=api/target/docker/stage
rm -rf $stage target-stage-sbt
if timeout 340 sbt --server --batch api/Docker/stage > target-stage-sbt.log 2>&1; then
  cp -R $stage target-stage-sbt
else
  echo "FAIL stage: sbt api/Docker/stage (see target-stage-sbt.log)"
  exit 1
fi
# The comparisons: the export's api stage against native-packager's tree, by path, the artifacts'
# bytes, the manifest and the script's classpath.
python3 - teq.lock target-stage-sbt <<'PY'
import hashlib, os, re, sys, zipfile
import importlib.util
spec = importlib.util.spec_from_file_location("c", "check-export.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
export, tree = c.load(sys.argv[1]), sys.argv[2]
api = export["projects"]["api"]
stage = api["stage"]
named = {f"{layer}/{m.get('to') or m['script']}": m for layer, ms in stage["layers"].items() for m in ms}
written = {os.path.relpath(os.path.join(d, f), tree) for d, _, fs in os.walk(tree) for f in fs}
script = next(m for m in named.values() if "script" in m)
skipped = {"Dockerfile", next(p for p in named if "script" in named[p]) + ".bat"}
failures = []
if set(named) != written - skipped:
    failures.append(f"the export names {sorted(set(named) - written)} beyond native-packager's tree, which has {sorted(written - skipped - set(named))} beside it")
runtime = {e for e in api["configurations"]["runtime"]["classpath"] if isinstance(e, str)}
for path, m in named.items():
    source = m.get("from", {})
    if isinstance(source, str) and (source not in runtime or hashlib.sha1(open(os.path.join(tree, path), "rb").read()).hexdigest() != c.fields(export, source)[1]):
        failures.append(f"{path} is not the bytes of {source}, a jar of api's runtime classpath")
    if "manifest" in m:
        text = zipfile.ZipFile(os.path.join(tree, path)).read("META-INF/MANIFEST.MF").decode()
        attributes = dict(line.split(": ", 1) for line in text.split("\r\n") if ": " in line)
        attributes.pop("Manifest-Version")
        if attributes != m["manifest"]:
            failures.append(f"{path}'s manifest {attributes} is not the export's {m['manifest']}")
bash = open(os.path.join(tree, next(p for p in named if "script" in named[p]))).read()
classpath = re.search(r'^declare -r app_classpath="(.*)"$', bash, re.M).group(1).replace("$lib_dir/", "lib/").split(":")
if classpath != script["classpath"]:
    failures.append(f"native-packager's script runs {classpath}, the export's {script['classpath']}")
main = re.search(r"^declare -a app_mainclass=\('(.*)'\)$", bash, re.M).group(1)
if main != stage["mainClass"]:
    failures.append(f"native-packager's script runs {main}, the export's {stage['mainClass']}")
print("\n".join(failures))
sys.exit(1 if failures else 0)
PY
if [ $? = 0 ]; then
  pass "the export's mappings are native-packager's tree, its artifacts' bytes, its manifest and its script's classpath and main class"
else
  fail "the export against native-packager's stage"
fi

same_dependencies() { for f in $(cd target-stage-sbt && find ./2 -type f); do cmp -s target-stage-sbt/$f $stage/$f || return 1; done; }
out=$(timeout 300 "$TEQ" stage api 2>&1)
if [ $? = 0 ] && [[ "$out" == *"api: staged "*" under api/target/docker/stage"* ]] &&
  [ "$(cd $stage && find . -type f | sort)" = "$(cd target-stage-sbt && find . -type f ! -name Dockerfile ! -name '*.bat' | sort)" ] && same_dependencies; then
  pass "teq stage api writes native-packager's tree, the same dependency jars ($(printf '%s' "$out" | grep -o 'in [0-9]* ms (built in [0-9]* ms)'))"
else
  fail "teq stage api: $out"
fi

expected=$repo/bench/app/expected/api.txt
out=$(timeout 300 "$TEQ" run api server 2> target-stage-run.err)
if [ $? = 0 ] && [ "$out" = "$(cat $expected)" ]; then pass "teq run api server prints scalac's output"; else fail "teq run api server (see target-stage-run.err)"; fi

if ! command -v docker > /dev/null || ! timeout 30 docker info > /dev/null 2>&1; then
  echo "stage: note: no Docker, the image is not built"
elif ! timeout 120 docker image inspect eclipse-temurin:21-jdk > /dev/null 2>&1 && ! timeout 300 docker pull -q eclipse-temurin:21-jdk > /dev/null 2>&1; then
  echo "stage: note: the base image eclipse-temurin:21-jdk is not to be had, the image is not built"
elif timeout 300 docker build -q -t $image . > target-stage-docker.log 2>&1 && timeout 120 docker run --rm $image > target-stage-image.out 2>&1 && cmp -s target-stage-image.out $expected; then
  pass "docker build from the example's Dockerfile over the stage: the image's entrypoint prints scalac's output"
else
  fail "the image (see target-stage-docker.log, target-stage-image.out)"
fi
exit $status
