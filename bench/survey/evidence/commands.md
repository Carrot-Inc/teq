# The commands that wrote the evidence

Records of what ran, in order, their paths named by role (`<master>` master's release binary, built from a
`git archive` export of 0e2ef0f12 with `GIT_DIR=/nonexistent TEQ_REVISION=0e2ef0f12`; `<spike N>` a spike's release
binary; `<... worktree>` a checkout). Each step ran detached under its own `timeout`, one cargo build at a time,
with `COURSIER_CACHE` and `LC_ALL=C.UTF-8` set. Not maintained scripts: the repository's are Scala.

## The instruction pairs of spikes 1 and 3 (`instr-driver.sh`)

```
#!/bin/bash
export LC_ALL=C.UTF-8 COURSIER_CACHE=<coursier cache>
cd <master worktree>
for s in shared-names tree-inventory; do
  echo "== $s $(date -u +%T)"
  INSTR_COUNTS=<worktree>/out/survey/spikes/$s/instr-counts.txt timeout 3000 bench/instr-compare.sh <master> <spike $s> 1 depth_1 gview_1 core-only realistic-api realistic-frontend > <worktree>/out/survey/spikes/$s/instr.log 2>&1
  echo "exit $?" >> <worktree>/out/survey/spikes/$s/instr.log
done
echo "== done $(date -u +%T)"
```

## Unit tests with stage 1's gates, warnings, the ship builds (`cargo-driver.sh`)

```
#!/bin/bash
# Each Rust spike: stage 1's gates cherry-picked uncommitted (the jar CRC's own test dropped on spike 1, whose
# function it was), cargo test --release, the tree put back, then tests/rust-warnings.sh on the spike as committed.
D=<worktree>/out/survey/spikes
for s in shared-names tree-inventory; do
  cd <spike $s worktree> || exit 1
  echo "== $s tests $(date -u +%T)"
  git cherry-pick -n b8da521ea > $D/$s/cherry.log 2>&1
  git rm -r -q --cached bench/survey > /dev/null 2>&1; rm -rf bench/survey
  if [ $s = shared-names ]; then
    python3 - <<'PY'
p="src/jvm/classfile.rs"; s=open(p).read()
i=s.find("\n#[cfg(test)]\nmod tests {\n    /// The jar writer's CRC-32 is the one zip.rs")
if i >= 0: open(p,"w").write(s[:i+1])
PY
  fi
  git diff HEAD --stat > $D/$s/gates.stat
  timeout 1800 cargo test --release -j 12 > $D/$s/test-release.log 2>&1
  echo "exit $?" >> $D/$s/test-release.log
  git reset -q --hard HEAD
  git status --short > $D/$s/after-reset.status
  echo "== $s warnings $(date -u +%T)"
  timeout 2400 tests/rust-warnings.sh > $D/$s/rust-warnings.log 2>&1
  echo "exit $?" >> $D/$s/rust-warnings.log
done
echo "== done $(date -u +%T)"
# The ship profile (fat LTO, one codegen unit, no PGO) of master and the two Rust spikes: wall time and peak RSS of
# the build, the raw, stripped and gzip -9 sizes of the binary.
for s in master shared-names tree-inventory; do
  if [ $s = master ]; then dir=<master export>; env="GIT_DIR=/nonexistent TEQ_REVISION=0e2ef0f12"; else dir=<spike $s worktree>; env=""; fi
  cd $dir || exit 1
  mkdir -p $D/ship
  echo "== $s ship $(date -u +%T)"
  env $env /usr/bin/time -v timeout 1500 cargo build --profile ship -j 12 > $D/ship/$s.log 2>&1
  echo "exit $?" >> $D/ship/$s.log
  cp target/ship/teq $D/ship/$s.teq && strip -o $D/ship/$s.stripped $D/ship/$s.teq && gzip -9 -c $D/ship/$s.stripped > $D/ship/$s.stripped.gz
  echo "$s raw $(stat -c %s $D/ship/$s.teq) stripped $(stat -c %s $D/ship/$s.stripped) gzip $(stat -c %s $D/ship/$s.stripped.gz) wall $(grep 'Elapsed (wall' $D/ship/$s.log | awk '{print $NF}') peak-kb $(grep 'Maximum resident' $D/ship/$s.log | awk '{print $NF}')" >> $D/ship/sizes.txt
done
echo "== ship done $(date -u +%T)"
```

