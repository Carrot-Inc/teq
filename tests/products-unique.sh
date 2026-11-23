#!/bin/bash
# tests/products-unique.sh <teq> <out dir> <unit dir>...: no two products of a build share a path, and every build of a
# unit writes the same products. For each unit (a directory: its .scala files at any depth, a tests/modules case's
# modules among them; a case with a generate.py is made in a copy first) the whole unit is built into products
# (`compiler check --std scala-library --products`, TEQ_UNIQUE_MODE=jvm `compiler build --target jvm --products`) at 1
# and at 8 workers (`--threads`), each RUNS times (default 3). Per build that exits 0, the manifest (teq-products.json)
# is read: a TASTy or class path two of its products name is a SHARED path (one product's definitions lost to the
# other's). Across the unit's builds, the exits must be the same and, where they built, the set of product files and
# their bytes. Prints per unit its builds, the shared paths and the builds that differ, named; every build bounded at
# 120 s. A build's exit is taken by the list of the admitted ones: 0, it built; teq's refusal by name, its exit with its
# line on its log: 1 with `<file>:<line>:<column>: error: ..` or `<n> definitions cannot be written as TASTy`, 3 with
# `<n> constructs not supported by the JVM backend`. A unit is NOT JUDGED and named when a build's exit is not admitted
# (the bound passed, 124; a signal's, 128 and its number; 2; a refusal's exit without its line), a build's manifest is
# missing, empty, not whole JSON or not the record of its directory's files (a file it lists not there, a product file
# it does not list), its directory does not exist or it holds no .scala file, and GIVEN TWICE when its directory is
# given again; a unit whose every build refuses does not build, and is named so. <out dir> is an absolute path, new or
# one a census made (it marks it); it writes and deletes only its own files there. Exits 1 when a path is shared, the
# builds differ, a unit is not judged or given twice.
# tests/products-unique.sh --self-test: the census over planted units, built by a stand-in teq that writes planted
# products and manifests (a shared path, builds that differ, a build past its bound, an exit not admitted (a signal's,
# 2, a refusal's exit without its line), a manifest missing, empty, cut, or not listing a file written or listing one
# not written, a unit given twice, missing or without sources; an out dir relative, the root, a file or one holding
# files no census wrote), each of which it must fail with its reason named, and good units (one a program that defines
# nothing, units whose every build refuses by one of teq's refusals), which it must pass. The self-test runs first in
# every census, which stops where it fails.
set -u
runs=${RUNS:-3}
mode=${TEQ_UNIQUE_MODE:-check}

build() { # build <products> <threads> <files...>, from the unit's directory
  local p=$1 n=$2
  shift 2
  if [ "$mode" = jvm ]; then
    timeout 120 "$teq" compiler build --target jvm --threads "$n" --products "$p" "$@" > "$p.log" 2>&1
  else
    timeout 120 "$teq" compiler check --std scala-library --threads "$n" --products "$p" "$@" > "$p.log" 2>&1
  fi
}

shared_paths() { # shared_paths <teq-products.json>: the paths two products name, one a line; exits 3 with why where the manifest cannot be read or is not the record of its directory's files
  python3 - "$1" <<'PY'
import collections, json, sys
try:
    text = open(sys.argv[1], encoding="utf-8").read()
except OSError:
    print("no record of its teq-products.json")
    sys.exit(3)
if not text.strip():
    print("its teq-products.json empty")
    sys.exit(3)
try:
    m = json.loads(text)
except ValueError:
    print("its teq-products.json not whole JSON (cut)")
    sys.exit(3)
products = m.get("products") if isinstance(m, dict) else None
if not isinstance(products, list) or not all(isinstance(p, dict) for p in products):
    print("its teq-products.json holds no list of products")
    sys.exit(3)
# The manifest is the record of the directory's product files: each file it lists (a product's TASTy and classes, the
# std's classes) is there, and each TASTy or class file there is listed. A program that defines nothing has an empty
# list and no file.
import os
here = os.path.dirname(os.path.abspath(sys.argv[1]))
listed = set()
for p in products:
    if isinstance(p.get("tasty"), str):
        listed.add(p["tasty"])
    for c in (p.get("classes") or []) + (p.get("std") or []):
        if isinstance(c, str):
            listed.add(c)
for c in m.get("std") or []:
    if isinstance(c, str):
        listed.add(c)
present = set()
for root, _, files in os.walk(here):
    for f in files:
        if f.endswith((".tasty", ".class")):
            present.add(os.path.relpath(os.path.join(root, f), here))
if listed - present:
    print("its teq-products.json lists a file that is not there: " + sorted(listed - present)[0])
    sys.exit(3)
if present - listed:
    print("a product file its teq-products.json does not list: " + sorted(present - listed)[0])
    sys.exit(3)
paths = collections.Counter()
for p in products:
    if p.get("tasty"):
        paths[p["tasty"]] += 1
    for c in p.get("classes", []) or []:
        paths[c if isinstance(c, str) else json.dumps(c, sort_keys=True)] += 1
print("\n".join(sorted(k for k, n in paths.items() if n > 1)))
PY
}

