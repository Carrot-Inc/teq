#!/bin/bash
# Tests of the class file reader: `teq classfile` over the fixtures under tests/classfile/fixtures
# (compiled from tests/classfile/src with javac; see tests/classfile/README) has to print what
# tests/classfile/expected holds (`tests/classfile.sh --update` rewrites it); truncated and
# corrupted class files have to be refused with a message, never a crash; every class of
# java.base in the JDK's ct.sym and every class file of scala-library has to parse; and the
# printer has to agree with `javap -s -p` on java.lang.String and java.util.Map. The JDK, the
# jar and javap parts skip, and count as passed, when they are not found.
cd "$(dirname "$0")/.."
teq=${TEQ:-./target/release/teq}
pass=0
fail=0
mkdir -p tests/classfile/expected
for f in tests/classfile/fixtures/fix/*.class; do
  name=$(basename "$f" .class)
  expected="tests/classfile/expected/$name.txt"
  actual=$(timeout 30 "$teq" classfile "$f" 2>&1)
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
[ "$1" = "--update" ] && exit 0

# Malformed input: a message and exit code 1, never a panic (101) or a signal.
bad=$(mktemp -d -t teq-classfile.XXXXXX)
python3 - "$bad" tests/classfile/fixtures/fix/Generics.class <<'PY'
import random, sys
out, src = sys.argv[1], sys.argv[2]
data = open(src, 'rb').read()
for n in [0, 4, 9, 10, 40, 200, 700, len(data) // 2, len(data) - 1]:
    open(f'{out}/trunc{n}.class', 'wb').write(data[:n])
rnd = random.Random(7)
for i in range(60):
    d = bytearray(data)
    for _ in range(rnd.randint(1, 4)):
        at = rnd.randrange(300) if i % 2 == 0 else len(d) - 1 - rnd.randrange(300)
        d[at] = rnd.randrange(256)
    open(f'{out}/corrupt{i}.class', 'wb').write(d)
open(f'{out}/lengths.class', 'wb').write(data[:10] + b'\xff\xff' + data[12:])
PY
crashed=0
refused=0
for f in "$bad"/*.class; do
  timeout 30 "$teq" classfile "$f" > /dev/null 2> "$bad/err"
  code=$?
  [ $code = 1 ] && refused=$((refused + 1))
  if [ $code -gt 1 ] || grep -q panicked "$bad/err"; then
    crashed=1
    echo "FAIL $(basename "$f"): exit code $code"
    head -3 "$bad/err"
  fi
done
if [ $crashed = 0 ] && [ $refused -ge 10 ]; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL malformed input: $refused of $(ls "$bad"/*.class | wc -l | tr -d ' ') refused"
fi
if timeout 30 "$teq" classfile "$bad/trunc200.class" 2>&1 | grep -q "truncated class file"; then
  pass=$((pass + 1))
else
  fail=$((fail + 1))
  echo "FAIL truncation message"
fi
rm -rf "$bad"

# Every class of java.base in the JDK's ct.sym, and every class file of scala-library.
if out=$(ulimit -t 120; timeout 120 "$teq" classfile --verify --jdk java.base/ 2>&1); then
  pass=$((pass + 1))
  echo "$out" | tail -1
else
  if grep -q "no JDK found" <<< "$out"; then
    echo "skip ct.sym: $out"
  else
    fail=$((fail + 1))
    echo "FAIL ct.sym: $out"
  fi
fi
M2=${COURSIER_CACHE:-$HOME/Library/Caches/Coursier/v1}/https/repo1.maven.org/maven2
jar=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
if [ -f "$jar" ]; then
  if out=$(ulimit -t 120; timeout 120 "$teq" classfile --verify "$jar" 2>&1); then
    pass=$((pass + 1))
    echo "$out" | tail -1
  else
    fail=$((fail + 1))
    echo "FAIL scala-library: $out"
  fi
  # The Java class files selected by their missing TASTy siblings are the ones without a Scala attribute.
  out=$(ulimit -t 120; timeout 120 "$teq" classfile --stats "$jar" 2>&1)
  if grep -q "; 0 carry a Scala attribute anyway; 58 Java" <<< "$out" && grep -q "3600 written by a Scala compiler" <<< "$(timeout 120 "$teq" classfile --verify "$jar" 2>&1)"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL scala-library Java selection"
    echo "$out" | head -3
  fi
else
  echo "skip scala-library: not in the coursier cache"
fi

# The printer against javap on the same class files.
if command -v javap > /dev/null && timeout 30 "$teq" classfile --list --jdk java.base/java/lang/String.sig > /dev/null 2>&1; then
  dir=$(mktemp -d -t teq-javap.XXXXXX)
  ctsym=$(timeout 30 "$teq" classfile --jdk-path 2>/dev/null)
  for entry in java.base/java/lang/String.sig java.base/java/util/Map.sig; do
    full=$(timeout 30 "$teq" classfile --list --jdk "$entry" | awk '{print $3}' | head -1)
    python3 - "$ctsym" "$full" "$dir/$(basename "$entry" .sig).class" <<'PY'
import sys, zipfile
zipfile.ZipFile(sys.argv[1]).extract(sys.argv[2], sys.argv[3] + '.d')
import shutil, os
shutil.move(os.path.join(sys.argv[3] + '.d', sys.argv[2]), sys.argv[3])
PY
  done
  for f in "$dir"/String.class "$dir"/Map.class tests/classfile/fixtures/fix/Generics.class tests/classfile/fixtures/fix/Named.class; do
    if out=$(timeout 120 python3 tests/classfile/javap_compare.py "$teq" "$f" 2>&1); then
      pass=$((pass + 1))
    else
      fail=$((fail + 1))
      echo "FAIL javap $(basename "$f"): $out"
    fi
  done
  rm -rf "$dir"
else
  echo "skip javap comparison: no javap or no ct.sym"
fi
# The erasure of type parameters, value classes and arrays as scalac 3.8.4's: the members of teq's
# class files of tests/classfile/erasure/erasure.scala as javap describes them (javap_members.py)
# have to be expected.txt, the same list of scalac's class files, made by
#   scala-cli compile -S 3.8.4 --jvm system --server=false tests/classfile/erasure/erasure.scala -d <dir>
#   python3 tests/classfile/javap_members.py <dir> tests/classfile/erasure/excluded.txt > tests/classfile/erasure/expected.txt
# The members excluded.txt names are left out on both sides: their presence differs whatever
# their erasure.
if command -v javap > /dev/null && [ -f "$jar" ]; then
  dir=$(mktemp -d -t teq-erasure.XXXXXX)
  if timeout 60 "$teq" compiler build tests/classfile/erasure/erasure.scala --target jvm --classpath "$jar" --products "$dir/out" > "$dir/log" 2>&1 \
    && timeout 120 python3 tests/classfile/javap_members.py "$dir/out" tests/classfile/erasure/excluded.txt > "$dir/all" \
    && grep '^erasure/' "$dir/all" > "$dir/actual" \
    && diff tests/classfile/erasure/expected.txt "$dir/actual" > "$dir/diff"; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
    echo "FAIL erasure: the members are not scalac's"
    head -20 "$dir/log" "$dir/diff"
  fi
  rm -rf "$dir"
else
  echo "skip erasure: no javap, or scala-library not in the coursier cache"
fi
echo "classfile: $pass passed, $fail failed"
[ $fail -eq 0 ]
