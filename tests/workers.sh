#!/bin/bash
# The parallel typer at two and sixteen workers, where one worker's output has to be the others':
# every scenario runs the same program at `--threads 1`, then at each count of WORKERS_COUNTS
# (`2 16` by default), and compares what it prints, several times where the workers' schedule
# decides whether a defect shows. `TEQ` names the binary; `TEQ_SHAKE` set in the environment
# reaches every run (src/shake.rs). Every forked build sweeps its merged program for a worker's
# id left (`TEQ_MERGE_SWEEP=1`), and the assertion-enabled build checks
# the parallel merge against the serial walk's order (`TEQ_MERGE_TRACE=1`).
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
COUNTS=${WORKERS_COUNTS:-2 16}
export TEQ_MERGE_SWEEP=${TEQ_MERGE_SWEEP:-1}
export TEQ_MERGE_TRACE=${TEQ_MERGE_TRACE:-1}
. tests/support/jars.sh
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
work=out/workers
rm -rf "$work"
mkdir -p "$work"

# one <name> <count> <teq arguments...>: one invocation at the count; `run <inputs...>` is a build of
# the inputs to $work/<name>.<count>.js followed by node (or, with `--target jvm`, to a jar followed
# by java on it beside the jars given, tests/support/jars.sh's run_jvm), the streams in that order as
# the raw `teq run` printed them, so that the program's output is compared across the counts with the
# compile's notes.
one() {
  local name=$1 t=$2
  shift 2
  if [ "$1" = run ]; then
    shift
    case " $* " in
      *" --target jvm "*) run_jvm 120 "$work/$name.$t.jar" "$(classpath_of "$@")" --build "$@" --threads "$t" ;;
      *) run_js 120 "$work/$name.$t.js" "$@" --threads "$t" ;;
    esac
  else
    timeout 120 "$TEQ" "$@" --threads "$t"
  fi
}
# The word after --classpath among the arguments, or nothing.
classpath_of() {
  while [ $# -gt 1 ]; do
    [ "$1" = --classpath ] && { echo "$2"; return; }
    shift
  done
}

# same_output <name> <trials> <teq arguments...>: the program's output at each count of COUNTS,
# each trial, against its output at one.
same_output() {
  local name=$1 trials=$2
  shift 2
  one "$name" 1 "$@" > "$work/$name.one" 2>&1
  local i t
  for i in $(seq 1 "$trials"); do
    for t in $COUNTS; do
      one "$name" $t "$@" > "$work/$name.$t" 2>&1
      if ! cmp -s "$work/$name.one" "$work/$name.$t"; then
        bad "$name: $t workers differ from one at trial $i: $(diff "$work/$name.one" "$work/$name.$t" | head -3 | tr '\n' ' ' | cut -c1-200)"
        return
      fi
    done
  done
  ok
}

# known <name> <mechanism> <trials> <teq arguments...>: a failure met above one worker by the measurement of
# the parallel typer's causes, reduced and left for its repair: counted apart and
# printed with its mechanism while the counts of COUNTS print otherwise than one, a pass once they do not.
known=0
known() {
  local name=$1 mechanism=$2 trials=$3
  shift 3
  one "$name" 1 "$@" > "$work/$name.one" 2>&1
  local i t
  for i in $(seq 1 "$trials"); do
    for t in $COUNTS; do
      one "$name" $t "$@" > "$work/$name.$t" 2>&1
      if ! cmp -s "$work/$name.one" "$work/$name.$t"; then
        echo "KNOWN $name: $t workers differ from one ($mechanism): $(diff "$work/$name.one" "$work/$name.$t" | grep '^> .' | head -1 | cut -c1-120)"
        known=$((known + 1))
        return
      fi
    done
  done
  ok
}

# same_lines <name> <pattern> <trials> <teq arguments...>: as same_output, of what the pattern
# matches and the exit status alone, where the rest (a profile's times) differs anyway.
same_lines() {
  local name=$1 pattern=$2 trials=$3
  shift 3
  { one "$name" 1 "$@" 2>&1; echo "exit $?"; } | grep -oE "$pattern|^exit [0-9]+" > "$work/$name.one"
  local i t
  for i in $(seq 1 "$trials"); do
    for t in $COUNTS; do
      { one "$name" $t "$@" 2>&1; echo "exit $?"; } | grep -oE "$pattern|^exit [0-9]+" > "$work/$name.$t"
      if ! cmp -s "$work/$name.one" "$work/$name.$t"; then
        bad "$name: $t workers differ from one at trial $i: $(diff "$work/$name.one" "$work/$name.$t" | head -3 | tr '\n' ' ' | cut -c1-200)"
        return
      fi
    done
  done
  ok
}

# same_build <name> <trials> <teq compiler build arguments...>: the files a build writes (`-o`) at each
# count of COUNTS, byte for byte, against one worker's.
same_build() {
  local name=$1 trials=$2
  shift 2
  timeout 120 "$TEQ" compiler build "$@" --threads 1 -o "$work/$name.one" > /dev/null 2>&1
  local i t
  for i in $(seq 1 "$trials"); do
    for t in $COUNTS; do
      rm -rf "$work/$name.$t"
      timeout 120 "$TEQ" compiler build "$@" --threads $t -o "$work/$name.$t" > /dev/null 2>&1
      if ! diff -rq "$work/$name.one" "$work/$name.$t" > "$work/$name.diff" 2>&1; then
        bad "$name: $t workers write other files than one at trial $i: $(head -2 "$work/$name.diff" | tr '\n' ' ' | cut -c1-200)"
        return
      fi
    done
  done
  ok
}

# An inline expansion whose result is the reference to a given the search found before the
# fork (`summonInline` after `warm` primed it), a node another worker made: the walk records
# nothing on it, and the JavaScript is one worker's.
same_build summon_before_fork 3 tests/workers/summon_before_fork

# An attached worker's interpreter reads the initialisers the signature phase registered
# before the fork (the prefix's `top_vals`), whichever worker runs the macro.
same_output registrations 8 interp tests/split/retype_reads

# A local class's opaque type marks the class in the type store by a worker's own id, which
# sizes no dense table (the run at two workers takes seconds and a quarter of a gigabyte when
# it does).
same_output opaque_local 1 interp tests/workers/opaque_local

# A `final val` of a literal type is its literal wherever it is read, by its signature, whichever
# worker types its initialiser and when: no read initialises its object (`constant_by_signature`).
same_output constant_ring 4 interp tests/workers/constant_ring

# A class checked for a macro's run on one worker while its member waits on the other: the
# check is a cell the other worker waits for, and `class_done` is published with the IR.
same_output class_cells 8 interp tests/workers/class_cells

# A typing of more than 65,535 variables names them by their place in it as any typing does
# (`TVarInfo::shown`, 24 bits), never by the count of variables its worker made before.
{ echo 'def id[T](x: T): T = x'; echo 'def arity[F[_, _], A, B](x: F[A, B]): String = "two"'; echo 'def big(): Unit = {'
  yes '  id(1)' | head -65536; echo '  println(arity(Tuple1(1)))'; echo '}'; } > "$work/many_vars.scala"
same_output many_vars 1 compiler check "$work/many_vars.scala"

# A by-name parameter's class, an arity class another worker may have made, is no tuple class:
# `(=> Int) => Int` is shown so at every count (`registered_arity_class`).
same_output byname_types 2 compiler check tests/errors/byname_types.scala

# A library class another thread is completing is waited for before its type parameters are read
# (`settle_class`, `complete_class_tparams`): an array test's element class reads them whole, and
# each of the five tests the type arguments leave unchecked is warned of at every count.
same_output array_test_unchecked 6 compiler check tests/errors/array_test_unchecked.scala --target jvm --werror

# An inline definition's check is its body's wherever a call asks for it first: its errors are
# reported once, at the definition, its calls not expanded, and its plain match warned of once,
# the sealed std class's children completed (`check_definition_in`, the merge's matches and the
# workers' pending sealed files).
same_output inline_definition_failed 8 compiler check tests/errors/inline_definition_failed.scala

# What an inline definition's check leaves for the end, a type argument's bounds or a match, is
# checked once whichever workers asked for the check; each expansion's own checks stay, a quiet
# one's beside a loud one's at the same place (the merge's dedupe by the definition).
same_output inline_definition_bounds 8 compiler check tests/errors/inline_definition_bounds.scala
same_output inline_expansions_quiet 8 compiler check tests/workers/inline_expansions_quiet --werror

# Two diagnostics at one place are presented in the order they were reported in, which the
# workers' schedule decides: a signature's error is reported by whichever worker completes the
# signature first. A build whose workers left two at one place is typed again by one worker
# (`diagnostics_meeting`). A larger file ahead of the error file gives that file to the second
# worker, the first stealing its last item, whose call completes the signature.
printf '// %s\n' "$(printf 'padding %.0s' $(seq 1 1000))" > "$work/padding.scala"
meeting=(compiler check "$work/padding.scala" tests/errors/param_clause_blank_line.scala)
gave=0
for t in $COUNTS; do
  TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" "${meeting[@]}" --threads $t 2>&1 | grep -q 'typed again by one worker: two diagnostics at one place' && gave=$((gave + 1))
done
[ $gave = $(echo $COUNTS | wc -w) ] && ok || bad "diagnostics_meeting: $gave of the counts $COUNTS gave way to one worker"
timeout 120 "$TEQ" "${meeting[@]}" --threads 1 > "$work/meeting.one" 2>&1
differ=""
for s in $(seq 1 12); do
  for t in $COUNTS; do
    TEQ_SHAKE="$s:item,claimed,computed" timeout 120 "$TEQ" "${meeting[@]}" --threads $t > "$work/meeting.$t" 2>&1
    cmp -s "$work/meeting.one" "$work/meeting.$t" || differ="$differ $t workers under TEQ_SHAKE=$s;"
  done
done
[ -z "$differ" ] && ok || bad "diagnostics_meeting_shaken: differ from one worker at$differ"

# A function literal against `PartialFunction` is a partial function literal in every worker, the
# std's `PartialFunction` another worker entered included (`is_partial_function`).
same_output partial_function_match_body 8 run tests/cases/partial_function_match_body.scala

# A given's instance a macro reads, made and published once through the given's body cell, which
# a worker whose macro reads it first waits for or makes (`check_given`).
same_output given_value 4 interp tests/workers/given_value

# A macro's runs on every worker read what other workers published after the fork: a top-level
# function, a class's body and its check, the templates of the calls the function makes, and
# the interpreter's cache of them (tests/support/fork-one-bundles.txt).
same_output bundle_entries 4 interp tests/workers/bundle_entries

# A local class's nested class read the enclosing field the enclosing check marks at its end; a
# body published inside the check leaves the nested class for the check's end.
same_output nested_in_check 4 run tests/workers/nested_in_check

# An anonymous class's merge, settled before the body making it is published, completes the
# signatures of the traits it mixes in; their publication inside the merge leaves the class to the
# publication that settled it.
same_output merge_completes_signature 4 run tests/workers/merge_completes_signature

# A body published inside a merge (the inferred signature the merge completed typed it) names a
# class that body made: the publication settles that class's merge before its seal.
same_output class_made_in_merge 4 interp tests/workers/class_made_in_merge

# A publication inside a merge leaves the class under merge alone, every entry of it: another
# entry's comparison would read the signature the merge is completing and give the build to one
# worker. One busy worker (`TEQ_IDLE_WORKERS=1`) keeps the order that met it.
same_output class_under_merge 2 interp tests/workers/class_under_merge
for t in $COUNTS; do
  if TEQ_IDLE_WORKERS=1 TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" interp tests/workers/class_under_merge --threads $t 2>&1 | grep -q 'typed again'; then
    bad "class_under_merge: a publication inside a merge gave the build to one worker at $t workers"
  else
    ok
  fi
done

# Where the system gives no mapping for the type store's overlays (an address-space limit here, the
# mappings being tens of GB of it), one worker types the build in place and says so, its output one
# worker's; the workers never share the store, which fails on a large build.
refused=$(ulimit -v 2000000 && one overlays_refused 4 interp tests/workers/class_under_merge 2>&1)
note="teq: no memory mapping for the type store's overlays; one worker types the build"
if [ "$(printf '%s\n' "$refused" | head -1)" = "$note" ] && [ "$(printf '%s\n' "$refused" | tail -n +2)" = "$(cat "$work/class_under_merge.one")" ]; then
  ok
else
  bad "overlays_refused: $(printf '%s\n' "$refused" | head -3 | tr '\n' ' ' | cut -c1-200)"
fi

# An abstract type member of a std trait another worker is completing is waited for, never read
# with the namer's placeholder bounds (`member_in_class`): `TEQ_ALIAS_ORDER` holds the first
# worker's completion until another worker has met it in
# progress.
TEQ_ALIAS_ORDER=MirroredElemLabels same_output alias_in_progress 3 run tests/workers/alias_in_progress

# The profile through the merge: the macro histograms' rows name the merged program's functions,
# literals and constructors, and a variable that only a profile's event reaches keeps its instance
# (`Profile::remap`, inside the merge's walk and before the variables' closure).
same_lines profile_macro '^5$|panicked|no record' 2 interp --profile tests/workers/profile_macro
same_lines profile_given 'given Show\[[^]]*\]' 2 compiler check --profile tests/workers/profile_given.scala
same_output shared_vals 4 interp tests/workers/shared_vals
same_output class_ir 8 interp tests/workers/class_ir

# A class another worker checked into its own chunk, whose check reaches a macro whose run
# constructs the class: its cell done, the check is that worker's, published with what reaches
# the class, and not made again here (without, as on master, the check again reaches the macro,
# the macro the construction and the construction the check, until the stack overflows at two
# workers in every run).
same_output class_recheck 3 compiler check tests/workers/class_recheck

# Two classes whose initialisers' macros construct each other: the workers checking them
# demand each other's class, and the wait that would close the cycle is refused and reported
# instead of retried.
same_output class_cycle 3 interp tests/workers/class_cycle

# The branch a folded condition takes is marked by expression in a bit set every worker sets
# and reads, a worker's own expressions included, and merged with them: the marks of a worker
# other than the first once landed past its range and vanished, and the interpreter then ran
# a chain's operands in another order.
same_output taken_marks 3 interp tests/workers/taken_marks

# A macro's run on one worker reads a top-level val of a file another worker's walk typed:
# the val's initialiser is read through that worker's chunk, and the file's initialiser runs
# every val the file defines, whichever worker typed which (without, "p.total has no value to
# read").
same_output file_vals 4 interp tests/workers/file_vals

# The std's and the libraries' classes are claimed and completed within one hold of the
# loader's lock, whose holder then never waits on a cell (without, a worker that claimed a
# class outside the lock waits for the lock while the holder waits for the class).
same_output lock_claims 4 run tests/workers/lock_claims

# A match type reduced through an alias that another worker is completing waits for it
# instead of reading the reduction as stuck, and keeps no stuck answer.
same_output alias_wait 4 run tests/workers/alias_wait

# Inferred signatures that read each other across files, some in a cycle: which member reports
# a cycle is the member the walk met first, which several workers do not keep to, so the build
# is typed again by one worker (the serial path) and reports what one worker reports.
same_output inferred_pairs 3 run tests/workers/inferred_pairs

# A cast names its target, a type a worker may make after the fork: the merge lists the
# expressions' types with the other kinds' before it renumbers them (`parallel::Kind::Exprs`).
same_output cast_target 3 run tests/workers/cast_target

# A package's member another worker entered from the jars between this worker's look at the
# package and its turn at the loader: the loader has nothing to enter for it, and the entries
# are read again (without, "value Macros is not a member of sourcecode" at sixteen workers).
jars_of tests/workers/package_reread
if [ -n "$JARS_MISSING" ]; then
  echo "skip package_reread: not in the coursier cache:$JARS_MISSING"
  ok
else
  same_output package_reread 4 run tests/workers/package_reread --classpath "$JARS_CP"
fi

# The type side of package_reread: a class of scala-library's jar that another worker entered
# between this worker's look at the package and its turn at the loader is read again from the
# entries (without, an interpolated pattern's binders are "not found" in 2 of 8 runs at
# sixteen workers, on master as well).
jars_of tests/workers/type_reread
if [ -n "$JARS_MISSING" ]; then
  echo "skip type_reread: not in the coursier cache:$JARS_MISSING"
  ok
else
  same_output type_reread 4 check tests/workers/type_reread --std=scala-library --classpath "$JARS_CP"
fi

# Thirty derivations from zio-json's jar in one file, whose items the workers share: an object of a jar another
# worker is completing is waited for, not read with its members half entered (`complete_loaded_module`), and a
# library body another worker's hold published is entered by its function only once its templates are published
# (`lock_released`) (without, "JsonCodec[M20] cannot be derived: the companion of the type class has no derived
# method" in every run at sixteen workers once literal types stopped giving these builds to one worker, and now and
# then "no builtin for the template $quoted" in magnolia's `Macro.isObject`).
jars_of tests/workers/jar_module_wait/zio30.scala
if [ -n "$JARS_MISSING" ]; then
  echo "skip jar_module_wait: not in the coursier cache:$JARS_MISSING"
  ok
else
  same_output jar_module_wait 4 run tests/workers/jar_module_wait --classpath "$JARS_CP"
fi

# A macro's runs on several workers make scala-library's `Ordering.Int`, whose trait `CachedReverse` calls
# `super.reverse`: the trait's body is typed under the loader's lock by the first worker whose run needs it, and its
# super calls are registered for every worker, which binds them when its interpreter makes the object (without,
# "scala.math.Ordering$Int$ has no member reverse" on the other workers in 2 of 3 runs at sixteen, which the API
# met in izumi's `Tag`).
jars_of tests/workers/mixin_supers
if [ -n "$JARS_MISSING" ]; then
  echo "skip mixin_supers: not in the coursier cache:$JARS_MISSING"
  ok
else
  same_output mixin_supers 4 check tests/workers/mixin_supers --std=scala-library --classpath "$JARS_CP"
fi

# The same for a trait of the program in a file of its own, which the quoted files' reach does not type before the fork:
# a macro's runs on several workers make `C`, whose trait `T` calls `super.f`; the call is bound from the class
# records where `C`'s record has no binding (another worker's class), which is not changed for it, and the trait's
# super calls are registered for every worker (without, "C has no member f" in 7 of 10 runs at two workers and 10 of
# 10 at sixteen; scalac prints "2 3 4 5 6 7 8 9").
same_output mixin_program 4 run tests/workers/mixin_program

# Imported definitions whose signatures the bodies complete, each in a file of its own: the
# bodies register an import where one of them first meets its definition, the workers' order,
# and the imports the bodies registered are numbered by their module and name once every body
# is typed (without, the output's `$imp` bindings are numbered otherwise in 6 of 6 builds at
# two and at sixteen workers; the application's `react-dom` and `react-dom/client` swapped).
same_build js_imports 3 tests/workers/js_imports

# The type variables another worker solved are read through that worker's table after the
# merge, where the JVM backend erases the types the bodies recorded.
same_build jvm_vars 2 tests/workers/jvm_vars --target jvm

# A type variable another worker solved to a type naming one of its local classes: the merge
# makes the type again under the class's new id, and the variables' instances and bounds with
# it, which the JVM backend erases a recorded variable's type through (without, "no record
# 150994950" in the backend at two and sixteen workers: `local_classes` of the cases, and
# `local_case_classes`, `local_enums`, `inline_arg_summoned_resolution` in both JVM modes).
same_build jvm_local_types 2 tests/workers/jvm_local_types --target jvm

# The classes stored inline bodies make (an anonymous class, a lambda's class of a trait with one
# abstract method), copied at each expansion by substitution in whichever worker types the call,
# named by the site of the expansion and numbered in its order: the files are those one worker
# writes, on both targets, the definition checks of the bodies in each worker that calls them
# consuming no name.
same_build inline_classes 3 tests/workers/inline_classes
same_build inline_classes_jvm 2 tests/workers/inline_classes --target jvm

# The same classes named and outlined at every worker count (tests/workers/inline_names: one
# factory twice under one outer site, an expansion discarded then a live one at the same site):
# the files at each count of COUNTS are those of this binary at one, on both targets and split
# into modules. (Stage 2 compared them with the retype path's at one worker as well, which the
# program's methods no longer take: the baseline is this binary's own at one worker.)
same_as_one_worker() {
  local name=$1 out=$2
  shift 2
  rm -rf "$work/$name.1"
  timeout 120 "$TEQ" compiler build "$@" --threads 1 $out "$work/$name.1" > /dev/null 2>&1
  local t
  for t in $COUNTS; do
    rm -rf "$work/$name.$t"
    timeout 120 "$TEQ" compiler build "$@" --threads $t $out "$work/$name.$t" > /dev/null 2>&1
    if ! diff -rq "$work/$name.1" "$work/$name.$t" > "$work/$name.diff" 2>&1; then
      bad "$name: $t workers write other files than one: $(head -2 "$work/$name.diff" | tr '\n' ' ' | cut -c1-200)"
      return
    fi
  done
  ok
}
same_as_one_worker inline_names -o tests/workers/inline_names
same_as_one_worker inline_names_jvm -o tests/workers/inline_names --target jvm
same_as_one_worker inline_names_split --split tests/workers/inline_names

# A member's result type written `@uncheckedVariance`, its signature completed outside the
# loader's lock by whichever worker reaches it first while another checks its class's
# variances: the mark is published with the signature (without, "contravariant type A occurs
# in covariant position" in 5 of 6 runs at sixteen workers under the shaker).
same_output variance_marks 3 run tests/workers/variance_marks

# A match type reduced to a literal type in one file's body, and a transparent method's folded
# constant in another's: literal types are the program's in every body, as scalac's are, whatever
# the order the bodies are typed in (before, a switch turned them on for the bodies typed after the
# first such body, which gave the build to one worker,
# or with the switch after the fork left out, "value * is not a member of 3" in every run at two and
# sixteen workers).
same_output literal_types 3 run tests/workers/literal_types

# A member's override check reads whether the method it overrides is an alternative, which
# the merge of an ancestor's overloaded entry makes it whenever a lookup gets there: what the
# checks decide from those flags is decided again once every body is typed (without,
# `Counted`'s bridge `handle(Object)` is missing in 4 of 6 builds at sixteen workers).
same_build lineage_merges 3 tests/workers/lineage_merges --target jvm

# A match whose case is reachable through an anonymous class another file's body makes: the
# checks of the matches run once every body is typed, as scalac's do, so no worker's order
# decides the warning (without, one worker warns "unreachable case" when the checker's file
# comes first, and several workers read another worker's class unmerged).
same_output match_hierarchy 3 run tests/workers/match_hierarchy --werror

# A macro whose object counts its runs across files: each run reads what the runs before it
# in file order wrote, which several workers' heaps do not keep, so the watch over the state
# the runs share gives the build to one worker (without, "actual 1"-shaped counts restart on
# every worker).
same_output macro_counter 2 run tests/workers/macro_counter

# The build the serial path types again says so in `--time`: the parallel attempt's time and
# why it gave way.
if timeout 120 "$TEQ" compiler check tests/workers/macro_counter --threads 2 --time 2>&1 | grep -q 'parallel attempt .*gave way to one worker: the macro next changed'; then
  ok
else
  bad "serial_report: --time does not say why the build went to one worker"
fi

# The same counter under `--macro-state per-worker`: each worker's
# runs count on their own heap, the build stays on its workers and `--time` counts the changes it let
# through, and at sixteen workers the counts restart on the workers, the departure from scalac's order
# the flag opts into (one worker's run, which forks nothing, prints scalac's counts).
per_worker() {
  local i out departed=0
  one per_worker 1 run tests/workers/macro_counter > "$work/per_worker.one" 2>&1
  cmp -s "$work/macro_counter.one" "$work/per_worker.one" || { bad "per_worker: one worker under the flag differs from one worker without it"; return; }
  for i in 1 2 3; do
    out=$(TEQ_SERIAL_TRACE=1 one per_worker 16 run tests/workers/macro_counter --macro-state per-worker 2>&1)
    case "$out" in *"typed again"*) bad "per_worker: the flag's build gave way to one worker"; return ;; esac
    [ "$out" != "$(cat "$work/per_worker.one")" ] && departed=1
  done
  [ $departed = 1 ] || { bad "per_worker: sixteen workers under the flag printed one worker's counts in three runs"; return; }
  if timeout 120 "$TEQ" compiler check tests/workers/macro_counter --threads 2 --macro-state per-worker --time 2>&1 | grep -q 'shared state changes let through .*--macro-state per-worker'; then ok; else bad "per_worker: --time does not count the changes the flag let through"; fi
}
per_worker

