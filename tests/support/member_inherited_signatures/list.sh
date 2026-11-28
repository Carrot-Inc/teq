#!/bin/bash
set -eo pipefail
for path in "$@"; do
  name=${path##*/}
  echo "${name%.jar}"
  classes=(Parent Subject)
  case $name in member_inherited_implemented_*) classes+=(Child) ;; esac
  for class in "${classes[@]}"; do
    echo "$class"
    javap -v -p -classpath "$path" "$class" | awk '
      /^  [^ ]/ { method = ($0 ~ / pick\(/); if (method) print $0 }
      method && $1 == "descriptor:" { print $1, $2 }'
  done
done
