# The sessions over several files that the three watch suites share (tests/split-watch.sh,
# tests/check-watch.sh, tests/jvm-watch.sh), sourced by each after its helpers. A suite gives
# `build`, `stop`, `field`, `has`, `ok`, `bad`, `expect` and
#   retype_start <inputs and flags>   a session of the suite's kind on them,
#   as_fresh <label> <inputs and flags>   what the session answered or wrote, compared with a
#                                         fresh build of the same sources,
#   retype_runs <label> <expected> <main class>   what the session's output prints, where the
#                                         suite has one,
#   sweep_end <label> <inputs and flags>  the same comparison at the end of the sweep,
#   retype_queries <file> <text> <inputs and flags>   the index's answers about the name at
#                                         `text`, where the session keeps an index.
#
# What a retype of one file does to the rest shows only when another file is edited next: a file
# typed again takes its classes out of the program's list and puts them back at its end, so
# whatever holds a position in that list from an earlier build reads another class there.

# tests/split/retype: macros that construct a class with an initialiser, read its lazy val and a
# top-level val and group by a key, called from a class, a top-level def and an object, an
# interpolator among them, an inline call whose argument the interpreter evaluates and one that
# is the first to ask for a signature whose body makes a class. The macros' own file is typed
# when the first of them runs, and makes a class at every run. The entry file holds the first
# class of the program, ahead of everything a macro's run reads. A component is edited, then
# the entry file, then each kind of dependent, and the entry file and the dependents once more. Then a lazy val and a top-level val that no macro has read are
# edited, and a caller is made to read them: the macro fails on any value but the new one. Last
# a type expression is added above a parameter typed `A @uncheckedVariance`.
retype_entry() {
  local src=$work/retype
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype "$src"
  retype_start "$src" "$@"
  expect "retype: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "retype: first build" "$src" "$@"
  local step file edit
  while IFS='|' read -r step file edit; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "retype: $step" "$(field ok),$(field incremental),$(field retyped)" 'true,true,["'"$src/$file"'"]'
    as_fresh "retype: $step" "$src" "$@"
  done <<'STEPS'
a component|panel.scala|s/"panel"/"board"/
the entry file|Main.scala|s/"retype"/"typed"/
an object's macro calls after the entry file|panel.scala|s/count("six")/count("six seven")/
a class's and a top-level def's after it|card.scala|s/"one two three"/"one two"/
an evaluated argument after it|fold.scala|s/List(1, 2, 3)/List(1, 2, 4)/
the entry file again|Main.scala|s/"typed"/"typed twice"/
the grouping macro after it|card.scala|s/"a bb cc ddd"/"a bb cc dddd"/
the evaluated argument's file after it|fold.scala|s/"fold"/"folded"/
the interpolator after it|panel.scala|s/words"four five"/words"four five six"/
values no macro has read|right.scala|s/lazy val n: Int = 1/lazy val n: Int = 2/; s/val rightTop: Int = 10/val rightTop: Int = 20/
a macro's first read of them|sides.scala|s/pick("left", 11)/pick("right", 22)/
a type ascription above an unchecked variance|crate.scala|s/val text = "crate"/val text: String = ("crate": String)/
a body below it|crate.scala|s/def label: String = "crate"/def label: String = "crates"/
STEPS
  retype_runs "retype: the session's output runs" "typed twice 0
card ace 2 1:1/2:2/4:1/4
folded 14 true
board 5 1:1/2:2/2
22
crate1 true" retype.Main
  retype_queries "$src/card.scala" 'count(' "$src" "$@"
  retype_queries "$src/sides.scala" 'Sides' "$src" "$@"
  stop
}

# tests/split/retype_reads: what a macro's run reads of other files without calling a def of
# theirs (the val of an object, a top-level val, the initialiser of a class it constructs), a
# def in the second package block of a file, and an inline def that the macro's implementation
# expands. The expansions elsewhere depend on each, so an edit of any of them takes the full
# path; the caller's own edit is typed incrementally.
retype_reads() {
  local src=$work/reads
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_reads "$src"
  retype_start "$src" "$@"
  expect "reads: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "reads: first build" "$src" "$@"
  local step file edit
  while IFS='|' read -r step file edit; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "reads: $step" "$(field ok),$(field incremental)" 'true,false'
    if has "a macro ran its definitions"; then ok; else bad "reads: $step: fallback reason: $(field fallback)"; fi
    as_fresh "reads: $step" "$src" "$@"
  done <<'STEPS'
the val of an object|conf.scala|s/"app-"/"web-"/
a top-level val|conf.scala|s/"-x"/"-y"/
the initialiser of a class|conf.scala|s/"<"/"["/
an inline def the implementation expands|inl.scala|s/inline def setting: Int = 1/inline def setting: Int = 2/
a def in a package block|helpers.scala|s/def value: Int = 1/def value: Int = 3/
STEPS
  sed -i.bak 's/tagged("one")/tagged("two")/' "$src/use.scala"
  build "$src/use.scala"
  expect "reads: the caller" "$(field ok),$(field incremental)" 'true,true'
  as_fresh "reads: the caller" "$src" "$@"
  retype_runs "reads: the session's output runs" "use web-two-y[w>23 5" reads.Main
  retype_queries "$src/use.scala" 'tagged(' "$src" "$@"
  stop
}

