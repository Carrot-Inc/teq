package dev.teq.mill

import java.io.File
import java.nio.file.Path

import scala.collection.mutable

import coursier.core as cs
import mill.*
import mill.api.{Evaluator, ExecResult, SelectMode}
import mill.javalib.{JavaModule, TestModule}
import mill.scalajslib.ScalaJSModule
import mill.scalajslib.api.{JsEnvConfig, ModuleKind}
import mill.scalalib.ScalaModule
import mill.scalanativelib.ScalaNativeModule

import dev.teq.sbt.{Exported, Json, Launchers, Lock, Release, Served, Sha1, TeqCommand}
import Exported.{Inputs as _, *}

/** `teqExportAll` of a Mill build (docs/TOOLING.md, "The export of a Mill build"): `teq.lock`, written from
  * what Mill resolves of the build's modules, compiling nothing and running none of the build's tasks that
  * make files. sbt-teq's export is the contract (`dev.teq.sbt.Export`): the same records, the same canonical
  * bytes (`Exported`, `Lock`), the module model mapped onto teq's projects and configurations:
  *
  *  - a project per resolved JavaModule instance, its key the module's segments as Mill renders them, every
  *    cross axis and nesting kept (`matrix[3.8.4]`, `cats.js`); a module's direct child `test`, a TestModule
  *    (or the one the overlay names), is the project's test configuration, not a project;
  *  - a Scala Native module, a Scala 2 module, a module without Scala (a Java module) or with Java sources,
  *    another TestModule, a module whose `compileClasspath`, `resolvedMvnDeps` or `resolvedRunMvnDeps` is the
  *    build's own task (told before anything of the module runs), and every module that depends on one of
  *    those, left out, each with its reason;
  *  - `compile` the module's sources and resources, its compile class path as Mill orders it
  *    (`JavaModule.compileClasspath`: the resolved artifacts, then each module of
  *    `transitiveModuleCompileModuleDeps` by its products and its local class path, then its own), marked
  *    `transitive: false`, Mill's `compileModuleDeps` being seen by their dependent alone; `runtime` its run
  *    class path (`runClasspath`: `resolvedRunMvnDeps`, `transitiveModuleRunModuleDeps`' products);
  *    `test` the test module's, compiled against its compile class path and run on its run class path
  *    (`runtimeClasspath` where they differ), in a sandbox of its own with `MILL_TEST_RESOURCE_DIR` and
  *    `MILL_WORKSPACE_ROOT` when Mill's run has them (`TestModule.testSandboxWorkingDir`, `allForkEnv`), its suites
  *    on one thread (`parallelism: 1`): Mill runs a JVM's suites one after another (`TestRunnerUtils`' queue), its
  *    `testParallelism` and `testForkGrouping` being about JVM processes, which teq's one runner per configuration
  *    does not have (grouping unsupported here), and the default grouping, over `discoveredTestClasses`, a compile
  *    the export never runs;
  *  - a jar by its coordinates, its repository and its path there, from Mill's resolution of the module's
  *    own dependencies with its own resolver settings (`Jvm.getArtifacts`), reconciled with the ordered
  *    class path Mill gives;
  *  - `generatedSources` that the build defines is never run: a command of the overlay stands for it, else it
  *    is recorded as Mill's (a generator of kind `mill`), its directory under Mill's `out/` the editor's root,
  *    and the verbs that need it refuse. A `sources` or `resources` task is read only where it reads source
  *    tasks alone, and one that returns a path under Mill's `out/` is Mill's generator too. */
