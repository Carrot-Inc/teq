# Sourced from the repository's root by a runner whose tests are held to the list of those expected to fail
# (tests/pending/<suite>.txt, tests/support/pending.py):
#   pending_begin <suite> <a directory of the run's own> "$@"   before the tests; reads --update and --add from
#                                                              the runner's arguments
#   pending_listed <test>     whether the test is on the list (a runner prints no difference of a listed one)
#   pending_pass <test>, pending_fail <test> <why>, pending_skip <test> <why>   each test's outcome, once
#   pending_end               after the tests: the run held to the list, its status (PENDING_PARTIAL=1 for one
#                             piece of a suite: the board's list keeps the other pieces' rows)
PENDING_SUITE=
PENDING_RESULTS=
PENDING_FLAGS=
declare -A PENDING_LISTED
pending_begin() {
  local arg name
  PENDING_SUITE=$1
  PENDING_RESULTS=$2/pending-results.txt
  shift 2
  PENDING_FLAGS=
  for arg in "$@"; do
    case $arg in --update | --add) PENDING_FLAGS="$PENDING_FLAGS $arg" ;; esac
  done
  mkdir -p "$(dirname "$PENDING_RESULTS")"
  : > "$PENDING_RESULTS"
  PENDING_LISTED=()
  while IFS=$'\t' read -r name _; do [ -n "$name" ] && PENDING_LISTED[$name]=1; done < <(cat "tests/pending/$PENDING_SUITE.txt" 2> /dev/null)
}
pending_listed() { [ -n "${PENDING_LISTED[$1]:-}" ]; }
pending_pass() { printf 'pass %s\n' "$1" >> "$PENDING_RESULTS"; }
pending_fail() { printf 'fail %s\t%s\n' "$1" "$(printf '%s' "$2" | tr '\t\n' '  ' | cut -c1-200)" >> "$PENDING_RESULTS"; }
pending_skip() { printf 'skip %s\t%s\n' "$1" "$(printf '%s' "$2" | tr '\t\n' '  ' | cut -c1-200)" >> "$PENDING_RESULTS"; }
pending_end() { python3 tests/support/pending.py "$PENDING_SUITE" "$PENDING_RESULTS" ${PENDING_PARTIAL:+--partial} $PENDING_FLAGS; }

# For a suite of checks that names each failure on a line `FAIL <check>[: <why>]` and counts its passes without
# naming them, the checks expected to fail are listed by name too (the text before the first `: `):
#   pending_fails_begin <suite> <a directory of the run's own> "$@"   before the checks; reads --pending-update and
#                                                                    --pending-add (the suite's own --update is
#                                                                    its expectations')
#   said_fail "<check>[: <why>]"   in the place of `echo "FAIL <check>[: <why>]"`
#   pending_fails_end <failures> [<passes>]   after the checks, with the suite's counts: the checks that failed
#                                  held to the list, a listed check that named no failure counted as passing now;
#                                  a failure the suite counted without a line fails the run
PENDING_FAILS=0
pending_fails_begin() {
  local arg flags=() suite=$1 dir=$2
  shift 2
  for arg in "$@"; do
    case $arg in
      --pending-update) flags+=(--update) ;;
      --pending-add) flags+=(--add) ;;
    esac
  done
  pending_begin "$suite" "$dir" "${flags[@]}"
  PENDING_FAILS=0
}
said_fail() {
  local name=${1%%: *} why=
  [ "$name" = "$1" ] || why=${1#*: }
  echo "FAIL $1"
  PENDING_FAILS=$((PENDING_FAILS + 1))
  pending_fail "$name" "$why"
}
pending_fails_end() {
  local status=0
  python3 tests/support/pending.py "$PENDING_SUITE" "$PENDING_RESULTS" --fails-only ${2:+--passed=$2} $PENDING_FLAGS || status=1
  if [ "${1:-0}" -gt "$PENDING_FAILS" ]; then
    echo "$PENDING_SUITE: $1 failures counted, $PENDING_FAILS named on a FAIL line: a failure without its line"
    status=1
  fi
  return $status
}
# said_fails_of <check> <text>: the failures a tool of the suite named itself on `FAIL` lines of its output, each
# as said_fail takes it; where it named none, the check with the output's first line.
said_fails_of() {
  local line any=0
  while IFS= read -r line; do
    said_fail "${line#FAIL }"
    any=1
  done < <(grep '^FAIL ' <<< "$2")
  [ $any = 1 ] || said_fail "$1: $(grep -m1 . <<< "$2")"
}