# What an initialiser's end stamps (`interp::watch`), each a way a value a run makes comes to
# be another run's without the watch's stamp, and each a counter over sixteen expansions that
# one worker takes from 1 to 16 (without, the counts restart on each worker's heap in every
# run at two and sixteen workers): a lazy val's initialiser assigning its object's other field
# an array it makes (`lazy_publish`); a cacheable object's closure keeping a run's array in a
# local of its scope (`cache_frame`); a run's array put into a cacheable slot, then another
# object's initialiser run inside the run, whose end took the run's new contents as its own
# (`nested_dirty`); and a counter a macro's run before the fork changes, in the first worker's
# heap alone (`prefix_touch`: sixteen 1s at one worker, 0s on the other workers).
same_output lazy_publish 2 run tests/workers/lazy_publish
same_output cache_frame 2 run tests/workers/cache_frame --cacheable-state Cache
same_output nested_dirty 2 run tests/workers/nested_dirty --cacheable-state Cache --cacheable-state Other
same_output prefix_touch 2 run tests/workers/prefix_touch

# An initialiser (a module's, a file's, an enum value's, a lazy val's or a lazy local's) may make values and change
# them, and publishes its value in its slot; a change of a container made before it began is state other runs share,
# which gives the build to one worker: two lazy vals counting in their
# object's variable (`lazy_counter`) or in an array it made (`lazy_array`), each expansion on a worker or the first in
# a file with quoted code, typed before the fork (`_prefix`), and two lazy locals counting in their enclosing frame,
# which the object's closures keep (`init_frame`). scalac prints "1, 2" and "11, 22" (without, a lazy initialiser
# resumed its object's epoch, whose changes the watch did not report: "1, 1" and "11, 11" at two and sixteen workers
# in every run).
for s in "lazy_counter lazy val" "lazy_counter_prefix lazy val" "lazy_array lazy val" "lazy_array_prefix lazy val" "init_frame lazy local"; do
  set -- $s
  name=$1
  shift
  same_output $name 2 run tests/workers/$name
  if timeout 120 "$TEQ" compiler check tests/workers/$name --threads 16 --time 2>&1 | grep -q "gave way to one worker: .*in the initialiser of the $* [^ ]*, made before the initialiser began"; then
    ok
  else
    bad "$name: the change of a container made before the initialiser of the $* did not give the build to one worker"
  fi
