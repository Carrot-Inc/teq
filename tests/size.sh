#!/bin/bash
# Output size: each program under tests/size is built, run under node against its .expected file
# (the output of the same program under Scala.js), and its byte count compared with the budgets
# in tests/size/budgets.txt: the development output, the --release output, and the release
# output through esbuild --minify and gzip, which is what a production bundle ships. Growth
# beyond 1% fails. SIZE_RECORD=1 rewrites the budgets, so that a deliberate change of size shows
# up in the diff of that file. esbuild is taken from $ESBUILD, the PATH or node_modules; without
# it the last column is neither measured nor checked.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
budgets=tests/size/budgets.txt
mkdir -p out/size
pass=0
fail=0
recorded=""
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
esbuild=${ESBUILD:-$(command -v esbuild)}
for candidate in node_modules/.bin/esbuild "$HOME/.config/yarn/global/node_modules/esbuild/bin/esbuild"; do
  [ -z "$esbuild" ] && [ -x "$candidate" ] && esbuild=$candidate
done
[ -z "$esbuild" ] && echo "note: esbuild not found, the minified column is skipped"
# Checks one measured size against its budget column.
budget() {
  local name=$1 what=$2 size=$3 column=$4
  local limit
  limit=$(awk -v n="$name" -v c="$column" '$1 == n { print $c }' "$budgets")
  if [ -z "$limit" ] || [ "$limit" = "-" ]; then
    bad "$name has no $what budget"
  elif [ "$size" -gt $((limit + limit / 100)) ]; then
    bad "$name $what is $size bytes, budget $limit"
  else
    pass=$((pass + 1))
    if [ "$size" -lt $((limit - limit / 100)) ]; then
      echo "note: $name $what is $size bytes, budget $limit; lower it with SIZE_RECORD=1"
    fi
  fi
}
for src in tests/size/*.scala; do
  name=$(basename "$src" .scala)
  out=out/size/$name.js
  release=out/size/$name.release.js
  if ! timeout 30 "$TEQ" compiler build "$src" -o "$out" > "out/size/$name.log" 2>&1; then
    bad "$name build"
    continue
  fi
  timeout 20 node "$out" > "out/size/$name.actual" 2>&1
  if ! diff "tests/size/$name.expected" "out/size/$name.actual" > /dev/null; then
    bad "$name output"
    continue
  fi
  if ! timeout 30 "$TEQ" compiler build "$src" -o "$release" --release > "out/size/$name.release.log" 2>&1; then
    bad "$name release build"
    continue
  fi
  timeout 20 node "$release" > "out/size/$name.release.actual" 2>&1
  if ! diff "tests/size/$name.expected" "out/size/$name.release.actual" > /dev/null; then
    bad "$name release output"
    continue
  fi
  size=$(wc -c < "$out" | tr -d ' ')
  release_size=$(wc -c < "$release" | tr -d ' ')
  minified=-
  if [ -n "$esbuild" ]; then
    if ! timeout 30 "$esbuild" "$release" --minify > "out/size/$name.min.js" 2> "out/size/$name.min.log"; then
      bad "$name esbuild"
      continue
    fi
    timeout 20 node "out/size/$name.min.js" > "out/size/$name.min.actual" 2>&1
    if ! diff "tests/size/$name.expected" "out/size/$name.min.actual" > /dev/null; then
      bad "$name minified output"
      continue
    fi
    minified=$(gzip -9 < "out/size/$name.min.js" | wc -c | tr -d ' ')
  fi
  recorded+="$name $size $release_size $minified"$'\n'
  [ -n "$SIZE_RECORD" ] && continue
  budget "$name" dev "$size" 2
  budget "$name" release "$release_size" 3
  [ "$minified" != - ] && budget "$name" "release+esbuild+gzip" "$minified" 4
done
if [ -n "$SIZE_RECORD" ]; then
  printf '%s' "$recorded" > "$budgets"
  cat "$budgets"
  [ $fail = 0 ]
  exit
fi
echo "$pass passed, $fail failed"
[ $fail = 0 ]
