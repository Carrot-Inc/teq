#!/bin/bash
# Builds every tests/cases program for the JVM with an assertion-enabled build (the cargo profile CHECKS_PROFILE names,
# docs/DEVELOPING.md, "The assertion-enabled build"), whose debug assertions the release compiles out, so that an
# assertion of the compiler fires where a program reaches it, and again with the release (TEQ) to compare their
# refusals by name. A build by the assertions binary that ends in a panic (an assertion among them: the profile
# aborts, exit 134), is killed by another signal, outlives its bound, or exits 0 without its jar fails. A refusal
# (exit 1, 2 or 3) fails where the release builds the program, and where the jvm list says it builds (the cases
# `tests/support/pending.py passing jvm` names and those tests/pending/jvm.txt lists past the build: verify, exception,
# output, timeout); a program the assertions binary builds and the release does not fails too, and so does one the
# release panics on, is killed on or outlives its bound on, whatever the list says. The programs are built as
# tests/run_jvm.sh builds them (their `// teq:` flags, scala-library's jar and their `// jars:` on the class path).
# They are built, not run: the assertions are the compiler's, and running adds nothing the row is for. A case of
# another system (`// os:`) is not built; one with a jar missing from the coursier cache is not run, a line naming it
# that the gate fails. Every program selected has a whole record of its own by each binary or fails, and a pass whose
# worker ends abnormally fails the run. JVM_ASSERTIONS_CASES names a few programs of tests/cases (by name, spaces
# between) to build them alone, the list held to them and the board left as it is (the gate's shard refuses it). Before
# any build the run fails where two inputs share a name, where the selector names nothing or a name that is no case,
# and where nothing is selected.
# The programs expected to panic are tests/pending/jvm-assertions.txt (tests/support/pending.py, under --fails-only:
# no board row for a program that builds, so a new case needs no run of this suite), each with its assertion's class,
# a word of ASSERTION_CLASSES below, never the site. The table is closed: a panic with another message fails, naming
# it, and a build that panics more than once is held only when every panic is of its class. The run fails on an
# unlisted failure, on a listed program that builds (builds now: --pending-update takes it out) and on a listed program
# that fails otherwise than its class says (moved). The list's flags are a suite of listed failures' (tests/support/
# pending.sh's pending_fails_args), --add and --update refused; nothing writes a program in: --pending-add is refused
# too, a new panic being a failure to mend, or to list by hand with its class. A failure's line carries the compiler's
# first error line or the panic's site and message.
# TEQ_CHECKS names the binary, whose `--version` must say `<this tree's revision> assertions` (the revision as build.rs
# stamps it: `git rev-parse --short HEAD`, or in a tree without its repository TEQ_REVISION, else GATE_REVISION), and
# which must be newer than the tree's compiler inputs (src, std, build.rs, Cargo.toml, Cargo.lock), none of them
# differing from the commit; without it the script builds it into target/.
# `--build <target directory>` only builds it there, its last line `jvm-assertions: built <binary> in <n> s`: the
# gate's shard runs it beside its other suites into target/gate-checks of its mirror of the landing worktree
# (tests/gate-shard.sh), a directory kept between gates (about 2.5 GB: the profile is incremental), and hands the
# binary as TEQ_CHECKS and that build's output as TEQ_CHECKS_BUILD_LOG; the run prints that directory's size.
# TEQ names the release (target/release/teq by default).
# The last line says how many programs reached the JVM writer with its assertions checked and how many stopped before
# it, the two passes' seconds and the build's, `cold` where cargo compiled a crate besides teq (a directory's first
# build, or its first after a toolchain or dependency change). JOBS programs at once (the cores by default), each
# bounded at JVM_ASSERTIONS_SECONDS (120). Before any build the judge runs on a planted corpus, scripts standing in
# for the two compilers (`jvm-assertions self-test: ...`).
cd "$(dirname "$0")/.."
export LC_COLLATE=C
export TZ=UTC

# The assertion-enabled build the row takes, by its cargo profile: the binary is <target directory>/<profile>/teq.
CHECKS_PROFILE=fast-checks

# build_checks <target directory>: the binary built there, cargo's output, then `jvm-assertions: built <binary> in <n> s`.
build_checks() {
  local started=$(date +%s)
  CARGO_TARGET_DIR=$1 cargo build --profile $CHECKS_PROFILE 2>&1 || return 1
  echo "jvm-assertions: built $1/$CHECKS_PROFILE/teq in $(($(date +%s) - started)) s"
}
if [ "${1:-}" = --build ]; then
  build_checks "${2:?the target directory}"
  exit
fi
. tests/support/pending.sh
pending_fails_args jvm-assertions "" "$@"
JOBS=${JOBS:-$(getconf _NPROCESSORS_ONLN 2> /dev/null || echo 4)}
out=out/jvm-assertions
update=
for arg in "$@"; do
  case $arg in
    --pending-update) update=--update ;;
    --pending-add) echo "jvm-assertions: --pending-add refused: a new panic is a failure to mend, or to list by hand in tests/pending/jvm-assertions.txt with its class, never written in by the run that met it"; exit 2 ;;
  esac
done
if [ -n "$update" ] && [ -n "${JVM_ASSERTIONS_CASES:-}" ]; then
  echo "jvm-assertions: --pending-update takes the whole corpus, not the programs JVM_ASSERTIONS_CASES names"
  exit 2
fi
rm -rf "${out:?}"
mkdir -p "$out"
case $(uname -s) in MINGW* | MSYS* | CYGWIN*) HERE_OS=windows ;; *) HERE_OS=unix ;; esac