done

# What the initialisers may change and stay on the workers: a macro's lazy locals counting in a local and an array
# of the run's own frame, made before them and stamped by no initialiser's end (`init_run_own`), and initialisers
# changing what they make (`init_fresh`).
for name in init_run_own init_fresh; do
  same_output $name 2 run tests/workers/$name
  if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/$name -o "$work/$name.js" --threads 16 2>&1 | grep -q 'typed again'; then
    bad "$name: an initialiser's change of what the run or the initialiser made gave the build to one worker"
  else
    ok
  fi
done

# The info messages of the macros' runs are held with their expansions' places until the build is accepted and
# printed once in the order of the places: two a run, in every file, at
# repeated sites and at sites nested in another inline method's (`info_messages`), and before the diagnostics of a
# run's error (`info_error`), both on the workers; and a run's message followed by a change of state other runs share,
# which gives the build to one worker, whose messages alone are printed (`info_giveway`) (without, every message gave
# the build to one worker).
for s in "info_messages 48 0" "info_error 5 0" "info_giveway 4 1"; do
  set -- $s
  same_output $1 2 run tests/workers/$1
  out=$(TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/$1 -o "$work/$1.js" --threads 16 2>&1)
  if [ "$(echo "$out" | grep -c '^info: ')" = $2 ] && [ "$(echo "$out" | grep -c 'typed again')" = $3 ]; then
    ok
  else
    bad "$1: $(echo "$out" | grep -c '^info: ') messages and $(echo "$out" | grep -c 'typed again') builds typed again, where $2 and $3 were expected"
  fi
