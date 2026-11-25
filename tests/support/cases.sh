# Sourced by the runners over a directory of programs (tests/run.sh, tests/run_interp.sh, tests/run_jvm.sh,
# tests/run_stdlib.sh and tests/split.sh over tests/cases; tests/run_errors.sh over tests/errors): a run over named
# programs alone, for tests/new-cases.sh, which runs every suite on a part's new cases.
#   case_sources <dir> <variable>   the programs of <dir> (<name>.scala, <name>/) in the glob's order, or, where the
#                                   variable (TEQ_CASES for tests/cases, TEQ_ERRORS for tests/errors) names some
#                                   (separated by spaces or new lines), those alone
#   cases_filter_begin <suite> <dir> <variable> <the runner's arguments>...
#                                   before the run: unfiltered, nothing; filtered, --update, --add and --write are refused and
#                                   a name <dir> has no program of is an error (each exit 2, before anything runs), and
#                                   CASES_FILTERED, CASES_TAKEN and CASES_TOTAL are set (CASES_FILTERED is empty
#                                   unfiltered)
#   cases_judged <results>          filtered: every program the variable names has a line in the run's results
#                                   (`<verdict> <name>`, the name bare or `<dir's last part>/<name>`; or the programs
#                                   the suite takes, before the run), else those it takes no test of are said on the
#                                   summary line and it returns 1: a suite whose own rules leave a program out never
#                                   reads "0 failed" over it
#   cases_line <line>               the runner's summary line: filtered, `<suite>: <taken> of <total> by <variable>: <line>`,
#                                   so that a filtered run never reads as the suite's line
#   cases_skipped <what>            a check the runner makes on a whole run alone, said as not run under the filter
# A filtered run holds its list as a piece of the suite does (tests/support/pending.py --partial): the board's rows
# of the programs it did not run stand. tests/gate.sh fails a line whose environment sets either variable.
CASES_FILTERED=
CASES_TAKEN=
CASES_TOTAL=
CASES_SUITE=
CASES_VARIABLE=
CASES_DIR=
CASES_UNJUDGED=
CASES_NONE=
case_sources() {
  local dir=$1 variable=$2 src name only
  only=${!variable:-}
  [ -n "$only" ] && only=" $(echo $only) "
  for src in "$dir"/*.scala "$dir"/*/; do
    [ -e "$src" ] || continue
    src=${src%/}
    if [ -n "$only" ]; then
      name=$(basename "$src" .scala)
      case $only in *" $name "*) ;; *) continue ;; esac
    fi
    echo "$src"
  done
}
cases_filter_begin() {
  local suite=$1 dir=$2 variable=$3 arg name missing=
  shift 3
  CASES_SUITE=$suite
  CASES_VARIABLE=$variable
  CASES_DIR=$dir
  [ -n "${!variable:-}" ] || return 0
  for arg in "$@"; do
    case $arg in
      --update | --add | --write)
        echo "$suite: $arg refused under $variable: a filtered run writes no list"
        exit 2 ;;
    esac
  done
  for name in ${!variable}; do
    [ -e "$dir/$name.scala" ] || [ -d "$dir/$name" ] || missing="$missing $name"
  done
  if [ -n "$missing" ]; then
    echo "$suite: $variable names no program of $dir:$missing"
    exit 2
  fi
  CASES_FILTERED=1
  CASES_TAKEN=$(case_sources "$dir" "$variable" | wc -l)
  CASES_TOTAL=$(case_sources "$dir" CASES_NONE | wc -l)
}
cases_judged() {
  local name judged
  [ -n "$CASES_FILTERED" ] || return 0
  judged=" $(awk '{ print $2 }' "$1" 2> /dev/null | tr '\n' ' ') "
  for name in ${!CASES_VARIABLE}; do
    [[ $judged == *" $name "* || $judged == *" ${CASES_DIR##*/}/$name "* ]] || CASES_UNJUDGED="$CASES_UNJUDGED $name"
  done
  [ -z "$CASES_UNJUDGED" ]
}
cases_line() {
  if [ -n "$CASES_FILTERED" ]; then
    echo "$CASES_SUITE: $CASES_TAKEN of $CASES_TOTAL by $CASES_VARIABLE${CASES_UNJUDGED:+, this suite taking no test of$CASES_UNJUDGED}: $*"
  else
    echo "$*"
  fi
}
cases_skipped() {
  [ -z "$CASES_FILTERED" ] || echo "$CASES_SUITE: not run under $CASES_VARIABLE (a whole run's check): $*"
}
