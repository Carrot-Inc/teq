#!/bin/bash
# The identities of the measurement's inputs: each corpus's files (a digest of their paths and bytes), the
# configurations, the frontend's jars, and the binaries measured.
cd "$(dirname "$0")/../.."
. tests/support/jars.sh
tree_digest() { (cd "$1" && find . -name '*.scala' | LC_ALL=C sort | xargs sha256sum) | sha256sum | cut -c1-16; }
echo "frontend shared+frontend $(cat <(cd out/budget/app && find shared frontend -name '*.scala' | LC_ALL=C sort | xargs sha256sum) | sha256sum | cut -c1-16) files $(find out/budget/app/shared out/budget/app/frontend -name '*.scala' | wc -l)"
echo "jzon $(tree_digest out/corpora/jzon) (scala3 3.8.4 tests/pos/jzon)"
echo "toy $(tree_digest out/corpora/toy) (bench/fix/toy.py)"
for c in bench/fix/*.conf; do echo "config $(basename "$c") $(sha256sum < "$c" | cut -c1-16)"; done
for name in scala-library cats-kernel cats-core sourcecode; do p=$(jar_of "$name"); echo "jar $name $(sha256sum < "$p" | cut -c1-16) $(basename "$p")"; done
for b in "$@"; do echo "binary $b $("$b" --version)"; done
