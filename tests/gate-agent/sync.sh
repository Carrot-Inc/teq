#!/bin/bash
# sync.sh <handle> <tree>: the tree mirrored to the machine, its tracked and untracked files but what git ignores
# (a teq-head's binary and manifest included): as the project of its directory's name when that is teq-<something>,
# else as the project teq.
. "$(dirname "$0")/common.sh"
m=$(machine_of "$1")
claimed "$m" "$1" || { echo "sync: $1 holds no claim" >&2; exit 1; }
# the landing is the project teq, master's and the reference's clones and the head's binary their own
name=$(basename "$2")
case $name in teq-*) ;; *) name=teq ;; esac
dest=$GATE_AGENT_ROOT/machines/$m/rw/$name
mkdir -p "$dest"
(cd "$2" && { git ls-files -co --exclude-standard 2> /dev/null || find . -type f; }) | grep -v '^\.git/' > "$GATE_AGENT_ROOT/files.$$"
rsync -a --files-from="$GATE_AGENT_ROOT/files.$$" "$2/" "$dest/" || exit 1
rm -f "$GATE_AGENT_ROOT/files.$$"
if [ $name = teq-head ] && [ "$GATE_AGENT_CORRUPT" = "$m" ]; then printf 'spoilt' >> "$dest/teq"; fi
