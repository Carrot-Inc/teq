#!/bin/bash
# The module harness: programs of tests/modules split
# into modules a, b and, for a diamond, c (b depends on a, c on a and b), each built in the
# product mode (`--products`) as one program (whole) and module by module against the
# upstream products (split). Every JVM build links against scala-library's jar, pinned on its
# class path, and every java run has the jar beside the classes. Per case:
#   classes  every class file and .tasty a module's products own is byte-identical between the
#            split and the whole build (the std classes the manifest marks are left out);
#   diags    a case with expect-errors.txt has b fail with the same diagnostics whole and split,
#            and those diagnostics are the file's;
#   manifest every product has its owning source, one of the module's, and a .tasty that one
#            of its class files names in a TASTY attribute; every class file has the Scala
#            marker (the manifest's contract, apart from how the classes were emitted);
#   subset   every module's sources built as one module into its own directory (on its class
#            path), then each module's sources rebuilt alone into a copy of it: the directory the
#            whole build's, files and manifest byte for byte, and with run.txt it runs as the
#            whole program; then the last module's sources removed by the manifest-only operation
#            (`--removed`), the directory a whole build's of the other sources (docs/TARGETS.md,
#            "The contract between the plugin and the compiler"); and, once per run, the own
#            directory named twice on the class path read once (`own-twice`), a definition
#            moved to another source keeping its products (`own-moved`), and a publication killed
#            after its files undone by the next build that reads the directory (`own-cut`);
#   check    the Scala.js module's check build over the same std (`teq compiler check --products
#            --std=scala-library`) writes the same .tasty as the JVM build;
#   check-lean  the Scala.js module's check build over teq's lean std, what a Scala.js module
#            ships: it builds, its manifest holds, and a .tasty it writes otherwise than the
#            check build over scala-library (the std's bindings differ: `Functor[List]`) has
#            scalac type the downstream modules against these pickles too; a downstream
#            whose check build fails with js-errors.txt's diagnostics (a macro reaching a
#            withheld body) fails as it must and has no lean products;
#   positions  (a case with positions.txt) the trees it lists, in its order, hold the spans it
#            gives them (scalac 3.8.4's) in the whole lean check build's pickles;
#   check-lean-whole  the lean check build of every module at once writes the pickles the
#            split lean check builds own, byte for byte;
#   run      a case with a `main` module builds it as a plain JVM program over the upstream
#            products and whole over every module's sources, and runs both: each prints the
#            case's run.txt;
#   scalac   scalac 3.8.4 types b (and c) against the upstream products directories
#            (`scala-cli compile -S 3.8.4 --classpath <dirs>`, -Werror), which is the writer's
#            validity oracle; a case whose b misuses a's API is refused there too;
#   readtasty scalac 3.8.4 reads every module's pickles as its own compilation units
#            (`-from-tasty`, tests/tasty/fromtasty/Gate.scala, all of them in one JVM): every
#            right-hand side forced and the tree checker run after readTasty (-Ycheck:all), the
#            upstream modules' products on the class path.
# A case with run.txt has an entry point in its last module, or in `main` over all of them,
# and each such case is checked by:
#   jvm-run  the downstream built as a plain JVM program over the upstream products and whole
#            over every module's sources, both run with java and scala-library's jar;
#   js-run   the downstream built for JavaScript over the upstream modules' Scala.js products
#            (`check --products`, the lean std) and whole, both run with node: each prints the
#            case's run.js.txt, else its run.txt (Scala.js's output where it is not the JVM's);
#            a case with js-errors.txt has its products build fail with those diagnostics,
#            and where they name a withheld body the interpreter's run over the products stops
#            there, printing nothing;
#   interp-run  the downstream in the interpreter over the same products and whole, the same
#            output and exit, which is run.txt's or run.js.txt's (a case without js-errors.txt;
#            the split programs too, against their .expected);
#   bundle   the two JavaScript builds byte-identical, as one file and split, in the development
#            and the release output, and under each line of bundle-flags.txt; no string of
#            absent.txt in either;
#   scalac-run  scalac 3.8.4 compiles the downstream over teq's products of the upstream, and
#            its classes run with java over teq's class files and scala-library: run.txt.
# Every case is checked by:
#   keys     the keys and tokens every product records are the whole build's, and no token is
#            two keys' (else its bundle leg is deferred as ineligible);
#   inits    what a JavaScript build over the case's Scala.js products derives from their bodies
#            (`TEQ_PRODUCT_INITS`) is each manifest's `inits`, the Scala.js and the JVM one.
# Every build of a case takes the case's directory as --sourceroot, or under
# sourceroots.txt `module` its module's.
# A check listed in tests/modules/deferred.txt (`<case> <check>  <reason>`) is one this stage
# leaves to a later one: run and reported, not counted as passed or failed.
# A check listed in tests/modules/known.txt (`<case> <check>  <reason>`) is a known departure:
# it is run and reported, and does not fail the suite. Without scala-cli the scalac checks are
# not made, which fails the suite as incomplete validation. tests/modules.sh <case>... runs only
# those cases. MODULES_OUT=<dir>
# keeps every build's products there, for comparing the products of two binaries.
cd "$(dirname "$0")/.."
root=$PWD
. tests/support/jars.sh
SL=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
if [ ! -f "$SL" ]; then
  echo "modules: scala-library 3.8.4 is not in the coursier cache, which every JVM build links against"
  exit 1