## Spike 1's unit tests again, then the suites (`chain.sh`)

```
#!/bin/bash
# After the cargo phase: spike 1's unit tests again (stage 1's gates but the jar CRC's own, the spike's
# classfile.rs kept), then the suites phase.
D=<worktree>/out/survey/spikes
until grep -q "== ship done" $D/cargo-driver.log; do sleep 10; done
cd <spike 1 worktree>
echo "== shared-names tests again $(date -u +%T)"
git cherry-pick -n b8da521ea > $D/shared-names/cherry2.log 2>&1
git checkout HEAD -- src/jvm/classfile.rs
git rm -r -q --cached bench/survey > /dev/null 2>&1; rm -rf bench/survey
git diff HEAD --stat > $D/shared-names/gates.stat
grep -rn "^<<<<<<<\|^>>>>>>>" src > $D/shared-names/markers.txt
timeout 1800 cargo test --release -j 12 > $D/shared-names/test-release.log 2>&1
echo "exit $?" >> $D/shared-names/test-release.log
git reset -q --hard HEAD
git status --short > $D/shared-names/after-reset.status
echo "== suites $(date -u +%T)"
$D/suites-driver.sh > $D/suites-driver.log 2>&1
echo "== chain done $(date -u +%T)"
```

## The suites phase (`suites-driver.sh`)

```
#!/bin/bash
# The suites phase, after the cargo phase: each step bounded, its log under the spike's directory.
export LC_ALL=C.UTF-8 COURSIER_CACHE=<coursier cache> SCALA3=<scala3 checkout>
D=<worktree>/out/survey/spikes
M=<master> S1=<spike 1> S3=<spike 3>
step() { local name=$1 log=$2; shift 2; echo "== $name $(date -u +%T)"; local s=$SECONDS; timeout 2400 "$@" > $log 2>&1; local code=$?; echo "exit $code seconds $((SECONDS - s))" >> $log; echo "   exit $code in $((SECONDS - s)) s"; }
# Spike 2: the whole corpus, old and new runner, master's binary.
cd <master worktree>
step stdlib-old $D/case-runner/full-old.out env TEQ=$M tests/run_stdlib.sh
rm -rf $D/case-runner/full-old-tests; cp -r out/stdlib-tests $D/case-runner/full-old-tests
cd <spike 2 worktree>
step stdlib-new-8 $D/case-runner/full-new.out env TEQ=$M JOBS=8 ./teq interp tests/run_stdlib.scala
rm -rf $D/case-runner/full-new-tests; cp -r out/stdlib-tests $D/case-runner/full-new-tests
step stdlib-new-1 $D/case-runner/full-new-1.out env TEQ=$M JOBS=1 ./teq interp tests/run_stdlib.scala
rm -rf $D/case-runner/full-new-1-tests; cp -r out/stdlib-tests $D/case-runner/full-new-1-tests
# Spike 1.
cd <spike 1 worktree>
step identity $D/shared-names/identity.log tests/support/identity.sh $M $S1 out/identity
step products-1 $D/shared-names/products-1.log $D/products-identity.sh $M $S1 <spike 1 worktree>/out/products-1 1
step products-4 $D/shared-names/products-4.log $D/products-identity.sh $M $S1 <spike 1 worktree>/out/products-4 4
step jar-check $D/shared-names/jar-check.log $D/shared-names/jar-check.sh
step typing $D/shared-names/scala3-typing.log env TEQ=$S1 python3 tests/scala3-typing/harness.py
step macros $D/shared-names/scala3-macros.log env TEQ=$S1 tests/scala3/run.sh --suite run-macros
step macros-interp $D/shared-names/scala3-macros-interp.log env TEQ=$S1 tests/scala3/run.sh --suite run-macros --target interp
git checkout -q -- tests/scala3; git status --short > $D/shared-names/after-suites.status
# Spike 3.
cd <spike 3 worktree>
step identity $D/tree-inventory/identity.log tests/support/identity.sh $M $S3 out/identity
step products-1 $D/tree-inventory/products-1.log $D/products-identity.sh $M $S3 <spike 3 worktree>/out/products-1 1
step typing $D/tree-inventory/scala3-typing.log env TEQ=$S3 python3 tests/scala3-typing/harness.py
step macros $D/tree-inventory/scala3-macros.log env TEQ=$S3 tests/scala3/run.sh --suite run-macros
step macros-interp $D/tree-inventory/scala3-macros-interp.log env TEQ=$S3 tests/scala3/run.sh --suite run-macros --target interp
step interp $D/tree-inventory/interp.log env TEQ=$S3 tests/run_interp.sh
step classpath-spike $D/tree-inventory/classpath-spike.log env TEQ=$S3 tests/classpath.sh
step classpath-master $D/tree-inventory/classpath-master.log env TEQ=$M tests/classpath.sh
git checkout -q -- tests/scala3; git status --short > $D/tree-inventory/after-suites.status
# The part's own branch: its release binary against master's, byte for byte but the version stamp.
cd <worktree>
step feat-release $D/feat-release.log cargo build --release -j 12
cp target/release/teq $D/feat-teq
{ $D/feat-teq --version; cmp -l $M $D/feat-teq | wc -l; stat -c %s $M $D/feat-teq; } > $D/feat-binary.txt 2>&1
echo "== done $(date -u +%T)"
```

