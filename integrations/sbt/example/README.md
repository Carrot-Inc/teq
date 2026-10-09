# sbt-teq example

The application corpus of `bench/app` (an observatory network's scheduling system over cats and
sourcecode, 64k lines on the frontend side) as an sbt 2 build with sbt-scalajs and sbt-teq:
`shared` and `frontend` are Scala.js projects, `frontend` depending on `shared`; `api` is a JVM
project over the `shared` and `api` roots, so its description takes the JVM jars, scala-library
among them, where the frontend's takes the `_sjs1_3` ones, and `teqBuild` writes its class files.
`api` has a `Test` configuration too, `api-test-src/` (checked in, like `browserdemo-src/`): one
suite of each framework the reference application uses (zio-test with `ZIOSpecDefault` and
`assertTrue`, munit's `ScalaCheckSuite`, scalatest's `AnyFunSuite` with matchers) over the
corpus's `Request`, and the discovery cases (an abstract base extending a framework's suite, a
concrete class extending it, a private suite).

```
TEQ_COMPILER=1 sbt api/compile api/test        # teq as the compiler (../README.md)
```

```
python3 ../../../bench/app/gen.py src    # the corpus's sources
sbt teqBuild                             # frontend/target/teq/out and api/target/teq/out
node frontend/target/teq/out/main.mjs
java -cp "api/target/teq/out:<api's runtime jars>" TeqMain   # check.sh takes them from teq.lock
```

The build takes the release of sbt-teq that `project/plugins.sbt` pins, from Maven Central, and
`teqBuild` runs the binary that `TEQ` or `ThisBuild / teqBinary` names (`teqBinary := file("teq")`
for the one on the PATH). Without either it fetches the compiler `ThisBuild / teqVersion` names, the
plugin's default when the build names none, from that version's GitHub release (`../README.md`,
"The teq binary"), and checks its digest. A branch's plugin is published
locally under a version of its own and named by `TEQ_PLUGIN_VERSION`, with `TEQ` naming the
branch's binary, which no repository holds:

```
(cd .. && sbt --batch 'set version := "0.1.1-mine-SNAPSHOT"; publishLocal')
TEQ_PLUGIN_VERSION=0.1.1-mine-SNAPSHOT TEQ=$PWD/../../../target/release/teq sbt teqBuild
```

