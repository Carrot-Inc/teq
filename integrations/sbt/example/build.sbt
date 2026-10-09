import org.scalajs.linker.interface.ModuleKind
import scala.concurrent.duration.*

// The application corpus of bench/app as an sbt build: generate its sources into src/ first with
// `python3 ../../../bench/app/gen.py src`.
ThisBuild / scalaVersion := "3.8.4"
ThisBuild / scalacOptions ++= Seq("-Wconf:any:s", "-Xmax-inlines", "80")
// Short, so that check.sh sees a link's resident go once it is idle (a minute by default).
ThisBuild / teqLinkIdle := 8.seconds
// The compiler teq.lock pins, its binaries from its GitHub release (bench/release.sh --pin moves it to each
// release, the plugin's version a line of its own), and the exemption that lets the export run with a branch's
// plugin, published locally under a SNAPSHOT version of its own (TEQ_PLUGIN_VERSION, project/plugins.sbt), and
// write the binaries table empty while the release it names is not published yet.
ThisBuild / teqVersion := "0.1.7"
ThisBuild / teqExportSnapshots := true
// teq as the build tool: teqExportAll writes teq.lock and the launchers teq and teq.cmd at the root,
// which the example commits and its checks run (`./teq ...`). Bare, as sbt 2 takes a setting
// of build.sbt for every project; the key by its name, so that the build loads with a plugin from
// before the setting, the release project/plugins.sbt pins.
SettingKey[Boolean]("teqBuildTool") := true

val libraries = libraryDependencies ++= Seq(
  "org.typelevel" %% "cats-core" % "2.13.0",
  "com.lihaoyi" %% "sourcecode" % "0.4.2",
)

def sourcesIn(dirs: String*) = Compile / unmanagedSourceDirectories := dirs.map(dir => (ThisBuild / baseDirectory).value / "src" / dir)

lazy val shared = project
  .enablePlugins(ScalaJSPlugin)
  .settings(libraries, sourcesIn("shared"))

lazy val frontend = project
  .enablePlugins(ScalaJSPlugin)
  .dependsOn(shared)
  .settings(
    libraries,
    sourcesIn("frontend"),
    scalaJSUseMainModuleInitializer := true,
    // One module per file for the pages and components, so a hot swap re-executes the edited
    // file alone: teqLinkJS (the sbt-driven dev loop, docs/TARGETS.md "The sbt plugin") and
    // vite-plugin-teq's own `teq watch` both read this setting.
    teqModulePerFile := Seq("meridian.frontend.page", "meridian.frontend.component"),
    // The corpus's class catalog, a macro's memo, holds nothing but a cache: declared, its changes
    // keep a build on its workers (docs/TARGETS.md, "The typer's workers"), as bench/programs.sh
    // declares it.
    teqCacheableState := Seq("meridian.web.css.Catalog"),
    teqDevCommand := Seq("npx", "vite", "--config", "vite.config.js"),
    teqMainClasses := Seq("meridian.frontend.Main"),
  )