## The products, TASTy and analysis comparison (`products-identity.sh`)

```
#!/bin/bash
# products-identity.sh <reference teq> <teq> <out dir> [threads]: every tests/cases program built by both binaries
# with --target jvm --products --analysis-version 3 (class files, .tasty files, the manifest, the analysis answer
# on stdout with its `ms` times removed), from the checkout in the working directory; prints a line per program that
# differs and a summary. Survey harness, not a suite.
ref=$1 new=$2 out=$3 threads=${4:-1}
export LC_COLLATE=C
. tests/support/jars.sh
SL=$(scala_library_jar)
[ -n "$SL" ] || { echo "no scala-library jar"; exit 2; }
rm -rf "$out"; mkdir -p "$out"
one() {
  local bin=$1 side=$2 src=$3 name
  name=$(basename "$src" .scala)
  local dir=$out/$side/$name
  mkdir -p "$dir/products"
  TEQ_THREADS=$threads timeout 60 "$bin" compiler build --target jvm --threads "$threads" --classpath "$SL" --products "$dir/products" --sourceroot "$PWD" --analysis-version 3 "$src" > "$dir/answer.raw" 2> "$dir/err"
  echo "exit $?" > "$dir/exit"
  sed -E 's/"ms":\{[^}]*\}/"ms":{}/g; s/"ms":[0-9.]+/"ms":0/g' "$dir/answer.raw" > "$dir/answer.json"; rm "$dir/answer.raw"
}
export -f one; export out threads SL
for src in tests/cases/*.scala tests/cases/*/; do echo "${src%/}"; done > "$out/list"
xargs -P "${JOBS:-8}" -I{} bash -c 'one "$0" ref {}' "$ref" < "$out/list"
xargs -P "${JOBS:-8}" -I{} bash -c 'one "$0" new {}' "$new" < "$out/list"
same=0 differ=0
while read -r src; do
  name=$(basename "$src" .scala)
  if diff -r -q "$out/ref/$name" "$out/new/$name" > /dev/null; then same=$((same + 1)); else differ=$((differ + 1)); echo "DIFFERS $name"; fi
done < "$out/list"
built=$(grep -l "^exit 0" "$out"/new/*/exit | wc -l)
echo "products identity: $same same, $differ differ, of $(wc -l < "$out/list") programs ($built built without error), threads $threads"
[ $differ = 0 ]
```

## The whole-jar comparison and the jar writer's cost (`jar-check.sh`)