done

# A local bound in a frame that an object's initialiser inside the run stamped, through a
# closure over the frame the object keeps: the array the local takes is the run's, which the
# runs after change (without, the counts restart on the other worker at two and sixteen).
same_output bind_frame 2 run tests/workers/bind_frame --cacheable-state Cache --cacheable-state Other

# Identity hashes are their values' contents, the same on every
# worker's heap, and give no build away: the hash of an array an object holds (`hash_array`), one an
# object's initialiser keeps after the runs before it hashed on the heap (`init_hash`), every kind of
# value the interpreter keeps one for (`hash_values`), and an object whose lazy val one run reads
# between the prefix's hash of it and the workers' (`hash_lazy`) (without, the first two gave the build
# to one worker, and a lazy slot hashed once read gave the workers' runs a hash of their own).
for name in hash_array init_hash hash_values hash_lazy; do
  same_output $name 2 run tests/workers/$name
  if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/$name -o "$work/$name.js" --threads 16 2>&1 | grep -q 'typed again'; then
    bad "$name: an identity hash gave the build to one worker"
  else
    ok
  fi
done

# A known cache's pooled buffers (izumi's `BufferPool`): what a run releases into the pool is the
# pool's, and the next run that takes a buffer and writes it stays on its worker; with the known
# caches off, the pool's state gives the build to one worker.
jars_of tests/workers/pool_buffers
if [ -n "$JARS_MISSING" ]; then
  echo "skip pool_buffers: not in the coursier cache:$JARS_MISSING"