`sbt teqExportAll` writes `teq.lock`, the fixture of the export (`../README.md`, "The
export"): the projects as the tools without sbt read them, `api`'s BuildInfo a `buildInfo`
generator, the labels of `browserdemo` and `mirrored` a command generator
(`generate-labels.mjs`, which sbt's compile runs too), the images of `browserdemo` another, a Scala
script the build's own binary runs (`teq interp scripts/images.scala`, below), the main classes and
the dev commands declared, `api`'s Docker stage, which sbt-native-packager writes for it as the
reference application's API is packaged. `./check-export.sh` (or `EXPORT_ONLY=1 ./check.sh`) checks it: two
exports the same bytes, the committed file but for its header, a dependency's lines, a
released compiler's binaries, a layer function of the build's own, the refusals, and with
`EXPORT_LINUX=1` the export on Linux in Docker.

`api` is packaged for Docker by sbt-native-packager into `api/target/docker/stage`, whose layers
the `Dockerfile` copies as the reference application's does. `teq stage api` writes the same
stage with no sbt, and `teq run api server` runs its main class (the alias `server`).
`./check-stage.sh` checks the export's stage against native-packager's own (`sbt
api/Docker/stage`: the tree, the dependencies' bytes, the manifest, the script's classpath and
main class), then `teq stage api`, `teq run api server`, and an image built from the
Dockerfile, whose entrypoint prints what scalac's build prints:

```
teq stage api && docker build -t api . && docker run --rm api
```

`./check-lists.sh` writes the gate's lists of the application's API side over `api`
(`bench/app/app-lists.sh`, through sbt-teq's `teqInputs`) and checks them; its header lists what it checks.

The frontend also builds through the vite plugin, [`../../vite`](../../vite), which builds the
export's project `frontend` (`vite.config.js`; `main.js` is the entry importing `scalajs:main.js`),
and the same plugin serves `browserdemo`'s page (`vite.config.browserdemo.js`). The example's export
pins a release from before the project verbs took the binary's top level, so `TEQ` names the binary:

```
npm install                              # vite, and the plugin from ../../vite
npx vite build                           # dist/frontend.js, bundled from `teq build frontend --release`
node dist/frontend.js
npx vite                                 # a dev server over `teq watch frontend`
teq dev browserdemo                 # no sbt: the generators, npm install, then
                                         # vite --config vite.config.browserdemo.js, restarted on a change
```

`browserdemo`'s images (`browserdemo-images/`) have a generator of the shape an application's icons
have: `scripts/images.scala` writes an object per image folder into the directory the build appends
and the module `browserdemo-images/assets/images.js` beside the images, which the generator's
`outputs` names. Its first word `teq` is the build's own binary (sbt-teq's resolved one under sbt,
the one running under `teq`), so it needs no JVM, scala-cli or shell: `GENERATOR_ONLY=1 ./check.sh`
runs it under sbt's compile, `teq compile` and `teq dev` with a `teq` and a `scala-cli` first on the
PATH that must not run.

`COMPILER_ONLY=1 ./check.sh` runs the compiler section alone, `RELEASE_ONLY=1 ./check.sh` the
resolution of a release's binary; `.jvmopts` gives the sbt server
the heap scalac needs for the corpus when the toggle is off.

The page of `browserdemo-src/` is two more projects, both with `TEQ_COMPILER=1` in sbt's
environment answering `fastLinkJS` and `fullLinkJS` from teq. It is written against Scala.js's
own facades, so scalac with the Scala.js linker builds it too, which the toggle decides. Its
entry, `main.scalajs.js`, is `import "scalajs:main.js"` and nothing else.

```
TEQ_COMPILER=1 sbt ~browserdemo/fastLinkJS                 # the stock Scala.js vite plugin:
npx vite --config vite.scalajs.config.js                   # it prints fastLinkJSOutput
TEQ_COMPILER=1 sbt ~mirrored/fastLinkJS                    # a build shaped like the reference
npx vite --config vite.mirrored.config.js                  # application's: a mirror, a task of its own
```

`mirrored` sets the linker's directories, wraps each link task to sync a mirror
(`project/Mirror.scala`), names the mirror in `servedOutput`, which the plugin in
`vite.mirrored.config.js` prints, and has a watch trigger of its own; both projects generate
`demo.Labels` from `browserdemo-labels.txt`, an input sbt knows through `fileInputs` alone.
`ThisBuild / teqLinkIdle` is 8 seconds here, so that the check sees an idle resident go.

`./check.sh` does all of it and compares both sides, and the vite bundle, with scalac's output in
`bench/app/expected`; of the dev server it checks that the entry and the output's `main.mjs` are
served; then the page under headless Chrome, `node check-scalajs-dev.mjs stock`, `toggle` and
`mirrored` (the file's head lists the checks and how a mode's time is bounded; `bin/sbt` on
vite's PATH keeps the printed directory the last line of sbt 2's output, as the stock plugin
reads it); then `api` through sbt's own tasks with `TEQ_COMPILER=1` in one sbt server session
(compile, run, the suites, a failing test, a type error and its unchanged retry, packageBin, a
body edit timed through sbt, a deleted file, `clean`, an upstream edit reaching the tests, forked
tests, the toggle off and on again), each printed as a line; then the Scala.js tests, and the
failures of a resident's first build, a failed retype, a deletion while a type error stands,
sibling projects compiled at once, an upstream inline body edit and the toggle of one project in
either direction over `sjscore` and `sjsapp` and their JVM pair `jvmcore` and `jvmapp`, and a
compile cancelled while its resident starts.
