#!/bin/bash
# teq-capped.sh <teq's arguments>: the binary TEQ_REAL names in a systemd scope of its own, capped at TEQ_CAP bytes
# of memory (4 GB without it) and no swap: a teq that runs away in memory is killed there and its test reads as a
# crash, where bare it would fill the machine (one of dotty's neg tests once took 12.8 GB). tests/board/dotty.py
# runs every teq of dotty's tests through it on a machine whose user manager lingers (a scope ends with its
# manager) and starts such a scope, which it asks once with `--version`, and bare on any other. systemd-run by
# its absolute path: a test that empties PATH must still reach it.
exec /usr/bin/systemd-run --user --quiet --scope -p MemoryMax="${TEQ_CAP:-4000000000}" -p MemorySwapMax=0 "${TEQ_REAL:?set TEQ_REAL to the teq binary}" "$@"