// The API side has a Test configuration with one suite of each framework the reference
// application uses (api-test-src/, checked in like browserdemo-src/): zio-test, munit with
// scalacheck, scalatest with matchers, and a JUnit-style one found by its annotated methods;
// `TEQ_COMPILER=1 sbt api/test` runs them through teq. Its BuildInfo has the keys of the reference
// application's: static ones, the commit, and the class directory of another configuration. It is
// packaged as the reference application's API is, by sbt-native-packager's Docker stage into its
// own target directory, which the Dockerfile beside it copies by layer.
lazy val api = project
  .enablePlugins(BuildInfoPlugin, JavaAppPackaging)
  .settings(
    libraries,
    sourcesIn("shared", "api"),
    Compile / mainClass := Some("meridian.server.Main"),
    teqRunAliases := Map("server" -> "meridian.server.Main"),
    dockerExposedPorts := Seq(8080),
    Docker / target := baseDirectory.value / "target" / "docker",
    // As the reference application's: the stage packages no scaladoc, which also fails on the corpus.
    Compile / packageDoc / publishArtifact := false,
    buildInfoPackage := "meridian.build",
    buildInfoKeys := Seq[BuildInfoKey](name, version, scalaVersion, sbtVersion),
    buildInfoKeys += BuildInfoKey.action("gitSha")(sys.env.getOrElse("SOURCE_VERSION", scala.sys.process.Process(Seq("git", "rev-parse", "HEAD")).!!.trim)),
    buildInfoKeys += BuildInfoKey("testClasses" -> (Test / classDirectory).value),
    Test / unmanagedSourceDirectories := Seq((ThisBuild / baseDirectory).value / "api-test-src"),
    libraryDependencies ++= Seq(
      "dev.zio" %% "zio-test" % "2.1.26" % Test,
      "dev.zio" %% "zio-test-sbt" % "2.1.26" % Test,
      "org.scalameta" %% "munit" % "1.3.4" % Test,
      "org.scalameta" %% "munit-scalacheck" % "1.3.0" % Test,
      "org.scalatest" %% "scalatest" % "3.2.20" % Test,
      // The JUnit framework, for a suite found by its `@org.junit.Test` methods.
      "com.github.sbt" % "junit-interface" % "0.13.3" % Test,
    ),
    testFrameworks += new TestFramework("zio.test.sbt.ZTestFramework"),
    // The JUnit-style suite is found by sbt through the analysis's method annotations, but teq
    // writes no annotation attributes into class files yet, so JUnit itself finds no `@Test` in
    // it: it stays discoverable and out of the runs.
    Test / testOptions += Tests.Exclude(Seq("meridian.apitest.JunitStyleSuite")),
  )

// A small hand-written page (browserdemo-src/, not bench/app's generated corpus, so it survives
// check.sh's `rm -rf src`) that mounts real React to the real DOM, written against Scala.js's own
// facade syntax so that scalac with the Scala.js linker and teq both build it: the toggle decides
// which (docs/TARGETS.md, "The vite plugin"). One module per file for the widgets, so an edit
// swaps the edited component alone. `demo.Labels` is a generated source, written by
// generate-labels.mjs from browserdemo-labels.txt, an input sbt knows through the generator's
// `fileInputs` alone, which has to retrigger a watched link like a source edit.

def demoPage(project: Project): Project = project
  .enablePlugins(ScalaJSPlugin)
  .settings(
    Compile / unmanagedSourceDirectories := Seq((ThisBuild / baseDirectory).value / "browserdemo-src"),
    scalaJSUseMainModuleInitializer := true,
    Compile / mainClass := Some("demo.run"),
    scalaJSLinkerConfig ~= (_.withModuleKind(ModuleKind.ESModule)),
    teqModulePerFile := Seq("demo.widgets"),
    Compile / teqGenerators += TeqCommand(Seq("node", "generate-labels.mjs", "browserdemo-labels.txt"), inputs = Seq("browserdemo-labels.txt")),
  )

// The page through the stock Scala.js vite plugin (vite.scalajs.config.js), which prints
// fastLinkJSOutput and fullLinkJSOutput. Its Test configuration (browserdemo-test-src/) holds
// suites of the frameworks a Scala.js application uses, which `test` runs under node through
// sbt-scalajs's test adapter: over the linker's output, or over teq's under `TEQ_COMPILER`.
// `demoFailing` adds suites that fail on purpose (browserdemo-test-failing/), for check.sh.
val demoFailing = settingKey[Boolean]("Whether browserdemo's tests include the suites that fail on purpose")

// Its images (browserdemo-images/) have a generator that the build's own binary runs, a Scala script
// under the interpreter, as an application's icons have theirs: `teq` is sbt-teq's resolved binary under
// sbt and the one running under `teq`, never a teq of the PATH. It writes the objects into the directory
// appended to `run` and the module assets/images.js beside the images, which `outputs` names, so that
// `teq` runs it again when the module is gone.
lazy val browserdemo = project
  .configure(demoPage)
  .settings(
    Compile / teqGenerators += TeqCommand(
      Seq("teq", "interp", "scripts/images.scala", "--", "browserdemo-images"),
      inputs = Seq("browserdemo-images/assets/images/**/*.svg"),
      outputs = Seq("browserdemo-images/assets/images.js"),
    ),
    teqDevCommand := Seq("npx", "vite", "--config", "vite.config.browserdemo.js"),
    demoFailing := false,
    Test / unmanagedSourceDirectories := Seq((ThisBuild / baseDirectory).value / "browserdemo-test-src") ++
      (if (demoFailing.value) Seq((ThisBuild / baseDirectory).value / "browserdemo-test-failing") else Nil),
    libraryDependencies ++= Seq(
      "dev.zio" %% "zio-test" % "2.1.26" % Test,
      "dev.zio" %% "zio-test-sbt" % "2.1.26" % Test,
      "org.scalameta" %% "munit" % "1.3.4" % Test,
      "com.lihaoyi" %% "utest" % "0.9.1" % Test,
    ),
    testFrameworks += new TestFramework("utest.runner.Framework"),
  )