# The assertions tests/pending/jvm-assertions.txt holds programs to: a class word (the list's detail) and after a tab
# the message's fixed text, an extended regular expression over the panic's message (the line after its site; an
# assertion without a message prints its condition as the source lays it out, its first line here). Never the site:
# its line moves with every edit of its file.
ASSERTION_CLASSES=$(
  cat << 'EOF'
abstractness-changed	^a symbol's abstractness changed after it was read$
kept-call-not-put-back	^assertion `left == right` failed: a kept call's node not put back$
head-rejection-kept-form	^assertion `left == right` failed: the head rejection of [^ ]+ read from its kept form$
base-types-rule-prefix	^assertion failed: info\.base_types\[1 \+
no-record-after-demand	^no record of [^ ]+ after its demand$
EOF
)

# build_one <src>: the program built by $BIN as tests/run_jvm.sh builds it, bounded at $BOUND seconds; its outcome in
# $DIR/<name>.$KIND, `<outcome> <name>` and after a tab what the log says (a panic: its message, then after a tab its
# site): built, refused (the compiler's exit 1, 2 or 3), panicked, timeout, killed (another signal), nojarfile (exit 0
# without its jar), otheros, nojar.
build_one() {
  local src=$1 name os flags code why site
  name=$(basename "$src" .scala)
  os=$(grep -h -o '^// os: [a-z]*' "$src" "$src"/*.scala 2> /dev/null | head -1)
  if [ -n "$os" ] && [ "${os#// os: }" != "$HERE_OS" ]; then
    echo "otheros $name" > "$DIR/$name.$KIND"
    return
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  . tests/support/jars.sh
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    printf 'nojar %s\t%s\n' "$name" "$JARS_MISSING" > "$DIR/$name.$KIND"
    return
  fi
  # the group's own standard error takes bash's notice of a build killed by a signal ("Aborted")
  {
    timeout -k 10 "$BOUND" "$BIN" compiler build "$src" $flags --classpath "$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar${JARS_CP:+:$JARS_CP}" --target jvm -o "$DIR/$name.$KIND.jar" > "$DIR/$name.$KIND.log" 2>&1
    code=$?
  } 2> /dev/null
  # every panic of the build, `<site>\t<message>` a line (threads may panic together before the abort)
  awk '/panicked at [^ ]*:[0-9]+:[0-9]+/ { match($0, /panicked at [^ ]*:[0-9]+:[0-9]+/); site = substr($0, RSTART + 12, RLENGTH - 12)
       msg = ""; getline msg; gsub(/\t/, " ", msg); print site "\t" substr(msg, 1, 300) }' "$DIR/$name.$KIND.log" > "$DIR/$name.$KIND.panics"
  if [ -s "$DIR/$name.$KIND.panics" ] || [ $code = 134 ]; then
    IFS=$'\t' read -r site why < "$DIR/$name.$KIND.panics"
    printf 'panicked %s\t%s\t%s\n' "$name" "${why:-exit $code}" "${site:-no site}" > "$DIR/$name.$KIND"
  elif [ $code = 124 ] || [ $code = 137 ]; then
    printf 'timeout %s\tnot ended within %s seconds\n' "$name" "$BOUND" > "$DIR/$name.$KIND"
  elif [ $code -gt 128 ]; then
    printf 'killed %s\tsignal %s\n' "$name" $((code - 128)) > "$DIR/$name.$KIND"
  elif [ $code != 0 ]; then
    why=$(grep -m 1 -i 'error' "$DIR/$name.$KIND.log")
    [ -n "$why" ] || why=$(grep -m 1 -v '^[[:space:]]*$' "$DIR/$name.$KIND.log")
    printf 'refused %s\texit %s: %s\n' "$name" $code "$(echo "$why" | tr '\t' ' ' | cut -c1-200)" > "$DIR/$name.$KIND"
  elif [ ! -s "$DIR/$name.$KIND.jar" ]; then
    printf 'nojarfile %s\texit 0 without its jar\n' "$name" > "$DIR/$name.$KIND"
  else
    echo "built $name" > "$DIR/$name.$KIND"
  fi
  rm -f "$DIR/$name.$KIND.jar"
  return 0
}
export -f build_one
export HERE_OS

# build_all <binary> <kind> <the programs, a file>: each built by it, JOBS at once, the outcomes $DIR/*.<kind>; its
# status xargs's, not 0 where a worker was killed or failed (xargs then starts no further build).
build_all() {
  BIN=$1 KIND=$2 xargs -P "$JOBS" -I{} bash -c 'build_one {}' < "$3"
}

# select_inputs <cases directory> <file>: the programs of the run into the file, a path a line: every case of the
# directory that exists, or those JVM_ASSERTIONS_CASES names; status 1 and the line saying why, before any build,
# where the selector names nothing or a name that is no case, where nothing is selected, or where two cases share a
# name (their outcomes would be one record).
select_inputs() {
  local dir=$1 to=$2 src n missing= twice
  if [ -z "${JVM_ASSERTIONS_CASES:-}" ]; then
    for src in "$dir"/*.scala "$dir"/*/; do [ ! -e "$src" ] || echo "${src%/}"; done > "$to"
  else
    [ -n "$(echo $JVM_ASSERTIONS_CASES)" ] || { echo "JVM_ASSERTIONS_CASES (\"$JVM_ASSERTIONS_CASES\") names no case"; return 1; }
    for n in $JVM_ASSERTIONS_CASES; do
      [ ! -f "$dir/$n.scala" ] || echo "$dir/$n.scala"
      [ ! -d "$dir/$n" ] || echo "$dir/$n"
      [ -f "$dir/$n.scala" ] || [ -d "$dir/$n" ] || missing="$missing $n"
    done > "$to"
    [ -z "$missing" ] || { echo "JVM_ASSERTIONS_CASES names no case of $dir:$missing"; return 1; }
  fi
  [ -s "$to" ] || { echo "no program to build: $dir holds no case"; return 1; }
  twice=$(sed 's|.*/||; s|\.scala$||' "$to" | LC_ALL=C sort | uniq -d | tr '\n' ' ')
  [ -z "$twice" ] || { echo "two inputs share a name, whose outcomes would be one record: $twice"; return 1; }
}

# record <file> <name>: the outcome record's line where the file is whole (it ends in a new line), its outcome one this
# script writes and its program the name; else nothing, status 1.
record() {
  local line head
  # read's status is 1 where the line ends without its new line
  [ -s "$1" ] && IFS= read -r line < "$1" || return 1
  head=${line%%$'\t'*}
  case ${head%% *} in built | refused | panicked | timeout | killed | nojarfile | otheros | nojar) ;; *) return 1 ;; esac
  [ "${head#* }" = "$2" ] || return 1
  RECORD=$line
}

