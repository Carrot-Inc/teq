#!/bin/bash
# tests/init-view/environment.sh <scala-library jar> [corpus dir]: what scalac's dumps and the view depend on beside
# the sources, on one line (dotty's tests differ between commits: java1 at main writes a type its 3.8.4 tag infers). The
# suite (tests/init-view.sh) and the corpus runner (tests/init-view-corpus.sh) print it as their first line, with no
# path of the machine (a JDK by its home directory's name, the corpus by its path in its checkout):
# - scalac's JDK: `java` on the PATH, which tests/init-view/initdump.py runs scalac 3.8.4 and InitDump with;
# - teq's JDK: the one whose lib/ct.sym teq reads Java classes from (src/classfile/jdk.rs `find_home`: JAVA_HOME,
#   then `java` on the PATH);
# - scala-library 3.8.4's jar, whose pickles the view reads, by its digest;
# - the corpus: its directory, its git commit where it is in a git checkout, and a digest of its .scala files.
# With `--key`, prints instead the digest of the environment the dumps depend on (both JDKs' versions and the
# library's digest), which the suite's and the runner's kept dumps are keyed by.
key=0
[ "$1" = --key ] && { key=1; shift; }
library=$1
corpus=$2
version() { "$1" -version 2>&1 | head -1 | sed -n 's/.*version "\([^"]*\)".*/\1/p'; }
scalac_java=$(command -v java 2> /dev/null)
if [ -n "$scalac_java" ]; then
  scalac_jdk="$(version "$scalac_java") ($(basename "$(dirname "$(dirname "$(readlink -f "$scalac_java")")")"))"
else
  scalac_jdk="none"
fi
teq_home=""
if [ -n "$JAVA_HOME" ] && [ -f "$JAVA_HOME/lib/ct.sym" ]; then
  teq_home=$JAVA_HOME
elif [ -n "$scalac_java" ]; then
  h=$(dirname "$(dirname "$(readlink -f "$scalac_java")")")
  [ -f "$h/lib/ct.sym" ] && teq_home=$h
fi
if [ -n "$teq_home" ]; then
  teq_jdk="$(version "$teq_home/bin/java") ($(basename "$teq_home"))"
else
  teq_jdk="none"
fi
lib_digest=$( [ -f "$library" ] && sha256sum "$library" | cut -c1-16 || echo none)
if [ "$key" = 1 ]; then
  printf '%s\n' "$scalac_jdk" "$teq_jdk" "$lib_digest" | sha256sum | cut -c1-16
  exit 0
fi
line="environment: scalac's JDK $scalac_jdk; teq's JDK $teq_jdk; scala-library 3.8.4 $lib_digest"
if [ -n "$corpus" ]; then
  commit=$(git -C "$corpus" rev-parse --short=10 HEAD 2> /dev/null)
  described=$(git -C "$corpus" describe --tags --always 2> /dev/null)
  inside=$(git -C "$corpus" rev-parse --show-prefix 2> /dev/null)
  files=$(cd "$corpus" && cat ./*.scala 2> /dev/null | sha256sum | cut -c1-16)
  line="$line; corpus ${inside:-$(basename "$corpus")} at ${commit:-no git}${described:+ ($described)}, its .scala files $files"
fi
echo "$line"