// The same page in a build shaped like the reference application's: the linker's directories
// set by the build, every link followed by the sync of a mirror that the dev server and the
// bundler read (project/Mirror.scala), a task of the build's own naming the mirror, which a
// plugin of the build's own prints (vite.mirrored.config.js), the linker's record dropped when
// its directory is gone, and a watch trigger of the build's own. None of it names teq.
val servedOutput = taskKey[File]("The directory the page is served from: the mirror of the linker's output")
val relinkIfOutputMissing = taskKey[Unit]("Forces a link when the linker's output directory is missing")

lazy val mirrored = project
  .configure(demoPage)
  .settings(
    Seq((fastLinkJS, "fastopt"), (fullLinkJS, "opt")).flatMap { (linkTask, suffix) =>
      Seq(
        Compile / linkTask / scalaJSLinkerOutputDirectory := baseDirectory.value / "target" / s"page-$suffix",
        Compile / linkTask / relinkIfOutputMissing := Def.uncached {
          val directory = (Compile / linkTask / scalaJSLinkerOutputDirectory).value
          val record = (Compile / linkTask / streams).value.cacheDirectory / "linking-report.bin"
          if (record.exists && !directory.exists) IO.delete(record)
        },
        Compile / linkTask := Def.uncached {
          val report = (Compile / linkTask).dependsOn(Compile / linkTask / relinkIfOutputMissing).value
          Mirror.sync((Compile / linkTask / scalaJSLinkerOutputDirectory).value, streams.value.log)
          report
        },
        Compile / linkTask / servedOutput := Def.uncached {
          val _ = (Compile / linkTask).value
          Mirror.of((Compile / linkTask / scalaJSLinkerOutputDirectory).value)
        },
      )
    },
    Compile / watchTriggers += (ThisBuild / baseDirectory).value.toGlob / "browserdemo-assets" / "*.svg",
  )

// Two Scala.js projects whose tests read the other's classes through its class directory
// (exportJars off), for check.sh: the tests' digests have to see an edit of the other project.
lazy val sjscore = project
  .enablePlugins(ScalaJSPlugin)
  .settings(
    exportJars := false,
    // sjscore-js-src holds what only the Scala.js side compiles: a macro over the lean std's javalib (FileMacros).
    Compile / unmanagedSourceDirectories := Seq((ThisBuild / baseDirectory).value / "sjscore-src", (ThisBuild / baseDirectory).value / "sjscore-js-src"),
  )

lazy val sjsapp = project
  .enablePlugins(ScalaJSPlugin)
  .dependsOn(sjscore)
  .settings(
    exportJars := false,
    // sjsapp-js-test-src holds the tests of what only the Scala.js side compiles (LeanStdSuite over FileMacros).
    Test / unmanagedSourceDirectories := Seq((ThisBuild / baseDirectory).value / "sjsapp-test-src", (ThisBuild / baseDirectory).value / "sjsapp-js-test-src"),
    libraryDependencies += "org.scalameta" %% "munit" % "1.3.4" % Test,
  )

// sjscore and sjsapp on the JVM, over the same sources, for check.sh: one project's tests over the
// other's class directory, with the toggle of either.
lazy val jvmcore = project
  .settings(
    exportJars := false,
    Compile / unmanagedSourceDirectories := Seq((ThisBuild / baseDirectory).value / "sjscore-src"),
  )

lazy val jvmapp = project
  .dependsOn(jvmcore)
  .settings(
    exportJars := false,
    Test / unmanagedSourceDirectories := Seq((ThisBuild / baseDirectory).value / "sjsapp-test-src"),
    libraryDependencies += "org.scalameta" %% "munit" % "1.3.4" % Test,
  )

lazy val root = project
  .in(file("."))
  .aggregate(frontend, api, browserdemo, mirrored)
  .disablePlugins(dev.teq.sbt.TeqPlugin)