else
  same_output pool_buffers 2 run tests/workers/pool_buffers --classpath "$JARS_CP" --std scala-library --target jvm
  for t in 2 4; do
    if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler check tests/workers/pool_buffers --classpath "$JARS_CP" --std scala-library --target jvm --threads $t 2>&1 | grep -q 'typed again'; then
      bad "pool_buffers: a run's write of a pooled buffer gave the build to one worker at $t workers"
    else
      ok
    fi
  done
  if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler check tests/workers/pool_buffers --classpath "$JARS_CP" --std scala-library --target jvm --threads 4 --no-known-caches 2>&1 | grep -q 'typed again.*BufferPool'; then
    ok
  else
    bad "pool_buffers: with the known caches off, the pool's state did not give the build to one worker"
  fi
fi

# A session's full build whose parallel attempt gives way is typed again by one worker before it
# is answered (earlier a session typed with one worker whatever its count; earlier still, a
# session at two and sixteen workers kept the attempt: "ok":true and no diagnostics over a file
# with a type error).
session=$(python3 - "$TEQ" <<'PY'
import subprocess, sys
path = "tests/workers/macro_counter/f7.scala"
text = open(path, "rb").read() + b'  val broken: Int = "wrong"\n'
commands = b"text " + path.encode() + b" " + str(len(text)).encode() + b"\n" + text + b"\nbuild\n\nquit\n"
for n in ("1", "16"):
    p = subprocess.run([sys.argv[1], "compiler", "watch", "--check", "tests/workers/macro_counter", "--threads", n], input=commands, capture_output=True, timeout=60)
    out = p.stdout.decode()
    print(n, '"ok":false' in out and "type mismatch" in out)
PY
)
if [ "$session" = "$(printf '1 True\n16 True')" ]; then
  ok