# tests/split/facade: a provider, a facade that exports two of its members (one of them an inline
# method) and a consumer that imports the facade. An edit of the facade's body types the facade
# alone; an edit of the inline body in the provider types the consumer again, whose expansion
# came from the provider through the facade (`inline_deps` records the provider, not the
# facade); a selector changed in an export clause takes the full path.
retype_facade() {
  local src=$work/facade
  rm -rf "$src" "$work/out"
  cp -r tests/split/facade "$src"
  retype_start "$src" "$@"
  expect "facade: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "facade: first build" "$src" "$@"
  retype_runs "facade: the first build runs" "hi x
42
facade" app.Main
  sed -i.bak 's/"facade"/"facade1"/' "$src/facade.scala"
  build "$src/facade.scala"
  expect "facade: its body edited" "$(field ok),$(field incremental),$(field retyped)" 'true,true,["'"$src/facade.scala"'"]'
  as_fresh "facade: its body edited" "$src" "$@"
  sed -i.bak 's/x \* 2/x * 3/' "$src/provider.scala"
  build "$src/provider.scala"
  expect "facade: the exported inline body edited" "$(field ok),$(field incremental)" 'true,true'
  if has '"'"$src/consumer.scala"'"'; then ok; else bad "facade: the exported inline body edited: the consumer is not typed again: $(field retyped)"; fi
  as_fresh "facade: the exported inline body edited" "$src" "$@"
  retype_runs "facade: the session's output runs with the new body" "hi x
63
facade1" app.Main
  sed -i.bak 's/{greet, twice}/{greet, twice, label}/' "$src/facade.scala"
  build "$src/facade.scala"
  expect "facade: a selector added" "$(field ok),$(field incremental)" 'true,false'
  if has 'export clauses'; then ok; else bad "facade: a selector added: fallback reason: $(field fallback)"; fi
  as_fresh "facade: a selector added" "$src" "$@"
  stop
}

# tests/split/facade for a session that checks: an export clause that names nothing is a full
# build with its error; the clause moved down a line is an incremental build that keeps the
# error where it moved to, as is an edit of the facade's body beside it; the clause removed is a
# full build without it.
retype_facade_errors() {
  local src=$work/facade-errors
  rm -rf "$src" "$work/out"
  cp -r tests/split/facade "$src"
  retype_start "$src" "$@"
  expect "facade errors: first build" "$(field ok),$(field incremental)" 'true,false'
  sed -i.bak 's/^  export prov.Provider.{greet, twice}$/  export prov.Provider.{greet, twice}\
  export prov.Provider.missing/' "$src/facade.scala"
  build "$src/facade.scala"
  expect "facade errors: an export of nothing added" "$(field ok),$(field incremental)" 'false,false'
  if has 'cannot export missing: it is not a member of Provider'; then ok; else bad "facade errors: an export of nothing added: $RESULT"; fi
  as_fresh "facade errors: an export of nothing added" "$src" "$@"
  sed -i.bak 's/^object Facade:$/\/\/ moved\
object Facade:/' "$src/facade.scala"
  build "$src/facade.scala"
  expect "facade errors: the clauses moved down a line" "$(field ok),$(field incremental)" 'false,true'
  if has '"line":6,' && has 'cannot export missing'; then ok; else bad "facade errors: the clauses moved down a line: $RESULT"; fi
  as_fresh "facade errors: the clauses moved down a line" "$src" "$@"
  sed -i.bak 's/"facade"/"facade1"/' "$src/facade.scala"
  build "$src/facade.scala"
  expect "facade errors: the body edited beside the error" "$(field ok),$(field incremental)" 'false,true'
  if has 'cannot export missing'; then ok; else bad "facade errors: the body edited beside the error: $RESULT"; fi
  as_fresh "facade errors: the body edited beside the error" "$src" "$@"
  sed -i.bak '/export prov.Provider.missing/d' "$src/facade.scala"
  build "$src/facade.scala"
  expect "facade errors: the export of nothing removed" "$(field ok),$(field incremental),$(field diagnostics)" 'true,false,[]'
  as_fresh "facade errors: the export of nothing removed" "$src" "$@"
  stop
}

