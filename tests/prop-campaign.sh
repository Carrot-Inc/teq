#!/bin/bash
# A campaign of the property-based tests of proptests/ (proptests/README.md, "Campaigns"): short
# discovery pieces on fresh seeds with shrinking off, each piece one property's test with its
# own seed, database and work directory, several at a time; a shrink run for each failure as its
# piece ends, kept to the failure's kind, the first failure of each kind first and every other
# while time remains; then a report, `report.md` in the campaign's directory. It exits 0 where
# nothing failed, 1 where something did (or a step of its own), 3 where nothing failed but a
# piece or a shrink run was cut (incomplete); `collect` and `nightly` exit with the campaign's
# status, 1 where the fetch fails and 3 where the campaign did not end.
#
#   tests/prop-campaign.sh here <seconds>             the campaign on this machine
#   tests/prop-campaign.sh start <seconds> [machine]  on a remote machine (default: a free one):
#                                                     claims it, mirrors this tree, builds teq
#                                                     and starts the campaign in the background;
#                                                     prints the handle the other two take
#   tests/prop-campaign.sh status <handle>            the campaign's progress there, once
#   tests/prop-campaign.sh collect <handle> [dir]     fetches its directory (default
#                                                     out/campaign/<its stamp>), frees the machine
#   tests/prop-campaign.sh nightly <seconds> [machine] start, a look every minute until the report
#                                                     is written (bounded), collect: a cron line's
#                                                     command
#   tests/prop-campaign.sh requeue <database>         makes the failures kept in a database
#                                                     directory replay as ones to shrink
#
# The machines are reached through the scripts the directory REMOTE_AGENT names.
# Discovery pieces of PIECE_SECONDS (60) run PARALLEL at a time (a quarter of the cores) until
# SHRINK_SECONDS (420) before the end of the budget; a piece ends at its deadline by itself
# (TEQ_PROP_UNTIL), and a timeout 400 s after it cuts one that hangs. A failure is shrunk as soon
# as its piece has ended, before a further piece starts, each shrink run bounded by
# SHRINK_SECONDS (the engine's 300 s of shrinking and the replays). scalac is asked about
# every generated program, PROP_PROGRAMS (20) programs to an input and to a run of scala-cli, its
# answers shared by the pieces. PROP_KNOWN names triggers to put back (TEQ_PROP_KNOWN); a failure
# that passes once they are taken out again is reported as that exclusion's. ROTATION lists the
# tests the pieces take in turn; TEQ_PROP_EXPRS, TEQ_PROP_CASES and TEQ_PROP_STEPS pass to
# them. TEQ is the binary (target/release/teq), CAMPAIGN_DIR the directory (out/campaign/<stamp>).
cd "$(dirname "$0")/.." || exit 1
repo=$(pwd)
REMOTE_DIR=out/campaign/remote

stamp() { date -u +%Y%m%d-%H%M%S; }
now() { date +%s; }

remote_start() {
  local seconds=$1 machine=${2:-auto} handle commit knobs="" knob
  for knob in PIECE_SECONDS PARALLEL SHRINK_SECONDS SHRINK_REPLAYS PROP_PROGRAMS PROP_KNOWN ROTATION TEQ_PROP_CASES TEQ_PROP_STEPS TEQ_PROP_EXPRS; do
    [ -z "${!knob}" ] || knobs="$knobs $knob=$(printf %q "${!knob}")"
  done
  handle=$("$REMOTE_AGENT/claim.sh" prop-campaign "$machine") || exit 1
  commit=$(git rev-parse --short HEAD)$(git diff --quiet HEAD -- . 2>/dev/null || echo "+changes")
  "$REMOTE_AGENT/sync.sh" "$handle" "$repo" > /dev/null || { "$REMOTE_AGENT/release.sh" "$handle"; exit 1; }
  # The login shell there points TEQ at the machine's own build; the campaign takes the mirror's.
  # The job alone goes to the background: a list sent there as a whole would hold the run open.
  "$REMOTE_AGENT/run.sh" "$handle" teq -t 60 "rm -rf $REMOTE_DIR; mkdir -p $REMOTE_DIR; \
   $knobs CAMPAIGN_COMMIT=$commit CAMPAIGN_ORIGIN=$(printf %q "$repo") CAMPAIGN_DIR=\$HOME/rw/teq/$REMOTE_DIR TEQ=\$HOME/rw/teq/target/release/teq \
    nohup bash -c 'cargo build --release --quiet 2> $REMOTE_DIR/build.log && tests/prop-campaign.sh here $seconds' \
    > $REMOTE_DIR/campaign.log 2>&1 < /dev/null &" || { "$REMOTE_AGENT/release.sh" "$handle"; exit 1; }
  echo "$handle"
}

