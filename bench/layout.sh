#!/bin/bash
# bench/layout.sh <plain|pgo> [a|b|compare]: whether the budget's rows stand still under an edit that
# changes nothing the compiler does. It builds the tree as it is (a), again with an unused function
# added to the typer and kept in the binary (b), and compares the two binaries on every budget
# program with bench/compare.sh: the median of COMPARE_RUNS (default 21) interleaved runs, a type
# or total phase that moves more than COMPARE_TOLERANCE (default 3%) fails; parse phases are shown
# only. Two copies of one binary move rows by up to 1.6% over 21 runs and by up to 4.6% over 11,
# docs/SPEED.md "Placement and the instruction count". Without a stage, all three; a build takes 35 s plain, 85 s pgo.
# plain is `cargo build --release`; pgo is bench/pgo.sh use, on the profile of the last
# `bench/pgo.sh gen && bench/pgo.sh train` (b's added function has no profile, as after an edit).
# LAYOUT_EDIT=field adds an unused field to the Typer struct in the place of the function, an edit
# that shifts the typer's field offsets and so its code.
# The check to run when a budget row moves on an edit that cannot explain it. A failure says the
# recipe's code layout moves that row by itself, so a change of that size on it says nothing about
# an edit: run the compare stage again at a load under 3, and if the row fails again, judge edits
# on it by retired instructions (`/usr/bin/time -l`), which layout leaves alone, not by time.
cd "$(dirname "$0")/.." || exit 1
recipe=$1
case $recipe in plain | pgo) ;; *) echo "usage: $0 <plain|pgo> [a|b|compare]"; exit 2 ;; esac
dir=target/layout
edit=${LAYOUT_EDIT:-fn}
mkdir -p $dir

build() {
  if [ $recipe = pgo ]; then
    bench/pgo.sh use > $dir/build.log 2>&1 || { cat $dir/build.log; return 1; }
    cp target/pgo/use/release/teq "$1"
  else
    timeout 360 cargo build --release --target-dir $dir/cargo > $dir/build.log 2>&1 || { tail -20 $dir/build.log; return 1; }
    cp $dir/cargo/release/teq "$1"
  fi
  echo "layout: $1 ($("$1" --version))"
}

probe() {
  local file=src/typer/expr.rs
  [ "$edit" = field ] && file=src/typer/mod.rs
  cp $file $dir/probe.orig
  trap "cp $dir/probe.orig $file" EXIT
  if [ "$edit" = field ]; then
    perl -0pi -e 's/(\n    pub syms: Symbols,\n)/$1    pub layout_probe: u64,\n/; s/(\n        Typer \{\n            asts,\n)/$1            layout_probe: 0,\n/' $file
  else
    cat >> $file <<'EOF'

#[used]
static LAYOUT_PROBE: fn(&[u64]) -> u64 = layout_probe;

#[inline(never)]
fn layout_probe(words: &[u64]) -> u64 {
    words.iter().fold(0x9e37_79b9, |h, w| h.rotate_left(5) ^ w.wrapping_mul(0x100_0000_01b3))
}
EOF
  fi
  grep -q layout_probe $file || { echo "layout: the $edit edit did not apply to $file"; return 1; }
  build $dir/$recipe-b
  local status=$?
  cp $dir/probe.orig $file
  trap - EXIT
  return $status
}

compare() {
  local a=$dir/$recipe-a b=$dir/$recipe-b
  [ -x $a ] && [ -x $b ] || { echo "layout: build $a and $b first ($0 $recipe a, $0 $recipe b)"; return 1; }
  cmp -s $a $b && { echo "layout: the two binaries are identical, the edit reached nothing"; return 1; }
  [ "$edit" = field ] || nm $b | grep -q layout_probe || { echo "layout: the added function is not in $b"; return 1; }
  COMPARE_RUNS=${COMPARE_RUNS:-21} COMPARE_STAT=${COMPARE_STAT:-median} COMPARE_TOLERANCE=${COMPARE_TOLERANCE:-3} bench/compare.sh $a $b
}

case ${2:-all} in
  a) build $dir/$recipe-a ;;
  b) probe ;;
  compare) compare ;;
  all) build $dir/$recipe-a && probe && compare ;;
  *) echo "usage: $0 <plain|pgo> [a|b|compare]"; exit 2 ;;
esac