# tests/split/retype_kept, for a session that checks: the errors a retype keeps are those of the
# incremental path alone. A bound of a signature written out fails on the type of a val that is
# left to inference; the val's new body gives it another type, which takes the full path, and
# the error goes with the build that had it, in the val's file and in another. What decides is
# the val's own inference, which reported no error: an error that stands elsewhere and that a
# retype reports again (an import in a class's body, in the val's file or in another) keeps no
# build off the full path.
retype_kept() {
  local src=$work/kept
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_kept "$src"
  retype_start "$src" "$@"
  expect "kept: first build" "$(field ok),$(field incremental)" 'false,false'
  as_fresh "kept: first build" "$src" "$@"
  local step file edit path
  while IFS='|' read -r step file edit path; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "kept: $step" "$(field incremental)" "$path"
    as_fresh "kept: $step" "$src" "$@"
  done <<'STEPS'
a body beside the bound's error|far.scala|s/"far"/"far1"/|true
the inferred type the bound hangs on|bound.scala|s/val x = 1/val x = ""/|false
the same in another file|other.scala|s/val y = 1/val y = ""/|false
the type that fails the bound again|bound.scala|s/val x = ""/val x = 2/|false
a body while the error stands|far.scala|s/"far1"/"far2"/|true
an inferred type beside an import's error of a class in its file|one.scala|s/val x = 1/val x = ""/|false
a body of that file after it|one.scala|s/"a"/"b"/|true
an inferred type beside an import's error of a class in another file|two.scala|s/val x = 1/val x = ""/|false
a body of the other file after it|three.scala|s/"e"/"e1"/|true
STEPS
  stop
}

# tests/split/retype_init: a file whose one val is made eager and literal again by an edit of its
# body, beside a def another module calls and makes a function value of, which is not typed
# again: the calls check the file's initialiser, and the def checks it at its entry, while the
# file has one, as in a fresh build.
retype_init() {
  local src=$work/init
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_init "$src"
  sed -i.bak 's/val counted: Int = noisy(1)/val counted: Int = 1/' "$src/lib.scala"
  retype_start "$src" "$@"
  expect "init: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "init: first build" "$src" "$@"
  local step file edit
  while IFS='|' read -r step file edit; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "init: $step" "$(field ok),$(field incremental),$(field retyped)" 'true,true,["'"$src/$file"'"]'
    as_fresh "init: $step" "$src" "$@"
  done <<'STEPS'
the val made eager|lib.scala|s/val counted: Int = 1/val counted: Int = noisy(1)/
the val a literal again|lib.scala|s/val counted: Int = noisy(1)/val counted: Int = 1/
the val eager again|lib.scala|s/val counted: Int = 1/val counted: Int = noisy(1)/
STEPS
  retype_runs "init: the session's output runs" "start
counted
argument
4
List(2, 4)" retypeinit.run
  stop
}

# tests/split/retype_state: a macro that counts its expansions in an object of its own and fails
# at the second, called once. Every build of a session starts without that state, as a fresh
# build does and as scalac's, whose macro classes are loaded anew for every run: the caller's
# second typing is the build's first expansion.
retype_state() {
  local src=$work/state
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_state "$src"
  retype_start "$src" "$@"
  expect "state: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "state: first build" "$src" "$@"
  local step file edit
  while IFS='|' read -r step file edit; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "state: $step" "$(field ok),$(field incremental)" 'true,true'
    as_fresh "state: $step" "$src" "$@"
  done <<'STEPS'
the caller typed again|use.scala|s/"use"/"used"/
and once more|use.scala|s/"used"/"used up"/
STEPS
  retype_runs "state: the session's output runs" "used up only" state.Main
  stop
}

# tests/split/retype_state_pair: a macro whose state connects its expansions, a counter that each
# has to find at the number its caller expects: one in `a.scala`, two in `b.scala`. A build of a
# session expands the macros of the files it types and no others, and starts without the state:
# after an edit of `b.scala` alone the counter is at one, where a fresh build of the same
# sources has no error. scalac 3.8.4 does the same when `b.scala` is compiled alone: under sbt
# 2.0.8 (zinc) the expansion in B reads 1 from a class loader of its own
# (2026-09-28), and under scala-cli's
# incremental compiler this program answers `actual 1`.
retype_state_pair() {
  local src=$work/pair
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_state_pair "$src"
  retype_start "$src" "$@"
  expect "pair: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "pair: first build" "$src" "$@"
  sed -i.bak 's|check(2)|check(2) // again|' "$src/b.scala"
  build "$src/b.scala"
  expect "pair: the second caller typed again" "$(field ok),$(field incremental)" 'false,true'
  if has '"message":"actual 1"'; then ok; else bad "pair: the counter in a build of the second caller alone: $RESULT"; fi
  sed -i.bak 's|check(1)|check(1) // again|' "$src/a.scala"
  sed -i.bak 's|// again|// and again|' "$src/b.scala"
  build "$src/a.scala" "$src/b.scala"
  expect "pair: both callers typed again" "$(field ok),$(field incremental)" 'true,true'
  as_fresh "pair: both callers typed again" "$src" "$@"
  stop
}