```
#!/bin/bash
# The jar writer's bytes and cost, master against spike 1: each program built to a jar at one worker by both
# binaries, the jars compared byte for byte (CRC-32 fields included), then the build's instructions under
# cachegrind with the functions' costs (cg_annotate), and the entries and bytes of each jar.
export LC_ALL=C.UTF-8 COURSIER_CACHE=<coursier cache>
cd <master worktree>
. tests/support/jars.sh
cp=""; for n in scala-library cats-kernel cats-core sourcecode; do cp="$cp:$(jar_of $n)"; done; cp=${cp#:}
D=<worktree>/out/survey/spikes/shared-names/jar
rm -rf $D; mkdir -p $D
M=<master> S=<spike 1>
for prog in core-only api many; do
  case $prog in
    core-only) args="out/budget/core-only" ;;
    api) args="out/budget/app/shared out/budget/app/api --classpath $cp" ;;
    many) args="out/budget/app/shared out/budget/app/api out/budget/app/frontend --classpath $cp" ;;
  esac
  for side in master spike; do
    bin=$M; [ $side = spike ] && bin=$S
    for threads in 1 8; do
      TEQ_CACHE_DIR=$D/cache-$side timeout 300 $bin compiler build --target jvm --threads $threads $args -o $D/$prog-$side-t$threads.jar > $D/$prog-$side-t$threads.log 2>&1
      echo "$prog $side threads $threads exit $?" >> $D/summary.txt
    done
    TEQ_THREADS=1 TEQ_CACHE_DIR=$D/cache-$side timeout 600 valgrind --tool=cachegrind --cache-sim=no --cachegrind-out-file=$D/$prog-$side.cg $bin compiler build --target jvm --threads 1 $args -o $D/$prog-$side-cg.jar > /dev/null 2> $D/$prog-$side.cg.log
    echo "$prog $side cachegrind $(awk '/I +refs:/ {gsub(",", "", $NF); print $NF}' $D/$prog-$side.cg.log)" >> $D/summary.txt
  done
  for t in 1 8; do cmp -s $D/$prog-master-t$t.jar $D/$prog-spike-t$t.jar && echo "$prog threads $t: jars identical ($(stat -c %s $D/$prog-spike-t$t.jar) bytes, $(unzip -l $D/$prog-spike-t$t.jar | tail -1 | awk '{print $2}') entries)" >> $D/summary.txt || echo "$prog threads $t: JARS DIFFER" >> $D/summary.txt; done
  cg_annotate $D/$prog-master.cg 2>/dev/null | grep -i "crc\|write_jar" | head -5 | sed "s/^/$prog master: /" >> $D/summary.txt
  cg_annotate $D/$prog-spike.cg 2>/dev/null | grep -i "crc\|write_jar" | head -5 | sed "s/^/$prog spike: /" >> $D/summary.txt
done
echo done >> $D/summary.txt
```

## The macro pair, the ship pair, stdlib and fold for spike 3, spike 2's harness rows (`extra-driver.sh`)