# stale_against <binary> <tree>: the tree's compiler inputs (src, std, build.rs, Cargo.toml, Cargo.lock) that are newer
# than the binary, and, where the tree is a repository's top, those that differ from its commit; nothing where the
# binary can be their build.
stale_against() {
  local newer changed=
  newer=$(cd "$2" && find src std build.rs Cargo.toml Cargo.lock -type f -newer "$1" 2> /dev/null | head -3 | tr '\n' ' ')
  if [ "$(cd "$2" && git rev-parse --show-toplevel 2> /dev/null)" = "$(cd "$2" && pwd -P)" ]; then
    changed=$(cd "$2" && git status --porcelain -- src std build.rs Cargo.toml Cargo.lock 2> /dev/null | head -3 | cut -c4- | tr '\n' ' ')
  fi
  [ -z "$newer$changed" ] || echo "${newer:+newer than it: $newer}${newer:+${changed:+; }}${changed:+differing from its commit: $changed}"
}

# identity <binary> <revision>: whether the binary is an assertion-enabled build of that revision, its --version
# `teq <version> <revision> assertions`, the two abbreviations of one commit (git lengthens its abbreviation as a
# repository grows, so a build's stamp and a later reading may differ in length: one must begin the other, each of
# seven characters at least); else the line saying why.
identity() {
  local v stamp
  v=$("$1" --version 2>&1 | head -1)
  [ -n "$2" ] || { echo "this tree's revision is unknown (no repository, and neither TEQ_REVISION nor GATE_REVISION)"; return 1; }
  stamp=$(echo "$v" | awk '{print $3}')
  [ ${#stamp} -ge 7 ] && [ ${#2} -ge 7 ] && { [[ $2 == "$stamp"* ]] || [[ $stamp == "$2"* ]]; } &&
    [ "$(echo "$v" | awk '{print $4}')" = assertions ] && [ -z "$(echo "$v" | awk '{print $5}')" ] ||
    { echo "$1 says \"$v\", not an assertion-enabled build of this tree's $2"; return 1; }
}

# class_of <message>: the class word of ASSERTION_CLASSES whose text the message matches into CLASS, else nothing.
class_of() {
  local word re
  CLASS=
  while IFS=$'\t' read -r word re; do
    [[ $1 =~ $re ]] && { CLASS=$word; return; }
  done <<< "$ASSERTION_CLASSES"
}

# panic_class <the build's panics, `<site>\t<message>` a line>: into CLASS, the class word every one of its panics is
# of, else `unclassed: <the first message of no class>`, else `several classes: <the words>`.
panic_class() {
  local site msg words= first=
  while IFS=$'\t' read -r site msg; do
    [ -n "$site$msg" ] || continue
    class_of "$msg"
    if [ -z "$CLASS" ]; then first=${first:-"unclassed: $msg"}
    else case " $words " in *" $CLASS "*) ;; *) words="${words:+$words }$CLASS" ;; esac; fi
  done <<< "$1"
  if [ -n "$first" ]; then CLASS=$first
  elif [ "$words" = "${words% *}" ]; then CLASS=$words
  else CLASS="several classes: $words"; fi
}

