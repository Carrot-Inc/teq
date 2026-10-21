#!/bin/bash
# bench/fix/corpora.sh: the three corpora of bench/fix/run.sh: bench/app's generated tree (bench/programs.sh's place,
# out/budget/app), scala3's tests/pos/jzon at the 3.8.4 tag (from the checkout SCALA3 names, by default the scala3
# harnesses': teq-ref/scala3 beside the main working tree, the git common directory's parent's sibling) into
# out/corpora/jzon, and the toy module (bench/fix/toy.py) into out/corpora/toy.
cd "$(dirname "$0")/../.."
common=$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)
scala3=${SCALA3:-$(dirname "${common:-$PWD/.git}")/../teq-ref/scala3}
git -C "$scala3" rev-parse -q --verify 3.8.4 > /dev/null || { echo "corpora: no 3.8.4 tag in $scala3 (set SCALA3)"; exit 1; }
[ -d out/budget/app/frontend ] || timeout 120 python3 bench/app/gen.py out/budget/app > /dev/null || { echo "corpora: bench/app did not generate"; exit 1; }
mkdir -p out/corpora/jzon
for f in $(git -C "$scala3" ls-tree -r --name-only 3.8.4 tests/pos/jzon) ; do
  git -C "$scala3" show "3.8.4:$f" > "out/corpora/jzon/$(basename "$f")" || { echo "corpora: no $f at 3.8.4 in $scala3"; exit 1; }
done
[ -d out/corpora/toy ] || python3 bench/fix/toy.py out/corpora/toy
ls out/corpora/jzon/*.scala > /dev/null && echo "corpora: frontend, jzon, toy"