remote_status() {
  "$REMOTE_AGENT/run.sh" "$1" teq -t 60 "tail -3 $REMOTE_DIR/progress.txt 2>/dev/null || tail -3 $REMOTE_DIR/campaign.log; \
    if [ -e $REMOTE_DIR/report.md ]; then echo finished; elif ! pgrep -u \$(id -u) -f 'prop-campaign[.]sh here' > /dev/null; then echo stopped; fi"
}

# Fetches the campaign's directory and frees the machine; the status is the campaign's own
# (exit.txt, which `here` writes as it exits), 3 where it has none (it did not end), 1 where the
# fetch or the release fails.
remote_collect() {
  local handle=$1 into=$2 fetched status=1
  fetched=$(mktemp -d "${TMPDIR:-/tmp}/prop-campaign.XXXXXX") || return 1
  if "$REMOTE_AGENT/get.sh" "$handle" teq "$REMOTE_DIR" "$fetched/"; then
    into=${into:-out/campaign/$(sed -n 's/^- stamp: //p' "$fetched/remote/report.md" 2>/dev/null | head -1)}
    [ "$into" != out/campaign/ ] || into=out/campaign/$(stamp)-unfinished
    if mkdir -p "$(dirname "$into")" && rm -rf "$into" && mv "$fetched/remote" "$into"; then
      echo "fetched into $into"
      status=$(cat "$into/exit.txt" 2>/dev/null)
      case $status in 0|1|3) ;; *) echo "the campaign did not end" >&2; status=3 ;; esac
    fi
  else
    echo "the campaign's directory could not be fetched" >&2
  fi
  rm -rf "$fetched"
  "$REMOTE_AGENT/release.sh" "$handle" || status=1
  return "$status"
}

# The engine replays a kept failure without shrinking it when its choices replay exactly: it
# takes the input as already shrunk. Moved to the key's `.secondary` entries, the failure is
# replayed as one to shrink. The layout is hegeltest-c 0.44.1's (the lock pins it):
# `<db>/<fnv1a64("native:" + key)>/<entry>`, the keys listed under the hash of
# "native:.hegel-keys", the secondary key the key with ".secondary" after it. A temporary of a
# save in flight stays where it is, and an entry gone meanwhile is passed over.
requeue() {
  python3 - "$1" <<'EOF'
import os, sys
def fnv(data):
    h = 0xcbf29ce484222325
    for byte in data:
        h = ((h ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return '%016x' % h
root = sys.argv[1]
keys = os.path.join(root, fnv(b'native:.hegel-keys'))
moved = 0
for name in os.listdir(keys) if os.path.isdir(keys) else []:
    key = open(os.path.join(keys, name), 'rb').read()
    if key.endswith(b'.secondary'):
        continue
    primary = os.path.join(root, fnv(b'native:' + key))
    secondary = os.path.join(root, fnv(b'native:' + key + b'.secondary'))
    for entry in os.listdir(primary) if os.path.isdir(primary) else []:
        # A save in flight: the engine writes `<entry>.tmp.<pid>.<n>` and renames it.
        if '.tmp.' in entry:
            continue
        os.makedirs(secondary, exist_ok=True)
        try:
            os.rename(os.path.join(primary, entry), os.path.join(secondary, entry))
            moved += 1
        except FileNotFoundError:
            pass
print(moved)
EOF
}

# The failure to shrink next: of the kind shrunk least so far, the earliest. A kind orders the
# queue and never closes it: kinds are coarse, and two defects can read alike.
next_failure() {
  awk -F'\t' '
    FILENAME == ARGV[1] { done[$2] = 1; count[$1]++; next }
    !($2 in done) && (best == "" || count[$1] + 0 < fewest) { best = $0; fewest = count[$1] + 0 }
    END { if (best != "") print best }' "$dir/shrunk.txt" "$dir/failures.txt"
}

# A run's environment without the triggers asked and without a kind kept, for telling whether a
# failure is a trigger's: a kind kept would pass a failure of another kind.
without_triggers() { grep -v '^export TEQ_PROP_KNOWN=\|^export TEQ_PROP_KEEP=' "$1"; }

# The campaign's status: 1 where something failed (a failure, a target past its bound), 3 where
# nothing did but a piece or a shrink run was cut, 0 otherwise.
campaign_outcome() {
  local run
  if [ -s "$dir/failures.txt" ] || [ -s "$dir/timeouts.txt" ]; then
    echo 1
    return
  fi
  [ ! -s "$dir/cut.txt" ] || { echo 3; return; }
  for run in "$dir"/shrinks/*/exit "$dir"/shrinks/*/*/exit; do
    case $(cat "$run" 2>/dev/null) in 124|137) echo 3; return ;; esac
  done
  echo 0
}

# Sourced with PROP_CAMPAIGN_LIB set, the script defines its functions and stops here
# (tests/prop-scripts.sh).
[ -z "$PROP_CAMPAIGN_LIB" ] || return 0 2> /dev/null

case $1 in
  start | status | collect | nightly)
    [ -x "${REMOTE_AGENT:-}/run.sh" ] || { echo "prop-campaign: set REMOTE_AGENT to the directory of the remote machines' scripts" >&2; exit 2; } ;;
