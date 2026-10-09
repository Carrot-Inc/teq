#!/bin/bash
# The capture's completeness census over the test corpora: every
# program of tests/cases and tests/split built in the product mode, a Scala.js module's check
# build (`teq compiler check --products`) or, for a program that asks for the JVM target, the JVM
# product build, with TEQ_CAPTURE_CENSUS set; the census of every build summed by
# tests/support/capture-census.sh, whose exit is this script's. A program the product mode does
# not build (one the interpreter alone runs, one whose jars are not in the coursier cache, one
# that fails) is listed and left out; one the writer refuses after the typing (a shape section
# 10 cannot state) is listed and counted, its census being written when the typing ends.
# --bench takes the budget's programs instead (bench/programs.sh: the application corpus's
# frontend and API, the core corpus, the feature variants, the macro and derivation programs),
# each checked with --products and the API also built for the JVM with --products.
# --workers [<counts>] builds every program of tests/split, tests/workers and the directories of
# tests/tasty/capture at one worker and at each count given (`1,4` unless given, `2,16` for the
# parallel typer's records) and compares their censuses, which the merge of the workers' records
# has to leave the same; a program whose build fails is left out, save a fixture of
# tests/tasty/capture, which fails, as does one whose records the merge's check finds a worker's
# id in and a comparison of no program. The script fails at once
# without a temporary directory to work in.
# CENSUS_OUT keeps the census file.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
tmp=$(mktemp -d "${TMPDIR:-/tmp}/capture-census.XXXXXX" 2> /dev/null) && [ -d "$tmp" ] && [ -w "$tmp" ] || { echo "capture-census: no temporary directory to work in" >&2; exit 1; }
trap 'rm -rf "$tmp"' EXIT
census=${CENSUS_OUT:-$tmp/census.txt}
: > "$census" || { echo "capture-census: cannot write $census" >&2; exit 1; }
built=0
left=()
refused=()
if [ "$1" = --workers ]; then
  counts=4
  [[ ${2:-} =~ ^[0-9]+(,[0-9]+)*$ ]] && counts=${2//,/ }
  counts=$(for t in $counts; do [ "$t" != 1 ] && echo "$t"; done | tr '\n' ' ')
  same=0
  differ=0
  for src in tests/split/*/ tests/workers/*/ tests/tasty/capture/*/; do
    src=${src%/}
    [ -e "$src" ] || continue
    name=$(basename "$src")
    for t in 1 $counts; do
      rm -rf "$tmp/products" "$tmp/census.$t"
      TEQ_CAPTURE_CENSUS=$tmp/census.$t timeout 120 "$TEQ" compiler check --products "$tmp/products" "$src" --threads $t > "$tmp/log.$t" 2>&1
    done
    if [ ! -s "$tmp/census.1" ]; then
      case $src in
        tests/tasty/capture/*)
          echo "FAIL $name: the fixture gave no census"
          differ=$((differ + 1))
          ;;
        *) left+=("$name") ;;
      esac
      continue
    fi
    for t in $counts; do
      if grep -q 'the merge left' "$tmp/log.$t"; then
        echo "FAIL $name at $t workers: $(grep -m1 'the merge left' "$tmp/log.$t" | cut -c1-200)"
        differ=$((differ + 1))
      elif cmp -s "$tmp/census.1" "$tmp/census.$t"; then
        same=$((same + 1))
      else
        echo "FAIL $name: the census differs at $t workers"
        diff "$tmp/census.1" "$tmp/census.$t" | head -5
        differ=$((differ + 1))
      fi
    done
  done
  echo "$same comparisons of a program's census at one worker and at $(echo $counts | sed 's/ /, /g') the same, $differ differ, ${#left[@]} programs left out"
  [ $differ = 0 ] && [ $same -gt 0 ]
  exit
fi
if [ "$1" = --bench ]; then
  work=out/budget
  . bench/programs.sh || { echo "capture-census: the programs could not be generated"; exit 1; }
  for prog in $programs; do
    args=$(program_args "$prog")
    modes=("compiler check")
    [ "$prog" = realistic-api ] && modes+=("compiler build --target jvm")
    for mode in "${modes[@]}"; do
      rm -rf "$tmp/products"
      if TEQ_CAPTURE_CENSUS=$census timeout 300 "$TEQ" $mode --products "$tmp/products" $args > "$tmp/log" 2>&1; then
        built=$((built + 1))
      else
        left+=("$prog $mode (fails: $(grep -m1 -E 'error|cannot' "$tmp/log" | cut -c1-100))")
      fi
    done
  done
  echo "$built builds of the budget's programs in the product mode, ${#left[@]} failed"
  for l in "${left[@]}"; do echo "  failed: $l"; done
  [ -n "$left_out" ] && echo "left out for want of jars: $left_out"
  tests/support/capture-census.sh "$census"
  exit
fi
for src in tests/cases/*.scala tests/cases/*/ tests/split/*/; do
  src=${src%/}
  name=$(basename "$src" .scala)
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    left+=("$name (jars)")
    continue
  fi
  case " $flags " in
    *" --target interp "*) left+=("$name (interpreter)"); continue ;;
    *" --target jvm "*) mode=(compiler build --target jvm) ;;
    *) mode=(compiler check) ;;
  esac
  kept=()
  for f in $flags; do
    case $f in --target | jvm | js | --all-mains) ;; *) kept+=("$f") ;; esac
  done
  [ -n "$JARS_CP" ] && kept+=(--classpath "$JARS_CP")
  rm -rf "$tmp/products"
  if TEQ_CAPTURE_CENSUS=$census timeout 60 "$TEQ" "${mode[@]}" --products "$tmp/products" "$src" "${kept[@]}" > "$tmp/log" 2>&1; then
    built=$((built + 1))
  elif grep -q 'cannot write TASTy' "$tmp/log"; then
    refused+=("$name ($(grep -m1 'cannot write TASTy' "$tmp/log" | sed 's/.*cannot write TASTy for //' | cut -c1-80))")
  else
    left+=("$name (fails: $(grep -m1 -E 'error|cannot' "$tmp/log" | cut -c1-100))")
  fi
done
echo "$built programs built in the product mode, ${#refused[@]} typed and refused by the writer (their census counted), ${#left[@]} left out"
for l in "${refused[@]}"; do echo "  refused by the writer: $l"; done
for l in "${left[@]}"; do echo "  left out: $l"; done
tests/support/capture-census.sh "$census"
