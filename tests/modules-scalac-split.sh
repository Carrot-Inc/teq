#!/bin/bash
# tests/modules-scalac-split.sh <case>...: scalac 3.8.4's own two builds of each tests/modules case, the whole build
# (every module's sources at once) and the split one (each module over its upstream modules' classes, as a
# multi-module build compiles it), on the JVM and on Scala.js 1.22.0, and what they differ in. A class the build over
# its upstream's products reads from their pickles is judged as dotty's reader reads it (NoInits from the pickled
# body, the parents unread: TreeUnpickler.scala:1084-1125), where the whole build judged it by its source, so the
# two builds can write a downstream class otherwise, and run otherwise.
#
# A file differs where what it holds differs, apart from its encoding: a class file as `javap -c -p -s` prints it (no
# constant pool index), a .tasty or a .sjsir as tests/modules-scalac-split/Canon.scala prints it (the trees, shared
# ones written out where they stand, no address or position).
# Where they differ it writes, in the case's directory, for tests/modules.sh to hold teq's two builds to:
#   split-differs.txt  the files the two builds write differently, a line `jvm <module>/<path>` (a .class or
#                      .tasty of the JVM builds) or `js <module>/<path>` (a .sjsir or .tasty of the Scala.js
#                      builds) each, after a `sources` line, the digest of the case's sources it was made from;
#   run.split.txt      the split JVM program's output, where it is not the whole one's;
#   run.split.js.txt   the split Scala.js program's output, where it is not run.split.txt's.
# Nothing in them is written by hand: rerun this script after a change to a case's sources. Where the two builds
# agree it writes nothing, and says which of those files the case should no longer have.
set -u
export COURSIER_CACHE=${COURSIER_CACHE:-$HOME/.cache/coursier/v1}
cd "$(dirname "$0")/.." || exit 1
root=$PWD
. tests/support/jars.sh
SL=$(jar_of scala-library)
[ -f "$SL" ] || { echo "no scala-library 3.8.4 in the coursier cache"; exit 1; }
work=$(mktemp -d "${TMPDIR:-/tmp}/scalac-split.XXXXXX") || exit 1
status=0

# The digest of a case's sources, by path and content: split-differs.txt's `sources` line.
digest() {
  local dir=$1
  (cd "$dir" && find . -name '*.scala' | LC_ALL=C sort | while IFS= read -r f; do printf '%s\n' "$f"; cat "$f"; done) | sha256sum | cut -c1-16
}

main_class() {
  python3 - "$@" <<'PYEOF'
import re, sys
for path in sys.argv[1:]:
    text = open(path).read()
    pkg = re.search(r'^package (\S+)', text, re.M)
    prefix = pkg.group(1) + '.' if pkg else ''
    m = re.search(r'@main def (\w+)', text)
    if m:
        print(prefix + m.group(1))
        break
    at = text.find('def main(')
    if at >= 0:
        objects = re.findall(r'^\s*object (\w+)', text[:at], re.M)
        if objects:
            print(prefix + objects[-1])
            break
PYEOF
}

# scli <workspace> <command> <args>...: scala-cli's command on scalac 3.8.4, offline, its workspace a fresh directory of
# its own. Its options follow the command, before the arguments, whose `--` hands what follows it to the program.
scli() {
  local ws=$1 cmd=$2
  shift 2
  mkdir -p "$ws" && (cd "$ws" && COURSIER_MODE=offline timeout 300 scala-cli --power "$cmd" -S 3.8.4 --jvm system --server=false --offline -q --workspace "$ws" "$@" 2>&1)
}

# srun <workspace> <command> <args>...: scli's command, a program's run, its standard output alone (scala-cli's own
# warnings and the program's errors go to <workspace>/run.err).
srun() {
  local ws=$1 cmd=$2
  shift 2
  mkdir -p "$ws" && (cd "$ws" && COURSIER_MODE=offline timeout 300 scala-cli --power "$cmd" -S 3.8.4 --jvm system --server=false --offline -q --workspace "$ws" "$@" 2> "$ws/run.err")
}