```
#!/bin/bash
# The rows the first phases leave: after chain.sh ends. Each step bounded, its log beside its spike's.
export LC_ALL=C.UTF-8 COURSIER_CACHE=<coursier cache> SCALA3=<scala3 checkout>
D=<worktree>/out/survey/spikes
M=<master> S1=<spike 1> S3=<spike 3>
until grep -q "== chain done" $D/chain.log; do sleep 10; done
step() { local name=$1 log=$2; shift 2; echo "== $name $(date -u +%T)"; local s=$SECONDS; timeout 3000 "$@" > $log 2>&1; local code=$?; echo "exit $code seconds $((SECONDS - s))" >> $log; echo "   exit $code in $((SECONDS - s)) s"; }
cd <master worktree>
# Spike 3 where its path runs: the macro-heavy programs of the budget and the derive line.
step instr-macros $D/tree-inventory/instr-macros.log env INSTR_COUNTS=$D/tree-inventory/instr-macros-counts.txt bench/instr-compare.sh $M $S3 1 macro-cls derive30 kittens30 schema30
# Spike 1's realistic-frontend on the ship pair (fat LTO, one codegen unit): the partition or the work.
step ship-pair $D/shared-names/ship-pair.log env INSTR_TOLERANCE=none INSTR_COUNTS=$D/shared-names/ship-pair-counts.txt bench/instr-compare.sh $D/ship/master.teq $D/ship/shared-names.teq 1 depth_1 gview_1 core-only realistic-api realistic-frontend
# Spike 3 under scala-library's std (macros expanded through the same API) and the folds, master alongside.
step stdlib-s3 $D/tree-inventory/stdlib.out env TEQ=$S3 tests/run_stdlib.sh
rm -rf $D/tree-inventory/stdlib-tests; cp -r out/stdlib-tests $D/tree-inventory/stdlib-tests
step fold-s3 $D/tree-inventory/fold.log env TEQ=$S3 tests/fold.sh
step fold-master $D/tree-inventory/fold-master.log env TEQ=$M tests/fold.sh
# Spike 2: the harness apart from the binary it tests, and the gate's own checks over the changed lists.
cd <plant worktree>
mkdir -p $D/case-runner/wrapper; rm -f $D/case-runner/wrapper/calls.log
printf '#!/bin/sh\necho "$*" >> %s/case-runner/wrapper/calls.log\nexec %s "$@"\n' "$D" "$M" > $D/case-runner/wrapper/teq; chmod +x $D/case-runner/wrapper/teq
true               tests/cases/capture_release_ctor_param.expected tests/stdlib-passing.txt tests/stdlib-summaries.txt 2>/dev/null
step wrapped $D/case-runner/wrapped.out env TEQ=$M ./teq interp tests/run_stdlib.scala -- --teq $D/case-runner/wrapper/teq
step nonexistent $D/case-runner/nonexistent.out env TEQ=$M ./teq interp tests/run_stdlib.scala -- --teq /nonexistent/teq
cd <spike 2 worktree>
step gate-coverage $D/case-runner/gate-coverage.log tests/gate-coverage.sh
step gate-scripts $D/case-runner/gate-scripts.log tests/gate-scripts.sh
echo "== extra done $(date -u +%T)"
```

## Master's release at a path of this worktree's length (`feat-compare.sh`)

```
#!/bin/bash
# This branch's release binary against master's built at a path of the same length (reduction-master and
# reduction-survey), so that the embedded build paths shift nothing: the bytes that differ.
D=<worktree>/out/survey/spikes
until grep -q "== extra done" $D/extra-driver.log; do sleep 10; done
cd <master worktree>
echo "== master release at the same path length $(date -u +%T)"
timeout 1500 cargo build --release -j 12 > $D/master-samepath.log 2>&1; echo "exit $?" >> $D/master-samepath.log
cp target/release/teq $D/master-samepath-teq
{ $D/master-samepath-teq --version; $D/feat-teq --version; stat -c '%s %n' $D/master-samepath-teq $D/feat-teq; echo "bytes differing: $(cmp -l $D/master-samepath-teq $D/feat-teq | wc -l)"; } > $D/feat-binary-samepath.txt 2>&1
cmp -l $D/master-samepath-teq $D/feat-teq | head -40 >> $D/feat-binary-samepath.txt
echo "== done $(date -u +%T)"
```

## This branch's pair and typing harness, spike 2's gate-scripts (`driver.sh`)

```
#!/bin/bash
export LC_ALL=C.UTF-8 COURSIER_CACHE=<coursier cache> SCALA3=<scala3 checkout>
D=<worktree>/out/survey/spikes
cd <master worktree>
echo "== feat pair $(date -u +%T)"
INSTR_COUNTS=$D/feat/instr-counts.txt timeout 3000 bench/instr-compare.sh <master> $D/feat-teq 1 depth_1 gview_1 core-only realistic-api realistic-frontend > $D/feat/instr.log 2>&1; echo "exit $?" >> $D/feat/instr.log
cd <worktree>
echo "== feat typing $(date -u +%T)"
TEQ=$D/feat-teq timeout 600 python3 tests/scala3-typing/harness.py > $D/feat/scala3-typing.log 2>&1; echo "exit $?" >> $D/feat/scala3-typing.log
cd <spike 2 worktree>
echo "== case-runner gate-scripts $(date -u +%T)"
timeout 1200 tests/gate-scripts.sh > $D/case-runner/gate-scripts.log 2>&1; echo "exit $?" >> $D/case-runner/gate-scripts.log
echo "== done $(date -u +%T)"
```

