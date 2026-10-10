package dev.teq.sbt

import java.io.File
import java.nio.file.Path
import scala.collection.mutable
import scala.sys.process.{Process, ProcessLogger}
import scala.util.control.NonFatal

import sbt.*
import sbt.Keys.*
import sbt.internal.BuildDependencies
import sbt.internal.teq.{TeqCompile, Watching}
import sbt.nio.Keys.{allInputFiles, fileInputs, watchOnTermination, watchTriggers}
import sbt.nio.file.Glob
import sbt.util.Logger

object TeqPlugin extends AutoPlugin:
  override def trigger = allRequirements
  override def requires = plugins.JvmPlugin

  object autoImport:
    val teqBinary = settingKey[File]("The teq binary overriding the resolved one, by default $TEQ; a bare name is looked up on the PATH")
    val teqVersion = settingKey[String]("The version of the teq compiler resolved when teqBinary is not set: by default the compiler this plugin was released with, or $TEQ_VERSION")
    val teqReleases = settingKey[String]("Where the compiler's releases are, from 0.1.7: <base>/v<version>/ holds each platform's binary, SHA256SUMS and the binary manifest (GitHub's releases of teq by default)")
    val teqArtifact = settingKey[String]("The name of the teq binary's artifact")
    val teqClassifier = settingKey[String]("The platform classifier of the teq binary's artifact, such as osx-aarch_64")
    @transient val teqResolvedBinary = taskKey[File]("The teq binary the tasks run: teqBinary when set, else the artifact resolved and copied under target/teq/bin")
    val teqCompiler = settingKey[Boolean]("Whether teq compiles the project in the place of scalac, as zinc's compiler: compile, test, run and packageBin (by default $TEQ_COMPILER)")
    @transient val teqCompilerCommand = taskKey[TeqCompile.Command]("The compile of this configuration by teq as zinc's compiler: its binary, flags, sources and identity")
    val teqTarget = settingKey[String]("What teq compiles to: js, or jvm for class files linked against the jars' bytecode (by default from the project's platform)")
    val teqMainClass = settingKey[Option[String]]("The entry point of a JVM build (--main), for a program with several")
    val teqOutput = settingKey[File]("The directory teq writes the split build into, one ES module per package, or the class files of a JVM build")
    val teqModulePerFile = settingKey[Seq[String]]("Packages whose files become modules of their own (--module-per-file)")
    val teqCacheableState = settingKey[Seq[String]]("Objects, by qualified name, that a macro's author declares to hold nothing but a cache: kept across a session's retypes (--cacheable-state)")
    val teqMacroState = settingKey[String]("With several workers, where a macro's change of state other runs share goes: ordered, the default, types the build again by one worker in scalac's order; per-worker keeps it to the worker (--macro-state)")
    val teqThreads = settingKey[Option[Int]]("How many workers type the bodies of every full build (--threads): teqBuild's, teqFullLinkJS's and the resident compiler's and link sessions'; by default the count teq selects by the program's size, the cores and the memory. A session's retypes are one worker's")
    val teqHot = settingKey[Boolean]("Whether a running page can re-execute the modules (--hot)")
    val teqRelease = settingKey[Boolean]("Whether teqBuild makes the production build (--release)")
    val teqSources = settingKey[Seq[File]]("Compile source directories of the project and of the projects it depends on")
    @transient val teqClasspath = taskKey[Seq[File]]("The library jars teq reads as TASTy, and on the JVM links against")
    @transient val teqScalacOptions = taskKey[Seq[String]]("The scalac options mapped onto teq's flags")
    val teqExtraSources = settingKey[Seq[File]]("Source roots compiled besides teqSources")
    val teqExcludes = settingKey[Seq[String]]("Paths left out of the build: teq drops the files whose path starts or ends with one")
    val teqLib = settingKey[Option[File]]("A directory of stand-ins compiled with the sources")
    val teqProductionSources = settingKey[Seq[File]]("Source roots added to the production build")
    val teqProductionExcludes = settingKey[Seq[String]]("Paths left out of the production build")
    val teqDescriptionKeys = settingKey[Map[String, String]]("String keys written into the project's description in teq.lock as given, for the tools that read it")
    @transient val teqBuild = taskKey[File]("Builds the project with teq from its description")
    val teqServedOutput = settingKey[File]("The directory teqLinkJS serves from: the dev build, split, --module-per-file, --hot")
    val teqFullServedOutput = settingKey[File]("The directory teqFullLinkJS writes its --release build into, one file main.js, for vite build")
    val teqLinkIdle = settingKey[scala.concurrent.duration.FiniteDuration]("How long the resident of a link stays after its last build; a watch that asked for a build holds it for as long as the watch is under way")
    val teqLinkWait = settingKey[scala.concurrent.duration.FiniteDuration]("How long a link waits for another sbt process to give up the directory it writes, before it fails")
    @transient val teqLinkJS = taskKey[File]("Keeps a resident `teq compiler watch` writing the dev build into teqServedOutput, for a Scala.js-style vite plugin")
    @transient val teqFullLinkJS = taskKey[File]("Runs the --release build into teqFullServedOutput, for vite build")
    @transient val teqExportAll = taskKey[File]("Writes teq.lock, the description of every project that teq, the language server and vite-plugin-teq read: under target/teq/, or at the build's root with the launchers under teqBuildTool")
    val teqBuildTool = settingKey[Boolean]("Whether the build takes teq as its build tool: teqExportAll writes teq.lock and the launchers teq and teq.cmd at the build's root, for the repository to commit (teqBuildTool := true in build.sbt)")
    val teqExportSnapshots = settingKey[Boolean]("Whether teqExportAll writes SNAPSHOT and dynamic versions (a library, the plugin, the compiler) where it would refuse them under teqBuildTool")
    val teqGenerators = settingKey[Seq[TeqCommand]]("Commands that generate a configuration's sources, which sbt's compile runs and teqExportAll exports for teq")
    @transient val teqGenerate = taskKey[Seq[File]]("Runs the configuration's teqGenerators into sourceManaged / teq")
    val teqDevCommand = settingKey[Seq[String]]("The command of a Scala.js project's dev loop (teq dev), run from the directory of the package.json at or above its base; by default that package's dev script when the base holds it")
    val teqRunAliases = settingKey[Map[String, String]]("Names for the project's main classes, which teq run takes in their place")
    val teqMainClasses = settingKey[Seq[String]]("The project's main classes for teqExportAll besides Compile / mainClass and the aliases' targets: declared, since finding them takes a compile")
    type TeqCommand = dev.teq.sbt.TeqCommand
    val TeqCommand = dev.teq.sbt.TeqCommand

  import autoImport.*

  override def globalSettings: Seq[Setting[?]] = Seq(
    teqBinary := sys.env.get("TEQ").fold(Unset)(file),
    // The compiler the plugin was released with, its own version line apart (docs/TOOLING.md, "Releases"); the
    // language server names its own by TEQ_VERSION when it adds the plugin to a build's export.
    teqVersion := sys.env.get("TEQ_VERSION").filter(_.nonEmpty).getOrElse(BuildInfo.compiler),
    teqReleases := Release.DefaultBase,
    teqArtifact := "teq",
    teqClassifier := platformClassifier(System.getProperty("os.name"), System.getProperty("os.arch")),
    teqCompiler := sys.env.get("TEQ_COMPILER").exists(v => Set("1", "true", "yes", "on")(v.trim.toLowerCase)),
    teqLinkIdle := scala.concurrent.duration.DurationInt(60).seconds,
    teqLinkWait := scala.concurrent.duration.DurationInt(30).seconds,
    teqBuildTool := false,
    teqExportSnapshots := false,
    onUnload ~= (previous => state => { Resident.stopAll(); Directory.releaseAll(); previous(state) }),
    sbt.internal.teq.Cancelling.atEnd(() => Directory.evaluationEnded()),
  )

  Resident.onStopped = Directory.release

  /** The default of `teqBinary` when `TEQ` is not in the environment: resolve the artifact. */
  private val Unset = file("")

  /** The group of a SNAPSHOT of the compiler published locally, the one compiler resolved as a Maven artifact
    * through the build's resolvers; every other is its release's (`Release`). */
  val SnapshotGroup = "build.teq"

  /** The classifier in the convention of protoc's artifacts, which sbt-protoc reads the same way. */
  def platformClassifier(osName: String, osArch: String): String =
    val name = osName.toLowerCase
    val os =
      if name.startsWith("mac") || name.startsWith("darwin") then "osx"
      else if name.startsWith("windows") then "windows"
      else if name.startsWith("linux") then "linux"
      else name.replaceAll("[^a-z0-9]+", "")
    val arch = osArch.toLowerCase match
      case "amd64" | "x86_64" | "x64" => "x86_64"
      case "aarch64" | "arm64" => "aarch_64"
      case other => other
    s"$os-$arch"

  /** The build's defaults a project reads where it sets none: `ThisBuild / teqThreads` reaches every project.
    * Without the driver the lock is this machine's description, not a committed one that a later
    * resolution would make drift, so that the export names a SNAPSHOT or dynamic version as resolved. */
  override def buildSettings: Seq[Setting[?]] = Seq(
    teqThreads := None,
    teqExportSnapshots := !Export.buildTool.value,
  ) ++ Export.buildSettings

  /** The projects of the build with this plugin enabled. */
  def projectsWithPlugin(build: sbt.internal.LoadedBuild): Seq[ProjectRef] =
    build.allProjectRefs.collect { case (ref, project) if project.autoPlugins.contains(TeqPlugin) => ref }

  override def projectSettings: Seq[Setting[?]] = Seq(
    teqTarget := (if platform.value == "jvm" then Jvm else "js"),
    teqMainClass := None,
    teqOutput := baseDirectory.value / "target" / "teq" / "out",
    teqModulePerFile := Nil,
    teqCacheableState := Nil,
    teqMacroState := "ordered",
    teqHot := false,
    teqRelease := false,
    teqSources := Def.settingDyn(sourceDirectories(configDependencies(buildDependencies.value, thisProjectRef.value, Compile))).value,
    teqClasspath := {
      val converter = fileConverter.value
      val jars = (Compile / externalDependencyClasspath).value.map(entry => converter.toPath(entry.data).toFile)
      if teqTarget.value == Jvm then jars.filter(_.getName.endsWith(".jar"))
      else jars.filter(isTastyJar(platform.value))
    },
    teqScalacOptions := (Compile / scalacOptions).value,
    teqExtraSources := Nil,
    teqExcludes := Nil,
    teqLib := None,
    teqProductionSources := Nil,
    teqProductionExcludes := Nil,
    teqDescriptionKeys := Map.empty,
    teqResolvedBinary := Def.taskIf {
      if teqBinary.value != Unset then teqBinary.value
      else resolveBinary.value
    }.value,
    teqBuild := {
      val description = describe.value
      IO.createDirectory(teqOutput.value)
      runTeq(description.teq, description.buildArgs(production = teqRelease.value), ArgsFile.of(baseDirectory.value, "compile", "build"), description.root, streams.value.log)
      teqOutput.value
    },
    // `clean` removes the argument files the plugin's commands start with (`ArgsFile`), the rest
    // of `target/teq` kept: the lock `teq.lock` where the export writes it there, the binary's
    // copy, the links' directories and their locks.
    cleanFiles ++= ArgsFile.files(baseDirectory.value),
    teqServedOutput := baseDirectory.value / "target" / "teq" / "served",
    teqFullServedOutput := baseDirectory.value / "target" / "teq" / "served-full",
    teqLinkJS := link(teqServedOutput).value,
    teqFullLinkJS := fullLink(teqFullServedOutput).value,
    teqDevCommand := Nil,
    teqRunAliases := Map.empty,
    teqMainClasses := Nil,
    teqGenerators := Nil,
    // The build's own binary for a command whose first word is `teq`, resolved only then.
    Compile / teqGenerate := Def.uncached(Def.taskIf {
      if (Compile / teqGenerators).value.exists(_.runsTeq) then
        val _ = (Compile / teqGenerate / allInputFiles).value
        Export.generate((Compile / teqGenerators).value, (LocalRootProject / baseDirectory).value, (Compile / sourceManaged).value / "teq", Some(teqResolvedBinary.value), streams.value.log)
      else
        val _ = (Compile / teqGenerate / allInputFiles).value
        Export.generate((Compile / teqGenerators).value, (LocalRootProject / baseDirectory).value, (Compile / sourceManaged).value / "teq", None, streams.value.log)
    }.value),
    Compile / teqGenerate / fileInputs ++= Export.generatorInputs((Compile / teqGenerators).value, (LocalRootProject / baseDirectory).value),
    Compile / sourceGenerators ++= (if (Compile / teqGenerators).value.nonEmpty then Seq((Compile / teqGenerate).taskValue) else Nil),
  ) ++ Export.projectSettings ++ Inputs.projectSettings ++ watched(teqLinkJS, teqServedOutput, Def.setting(true)) ++
    inConfig(Compile)(compilerSettings) ++ inConfig(Test)(compilerSettings)

  /** The dev link into a directory, which `teqLinkJS` runs into `teqServedOutput` and a
    * Scala.js project's `fastLinkJS` under `teqCompiler` into the linker's own: the split build
    * with `teqModulePerFile` and `--hot`, by the directory's resident `teq compiler watch`, and the stub
    * `main.js` beside its `main.mjs`. The task is the directory's one writer while it runs
    * (`Directory`), and what the other compiler or another kind of build left there goes first.
    * The resident ends `teqLinkIdle` after its last build, unless a watch (`~`) asked for a
    * build and is still under way then, whichever task it is a watch of. */
  private[sbt] def link(directory: Def.Initialize[File]): Def.Initialize[Task[File]] = Def.task {
    val description = describe.value
    val log = streams.value.log
    val out = directory.value
    val life = Resident.Life(teqLinkIdle.value, Watching.of(state.value).map(channel => () => Watching.underWay(channel)))
    val identity = s"${binaryVersion(teqResolvedBinary.value)} dev"
    Directory.writing(out, teqLinkWait.value, log) { _ =>
      Directory.takeOver(out, Some(identity), log)
      IO.createDirectory(out)
      val dev = description.copy(out = relativeTo(description.root.toPath, out), hot = true)
      linkBuild(out, dev.teq +: dev.watchArgs, ArgsFile.into(baseDirectory.value, "compile", "link", out), dev.root, log, life)
      writeDevStub(out)
      Directory.mark(out, identity)
    }
    out
  }

  /** The full link into a directory (`teqFullLinkJS`, a Scala.js project's `fullLinkJS` under
    * `teqCompiler`): the `--release` build of the whole program with the production sources as
    * one file, `main.js`, which a bundler's minifier shortens as it cannot the names of split
    * modules. Written beside the directory and renamed into it. */
  private[sbt] def fullLink(directory: Def.Initialize[File]): Def.Initialize[Task[File]] = Def.task {
    val description = describe.value
    val log = streams.value.log
    val out = directory.value
    val identity = s"${binaryVersion(teqResolvedBinary.value)} full"
    Directory.writing(out, teqLinkWait.value, log) { _ =>
      Directory.takeOver(out, Some(identity), log)
      IO.createDirectory(out)
      val written = new File(out.getParentFile, out.getName + ".main.js.tmp")
      val full = description.copy(out = relativeTo(description.root.toPath, written), hot = false, release = true)
      runTeq(full.teq, full.fileArgs(production = true), ArgsFile.into(baseDirectory.value, "compile", "full", out), full.root, log)
      java.nio.file.Files.move(written.toPath, (out / "main.js").toPath, java.nio.file.StandardCopyOption.ATOMIC_MOVE, java.nio.file.StandardCopyOption.REPLACE_EXISTING)
      Directory.mark(out, identity)
    }
    out
  }

  /** The link of a Scala.js project's `Test` configuration (its `fastLinkJS` under
    * `teqCompiler`): the split build of the test sources with the main ones and the test class
    * path, whose entry point starts Scala.js's test bridge (`TestEntry`; the bridge is in the
    * std, as the test interface the frameworks' jars name is), by the directory's resident `teq
    * watch`, without `--hot`, and a `main.js` that re-exports `main.mjs`, which sbt-scalajs's
    * test adapter imports under node once it has set the com channel up. */
  private[sbt] def testLink(directory: Def.Initialize[File]): Def.Initialize[Task[File]] = Def.task {
    val description = describeTest.value
    val log = streams.value.log
    val out = directory.value
    val life = Resident.Life(teqLinkIdle.value, Watching.of(state.value).map(channel => () => Watching.underWay(channel)))
    val identity = s"${binaryVersion(teqResolvedBinary.value)} test"
    Directory.writing(out, teqLinkWait.value, log) { _ =>
      Directory.takeOver(out, Some(identity), log)
      IO.createDirectory(out)
      val test = description.copy(out = relativeTo(description.root.toPath, out), hot = false)
      linkBuild(out, test.teq +: test.watchArgs, ArgsFile.into(baseDirectory.value, "test", "link", out), test.root, log, life)
      val stub = out / "main.js"
      if !stub.exists || IO.read(stub) != TestStub then IO.write(stub, TestStub)
      Directory.mark(out, identity)
    }
    out
  }

  /** The full link of the `Test` configuration (its `fullLinkJS` under `teqCompiler`): the
    * `--release` build of the same program as one file, `main.js`. */
  private[sbt] def testFullLink(directory: Def.Initialize[File]): Def.Initialize[Task[File]] = Def.task {
    val description = describeTest.value
    val log = streams.value.log
    val out = directory.value
    val identity = s"${binaryVersion(teqResolvedBinary.value)} test full"
    Directory.writing(out, teqLinkWait.value, log) { _ =>
      Directory.takeOver(out, Some(identity), log)
      IO.createDirectory(out)
      val written = new File(out.getParentFile, out.getName + ".main.js.tmp")
      val full = description.copy(out = relativeTo(description.root.toPath, written), hot = false, release = true)
      runTeq(full.teq, full.fileArgs(production = false), ArgsFile.into(baseDirectory.value, "test", "full", out), full.root, log)
      java.nio.file.Files.move(written.toPath, (out / "main.js").toPath, java.nio.file.StandardCopyOption.ATOMIC_MOVE, java.nio.file.StandardCopyOption.REPLACE_EXISTING)
      Directory.mark(out, identity)
    }
    out
  }

  private[sbt] val TestStub = "export * from \"./main.mjs\";\n"

  /** The entry point of a test link: the start of Scala.js's test bridge, which the test
    * adapter's module initializer names (`org.scalajs.testing.bridge.Bridge.start`), in a
    * package of the plugin's, since a program with a file in a Scala.js package keeps the std's
    * facades out. */
  private val TestEntry = "dev.teq.sbt.TestMain"
  private val TestEntrySource =
    "package dev.teq.sbt\n\nobject TestMain:\n  def main(args: Array[String]): Unit = org.scalajs.testing.bridge.Bridge.start()\n"

  /** The description of a Scala.js project's `Test` configuration: the roots of `Test` and of
    * what it extends, of the projects it depends on in it, dependencies first, with
    * `teqExtraSources` and `teqLib` and the root of the entry point, the Scala 3 jars of its
    * dependency classpath, its scalac options, and the entry point. */
  private val describeTest: Def.Initialize[Task[Description]] = Def.taskDyn {
    val pairs = configDependencies(buildDependencies.value, thisProjectRef.value, Test)
    Def.task {
      generatedSourcesOf(pairs).value
      val description = describe.value
      val root = description.root.toPath
      val entryRoot = baseDirectory.value / "target" / "teq" / "test-entry"
      val entry = entryRoot / "dev" / "teq" / "sbt" / "TestMain.scala"
      if !entry.exists || IO.read(entry) != TestEntrySource then IO.write(entry, TestEntrySource)
      val roots = (sourceDirectories(pairs).value ++ teqExtraSources.value).filter(_.isDirectory).distinct :+ entryRoot
      val converter = fileConverter.value
      val jars = (Test / externalDependencyClasspath).value.map(entry => converter.toPath(entry.data).toFile).filter(isTastyJar(platform.value))
      description.copy(
        sources = roots.map(relativeTo(root, _)),
        classpath = jars.map(_.getAbsolutePath),
        options = ScalacOptions((Test / scalacOptions).value),
        small = Nil,
        main = Some(TestEntry),
      )
    }
  }

  /** What a watch of a link task needs, scoped to the task: its triggers (`when` the task is
    * teq's), and the end of its resident when the watch ends, ahead of the hook the build has
    * for the project, which runs as it would without this one. The hook is sbt's for the
    * watched task alone: the resident of a link that a watch of another task asked for ends
    * by its idle bound, once that watch is over (`Resident.Life`). */
  private[sbt] def watched(task: TaskKey[?], directory: Def.Initialize[File], when: Def.Initialize[Boolean]): Seq[Setting[?]] = Seq(
    task / watchTriggers ++= Def.settingDyn(if when.value then linkTriggers else Def.setting(Seq.empty[Glob])).value,
    task / watchOnTermination := {
      val outer = watchOnTermination.?.value.getOrElse(Watching.defaultOnTermination)
      val out = directory.value
      val teq = when.value
      (action, command, count, state) =>
        if teq then Resident.stop(out)
        outer(action, command, count, state)
    },
  )

  /** What retriggers a watched link: every source root (the Compile source directories of the
    * project and of the projects it depends on, `teqExtraSources` and `teqLib`) and every file
    * input a task of theirs declares, a source generator's among them. sbt's watch takes a
    * task's triggers in the place of the file inputs its dependencies bring
    * (`WatchTransitiveDependencies`), so with any trigger in the build, an application's own
    * included, the inputs have to be triggers to count. */
  private val linkTriggers: Def.Initialize[Seq[Glob]] = Def.setting {
    val roots = unmanagedSourceDirectories.all(ScopeFilter(inDependencies(ThisProject), inConfigurations(Compile))).value.flatten ++
      teqExtraSources.value ++ teqLib.value.toSeq
    val inputs = fileInputs.?.all(ScopeFilter(inDependencies(ThisProject), inConfigurations(Compile) || inZeroConfiguration, inAnyTask)).value.flatten.flatten
    (roots.distinct.flatMap(dir => Seq(dir.toGlob / ** / "*.scala", dir.toGlob / ** / "*.java")) ++ inputs).distinct
  }

  /** teq as the configuration's compiler under `teqCompiler` (docs/TARGETS.md, "The module
    * model"): zinc's own incremental compile, with teq as its Scala compiler
    * (`TeqCompile.Compiler`, put into `compileInputs`' compilers) over the sources zinc
    * invalidated, a JVM build of class files and TASTy, or for a Scala.js project a check of
    * TASTy whose class products are stamps (its links and tests are teq's, `TeqScalaJSPlugin`),
    * each source against the configuration's own class directory and the upstream ones. The
    * run's rollback is zinc's class-file manager with the plugin's journal of what teq published
    * (`TeqCompile.Journal`), and after the run the manifest of the products drops the sources the
    * configuration no longer has (`TeqCompile.reconcile`), which zinc calls no compiler for. The
    * setup's `extra` and the compile cache's key carry teq's identity (the binary's digest, what
    * it builds, the flags that change its output), so that a change of any of them compiles
    * everything again; the class directory's marker is checked before either compiler writes
    * into it. zinc's own compiler stays for `doc` and `console`, and a configuration with Java
    * sources is refused. Added to `Compile` and `Test`; a configuration of the project's own
    * (`IntegrationTest`) takes them with `inConfig(IntegrationTest)(TeqPlugin.compilerSettings)`. */
  def compilerSettings: Seq[Setting[?]] = Seq(
    teqCompilerCommand := compilerCommand.value,
    compileIncremental := Def.uncached(Def.taskIf {
      if teqCompiler.value then
        val result = compileIncremental.value
        TeqCompile.reconcile(teqCompilerCommand.value, streams.value.log)
        result
      else compileIncremental.value
    }.value),
    compile / compileInputs := Def.uncached(Def.taskIf {
      if teqCompiler.value then
        val inputs = (compile / compileInputs).value
        val cmd = teqCompilerCommand.value
        TeqCompile.refuseJava(cmd.sources)
        TeqCompile.Marker.check(classDirectory.value, compileAnalysisFile.value, teq = true, streams.value.log)
        val journal = new TeqCompile.Journal(cmd.classes)
        val compilers = inputs.compilers.withScalac(new TeqCompile.Compiler(cmd, inputs.compilers.scalac, journal))
        val setup = inputs.setup
          .withIncrementalCompilerOptions(TeqCompile.withJournal(inputs.setup.incrementalCompilerOptions, journal))
          .withExtra(inputs.setup.extra :+ pair("teq", cmd.identity))
        inputs.withCompilers(compilers).withSetup(setup)
      else
        val inputs = (compile / compileInputs).value
        TeqCompile.Marker.check(classDirectory.value, compileAnalysisFile.value, teq = false, streams.value.log)
        inputs
    }.value),
    compile / compileInputs2 := Def.uncached(Def.taskIf {
      if teqCompiler.value then
        val inputs = (compile / compileInputs2).value
        inputs.copy(incrementalOptions = inputs.incrementalOptions :+ ("teq" -> teqCompilerCommand.value.identity))
      else (compile / compileInputs2).value
    }.value),
  )

  /** The configuration's compile by teq: the binary, the build's root, the class directory, a
    * JVM build or a Scala.js project's check (`teqTarget`), the flags of `teqThreads`,
    * `teqCacheableState` and `teqMacroState` and the scalac options mapped as `teqScalacOptions`
    * maps them, the configuration's sources and class path, and its identity: what of these
    * changes the products, the worker count left out. */
  private val compilerCommand: Def.Initialize[Task[TeqCompile.Command]] = Def.task {
    val log = streams.value.log
    val root = (LocalRootProject / baseDirectory).value.toPath.toAbsolutePath.normalize.toFile
    val jvm = teqTarget.value == Jvm
    val options = ScalacOptions(scalacOptions.value)
    if options.ignored.nonEmpty then log.debug(s"teq: scalac options without a teq flag: ${options.ignored.mkString(" ")}")
    val binary = teqResolvedBinary.value
    val teq = if binary.getParentFile == null then binary.getName else binary.getAbsolutePath
    val flags = teqCacheableState.value.flatMap(Seq("--cacheable-state", _)) ++ macroStateFlags(teqMacroState.value) ++ options.flags(jvm)
    val threads = teqThreads.value.toSeq.flatMap(n => Seq("--threads", n.toString))
    val converter = fileConverter.value
    val classpath = dependencyClasspath.value.map(entry => converter.toPath(entry.data).toFile)
    val identity = (Seq(binaryDigest(binary), if jvm then "jvm" else "check") ++ flags).mkString(" ")
    val argsPlace = ArgsFile.of(baseDirectory.value, configuration.value.name, "batch")
    TeqCompile.Command(teq, root, classDirectory.value, check = !jvm, platform.value, threads ++ flags, sources.value, classpath, identity, argsPlace)
  }

  private def pair(key: String, value: String): xsbti.T2[String, String] = new xsbti.T2[String, String]:
    def get1(): String = key
    def get2(): String = value

  private val digests = new java.util.concurrent.ConcurrentHashMap[(String, Long, Long), String]()

  /** The SHA-256 of the binary, once per binary (path, size, modification time), a bare name's as
    * the PATH resolves it; one found nowhere is named by what it prints to `--version`. */
  private def binaryDigest(named: File): String =
    val binary =
      if named.getParentFile != null then named
      else sys.env.getOrElse("PATH", "").split(File.pathSeparator).iterator.map(new File(_, named.getName)).find(f => f.isFile && f.canExecute).getOrElse(named)
    if !binary.isFile then binaryVersion(named)
    else
      digests.computeIfAbsent((binary.getPath, binary.length, binary.lastModified), _ => {
        val digest = java.security.MessageDigest.getInstance("SHA-256")
        val in = new java.io.FileInputStream(binary)
        try
          val buffer = new Array[Byte](1 << 16)
          var n = in.read(buffer)
          while n > 0 do
            digest.update(buffer, 0, n)
            n = in.read(buffer)
        finally in.close()
        digest.digest().map(b => f"${b & 0xff}%02x").mkString
      })

  private val versions = new java.util.concurrent.ConcurrentHashMap[(String, Long, Long), String]()

  /** What the binary prints to `--version`, once per binary (path, size, modification time). */
  private def binaryVersion(binary: File): String =
    val key = (binary.getPath, binary.length, binary.lastModified)
    versions.computeIfAbsent(key, _ => try Process(Seq(binary.getPath, "--version")).!!.trim catch case _: Exception => "unknown")

  /** `scalajs:main.js` (the Scala.js vite plugin's own convention) resolves into this one-line
    * module, which re-exports teq's own `main.mjs`: teq's naming (`docs/TARGETS.md`, "Module
    * splitting") stays the one entry every tool reads, from a plain `node out/main.mjs` to
    * `vite-plugin-teq`'s `scalajs:main.js`, and the served directory adds the `.js` alias a
    * Scala.js-style plugin expects instead of teq gaining a naming flag of its own for one
    * consumer. teq's own directory bookkeeping (`docs/TARGETS.md`, "Module splitting": stale
    * modules removed) only touches `.mjs` files, so this file is never disturbed by a rebuild. */
  private def writeDevStub(out: File): Unit =
    val stub = out / "main.js"
    if !stub.exists || IO.read(stub) != DevStub then IO.write(stub, DevStub)

  /** The stub of a dev build carries what an application's entry would otherwise have to: the
    * refresh runtime ahead of the program, so that its module body runs before React DOM's
    * (a module's imports are evaluated in their order), and the acceptance of its own hot
    * updates, where the re-execution of the chain above an edited module ends; an update that
    * failed (the server's client calls the acceptance back with nothing) is told to the
    * runtime's state on the page, which loads the page again once the build is whole. The
    * runtime and the acceptance do nothing without a dev server's `import.meta.hot`. */
  private[sbt] val DevStub =
    "import \"./hot-refresh.mjs\";\nexport * from \"./main.mjs\";\n" +
      "if (import.meta.hot) import.meta.hot.accept((updated) => { if (updated === undefined) globalThis.__teqHot?.failed_(\"main\", import.meta.url); });\n"

  /** Runs the source generators of the project and of the projects it depends on, so that the managed source
    * directories the description lists hold what scalac would compile. */
  private val generatedSources: Def.Initialize[Task[Unit]] = Def.taskDyn {
    generatedSourcesOf(configDependencies(buildDependencies.value, thisProjectRef.value, Compile))
  }

  private def generatedSourcesOf(pairs: Seq[(ProjectRef, Configuration)]): Def.Initialize[Task[Unit]] =
    pairs
      .map((ref, config) => ref / config / managedSources)
      .foldLeft(Def.task(()))((before, sources) => Def.task { before.value; sources.value; () })

  private val describe: Def.Initialize[Task[Description]] = Def.task {
    generatedSources.value
    val root = (LocalRootProject / baseDirectory).value.toPath.toAbsolutePath.normalize
    def path(file: File) = relativeTo(root, file)
    val existing = (teqSources.value ++ teqExtraSources.value).filter(_.isDirectory).distinct
    Description(
      root = root.toFile,
      teq = teqResolvedBinary.value match
        case bare if bare.getParentFile == null => bare.getName
        case binary => binary.getAbsolutePath,
      lib = teqLib.value.map(path),
      sources = existing.map(path),
      excludes = teqExcludes.value,
      classpath = teqClasspath.value.map(_.getAbsolutePath),
      jvm = Option.when(teqTarget.value == Jvm)(JvmBuild(teqMainClass.value)),
      options = ScalacOptions(teqScalacOptions.value),
      out = path(teqOutput.value),
      small = teqModulePerFile.value,
      cacheableState = teqCacheableState.value,
      macroState = teqMacroState.value,
      threads = teqThreads.value,
      hot = teqHot.value,
      release = teqRelease.value,
      productionSources = teqProductionSources.value.map(path),
      productionExcludes = teqProductionExcludes.value,
    )
  }

  /** Runs `teq` to completion, its arguments in their file at `argsPlace`, its diagnostics going
    * to sbt's log; a non-zero exit fails the task. */
  private def runTeq(teq: String, args: Seq[String], argsPlace: ArgsFile.Place, root: File, log: Logger): Unit =
    val stderr = mutable.ArrayBuffer.empty[String]
    val code =
      try Process(ArgsFile.command(teq +: args, argsPlace), root).!(ProcessLogger(line => log.info(line), line => stderr += line))
      catch case e: java.io.IOException => throw new MessageOnlyException(s"teq could not be started as $teq (teqBinary or TEQ overrides it): ${e.getMessage}")
    if code != 0 then
      stderr.foreach(line => log.error(line))
      throw new MessageOnlyException(s"teq compiler build exited with code $code")
    stderr.foreach(line => if line.contains(": warning: ") then log.warn(line) else log.info(line))

  /** A build of the resident `teq compiler watch` behind `teqLinkJS`: the answer's total and the modules
    * written go to the log, an error fails the task, with its diagnostics in the log as teq
    * prints them, and leaves the process for the next build. */
  private def linkBuild(served: File, command: Seq[String], argsPlace: ArgsFile.Place, root: File, log: Logger, life: Resident.Life): Unit =
    val line = Resident.build(served, command, argsPlace, root, log, life)
    if line.contains("\"ok\":false") then
      val errors = try Json.parse(line)("errors").items catch case _: Json.Malformed => Nil
      if errors.isEmpty then log.error(s"teq: $line")
      for error <- errors do
        val at = Seq(error("file"), error("line"), error("col")).map(_.str).filter(_.nonEmpty).mkString(":")
        log.error(s"${if at.isEmpty then "teq" else at}: error: ${error("message").str}")
        for text <- Seq(error("source").str, error("caret").str) if text.nonEmpty do log.error(text)
      throw new MessageOnlyException(s"teq: ${errors.size max 1} error${if errors.size > 1 then "s" else ""}")
    val total = "\"total\":([0-9.]+)".r.findFirstMatchIn(line).map(_.group(1))
    val changed = "\"changed\":(\\[[^\\]]*\\])".r.findFirstMatchIn(line).map(_.group(1)).getOrElse("[]")
    log.info(s"teq: ${total.fold("built")(ms => s"built in ${ms}ms")}; changed $changed")

  /** One body of `resolveBinary` at a time for a copy, across the projects' tasks and the commands of a session. */
  private val copyLocks = scala.collection.concurrent.TrieMap.empty[String, AnyRef]

  private val resolveBinary: Def.Initialize[Task[File]] = Def.taskDyn {
    if Export.isSnapshot(teqVersion.value) then resolveSnapshot else resolveRelease
  }

  /** The binary of a release from 0.1.7 on, verified (`Release.binary`) before anything runs it, then copied under
    * target/teq/bin as a resolved one is; an earlier release is refused before any request. */
  private val resolveRelease: Def.Initialize[Task[File]] = Def.task {
    val log = streams.value.log
    val (base, version, classifier) = (teqReleases.value, teqVersion.value, teqClassifier.value)
    val root = (ThisBuild / baseDirectory).value
    val lock = Seq(root / "teq.lock", root / "target" / "teq" / "teq.lock").find(_.isFile)
    val pin = lock.flatMap(Release.pinIn(_, version, classifier))
    val cache = Export.cacheRoot().getOrElse(baseDirectory.value / "target" / "teq")
    val served = Served(allCredentials.value, log)
    val verified = Release.floor(version).toLeft(()).flatMap(_ => Release.binary(base, version, classifier, cache, pin, served, log)) match
      case Right(v) => v
      case Left(why) => throw new MessageOnlyException(
        s"teq: $why. Set teqBinary (or TEQ in the environment) to a local binary, or teqVersion to a released compiler")
    val name = teqArtifact.value
    val binary = baseDirectory.value / "target" / "teq" / "bin" / s"$name-$version-$classifier${Release.executableSuffix(classifier)}"
    copyLocks.getOrElseUpdate(binary.getAbsolutePath, new AnyRef).synchronized {
      // The copy that runs, by its bytes (Release.digestOf, which no write that keeps a file's date escapes).
      if !Release.digestOf(binary).exists(_._2 == verified.sha1) then
        IO.copyFile(verified.file, binary, preserveLastModified = true)
        binary.setExecutable(true)
        val wanted = s"teq $version"
        val printed =
          try Process(Seq(binary.getAbsolutePath, "--version")).!!.trim
          catch case e: Exception => s"nothing (${e.getMessage})"
        if printed != wanted && !printed.startsWith(wanted + " ") then
          IO.delete(binary)
          IO.delete(Sha1.stamp(binary))
          throw new MessageOnlyException(s"teq: the release v$version's binary for $classifier printed $printed to --version instead of $wanted")
        Sha1.writeStamp(binary)
        log.info(s"teq: $printed from ${Release.assetUrl(base, version, classifier)}, sha256 ${verified.sha256}")
      Export.share(binary, verified.sha1, binary.getName, Export.cacheRoot(), log)
      binary
    }
  }

  /** The binary of a SNAPSHOT of the compiler published locally, as a Maven artifact through the build's resolvers. */
  private val resolveSnapshot: Def.Initialize[Task[File]] = Def.task {
    val log = streams.value.log
    val (org, name, version, classifier) = (SnapshotGroup, teqArtifact.value, teqVersion.value, teqClassifier.value)
    val coordinates = s"$org:$name:$version, classifier $classifier, type exe"
    def fail(why: String) = throw new MessageOnlyException(
      s"teq: no binary from $coordinates: $why. Set teqBinary (or TEQ in the environment) to a local binary, " +
        "or teqVersion and teqClassifier to a published one")
    val lm = dependencyResolution.value
    val module = (org % name % version).artifacts(Artifact(name, "exe", "exe", classifier))
    def failed(e: Throwable) = s"resolution failed (${Option(e.getMessage).getOrElse(e.toString).linesIterator.mkString(" ")})"
    def resolve(): Either[String, File] =
      try
        lm.update(lm.wrapDependencyInModule(module), librarymanagement.UpdateConfiguration(), librarymanagement.UnresolvedWarningConfiguration(), log) match
          case Left(warning) => Left(failed(warning.resolveException))
          case Right(report) =>
            val files = for
              conf <- report.configurations
              m <- conf.modules
              (artifact, file) <- m.artifacts
              if artifact.classifier.contains(classifier)
            yield file
            files.headOption.toRight("resolution found no file")
      catch case NonFatal(e) => Left(failed(e))
    val binary = baseDirectory.value / "target" / "teq" / "bin" / s"$name-$version-$classifier${Release.executableSuffix(classifier)}"
    copyLocks.getOrElseUpdate(binary.getAbsolutePath, new AnyRef).synchronized {
      resolve().flatMap(file => Sha1.of(file).map(digest => (file, digest)).toRight(s"$file cannot be read")) match
        case Left(why) if binary.isFile =>
          log.warn(s"teq: $why; the copy $binary serves")
          binary
        case Left(why) => fail(why)
        case Right((file, digest)) =>
          if !Sha1.stamped(binary).contains(digest) then
            IO.copyFile(file, binary, preserveLastModified = true)
            binary.setExecutable(true)
            val wanted = s"teq ${version.stripSuffix("-SNAPSHOT")}"
            val printed =
              try Process(Seq(binary.getAbsolutePath, "--version")).!!.trim
              catch case e: Exception => s"nothing (${e.getMessage})"
            if printed != wanted && !printed.startsWith(wanted + " ") then
              IO.delete(binary)
              IO.delete(Sha1.stamp(binary))
              fail(s"$file printed $printed to --version instead of $wanted")
            Sha1.writeStamp(binary)
            log.info(s"teq: $printed from $coordinates")
          Export.share(binary, digest, binary.getName, Export.cacheRoot(), log)
          binary
    }
  }

  /** The (project, configuration) pairs whose sources the configuration compiles with,
    * dependencies first: the configuration and the ones it extends (`Test` holds `Compile`),
    * and for each project depended on the configurations its mapping names for these
    * (`compile->compile;test->test`), with their own dependencies in turn. */
  private def configDependencies(deps: BuildDependencies, project: ProjectRef, config: Configuration): Seq[(ProjectRef, Configuration)] =
    val order = mutable.ArrayBuffer.empty[(ProjectRef, Configuration)]
    val seen = mutable.Set.empty[(ProjectRef, String)]
    def extended(c: Configuration): Seq[Configuration] = (c +: c.extendsConfigs.flatMap(extended)).distinct
    def visit(ref: ProjectRef, c: Configuration): Unit =
      if seen.add((ref, c.name)) then
        val own = extended(c).map(_.name).toSet
        for dep <- deps.classpath.getOrElse(ref, Nil) do
          for target <- mappedConfigs(dep.configuration, own) do
            visit(dep.project, Configuration.of(target.capitalize, target))
        for c <- extended(c).reverse do
          if !order.exists((r, x) => r == ref && x.name == c.name) then order += ((ref, c))
    visit(project, config)
    order.toSeq

  /** The configurations of a dependency that a mapping such as `compile->compile;test->test`
    * gives the configurations `from` (by default `compile` for `compile`). */
  private def mappedConfigs(mapping: Option[String], from: Set[String]): Seq[String] =
    mapping.getOrElse("compile").split(";").toSeq.flatMap { part =>
      val (sources, targets) = part.split("->") match
        case Array(s, t) => (s.split(",").map(_.trim).toSeq, t.split(",").map(_.trim).toSeq)
        case Array(s) => (s.split(",").map(_.trim).toSeq, s.split(",").map(_.trim).toSeq)
        case _ => (Nil, Nil)
      if sources.exists(from) then targets.filter(_.nonEmpty) else Nil
    }.distinct

  private def sourceDirectories(pairs: Seq[(ProjectRef, Configuration)]): Def.Initialize[Seq[File]] =
    pairs
      .map((ref, config) => (ref / config / unmanagedSourceDirectories).zipWith(ref / config / managedSourceDirectories)(_ ++ _))
      .foldLeft(Def.setting(Seq.empty[File]))((all, dirs) => all.zipWith(dirs)(_ ++ _))

  private[teq] val Jvm = "jvm"

  private val stdJars = Seq("scala-library-", "scala3-library_", "scalajs-library_", "scalajs-scalalib_")

  /** A Scala 3 library of the project's platform: `_sjs1_3-*.jar` on Scala.js, `_3-*.jar` on the JVM. */
  private def isTastyJar(platform: String)(jar: File): Boolean =
    val name = jar.getName
    val ofPlatform =
      if platform == "jvm" then name.contains("_3-") && !name.contains("_sjs") && !name.contains("_native")
      else name.contains(s"_${platform}_3-")
    name.endsWith(".jar") && ofPlatform && !stdJars.exists(name.startsWith)

  private def relativeTo(root: Path, file: File): String =
    val path = file.toPath.toAbsolutePath.normalize
    if path.startsWith(root) then root.relativize(path).toString.replace('\\', '/') match
      case "" => "."
      case relative => relative
    else path.toString

  private[sbt] final case class ScalacOptions(
    maxInlines: Option[Int],
    strictEquality: Boolean,
    kindProjector: Boolean,
    werror: Boolean,
    javaOutputVersion: Option[Int],
    ignored: Seq[String],
    wunused: Seq[String] = Nil,
    deprecation: Boolean = false,
    feature: Boolean = false,
    wtostringInterpolated: Boolean = false,
    wconf: Seq[String] = Nil,
    language: Seq[String] = Nil,
  ):
    /** Whether the unused imports are reported, as scalac's `WunusedHas.imports` reads the kinds. */
    def wunusedImports: Boolean =
      (wunused.contains("all") || wunused.contains("imports") || wunused.contains("linted")) && !wunused.contains("strict-no-implicit-warn")

    /** The teq flags the options map onto; the output version is a JVM build's alone. */
    def flags(jvm: Boolean): Seq[String] =
      maxInlines.toSeq.flatMap(n => Seq("--max-inlines", n.toString)) ++
        (if strictEquality then Seq("--strict-equality") else Nil) ++
        (if kindProjector then Seq("--kind-projector") else Nil) ++
        (if language.nonEmpty then Seq("--language", language.mkString(",")) else Nil) ++
        (if werror then Seq("--werror") else Nil) ++
        (if wunused.nonEmpty then Seq("--wunused", wunused.mkString(",")) else Nil) ++
        (if deprecation then Seq("--deprecation") else Nil) ++
        (if feature then Seq("--feature") else Nil) ++
        (if wtostringInterpolated then Seq("--wtostring-interpolated") else Nil) ++
        wconf.flatMap(rules => Seq("--wconf", rules)) ++
        (if jvm then javaOutputVersion.toSeq.flatMap(n => Seq("--java-output-version", n.toString)) else Nil)

  private[sbt] object ScalacOptions:
    private val withArgument = Set("-source", "-release", "-encoding", "-java-output-version")
    /** `-release` names the class files' version too, unless `-java-output-version` does. */
    private val outputVersion = Seq("-java-output-version", "-release")

    def apply(options: Seq[String]): ScalacOptions =
      var maxInlines = Option.empty[Int]
      var strictEquality = false
      var kindProjector = false
      var werror = false
      var deprecation = false
      var feature = false
      var wtostringInterpolated = false
      val wunused = mutable.ArrayBuffer.empty[String]
      val wconf = mutable.ArrayBuffer.empty[String]
      val language = mutable.ArrayBuffer.empty[String]
      val versions = mutable.Map.empty[String, Int]
      val ignored = mutable.ArrayBuffer.empty[String]
      var rest = options.toList
      while rest.nonEmpty do
        rest match
          case option :: n :: tail if outputVersion.contains(option) && n.toIntOption.isDefined =>
            versions(option) = n.toInt
            if option == "-release" then ignored += s"$option $n"
            rest = tail
          case option :: tail if outputVersion.exists(name => option.startsWith(name + ":")) && option.dropWhile(_ != ':').drop(1).toIntOption.isDefined =>
            val name = option.takeWhile(_ != ':')
            versions(name) = option.dropWhile(_ != ':').drop(1).toInt
            if name == "-release" then ignored += option
            rest = tail
          case "-Xmax-inlines" :: n :: tail if n.toIntOption.isDefined =>
            maxInlines = n.toIntOption
            rest = tail
          case option :: tail if option.startsWith("-Xmax-inlines:") && option.stripPrefix("-Xmax-inlines:").toIntOption.isDefined =>
            maxInlines = option.stripPrefix("-Xmax-inlines:").toIntOption
            rest = tail
          case "-Xkind-projector" :: tail =>
            kindProjector = true
            rest = tail
          case ("-Werror" | "-Xfatal-warnings") :: tail =>
            werror = true
            rest = tail
          case ("-deprecation" | "--deprecation") :: tail =>
            deprecation = true
            rest = tail
          case ("-feature" | "--feature") :: tail =>
            feature = true
            rest = tail
          case "-Wtostring-interpolated" :: tail =>
            wtostringInterpolated = true
            rest = tail
          // scalac's `-Wconf` rules, in their order: teq's `--wconf`, the rightmost deciding.
          case option :: tail if option.startsWith("-Wconf:") =>
            wconf += option.stripPrefix("-Wconf:")
            rest = tail
          // scalac's `-Wunused` with its kinds, each given adding to the ones before, which teq
          // takes as they are (`--wunused`); a bare `-Wunused` is every kind.
          case "-Wunused" :: tail =>
            wunused += "all"
            rest = tail
          case option :: tail if option.startsWith("-Wunused:") =>
            wunused ++= option.stripPrefix("-Wunused:").split(",").toSeq.map(_.trim).filter(_.nonEmpty)
            rest = tail
          // `-Wall` asks for every unused kind and `-Wtostring-interpolated` among the rest of its
          // warnings, which teq does not give.
          case "-Wall" :: tail =>
            wunused += "all"
            wtostringInterpolated = true
            ignored += "-Wall"
            rest = tail
          case option :: tail if option.startsWith("-language:") =>
            val (strict, others) = option.stripPrefix("-language:").split(",").toSeq.map(_.trim).filter(_.nonEmpty).partition(_ == "strictEquality")
            if strict.nonEmpty then strictEquality = true
            language ++= others
            rest = tail
          case option :: argument :: tail if withArgument(option) =>
            ignored += s"$option $argument"
            rest = tail
          case option :: tail =>
            ignored += option
            rest = tail
          case Nil => ()
      val javaOutputVersion = outputVersion.collectFirst { case name if versions.contains(name) => versions(name) }
      ScalacOptions(maxInlines, strictEquality, kindProjector, werror, javaOutputVersion, ignored.distinct.toSeq, wunused.distinct.toSeq, deprecation, feature, wtostringInterpolated, wconf.toSeq, language.distinct.toSeq)

  private final case class Description(
    root: File,
    teq: String,
    lib: Option[String],
    sources: Seq[String],
    excludes: Seq[String],
    classpath: Seq[String],
    jvm: Option[JvmBuild],
    options: ScalacOptions,
    out: String,
    small: Seq[String],
    cacheableState: Seq[String],
    macroState: String,
    threads: Option[Int],
    hot: Boolean,
    release: Boolean,
    productionSources: Seq[String],
    productionExcludes: Seq[String],
    main: Option[String] = None,
  ):
    /** The arguments of `teq compiler build`, from `root`. */
    def buildArgs(production: Boolean): Seq[String] =
      val inputs = lib.toSeq ++ sources ++ (if production then productionSources else Nil)
      val left = excludes ++ (if production then productionExcludes else Nil)
      val output = jvm match
        case Some(build) =>
          Seq("--target", Jvm, "--std=scala-library", "-o", out) ++ build.mainClass.toSeq.flatMap(Seq("--main", _))
        case None =>
          Seq("--split", out) ++
            (if small.nonEmpty then Seq("--module-per-file", small.mkString(",")) else Nil) ++
            (if hot then Seq("--hot") else Nil) ++
            (if production then Seq("--release") else Nil)
      Seq("compiler", "build") ++ inputs ++ left.flatMap(Seq("--exclude", _)) ++
        (if classpath.nonEmpty then Seq("--classpath", classpath.mkString(File.pathSeparator)) else Nil) ++
        output ++ cacheableState.flatMap(name => Seq("--cacheable-state", name)) ++ macroStateFlags(macroState) ++ threadsFlags ++ options.flags(jvm.isDefined) ++ Seq("--time")

    /** The arguments of `teq compiler build` into the one file `out`, as vite-plugin-teq builds it for
      * `vite build`. */
    def fileArgs(production: Boolean): Seq[String] =
      if jvm.isDefined then throw new MessageOnlyException("teqFullLinkJS builds JavaScript; this project's teqTarget is jvm")
      val inputs = lib.toSeq ++ sources ++ (if production then productionSources else Nil)
      val left = excludes ++ (if production then productionExcludes else Nil)
      Seq("compiler", "build") ++ inputs ++ left.flatMap(Seq("--exclude", _)) ++
        (if classpath.nonEmpty then Seq("--classpath", classpath.mkString(File.pathSeparator)) else Nil) ++
        cacheableState.flatMap(name => Seq("--cacheable-state", name)) ++ macroStateFlags(macroState) ++ main.toSeq.flatMap(Seq("--main", _)) ++
        threadsFlags ++ options.flags(jvm = false) ++ Seq("-o", out) ++ (if production || release then Seq("--release") else Nil) ++ Seq("--time")

    /** `--threads` for `teqThreads`, given to every build of the description, the resident's too. */
    private def threadsFlags: Seq[String] = threads.toSeq.flatMap(n => Seq("--threads", n.toString))

    /** The arguments of the resident `teq compiler watch` behind `teqLinkJS`: the same dev shape as
      * `buildArgs(production = false)`, without `--time` (the JSON reply already carries the
      * timings) and without a JVM build, which has no Scala.js-style dev loop to serve; `--hot`
      * where the description asks for it, the entry point it names. */
    def watchArgs: Seq[String] =
      if jvm.isDefined then throw new MessageOnlyException("teqLinkJS builds JavaScript; this project's teqTarget is jvm")
      val inputs = lib.toSeq ++ sources
      Seq("compiler", "watch") ++ inputs ++ excludes.flatMap(Seq("--exclude", _)) ++
        (if classpath.nonEmpty then Seq("--classpath", classpath.mkString(File.pathSeparator)) else Nil) ++
        Seq("--split", out) ++
        (if small.nonEmpty then Seq("--module-per-file", small.mkString(",")) else Nil) ++
        cacheableState.flatMap(name => Seq("--cacheable-state", name)) ++ macroStateFlags(macroState) ++ main.toSeq.flatMap(Seq("--main", _)) ++
        threadsFlags ++ (if hot then Seq("--hot") else Nil) ++ options.flags(jvm = false)

  /** `--macro-state` for `teqMacroState`: nothing for the default, `ordered`. */
  private def macroStateFlags(state: String): Seq[String] = state match
    case "ordered" => Nil
    case "per-worker" => Seq("--macro-state", "per-worker")
    case other => throw new MessageOnlyException(s"teqMacroState is ordered or per-worker, not $other")

  /** A JVM build: class files in `out`, linked against the jars (`--std=scala-library`). */
  private final case class JvmBuild(mainClass: Option[String])