fi
teq=${TEQ:-./target/release/teq}
case $teq in /*) ;; *) teq=$root/$teq ;; esac
if [ -n "$MODULES_OUT" ]; then
  out=$MODULES_OUT
  rm -rf "$out" && mkdir -p "$out"
else
  out=$(mktemp -d "${TMPDIR:-/tmp}/teq-modules.XXXXXX")
  trap 'rm -rf "$out"' EXIT
fi
known=tests/modules/known.txt
deferred=tests/modules/deferred.txt
pass=0
fail=0
known_count=0
deferred_count=0
skipped=0
scalac_ok=0
scalac_refused=0
have_scalac=1
command -v scala-cli > /dev/null 2>&1 || have_scalac=0
have_java=1
command -v java > /dev/null 2>&1 || have_java=0

is_known() { [ -f "$known" ] && grep -q "^$1 $2 " "$known"; }
is_deferred() { [ -f "$deferred" ] && grep -q "^$1 $2 " "$deferred"; }
result() {
  local name=$1 check=$2 ok=$3 detail=$4
  if is_deferred "$name" "$check"; then
    deferred_count=$((deferred_count + 1))
    if [ "$ok" = 1 ]; then echo "deferred $name $check passes now: take it off $deferred"; else echo "deferred $name $check"; fi
    return
  fi
  if [ "$ok" = 1 ]; then
    pass=$((pass + 1))
    if is_known "$name" "$check"; then echo "known $name $check passes now: take it off $known"; fi
  elif is_known "$name" "$check"; then
    known_count=$((known_count + 1))
    echo "known $name $check"
  else
    fail=$((fail + 1))
    echo "FAIL $name $check"
    [ -n "$detail" ] && printf '%s\n' "$detail" | head -30
  fi
}

# The sources of a module, sorted.
sources() { find "$1" -name '*.scala' | LC_ALL=C sort; }

# The --sourceroot of a module's builds: the case's directory, or the module's own.
sroot() {
  if [ "$(cat "$dir/sourceroots.txt" 2> /dev/null)" = module ]; then echo "$dir/$1"; else echo "$dir"; fi
}

# Whether two pickles are the same apart from what a Scala.js check build records of its
# expansions alone (TeqOrigins kind 8's leaves and leaf tests): every byte
# but the UUID's and the origins section's, which is the last, and the origins as listed.
same_pickle() {
  cmp -s "$1" "$2" && return 0
  python3 - "$1" "$2" <<'PYEOF' || return 1
import sys
def parts(path):
    b = open(path, 'rb').read()
    pos = 4
    def nat():
        nonlocal pos
        x = 0
        while True:
            c = b[pos]; pos += 1
            x = (x << 7) | (c & 0x7f)
            if c & 0x80:
                return x
    nat(); nat(); nat()
    n = nat(); pos += n
    header = b[:pos]
    pos += 16
    n = nat(); names = b[pos:pos + n]; pos += n
    sections = []
    while pos < len(b):
        name = nat(); n = nat(); sections.append((name, b[pos:pos + n])); pos += n
    return header, names, sections[:-1]
sys.exit(0 if parts(sys.argv[1]) == parts(sys.argv[2]) else 1)
PYEOF
  [ "$(origins_listing "$1")" = "$(origins_listing "$2")" ]
}
origins_listing() { "$teq" tasty --trees "$1" 2> /dev/null | sed -n '/^TeqOrigins:/,$p' | sed -E 's/, leaves \[[0-9, ]*\]//; s/, leaf tests \[[0-9, ]*\]//'; }

# A TASTy file's key and token, as its TeqOrigins section records them.
origin_of() { "$teq" tasty --trees "$1" 2> /dev/null | grep -m1 '^TeqOrigins:' | sed 's/^TeqOrigins: version [0-9]*, //'; }

# The class a downstream's entry point is: `p.run` for `@main def run`, `p.Use` for an object
# with a main method.
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

# The files a module's products own: the manifest's class files and .tasty files.
owned() {
  python3 - "$1/teq-products.json" <<'EOF'
import json, sys
m = json.load(open(sys.argv[1]))
for p in m["products"]:
    if p.get("tasty"):
        print(p["tasty"])
    for c in p["classes"]:
        print(c)
EOF
}

# The manifest's contract, apart from how the class files were emitted: every product has the
# owning source, one of the module's; a product's .tasty is there and one of its class files
# carries its UUID in a TASTY attribute; every class file of the products has the Scala marker.
manifest_check() {
  python3 - "$1" "$2" <<'EOF'
import json, os, struct, sys
out, srcs = sys.argv[1], set(sys.argv[2].split("\n"))
m = json.load(open(os.path.join(out, "teq-products.json")))

def attributes(path):
    b = open(path, "rb").read()
    count = struct.unpack(">H", b[8:10])[0]
    pool, at, i = [None] * count, 10, 1
    while i < count:
        tag = b[at]
        at += 1
        if tag == 1:
            n = struct.unpack(">H", b[at:at + 2])[0]
            pool[i] = b[at + 2:at + 2 + n].decode("utf-8", "replace")
            at += 2 + n
        elif tag in (3, 4, 9, 10, 11, 12, 17, 18):
            at += 4
        elif tag in (5, 6):
            at += 8
            i += 1
        elif tag in (7, 8, 16, 19, 20):
            at += 2
        elif tag == 15:
            at += 3
        i += 1
    at += 6
    at += 2 + 2 * struct.unpack(">H", b[at:at + 2])[0]
    for _ in range(2):
        members = struct.unpack(">H", b[at:at + 2])[0]
        at += 2
        for _ in range(members):
            at += 6
            n = struct.unpack(">H", b[at:at + 2])[0]
            at += 2
            for _ in range(n):
                at += 6 + struct.unpack(">I", b[at + 2:at + 6])[0]
    out = {}
    for _ in range(struct.unpack(">H", b[at:at + 2])[0]):
        name, size = struct.unpack(">HI", b[at + 2:at + 8])
        out[pool[name]] = b[at + 8:at + 8 + size]
        at += 6 + size
    return out

def uuid(path):
    b = open(path, "rb").read()
    r = [4]
    def nat():
        x = 0
        while True:
            d = b[r[0]]
            r[0] += 1
            x = x * 128 + (d & 0x7f)
            if d & 0x80:
                return x
    nat(); nat(); nat()
    n = nat()
    r[0] += n
    return b[r[0]:r[0] + 16]

bad = []
for p in m["products"]:
    what = p.get("tasty") or ",".join(p["classes"])
    if p.get("source") not in srcs:
        bad.append(f"{what}: owning source {p.get('source')!r} is none of the module's")
    attrs = {c: attributes(os.path.join(out, c)) for c in p["classes"]}
    for c, a in attrs.items():
        if "Scala" not in a:
            bad.append(f"{c}: no Scala attribute")
    if p.get("tasty"):
        t = os.path.join(out, p["tasty"])
        if not os.path.exists(t):
            bad.append(f"{p['tasty']}: missing")
        elif attrs and not any(a.get("TASTY") == uuid(t) for a in attrs.values()):
            bad.append(f"{p['tasty']}: no class file carries its UUID")
print("\n".join(bad))
sys.exit(1 if bad else 0)
EOF
}

# The products scalac is given: a directory's own classes and pickles, without the std
# classes the manifest marks, which teq's reader leaves out too (they are teq's own std until
# the one-artifact std, and would stand for scala-library's classes of the same names).
view() {
  local dir=$1 dest=$2
  mkdir -p "$dest"
  (cd "$dir" && owned "$dir") | while IFS= read -r f; do
    mkdir -p "$dest/$(dirname "$f")"
    cp "$dir/$f" "$dest/$f"
  done
}

# scalac over a module's sources against the upstream products; prints its output.
scalac() {
  local cp=$1 dest=$2
  shift 2
  local ws=$out/ws-$RANDOM
  mkdir -p "$ws" "$dest"
  (cd "$ws" && COURSIER_MODE=offline timeout 300 scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline -q \
    --workspace "$ws" --scalac-option -Werror --classpath "$cp" -d "$dest" "$@" 2>&1)
}

cases=("$@")
every_case=""
if [ ${#cases[@]} -eq 0 ]; then
  every_case=1
  for d in tests/modules/*/; do cases+=("$(basename "$d")"); done