## Identity and interp again with the lila jars (`rerun-jars.sh`)

```
#!/bin/bash
# The programs the missing lila jars had left out (identity.sh left them out and counted them identical; the
# interpreter suite counted them as passed), again with the jars in the cache.
export LC_ALL=C.UTF-8 COURSIER_CACHE=<coursier cache> SCALA3=<scala3 checkout>
D=<worktree>/out/survey/spikes
M=<master>
re='(cases-)?(jar_case_class_selectors|macwire_[a-z_]*|parser_combinators_[a-z_]*|play_json_macros)'
for s in shared-names tree-inventory; do
  cd <spike $s worktree>
  echo "== $s identity $(date -u +%T)"
  IDENTITY_PROGRAMS=$re timeout 1200 tests/support/identity.sh $M <spike $s> out/identity-jars > $D/$s/identity-jars.log 2>&1; echo "exit $?" >> $D/$s/identity-jars.log
  ls out/identity-jars/js/new 2>/dev/null | wc -l >> $D/$s/identity-jars.log
done
cd <spike 3 worktree>
echo "== interp $(date -u +%T)"
TEQ=<spike 3> timeout 1200 tests/run_interp.sh > $D/tree-inventory/interp-jars.log 2>&1; echo "exit $?" >> $D/tree-inventory/interp-jars.log
cd <master worktree>
TEQ=$M timeout 1200 tests/run_interp.sh > $D/tree-inventory/interp-jars-master.log 2>&1; echo "exit $?" >> $D/tree-inventory/interp-jars-master.log
echo "== done $(date -u +%T)"
```

## The interpreter rows: size for spike 3, interp, stdlib, fold and size for spike 1 (`interp-rows.sh`)

```
#!/bin/bash
# The interpreter rows the brief requires of a spike touching the interpreter: tests/size.scala for spike 3 and
# master; for spike 1 (interp/quoted.rs's views) interp, stdlib, fold and size too.
export LC_ALL=C.UTF-8 COURSIER_CACHE=<coursier cache>
D=<worktree>/out/survey/spikes
M=<master> S1=<spike 1> S3=<spike 3>
step() { local name=$1 log=$2; shift 2; echo "== $name $(date -u +%T)"; local s=$SECONDS; timeout 1800 "$@" > $log 2>&1; local code=$?; echo "exit $code seconds $((SECONDS - s))" >> $log; echo "   exit $code in $((SECONDS - s)) s"; }
cd <spike 3 worktree>
step size-s3 $D/tree-inventory/size.log env TEQ=$M ./teq interp tests/size.scala -- --teq $S3
step size-master $D/tree-inventory/size-master.log env TEQ=$M ./teq interp tests/size.scala -- --teq $M
cd <spike 1 worktree>
step size-s1 $D/shared-names/size.log env TEQ=$M ./teq interp tests/size.scala -- --teq $S1
step interp-s1 $D/shared-names/interp.log env TEQ=$S1 tests/run_interp.sh
step fold-s1 $D/shared-names/fold.log env TEQ=$S1 tests/fold.sh
cd <master worktree>
step stdlib-s1 $D/shared-names/stdlib.out env TEQ=$S1 tests/run_stdlib.sh
rm -rf $D/shared-names/stdlib-tests; cp -r out/stdlib-tests $D/shared-names/stdlib-tests
echo "== done $(date -u +%T)"
```

## The net diffs by category (`netdiff.py`)

