#!/bin/bash
dir=$(dirname "$0")
status=0
"$dir/run.py" --check || status=$?
"$dir/products.py" --check || status=$?
exit "$status"