private[mill] object Export:
  val FileName = Lock.FileName
  private val CentralRoot = "https://repo1.maven.org/maven2/"

  /** What the export read of a module: Mill's values, resolved, nothing compiled. */
  final case class Facts(
    scalaVersion: Option[String],
    scalacOptions: Seq[String],
    sources: Roots,
    resources: Roots,
    localCompile: Roots,
    compileJars: Seq[os.Path],
    runJars: Seq[os.Path],
    artifacts: Seq[Artifact],
    runArtifacts: Seq[Artifact],
    repositories: Seq[Repository],
    declaredVersions: Seq[String],
    mainClass: Option[String],
    forkArgs: Seq[String],
    forkEnv: Map[String, String],
    forkWorkingDir: os.Path,
    testFramework: Option[String],
    sandbox: Boolean,
    moduleKind: Option[String],
    jsEnv: Option[Json.Value],
  )

  /** What the export read of a task of paths (`sources`, `resources`, `localCompileClasspath`): its paths
    * where its definition is static (`static`); where it is a computation of the build's or a plugin's, the
    * task's name in `computed` and, in `paths`, the source tasks' paths it reads, never run. */
  final case class Roots(paths: Seq[os.Path], computed: Option[String])

  /** A resolved artifact: its coordinates, the URL it was fetched from, and the file. */
  final case class Artifact(organization: String, name: String, version: String, classifier: Option[String], url: String, file: os.Path)

  /** The export at the build's root: written, with the launchers under the overlay's `buildTool`; the file,
    * or why it was not written. */
  def run(evaluator: Evaluator, root: os.Path, log: String => Unit, warn: String => Unit): Either[String, os.Path] =
    for
      overlay <- Overlay.read(root)
      modules <- resolved(evaluator)
      lock <- lockOf(evaluator, root, overlay, modules, log, warn)
    yield
      val file = if overlay.buildTool then root / FileName else root / "target" / "teq" / FileName
      val text = Lock.canonical(lock)
      if !os.isFile(file) || os.read(file) != text then os.write.over(file, text, createFolders = true)
      if overlay.buildTool then
        log(s"teq: wrote $file")
        for warning <- Inputs.crlfWarning(root) do warn(warning)
        for report <- Launchers.write(root.toIO) do
          if report.kept then warn(s"teq: ${report.message}") else log(s"teq: ${report.message}")
      else
        log(s"teq: wrote $file, which teq, the language server and vite-plugin-teq read (\"buildTool\": true in ${Overlay.FileName} writes it at the build's root with its launchers, for the repository to commit)")
        if os.isFile(root / FileName) then
          warn(s"teq: ${root / FileName} is not this export's, and every reader takes it before $file: remove it, or set \"buildTool\": true in ${Overlay.FileName} to export there")
      file

  /** Every JavaModule instance of the build, by its segments' order. */
  private def resolved(evaluator: Evaluator): Either[String, Seq[JavaModule]] =
    evaluator.resolveModulesOrTasks(Seq("__:JavaModule"), SelectMode.Separated, resolveToModuleTasks = true) match
      case mill.api.Result.Success(found) =>
        Right(found.collect { case Left(m: JavaModule) => m }.distinct.sortBy(key)(using Json.utf8))
      case f: mill.api.Result.Failure => Left(s"the build's modules do not resolve: ${f.error}")

  /** A module's key in the lock: its segments as Mill 1.0 renders them, a label after the first led by `.`
    * and a cross module's axis values within `[` `]` (`matrix[3.8.4]`, `cats.js`), whatever the Mill that
    * runs the export renders, so that one build has one lock under every Mill. */
  def key(m: mill.api.Module): String =
    m.moduleSegments.value.zipWithIndex.map {
      case (mill.api.daemon.Segment.Label(s), 0) => s
      case (mill.api.daemon.Segment.Label(s), _) => "." + s
      case (c: mill.api.daemon.Segment.Cross, _) => c.value.mkString("[", ",", "]")
    }.mkString

  /** How Mill names a module's task, for a reason. */
  private def taskName(m: JavaModule, task: String) = s"${key(m)}.$task"

  /** The packages of Mill's own modules: a task one of their classes defines is Mill's, not the build's or a
    * plugin's (whose definitions, `Task.Named`'s `ctx.enclosing`, name their own classes). */
  private val MillLibraries = Seq("mill.javalib.", "mill.scalalib.", "mill.scalajslib.", "mill.scalanativelib.")

  /** Whether a module's task is the definition of Mill's own modules, which the export reads as Mill's
    * documentation says it computes, not one of the build's (or a plugin's) that it would have to run. */
  private def millDefines(task: Task.Named[?]): Boolean = MillLibraries.exists(task.ctx.enclosing.startsWith)

  /** Whether a task's definition is static, told before anything runs: every task it depends on, itself
    * included, a source task (`Task.Sources`, `Task.Source`: paths, hashed) or a task of Mill's own modules'
    * definitions (`millDefines`); never a computation of the build's or a plugin's, which may run a generator,
    * nor a task without a definition of its own (an anonymous one), whose definer cannot be told. */
  private def static(evaluator: Evaluator, task: Task[?]): Boolean =
    evaluator.transitiveTasks(Seq(task)).forall {
      case _: Task.Sources | _: Task.Source => true
      case _: Task.Command[?] | _: Task.Worker[?] => false
      case n: Task.Named[?] => millDefines(n)
      case _ => false
    }

  private def lockOf(evaluator: Evaluator, root: os.Path, overlay: Overlay, all: Seq[JavaModule], log: String => Unit, warn: String => Unit): Either[String, Json.Value] =
    val ids = all.map(m => key(m) -> m).toMap
    val id = (m: JavaModule) => key(m)
    // The test configurations: each module's child `test` that is a TestModule, or the module the overlay names.
    val testOf: Map[JavaModule, JavaModule] = all.flatMap { m =>
      val named = overlay.module(id(m)).test.flatMap(ids.get)
      val child = m.moduleDirectChildren.collectFirst { case t: (JavaModule & TestModule) if t.moduleSegments.last.value == "test" => t }
      named.orElse(child).map(m -> _)
    }.toMap
    val folded: Map[JavaModule, JavaModule] = testOf.map(_.swap)
    val refusals = mutable.ArrayBuffer.empty[String]
    for (owner, name) <- overlay.modules.toSeq.flatMap((o, m) => m.test.map(o -> _)) if !ids.contains(name) do
      refusals += s"${Overlay.FileName} names $name the test module of $owner, and the build has no module $name"
    for name <- overlay.modules.keys.toSeq.sorted if !ids.contains(name) do
      refusals += s"${Overlay.FileName} declares the module $name, which the build does not have"
    if Inputs.allowsNestedBuilds(root) then
      refusals += "the build allows nested build files (mill-allow-nested-build-mill), whose scripts teq's readers do not find among its inputs: build it with mill"
    // A committed lock names nothing of one machine: under the driver a declared input is the build's.
    for input <- overlay.inputs if overlay.buildTool && !(root / os.RelPath(input)).startsWith(root) do
      refusals += s"${Overlay.FileName} declares the input $input, outside the build's root, which a lock the repository commits cannot name"

    // What leaves a module out, before its facts are read.
    val reasons = mutable.LinkedHashMap.empty[JavaModule, String]
    for m <- all if !folded.contains(m) do
      m match
        case _: ScalaNativeModule => reasons(m) = "a Scala Native module, which teq does not build"
        case _: TestModule => reasons(m) = s"a test module that is no module's test configuration (a module's child `test`, or the one ${Overlay.FileName} names)"
        case s: ScalaModule => ()
        case _ => reasons(m) = "a Java module, whose sources teq does not compile"

    val described = all.filter(m => !folded.contains(m) && !reasons.contains(m))
    /** Why a module's class path tasks are no configuration's, told from their definitions before anything of
      * the module runs: its `compileClasspath`, `resolvedMvnDeps` or `resolvedRunMvnDeps` the build's own task,
      * which reading its facts would run with everything it calls. */
    def owned(m: JavaModule): Option[String] =
      Seq("compileClasspath" -> m.compileClasspath, "resolvedMvnDeps" -> m.resolvedMvnDeps, "resolvedRunMvnDeps" -> m.resolvedRunMvnDeps)
        .collectFirst { case (name, task) if !millDefines(task) => s"its $name is the build's own task, which the export does not run" }
    for m <- described; why <- owned(m) do reasons(m) = why
    // A test module held to the same, before its configuration is accepted: the project keeps its compile and
    // runtime configurations, and its test refuses with the reason, its facts never read.
    val testReasons = mutable.LinkedHashMap.empty[JavaModule, String]
    for m <- described if !reasons.contains(m); t <- testOf.get(m); why <- owned(t) do testReasons(m) = s"its test module ${id(t)}: $why"
    val read = described.filterNot(reasons.contains)
    val withTests = read ++ read.filterNot(testReasons.contains).flatMap(testOf.get)
    val facts: Map[JavaModule, Facts] = factsOf(evaluator, withTests) match
      case Left(why) => return Left(why)
      case Right(f) => f

    /** Why a module's facts are no configuration's: its Scala, its Java sources, a local class path the build
      * computes (never read as empty); none when they are. */
    def unfit(m: JavaModule): Option[String] =
      val f = facts(m)
      f.scalaVersion.filter(v => !v.startsWith("3.")).map(v => s"Scala $v, which teq does not compile (it compiles Scala 3)")
        .orElse(javaSources(f.sources.paths).map(n => s"its sources hold $n, which teq does not compile"))
        .orElse(f.localCompile.computed.map(t => s"its local class path, $t, is the build's own computation, which the export does not run"))
    for m <- read; why <- unfit(m) do reasons(m) = why
    for m <- read if !reasons.contains(m) && !testReasons.contains(m); t <- testOf.get(m); why <- unfit(t) do testReasons(m) = s"its test module ${id(t)}: $why"
    /** A module dependency as the lock names it: a test module its owner's test, left out with it. */
    def leftOut(d: JavaModule): Option[JavaModule] = folded.get(d) match
      case Some(owner) => Option.when(reasons.contains(owner) || testReasons.contains(owner))(owner)
      case None => Option.when(reasons.contains(d))(d)
    // A module that depends on one left out is left out, and a test configuration that does is refused: the
    // lock names no products it does not hold.
    var changed = true
    while changed do
      changed = false
      for m <- described if !reasons.contains(m) do
        (m.moduleDeps ++ m.compileModuleDeps ++ m.runModuleDeps).flatMap(leftOut).headOption.foreach { d =>
          reasons(m) = s"it depends on ${id(d)}, which is left out"
          changed = true
        }
        for t <- testOf.get(m) if !testReasons.contains(m) do
          (t.moduleDeps ++ t.compileModuleDeps ++ t.runModuleDeps).filterNot(_ == m).flatMap(leftOut).headOption.foreach { d =>
            testReasons(m) = s"its test module ${id(t)} depends on ${id(d)}, which is left out"
            changed = true
          }
    for (m, why) <- reasons do log(s"teq: ${id(m)} is left out of $FileName: $why")
    for (m, why) <- testReasons if !reasons.contains(m) do log(s"teq: ${id(m)}'s test configuration is left out of $FileName: $why")

    val projects = described.filterNot(reasons.contains)
    val productOf = (m: JavaModule) => folded.get(m) match
      case Some(owner) => Option.when(projects.contains(owner) && !testReasons.contains(owner))((id(owner), "test"))
      case None if projects.contains(m) => Some((id(m), "compile"))
      case None => None
    val parts = projects.map(m => part(root, overlay, m, testOf.get(m).filter(_ => !testReasons.contains(m)), testReasons.get(m), facts, productOf))
    val served = Served(Map.empty)
    val teq = overlay.teq.getOrElse(BuildInfo.compiler)
    val repositories = parts.flatMap(_.repositories).distinct
    val resolvedBinaries = binariesOf("build.teq", "teq", teq, overlay.releases.getOrElse(Release.DefaultBase),
      Seq(s"${Overlay.FileName}'s teq" -> teq, "the mill-teq plugin's version" -> BuildInfo.version), overlay.allowsSnapshots,
      s"\"snapshots\": true in ${Overlay.FileName}", repositories, served, warn)
    val binaries =
      if overlay.buildTool || resolvedBinaries.refusals.isEmpty then resolvedBinaries
      else
        for why <- resolvedBinaries.refusals do warn(s"teq: $why; $FileName names no binary")
        resolvedBinaries.copy(found = Nil, refusals = Nil)
    val (repos, repositoryId, conflicts) = shared(repositories ++ binaries.repositories)
    val (jars, twofold) = table(parts.flatMap(_.jars))
    val lock = document(Inputs.of(root, overlay.inputs), parts, repos, repositoryId, jars, binaries, Map("exporter" -> Json.Str("mill")))
    val all_refusals = refusals.toSeq ++ parts.flatMap(_.refusals) ++ binaries.refusals ++ conflicts ++ twofold ++ Lock.problems(lock)
    if all_refusals.nonEmpty then Left(s"teq: $FileName cannot be written:\n" + all_refusals.distinct.map("  " + _).mkString("\n"))
    else Right(lock)

  /** The Java files among a module's sources, counted; none when it holds none. */
  private def javaSources(roots: Seq[os.Path]): Option[String] =
    val n = roots.filter(os.exists).flatMap(r => if os.isDir(r) then os.walk(r).filter(p => os.isFile(p) && p.last.endsWith(".java")) else Seq(r).filter(_.last.endsWith(".java"))).size
    Option.when(n > 0)(if n == 1 then "a Java file" else s"$n Java files")

  /** Every module's facts, read in one evaluation. */
  private def factsOf(evaluator: Evaluator, modules: Seq[JavaModule]): Either[String, Map[JavaModule, Facts]] =
    val tasks = modules.map(m => factsTask(evaluator, m))
    evaluator.execute(tasks).values match
      case mill.api.Result.Success(values) => Right(modules.zip(values).toMap)
      case f: mill.api.Result.Failure => Left(s"teq: the build's modules could not be read: ${f.error}")

  private def paths(refs: Seq[PathRef]): Seq[os.Path] = refs.map(_.path)

  /** A task of paths read where its definition is static; else its name, with the paths of the source tasks
    * it reads (the editor's roots), the computation never run. */
  private def sourceTask(evaluator: Evaluator, m: JavaModule, name: String, task: Task[Seq[PathRef]]): Task[Roots] =
    if static(evaluator, task) then Task.Anon { Roots(paths(task()), None) }
    else
      val read: Seq[Task[Seq[PathRef]]] = evaluator.transitiveTasks(Seq(task)).collect {
        case s: Task.Sources => s
        case s: Task.Source => Task.Anon { Seq(s()) }
      }
      val all = Task.sequence(read)
      Task.Anon { Roots(all().flatten.map(_.path).distinct, Some(taskName(m, name))) }

  private def factsTask(evaluator: Evaluator, m: JavaModule): Task[Facts] =
    val scala: Task[(Option[String], Seq[String])] = m match
      case s: ScalaModule => Task.Anon { (Some(s.scalaVersion()), s.scalacOptions()) }
      case _ => Task.Anon { (None, Nil) }
    val js: Task[(Option[String], Option[Json.Value])] = m match
      case j: ScalaJSModule => Task.Anon { (Some(moduleKind(j.moduleKind())), Some(jsEnv(j.jsEnvConfig()))) }
      case _ => Task.Anon { (None, None) }
    val test: Task[(Option[String], Boolean)] = m match
      case t: TestModule => Task.Anon { (Some(t.testFramework()), t.testSandboxWorkingDir()) }
      case _ => Task.Anon { (None, false) }
    val sources = sourceTask(evaluator, m, "sources", m.sources)
    val resources = sourceTask(evaluator, m, "resources", m.resources)
    val localCompile = sourceTask(evaluator, m, "localCompileClasspath", m.localCompileClasspath)
    val artifacts = artifactsTask(m, runtime = false)
    val runArtifacts = artifactsTask(m, runtime = true)
    Task.Anon {
      val (scalaVersion, scalacOptions) = scala()
      val (kind, env) = js()
      val (framework, sandbox) = test()
      Facts(
        scalaVersion,
        scalacOptions,
        sources(),
        resources(),
        localCompile(),
        paths(m.resolvedMvnDeps()),
        paths(m.resolvedRunMvnDeps()),
        artifacts(),
        runArtifacts(),
        m.allRepositories().collect { case r: coursier.maven.MavenRepository => r }.flatMap(repository),
        m.mvnDeps().map(_.version),
        m.mainClass(),
        m.forkArgs(),
        m.forkEnv(),
        m.forkWorkingDir(),
        framework,
        sandbox,
        kind,
        env,
      )
    }

  /** A Maven repository of the build's over HTTP, as the lock names it: Maven Central `maven-central`, any
    * other by its host, with that host for the credentials a reader presents when it has them. */
  private def repository(r: coursier.maven.MavenRepository): Option[Repository] =
    val url = if r.root.endsWith("/") then r.root else r.root + "/"
    Option.when(url.startsWith("https://") || url.startsWith("http://")) {
      if url == CentralRoot then Repository("maven-central", url, None)
      else Repository(slug(java.net.URI.create(url).getHost), url, Some(java.net.URI.create(url).getHost))
    }

  /** The artifacts of a module's resolution, as `resolvedMvnDeps` (compile) or `resolvedRunMvnDeps` resolves
    * them, with the module's own repositories and resolver settings: each with its coordinates and URL. */
  @annotation.nowarn("cat=deprecation") // coursier's calls as Mill's own resolvedMvnDeps makes them
  private def artifactsTask(m: JavaModule, runtime: Boolean): Task[Seq[Artifact]] = Task.Anon {
    val usage = if runtime then cs.VariantSelector.VariantMatcher.Runtime else cs.VariantSelector.VariantMatcher.Api
    val params0 = m.resolutionParams()
    val params = params0
      .withDefaultConfiguration(if runtime then cs.Configuration.runtime else cs.Configuration.compile)
      .withDefaultVariantAttributes(cs.VariantSelector.AttributesBased(params0.defaultVariantAttributes.map(_.matchers).getOrElse(Map()) ++ Seq("org.gradle.usage" -> usage)))
    val self = m.coursierDependencyTask()
    val deps = if runtime then Seq(self.withConfiguration(cs.Configuration.runtime)) else Seq(self.withConfiguration(cs.Configuration.provided), self)
    val result = mill.util.Jvm.getArtifacts(
      m.allRepositories(),
      deps,
      mapDependencies = Some(m.mapDependencies()),
      customizer = m.resolutionCustomizer(),
      ctx = Some(Task.ctx()),
      coursierCacheCustomizer = m.coursierCacheCustomizer(),
      artifactTypes = Some(m.artifactTypes()),
      resolutionParams = params,
      checkGradleModules = m.checkGradleModules(),
      config = m.coursierConfigModule().coursierConfig(),
    ).get
    result.fullDetailedArtifacts.collect { case (dep, publication, artifact, Some(file)) =>
      Artifact(dep.module.organization.value, dep.module.name.value, dep.version, Option(publication.classifier.value).filter(_.nonEmpty), artifact.url, os.Path(file.getAbsoluteFile))
    }
  }

  private def moduleKind(k: ModuleKind): String = k match
    case ModuleKind.NoModule => "NoModule"
    case ModuleKind.CommonJSModule => "CommonJSModule"
    case ModuleKind.ESModule => "ESModule"

  /** A Scala.js module's JavaScript environment as the lock names it. */
  private def jsEnv(c: JsEnvConfig): Json.Value =
    def env(kind: String, executable: String, args: Seq[String], vars: Map[String, String]) =
      Json.Obj(Map("kind" -> Json.Str(kind), "executable" -> Json.Str(executable), "args" -> Json.Arr(args.map(Json.Str(_))), "env" -> Json.Obj(vars.map((k, v) => k -> Json.Str(v)))))
    c match
      case n: JsEnvConfig.NodeJs => env("node", n.executable, n.args, n.env)
      case d: JsEnvConfig.JsDom => env("jsdom", d.executable, d.args, d.env)
      case d: JsEnvConfig.ExoegoJsDomNodeJs => env("exoego-jsdom", d.executable, d.args, d.env)
      case other => Json.Obj(Map("kind" -> Json.Str(other.getClass.getSimpleName.stripSuffix("$").toLowerCase), "args" -> Json.Arr(Nil), "env" -> Json.Obj(Map.empty)))

  /** A project's part of the lock: its configurations, its run, dev and description blocks, and the
    * repositories and jars its class paths name. `productOf` gives a module's products as the lock names them,
    * none for one the lock does not hold. */
  private def part(root: os.Path, overlay: Overlay, m: JavaModule, test: Option[JavaModule], untested: Option[String], facts: Map[JavaModule, Facts],
      productOf: JavaModule => Option[(String, String)]): Part =
    val name = key(m)
    val f = facts(m)
    val declared = overlay.module(name)
    val refusals = mutable.ArrayBuffer.empty[String]
    val unsupported = mutable.LinkedHashMap.empty[String, mutable.ArrayBuffer[String]]
    def record(verb: String, why: String) = unsupported.getOrElseUpdate(verb, mutable.ArrayBuffer.empty) += why
    for why <- untested do record("test", why)
    val jars = mutable.ArrayBuffer.empty[Jar]
    val js = m.isInstanceOf[ScalaJSModule]
    val snapshots = overlay.allowsSnapshots
    def path(what: String, p: os.Path): String =
      if p.startsWith(root) then relativeTo(root.toNIO, p.toIO)
      else
        if overlay.buildTool then refusals += s"$name: $what $p is outside the build's root"
        absolute(p.toIO)
    def strs(items: Seq[String]) = Json.Arr(items.map(Json.Str(_)))
    def product(k: (String, String)) = Json.Obj(Map("project" -> Json.Str(k._1), "configuration" -> Json.Str(k._2)))

    /** The class path's artifacts, Mill's ordered paths, each by its coordinates where the resolution
      * accounts for it, else a file of this machine (refused under the driver). */
    def artifacts(paths: Seq[os.Path], resolved: Seq[Artifact], repositories: Seq[Repository], configuration: String): Seq[Json.Value] =
      val byFile = resolved.groupBy(_.file)
      paths.flatMap { p =>
        byFile.get(p).flatMap(_.headOption) match
          case Some(a) =>
            val k = jarKey(a.organization, a.name, a.version, a.classifier)
            if !snapshots && isSnapshot(a.version) then
              refusals += s"$name: ${a.organization}:${a.name}:${a.version} is a SNAPSHOT, which a later resolution can make another file (\"snapshots\": true in ${Overlay.FileName} allows it)"
            repositoryOf(repositories, a.url) match
              case Some(r) =>
                val sha1 = Sha1.of(p.toIO).getOrElse { refusals += s"$name: $p cannot be read"; "" }
                jars += Jar(k, r, a.url.stripPrefix(r.url), sha1, os.size(p), s"$name/$configuration")
                Some(Json.Str(k))
              case None if overlay.buildTool =>
                refusals += s"$name: $p (${a.organization}:${a.name}:${a.version}) comes from ${a.url}, which is none of the build's Maven repositories"
                None
              case None => Some(Json.Obj(Map("file" -> Json.Str(path("the classpath entry", p)))))
          case None if overlay.buildTool =>
            refusals += s"$name: the $configuration class path holds $p, which the resolution of ${name}'s dependencies does not account for"
            None
          case None => Some(Json.Obj(Map("file" -> Json.Str(path("the classpath entry", p)))))
      }

    /** The local class path's files of a module (its compile resources and unmanaged class path that
      * exist), named by the lock. */
    def local(d: JavaModule): Seq[Json.Value] =
      facts(d).localCompile.paths.filter(os.exists).map(p => Json.Obj(Map("file" -> Json.Str(path("the classpath entry", p)))))

    /** The module dependencies' part of a class path: each one's local files, then its products. */
    def modules(of: JavaModule, deps: Seq[JavaModule]): Seq[Json.Value] =
      deps.filterNot(_ == of).flatMap { d =>
        productOf(d) match
          case Some(k) => local(d) :+ product(k)
          case None =>
            refusals += s"$name: its class path holds ${key(d)}'s products, which the lock does not hold"
            Nil
      }

    def flags(options: Seq[String]): (Json.Value, Option[Int]) = flagsOf(options, js)

    def roots(what: String, r: Roots): Seq[String] = r.paths.map(path(what, _))
    val outDir = root / "out"
    /** A root under Mill's `out/`: what a task of the build wrote, Mill's generator's output. */
    def generated(r: Roots): Seq[os.Path] = r.paths.filter(_.startsWith(outDir))
    /** Where Mill writes a module's `generatedSources`, `out/<module>/generatedSources.dest`: the editor's root
      * while the verbs refuse, whatever Mill last generated there. Never the task's cached paths, which a
      * checkout has only once Mill ran it: the lock is the same whatever `out/` holds. */
    def lastGenerated(d: JavaModule): Seq[String] =
      Seq(path("Mill's generated sources", outDir / d.moduleSegments.parts / "generatedSources.dest"))
    /** A configuration's generators: the overlay's commands for its `generatedSources`, else Mill's when the build
      * defines it; Mill's for a sources task the export does not read or that returns a task's output. */
    def generators(d: JavaModule, commands: Seq[TeqCommand], output: String): (Seq[Json.Value], Seq[String]) =
      val df = facts(d)
      val own = !millDefines(d.generatedSources)
      val (gens, roots) =
        if own && commands.nonEmpty then (commands.map(commandJson(root.toNIO, _, output, name, refusals, overlay.buildTool)), Seq(output))
        else if own then (Seq(Json.Obj(Map("kind" -> Json.Str("mill"), "task" -> Json.Str(taskName(d, "generatedSources")),
          "reason" -> Json.Str("the build defines generatedSources, which the export does not run")))), lastGenerated(d))
        else
          if commands.nonEmpty then refusals += s"${Overlay.FileName} declares generators of ${key(d)}, whose generatedSources is Mill's own: declare none"
          (Nil, Nil)
      val unread = df.sources.computed.map(t => Json.Obj(Map("kind" -> Json.Str("mill"), "task" -> Json.Str(t), "reason" -> Json.Str("the build computes its sources, which the export does not run"))))
      val out = generated(df.sources).map(p => Json.Obj(Map("kind" -> Json.Str("mill"), "task" -> Json.Str(taskName(d, "sources")), "reason" -> Json.Str(s"it returns ${path("a source root", p)}, a task's output"))))
      (gens ++ unread ++ out, roots)
    def resourceGenerators(d: JavaModule): Seq[Json.Value] =
      val df = facts(d)
      df.resources.computed.map(t => Json.Obj(Map("kind" -> Json.Str("mill"), "task" -> Json.Str(t), "reason" -> Json.Str("the build computes its resources, which the export does not run")))).toSeq ++
        generated(df.resources).map(_ => Json.Obj(Map("kind" -> Json.Str("mill"), "task" -> Json.Str(taskName(d, "resources")))))
    def generatorFields(gens: Seq[Json.Value], res: Seq[Json.Value]) =
      Option.when(gens.nonEmpty)("generators" -> Json.Arr(gens)) ++ Option.when(res.nonEmpty)("resourceGenerators" -> Json.Arr(res))

    for v <- f.declaredVersions if !snapshots && isDynamic(v) do
      refusals += s"$name: the dependency version $v is dynamic, which a later resolution can make another (\"snapshots\": true in ${Overlay.FileName} allows it)"

    val (compileFlags, compileVersion) = flags(f.scalacOptions)
    val (compileGenerators, generatedRoots) = generators(m, declared.generators, generatedDirectory(name))
    val sourceRoots = (roots("the source directory", f.sources).filterNot(r => generated(f.sources).map(path("", _)).contains(r)) ++
      generated(f.sources).map(path("a source root", _)) ++ generatedRoots).distinct
    val mainClasses = (f.mainClass.toSeq ++ declared.runAliases.values ++ declared.mainClasses).distinct.sorted(using Json.utf8)
    val compile = Json.Obj(Map(
      "sources" -> strs(sourceRoots),
      "resources" -> strs(roots("the resource directory", f.resources)),
      "classpath" -> Json.Arr(artifacts(f.compileJars, f.artifacts, f.repositories, "compile") ++ modules(m, m.transitiveModuleCompileModuleDeps) ++ local(m)),
      "flags" -> compileFlags,
      "generators" -> Json.Arr(compileGenerators),
      "mainClasses" -> strs(mainClasses),
      "transitive" -> Json.Bool(false),
    ) ++ generatorFields(Nil, resourceGenerators(m)))
    val runtime = Json.Obj(Map(
      "sources" -> strs(Nil),
      "resources" -> strs(Nil),
      "classpath" -> Json.Arr(artifacts(f.runJars, f.runArtifacts, f.repositories, "runtime") ++ modules(m, m.transitiveModuleRunModuleDeps) ++ local(m) ++ productOf(m).map(product)),
    ))
    val (testConfiguration, testVersion) = test match
      case None => (None, None)
      case Some(t) =>
        val tf = facts(t)
        val (testFlags, version) = flags(tf.scalacOptions)
        val (testGenerators, testGenerated) = generators(t, Nil, s"target/teq/$name/test/src_managed")
        val compileClasspath = artifacts(tf.compileJars, tf.artifacts, tf.repositories, "test") ++ modules(t, t.transitiveModuleCompileModuleDeps) ++ local(t)
        val runClasspath = artifacts(tf.runJars, tf.runArtifacts, tf.repositories, "test") ++ modules(t, t.transitiveModuleRunModuleDeps) ++ local(t)
        val resources = roots("the resource directory", tf.resources)
        val js = t match
          case _: ScalaJSModule => tf.moduleKind.map("moduleKind" -> Json.Str(_)).toSeq ++ tf.jsEnv.map("jsEnv" -> _)
          case _ => Seq("parallelism" -> Json.Num("1"))
        val conf = Json.Obj(Map(
          "sources" -> strs((roots("the source directory", tf.sources) ++ testGenerated).distinct),
          "resources" -> strs(resources),
          "classpath" -> Json.Arr(compileClasspath),
          "flags" -> testFlags,
          "fork" -> Json.Bool(true),
          "baseDirectory" -> Json.Str(if tf.sandbox then "." else path("forkWorkingDir", tf.forkWorkingDir)),
          "envVars" -> Json.Obj(tf.forkEnv.map((k, v) => k -> Json.Str(v))),
          "envPaths" -> Json.Obj(Map("MILL_WORKSPACE_ROOT" -> strs(Seq(".")), "MILL_TEST_RESOURCE_DIR" -> strs(resources))),
          "javaOptions" -> strs(tf.forkArgs),
          "frameworks" -> Json.Arr(tf.testFramework.toSeq.map(c => Json.Obj(Map("class" -> Json.Str(c), "arguments" -> strs(Nil))))),
          "exclude" -> strs(Nil),
          "sandbox" -> Json.Bool(tf.sandbox),
          "transitive" -> Json.Bool(false),
        ) ++ Option.when(runClasspath != compileClasspath)("runtimeClasspath" -> Json.Arr(runClasspath)) ++ js ++
          generatorFields(testGenerators, resourceGenerators(t)))
        (Some(conf), version)
    val run = Option.when(!js)("run" -> Json.Obj(Map(
      "baseDirectory" -> Json.Str(path("forkWorkingDir", f.forkWorkingDir)),
      "envVars" -> Json.Obj(f.forkEnv.map((k, v) => k -> Json.Str(v))),
      "envPaths" -> Json.Obj(Map("MILL_WORKSPACE_ROOT" -> strs(Seq(".")))),
      "javaOptions" -> strs(f.forkArgs),
      "aliases" -> Json.Obj(declared.runAliases.map((k, v) => k -> Json.Str(v))),
    ) ++ f.mainClass.map(c => "mainClass" -> Json.Str(c))))
    val dev = (if js then devOf(root.toNIO, m.moduleDir.toIO, declared.devCommand) else None).map("dev" -> _)
    val description = Json.Obj(Map(
      "excludes" -> strs(Nil),
      "modulePerFile" -> strs(Nil),
      "cacheableState" -> strs(Nil),
      "macroState" -> Json.Str("ordered"),
      "hot" -> Json.Bool(false),
      "release" -> Json.Bool(false),
      "keys" -> Json.Obj(Map.empty),
      "out" -> Json.Str(s"target/teq/$name/out"),
      "production" -> Json.Obj(Map("sources" -> strs(Nil), "excludes" -> strs(Nil))),
    ))
    Part(
      name,
      Json.Obj(Map(
        "base" -> Json.Str(path("the module's directory", m.moduleDir)),
        "platform" -> Json.Str(if js then "js" else "jvm"),
        "scalaVersion" -> Json.Str(f.scalaVersion.getOrElse("")),
        "configurations" -> Json.Obj(Map("compile" -> compile, "runtime" -> runtime) ++ testConfiguration.map("test" -> _)),
        "description" -> description,
      ) ++ run ++ dev ++ Option.when(unsupported.nonEmpty)("unsupported" -> Json.Obj(unsupported.map((verb, whys) => verb -> strs(whys.distinct.toSeq)).toMap))),
      f.repositories ++ test.toSeq.flatMap(t => facts(t).repositories),
      jars.toSeq,
      (compileVersion ++ testVersion).toSeq,
      refusals.toSeq,
    )
