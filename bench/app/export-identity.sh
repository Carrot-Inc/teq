#!/bin/bash
# bench/app/export-identity.sh <build.json> <teq> <master teq> <out dir>: the application's export (the build file
# sbt-teq's teqExport writes) built by <teq> into <out dir> with bench/app/export-build.mjs, and compared file by
# file with master's build of the same inputs. Master's build is saved in out/app-export/<key> of this checkout, the
# key naming master's revision and a digest of the build file and of every source file it names, so it is built
# once per master and inputs and reused by every later comparison; <out dir>.saved names that build's directory for
# bench/app/export-accept.sh. Reads the application's checkout, never writes there.
[ $# = 4 ] || { echo "usage: bench/app/export-identity.sh <build.json> <teq> <master teq> <out dir>" >&2; exit 2; }
absolute() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
file=$(absolute "$1") teq=$(absolute "$2") master=$(absolute "$3") out=$(absolute "$4")
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/../.."
mkdir -p out/app-export "$(dirname "$out")"
key=$(python3 - "$file" "$("$master" --version)" <<'PY'
import hashlib, json, os, sys
file, version = sys.argv[1], sys.argv[2]
d = json.load(open(file))
root = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(file)), d.get("root", "../../..")))
h = hashlib.sha256(version.encode() + b"\0" + open(file, "rb").read())
for src in d["sources"]:
    base = os.path.join(root, src)
    for cur, dirs, files in os.walk(base):
        dirs.sort()
        for f in sorted(files):
            p = os.path.join(cur, f)
            h.update(os.path.relpath(p, root).encode() + b"\0" + open(p, "rb").read())
print(version.split()[2] + "-" + h.hexdigest()[:16])
PY
) || { echo "export-identity: cannot read $file or its sources"; exit 1; }
saved=out/app-export/$key
if [ ! -d "$saved" ]; then
  rm -rf "$saved.partial"
  node "$here/export-build.mjs" "$file" "$master" "$saved.partial" > "$saved.log" 2>&1 || { echo "export-identity: master's build failed: $(tail -1 "$saved.log")"; exit 1; }
  mv "$saved.partial" "$saved"
fi
rm -rf "$out"
echo "$PWD/$saved" > "$out.saved"
node "$here/export-build.mjs" "$file" "$teq" "$out" > "$out.build.log" 2>&1 || { echo "export-identity: the build failed: $(tail -1 "$out.build.log")"; exit 1; }
n=$(find "$saved" -type f | wc -l | tr -d ' ')
if diff -rq "$saved" "$out" > "$out.diff" 2>&1; then
  echo "export-identity: $n files identical to master's build ($saved)"
else
  head -20 "$out.diff"
  echo "export-identity: $(wc -l < "$out.diff" | tr -d ' ') of $n files differ from master's build ($saved; the list in $out.diff)"
  exit 1
fi
