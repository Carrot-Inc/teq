#!/bin/bash
# Scala CLI scripts under `teq interp`: each run of tests/scripts/runs.sh (`run <name> <script> <argument>...`)
# starts `teq interp tests/scripts/<script> -- <argument>...` and compares its stdout with <name>.expected, its
# status with <name>.status (0 without one) and, where it fails, the first line of its stderr with
# <name>.stderr: what Scala CLI 1.17.1 gives for `scala-cli run <script> -- <argument>...` under scalac 3.8.4
# (`--record` runs Scala CLI and writes them again).
#
# tests/interp-script-passing.txt lists the runs that pass; a listed run that fails is a regression and fails the
# script, an unlisted one that passes is reported so that it can be added (`--update` rewrites the list).
cd "$(dirname "$0")/.."
export LC_COLLATE=C
TEQ=${TEQ:-./target/release/teq}
LIST=tests/interp-script-passing.txt
mode=check
case "$1" in --record) mode=record ;; --update) mode=update ;; esac
out=out/interp-script
rm -rf "$out"
mkdir -p "$out"

run() {
  local name=$1 script=$2
  shift 2
  local dir=tests/scripts
  if [ $mode = record ]; then
    timeout 300 scala-cli run -S 3.8.4 --jvm system "$dir/$script" --server=false -q -- "$@" > "$dir/$name.expected" 2> "$out/$name.ref.err"
    local s=$?
    rm -f "$dir/$name.status" "$dir/$name.stderr"
    if [ $s != 0 ]; then
      echo $s > "$dir/$name.status"
      grep -v '^WARNING\|^\[' "$out/$name.ref.err" | grep -m1 . > "$dir/$name.stderr"
    fi
    echo "recorded $name ($s)"
    return
  fi
  timeout 60 "$TEQ" interp "$dir/$script" -- "$@" > "$out/$name.actual" 2> "$out/$name.err"
  judge "$name" $? "$dir" ""
}
# `run_dir <name> <directory> <main>`: the directory as the input, its scripts in the packages of their
# subdirectories, the object `<main>` the one that runs (`--main`, Scala CLI's `--main-class`).
run_dir() {
  local name=$1 input=$2 main=$3
  local dir=tests/scripts
  if [ $mode = record ]; then
    timeout 300 scala-cli run -S 3.8.4 --jvm system "$dir/$input" --main-class "$main" --server=false -q > "$dir/$name.expected" 2> "$out/$name.ref.err"
    local s=$?
    rm -f "$dir/$name.status" "$dir/$name.stderr"
    if [ $s != 0 ]; then
      echo $s > "$dir/$name.status"
      grep -v '^WARNING\|^\[' "$out/$name.ref.err" | grep -m1 . > "$dir/$name.stderr"
    fi
    echo "recorded $name ($s)"
    return
  fi
  timeout 60 "$TEQ" interp "$dir/$input" --main "$main" > "$out/$name.actual" 2> "$out/$name.err"
  judge "$name" $? "$dir" ""
}
# A run's stdout against <dir>/<name>.expected (or the file given), its status against <dir>/<name>.status (0
# without one), the first line of its stderr against <dir>/<name>.stderr where there is one.
judge() {
  local name=$1 s=$2 dir=$3 label=$4 expected=${5:-$3/$1.expected} want=0 why=""
  [ -f "$dir/$name.status" ] && want=$(cat "$dir/$name.status")
  if ! diff -q "$expected" "$out/$name.actual" > /dev/null; then
    why="output"
  elif [ "$s" != "$want" ]; then
    why="status $s, not $want"
  elif [ -f "$dir/$name.stderr" ] && [ "$(grep -m1 . "$out/$name.err")" != "$(cat "$dir/$name.stderr")" ]; then
    why="stderr"
  fi
  if [ -z "$why" ]; then
    echo "$label$name" >> "$out/passed"
  else
    echo "$label$name: $why ($(grep -m1 . "$out/$name.err" | cut -c1-160))" >> "$out/failed"
  fi
}
# `refusal_cp <name> <file> <jar>...`: tests/scripts/<file> under --classpath of the coursier cache's jars named by
# their paths in it, against <name>.status and <name>.stderr alone, the cache written `<m2>`: teq's own refusal,
# which no Scala CLI run records.
refusal_cp() {
  local name=$1 file=$2
  shift 2
  [ $mode = record ] && return
  local m2=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}/https/repo1.maven.org/maven2 cp="" jar
  for jar in "$@"; do cp="$cp:$m2/$jar"; done
  timeout 120 "$TEQ" interp "tests/scripts/$file" --classpath "${cp#:}" > "$out/$name.actual" 2> "$out/$name.raw.err"
  local s=$?
  sed -e "s|$m2|<m2>|g" "$out/$name.raw.err" > "$out/$name.err"
  : > "$out/$name.empty"
  judge "$name" $s tests/scripts "refusal " "$out/$name.empty"
}
. tests/scripts/runs.sh

