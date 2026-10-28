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
# upstream <fixture>: the fixture's macros and the state their runs read (tests/workers/<fixture>/lib) built for
# its program (tests/workers/<fixture>/use) to read off $work/<fixture>_lib: a macro of the run suspends the unit
# that meets it at typing (dotty's `Inliner.expandMacro`), so the macros whose runs on the workers the watch's
# rows test are an upstream module's, where an application keeps its macros.
upstream() {
  rm -rf "$work/$1_lib"
  timeout 120 "$TEQ" compiler check --products "$work/$1_lib" --sourceroot "tests/workers/$1/lib" tests/workers/$1/lib/*.scala --threads 1 > /dev/null 2>&1
}
# fixture <fixture>: the arguments that compile it: its directory, or its program and its macros' library.
fixture() {
  if [ -d "tests/workers/$1/use" ]; then
    echo "tests/workers/$1/use --classpath $work/$1_lib"
  else
    echo "tests/workers/$1"
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

# Two diagnostics at one place are presented in the order one worker's walk reports them, whichever worker types what:
# a signature's error is reported by whichever worker completes the signature first, and the merge places the
# workers' diagnostics by the walk's order, a completion's at its first demand or the build given to one worker
# (`merge_diagnostics`, `events_displaced`). A larger file ahead of the error file gives that file to the second
# worker, the first stealing its last item, whose call completes the signature; shaken, the output is one worker's.
printf '// %s\n' "$(printf 'padding %.0s' $(seq 1 1000))" > "$work/padding.scala"
meeting=(compiler check "$work/padding.scala" tests/errors/param_clause_blank_line.scala)
timeout 120 "$TEQ" "${meeting[@]}" --threads 1 > "$work/meeting.one" 2>&1
differ=""
for s in $(seq 1 12); do
  for t in $COUNTS; do
    TEQ_SHAKE="$s:item,claimed,computed" timeout 120 "$TEQ" "${meeting[@]}" --threads $t > "$work/meeting.$t" 2>&1
    cmp -s "$work/meeting.one" "$work/meeting.$t" || differ="$differ $t workers under TEQ_SHAKE=$s;"
  done
done
[ -z "$differ" ] && ok || bad "diagnostics_meeting_shaken: differ from one worker at$differ"

# Two bindings of one name that denote one object, a val declared as its singleton and a wildcard import of it, are
# no ambiguity at any count (dotty's `isSameRef`, teq's `names_object`): a val whose signature another worker is
# completing is waited for, as one worker's walk reads it complete, and only this worker's own completion under way
# names nothing (before, "reference to TimelineAnimation is ambiguous" in 11 of 60 runs at eight workers shaken at
# `computed`, 2 of 20 unshaken, on master too; scalac accepts it). One worker against the hook at two and eight
# workers, each shaken at forty seeds.
timeout 120 "$TEQ" compiler check tests/cases/same_ref_bindings.scala --threads 1 > "$work/same_ref.one" 2>&1
differ=""
for s in $(seq 1 40); do
  for t in 2 8; do
    hook=()
    [ $t = 2 ] && hook=(TEQ_IDLE_WORKERS=0)
    env "${hook[@]}" TEQ_SHAKE="$s:computed" timeout 120 "$TEQ" compiler check tests/cases/same_ref_bindings.scala --threads $t > "$work/same_ref.$t" 2>&1
    cmp -s "$work/same_ref.one" "$work/same_ref.$t" || differ="$differ $t workers under TEQ_SHAKE=$s;"
  done
done
[ -z "$differ" ] && ok || bad "same_ref_shaken: differ from one worker at$differ"

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

# Two classes whose initialisers' macros construct each other, the macros of the run: their files suspend at
# typing (dotty's `Inliner.expandMacro`) and are typed again after the macros' file, as one worker types them.
# The macros construct the program's classes, so they cannot be an upstream module's: the workers' demand of
# each other's class through such runs, whose closing wait is refused and reported, is no longer met here.
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

# A signature inferred over a join's wildcard of variables (`:=`'s `B` of each pair, left open by
# its interpolation as dotty leaves a nonvariant one) is published with their instance: `zonk`
# reads through the wildcard's bounds. Before, the variables reached the other workers open, whose
# conversions bounded them in the table's dummy and set aside a bound they never pushed (a panic
# above one worker, the type `AttrPair[? >: B <: B, ..]` at one); scalac prints 2 six times.
same_output wild_join_signature 3 run tests/workers/wild_join_signature
if [ "$(cat "$work/wild_join_signature.one")" = "2,2,2,2,2,2" ]; then
  ok
else
  bad "wild_join_signature: one worker's output is not scalac's: $(head -2 "$work/wild_join_signature.one" | tr '\n' ' ' | cut -c1-200)"
fi

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

# A library's block typed again under the loader's lock (an argument typed once more against
# another expected type) enters its locals afresh, and its statements read the last entry, as one
# worker's map gives it: reactivemongo's `productReader`, whose partial function reads the last
# typing's `defaultValueAnns` (without, the shared map kept the first typing's local, and "the local
# defaultValueAnns is not in scope" when another worker runs the macro, at two workers and on master).
jars_of tests/cases/bson_macros.scala
if [ -n "$JARS_MISSING" ]; then
  echo "skip library_block_retyped: not in the coursier cache:$JARS_MISSING"
  ok
else
  same_output library_block_retyped 2 interp tests/cases/bson_macros.scala --classpath "$JARS_CP"
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

# The values of an enum extending `java.lang.Enum` are `Mirror.Singleton`s at every count, as dotty's `DesugarEnums`
# makes each value's class (`ExtendsSingletonMirror`, whatever the enum extends): scala-java-time's `ChronoUnit` from
# its jar and an enum of the program's (without, the parent hung on whether the enum's own parents were complete when
# the value's were made, which the order of the typing decides: none of the nineteen here at any count, some of
# `ChronoUnit`'s in the application's export, other ones at another count; scalac's classes have it, 16 and 3).
jars_of tests/workers/enum_singleton_mirror
if [ -n "$JARS_MISSING" ]; then
  echo "skip enum_singleton_mirror: not in the coursier cache:$JARS_MISSING"
  ok
else
  mirrored=ok
  for t in 1 $COUNTS; do
    got=$(one enum_singleton_mirror "$t" run tests/workers/enum_singleton_mirror --classpath "$JARS_CP" 2>&1 | tr '\n' ' ')
    if [ "$got" != "16 3 " ]; then
      bad "enum_singleton_mirror: $t workers print $got, not scalac's 16 3"
      mirrored=
      break
    fi
  done
  [ -n "$mirrored" ] && ok
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

# A member's signature completed on a worker names the worker's type parameter (`T | UB`), and its
# publication makes the signature's types in the base: the merge makes them again under the
# parameter's new id and retires the originals (`TypeStore::retire_remade`), which the JVM backend's
# completion of the classes a union erases through scans the store for (without, "no record
# 134217728" in `complete_erased_jar_classes` at two and sixteen workers; boundederasure/a of
# tests/modules at two, three and four).
same_build jvm_union_tparams 2 tests/workers/jvm_union_tparams --target jvm

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
# every worker). The macros of this file's fixtures that test the watch are transparent, which the
# typer expands on its workers; a plain one's runs are the inlining phase's, after the merge
# (dotty's `Inlining` after the typer), on one worker below the count of kept calls that pays for
# the phase's fork (`PHASE_FORK_CALLS`), which keeps the build on its workers (`macro_counter_plain`).
upstream macro_counter
same_output macro_counter 2 run tests/workers/macro_counter/use --classpath "$work/macro_counter_lib"
same_output macro_counter_plain 2 run tests/workers/macro_counter_plain
if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/macro_counter_plain -o "$work/macro_counter_plain.js" --threads 16 2>&1 | grep -q 'typed again'; then
  bad "macro_counter_plain: a plain macro's runs gave the build to one worker"
else
  ok
fi

# The inlining phase's second queue, forked whatever the count of kept calls (`TEQ_PHASE_FORK=1`):
# each unit of the walk (a member's or a class statement's calls) expanded whole by one worker, the
# records merged in the walk's order. The expansion cases print one worker's output at two and
# sixteen workers (a class statement's calls after the method before it, `inline_phase_walk_order`'s
# counters as scalac's); a plain macro's counter, whose runs the phase's workers watch as the body
# phase's are, gives the build to one worker and prints one worker's counts; every export case's
# products at two, eight and sixteen workers are one worker's byte for byte (below). Three trials: the
# phase's fork lays its overlays over the pages the body phase's merge gives back, which it waits for
# (without, a worker's first types read as zeroes now and then, `mirror_derives`' `Point` taking the
# `SumOf` case in about three runs in eight under the assertion-enabled build).
export TEQ_PHASE_FORK=1
for c in inline_phase_scopes inline_phase_bridge inline_phase_transparent_nested inline_phase_walk_order inline_phase_sibling_tree inline_phase_macro_reads_kept mirror_derives inline_outline; do
  if [ -d "tests/cases/$c" ]; then src="tests/cases/$c"; else src="tests/cases/$c.scala"; fi
  same_output "phase_fork_$c" 3 run "$src"
done
same_output phase_fork_macro_counter_plain 2 run tests/workers/macro_counter_plain
if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/macro_counter_plain -o "$work/phase_fork_counter.js" --threads 16 2>&1 | grep -q 'typed again'; then
  ok
else
  bad "phase_fork_macro_counter_plain: a plain macro's counter in the forked phase did not give the build to one worker"
fi
# The rows of the inlining phase's rounds and order, at two, eight and sixteen workers against one.
saved_counts=$COUNTS
COUNTS="2 8 16"
# A macro of the program whose run reaches another file's object, whose `derives` holds a plain
# inline call (the application's API: a literal's macro validating through its model's companion):
# the file calling the macro, directly or through an inline method whose anonymous class calls it, is
# suspended (`Inliner.expandMacro`'s `macroDependencies`) and expanded
# in a round of the phase after the merge of the round before it, so no worker's run meets another
# worker's kept call (without, the build went to one worker at two, eight and sixteen). One worker
# prints scalac's output.
same_output phase_fork_macro_reach 3 run tests/workers/phase_fork_macro_reach
if [ "$(cat "$work/phase_fork_macro_reach.one")" = "$(printf 'Code(ABC)\nCode(XYZ)\nName(n)\nQQ:Z')" ]; then
  ok
else
  bad "phase_fork_macro_reach: one worker's output is not scalac's: $(head -3 "$work/phase_fork_macro_reach.one" | tr '\n' ' ' | cut -c1-200)"
fi
if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/phase_fork_macro_reach -o "$work/phase_fork_macro_reach.js" --threads 8 2>&1 | grep -q 'typed again'; then
  bad "phase_fork_macro_reach: a macro's run reaching another file's kept calls gave the build to one worker"
else
  ok
fi
# A library's macro in an inline method of an object of the program whose template holds a plain
# inline call (the application's API: typename's `TypeName` given in an inline method of a
# database object): the run makes no receiver its splice does not name, as dotty's interpreter
# loads the module of the splice's static call alone (`Splicer.checkValidMacroBody`), so it meets
# no other unit's kept call (without, the object was made for the run and the build went to one
# worker at two, eight and sixteen). One worker prints scalac's output.
jars_of tests/workers/phase_fork_receiver
if [ -n "$JARS_MISSING" ]; then
  echo "skip phase_fork_receiver: not in the coursier cache:$JARS_MISSING"
  ok
else
  same_output phase_fork_receiver 3 run tests/workers/phase_fork_receiver --classpath "$JARS_CP"
  if [ "$(cat "$work/phase_fork_receiver.one")" = "$(printf '2\n3\nP(1)')" ]; then
    ok
  else
    bad "phase_fork_receiver: one worker's output is not scalac's: $(head -3 "$work/phase_fork_receiver.one" | tr '\n' ' ' | cut -c1-200)"
  fi
  if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/phase_fork_receiver --classpath "$JARS_CP" -o "$work/phase_fork_receiver.js" --threads 8 2>&1 | grep -q 'typed again'; then
    bad "phase_fork_receiver: a library macro's run made an expansion's receiver and gave the build to one worker"
  else
    ok
  fi
fi
# A counter macro of an upstream module called by two files, the second holding an unused quote: the
# phase takes the units in the walk's order, as `Phase.runOn` does, a file's quoted code suspending
# nothing (before, the file with the quote went first, 2 and 1 at every count; scalac prints 1 and 2).
rm -rf "$work/order_quote_lib"
timeout 120 "$TEQ" compiler check --products "$work/order_quote_lib" --sourceroot tests/workers/phase_order_quote/lib tests/workers/phase_order_quote/lib/M.scala --threads 1 > /dev/null 2>&1
same_output phase_order_quote 3 run tests/workers/phase_order_quote/use --classpath "$work/order_quote_lib"
if [ "$(cat "$work/phase_order_quote.one")" = "$(printf '1\n2')" ]; then
  ok
else
  bad "phase_order_quote: one worker's output is not scalac's: $(head -2 "$work/phase_order_quote.one" | tr '\n' ' ' | cut -c1-200)"
fi
# The same counter called by two files, the first holding an inline branch that would expand a macro of
# the run and is not taken: the phase suspends a file on the macro its expansion meets
# (`Inliner.expandMacro`'s `macroDependencies`), and the files keep the walk's order (before, the
# branch's reach suspended the first file, 2 and 1 at every count; scalac prints 1 and 2). With the
# branch taken before the file's counter, the file is suspended from that call on and its counter runs
# after the second file's (`Run.compileSuspendedUnits`; scalac prints 2 and 1). The fork forced.
for name in phase_dead_macro phase_suspended_file; do
  TEQ_PHASE_FORK=1 same_output $name 3 run tests/workers/$name --classpath "$work/order_quote_lib"
done
for want in "phase_dead_macro 1 2" "phase_suspended_file 2 1"; do
  read -r name lines <<< "$want"
  if [ "$(cat "$work/$name.one")" = "$(printf '%s\n' $lines)" ]; then
    ok
  else
    bad "$name: one worker's output is not scalac's: $(head -2 "$work/$name.one" | tr '\n' ' ' | cut -c1-200)"
  fi
done
# A unit suspended at typing inside a match (a transparent macro of the run in a case's body) and one
# suspended in a signature before a class with a default and its companion: the checks of the matches
# the suspended unit's typing deferred go with the typing, as `Phase.runOn` keeps a unit that suspended
# out of the phases after the typer, `PatternMatcher`'s `SpaceEngine.checkMatch` among them (before,
# the cases after the cut read as wildcards and the check warned of an unreachable case), and the
# getters the walk reaches after the cut stay for their readers, as `Namer`'s completer of a getter
# runs when a reader asks (before, the getter was published suspended and the reader's file suspended
# with it, its counter after the suspended file's: 1, 1 and 2). scalac prints 1, 2 and 1 for both,
# warning of nothing.
for name in phase_suspended_match phase_suspended_getter; do
  TEQ_PHASE_FORK=1 same_output $name 3 run tests/workers/$name --classpath "$work/order_quote_lib"
  if [ "$(cat "$work/$name.one")" = "$(printf '%s\n' 1 2 1)" ]; then
    ok
  else
    bad "$name: one worker's output is not scalac's: $(head -3 "$work/$name.one" | tr '\n' ' ' | cut -c1-200)"
  fi
done
# A macro of the program whose run executes a local trait, and a local class, whose member reaches a
# local given only through a plain call's expansion: the calling file is suspended, and the producer's
# traits lifted and captures completed before the second round runs the macro, as dotty's later run
# finds the producer through `LambdaLift` (before, `the local given_TC is not in scope` at every count;
# scalac prints 42 and 7). And the same macro as the body of a given that `summonInline` finds in an
# inline method's expansion, which no callee's body tells: met at expansion, its unit suspended on a
# worker of the forked phase and its file on one worker, the code emitted alike (before, `No given
# instance of type Box was found` at every count, the given's run failing; scalac prints 42 and 8).
for name in phase_macro_local_trait phase_macro_summoned; do
  TEQ_PHASE_FORK=1 same_output $name 3 run tests/workers/$name
done
same_js=ok
for t in $COUNTS; do
  cmp -s "$work/phase_macro_summoned.1.js" "$work/phase_macro_summoned.$t.js" || same_js=$t
done
if [ "$same_js" = ok ]; then
  ok
else
  bad "phase_macro_summoned: $same_js workers emit other code than one"
fi
for want in "phase_macro_local_trait 42 7" "phase_macro_summoned 42 8"; do
  read -r name lines <<< "$want"
  if [ "$(cat "$work/$name.one")" = "$(printf '%s\n' $lines)" ]; then
    ok
  else
    bad "$name: one worker's output is not scalac's: $(head -2 "$work/$name.one" | tr '\n' ' ' | cut -c1-200)"
  fi
done
# A macro of the program without state met first in two files whose later members make anonymous classes:
# one worker defers each file's calls from that one on, as dotty suspends the file's unit, and a worker of
# the forked phase its unit's, the file's later units staying in the first round, which changes the order
# of no expansion but those of a macro with state (whose run gives the build to one worker, the counter
# rows above): the emitted code is one worker's byte for byte, and scalac's output.
same_output phase_suspended_products 3 run tests/workers/phase_suspended_products
if [ "$(cat "$work/phase_suspended_products.one")" = "$(printf '%s\n' 99 9 16 9 25 99 36)" ]; then
  ok
else
  bad "phase_suspended_products: one worker's output is not scalac's: $(head -2 "$work/phase_suspended_products.one" | tr '\n' ' ' | cut -c1-200)"
fi
same_js=ok
for t in $COUNTS; do
  cmp -s "$work/phase_suspended_products.1.js" "$work/phase_suspended_products.$t.js" || same_js=$t
done
if [ "$same_js" = ok ]; then
  ok
else
  bad "phase_suspended_products: $same_js workers emit other code than one"
fi
# The counter called first in a file that then takes a branch calling a macro of the run: the file's
# round is given up whole and the file goes through the phase again after the second file, its
# counter's expansion made again, as dotty's `Phase.runOn` resets a suspended unit and its later run
# compiles the whole unit (before, the expansions before the suspended call stood, 1 and 2 at every
# count; scalac prints 3 and 2). A file suspended for a macro of a second file, whose own macro a
# third file calls, that macro's run reading the first file's local trait: rounds repeated while
# files suspend, as `Driver.finish` repeats `Run.compileSuspendedUnits`, each producer's traits
# lifted before its consumer's round (before, `the local given_TC is not in scope` at every count;
# scalac prints 42, its trace a third run). And a file whose expansions before its suspended call
# make anonymous classes, given up and made again: one worker's code at every count.
same_output phase_retry_file 3 run tests/workers/phase_retry_file --classpath "$work/order_quote_lib"
same_output phase_third_round 3 run tests/workers/phase_third_round
same_output phase_retry_products 3 run tests/workers/phase_retry_products
# The suspended file typed again from its tree, as dotty's later run types the suspended unit: its
# transparent macro, run at typing time, runs again after the second file's counter (before, the
# file's typing kept, 1 and 2 at every count; scalac prints 3 and 2). And a transparent macro of the
# run met at typing time suspends its file there, which the round after the producer's lifting
# types again (`Inliner.expandMacro` in the typer; before, `the local given_TC is not in scope`;
# scalac prints 42 and 7). A file reading a `val` whose inferred type the macro gives suspends too, as
# dotty's `SuspendCompleter` suspends each reader of the definition (scalac prints 43 and 49).
same_output phase_retry_transparent 3 run tests/workers/phase_retry_transparent --classpath "$work/order_quote_lib"
same_output phase_transparent_producer 3 run tests/workers/phase_transparent_producer
same_output phase_transparent_reader 3 run tests/workers/phase_transparent_reader
# A later run's macros make the program's objects anew and keep the classpath's, as dotty's later run takes a
# macro class loader of its own (`Driver.finish`): a counter of the program run in the second run and again in
# the third counts from its start (before, the third run read the second's object; scalac prints 1 and 101).
same_output phase_run_objects 3 run tests/workers/phase_run_objects
# A chain of inferred signatures over a transparent macro of the run (`A.value` the macro, `B.value = A.value + 1`,
# `C.value = B.value + 1`): every completion on the way is suspended with it and each reader's file waits, as dotty's
# `SuspendException` leaves every completer (before, a completion that read the suspended one published its error
# value: undefined at every count, in the interpreter too; scalac prints 45). And a file whose typing suspended runs
# no macro after it, as dotty unwinds the unit's typer there (before, the upstream counter after the source macro ran
# in the typing given up and again in the later run: 3 and 2 at every count; scalac prints 2 and 1).
same_output phase_inferred_chain 3 run tests/workers/phase_inferred_chain
same_output phase_inferred_chain_interp 3 interp tests/workers/phase_inferred_chain
same_output phase_retry_tail 3 run tests/workers/phase_retry_tail --classpath "$work/order_quote_lib"
# Another file's inline method expanded where the unwinding unit's argument suspended is checked as its own definition
# (before, its record was typed as the unit's rest, an error value, and every later call read it: a type mismatch,
# found Unit; scalac prints 84 and 10).
same_output phase_unwinding_definition 3 run tests/workers/phase_unwinding_definition
# A class after the suspension in its unit's object, checked on the way, reads its parents' declared signatures: a
# completion begins while the unit unwinds, its reach into the unit's code suspending it (before, none began, and the
# class's member was named as a root of its own: "inherits send as (x: Int): Int and as (x: Int): Int"; scalac prints
# 2 and S.Cat).
same_output phase_unwinding_class 3 run tests/workers/phase_unwinding_class
# A macro's run that reaches a definition of a unit its round has not completed suspends its own unit for the next
# round, as dotty's interpreter suspends on a missing class of the current run (`suspendOnMissing`): `A`'s macro runs
# `H.value`, whose file waits on `N`'s macro and whose local trait reads a captured given (before, the run prepared
# `H` in place before its captures: "the local given_TC is not in scope" at every count; scalac prints 42).
same_output phase_dynamic_producer 3 run tests/workers/phase_dynamic_producer
# A retyped unit's walk takes its new trees' classes of blocks (before, the old index left a class's calls after the
# block's: 2,3,1 at every count; scalac prints 1,2,3).
same_output phase_retry_local_order 3 run tests/workers/phase_retry_local_order --classpath "$work/order_quote_lib"
# A forked round suspends a file only where its expansion meets a macro of the run, as one worker does: `A`'s untaken
# branch names `M`'s macro, its later error is reported beside `Z`'s, whose 260 plain calls fork the round (before,
# the static schedule held `A` back and `Z`'s error gated its round: Z_ERROR alone above one worker).
same_output phase_untaken_parallel_errors 3 compiler check tests/workers/phase_untaken_parallel_errors
# The third run's macros make the classpath's objects anew as well, as dotty's third run takes a macro class loader of
# its own (`Driver.finish`), the first two sharing the setup's: an upstream counter run before a source macro whose
# run reaches another waiting producer counts from its start in the third run (before, the classpath's objects stood
# through every run: 4,2,H at every count; scalac prints 1,2,H).
same_output phase_third_round_counter 3 run tests/workers/phase_third_round_counter --classpath "$work/order_quote_lib"
# A file's top-level vals, defs and givens, and an opaque type's companion, are walked after its classes and
# objects, as dotty's desugaring wraps them in the file's `<file>$package`, placed last (`Desugar.packageDef`;
# before, the source's order, 1,2,3,4,5,6,7 at every count; scalac prints 4,1,5,2,6,7,3).
same_output phase_top_level_order 3 run tests/workers/phase_top_level_order --classpath "$work/order_quote_lib"
# A `package object`'s members are the object's, in their order (no package object wraps them again): its vals
# and its nested object are walked as written (before, the vals after the object, as the file's top-level ones;
# 2,1,3 at every count; scalac prints 1,2,3).
upstream phase_package_object_order
same_output phase_package_object_order 3 run tests/workers/phase_package_object_order/use --classpath "$work/phase_package_object_order_lib"
# A file whose expansion meets a macro of the run through an implicit search gives up its whole round at every count:
# `A`'s later error goes with it, and `Z`'s error keeps the next round from running (before, the forked round gave up
# the one unit and reported A_LATE beside Z_ERROR; scalac reports Z_ERROR alone).
same_output phase_given_suspension_errors 3 compiler check tests/workers/phase_given_suspension_errors
if [ "$(grep -c 'error: A_LATE' "$work/phase_given_suspension_errors.one")" = 0 ] && grep -q 'error: Z_ERROR' "$work/phase_given_suspension_errors.one"; then
  ok
else
  bad "phase_given_suspension_errors: one worker's errors are not scalac's: $(grep 'error:' "$work/phase_given_suspension_errors.one" | tr '\n' ' ' | cut -c1-200)"
fi
# Its opposite order: what `A` reported before its suspension stands at every count, as dotty's reporter keeps it
# through the unit's reset (before, the give-up dropped `A_EARLY` at every count; scalac reports both errors).
same_output phase_error_before_suspension 3 compiler check tests/workers/phase_error_before_suspension
if grep -q 'error: A_EARLY' "$work/phase_error_before_suspension.one" && grep -q 'error: Z_ERROR' "$work/phase_error_before_suspension.one"; then
  ok
else
  bad "phase_error_before_suspension: one worker's errors are not scalac's: $(grep 'error:' "$work/phase_error_before_suspension.one" | tr '\n' ' ' | cut -c1-200)"
fi
# The typing's counterpart: `B` reports its error, then reads `A.inferred`, whose transparent macro of the run
# suspended `A`, and suspends with it; what `B` reported before stands, as dotty's `TyperPhase.typeCheck` catches the
# `SuspendException` without retracting it, and `Z`'s error and `B`'s keep the phase from running (before, the
# suspended file's every body diagnostic went, and `B`'s error with it, at 1, 8 and 16 workers; scalac reports
# both), checked whole and built split.
same_output phase_typing_prefix 3 compiler check tests/workers/phase_typing_prefix
if [ "$(grep -c 'B.scala:2:23: error' "$work/phase_typing_prefix.one")" = 1 ] && [ "$(grep -c 'Z.scala:2:24: error' "$work/phase_typing_prefix.one")" = 1 ] && [ "$(grep -c ': error' "$work/phase_typing_prefix.one")" = 2 ]; then
  ok
else
  bad "phase_typing_prefix: one worker's errors are not scalac's: $(grep 'error:' "$work/phase_typing_prefix.one" | tr '\n' ' ' | cut -c1-200)"
fi
same_output phase_typing_prefix_split 3 compiler build tests/workers/phase_typing_prefix --split "$work/phase_typing_prefix_split.out"
if cmp -s <(grep ': error' "$work/phase_typing_prefix.one") <(grep ': error' "$work/phase_typing_prefix_split.one"); then
  ok
else
  bad "phase_typing_prefix_split: the split build's errors are not the check's: $(grep 'error:' "$work/phase_typing_prefix_split.one" | tr '\n' ' ' | cut -c1-200)"
fi
# What stands of a unit suspended at typing is what dotty's traversal reached before the suspension, and what
# that demanded, wherever it lives (`keep_typing_reports`):
# - phase_typing_late_demand: `B.use` reads `Later.bad`, whose body holds an error, before `B.dependent` reads
#   `A.inferred`; the completion `B.use` demanded stands whichever worker typed it (before, it went at 4, 8 and 16
#   workers; scalac reports B:5 and Z:2);
# - phase_typing_top_before: a top-level val with an error stands before `object B`, but dotty's desugaring puts it
#   in the file's package object after `B` (`Desugar.packageDef`), and `B` suspends before it is typed (before, its
#   error stood; scalac reports Z:2 alone).
# Each at 1, 2, 4, 8 and 16 workers, three trials, and at 4 and 8 under the shaker's item, claim and computation
# points (seeds 1 to 4), the errors exactly scalac's.
typing_rows() {
  local name=$1 want=$2 got t i s
  for t in 1 2 4 8 16; do
    for i in 1 2 3; do
      got=$(timeout 120 "$TEQ" compiler check "tests/workers/$name" --threads $t 2>&1 | grep -o '[A-Z][A-Za-z]*\.scala:[0-9]*:[0-9]*: error' | sed 's/: error//' | tr '\n' ' ')
      if [ "$got" != "$want" ]; then
        bad "$name: $t workers report $got, scalac $want"
        return
      fi
    done
  done
  for s in 1 2 3 4; do
    for t in 4 8; do
      got=$(TEQ_SHAKE="$s:item,claimed,computed" timeout 120 "$TEQ" compiler check "tests/workers/$name" --threads $t 2>&1 | grep -o '[A-Z][A-Za-z]*\.scala:[0-9]*:[0-9]*: error' | sed 's/: error//' | tr '\n' ' ')
      if [ "$got" != "$want" ]; then
        bad "$name: $t workers under TEQ_SHAKE=$s report $got, scalac $want"
        return
      fi
    done
  done
  ok
}
typing_rows phase_typing_late_demand "B.scala:5:33 Z.scala:2:24 "
typing_rows phase_typing_top_before "Z.scala:2:24 "
# - phase_typing_demanded_after_cut: `B.z`, after `B`'s cut, is not walked by `B`'s unwinding typer, and `C`'s read
#   completes it, its error standing (before, the unwinding walk published it suspended, `C` suspended on it, and the
#   error went at one worker; scalac reports B:3 and Z:2).
typing_rows phase_typing_demanded_after_cut "B.scala:3:29 Z.scala:2:24 "
# The completions a definition demands through a default and through a type, at their shared boundary (review of
# f35238245, finding 1):
# - phase_typing_demand_default: `B.use` calls `Later.get()`, whose default holds an error, before `B.dependent`
#   reads `A.inferred`; the default is its getter's, completed by the call (`tryDefault`), its error standing
#   (before, the default was typed with `get`'s body, after `B`'s cut, and the error went; scalac reports B:5, Z:2);
# - phase_typing_demand_typeof: `type Use = Later.bad.type` completes `Later.bad`, whose body holds an error, as the
#   singleton path's typing reads its info (`typedSingletonTypeTree`, `checkStable`); scalac reports B:5 and Z:2;
# - phase_typing_alias_after_cut: `type Bad = Missing` after `B`'s cut is never reached by dotty's traversal, and
#   its completion owns what it reports (before, teq's eager phase reported it; scalac reports Z:2 alone).
typing_rows phase_typing_demand_default "B.scala:5:40 Z.scala:2:24 "
typing_rows phase_typing_demand_typeof "B.scala:5:33 Z.scala:2:24 "
typing_rows phase_typing_alias_after_cut "Z.scala:2:24 "
# An alias whose right side reads a val a macro of the run suspends, and a default that reads one, suspend their
# readers' units as a `SuspendCompleter` does, and the retry completes them again (scalac prints 8 for both).
same_output phase_typing_alias_suspends 3 interp tests/workers/phase_typing_alias_suspends
same_output phase_typing_getter_suspends 3 interp tests/workers/phase_typing_getter_suspends
# What an expansion reports after it meets a macro of the run goes with the file's round: `T`'s `inline if`
# cannot reduce once the transparent macro in its condition suspends the file, and dotty's `SuspendException`
# leaves the unit before that (before, the kept error ended the rounds at every count; scalac prints `shown`).
same_output phase_suspension_suffix 3 run tests/workers/phase_suspension_suffix
if [ "$(grep -c 'error: [AZ]_ERROR' "$work/phase_untaken_parallel_errors.one")" = 2 ]; then
  ok
else
  bad "phase_untaken_parallel_errors: one worker does not report both errors: $(grep 'error:' "$work/phase_untaken_parallel_errors.one" | tr '\n' ' ' | cut -c1-200)"
fi
for want in "phase_retry_file 3 2" "phase_third_round 42" "phase_retry_products 9 16 99 25 36 49" "phase_retry_transparent 3 2" "phase_transparent_producer 42 7" "phase_transparent_reader 43 49" "phase_run_objects 1 101" "phase_inferred_chain 45" "phase_inferred_chain_interp 45" "phase_retry_tail 2 1" "phase_unwinding_definition 84 10" "phase_unwinding_class 2 S.Cat" "phase_dynamic_producer 42" "phase_retry_local_order 1,2,3" "phase_third_round_counter 1 2 H" "phase_top_level_order 4,1,5,2,6,7,3" "phase_suspension_suffix shown" "phase_package_object_order 1,2,3" "phase_typing_alias_suspends 8" "phase_typing_getter_suspends 8"; do
  read -r name lines <<< "$want"
  if [ "$(cat "$work/$name.one")" = "$(printf '%s\n' $lines)" ]; then
    ok
  else
    bad "$name: one worker's output is not scalac's: $(head -2 "$work/$name.one" | tr '\n' ' ' | cut -c1-200)"
  fi
done
same_js=ok
for t in $COUNTS; do
  cmp -s "$work/phase_retry_products.1.js" "$work/phase_retry_products.$t.js" || same_js=$t
done
if [ "$same_js" = ok ]; then
  ok
else
  bad "phase_retry_products: $same_js workers emit other code than one"
fi
# A local trait whose member reaches a local given only through a plain call's expansion, lifted after the
# phase with the classes mixing it in (`lift_deferred_traits`; before, `given_TC is not defined` at every
# count; scalac prints 42 and 112).
same_output phase_fork_lifted_trait_captures 3 run tests/cases/inline_phase_lifted_trait_captures.scala
if [ "$(cat "$work/phase_fork_lifted_trait_captures.one")" = "$(cat tests/cases/inline_phase_lifted_trait_captures.expected)" ]; then
  ok
else
  bad "phase_fork_lifted_trait_captures: one worker's output is not scalac's: $(head -2 "$work/phase_fork_lifted_trait_captures.one" | tr '\n' ' ' | cut -c1-200)"
fi
# A macro inspecting a member's retained tree (`--retain-trees`) whose plain call an earlier segment of
# the phase expanded, 320 times over eight files: every worker of the fork reads the journal's typed
# record as the main worker does (before, the workers forked after that segment reflected the
# expansion, 137, 46 and 17 at two, eight and sixteen; scalac with -Yretain-trees prints 320).
same_output phase_fork_retained_journal 3 run tests/workers/phase_fork_retained_journal --retain-trees
if [ "$(cat "$work/phase_fork_retained_journal.one")" = "320" ]; then
  ok
else
  bad "phase_fork_retained_journal: one worker's output is not scalac's: $(head -2 "$work/phase_fork_retained_journal.one" | tr '\n' ' ' | cut -c1-200)"
fi
COUNTS=$saved_counts
# The application corpus's frontend and API (bench/app), whose plain macros run in the phase's workers and
# demand std bodies and classes, which a worker types under the loader's lock: the unboxings its unit's
# expansions wait for are the unit's, not the hold's (without, the lock holder published its expansion's
# records with its own overlay's types, and another worker's `cats.Eq[Boolean]` was not found in about one
# run in five at 2, 8 and 16 workers; the assertion-enabled build's view check names the read).
app_cp=""
for name in scala-library cats-kernel cats-core sourcecode; do
  path=$(jar_of "$name")
  [ -e "$path" ] || app_cp=missing
  [ "$app_cp" = missing ] || app_cp="$app_cp:$path"
done
app_cp=${app_cp#:}
if [ "$app_cp" = missing ]; then
  echo "skip phase_fork_app_frontend: the application corpus's jars are not in the coursier cache"
  ok
elif timeout 120 python3 bench/app/gen.py "$work/app" > /dev/null; then
  same_output phase_fork_app_frontend 3 compiler check "$work/app/shared" "$work/app/frontend" --classpath "$app_cp" --cacheable-state meridian.web.css.Catalog
  same_output phase_fork_app_api 2 compiler check "$work/app/shared" "$work/app/api" --classpath "$app_cp"
  # Positive controls: one worker's check is clean, so that the rows compare the application's inlining.
  for name in phase_fork_app_frontend phase_fork_app_api; do
    if grep -q " error\|^teq: \|panicked" "$work/$name.one"; then
      bad "$name: one worker's check is not clean: $(head -2 "$work/$name.one" | tr '\n' ' ' | cut -c1-200)"
    else
      ok
    fi
  done
else
  bad "phase_fork_app_frontend: the application corpus was not generated"
fi
unset TEQ_PHASE_FORK

# The build the serial path types again says so in `--time`: the parallel attempt's time and
# why it gave way.
if timeout 120 "$TEQ" compiler check tests/workers/macro_counter/use --classpath "$work/macro_counter_lib" --threads 2 --time 2>&1 | grep -q 'parallel attempt .*gave way to one worker: the macro next changed'; then
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
  one per_worker 1 run tests/workers/macro_counter/use --classpath "$work/macro_counter_lib" > "$work/per_worker.one" 2>&1
  cmp -s "$work/macro_counter.one" "$work/per_worker.one" || { bad "per_worker: one worker under the flag differs from one worker without it"; return; }
  for i in 1 2 3; do
    out=$(TEQ_SERIAL_TRACE=1 one per_worker 16 run tests/workers/macro_counter/use --classpath "$work/macro_counter_lib" --macro-state per-worker 2>&1)
    case "$out" in *"typed again"*) bad "per_worker: the flag's build gave way to one worker"; return ;; esac
    [ "$out" != "$(cat "$work/per_worker.one")" ] && departed=1
  done
  [ $departed = 1 ] || { bad "per_worker: sixteen workers under the flag printed one worker's counts in three runs"; return; }
  if timeout 120 "$TEQ" compiler check tests/workers/macro_counter/use --classpath "$work/macro_counter_lib" --threads 2 --macro-state per-worker --time 2>&1 | grep -q 'shared state changes let through .*--macro-state per-worker'; then ok; else bad "per_worker: --time does not count the changes the flag let through"; fi
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
upstream prefix_touch
same_output prefix_touch 2 run $(fixture prefix_touch)

# An initialiser (a module's, a file's, an enum value's, a lazy val's or a lazy local's) may make values and change
# them, and publishes its value in its slot; a change of a container made before it began is state other runs share,
# which gives the build to one worker: two lazy vals counting in their
# object's variable (`lazy_counter`) or in an array it made (`lazy_array`), each expansion on a worker or the first in
# a file with quoted code, typed before the fork (`_prefix`), and two lazy locals counting in their enclosing frame,
# which the object's closures keep (`init_frame`); the macros and the object upstream (`upstream`). scalac prints "1, 2" and "11, 22" (without, a lazy initialiser
# resumed its object's epoch, whose changes the watch did not report: "1, 1" and "11, 11" at two and sixteen workers
# in every run).
for s in "lazy_counter lazy val" "lazy_counter_prefix lazy val" "lazy_array lazy val" "lazy_array_prefix lazy val" "init_frame lazy local"; do
  set -- $s
  name=$1
  shift
  upstream $name
  same_output $name 2 run tests/workers/$name/use --classpath "$work/${name}_lib"
  if timeout 120 "$TEQ" compiler check tests/workers/$name/use --classpath "$work/${name}_lib" --threads 16 --time 2>&1 | grep -q "gave way to one worker: .*in the initialiser of the $* [^ ]*, made before the initialiser began"; then
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
upstream info_giveway
for s in "info_messages 48 0" "info_error 5 0" "info_giveway 4 1"; do
  set -- $s
  same_output $1 2 run $(fixture $1)
  out=$(TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build $(fixture $1) -o "$work/$1.js" --threads 16 2>&1)
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

# A macro's code over the frames the interpreter opens, run by every expansion so that its execution units are
# promoted and read their locals at the addresses of their layouts (src/interp/code.rs) on each worker: blocks,
# lambdas, local defs, binding cases, defaults, a catch case, a lazy local, a local class and a case lambda over a
# tuple, which change nothing other runs share and keep the build on the workers.
same_output frame_layouts 2 run tests/workers/frame_layouts
if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler build tests/workers/frame_layouts -o "$work/frame_layouts.js" --threads 16 2>&1 | grep -q 'typed again'; then
  bad "frame_layouts: the macro's runs gave the build to one worker"
else
  ok
fi

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
# caches off as well, its plain macro's runs being the inlining phase's on one worker after the merge.
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
  if TEQ_SERIAL_TRACE=1 timeout 120 "$TEQ" compiler check tests/workers/pool_buffers --classpath "$JARS_CP" --std scala-library --target jvm --threads 4 --no-known-caches 2>&1 | grep -q 'typed again'; then
    bad "pool_buffers: with the known caches off, the inlining phase's runs gave the build to one worker"
  else
    ok
  fi
fi

# A session's full build whose parallel attempt gives way is typed again by one worker before it
# is answered (earlier a session typed with one worker whatever its count; earlier still, a
# session at two and sixteen workers kept the attempt: "ok":true and no diagnostics over a file
# with a type error).
session=$(python3 - "$TEQ" "$work/macro_counter_lib" <<'PY'
import subprocess, sys
path = "tests/workers/macro_counter/use/f7.scala"
text = open(path, "rb").read() + b'  val broken: Int = "wrong"\n'
commands = b"text " + path.encode() + b" " + str(len(text)).encode() + b"\n" + text + b"\nbuild\n\nquit\n"
for n in ("1", "16"):
    p = subprocess.run([sys.argv[1], "compiler", "watch", "--check", "tests/workers/macro_counter/use", "--classpath", sys.argv[2], "--threads", n], input=commands, capture_output=True, timeout=60)
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
session_recovers macro_counter tests/workers/macro_counter/use/f3.scala next $(fixture macro_counter)
session_recovers prefix_touch tests/workers/prefix_touch/use/u3.scala mh $(fixture prefix_touch)
# The records of an earlier expansion kept (`macro_records_stale`, `macro_records_stale_shown` of
# tests/errors), plain macros whose runs are the inlining phase's on one worker after the merge: every
# full build stays on its workers and answers as one worker's session does.
session_stays() {
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
    if [ "$(cut -d' ' -f3- "$work/$name.session.log" | tr '\n' ';')" != "0 joined $t;1 joined $t;end 2;" ]; then
      bad "session $name: at $t workers the full builds did not stay on their workers: $(tr '\n' ';' < "$work/$name.session.log")"
      return
    fi
  done
  ok
}
session_stays macro_records_stale tests/errors/macro_records_stale/use.scala site tests/errors/macro_records_stale
session_stays macro_records_stale_shown tests/errors/macro_records_stale_shown/use.scala shown tests/errors/macro_records_stale_shown

# A session that gave way at the rule's count types its later full builds with one worker for its lifetime
# and says so once (docs/TARGETS.md, "The typer's workers"): a program above the threshold (eight files of
# comments beside it), at the automatic count, its first build retried with the note, its next two full
# builds (for the session's memory) one worker's without a second note, every answer one worker's session's.
# The note ends as given: a module's state (`macro_counter`, its macro upstream) names the module to declare
# before a restart, a cycle of inferred signatures (`inferred_pairs`) declares nothing.
session_remembers() {
  local src=$work/remember t note from=tests/workers/$1 cp=()
  rm -rf "$src"
  if [ -d "$from/use" ]; then
    from=$from/use
    cp=(--classpath "$work/$1_lib")
  fi
  cp -r "$from" "$src"
  for t in 0 1 2 3 4 5 6 7; do
    { printf '//'; head -c 70000 /dev/zero | tr '\0' x; printf '\nobject Padding%s\n' $t; } > "$src/pad$t.scala"
  done
  for t in one auto; do
    : > "$work/remember.$t.log"
    printf 'build\nbuild\nquit\n' | env -u TEQ_THREADS -u TEQ_SESSION_WORKERS -u TEQ_SESSION_THREADS TEQ_COMPACT_EVERY=1 TEQ_SESSION_WORKERS_LOG=$work/remember.$t.log \
      timeout 120 "$TEQ" compiler watch --check "$src" "${cp[@]}" $([ $t = one ] && echo --threads 1) 2> "$work/remember.$t.err" | sed -E 's/"ms":\{[^}]*\}//' > "$work/remember.$t"
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
  elif [ "$1" = inferred_pairs ] && grep -q declare "$work/remember.auto.err"; then
    bad "session remembers: $1: the note asks for a declaration: $(head -c 300 "$work/remember.auto.err")"
  else
    ok
  fi
}
session_remembers macro_counter "; later full builds of this session type with one worker, until the session restarts with p.Counter declared"
session_remembers inferred_pairs "; later full builds of this session type with one worker"

# A file removed: the full build that finds it gone, at a count whose attempt gives way, names it among the
# answer's `removed` as one worker's does (without, the retry read the entries the attempt had already left
# the file out of, and answered `removed` with nothing).
session_removal() {
  local t
  for t in 1 $COUNTS; do
    rm -rf "$work/removal"
    cp -r tests/workers/macro_counter/use "$work/removal"
    printf 'package p\nclass Gone\n' > "$work/removal/gone.scala"
    timeout 120 python3 - "$TEQ" "$work/removal" "$t" "$work/macro_counter_lib" > "$work/removal.$t" 2>&1 <<'PY'
import json, os, subprocess, sys
teq, src, t, lib = sys.argv[1:]
p = subprocess.Popen([teq, "compiler", "watch", "--check", "--index", src, "--classpath", lib, "--threads", t], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
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
# ninetieth as after the thirtieth, each the minimum over three consecutive builds (an object of about 4 KB
# toggles between builds on master's binary too, so one reading lands on it one time in twelve), within 2 KB in
# all, through the fork at one worker and at eight (without,
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
early, late = [], []
for i in range(1, 91):
    send("build")
    if i in (28, 29, 30):
        early.append(send("stats")["stats"]["allocator"]["live"])
    if i in (88, 89, 90):
        late.append(send("stats")["stats"]["allocator"]["live"])
p.stdin.write("quit\n")
p.stdin.flush()
p.wait(timeout=30)
print(min(late) - min(early))
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
# workers (the third and second taken for the session's memory) numbers each as the first, from the start. The
# fixture's macro is plain, so its runs are the inlining phase's, watched where the phase forks (forced here).
if "$TEQ" --version | grep -q assertions; then
  rm -f "$work/inventory.log"
  printf 'build\nbuild\nquit\n' | env -u TEQ_THREADS TEQ_PHASE_FORK=1 TEQ_COMPACT_EVERY=1 TEQ_SESSION_INVENTORY=$PWD/$work/inventory.log timeout 120 "$TEQ" compiler watch --check tests/workers/macro_cache --cacheable-state p.Squares --threads 2 > /dev/null 2>&1
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

# A record a publication sealed is never changed before the merge (`src/arena.rs`'s check of a published record, the
# assertion-enabled build's): a method with a default argument whose body is typed on demand and published before the
# traversal reaches its getters, its record's defaults waiting for the merge (`fill_own_getters`); and a local enum's
# cases, settled in place once their block knows what they capture, which no publication registers before
# (`settling_classes`, `settle_local_class`). Each under the hook and at four workers, against one worker; the
# release builds carry no such check, so the cases prove the order of their output alone there.
saved_counts=$COUNTS
COUNTS=4
same_output default_getter_sealed 1 compiler check tests/workers/default_getter_sealed
same_output local_enum_sealed 1 compiler check tests/cases/enum_case_defaults.scala
for c in tests/workers/default_getter_sealed tests/cases/enum_case_defaults.scala; do
  if TEQ_IDLE_WORKERS=0 timeout 120 "$TEQ" compiler check "$c" --threads 4 2>&1 | grep -q 'after a publication sealed it'; then
    bad "$c: a record changed after its publication sealed it, under the hook"
  else
    ok
  fi
done
COUNTS=$saved_counts

# The loader's lock holder reads the base's types alone (`types::view`, the assertion-enabled build's check): a
# library class's inherited alternatives merged under the lock read no base instance the worker's own overlay holds,
# which its memo of base types kept from outside the lock (`base_type_of`'s `class_base_memo`; before, "the loader's
# lock holder read types from the holder's own overlay" in every forked check of zio-json's derivation for a case
# class nested in a class, one forked worker included, on master too). Under the hook at two and eight workers; the
# release builds carry no such check, so the case proves nothing there.
jars_of tests/classpath/jvm/nested_object_codec.scala
if [ -n "$JARS_MISSING" ]; then
  echo "skip base_memo_view: not in the coursier cache:$JARS_MISSING"
  ok
else
  for t in 2 8; do
    if TEQ_IDLE_WORKERS=0 timeout 120 "$TEQ" compiler check tests/classpath/jvm/nested_object_codec.scala --classpath "$JARS_CP" --target jvm --std scala-library --threads $t 2>&1 | grep -q 'view check'; then
      bad "base_memo_view: a view check under the hook at $t workers"
    else
      ok
    fi
  done
fi

# The same for `base_type`'s cache (dotty's `baseTypeCache`), kept apart per view as `base_type_of`'s memo is
# (`ViewMemos`, swapped where the loader's lock is taken and released): the Java constructors of a std class absorbed
# under the lock compare their parameters through it (tests/cases/charsequence_release.scala, `new StringBuilder(...)`
# on the JVM, and the push pass's reduction of it, tests/workers/string_builder_view), and a given search in a std body
# typed under the lock for an inferred signature reads a candidate's base through it (tests/cases/std_maps_sets.scala);
# before, "the loader's lock holder read types from the holder's own overlay" in 10 of 10 runs of each of the first
# two under the hook at eight workers. Under the hook at two and eight workers.
for p in tests/cases/charsequence_release.scala tests/workers/string_builder_view tests/cases/std_maps_sets.scala; do
  for t in 2 8; do
    if TEQ_IDLE_WORKERS=0 timeout 120 "$TEQ" compiler check "$p" --std scala-library --target jvm --threads $t 2>&1 | grep -q 'view check'; then
      bad "base_cache_view: a view check in $p under the hook at $t workers"
    else
      ok
    fi
  done
done

# Pending: an import whose `given T` selectors bound the givens it brings (`Namer.importBound`), the bound's type made
# by a worker after the fork, which the merge's renumbering finds no listing of (src/typer/merge/parallel.rs's
# "the merge's renumbering met type N made after the fork, which no listing handed over"): every forked build of these
# three aborts there, at two and at sixteen workers, on master (8907746df3) too. Kept for the repair, counted apart.
for p in tests/errors/given_import_bound_constrained.scala tests/cases/given_import_bounds.scala tests/cases/given_import_bound_situated.scala; do
  known "$(basename "$p" .scala)" "an import's given bound made after the fork, which no listing hands the merge" 1 compiler check "$p"
done

# What a worker reads of its peers' records answers as one worker's walk does (`Worker::attach`'s sweep): a kept call
# of the inlining phase folding a literal an earlier expansion evaluated, on the phase's every worker
# (`InlineState::evaluated`, handed over with `PhaseTables`; before, "Cannot reduce `inline if`" at one worker alone);
# a nested class's secondary constructor whose default reads the enclosing instance, the constructor and its getter
# typed by different workers, one outer parameter for both (`ctor_outer_params`, shared as `outer_this` is; before,
# the getter read an unbound local); and a match type seen through a prefix from eight files, one derived alias for
# every worker (`derived_aliases`, shared as `derived_opaques` is; before, eight type mismatches at eight workers).
TEQ_PHASE_FORK=1 same_output phase_evaluated_literal 3 compiler check tests/workers/phase_evaluated_literal
same_output ctor_outer_getter 3 run tests/workers/ctor_outer_getter
same_output derived_match_alias 3 compiler check tests/workers/derived_match_alias
TEQ_IDLE_WORKERS=0 same_output derived_match_alias_attached 1 compiler check tests/workers/derived_match_alias

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

# Every export case's products at one, two, eight and sixteen workers, byte for byte: forwarders
# are members made where their owners complete, on whichever worker completes them, and the merge
# keeps each one (before, an anonymous class's late forwarders vanished above one worker, `Env.tasty`
# of export_anonymous_value 966 bytes at one and 753 at two). A case reading jars is left out.
saved_counts=$COUNTS
COUNTS="2 8 16"
for src in tests/cases/*export* tests/modules/*export*; do
  case "$src" in *.expected) continue ;; esac
  name=$(basename "$src" .scala)
  if [ -d "$src" ]; then
    files=$(find "$src" -name '*.scala' | sort)
  else
    files=$src
  fi
  grep -q '^// jars:' $files && continue
  if [ -d "$src" ]; then root=$src; else root=tests/cases; fi
  same_products "export_$name" "$root" $files
  TEQ_PHASE_FORK=1 same_products "phase_fork_export_$name" "$root" $files
done
# A plain call inside a transparent expansion keeps the expansion's `INLINED` around it in the products
# whichever worker of the phase expands it: the worker adopts the main worker's records of the unit's
# kept calls (`Capture::adopt`), as dotty pickles before `Inlining` (before, a worker other than the
# main one dropped the wrapper: `Use.tasty` 641 bytes for 724 at sixteen workers, and the macro chain's
# products differed with the fork forced).
for name in phase_fork_products phase_fork_chain_wrap; do
  TEQ_PHASE_FORK=1 same_products "$name" "tests/workers/$name" $(find "tests/workers/$name" -name '*.scala' | sort)
done
COUNTS=$saved_counts

echo "workers: $pass passed, $fail failed, $known known"
[ $fail = 0 ]
