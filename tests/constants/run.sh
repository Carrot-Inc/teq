#!/bin/bash
# The constant reference under forced schedules: the cells of tests/constants/gen.py that the twelve pairs and one
# control of each other kind take (all of them with --all), each a constant val read from another file, and each with
# a target (the JVM, JavaScript), built once at one worker and once at four under
# TEQ_FORCE_CONST=unpublished:<the cell's constant>, which withholds the val's initialiser from the reader until the
# reader has decided the constant. The two builds' outputs are one form, and the hook's log says it fired with two
# workers. tests/pending/constants.txt lists the pairs that are not yet one form.
cd "$(dirname "$0")/../.." || exit 1
. tests/support/jars.sh
SL=$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar
[ -f "$SL" ] || { echo "constants: scala-library 3.8.4 is not in the coursier cache"; exit 1; }
exec python3 tests/constants/run.py "${TEQ:-./target/release/teq}" "$SL" "$@"