canon_ws=$work/ws-canon
# canon <out> <file>...: Canon.scala's text of each .tasty and .sjsir file, after a line `=== <file>` each; it fails
# where Canon does not read them all.
canon() {
  local out=$1
  shift
  [ $# -gt 0 ] || { : > "$out"; return; }
  srun "$canon_ws" run "$root/tests/modules-scalac-split/Canon.scala" -- "$@" > "$out" \
    || { echo "Canon failed on $*: $(tail -n 3 "$canon_ws/run.err")" >&2; return 1; }
}
# javap_text <dir> <file>: a class file's members and code, its constant pool indices left out.
javap_text() {
  javap -c -p -s -cp "$1" "${2%.class}" 2>&1 | sed 's/#[0-9][0-9]*/#/g; s/^Compiled from.*//'
}

# differing <whole> <split> <module> <kind> <suffix>...: the files under <split> that <whole> holds otherwise or not,
# what they hold apart from its encoding; a class file also where its own pickle differs, which its TASTY attribute
# names by its UUID (the pickle's name the longest prefix of the class file's in its directory).
differing() {
  local whole=$1 split=$2 m=$3 kind=$4
  shift 4
  local pat=()
  for s in "$@"; do pat+=(-o -name "*$s"); done
  local raw=() f
  while IFS= read -r f; do
    f=${f#./}
    if [ ! -f "$whole/$f" ]; then echo "$kind $m/$f"; elif ! cmp -s "$whole/$f" "$split/$f"; then raw+=("$f"); fi
  done < <(cd "$split" && find . -type f \( "${pat[@]:1}" \) | LC_ALL=C sort)
  [ ${#raw[@]} -gt 0 ] || return 0
  local c=() code=()
  for f in "${raw[@]}"; do
    case $f in
      *.class) [ "$(javap_text "$whole" "$f")" = "$(javap_text "$split" "$f")" ] || code+=("$f") ;;
      *) c+=("$f") ;;
    esac
  done
  if [ ${#c[@]} -gt 0 ]; then
    canon "$work/canon-w.txt" "${c[@]/#/$whole/}" || return 1
    canon "$work/canon-s.txt" "${c[@]/#/$split/}" || return 1
  else
    : > "$work/canon-w.txt"; : > "$work/canon-s.txt"
  fi
  printf '%s\n' "${raw[@]}" > "$work/raw.txt"
  printf '%s\n' "${code[@]}" > "$work/code.txt"
  python3 - "$work/canon-w.txt" "$work/canon-s.txt" "$whole" "$split" "$kind" "$m" "$work/raw.txt" "$work/code.txt" <<'PYEOF'
import sys, os
def parts(p, root):
    out = {}
    for part in open(p).read().split('=== ')[1:]:
        head, _, body = part.partition('\n')
        out[head[len(root) + 1:]] = body
    return out
w, s = parts(sys.argv[1], sys.argv[3]), parts(sys.argv[2], sys.argv[4])
kind, m = sys.argv[5], sys.argv[6]
raw = [l for l in open(sys.argv[7]).read().split('\n') if l]
code = {l for l in open(sys.argv[8]).read().split('\n') if l}
listed = {f for f in s if w.get(f) != s[f]}
def pickle_of(cls):
    d, b = os.path.split(cls[:-len('.class')])
    names = [os.path.basename(t)[:-len('.tasty')] for t in raw if t.endswith('.tasty') and os.path.dirname(t) == d]
    names = [n for n in names if b == n or b.startswith(n + '$')]
    return os.path.join(d, max(names, key=len) + '.tasty') if names else None
for f in raw:
    if f.endswith('.class'):
        if f in code or pickle_of(f) in listed:
            print(f"{kind} {m}/{f}")
    elif f in listed:
        print(f"{kind} {m}/{f}")
PYEOF
}

for name in "$@"; do
  dir=tests/modules/$name
  [ -d "$dir/a" ] || { echo "no case $name"; status=1; continue; }
  mods=(a)
  [ -d "$dir/b" ] && mods+=(b)
  [ -d "$dir/c" ] && mods+=(c)
  [ -d "$dir/main" ] && mods+=(main)
  w=$work/$name
  mkdir -p "$w"
  all=()
  for m in "${mods[@]}"; do
    while IFS= read -r f; do all+=("$root/$f"); done < <(find "$dir/$m" -name '*.scala' | LC_ALL=C sort)
  done
  failed=""
  for platform in jvm js; do
    pf=()
    [ $platform = js ] && pf=(--js --js-version 1.22.0)
    scli "$w/ws-$platform-whole" compile "${pf[@]}" -d "$w/$platform-whole" "${all[@]}" > "$w/$platform-whole.log" || failed="$failed $platform-whole"
    cp=""
    for m in "${mods[@]}"; do
      srcs=()
      while IFS= read -r f; do srcs+=("$root/$f"); done < <(find "$dir/$m" -name '*.scala' | LC_ALL=C sort)
      scli "$w/ws-$platform-$m" compile "${pf[@]}" ${cp:+--classpath "$cp"} -d "$w/$platform-$m" "${srcs[@]}" > "$w/$platform-$m.log" || failed="$failed $platform-$m"
      cp=${cp:+$cp:}$w/$platform-$m
    done
  done
  if [ -n "$failed" ]; then
    echo "$name: scalac failed:$failed (logs in $w)"
    status=1
    continue
  fi
  # A list is written only of what Canon read whole: a failure leaves the case's files as they are.
  if ! entries=$(for m in "${mods[@]}"; do
    differing "$w/jvm-whole" "$w/jvm-$m" "$m" jvm .class .tasty || exit 1
    differing "$w/js-whole" "$w/js-$m" "$m" js .sjsir .tasty || exit 1
  done); then
    echo "$name: Canon failed, nothing written (logs in $w)"
    status=1
    continue
  fi
  split_out=""
  split_js=""
  if [ -f "$dir/run.txt" ]; then
    last=${mods[${#mods[@]}-1]}
    lsrcs=()
    while IFS= read -r f; do lsrcs+=("$root/$f"); done < <(find "$dir/$last" -name '*.scala' | LC_ALL=C sort)
    main=$(main_class "${lsrcs[@]}")
    cp=""
    for m in "${mods[@]}"; do cp=${cp:+$cp:}$w/jvm-$m; done
    whole_out=$(timeout 60 java -XX:-UsePerfData -cp "$w/jvm-whole:$SL" "$main" 2>&1)
    split_out=$(timeout 60 java -XX:-UsePerfData -cp "$cp:$SL" "$main" 2>&1)
    [ "$whole_out" = "$(cat "$dir/run.txt")" ] || echo "$name: scalac's whole JVM program prints otherwise than run.txt: $whole_out"
    up=""
    for m in "${mods[@]}"; do [ "$m" = "$last" ] || up=${up:+$up:}$w/js-$m; done
    split_js=$(srun "$w/ws-js-run" run --js --js-version 1.22.0 ${up:+--classpath "$up"} --main-class "$main" "${lsrcs[@]}")
    [ "$whole_out" = "$split_out" ] && split_out=""
    # What tests/modules.sh holds the split Scala.js programs to without run.split.js.txt: run.split.txt, else the
    # whole program's Scala.js output, run.js.txt or run.txt.
    js_want=$split_out
    if [ -z "$js_want" ]; then
      if [ -f "$dir/run.js.txt" ]; then js_want=$(cat "$dir/run.js.txt"); else js_want=$(cat "$dir/run.txt"); fi
    fi
    [ "$split_js" = "$js_want" ] && split_js=""
  fi
  files=()
  if [ -n "$entries" ]; then
    { echo "# scalac 3.8.4's own whole and split builds of this case write these files differently: written by"
      echo "# tests/modules-scalac-split.sh, never by hand; tests/modules.sh requires teq's two builds to differ"
      echo "# in the rows they stand for, and in these files alone."
      echo "sources $(digest "$dir")"
      printf '%s\n' "$entries"; } > "$dir/split-differs.txt"
    files+=(split-differs.txt)
  fi
  [ -n "$split_out" ] && { printf '%s\n' "$split_out" > "$dir/run.split.txt"; files+=(run.split.txt); }
  [ -n "$split_js" ] && { printf '%s\n' "$split_js" > "$dir/run.split.js.txt"; files+=(run.split.js.txt); }
  for f in split-differs.txt run.split.txt run.split.js.txt; do
    [[ " ${files[*]} " == *" $f "* ]] || { [ -f "$dir/$f" ] && echo "$name: scalac's builds agree on what $f holds: take $dir/$f off"; }
  done
  if [ ${#files[@]} -gt 0 ]; then echo "$name: ${files[*]}"; else echo "$name: scalac's two builds agree"; fi
done
exit $status