census() { # census <teq> <out dir> <unit dir>...
  teq=$(cd "$(dirname "${1:?a teq}")" && pwd)/$(basename "$1")
  out=${2:?an out dir}
  shift 2
  # The out directory: an absolute path, not the root, and either new or one a census made (its mark,
  # .products-unique-out); a directory holding files no census wrote is refused and left as it is. Inside it the
  # census deletes nothing but its units' own work directories.
  case "$out" in
    /) echo "products-unique: the out dir is the root: refused"; return 1 ;;
    /*) ;;
    *) echo "products-unique: the out dir $out is not an absolute path: refused"; return 1 ;;
  esac
  if [ -e "$out" ] && [ ! -d "$out" ]; then echo "products-unique: the out dir $out is not a directory: refused"; return 1; fi
  if [ -d "$out" ] && [ -n "$(ls -A "$out")" ] && [ ! -f "$out/.products-unique-out" ]; then
    echo "products-unique: the out dir $out holds files no census wrote (no .products-unique-out): refused"; return 1
  fi
  mkdir -p "$out" && touch "$out/.products-unique-out" || { echo "products-unique: the out dir $out cannot be made"; return 1; }
  out=$(cd "$out" && pwd)
  local units=0 built=0 shared_total=0 differ_total=0 nj=0 twice=0
  local unit real name w src files p n r st s path shared timed unread sums exits codes distinct why unadmitted named
  declare -A seen=()
  for unit in "$@"; do
    units=$((units + 1))
    real=$(cd "$unit" 2> /dev/null && pwd -P)
    if [ -z "$real" ] || [ ! -d "$unit" ]; then echo "$unit: NOT JUDGED: no such directory"; nj=$((nj + 1)); continue; fi
    unit=$(cd "$unit" && pwd)
    name=$(basename "$unit")
    # A corpus's unit kept as <name>/src is named by its directory.
    [ "$name" = src ] && name=$(basename "$(dirname "$unit")")
    if [ -n "${seen[$real]:-}" ]; then echo "$name: GIVEN TWICE ($unit)"; twice=$((twice + 1)); continue; fi
    seen[$real]=1
    w=$out/$(echo "$unit" | tr '/' '_')
    case "$w" in "$out"/?*) rm -rf "${w:?}" ;; esac
    mkdir -p "$w"
    src=$unit
    if [ -f "$unit/generate.py" ]; then
      cp -R "$unit" "$w/case" && timeout 120 python3 "$unit/generate.py" "$w/case" > /dev/null 2>&1 || { echo "$name: NOT JUDGED: generate.py"; nj=$((nj + 1)); continue; }
      src=$w/case
    fi
    mapfile -t files < <(cd "$src" && find . -name '*.scala' | sed 's|^\./||' | sort)
    if [ "${#files[@]}" = 0 ]; then echo "$name: NOT JUDGED: no .scala files"; nj=$((nj + 1)); continue; fi
    shared=0 timed=0 unread=0 unadmitted=0 sums=() exits=()
    for n in 1 8; do
      for r in $(seq 1 "$runs"); do
        p=$w/b-$n-$r
        (cd "$src" && build "$p" "$n" "${files[@]}")
        st=$?
        if [ "$st" = 124 ]; then echo "$name: NOT JUDGED: the build at $n workers, run $r, passed 120 s"; timed=1; break 2; fi
        # teq's refusals by name, each its exit with its line (src/main.rs prints each before that exit): 1 with an
        # error line or the summary of definitions it cannot write as TASTy, 3 with the summary of constructs its JVM
        # backend does not support.
        case $st in
          1) named='^.+:[0-9]+:[0-9]+: error: |^[0-9]+ definitions? cannot be written as TASTy$' ;;
          3) named='^[0-9]+ constructs? not supported by the JVM backend$' ;;
          *) named= ;;
        esac
        if [ "$st" != 0 ] && { [ -z "$named" ] || ! sed 's/\x1b\[[0-9;]*m//g' "$p.log" | grep -qE "$named"; }; then
          why="exited $st"
          [ "$st" -gt 128 ] 2> /dev/null && why="$why (signal $((st - 128)))"
          [ "$st" = 1 ] && why="$why with no teq error or TASTy summary on its log"
          [ "$st" = 3 ] && why="$why with no JVM backend summary on its log"
          echo "$name: NOT JUDGED: the build at $n workers, run $r, $why ($(tail -1 "$p.log" | cut -c1-100))"
          unadmitted=1
          break 2
        fi
        exits+=("$n-$r $st")
        [ "$st" = 0 ] || continue
        s=$(shared_paths "$p/teq-products.json")
        if [ $? != 0 ]; then echo "$name: NOT JUDGED: the build at $n workers, run $r: $s"; unread=1; break 2; fi
        for path in $s; do echo "$name: SHARED $path (at $n workers, run $r)"; shared=$((shared + 1)); done
        sums+=("$n-$r $(cd "$p" && find . -type f ! -name teq-products.json ! -name '*.log' | sort | xargs -r sha256sum | sha256sum | cut -c1-16)")
      done
    done
    shared_total=$((shared_total + shared))
    if [ "$timed" = 1 ] || [ "$unread" = 1 ] || [ "$unadmitted" = 1 ]; then nj=$((nj + 1)); continue; fi
    codes=$(printf '%s\n' "${exits[@]}" | awk '{print $2}' | sort -u | wc -l)
    if [ "$codes" != 1 ]; then
      echo "$name: DIFFER: its builds exit differently: $(printf '%s; ' "${exits[@]}")"
      differ_total=$((differ_total + 1))
      continue
    fi
    if [ "$st" != 0 ]; then echo "$name: does not build ($(tail -1 "$p.log" | cut -c1-100))"; continue; fi
    built=$((built + 1))
    distinct=$(printf '%s\n' "${sums[@]}" | awk '{print $2}' | sort -u | wc -l)
    if [ "$distinct" != 1 ]; then
      echo "$name: DIFFER: $distinct distinct product sets over its builds: $(printf '%s; ' "${sums[@]}")"
      differ_total=$((differ_total + 1))
    fi
    case "$w" in "$out"/?*) rm -rf "${w:?}" ;; esac
  done
  echo "products-unique: $units units, $built built, $nj NOT JUDGED, $twice given twice; $shared_total shared paths; $differ_total units whose builds differ"
  [ "$shared_total" = 0 ] && [ "$differ_total" = 0 ] && [ "$nj" = 0 ] && [ "$twice" = 0 ]
}

self_test() {
  local tmp cases=0 good=0 name expect word kinds k units got rc
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/products-unique-self-test.XXXXXX") || { echo "products-unique self-test: no temporary directory"; return 1; }
  # The stand-in teq: run from a unit's directory, it does what the unit's `plant` file names, at the workers and the
  # run its products directory (b-<workers>-<run>) gives: writes a/A.tasty and a/A.class and a manifest naming them, or
  # a shared path, other bytes, an exit past the bound (124) or a refusal, a manifest missing, empty, cut, listing no
  # product of the files written or a file not written, or nothing at all for a program that defines nothing.
  cat > "$tmp/teq" <<'PY'
#!/usr/bin/env python3
import json, os, sys
a = sys.argv[1:]
p, n = a[a.index("--products") + 1], a[a.index("--threads") + 1]
run = os.path.basename(p).rsplit("-", 1)[1]
kind = open("plant").read().strip()
if kind == f"timeout-{n}":
    sys.exit(124)
if kind in (f"refuses-at-{n}", "refuses"):
    print("A.scala:1:8: error: refused")
    sys.exit(1)
if kind in (f"undiagnosed-at-{n}", "undiagnosed"):
    print("error: refused")
    sys.exit(1)
if kind == "tasty-refusal":
    print("A.scala: cannot write TASTy for x\n1 definition cannot be written as TASTy")
    sys.exit(1)
if kind == "tasty-without-the-summary":
    print("A.scala: cannot write TASTy for x")
    sys.exit(1)
if kind == "jvm-backend-refusal":
    print("A.scala: x\n2 constructs not supported by the JVM backend")
    sys.exit(3)
if kind == "exit-3":
    print("something failed")
    sys.exit(3)
if kind == "exit-2-with-a-refusal-line":
    print("error: x/teq-products.json: these products are of ABI 0 (written by teq 0.1.10)")
    sys.exit(2)
if kind in (f"exit-2-at-{n}", "exit-2"):
    print("cannot open x: No such file or directory (os error 2)")
    sys.exit(2)
if kind == "aborts":
    print("fatal runtime error: stack overflow, aborting")
    sys.exit(134)
for code, signal in ((137, "killed"), (139, "crashed")):
    if kind in (f"{signal}-at-{n}", signal):
        sys.exit(code)
files = {} if kind == "nothing" else {"a/A.tasty": "T", "a/A.class": "D" if kind == f"differ-at-{n}" else "C"}
if kind == "lists-missing":
    del files["a/A.tasty"]
products = [{"source": "A.scala", "tasty": "a/A.tasty", "classes": ["a/A.class"]}]
if kind == f"differ-in-run-{run}":
    files["a/B.class"] = "B"
    products.append({"source": "A.scala", "tasty": None, "classes": ["a/B.class"]})
if kind == "shared-tasty":
    products.append({"source": "B.scala", "tasty": "a/A.tasty", "classes": []})
if kind == "shared-class":
    products.append({"source": "B.scala", "tasty": None, "classes": ["a/A.class"]})
manifest = json.dumps({"products": [] if kind in ("no-products", "nothing") else products})
manifest = {f"empty-manifest-{n}": "", f"cut-manifest-{n}": manifest[:25]}.get(kind, manifest)
for rel, text in files.items():
    os.makedirs(os.path.dirname(os.path.join(p, rel)), exist_ok=True)
    open(os.path.join(p, rel), "w").write(text)
os.makedirs(p, exist_ok=True)
if kind != f"no-manifest-{n}":
    open(os.path.join(p, "teq-products.json"), "w").write(manifest)
PY
  chmod +x "$tmp/teq"
  # Per case: its name, the census's exit, a text it must print, the mode, and its units, each a planted kind (one
  # given twice: its unit twice; `missing`: a directory that does not exist; `no-sources`: one without .scala files).
  while IFS='|' read -r name expect word m kinds; do
    units=()
    for k in $kinds; do
      units+=("$tmp/units/$name-$k")
      [ "$k" = missing ] || [ -d "$tmp/units/$name-$k" ] && continue
      mkdir -p "$tmp/units/$name-$k"
      echo "$k" > "$tmp/units/$name-$k/plant"
      [ "$k" = no-sources ] || echo 'object A' > "$tmp/units/$name-$k/A.scala"
    done
    got=$( (runs=2 mode=$m census "$tmp/teq" "$tmp/out/$name" "${units[@]}") 2>&1)
    rc=$?
    cases=$((cases + 1))
    if [ "$rc" = "$expect" ] && [[ "$got" == *"$word"* ]]; then
      good=$((good + 1))
    else
      echo "products-unique self-test: NOT AS EXPECTED: $name: exit $rc, $expect expected, \`$word\` expected; $(printf '%s' "$got" | tail -3 | tr '\n' ' ')"
    fi
  done <<'CASES'
passes|0|products-unique: 1 units, 1 built, 0 NOT JUDGED, 0 given twice; 0 shared paths; 0 units|check|good
passes-as-jvm|0|products-unique: 1 units, 1 built, 0 NOT JUDGED, 0 given twice; 0 shared paths; 0 units|jvm|good
two-good-units|0|products-unique: 2 units, 2 built, 0 NOT JUDGED|check|good other-good
shared-tasty|1|SHARED a/A.tasty (at 1 workers, run 1)|check|shared-tasty
shared-class|1|SHARED a/A.class (at 1 workers, run 1)|check|shared-class
differ-at-8-workers|1|DIFFER: 2 distinct product sets|check|differ-at-8
differ-in-run-2|1|DIFFER: 2 distinct product sets|check|differ-in-run-2
refuses-at-8-workers|1|DIFFER: its builds exit differently|check|refuses-at-8
does-not-build|0|does not build (A.scala:1:8: error: refused)|check|refuses
exit-1-without-its-error|1|NOT JUDGED: the build at 1 workers, run 1, exited 1 with no teq error or TASTy summary on its log (error: refused)|check|undiagnosed-at-1
exit-1-without-its-error-at-8|1|NOT JUDGED: the build at 8 workers, run 1, exited 1 with no teq error or TASTy summary on its log|check|undiagnosed-at-8
a-refusal-written-as-tasty-s|0|does not build (1 definition cannot be written as TASTy)|check|tasty-refusal
cannot-write-tasty-without-the-summary|1|NOT JUDGED: the build at 1 workers, run 1, exited 1 with no teq error or TASTy summary on its log (A.scala: cannot write TASTy for x)|check|tasty-without-the-summary
refused-by-the-jvm-backend|0|does not build (2 constructs not supported by the JVM backend)|jvm|jvm-backend-refusal
exit-3-without-its-line|1|NOT JUDGED: the build at 1 workers, run 1, exited 3 with no JVM backend summary on its log (something failed)|jvm|exit-3
exit-2-with-a-refusal-line|1|NOT JUDGED: the build at 1 workers, run 1, exited 2 (error: x/teq-products.json|check|exit-2-with-a-refusal-line
exit-2-at-8-workers|1|NOT JUDGED: the build at 8 workers, run 1, exited 2 (cannot open x|check|exit-2-at-8
exit-2-at-1-worker|1|NOT JUDGED: the build at 1 workers, run 1, exited 2|check|exit-2-at-1
killed-at-1-worker|1|NOT JUDGED: the build at 1 workers, run 1, exited 137 (signal 9)|check|killed-at-1
killed-at-8-workers|1|NOT JUDGED: the build at 8 workers, run 1, exited 137 (signal 9)|check|killed-at-8
crashed-at-8-workers|1|NOT JUDGED: the build at 8 workers, run 1, exited 139 (signal 11)|check|crashed-at-8
crashed-at-1-worker|1|NOT JUDGED: the build at 1 workers, run 1, exited 139 (signal 11)|check|crashed-at-1
aborts|1|NOT JUDGED: the build at 1 workers, run 1, exited 134 (signal 6) (fatal runtime error: stack overflow, aborting)|check|aborts
killed-at-every-build|1|NOT JUDGED: the build at 1 workers, run 1, exited 137 (signal 9)|check|killed
crashed-at-every-build|1|NOT JUDGED: the build at 1 workers, run 1, exited 139 (signal 11)|check|crashed
exit-2-at-every-build|1|NOT JUDGED: the build at 1 workers, run 1, exited 2|check|exit-2
exit-1-without-its-error-at-every-build|1|NOT JUDGED: the build at 1 workers, run 1, exited 1 with no teq error or TASTy summary on its log|check|undiagnosed
timeout-at-1-worker|1|NOT JUDGED: the build at 1 workers, run 1, passed 120 s|check|timeout-1
timeout-at-8-workers|1|NOT JUDGED: the build at 8 workers, run 1, passed 120 s|check|timeout-8
no-manifest-at-1-worker|1|NOT JUDGED: the build at 1 workers, run 1: no record of its teq-products.json|check|no-manifest-1
no-manifest-at-8-workers|1|NOT JUDGED: the build at 8 workers, run 1: no record of its teq-products.json|check|no-manifest-8
empty-manifest|1|NOT JUDGED: the build at 8 workers, run 1: its teq-products.json empty|check|empty-manifest-8
cut-manifest|1|NOT JUDGED: the build at 1 workers, run 1: its teq-products.json not whole JSON|check|cut-manifest-1
manifest-of-no-products|1|NOT JUDGED: the build at 1 workers, run 1: a product file its teq-products.json does not list: a/A.class|check|no-products
manifest-lists-a-missing-file|1|NOT JUDGED: the build at 1 workers, run 1: its teq-products.json lists a file that is not there: a/A.tasty|check|lists-missing
a-unit-that-defines-nothing|0|products-unique: 1 units, 1 built, 0 NOT JUDGED|check|nothing
given-twice|1|GIVEN TWICE|check|good good
no-such-unit|1|NOT JUDGED: no such directory|check|missing
no-sources|1|NOT JUDGED: no .scala files|check|no-sources
one-bad-of-two|1|SHARED a/A.tasty|check|good shared-tasty
CASES
  # The out directory's guard: refused where it is not absolute, is the root or a file, or holds files no census
  # wrote (its witness kept); a census's own directory is used again, and a unit that does not exist empties nothing
  # in it (the witness, a file the census did not write, kept in both).
  mkdir -p "$tmp/units/out-good" && echo good > "$tmp/units/out-good/plant" && echo 'object A' > "$tmp/units/out-good/A.scala"
  mkdir -p "$tmp/foreign" && echo kept > "$tmp/foreign/witness"
  touch "$tmp/a-file"
  mkdir -p "$tmp/out/own" && touch "$tmp/out/own/.products-unique-out" && echo kept > "$tmp/out/own/witness"
  out_case() { # out_case <name> <exit> <text> <out dir> <witness file, or -> <unit>...
    local name=$1 expect=$2 word=$3 dir=$4 witness=$5
    shift 5
    got=$( (cd "$tmp" && runs=2 mode=check census "$tmp/teq" "$dir" "$@") 2>&1)
    rc=$?
    cases=$((cases + 1))
    if [ "$rc" = "$expect" ] && [[ "$got" == *"$word"* ]] && { [ "$witness" = - ] || [ "$(cat "$witness" 2> /dev/null)" = kept ]; } && [ ! -e "$tmp/rel-out" ]; then
      good=$((good + 1))
    else
      echo "products-unique self-test: NOT AS EXPECTED: $name: exit $rc, $expect expected, \`$word\` expected, witness $witness; $(printf '%s' "$got" | tail -3 | tr '\n' ' ')"
    fi
  }
  out_case out-relative 1 "not an absolute path" rel-out - "$tmp/units/out-good"
  out_case out-root 1 "is the root" / - "$tmp/units/out-good"
  out_case out-a-file 1 "is not a directory" "$tmp/a-file" - "$tmp/units/out-good"
  out_case out-foreign 1 "holds files no census wrote" "$tmp/foreign" "$tmp/foreign/witness" "$tmp/units/out-good"
  out_case out-own-again 0 "1 units, 1 built" "$tmp/out/own" "$tmp/out/own/witness" "$tmp/units/out-good"
  out_case out-missing-unit 1 "NOT JUDGED: no such directory" "$tmp/out/own" "$tmp/out/own/witness" "$tmp/units/no-such"
  case "$tmp" in "${TMPDIR:-/tmp}"/products-unique-self-test.?*) rm -rf "$tmp" ;; esac
  echo "products-unique self-test: $cases cases, $good as expected"
  [ "$good" = "$cases" ]
}

if [ "${1:-}" = --self-test ]; then
  self_test
  exit
fi
self_test || { echo "products-unique: the census's self-test failed"; exit 1; }
census "$@"
