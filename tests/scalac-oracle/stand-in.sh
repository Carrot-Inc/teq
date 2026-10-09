#!/bin/bash
# A stand-in for teq or scala-cli in tests/scalac-oracle.sh: prints the file STAND_IN_OUT names on stdout and
# STAND_IN_ERR's on stderr, and exits STAND_IN_EXIT (default 0), whatever its arguments.
[ -z "$STAND_IN_ERR" ] || cat "$STAND_IN_ERR" >&2
[ -z "$STAND_IN_OUT" ] || cat "$STAND_IN_OUT"
exit "${STAND_IN_EXIT:-0}"