```
# netdiff.py <repo> <base> <head>: the diff's added and removed lines by category: code, comment (// and /// and
# # in shell, a doc comment's lines), blank; tests (a Rust #[cfg(test)] module's lines are not told apart here, the
# spikes add none) by file kind.
import subprocess,sys,re,collections
repo,base,head=sys.argv[1:4]
d=subprocess.run(["git","-C",repo,"diff","-U0","--no-color",base,head],capture_output=True,text=True).stdout
c=collections.defaultdict(lambda: collections.Counter())
f=None
for l in d.split("\n"):
    if l.startswith("+++ "): f=l[6:] if l!="+++ /dev/null" else f; continue
    if l.startswith("--- "):
        if l!="--- /dev/null": f=l[6:]
        continue
    if not l or l[0] not in "+-" : continue
    sign=l[0]; t=l[1:].strip()
    kind=("rust" if f.endswith(".rs") else "scala" if f.endswith(".scala") else "shell" if f.endswith(".sh") else "doc" if f.endswith(".md") else "other")
    cat="blank" if not t else "comment" if (t.startswith("//") or (kind=="shell" and t.startswith("#") and not t.startswith("#!"))) else "code"
    c[(kind,cat)][sign]+=1
tot=collections.Counter()
for (kind,cat),v in sorted(c.items()):
    print(f"{kind:6} {cat:8} +{v['+']:4} -{v['-']:4} net {v['+']-v['-']:+5}")
    tot[cat+'+']+=v['+']; tot[cat+'-']+=v['-']
for cat in ("code","comment","blank"): print(f"total  {cat:8} +{tot[cat+'+']:4} -{tot[cat+'-']:4} net {tot[cat+'+']-tot[cat+'-']:+5}")
```

## The baseline and the oracle

```
git worktree add --detach <master worktree> 0e2ef0f12
cd <master worktree>
./teq interp <worktree>/bench/survey/survey.scala -- --out <worktree>/out/survey \
  --crates <worktree>/bench/survey/crates.txt --jscpd <worktree>/bench/survey/jscpd
# the tokei part again after the recursive count (same revision, the baseline merged):
./teq interp <worktree>/bench/survey/survey.scala -- --out <worktree>/out/survey --only tokei ...
scala-cli run -S 3.8.4 --jvm system --server=false bench/survey/oracle.scala > out/survey/oracle.log
cargo clippy --release -j 8 --message-format short -- -W clippy::all     # CARGO_TARGET_DIR apart, at master
```

## The probes

```
# deflate: probes/deflate/run.sh, each flate2 backend against oracle.log's deflate lines
cargo run --release -q --no-default-features --features <miniz|zlib-rs|zlib-c> -- oracle.log
# json
cargo run --release -q
# refusal: a macro reading Apply.fun off a literal through a cast, both binaries
<master> interp probes/refusal > master.out 2>&1; <spike 3> interp probes/refusal > spike.out 2>&1
```

## The function-walker scan (`baseline/texpr-walkers.txt`), Python run once at master

```python
import re,glob
rec=re.compile(r'^\s*(self\.\w+\(\*?&?\w+\);|self\.\w+\(\*?&?\w+\)|for &?\w+ in .*\{|\}|\{|\};?|if let Some\(\w+\) = \w+ \{)\s*,?$')
arm=re.compile(r'^(\s*)(TExpr::\w+(\(.*?\))?(\s*\|\s*TExpr::\w+(\(.*?\))?)*)\s*=>\s*(.*)$')
# for each `match <prog>.expr(e)` with 15 arms or more: the arms whose every body line is a recursive call on a
# binding, a loop over a list calling one, or a brace; their lines summed per walker
```

## The shapes of install_trees (`spikes/tree-inventory/shapes.txt`), Python run once at master

```python
# the registrations q!(it, "...") of quoted.rs 3562-4787, each classified: an accessor or unapply of one match
# arm over it.view(t)? (`let t = it.tree_arg(a, 0|1)?; match it.view(t)? { View::K(..) => ..., _ => it.unsupported(..) }`),
# others over a view, constructors (.apply, .copy, .of), the rest
```

## The names and messages kept (`spikes/tree-inventory/removed-*.txt`)

```
grep -oE 'q!\(it, "[^"]+"' kinds-before.rs | sed ... | sed 's/^/scala.quoted./' > removed-names.txt
grep -oE 'unsupported\("[^"]+"\)' kinds-before.rs | sed ... > removed-messages.txt
while read -r n; do grep -qaF -- "$n" <spike 3> || echo "missing: $n"; done < removed-names.txt   # and messages
# kinds-before.rs: the 53 registrations of the 15 kinds and Return and Unapply, extracted from master's quoted.rs
```