# verdicts <dir> <the names that must build, a file> <the programs' names, a file>: $dir/results.txt (each program's
# outcome by the assertions binary, the release's after a tab, then the first panic's message and site and how many
# more there were) and $dir/verdicts.txt, each program's verdict as tests/support/pending.py reads it (a panic's
# detail its class word, or `unclassed: <message>`). A program without an outcome by either binary fails, and the
# release's panic, kill or timeout fails before any other reading.
verdicts() {
  local dir=$1 must=$2 names=$3 head rest site outcome name release rrest rsite more pans
  local -a pan
  local -A must_build release_failed
  while IFS= read -r name; do [ -z "$name" ] || must_build[$name]=1; done < "$must"
  while IFS= read -r name; do
    if record "$dir/$name.result" "$name"; then IFS=$'\t' read -r head rest site <<< "$RECORD"
    else head="missing $name" rest="no outcome by the assertions binary (no whole record of its own: its worker died, or the record was cut or names another program)" site=; fi
    if record "$dir/$name.release" "$name"; then IFS=$'\t' read -r release rrest rsite <<< "$RECORD"; release=${release%% *}
    else release=missing rrest="no outcome (no whole record of its own)" rsite=; fi
    more=
    pan=()
    [ ! -s "$dir/$name.result.panics" ] || mapfile -t pan < "$dir/$name.result.panics"
    [ ${#pan[@]} -le 1 ] || more="$((${#pan[@]} - 1)) more panics, the next at ${pan[1]/$'\t'/: }"
    # the release's outcome second and no field empty but the last: bash's read folds a run of tabs
    printf '%s\t%s\t%s\t%s\t%s\n' "$head" "$release" "${rest:--}" "${site:--}" "$more"
    case $release in
      built | refused | otheros | nojar) ;;
      *) release_failed[$name]="the release $release${rrest:+: $rrest}${rsite:+ at $rsite}" ;;
    esac
  done < "$names" > "$dir/results.txt"
  while IFS=$'\t' read -r head release rest site more; do
    outcome=${head%% *} name=${head#* }
    [ "$rest" != - ] || rest=
    if [ -n "${release_failed[$name]:-}" ] && [ "$outcome" != otheros ] && [ "$outcome" != nojar ]; then
      printf 'fail %s\t%s\n' "$name" "${release_failed[$name]}"
      continue
    fi
    case $outcome in
      built)
        if [ "$release" = built ] || [ "$release" = otheros ]; then
          printf 'pass %s\n' "$name"
        else
          printf 'fail %s\tbuilt by the assertions binary, not by the release (%s)\n' "$name" "$release"
        fi
        ;;
      refused)
        if [ "$release" != refused ]; then
          printf 'fail %s\trefused by the assertions binary (%s), not by the release (%s)\n' "$name" "$rest" "$release"
        elif [ -n "${must_build[$name]:-}" ]; then
          printf 'fail %s\trefused where the jvm list says it builds: %s\n' "$name" "$rest"
        else
          printf 'pass %s\n' "$name"
        fi
        ;;
      panicked)
        pans=
        [ ! -s "$dir/$name.result.panics" ] || { mapfile -t pan < "$dir/$name.result.panics"; printf -v pans '%s\n' "${pan[@]}"; }
        if [ -n "$pans" ]; then panic_class "$pans"; else CLASS="unclassed: $rest"; fi
        printf 'fail %s\t%s\n' "$name" "$CLASS"
        ;;
      otheros) printf "skip %s\tanother system's\n" "$name" ;;
      nojar) printf 'skip %s\ta jar not in the coursier cache: %s\n' "$name" "$rest" ;;
      missing) printf 'fail %s\t%s\n' "$name" "$rest" ;;
      *) printf 'fail %s\t%s: %s\n' "$name" "$outcome" "$rest" ;;
    esac
  done < "$dir/results.txt" > "$dir/verdicts.txt"
}

# judge <dir> <the list>: the run's lines over $dir/results.txt, $dir/verdicts.txt and the list, a FAIL line a failure
# (an unlisted one, a listed program that builds, one that fails otherwise than its class), the counts last; 1 on any
# FAIL.
judge() {
  local dir=$1 list=$2 failed=0 held=0 otherwise=0 alike=0 checked=0 stopped=0 others=0 missing=0 verdict name note
  local listed what head rest site release outcome
  declare -A class_word listed_as said kind
  while IFS=$'\t' read -r name rest; do class_word[$name]=1; done <<< "$ASSERTION_CLASSES"
  while IFS=$'\t' read -r name note; do [ -n "$name" ] && [ "${name#\#}" = "$name" ] && listed_as[$name]=$note; done < <(cat "$list" 2> /dev/null)
  while IFS=$'\t' read -r head release rest site more; do
    outcome=${head%% *} name=${head#* }
    kind[$name]=$outcome
    case $outcome in
      panicked) said[$name]="panicked at $site: $rest${more:+; $more}" ;;
      built) said[$name]="built" ;;
      missing) said[$name]=$rest ;;
      *) said[$name]="$outcome: $rest" ;;
    esac
    case $outcome in
      built) checked=$((checked + 1)) ;;
      otheros) others=$((others + 1)) ;;
      nojar) missing=$((missing + 1)) ;;
      *) stopped=$((stopped + 1)) ;;
    esac
  done < "$dir/results.txt"
  while IFS=$'\t' read -r head note; do
    verdict=${head%% *} name=${head#* } listed=${listed_as[$name]-}
    what=${said[$name]}
    case $verdict in
      pass)
        [ "${kind[$name]}" != refused ] || alike=$((alike + 1))
        if [ -n "${listed_as[$name]+1}" ]; then
          echo "FAIL jvm-assertions $name: $what, where tests/pending/jvm-assertions.txt lists $listed: builds now (take it out: --pending-update)"
          failed=$((failed + 1))
        fi
        ;;
      fail)
        if [[ $note == "the release "* || $note == "no outcome "* ]]; then
          # whatever the list says of the assertions binary's build
          echo "FAIL jvm-assertions $name: $note"
          failed=$((failed + 1))
          continue
        elif [ -z "${listed_as[$name]+1}" ]; then
          case $note in
            "unclassed: "*) echo "FAIL jvm-assertions $name: a message of no class of the script's table: $what" ;;
            *) [ "${kind[$name]}" = panicked ] && echo "FAIL jvm-assertions $name: $what" || echo "FAIL jvm-assertions $name: $note" ;;
          esac
        elif [ -z "$listed" ] || [ -z "${class_word[$listed]:-}" ]; then
          echo "FAIL jvm-assertions $name: tests/pending/jvm-assertions.txt lists \"$listed\", no class of the script's table: $what"
        elif [ "$note" != "$listed" ]; then
          echo "FAIL jvm-assertions $name: $what, where tests/pending/jvm-assertions.txt lists $listed: moved"
        else
          held=$((held + 1))
          continue
        fi
        failed=$((failed + 1))
        [ "${kind[$name]}" != panicked ] || otherwise=$((otherwise + 1))
        ;;
      skip) case $note in "a jar not in"*) echo "not run jvm-assertions $name: $note" ;; esac ;;
    esac
  done < "$dir/verdicts.txt"
  # the failures' counts first: the gate's table shows a row's first 150 characters
  echo "jvm-assertions: the writer's assertions checked on $checked programs, $stopped stopped before it ($otherwise panicked otherwise, $((failed - otherwise)) failed otherwise, $held panicked as tests/pending/jvm-assertions.txt lists, $alike refused as the release refuses them), $others another system's, $missing not run${PASS_SECONDS:+; ${PASS_SECONDS} s}${BUILT:+; $BUILT}${SELECTED:+; $SELECTED}"
  [ $failed = 0 ]
}

