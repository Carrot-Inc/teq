#!/bin/bash
# tests/init-view/selftest.sh: the init-view judgments' own self-test, run by tests/init-view.sh after its judgments: the
# mutations they must refuse, each with the verdict it must give: the suite fails closed.
#
# The comparator, on a program this run judged equal in all three (`diamond`, scalac's dump and teq's):
#   unmutated: positions pass, whole dump pass
#   a line removed from scalac's dump / from teq's: both fail
#   a position changed: both fail          a symbol changed: positions pass, whole dump fail
#   a tree's kind changed: both fail       teq's dump, scalac's, or both emptied: both fail
# The suite, run on that one program in a root of its own (a copy of the suite's scripts and its scalac dump):
#   as it is: exit 0, one pass in each judgment
#   the view of teq's products failing (exit 1, no output): exit 1, VIEW FAILED
#   that view printing nothing after a passing run (its old dump kept): exit 1, VIEW FAILED (the old dump not judged)
#   coursier absent: exit 0, nothing passed, the program NOT RUN in all three
#   the passing program listed as pending: exit 1 (a listed program that passes)
#   a kept dump of another JDK, spoiled: exit 0, one pass (the digest names the environment: made again, not judged)
#   a program failing in a root at another absolute path: its recorded line names its source relative to the root
# The corpus runner on that program as a corpus of one, a spoiled dump kept under another stamp (another corpus commit
# or JDK): made again, the view equal whole.
# NOT RUN where this run has no dumps of the program (no scalac here). Env: TEQ (absolute).
cd "$(dirname "$0")/../.."
[ -n "${INIT_VIEW_SELFTEST:-}" ] && exit 0
prog=diamond
work=out/init-view
teq_dump="$work/teq/$prog/pickler.dump"
digest_dir=$(ls -td "$work/oracle/$prog"-*/ 2> /dev/null | grep -E "/$prog-[0-9a-f]{16}/$" | head -1)
if [ "$(cat "$work/teq/$prog/status" 2> /dev/null)" != ran ] || [ -z "$digest_dir" ] || [ ! -s "${digest_dir}pickler.dump" ]; then
  echo "init-view self-test: NOT RUN (no dumps of $prog in this run)"
  exit 0
fi
tmp=$(mktemp -d)
trap 'rm -rf "${tmp:?}"' EXIT
fails=0
verdict() { if python3 tests/init-view/compare.py "$@" > /dev/null 2>&1; then echo pass; else echo fail; fi; }
# expect <name> <positions verdict> <whole verdict> <scalac dump> <teq dump>
expect() {
  local p w
  p=$(verdict --positions "$4" "$5"); w=$(verdict "$4" "$5")
  if [ "$p" = "$2" ] && [ "$w" = "$3" ]; then echo "  ok   $1: positions $p, whole $w"
  else echo "  FAIL $1: positions $p (expected $2), whole $w (expected $3)"; fails=$((fails + 1)); fi
}
s="${digest_dir}pickler.dump"
cp "$s" "$tmp/s"; cp "$teq_dump" "$tmp/t"
echo "init-view self-test, the comparator ($prog):"
expect "unmutated" pass pass "$tmp/s" "$tmp/t"
sed '$d' "$tmp/s" > "$tmp/s1"; expect "a line removed from scalac's dump" fail fail "$tmp/s1" "$tmp/t"
sed '$d' "$tmp/t" > "$tmp/t1"; expect "a line removed from teq's dump" fail fail "$tmp/s" "$tmp/t1"
python3 - "$tmp/t" "$tmp/t2" "$tmp/t3" "$tmp/t4" <<'PY'
import re, sys
t = open(sys.argv[1]).read().splitlines()
def first(pred):
    return next(i for i, l in enumerate(t) if pred(l))
i = first(lambda l: re.search(r"\[s\d+ (\d+)\.\.", l) and " sym=" in l)
pos = t[:]; pos[i] = re.sub(r"\[s(\d+) (\d+)\.\.", lambda m: f"[s{m.group(1)} {int(m.group(2)) + 1}..", pos[i], count=1)
sym = t[:]; sym[i] = re.sub(r" sym=(\S+)", r" sym=\1z", sym[i], count=1)
kind = t[:]; j = first(lambda l: l.strip().startswith("ValDef ")); kind[j] = kind[j].replace("ValDef ", "DefDef ", 1)
for path, lines in zip(sys.argv[2:], (pos, sym, kind)):
    open(path, "w").write("\n".join(lines) + "\n")
