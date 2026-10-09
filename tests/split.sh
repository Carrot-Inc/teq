#!/bin/bash
# The split output (`--split dir`): every program of tests/cases and tests/interop is built as a
# directory of modules and run from its main.mjs against the same .expected file its single-file
# build is checked with (a harness imports the entry in the place of the single file), but one of
# the JVM's platform (`//> using platform jvm`), which has no JavaScript run. Then the
# cases under tests/split: two packages that refer to each other in both directions, also while
# loading (cycle), enums shared across packages (enum_shared), two files of one package as two
# modules under --module-per-file (small), what a module takes of a per-file one (taken) and what
# it holds of one (held), the order in which a build reaches a page (publish); every one of
# them with each package per file under --hot; hello world's module count and size; the module names, guards
# and $hot of --module-per-file and --hot, and the objects --hot reports; that an empty file added in front
# of the others, or the inputs given under another path, leave the output as it was; and the
# incremental writes: a rebuild of an unchanged program writes nothing, an edit of one file
# rewrites its module alone, and modules of an earlier build that are gone are removed; that a
# macro program built with the interpreter's natives and without them (TEQ_NO_NATIVES=1) agrees;
# and no module of those builds declares a name twice; and that the core corpus (bench/gen.py),
# which is large enough for the parser's, the emitter's and the writer's threads, gives the same
# bytes with one worker (`TEQ_WORKERS=1`) as with the machine's, split and as one file. A
# program's `// jars:` line puts those jars on its class path (tests/support/jars.sh); without
# one of them in the coursier cache it counts as passed.
cd "$(dirname "$0")/.."
TEQ=${TEQ:-./target/release/teq}
. tests/support/jars.sh
STUB=tests/interop/scalajs-stub
mkdir -p out/split
built=()
pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() {
  echo "FAIL $1"
  fail=$((fail + 1))
}
check() {
  if diff -q "$1" "$2" > /dev/null; then ok; else
    bad "$3"
    diff "$1" "$2" | head -${DIFF_LINES:-10}
  fi
}
expect() {
  if [ "$2" = "$3" ]; then ok; else bad "$1: expected $3, found $2"; fi
}

