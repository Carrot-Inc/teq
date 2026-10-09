#!/bin/bash
# build.sh <name> <commit> [patch-python]: a release build of the commit's tree in /tmp/pt3-lap/<name>, the commit read
# from the checkout LAP_TREE names (by default this one)
set -e
name=$1; commit=$2; d=/tmp/pt3-lap/$name
tree=${LAP_TREE:-$(cd "$(dirname "$0")/../.." && pwd)}
rm -rf $d; mkdir -p $d
git -C "$tree" archive $commit | tar -x -m -C $d
cd $d
if [ -n "$3" ]; then python3 -c "$3"; fi
cargo build --release > build.log 2>&1
echo "built $name: $(./target/release/teq --version)" >> /tmp/pt3-lap/builds.log
