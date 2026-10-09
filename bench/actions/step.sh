#!/bin/bash
# bench/actions/step.sh <steps> <carry dir> <out tar> [<in tar>...]: bench/ship.sh --step <steps> in a job of the
# release workflow (docs/DEVELOPING.md, "Releases"), from the root of a checkout of the release's commit: the tars
# of the jobs before it unpacked into <carry dir>, the steps run under GNU time where the machine has it (a runner
# of GitHub's own, unlike the images, may not), the products they made packed into
# <out tar> (permissions kept, which an artifact's zip loses), and the job's measures into the run's summary: each
# step's wall time and exit (the ship's own lines), the peak resident memory, the disk free before and after. For
# `stage`, <out tar> holds the staged set as binaries/ and held/ and the profiles that guided them as profiles/
# (bench/ship-profiles.sh's asset), and the summary the tools its binaries record, the lines a qualifying commit
# gives bench/ship-qualified.txt.
set -uo pipefail
cd "$(dirname "$0")/../.." || exit 1
. bench/ship-manifest.sh || exit 1
[ $# -ge 3 ] || { echo "usage: $0 <steps> <carry dir> <out tar> [<in tar>...]" >&2; exit 2; }
steps=$1 carry=$2 out=$3
shift 3
summary=${GITHUB_STEP_SUMMARY:-/dev/null}
fail() { echo "step: $*" >&2; exit 1; }
mkdir -p "$carry" || exit 1
for tar in "$@"; do tar -xf "$tar" -C "$carry" || fail "$tar could not be unpacked"; done
stamp=$(mktemp) || exit 1
disk_before=$(df -h --output=avail . | tail -1 | tr -d ' ')
start=$(date +%s)
measure=()
[ ! -x /usr/bin/time ] || measure=(/usr/bin/time -v -o "$stamp.time")
"${measure[@]}" bench/ship.sh --step "$steps" --carry "$carry" "$(git rev-parse HEAD)" 2>&1 | tee "$stamp.log"
status=${PIPESTATUS[0]}
took=$(( $(date +%s) - start ))
peak=$(sed -n 's/^\tMaximum resident set size (kbytes): //p' "$stamp.time" 2> /dev/null)
{
  echo "### bench/ship.sh --step $steps: exit $status in $((took / 60)) min $((took % 60)) s"
  echo
  echo '```'
  sed -n '/^ship: steps$/,$p' "$stamp.log" | grep -E '^  [a-z-]+ +exit' || true
  echo '```'
  echo
  echo "Peak resident memory $([ -n "$peak" ] && echo "$((peak / 1024)) MiB" || echo "not measured (no GNU time)"); disk free $disk_before before, $(df -h --output=avail . | tail -1 | tr -d ' ') after; $(nproc) cores, $(uname -m)."
} >> "$summary"
[ "$status" -eq 0 ] || { rm -f -- "$stamp" "$stamp.time" "$stamp.log"; fail "bench/ship.sh --step $steps failed (exit $status)"; }
if [ "$steps" = stage ]; then
  staged=$(mktemp -d) || exit 1
  mkdir -p "$staged/binaries" "$staged/held" "$staged/profiles" || exit 1
  [ ! -d "$carry/integrations/sbt/binary/binaries" ] || cp -a "$carry/integrations/sbt/binary/binaries/." "$staged/binaries/"
  [ ! -d "$carry/out/ship/held" ] || cp -a "$carry/out/ship/held/." "$staged/held/"
  [ ! -d "$carry/out/ship/profiles" ] || cp -a "$carry/out/ship/profiles/." "$staged/profiles/"
  tar -cf "$out" -C "$staged" binaries held profiles || fail "$out not written"
  {
    echo
    echo "The tools the binaries record (the lines of \`bench/ship-qualified.txt\` a qualifying commit writes): each build's"
    echo "toolchain, the native suite's environment, and the environment of a training this release made itself:"
    echo
    for manifest in $(find "$staged" -name teq.manifest | LC_ALL=C sort); do
      sed -n 's/^tuple //p; s/^suite-tuple //p' "$manifest"
      ! manifest_own_training "$manifest" "$(manifest_get "$manifest" commit)" || sed -n 's/^training tuple //p' "$manifest"
    done | LC_ALL=C sort -u | sed 's/^/    /'
    echo
    echo "Held back by \`bench/ship-qualified.txt\`: $(ls "$staged/held" | tr '\n' ' ')"
    echo
    echo "The profiles: $(ls "$staged/profiles" | tr '\n' ' ')($(for f in "$staged"/profiles/*.tar; do [ -f "$f" ] && tar -xOf "$f" profiles.txt | sed -n 's/^trained \([a-z0-9_]*\) \([^ ]*\) .*/\1 trained by \2/p' | tr '\n' ',' | sed 's/,$//; s/,/, /g'; done))"
  } >> "$summary"
  rm -rf -- "$staged"
else
  # The products of these steps alone: the files the run wrote into the carry.
  (cd "$carry" && find . -type f -newer "$stamp" | LC_ALL=C sort) > "$stamp.made"
  [ -s "$stamp.made" ] || fail "the steps $steps made no product"
  tar -cf "$out" -C "$carry" -T "$stamp.made" || fail "$out not written"
  echo "step: $(wc -l < "$stamp.made" | tr -d ' ') products of $steps into $out"
fi
rm -f -- "$stamp" "$stamp.time" "$stamp.log" "$stamp.made"
