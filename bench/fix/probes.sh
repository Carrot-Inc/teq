#!/bin/bash
# bench/fix/probes.sh <label> <teq> [kinds]: jzon's rules' phase at one worker, with the rules' CPU time from TEQ_FIX_TRACE,
# for each shipped rule alone and for the cost probes of bench/fix/probes (an empty rule, a pre-order `traverse`, a
# `collect` of every node, `tokens.toList`, a kind a token, three `is` a token, two extractors); with `kinds`, the ways to
# read a token's kind of bench/fix/probes/Kinds.scala instead (a rule of `package teq.fix.probe`, so that it reads the
# `private[fix]` members). Lines `<label> <probe> <rules ms> <CPU ms>` to FIX_PROBES (default out/measure/probes.tsv),
# each only from a probe whose job completed: the corpus present (under FIX_CORPORA, default out/corpora), teq's exit 0
# or 1, its phases' trace with the rules' time; a probe that fails appends nothing, keeps teq's diagnostics in
# out/measure/probes-rejected/ and makes the runner exit 1 (bench/fix/selftest.sh holds it to that).
cd "$(dirname "$0")/../.."
label=$1 teq=$(cd "$(dirname "$2")" && pwd)/$(basename "$2") mode=${3:-}
out=${FIX_PROBES:-out/measure/probes.tsv}
corpus=$PWD/${FIX_CORPORA:-out/corpora}/jzon
case ${FIX_CORPORA:-} in /*) corpus=$FIX_CORPORA/jzon ;; esac
ls "$corpus"/*.scala > /dev/null 2>&1 || { echo "probes: no corpus at $corpus (bench/fix/corpora.sh makes it)"; exit 1; }
mkdir -p out/measure/cache-probes out/measure/probes-rejected
if [ "$mode" = kinds ]; then
  confs=$(ls bench/fix/probes/k-*.conf)
else
  confs="bench/fix/one-DisableSyntax.conf bench/fix/one-CatsImplicitsImport.conf bench/fix/one-TypelevelAs.conf bench/fix/one-EmptyCollectionsUnified.conf"
  for p in Empty Traverse Walk TokList TokKind TokIs Infix ApplySel; do confs="$confs bench/fix/probes/$p.conf"; done
fi
root=$PWD
failed=0
for conf in $confs; do
  probe=$(basename "$conf" .conf)
  err=$(cd "$(dirname "$conf")" && TEQ_CACHE_DIR=$root/out/measure/cache-probes TEQ_FIX_TRACE=1 timeout 300 "$teq" compiler fix "$corpus" --config "$(basename "$conf")" --check --threads 1 2>&1 >/dev/null)
  code=$?
  line=$(echo "$err" | grep 'ms in all')
  rules=$(echo "$line" | sed -n 's/.*the rules \([0-9]*\) ms.*/\1/p'); cpu=$(echo "$line" | sed -n 's/.*, \([0-9]*\) ms of CPU.*/\1/p')
  if { [ $code != 0 ] && [ $code != 1 ]; } || [ -z "$rules" ] || [ -z "$cpu" ]; then
    echo "$err" > "out/measure/probes-rejected/$label-$probe.log"
    echo "REJECTED probe $probe: teq exited $code: $(echo "$err" | grep -v '^fix: ' | head -2 | tr '\n' ' ')(out/measure/probes-rejected/$label-$probe.log)"
    failed=1
    continue
  fi
  printf '%s\t%s\t%s\t%s\n' "$label" "$probe" "$rules" "$cpu" | tee -a "$out"
done
exit $failed