fi
for name in "${cases[@]}"; do
  dir=tests/modules/$name
  [ -d "$dir/a" ] || { echo "no case $name"; fail=$((fail + 1)); continue; }
  mods=(a)
  [ -d "$dir/b" ] && mods+=(b)
  [ -d "$dir/c" ] && mods+=(c)
  expect=$dir/expect-errors.txt
  lean_differs=""
  w=$out/$name/whole
  whole_srcs=()
  for m in "${mods[@]}"; do while IFS= read -r f; do whole_srcs+=("$f"); done < <(sources "$dir/$m"); done
  "$teq" compiler build --target jvm --products "$w" --sourceroot "$dir" --classpath "$SL" "${whole_srcs[@]}" > "$out/$name.whole.log" 2>&1
  whole_code=$?
  # Split: each module against its upstream modules' products.
  cp=""
  split_failed=""
  for m in "${mods[@]}"; do
    srcs=()
    while IFS= read -r f; do srcs+=("$f"); done < <(sources "$dir/$m")
    args=(compiler build --target jvm --products "$out/$name/$m" --sourceroot "$(sroot "$m")")
    mcp=$cp
    # A module whose case has `<m>-old` is built with the products of that older version of
    # its sources first on its class path, as zinc leaves an invalidated source's classes in
    # the module's own directory: the sources are what the build compiles.
    if [ -d "$dir/$m-old" ]; then
      old=()
      while IFS= read -r f; do old+=("$f"); done < <(sources "$dir/$m-old")
      oargs=(compiler build --target jvm --products "$out/$name/$m-old" --sourceroot "$(sroot "$m")" --classpath "$SL${cp:+:$cp}")
      "$teq" "${oargs[@]}" "${old[@]}" > "$out/$name.$m-old.log" 2>&1 || split_failed="$split_failed $m-old"
      mcp=$out/$name/$m-old${cp:+:$cp}
    fi
    args+=(--classpath "$SL${mcp:+:$mcp}")
    "$teq" "${args[@]}" "${srcs[@]}" > "$out/$name.$m.log" 2>&1 || split_failed="$split_failed $m"
    cp=${cp:+$cp:}$out/$name/$m
  done
  if [ -f "$expect" ]; then
    # b misuses a: both builds fail on b's errors, the same ones.
    ok=0
    detail=""
    if [ $whole_code -ne 0 ] && [ "$split_failed" = " b" ]; then
      if diff <(cat "$out/$name.whole.log") <(cat "$out/$name.b.log") > /dev/null && diff "$expect" "$out/$name.b.log" > /dev/null; then
        ok=1
      else
        detail=$(diff "$out/$name.whole.log" "$out/$name.b.log"; diff "$expect" "$out/$name.b.log")
      fi
    else
      detail="whole exit $whole_code, split failed in:$split_failed"
    fi
    result "$name" diags $ok "$detail"
  else
    if [ $whole_code -ne 0 ] || [ -n "$split_failed" ]; then
      result "$name" build 0 "$(cat "$out/$name.whole.log"; for m in $split_failed; do cat "$out/$name.$m.log"; done)"
      continue
    fi
    ok=1
    detail=""
    for m in "${mods[@]}"; do
      while IFS= read -r f; do
        if ! cmp -s "$out/$name/$m/$f" "$w/$f"; then
          ok=0
          detail="$detail$m: $f differs"$'\n'
        fi
      done < <(owned "$out/$name/$m")
    done
    # Modules rooted apart have keys the whole build does not: their products are their own.
    if [ "$(sroot a)" != "$dir" ]; then
      echo "ineligible $name classes: its modules are rooted apart, their keys not the whole build's"
    else
      result "$name" classes $ok "$detail"
    fi
    ok=1
    detail=""
    for m in "${mods[@]}"; do
      log=$(manifest_check "$out/$name/$m" "$(sources "$dir/$m")") || { ok=0; detail="$detail$m: $log"$'\n'; }
    done
    result "$name" manifest $ok "$detail"
    ok=1
    detail=""
    sub=$out/$name/subset
    rm -rf "$sub" && mkdir -p "$sub"
    main=$(main_class "${whole_srcs[@]}")
    if "$teq" compiler build --target jvm --products "$sub/whole" --sourceroot "$dir" --classpath "$sub/whole:$SL" "${whole_srcs[@]}" > "$out/$name.subset.log" 2>&1; then
      for m in "${mods[@]}"; do
        srcs=()
        while IFS= read -r f; do srcs+=("$f"); done < <(sources "$dir/$m")
        cp -R "$sub/whole" "$sub/$m"
        if ! "$teq" compiler build --target jvm --products "$sub/$m" --sourceroot "$dir" --classpath "$sub/$m:$SL" "${srcs[@]}" >> "$out/$name.subset.log" 2>&1; then
          ok=0
          detail="$detail$m alone: the build failed"$'\n'
          continue
        fi
        differs=$(diff -rq "$sub/whole" "$sub/$m" | sed "s|$sub/||g" | head -5)
        [ -z "$differs" ] || { ok=0; detail="$detail$m alone: $differs"$'\n'; }
        if [ -f "$dir/run.txt" ] && [ -n "$main" ]; then
          got=$(timeout 60 java -cp "$sub/$m:$SL" "$main" 2>&1)
          [ "$got" = "$(cat "$dir/run.txt")" ] || { ok=0; detail="$detail$m alone runs: $(head -3 <<< "$got")"$'\n'; }
        fi
      done
      if [ ${#mods[@]} -gt 1 ]; then
        last=${mods[${#mods[@]}-1]}
        rest=()
        removed=()
        for m in "${mods[@]}"; do
          while IFS= read -r f; do
            if [ "$m" = "$last" ]; then removed+=(--removed "$f"); else rest+=("$f"); fi
          done < <(sources "$dir/$m")
        done
        cp -R "$sub/whole" "$sub/removed"
        if "$teq" compiler build --target jvm --products "$sub/removed" "${removed[@]}" >> "$out/$name.subset.log" 2>&1 \
          && "$teq" compiler build --target jvm --products "$sub/rest" --sourceroot "$dir" --classpath "$sub/rest:$SL" "${rest[@]}" >> "$out/$name.subset.log" 2>&1; then
          differs=$(diff -rq "$sub/rest" "$sub/removed" | sed "s|$sub/||g" | head -5)
          [ -z "$differs" ] || { ok=0; detail="$detail$last removed: $differs"$'\n'; }
        else
          ok=0
          detail="$detail$last removed: a build failed"$'\n'
        fi
      fi
    else
      ok=0
      detail="the whole build failed: $(head -5 "$out/$name.subset.log")"
    fi
    result "$name" subset $ok "$detail"
    # The Scala.js module's check build over scala-library writes the same pickles.
    ok=1
    detail=""
    cp=""
    for m in "${mods[@]}"; do
      srcs=()
      while IFS= read -r f; do srcs+=("$f"); done < <(sources "$dir/$m")
      if ! "$teq" compiler check --products "$out/$name/js-$m" --sourceroot "$(sroot "$m")" --std=scala-library --classpath "$SL${cp:+:$cp}" "${srcs[@]}" > "$out/$name.js-$m.log" 2>&1; then
        ok=0
        detail="$detail$(cat "$out/$name.js-$m.log")"$'\n'
      fi
      cp=${cp:+$cp:}$out/$name/js-$m
      for t in $(cd "$out/$name/$m" && find . -name '*.tasty' | LC_ALL=C sort); do
        same_pickle "$out/$name/$m/$t" "$out/$name/js-$m/$t" || { ok=0; detail="$detail$m: $t differs from the check build's"$'\n'; }
      done
    done
    result "$name" check $ok "$detail"
    # The check build over the lean std: its own pickles, scalac's to judge where they differ.
    ok=1
    detail=""
    cp=""
    lean_differs=""
    # A downstream whose check build runs a macro into a body its upstream withholds fails as
    # its products build does, with js-errors.txt's diagnostics, and has no lean products.
    lean_failed=""
    for m in "${mods[@]}"; do
      srcs=()
      while IFS= read -r f; do srcs+=("$f"); done < <(sources "$dir/$m")
      args=(compiler check --products "$out/$name/jsl-$m" --sourceroot "$(sroot "$m")")
      [ -n "$cp" ] && args+=(--classpath "$cp")
      if ! "$teq" "${args[@]}" "${srcs[@]}" > "$out/$name.jsl-$m.log" 2>&1; then
        if [ -f "$dir/js-errors.txt" ] && [ "$(sed "s|$out|\$OUT|g" "$out/$name.jsl-$m.log")" = "$(cat "$dir/js-errors.txt")" ]; then
          lean_failed="$lean_failed $m "
          continue
        fi
        ok=0
        detail="$detail$(cat "$out/$name.jsl-$m.log")"$'\n'
        continue
      fi
      cp=${cp:+$cp:}$out/$name/jsl-$m
      log=$(manifest_check "$out/$name/jsl-$m" "$(sources "$dir/$m")") || { ok=0; detail="$detail$m: $log"$'\n'; }
      for t in $(cd "$out/$name/jsl-$m" && find . -name '*.tasty' | sort); do
        cmp -s "$out/$name/jsl-$m/$t" "$out/$name/js-$m/$t" || lean_differs="$lean_differs $m:$t"
      done
    done
    result "$name" check-lean $ok "$detail"
    # And the split lean products own what the whole lean check build writes, as the JVM's split
    # and whole products do.
    ok=1
    detail=""
    if ! "$teq" compiler check --products "$out/$name/jsl-whole" --sourceroot "$dir" "${whole_srcs[@]}" > "$out/$name.jsl-whole.log" 2>&1; then
      ok=0
      detail=$(cat "$out/$name.jsl-whole.log")
    else
      for m in "${mods[@]}"; do
        [[ "$lean_failed" == *" $m "* ]] && continue
        [ -f "$out/$name/jsl-$m/teq-products.json" ] || { ok=0; detail="$detail$m: no split lean products"$'\n'; continue; }
        while IFS= read -r f; do
          cmp -s "$out/$name/jsl-$m/$f" "$out/$name/jsl-whole/$f" || { ok=0; detail="$detail$m: $f differs from the whole lean check build's"$'\n'; }
        done < <(owned "$out/$name/jsl-$m")
      done
    fi
    if [ "$(sroot a)" != "$dir" ]; then
      echo "ineligible $name check-lean-whole: its modules are rooted apart, their keys not the whole build's"
    else
      result "$name" check-lean-whole $ok "$detail"
    fi
    if [ -f "$dir/positions.txt" ]; then
      ok=1
      detail=""
      for p in $(grep -v '^#' "$dir/positions.txt" | awk '{ print $1 }' | sort -u); do
        if ! "$teq" tasty --positions "$out/$name/jsl-whole/$p" > "$out/$name.positions" 2>&1; then
          ok=0
          detail="$detail$p: $(head -2 "$out/$name.positions")"$'\n'
          continue
        fi
        missing=$(python3 - "$out/$name.positions" "$dir/positions.txt" "$p" <<'PY'
import re, sys
got = [re.sub(r' point [0-9]+', '', l.split(' ', 1)[1]) for l in open(sys.argv[1]) if re.match(r'^[0-9]+ ', l)]
want = [l.split(' ', 1)[1].strip() for l in open(sys.argv[2]) if not l.startswith('#') and l.split(' ', 1)[0] == sys.argv[3]]
i = 0
for w in want:
    while i < len(got) and got[i].strip() != w:
        i += 1
    if i == len(got):
        print(w)
        break
    i += 1
PY
)
        [ -n "$missing" ] && { ok=0; detail="$detail$p: no $missing after the lines before it"$'\n'; }
      done
      result "$name" positions $ok "$detail"
    fi
  fi
  ineligible=""
  if [ ! -f "$expect" ] && [ -z "$split_failed" ] && [ $whole_code -eq 0 ]; then
    # keys: every pickle records the key and token of the whole build's, and no token is two
    # keys', which would make the case ineligible for the bundle's identity.
    ok=1
    detail=""
    : > "$out/$name.tokens"
    for m in "${mods[@]}"; do
      for pair in "$m:$w" "jsl-$m:$out/$name/jsl-whole"; do
        pdir=$out/$name/${pair%%:*}
        wdir=${pair#*:}
        [ -f "$pdir/teq-products.json" ] || continue
        for t in $(cd "$pdir" && find . -name '*.tasty' | LC_ALL=C sort); do
          o=$(origin_of "$pdir/$t")
          printf '%s
' "$o" >> "$out/$name.tokens"
          [ "$(sroot "$m")" = "$dir" ] || continue
          [ "$o" = "$(origin_of "$wdir/$t")" ] || { ok=0; detail="$detail$m: $t records $o, the whole build $(origin_of "$wdir/$t")"$'
'; }
        done
      done
    done
    collided=$(sed 's/.*key "\(.*\)", token \(.*\)/\2 \1/' "$out/$name.tokens" | LC_ALL=C sort -u | awk '{ if ($1 == last) print; last = $1 }')
    [ -n "$collided" ] && ineligible="tokens held by two keys: $collided"
    result "$name" keys $ok "$detail"
    # inits: what the bodies of the Scala.js products say against both manifests' inits.
    ok=1
    detail=""
    ldirs_all=""
    for m in "${mods[@]}"; do ldirs_all=${ldirs_all:+$ldirs_all:}$out/$name/jsl-$m; done
    printf 'package harnessinits\n' > "$out/$name/inits.scala"
    rm -f "$out/$name/inits.txt"
    TEQ_PRODUCT_INITS="$out/$name/inits.txt" "$teq" compiler check --sourceroot "$dir" --classpath "$ldirs_all" "$out/$name/inits.scala" > "$out/$name.inits.log" 2>&1
    for m in "${mods[@]}"; do
      derived=$(awk -F'\t' -v d="$out/$name/jsl-$m" '$1 == d { print $2 }' "$out/$name/inits.txt" 2> /dev/null | LC_ALL=C sort)
      for manifest in "$out/$name/jsl-$m/teq-products.json" "$out/$name/$m/teq-products.json"; do
        [ -f "$manifest" ] || continue
        listed=$(python3 -c 'import json, sys; print("\n".join(sorted(i for e in json.load(open(sys.argv[1]))["products"] for i in e["inits"])))' "$manifest" | LC_ALL=C sort)
        [ "$derived" = "$listed" ] || { ok=0; detail="$detail$m: the bodies give [$(echo $derived)], $manifest lists [$(echo $listed)]"$'\n'; }
      done
    done
    result "$name" inits $ok "$detail"
  fi
  # A case with run.txt runs its downstream, `main` over every module or else its last module
  # over the others: on the JVM, on JavaScript, and compiled by scalac over teq's products.
  down=""
  if [ -f "$dir/run.txt" ] && [ ! -f "$expect" ]; then
    if [ -d "$dir/main" ]; then down=main; else down=${mods[${#mods[@]}-1]}; fi
    ups=()
    for m in "${mods[@]}"; do [ "$m" != "$down" ] && ups+=("$m"); done
    down_srcs=()
    while IFS= read -r f; do down_srcs+=("$f"); done < <(sources "$dir/$down")
    all_srcs=("${whole_srcs[@]}")
    [ "$down" = main ] && all_srcs+=("${down_srcs[@]}")
    jdirs=""
    ldirs=""
    for m in "${ups[@]}"; do jdirs=${jdirs:+$jdirs:}$out/$name/$m; ldirs=${ldirs:+$ldirs:}$out/$name/jsl-$m; done
    expected=$(cat "$dir/run.txt")
    js_expected=$expected
    [ -f "$dir/run.js.txt" ] && js_expected=$(cat "$dir/run.js.txt")
    # jvm-run: a plain JVM program over the upstream products, and whole.
    if [ $have_java = 1 ]; then
      ok=1
      detail=""
      "$teq" compiler build --target jvm -o "$out/$name/whole.jar" --sourceroot "$dir" --classpath "$SL" "${all_srcs[@]}" > "$out/$name.run-whole.log" 2>&1 || { ok=0; detail="$(cat "$out/$name.run-whole.log")"; }
      "$teq" compiler build --target jvm -o "$out/$name/main.jar" --sourceroot "$(sroot "$down")" --classpath "$SL:$jdirs" "${down_srcs[@]}" > "$out/$name.run-split.log" 2>&1 || { ok=0; detail="$detail$(cat "$out/$name.run-split.log")"; }
      if [ $ok = 1 ]; then
        whole_out=$(timeout 60 java -cp "$out/$name/whole.jar:$SL" TeqMain 2>&1)
        split_out=$(timeout 60 java -cp "$out/$name/main.jar:$jdirs:$SL" TeqMain 2>&1)
        if [ "$whole_out" != "$expected" ] || [ "$split_out" != "$expected" ]; then
          ok=0
          detail="whole: $whole_out"$'\n'"split: $split_out"$'\n'"expected: $expected"
        fi
      fi
      result "$name" jvm-run $ok "$detail"
    fi
    # js-run: the downstream on JavaScript over the upstream's Scala.js products, and whole.
    ok=1
    detail=""
    js_failed=""
    "$teq" compiler build --sourceroot "$dir" "${all_srcs[@]}" -o "$out/$name/js-whole.js" > "$out/$name.js-whole.log" 2>&1 || { ok=0; detail="whole: $(cat "$out/$name.js-whole.log")"; }
    "$teq" compiler build --sourceroot "$(sroot "$down")" --classpath "$ldirs" "${down_srcs[@]}" -o "$out/$name/js-prod.js" > "$out/$name.js-prod.log" 2>&1 || js_failed=1
    if [ $ok = 1 ]; then
      whole_out=$(timeout 60 node "$out/$name/js-whole.js" 2>&1)
      [ "$whole_out" = "$js_expected" ] || { ok=0; detail="whole: $whole_out"$'\n'"expected: $js_expected"; }
    fi
    if [ -f "$dir/js-errors.txt" ]; then
      # The products build needs a body its upstream withholds, or would declare a name twice.
      got=$(sed "s|$out|\$OUT|g" "$out/$name.js-prod.log")
      if [ -z "$js_failed" ] || [ "$got" != "$(cat "$dir/js-errors.txt")" ]; then
        ok=0
        detail="$detail"$'\n'"products build (expected to fail with js-errors.txt): $got"
      fi
      # The interpreter stops where it needs the withheld body: nothing the program prints runs.
      if grep -q "is withheld from the products" "$dir/js-errors.txt"; then
        interp_out=$(timeout 60 "$teq" interp --sourceroot "$(sroot "$down")" --classpath "$ldirs" "${down_srcs[@]}" 2> "$out/$name.interp-prod.log")
        interp_code=$?
        if [ $interp_code != 1 ] || [ -n "$interp_out" ] || ! grep -q "is withheld from the products" "$out/$name.interp-prod.log"; then
          ok=0
          detail="$detail"$'\n'"interpreter over the products (expected to stop, exit 1, print nothing): exit $interp_code, printed: $interp_out"
        fi
      fi
    elif [ -n "$js_failed" ]; then
      ok=0
      detail="$detail"$'\n'"products: $(cat "$out/$name.js-prod.log")"
    else
      prod_out=$(timeout 60 node "$out/$name/js-prod.js" 2>&1)
      [ "$prod_out" = "$js_expected" ] || { ok=0; detail="$detail"$'\n'"products: $prod_out"$'\n'"expected: $js_expected"; }
    fi
    result "$name" js-run $ok "$detail"
    # interp-run: the downstream in the interpreter over the upstream's Scala.js products and
    # whole, the same output and exit, and that output the case's (run.txt, or run.js.txt).
    if [ ! -f "$dir/js-errors.txt" ]; then
      whole_i=$(timeout 60 "$teq" interp --sourceroot "$dir" "${all_srcs[@]}" 2>&1)
      whole_c=$?
      prod_i=$(timeout 60 "$teq" interp --sourceroot "$(sroot "$down")" --classpath "$ldirs" "${down_srcs[@]}" 2>&1)
      prod_c=$?
      ok=1
      [ "$whole_i" = "$prod_i" ] && [ $whole_c = 0 ] && [ $prod_c = 0 ] || ok=0
      [ "$whole_i" = "$expected" ] || [ "$whole_i" = "$js_expected" ] || ok=0
      result "$name" interp-run $ok "whole (exit $whole_c): $whole_i"$'\n'"products (exit $prod_c): $prod_i"$'\n'"expected: $expected"
    fi
    # bundle: the whole build's JavaScript and the products build's, byte for byte.
    if [ -n "$ineligible" ]; then
      deferred_count=$((deferred_count + 1))
      echo "ineligible $name bundle: $ineligible"
    elif [ ! -f "$dir/js-errors.txt" ] && [ -z "$js_failed" ]; then
      ok=1
      detail=""
      cmp -s "$out/$name/js-whole.js" "$out/$name/js-prod.js" || { ok=0; detail="one file: $(diff "$out/$name/js-whole.js" "$out/$name/js-prod.js" | head -8)"; }
      variants=("--release" "--split" "--release --split")
      while IFS= read -r line; do [ -n "$line" ] && variants+=("--split $line" "--release --split $line"); done < <(cat "$dir/bundle-flags.txt" 2> /dev/null)
      v=0
      for flags in "${variants[@]}"; do
        v=$((v + 1))
        wv=$out/$name/bundle-whole-$v
        pv=$out/$name/bundle-prod-$v
        case "$flags" in
          *--split*) wo=(${flags/--split/--split $wv}); po=(${flags/--split/--split $pv}) ;;
          *) wo=($flags -o $wv.js); po=($flags -o $pv.js) ;;
        esac
        "$teq" compiler build --sourceroot "$dir" "${wo[@]}" "${all_srcs[@]}" > "$out/$name.bundle-$v.log" 2>&1 || { ok=0; detail="$detail"$'\n'"$flags whole: $(cat "$out/$name.bundle-$v.log")"; continue; }
        "$teq" compiler build --sourceroot "$(sroot "$down")" "${po[@]}" --classpath "$ldirs" "${down_srcs[@]}" >> "$out/$name.bundle-$v.log" 2>&1 || { ok=0; detail="$detail"$'\n'"$flags products: $(cat "$out/$name.bundle-$v.log")"; continue; }
        if [ -d "$wv" ]; then
          diff -r "$wv" "$pv" > "$out/$name.bundle-$v.diff" 2>&1 || { ok=0; detail="$detail"$'\n'"$flags: $(head -8 "$out/$name.bundle-$v.diff")"; }
        else
          cmp -s "$wv.js" "$pv.js" || { ok=0; detail="$detail"$'\n'"$flags: $(diff "$wv.js" "$pv.js" | head -8)"; }
        fi
      done
      while IFS= read -r absent; do
        [ -n "$absent" ] || continue
        if grep -q -- "$absent" "$out/$name/js-prod.js" "$out/$name/js-whole.js"; then ok=0; detail="$detail"$'\n'"$absent is in the output"; fi
      done < <(cat "$dir/absent.txt" 2> /dev/null)
      result "$name" bundle $ok "$detail"
    fi
  fi
  if [ $have_scalac = 0 ]; then
    skipped=$((skipped + 1))
    continue
  fi
  # scalac types each downstream module against the upstream products.
  for m in "${mods[@]}"; do [ -d "$out/$name/$m" ] && view "$out/$name/$m" "$out/$name/view-$m"; done
  # scalac reads each module's pickles, the upstream modules' views on the class path.
  if [ ! -f "$expect" ]; then
    cp=""
    for m in "${mods[@]}"; do
      cp=${cp:+$cp:}$out/$name/view-$m
      tastys=$(cd "$out/$name/view-$m" && find . -name '*.tasty' | LC_ALL=C sort | sed "s|^\./|$out/$name/view-$m/|")
      [ -n "$tastys" ] && printf 'read\t%s\t%s\t%s\n' "$name readtasty-$m" "$cp" "$(printf '%s\t' $tastys | sed 's/\t$//')" >> "$out/fromtasty.jobs"
    done
  fi
  cp=$out/$name/view-a
  for m in "${mods[@]:1}"; do
    srcs=()
    while IFS= read -r f; do srcs+=("$root/$f"); done < <(sources "$dir/$m")
    log=$(scalac "$cp" "$out/$name/scalac-$m" "${srcs[@]}")
    code=$?
    if [ -f "$expect" ]; then
      [ $code -ne 0 ] && ok=1 || ok=0
      [ $ok = 1 ] && scalac_refused=$((scalac_refused + 1))
    else
      [ $code -eq 0 ] && ok=1 || ok=0
      [ $ok = 1 ] && scalac_ok=$((scalac_ok + 1))
    fi
    result "$name" "scalac-$m" $ok "$log"
    cp=$cp:$out/$name/view-$m
  done
  # scalac-run: scalac's classes of the downstream run over teq's class files of the upstream.
  if [ -n "$down" ] && [ $have_java = 1 ]; then
    ok=1
    detail=""
    if [ "$down" = main ]; then
      srcs=()
      for f in "${down_srcs[@]}"; do srcs+=("$root/$f"); done
      log=$(scalac "$cp" "$out/$name/scalac-main" "${srcs[@]}") || { ok=0; detail=$log; }
    fi
    if [ $ok = 1 ]; then
      main=$(main_class "${down_srcs[@]}")
      got=$(timeout 60 java -cp "$out/$name/scalac-$down:$jdirs:$SL" "$main" 2>&1)
      [ "$got" = "$expected" ] || { ok=0; detail="$main printed: $got"$'\n'"expected: $expected"; }
    fi
    result "$name" scalac-run $ok "$detail"
  fi
  # And against the lean check build's pickles where they are not the scala-library build's.
  [ -n "$lean_differs" ] || continue
  for m in "${mods[@]}"; do [ -d "$out/$name/jsl-$m" ] && view "$out/$name/jsl-$m" "$out/$name/viewl-$m"; done
  cp=$out/$name/viewl-a
  for m in "${mods[@]:1}"; do
    srcs=()
    while IFS= read -r f; do srcs+=("$root/$f"); done < <(sources "$dir/$m")
    log=$(scalac "$cp" "$out/$name/scalacl-$m" "${srcs[@]}")
    [ $? -eq 0 ] && ok=1 || ok=0
    [ $ok = 1 ] && scalac_ok=$((scalac_ok + 1))
    result "$name" "scalac-lean-$m" $ok "$log (the lean pickles that differ:$lean_differs)"
    cp=$cp:$out/$name/viewl-$m
  done
done
# The own directory named twice on the class path is read once, through its manifest: a source
# that no longer defines `Old` while naming it is refused, and the directory is left as it was.
if [ -n "$every_case" ]; then
  od=$out/own-twice
  mkdir -p "$od"
  printf 'package test\nobject Old:\n  def n: Int = 42\n' > "$od/A.scala"
  ok=1
  detail=""
  if "$teq" compiler build --target jvm --products "$od/out" --sourceroot "$od" --classpath "$od/out:$SL" "$od/A.scala" > "$od/log" 2>&1; then
    cp -R "$od/out" "$od/before"
    printf 'package test\nobject New:\n  def n: Int = Old.n\n' > "$od/A.scala"
    if "$teq" compiler build --target jvm --products "$od/out" --sourceroot "$od" --classpath "$od/out:$od/out:$SL" "$od/A.scala" >> "$od/log" 2>&1; then
      ok=0
      detail="the build naming its own removed object succeeded"$'\n'
    fi
    grep -q "not found: Old" "$od/log" || { ok=0; detail="$detail$(tail -3 "$od/log")"$'\n'; }
    diff -rq "$od/before" "$od/out" > /dev/null || { ok=0; detail="${detail}the directory changed"; }
  else
    ok=0
    detail="the first build failed: $(head -3 "$od/log")"
  fi
  result own-twice subset $ok "$detail"
  # A definition moved to another source keeps its products through the build of the source it
  # left, and through that source's removal: the build of the new owner alone, then of the old
  # one alone (or its manifest-only removal), and the downstream runs over the directory.
  od=$out/own-moved
  mkdir -p "$od"
  printf 'package move\nobject X { def n: Int = 42 }\nobject A\n' > "$od/A.scala"
  printf 'package move\nobject B\n' > "$od/B.scala"
  printf '@main def run(): Unit = println(move.X.n)\n' > "$od/Use.scala"
  ok=1
  detail=""
  own=(--target jvm --products "$od/out" --sourceroot "$od" --classpath "$od/out:$SL")
  if "$teq" compiler build "${own[@]}" "$od/A.scala" "$od/B.scala" > "$od/log" 2>&1; then
    printf 'package move\nobject A\n' > "$od/A.scala"
    printf 'package move\nobject X { def n: Int = 43 }\nobject B\n' > "$od/B.scala"
    "$teq" compiler build "${own[@]}" "$od/B.scala" >> "$od/log" 2>&1 || { ok=0; detail="${detail}the new owner's build failed"$'\n'; }
    cp -R "$od/out" "$od/removed"
    "$teq" compiler build "${own[@]}" "$od/A.scala" >> "$od/log" 2>&1 || { ok=0; detail="${detail}the old owner's build failed"$'\n'; }
    "$teq" compiler build --target jvm --products "$od/removed" --removed "$od/A.scala" >> "$od/log" 2>&1 || { ok=0; detail="${detail}the removal failed"$'\n'; }
    for d in out removed; do
      for f in move/X.class 'move/X$.class' move/X.tasty; do
        [ -f "$od/$d/$f" ] || { ok=0; detail="$detail$d: $f is gone"$'\n'; }
      done
    done
    if [ $have_java = 1 ] && "$teq" compiler build --target jvm -o "$od/use.jar" --classpath "$od/out:$SL" "$od/Use.scala" >> "$od/log" 2>&1; then
      got=$(timeout 60 java -cp "$od/use.jar:$od/out:$SL" run 2>&1)
      [ "$got" = 43 ] || { ok=0; detail="${detail}the downstream printed: $(head -3 <<< "$got")"$'\n'; }
    elif [ $have_java = 1 ]; then
      ok=0
      detail="${detail}the downstream's build failed: $(tail -3 "$od/log")"
    fi
  else
    ok=0
    detail="the first build failed: $(head -3 "$od/log")"
  fi
  result own-moved subset $ok "$detail"
  # A publication killed after its files and before its manifest (`TEQ_CUT_PUBLICATION`) is
  # undone by the next build that reads the directory: a downstream built then reads the last
  # whole publication's products, and the directory is that publication's byte for byte; the
  # build after it publishes the new ones.
  od=$out/own-cut
  mkdir -p "$od"
  printf 'package cut\nobject A:\n  def n: Int = 1\n' > "$od/A.scala"
  printf '@main def run(): Unit = println(cut.A.n)\n' > "$od/Use.scala"
  ok=1
  detail=""
  own=(--target jvm --products "$od/out" --sourceroot "$od" --classpath "$od/out:$SL")
  if "$teq" compiler build "${own[@]}" "$od/A.scala" > "$od/log" 2>&1; then
    cp -R "$od/out" "$od/before"
    printf 'package cut\nobject A:\n  def n: Int = 2\n' > "$od/A.scala"
    TEQ_CUT_PUBLICATION=1 "$teq" compiler build "${own[@]}" "$od/A.scala" >> "$od/log" 2>&1 && { ok=0; detail="${detail}the cut build exited 0"$'\n'; }
    [ -d "$od/out.teq-stage" ] || { ok=0; detail="${detail}the cut build left no staging directory"$'\n'; }
    if [ $have_java = 1 ]; then
      "$teq" compiler build --target jvm -o "$od/use.jar" --classpath "$od/out:$SL" "$od/Use.scala" >> "$od/log" 2>&1 || { ok=0; detail="${detail}the downstream's build failed"$'\n'; }
      diff -r "$od/before" "$od/out" > /dev/null && [ ! -d "$od/out.teq-stage" ] || { ok=0; detail="${detail}the directory is not the last publication's: $(diff -rq "$od/before" "$od/out" | head -3)"$'\n'; }
      got=$(timeout 60 java -cp "$od/use.jar:$od/out:$SL" run 2>&1)
      [ "$got" = 1 ] || { ok=0; detail="${detail}the downstream over the undone directory printed: $(head -3 <<< "$got")"$'\n'; }
      "$teq" compiler build "${own[@]}" "$od/A.scala" >> "$od/log" 2>&1 || { ok=0; detail="${detail}the next publication failed"$'\n'; }
      got=$(timeout 60 java -cp "$od/use.jar:$od/out:$SL" run 2>&1)
      [ "$got" = 2 ] || { ok=0; detail="${detail}the downstream over the next publication printed: $(head -3 <<< "$got")"$'\n'; }
    fi
  else
    ok=0
    detail="the first build failed: $(head -3 "$od/log")"
  fi
  result own-cut subset $ok "$detail"
fi
# The programs of tests/split as two modules (tests/modules/split-programs.txt), on JavaScript:
# the upstream's Scala.js products, the downstream built whole and over them.
if [ -n "$every_case" ] || [ -n "$SPLIT_PROGRAMS" ]; then
  while read -r prog upfiles; do
    case "$prog" in ''|'#'*) continue ;; esac
    sdir=tests/split/$prog
    name=split-$prog
    ups=()
    downs=()
    while IFS= read -r f; do
      if [[ " $upfiles " == *" $(basename "$f") "* ]]; then ups+=("$f"); else downs+=("$f"); fi
    done < <(sources "$sdir")
    # The program's own flags (`// teq:`), of its split output.
    split_flags=$(grep -h -o '^// teq: .*' "$sdir"/*.scala 2> /dev/null | head -1 | sed 's|^// teq: ||')
    flags=""
    expected=$(cat "tests/split/$prog.expected")
    mkdir -p "$out/$name"
    if ! "$teq" compiler check --products "$out/$name/up" --sourceroot "$sdir" "${ups[@]}" > "$out/$name.up.log" 2>&1; then
      result "$name" build 0 "$(cat "$out/$name.up.log")"
      continue
    fi
    ok=1
    detail=""
    "$teq" compiler build --sourceroot "$sdir" $flags "${ups[@]}" "${downs[@]}" -o "$out/$name/whole.js" > "$out/$name.whole.log" 2>&1 || { ok=0; detail="whole: $(cat "$out/$name.whole.log")"; }
    "$teq" compiler build --sourceroot "$sdir" $flags --classpath "$out/$name/up" "${downs[@]}" -o "$out/$name/prod.js" > "$out/$name.prod.log" 2>&1 || { ok=0; detail="$detail"$'\n'"products: $(cat "$out/$name.prod.log")"; }
    if [ $ok = 1 ]; then
      for b in whole prod; do
        got=$(cd "$out/$name" && timeout 60 node "$b.js" 2>&1)
        [ "$got" = "$expected" ] || { ok=0; detail="$detail"$'\n'"$b: $got"$'\n'"expected: $expected"; }
      done
    fi
    result "$name" js-run $ok "$detail"
    [ $ok = 1 ] || continue
    whole_i=$(timeout 60 "$teq" interp --sourceroot "$sdir" $flags "${ups[@]}" "${downs[@]}" 2>&1)
    whole_c=$?
    prod_i=$(timeout 60 "$teq" interp --sourceroot "$sdir" $flags --classpath "$out/$name/up" "${downs[@]}" 2>&1)
    prod_c=$?
    ok=1
    [ "$whole_i" = "$prod_i" ] && [ "$whole_i" = "$expected" ] && [ $whole_c = 0 ] && [ $prod_c = 0 ] || ok=0
    result "$name" interp-run $ok "whole (exit $whole_c): $whole_i"$'\n'"products (exit $prod_c): $prod_i"$'\n'"expected: $expected"
    ok=1
    detail=""
    cmp -s "$out/$name/whole.js" "$out/$name/prod.js" || { ok=0; detail="one file: $(diff "$out/$name/whole.js" "$out/$name/prod.js" | head -8)"; }
    v=0
    for vflags in "--release" "--split $split_flags" "--release --split $split_flags"; do
      v=$((v + 1))
      case "$vflags" in
        *--split*) wo=(${vflags/--split/--split $out/$name/whole-$v}); po=(${vflags/--split/--split $out/$name/prod-$v}) ;;
        *) wo=($vflags -o $out/$name/whole-$v.js); po=($vflags -o $out/$name/prod-$v.js) ;;
      esac
      "$teq" compiler build --sourceroot "$sdir" $flags "${wo[@]}" "${ups[@]}" "${downs[@]}" > /dev/null 2>&1
      "$teq" compiler build --sourceroot "$sdir" $flags "${po[@]}" --classpath "$out/$name/up" "${downs[@]}" > /dev/null 2>&1
      if [ -d "$out/$name/whole-$v" ]; then
        diff -r "$out/$name/whole-$v" "$out/$name/prod-$v" > "$out/$name.bundle-$v.diff" 2>&1 || { ok=0; detail="$detail"$'\n'"$vflags: $(head -8 "$out/$name.bundle-$v.diff")"; }
      else
        cmp -s "$out/$name/whole-$v.js" "$out/$name/prod-$v.js" || { ok=0; detail="$detail"$'\n'"$vflags: $(diff "$out/$name/whole-$v.js" "$out/$name/prod-$v.js" | head -8)"; }
      fi
    done
    result "$name" bundle $ok "$detail"
    # inits: the upstream's bodies against its manifest's.
    printf 'package harnessinits\n' > "$out/$name/inits.scala"
    TEQ_PRODUCT_INITS="$out/$name/inits.txt" "$teq" compiler check --sourceroot "$sdir" --classpath "$out/$name/up" "$out/$name/inits.scala" > /dev/null 2>&1
    derived=$(awk -F'\t' '{ print $2 }' "$out/$name/inits.txt" 2> /dev/null | LC_ALL=C sort)
    listed=$(python3 -c 'import json, sys; print("\n".join(sorted(i for e in json.load(open(sys.argv[1]))["products"] for i in e["inits"])))' "$out/$name/up/teq-products.json" | LC_ALL=C sort)
    [ "$derived" = "$listed" ] && ok=1 || ok=0
    result "$name" inits $ok "the bodies give [$(echo $derived)], the manifest lists [$(echo $listed)]"
  done < tests/modules/split-programs.txt
fi
# The reads of every case's pickles, in one JVM.
if [ $have_scalac = 1 ] && [ -s "$out/fromtasty.jobs" ]; then
  (cd "$out" && COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
    "$root/tests/tasty/fromtasty/Gate.scala" --dep org.scala-lang:scala3-compiler_3:3.8.4 -- fromtasty.jobs > fromtasty.out 2> fromtasty.err)
  while IFS=$'\t' read -r _ job _; do
    jcase=${job%% *}
    jcheck=${job#* }
    if grep -qxF "ok $job" "$out/fromtasty.out"; then
      result "$jcase" "$jcheck" 1 ""
    else
      detail=$(awk -v j="FAIL $job" '$0 == j { on = 1; next } /^(ok|FAIL) / { on = 0 } on' "$out/fromtasty.out")
      [ -n "$detail" ] || detail="no answer from the reader of scalac: $(grep -v hint "$out/fromtasty.err" | head -3)"
      result "$jcase" "$jcheck" 0 "$detail"
    fi
  done < "$out/fromtasty.jobs"
fi
note=""
if [ $skipped -gt 0 ]; then
  note=" (incomplete validation: no scala-cli, so scalac checked none of $skipped cases)"
  fail=$((fail + 1))
fi
echo "modules: $pass passed, $fail failed, $known_count known departures, $deferred_count deferred; scalac typed $scalac_ok modules and refused $scalac_refused misuses as expected$note"
[ $fail -eq 0 ]
