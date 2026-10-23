#!/bin/bash
# tests/semanticdb/identity-run.sh <teq> <out dir> [cases|typing|all]: the transparency of the semantic typing
# (tests/semanticdb/TABLE.md, section 6): `--semantic-types` makes the typer keep, as `Type::Retained`, what dotty's
# types keep and teq's canonical ones erase, for teq fix's SemanticDB forms, and must change nothing else. `cases`:
# every program of tests/cases and tests/errors built by <teq> at one worker with and without the flag, compared byte
# for byte (tests/support/identity.sh, the kinds js, jvm and diags); `typing`: the scala3-typing harness's every test
# classified by <teq> at one worker with and without it, each test's category and detail compared. Both by default.
# Both sides run at one worker (the flag implies one: teq fix types with one), so the flag is the one difference.
# Prints what differs and exits 1 when anything does. JOBS as identity.sh and the harness take it, SCALA3 as the harness
# does (a checkout with tests/neg, pos and warn at expected.txt's commit). Long: detach it.
set -u
cd "$(dirname "$0")/../.."
teq=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
out=$2
what=${3:-all}
mkdir -p "$out"
status=0
if [ "$what" = cases ] || [ "$what" = all ]; then
  if ! REF_FLAGS="--threads 1" NEW_FLAGS="--threads 1 --semantic-types" tests/support/identity.sh "$teq" "$teq" "$out/cases" js,jvm,diags > "$out/cases.log" 2>&1; then
    status=1
  fi
  tail -4 "$out/cases.log"
fi
if [ "$what" = typing ] || [ "$what" = all ]; then
  for side in ref new; do
    flags="--threads 1"
    [ $side = new ] && flags="--threads 1 --semantic-types"
    printf '#!/bin/bash\nexec "%s" "$@" %s\n' "$teq" "$flags" > "$out/teq-$side"
    chmod +x "$out/teq-$side"
    TEQ="$out/teq-$side" python3 tests/scala3-typing/harness.py --verbose ${JOBS:+--jobs "$JOBS"} > "$out/typing-$side.log" 2>&1
    grep -P '^[a-z]+/[^\t]+\t' "$out/typing-$side.log" | sort > "$out/typing-$side.rows"
  done
  if ! diff "$out/typing-ref.rows" "$out/typing-new.rows" > "$out/typing.diff"; then
    echo "typing: $(grep -c '^>' "$out/typing.diff") tests classified otherwise with --semantic-types (typing.diff)"
    status=1
  else
    echo "typing: $(wc -l < "$out/typing-ref.rows") tests classified alike"
  fi
fi
exit $status