esac
case $1 in
  start) [ -n "$2" ] || { echo "usage: $0 start <seconds> [machine]" >&2; exit 2; }; remote_start "$2" "$3"; exit ;;
  status) [ -n "$2" ] || { echo "usage: $0 status <handle>" >&2; exit 2; }; remote_status "$2"; exit ;;
  collect) [ -n "$2" ] || { echo "usage: $0 collect <handle> [dir]" >&2; exit 2; }; remote_collect "$2" "$3"; exit ;;
  nightly)
    [ -n "$2" ] || { echo "usage: $0 nightly <seconds> [machine]" >&2; exit 2; }
    handle=$(remote_start "$2" "$3") || exit 1
    echo "started on $handle"
    for _ in $(seq 1 $(( $2 / 60 + 30 ))); do
      sleep 60
      state=$(remote_status "$handle" 2>&1 | tail -1)
      [ "$state" = finished ] || [ "$state" = stopped ] && break
    done
    remote_collect "$handle"
    exit ;;
  requeue) [ -d "$2" ] || { echo "usage: $0 requeue <database>" >&2; exit 2; }; requeue "$2"; exit ;;
  here) [ -n "$2" ] || { echo "usage: $0 here <seconds>" >&2; exit 2; } ;;
  *) awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "$0"; exit 2 ;;
esac