PY
expect "a position changed" fail fail "$tmp/s" "$tmp/t2"
expect "a symbol changed" pass fail "$tmp/s" "$tmp/t3"
expect "a tree's kind changed" fail fail "$tmp/s" "$tmp/t4"
: > "$tmp/e"
expect "teq's dump emptied" fail fail "$tmp/s" "$tmp/e"
expect "scalac's dump emptied" fail fail "$tmp/e" "$tmp/t"
expect "both emptied" fail fail "$tmp/e" "$tmp/e"
# The suite in a root of its own: its scripts, the one program, its scalac dump (the digest the same: the same files).
root="$tmp/root"
mkdir -p "$root/tests/init-view/programs" "$root/tests/pending" "$root/tests/board/suites" "$root/$work/oracle"
cp -r tests/support "$root/tests/support"
cp tests/init-view.sh tests/init-view-full.sh tests/init-view-scalac.sh tests/init-view-corpus.sh "$root/tests/"
cp tests/init-view/*.py tests/init-view/*.scala tests/init-view/selftest.sh tests/init-view/environment.sh tests/init-view/records.sh "$root/tests/init-view/"
cp -r "tests/init-view/programs/$prog" "$root/tests/init-view/programs/"
cp -r "${digest_dir%/}" "$root/$work/oracle/"
# scalac's dump names the program's source by its absolute path: the root's own.
sed -i "s|$PWD/tests/init-view/programs/|$root/tests/init-view/programs/|g" "$root/$work/oracle/$(basename "$digest_dir")/pickler.dump"
for p in init-view init-view-full init-view-scalac; do : > "$root/tests/pending/$p.txt"; done
# the board lists hold the one program's row (a test with no row fails a run: tests/support/pending.py's unrecorded)
for p in init-view init-view-full init-view-scalac; do
  printf '# test\tverdict\tthe other ways\tthe first line of the difference\n%s\tpass\t-\t\n' "$prog" > "$root/tests/board/suites/$p.tsv"
done
# A compiler whose view of teq's products fails (`fail`) or prints nothing (`empty`), any other command the real one's.
cat > "$tmp/teq-mutant" <<EOF
#!/bin/bash
if [ "\$1" = tasty ] && [[ "\$2" == *out/init-view/teq/* ]]; then
  case "\$MUTANT" in fail) echo "self-test: the view fails" >&2; exit 1 ;; empty) exit 0 ;; esac
fi
exec "$TEQ" "\$@"
EOF
chmod +x "$tmp/teq-mutant"
# suite <name> <expected exit> <expected text> [env...]: the suite in the root, its exit and a line it must print.
suite() {
  local name=$1 want=$2 text=$3 code
  shift 3
  (cd "$root" && env INIT_VIEW_SELFTEST=1 TEQ="$tmp/teq-mutant" "$@" bash tests/init-view.sh) > "$tmp/suite.log" 2>&1
  code=$?
  if [ "$code" = "$want" ] && grep -q -- "$text" "$tmp/suite.log"; then echo "  ok   $name: exit $code, \`$text\`"
  else echo "  FAIL $name: exit $code (expected $want), \`$text\` $(grep -q -- "$text" "$tmp/suite.log" && echo printed || echo "not printed")"; fails=$((fails + 1)); fi
}
echo "init-view self-test, the suite ($prog in a root of its own):"
suite "as it is" 0 "init-view-scalac: 1 pass, 0 fail" MUTANT=none
suite "the view failing" 1 "VIEW FAILED $prog" MUTANT=fail
suite "as it is, again" 0 "init-view-full: 1 pass, 0 fail" MUTANT=none
suite "the view printing nothing over its kept dump" 1 "VIEW FAILED $prog" MUTANT=empty
suite "coursier absent" 0 "init-view: 0 pass, 0 fail as tests/pending/init-view.txt lists, 1 not run" MUTANT=none COURSIER=/nonexistent/cs
printf '%s\tself-test: listed though it passes\n' "$prog" > "$root/tests/pending/init-view.txt"
suite "the passing program listed as pending" 1 "pass now" MUTANT=none
: > "$root/tests/pending/init-view.txt"
# A line the suite records holds no path of the machine: the program failing in the root (its scalac dump's `source`
# line changed), the line it records names the source relative to the root, as from the tree's own root.
kept=$(ls -d "$root/$work/oracle/$prog"-*/ | head -1)
cp "${kept}pickler.dump" "$tmp/kept.dump"
sed -i '0,/^source /s/ content=yes / content=self-test /' "${kept}pickler.dump"
(cd "$root" && env INIT_VIEW_SELFTEST=1 TEQ="$tmp/teq-mutant" MUTANT=none bash tests/init-view.sh) > "$tmp/suite.log" 2>&1
if grep -q "path=tests/init-view/programs/$prog/" "$tmp/suite.log" && ! grep -qF "$root" "$tmp/suite.log" && ! grep -qF "$tmp" "$tmp/suite.log"; then
  echo "  ok   a failing program in a root elsewhere: its line names the source relative to the root"
