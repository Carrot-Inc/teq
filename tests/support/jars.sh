# The jars a test names on its `// jars:` line, sourced by the runners from the repository root:
# scala-library, scalajs-library (1.22.0, what the std's `scala.scalajs` stands for), cats-kernel, cats-core, cats-free, cats-kernel-sjs, cats-core-sjs (the application's
# Scala.js builds), circe-numbers, circe-core, circe-generic (circe 0.14.16's `_3` jars), alleycats-core, kittens, shapeless3-deriving, zio-json, magnolia, zio, sourcecode,
# scala-java-time, scala-java-time-tzdb, monocle-core, monocle-macro, refined, atto-core,
# case-insensitive, sourcecode-0.4.4 (the application's version), magnolia123 (Magnolia 1.3.23,
# tapir's), tapir-core, tapir-json-zio, tapir-client, tapir-sttp-client4, tapir-refined,
# tapir-cats, sttp-client4-core, sttp-client4-zio, sttp-model, sttp-shared-core, sttp-shared-ws,
# sttp-shared-zio, zio-streams, zio-test and zio-test-sbt, chimney 1.11.0 with chimney-macro-commons 2.2.0 and
# scala-collection-compat 2.14.0 (its dependencies), scalajs-dom 2.8.1, and the ZIO runtime's
# set (izumi-reflect with its boopickle jar, zio-stacktracer, zio-internal-macros, zio-managed,
# zio-interop-cats, zio-interop-tracer, cats-effect-kernel, cats-effect-std, cats-effect, cats-mtl, fs2-core,
# macrotask-executor), scalajs-react 4.0.0's thirteen jars (sjr-<artifact>) with microlibs'
# compile-time and types 4.2.1 from the coursier cache; scalatest 3.2.20's core, matchers-core,
# shouldmatchers, mustmatchers and compatible jars with scalactic (scalatest-<artifact>, scalactic); junit 4.13.2; the JVM `_3` artifacts of the ZIO runtime's
# set (zio-jvm, zio-stacktracer-jvm, zio-internal-macros-jvm, izumi-reflect-jvm,
# izumi-reflect-boopickle-jvm, scala-collection-compat-jvm, zio-test-jvm), chimney-jvm and chimney-macro-commons-jvm; abi-callbacks-lib,
# tests/support/abi_callbacks_lib.scala compiled by scalac (scala-cli) into a jar; typetest-lib,
# tests/support/typetest_lib.scala compiled the same way; predef-lib,
# tests/support/predef_lib.scala compiled the same way; depfun-lib, tests/support/depfun_lib.scala
# compiled the same way; view-lib, tests/support/view_lib.scala compiled the same way; bundle-lib,
# tests/support/bundle's properties file as a jar (a resource bundle); reflect-lib,
# tests/support/reflect_lib.scala compiled the same way for Scala.js; erasure-lib,
# tests/support/erasure_lib.scala compiled the same way for the JVM; vctraits-lib,
# tests/support/vctraits_lib.scala compiled the same way for the JVM; traitfields-lib,
# tests/support/traitfields_lib.scala compiled the same way for the JVM; deferred-lib,
# tests/support/deferred_lib.scala compiled the same way for the JVM; outer-lib,
# tests/support/outer_lib.scala compiled the same way; defaults-lib, tests/support/defaults_lib.scala
# compiled the same way; lazyorder-lib, tests/support/lazyorder_lib.scala compiled the same way; state-lib,
# tests/support/state_lib.scala compiled the same way under a directory named like scala-library's; scala2-lib,
# tests/support/scala2_lib compiled by Scala 2.13;
# fixtures, the TASTy files of tests/tasty as a jar but those of fix.rdecl and fix.rjava, and fixtures-jvm the
# class files of their sources for the JVM (`fixtures_jvm_jar`); fixtures-rdecl, those
# (tests/tasty/src/reader_decl.scala and reader_java.scala), which the bodies of fix.reader reach in another jar; fixtures-shadow, those of
# tests/tasty/fixtures-shadow, which define again what fixtures defines in fix.shadow; javafix, the Java class files of
# tests/classfile/fixtures as a jar, and javafixdir, the same as a directory; tasty-inspector, scala3-tasty-inspector
# 3.8.4 from the coursier cache.
M2=${COURSIER_CACHE:-$HOME/Library/Caches/Coursier/v1}/https/repo1.maven.org/maven2
# The jars built here are named for the checkout, so that the suites of two worktrees running at
# once do not rewrite each other's.
scratch=${TMPDIR:-/tmp}/teq-$(printf %s "$PWD" | cksum | cut -d' ' -f1)
# Each jar is written beside its path and renamed into it in one step, and each build runs in a
# directory of its own process, so that a suite of a parallel job reads the whole jar or the
# earlier one, never a jar being written.
# The fixtures of tests/tasty as a jar, in the packages their sources declare.
fixtures_jar() {
  local jar=$scratch-fixtures.jar
  python3 - "$jar.$$" <<'PY' && mv -f "$jar.$$" "$jar"
import os, sys, zipfile
members = {'Container', 'Holder', 'Registry', 'UsesRegistry'}
bodies = {'Bodies', 'Base', 'Point', 'Pretty'}
cake = {'Defs', 'DefsPlatform', 'Cake', 'Outer', 'Module', 'ModuleImpl', 'ModulePlatform', 'ModulePlatformImpl', 'SelfDefs', 'SelfUses', 'SelfOther', 'SelfImpl', 'PromiseDefs', 'PromiseDefsPlatform', 'Promises', 'PromisesPlatform', 'PromisesImpl',
        'ExTypes', 'ExExs', 'ExUses', 'ExImpl', 'PatFlags', 'NameDefs', 'NameDefsPlatform', 'PatImpl', 'SupP', 'CacheTypes', 'CacheTypesPlatform', 'CacheUses', 'SupImpl',
        'FoTypes', 'FoExs', 'FoDefs', 'FoDeriv', 'FoMod', 'FoOuters', 'FoMod2', 'FoImpl',
        'FoHier', 'FoHierUse', 'FoPaths', 'FoInto',
        'FoCap', 'FoEff', 'FoReq', 'FoBackend', 'FoPlain', 'FoCapBackend', 'FoAbstract', 'FoConcrete',
        'FoGR', 'FoMapped', 'FoBoth', 'FoIgnore', 'FoLeaf', 'FoRunner', 'FoMaps'}
facades = {'JsStack', 'JsMap', 'Date', 'JsMath', 'JsGlobals', 'JsArr', 'JsPath', 'JsPathDefault', 'JsEmitter', 'JsBindings', 'JsOptions', 'JsPoint', 'JsMathOps', 'facades$package'}
runtime = {'Folds', 'Wilds', 'Atomics', 'Refs', 'Inspectors', 'Apps', 'Clocks', 'Cell', 'Metric', 'Promise', 'Ref', 'PatchRef', 'Cause', 'Bag', 'Slot', 'Slots', 'Token', 'Tokens', 'Keeper', 'Effects', 'Concurrents', 'Pairs'}
pext = {'pext_inlined$package': 'fix/pext/inlined', 'pext_dsl$package': 'fix/pext/dsl', 'PonApi': 'fix/ponapi', 'Token$package': 'fix/ponforeign'}
tname = {'Variance', 'Counter'}
shadow = {'Shadowed', 'shadow$package', 'Eff', 'EffApi', 'EffLib', 'EffRunner', 'EffScope', 'EffSlot', 'EffCell', 'EffAliases', 'EffCells', 'EffLens', 'EffState', 'EffLenses', 'EffStates'}
guard = {'Steps', 'guard$package'}
retain = {'Fmt', 'Loud', 'Sizes', 'Sized', 'Grids', 'Grid'}
overloads = {'InfoOps', 'MetaOps', 'Ep', 'Eps', 'OvSized', 'OvBox'}
wave6 = {'W6Ctx', 'W6Use', 'W6Tags', 'W6Box', 'W6Thunk', 'wave6$package'}
clash = {'ClashOverloaded', 'ClashSingle', 'ClashMaker'}
hkb = {'HkStrm', 'HkLow', 'HkTarget', 'hkbounds$package', 'HkCompiler'}
reader = {'RdCalls', 'RdLocal', 'RdEv', 'RdNow', 'RdMemo', 'RdFn', 'RdFM', 'RdEvs', 'RdCursor', 'RdPick', 'RdStart'}
rdecl = {'RdBox', 'RdFoo', 'RdF1', 'RdFooBox', 'RdNamed', 'RdTraces', 'RdShape', 'RdSquare', 'RdTokA', 'RdTokB', 'RdWrap', 'RdTok', 'RdMarker', 'RdApi', 'RdIn', 'RdInBox', 'RdArr', 'RdCustom', 'RdJava', 'RdSub'}
depparams = {'DpCtx', 'DpFn', 'DpTypes', 'DpBase', 'DpDerived', 'DpInherited', 'DpClauses', 'DpModes', 'DpParent', 'DpMembers', 'DpPoly', 'DpApprox'}
depbounds = {'DbCtx', 'DbParent', 'DbBounds', 'DbPolyNames'}
polybounds = {'PbCtx', 'PbParent', 'PbLower'}
unmod = {'UmBodies', 'UmSyntax', 'UmReuse', 'UmDefine', 'UmMethod', 'UmNested', 'UmSetter33', 'UmArg', 'UmStep', 'UmSub', 'UmScope', 'UmAliases', 'UmPt', 'UmEq', 'UmSetterOnly', 'UmSetterUse'}
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    for f in sorted(os.listdir('tests/tasty/fixtures')):
        stem = f[:-len('.tasty')]
        if stem in rdecl:
            continue
        pkg = 'fix/reader' if stem in reader else pext[stem] if stem in pext else 'fix/q38' if stem == 'Quotes38' else 'fix/clash' if stem in clash else 'fix/hkb' if stem in hkb else 'fix/unmod' if stem in unmod else 'fix/depparams' if stem in depparams else 'fix/depbounds' if stem in depbounds else 'fix/polybounds' if stem in polybounds else 'fix/wave6' if stem in wave6 else 'fix/retain' if stem in retain else 'fix/shadow' if stem in shadow else 'fix/guard' if stem in guard else 'fix/overloads' if stem in overloads else 'fix/tname' if stem in tname else 'fix/facades' if stem in facades else 'fix/members' if stem in members else 'fix/runtime' if stem in runtime else 'fix/rassoc' if stem in ('RightAssoc', 'Vec') else 'fix/bodies' if stem in bodies else 'fix/cake' if stem in cake else 'fix/qpat' if stem in ('LayerBox', 'LayerMacros', 'QpFns', 'QpProbe') else 'fix/qp38' if stem == 'Qp38' else 'fix/shapes'
        z.write(os.path.join('tests/tasty/fixtures', f), pkg + '/' + f)
PY
  echo "$jar"
}
# The declarations of tests/tasty/src/reader_decl.scala and reader_java.scala, which the fixtures
# jar leaves out: the bodies of fix.reader reach them in a jar of their own.
fixtures_rdecl_jar() {
  local jar=$scratch-fixtures-rdecl.jar
  python3 - "$jar.$$" <<'PY' && mv -f "$jar.$$" "$jar"
import os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    for f in sorted(os.listdir('tests/tasty/fixtures')):
        stem = f[:-len('.tasty')]
        if stem in {'RdBox', 'RdFoo', 'RdF1', 'RdFooBox', 'RdNamed', 'RdTraces', 'RdShape', 'RdSquare', 'RdTokA', 'RdTokB', 'RdWrap', 'RdTok', 'RdMarker', 'RdApi', 'RdIn', 'RdInBox', 'RdArr', 'RdCustom'}:
            z.write(os.path.join('tests/tasty/fixtures', f), 'fix/rdecl/' + f)
        elif stem in {'RdJava', 'RdSub'}:
            z.write(os.path.join('tests/tasty/fixtures', f), 'fix/rjava/' + f)
PY
  echo "$jar"
}
# The second copies of tests/tasty/fixtures-shadow, in fix.shadow as in the fixtures jar.
fixtures_shadow_jar() {
  local jar=$scratch-fixtures-shadow.jar
  python3 - "$jar.$$" <<'PY' && mv -f "$jar.$$" "$jar"
import os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    for f in sorted(os.listdir('tests/tasty/fixtures-shadow')):
        z.write(os.path.join('tests/tasty/fixtures-shadow', f), 'fix/shadow/' + f)
PY
  echo "$jar"
}
# The class files of the fixtures of tests/tasty that programs run on the JVM, which runs a jar's
# bytecode where the fixtures jar holds TASTy alone: tests/tasty/src compiled by scalac 3.8.4 as the
# TASTy fixtures were (fix.cake, fix.guard, fix.retain, fix.runtime, fix.tname, fix.wave6, fix.shadow,
# fix.rdecl and fix.rjava; fix.reader against reader_decl_v1.scala and reader_java_v1.scala), with
# tests/classfile/fixtures' Java classes they reach. Built when missing or older than a source.
fixtures_jvm_jar() {
  local jar=$scratch-fixtures-jvm.jar dir=$scratch-fixtures-jvm.$$ src=tests/tasty/src f stale=0
  local main="cake.scala cake3.scala cake4.scala guard.scala retain.scala runtime.scala targetname.scala wave6.scala shadow.scala reader_decl.scala reader_java.scala"
  [ -f "$jar" ] || stale=1
  for f in $main reader.scala reader_decl_v1.scala reader_java_v1.scala; do [ "$src/$f" -nt "$jar" ] && stale=1; done
  if [ $stale = 1 ]; then
    rm -rf "$dir" "$jar" && mkdir -p "$dir/main" "$dir/v1" "$dir/reader" "$dir/out/fix"
    for f in $main; do cp "$src/$f" "$dir/main/"; done
    cp "$src/reader_decl_v1.scala" "$src/reader_java_v1.scala" "$dir/v1/" && cp "$src/reader.scala" "$dir/reader/"
    local javafix=$PWD/tests/classfile/fixtures
    (cd "$dir/main" && timeout 200 scala-cli --power compile -S "${SCALA_VERSION:-3.8.4}" --server=false -q . --classpath "$javafix" -d ../out > ../main.log 2>&1) \
      && (cd "$dir/v1" && timeout 200 scala-cli --power compile -S "${SCALA_VERSION:-3.8.4}" --server=false -q . --classpath "$javafix" -d ../v1-out > ../v1.log 2>&1) \
      && (cd "$dir/reader" && timeout 200 scala-cli --power compile -S "${SCALA_VERSION:-3.8.4}" --server=false -q . --classpath "../v1-out:$javafix" -d ../out > ../reader.log 2>&1) \
      && cp tests/classfile/fixtures/fix/*.class "$dir/out/fix/" \
      && python3 - "$jar.$$" "$dir/out" <<'PY' && mv -f "$jar.$$" "$jar"
import os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    for top, _, files in sorted(os.walk(sys.argv[2])):
        for f in sorted(files):
            if f.endswith('.class'):
                path = os.path.join(top, f)
                z.write(path, os.path.relpath(path, sys.argv[2]))
PY
    rm -rf "$dir"
  fi
  echo "$jar"
}
# The class files of tests/classfile/fixtures (javac's, see tests/classfile/README) as a jar.
javafix_jar() {
  local jar=$scratch-javafix.jar
  python3 - "$jar.$$" <<'PY' && mv -f "$jar.$$" "$jar"
import os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    for f in sorted(os.listdir('tests/classfile/fixtures/fix')):
        z.write(os.path.join('tests/classfile/fixtures/fix', f), 'fix/' + f)
PY
  echo "$jar"
}
# The scalac side of the ABI probes, compiled by scala-cli when missing or older than its source.
# The properties file of tests/support/bundle as a jar, a resource bundle on the class path.
bundle_jar() {
  local jar=$scratch-bundle-lib.jar
  python3 - "$jar.$$" tests/support/bundle <<'PY' && mv -f "$jar.$$" "$jar"
import os, sys, zipfile
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    for name in sorted(os.listdir(sys.argv[2])):
        z.write(os.path.join(sys.argv[2], name), name)
PY
  echo "$jar"
}

abi_callbacks_jar() {
  local src=tests/support/abi_callbacks_lib.scala
  local jar=$scratch-abi-callbacks-lib.jar
  local dir=$scratch-abi-callbacks-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/abi_callbacks_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
typetest_lib_jar() {
  local src=tests/support/typetest_lib.scala
  local jar=$scratch-typetest-lib.jar
  local dir=$scratch-typetest-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/typetest_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/state_lib.scala compiled by scalac (scala-cli) into a jar under a directory and a
# file named like the standard library's, which they are not.
state_lib_jar() {
  local src=tests/support/state_lib.scala
  local dir=$scratch-state-lib
  local jar=$dir/scala-library-tools/scala-library-tools-1.0.jar
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    local work=$dir.$$
    mkdir -p "$dir/scala-library-tools" "$work" && cp "$src" "$work/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$work/state_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$work"
  fi
  echo "$jar"
}
reflect_lib_jar() {
  local src=tests/support/reflect_lib.scala
  local jar=$scratch-reflect-lib.jar
  local dir=$scratch-reflect-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/reflect_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
erasure_lib_jar() {
  local src=tests/support/erasure_lib.scala
  local jar=$scratch-erasure-lib.jar
  local dir=$scratch-erasure-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/erasure_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
vctraits_lib_jar() {
  local src=tests/support/vctraits_lib.scala
  local jar=$scratch-vctraits-lib.jar
  local dir=$scratch-vctraits-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/vctraits_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/traitfields_lib.scala compiled by scalac into a jar: traits with fields of every
# kind, which a class of another build implements.
traitfields_lib_jar() {
  local src=tests/support/traitfields_lib.scala
  local jar=$scratch-traitfields-lib.jar
  local dir=$scratch-traitfields-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/traitfields_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/deferred_lib.scala compiled by scalac into a jar: traits with deferred and abstract
# givens, which a class of another build implements.
deferred_lib_jar() {
  local src=tests/support/deferred_lib.scala
  local jar=$scratch-deferred-lib.jar
  local dir=$scratch-deferred-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/deferred_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/outer_lib.scala compiled by scalac into a jar: a trait nested in a trait, with
# scalac's name for its outer accessor.
outer_lib_jar() {
  local src=tests/support/outer_lib.scala
  local jar=$scratch-outer-lib.jar
  local dir=$scratch-outer-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/outer_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/defaults_lib.scala compiled by scalac into a jar: constructors with defaults, their
# getters scalac's.
defaults_lib_jar() {
  local src=tests/support/defaults_lib.scala
  local jar=$scratch-defaults-lib.jar
  local dir=$scratch-defaults-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/defaults_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/lazyorder_lib.scala compiled by scalac into a jar: an object's lazy vals.
lazyorder_lib_jar() {
  local src=tests/support/lazyorder_lib.scala
  local jar=$scratch-lazyorder-lib.jar
  local dir=$scratch-lazyorder-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/lazyorder_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/depfun_lib.scala compiled by scalac into a jar: dependent function types in TASTy.
depfun_lib_jar() {
  local src=tests/support/depfun_lib.scala
  local jar=$scratch-depfun-lib.jar
  local dir=$scratch-depfun-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/depfun_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/view_lib.scala compiled by scalac into a jar: what the loader's lock holder types while
# a worker holds its own equivalent (the parallel typer's views).
view_lib_jar() {
  local src=tests/support/view_lib.scala
  local jar=$scratch-view-lib.jar
  local dir=$scratch-view-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/view_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/predef_lib.scala compiled by scalac into a jar: bodies through scala-library's Predef.
predef_lib_jar() {
  local src=tests/support/predef_lib.scala
  local jar=$scratch-predef-lib.jar
  local dir=$scratch-predef-lib.$$
  if [ ! -f "$jar" ] || [ "$src" -nt "$jar" ]; then
    mkdir -p "$dir" && cp "$src" "$dir/"
    timeout 200 scala-cli --power package --library -S "${SCALA_VERSION:-3.8.4}" "$dir/predef_lib.scala" --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
    rm -rf "$dir"
  fi
  echo "$jar"
}
# tests/support/scala2_lib/, a library compiled by Scala 2.13 (its class files carry pickles and
# no TASTy), as a jar.
scala2_lib_jar() {
  local src=tests/support/scala2_lib
  local jar=$scratch-scala2-lib.jar
  if [ ! -f "$jar" ] || [ -n "$(find "$src" -newer "$jar" -name '*.scala')" ]; then
    timeout 200 scala-cli --power package --library -S 2.13.16 "$src"/*.scala --server=false -q -o "$jar.$$" -f > /dev/null 2>&1 \
      && mv -f "$jar.$$" "$jar" || rm -f "$jar.$$" "$jar"
  fi
  echo "$jar"
}
# tests/support/places_lib (its README): the library compiled by scalac against decl_v1/, as
# places-lib-1.0.jar with places-lib-1.0-sources.jar beside it (lib/'s files under pl/), the same jar
# alone in another directory (places-lib-nosrc), and decl/ as places-decl-1.0.jar, which the tests
# read the library against.
places_lib_build() {
  local src=tests/support/places_lib
  local dir=$scratch-places
  local stamp=$dir/built
  if [ ! -f "$stamp" ] || [ -n "$(find "$src" -newer "$stamp" -name '*.scala')" ]; then
    local work=$dir.$$
    rm -rf "$work" && mkdir -p "$work/with-sources" "$work/without-sources" "$work/ws"
    local s=${SCALA_VERSION:-3.8.4}
    cp -R "$src" "$work/src"
    (cd "$work/src" \
      && timeout 200 scala-cli --power compile -S "$s" --server=false -q decl_v1 -d "$work/decl-v1" > /dev/null 2>&1 \
      && timeout 200 scala-cli --power compile -S "$s" --server=false -q --classpath "$work/decl-v1" lib -d "$work/lib" > /dev/null 2>&1 \
      && timeout 200 scala-cli --power compile -S "$s" --server=false -q decl -d "$work/decl" > /dev/null 2>&1) || { rm -rf "$work"; return; }
    python3 - "$work" <<'PY' || { rm -rf "$work"; return; }
import os, sys, zipfile
d = sys.argv[1]
def jar(path, root, keep):
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as z:
        for base, _, files in sorted(os.walk(root)):
            for f in sorted(files):
                rel = os.path.relpath(os.path.join(base, f), root)
                if keep(rel):
                    z.write(os.path.join(base, f), rel)
jar(f'{d}/with-sources/places-lib-1.0.jar', f'{d}/lib', lambda r: r.startswith('pl/') and not r.startswith('pl/PlBox'))
jar(f'{d}/with-sources/places-lib-1.0-sources.jar', f'{d}/src/lib', lambda r: r.endswith('.scala'))
jar(f'{d}/without-sources/places-lib-1.0.jar', f'{d}/lib', lambda r: r.startswith('pl/') and not r.startswith('pl/PlBox'))
jar(f'{d}/places-decl-1.0.jar', f'{d}/decl', lambda r: r.startswith('pl/'))
PY
    touch "$work/built"
    rm -rf "$dir.old"
    [ -d "$dir" ] && mv "$dir" "$dir.old"
    mv "$work" "$dir" && rm -rf "$dir.old"
  fi
}
places_lib_jar() {
  places_lib_build
  echo "$scratch-places/with-sources/places-lib-1.0.jar"
}
places_lib_nosrc_jar() {
  places_lib_build
  echo "$scratch-places/without-sources/places-lib-1.0.jar"
}
places_decl_jar() {
  places_lib_build
  echo "$scratch-places/places-decl-1.0.jar"
}
jar_of() {
  case $1 in
    fixtures) fixtures_jar ;;
    fixtures-shadow) fixtures_shadow_jar ;;
    fixtures-rdecl) fixtures_rdecl_jar ;;
    fixtures-jvm) fixtures_jvm_jar ;;
    javafix) javafix_jar ;;
    javafixdir) echo tests/classfile/fixtures ;;
    abi-callbacks-lib) abi_callbacks_jar ;;
    typetest-lib) typetest_lib_jar ;;
    bundle-lib) bundle_jar ;;
    reflect-lib) reflect_lib_jar ;;
    erasure-lib) erasure_lib_jar ;;
    vctraits-lib) vctraits_lib_jar ;;
    traitfields-lib) traitfields_lib_jar ;;
    deferred-lib) deferred_lib_jar ;;
    outer-lib) outer_lib_jar ;;
    defaults-lib) defaults_lib_jar ;;
    predef-lib) predef_lib_jar ;;
    depfun-lib) depfun_lib_jar ;;
    lazyorder-lib) lazyorder_lib_jar ;;
    view-lib) view_lib_jar ;;
    state-lib) state_lib_jar ;;
    scala2-lib) scala2_lib_jar ;;
    places-lib) places_lib_jar ;;
    places-lib-nosrc) places_lib_nosrc_jar ;;
    places-decl) places_decl_jar ;;
    zio-jvm) echo "$M2/dev/zio/zio_3/2.1.26/zio_3-2.1.26.jar" ;;
    zio-stacktracer-jvm) echo "$M2/dev/zio/zio-stacktracer_3/2.1.26/zio-stacktracer_3-2.1.26.jar" ;;
    zio-internal-macros-jvm) echo "$M2/dev/zio/zio-internal-macros_3/2.1.26/zio-internal-macros_3-2.1.26.jar" ;;
    izumi-reflect-jvm) echo "$M2/dev/zio/izumi-reflect_3/3.0.9/izumi-reflect_3-3.0.9.jar" ;;
    izumi-reflect-boopickle-jvm) echo "$M2/dev/zio/izumi-reflect-thirdparty-boopickle-shaded_3/3.0.9/izumi-reflect-thirdparty-boopickle-shaded_3-3.0.9.jar" ;;
    chimney-jvm) echo "$M2/io/scalaland/chimney_3/1.11.0/chimney_3-1.11.0.jar" ;;
    chimney-macro-commons-jvm) echo "$M2/io/scalaland/chimney-macro-commons_3/2.2.0/chimney-macro-commons_3-2.2.0.jar" ;;
    scala-collection-compat-jvm) echo "$M2/org/scala-lang/modules/scala-collection-compat_3/2.14.0/scala-collection-compat_3-2.14.0.jar" ;;
    scala-library) echo "$M2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar" ;;
    tasty-inspector) echo "$M2/org/scala-lang/scala3-tasty-inspector_3/3.8.4/scala3-tasty-inspector_3-3.8.4.jar" ;;
    scalajs-library) echo "$M2/org/scala-js/scalajs-library_2.13/1.22.0/scalajs-library_2.13-1.22.0.jar" ;;
    scalatest-core) echo "$M2/org/scalatest/scalatest-core_3/3.2.20/scalatest-core_3-3.2.20.jar" ;;
    scalatest-matchers-core) echo "$M2/org/scalatest/scalatest-matchers-core_3/3.2.20/scalatest-matchers-core_3-3.2.20.jar" ;;
    scalatest-shouldmatchers) echo "$M2/org/scalatest/scalatest-shouldmatchers_3/3.2.20/scalatest-shouldmatchers_3-3.2.20.jar" ;;
    scalatest-mustmatchers) echo "$M2/org/scalatest/scalatest-mustmatchers_3/3.2.20/scalatest-mustmatchers_3-3.2.20.jar" ;;
    scalatest-compatible) echo "$M2/org/scalatest/scalatest-compatible/3.2.20/scalatest-compatible-3.2.20.jar" ;;
    scalactic) echo "$M2/org/scalactic/scalactic_3/3.2.20/scalactic_3-3.2.20.jar" ;;
    junit) echo "$M2/junit/junit/4.13.2/junit-4.13.2.jar" ;;
    cats-kernel) echo "$M2/org/typelevel/cats-kernel_3/2.13.0/cats-kernel_3-2.13.0.jar" ;;
    cats-core) echo "$M2/org/typelevel/cats-core_3/2.13.0/cats-core_3-2.13.0.jar" ;;
    circe-numbers) echo "$M2/io/circe/circe-numbers_3/0.14.16/circe-numbers_3-0.14.16.jar" ;;
    circe-core) echo "$M2/io/circe/circe-core_3/0.14.16/circe-core_3-0.14.16.jar" ;;
    circe-generic) echo "$M2/io/circe/circe-generic_3/0.14.16/circe-generic_3-0.14.16.jar" ;;
    zio-json) echo "$M2/dev/zio/zio-json_sjs1_3/0.9.2/zio-json_sjs1_3-0.9.2.jar" ;;
    zio-json-jvm) echo "$M2/dev/zio/zio-json_3/0.9.2/zio-json_3-0.9.2.jar" ;;
    zio-streams-jvm) echo "$M2/dev/zio/zio-streams_3/2.1.26/zio-streams_3-2.1.26.jar" ;;
    zio-test-jvm) echo "$M2/dev/zio/zio-test_3/2.1.26/zio-test_3-2.1.26.jar" ;;
    magnolia-jvm) echo "$M2/com/softwaremill/magnolia1_3/magnolia_3/1.3.23/magnolia_3-1.3.23.jar" ;;
    pureconfig-core-jvm) echo "$M2/com/github/pureconfig/pureconfig-core_3/0.17.10/pureconfig-core_3-0.17.10.jar" ;;
    typesafe-config) echo "$M2/com/typesafe/config/1.4.9/config-1.4.9.jar" ;;
    kantan-csv) echo "$M2/com/nrinaudo/kantan.csv_2.13/0.8.0/kantan.csv_2.13-0.8.0.jar" ;;
    kantan-codecs) echo "$M2/com/nrinaudo/kantan.codecs_2.13/0.6.0/kantan.codecs_2.13-0.6.0.jar" ;;
    kantan-csv-java8) echo "$M2/com/nrinaudo/kantan.csv-java8_2.13/0.8.0/kantan.csv-java8_2.13-0.8.0.jar" ;;
    kantan-codecs-java8) echo "$M2/com/nrinaudo/kantan.codecs-java8_2.13/0.6.0/kantan.codecs-java8_2.13-0.6.0.jar" ;;
    magnolia) echo "$M2/com/softwaremill/magnolia1_3/magnolia_sjs1_3/1.3.18/magnolia_sjs1_3-1.3.18.jar" ;;
    zio) echo "$M2/dev/zio/zio_sjs1_3/2.1.26/zio_sjs1_3-2.1.26.jar" ;;
    sourcecode) echo "$M2/com/lihaoyi/sourcecode_3/0.4.2/sourcecode_3-0.4.2.jar" ;;
    sourcecode-0.4.4) echo "$M2/com/lihaoyi/sourcecode_3/0.4.4/sourcecode_3-0.4.4.jar" ;;
    scala-java-time) echo "$M2/io/github/cquiroz/scala-java-time_sjs1_3/2.7.0/scala-java-time_sjs1_3-2.7.0.jar" ;;
    scala-java-time-tzdb) echo "$M2/io/github/cquiroz/scala-java-time-tzdb_sjs1_3/2.7.0/scala-java-time-tzdb_sjs1_3-2.7.0.jar" ;;
    cats-free) echo "$M2/org/typelevel/cats-free_3/2.13.0/cats-free_3-2.13.0.jar" ;;
    cats-effect-kernel-jvm) echo "$M2/org/typelevel/cats-effect-kernel_3/3.7.0/cats-effect-kernel_3-3.7.0.jar" ;;
    cats-effect-std-jvm) echo "$M2/org/typelevel/cats-effect-std_3/3.7.0/cats-effect-std_3-3.7.0.jar" ;;
    cats-effect-jvm) echo "$M2/org/typelevel/cats-effect_3/3.7.0/cats-effect_3-3.7.0.jar" ;;
    cats-mtl-jvm) echo "$M2/org/typelevel/cats-mtl_3/1.6.0/cats-mtl_3-1.6.0.jar" ;;
    cats-parse) echo "$M2/org/typelevel/cats-parse_3/1.1.0/cats-parse_3-1.1.0.jar" ;;
    fs2-core-jvm) echo "$M2/co/fs2/fs2-core_3/3.13.0/fs2-core_3-3.13.0.jar" ;;
    fs2-io) echo "$M2/co/fs2/fs2-io_3/3.13.0/fs2-io_3-3.13.0.jar" ;;
    scodec-bits) echo "$M2/org/scodec/scodec-bits_3/1.2.5/scodec-bits_3-1.2.5.jar" ;;
    scodec-core) echo "$M2/org/scodec/scodec-core_3/2.2.2/scodec-core_3-2.2.2.jar" ;;
    scodec-cats) echo "$M2/org/scodec/scodec-cats_3/1.2.0/scodec-cats_3-1.2.0.jar" ;;
    ip4s-core) echo "$M2/com/comcast/ip4s-core_3/3.8.0/ip4s-core_3-3.8.0.jar" ;;
    literally) echo "$M2/org/typelevel/literally_3/1.2.0/literally_3-1.2.0.jar" ;;
    natchez-core) echo "$M2/org/tpolecat/natchez-core_3/0.3.8/natchez-core_3-0.3.8.jar" ;;
    sourcepos) echo "$M2/org/tpolecat/sourcepos_3/1.1.0/sourcepos_3-1.1.0.jar" ;;
    typename) echo "$M2/org/tpolecat/typename_3/1.1.0/typename_3-1.1.0.jar" ;;
    twiddles-core) echo "$M2/org/typelevel/twiddles-core_3/0.6.3/twiddles-core_3-0.6.3.jar" ;;
    skunk-core) echo "$M2/org/tpolecat/skunk-core_3/0.6.5/skunk-core_3-0.6.5.jar" ;;
    cats-kernel-sjs) echo "$M2/org/typelevel/cats-kernel_sjs1_3/2.13.0/cats-kernel_sjs1_3-2.13.0.jar" ;;
    cats-core-sjs) echo "$M2/org/typelevel/cats-core_sjs1_3/2.13.0/cats-core_sjs1_3-2.13.0.jar" ;;
    alleycats-core) echo "$M2/org/typelevel/alleycats-core_sjs1_3/2.13.0/alleycats-core_sjs1_3-2.13.0.jar" ;;
    kittens) echo "$M2/org/typelevel/kittens_sjs1_3/3.5.0/kittens_sjs1_3-3.5.0.jar" ;;
    shapeless3-deriving) echo "$M2/org/typelevel/shapeless3-deriving_sjs1_3/3.5.0/shapeless3-deriving_sjs1_3-3.5.0.jar" ;;
    munit) echo "$M2/org/scalameta/munit_sjs1_3/1.3.4/munit_sjs1_3-1.3.4.jar" ;;
    munit-diff) echo "$M2/org/scalameta/munit-diff_sjs1_3/1.3.4/munit-diff_sjs1_3-1.3.4.jar" ;;
    monocle-core) echo "$M2/dev/optics/monocle-core_sjs1_3/3.3.0/monocle-core_sjs1_3-3.3.0.jar" ;;
    monocle-macro) echo "$M2/dev/optics/monocle-macro_sjs1_3/3.3.0/monocle-macro_sjs1_3-3.3.0.jar" ;;
    refined) echo "$M2/eu/timepit/refined_sjs1_3/0.11.4/refined_sjs1_3-0.11.4.jar" ;;
    atto-core) echo "$M2/org/tpolecat/atto-core_sjs1_3/0.9.5/atto-core_sjs1_3-0.9.5.jar" ;;
    case-insensitive) echo "$M2/org/typelevel/case-insensitive_sjs1_3/1.5.0/case-insensitive_sjs1_3-1.5.0.jar" ;;
    magnolia123) echo "$M2/com/softwaremill/magnolia1_3/magnolia_sjs1_3/1.3.23/magnolia_sjs1_3-1.3.23.jar" ;;
    tapir-core) echo "$M2/com/softwaremill/sttp/tapir/tapir-core_sjs1_3/1.13.29/tapir-core_sjs1_3-1.13.29.jar" ;;
    tapir-json-zio) echo "$M2/com/softwaremill/sttp/tapir/tapir-json-zio_sjs1_3/1.13.29/tapir-json-zio_sjs1_3-1.13.29.jar" ;;
    tapir-client) echo "$M2/com/softwaremill/sttp/tapir/tapir-client_sjs1_3/1.13.29/tapir-client_sjs1_3-1.13.29.jar" ;;
    tapir-sttp-client4) echo "$M2/com/softwaremill/sttp/tapir/tapir-sttp-client4_sjs1_3/1.13.29/tapir-sttp-client4_sjs1_3-1.13.29.jar" ;;
    tapir-refined) echo "$M2/com/softwaremill/sttp/tapir/tapir-refined_sjs1_3/1.13.29/tapir-refined_sjs1_3-1.13.29.jar" ;;
    tapir-cats) echo "$M2/com/softwaremill/sttp/tapir/tapir-cats_sjs1_3/1.13.29/tapir-cats_sjs1_3-1.13.29.jar" ;;
    sttp-client4-core) echo "$M2/com/softwaremill/sttp/client4/core_sjs1_3/4.0.26/core_sjs1_3-4.0.26.jar" ;;
    sttp-client4-zio) echo "$M2/com/softwaremill/sttp/client4/zio_sjs1_3/4.0.26/zio_sjs1_3-4.0.26.jar" ;;
    sttp-model) echo "$M2/com/softwaremill/sttp/model/core_sjs1_3/1.7.18/core_sjs1_3-1.7.18.jar" ;;
    sttp-shared-core) echo "$M2/com/softwaremill/sttp/shared/core_sjs1_3/1.5.2/core_sjs1_3-1.5.2.jar" ;;
    sttp-shared-ws) echo "$M2/com/softwaremill/sttp/shared/ws_sjs1_3/1.5.2/ws_sjs1_3-1.5.2.jar" ;;
    sttp-shared-zio) echo "$M2/com/softwaremill/sttp/shared/zio_sjs1_3/1.5.2/zio_sjs1_3-1.5.2.jar" ;;
    zio-streams) echo "$M2/dev/zio/zio-streams_sjs1_3/2.1.26/zio-streams_sjs1_3-2.1.26.jar" ;;
    zio-test) echo "$M2/dev/zio/zio-test_sjs1_3/2.1.26/zio-test_sjs1_3-2.1.26.jar" ;;
    zio-test-sbt) echo "$M2/dev/zio/zio-test-sbt_sjs1_3/2.1.26/zio-test-sbt_sjs1_3-2.1.26.jar" ;;
    chimney) echo "$M2/io/scalaland/chimney_sjs1_3/1.11.0/chimney_sjs1_3-1.11.0.jar" ;;
    chimney-macro-commons) echo "$M2/io/scalaland/chimney-macro-commons_sjs1_3/2.2.0/chimney-macro-commons_sjs1_3-2.2.0.jar" ;;
    scala-collection-compat) echo "$M2/org/scala-lang/modules/scala-collection-compat_sjs1_3/2.14.0/scala-collection-compat_sjs1_3-2.14.0.jar" ;;
    scalajs-dom) echo "$M2/org/scala-js/scalajs-dom_sjs1_3/2.8.1/scalajs-dom_sjs1_3-2.8.1.jar" ;;
    izumi-reflect) echo "$M2/dev/zio/izumi-reflect_sjs1_3/3.0.9/izumi-reflect_sjs1_3-3.0.9.jar" ;;
    izumi-reflect-boopickle) echo "$M2/dev/zio/izumi-reflect-thirdparty-boopickle-shaded_sjs1_3/3.0.9/izumi-reflect-thirdparty-boopickle-shaded_sjs1_3-3.0.9.jar" ;;
    zio-stacktracer) echo "$M2/dev/zio/zio-stacktracer_sjs1_3/2.1.26/zio-stacktracer_sjs1_3-2.1.26.jar" ;;
    zio-internal-macros) echo "$M2/dev/zio/zio-internal-macros_sjs1_3/2.1.26/zio-internal-macros_sjs1_3-2.1.26.jar" ;;
    zio-managed) echo "$M2/dev/zio/zio-managed_sjs1_3/2.1.26/zio-managed_sjs1_3-2.1.26.jar" ;;
    zio-interop-cats) echo "$M2/dev/zio/zio-interop-cats_sjs1_3/23.1.0.13/zio-interop-cats_sjs1_3-23.1.0.13.jar" ;;
    zio-interop-tracer) echo "$M2/dev/zio/zio-interop-tracer_sjs1_3/23.1.0.13/zio-interop-tracer_sjs1_3-23.1.0.13.jar" ;;
    cats-effect-kernel) echo "$M2/org/typelevel/cats-effect-kernel_sjs1_3/3.6.3/cats-effect-kernel_sjs1_3-3.6.3.jar" ;;
    cats-effect-std) echo "$M2/org/typelevel/cats-effect-std_sjs1_3/3.6.3/cats-effect-std_sjs1_3-3.6.3.jar" ;;
    cats-effect) echo "$M2/org/typelevel/cats-effect_sjs1_3/3.6.3/cats-effect_sjs1_3-3.6.3.jar" ;;
    cats-mtl) echo "$M2/org/typelevel/cats-mtl_sjs1_3/1.6.0/cats-mtl_sjs1_3-1.6.0.jar" ;;
    fs2-core) echo "$M2/co/fs2/fs2-core_sjs1_3/3.12.2/fs2-core_sjs1_3-3.12.2.jar" ;;
    macrotask-executor) echo "$M2/org/scala-js/scala-js-macrotask-executor_sjs1_3/1.1.1/scala-js-macrotask-executor_sjs1_3-1.1.1.jar" ;;
    sjr-core) echo "$M2/com/github/japgolly/scalajs-react/core_sjs1_3/4.0.0/core_sjs1_3-4.0.0.jar" ;;
    sjr-core-generic) echo "$M2/com/github/japgolly/scalajs-react/core-generic_sjs1_3/4.0.0/core-generic_sjs1_3-4.0.0.jar" ;;
    sjr-core-ext-cats) echo "$M2/com/github/japgolly/scalajs-react/core-ext-cats_sjs1_3/4.0.0/core-ext-cats_sjs1_3-4.0.0.jar" ;;
    sjr-core-ext-cats_effect) echo "$M2/com/github/japgolly/scalajs-react/core-ext-cats_effect_sjs1_3/4.0.0/core-ext-cats_effect_sjs1_3-4.0.0.jar" ;;
    sjr-core-bundle-cats_effect) echo "$M2/com/github/japgolly/scalajs-react/core-bundle-cats_effect_sjs1_3/4.0.0/core-bundle-cats_effect_sjs1_3-4.0.0.jar" ;;
    sjr-facade) echo "$M2/com/github/japgolly/scalajs-react/facade_sjs1_3/4.0.0/facade_sjs1_3-4.0.0.jar" ;;
    sjr-util) echo "$M2/com/github/japgolly/scalajs-react/util_sjs1_3/4.0.0/util_sjs1_3-4.0.0.jar" ;;
    sjr-util-fallbacks) echo "$M2/com/github/japgolly/scalajs-react/util-fallbacks_sjs1_3/4.0.0/util-fallbacks_sjs1_3-4.0.0.jar" ;;
    sjr-util-cats_effect) echo "$M2/com/github/japgolly/scalajs-react/util-cats_effect_sjs1_3/4.0.0/util-cats_effect_sjs1_3-4.0.0.jar" ;;
    sjr-callback) echo "$M2/com/github/japgolly/scalajs-react/callback_sjs1_3/4.0.0/callback_sjs1_3-4.0.0.jar" ;;
    sjr-callback-ext-cats) echo "$M2/com/github/japgolly/scalajs-react/callback-ext-cats_sjs1_3/4.0.0/callback-ext-cats_sjs1_3-4.0.0.jar" ;;
    sjr-callback-ext-cats_effect) echo "$M2/com/github/japgolly/scalajs-react/callback-ext-cats_effect_sjs1_3/4.0.0/callback-ext-cats_effect_sjs1_3-4.0.0.jar" ;;
    sjr-extra) echo "$M2/com/github/japgolly/scalajs-react/extra_sjs1_3/4.0.0/extra_sjs1_3-4.0.0.jar" ;;
    microlibs-compile-time) echo "$M2/com/github/japgolly/microlibs/compile-time_sjs1_3/4.2.1/compile-time_sjs1_3-4.2.1.jar" ;;
    microlibs-types) echo "$M2/com/github/japgolly/microlibs/types_sjs1_3/4.2.1/types_sjs1_3-4.2.1.jar" ;;
    *) echo "unknown jar $1" >&2; exit 2 ;;
  esac
}
# A set name on a `// jars:` line stands for its jars: `scalajs-react` for the application's
# scalajs-react 4.0.0 (callback, callback-ext-cats, callback-ext-cats_effect, core-bundle-cats_effect,
# extra) with what their poms name, in the order sbt puts them: a jar's copy of a class shadows the
# copies after it (callback's `EffectFallbacks1` over util-fallbacks' dummy, core-bundle-cats_effect's
# `DefaultEffects`), and `sjr-core`, whose `DefaultEffects` is the Callback one, is not in the set.
set_of() {
  local name
  for name in "$@"; do
    case $name in
      scalajs-react) echo sjr-callback sjr-callback-ext-cats sjr-callback-ext-cats_effect sjr-core-bundle-cats_effect \
        sjr-extra sjr-core-ext-cats_effect sjr-util-cats_effect sjr-core-ext-cats sjr-core-generic sjr-facade sjr-util \
        sjr-util-fallbacks microlibs-compile-time microlibs-types sourcecode-0.4.4 scalajs-dom cats-kernel-sjs \
        cats-core-sjs cats-effect-kernel cats-effect-std cats-effect cats-mtl macrotask-executor ;;
      *) echo "$name" ;;
    esac
  done
}
# The `// jars:` line of a test, a file or a directory of them: the class path in JARS_CP (empty
# without one) and the names that are not in the coursier cache in JARS_MISSING.
jars_of() {
  JARS_CP=""
  JARS_MISSING=""
  local name path
  for name in $(set_of $(grep -h -o '^// jars: .*' "$1" "$1"/*.scala 2> /dev/null | head -1 | sed 's|^// jars: ||')); do
    path=$(jar_of "$name")
    [ -e "$path" ] || JARS_MISSING="$JARS_MISSING $name"
    JARS_CP="$JARS_CP:$path"
  done
  JARS_CP=${JARS_CP#:}
}
# The scala-library jar a JVM build links against when --classpath names none, for `run_jvm`: the newest 3.x
# release of the coursier caches, found as the compiler's `classpath::find_scala_library` (src/classpath.rs) finds
# it. The two must agree, so a change of either is a change of both: the caches COURSIER_CACHE when it is set (even
# empty), ~/Library/Caches/Coursier/v1 and ~/.cache/coursier/v1, in that order (on Windows coursier's own,
# `task::fetch::coursier_cache`: a non-empty COURSIER_CACHE, else %LOCALAPPDATA%/Coursier/cache/v1); a version
# whose dot-separated parts all read as numbers (`u32`) and whose first is 3, with its jar a file; the greatest,
# the first found on a tie. Nothing is printed when there is none, as the compiler then refuses the build.
scala_library_jar() {
  python3 - <<'PY'
import os, re
env = os.environ
if os.name == "nt":
    cache = env.get("COURSIER_CACHE") or (os.path.join(env["LOCALAPPDATA"], "Coursier", "cache", "v1") if env.get("LOCALAPPDATA") else None)
    caches = [cache] if cache else []
else:
    home = env.get("HOME", "")
    caches = ([env["COURSIER_CACHE"]] if "COURSIER_CACHE" in env else []) + [os.path.join(home, "Library/Caches/Coursier/v1"), os.path.join(home, ".cache/coursier/v1")]
best = None
for cache in caches:
    d = os.path.join(cache, "https/repo1.maven.org/maven2/org/scala-lang/scala-library")
    try:
        entries = os.listdir(d)
    except OSError:
        continue
    for version in entries:
        parts = version.split(".")
        if not all(re.fullmatch(r"\+?[0-9]+", p) and int(p) < 2 ** 32 for p in parts):
            continue
        parts = [int(p) for p in parts]
        jar = os.path.join(d, version, f"scala-library-{version}.jar")
        if parts[0] == 3 and os.path.isfile(jar) and (best is None or parts > best[0]):
            best = (parts, jar)
if best:
    print(best[1])
PY
}
# run_js <seconds> <out> <build arguments...> [-- <program arguments...>]: `teq compiler build ... -o <out>`,
# then node on the file with the program's arguments, their streams in that order (the raw `teq run`, retired
# with the project verbs at the top level, printed the same stream); a build that fails keeps its exit code and
# message, and node does not run. The build and the run are under one bound, as the raw run that did both was:
# the timeout ends them, and whatever they started, at <seconds> (exit 124). Self-contained, so that it runs as
# well in a shell it is exported to.
run_js() {
  local seconds=$1
  shift
  TEQ=$TEQ timeout "$seconds" bash -c 'out=$1 build=()
    shift
    while [ $# -gt 0 ] && [ "$1" != -- ]; do build+=("$1"); shift; done
    [ $# = 0 ] || shift
    "$TEQ" compiler build "${build[@]}" -o "$out" && exec node "$out" "$@"' run_js "$@"
}
# run_jvm <seconds> <jar or class directory> [<class path>] [--build <build arguments...>] [-- <program
# arguments...>]: java on a JVM build's output as the raw `teq run --target jvm` started it: -Xss512m (the std's
# recursion needs the stack), -Xshare:auto, the output first on the class path, then the jars the build was given,
# with the scala-library jar the build linked against put first where they name none (as the compiler puts it on
# its own class path, `scala_library_jar`, looked up before the bound), and TeqMain, the entry point of a build
# without --all-mains. With --build, `teq compiler build <build arguments...> -o <out>` first, under the same one
# bound (the build arguments give --target jvm), its failure keeping its exit code and message; without, the
# output is built already and java alone has the bound.
run_jvm() {
  local seconds=$1 out=$2 cp=
  shift 2
  if [ $# -gt 0 ] && [ "$1" != -- ] && [ "$1" != --build ]; then
    cp=$1
    shift
  fi
  case ":$cp:" in *"/scala-library-"*) ;; *) cp="$(scala_library_jar)${cp:+:$cp}" ;; esac
  TEQ=$TEQ timeout "$seconds" bash -c 'out=$1 cp=$2
    shift 2
    if [ "${1-}" = --build ]; then
      shift
      build=()
      while [ $# -gt 0 ] && [ "$1" != -- ]; do build+=("$1"); shift; done
      "$TEQ" compiler build "${build[@]}" -o "$out" || exit
    fi
    [ $# = 0 ] || shift
    exec java -Xss512m -Xshare:auto -cp "$out:$cp" TeqMain "$@"' run_jvm "$out" "$cp" "$@"
}
# jars_warm: every jar this file builds (the cases of jar_of that call a builder), built now rather than at its first
# use inside a suite's bound, as a fresh machine needs (bench/ship.sh before its steps); fails naming those that could
# not be built.
jars_warm() {
  local name path failed=
  for name in $(sed -n 's/^    \([a-z0-9-]*\)) [a-z0-9_]*_jar ;;$/\1/p' "${BASH_SOURCE[0]}"); do
    path=$(jar_of "$name") && [ -e "$path" ] || failed="$failed $name"
  done
  [ -z "$failed" ] || { echo "jars: not built:$failed" >&2; return 1; }
}