# The fixture: a small build's Scala CLI script whose `//> using dep` lines resolve through the build's committed
# lock (tests/interp-script/fixture/teq.lock), its jars fetched as the build's verbs fetch them: a jar that cannot
# be had fails the run, as any other failure does. Its runs share one home directory, made afresh.
# The coursier cache's Maven Central, whose jars a run names with --classpath.
M2C=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}/https/repo1.maven.org/maven2
fixture_dir=tests/interp-script/fixture
expected_dir=tests/interp-script/expected
home=$(mktemp -d)
# The files an `excluded` run writes into the fixture for itself, removed however the script ends.
placed=()
trap 'rm -rf "$home" "${placed[@]}"' EXIT
rm -rf "$home"
fixture() {
  local name=$1 script=$2
  shift 2
  local args=()
  for a in "$@"; do args+=("${a//HOME/$home}"); done
  if [ $mode = record ]; then
    # Scala CLI's own warnings on the build's directives left out, the program's stderr alone recorded.
    timeout 300 scala-cli run -S 3.8.4 --jvm system "$fixture_dir/scripts/$script" --server=false -q --suppress-directives-in-multiple-files-warning --suppress-outdated-dependency-warning -- "${args[@]}" > "$expected_dir/$name.expected" 2> "$out/$name.ref.err"
    local s=$?
    rm -f "$expected_dir/$name.status" "$expected_dir/$name.stderr"
    if [ $s != 0 ]; then
      echo $s > "$expected_dir/$name.status"
      grep -v '^WARNING\|^\[' "$out/$name.ref.err" | grep -m1 . > "$expected_dir/$name.stderr"
    fi
    echo "recorded fixture $name ($s)"
    return
  fi
  # `--module` stands before the arguments, as teq's own option.
  local own=()
  if [ "${args[0]}" = --module ]; then own=(--module "${args[1]}"); args=("${args[@]:2}"); fi
  timeout 120 "$TEQ" interp "$fixture_dir/scripts/$script" "${own[@]}" -- "${args[@]}" > "$out/$name.actual" 2> "$out/$name.err"
  judge "$name" $? "$expected_dir" "fixture "
}
# `products <name> <script> <root>...`: a script whose configuration builds on others, their products on its classpath,
# which teq interp runs with their sources (the configuration's product closure, `scriptdeps::product_inputs`).
# Scala CLI reads no lock, so its recorded run is given those sources, the roots after the script, explicitly. A root
# written `<root>=<scalac option>` is a product the build compiles apart with that option (its configuration's
# `-source`): the recording compiles it so, and gives the run its classes in the place of its sources.
products() {
  local name=$1 script=$2
  shift 2
  if [ $mode = record ]; then
    local roots=() classes=() r built
    for r in "$@"; do
      if [[ $r == *=* ]]; then
        built=$out/$name.${#classes[@]}.classes
        rm -rf "$built"
        if ! timeout 300 scala-cli compile -S 3.8.4 --jvm system "$fixture_dir/${r%%=*}" --scalac-option "${r#*=}" --compilation-output "$built" --server=false -q > /dev/null 2> "$out/$name.compile.err"; then
          echo "products $name: ${r%%=*} did not compile (see $out/$name.compile.err)"
          return
        fi
        classes+=("$built")
      else
        roots+=("$fixture_dir/$r")
      fi
    done
    [ ${#classes[@]} -gt 0 ] && roots+=(--classpath "$(IFS=:; echo "${classes[*]}")")
    timeout 300 scala-cli run -S 3.8.4 --jvm system "$fixture_dir/scripts/$script" "${roots[@]}" --server=false -q --suppress-directives-in-multiple-files-warning --suppress-outdated-dependency-warning > "$expected_dir/$name.expected" 2> "$out/$name.ref.err"
    local s=$?
    rm -f "$expected_dir/$name.status" "$expected_dir/$name.stderr"
    if [ $s != 0 ]; then
      echo $s > "$expected_dir/$name.status"
      grep -v '^WARNING\|^\[' "$out/$name.ref.err" | grep -m1 . > "$expected_dir/$name.stderr"
    fi
    echo "recorded products $name ($s)"
    return
  fi
  timeout 120 "$TEQ" interp "$fixture_dir/scripts/$script" > "$out/$name.actual" 2> "$out/$name.err"
  judge "$name" $? "$expected_dir" "products "
}
# `excluded <name> <spelling> <script> <root> <file>...`: a script whose product closure holds the fixture's <root>,
# given as an input with `--exclude` naming each <file>, which tests/interp-script/excluded/<file> writes under the
# root for the run alone: an excluded file is neither typed nor read for its directives, the closure's or the
# input's (Scala CLI's `CrossSources.excludeSources` leaves it out of the inputs on disk), whatever spelling of the
# path excludes it. The <spelling> `name` excludes the file by its name; `dot` runs from the fixture's root with every
# path `./`-spelled, the exclusion the file's own path (`./<root>/<file>`). Scala CLI's recorded run excludes it with
# its glob, `**/<file's name>`.
excluded() {
  local name=$1 spelling=$2 script=$3 root=$4 f s excludes=()
  shift 4
  for f in "$@"; do
    placed+=("$fixture_dir/$root/$f")
    cp "tests/interp-script/excluded/$f" "$fixture_dir/$root/$f"
  done
  if [ $mode = record ]; then
    for f in "$@"; do excludes+=(--exclude "**/${f##*/}"); done
    if [ "$spelling" = dot ]; then
      (cd "$fixture_dir" && timeout 300 scala-cli run -S 3.8.4 --jvm system "./scripts/$script" "./$root" "${excludes[@]}" --server=false -q --suppress-directives-in-multiple-files-warning --suppress-outdated-dependency-warning) > "$expected_dir/$name.expected" 2> "$out/$name.ref.err"
    else
      timeout 300 scala-cli run -S 3.8.4 --jvm system "$fixture_dir/scripts/$script" "$fixture_dir/$root" "${excludes[@]}" --server=false -q --suppress-directives-in-multiple-files-warning --suppress-outdated-dependency-warning > "$expected_dir/$name.expected" 2> "$out/$name.ref.err"
    fi
    s=$?
    rm -f "$expected_dir/$name.status" "$expected_dir/$name.stderr"
    if [ $s != 0 ]; then
      echo $s > "$expected_dir/$name.status"
      grep -v '^WARNING\|^\[' "$out/$name.ref.err" | grep -m1 . > "$expected_dir/$name.stderr"
    fi
    echo "recorded excluded $name ($s)"
  elif [ "$spelling" = dot ]; then
    for f in "$@"; do excludes+=(--exclude "./$root/$f"); done
    local teq
    teq=$(realpath "$TEQ")
    (cd "$fixture_dir" && timeout 120 "$teq" interp "./scripts/$script" "./$root" "${excludes[@]}") > "$out/$name.actual" 2> "$out/$name.err"
    judge "$name" $? "$expected_dir" "excluded "
  else
    for f in "$@"; do excludes+=(--exclude "${f##*/}"); done
    timeout 120 "$TEQ" interp "$fixture_dir/scripts/$script" "$fixture_dir/$root" "${excludes[@]}" > "$out/$name.actual" 2> "$out/$name.err"
    judge "$name" $? "$expected_dir" "excluded "
  fi
  for f in "$@"; do rm -f "$fixture_dir/$root/$f"; done
}
refusal() {
  local name=$1 script=$2
  shift 2
  [ $mode = record ] && return
  local file="$fixture_dir/scripts/$script"
  # A script outside every build: one of the fixture's in a directory of its own.
  if [ "$script" = OUTSIDE ]; then
    file="$home.outside/outside.sc"
    mkdir -p "$home.outside" && cp "$fixture_dir/scripts/no-sources.sc" "$file"
  fi
  # Teq's own options, after the script (`--classpath ""`).
  timeout 120 "$TEQ" interp "$file" "$@" > "$out/$name.actual" 2> "$out/$name.raw.err"
  local s=$?
  sed -e "s|$PWD|<repo>|g" -e "s|$home.outside|<outside>|g" "$out/$name.raw.err" > "$out/$name.err"
  rm -rf "$home.outside"
  : > "$out/$name.empty"
  judge "$name" $s "$expected_dir" "refusal " "$out/$name.empty"
}
. tests/interp-script/runs.sh
[ $mode = record ] && exit 0

touch "$out/passed" "$out/failed"
sort -o "$out/passed" "$out/passed"
status=0
if [ $mode = update ]; then
  cp "$out/passed" "$LIST"
  echo "$(wc -l < "$LIST" | tr -d ' ') runs listed in $LIST"
fi
[ -f "$LIST" ] || touch "$LIST"
regressed=$(comm -23 "$LIST" "$out/passed")
new=$(comm -13 "$LIST" "$out/passed")
[ -s "$out/failed" ] && { echo "failing:"; sed 's/^/  /' "$out/failed"; }
[ -n "$new" ] && echo "passing, not listed in $LIST: $(echo $new)"
if [ -n "$regressed" ]; then
  echo "REGRESSED: $(echo $regressed)"
  status=1
fi
echo "$(wc -l < "$out/passed" | tr -d ' ') passed, $(wc -l < "$out/failed" | tr -d ' ') failed"
exit $status