# build_kind <cargo's log>: cold where cargo compiled a crate besides teq, else warm.
build_kind() {
  if grep '^ *Compiling ' "$1" | grep -qv '^ *Compiling teq '; then echo cold; else echo warm; fi
}

# expected <the assertions binary's record> <the release's record> <the list's state>: a program's verdict by rule,
# written apart from the judge's code: one without a whole record of its own by the assertions binary fails; so does
# one whose release record is not whole or not its own, or says the release panicked, was killed, outlived its bound
# or wrote no jar; else one the assertions binary built passes (PASS) where the release built it and the list does
# not name it; one it refused passes (ALIKE) where the release refused it and the list does not name it; one that
# panicked is held (HELD) where its every panic is of the class the list gives it; everything else fails (FAIL).
# The records: built, refused, a panic of the class abstractness-changed (panick), of head-rejection-kept-form
# (panick2), of no class (panicu), several (the first of abstractness-changed, the next of none), a signal, a timeout,
# a build without its jar, a record cut short, one naming another program, none; the list's states: listed under
# abstractness-changed (lk), not listed (no), listed under head-rejection-kept-form (lk2).
expected() {
  EXPECTED=FAIL
  case $1 in truncated | othername | none) return ;; esac
  case $2 in built | refused) ;; *) return ;; esac
  case $1:$2:$3 in
    built:built:no) EXPECTED=PASS ;;
    refused:refused:no) EXPECTED=ALIKE ;;
    panick:*:lk | panick2:*:lk2) EXPECTED=HELD ;;
  esac
}

# self_test: the judge over a generated table of its inputs. Every input kind of a program (a file case, a directory
# case) × the assertions binary's record × the release's × the list's state, a program a cell, its two records those
# the scripts standing in for the two compilers wrote through build_all for that input kind and record kind (each
# held to its outcome), the cell's name in them, cut short, swapped or removed where the cell says so; its verdict
# held to `expected`; a refusal where the jvm list says the program builds; the inputs that fail a run
# before any build (two cases of one name, by the corpus and by the selector; a case path that exists nowhere; no
# case; a selector of white space; one naming no case) through select_inputs, beside a selection that holds; a worker
# killed in a pass; an assertions binary of another revision, and one older than the tree's compiler inputs, beside
# their controls; build_kind over two planted logs; the fifth class's message held under its class, and another message
# at its site failing under it. The review's nine probes are cells of it.
self_test() {
  local d=$out/self-test cells=0 ok=0 otherwise= i c r l k name p kill_status why want got line rest
  local -a pan
  local records="built refused panick panick2 panicu several signal timeout nojarfile truncated othername none"
  mkdir -p "$d/cases" "$d/results" "$d/kill"
  cat > "$d/teq" << 'EOF'
#!/bin/bash
ulimit -c 0
jar= me=$(basename "$0")
for ((i = 1; i <= $#; i++)); do [ "${!i}" = -o ] && { j=$((i + 1)); jar=${!j}; }; done
said() { echo "thread '$1' (4242) panicked at src/planted.rs:$2:9:"; echo "$3"; }
at() { echo "thread 'main' (4242) panicked at src/typer/substitution.rs:292:13:"; echo "$1"; }
name=$(basename "$3" .scala)
case $name in
  x-killworker) [ $me != teq ] || { kill -KILL "$(ps -o ppid= -p $PPID | tr -d ' ')"; exit 1; } ;;
esac
case ${name##*-} in
  refused) echo "-- [E007] Type Mismatch Error: planted" >&2; exit 1 ;;
  panick) said main 1 "a symbol's abstractness changed after it was read"; kill -ABRT $$ ;;
  panick2) said main 2 "assertion \`left == right\` failed: the head rejection of a.B.c read from its kept form"; kill -ABRT $$ ;;
  panicu) said main 3 "index out of bounds: the len is 1 but the index is 2"; kill -ABRT $$ ;;
  several) said worker 4 "a symbol's abstractness changed after it was read"; said main 5 "index out of bounds: the len is 1 but the index is 2"; kill -ABRT $$ ;;
  signal) kill -TERM $$ ;;
  timeout) exec sleep 5 ;;
  nojarfile) exit 0 ;;
  panick5) at "no record of G after its demand"; kill -ABRT $$ ;;
  panicw) at "no record of G after its demand, nor of its forwarder"; kill -ABRT $$ ;;