budget=$2
started=$(now)
campaign_stamp=$(stamp)
absolute() { case $1 in /*) echo "$1" ;; *) echo "$repo/$1" ;; esac; }
dir=$(absolute "${CAMPAIGN_DIR:-out/campaign/$campaign_stamp}")
TEQ=$(absolute "${TEQ:-target/release/teq}")
piece_seconds=${PIECE_SECONDS:-60}
parallel=${PARALLEL:-$(( $(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4) / 4 ))}
[ "$parallel" -ge 1 ] || parallel=1
shrink_seconds=${SHRINK_SECONDS:-420}
end=$(( started + budget ))
discovery_end=$(( end - shrink_seconds ))
[ $discovery_end -gt $(( started + piece_seconds )) ] || discovery_end=$(( started + piece_seconds ))
rotation=${ROTATION:-targets_agree split_session_agrees_with_fresh order_of_inputs_does_not_matter targets_agree check_session_agrees_with_fresh targets_agree split_session_agrees_with_fresh pilot_split check_session_agrees_with_fresh pilot_check}
mkdir -p "$dir/pieces" "$dir/shrinks" "$dir/findings" || exit 1
trap 'echo $? > "$dir/exit.txt"' EXIT
progress() { echo "$(date -u +%H:%M:%S) $*" | tee -a "$dir/progress.txt"; }
[ -x "$TEQ" ] || { progress "no teq binary at $TEQ"; exit 1; }

cd proptests || exit 1
if ! timeout 60 cargo metadata --offline --locked --format-version 1 > /dev/null 2>&1; then
  timeout 300 cargo fetch --locked > "$dir/fetch.log" 2>&1 || { progress "the crates of proptests/ cannot be fetched (see $dir/fetch.log)"; exit 1; }
fi
if ! timeout 300 cargo test --no-run --offline --locked > "$dir/build.log" 2>&1; then
  progress "the crate does not build (see $dir/build.log)"
  exit 1
fi
# `Executable tests/session.rs (target/debug/deps/session-...)`
binary_of() {
  local file
  case $1 in
    targets_agree|scalac_batched_answers_as_alone) file=targets ;;
    order_of_inputs_does_not_matter) file=order ;;
    *) file=session ;;
  esac
  sed -n "s|^ *Executable tests/$file\.rs (\(.*\))\$|$repo/proptests/\1|p" "$dir/build.log" | head -1
}
for test in $rotation; do
  [ -n "$(binary_of "$test")" ] || { progress "no test binary for $test (see $dir/build.log)"; exit 1; }
done

cat > "$dir/hegel.toml" <<'EOF'
# Discovery reports the first failure unshrunk; a shrink run replays the kept failure and shrinks
# it. Both extend `base`, so that no environment's profile (`ci` disables the database) enters.
[profiles.discover]
extends = "base"
phases = ["explicit", "reuse", "generate", "target"]

[profiles.shrink]
extends = "base"
phases = ["reuse", "shrink"]

[profiles.replay]
extends = "base"
phases = ["reuse"]
EOF

# The environment of a piece, kept as a file that a shrink run and a reader source again.
write_env() {
  local piece=$1 test=$2 seed=$3 until=$4
  {
    echo "export TEQ='$TEQ'"
    echo "export HEGEL_CONFIG='$dir/hegel.toml'"
    echo "export HEGEL_SEED=$seed"
    echo "export HEGEL_TEST_CASES=1000000"
    echo "export TEQ_PROP_SCALAC=always"
    echo "export TEQ_PROP_SCALAC_CACHE='$dir/scalac'"
    echo "export TEQ_PROP_PROGRAMS=${PROP_PROGRAMS:-20}"
    [ -z "$TEQ_PROP_CASES" ] || echo "export TEQ_PROP_CASES=$TEQ_PROP_CASES"
    [ -z "$TEQ_PROP_STEPS" ] || echo "export TEQ_PROP_STEPS=$TEQ_PROP_STEPS"
    [ -z "$TEQ_PROP_EXPRS" ] || echo "export TEQ_PROP_EXPRS=$TEQ_PROP_EXPRS"
    [ -z "$PROP_KNOWN" ] || echo "export TEQ_PROP_KNOWN='$PROP_KNOWN'"
    echo "test=$test"
    echo "binary='$(binary_of "$test")'"
  } > "$piece/env.sh"
}

# Runs a test of a piece or a shrink run in the background, its exit code written when it ends.
launch() {
  local where=$1 profile=$2 bound=$3 until=$4
  (
    . "$where/env.sh"
    export HEGEL_DEFAULT_PROFILE=$profile HEGEL_DATABASE=$where/db TEQ_PROP_WORK=$where/work
    export TEQ_PROP_REPLAYS=${replays:-0}
    [ -z "$until" ] || export TEQ_PROP_UNTIL=$until
    [ -n "$until" ] || unset TEQ_PROP_UNTIL
    timeout -k 10 "$bound" "$binary" "$test" --exact --test-threads=1 > "$where/log.txt" 2>&1
    code=$?
    date +%s > "$where/ended"
    echo $code > "$where/exit"
  ) &
}

running() { local n=0 p; for p in "$dir"/pieces/*/ "$dir"/shrinks/*/ "$dir"/shrinks/*/*/; do [ -d "$p" ] && [ -e "$p/started" ] && [ ! -e "$p/exit" ] && n=$((n + 1)); done; echo $n; }

# The piece's panic: the lines after `panicked at`, up to the engine's reproduction line.
panic_block() {
  case $1 in
    */report.txt) cat "$1" ;;
    *) awk '/panicked at/ && !seen { seen = 1; on = 1; next }
         on && (/^To reproduce this failure/ || /^note: run with/ || /^---- /) { exit }
         on { print }' "$1" ;;
  esac
}

# A failure's signature, for putting the first of each kind first in the shrink queue: the kind
# its report names (`the failure's kind:`, proptests/src/confirm.rs), or, in a report without
# one, the same read from its lines: the targets' disagreements as which targets print alike, a
# refusal by its target and first error line, a session's by the kind of build and the lines of
# what differs, numbers and paths left out. It merges what should stand apart at worst, which
# costs time, not a finding.
kind_in() { panic_block "$1" | sed -n "s/^the failure's kind: //p" | head -1; }

