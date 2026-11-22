#!/bin/bash
# tests/init-view/records.sh: sourced by the runners of tests/init-view: an output's sum, written by its producer right
# after the output (tests/init-view/records.py reads it): `<step>.sum` beside `<step>.<ext>`, one line `<id> <bytes>
# <sha256>`, the id the unit's directory's name and the output's path under it (`head:` before it where the producer
# names the head's side).

# record_line <unit dir> <output> [<side>]: the sum's line of the output.
record_line() {
  local unit=${1%/} f=$2 side=${3:-} id
  id="$(basename "$(cd "$unit" && pwd)")/${f#"$unit"/}"
  [ -n "$side" ] && id="$side:$id"
  printf '%s %s %s\n' "$id" "$(wc -c < "$f" | tr -d ' ')" "$(sha256sum < "$f" | cut -d' ' -f1)"
}

# record_seal <unit dir> <output> [<side>]: writes the output's sum; an output its step never wrote has none.
record_seal() {
  local f=$2
  [ -f "$f" ] || return 0
  record_line "$@" > "${f%.*}.sum"
}

# record_ok <unit dir> <output> [<side>]: whether the output is what its sum says.
record_ok() {
  local f=$2
  [ -f "$f" ] && [ -f "${f%.*}.sum" ] && [ "$(record_line "$@")" = "$(cat "${f%.*}.sum")" ]
}