else
  bad "watch_session: $(echo $session)"
fi

# A macro memoising into a map of an object declared as cacheable state: its changes are a
# cache's, no state another run reads differently, and the build stays on its workers.
same_output macro_cache 2 run tests/workers/macro_cache --cacheable-state p.Squares
if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/macro_cache -o "$work/macro_cache.js" --cacheable-state p.Squares --threads 16 2>&1 | grep -q 'typed again'; then
  bad "macro_cache: a cacheable state's changes gave the build to one worker"
else
  ok
fi

# A val beside an inherited Java method without parameters (`get()` of `Supplier`, the application's
# `accessKeyId()` of `AwsCredentials`), which `mark_accessors` names apart (`name_beside_method`): the name is
# computed outside the loader's lock and the member's map entry and flag are published together under it
# (`shared_write`), the member being a record of the shared region (without, as the measurement of the parallel
# typer's causes met it, the forked worker's check asserts "a shared record changed outside the loader's lock" at
# two and sixteen workers in every run, where one worker and scalac print "ab").
same_output beside_java_method 2 run tests/workers/beside_java_method

# A session's full build whose parallel attempt gives way is typed again at once by one worker, as the
# session's next full build would be, and answered once: a check session
# with the navigation index, its count forced so that every attempt gives way, answers its first build,
# two retypes of a file, a build that takes the full path again and the queries after each
# (tests/support/session.py) as the same session at one worker does, byte for byte, and logs each full
# build `retried <n>`; the assertion-enabled build asserts at every retry that the attempt left no
# lock, no staged completion, no overlay and no worker's thread number behind. The counters of the
# bodies (`macro_counter`), the counter a run before the fork changes (`prefix_touch`), and the
# records of an earlier expansion kept, which the build reports (`macro_records_stale`,
# `macro_records_stale_shown` of tests/errors).
session_recovers() {
  local name=$1 file=$2 text=$3 t
  shift 3
  timeout 120 python3 tests/support/session.py "$TEQ" "$file" "$text" -- "$@" --threads 1 > "$work/$name.session.one" 2>&1
  for t in $COUNTS; do
    : > "$work/$name.session.log"
    TEQ_SESSION_WORKERS_LOG=$work/$name.session.log timeout 120 python3 tests/support/session.py "$TEQ" "$file" "$text" -- "$@" --threads $t > "$work/$name.session.$t" 2>&1
    if ! cmp -s "$work/$name.session.one" "$work/$name.session.$t"; then
      bad "session $name: $t workers answer otherwise than one: $(diff "$work/$name.session.one" "$work/$name.session.$t" | head -3 | tr '\n' ' ' | cut -c1-240)"
      return
    fi
    if [ "$(cut -d' ' -f3- "$work/$name.session.log" | tr '\n' ';')" != "0 retried $t;1 retried $t;end 2;" ]; then
      bad "session $name: at $t workers the full builds were not both retried: $(tr '\n' ';' < "$work/$name.session.log")"
      return
    fi
  done
  ok
}
session_recovers macro_counter tests/workers/macro_counter/f3.scala next tests/workers/macro_counter
session_recovers prefix_touch tests/workers/prefix_touch/u3.scala mh tests/workers/prefix_touch
session_recovers macro_records_stale tests/errors/macro_records_stale/use.scala site tests/errors/macro_records_stale
session_recovers macro_records_stale_shown tests/errors/macro_records_stale_shown/use.scala shown tests/errors/macro_records_stale_shown

