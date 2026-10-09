#!/bin/bash
# Golden tests of the TASTy reader: `teq tasty` over the fixtures under tests/tasty/fixtures
# (compiled from tests/tasty/src with Scala 3.8.4; see tests/tasty/README) has to print what
# tests/tasty/expected holds, and `teq tasty --body` the bodies of tests/tasty/src/bodies.scala.
# `tests/tasty.sh --update` rewrites the expectations.
cd "$(dirname "$0")/.."
teq=${TEQ:-./target/release/teq}
pass=0
fail=0
mkdir -p tests/tasty/expected
for f in tests/tasty/fixtures/*.tasty; do
  name=$(basename "$f" .tasty)
  expected="tests/tasty/expected/$name.txt"
  actual=$("$teq" tasty "$f" 2>&1)
  if [ "$1" = "--update" ]; then
    printf '%s\n' "$actual" > "$expected"
    continue
  fi
  if [ "$actual" == "$(cat "$expected" 2>/dev/null)" ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL $name"
    diff <(printf '%s\n' "$actual") "$expected" | head -20
  fi
done
# Bodies: `teq tasty --body` of the definitions of tests/tasty/src/bodies.scala, compared with
# tests/tasty/expected/bodies/<name>.txt.
mkdir -p tests/tasty/expected/bodies
for name in Bodies Base Point Pretty; do
  expected="tests/tasty/expected/bodies/$name.txt"
  actual=$("$teq" tasty --body "tests/tasty/fixtures/$name.tasty" "fix.bodies.$name" 2>&1)
  if [ "$1" = "--update" ]; then
    printf '%s\n' "$actual" > "$expected"
    continue
  fi
  if [ "$actual" == "$(cat "$expected" 2>/dev/null)" ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL body $name"
    diff <(printf '%s\n' "$actual") "$expected" | head -20
  fi
done
# The capture of the bodies' typed form: the census of the product
# check of each program of tests/tasty/capture (a file, or a directory of files, whose macros
# another file than theirs calls) and of tests/tasty/src/bodies.scala has to be
# tests/tasty/capture/<name>.census, which holds no missing construct and no orphan (the records
# dropped with discarded typings are left out of the comparison). A directory's is checked at one
# worker and at four, with the merge's check on: the capture is the same however the files are
# typed.
capture_tmp=$(mktemp -d "${TMPDIR:-/tmp}/tasty-capture.XXXXXX" 2> /dev/null) && [ -d "$capture_tmp" ] || { echo "tasty: no temporary directory for the capture's census"; exit 1; }
# The products of the capture's programs and of tests/tasty/probes, kept for scalac's read and the
# bodies' census below.
extra=$(mktemp -d "${TMPDIR:-/tmp}/tasty-extra.XXXXXX" 2> /dev/null) && [ -d "$extra" ] || { echo "tasty: no temporary directory for the capture's products"; exit 1; }
extra_units=()
for src in tests/tasty/capture/*.scala tests/tasty/capture/*/ tests/tasty/src/bodies.scala; do
  src=${src%/}
  [ -e "$src" ] || continue
  name=$(basename "$src" .scala)
  expected="tests/tasty/capture/$name.census"
  threads=1
  [ -d "$src" ] && threads="1 4"
  for t in $threads; do
    rm -rf "$capture_tmp/p" "$capture_tmp/census"
    TEQ_BODIES_CENSUS=$capture_tmp/bodies TEQ_CAPTURE_CENSUS=$capture_tmp/census timeout 60 "$teq" compiler check --products "$capture_tmp/p" "$src" --threads $t > "$capture_tmp/log" 2>&1
    built=$?
    if [ $built != 0 ]; then
      fail=$((fail + 1))
      echo "FAIL capture $name at $t worker(s): the product check fails: $(head -2 "$capture_tmp/log")"
    elif [ $t = 1 ]; then
      cp -R "$capture_tmp/p" "$extra/capture-$name"
      cat "$capture_tmp/bodies" >> "$extra/bodies.census" 2> /dev/null
      extra_units+=("capture-$name")
    fi
    rm -f "$capture_tmp/bodies"
    actual=$(tests/support/capture-census.sh "$capture_tmp/census" 2>&1 | sed 's/^discarded records [0-9]*, //')
    if [ "$1" = "--update" ]; then
      printf '%s\n' "$actual" > "$expected"
      break
    fi
    if [ "$actual" == "$(cat "$expected" 2>/dev/null)" ] && ! grep -q '^missing\|^orphans:' "$expected" && ! grep -q 'the merge left' "$capture_tmp/log"; then
      pass=$((pass + 1))
    else
      fail=$((fail + 1))
      echo "FAIL capture $name at $t worker(s)"
      grep -m3 'the merge left' "$capture_tmp/log"
      diff <(printf '%s\n' "$actual") "$expected" | head -20
    fi
  done
done
# The parts the capture keeps of each interpolation of tests/tasty/src/interparts.scala have to
# be the ones scalac 3.8.4 pickles in its `StringContext.apply` (the fixture InterpParts.tasty):
# as written, `$$` and unicode escapes aside.
rm -rf "$capture_tmp/p" "$capture_tmp/parts"
TEQ_CAPTURE_CENSUS=$capture_tmp/census TEQ_CAPTURE_PARTS=$capture_tmp/parts timeout 60 "$teq" compiler check --products "$capture_tmp/p" tests/tasty/src/interparts.scala > /dev/null 2>&1
compared=0
while IFS=$'\t' read -r def captured; do
  pickled=$("$teq" tasty --body tests/tasty/fixtures/InterpParts.tasty "fix.interparts.InterpParts.$def" 2>&1 | sed -n 's/.*StringContext\.apply(\(\[[^*]*\]\)\*).*/\1/p' | head -1)
  if [ "$captured" == "$pickled" ]; then
    compared=$((compared + 1))
  else
    fail=$((fail + 1))
    echo "FAIL capture interpolation parts of $def: $captured, scalac's $pickled"
  fi