else
  echo "  FAIL a failing program in a root elsewhere: $(grep -m1 "content=self-test" "$tmp/suite.log" | cut -c1-200)"; fails=$((fails + 1))
fi
cp "$tmp/kept.dump" "${kept}pickler.dump"
# A kept dump whose environment is not this run's is made again, never compared: the dump
# kept in the root is spoiled (a line added, so judging it fails), then
# - the suite runs with teq's JDK another (a JAVA_HOME whose `java -version` is another, its ct.sym the real one): the
#   digest names the environment, so the spoiled dump is not found and scalac makes the program's again;
# - the corpus runner runs on the program as a corpus of one with the spoiled dump kept under another stamp (a dump of
#   another corpus commit or JDK): it makes the dump again.
kept=$(ls -d "$root/$work/oracle/$prog"-*/ | head -1)
echo "Template [s1 0..0@0 1:0-1:0] self-test: a line of another environment's dump" >> "${kept}pickler.dump"
real_java=$(readlink -f "$(command -v java)")
real_home=$(dirname "$(dirname "$real_java")")
mkdir -p "$tmp/jdk-other/bin" "$tmp/jdk-other/lib"
ln -s "$real_home/lib/ct.sym" "$tmp/jdk-other/lib/ct.sym"
cat > "$tmp/jdk-other/bin/java" <<EOF
#!/bin/bash
if [ "\$1" = -version ]; then echo 'openjdk version "0-self-test"' >&2; exit 0; fi
exec "$real_java" "\$@"
EOF
chmod +x "$tmp/jdk-other/bin/java"
suite "a kept dump of another JDK (spoiled), made again" 0 "init-view-scalac: 1 pass, 0 fail" MUTANT=none JAVA_HOME="$tmp/jdk-other"
mkdir -p "$tmp/corpus"
cp tests/init-view/programs/$prog/*.scala "$tmp/corpus/"
stem=$(cd "$tmp/corpus" && ls *.scala | head -1 | sed 's/\.scala$//')
unit="$root/out/init-view-corpus/self-test/$stem"
mkdir -p "$unit"
cp -r "$kept" "$unit/oracle"
echo "another-commit" > "$unit/oracle/stamp"
(cd "$root" && env TEQ="$TEQ" bash tests/init-view-corpus.sh "$tmp/corpus" self-test) > "$tmp/corpus.log" 2>&1
if grep -q "the view alone (of scalac's pickles): 1 of 1 units equal whole" "$tmp/corpus.log" && ! grep -q "self-test: a line of another" "$unit/oracle/pickler.dump"; then
  echo "  ok   a kept corpus dump of another commit (spoiled), made again: 1 of 1 equal whole"
else
  echo "  FAIL a kept corpus dump of another commit (spoiled): $(grep "the view alone" "$tmp/corpus.log")"; fails=$((fails + 1))
fi
echo "init-view self-test: $fails failed"
[ "$fails" = 0 ]
