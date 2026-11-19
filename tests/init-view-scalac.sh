#!/bin/bash
# The initialization checker's view ALONE: teq's view of scalac's OWN pickles of tests/init-view's programs (the
# classes initdump.py keeps beside scalac's dumps) against scalac's InitDump after `pickler`, line for line. No writer
# of teq's is in it, so what differs is the view's, or a fact of scalac's typer the pickle does not carry
# (tests/init-view/by_kind.py names which). Run after tests/init-view.sh, whose oracle directories it reads. The
# programs expected to differ are tests/pending/init-view-scalac.txt (tests/support/pending.sh: --update, --add).
# Env: TEQ (default ./target/release/teq); COURSIER.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/pending.sh
work=out/init-view
pending_begin init-view-scalac "$work/scalac" "$@"
library=$("${COURSIER:-cs}" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
# The digest of the scalac dumps kept also names the environment they depend on (tests/init-view/environment.sh:
# both JDKs and the library), so a dump made under another JDK is made again, not judged.
env_key=$(bash tests/init-view/environment.sh --key "$library")
pass=0
fail=0
skipped=0
for dir in tests/init-view/programs/*/; do
  name=$(basename "$dir")
  files=("$dir"*.scala)
  # A program's members.txt names the library classes whose trees both dumps print (INITDUMP_MEMBERS,
  # TEQ_INIT_VIEW_MEMBERS); it is part of the digest.
  members=; [ -f "$dir/members.txt" ] && members=$(cd "$dir" && pwd)/members.txt
  digest=$( { cat "${files[@]}" ${members:+"$members"} tests/init-view/InitDump.scala tests/init-view/initdump.py; echo "$env_key"; } | sha256sum | cut -c1-16)
  oracle="$work/oracle/$name-$digest"
  out="$work/teq/$name"
  mkdir -p "$out"
  # NOT RUN where tests/init-view.sh did not run the program (no scalac or scala-library here).
  if [ "$(cat "$out/status" 2> /dev/null)" = notrun ] || [ -z "$library" ]; then
    pending_skip "$name" "not run by tests/init-view.sh (no scalac or scala-library here)"
    skipped=$((skipped + 1))
    continue
  fi
  # scalac's dump and classes missing where the program ran: a failure, never skipped.
  if [ ! -s "$oracle/pickler.dump" ] || [ ! -d "$oracle/classes" ]; then
    pending_listed "$name" || echo "FAIL $name: no dump or classes of scalac's"
    pending_fail "$name" "no dump or classes of scalac's"
    fail=$((fail + 1))
    continue
  fi
  rm -f "$out/scalac-view.dump"
  # A view that fails or prints nothing: a failure no pending list takes (tests/init-view.sh fails the suite on it).
  if ! TEQ_INIT_VIEW_MEMBERS=$members TEQ_INIT_VIEW_CLASSPATH="$library" TEQ_INIT_VIEW="$(cd "$dir" && pwd)" timeout 60 "$TEQ" tasty "$oracle/classes" > "$out/scalac-view.dump" 2> "$out/scalac-view.err" || [ ! -s "$out/scalac-view.dump" ]; then
    echo "VIEW FAILED $name: the view of scalac's pickles (exit or empty): $(head -1 "$out/scalac-view.err")" | tee -a "$work/view-failures"
    pending_fail "$name" "the view failed"
    fail=$((fail + 1))
    continue
  fi
  if result=$(python3 tests/init-view/compare.py "$oracle/pickler.dump" "$out/scalac-view.dump"); then
    pending_pass "$name"
    pass=$((pass + 1))
  else
    pending_listed "$name" || echo "FAIL $name: $result"
    pending_fail "$name" "$result"
    fail=$((fail + 1))
  fi
done
echo "init-view-scalac: $pass equal, $fail differ, $skipped not run"
pending_end
