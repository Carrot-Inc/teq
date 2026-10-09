#!/bin/bash
# Regression suite over Scala 3's tests/run: every test listed in passing.txt has to keep passing.
# Needs a scala/scala3 checkout in $SCALA3 (see run.py); skips with a message without one.
cd "$(dirname "$0")/../.."
exec python3 tests/scala3/run.py "$@"