# A session that gave way at the rule's count types its later full builds with one worker for its lifetime
# and says so once (docs/TARGETS.md, "The typer's workers"): a program above the threshold (eight files of
# comments beside it), at the automatic count, its first build retried with the note, its next two full
# builds (for the session's memory) one worker's without a second note, every answer one worker's session's.
# The note ends as given: a module's state (`macro_counter`) names the module to declare before a restart,
# a cycle of completions (`class_cycle`) declares nothing.
session_remembers() {
  local src=$work/remember t note
  rm -rf "$src"
  cp -r "tests/workers/$1" "$src"
  for t in 0 1 2 3 4 5 6 7; do
    { printf '//'; head -c 70000 /dev/zero | tr '\0' x; printf '\nobject Padding%s\n' $t; } > "$src/pad$t.scala"
  done
  for t in one auto; do
    : > "$work/remember.$t.log"
    printf 'build\nbuild\nquit\n' | env -u TEQ_THREADS -u TEQ_SESSION_WORKERS -u TEQ_SESSION_THREADS TEQ_COMPACT_EVERY=1 TEQ_SESSION_WORKERS_LOG=$work/remember.$t.log \
      timeout 120 "$TEQ" compiler watch --check "$src" $([ $t = one ] && echo --threads 1) 2> "$work/remember.$t.err" | sed -E 's/"ms":\{[^}]*\}//' > "$work/remember.$t"
  done
  local how
  how=$(cut -d' ' -f4- "$work/remember.auto.log" | tr '\n' ';')
  if ! cmp -s "$work/remember.one" "$work/remember.auto"; then
    bad "session remembers: $1: the session at the automatic count answers otherwise than at one worker: $(diff "$work/remember.one" "$work/remember.auto" | head -3 | cut -c1-200 | tr '\n' ' ')"
  elif ! echo "$how" | grep -qE '^retried [0-9]+;one;one;3;$'; then
    bad "session remembers: $1: the full builds were typed $how, not retried once and then by one worker"
  elif [ "$(grep -c 'later full builds of this session type with one worker' "$work/remember.auto.err")" != 1 ] || [ "$(wc -l < "$work/remember.auto.err" | tr -d ' ')" != 1 ]; then
    bad "session remembers: $1: the session's stderr is not the one note: $(head -c 300 "$work/remember.auto.err")"
  elif note=$(cat "$work/remember.auto.err") && [ "${note##*from the start}" != "$2" ]; then
    bad "session remembers: $1: the note ends otherwise: ${note##*from the start}"
  elif [ "$1" = class_cycle ] && grep -q declare "$work/remember.auto.err"; then
    bad "session remembers: $1: the note asks for a declaration: $(head -c 300 "$work/remember.auto.err")"
  else
    ok
  fi
}
session_remembers macro_counter "; later full builds of this session type with one worker, until the session restarts with p.Counter declared"
session_remembers class_cycle "; later full builds of this session type with one worker"

# A file removed: the full build that finds it gone, at a count whose attempt gives way, names it among the
# answer's `removed` as one worker's does (without, the retry read the entries the attempt had already left
# the file out of, and answered `removed` with nothing).
session_removal() {
  local t
  for t in 1 $COUNTS; do
    rm -rf "$work/removal"
    cp -r tests/workers/macro_counter "$work/removal"
    printf 'package p\nclass Gone\n' > "$work/removal/gone.scala"
    timeout 120 python3 - "$TEQ" "$work/removal" "$t" > "$work/removal.$t" 2>&1 <<'PY'
import json, os, subprocess, sys
teq, src, t = sys.argv[1:]
p = subprocess.Popen([teq, "compiler", "watch", "--check", "--index", src, "--threads", t], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def answer():
    a = json.loads(p.stdout.readline())
    a.pop("ms", None)
    print(json.dumps(a, sort_keys=True))
answer()
os.remove(os.path.join(src, "gone.scala"))
p.stdin.write("build\n")
p.stdin.flush()
answer()
p.stdin.write("quit\n")
p.stdin.flush()
p.wait(timeout=60)
PY
    if [ "$t" != 1 ] && ! cmp -s "$work/removal.1" "$work/removal.$t"; then
      bad "session removal: $t workers answer otherwise than one: $(diff "$work/removal.1" "$work/removal.$t" | head -3 | cut -c1-240 | tr '\n' ' ')"
      return
    fi
  done
  grep -q '"removed": \[".*gone.scala"\]' "$work/removal.1" && ok || bad "session removal: one worker's answer names no removed file: $(tail -1 "$work/removal.1" | cut -c1-200)"
}
session_removal

# A session's forked full builds keep nothing of theirs: the allocator's blocks in use (`stats`) after the
# ninetieth as after the thirtieth, within 2 KB in all, through the fork at one worker and at eight (without,
# every forked build lost the peers' table of every shared arena, 48 KB a build, and every worker and merge
# thread its own handle, some sixteen bytes a thread, 6.7 KB over the sixty builds at eight; the fixed builds
# measured 0 and 32 to 240 bytes).
session_flat() {
  local how
  for how in "TEQ_FORK=1 --threads 1" "--threads 8"; do
    local grown
    grown=$(timeout 120 python3 - "$TEQ" $how <<'PY'
import json, os, subprocess, sys
teq = sys.argv[1]
env = {k: v for k, v in os.environ.items() if not k.startswith("TEQ_")}
env["TEQ_COMPACT_EVERY"] = "1"
args = []
for a in sys.argv[2:]:
    if "=" in a:
        k, v = a.split("=", 1)
        env[k] = v
    else:
        args.append(a)
p = subprocess.Popen([teq, "compiler", "watch", "--check", "tests/split/cycle", *args], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
def send(c):
    p.stdin.write(c + "\n")
    p.stdin.flush()
    return json.loads(p.stdout.readline())
json.loads(p.stdout.readline())
live = []
for i in range(1, 91):
    send("build")
    if i in (30, 90):
        live.append(send("stats")["stats"]["allocator"]["live"])
p.stdin.write("quit\n")
p.stdin.flush()
p.wait(timeout=30)
print(live[1] - live[0])
PY
)
    if [ -n "$grown" ] && [ "$grown" -lt 2048 ]; then ok; else bad "session flat ($how): the blocks in use grew by $grown bytes over sixty forked full builds"; fi
  done
}
session_flat

# A session's count (docs/TARGETS.md, "The typer's workers"): `--threads`, then `TEQ_THREADS`, then
# the suites' switches (`TEQ_SESSION_WORKERS` every full build, `TEQ_SESSION_THREADS` the first),
# then the rule over the program's texts at each full build; `TEQ_SESSION_WORKERS=1` keeps it at one.
# A program over the threshold (bench/gen.py's, 51 files), two full builds (the first and one taken
# for the session's memory), the count of each as the session logs it.
session_count() {
  local what=$1 want=$2 got
  shift 2
  : > "$work/count.log"
  printf 'build\nquit\n' | env -u TEQ_THREADS -u TEQ_SESSION_WORKERS -u TEQ_SESSION_THREADS -u TEQ_FORK TEQ_COMPACT_EVERY=1 TEQ_SESSION_WORKERS_LOG=$work/count.log "$@" > /dev/null 2>&1
  got=$(cut -d' ' -f4- "$work/count.log" | head -2 | tr '\n' ';')
  [ -z "$want" ] && return
  if [ "$got" = "$want" ]; then ok; else bad "session count, $what: $got where $want"; fi
}
if timeout 60 python3 bench/gen.py "$work/count-src" 51 22 > /dev/null 2>&1; then
  big=$(cd "$work/count-src" && pwd)
  watch() { echo timeout 120 "$TEQ" compiler watch --check "$big" "$@"; }
  # The rule's count is the machine's (its cores and memory): one count of two or more, the same for
  # both builds, which the switch of the first build alone leaves the second to.
  session_count "the rule" "" $(watch)
  rule=$(cut -d' ' -f4- "$work/count.log" | head -2 | sort -u)
  case $rule in
    "joined "[2-9]* | "joined "[1-9][0-9]*) ok ;;
    *) bad "session count, the rule: one count of two or more expected for both builds: $(tr '\n' ';' < "$work/count.log")" ;;
  esac
  session_count "--threads over TEQ_THREADS and the switches" "joined 3;joined 3;" TEQ_THREADS=5 TEQ_SESSION_WORKERS=4 $(watch --threads 3)
  session_count "TEQ_THREADS over the switches" "joined 5;joined 5;" TEQ_THREADS=5 TEQ_SESSION_WORKERS=4 $(watch)
  session_count "TEQ_SESSION_WORKERS over the rule" "joined 4;joined 4;" TEQ_SESSION_WORKERS=4 $(watch)
  session_count "TEQ_SESSION_THREADS for the first build" "joined 3;$rule;" TEQ_SESSION_THREADS=3 $(watch)
  session_count "TEQ_SESSION_WORKERS=1 pins" "one;one;" TEQ_SESSION_WORKERS=1 $(watch)
  session_count "--threads 1 pins" "one;one;" TEQ_SESSION_WORKERS=4 $(watch --threads 1)
