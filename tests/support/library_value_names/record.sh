#!/usr/bin/env bash
# record.sh <givenfold-lib jar>: scalac.txt from scalac 3.8.4's own record of the client. library_value_names.scala
# is compiled by scala-cli against the jar with -Ydump-sbt-inc, and the Used Names of `module class Use$` are kept,
# one name a line, sorted bytewise and unique. A name in another scope than Default is followed by its scope.
set -e
d=$(mktemp -d)
trap 'rm -rf "${d:?}"' EXIT
cp "$(dirname "$0")/library_value_names.scala" "$d/"
(cd "$d" && scala-cli compile -S 3.8.4 --server=false -q --jar "$1" -O -Ydump-sbt-inc library_value_names.scala)
awk '/^Used Names:/ { u = 1; next } /^Dependencies:/ { u = 0 } u && sub(/^module class Use\$: /, "")' "$d/library_value_names.inc" | tr ',' '\n' | sed 's/^ *//; /^$/d' \
  | sed -E 's/ in \[Default\]$//; s/ in \[(.*)\]$/ \1/' | LC_ALL=C sort -u
