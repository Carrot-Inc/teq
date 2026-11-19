#!/bin/bash
# The initialization checker's view (src/init/) held to scalac's WHOLE: the after-`pickler` dumps tests/init-view.sh
# makes (scalac's through tests/init-view/InitDump.scala, teq's view of its products) compared line for line, the
# symbols, flags, types and classes with the positions (tests/init-view/compare.py without --positions). Run after
# tests/init-view.sh, whose outputs it reads: out/init-view/oracle/<program>-<digest>/ and out/init-view/teq/<program>/.
# The programs expected to differ are tests/pending/init-view-full.txt (tests/support/pending.sh: --update, --add);
# tests/init-view/census.py counts the differing fields by kind.
cd "$(dirname "$0")/.."
library=$("${COURSIER:-cs}" fetch --classpath --intransitive org.scala-lang:scala-library:3.8.4 2> /dev/null | tail -1)
# The digest of the scalac dumps kept also names the environment they depend on (tests/init-view/environment.sh:
# both JDKs and the library), so a dump made under another JDK is made again, not judged.
env_key=$(bash tests/init-view/environment.sh --key "$library")
. tests/support/pending.sh
work=out/init-view
pending_begin init-view-full "$work/full" "$@"
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
  view="$work/teq/$name/pickler.dump"
  # The program's status in tests/init-view.sh's run: NOT RUN there is not run here; a failure there (no products, a
  # view that failed or printed nothing) or no run of it is a failure here, never skipped (fails closed).
  status=$(cat "$work/teq/$name/status" 2> /dev/null)
  if [ "$status" = notrun ]; then
    pending_skip "$name" "not run by tests/init-view.sh (no scalac or scala-library here)"
    skipped=$((skipped + 1))
    continue
  fi
  if [ "$status" != ran ] || [ ! -s "$oracle/pickler.dump" ] || [ ! -s "$view" ]; then
    pending_listed "$name" || echo "FAIL $name: no dump of this run's to judge (status ${status:-none})"
    pending_fail "$name" "no dump of this run's (status ${status:-none})"
    fail=$((fail + 1))
    continue
  fi
  if result=$(python3 tests/init-view/compare.py "$oracle/pickler.dump" "$view"); then
    pending_pass "$name"
    pass=$((pass + 1))
  else
    pending_listed "$name" || echo "FAIL $name: $result"
    pending_fail "$name" "$result"
    fail=$((fail + 1))
  fi
done
echo "init-view-full: $pass equal, $fail differ, $skipped not run"
pending_end