else
  bad "session count: the program over the threshold was not generated"
fi

# The interpreter's watch numbers the typing thread's epochs and containers anew at every full build of a
# session (`interp::dispose`): in the assertion-enabled build, whose `TEQ_SESSION_INVENTORY` reads the typing
# thread's counters before each full build's typer and after its run, a session of three full builds at two
# workers (the third and second taken for the session's memory) numbers each as the first, from the start.
if "$TEQ" --version | grep -q assertions; then
  rm -f "$work/inventory.log"
  printf 'build\nbuild\nquit\n' | env -u TEQ_THREADS TEQ_COMPACT_EVERY=1 TEQ_SESSION_INVENTORY=$PWD/$work/inventory.log timeout 120 "$TEQ" compiler watch --check tests/workers/macro_cache --cacheable-state p.Squares --threads 2 > /dev/null 2>&1
  numbered=$(sed -nE 's/^[0-9]+ full build ([0-9]) after the run;.* next epoch=([0-9]+);.*/\1 \2/p' "$work/inventory.log" | tr '\n' ';')
  started=$(sed -nE 's/^[0-9]+ full build ([0-9]) before the typer;.* next epoch=([0-9]+);.* epoch owners=([0-9]+);.* containers made=([0-9]+);.*/\1 \2 \3 \4/p' "$work/inventory.log" | tr '\n' ';')
  case "$numbered" in
    "0 "[0-9]*";1 "*";2 "*";") ;;
    *) numbered=missing ;;
  esac
  if [ "$numbered" != missing ] && [ "$(echo "$numbered" | cut -d';' -f1 | cut -d' ' -f2)" -gt 1 ] && [ "$started" = "0 1 0 1;1 1 0 1;2 1 0 1;" ]; then ok; else
    bad "the watch's numbering across a session's full builds: after the runs $numbered, before the typers $started"
  fi
else
  echo "workers: the watch's numbering across a session's full builds unproven: the evidence is the assertion-enabled build's TEQ_SESSION_INVENTORY"
fi

# A named local class an inline method's body defines, the definition checked on each worker whose
# expansion asks for it first: the pickle of the body holds the class its block defines once
# (`Capture::block_classes`), at every count as at one (before, the products at two workers held a
# `TYPEDEF` of each worker's class, the definition reconstructed from the class's span).
ledger=tests/modules/ledgerlocalclass
# same_products <name> <sourceroot> <sources...>: the products a check writes at each count of COUNTS,
# byte for byte, against one worker's.
same_products() {
  local name=$1
  shift
  rm -rf "$work/$name.one"
  timeout 120 "$TEQ" compiler check --products "$work/$name.one" --sourceroot "$@" --threads 1 > /dev/null 2>&1
  local t
  for t in $COUNTS; do
    rm -rf "$work/$name.$t"
    timeout 120 "$TEQ" compiler check --products "$work/$name.$t" --sourceroot "$@" --threads $t > /dev/null 2>&1
    if ! diff -rq "$work/$name.one" "$work/$name.$t" > "$work/$name.diff" 2>&1; then
      bad "$name: $t workers write other products than one: $(head -2 "$work/$name.diff" | tr '\n' ' ' | cut -c1-200)"
      return
    fi
  done
  ok
}
same_products ledger_local_class "$ledger" "$ledger"/a/*.scala "$ledger"/b/*.scala

# A body its upstream's products withhold, which a downstream's macro runs into: the diagnostic
# names the member's place in the upstream's source at every count (before, at one worker the
# place of a definition the member's body makes, which has none, and the pseudo file's line).
withheld=tests/modules/macrowithheld
rm -rf "$work/withheld_upstream"
timeout 120 "$TEQ" compiler check --products "$work/withheld_upstream" --sourceroot "$withheld" "$withheld"/a/*.scala --threads 1 > /dev/null 2>&1
same_output withheld_place 2 compiler check --sourceroot "$withheld" --classpath "$work/withheld_upstream" "$withheld"/b/*.scala

echo "workers: $pass passed, $fail failed, $known known"
[ $fail = 0 ]