signature() {
  local test=$1 file=$2 parts
  parts=$(kind_in "$file")
  [ -z "$parts" ] || { echo "$test: $parts" | cut -c1-400; return; }
  parts=$(panic_block "$file" | awk '
    function norm(s) { gsub(/\/[^ :)]*/, "<path>", s); gsub(/[0-9]+/, "#", s); return substr(s, 1, 120) }
    function flush(   i, k, keys, members, text) {
      if (group == "") return
      keys = 0
      for (i = 1; i <= ng; i++) {
        for (k = 1; k <= keys; k++) if (value[k] == tv[i]) break
        if (k > keys) { keys = k; value[k] = tv[i]; members[k] = tn[i] } else members[k] = members[k] " " tn[i]
      }
      text = members[1]; for (k = 2; k <= keys; k++) text = text " | " members[k]
      print group ": " text
      group = ""; ng = 0
    }
    /^the history, / || /^the program: / || /^the history.s files: / { flush(); exit }
    /^line [0-9]+\.[fr]\.[vt]: the targets print$/ { flush(); group = $2; sub(/^[0-9]+\.[fr]\./, "", group); sub(/:$/, "", group); next }
    group != "" && /^    [a-z]+: / { t = $1; sub(/:$/, "", t); v = $0; sub(/^    [a-z]+: /, "", v); ng++; tn[ng] = t; tv[ng] = v; next }
    { flush() }
    /^line [0-9]+\.[fr]\.[vt]: on [a-z]+ the folded form prints/ { leg = $2; sub(/^[0-9]+\.[fr]\./, "", leg); sub(/:$/, "", leg); print leg ": folded differs on " $4; next }
    /^line labels of / { t = $4; sub(/:$/, "", t); print "labels differ on " t; next }
    /is refused or ends with an error/ { t = $1; sub(/:$/, "", t); refused = t; next }
    refused != "" {
      m = $0; sub(/^.*error: /, "", m)
      gsub(/value [^ ]+ is not a member of .*/, "value _ is not a member of _", m)
      gsub(/operator [^ ]+ cannot be applied to .*/, "operator _ cannot be applied to _", m)
      gsub(/found .*, required .*/, "found _, required _", m)
      print refused " refuses: " norm(m); exit
    }
    /^scalac refuses the program/ { print "scalac refuses the program"; next }
    /differs from a fresh build$/ { print ($0 ~ /\(incremental/ ? "incremental" : "full") " build differs"; next }
    /^line / || /^ / || /^of [0-9]+ cases:$/ || /of the program.s lines differ/ || /^(folded|at run time): / { next }
    { print norm($0) }
    END { flush() }' | sort -u | head -12 | paste -sd ';' -)
  echo "$test: ${parts:-no message}" | cut -c1-400
}

progress "campaign of $budget s in $dir: $parallel at a time, pieces of $piece_seconds s until $(date -u -r $discovery_end +%H:%M:%S 2>/dev/null || date -u -d @$discovery_end +%H:%M:%S)"
slots=$(echo $rotation | wc -w)
: > "$dir/failures.txt"
: > "$dir/examined.txt"
: > "$dir/shrunk.txt"

# The pieces that ended since the last look: a failure goes to failures.txt with its signature.
examine() {
  local piece test sig
  for piece in "$dir"/pieces/*/; do
    piece=${piece%/}
    [ -e "$piece/exit" ] || continue
    grep -qxF "$piece" "$dir/examined.txt" && continue
    echo "$piece" >> "$dir/examined.txt"
    case $(cat "$piece/exit") in
      0) ;;
      124|137) echo "$piece" >> "$dir/cut.txt"; progress "${piece#$dir/} cut by its bound" ;;
      *)
        test=$(sed -n 's/^test=//p' "$piece/env.sh")
        sig=$(signature "$test" "$piece/log.txt")
        case $sig in
          *": timeout: "*) printf '%s\t%s\n' "$sig" "$piece" >> "$dir/timeouts.txt"; progress "${piece#$dir/} ran past a target's bound: $sig" ;;
          *) printf '%s\t%s\n' "$sig" "$piece" >> "$dir/failures.txt"; progress "${piece#$dir/} failed: $(echo "$sig" | cut -c1-160)" ;;
        esac ;;
    esac
  done
}

start_shrink() {
  local sig=$1 piece=$2 shrink moved kind
  s=$((s + 1))
  shrink=$dir/shrinks/$(printf %03d $s)
  mkdir -p "$shrink"
  cp -R "$piece/db" "$shrink/db" 2> /dev/null
  cp "$piece/env.sh" "$shrink/env.sh"
  # The shrink run keeps to the kind it was given: a failure of another kind counts as a pass.
  kind=$(kind_in "$piece/log.txt")
  [ -z "$kind" ] || echo "export TEQ_PROP_KEEP=$(printf %q "$kind")" >> "$shrink/env.sh"
  printf '%s\n%s\n' "$sig" "$piece" > "$shrink/of.txt"
  printf '%s\t%s\n' "$sig" "$piece" >> "$dir/shrunk.txt"
  moved=$(requeue "$shrink/db")
  [ "$moved" -ge 1 ] 2> /dev/null || progress "the failure of ${piece#$dir/} is not in its database"
  date +%s > "$shrink/started"
  replays=${SHRINK_REPLAYS:-3} launch "$shrink" shrink "$shrink_seconds" ""
}

start_piece() {
  local test piece seed until
  n=$((n + 1))
  test=$(echo $rotation | cut -d' ' -f$(( (n - 1) % slots + 1 )))
  piece=$dir/pieces/$(printf %04d $n)-$test
  mkdir -p "$piece"
  seed=$(od -An -N4 -tu4 /dev/urandom | tr -d ' ')
  until=$(( $(now) + piece_seconds ))
  [ $until -le $discovery_end ] || until=$discovery_end
  write_env "$piece" "$test" "$seed" "$until"
  date +%s > "$piece/started"
  replays=0 launch "$piece" discover $(( piece_seconds + 400 )) "$until"
}

# A failure is shrunk as soon as its piece has ended, before a further piece starts; pieces start
# until SHRINK_SECONDS before the end, so that what the last of them find can be shrunk too.
n=0
s=0
for _ in $(seq 1 $(( budget / 2 + 1200 ))); do
  examine
  while [ "$(running)" -lt "$parallel" ]; do
    next=$(next_failure)
    if [ -n "$next" ] && [ $(( $(now) + shrink_seconds / 2 )) -le "$end" ]; then
      start_shrink "$(echo "$next" | cut -f1)" "$(echo "$next" | cut -f2)"
    elif [ "$(now)" -lt $(( discovery_end - 10 )) ]; then
      start_piece
    else
      break
    fi
  done
  if [ "$(now)" -ge $(( discovery_end - 10 )) ] && [ "$(running)" = 0 ]; then
    examine
    next=$(next_failure)
    [ -n "$next" ] && [ $(( $(now) + shrink_seconds / 2 )) -le "$end" ] || break
  fi
  sleep 2
done
pieces=$n
progress "discovery and shrinking done: $pieces pieces, $(wc -l < "$dir/failures.txt" | tr -d ' ') failing, $(cut -f1 "$dir/failures.txt" | sort -u | wc -l | tr -d ' ') signatures, $s shrink runs"

# A shrunk program of the generated expressions is replayed once more and reduced to the cases
# that fail by themselves (TEQ_PROP_REDUCE), which costs a run of its own and so is not done at
# every failure while shrinking.
for shrink in "$dir"/shrinks/*/; do
  shrink=${shrink%/}
  [ -d "$shrink/db" ] && grep -q '^test=targets_agree$' "$shrink/env.sh" || continue
  case $(cat "$shrink/exit" 2>/dev/null) in 101|124|137) ;; *) continue ;; esac
  for _ in $(seq 1 600); do
    [ "$(running)" -lt "$parallel" ] && break
    sleep 2
  done
  mkdir -p "$shrink/reduced"
  cp -R "$shrink/db" "$shrink/reduced/db"
  { cat "$shrink/env.sh"; echo "export TEQ_PROP_REDUCE=1"; } > "$shrink/reduced/env.sh"
  date +%s > "$shrink/reduced/started"
  launch "$shrink/reduced" replay 300 ""