esac
echo planted > "$jar"
EOF
  chmod +x "$d/teq"
  cp "$d/teq" "$d/teq-release"
  # the prototypes: a program p-<input kind>-<record kind> for each, built by both stand-ins through build_all, each
  # record held to the outcome its kind is
  mkdir -p "$d/proto"
  for i in f d; do
    for k in $records; do
      if [ $i = f ]; then
        echo "object Main" > "$d/cases/p-$i-$k.scala"
        echo "$d/cases/p-$i-$k.scala"
      else
        mkdir -p "$d/cases/p-$i-$k"
        echo "object Main" > "$d/cases/p-$i-$k/Main.scala"
        echo "$d/cases/p-$i-$k"
      fi
    done
  done > "$d/protos"
  (export DIR=$d/proto BOUND=1 && build_all "$d/teq" result "$d/protos") 2> "$d/stderr" || otherwise="$otherwise the-assertions-pass"
  [ ! -s "$d/stderr" ] || otherwise="$otherwise the-assertions-pass's-output"
  (export DIR=$d/proto BOUND=1 && build_all "$d/teq-release" release "$d/protos") 2> /dev/null || otherwise="$otherwise the-release-pass"
  for i in f d; do
    for k in $records; do
      case $k in
        built | truncated | othername | none) want=built ;;
        refused) want=refused ;;
        panick | panick2 | panicu | several) want=panicked ;;
        signal) want=killed ;;
        *) want=$k ;;
      esac
      for p in result release; do
        cells=$((cells + 1))
        if record "$d/proto/p-$i-$k.$p" "p-$i-$k" && [ "${RECORD%% *}" = $want ]; then ok=$((ok + 1)); else otherwise="$otherwise p-$i-$k.$p:${RECORD%% *}"; fi
      done
    done
  done
  # the fifth class's prototypes, by the assertions stand-in: its message, and another message at its site that the
  # class's text, anchored whole, does not take
  for k in panick5 panicw; do echo "object Main" > "$d/cases/p-f-$k.scala"; echo "$d/cases/p-f-$k.scala"; done > "$d/protos5"
  (export DIR=$d/proto BOUND=1 && build_all "$d/teq" result "$d/protos5") 2> "$d/stderr5" || otherwise="$otherwise the-fifth-pass"
  [ ! -s "$d/stderr5" ] || otherwise="$otherwise the-fifth-pass's-output"
  for k in panick5 panicw; do
    cells=$((cells + 1))
    if record "$d/proto/p-f-$k.result" "p-f-$k" && [ "${RECORD%% *}" = panicked ]; then ok=$((ok + 1)); else otherwise="$otherwise p-f-$k.result:${RECORD%% *}"; fi
  done
  # put <the prototype's record> <the cell's record, a file> <the record kind>: the prototype's record, the cell's name
  # in it (and its panics beside it), or the record cut short, naming another program, or none, as the kind says
  put() {
    local line head
    case $3 in
      truncated) printf built > "$2"; return ;;
      othername) printf 'built x-another\n' > "$2"; return ;;
      none) return ;;
    esac
    IFS= read -r line < "$1"
    head=${line%%$'\t'*}
    printf '%s %s%s\n' "${head%% *}" "$name" "${line#"$head"}" > "$2"
    [ ! -s "$1.panics" ] || { mapfile -t pan < "$1.panics"; printf '%s\n' "${pan[@]}" > "$2.panics"; }
  }
  # the table: a program a cell, named <input kind>-<assertions record>-<release record>-<list state>
  : > "$d/list"
  for i in f d; do
    for c in $records; do
      for r in $records; do
        for l in lk no lk2; do
          name=$i-$c-$r-$l
          put "$d/proto/p-$i-$c.result" "$d/results/$name.result" $c
          put "$d/proto/p-$i-$r.release" "$d/results/$name.release" $r
          expected $c $r $l
          echo "$name $EXPECTED"
          case $l in
            lk) printf '%s\tabstractness-changed\n' $name >> "$d/list" ;;
            lk2) printf '%s\thead-rejection-kept-form\n' $name >> "$d/list" ;;
          esac
        done
      done
    done
  done > "$d/expected"
  # a refusal alike where the jvm list says the program builds: FAIL
  name=x-refusedmust
  put "$d/proto/p-f-refused.result" "$d/results/$name.result" refused
  put "$d/proto/p-f-refused.release" "$d/results/$name.release" refused
  echo "x-refusedmust FAIL" >> "$d/expected"
  # the fifth class: its panic held where the list names the class; another message at the same site, listed under it,
  # fails
  for c in panick5:HELD panicw:FAIL; do
    name=x-fifth-${c%%:*}
    put "$d/proto/p-f-${c%%:*}.result" "$d/results/$name.result" ${c%%:*}
    put "$d/proto/p-f-built.release" "$d/results/$name.release" built
    printf '%s\tno-record-after-demand\n' $name >> "$d/list"
    echo "$name ${c#*:}" >> "$d/expected"
  done
  # an empty line among them, as `pending.py passing` writes for an empty board
  printf '%s\n' "" x-refusedmust > "$d/must"
  cut -d' ' -f1 "$d/expected" > "$d/names"
  verdicts "$d/results" "$d/must" "$d/names"
  judge "$d/results" "$d/list" > "$d/judge.out"
  local -A failed_line verdict_of outcome_of
  while IFS= read -r line; do
    case $line in "FAIL jvm-assertions "*) line=${line#FAIL jvm-assertions }; failed_line[${line%%: *}]=1 ;; esac
  done < "$d/judge.out"
  while IFS=$'\t' read -r line rest; do verdict_of[${line#* }]=${line%% *}; done < "$d/results/verdicts.txt"
  while IFS=$'\t' read -r line rest; do outcome_of[${line#* }]=${line%% *}; done < "$d/results/results.txt"
  while IFS=' ' read -r name want; do
    if [ -n "${failed_line[$name]:-}" ]; then got=FAIL
    elif [ "${verdict_of[$name]:-}" = pass ]; then
      [ "${outcome_of[$name]:-}" = refused ] && got=ALIKE || got=PASS
    else got=HELD; fi
    cells=$((cells + 1))
    if [ "$got" = "$want" ]; then ok=$((ok + 1)); else otherwise="$otherwise $name:$want/$got"; fi
  done < "$d/expected"
  # the review's probes among the cells, a listed program that builds and the fifth class's planted message, each
  # with its reason
  probe() { # probe <cell> <the reason's words in its FAIL line>
    grep -q "^FAIL jvm-assertions $1: .*$2" "$d/judge.out" || otherwise="$otherwise probe:$1"
  }
  probe f-several-built-lk "more panics, the next at src/planted.rs:5:9: index out of bounds.*: moved"
  probe f-panick-panicu-lk "the release panicked: index out of bounds"
  probe d-truncated-built-no "no outcome by the assertions binary"
  probe f-built-built-lk "built, where tests/pending/jvm-assertions.txt lists abstractness-changed: builds now (take it out: --pending-update)$"
  probe x-fifth-panicw "panicked at src/typer/substitution.rs:292:13: no record of G after its demand, nor of its forwarder, where tests/pending/jvm-assertions.txt lists no-record-after-demand: moved$"
  # the inputs refused before any build, and a selection that holds
  mkdir -p "$d/twice/p" "$d/one" "$d/none" "$d/dangling"
  echo "object Main" > "$d/twice/p.scala"
  echo "object Main" > "$d/twice/p/Main.scala"
  echo "object Main" > "$d/one/p.scala"
  ln -sf "$d/nowhere.scala" "$d/dangling/x.scala"
  refused_before() { # refused_before <cell> <selector> <cases directory> <the reason's words>
    cells=$((cells + 1))
    if why=$(JVM_ASSERTIONS_CASES=$2 select_inputs "$3" "$d/selected"); then otherwise="$otherwise $1:held"
    elif [[ $why != *"$4"* ]]; then otherwise="$otherwise $1:$why"
    else ok=$((ok + 1)); fi
  }
  refused_before duplicate "" "$d/twice" "two inputs share a name"
  refused_before duplicate-selected p "$d/twice" "two inputs share a name"
  refused_before dangling "" "$d/dangling" "holds no case"
  refused_before empty "" "$d/none" "holds no case"
  refused_before zero "   " "$d/twice" "names no case"
  refused_before nosuch nosuch "$d/twice" "names no case of"
  cells=$((cells + 1))
  if JVM_ASSERTIONS_CASES=p select_inputs "$d/one" "$d/selected" > /dev/null && [ "$(cat "$d/selected")" = "$d/one/p.scala" ]; then ok=$((ok + 1)); else otherwise="$otherwise selected"; fi
  # a worker killed in the middle of the assertions binary's pass: the pass's status, and the program without an outcome
  echo "object Main" > "$d/cases/x-killworker.scala"
  printf '%s\n' "$d/cases/x-killworker.scala" "$d/cases/p-f-built.scala" > "$d/kill/inputs"
  printf '%s\n' x-killworker p-f-built > "$d/kill/names"
  (export DIR=$d/kill BOUND=1 && build_all "$d/teq" result "$d/kill/inputs") 2> "$d/kill/stderr"
  kill_status=$?
  (export DIR=$d/kill BOUND=1 && build_all "$d/teq-release" release "$d/kill/inputs") 2>> "$d/kill/stderr"
  verdicts "$d/kill" "$d/must" "$d/kill/names"
  judge "$d/kill" "$d/list" > "$d/kill/judge.out"
  cells=$((cells + 1))
  if [ $kill_status != 0 ] && grep -q "^FAIL jvm-assertions x-killworker: no outcome by the assertions binary" "$d/kill/judge.out"; then ok=$((ok + 1)); else otherwise="$otherwise worker"; fi
  # an assertions binary of another revision, one older than the tree's compiler inputs, and their controls
  printf '#!/bin/bash\necho "teq 0.1.0 %s assertions"\n' deadbeef0 > "$d/stale-teq"
  printf '#!/bin/bash\necho "teq 0.1.0 %s assertions"\n' c0ffee000 > "$d/good-teq"
  chmod +x "$d/stale-teq" "$d/good-teq"
  mkdir -p "$d/tree/src"
  : > "$d/tree-older"
  touch -t 200001010000 "$d/tree-older"
  echo "fn main() {}" > "$d/tree/src/main.rs"
  : > "$d/tree-newer"
  cells=$((cells + 6))
  if ! identity "$d/stale-teq" c0ffee000 > /dev/null; then ok=$((ok + 1)); else otherwise="$otherwise stale"; fi
  if identity "$d/good-teq" c0ffee000 > /dev/null; then ok=$((ok + 1)); else otherwise="$otherwise good-revision"; fi
  # the stamp and the reading abbreviations of one commit of other lengths: taken; of fewer than seven: refused
  if identity "$d/good-teq" c0ffee00012 > /dev/null; then ok=$((ok + 1)); else otherwise="$otherwise good-longer"; fi
  if ! identity "$d/good-teq" c0ffee > /dev/null; then ok=$((ok + 1)); else otherwise="$otherwise short"; fi
  if [ -n "$(stale_against "$PWD/$d/tree-older" "$d/tree")" ]; then ok=$((ok + 1)); else otherwise="$otherwise dirty"; fi
  if [ -z "$(stale_against "$PWD/$d/tree-newer" "$d/tree")" ]; then ok=$((ok + 1)); else otherwise="$otherwise fresh"; fi
  printf '%s\n' "   Compiling libc v0.2.0" "   Compiling teq v0.1.0 (.)" > "$d/cold.log"
  printf '%s\n' "   Compiling teq v0.1.0 (.)" > "$d/warm.log"
  cells=$((cells + 1))
  if [ "$(build_kind "$d/cold.log") $(build_kind "$d/warm.log")" = "cold warm" ]; then ok=$((ok + 1)); else otherwise="$otherwise build_kind"; fi
  if [ -n "$otherwise" ] || [ $ok != $cells ]; then
    echo "FAIL jvm-assertions self-test: $cells cells, $ok as the rule says, otherwise:$(echo $otherwise | tr ' ' '\n' | head -20 | tr '\n' ' ')"
    return 1
  fi
  echo "jvm-assertions self-test: $cells cells, $ok as the rule says, 0 otherwise (48 records the stand-ins wrote: 2 input kinds × 12 record kinds × 2 binaries; 864 programs: 2 input kinds × 12 records by each binary × 3 list states; a refusal where the program must build; the fifth class's message and another at its site, 2 records and 2 programs, the one held under its class, the other failing; 6 inputs refused before any build and a selection that holds; a killed worker; 6 binaries: another revision, a stamp of fewer than seven characters, older than the tree's sources, and their controls, one stamped at another length; build_kind), the review's nine probes among them (worker, panic, release-panic, stale, duplicate, truncated, dirty, zero, empty)"
}

self_test || exit 1
why=$(select_inputs tests/cases "$out/inputs.txt") || { echo "FAIL jvm-assertions: $why"; exit 1; }
sed 's|.*/||; s|\.scala$||' "$out/inputs.txt" > "$out/names.txt"
SELECTED=
[ -z "${JVM_ASSERTIONS_CASES:-}" ] || SELECTED="$(grep -c . "$out/names.txt") programs selected by JVM_ASSERTIONS_CASES"
# the revision build.rs stamps the tree's builds with
revision=$(git rev-parse --short HEAD 2> /dev/null) || revision=${TEQ_REVISION:-${GATE_REVISION:-}}
if [ -z "$TEQ_CHECKS" ]; then
  build_checks target > "$out/build.log" || { tail -5 "$out/build.log"; echo "FAIL jvm-assertions: the assertion-enabled build failed"; exit 1; }
  TEQ_CHECKS_BUILD_LOG=$out/build.log
  TEQ_CHECKS=$(sed -n 's/^jvm-assertions: built \(.*\) in [0-9]* s$/\1/p' "$out/build.log")
fi
BUILT=
[ -z "$TEQ_CHECKS_BUILD_LOG" ] || BUILT="its build $(build_kind "$TEQ_CHECKS_BUILD_LOG"), $(sed -n 's/^jvm-assertions: built .* in \([0-9]*\) s$/\1/p' "$TEQ_CHECKS_BUILD_LOG") s"
CHECKS=$(cd "$(dirname "$TEQ_CHECKS")" && pwd)/$(basename "$TEQ_CHECKS")
RELEASE=${TEQ:-target/release/teq}
[ -x "$RELEASE" ] || { echo "FAIL jvm-assertions: no release binary at $RELEASE (TEQ)"; exit 1; }
RELEASE=$(cd "$(dirname "$RELEASE")" && pwd)/$(basename "$RELEASE")
why=$(identity "$CHECKS" "$revision") || { echo "FAIL jvm-assertions: $why"; exit 1; }
# a binary the run did not build: refused where the tree's compiler inputs are newer, or not its commit's
if [ "$TEQ_CHECKS_BUILD_LOG" != "$out/build.log" ]; then
  why=$(stale_against "$CHECKS" .)
  [ -z "$why" ] || { echo "FAIL jvm-assertions: $CHECKS cannot be the build of this tree's compiler inputs ($why): unset TEQ_CHECKS for the row to build it"; exit 1; }
fi
echo "jvm-assertions: the assertions binary $("$CHECKS" --version), the release $("$RELEASE" --version)"
built_in=$(sed -n "s|^jvm-assertions: built \(.*\)/$CHECKS_PROFILE/teq in [0-9]* s\$|\1|p" "${TEQ_CHECKS_BUILD_LOG:-/dev/null}")
[ -z "$built_in" ] || echo "jvm-assertions: its build's directory $built_in holds $(du -sh "$built_in" | cut -f1)"
. tests/support/jars.sh
export M2 BOUND=${JVM_ASSERTIONS_SECONDS:-120} DIR=$out/results
mkdir -p "$DIR"
status=0
started=$(date +%s)
build_all "$CHECKS" result "$out/inputs.txt" || { echo "FAIL jvm-assertions: the assertions binary's pass ended abnormally (xargs exit $?: a worker killed or failed)"; status=1; }
checks_seconds=$(($(date +%s) - started))
build_all "$RELEASE" release "$out/inputs.txt" || { echo "FAIL jvm-assertions: the release's pass ended abnormally (xargs exit $?: a worker killed or failed)"; status=1; }
release_seconds=$(($(date +%s) - started - checks_seconds))
{
  python3 tests/support/pending.py passing jvm
  awk -F'\t' '{ split($2, w, /[ ;:]/) } w[1] ~ /^(verify|exception|output|timeout)$/ { print $1 }' tests/pending/jvm.txt
} | LC_ALL=C sort -u > "$out/must-build.txt"
verdicts "$DIR" "$out/must-build.txt" "$out/names.txt"
if [ -n "$SELECTED" ]; then
  echo "jvm-assertions: $SELECTED, the list held to them alone, the board left as it is"
else
  python3 tests/support/pending.py jvm-assertions "$DIR/verdicts.txt" --fails-only --passed="$(grep -c '^pass ' "$DIR/verdicts.txt")" $update || status=1
fi
PASS_SECONDS="$checks_seconds and the release's $release_seconds" BUILT=$BUILT SELECTED=$SELECTED judge "$DIR" tests/pending/jvm-assertions.txt || status=1
exit $status
