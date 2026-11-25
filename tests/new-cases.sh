#!/bin/bash
# tests/new-cases.sh <base> [--plan]: every suite of the gate that makes tests out of the programs a change adds over
# <base> (tests/cases, tests/errors, tests/modules), on those alone where the suite can be given names and whole where
# it cannot, never with --update, each held to its lists; an owner runs it before a push. First the board's names
# side (tests/support/board_names.py: a list lacking a row fails a run). Then a line a suite, `<suite>: <its last
# line>` or `<suite>: not reached (<the rule>)`. Last, a line naming all fifteen suites, run or not reached and why.
# The plan is tests/support/new_cases.py's, which names the fifteen and their rules; --plan prints it and runs
# nothing. Exit 0 when every run suite passed and every list has its rows, 1 when not, 2 for a base it cannot read.
# Each run is bounded, killed 10 s past its bound if it ignores the bound's signal (a bound's kill is its exit
# status); TEQ names the binary as for the suites. A suite is red on an exit status other than 0 and on a run that
# printed no line.
cd "$(dirname "$0")/.." || exit 2
base=${1:-}
plan_only=
[ "${2:-}" = --plan ] && plan_only=1
if [ -z "$base" ] || [ $# -gt 2 ] || { [ $# = 2 ] && [ -z "$plan_only" ]; }; then
  echo "usage: tests/new-cases.sh <base> [--plan]" >&2
  exit 2
fi
git rev-parse -q --verify "$base^{commit}" > /dev/null || { echo "new-cases: $base names no commit"; exit 2; }
plan=$(python3 -I -B tests/support/new_cases.py "$base") || exit 2
echo "new-cases: $(head -1 <<< "$plan" | cut -f2)"
status=0
names=$(timeout -k 10 300 python3 -I -B tests/support/board_names.py . 2>&1)
lacking=$(tail -1 <<< "$names" | sed 's/^need a run with --update: //')
if [ "$lacking" = none ]; then
  echo "board rows: every list over the tree's programs has their rows"
else
  grep -v -E ' 0 with no row|^need a run' <<< "$names" | sed 's/^/board rows: /'
  status=1
fi
out=out/new-cases
mkdir -p "$out"
ran=() reached_not=() red=()
while IFS=$'\t' read -r -u 3 suite how what; do
  [ "$suite" = "#" ] && continue
  if [ "$how" = none ]; then
    echo "$suite: not reached ($what)"
    reached_not+=("$suite ($what)")
    continue
  fi
  if [ -n "$plan_only" ]; then
    echo "$suite: would run $how${what:+: $what}"
    ran+=("$suite")
    continue
  fi
  log=$out/$suite.log
  case $suite in
    cases) TEQ_CASES=$what timeout -k 10 3600 tests/run.sh > "$log" 2>&1 ;;
    interp) TEQ_CASES=$what timeout -k 10 3600 tests/run_interp.sh > "$log" 2>&1 ;;
    jvm) TEQ_CASES=$what timeout -k 10 3600 tests/run_jvm.sh > "$log" 2>&1 ;;
    jvm-21) TEQ_CASES=$what timeout -k 10 3600 tests/run_jvm.sh --java-output-version=21 > "$log" 2>&1 ;;
    stdlib) TEQ_CASES=$what timeout -k 10 3600 tests/run_stdlib.sh > "$log" 2>&1 ;;
    split) TEQ_CASES=$what timeout -k 10 3600 tests/split.sh > "$log" 2>&1 ;;
    errors) TEQ_ERRORS=$what timeout -k 10 3600 tests/run_errors.sh > "$log" 2>&1 ;;
    pairs-twin) TEQ_CASES=$(sed 's|cases/||g; s|,| |g' <<< "$what") timeout -k 10 3600 tests/pairs-twin.sh > "$log" 2>&1 ;;
    modules) timeout -k 10 3600 tests/modules.sh $what > "$log" 2>&1 ;;
    tasty-exec) timeout -k 10 3600 ./teq interp tests/tasty-exec.scala -- $what > "$log" 2>&1 ;;
    workers | split-watch | jvm-watch | analysis-bytes) timeout -k 10 3600 "tests/$suite.sh" > "$log" 2>&1 ;;
  esac < /dev/null
  code=$?
  # a suite's verdict is its exit status and its own last line together: a run that printed nothing failed
  last=$(grep -v '^[[:space:]]*$' "$log" | tail -1 | cut -c1-300)
  echo "$suite: ${last:-no line of its own}$([ $code = 0 ] || echo " (exit $code)")"
  ran+=("$suite")
  [ $code = 0 ] && [ -n "$last" ] || { red+=("$suite"); status=1; }
done 3< <(tail -n +2 <<< "$plan")
# Every suite of the plan said, run or not reached: a plan that lost one fails the command.
if [ $((${#ran[@]} + ${#reached_not[@]})) != 15 ]; then
  echo "new-cases: the plan named $((${#ran[@]} + ${#reached_not[@]})) suites, not fifteen"
  status=1
fi
summary="new-cases: $([ -n "$plan_only" ] && echo "would run" || echo "ran") ${#ran[@]}: ${ran[*]:-none}; not reached by rule ${#reached_not[@]}: $(IFS=';'; echo "${reached_not[*]:-none}" | sed 's/;/; /g')"
[ -z "$plan_only" ] && [ ${#red[@]} -gt 0 ] && summary="$summary; red: ${red[*]}"
echo "$summary"
exit $status
