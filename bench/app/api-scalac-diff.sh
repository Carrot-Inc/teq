#!/bin/bash
# bench/app/api-scalac-diff.sh <teq> <out dir>: scalac 3.8.4 as the oracle of teq's diagnostics on the application's
# API side, main and test (bench/app/diagnostics.py app, whose docstring has the passes, the records, their
# comparison and the known list): the lists bench/app/app-lists.sh writes (APP_ROOT, APP_MODULES, APP_CLASSPATH,
# APP_FLAGS, APP_SCALAC_OPTIONS and their APP_TEST_ counterparts), the known list SCALAC_KNOWN (a path kept outside
# the repository with the application; no rows when it is unset), SCALAC_DRAFT a file for every difference as a row
# of that list. Needs scala-cli, which fetches scalac 3.8.4 once; the two compiles by scalac take minutes (the gate's
# `--only app-scalac`, not a landing's line). Prints a line per scope and per difference not listed; exits 0 when
# both scopes complete and every difference is listed.
[ $# = 2 ] || { echo "usage: bench/app/api-scalac-diff.sh <teq> <out dir>" >&2; exit 2; }
exec python3 "$(dirname "$0")/diagnostics.py" app "$1" "$2"