for src in tests/cases/*.scala tests/cases/*/ tests/interop/*.scala tests/interop/*/ tests/split/*/; do
  src=${src%/}
  name=$(basename "$src" .scala)
  suite=$(dirname "$src")
  expected="$suite/$name.expected"
  [ "$src" = "$STUB" ] && continue
  [ -f "$expected" ] || continue
  # A program of the JVM's platform (files, the process) has no JavaScript run (tests/run.sh).
  grep -q -h '^//> using platform jvm' "$src" "$src"/*.scala 2> /dev/null && continue
  # The interop programs are written against the stub; a case takes the std's Scala.js layer.
  extra=""
  if [ "$suite" = tests/interop ] && grep -rq 'scala\.scalajs' "$src" && ! grep -rq '^package scala\.scalajs' "$src"; then
    extra=$STUB
  fi
  flags=$(grep -h -o '^// teq: .*' "$src" "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  jars_of "$src"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $name: not in the coursier cache:$JARS_MISSING"
    ok
    continue
  fi
  [ -n "$JARS_CP" ] && flags="$flags --classpath $JARS_CP"
  dir="out/split/$name"
  actual="$dir.actual"
  harness="$suite/$name.harness.mjs"
  if [ -f "$harness" ]; then
    sed "s|\"../../out/interop/$name.mjs\"|\"./$name/main.mjs\"|g" "$harness" > "$dir.harness.mjs"
    timeout 20 "$TEQ" compiler build "$src" $extra $flags --split "$dir" > "$actual" 2>&1 && timeout 20 node "$dir.harness.mjs" >> "$actual" 2>&1
  else
    # The build and node on its entry under one bound, as the raw `teq run` that did both was.
    timeout 20 bash -c '"$0" compiler build "$@" && exec node "${@: -1}/main.mjs"' "$TEQ" "$src" $extra $flags --split "$dir" > "$actual" 2>&1
  fi
  check "$expected" "$actual" "$name"
  built+=("$dir")
done

# Under --module-per-file and --hot a module that is not per file takes what it refers to in a
# per-file module through the runtime, in the place of imports: every program of tests/split runs
# as before with each of its packages per file in turn.
for src in tests/split/*/; do
  src=${src%/}
  name=$(basename "$src")
  expected="tests/split/$name.expected"
  [ -f "$expected" ] || continue
  flags=$(grep -h -o '^// teq: .*' "$src"/*.scala 2>/dev/null | head -1 | sed 's|^// teq: ||')
  for module in out/split/$name/*.mjs; do
    package=$(basename "$module" .mjs)
    case $package in main | rt | std | 'main$' | _root_) continue ;; esac
    dir="out/split/$name.hot"
    rm -rf "$dir"
    timeout 20 "$TEQ" compiler build "$src" $flags --split "$dir" --module-per-file "$package" --hot > "$dir.actual" 2>&1 && timeout 20 node "$dir/main.mjs" >> "$dir.actual" 2>&1
    check "$expected" "$dir.actual" "$name with $package per file under --hot"
  done
done

# Every module parses: a name declared twice, among the early errors, fails a module as a whole.
if dups=$(timeout 60 node --no-warnings --experimental-vm-modules tests/support/parse-modules.mjs "${built[@]}"); then ok; else
  bad "modules that do not parse"
  echo "$dups" | head -${DIFF_LINES:-10}
fi

# Hello world: the entry, the runtime, the standard library and the program's package.
dir=out/split/hello
rm -rf $dir
timeout 20 "$TEQ" compiler build tests/dce/hello.scala --split $dir > $dir.log 2>&1 || bad "hello build"
expect "modules of hello" "$(ls $dir | sort | tr '\n' ' ')" "_root_.mjs main.mjs rt.mjs std.mjs "
size=$(cat $dir/*.mjs | wc -c)
if [ "$size" -lt 6000 ]; then ok; else bad "hello is $size bytes"; fi

# --module-per-file: a prefix names a package or its subpackages, not a package that merely starts with
# it, and their files become modules of their own; --hot guards $init and $enums and adds $hot.
dir=out/split/small-flags
rm -rf $dir
timeout 20 "$TEQ" compiler build tests/split/cycle --split $dir --module-per-file a > $dir.log 2>&1 || bad "small build"
expect "modules under --module-per-file a" "$(ls $dir | sort | tr '\n' ' ')" "a.a.mjs app.mjs b.mjs main.mjs rt.mjs std.mjs "
if grep -q '\$hot' $dir/a.a.mjs; then bad "\$hot without --hot"; else ok; fi
rm -rf $dir
timeout 20 "$TEQ" compiler build tests/split/cycle --split $dir --module-per-file a,b --hot > $dir.log 2>&1 || bad "hot build"
expect "modules under --module-per-file a,b" "$(ls $dir | sort | tr '\n' ' ')" "a.a.mjs app.mjs b.b.mjs hot-build.mjs hot-refresh.mjs main.mjs rt.mjs std.mjs "
expect "\$hot of a.a.mjs calls its object accessors" "$(grep -o 'function \$hot() {.*}' $dir/a.a.mjs)" 'function $hot() { AConst$(); }'
expect "the accessor of AConst reports the object" "$(grep -o 'function AConst\$() {.*}' $dir/a.a.mjs)" 'function AConst$() { return AConst$i ?? $hotObj(new AConst$M(), "a.AConst$"); }'
# What --hot writes for a development server. Every module ends with a footer that tells the
# runtime that it ran, with its address and the hash of its text: a --module-per-file module
# gives its $hot(), which a swap's main.mjs runs; a module that must not run again (app.mjs,
# std.mjs, rt.mjs, hot-refresh.mjs) accepts an update of its own and gives none; main.mjs tells
# that the page has booted or a swap has run. A module that is not split per file imports no
# per-file module and takes what it refers to in one through the runtime. No module accepts an
# update of another. hot-build.mjs names the build by an id, the hash of its modules' hashes,
# and lists the hash of every module, its own and the stub's excepted, and the id of the build
# that wrote it; every hash and id is the one the module's footer has.
hash='[0-9a-f]\{16\}'
footer() { tail -1 "$1" | sed "s/\"$hash\"/HASH/; s/\"$hash\"/ID/"; }
swapped='if (import.meta.hot) $hotRan(import.meta.url, HASH, ID, $hot);'
held='if (import.meta.hot) { import.meta.hot.accept($hotFailing(import.meta.url)); $hotRan(import.meta.url, HASH, ID); }'
expect "main.mjs tells that the page has booted" "$(footer $dir/main.mjs)" 'if (import.meta.hot) $hotBooted(import.meta.url, HASH, ID);'
expect "a.a.mjs gives its \$hot" "$(footer $dir/a.a.mjs)" "$swapped"
expect "a.a.mjs gives what app.mjs takes, again once its enum values exist" "$(grep -F '$hotProvide(' $dir/a.a.mjs)" '$hotProvide("a.a");
$hotProvide("a.a", () => ({ $file17k0ehk, Color$Green, Color$Red, Widget, aTop$, describe }));'
expect "app.mjs imports no per-file module" "$(grep -c '^import .*"\./[ab]\.[ab]\.mjs"' $dir/app.mjs)" "0"
expect "app.mjs takes a.a's definitions through the runtime" "$(grep -B1 -F '$hotUse("a.a"' $dir/app.mjs)" 'let $file17k0ehk, Color$Green, Color$Red, Widget, aTop$, describe;
$hotUse("a.a", ($m) => { $file17k0ehk = $m.$file17k0ehk; Color$Green = $m.Color$Green; Color$Red = $m.Color$Red; Widget = $m.Widget; aTop$ = $m.aTop$; describe = $m.describe; });'
for module in app std rt hot-refresh; do
  expect "$module.mjs must not run again" "$(footer $dir/$module.mjs)" "$held"
done
expect "no module accepts an update of another" "$(grep -l 'import\.meta\.hot\.accept(\[\|import\.meta\.hot\.accept("' $dir/*.mjs | wc -l | tr -d ' ')" "0"
expect "main.mjs imports the listing" "$(grep -c '^import "./hot-build.mjs";$' $dir/main.mjs)" "1"
listed=$(sed -n 's/^    "\(.*\)": \["\(.*\)", "\(.*\)"\],$/\1 \2 \3/p' $dir/hot-build.mjs)
expect "hot-build.mjs lists every module but itself" "$(echo "$listed" | cut -d' ' -f1 | tr '\n' ' ')" "a.a app b.b hot-refresh main rt std "
expect "under the hash and the writer each module's footer has" "$(echo "$listed" | while read -r name h w; do tail -1 $dir/$name.mjs | grep -q "\"$h\", \"$w\"" || echo "$name"; done)" ""
build_id=$(sed -n 's/^  \$hotBuild(import.meta.url, "\([0-9a-f]*\)", {$/\1/p' $dir/hot-build.mjs)
expect "hot-build.mjs names its build by an id" "$(echo -n "$build_id" | wc -c | tr -d ' ')" "16"
expect "and a build into an empty directory wrote every module: every writer is its id" "$(echo "$listed" | while read -r name h w; do [ "$w" = "$build_id" ] || echo "$name"; done)" ""
expect "hot-refresh.mjs is teq's own runtime between an import and the footer" "$(sed '1d;$d' $dir/hot-refresh.mjs | sed '$d' | diff - runtime/hot-refresh.mjs > /dev/null && sed -n 1p $dir/hot-refresh.mjs)" 'import { $hotRan, $hotFailing } from "./rt.mjs";'
expect "the runtime does nothing under node" "$(cd $dir && timeout 20 node --input-type=module -e 'await import("./hot-refresh.mjs"); console.log(typeof globalThis.$teqHotObject)' 2>&1)" "undefined"
# The hash names the text: a build of the same sources into another directory writes the same
# footers and the same listing. An edit of one file changes that file's hash alone, and the id
# of the build: a build into an empty directory writes every module with its own id, where a
# build over an earlier one rewrites the edited module alone (below, and a session in
# tests/split-watch.sh), so two builds into empty directories differ in the ids of their
# footers and in nothing else beside the edit.
rm -rf $dir.again $dir.edited $dir.src
timeout 20 "$TEQ" compiler build tests/split/cycle --split $dir.again --module-per-file a,b --hot > $dir.log 2>&1 || bad "hot build again"
if diff -r $dir $dir.again > /dev/null; then ok; else bad "two --hot builds of the same sources differ"; fi
cp -r tests/split/cycle $dir.src
sed -i.bak 's/"tagged"/"marked"/' $dir.src/b.scala && rm $dir.src/b.scala.bak
timeout 20 "$TEQ" compiler build $dir.src --split $dir.edited --module-per-file a,b --hot > $dir.log 2>&1 || bad "hot build of an edit"
without_ids() { for f in "$1"/*.mjs; do sed -E -i.bak 's/(\$hot(Ran|Booted)\(import\.meta\.url, "[0-9a-f]{16}", )"[0-9a-f]{16}"/\1"ID"/' "$f" && rm "$f.bak"; done; }
rm -rf $dir.ids; cp -r $dir $dir.ids; cp -r $dir.edited $dir.edited.ids; without_ids $dir.ids; without_ids $dir.edited.ids
expect "an edit changes the text and the listing, and no other file but for the ids" "$(diff -rq $dir.ids $dir.edited.ids | sed 's/.*\/\([^ /]*\) differ/\1/' | tr '\n' ' ')" "b.b.mjs hot-build.mjs "
expect "and in the listing the hash of that file alone, the ids apart" "$(diff <(sed -E 's/"[0-9a-f]{16}"(\]|, \{)/"ID"\1/' $dir/hot-build.mjs) <(sed -E 's/"[0-9a-f]{16}"(\]|, \{)/"ID"\1/' $dir.edited/hot-build.mjs) | grep -c '^[<>]')" "2"
# A build over an earlier build's directory rewrites the edited module alone, and the listing
# names it as the one module the build wrote.
rm -rf $dir.over; cp -r $dir $dir.over
timeout 20 "$TEQ" compiler build $dir.src --split $dir.over --module-per-file a,b --hot > $dir.log 2>&1 || bad "hot build over an earlier build"
expect "a build over an earlier build's directory rewrites the edited module and the listing" "$(diff -rq $dir $dir.over | sed 's/.*\/\([^ /]*\) differ/\1/' | tr '\n' ' ')" "b.b.mjs hot-build.mjs "
expect "and the listing names it as the one module of the build's own writing" "$(sed -n 's/^    "\(.*\)": \["\(.*\)", "\(.*\)"\],$/\1 \3/p' $dir.over/hot-build.mjs | while read -r name w; do [ "$w" = "$build_id" ] || echo "$name"; done | tr '\n' ' ')" "b.b "
rm -rf $dir.over
expect "the id is the hash of the hashes: the same for the same listing, another for another" "$(sed -n 's/^  \$hotBuild(import.meta.url, "\([0-9a-f]*\)", {$/\1/p' $dir.again/hot-build.mjs) $([ "$(sed -n 's/^  \$hotBuild(import.meta.url, "\([0-9a-f]*\)", {$/\1/p' $dir.edited/hot-build.mjs)" != "$build_id" ] && echo other)" "$build_id other"
rm -rf $dir.ids $dir.edited.ids

# A page of --hot modules as a development server runs it (tests/support/hot-page.mjs), over
# tests/split/publish: `parts` per file, `logic` and `text` shared, the edits of each scenario
# built into a directory of their own and published into the page's file by file.
# Every build of an edit is made over a copy of the build it follows, as a session's is: the
# modules the edit did not change keep the writer they have, which the listing names.
page=out/split/page
rm -rf out/split/publish.*
edit() {
  local name=$1 over=$2
  shift 2
  rm -rf out/split/publish.$name.src out/split/publish.$name
  cp -r tests/split/publish out/split/publish.$name.src
  [ "$over" = - ] || cp -r out/split/publish.$over out/split/publish.$name
  (cd out/split/publish.$name.src && "$@")
  timeout 20 "$TEQ" compiler build out/split/publish.$name.src --split out/split/publish.$name --module-per-file parts --hot > out/split/publish.$name.log 2>&1 || bad "publish build $name: $(head -3 out/split/publish.$name.log)"
}
b=out/split/publish
edit first - true
edit body first sed -i.bak 's/"hello "/"hi "/' greet.scala
edit renamed first sed -i.bak 's/greet/welcome/' greet.scala logic.scala
edit two first sed -i.bak -e 's/"!"/"?"/' -e 's/"\."/";"/' logic.scala text.scala
edit gone first sh -c 'rm extra.scala && sed -i.bak "/parts.extra/d" main.scala'
edit one first sed -i.bak 's/"!"/"?"/' logic.scala
edit other first sed -i.bak 's/"!"/"?!"/' logic.scala
# A class in text.scala, which gives text.mjs an $init that main.mjs then imports: over the
# first build, and after `one`, where logic is restored to the first text as well.
boxed='printf "\nclass Box(val value: String)\n" >> text.scala && sed -i.bak "s/line + mark/Box(line + mark).value/" text.scala'
edit needs first sh -c "$boxed"
edit boxed one sh -c "$boxed"
# A def that throws when called, by JavaScript's own error, so that no build differs from the
# first in the standard library's module.
edit throws first sed -i.bak 's/"hello " + name/(null: String).length.toString/' greet.scala
# An eager val beside the per-file def that the shared module takes through the runtime: the
# file's initialiser runs once per instance of its module, though the shared module calls the
# copy it took before the initialiser rebound itself.
edit inits first sh -c 'printf "\nval greeted: Int =\n  println(\"greet file\")\n  1\n" >> greet.scala'
hot_page() { timeout 20 node tests/support/hot-page.mjs $page "$@" 2>&1 | grep -v '^hello a\|^hi a\|^extra$'; }
# A swap: the per-file module and main.mjs run again, the module's objects are made anew, and
# shared code, which did not run again, calls the new code from then on. Nothing is reloaded, and
# nothing waits, before the listing of the build has arrived and after.
expect "a swap runs the new code in shared code that did not run again" "$(hot_page boot:$b.first call:logic:line:b publish:$b.body:parts.greet swap:parts.greet call:logic:line:b show publish:$b.body:hot-build update:hot-build show)" 'line: hello b!
line: hi b!
reloads=0 nothing pending
reloads=0 nothing pending'
expect "a file's initialiser runs once per instance of its module, its copies taken before included" "$(hot_page boot:$b.inits call:logic:line:b call:logic:line:b swap:parts.greet call:logic:line:b show)" 'greet file
line: hello b!
line: hello b!
greet file
line: hello b!
reloads=0 nothing pending'
dir=out/split/small-flags
expect "a swap constructs the module's objects anew" "$(timeout 20 node tests/support/hot-page.mjs $page boot:$dir made:a.AConst\$ swap:a.a made:a.AConst\$ show 2>&1 | grep ' made \|^reloads')" 'a.AConst$ made 1 times
a.AConst$ made 2 times
reloads=0 nothing pending'
expect "a swap wider than a swap may be is a wish, and constructs nothing" "$(timeout 20 node tests/support/hot-page.mjs $page boot:$dir upto:0 swap:a.a made:a.AConst\$ show 2>&1 | grep ' made \|^reloads')" 'a.AConst$ made 1 times
reloads=1 wish="a swap of 1 modules" waits='
# The page is loaded again when the last listing has, for every module the page ran, the text
# the page ran, whatever the order in which a build's files and their updates arrive. A shared
# module that takes a renamed def, published and run before the per-file module that gives it:
expect "the consumer before the provider" "$(hot_page boot:$b.first publish:$b.renamed:logic update:logic show publish:$b.renamed:parts.greet swap:parts.greet show publish:$b.renamed:hot-build update:hot-build show)" 'reloads=0 wish="logic ran again" waits=logic
failed: greet is not a function
reloads=0 wish="logic ran again" waits=logic,parts.greet failed=main
reloads=1 wish="logic ran again" waits= failed=main'
expect "the listing before the modules it lists" "$(hot_page boot:$b.first publish:$b.renamed:logic publish:$b.renamed:parts.greet publish:$b.renamed:hot-build update:hot-build show update:logic show swap:parts.greet show)" 'reloads=0 nothing pending
reloads=0 wish="logic ran again" waits=parts.greet
failed: greet is not a function
reloads=1 wish="logic ran again" waits= failed=main'
expect "two shared modules of one build, the second not yet there when the first runs" "$(hot_page boot:$b.first publish:$b.two:logic update:logic show publish:$b.two:text update:text show publish:$b.two:hot-build update:hot-build show)" 'reloads=0 wish="logic ran again" waits=logic
reloads=0 wish="logic ran again" waits=logic,text
reloads=1 wish="logic ran again" waits='
expect "a module the page ran is gone from the build, a shared one rewritten" "$(hot_page boot:$b.first publish:$b.gone:app update:app show remove:parts.extra publish:$b.gone:main swap:main show publish:$b.gone:hot-build update:hot-build show)" 'reloads=0 wish="app ran again" waits=app
reloads=0 wish="app ran again" waits=app,main
reloads=1 wish="app ran again" waits='
expect "two builds before the page has taken the first" "$(hot_page boot:$b.first publish:$b.one:logic update:logic show publish:$b.other:logic publish:$b.other:hot-build update:hot-build show update:logic show)" 'reloads=0 wish="logic ran again" waits=logic
reloads=0 wish="logic ran again" waits=logic
reloads=1 wish="logic ran again" waits='
expect "a listing of an update before the last one does not count" "$(hot_page boot:$b.first publish:$b.other:logic publish:$b.other:hot-build update:hot-build@5 publish:$b.one:hot-build update:hot-build@4 publish:$b.other:hot-build update:logic@6 show)" 'reloads=1 wish="logic ran again" waits='
expect "a shared module run again with the text it has is loaded again at once" "$(hot_page boot:$b.first update:text show)" 'reloads=1 wish="text ran again" waits='
# Agreement of the texts is not evidence of the build: a build that restores a text an earlier
# listing has (logic, edited by `one` and restored by `boxed`, which adds a class to text.scala
# and so an import to main.mjs) must not pass for that earlier build while it is half
# published: the restored logic is of another writer than the first listing names.
expect "a restored text agreeing with an old listing does not load the page again" "$(hot_page boot:$b.first publish:$b.one:logic update:logic show publish:$b.boxed:main swap:main show publish:$b.boxed:logic update:logic show publish:$b.boxed:text update:text show publish:$b.boxed:hot-build update:hot-build show)" 'reloads=0 wish="logic ran again" waits=logic
failed: The requested module '"'"'./text.mjs'"'"' does not provide an export named '"'"'$init'"'"'
reloads=0 wish="logic ran again" waits=logic failed=main
reloads=0 wish="logic ran again" waits=logic? failed=main
reloads=0 wish="logic ran again" waits=logic?,text failed=main
reloads=1 wish="logic ran again" waits= failed=main'
# An update of main.mjs that fails, published before the shared module it takes a new export
# of: the page keeps what it has, waits for nothing of main's, and is loaded again once the
# listing of an update after the failed one lists what the rest of the page has run.
expect "main published before the shared module it needs" "$(hot_page boot:$b.first publish:$b.needs:main swap:main show publish:$b.needs:text update:text show publish:$b.needs:hot-build update:hot-build show)" 'failed: The requested module '"'"'./text.mjs'"'"' does not provide an export named '"'"'$init'"'"'
reloads=0 wish="main'"'"'s update failed" waits= failed=main
reloads=0 wish="main'"'"'s update failed" waits=text failed=main
reloads=1 wish="main'"'"'s update failed" waits= failed=main'
# A swap that throws as it runs (a per-file module's new def, called by main): main's update
# failed, the footers of the modules run before it ran; the swap of a text that runs ends it.
expect "a failed update whose module then runs is over: nothing pending" "$(hot_page boot:$b.first publish:$b.throws:parts.greet swap:parts.greet show publish:$b.body:parts.greet swap:parts.greet show publish:$b.body:hot-build update:hot-build show 2>&1 | sed 's/^failed: .*null.*/failed: a null'"'"'s length/')" 'failed: a null'"'"'s length
reloads=0 wish="main'"'"'s update failed" waits=parts.greet failed=main
reloads=0 nothing pending
reloads=0 nothing pending'
expect "a page that loads while a build is published is loaded again when it is whole" "$(hot_page publish:$b.first:main boot:$b.first show 2>&1 | tail -1; rm -rf $page.half; cp -r $b.first $page.half; cp $b.two/logic.mjs $page.half/; hot_page boot:$page.half show publish:$b.two:text update:text show publish:$b.two:hot-build update:hot-build show)" 'reloads=0 nothing pending
reloads=0 wish="the page loaded while a build was published" waits=logic
reloads=0 wish="the page loaded while a build was published" waits=logic,text
reloads=1 wish="the page loaded while a build was published" waits='

# A value the page holds keeps matching in shared code: a per-file module must not run again if
# a module that is not per file holds one of its classes to what it is, as a parent, in a type
# test or a pattern, or by a compared value of its enum. What shared code constructs, calls or
# reads takes what a swap made, and leaves the module to be swapped.
dir=out/split/linked
rm -rf $dir
timeout 20 "$TEQ" compiler build tests/split/inherit --split $dir --module-per-file stock --hot > $dir.log 2>&1 || bad "linked build"
expect "a per-file module whose class another module's class extends must not run again" "$(footer $dir/stock.stock.mjs)" "$held"
dir=out/split/held-uses
rm -rf $dir $dir.src
mkdir -p $dir.src
cp tests/split/held/ui.scala tests/split/held/views.scala $dir.src/
printf 'package app\n\n@main def run(): Unit = println(logic.probe(ui.first, ui.Mark.Red))\n' > $dir.src/main.scala
held_by() {
  printf 'package logic\n\nimport ui.{Mark, Selection}\n\ndef probe(x: Any, mark: Mark): Any = %s\n' "$2" > $dir.src/logic.scala
  timeout 20 "$TEQ" compiler build $dir.src --split $dir --module-per-file ui,views --hot > $dir.log 2>&1 || bad "held build: $1: $(head -3 $dir.log)"
  expect "$1" "$(footer $dir/ui.ui.mjs)" "$3"
}
held_by "a type test in shared code holds the class" 'x.isInstanceOf[Selection]' "$held"
held_by "a pattern in shared code holds the class" 'x match { case Selection(i) => i; case _ => -1 }' "$held"
held_by "an enum value as a pattern in shared code holds the enum" 'mark match { case Mark.Red => 1; case _ => 2 }' "$held"
held_by "an enum value compared in shared code holds the enum" 'mark == Mark.Red' "$held"
held_by "an enum value compared by reference in shared code holds the enum" 'mark eq Mark.Green' "$held"
held_by "an object compared by reference in shared code holds its module" 'x.asInstanceOf[AnyRef] eq ui.Token' "$held"
held_by "an object compared by reference with ne holds its module" 'x.asInstanceOf[AnyRef] ne ui.Token' "$held"
held_by "an object compared with == in shared code holds its module" 'x == ui.Token' "$held"
held_by "an object as a pattern in shared code holds its module" 'x match { case ui.Token => 1; case _ => 2 }' "$held"
held_by "a def called on an object holds nothing" 'ui.Token.label' "$swapped"
held_by "a class constructed, a def called and an enum value passed on hold nothing" '(Selection(2), ui.first, List(Mark.Red, mark))' "$swapped"
# The update of a held module, as a development server makes it: the module accepts it, is run
# again alone, and the page is loaded again once the listing has its text.
dir=out/split/held
rm -rf $dir $dir.edited $dir.src
timeout 20 "$TEQ" compiler build tests/split/held --split $dir --module-per-file ui,views --hot > $dir.log 2>&1 || bad "held build"
expect "a module shared code holds must not run again" "$(footer $dir/ui.ui.mjs)" "$held"
expect "a module shared code calls is swapped" "$(footer $dir/views.views.mjs)" "$swapped"
cp -r tests/split/held $dir.src
sed -i.bak 's/Selection(1)/Selection(2)/' $dir.src/ui.scala && rm $dir.src/ui.scala.bak
cp -r $dir $dir.edited
timeout 20 "$TEQ" compiler build $dir.src --split $dir.edited --module-per-file ui,views --hot > $dir.log 2>&1 || bad "held build of an edit"
expect "the held module run again loads the page again" "$(timeout 20 node tests/support/hot-page.mjs $page boot:$dir publish:$dir.edited:ui.ui update:ui.ui show publish:$dir.edited:hot-build update:hot-build show 2>&1 | tail -2)" 'reloads=0 wish="ui.ui ran again" waits=ui.ui
reloads=1 wish="ui.ui ran again" waits='
dir=out/split/small-flags
# A second $init and $enums, as main.mjs runs them when a page re-executes it, change nothing;
# every object constructed is reported to $teqHotObject, once per construction.
cat > $dir.probe.mjs <<'EOF'
const reported = [];
globalThis.$teqHotObject = (o, id) => reported.push(`${id}:${o.constructor.name}`);
await import("./small-flags/main.mjs");
import { $enums, $init, $hot, Color$Red, AConst$ } from "./small-flags/a.a.mjs";
const red = Color$Red;
$init(); $enums();
console.log(`${red === Color$Red} ${typeof $hot} ${AConst$().value}`);
$hot();
console.log(reported.filter((r) => r.startsWith("a.")).join(" "));
EOF
probe=$(cd out/split && timeout 20 node ./small-flags.probe.mjs 2>&1)
expect "\$enums runs once and \$hot is callable" "$(echo "$probe" | tail -2 | head -1)" "true function A+mode-Fast"
expect "objects reported under --hot" "$(echo "$probe" | tail -1)" "a.AConst\$:AConst\$M"
timeout 20 "$TEQ" compiler build tests/split/cycle --module-per-file a > $dir.log 2>&1
expect "--module-per-file without --split" "$?,$(head -c 6 $dir.log)" "2,usage:"

# Names do not depend on the place of a file in the compilation: a file added in front of the
# others, or the same inputs given under another path, leave every module as it was; the names
# a macro makes (tests/cases/macro_fresh_names) among them.
teq=$(cd "$(dirname "$TEQ")" && pwd)/$(basename "$TEQ")
for case in tests/split/outline tests/split/cycle tests/cases/macro_fresh_names; do
  work=$(mktemp -d)
  cp $case/*.scala "$work"
  timeout 20 "$TEQ" compiler build "$work" --split "$work/before" > /dev/null 2>&1
  touch "$work/0.scala"
  timeout 20 "$TEQ" compiler build "$work" --split "$work/after" > /dev/null 2>&1
  rm "$work/0.scala"
  (cd "$work" && timeout 20 "$teq" compiler build . --split relative > /dev/null 2>&1)
  if diff -r "$work/before" "$work/after" > /dev/null && diff -r "$work/before" "$work/relative" > /dev/null; then ok; else
    bad "$case: output depends on the file list"
    diff -r "$work/before" "$work/after" | head -${DIFF_LINES:-10}
    diff -r "$work/before" "$work/relative" | head -${DIFF_LINES:-10}
  fi
  rm -rf "$work"
done

# A file's identity in a name (`program_keys`) is the shortest suffix of its path that no other
# file of the build shares: `b/Use.scala` of tests/split/quote_keys goes by `Use.scala` alone and
# by `b/Use.scala` beside `a/Use.scala`, whichever order the files are given in; its quote copy
# is named by that identity.
work=$(mktemp -d)
q=tests/split/quote_keys
timeout 20 "$TEQ" compiler build $q/Macros.scala $q/b/Use.scala --no-outline --split "$work/alone" > /dev/null 2>&1
timeout 20 "$TEQ" compiler build $q/Macros.scala $q/a/Use.scala $q/b/Use.scala --no-outline --split "$work/beside" > /dev/null 2>&1
timeout 20 "$TEQ" compiler build $q/b/Use.scala $q/Macros.scala $q/a/Use.scala --no-outline --split "$work/other-order" > /dev/null 2>&1
# The copies' names, and the site's part of each: the tag of the site's file and its offset.
copies() { grep -oh 'class Macros[A-Za-z0-9_$]*anon[A-Za-z0-9_$]*' "$1"/*.mjs 2> /dev/null | sort | tr '\n' ' '; }
sites() { copies "$1" | tr ' ' '\n' | grep '\$' | sed 's/.*\$//' | tr '\n' ' '; }
expect "quote_keys: a file given alone goes by its name" "$(sites "$work/alone")" '52k7xc_40 '
expect "quote_keys: beside a file of its name it goes by one component more" "$(sites "$work/beside")" '1rc96ct_40 '
expect "quote_keys: the identities do not depend on the order of the inputs" "$(copies "$work/other-order")" "$(copies "$work/beside")"
rm -rf "$work"

# A file is one input however often the inputs reach it: given with the directory that holds it,
# given twice, through a link to the directory, or under another spelling of its case where the
# file system ignores case. Each build has to equal the directory's own.
work=$(mktemp -d)
q=tests/split/quote_keys
timeout 20 "$TEQ" compiler build $q --no-outline --split "$work/once" > /dev/null 2>&1 || bad "quote_keys: the build of the directory"
ln -s "$(cd $q && pwd)" "$work/link"
same_files() {
  local what=$1
  shift
  if timeout 20 "$TEQ" compiler build "$@" --no-outline --split "$work/again" > "$work/again.log" 2>&1 && diff -r "$work/once" "$work/again" > /dev/null; then ok; else
    bad "quote_keys: $what: not the directory's build ($(grep -m1 error "$work/again.log" | cut -c1-120))"
  fi
  rm -rf "$work/again"
}
same_files "a file given beside the directory that holds it" $q $q/b/Use.scala
same_files "a file given twice" $q/Macros.scala $q/a/Use.scala $q/b/Use.scala $q/b/Use.scala
same_files "a file reached through a link too" $q "$work/link/b/Use.scala"
# Under a mount point, whose root may share its inode with another entry of the parent's listing
# (Linux's /tmp beside /proc).
cp -R $q "$work/copy"
same_files "files given by their paths in the temporary directory" "$work/copy/Macros.scala" "$work/copy/a/Use.scala" "$work/copy/b/Use.scala"
if [ -e $q/B/USE.SCALA ]; then
  same_files "a file given under another spelling of its case" $q $q/B/USE.SCALA
fi
rm -rf "$work"

# What an export clause resolves to does not depend on the order of the inputs: the programs
# below (a jar trait's members through an object that imports itself, a facade of inherited,
# renamed, excluded and given members and of a package object's parent, an import over a
# hierarchy that passes through the class under completion, and the exclusions and the cycles
# among the error programs, one of them reported at whichever object's table is demanded first
# unless the report picks the cycle's first member) give the same diagnostics for every order
# of their files and with two roots holding them in either order, the same output run in the
# reverse order, and the same modules with the signature phase turned around
# (TEQ_DEMAND_ORDER=reversed: the completions and the scope tables demanded last to first).
for prog in tests/cases/exports_order_jar tests/cases/exports_order_facade tests/cases/exports_order_import_cycle tests/errors/exports_order_excluded tests/errors/exports_order_cycle tests/errors/exports_cycle_alone.scala tests/errors/exports_cycle_split; do
  name=$(basename "$prog" .scala)
  jars_of "$prog"
  if [ -n "$JARS_MISSING" ]; then
    echo "skip $name: not in the coursier cache:$JARS_MISSING"
    ok
    continue
  fi
  cp=""
  [ -n "$JARS_CP" ] && cp="--classpath $JARS_CP"
  work=$(mktemp -d)
  if [ -d "$prog" ]; then files=("$prog"/*.scala); else files=("$prog"); fi
  diagnostics() { timeout 20 "$TEQ" compiler check "$@" $cp 2>&1 | grep -v ' errors\? found$' | sort; }
  diagnostics "${files[@]}" > "$work/first"
  same=1
  while read -r order; do
    diagnostics $order > "$work/other"
    if ! diff -q "$work/first" "$work/other" > /dev/null; then
      same=0
      bad "$name: the files in the order $(for f in $order; do basename "$f"; done | tr '\n' ' ')give other diagnostics"
      diff "$work/first" "$work/other" | head -${DIFF_LINES:-10}
      break
    fi
  done < <(python3 -c 'import sys, itertools; [print(" ".join(p)) for p in itertools.permutations(sys.argv[1:])]' "${files[@]}")
  [ $same = 1 ] && ok
  if [ ${#files[@]} -gt 1 ]; then
    mkdir -p "$work/a" "$work/b"
    cp "${files[0]}" "$work/a"
    cp "${files[@]:1}" "$work/b"
    diagnostics "$work/a" "$work/b" > "$work/ab"
    diagnostics "$work/b" "$work/a" > "$work/ba"
    check "$work/ab" "$work/ba" "$name: two roots in the other order give other diagnostics"
  fi
  TEQ_DEMAND_ORDER=reversed diagnostics "${files[@]}" > "$work/turned"
  check "$work/first" "$work/turned" "$name: the signature phase turned around gives other diagnostics"
  if [ -f "tests/cases/$name.expected" ]; then
    reversed=$(python3 -c 'import sys; print(" ".join(reversed(sys.argv[1:])))' "${files[@]}")
    run_js 20 "$work/reversed.js" $reversed $cp > "$work/reversed.out" 2>&1
    check "tests/cases/$name.expected" "$work/reversed.out" "$name run in the reverse order"
    timeout 20 "$TEQ" compiler build "$prog" $cp --split "$work/walk" > /dev/null 2>&1
    TEQ_DEMAND_ORDER=reversed timeout 20 "$TEQ" compiler build "$prog" $cp --split "$work/turned-modules" > /dev/null 2>&1
    if diff -r "$work/walk" "$work/turned-modules" > /dev/null; then ok; else
      bad "$name: the signature phase turned around gives other modules"
      diff -r "$work/walk" "$work/turned-modules" | head -${DIFF_LINES:-10}
    fi
  fi
  rm -rf "$work"
done

# A build's bytes do not depend on the order of the inputs: a program's files given as files in
# another order, and the directory that holds them, give the same modules.
work=$(mktemp -d)
timeout 20 "$TEQ" compiler build tests/split/cycle --split "$work/directory" > /dev/null 2>&1
timeout 20 "$TEQ" compiler build $(ls tests/split/cycle/*.scala | sort -r) --split "$work/reversed" > /dev/null 2>&1
q=tests/split/quote_keys
timeout 20 "$TEQ" compiler build $q/Macros.scala $q/a/Use.scala $q/b/Use.scala --no-outline --split "$work/given" > /dev/null 2>&1
timeout 20 "$TEQ" compiler build $q/b/Use.scala $q/Macros.scala $q/a/Use.scala --no-outline --split "$work/permuted" > /dev/null 2>&1
if diff -r "$work/directory" "$work/reversed" > /dev/null; then ok; else
  bad "cycle: the files in another order give other bytes"
  diff -r "$work/directory" "$work/reversed" | head -${DIFF_LINES:-10}
fi
if diff -r "$work/given" "$work/permuted" > /dev/null; then ok; else
  bad "quote_keys: the files in another order give other bytes"
  diff -r "$work/given" "$work/permuted" | head -${DIFF_LINES:-10}
fi
rm -rf "$work"

# The natives that run in place of the std's bodies in a macro (src/interp/natives.rs) change
# nothing the macro emits: the program built with them and without them agrees byte for byte.
work=$(mktemp -d)
timeout 20 "$TEQ" compiler build tests/cases/macro_fresh_names --split "$work/natives" > /dev/null 2>&1
TEQ_NO_NATIVES=1 timeout 20 "$TEQ" compiler build tests/cases/macro_fresh_names --split "$work/bodies" > /dev/null 2>&1
if [ -d "$work/natives" ] && diff -r "$work/natives" "$work/bodies" > /dev/null; then ok; else
  bad "macro_fresh_names: the output depends on the natives"
  diff -r "$work/natives" "$work/bodies" | head -${DIFF_LINES:-10}
fi
rm -rf "$work"

# Under --hot a re-executed module registers its reflectively instantiatable classes anew: the
# registry then holds the class of the new module instance.
dir=out/split/reflect-hot
rm -rf $dir
timeout 20 "$TEQ" compiler build tests/split/reflect --split $dir --module-per-file plugins --hot > $dir.log 2>&1 || bad "reflect hot build"
# The module built again without Solo's annotation, run in the place of the first: its
# registrations replace the module's old ones, and Solo is no longer found.
edited=$(mktemp -d)
cp tests/split/reflect/*.scala "$edited"
perl -0pi -e 's/\@EnableReflectiveInstantiation\nclass Solo/class Solo/; s/class Kid extends Kin/class Kid/' "$edited/plugins.scala"
timeout 20 "$TEQ" compiler build "$edited" --split "$edited/out" --module-per-file plugins --hot > $dir.edited.log 2>&1 || bad "reflect hot edited build"
cp "$edited/out/plugins.plugins.mjs" $dir/plugins.plugins.edited.mjs
rm -rf "$edited"
cat > $dir.probe.mjs <<'EOF'
await import("./reflect-hot/main.mjs");
const { $reflectedClasses, $reflectKeep } = await import("./reflect-hot/rt.mjs");
const before = $reflectedClasses.get("plugins.Greeter");
const again = await import("./reflect-hot/plugins.plugins.mjs?again");
again.$init();
const after = $reflectedClasses.get("plugins.Greeter");
console.log(`${before !== after} ${before.cls !== after.cls} ${after.cls.$qname} ${$reflectedClasses.has("plugins.Solo")}`);
const kid = () => $reflectedClasses.get("plugins.Pair").ctors[0][0]()[1];
const kidBefore = kid().$ancestors.join();
const edited = await import("./reflect-hot/plugins.plugins.edited.mjs");
edited.$init();
console.log(`${$reflectedClasses.has("plugins.Solo")} ${$reflectedClasses.get("plugins.Greeter").cls !== after.cls} ${kidBefore}/${kid().$ancestors.join()}`);
$reflectKeep(["std", "app", "zeta"]);
console.log(`${$reflectedClasses.size}`);
EOF
probe=$(cd out/split && timeout 20 node ./reflect-hot.probe.mjs 2>&1)
expect "a re-executed module registers anew" "$(echo "$probe" | tail -3 | head -1)" "true true plugins.Greeter true"
expect "a re-executed module drops what it no longer registers" "$(echo "$probe" | tail -2 | head -1)" "false true plugins.Kin/"
# main.mjs, run again, names the modules of the build: what a module gone from it registered goes.
expect "main.mjs names the modules that register" "$(grep -o '^\$reflectKeep(.*' $dir/main.mjs)" '$reflectKeep(["std", "app", "plugins.plugins", "zeta"]);'
expect "the registrations of a module gone from the build are dropped" "$(echo "$probe" | tail -1)" "0"

# Incremental writes, on a copy of the cycle case: --time reports how many modules were written.
work=$(mktemp -d)
cp tests/split/cycle/*.scala "$work"
dir=$work/out
written() { timeout 20 "$TEQ" compiler build "$work" --split "$dir" --time 2>&1 | grep -o '[0-9]* of [0-9]* modules'; }
expect "first build" "$(written)" "6 of 6 modules"
expect "unchanged rebuild" "$(written)" "0 of 6 modules"
sed -i.bak 's/"tagged"/"marked"/' "$work/b.scala"
# A rewritten module is a new file (written next to the old one and renamed), so its inode changes.
before=$(ls -i "$dir")
expect "one-file edit" "$(written)" "1 of 6 modules"
after=$(ls -i "$dir")
expect "modules rewritten by the edit" "$(diff <(echo "$before") <(echo "$after") | grep '^>' | tr -s ' ' | cut -d' ' -f3 | tr '\n' ' ')" "b.mjs "
expect "the edit runs" "$(timeout 20 node "$dir/main.mjs" | sed -n 3p)" "w/marked/B+green"
touch "$dir/stale.mjs" "$dir/half.mjs.tmp"
# Package b is unreached from this entry point, so its module goes with the stale files.
cat > "$work/main.scala" <<'EOF'
package app
import a.*
@main def run(): Unit = println(Color.Red.greet)
EOF
written > /dev/null
expect "stale modules removed" "$(ls "$dir" | sort | tr '\n' ' ')" "a.mjs app.mjs main.mjs rt.mjs std.mjs "
rm -rf "$work"

# The same under --module-per-file --hot: an edit of one file rewrites the module of that file alone.
work=$(mktemp -d)
cp tests/split/cycle/*.scala "$work"
dir=$work/out
written() { timeout 20 "$TEQ" compiler build "$work" --split "$dir" --module-per-file a,b --hot --time 2>&1 | grep -o '[0-9]* of [0-9]* modules'; }
# The listing of a --hot build, hot-build.mjs, is a file of the output like the modules it lists.
expect "small: first build" "$(written)" "8 of 8 modules"
expect "small: unchanged rebuild" "$(written)" "0 of 8 modules"
sed -i.bak 's/"tagged"/"marked"/' "$work/b.scala"
before=$(ls -i "$dir")
expect "small: one-file edit" "$(written)" "2 of 8 modules"
after=$(ls -i "$dir")
expect "small: module rewritten by the edit, and the listing" "$(diff <(echo "$before") <(echo "$after") | grep '^>' | tr -s ' ' | cut -d' ' -f3 | tr '\n' ' ')" "b.b.mjs hot-build.mjs "
expect "small: the edit runs" "$(timeout 20 node "$dir/main.mjs" | sed -n 3p)" "w/marked/B+green"
rm -rf "$work"

# A jar object's lazy vals are declared in its template's order whatever order the program reads
# them in (tests/support/lazyorder_lib.scala): two programs reading them in other orders print what
# scalac's do and write the library's module byte for byte alike. Without the jar it counts as passed.
lib=$(lazyorder_lib_jar)
if [ -f "$lib" ]; then
  work=$(mktemp -d)
  for order in abc cab; do
    vals=$(echo "$order" | sed 's/./lazyorderlib.Library.& + /g; s/ + $//')
    printf '@main def main(): Unit = println(%s)\n' "$vals" > "$work/$order.scala"
    timeout 20 "$TEQ" compiler build "$work/$order.scala" --classpath "$lib" --split "$work/$order" > "$work/$order.log" 2>&1 || bad "lazy order: the build of $order: $(head -2 "$work/$order.log")"
    expect "lazy order: $order runs as scalac's" "$(timeout 20 node "$work/$order/main.mjs" | tr '\n' ' ')" "$(echo "$order" | sed 's/./& /g')6 "
  done
  check "$work/abc/lazyorderlib.mjs" "$work/cab/lazyorderlib.mjs" "lazy order: the library's module differs with the order the program reads its vals in"
  rm -rf "$work"
else
  ok
fi

# A test that no reached code makes, a cast in a package nothing calls, changes no module: the
# numbers of the tested traits and the marks of the products by rule come from the reached tests
# (`Reach::tested_traits`).
work=$(mktemp -d)
mkdir -p "$work/a" "$work/b"
for v in a b; do printf 'package kept\ncase class P(n: Int)\n@main def run(): Unit = println(P(1))\n' > "$work/$v/Main.scala"; done
printf 'package dead\ndef unused(x: Any): Any = x\n' > "$work/a/Dead.scala"
printf 'package dead\ndef unused(x: Any): Any = x.asInstanceOf[Product]\n' > "$work/b/Dead.scala"
for v in a b; do timeout 20 "$TEQ" compiler build "$work/$v" --split "$work/$v.out" > "$work/$v.log" 2>&1 || bad "unreached cast: the build of $v: $(head -2 "$work/$v.log")"; done
expect "an unreached cast in another package changes no module" "$(diff -rq "$work/a.out" "$work/b.out" 2>&1 | sed 's/.*\/\([^ /]*\) differ/\1/' | tr '\n' ' ')" ""
rm -rf "$work"

# The same bytes whatever the number of workers.
work=$(mktemp -d)
timeout 60 python3 bench/gen.py "$work/src" 51 22 > /dev/null || bad "workers: the core corpus"
timeout 60 "$TEQ" compiler build "$work/src" --split "$work/all" > "$work/all.log" 2>&1 || bad "workers: the split build: $(head -2 "$work/all.log")"
TEQ_WORKERS=1 timeout 120 "$TEQ" compiler build "$work/src" --split "$work/one" > "$work/one.log" 2>&1 || bad "workers: the split build with one worker: $(head -2 "$work/one.log")"
if diff -rq "$work/all" "$work/one" > "$work/split.diff" 2>&1; then ok; else bad "workers: the split build differs with one worker: $(head -3 "$work/split.diff" | tr '\n' ' ')"; fi
timeout 60 "$TEQ" compiler build "$work/src" -o "$work/all.js" > "$work/all.log" 2>&1 || bad "workers: the build: $(head -2 "$work/all.log")"
TEQ_WORKERS=1 timeout 120 "$TEQ" compiler build "$work/src" -o "$work/one.js" > "$work/one.log" 2>&1 || bad "workers: the build with one worker: $(head -2 "$work/one.log")"
check "$work/all.js" "$work/one.js" "workers: the build differs with one worker"
rm -rf "$work"

echo "$pass passed, $fail failed"
[ $fail = 0 ]
