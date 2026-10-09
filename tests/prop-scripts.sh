#!/bin/bash
# Checks of what tests/prop-campaign.sh decides by itself, without a campaign: the shrink queue,
# `requeue` beside a save in flight, the environment that tells a trigger's failure, the
# campaign's status and `collect`'s. Every check is bounded; the script exits 1 on a failure.
cd "$(dirname "$0")/.." || exit 1
PROP_CAMPAIGN_LIB=1 . tests/prop-campaign.sh || { echo "FAIL the campaign script does not load"; exit 1; }
work=$(mktemp -d "${TMPDIR:-/tmp}/prop-scripts.XXXXXX") || exit 1
trap 'rm -rf "$work"' EXIT
pass=0
fail=0
check() {
  if [ "$2" = "$3" ]; then pass=$((pass + 1)); else echo "FAIL $1: $2, where $3 is expected"; fail=$((fail + 1)); fi
}

# The queue: three failures of one kind and one of another; with the first two shrunk, the other
# kind comes next and the third of the first kind after it, since a kind never closes the queue.
dir=$work/queue
mkdir -p "$dir"
printf 'k1\tp1\nk1\tp2\nk2\tp3\nk1\tp4\n' > "$dir/failures.txt"
: > "$dir/shrunk.txt"
check "the first of all" "$(next_failure)" "$(printf 'k1\tp1')"
printf 'k1\tp1\n' >> "$dir/shrunk.txt"
check "another kind before a second of one" "$(next_failure)" "$(printf 'k2\tp3')"
printf 'k2\tp3\nk1\tp2\n' >> "$dir/shrunk.txt"
check "a third of one kind" "$(next_failure)" "$(printf 'k1\tp4')"
printf 'k1\tp4\n' >> "$dir/shrunk.txt"
check "nothing left" "$(next_failure)" ""

# requeue: an entry moves to the secondary key, a temporary of a save in flight stays.
fnv() { python3 -c 'import sys
h = 0xcbf29ce484222325
for b in sys.argv[1].encode():
    h = ((h ^ b) * 0x100000001b3) & 0xffffffffffffffff
print("%016x" % h)' "$1"; }
db=$work/db
mkdir -p "$db/$(fnv 'native:.hegel-keys')" "$db/$(fnv 'native:t::p')"
printf 't::p' > "$db/$(fnv 'native:.hegel-keys')/k"
printf 'entry' > "$db/$(fnv 'native:t::p')/0123456789abcdef"
printf 'half' > "$db/$(fnv 'native:t::p')/fedcba9876543210.tmp.42.0"
check "requeue's count" "$(requeue "$db")" 1
check "the entry moved" "$(ls "$db/$(fnv 'native:t::p.secondary')")" 0123456789abcdef
check "the temporary stayed" "$(ls "$db/$(fnv 'native:t::p')")" fedcba9876543210.tmp.42.0

# The environment of a trigger's classification has neither the triggers nor a kind kept.
printf "export TEQ=/t\nexport TEQ_PROP_KNOWN='a,b'\nexport TEQ_PROP_KEEP=v:\\\\ x\ntest=targets_agree\n" > "$work/env.sh"
check "without triggers" "$(without_triggers "$work/env.sh" | tr '\n' ' ')" "export TEQ=/t test=targets_agree "

# The campaign's status.
dir=$work/outcome
mkdir -p "$dir/shrinks/001"
check "nothing failed" "$(campaign_outcome)" 0
echo 124 > "$dir/shrinks/001/exit"
check "a shrink run cut" "$(campaign_outcome)" 3
printf 'k\tp\n' > "$dir/failures.txt"
check "a failure" "$(campaign_outcome)" 1

# collect: a fetch that fails is a failure whatever the release says; a fetched campaign gives
# its own status, one that did not end 3.
REMOTE_AGENT=$work/remote
mkdir -p "$REMOTE_AGENT"
printf '#!/bin/bash\nexit 0\n' > "$REMOTE_AGENT/release.sh"
printf '#!/bin/bash\nexit 1\n' > "$REMOTE_AGENT/get.sh"
chmod +x "$REMOTE_AGENT"/*.sh
remote_collect machine/x "$work/fetched" > /dev/null 2>&1
check "a failed fetch" "$?" 1
for status in 0 1 none; do
  printf '#!/bin/bash\nmkdir -p "$4/remote" && echo "- stamp: s" > "$4/remote/report.md"%s\n' "$([ $status = none ] || echo " && echo $status > \"\$4/remote/exit.txt\"")" > "$REMOTE_AGENT/get.sh"
  rm -rf "$work/fetched"
  remote_collect machine/x "$work/fetched" > /dev/null 2>&1
  check "a fetched campaign of status $status" "$?" "$([ $status = none ] && echo 3 || echo $status)"
done

echo "$pass passed, $fail failed"
[ $fail = 0 ]