# tests/split/retype_errors, for a session that checks (the suite's `diagnostics` is the whole
# list of an answer's): a file for each kind of error that is
# reported where something of a file is resolved and not where a body is typed. What a retype
# does not resolve again keeps its error through the edits of the file's bodies, at the place
# the edit moved it to: a signature written out (a parameter's type, a constructor's, a
# given's, a using clause's, a type parameter's bound, a self type), a type argument of one
# against its bound, an import and an export clause of the file, a parent, a type alias, a
# conflict between inherited members; an edit that the comparison of the signatures lets
# through inside such an error's span (spaces, line breaks, backticks) moves its end. What a
# retype builds again reports again, once: the
# imports in the body of a class, a super call that binds to a value, a val whose type is
# inferred. Each kind is left standing by an edit of a body and then repaired by an edit of its
# own, which takes the full path; every answer is a fresh session's. A kept error decides
# nothing: a val's new type takes the full path while errors of bodies stand elsewhere.
retype_errors() {
  local src=$work/errors
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_errors "$src"
  retype_start "$src" "$@"
  expect "errors: first build" "$(field ok),$(field incremental)" 'false,false'
  as_fresh "errors: first build" "$src" "$@"
  local step file edit path
  while IFS='|' read -r step file edit path; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "errors: $step" "$(field incremental)" "$path"
    as_fresh "errors: $step" "$src" "$@"
  done <<'STEPS'
a file without one|fine.scala|s/"f"/"f1"/|true
a signature's|sig.scala|s/"a"/"a1"/|true
a signature's, moved down a line|sig.scala|s/^class A:/\/\/ moved\nclass A:/|true
a signature's beside an error of the body|sig.scala|s/"a1"/"a1" + 1.nothing/|true
a signature's after the body's is gone|sig.scala|s/"a1" + 1.nothing/"a2"/|true
a bound's|bound.scala|s/"b"/"b1"/|true
an import's|imp.scala|s/"c"/"c1"/|true
the imports' of classes, one nested and one a given's|classimp.scala|s/"a"/"b"/|true
the export clauses', the file's and a class's|exp.scala|s/"e"/"e1"/|true
a parent's|parent.scala|s/"p"/"p1"/|true
a type alias's|alias.scala|s/"alias"/"alias1"/|true
a constructor's, a type parameter's and a self type's|ctor.scala|s/"ctor"/"ctor1"/|true
a given's and a using clause's|giv.scala|s/"u"/"u1"/|true
an inherited conflict's|conflict.scala|s/"both"/"both1"/|true
spaces inside a type argument's|span.scala|s/List\[Int\]/List[  Int  ]/|true
line breaks inside it|span.scala|s/List\[  Int  \]/List[\nInt\n]/|true
backticks around a type's name|span.scala|s/y: Absent/y: `Absent`/|true
a comment and a tab in front of one|span.scala|s/def ticked(/\/* é *\/\tdef ticked(/|true
spaces inside an import|imp.scala|s/import errs.nowhere.Thing/import errs .nowhere  .Thing/|true
backticks around a class's name|conflict.scala|s/class Both extends/class `Both` extends/|true
a super call's|sup.scala|s/"k"/"k1"/|true
an inferred val's, reported again|inferred.scala|s/"i"/"i1"/|true
an inferred type a bound hangs on, beside errors of bodies|hangs.scala|s/val width = 1/val width = ""/|false
a signature's after the full build|sig.scala|s/"a2"/"a3"/|true
the signature repaired|sig.scala|s/x: Missing/x: Int/|false
the bound repaired|bound.scala|s/Box\[Int\]/Box[String]/|false
the import repaired|imp.scala|s/^import errs .nowhere  .Thing$//|false
the spans' errors repaired|span.scala|s/Box\[List\[/List[List[/; s/`Absent`/Int/|false
the classes' imports repaired|classimp.scala|s/import errs.nowhere.X/import errs.Source.given1/; s/import errs.elsewhere.Y/import errs.Source.given1/; s/import errs.nothere.Z/import errs.Source.given1/|false
the export clauses repaired|exp.scala|s/export Source.missing/export Source.given1/; s/^export Source.gone$//|false
the parent repaired|parent.scala|s/ extends Nowhere//|false
the type alias repaired|alias.scala|s/NoSuchType/Int/|false
the constructor, the type parameter and the self type repaired|ctor.scala|s/NoCtorType/Int/; s/NoBound/Int/; s/NoSelf/Any/|false
the given and the using clause repaired|giv.scala|s/NoGivenType/Int/g; s/NoUsing/Int/|false
the inherited conflict repaired|conflict.scala|s/def m: Int = 2/def n: Int = 2/|false
the super call repaired|sup.scala|s/val name: String = "x"/def name: String = "x"/|false
a body while the inferred vals' errors stand|fine.scala|s/"f1"/"f2"/|true
the inferred vals repaired|inferred.scala|s/[123].nothing/1/|false
STEPS
  # The repairs leave imports unused (`classimp.scala`'s): with the index, those are hints.
  local repaired
  repaired=$(diagnostics | python3 -c 'import json, sys; print(json.dumps([d for d in json.load(sys.stdin) if d.get("severity") != "hint"]))')
  expect "errors: every error repaired" "$(field ok),$repaired" 'true,[]'
  sed -i.bak 's/"f2"/"f3"/' "$src/fine.scala"
  build "$src/fine.scala"
  expect "errors: a body after the repairs" "$(field ok),$(field incremental)" 'true,true'
  retype_queries "$src/sig.scala" 'label' "$src" "$@"
  stop
}

# tests/split/retype_roots: what a body asks of the whole output, the program's one call through
# a template on a receiver (`productElement` of a product of its own, in another module) and its
# one `getClass`, which puts the qualified name on every class. Both are taken out by an edit
# and put back: the modules are a fresh build's each time, the other files' among them.
retype_roots() {
  local src=$work/roots
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_roots "$src"
  retype_start "$src" "$@"
  expect "roots: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "roots: first build" "$src" "$@"
  local step file edit
  while IFS='|' read -r step file edit; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "roots: $step" "$(field ok),$(field incremental)" 'true,true'
    as_fresh "roots: $step" "$src" "$@"
  done <<'STEPS'
another file|other.scala|s/"other"/"others"/
the call through the template taken out|use.scala|s/p.productElement(0)/p.productArity/
the getClass taken out|use.scala|s/x.getClass.getName/"named"/
both put back|use.scala|s/p.productArity/p.productElement(1)/; s/= "named"/= x.getClass.getName/
STEPS
  retype_runs "roots: the session's output runs" "use 2 roots.other.Other others" roots.Main
  retype_queries "$src/use.scala" 'productElement' "$src" "$@"
  stop
}

# tests/split/retype_quotes: a macro whose quote makes a class, expanded once in one file and
# twice in another. Each expansion has a copy of the class, and the copies of a file that is
# typed again have to be named as a fresh build names them, whichever file was typed again
# before and whether an edit added an expansion, which holds since the copies are named by the
# site of their expansion.
retype_quotes() {
  local src=$work/quotes
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_quotes "$src"
  retype_start "$src" "$@"
  expect "quotes: first build" "$(field ok),$(field incremental)" 'true,false'
  as_fresh "quotes: first build" "$src" "$@"
  local step file edit
  while IFS='|' read -r step file edit; do
    sed -i.bak "$edit" "$src/$file"
    build "$src/$file"
    expect "quotes: $step" "$(field ok),$(field incremental)" 'true,true'
    as_fresh "quotes: $step" "$src" "$@"
  done <<'STEPS'
the first file|one.scala|s/"one"/"uno"/
the second file|two.scala|s/"two"/"dos"/
the first file again|one.scala|s/shown("first")/shown("1st")/
an expansion more in the first file|one.scala|s/= "uno"/= "uno" + shown("more").show/
STEPS
  retype_runs "quotes: the session's output runs" "unomore! 1st! dos second! third!" quotes.Main
  stop
}

# tests/split/retype_cacheable: objects declared to hold cacheable state (`--cacheable-state`), kept
# across the retypes of a session with what they reach, and never from a full build into the retypes:
# at one worker and at two, the same answers. `cacheable.Cache` counts its expansions (the
# declaration broken on purpose, so that keeping shows in the output and in a warning: a fresh build
# counts one, the first retype after it makes the object anew and counts one again, the kept object
# counts on from the next, and an edit of the object's file makes it again) and holds
# `Token` and an enum case, which stay the one instances when the enum's companion is made again,
# and a closure over itself, a cycle the registry holds, which the retypes keep whole;
# `cacheable.Holder` holds a tree and is made anew with one warning a session, at the first retype
# that would keep it;
# `cacheable.Outer.Inner` is nested in an object; `scala.Option` is the std's or a jar's and
# accepted. An edit of the body of the objects' file takes the full path and makes them again. A
# macro that keeps its expansion's site for the next one gets the diagnostic of a stale record. A
# file a macro reads is read again in every build, an edit of equal length with the time put back
# included (not over scala-library, the JVM's one std or `--std=scala-library`, where the std's
# `getJPath` does not type; the reading macro is a plain inline there: a runner over the JVM sets
# `retype_linked`). An expansion that runs out of its call depth half way through filling `Half` drops
# the caches, so the next one finds it empty. A retype whose expansion reads the file and then runs
# out of its depth leaves no caches, and the next retype reads the file again all the same. A name
# that stops resolving is an error of the build that finds so, and a name that never resolved is the
# same error at the start and after an edit. A build after a failed full one is a retype under
# `--check` and a full build where something is emitted (`$after_failed_full`).
retype_cacheable() {
  local at
  for at in 1 2; do
    retype_cacheable_at $at "$@"
  done
}
retype_cacheable_at() {
  local at=$1
  shift
  local src=$work/cacheable linked=${retype_linked-} data
  rm -rf "$src" "$work/out"
  cp -r tests/split/retype_cacheable "$src"
  case " $* " in
    *" --std=scala-library "*) linked=1 ;;
  esac
  [ -n "$linked" ] && printf 'package cacheable\n\ninline def data: String = "first line of the data"\n' > "$src/datamacro.scala"
  retype_start "$src" "$@" --threads $at --cacheable-state cacheable.Cache --cacheable-state cacheable.Holder --cacheable-state cacheable.Outer.Inner --cacheable-state scala.Option
  expect "cacheable at $at: first build" "$(field ok),$(field incremental)" 'true,false'
  retype_runs "cacheable at $at: the first build runs" "use 1 true 7 1 first line of the data" cacheable.Main
  if has '"uses 1 1"' && has '"same true"'; then ok; else bad "cacheable at $at: the first build's count and token: $RESULT"; fi
  sed -i.bak 's/"use"/"used"/' "$src/use.scala"
  build "$src/use.scala"
  expect "cacheable at $at: the caller typed again" "$(field ok),$(field incremental)" 'true,true'
  if has 'made anew'; then bad "cacheable at $at: a warning where nothing was kept from the full build: $RESULT"; else ok; fi
  retype_runs "cacheable at $at: the first retype after a full build makes the objects anew" "used 1 true 7 1 first line of the data" cacheable.Main
  if has '"uses 1 1"' && has '"same true"'; then ok; else bad "cacheable at $at: the count and the token after the first retype: $RESULT"; fi
  sed -i.bak 's/"used"/"used up"/' "$src/use.scala"
  build "$src/use.scala"
  expect "cacheable at $at: typed again once more" "$(field ok),$(field incremental)" 'true,true'
  if has 'made anew, since it holds a tree'; then ok; else bad "cacheable at $at: the warning on the object holding a tree: $RESULT"; fi
  retype_runs "cacheable at $at: the kept objects count on, the token stays, the tree's holder is made anew" "used up 2 true 7 2 first line of the data" cacheable.Main
  if has '"uses 2 2"' && has '"same true"'; then ok; else bad "cacheable at $at: the count and the token after a second retype: $RESULT"; fi
  if [ -z "$linked" ]; then
    cp -p "$src/data.txt" "$work/data.bak"
    printf 'first line of the DATA\n' > "$src/data.txt"
    touch -r "$work/data.bak" "$src/data.txt"
    sed -i.bak 's/"reads"/"read"/' "$src/reads.scala"
    build "$src/reads.scala"
    expect "cacheable at $at: the reader typed again" "$(field ok),$(field incremental)" 'true,true'
    retype_runs "cacheable at $at: the file a macro reads is read again" "used up 2 true 7 2 first line of the DATA" cacheable.Main
  fi
  printf 'package cacheable\n\nobject Budget:\n  def label: String = "b"\n  def a: Int = half\n  def b: Int = halfMade\n' > "$src/budget.scala"
  build "$src/budget.scala"
  expect "cacheable at $at: a file added, an expansion out of its depth" "$(field ok),$(field incremental)" 'false,false'
  if has "exception in macro: java.lang.StackOverflowError" && has '"message":"half made 0'; then ok; else bad "cacheable at $at: the half-made cache after the depth ran out: $RESULT"; fi
  sed -i.bak 's/"b"/"b1"/' "$src/budget.scala"
  build "$src/budget.scala"
  expect "cacheable at $at: the budget's file typed again" "$(field ok),$(field incremental)" "false,$after_failed_full"
  if has '"message":"half made 0'; then ok; else bad "cacheable at $at: the half-made cache after the retype: $RESULT"; fi
  rm "$src/budget.scala"
  build "$src/budget.scala"
  expect "cacheable at $at: the budget's file removed" "$(field ok),$(field incremental)" 'true,false'
  if [ -z "$linked" ]; then
    printf 'package cacheable\n\nobject Deep:\n  def label: String = "d"\n  def a: String = readDeep(false)\n' > "$src/zdeep.scala"
    build "$src/zdeep.scala"
    expect "cacheable at $at: a reader added" "$(field ok),$(field incremental)" 'true,false'
    sed -i.bak 's/readDeep(false)/readDeep(true)/' "$src/zdeep.scala"
    build "$src/zdeep.scala"
    expect "cacheable at $at: a file read, then the depth out" "$(field ok),$(field incremental)" 'false,true'
    printf 'first line of the DAT2\n' > "$src/data.txt"
    touch -r "$work/data.bak" "$src/data.txt"
    sed -i.bak 's/readDeep(true)/readDeep(false)/' "$src/zdeep.scala"
    build "$src/zdeep.scala"
    expect "cacheable at $at: the reader typed again after the depth ran out" "$(field ok),$(field incremental)" 'true,true'
    if has '"read first line of the DAT2"'; then ok; else bad "cacheable at $at: the file read again after the depth ran out: $RESULT"; fi
    printf 'first line of the DATA\n' > "$src/data.txt"
    touch -r "$work/data.bak" "$src/data.txt"
    rm "$src/zdeep.scala"
    build "$src/zdeep.scala"
    expect "cacheable at $at: the deep reader removed" "$(field ok),$(field incremental)" 'true,false'
  fi
  if [ -z "$linked" ]; then data=DATA; else data=data; fi
  sed -i.bak 's/Cache.uses += 1$/Cache.uses += 1 + 0/' "$src/macros.scala"
  build "$src/macros.scala"
  expect "cacheable at $at: a body of the object's file edited" "$(field ok),$(field incremental)" 'true,false'
  if has '"uses 1 1"'; then ok; else bad "cacheable at $at: the object made again after its file's body edit: $RESULT"; fi
  retype_runs "cacheable at $at: made again by the full build of its file" "used up 1 true 7 1 first line of the $data" cacheable.Main
  printf 'package cacheable\n\nimport scala.quoted.*\n\nobject Keeper:\n  var owner: Any = null\n  def impl(using Quotes): Expr[String] =\n    import quotes.reflect.*\n    if owner == null then owner = Symbol.spliceOwner\n    Expr(owner.asInstanceOf[Symbol].owner.name)\n\ninline def site: String = ${ Keeper.impl }\n' > "$src/keeper.scala"
  printf 'package cacheable\n\nobject Sites:\n  def first: String = site\n  def second: String = site\n' > "$src/sites.scala"
  build "$src/keeper.scala" "$src/sites.scala"
  expect "cacheable at $at: a site's symbol kept for the next expansion" "$(field ok),$(field incremental)" 'false,false'
  if has "a symbol of another expansion's site was used again" && has '(called by site)'; then ok; else bad "cacheable at $at: the stale record's diagnostic: $RESULT"; fi
  rm "$src/keeper.scala" "$src/sites.scala"
  build "$src/keeper.scala" "$src/sites.scala"
  expect "cacheable at $at: the keeper removed" "$(field ok),$(field incremental)" 'true,false'
  sed -i.bak 's/Cache/Cache2/g' "$src/macros.scala"
  build "$src/macros.scala"
  expect "cacheable at $at: the object renamed" "$(field ok),$(field incremental)" 'false,false'
  if has '"message":"--cacheable-state cacheable.Cache: no such object"'; then ok; else bad "cacheable at $at: the name that stopped resolving: $RESULT"; fi
  sed -i.bak 's/Cache2/Cache/g' "$src/macros.scala"
  build "$src/macros.scala"
  expect "cacheable at $at: the object renamed back" "$(field ok),$(field incremental)" 'true,false'
  if has '"uses 1 1"'; then ok; else bad "cacheable at $at: the object's file edited, the object made again: $RESULT"; fi
  retype_runs "cacheable at $at: made again by the full build" "used up 1 true 7 1 first line of the $data" cacheable.Main
  stop
  rm -rf "$work/out"
  retype_start "$src" "$@" --threads $at --cacheable-state cacheable.Nowhere
  expect "cacheable at $at: a name that matches nothing" "$(field ok),$(field incremental)" 'false,false'
  if has '"message":"--cacheable-state cacheable.Nowhere: no such object"'; then ok; else bad "cacheable at $at: the error of the name: $RESULT"; fi
  sed -i.bak 's/"used up"/"used"/' "$src/use.scala"
  build "$src/use.scala"
  expect "cacheable at $at: the same error after an edit" "$(field ok),$(field incremental)" "false,$after_failed_full"
  if has '"message":"--cacheable-state cacheable.Nowhere: no such object"'; then ok; else bad "cacheable at $at: the error of the name after an edit: $RESULT"; fi
  stop
}

# tests/split/retype_jar_state: a macro counts with an object of tests/support/state_lib.scala's
# jar, which lies under a directory and has a file name that begin like scala-library's (counted as
# passed without scala-cli to build it). Not declared, the object is made anew in every build, a
# retype of the caller included, as scalac makes a jar's classes anew for every compiler run;
# declared to hold cacheable state, it counts on from one retype to the next (the first retype after
# a full build makes it anew: a full build's state is not the retypes').
retype_jar_state() {
  local jar src=$work/jarstate declared n
  jar=$(jar_of state-lib)
  if [ ! -f "$jar" ]; then
    echo "skip the jar's state: state-lib was not built"
    ok
    return
  fi
  for declared in "" "--cacheable-state statelib.Counter"; do
    rm -rf "$src" "$work/out"
    cp -r tests/split/retype_jar_state "$src"
    retype_start "$src" "$@" --classpath "$jar" $declared
    expect "jar state ($declared): first build" "$(field ok),$(field incremental)" 'true,false'
    if has '"counter 1"'; then ok; else bad "jar state ($declared): the first count: $RESULT"; fi
    retype_runs "jar state ($declared): the first build runs" "use 1" jarstate.Main
    sed -i.bak 's/"use"/"used"/' "$src/use.scala"
    build "$src/use.scala"
    expect "jar state ($declared): the caller typed again" "$(field ok),$(field incremental)" 'true,true'
    if has '"counter 1"'; then ok; else bad "jar state ($declared): the count after the first retype, 1 expected: $RESULT"; fi
    retype_runs "jar state ($declared): the retype runs" "used 1" jarstate.Main
    sed -i.bak 's/"used"/"used up"/' "$src/use.scala"
    build "$src/use.scala"
    expect "jar state ($declared): the caller typed again once more" "$(field ok),$(field incremental)" 'true,true'
    if [ -z "$declared" ]; then n=1; else n=2; fi
    if has "\"counter $n\""; then ok; else bad "jar state ($declared): the count after a second retype, $n expected: $RESULT"; fi
    retype_runs "jar state ($declared): the second retype runs" "used up $n" jarstate.Main
    stop
  done
}

# retype_sweep <flags>: the realistic frontend of bench/app, generated here, over the jars it
# reads (counted as passed without them). A comment goes onto the second line of a file, which
# moves every position of the file and types all of it again: of the first file in the order
# of the paths, the entry file, and every eighth after it, or every `RETYPE_SWEEP_EVERY`th (1
# for all of them, a thousand builds); then of the entry file and every thirty-seventh after
# it, four of which the first round typed and the others it did not. Every answer has to be
# ok, and the suite's
# `sweep_end <label> <inputs and flags>` compares what the session holds at the end with a fresh
# build. The sweep ends at the first answer that is not ok: with an error standing every later
# build is a full one. The catalog of the corpus's class macros is declared to hold cacheable
# state, as the application's is.
retype_sweep() {
  local cp="" name path every=${RETYPE_SWEEP_EVERY:-8}
  for name in scala-library cats-kernel cats-core sourcecode; do
    path=$(jar_of "$name")
    if [ ! -e "$path" ]; then
      echo "skip the sweep: $name is not in the coursier cache"
      ok
      return
    fi
    cp="$cp:$path"
  done
  local app=$work/app
  rm -rf "$app" "$work/out"
  if ! timeout 120 python3 bench/app/gen.py "$app" > "$work/gen.log" 2>&1; then
    bad "sweep: the corpus was not generated: $(tail -1 "$work/gen.log")"
    return
  fi
  local inputs="$app/shared $app/frontend --classpath ${cp#:} --cacheable-state meridian.web.css.Catalog"
  retype_start $inputs "$@"
  expect "sweep: first build" "$(field ok)" 'true'
  local file round at typed=0
  local files=$(find "$app/shared" "$app/frontend" -name '*.scala' | LC_ALL=C sort)
  expect "sweep: the entry file is the first in order" "$(echo "$files" | head -1)" "$app/frontend/frontend/Main.scala"
  for round in $every 37; do
    at=0
    for file in $files; do
      at=$((at + 1))
      [ $(((at - 1) % round)) = 0 ] || continue
      sed -i.bak '1a\
// sweep
' "$file"
      build "$file"
      if [ "$(field ok)" != true ]; then
        bad "sweep: ${file#$app/} after $typed files: $(echo "$RESULT" | head -c 400)"
        stop
        return
      fi
      typed=$((typed + 1))
    done
    ok
  done
  sweep_end "sweep: after $typed files" $inputs "$@"
  stop
}