done < <(cat "$capture_tmp/parts" 2> /dev/null)
if [ "$compared" -gt 0 ] && [ "$compared" == "$(grep -c '' "$capture_tmp/parts")" ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL capture interpolation parts: $compared of $(grep -c '' "$capture_tmp/parts" 2> /dev/null) as scalac pickles them"
fi
rm -rf "$capture_tmp"
# The worker comparison refuses to pass without the storage it works in or without the census of
# a capture fixture, rather than count the programs left out.
if [ "$1" != "--update" ]; then
  if TMPDIR=/nonexistent/capture-census tests/capture-census.sh --workers > /dev/null 2>&1; then
    fail=$((fail + 1))
    echo "FAIL capture census without a temporary directory: passed"
  else
    pass=$((pass + 1))
  fi
  if TEQ=$(command -v false) tests/capture-census.sh --workers > /dev/null 2>&1; then
    fail=$((fail + 1))
    echo "FAIL capture census without the fixtures' censuses: passed"
  else
    pass=$((pass + 1))
  fi
fi
# Every body of scala-library, cats-kernel and cats-core has to decode without an error; a jar
# not in the coursier cache fails the check as incomplete validation.
M2=${COURSIER_CACHE:-$HOME/Library/Caches/Coursier/v1}/https/repo1.maven.org/maven2
for jar in org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar \
           org/typelevel/cats-kernel_3/2.13.0/cats-kernel_3-2.13.0.jar \
           org/typelevel/cats-core_3/2.13.0/cats-core_3-2.13.0.jar; do
  [ "$1" = "--update" ] && break
  if [ ! -f "$M2/$jar" ]; then
    fail=$((fail + 1))
    echo "FAIL bodies: no $M2/$jar in the coursier cache (incomplete validation)"
    continue
  fi
  if out=$(timeout 120 "$teq" tasty --bodies "$M2/$jar" 2>&1) && grep -q " 0 errors$" <<< "$out"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL bodies $(basename "$jar")"
    echo "$out" | head -20
  fi
done
# izumi-reflect's classes enter through the typer without an error: `Inspector`'s overloads told
# apart by `@targetName` and `FullReference`'s two `copy$default$1` are what scalac accepted.
izumi="$M2/dev/zio/izumi-reflect_sjs1_3/3.0.9/izumi-reflect_sjs1_3-3.0.9.jar"
if [ "$1" != "--update" ] && [ ! -f "$izumi" ]; then
  fail=$((fail + 1))
  echo "FAIL load: no $izumi in the coursier cache (incomplete validation)"
elif [ "$1" != "--update" ]; then
  if out=$(timeout 60 "$teq" tasty --load "$izumi" 2>&1) && grep -q " 0 errors in " <<< "$out"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL load $(basename "$izumi")"
    echo "$out" | head -20
  fi
fi
# The reader's listing: what the typer resolved of the fixtures'
# bodies (the classes of fix.*) each reader program of tests/classpath/js reaches, the
# expansions' traces and the std adaptations, sorted, has to be
# tests/tasty/expected/reader/<program>[.<std mode>].txt, and the same at four workers with the
# merge's check on.
. tests/support/jars.sh
mkdir -p tests/tasty/expected/reader
reader_tmp=$(mktemp -d "${TMPDIR:-/tmp}/tasty-reader.XXXXXX") || { echo "tasty: no temporary directory for the reader's listing"; exit 1; }
for src in tests/classpath/js/reader_*.scala; do
  name=$(basename "$src" .scala)
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    fail=$((fail + 1))
    echo "FAIL reader $name: not in the coursier cache:$JARS_MISSING (incomplete validation)"
    continue
  fi
  modes=$(grep -h -o '^// std: .*' "$src" | head -1 | sed 's|^// std: ||')
  for mode in ${modes:-lean}; do
    expected="tests/tasty/expected/reader/$name.txt"
    [ "$mode" = lean ] || expected="tests/tasty/expected/reader/$name.$mode.txt"
    for threads in 1 4; do
      rm -f "$reader_tmp/dump"
      TEQ_READER_DUMP="$reader_tmp/dump" timeout 60 "$teq" compiler build "$src" --std="$mode" --classpath "$JARS_CP" --threads "$threads" -o "$reader_tmp/out.js" > "$reader_tmp/log" 2>&1
      status=$?
      actual=$(grep -E $'^[a-z]+\tfix\\.' "$reader_tmp/dump" 2> /dev/null | LC_ALL=C sort -u)
      # A listing counts only from a build that succeeded and wrote one.
      if [ "$status" -ne 0 ] || [ -z "$actual" ]; then
        fail=$((fail + 1))
        echo "FAIL reader $name ($mode, $threads workers): exit $status, $(cat "$reader_tmp/dump" 2> /dev/null | grep -c .) listing lines: $(grep -m1 . "$reader_tmp/log")"
        continue
      fi
      if [ "$1" = "--update" ]; then
        [ "$threads" = 1 ] && printf '%s\n' "$actual" > "$expected"
        continue
      fi
      if [ "$actual" == "$(cat "$expected" 2> /dev/null)" ]; then
        pass=$((pass + 1))
      else
        fail=$((fail + 1))
        echo "FAIL reader $name ($mode, $threads workers)"
        diff <(printf '%s\n' "$actual") "$expected" | head -20
      fi
    done
  done
done
rm -rf "$reader_tmp"
# The reader over teq's products: the same programs with the
# declarations fix.reader's bodies reach (reader_decl.scala and reader_java.scala, fix.rdecl and
# fix.rjava) pickled by teq in a jar in the place of scalac's, fix.reader staying scalac's (teq
# does not type reader.scala: RdPick.same's `put(b, b.get)`). Their bodies are decoded and
# converted as a jar's: the listing has to differ from the program's pinned one by what
# tests/tasty/expected/reader/<program>[.<std mode>].products.diff records, each line a row of the
# section's table, and a program the JavaScript target runs prints its .expected.
. tests/support/jars.sh
jars_of tests/classpath/js/reader_java.scala
javafix=$(printf '%s\n' "$JARS_CP" | tr ':' '\n' | grep -m1 javafix)
products_tmp=$(mktemp -d "${TMPDIR:-/tmp}/tasty-products.XXXXXX") || { echo "tasty: no temporary directory for the reader over products"; exit 1; }
teq_abs=$(cd "$(dirname "$teq")" && pwd)/$(basename "$teq")
if ! (cd tests/tasty/src && "$teq_abs" compiler check --products "$products_tmp/decl" reader_decl.scala reader_java.scala --classpath "$javafix") > "$products_tmp/decl.log" 2>&1; then
  fail=$((fail + 1))
  echo "FAIL reader products: $(head -3 "$products_tmp/decl.log")"
else
  (cd "$products_tmp/decl" && python3 -c "
import os, zipfile
with zipfile.ZipFile('../decl.jar', 'w') as z:
    for d, _, fs in sorted(os.walk('.')):
        for f in sorted(fs):
            z.write(os.path.join(d, f), os.path.relpath(os.path.join(d, f), '.'))")
  for src in tests/classpath/js/reader_*.scala; do
    name=$(basename "$src" .scala)
    jars_of "$src"
    cp=$(printf '%s\n' "$JARS_CP" | tr ':' '\n' | grep -v fixtures-rdecl | tr '\n' ':')$products_tmp/decl.jar
    modes=$(grep -h -o '^// std: .*' "$src" | head -1 | sed 's|^// std: ||')
    targets=$(grep -h -o '^// targets: .*' "$src" | head -1)
    for mode in ${modes:-lean}; do
      listing="tests/tasty/expected/reader/$name.txt"
      expected="tests/tasty/expected/reader/$name.products.diff"
      [ "$mode" = lean ] || { listing="tests/tasty/expected/reader/$name.$mode.txt"; expected="tests/tasty/expected/reader/$name.$mode.products.diff"; }
      rm -f "$products_tmp/dump"
      TEQ_READER_DUMP="$products_tmp/dump" timeout 60 "$teq" compiler build "$src" --std="$mode" --classpath "$cp" -o "$products_tmp/out.js" > "$products_tmp/log" 2>&1
      status=$?
      actual=$(grep -E $'^[a-z]+\tfix\\.' "$products_tmp/dump" 2> /dev/null | LC_ALL=C sort -u)
      if [ "$status" -ne 0 ] || [ -z "$actual" ]; then
        fail=$((fail + 1))
        echo "FAIL reader products $name ($mode): exit $status: $(grep -m1 . "$products_tmp/log")"
        continue
      fi
      differs=$(diff <(printf '%s\n' "$actual") "$listing")
      if [ "$1" = "--update" ]; then
        if [ -n "$differs" ]; then printf '%s\n' "$differs" > "$expected"; else rm -f "$expected"; fi
      elif [ "$differs" == "$(cat "$expected" 2> /dev/null)" ]; then
        pass=$((pass + 1))
      else
        fail=$((fail + 1))
        echo "FAIL reader products $name ($mode)"
        diff <(printf '%s\n' "$differs") "$expected" | head -20
      fi
      [ -z "$targets" ] || [[ "$targets " == *" js "* ]] || continue
      [ "$1" = "--update" ] && continue
      if timeout 30 node "$products_tmp/out.js" > "$products_tmp/out.txt" 2>&1 && cmp -s "$products_tmp/out.txt" "tests/classpath/js/$name.expected"; then
        pass=$((pass + 1))
      else
        fail=$((fail + 1))
        echo "FAIL reader products $name ($mode): the program's output"
        diff "$products_tmp/out.txt" "tests/classpath/js/$name.expected" | head -5
      fi
    done
  done
fi
rm -rf "$products_tmp"
# The probes of tests/tasty/probes, built in the product mode as the capture's programs above, and
# the bodies' census of both (TEQ_BODIES_CENSUS, every producer and every reason a body is
# withheld, summed) has to be tests/tasty/expected/bodies.census: a body that the writer starts or
# stops withholding there shows.
for src in tests/tasty/probes/*.scala; do
  name=$(basename "$src" .scala)
  if TEQ_BODIES_CENSUS=$extra/bodies.census timeout 60 "$teq" compiler check --products "$extra/probe-$name" "$src" > "$extra/probe-$name.log" 2>&1; then
    extra_units+=("probe-$name")
  else
    fail=$((fail + 1))
    echo "FAIL probe $name: the product check fails: $(head -2 "$extra/probe-$name.log")"
  fi
done
census_actual=$(tests/support/bodies-census.sh "$extra/bodies.census" 2> /dev/null)
if [ "$1" = "--update" ]; then
  printf '%s\n' "$census_actual" > tests/tasty/expected/bodies.census
elif [ "$census_actual" == "$(cat tests/tasty/expected/bodies.census 2> /dev/null)" ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL bodies census of the capture's programs and the probes"
  diff <(printf '%s\n' "$census_actual") tests/tasty/expected/bodies.census | head -10
fi
# The twin: the fixtures' sources compiled by teq's writer (`teq compiler check --products`) and every
# pickle that has a fixture of its name printed beside scalac's, both by `teq tasty`; the
# difference of the two printouts has to be what tests/tasty/twin/<name>.diff records, each
# line a known difference of the writer's. The sources compiled with Scala.js or an
# older Scala (tests/tasty/README) are left aside, and so are those teq does not type, which
# tests/tasty/twin/untyped.txt lists with the reason. Every fixture of a twinned source has a
# pickle of teq's by its name, or unproduced.txt gives the reason. `tests/tasty.sh --update`
# rewrites the diffs, to be read against that classification.
twin=$(mktemp -d "${TMPDIR:-/tmp}/teq-twin.XXXXXX")
untyped=tests/tasty/twin/untyped.txt
unproduced=tests/tasty/twin/unproduced.txt
mkdir -p tests/tasty/twin
# The source each fixture was compiled from, by the file name its pickle records: every fixture
# of a twinned source has to be among teq's pickles of it, or be listed in unproduced.txt with
# the reason.
for f in tests/tasty/fixtures/*.tasty; do
  echo "$(basename "$f" .tasty) $("$teq" tasty --names "$f" | awk '$2 == "simple" && $3 ~ /\.scala$/ { print $3; exit }')"
done > "$twin/sources"
twin_units=()
for src in tests/tasty/src/*.scala; do
  name=$(basename "$src" .scala)
  # reader.scala's bodies were compiled against reader_decl_v1.scala, an older version of what
  # the fixtures hold: no unit teq compiles is the one scalac pickled.
  # reader_java.scala and its first version read tests/classfile's Java classes, which the twin's
  # compilation has no class path for.
  case $name in quotepat|quotepat_calls|facades|jsstack|unmod|unmod33|effects|pext_dsl|reader|reader_decl_v1|reader_java|reader_java_v1) continue ;; esac
  grep -q "^$name " "$untyped" 2> /dev/null && continue
  srcs=("$src")
  # pext_dsl exports from pext_inlined: the two are one unit, as scalac compiled them.
  [ "$name" = pext_inlined ] && srcs+=(tests/tasty/src/pext_dsl.scala)
  if ! "$teq" compiler check --products "$twin/$name" "${srcs[@]}" > "$twin/$name.log" 2>&1; then
    fail=$((fail + 1))
    echo "FAIL twin $name: $(head -3 "$twin/$name.log")"
    continue
  fi
  twin_units+=("$name")
  for src in "${srcs[@]}"; do
    for stem in $(awk -v s="$(basename "$src")" '$2 == s { print $1 }' "$twin/sources"); do
      [ -n "$(find "$twin/$name" -name "$stem.tasty")" ] && continue
      if grep -q "^$stem " "$unproduced" 2> /dev/null; then
        continue
      fi
      fail=$((fail + 1))
      echo "FAIL twin $name: scalac's $stem.tasty has no counterpart among teq's pickles"
    done
  done
done
# facades.scala and unmod33.scala, whose fixtures scalac compiled for Scala.js and with Scala 3.3,
# have no printout to be compared with: their pickles are pinned and read by scalac below like the
# twin's, as scalac compiling for Scala.js reads them (`-scalajs`, under which scalajs-library's
# `js.|` is a union type) with scalajs-library on the class path, which their `js.native`
# bodies name.
js_units=()
for name in facades unmod33; do
  if ! "$teq" compiler check --products "$twin/$name" "tests/tasty/src/$name.scala" > "$twin/$name.log" 2>&1; then
    fail=$((fail + 1))
    echo "FAIL twin $name: $(head -3 "$twin/$name.log")"
    continue
  fi
  js_units+=("$name")
done
for name in "${twin_units[@]}"; do
  for t in $(cd "$twin/$name" && find . -name '*.tasty' | LC_ALL=C sort); do
    stem=$(basename "$t" .tasty)
    if [ ! -f "tests/tasty/fixtures/$stem.tasty" ]; then
      fail=$((fail + 1))
      echo "FAIL twin $name: teq's $stem.tasty has no scalac fixture to be compared with"
      continue
    fi
    expected="tests/tasty/twin/$stem.diff"
    actual=$(diff <("$teq" tasty "tests/tasty/fixtures/$stem.tasty" | tail -n +2 | sed 's/<body@[0-9]*>/<body>/g') \
                  <("$teq" tasty "$twin/$name/$t" | tail -n +2 | sed 's/<body@[0-9]*>/<body>/g'))
    if [ "$1" = "--update" ]; then
      if [ -n "$actual" ]; then printf '%s\n' "$actual" > "$expected"; else rm -f "$expected"; fi
    elif [ "$actual" == "$(cat "$expected" 2> /dev/null)" ]; then
      pass=$((pass + 1))
    else
      fail=$((fail + 1))
      echo "FAIL twin $stem"
      diff <(printf '%s\n' "$actual") "$expected" | head -20
    fi
    # The forms: the bodies of the two pickles as `teq tasty
    # --body` prints them, normalised under the recorded equivalences
    # (tests/tasty/forms/normalize.py); what differs still is pinned in <stem>.forms.diff, whose
    # every line the section's table classifies.
    forms="tests/tasty/twin/$stem.forms.diff"
    pkg=$("$teq" tasty "tests/tasty/fixtures/$stem.tasty" | sed -n 's/^package //p' | head -1)
    qualified=$stem
    [ -n "$pkg" ] && [ "$pkg" != "<empty>" ] && qualified=$pkg.$stem
    forms_actual=$(diff <("$teq" tasty --body --signatures "tests/tasty/fixtures/$stem.tasty" "$qualified" 2>&1 | tail -n +2 | python3 tests/tasty/forms/normalize.py) \
                        <("$teq" tasty --body --signatures "$twin/$name/$t" "$qualified" 2>&1 | tail -n +2 | python3 tests/tasty/forms/normalize.py))
    if [ "$1" = "--update" ]; then
      if [ -n "$forms_actual" ]; then printf '%s\n' "$forms_actual" > "$forms"; else rm -f "$forms"; fi
    elif [ "$forms_actual" == "$(cat "$forms" 2> /dev/null)" ]; then
      pass=$((pass + 1))
    else
      fail=$((fail + 1))
      echo "FAIL twin forms $stem"
      diff <(printf '%s\n' "$forms_actual") "$forms" | head -20
    fi
  done
done
# The UUIDs: every pickle's is TastyPickler's hash of its sections, which tests/tasty/origins/check.py
# recomputes (and checks on scalac's fixtures as well), and the twin's pickles' are pinned in
# tests/tasty/twin/uuids.txt, so that a change of the bytes teq writes is one on purpose; a second
# build of every twinned source writes the same bytes, and so does a build of bodies.scala beside a
# file it does not use.
# In the C locale's order, the same on every machine.
uuids=$(for name in "${twin_units[@]}" "${js_units[@]}"; do
  for t in $(cd "$twin/$name" && find . -name '*.tasty'); do
    printf '%s %s\n' "$name/${t#./}" "$("$teq" tasty --trees "$twin/$name/$t" | sed -n 's/.*UUID \([0-9a-f]*\)$/\1/p')"
  done
done | LC_ALL=C sort)
if out=$(python3 tests/tasty/origins/check.py uuid $(find "$twin" -name '*.tasty' | LC_ALL=C sort) tests/tasty/fixtures/*.tasty 2>&1); then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  printf '%s\n' "$out" | head -10
fi
if [ "$1" = "--update" ]; then
  printf '%s\n' "$uuids" > tests/tasty/twin/uuids.txt
elif [ "$uuids" == "$(cat tests/tasty/twin/uuids.txt 2> /dev/null)" ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL twin UUIDs"
  diff <(printf '%s\n' "$uuids") tests/tasty/twin/uuids.txt | head -10
fi
again_ok=1
for name in "${twin_units[@]}" "${js_units[@]}"; do
  srcs=("tests/tasty/src/$name.scala")
  [ "$name" = pext_inlined ] && srcs+=(tests/tasty/src/pext_dsl.scala)
  "$teq" compiler check --products "$twin/again/$name" "${srcs[@]}" > /dev/null 2>&1
  for t in $(cd "$twin/$name" && find . -name '*.tasty'); do
    cmp -s "$twin/$name/$t" "$twin/again/$name/$t" || { again_ok=0; echo "FAIL twin $name: a second build writes $t otherwise"; }
  done
done
"$teq" compiler check --products "$twin/beside" tests/tasty/src/bodies.scala tests/tasty/origins/several/Several.scala > /dev/null 2>&1
for t in $(cd "$twin/bodies" && find . -name '*.tasty'); do
  cmp -s "$twin/bodies/$t" "$twin/beside/$t" || { again_ok=0; echo "FAIL twin bodies: a build beside an unused file writes $t otherwise"; }
done
[ $again_ok = 1 ] && pass=$((pass + 1)) || fail=$((fail + 1))
# The TASTy inspector's shape over its own jars (tests/tasty/inspector): Inspector.scala's
# `(quotes: Quotes) ?=> List[Tasty[quotes.type]] => T` over scala-library's `Quotes` and
# scala3-tasty-inspector's `Tasty`, built by teq in the product mode and by scalac 3.8.4: the two
# signatures' printouts differ by what tests/tasty/inspector/Inspector.diff records, of section 7's
# classes; UseInspector.scala is typed by scalac over teq's products (-Werror), and built by teq in
# the product mode over scalac's classes and over its own products; scalac reads teq's pickles
# below with the twin's. Without scala-cli or the jar a failure (incomplete validation).
insp=tests/tasty/inspector
insp_tmp=$(mktemp -d "${TMPDIR:-/tmp}/tasty-inspector.XXXXXX") || { echo "tasty: no temporary directory for the inspector's shape"; exit 1; }
insp_jar=$(jar_of tasty-inspector)
insp_cp=$(jar_of scala-library):$insp_jar
insp_read=""
insp_scalac() {
  local dest=$1
  shift
  (cd "$insp_tmp" && COURSIER_MODE=offline timeout 300 scala-cli --power compile -S 3.8.4 --jvm system --server=false --offline -q \
    --workspace "$insp_tmp" --scalac-option -Werror -d "$dest" "$@" 2>&1)
}
if ! command -v scala-cli > /dev/null || [ ! -f "$insp_jar" ]; then
  fail=$((fail + 1))
  echo "FAIL inspector: no scala-cli or no scala3-tasty-inspector 3.8.4 in the coursier cache (incomplete validation)"
elif ! "$teq" compiler build --target jvm --products "$insp_tmp/tq" --classpath "$insp_cp" "$insp/Inspector.scala" > "$insp_tmp/tq.log" 2>&1; then
  fail=$((fail + 1))
  echo "FAIL inspector: teq's products: $(head -3 "$insp_tmp/tq.log")"
elif ! log=$(insp_scalac "$insp_tmp/sc" --classpath "$insp_jar" "$PWD/$insp/Inspector.scala"); then
  fail=$((fail + 1))
  echo "FAIL inspector: scalac's classes: $log"
else
  insp_read=$insp_tmp/tq
  actual=$(diff <("$teq" tasty "$insp_tmp/sc/tinsp/Inspector.tasty" | tail -n +2 | sed 's/<body@[0-9]*>/<body>/g') \
                <("$teq" tasty "$insp_tmp/tq/tinsp/Inspector.tasty" | tail -n +2 | sed 's/<body@[0-9]*>/<body>/g'))
  if [ "$1" = "--update" ]; then
    printf '%s\n' "$actual" > "$insp/Inspector.diff"
  elif [ "$actual" == "$(cat "$insp/Inspector.diff" 2> /dev/null)" ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL inspector twin"
    diff <(printf '%s\n' "$actual") "$insp/Inspector.diff" | head -20
  fi
  if [ "$1" != "--update" ]; then
    if log=$(insp_scalac "$insp_tmp/sc-use" --classpath "$insp_tmp/tq:$insp_jar" "$PWD/$insp/UseInspector.scala"); then
      pass=$((pass + 1))
    else
      fail=$((fail + 1))
      echo "FAIL inspector: scalac over teq's products: $log"
    fi
    for up in sc tq; do
      if "$teq" compiler build --target jvm --products "$insp_tmp/use-over-$up" --classpath "$insp_cp:$insp_tmp/$up" "$insp/UseInspector.scala" > "$insp_tmp/use-over-$up.log" 2>&1; then
        pass=$((pass + 1))
      else
        fail=$((fail + 1))
        echo "FAIL inspector: teq over the $up products: $(head -3 "$insp_tmp/use-over-$up.log")"
      fi
    done
  fi
fi
# scalac 3.8.4 reads every twinned source's pickles as its own compilation units: -from-tasty
# with the outline admitted, every right-hand side forced and the tree
# checker run after readTasty, all of them in one JVM (tests/tasty/fromtasty/Gate.scala); without
# scala-cli a failure (incomplete validation).
if [ "$1" != "--update" ]; then
  if ! command -v scala-cli > /dev/null; then
    fail=$((fail + 1))
    echo "FAIL twin read: no scala-cli, so scalac reads none of the pickles (incomplete validation)"
  else
    : > "$twin/fromtasty.jobs"
    for name in "${twin_units[@]}"; do
      printf 'read\t%s\t%s\t%s\n' "$name" "$twin/$name" "$(find "$twin/$name" -name '*.tasty' | LC_ALL=C sort | tr '\n' '\t' | sed 's/\t$//')" >> "$twin/fromtasty.jobs"
    done
    sjs=$(jar_of scalajs-library)
    for name in "${js_units[@]}"; do
      printf 'readjs\t%s\t%s\t%s\n' "$name" "$twin/$name:$sjs" "$(find "$twin/$name" -name '*.tasty' | LC_ALL=C sort | tr '\n' '\t' | sed 's/\t$//')" >> "$twin/fromtasty.jobs"
    done
    # The capture's programs and the probes, Scala.js modules' check builds.
    for name in "${extra_units[@]}"; do
      printf 'readjs\t%s\t%s\t%s\n' "$name" "$extra/$name:$sjs" "$(find "$extra/$name" -name '*.tasty' | LC_ALL=C sort | tr '\n' '\t' | sed 's/\t$//')" >> "$twin/fromtasty.jobs"
    done
    # The inspector's shape, its jar on the class path.
    insp_units=()
    if [ -n "$insp_read" ]; then
      insp_units=(inspector)
      printf 'read\tinspector\t%s\t%s\n' "$insp_read:$insp_jar" "$(find "$insp_read" -name '*.tasty' | LC_ALL=C sort | tr '\n' '\t' | sed 's/\t$//')" >> "$twin/fromtasty.jobs"
    fi
    root=$PWD
    (cd "$twin" && COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
      "$root/tests/tasty/fromtasty/Gate.scala" --dep org.scala-lang:scala3-compiler_3:3.8.4 -- fromtasty.jobs > fromtasty.out 2> fromtasty.err)
    for name in "${twin_units[@]}" "${js_units[@]}" "${extra_units[@]}" "${insp_units[@]}"; do
      if grep -qxF "ok $name" "$twin/fromtasty.out"; then
        pass=$((pass + 1))
      else
        fail=$((fail + 1))
        echo "FAIL twin read $name"
        awk -v j="FAIL $name" '$0 == j { on = 1; next } /^(ok|FAIL) / { on = 0 } on' "$twin/fromtasty.out" | head -5
      fi
    done
  fi
fi
rm -rf "$twin" "$extra" "$insp_tmp"
# The writer's allowlist of the std's selections: what
# tests/tasty/stdshapes/generate.sh makes of the std as it is, the keys scala-library resolves.
if [ "$1" != "--update" ]; then
  if out=$(TEQ=$teq tests/tasty/stdshapes/generate.sh --check 2>&1); then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    printf '%s\n' "$out" | head -10
  fi
fi
# The annotations: scalac reads teq's pickles of
# tests/tasty/annotations/lib.scala, the lean std's check build and the JVM build over
# scala-library, as its own (tests/tasty/annotations/check.sh). Without scala-cli a failure
# (incomplete validation).
if [ "$1" != "--update" ]; then
  if ! command -v scala-cli > /dev/null; then
    fail=$((fail + 1))
    echo "FAIL annotations: no scala-cli, so scalac reads none of the annotations (incomplete validation)"
  else
    ann_tmp=$(mktemp -d "${TMPDIR:-/tmp}/tasty-annotations.XXXXXX") || { echo "tasty: no temporary directory for the annotations"; exit 1; }
    tests/tasty/annotations/check.sh "$(cd "$(dirname "$teq")" && pwd)/$(basename "$teq")" "$ann_tmp" > "$ann_tmp/out" 2>&1
    ann_status=$?
    pass=$((pass + $(grep -c '^ok ' "$ann_tmp/out")))
    if [ $ann_status != 0 ]; then
      fail=$((fail + 1))
      grep -v '^ok ' "$ann_tmp/out" | head -20
    fi
    rm -rf "$ann_tmp"
  fi
fi
# The positions: `teq tasty --positions` of every fixture
# against what scalac 3.8.4's unpickler gives its trees (tests/tasty/positions/Oracle.scala under
# scala-cli, each fixture in its package's directory), compared per kind of tree by
# tests/tasty/positions/check.py. Left out: the two fixtures of TASTy 28.3's quote patterns
# (LayerMacros, QpProbe), whose trees scalac's unpickler rewrites into others, and UmBodies, which
# it does not read without the Scala.js internals its pickle names. Without scala-cli the comparison is not made, which
# fails as incomplete validation.
if [ "$1" != "--update" ]; then
  if ! command -v scala-cli > /dev/null; then
    fail=$((fail + 1))
    echo "FAIL positions: no scala-cli, so no comparison with scalac's unpickler (incomplete validation)"
  else
    pos_tmp=$(mktemp -d "${TMPDIR:-/tmp}/tasty-positions.XXXXXX") || { echo "tasty: no temporary directory for the positions"; exit 1; }
    : > "$pos_tmp/requested.txt"
    for f in tests/tasty/fixtures/*.tasty; do
      pkg=$("$teq" tasty "$f" 2> /dev/null | sed -n 's/^package //p' | head -1 | tr . /)
      mkdir -p "$pos_tmp/cp/$pkg"
      cp "$f" "$pos_tmp/cp/$pkg/"
      echo "cp/$pkg/$(basename "$f")" >> "$pos_tmp/requested.txt"
    done
    # The places' library (below), whose shared tree reads in the source its target's switch names.
    places_lib_build
    mkdir -p "$pos_tmp/cp/pl"
    cp "$scratch"-places/lib/pl/*.tasty "$scratch"-places/decl-v1/pl/PlBox* "$pos_tmp/cp/pl/" 2> /dev/null
    printf '%s\n' cp/pl/Places.tasty cp/pl/Helpers.tasty cp/pl/PlBox.tasty >> "$pos_tmp/requested.txt"
    LC_ALL=C sort -o "$pos_tmp/requested.txt" "$pos_tmp/requested.txt"
    sjs=$(ls "$M2"/org/scala-js/scalajs-library_2.13/*/scalajs-library_2.13-*.jar 2> /dev/null | grep -v sources | tail -1)
    sjs3="$M2/org/scala-lang/scala3-library_sjs1_3/3.8.4/scala3-library_sjs1_3-3.8.4.jar"
    root=$PWD
    abs_teq=$(cd "$(dirname "$teq")" && pwd)/$(basename "$teq")
    for r in $(cat "$pos_tmp/requested.txt"); do
      [ -f "$pos_tmp/$r" ] || { fail=$((fail + 1)); echo "FAIL positions: $r was not copied for scalac's unpickler (incomplete validation)"; }
    done
    # The oracle has to end well and to answer for every file it was given: what it left out is
    # no pass.
    if [ -z "$sjs" ] || [ ! -f "$sjs3" ]; then
      fail=$((fail + 1))
      echo "FAIL positions: no scalajs-library_2.13 or scala3-library_sjs1_3 3.8.4 in the coursier cache, which scalac's unpickler needs for the Scala.js fixtures (incomplete validation)"
    elif ! (cd "$pos_tmp" && COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q \
      "$root/tests/tasty/positions/Oracle.scala" -- "-cp=cp:$sjs:$sjs3:$root/tests/classfile/fixtures" $(cat requested.txt) > oracle.txt 2> oracle.err); then
      fail=$((fail + 1))
      echo "FAIL positions: scalac's unpickler did not end well (incomplete validation): $(grep -v hint "$pos_tmp/oracle.err" | head -3)"
    elif out=$(cd "$pos_tmp" && python3 "$root/tests/tasty/positions/check.py" "$abs_teq" oracle.txt --requested requested.txt --skip /fix/qpat/LayerMacros.tasty /fix/qpat/QpProbe.tasty --unread /UmBodies.tasty); then
      pass=$((pass + $(echo "$out" | sed -n 's/^positions: \([0-9]*\) passed.*/\1/p')))
    else
      fail=$((fail + 1))
      echo "$out" | head -40
      grep -m3 -i error "$pos_tmp/oracle.err"
    fi
    rm -rf "$pos_tmp"
  fi
fi
# The places: the diagnostics of the bodies of
# tests/support/places_lib's jar that do not type against the declarations read with them, each at
# the library's source and position: with the sources jar beside the jar (the line's text, a column
# of bytes past supplementary characters), without it (the primary source's line from its line
# table, a switched source's path alone), a pickle without its Attributes (the path from its
# Positions), one without its Positions (SOURCEFILEattr's path alone) and one without either (the
# pseudo file's member line); a shared tree read through two occurrences at its target's place, in
# the source the switch at the target names (Places.one and two); an error inside an inlined
# library body at the call, with the line it was inlined from. Each case's diagnostics are
# tests/tasty/places/<case>.txt; without scala-cli to build the library a failure (incomplete
# validation).
places_lib=$(places_lib_jar)
places_nosrc=$(places_lib_nosrc_jar)
places_decl=$(places_decl_jar)
if [ ! -f "$places_lib" ] || [ ! -f "$places_nosrc" ] || [ ! -f "$places_decl" ]; then
  fail=$((fail + 1))
  echo "FAIL places: tests/support/places_lib not built (no scala-cli?): incomplete validation"
else
  ptmp=$(mktemp -d "${TMPDIR:-/tmp}/teq-places.XXXXXX") || { echo "tasty: no temporary directory for the places"; exit 1; }
  for strip in noattr:Attributes noposition:Positions "bare:Attributes Positions"; do
    mkdir -p "$ptmp/${strip%%:*}"
    python3 tests/tasty/places/strip.py "$places_nosrc" "$ptmp/${strip%%:*}/places-lib-1.0.jar" ${strip#*:}
  done
  for case in "sources build Use $places_lib" "nosrc build Use $places_nosrc" "noattr build Use $ptmp/noattr/places-lib-1.0.jar" \
      "noposition build Use $ptmp/noposition/places-lib-1.0.jar" "bare build Use $ptmp/bare/places-lib-1.0.jar" "inlined check Inl $places_lib"; do
    read -r name command prog jar <<< "$case"
    flags=()
    [ "$command" = build ] && flags=(-o "$ptmp/out.js")
    TEQ_CACHE_DIR=$ptmp/cache timeout 60 "$teq" compiler $command tests/tasty/places/$prog.scala --classpath "$jar:$places_decl" "${flags[@]}" > "$ptmp/$name.log" 2>&1
    status=$?
    # The diagnostics alone: the build's report follows them, a forked build's from its workers' section.
    actual=$(sed -E '/^(workers|classpath)$/,$d' "$ptmp/$name.log")
    expected=tests/tasty/places/$name.txt
    if [ "$1" = "--update" ]; then
      printf '%s\n' "$actual" > "$expected"
    elif [ $status = 1 ] && [ "$actual" == "$(cat "$expected" 2> /dev/null)" ]; then
      pass=$((pass + 1))
    else
      fail=$((fail + 1))
      echo "FAIL places $name (exit $status)"
      diff <(printf '%s\n' "$actual") "$expected" | head -20
    fi
  done
  rm -rf "$ptmp"
fi
# The origins: teq's TeqOrigins section, written into
# the pickles of the products of tests/tasty/origins and read back. Every pickle's section names
# its source's key and a token, and its definitions are its Positions entries with a point, as
# byte offsets of the source's text (check.py roundtrip: supplementary characters before them);
# the listing's `origin` and `token` lines of a program reading the products (what each records,
# the tokens two identities hold: keys that collide compiled apart, one module with and without a
# third key that displaces a tag, one key in two modules (and in two directories of one name), a
# program's own file against a product,
# a source of several pickles, a transparent method of one file making an anonymous class in another
# file's bodies, the products in either order) are tests/tasty/origins/expected.txt;
# a section of another version is ignored with a line of the classpath trace, a payload malformed
# within its framing is a warning on the jar and read as absent, a broken frame a failed read;
# scalac 3.8.4's TastyPrinter prints a product's pickle with the section, which it does not print
# (without scala-cli a failure: incomplete validation).
otmp=$(mktemp -d "${TMPDIR:-/tmp}/teq-origins.XXXXXX") || { echo "tasty: no temporary directory for the origins"; exit 1; }
o=tests/tasty/origins
origins_build() {
  local dir=$1
  shift
  if ! "$teq" compiler build --target jvm --products "$otmp/$dir" "$@" > "$otmp/$dir.log" 2>&1; then
    fail=$((fail + 1))
    echo "FAIL origins: the products $dir: $(head -3 "$otmp/$dir.log")"
  fi
}
origins_build pa $o/collide/Use29115316.scala
origins_build pb $o/collide/Use56507032.scala
origins_build pab $o/collide/Use29115316.scala $o/collide/Use56507032.scala
origins_build pabz $o/collide/Use29115316.scala $o/collide/Use56507032.scala $o/collide/ZZZ87kqi0.scala
origins_build sa $o/same/a/Use.scala
origins_build sb $o/same/b/Use.scala
# Two modules' products in directories of one name, as every sbt module's `classes`: two artifacts.
mkdir -p "$otmp/a" "$otmp/b"
origins_build a/classes $o/same/a/Use.scala
origins_build b/classes $o/same/b/Use.scala
origins_build sv $o/several/Several.scala
origins_build un $o/unicode/Notes.scala
# A transparent method of one file making an anonymous class, expanded in another file's bodies:
# the class's name reads both files' identities (the definition's and the expansion site's).
origins_build ex $o/expand/Make.scala $o/expand/Use.scala
origins_build oc $o/classes/Classes.scala
# The classes bodies define: the name each record
# gives back (`origins::class_name`, read beside the pickle) is a class the build wrote, and every
# anonymous and named local class of the build has its record, the transparent expansion's three
# with both files' identities among them; and each of the expansion's `INLINED`s has the record
# of the method it expands, in the other file.
for dir in ex oc; do
  named=$(for t in $(find "$otmp/$dir" -name '*.tasty' | LC_ALL=C sort); do
            "$teq" tasty --trees "$t" | sed -n 's/^ *[0-9]*: .*, the class \(.*\)$/\1/p' | sed "s|^|$(dirname "$t")/|"
          done | LC_ALL=C sort)
  made=$(find "$otmp/$dir" -path "$otmp/$dir/scala" -prune -o -name '*.class' -print | grep -E '[$][$]anon[$]|[$][0-9a-z]+_[0-9]+[.]class$' | sed 's/[.]class$//' | LC_ALL=C sort)
  if [ -n "$named" ] && [ "$named" == "$made" ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL origins classes $dir"
    diff <(printf '%s\n' "$named") <(printf '%s\n' "$made") | head -10
  fi
done
callees=$("$teq" tasty --trees "$otmp/ex/ex/Use.tasty" | grep -c '^ *[0-9]*: ex[.]Make[$][.]greeter(java[.]lang[.]String:ex[.]Greeter) of source 1 byte')
if [ "$callees" = 3 ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL origins callees: $callees records of Make.greeter in Use.tasty, where its three INLINEDs are"
fi
# A top-level transparent method's expansion: its record names the file's `$package` object, the
# same bytes beside an unused file and through the fork at two and four workers.
origins_build tl $o/toplevel/Make.scala $o/toplevel/Use.scala
origins_build tl-beside $o/toplevel/Make.scala $o/toplevel/Use.scala $o/several/Several.scala
export TEQ_FORK=1
origins_build tl-fork2 $o/toplevel/Make.scala $o/toplevel/Use.scala --threads 2
origins_build tl-fork4 $o/toplevel/Make.scala $o/toplevel/Use.scala --threads 4
unset TEQ_FORK
callees=$("$teq" tasty --trees "$otmp/tl/ot/Use.tasty" | grep -c '^ *[0-9]*: ot[.]Make[$]package[$][.]inc(scala[.]Int:scala[.]Int) of source 1 byte')
same=1
for d in tl-beside tl-fork2 tl-fork4; do cmp -s "$otmp/tl/ot/Use.tasty" "$otmp/$d/ot/Use.tasty" || same=0; done
if [ "$callees" = 1 ] && [ $same = 1 ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL origins top-level callee: $callees records of inc in Use.tasty, the same bytes elsewhere: $same"
fi
for case in "pa collide/Use29115316.scala oa" "pabz collide/ZZZ87kqi0.scala oz" "sv several/Several.scala sv" "un unicode/Notes.scala un"; do
  read -r dir src pkg <<< "$case"
  pickles=$(ls "$otmp/$dir/$pkg"/*.tasty 2> /dev/null)
  if [ -n "$pickles" ] && out=$(python3 $o/check.py roundtrip "$teq" "$o/$src" $pickles 2>&1); then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL origins roundtrip $dir: ${out:-no pickles}"
  fi
done
# origins_listing <case> <class path> <source>: the program's `origin` and `token` lines, sorted,
# the products' alone: the JVM's std, scala-library's jar, whose pickles hold no section, is read
# beside them.
origins_listing() {
  rm -f "$otmp/dump"
  if ! TEQ_READER_DUMP="$otmp/dump" "$teq" compiler check --target jvm "$3" --classpath "$2" > "$otmp/$1.check.log" 2>&1; then
    echo "$1	the check fails: $(head -1 "$otmp/$1.check.log")"
    return
  fi
  grep -E $'^(origin|token)\t' "$otmp/dump" 2> /dev/null | grep -v $'\tnone: the pseudo files\' tags' | LC_ALL=C sort -u | sed "s/^/$1	/"
}
listing=$(
  origins_listing apart "$otmp/pa:$otmp/pb" $o/use/Pair.scala
  origins_listing together "$otmp/pab" $o/use/Pair.scala
  origins_listing displaced "$otmp/pabz" $o/use/Three.scala
  origins_listing same "$otmp/sa:$otmp/sb" $o/use/Same.scala
  origins_listing classes "$otmp/a/classes:$otmp/b/classes" $o/use/Same.scala
  origins_listing own "$otmp/pa" $o/use/own/Use56507032.scala
  origins_listing several "$otmp/sv" $o/use/SeveralUse.scala
  origins_listing unicode "$otmp/un" $o/use/NotesUse.scala
  origins_listing expand "$otmp/ex" $o/use/ExpandUse.scala
)
if [ "$1" = "--update" ]; then
  printf '%s\n' "$listing" > $o/expected.txt
elif [ "$listing" == "$(cat $o/expected.txt 2> /dev/null)" ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL origins listing"
  diff <(printf '%s\n' "$listing") $o/expected.txt | head -20
fi
for pair in "apart $otmp/pb:$otmp/pa $o/use/Pair.scala" "same $otmp/sb:$otmp/sa $o/use/Same.scala" "classes $otmp/b/classes:$otmp/a/classes $o/use/Same.scala"; do
  read -r name reversed src <<< "$pair"
  if [ "$(origins_listing "$name" "$reversed" "$src")" == "$(printf '%s\n' "$listing" | grep "^$name	")" ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL origins: the listing of $name changes with the class path's order"
  fi
done
# The section of another version, a payload malformed within its framing, a broken frame.
for how in version variant frame; do
  rm -rf "$otmp/p-$how" && cp -R "$otmp/pa" "$otmp/p-$how"
  python3 $o/check.py patch "$otmp/pa/oa/UseA.tasty" "$otmp/p-$how/oa/UseA.tasty" $how
  rm -f "$otmp/dump"
  TEQ_CLASSPATH_DETAIL=1 TEQ_READER_DUMP="$otmp/dump" "$teq" compiler check --target jvm $o/use/own/Use56507032.scala --classpath "$otmp/p-$how" > "$otmp/$how.log" 2>&1
  status=$?
  case $how in
    version) grep -q $'^origin\toa.UseA\tp-version\tversion 2 unknown, ignored$' "$otmp/dump" && grep -q 'TeqOrigins version 2 unknown to this reader, ignored' "$otmp/$how.log" && [ $status = 0 ] ;;
    variant) grep -q $'^origin\toa.UseA\tp-variant\tmalformed: a variant past the section$' "$otmp/dump" && grep -q "warning: oa/UseA.tasty: teq's TeqOrigins section cannot be read (a variant past the section)" "$otmp/$how.log" && [ $status = 0 ] ;;
    frame) grep -q 'oa/UseA.tasty: truncated section' "$otmp/$how.log" && [ $status = 1 ] ;;
  esac
  if [ $? = 0 ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL origins $how: exit $status, $(grep -v '^\[classpath\]' "$otmp/$how.log" | head -3)"
  fi
done
if [ "$1" != "--update" ]; then
  if ! command -v scala-cli > /dev/null; then
    fail=$((fail + 1))
    echo "FAIL origins: no scala-cli, so scalac's printer is not run on a pickle with the section (incomplete validation)"
  else
    echo 'object Dummy' > "$otmp/Dummy.scala"
    (cd "$otmp" && COURSIER_MODE=offline timeout 300 scala-cli --power run -S 3.8.4 --jvm system --server=false --offline -q Dummy.scala \
      --dep org.scala-lang:scala3-compiler_3:3.8.4 --main-class dotty.tools.dotc.core.tasty.TastyPrinter -- pa/oa/UseA.tasty > printer.txt 2>&1)
    status=$?
    printed=$(sed 's/\x1b\[[0-9;]*m//g' "$otmp/printer.txt")
    if [ $status = 0 ] && grep -qE '^ +[0-9]+: TeqOrigins$' <<< "$printed" && grep -q '^Positions (' <<< "$printed" && grep -q '^Attributes (' <<< "$printed" && ! grep -q '^TeqOrigins' <<< "$printed"; then
      pass=$((pass + 1))
    else
      fail=$((fail + 1))
      echo "FAIL origins: scalac's TastyPrinter on a product's pickle, exit $status: $(grep -v hint <<< "$printed" | head -3)"
    fi
  fi
fi
rm -rf "$otmp"
# A version this reader does not know has to be refused with a message, not misread.
bad=$(mktemp -t teq-tasty.XXXXXX).tasty
printf '\x5c\xa1\xab\x1f\x9d\x80\x80' > "$bad"
if "$teq" tasty "$bad" 2>&1 | grep -q "TASTy version 29.0 is not supported"; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL version check"
fi
rm -f "$bad"
echo "tasty: $pass passed, $fail failed"
[ $fail -eq 0 ]