## The net diffs (`spikes/*/netdiff.txt`), Python run once per spike

```python
# git diff -U0 0e2ef0f12 <spike>: each added or removed line by file kind (rust, scala, shell, doc) and category
# (blank; comment: // or /// or, in shell, #; code: the rest)
```

## Spike 2's planted subset and cancellation

`spikes/case-runner/recipe/plant.scala` replays them whole and is what to run: `subset.txt` (the 23 names whose 46
entries of tests/cases the run kept), `planted/` (the two planted cases with their expectations), the setup, every
phase on both runners and the cancellation; `spikes/case-runner/recipe-run/` holds its run. The first run's shell
commands, of which this record had left out the two `.expected` files (without one a runner selects no case):

```
git worktree add --detach <plant worktree> 227d531cd
git show 0e2ef0f12:tests/run_stdlib.sh > tests/run_stdlib_old.sh
keep="bigint_ops buffer_append_varargs either_right access_qualifiers boolean_strict_operators capture_release_ctor_param \
  cast_statement_werror map_store_callbacks sourcecode anon_js_object_undefined abandoned_operand_frames \
  abstract_constructor_extractor $(grep -v -x -e bigint_ops tests/stdlib-passing.txt | awk 'NR%150==7')"
# every entry of tests/cases whose name is not in $keep removed; the list and the summaries cut to $keep
echo planted >> tests/cases/capture_release_ctor_param.expected
grep -vx access_qualifiers tests/stdlib-passing.txt > l && mv l tests/stdlib-passing.txt
sed -i 's/there were 10 deprecation/there were 11 deprecation/' tests/stdlib-summaries.txt
printf '@main def m(): Unit =\n  while true do ()\n' > tests/cases/zz_planted_loop.scala
echo x > tests/cases/zz_planted_loop.expected
printf '@main def m(): Unit =\n  val x: Int = "s"\n  println(x)\n' > tests/cases/zz_planted_error.scala
echo x > tests/cases/zz_planted_error.expected
STDLIB_REASONS=1 TEQ=<master> tests/run_stdlib_old.sh; STDLIB_REASONS=1 TEQ=<master> ./teq interp tests/run_stdlib.scala
COURSIER_CACHE=/nonexistent ...; ... --update; TEQ=/nonexistent/teq ...; ... -- --teq /nonexistent/teq
setsid -w bash -c 'echo $$ > sid; exec <runner>' & sleep 8; kill -TERM $(cat sid); ps -o pid=,args= -s $(cat sid)
```

## The whole-corpus comparison (`spikes/case-runner/full-compare.txt`)

```
for a in full-old-tests/*.actual; do b=full-new-tests/$(basename $a)
  cmp -s $a $b || cmp -s <(sed 's|<master worktree>/|<root>/|g' $a) <(sed 's|<spike 2 worktree>/|<root>/|g' $b) || ...
done   # identical; the checkout's path alone; --time's figures alone (lines with ms, s, KB, MB left out); otherwise
```

## The lila jars (fetched alone, each against Maven Central's `.sha1`)

```
for p in org/playframework/play-json_3/3.0.6/play-json_3-3.0.6.jar org/playframework/play-functional_3/3.0.6/play-functional_3-3.0.6.jar \
         com/softwaremill/macwire/macros_3/2.6.7/macros_3-2.6.7.jar com/softwaremill/macwire/util_3/2.6.7/util_3-2.6.7.jar \
         org/scala-lang/modules/scala-parser-combinators_3/2.4.0/scala-parser-combinators_3-2.4.0.jar; do
  curl -sS --fail -o $t https://repo1.maven.org/maven2/$p; [ "$(curl -sS https://repo1.maven.org/maven2/$p.sha1)" = "$(sha1sum < $t)" ] && mv $t <coursier cache>/https/repo1.maven.org/maven2/$p
done
```

## This branch's binary against master's

```
cd <master worktree> && cargo build --release      # the same path length as this worktree's
cmp -l <master worktree>/target/release/teq target/release/teq | wc -l
nm <each> | awk '{print $NF}' | sort | diff ...    # the symbols differ only in ThinLTO's .llvm.<hash> suffixes
```