done
for _ in $(seq 1 360); do
  [ "$(running)" = 0 ] && break
  sleep 2
done

# With triggers asked, a shrunk failure (or the smallest a cut shrink run kept) that passes once
# they are taken out is theirs.
if [ -n "$PROP_KNOWN" ]; then
  for shrink in "$dir"/shrinks/*/; do
    shrink=${shrink%/}
    case $(cat "$shrink/exit" 2>/dev/null) in 101|124|137) ;; *) continue ;; esac
    for _ in $(seq 1 600); do
      [ "$(running)" -lt "$parallel" ] && break
      sleep 2
    done
    mkdir -p "$shrink/unasked"
    cp -R "$shrink/db" "$shrink/unasked/db"
    without_triggers "$shrink/env.sh" > "$shrink/unasked/env.sh"
    date +%s > "$shrink/unasked/started"
    launch "$shrink/unasked" replay 300 ""
  done
  for _ in $(seq 1 360); do
    [ "$(running)" = 0 ] && break
    sleep 2
  done
fi

cd "$repo" || exit 1
count_inputs() {
  cat "$dir"/pieces/*/work/stats/$1.jsonl 2>/dev/null | grep -vc '"outcome":"passed before"'
}
tally() {
  ls "$dir/pieces" | sed 's/^[0-9]*-//' | sort | uniq -c | awk '{ printf "%s%s %s", (NR > 1 ? ", " : ""), $2, $1 }'
}
per_program() {
  cat "$dir"/pieces/*/work/stats/scalac.jsonl 2>/dev/null | awk -F'[:,}]' -v alone="$1" '
    { programs = $2; answered = $4; millis = $6
      if ((alone && programs == 1) || (!alone && programs > 1)) { runs++; n += programs; total += millis } }
    END { if (n) printf "%d runs of %d programs, %.2f s a program", runs, n, total / n / 1000; else printf "none" }'
}
{
  echo "# Property campaign $campaign_stamp"
  echo
  echo "- stamp: $campaign_stamp"
  echo "- tree: ${CAMPAIGN_COMMIT:-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)}; binary: $("$TEQ" --version 2>&1 | head -1)"
  echo "- machine: $(hostname), $(getconf _NPROCESSORS_ONLN 2>/dev/null) cores; load at the end $(uptime | sed 's/.*averages*: //')"
  echo "- budget $budget s from $(date -u -r $started +%H:%M:%SZ 2>/dev/null || date -u -d @$started +%H:%M:%SZ), taken $(( $(now) - started )) s; discovery $pieces pieces of $piece_seconds s, $parallel at a time: $(tally)"
  echo "- inputs run (not counting the engine's repeats): session $(( $(count_inputs split_session_agrees_with_fresh) + $(count_inputs check_session_agrees_with_fresh) + $(count_inputs pilot_split) + $(count_inputs pilot_check) )) histories, targets $(count_inputs targets_agree) programs, order $(count_inputs order) arrangements"
  echo "- scalac: batched $(per_program 0); alone $(per_program 1)"
  echo "- triggers asked: ${PROP_KNOWN:-none}"
  echo "- failing pieces: $(wc -l < "$dir/failures.txt" | tr -d ' '), signatures $(cut -f1 "$dir/failures.txt" | sort -u | wc -l | tr -d ' '), shrink runs $s; pieces cut by their bound: $(cat "$dir/cut.txt" 2>/dev/null | wc -l | tr -d ' ')"
  echo
  echo "## Findings"
  f=0
  for shrink in "$dir"/shrinks/*/; do
    shrink=${shrink%/}
    [ -d "$shrink" ] || continue
    f=$((f + 1))
    sig=$(sed -n 1p "$shrink/of.txt"); piece=$(sed -n 2p "$shrink/of.txt")
    test=$(sed -n 's/^test=//p' "$shrink/env.sh")
    seed=$(sed -n 's/^export HEGEL_SEED=//p' "$shrink/env.sh")
    code=$(cat "$shrink/exit" 2>/dev/null)
    echo
    echo "### $f. $sig"
    echo
    echo "- test \`$test\`, seed $seed, piece \`${piece#$dir/}\`, which failed $(( $(cat "$piece/ended") - $(cat "$piece/started") )) s after it started"
    case $code in
      101) state="shrunk" ;;
      0) state="did not fail again when replayed (flaky: see the piece's log)" ;;
      124|137) state="cut by its bound of $shrink_seconds s while shrinking; the smallest failure it kept is shown" ;;
      *) state="the shrink run ended with $code" ;;
    esac
    echo "- shrink run \`${shrink#$dir/}\`: $state"
    if [ -n "$PROP_KNOWN" ]; then
      case $(cat "$shrink/unasked/exit" 2>/dev/null) in
        0) echo "- a known exclusion: passes with the triggers taken out again ($PROP_KNOWN)" ;;
        101) echo "- new: fails with the triggers of $PROP_KNOWN taken out" ;;
        *) echo "- known exclusion: not decided" ;;
      esac
    else
      echo "- new: no trigger was asked (triage decides whether it is a known defect's shape the exclusion misses)"
    fi
    # The reduced replay's report where there is one; a cut shrink run's smallest failure so far
    # is the last one it kept.
    log=$shrink/log.txt
    if [ "$(cat "$shrink/reduced/exit" 2>/dev/null)" = 101 ]; then
      log=$shrink/reduced/log.txt
    elif [ "$code" != 101 ]; then
      log=$(ls -t "$shrink"/work/*/failures/*/report.txt 2>/dev/null | head -1)
      [ -n "$log" ] || log=$piece/log.txt
    fi
    kind=$(kind_in "$log")
    found=$(kind_in "$piece/log.txt")
    [ -z "$kind" ] || [ "$kind" = "$found" ] || echo "- the shrunk failure's kind is not the one found (\`$found\`): \`$kind\`"
    kept=$(panic_block "$log" | sed -n 's/^the program: //p; s/^the history.s files: //p' | tail -1)
    case $kept in */cases.scala) kept=$(dirname "$kept") ;; esac
    if [ -n "$kept" ] && [ -e "$kept" ]; then
      mkdir -p "$dir/findings/$f"
      cp -R "$kept" "$dir/findings/$f/"
      echo "- kept: \`findings/$f/$(basename "$kept")\` ($(ls "$kept" | tr '\n' ' ' | sed 's/ $//'))"
    fi
    mkdir -p "$dir/findings/$f"
    grep -o '#\[hegel::reproduce_failure("[^"]*")\]' "$log" | tail -1 > "$dir/findings/$f/reproduce.txt"
    [ -s "$dir/findings/$f/reproduce.txt" ] && echo "- to replay: the attribute in \`findings/$f/reproduce.txt\` on the test, with the variables of \`${shrink#$dir/}/env.sh\`"
    echo
    echo '```'
    panic_block "$log" | head -${REPORT_LINES:-60} | cut -c1-300
    echo '```'
  done
  [ $f -gt 0 ] || printf '\nNone.\n'
  if [ -s "$dir/failures.txt" ]; then
    echo
    echo "## Every failing piece, by signature"
    echo
    sort "$dir/failures.txt" | awk -F'\t' -v dir="$dir/" '{ sub(dir, "", $2); print "- " $1 ": `" $2 "`" }'
  fi
  if [ -s "$dir/timeouts.txt" ]; then
    echo
    echo "## Programs a target did not finish in its bound (not shrunk: every attempt waits the bound out)"
    echo
    awk -F'\t' -v dir="$dir/" '{ sub(dir, "", $2); print "- " $1 ": `" $2 "`" }' "$dir/timeouts.txt"
  fi
  if [ -s "$dir/cut.txt" ]; then
    echo
    echo "## Pieces cut by their bound, incomplete rather than failing (a hang, or an input that ran long; the piece's log and work directory keep what it did)"
    echo
    sed "s|^$dir/|- |" "$dir/cut.txt"
  fi
  echo
  echo "## Nightly"
  echo
  echo "From the reference machine, for a remote machine, with \`crontab -e\`:"
  echo
  echo '```'
  echo "0 2 * * * cd ${CAMPAIGN_ORIGIN:-$repo} && tests/prop-campaign.sh nightly $budget >> out/campaign/nightly.log 2>&1"
  echo '```'
  echo
  echo "The job holds a machine's claim for the campaign's length and needs ssh to the machines without"
  echo "a prompt; \`tests/prop-campaign.sh here $budget\` runs one where it stands."
} > "$dir/report.md"
rm -rf "$dir/scalac"
outcome=$(campaign_outcome)
progress "report: $dir/report.md; $(case $outcome in 0) echo "nothing failed" ;; 1) echo "failures found" ;; *) echo "incomplete: a piece or a shrink run was cut" ;; esac) (exit $outcome)"
exit "$outcome"
