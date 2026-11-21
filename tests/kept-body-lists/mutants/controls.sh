#!/bin/bash
# tests/kept-body-lists/mutants/controls.sh [target dir]: the kept-body-lists suite's controls. Each <answer>.patch here
# breaks one answer `filled_clause` (src/typer/loader/bodies.rs) gives a kept body's list, and the suite must fail on
# every one of them:
#   alternative    which alternative: `signed_among` names an alternative other than the one the signed name names
#   clause         which clause: list N is answered with the clause at N xor 1, a method's and a constructor's
#   using          using or not: the clause's using flag negated
#   normalization  the constructor normalization: a constructor's list 1 taken as the normalization's added empty list
#   extractor      the extractor's implicits: the pickle's implicit arguments of an extractor not kept
#   named          the named list: a clause's parameter names rotated by one
# For the unmodified tree and then each patch: one scratch git worktree of HEAD (reset before each patch), the patch
# applied, `cargo build --profile fast` into one target directory the builds share (the first argument, else one in
# the scratch directory), the binary copied aside; the suite run with TEQ= that binary, its per-cell verdicts written
# (run.py's KB_LISTS_RESULTS). It fails where a patch does not apply, a build fails, a mutant's binary is the
# unmodified one byte for byte, the unmodified run is not green (run.py's exit 0: every cell scalac's or as
# tests/pending/kept-body-lists.txt lists), or a mutant turns no cell the unmodified binary passes into a failure (a
# timed-out cell counts for none). It prints, per mutant, the cells it turns counted by the program's shape.
# Seven fast builds: no gate line; run it when filled_clause, gen.py or a patch changes.
cd "$(dirname "$0")/../../.." || exit 2
REPO=$PWD
WORK=$(mktemp -d "${TMPDIR:-/tmp}/kb-lists-controls.XXXXXX") || exit 2
TREE=$WORK/tree
TARGET=${1:-$WORK/target}
cleanup() { git -C "$REPO" worktree remove --force "$TREE" > /dev/null 2>&1; rm -rf -- "${WORK:?}"; }
trap cleanup EXIT
git worktree add -q --detach "$TREE" HEAD || { echo "controls: no scratch worktree"; exit 1; }
fail=0
build() { # build <name>: the scratch tree as it stands, built, its binary at $WORK/teq-<name>
  (cd "$TREE" && CARGO_TARGET_DIR="$TARGET" timeout 3000 cargo build --profile fast > "$WORK/build-$1.log" 2>&1) || { echo "FAIL $1: the build fails: $(grep -m1 '^error' "$WORK/build-$1.log")"; return 1; }
  cp "$TARGET/fast/teq" "$WORK/teq-$1"
}
suite() { # suite <name>: the suite on $WORK/teq-<name>, its verdicts at $WORK/res-<name>.txt; the run's exit
  TEQ="$WORK/teq-$1" KB_LISTS_RESULTS="$WORK/res-$1.txt" timeout 3000 python3 tests/kept-body-lists/run.py > "$WORK/run-$1.out" 2>&1
}
build base || exit 1
suite base
rc=$?
if [ $rc != 0 ]; then
  echo "FAIL the unmodified tree's run is not green (exit $rc): $(tail -3 "$WORK/run-base.out" | tr '\n' ' ')"
  exit 1
fi
echo "ok   unmodified: $(grep '^kept-body-lists:' "$WORK/run-base.out")"
for p in tests/kept-body-lists/mutants/*.patch; do
  name=$(basename "$p" .patch)
  git -C "$TREE" checkout -q -- . && git -C "$TREE" clean -qfd > /dev/null
  if ! git -C "$TREE" apply "$REPO/$p"; then echo "FAIL $name: the patch does not apply"; fail=1; continue; fi
  build "$name" || { fail=1; continue; }
  if cmp -s "$WORK/teq-base" "$WORK/teq-$name"; then echo "FAIL $name: the mutant's binary is the unmodified one"; fail=1; continue; fi
  suite "$name"
  python3 - "$WORK/res-base.txt" "$WORK/res-$name.txt" "$name" <<'PYEOF' || fail=1
import collections, re, sys
def read(p):
    v = {}
    for l in open(p):
        if l.strip():
            kind, rest = l.rstrip("\n").split(" ", 1)
            v[rest.split("\t")[0]] = kind
    return v
base, mut, name = read(sys.argv[1]), read(sys.argv[2]), sys.argv[3]
turned = sorted(c for c, k in mut.items() if k == "fail" and base.get(c) == "pass")
timeouts = sum(1 for k in mut.values() if k == "timeout")
shape = lambda c: re.match(r"(k[bcr]_[a-z]+_[a-z]+)", c).group(1) if re.match(r"(k[bcr]_[a-z]+_[a-z]+)", c) else c
by = collections.Counter(shape(c) for c in turned)
if not turned:
    print(f"FAIL {name}: the suite stays green on it (no cell the unmodified binary passes fails; {timeouts} timed out)")
    sys.exit(1)
print(f"ok   {name}: {len(turned)} cells turned ({timeouts} timed out, counted for none): " + ", ".join(f"{k} {v}" for k, v in sorted(by.items())))
PYEOF
done
[ $fail = 0 ] && echo "controls: every mutant turns the suite" || echo "controls: FAILED"
exit $fail
