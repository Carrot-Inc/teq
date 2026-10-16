# Sourced from the repository's root by the long suites (tests/lsp.sh, tests/jvm-watch.sh, tests/modules.sh,
# tests/split.sh), which the gate runs as pieces on several machines (docs/DEVELOPING.md, "The landing gate"). A suite
# defines `suite_units`, which prints every unit it runs (a program with its checks, a scenario, a session) one a
# line in its order, the unit's name followed by the sites its body runs at when it runs at more than one (`-` the
# one site of a unit otherwise), and calls `pieces_option <suite> <its first argument> <its second>` before any
# setup:
#   --pieces <n>   prints the n pieces' units (tests/support/pieces.py list) and exits without running anything;
#   --piece <k>/<n>  runs piece k alone, after a line naming its units: the one place that decides whether a unit
#                  runs asks `piece_has <unit>`, true for the piece's units alone (another place that runs more of
#                  the unit `piece_holds <unit>`), each body of a unit begins with `piece_ran <unit> [<site>]`, and
#                  the suite ends with `pieces_end <exit> <field>...`, which fails the piece unless every unit the
#                  suite lists was asked for exactly once and every site of every unit of the piece ran exactly once,
#                  nothing else having run, then prints the piece's summary as its last line (tests/support/pieces.py
#                  has its fields) and exits;
# and returns 0 when it took the two arguments, which the suite then shifts; another option-shaped argument the suite
# refuses with `pieces_refuse`. A selector out of range or misspelled exits 2 before any setup; a piece with no units
# says so, prints its summary and exits 0. Without --piece the suite runs whole and unchanged, `piece_has` and
# `piece_holds` always true and `pieces_end` writing the summary only to the file PIECES_SUMMARY names, if any. A
# body's stamp, `<unit> <site>`, goes to a file of the piece's own (PIECES_RAN, which the suite's workers and the
# language server's driver inherit), appended at the end to the file PIECES_STAMPS names, if any, and in a whole run
# to that file alone: the execution identities tests/gate-scripts.sh holds to the units for every number of pieces.
# Under PIECES_STAMP_ONLY=1 `piece_ran` is false, and a body written `piece_ran <unit> || return` (or `|| continue`)
# ends at its stamp: the many programs' bodies, cheap to count where running them all for every n is not.
# Under PIECES_DRY=1 a piece runs no unit (`piece_has` false for every one), and `pieces_end` holds that every unit
# of the suite was asked for once and none ran.
PIECE=
PIECE_N=1
PIECE_UNITS=
PIECES_ALL=
PIECES_ASKED=
PIECES_SUITE=
export PIECES_RAN=${PIECES_RAN:-$PIECES_STAMPS}
pieces_usage() {
  echo "usage: tests/$PIECES_SUITE.sh [--piece <k>/<n> | --pieces <n>], 1 <= k <= n" >&2
  exit 2
}
# pieces_refuse <suite> <argument>...: the usage, for an option-shaped argument the suite does not take.
pieces_refuse() {
  local a
  PIECES_SUITE=$1
  shift
  for a in "$@"; do
    case $a in -*) echo "tests/$PIECES_SUITE.sh: no option $a" >&2; pieces_usage ;; esac
  done
}
pieces_option() {
  PIECES_SUITE=$1
  case $2 in
    --pieces)
      [[ $3 =~ ^[1-9][0-9]*$ ]] || pieces_usage
      suite_units | cut -d' ' -f1 | python3 tests/support/pieces.py list "$1" "$3" || exit 2
      exit 0
      ;;
    --piece)
      [[ $3 =~ ^([1-9][0-9]*)/([1-9][0-9]*)$ ]] && [ "${BASH_REMATCH[1]}" -le "${BASH_REMATCH[2]}" ] || pieces_usage
      PIECE=$3
      PIECE_N=${3#*/}
      PIECES_ALL=$(suite_units)
      PIECE_UNITS=$'\n'$(printf '%s\n' "$PIECES_ALL" | cut -d' ' -f1 | python3 tests/support/pieces.py select "$1" "${3%/*}" "$PIECE_N")$'\n' || exit 2
      local units
      read -r -d '' -a units <<< "$PIECE_UNITS"
      echo "pieces: $1 $PIECE: ${#units[@]} units:" "${units[@]}"
      [ ${#units[@]} -gt 0 ] || { echo "pieces: $1 $PIECE summary: exit=0 units=0"; exit 0; }
      PIECES_RAN=$(mktemp "${TMPDIR:-/tmp}/pieces-ran.XXXXXX") || exit 2
      return 0
      ;;
  esac
  return 1
}
piece_has() {
  [ -n "$PIECE" ] || return 0
  PIECES_ASKED=$PIECES_ASKED$1$'\n'
  [ -z "$PIECES_DRY" ] || return 1
  [[ $PIECE_UNITS == *$'\n'"$1"$'\n'* ]]
}
piece_holds() {
  [ -n "$PIECE" ] || return 0
  [ -z "$PIECES_DRY" ] || return 1
  [[ $PIECE_UNITS == *$'\n'"$1"$'\n'* ]]
}
piece_ran() {
  [ -z "$PIECES_RAN" ] || echo "$1 ${2:--}" >> "$PIECES_RAN"
  [ -z "$PIECES_STAMP_ONLY" ]
}
pieces_end() {
  local code=$1 problems units
  shift
  if [ -z "$PIECE" ]; then
    [ -z "$PIECES_SUMMARY" ] || echo "pieces: $PIECES_SUITE whole summary: exit=$code units=$(suite_units | wc -l | tr -d ' ') $*" > "$PIECES_SUMMARY"
    return 0
  fi
  problems=$(printf '%s\f%s\f%s\f%s' "$PIECES_ALL" "$PIECE_UNITS" "$PIECES_ASKED" "$(cat "$PIECES_RAN")" | PIECES_DRY=$PIECES_DRY python3 -c '
import collections, os, sys
listed, mine, asked, ran = sys.stdin.read().split("\f")
sites = {l.split()[0]: l.split()[1:] or ["-"] for l in listed.splitlines() if l.strip()}
asked = collections.Counter(asked.split())
ran = collections.Counter(tuple(l.split()) for l in ran.splitlines() if l.strip())
due = collections.Counter() if os.environ["PIECES_DRY"] else collections.Counter((u, s) for u in mine.split() for s in sites[u])
for unit in sites:
    if asked[unit] != 1:
        print(f"FAIL pieces: the unit {unit} was asked for {asked[unit]} times, not once")
for unit in sorted(set(asked) - set(sites)):
    print(f"FAIL pieces: a unit {unit} was asked for that the suite does not list")
at = lambda site: "" if site == "-" else f" at {site}"
for (unit, site) in sorted(due - ran):
    print(f"FAIL pieces: the unit {unit} never ran{at(site)}")
for (unit, site) in sorted(ran - due):
    print(f"FAIL pieces: the unit {unit} ran {ran[(unit, site)]} times{at(site)}, {due[(unit, site)]} due")')
  if [ -n "$problems" ]; then
    echo "$problems"
    code=1
  fi
  [ -z "$PIECES_STAMPS" ] || cat "$PIECES_RAN" >> "$PIECES_STAMPS"
  rm -f "$PIECES_RAN"
  read -r -d '' -a units <<< "$PIECE_UNITS"
  echo "pieces: $PIECES_SUITE $PIECE summary: exit=$code units=${#units[@]} $*"
  exit "$code"
}
