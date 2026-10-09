#!/bin/bash
# The cost of macro expansion on the four programs of bench/macros/gen.py: each is typed with
# --profile RUNS times (default 3) and the best run's type phase, the macro expansions' self
# time and the time per expansion are printed, with the top of the histogram of the best run
# when VERBOSE=1. TEQ names the binary (default target/release/teq).
#
#   bench/macros/run.sh                 all four programs
#   bench/macros/run.sh validate        one of them
cd "$(dirname "$0")/../.."
TEQ=${TEQ:-./target/release/teq}
runs=${RUNS:-3}
work=out/bench/macros
mkdir -p "$work"
if [ ! -f "$work/validate/Macros.scala" ] || [ bench/macros/gen.py -nt "$work/validate/Macros.scala" ]; then
  python3 bench/macros/gen.py "$work" > /dev/null
fi
programs=${*:-validate derive quotes cls}
printf '%-10s %10s %14s %12s %12s\n' program "type" "expansions" "self" "per site"
for p in $programs; do
  best=""
  for _ in $(seq "$runs"); do
    out=$(timeout 300 "$TEQ" compiler build "$work/$p" -o "$work/$p.mjs" --profile 2>&1)
    if grep -q " error: " <<< "$out"; then
      echo "$p: errors"
      grep " error: " <<< "$out" | head -3
      continue 2
    fi
    type_ms=$(sed -n 's/^profile: type phase \([0-9.]*\) ms.*/\1/p' <<< "$out")
    line=$(grep '^  macro expansion  ' <<< "$out")
    count=$(awk '{print $3}' <<< "$line")
    self=$(awk '{print $4}' <<< "$line")
    unit=$(awk '{print $5}' <<< "$line")
    [ "$unit" = "ns" ] && self=$(awk -v n="$self" 'BEGIN { printf "%.3f", n / 1e6 }')
    if [ -z "$best" ] || awk -v a="$self" -v b="$best_self" 'BEGIN { exit !(a < b) }'; then
      best="$out"
      best_self=$self
      best_type=$type_ms
      best_count=$count
    fi
  done
  per=$(awk -v s="$best_self" -v n="$best_count" 'BEGIN { printf "%.1f us", s * 1000 / n }')
  printf '%-10s %8s ms %14s %9s ms %12s\n' "$p" "$best_type" "$best_count" "$best_self" "$per"
  if [ -n "$VERBOSE" ]; then
    grep -A 14 '^macro expansion histogram' <<< "$best" | sed 's/^/    /'
  fi
done
