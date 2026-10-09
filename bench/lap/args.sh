#!/bin/bash
# args.sh <program>: the arguments of `teq compiler check` for a program of bench/programs.sh, absolute.
cd "$(dirname "$0")/../.."
work=$PWD/out/budget; . bench/programs.sh > /dev/null 2>&1
program_args $1
