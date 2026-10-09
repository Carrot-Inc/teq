#!/bin/bash
# bench/app/export-accept.sh <manifest> <build.json> <teq> <master teq> <out dir>: the companion of
# bench/app/export-identity.sh for a landing that changes the export on purpose. The identity check runs as it is (its
# line printed, its failure on any difference kept for the gate's own line); this one passes only when both export
# builds succeeded and the export's differences from master's build are exactly the manifest's, each permitted
# difference bound to both builds' bytes and to the evidence that ties it to a scalac-conforming correction (the
# format in bench/app/export_accept.py, whose --draft writes the entries and --self-test checks the comparison).
[ $# = 5 ] || { echo "usage: bench/app/export-accept.sh <manifest> <build.json> <teq> <master teq> <out dir>" >&2; exit 2; }
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
manifest=$(absolute "$1") out=$(absolute "$5")
[ -f "$manifest" ] || { echo "export-accept: no manifest $manifest"; exit 1; }
here=$(cd "$(dirname "$0")" && pwd)
line=$("$here/export-identity.sh" "$2" "$3" "$4" "$out" | tail -1)
echo "$line"
case $line in
  *"files identical to master's build"* | *"files differ from master's build"*) ;;
  *) echo "export-accept: the export's builds did not both succeed"; exit 1 ;;
esac
saved=$(cat "$out.saved" 2> /dev/null)
[ -d "$saved" ] && [ -d "$out" ] || { echo "export-accept: master's or the landing's build is missing"; exit 1; }
python3 "$here/export_accept.py" "$manifest" "$saved" "$out" "$here/../.."
