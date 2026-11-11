# The programs of tests/cases a suite runs (tests/run.sh, tests/run_interp.sh, tests/run_jvm.sh), one a line in the glob's
# order: every tests/cases/*.scala and tests/cases/*/, or, when TEQ_CASES names some (separated by spaces or new lines),
# those alone. A gate runs the few cases a batch broke this way, on each branch of the batch or a lane's own build.
case_sources() {
  local src name only
  [ -n "$TEQ_CASES" ] && only=" $(echo $TEQ_CASES) "
  for src in tests/cases/*.scala tests/cases/*/; do
    [ -e "$src" ] || continue
    src=${src%/}
    [ -e "$src" ] || continue
    if [ -n "$only" ]; then
      name=$(basename "$src" .scala)
      case $only in *" $name "*) ;; *) continue ;; esac
    fi
    echo "$src"
  done
}
