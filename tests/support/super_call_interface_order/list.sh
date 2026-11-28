#!/bin/bash
set -eo pipefail
classes=()
for body in Method Constructor Lambda; do
  for lines in OneLine TwoLines; do
    for first in PredicateFirst OperatorFirst; do
      classes+=("$body$lines$first")
    done
  done
done
javap -v -p -classpath "$1" "${classes[@]}" | awk '/^public class / { print }'
# The declaration javap renders from a generic Signature can omit interfaces present in the header.
javap -v -p -classpath "$1" Nested | awk '
  $1 == "interfaces:" { print "Nested interfaces:", $2 + 0 }
  /invokespecial.*InterfaceMethod/ { print $NF }'
