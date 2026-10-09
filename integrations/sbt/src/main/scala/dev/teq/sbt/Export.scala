package dev.teq.sbt

import java.io.File
import java.net.URI
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, LinkOption, Path, StandardCopyOption}
import java.security.MessageDigest
import java.util.zip.ZipFile
import scala.collection.mutable
import scala.sys.process.{Process, ProcessLogger}
import scala.util.control.NonFatal

import sbt.*
import sbt.Keys.*
import sbt.internal.teq.InterDependencies
import sbt.librarymanagement.{DependencyResolution, MavenRepository, UnresolvedWarningConfiguration, UpdateConfiguration}
import sbt.nio.Keys.{allInputFiles, fileInputs}
import sbt.nio.file.Glob
import sbt.util.Logger

import TeqPlugin.autoImport.*

/** A source generator that is a program the repository holds: run from `cwd`, a directory under
  * the build's root, with `run`'s arguments and the directory to write into appended, whenever
  * sbt's compile asks for the configuration's sources. The first word `teq` is the build's own
  * binary (`teqResolvedBinary`; under `teq`, the one running), so that `teq interp` runs a script
  * with no other tool. `inputs` are globs under the build's root whose change has a watch run it
  * again; `teq` runs it again when they or the files its arguments name change, or when one of
  * `outputs` is missing: files or directories under the build's root the program writes besides
  * the directory appended (a module of the application's assets, say). */
final case class TeqCommand(run: Seq[String], inputs: Seq[String] = Nil, cwd: String = ".", outputs: Seq[String] = Nil):
  /** Whether the build's own binary runs it. */
  def runsTeq: Boolean = run.headOption.contains("teq")

/** `teqExportAll`: `teq.lock` (docs/TARGETS.md, "The export and the project verbs"), the one
  * description of the build for the tools that work without sbt: under `target/teq/`, or at the
  * build's root with the launchers beside it under `teqBuildTool`. What `teq` cannot reproduce
  * is recorded on the project and block it concerns, for its verbs to refuse at use; what no
  * reader could use is refused. */
private[sbt] object Export:
  val FileName = Lock.FileName
  val Classifiers = Seq("osx-aarch_64", "osx-x86_64", "linux-x86_64", "linux-aarch_64", "windows-x86_64")
  private val Configurations = Set("compile", "runtime", "test")
  private val MavenCentral = "maven-central"
  private val MavenCentralRoot = "https://repo1.maven.org/maven2/"
  /** The class files' version teq writes where no option names one. */
  private val DefaultJavaOutputVersion = 17

  @transient val teqExportPart = taskKey[Part]("This project's part of teq.lock")
  @transient val teqExportBinaries = taskKey[Binaries]("The teq binary of every classifier the build's repositories serve, for teq.lock")

  final case class Repository(id: String, url: String, credentials: Option[String]):
    def json: Json.Value = Json.Obj(Map("id" -> Json.Str(id), "url" -> Json.Str(url)) ++ credentials.map(host => "credentials" -> Json.Str(host)))

  /** A record of the jar table under its key, and the configuration whose classpath named it; its
    * repository by its URL and credentials host, whatever the project named it. */
  final case class Jar(key: String, repository: Repository, path: String, sha1: String, size: Long, where: String):
    def record: (String, Option[String], String, String, Long) = (repository.url, repository.credentials, path, sha1, size)
    /** Its record in the lock, its fields: the repository's id, the sha1, the size, and the path
      * where it is not the Maven layout of the key. */
    def fields(id: Repository => String): Json.Value =
      Json.Str((Seq(id(repository), sha1, size.toString) ++ Option.when(mavenLayout(key).forall(_ != path))(path)).mkString(" "))
    override def toString = s"${repository.url}$path (sha1 $sha1, $size bytes) for $where"

  /** A project's part of the lock: `sources`, its description's source roots, a managed source
    * directory of a project by that project (`withSources` writes them); `bySbt`, whether its
    * compile generators are sbt's (none without generators); `skipped`, why it is left out (a
    * platform no reader takes). */
  final case class Part(name: String, json: Json.Value, repositories: Seq[Repository], jars: Seq[Jar], javaOutputVersions: Seq[Int], refusals: Seq[String], sources: Seq[Either[String, Managed]] = Nil, bySbt: Option[Boolean] = None, skipped: Option[String] = None)

  /** A project's managed source directory of Compile among a description's source roots: the
    * project's name, the directory as the lock names it (relative to the build's root; outside
    * it absolute without the driver, none under it), and whether the project has generators.
    * What it stands for is known once every project's part says whose its generators are. */
  final case class Managed(owner: String, path: Option[String], generates: Boolean)

  /** A binary of the compiler for a classifier: its URL and what the export pins of it, and the Maven repository it
    * resolves from for a SNAPSHOT published locally (none for a release's asset). */
  final case class Binary(classifier: String, url: String, sha1: String, size: Long, repository: Option[Repository] = None)

  final case class Binaries(version: String, found: Seq[Binary], refusals: Seq[String]):
    def repositories: Seq[Repository] = found.flatMap(_.repository).distinct
    /** The lock's `binaries`: per classifier the binary's fields, its URL, sha1 and size. */
    def block: Json.Value = Json.Obj(found.map(b => b.classifier -> Json.Str(s"${b.url} ${b.sha1} ${b.size}")).toMap)

  /** The settings of `teqExportAll` and of its parts: the task at the build's level, a part per
    * project, the binaries resolved by the build's first project with the plugin. */
  def buildSettings: Seq[Setting[?]] = Seq(
    teqExportAll := all.value,
    teqExportAll / aggregate := false,
  )

  def projectSettings: Seq[Setting[?]] = Seq(
    teqExportPart := part.value,
    teqExportBinaries := binaries.value,
  )

  private val all: Def.Initialize[Task[File]] = Def.taskDyn {
    val build = loadedBuild.value
    val refs = described(build, settingsData.value, buildDependencies.value)
    if refs.isEmpty then throw new MessageOnlyException("teq: no project of the build has the teq plugin")
    val filter = ScopeFilter(inProjects(refs*))
    Def.task {
      val log = streams.value.log
      val (skipped, described) = teqExportPart.all(filter).value.sortBy(_.name)(using utf8).partition(_.skipped.isDefined)
      for part <- skipped; why <- part.skipped do log.info(s"teq: ${part.name} is left out of $FileName: $why")
      val parts = withSources(described)
      val resolved = (refs.head / teqExportBinaries).value
      val root = rootOf((LocalRootProject / baseDirectory).value)
      val buildTool = Export.buildTool.value
      // Without the driver a table that cannot be filled stays empty, its reasons warnings: the
      // editor runs the compiler it has, and nothing of this machine's lock fetches one.
      val binaries =
        if buildTool || resolved.refusals.isEmpty then resolved
        else
          for why <- resolved.refusals do log.warn(s"teq: $why; $FileName names no binary")
          resolved.copy(found = Nil, refusals = Nil)
      val (repositories, id, conflicts) = shared(parts.flatMap(_.repositories) ++ binaries.repositories)
      val (jars, twofold) = table(parts.flatMap(_.jars))
      val lock = document(root, parts, repositories, id, jars, binaries)
      val refusals = parts.flatMap(_.refusals) ++ binaries.refusals ++ conflicts ++ twofold ++ Lock.problems(lock)
      if refusals.nonEmpty then
        throw new MessageOnlyException(s"teq: ${FileName} cannot be written:\n" + refusals.distinct.map("  " + _).mkString("\n"))
      val file = location(root.toFile, buildTool)
      val text = Lock.canonical(lock)
      if !file.isFile || IO.read(file, UTF_8) != text then IO.write(file, text, UTF_8)
      if buildTool then
        log.info(s"teq: wrote $file")
        // The lock the repository commits records the bytes of this checkout's build files.
        for warning <- crlfWarning(root) do log.warn(warning)
        // The launchers beside it, which run the compiler it pins.
        for report <- Launchers.write(root.toFile) do
          if report.kept then log.warn(s"teq: ${report.message}") else log.info(s"teq: ${report.message}")
      else
        log.info(s"teq: wrote $file, which teq, the language server and vite-plugin-teq read (teqBuildTool := true writes it at the build's root with its launchers, for the repository to commit)")
        val atRoot = root.toFile / FileName
        if atRoot.isFile then
          log.warn(s"teq: $atRoot is not this export's, and every reader takes it before $file: remove it, or set teqBuildTool := true to export there")
      file
    }
  }

  /** The parts with their descriptions' sources: a managed source directory of a project whose
    * generators teq runs is that project's directory of the export; of one whose generators
    * sbt runs, or of a project without a part (the plugin's, or a platform no reader takes), whose
    * generators teq never runs, the directory itself, when the lock can name it; of a project
    * without generators none. */
  private[sbt] def withSources(parts: Seq[Part]): Seq[Part] =
    val bySbt = parts.map(p => p.name -> p.bySbt).toMap
    parts.map { part =>
      val sources = part.sources.flatMap {
        case Left(root) => Some(root)
        case Right(Managed(owner, path, generates)) =>
          bySbt.get(owner) match
            case Some(Some(false)) => Some(generatedDirectory(owner))
            case Some(Some(true)) => path
            case Some(None) => None
            case None => path.filter(_ => generates)
      }.distinct
      part.json match
        case Json.Obj(fields) =>
          fields.get("description") match
            case Some(Json.Obj(d)) => part.copy(json = Json.Obj(fields.updated("description", Json.Obj(d.updated("sources", strings(sources))))))
            case _ => part
        case _ => part
    }

  /** Whether the build takes teq as its build tool: `teqBuildTool` true in the build's scope or in
    * any project's. sbt 2 makes a bare `teqBuildTool := true` of build.sbt a setting of every
    * project, the root's among them, and leaves the build's scope at the default; the lock's place is
    * the whole build's, so that a project's own `.settings(teqBuildTool := true)` takes it too. */
  val buildTool: Def.Initialize[Boolean] = Def.setting {
    (ThisBuild / teqBuildTool).value || teqBuildTool.all(ScopeFilter(inAnyProject)).value.exists(identity)
  }

  /** Where the export writes the lock: at the build's root under `teqBuildTool`, else under its
    * `target/teq/`; a reader looks at a directory's `teq.lock`, then at its `target/teq/teq.lock`,
    * the build's root being that directory either way. */
  def location(root: File, buildTool: Boolean): File =
    if buildTool then root / FileName else root / "target" / "teq" / FileName

  /** The projects the export describes: those with the plugin, but an aggregate that holds no
    * source of its own and that no project depends on, such as the build's root. */
  private def described(build: sbt.internal.LoadedBuild, data: Def.Settings, deps: sbt.internal.BuildDependencies): Seq[ProjectRef] =
    val dependedOn = deps.classpath.values.flatten.map(_.project).toSet
    def holdsSources(ref: ProjectRef) =
      Seq(Compile, Test).flatMap(c => (ref / c / unmanagedSourceDirectories).get(data).toSeq.flatten)
        .exists(dir => dir.isDirectory && (dir ** ("*.scala" || "*.java")).get().nonEmpty)
    val aggregates = build.allProjectRefs.collect { case (ref, project) if project.aggregate.nonEmpty => ref }.toSet
    TeqPlugin.projectsWithPlugin(build).filter(ref => !aggregates(ref) || dependedOn(ref) || holdsSources(ref)).sortBy(_.project)(using utf8)

  /** One repository for each URL and credentials host the projects' resolvers (and the binaries')
    * reach, by the id the first to name it gives it, the projects in the order of their names: two
    * projects' resolvers may name one repository differently. With the repositories in that order,
    * the id each declared one is written with, and a refusal for an id that names two URLs. */
  private[sbt] def shared(declared: Seq[Repository]): (Seq[Repository], Repository => String, Seq[String]) =
    val first = declared.groupBy(r => (r.url, r.credentials)).map((identity, named) => identity -> named.head)
    val repositories = declared.map(r => first((r.url, r.credentials))).distinct
    val conflicts = repositories.groupBy(_.id).toSeq.sortBy(_._1)(using utf8).collect {
      case (id, named) if named.size > 1 => s"the repository id $id names ${named.map(_.url).mkString(" and ")}: give the resolvers names of their own"
    }
    (repositories, r => first((r.url, r.credentials)).id, conflicts)

  /** The jar table, one record per key, and a refusal for every key that names two jars (the
    * same coordinates from two repositories, at two paths, or with two digests or sizes) and for
    * a path with a space, which the record's fields cannot hold. */
  private[sbt] def table(jars: Seq[Jar]): (Seq[Jar], Seq[String]) =
    val byKey = jars.groupBy(_.key).toSeq.sortBy(_._1)(using utf8)
    val refusals = byKey.flatMap { (key, named) =>
      named.distinctBy(_.record) match
        case Seq(_) => None
        case first +: second +: _ => Some(s"the jar $key is two files, $first and $second: one key names one jar")
    } ++ byKey.map(_._2.head).filter(_.path.contains(' ')).map(j => s"the jar ${j.key} is at $j, a path with a space, which $FileName's fields cannot hold")
    (byKey.map(_._2.head), refusals)

  private def document(root: Path, parts: Seq[Part], repositories: Seq[Repository], id: Repository => String, jars: Seq[Jar], binaries: Binaries): Json.Value =
    val outputVersion = (parts.flatMap(_.javaOutputVersions) :+ DefaultJavaOutputVersion).max
    val (inputFiles, inputHash) = inputs(root)
    Json.Obj(Map(
      "teq" -> Json.Str(binaries.version),
      "format" -> num(Lock.Format),
      "binaries" -> binaries.block,
      "inputs" -> Json.Obj(Map("files" -> Json.Obj(inputFiles.map((f, d) => f -> Json.Str(d)).toMap), "sha256" -> Json.Str(inputHash))),
      "repositories" -> Json.Arr(repositories.map(_.json)),
      "jars" -> Json.Obj(jars.map(j => j.key -> j.fields(id)).toMap),
      "java" -> Json.Obj(Map("outputVersion" -> num(outputVersion))),
      "projects" -> Json.Obj(parts.map(p => p.name -> p.json).toMap),
    ))

  /** The build definition files (`definitionFiles`) with the SHA-256 of their contents, in the
    * order of their paths' bytes, and the SHA-256 of `<path>\0<digest>\n` over them. */
  def inputs(root: Path): (Seq[(String, String)], String) =
    val files = definitionFiles(root).flatMap { (path, p) =>
      try Some(path -> hex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(p))))
      catch case NonFatal(_) => None
    }
    val sha = MessageDigest.getInstance("SHA-256")
    for (path, digest) <- files do sha.update(s"$path\u0000$digest\n".getBytes(UTF_8))
    (files, hex(sha.digest()))

  /** How a repository keeps the build definition files LF in every checkout, so that a lock
    * exported from one checkout is current in another, and exports again a lock recorded from CRLF;
    * teq's note on a file that differs from the lock's record by line ends alone gives the same
    * (`task::export::LF_REMEDY`). */
  val LfRemedy = ".gitattributes keeps the build's files LF in every checkout with *.sbt text eol=lf, project/**/*.scala text eol=lf and project/build.properties text eol=lf: add them, delete the files and check them out again, then export again"

  /** The warning of an export whose build definition files hold CRLF (Git for Windows' checkout
    * of text by default): the lock records their bytes, which a checkout with LF does not have;
    * none when every file is LF. Outside a Git work tree there is no checkout to correct, so it
    * names the export on the other copy's machine instead of `LfRemedy`. */
  def crlfWarning(root: Path): Option[String] =
    def holdsCrlf(p: Path) =
      try
        val bytes = Files.readAllBytes(p)
        (1 until bytes.length).exists(i => bytes(i) == '\n' && bytes(i - 1) == '\r')
      catch case NonFatal(_) => false
    val crlf = definitionFiles(root).collect { case (path, p) if holdsCrlf(p) => path }
    Option.when(crlf.nonEmpty) {
      val (have, their, them) = if crlf.size == 1 then ("has", "its", "it") else ("have", "their", "them")
      val files = s"teq: ${crlf.mkString(", ")} $have CRLF line ends, and $FileName records $their bytes"
      if inGitWorkTree(root) then s"$files, so that a checkout with LF finds it stale: $LfRemedy"
      else s"$files, so that a copy of the build that differs from $them by line ends alone finds it stale until it is exported on that copy's machine"
    }

  /** Whether the build's root is in a Git work tree: a `.git` at it or above, a repository's
    * directory or the file of a worktree or a submodule. */
  private def inGitWorkTree(root: Path): Boolean =
    Iterator.iterate(root.toAbsolutePath.normalize)(_.getParent).takeWhile(_ != null).exists(dir => Files.exists(dir.resolve(".git")))

  /** The build definition files, by path relative to the root with `/` in the order of the paths'
    * bytes: every `.sbt` at the root, `project/build.properties`, and every `.sbt` and `.scala`
    * under `project/` outside its `target` and hidden directories. */
  private def definitionFiles(root: Path): Seq[(String, Path)] =
    def isDirectory(p: Path) = Files.isDirectory(p, LinkOption.NOFOLLOW_LINKS)
    def listed(dir: Path): Seq[Path] =
      if !Files.isDirectory(dir) then Nil
      else
        val stream = Files.list(dir)
        try stream.toArray.toSeq.map(_.asInstanceOf[Path]) finally stream.close()
    def definitions(dir: Path): Seq[Path] = listed(dir).flatMap { p =>
      val name = p.getFileName.toString
      if isDirectory(p) then (if name == "target" || name.startsWith(".") then Nil else definitions(p))
      else if name.endsWith(".sbt") || name.endsWith(".scala") then Seq(p)
      else Nil
    }
    val atRoot = listed(root).filter(p => Files.isRegularFile(p) && p.getFileName.toString.endsWith(".sbt"))
    val properties = Seq(root.resolve("project").resolve("build.properties")).filter(Files.isRegularFile(_))
    (atRoot ++ properties ++ definitions(root.resolve("project"))).map(p => root.relativize(p).toString.replace('\\', '/') -> p).distinctBy(_._1).sortBy(_._1)(using utf8)

  private def hex(bytes: Array[Byte]): String = bytes.map(b => f"${b & 0xff}%02x").mkString

  val utf8: Ordering[String] = Json.utf8

  private def num(n: Long): Json.Value = Json.Num(n.toString)
  private def strings(items: Seq[String]): Json.Value = Json.Arr(items.map(Json.Str(_)))

  def rootOf(base: File): Path = base.toPath.toAbsolutePath.normalize

  /** A path under the build's root, relative with `/`; `.` for the root itself. */
  def relativeTo(root: Path, file: File): String =
    val path = file.toPath.toAbsolutePath.normalize
    root.relativize(path).toString.replace('\\', '/') match
      case "" => "."
      case relative => relative

  private def under(root: Path, file: File): Boolean = file.toPath.toAbsolutePath.normalize.startsWith(root)

  /** A path outside the build's root as a lock without the driver names it: absolute, with `/`. */
  private def absolute(file: File): String = file.toPath.toAbsolutePath.normalize.toString.replace('\\', '/')

  /** What a mapping of the stage holds, as a reason names it: a jar by its key, a product by its
    * project and configuration, a file by its path in the lock. */
  private def mapped(from: Json.Value): String = from match
    case Json.Str(key) => key
    case o: Json.Obj if !o("file").isNull => o("file").str
    case o: Json.Obj => s"${o("project").str}/${o("configuration").str}'s products"
    case other => other.toString

  /** What sbt resolved from a configuration's classpath, and the facts the export reads with it. */
  private final case class Config(
    config: Configuration,
    classpathConfiguration: Configuration,
    external: Classpath,
    sourceDirectories: Seq[File],
    resourceDirectories: Seq[File],
    scalacOptions: Seq[String],
    generators: Seq[Task[Seq[File]]],
    resourceGenerators: Seq[Task[Seq[File]]],
  )

  private def config(c: Configuration): Def.Initialize[Task[Config]] = Def.task {
    Config(
      c,
      (c / classpathConfiguration).value,
      (c / externalDependencyClasspath).value,
      (c / unmanagedSourceDirectories).value,
      (c / unmanagedResourceDirectories).value,
      (c / scalacOptions).value,
      (c / sourceGenerators).value,
      (c / resourceGenerators).value,
    )
  }

  /** sbt-native-packager's plugins whose Docker stage the export carries. */
  private val Packaging = Seq("com.typesafe.sbt.packager.archetypes.JavaAppPackaging", "com.typesafe.sbt.packager.docker.DockerPlugin")

  private val part: Def.Initialize[Task[Part]] = Def.taskDyn {
    val labels = thisProject.value.autoPlugins.map(_.label).toSet
    val buildInfo = labels("sbtbuildinfo.BuildInfoPlugin")
    val generator: Def.Initialize[Task[Either[Seq[String], Json.Value]]] =
      if buildInfo then BuildInfoGenerator.of else Def.task(Left(Seq("sbt-buildinfo is not enabled")))
    val docker: Def.Initialize[Task[Option[NativeStage]]] =
      if Packaging.forall(labels) then Def.task(Some(DockerStage.of.value)) else Def.task(None)
    val structure = buildStructure.value
    val buildRoot = rootOf((LocalRootProject / baseDirectory).value)
    val project = thisProjectRef.value
    def scope(c: Configuration) = Scope(Select(project), Select(ConfigKey(c.name)), Zero, Zero)
    // A session's `set` of the key counts as the build's declaration too (`set api / Compile / mainClass := None`
    // in CI's command line): its setting comes last, so the value read is the session's. sbt keeps a `set` under
    // `append`, by project, and a `set every` under `rawAppend`.
    val sessionSettings = Project.extract(state.value).session
    val session = sessionSettings.append.values.flatten.map(_._1).toSeq ++ sessionSettings.rawAppend
    val mainClassSet = setByBuild(structure, buildRoot, scope(Compile), mainClass.key) || setBySession(session, scope(Compile), mainClass.key)
    val runMainClassSet = setByBuild(structure, buildRoot, scope(Compile).copy(task = Select(run.key)), mainClass.key) ||
      setBySession(session, scope(Compile).copy(task = Select(run.key)), mainClass.key)
    val resourceGeneratorsSet = Seq(Compile, Test).filter(c => setByBuild(structure, buildRoot, scope(c), resourceGenerators.key)).map(_.name)
    // The main class the build declares, `Compile / mainClass` (in a file of the build or a session's
    // `set`): among `mainClasses`, the Docker stage's start script's and the product jar's manifest's, as sbt-native-packager packages it;
    // when the build declares none, the stage block and the manifest carry none and `teq stage`
    // implies the class from the products, as sbt discovers it after a compile (the block
    // says when the build set the key to None, which keeps the manifest without the class).
    // The one sbt's `run` runs is `Compile / run / mainClass` when the build sets it in that scope,
    // else the same: the run block's; none when the build declares neither (sbt then discovers).
    val declaredMain: Def.Initialize[Task[Option[String]]] = if mainClassSet then Def.task((Compile / mainClass).value) else Def.task(None)
    val runMain: Def.Initialize[Task[Option[String]]] = if runMainClassSet then Def.task((Compile / run / mainClass).value) else declaredMain
    Def.task {
      val root = rootOf((LocalRootProject / baseDirectory).value)
      val ref = thisProjectRef.value
      val name = ref.project
      val data = settingsData.value
      val deps = buildDependencies.value
      val converter = fileConverter.value
      val cache = csrCacheDirectory.value
      val lm = dependencyResolution.value
      val log = streams.value.log
      val snapshots = teqExportSnapshots.value
      val buildTool = Export.buildTool.value
      val configs = Seq(config(Compile).value, config(Runtime).value, config(Test).value)
      val sbtPlatform = platform.value
      val resolvers = externalResolvers.value
      val commands = (Compile / teqGenerators).value
      val buildInfoBlock = generator.value
      val native = docker.value
      val aliases = teqRunAliases.value
      val declared = declaredMain.value
      val runClass = runMain.value
      val mainClasses = (declared.toSeq ++ aliases.values ++ teqMainClasses.value).distinct.sorted(using utf8)
      val testOptions = (Test / Keys.testOptions).value
      val frameworksDeclared = (Test / testFrameworks).value
      val testFork = (Test / fork).value
      val testBase = (Test / baseDirectory).value
      val testEnv = (Test / Keys.envVars).value
      val testJavaOptions = (Test / javaOptions).value
      val runFork = (Compile / run / fork).value
      val runBase = (Compile / run / baseDirectory).value
      val runEnv = (Compile / run / Keys.envVars).value
      val runJavaOptions = (Compile / run / javaOptions).value
      val base = baseDirectory.value
      val devCommand = teqDevCommand.value
      val mainClass = teqMainClass.value
      val dependencies = libraryDependencies.value
      val selectedSources = teqSources.value ++ teqExtraSources.value
      val projects = loadedBuild.value.allProjectRefs.map(_._1)
      val scalaVersionOf = scalaVersion.value
      val description = Map(
        "excludes" -> strings(teqExcludes.value),
        "modulePerFile" -> strings(teqModulePerFile.value),
        "cacheableState" -> strings(teqCacheableState.value),
        "macroState" -> Json.Str(teqMacroState.value),
        "hot" -> Json.Bool(teqHot.value),
        "release" -> Json.Bool(teqRelease.value),
        "keys" -> Json.Obj(teqDescriptionKeys.value.map((k, v) => k -> Json.Str(v))),
      ) ++ teqThreads.value.map(n => "threads" -> num(n))
      val out = teqOutput.value
      val lib = teqLib.value
      val productionSources = teqProductionSources.value
      val productionExcludes = teqProductionExcludes.value

      val refusals = mutable.ArrayBuffer.empty[String]
      // What teq cannot reproduce of the project but its generators, by the verb it keeps
      // from running: the lock records it and the verb refuses at use.
      val unsupported = mutable.LinkedHashMap.empty[String, mutable.ArrayBuffer[String]]
      def record(verb: String, why: String) = unsupported.getOrElseUpdate(verb, mutable.ArrayBuffer.empty) += why
      val jars = mutable.ArrayBuffer.empty[Jar]
      // The artifacts written as files of this machine, which the stage names as the classpath does.
      val asFiles = mutable.Map.empty[File, Json.Value]
      /** A path relative to the build's root; outside it, under the driver a refusal (the
        * committed lock names nothing of one machine), else the absolute path with `/`, the lock
        * being this machine's description. */
      def path(what: String, file: File): String =
        if under(root, file) then relativeTo(root, file)
        else
          if buildTool then refusals += s"$name: $what $file is outside the build's root"
          absolute(file)
      def roots(what: String, dirs: Seq[File]) = dirs.map(path(what, _))

      val js = sbtPlatform == "sjs1"
      // A platform no reader takes (Scala Native) leaves the project out, so that a cross build exports.
      val skipped = Option.when(!js && sbtPlatform != "jvm")(s"its platform $sbtPlatform is neither the JVM nor Scala.js")
      val repositories = repositoriesOf(resolvers)

      val (compile, runtime, test) = (configs(0), configs(1), configs(2))
      // A generator teq cannot run (neither a TeqCommand nor sbt-buildinfo's of a shape it
      // writes) is recorded as sbt's, by its task's name, for teq to refuse the verbs that
      // need its output; the editor reads what sbt last generated.
      def sbtRuns(task: Task[?], reason: Option[String] = None): Json.Value =
        Json.Obj(Map("kind" -> Json.Str("sbt"), "task" -> Json.Str(task.get(Keys.taskDefinitionKey).fold("an unnamed task")(_.key.label))) ++ reason.map("reason" -> Json.Str(_)))
      val compileGenerators = compile.generators.flatMap { task =>
        task.get(Keys.taskDefinitionKey).map(_.key.label) match
          case Some("teqGenerate") => commands.map(commandJson(root, _, generatedDirectory(name), name, refusals, buildTool))
          case Some("buildInfo") if buildInfo => Seq(buildInfoBlock.fold(whys => sbtRuns(task, Some(whys.mkString("; "))), identity))
          case _ => Seq(sbtRuns(task))
      }
      def bySbt(generators: Seq[Json.Value]) = generators.exists(_("kind").str == "sbt")
      val runtimeGenerators = runtime.generators.filterNot(compile.generators.contains).map(sbtRuns(_))
      val testGenerators = test.generators.filterNot(compile.generators.contains).map(sbtRuns(_))
      // sbt's own resource generator, the plugin descriptors, is the one a configuration has unless
      // the build or a plugin adds another, or the build sets the list; teq runs none.
      def resourceGeneratorsOf(c: Config) =
        (if resourceGeneratorsSet.contains(c.config.name) then c.resourceGenerators else c.resourceGenerators.drop(1)).map(sbtRuns(_))
      /** sbt's managed source directories of a configuration, which a configuration whose
        * generators are sbt's reads in the place of teq's directory; one outside the build's
        * root is written as `path` writes it without the driver, and left out with a warning under
        * it, which a committed lock cannot name (the editor then misses what sbt generates there,
        * which no verb of teq reads). */
      def managed(c: Configuration) =
        val (inside, outside) = (ref / c / managedSourceDirectories).get(data).toSeq.flatten.partition(under(root, _))
        if buildTool then
          for dir <- outside do
            log.warn(s"teq: $name: sbt's managed source directory $dir of ${c.name} is outside the build's root, which $FileName cannot name under teqBuildTool: the language server and vite-plugin-teq do not read what its generators write there")
          inside.map(relativeTo(root, _))
        else (inside ++ outside).map(path("the managed source directory", _))
      val generatedSources =
        if bySbt(compileGenerators) then managed(Compile)
        else Option.when(compileGenerators.nonEmpty)(generatedDirectory(name)).toSeq
      def generatorFields(generators: Seq[Json.Value], resources: Seq[Json.Value]) =
        Option.when(generators.nonEmpty)("generators" -> Json.Arr(generators)) ++ Option.when(resources.nonEmpty)("resourceGenerators" -> Json.Arr(resources))

      /** A resolved module's artifact as its key, its record added to the part's jars. */
      def artifact(entry: Attributed[xsbti.HashedVirtualFileRef], configuration: String): Option[Json.Value] =
        val file = converter.toPath(entry.data).toFile
        val module = entry.get(Keys.moduleIDStr).map(Json.parse)
        val organization = module.fold("")(_("organization").str)
        val moduleName = module.fold(file.getName)(_("name").str)
        val version = module.fold("")(_("revision").str)
        val key = jarKey(organization, moduleName, version, classifierOf(entry))
        if !snapshots && isSnapshot(version) then
          refusals += s"$name: $organization:$moduleName:$version is a SNAPSHOT, which a later resolution can make another file (teqExportSnapshots := true allows it)"
        val url = entry.get(Keys.artifactStr).map(Json.parse(_)("url").str).filter(_.nonEmpty).orElse(cachedUrl(cache, file))
        // sbt's own copy of a Scala library, from its boot directory, has no URL: the jar is the one
        // the build's resolvers give for its module.
        val boot = organization == "org.scala-lang" && url.isEmpty
        val located =
          if boot then
            bootRepository(key, resolvedUrl(lm, organization % moduleName % version, cache, log), repositories) match
              case Right(found) => Right(found)
              case Left(why) => Left(why)
          else url.flatMap(u => repositoryOf(repositories, u).map(r => (r, u.stripPrefix(r.url))))
            .toRight(s"$file ($organization:$moduleName:$version) comes from ${url.getOrElse("no repository")}, which is none of the build's Maven repositories")
        located match
          // Under the driver the jar is refused, since a reader elsewhere could not fetch it;
          // without it, it is this machine's file.
          case Left(why) if buildTool =>
            refusals += s"$name: $why"
            None
          case Left(_) =>
            val entry = Json.Obj(Map("file" -> Json.Str(path("the classpath entry", file))))
            asFiles(file) = entry
            Some(entry)
          case Right((repository, at)) =>
            val sha1 = Sha1.of(file).getOrElse { refusals += s"$name: $file cannot be read"; "" }
            jars += Jar(key, repository, at, sha1, file.length, s"$name/$configuration")
            Some(Json.Str(key))

      def entries(c: Config): Seq[Either[(ProjectRef, String), Attributed[xsbti.HashedVirtualFileRef]]] =
        val own = productOf(data, ref, c.config.name).map(ref -> _)
        val internal = InterDependencies.of(ref, c.classpathConfiguration, c.config, data, deps).flatMap((dep, conf) => productOf(data, dep, conf).map(dep -> _))
        val products = (own.toSeq ++ internal).distinct.filterNot(_ == (ref -> c.config.name))
        for (dep, conf) <- products if !Configurations(conf) do
          refusals += s"$name: the ${c.config.name} classpath holds ${dep.project}'s $conf products, a configuration the export does not carry"
        products.map(Left(_)) ++ c.external.map(Right(_))

      def classpath(c: Config): Seq[Json.Value] = entries(c).flatMap {
        case Left((dep, conf)) => Some(Json.Obj(Map("project" -> Json.Str(dep.project), "configuration" -> Json.Str(conf))))
        // A resolved module's artifact carries its module; an entry without one is a file of the build.
        case Right(entry) if entry.get(Keys.moduleIDStr).isDefined => artifact(entry, c.config.name)
        case Right(entry) => Some(Json.Obj(Map("file" -> Json.Str(path("the classpath entry", converter.toPath(entry.data).toFile)))))
      }

      def flags(options: Seq[String]): (Json.Value, Option[Int]) =
        val o = TeqPlugin.ScalacOptions(options)
        val fields = Map(
          "strictEquality" -> Json.Bool(o.strictEquality),
          "kindProjector" -> Json.Bool(o.kindProjector),
          "werror" -> Json.Bool(o.werror),
          "ignoredScalacOptions" -> strings(o.ignored),
        ) ++ o.maxInlines.map(n => "maxInlines" -> num(n)) ++ o.javaOutputVersion.map(n => "javaOutputVersion" -> num(n)) ++
          Option.when(o.wunusedImports)("wunusedImports" -> Json.Bool(true))
        (Json.Obj(fields), o.javaOutputVersion.filter(_ => !js))
      val (compileFlags, compileVersion) = flags(compile.scalacOptions)
      val (testFlags, testVersion) = flags(test.scalacOptions)

      for option <- testOptions do option match
        case _: Tests.Filter | _: Tests.Filters =>
          record("test", "Test / testOptions holds a Tests.Filter, a function the export cannot carry: name the suites to leave out in a Tests.Exclude")
        case _: Tests.Setup | _: Tests.Cleanup =>
          record("test", "Test / testOptions holds a Tests.Setup or Tests.Cleanup, a function the export cannot carry")
        case _ => ()
      val frameworks = loadedFrameworks(frameworksDeclared, test, converter, js).map { (framework, implementation) =>
        val arguments = testOptions.flatMap {
          case Tests.Argument(None, args) => args
          case Tests.Argument(Some(f), args) if f == framework => args
          case _ => Nil
        }
        Json.Obj(Map("class" -> Json.Str(implementation), "arguments" -> strings(arguments)))
      }
      val excluded = testOptions.collect { case Tests.Exclude(names) => names }.flatten.distinct.sorted(using utf8)
      def context(forked: Boolean, dir: File, what: String) = if forked then path(what, dir) else "."
      def envVars(vars: Map[String, String]) = Json.Obj(vars.map((k, v) => k -> Json.Str(v)))

      val configurations = Map(
        "compile" -> Json.Obj(Map(
          "sources" -> strings(roots("the source directory", compile.sourceDirectories) ++ generatedSources),
          "resources" -> strings(roots("the resource directory", compile.resourceDirectories)),
          "classpath" -> Json.Arr(classpath(compile)),
          "flags" -> compileFlags,
          "generators" -> Json.Arr(compileGenerators),
          "mainClasses" -> strings(mainClasses),
        ) ++ generatorFields(Nil, resourceGeneratorsOf(compile))),
        "runtime" -> Json.Obj(Map(
          "sources" -> strings(Nil),
          "resources" -> strings(Nil),
          "classpath" -> Json.Arr(classpath(runtime)),
        ) ++ generatorFields(runtimeGenerators, Nil)),
        "test" -> Json.Obj(Map(
          "sources" -> strings(roots("the source directory", test.sourceDirectories) ++ (if testGenerators.nonEmpty then managed(Test) else Nil)),
          "resources" -> strings(roots("the resource directory", test.resourceDirectories)),
          "classpath" -> Json.Arr(classpath(test)),
          "flags" -> testFlags,
          "fork" -> Json.Bool(testFork),
          "baseDirectory" -> Json.Str(context(testFork, testBase, "Test / baseDirectory")),
          "envVars" -> envVars(testEnv),
          "javaOptions" -> strings(testJavaOptions),
          "frameworks" -> Json.Arr(frameworks.sortBy(_("class").str)(using utf8)),
          "exclude" -> strings(excluded),
        ) ++ generatorFields(testGenerators, resourceGeneratorsOf(test))),
      )

      // sbt's managed source directories are not teq's: a project's generated sources are
      // under its own directory of the export, where the project has generators teq runs,
      // and in sbt's where sbt runs them, which `withSources` decides once every project's part
      // says whose they are.
      val managedOf: Map[Path, Managed] = projects.flatMap { p =>
        val generates = (p / Compile / sourceGenerators).get(data).exists(_.nonEmpty)
        (p / Compile / managedSourceDirectories).get(data).toSeq.flatten.map { dir =>
          val named = if under(root, dir) then Some(relativeTo(root, dir)) else Option.when(!buildTool)(absolute(dir))
          dir.toPath.toAbsolutePath.normalize -> Managed(p.project, named, generates)
        }
      }.toMap
      val descriptionSources = selectedSources.map(dir => managedOf.get(dir.toPath.toAbsolutePath.normalize).toRight(path("the source directory", dir))).distinct

      for module <- dependencies if !snapshots && isDynamic(module.revision) do
        refusals += s"$name: ${module.organization}:${module.name}:${module.revision} is a dynamic version, which a later resolution can make another (teqExportSnapshots := true allows it)"

      // Every JVM project runs: `teq run <project>` takes the block's `mainClass`, the one
      // sbt's `run` runs, else the one its build finds, as sbt's `run` does. `mainClass` is absent
      // when the build declares none.
      val runBlock = Option.when(!js)("run" -> Json.Obj(Map(
        "baseDirectory" -> Json.Str(context(runFork, runBase, "run / baseDirectory")),
        "envVars" -> envVars(runEnv),
        "javaOptions" -> strings(runJavaOptions),
        "aliases" -> Json.Obj(aliases.map((k, v) => k -> Json.Str(v))),
      ) ++ runClass.map(main => "mainClass" -> Json.Str(main))))
      val devBlock = (if js then devOf(root, base, devCommand) else None).map("dev" -> _)

      // What the stage cannot reproduce: the stage block is left out and teq stage refuses.
      val unstaged = mutable.ArrayBuffer.empty[String]
      /** sbt-native-packager's `Docker / stage`: the jars of `scriptClasspathOrdering` (the
        * project's own, then the runtime classpath's, named by `makeJarName`), then the start
        * script, each in the layer `dockerGroupLayers` gives its path in the image. The block's
        * `mainClass` and the project's own jar's `Main-Class` are `Compile / mainClass` where the
        * build declares it; where the build sets it to `None` the block says so (`mainClassNone`),
        * since native-packager's start script then runs the one discovered class while sbt's
        * `packageBin` writes no `Main-Class`; where the build leaves it alone the block carries
        * neither, and `teq stage` implies the class for the script and the manifest, as sbt
        * takes the single discovered one after a compile. */
      def stageOf(native: NativeStage): Option[Json.Value] =
        unstaged ++= native.refusals.map(_.stripPrefix(s"$name: "))
        def productJar(dep: ProjectRef, conf: String): Option[(String, File)] =
          for
            id <- (dep / projectID).get(data)
            art <- (dep / ConfigKey(conf) / packageBin / Keys.artifact).get(data)
            file <- (dep / ConfigKey(conf) / packageBin / artifactPath).get(data)
          yield (jarName(id.organization, id.name, id.revision, art.name, art.classifier), converter.toPath(file).toFile)
        def manifest(dep: ProjectRef): Json.Value =
          def setting[T](key: SettingKey[T]) = (dep / Compile / packageBin / key).get(data)
          val (n, ver, org) = (setting(Keys.name).getOrElse(""), setting(Keys.version).getOrElse(""), setting(organization).getOrElse(""))
          val vendor = setting(organizationName).getOrElse(org)
          Json.Obj(Map(
            "Specification-Title" -> Json.Str(n),
            "Specification-Version" -> Json.Str(ver),
            "Specification-Vendor" -> Json.Str(vendor),
            "Implementation-Title" -> Json.Str(n),
            "Implementation-Version" -> Json.Str(ver),
            "Implementation-Vendor" -> Json.Str(vendor),
            "Implementation-Vendor-Id" -> Json.Str(org),
          ) ++ setting(homepage).flatten.map(url => "Implementation-URL" -> Json.Str(url.toString)) ++ declared.filter(_ => dep == ref).map(main => "Main-Class" -> Json.Str(main)))
        // (the source, the file native-packager maps, its path under the install location, whether it is a jar of the build's projects)
        val own = productJar(ref, "compile").map((jar, file) => (Json.Obj(Map("project" -> Json.Str(name), "configuration" -> Json.Str("compile"))) -> Some(manifest(ref)), file, s"lib/$jar", true))
        val libraries = entries(configs(1)).flatMap {
          case Left((dep, conf)) =>
            productJar(dep, conf).map((jar, file) => (Json.Obj(Map("project" -> Json.Str(dep.project), "configuration" -> Json.Str(conf))) -> Option.when(conf == "compile")(manifest(dep)), file, s"lib/$jar", true))
          case Right(entry) =>
            val file = converter.toPath(entry.data).toFile
            if !file.isFile then None
            else
              val module = entry.get(Keys.moduleIDStr).map(Json.parse)
              val art = entry.get(Keys.artifactStr).map(Json.parse)
              val jar = (module, art) match
                case (Some(m), Some(a)) => jarName(m("organization").str, m("name").str, m("revision").str, a("name").str, classifierOf(entry))
                case _ => file.getName
              val from = module match
                case Some(m) => asFiles.getOrElse(file, Json.Str(jarKey(m("organization").str, m("name").str, m("revision").str, classifierOf(entry))))
                case None => Json.Obj(Map("file" -> Json.Str(path("the classpath entry", file))))
              Some((from -> None, file, s"lib/$jar", false))
        }
        val jars = (own.toSeq ++ libraries).distinctBy(m => (m._2, m._3))
        val products = ref +: entries(configs(1)).collect { case Left((dep, _)) => dep }
        for dep <- products.distinct if setInBuild(structure, buildRoot, dep, packageOptions.key, _ => true) do
          unstaged += s"the build sets ${dep.project}'s packageOptions, which the jar teq makes of its products for the Docker stage does not carry"
        for (to, clashing) <- jars.groupBy(_._3) if clashing.size > 1 do
          unstaged += s"the Docker stage maps ${clashing.map(m => mapped(m._1._1)).mkString(" and ")} to $to"
        val install = native.installLocation.stripSuffix("/")
        val scriptPath = s"bin/${native.scriptName}"
        val mappings: Seq[(Json.Value, File, String, Boolean)] =
          jars.map { case ((from, manifest), file, to, artifact) =>
            (Json.Obj(Map("from" -> from, "to" -> Json.Str(s"${install.stripPrefix("/")}/$to")) ++ manifest.map("manifest" -> _)), file, to, artifact)
          } :+ (Json.Obj(Map("script" -> Json.Str(s"${install.stripPrefix("/")}/$scriptPath"), "classpath" -> strings(jars.map(_._3)))), native.scriptsDirectory / scriptPath, scriptPath, false)
        val layered = mappings.flatMap { (json, file, to, artifact) =>
          native.layer(file, s"$install/$to", artifact) match
            case Right(Some(layer)) => Some(layer -> json)
            case Right(None) =>
              unstaged += s"dockerGroupLayers puts $install/$to in no layer, which teq does not stage"
              None
            case Left(why) =>
              unstaged += why
              None
        }
        Some(Json.Obj(Map(
          "kind" -> Json.Str("docker"),
          "name" -> Json.Str(native.packageName),
          "exposedPorts" -> Json.Arr(native.exposedPorts.sorted.map(n => num(n))),
          "directory" -> Json.Str(path("Docker / stagingDirectory", native.stagingDirectory)),
          "layers" -> Json.Obj(layered.groupMap(_._1)(_._2).map((layer, ms) => layer.toString -> Json.Arr(ms))),
        ) ++ declared.map(main => "mainClass" -> Json.Str(main)) ++ Option.when(mainClassSet && declared.isEmpty)("mainClassNone" -> Json.Bool(true))))
      val stageBlock = native.filter(_ => !js).flatMap(stageOf).filter(_ => unstaged.isEmpty).map("stage" -> _)
      unstaged.distinct.foreach(record("stage", _))
      val fullDescription = description ++ Map(
        "out" -> Json.Str(path("teqOutput", out)),
        "production" -> Json.Obj(Map(
          "sources" -> strings(productionSources.map(path("the production source", _))),
          "excludes" -> strings(productionExcludes),
        )),
      ) ++ lib.map(dir => "lib" -> Json.Str(path("teqLib", dir))) ++
        mainClass.filter(_ => !js).map(main => "mainClass" -> Json.Str(main))

      Part(
        name,
        Json.Obj(Map(
          "base" -> Json.Str(path("the base directory", base)),
          "platform" -> Json.Str(if js then "js" else sbtPlatform),
          "scalaVersion" -> Json.Str(scalaVersionOf),
          "configurations" -> Json.Obj(configurations),
          "description" -> Json.Obj(fullDescription),
        ) ++ runBlock ++ devBlock ++ stageBlock ++
          Option.when(unsupported.nonEmpty)("unsupported" -> Json.Obj(unsupported.map((verb, whys) => verb -> strings(whys.distinct.toSeq)).toMap))),
        repositories,
        jars.toSeq,
        (compileVersion ++ testVersion).toSeq,
        refusals.toSeq,
        descriptionSources,
        Option.when(compileGenerators.nonEmpty)(bySbt(compileGenerators)),
        skipped,
      )
    }
  }

  def generatedDirectory(project: String): String = s"target/teq/$project/compile/src_managed"

  /** A jar's key in the table: `organization:name:version`, and `:classifier` when it has one. */
  private[sbt] def jarKey(organization: String, name: String, version: String, classifier: Option[String]): String =
    (Seq(organization, name, version) ++ classifier.filter(_.nonEmpty)).mkString(":")

  /** The Maven layout of a jar's key, the path a reader derives where the table names none:
    * `org/with/slashes/name/version/name-version[-classifier].jar`; none for a string that is no
    * `organization:name:version[:classifier]` (`task::export::maven_path` in teq). */
  private[sbt] def mavenLayout(key: String): Option[String] =
    key.split(":", -1).toSeq match
      case parts if (parts.size == 3 || parts.size == 4) && parts.forall(_.nonEmpty) =>
        val Seq(organization, name, version) = parts.take(3)
        Some(s"${organization.replace('.', '/')}/$name/$version/$name-$version${parts.drop(3).map("-" + _).mkString}.jar")
      case _ => None

  /** sbt-native-packager's `makeJarName`: `<organization>.<name>-[<artifact name less the
    * name>-]<revision>[-<classifier>].jar`. */
  private[sbt] def jarName(organization: String, name: String, revision: String, artifactName: String, classifier: Option[String]): String =
    organization + "." + name + "-" + Option(artifactName.replace(name, "")).filter(_.nonEmpty).map(_ + "-").getOrElse("") + revision +
      classifier.filter(_.nonEmpty).map("-" + _).getOrElse("") + ".jar"

  private def classifierOf(entry: Attributed[?]): Option[String] =
    entry.get(Keys.artifactStr).map(Json.parse(_)("classifier").str).filter(_.nonEmpty)

  /** Whether the build itself sets a key in a scope, by where its last setting there is defined: a
    * file of the build, not sbt's or a plugin's defaults. `Compile / mainClass` is otherwise sbt's
    * pick among the main classes a compile found, which the export never runs. A session's `set`
    * has a position of its own (`<set>`), no file: `setBySession` tells those. */
  private[sbt] def setByBuild(structure: sbt.internal.BuildStructure, root: Path, scope: Scope, key: AttributeKey[?]): Boolean =
    structure.settings.filter(s => s.key.key == key && s.key.scope == scope).lastOption.exists(s => inBuildFile(s.pos, root))

  /** Whether the session sets a key in a scope: a `set` or `set every` of the command line or the
    * shell (`session`, sbt's `SessionSettings.append` and `rawAppend` together), which sbt appends
    * after the build's own settings, so that where it does, the value in force is the session's.
    * The scope is the setting's own: sbt stores a `set every` as one setting per scope where the
    * key is defined, and a `set` of a scope no project's lookup reaches (`Zero / Compile /
    * mainClass`, "used by no settings") sets nothing, so it counts for nothing: counting it would
    * have the export read sbt's default, which compiles. */
  private[sbt] def setBySession(session: Seq[Setting[?]], scope: Scope, key: AttributeKey[?]): Boolean =
    session.exists(s => s.key.key == key && s.key.scope == scope)

  /** Whether a file of the build sets the key in a scope of the project, of the build or of every
    * project that `matching` takes. */
  private[sbt] def setInBuild(structure: sbt.internal.BuildStructure, root: Path, project: ProjectRef, key: AttributeKey[?], matching: Scope => Boolean): Boolean =
    structure.settings.exists { s =>
      val ours = s.key.scope.project match
        case Select(p: ProjectRef) => p == project
        case Select(_: BuildRef) | Zero => true
        case _ => false
      s.key.key == key && ours && matching(s.key.scope) && inBuildFile(s.pos, root)
    }

  /** Whether a position names a regular file under the build's root. A position's path is whatever the
    * setting's macro recorded: a plugin's setting can carry its source text (`dockerGroupLayers := { ... }`),
    * which Windows refuses as a path (`InvalidPathException` on the colon) where Linux resolves it to a file that
    * does not exist; either way it is no file of the build. */
  private[sbt] def inBuildFile(position: sbt.internal.util.SourcePosition, root: Path): Boolean = position match
    case p: sbt.internal.util.FilePosition =>
      try
        val file = new File(p.path)
        val path = (if file.isAbsolute then file else root.resolve(p.path).toFile).toPath.toAbsolutePath.normalize
        path.startsWith(root) && Files.isRegularFile(path)
      catch case _: java.nio.file.InvalidPathException => false
    case _ => false

  /** The tasks that the last definition of a key in a scope reads, by name, and the keys it reads
    * that do not resolve: none when it reads settings alone, which evaluating it runs nothing of.
    * Its own key among them, which `Setting.dependencies` leaves out: the definition it replaces. */
  private[sbt] def taskDependencies(structure: sbt.internal.BuildStructure, data: Def.Settings, scope: Scope, key: AttributeKey[?]): Seq[String] =
    structure.settings.filter(s => s.key.key == key && s.key.scope == scope).lastOption.toSeq.flatMap(_.init.dependencies).filter { dep =>
      data.get(dep) match
        case Some(_: Task[?]) | None => true
        case Some(_) => false
    }.map(_.key.label).distinct

  /** The product directory a configuration of a project puts on a classpath, by the
    * configuration whose sources make it: the configuration itself, or the one it extends whose
    * class directory it shares (`runtime` gives `compile`'s); none for a configuration without
    * a class directory (`optional`, `provided`). */
  private def productOf(data: Def.Settings, project: ProjectRef, conf: String): Option[String] =
    def directory(c: String) = (project / ConfigKey(c) / classDirectory).get(data)
    val configurations = (project / ivyConfigurations).get(data).getOrElse(Nil)
    def source(c: Configuration): Configuration =
      c.extendsConfigs.find(e => directory(e.name).isDefined && directory(e.name) == directory(c.name)).fold(c)(source)
    configurations.find(_.name == conf).filter(c => directory(c.name).isDefined).map(c => source(c).name)

  /** The test frameworks of `testFrameworks` that the test classpath's libraries implement, each
    * by the first of its classes found (`.sjsir` on Scala.js), as sbt's `loadedTestFrameworks`
    * loads them; each framework once. */
  private def loadedFrameworks(frameworks: Seq[TestFramework], test: Config, converter: xsbti.FileConverter, js: Boolean): Seq[(TestFramework, String)] =
    val suffix = if js then ".sjsir" else ".class"
    val entries = test.external.map(e => converter.toPath(e.data).toFile)
    val wanted = frameworks.flatMap(_.implClassNames).map(n => n.replace('.', '/') + suffix).toSet
    val present = entries.flatMap { entry =>
      if entry.isDirectory then wanted.filter(name => new File(entry, name).isFile)
      else if entry.isFile && entry.getName.endsWith(".jar") then
        try
          val zip = new ZipFile(entry)
          try wanted.filter(name => zip.getEntry(name) != null) finally zip.close()
        catch case NonFatal(_) => Nil
      else Nil
    }.toSet
    frameworks.distinct.flatMap(f => f.implClassNames.find(n => present(n.replace('.', '/') + suffix)).map(f -> _)).distinctBy(_._2)

  private[sbt] def isSnapshot(version: String): Boolean = version.endsWith("-SNAPSHOT")

  private[sbt] def isDynamic(revision: String): Boolean =
    revision.endsWith("+") || revision.startsWith("latest.") || revision.exists("[]()".contains(_))

  /** The build's remote Maven repositories, Maven Central as `maven-central` and the others by
    * their names, each with its host for the credentials a reader presents when it has them. */
  def repositoriesOf(resolvers: Seq[Resolver]): Seq[Repository] =
    val seen = mutable.Map.empty[String, String]
    resolvers.collect { case m: MavenRepository if m.root.startsWith("https://") || m.root.startsWith("http://") => m }.flatMap { m =>
      val url = if m.root.endsWith("/") then m.root else m.root + "/"
      if seen.values.exists(_ == url) then None
      else
        val base = if url == MavenCentralRoot then MavenCentral else slug(m.name)
        val id = Iterator.from(1).map(n => if n == 1 then base else s"$base-$n").find(id => !seen.contains(id)).get
        seen(id) = url
        Some(Repository(id, url, Option.when(id != MavenCentral)(URI.create(url).getHost)))
    }

  /** The URL of a module's jar as the build's resolvers resolve it (the module alone, none of its
    * dependencies), or why it does not resolve. */
  private def resolvedUrl(lm: DependencyResolution, module: ModuleID, cache: File, log: Logger): Either[String, Option[String]] =
    try
      lm.update(lm.wrapDependencyInModule(module.intransitive()), UpdateConfiguration(), UnresolvedWarningConfiguration(), log) match
        case Left(warning) => Left(warning.resolveException.messages.flatMap(_.linesIterator).map(_.trim).filter(_.nonEmpty).mkString("; "))
        case Right(report) =>
          (for
            conf <- report.configurations
            m <- conf.modules if m.module.organization == module.organization && m.module.name == module.name
            (artifact, file) <- m.artifacts if artifact.classifier.isEmpty && artifact.extension == "jar"
          yield artifact.url.map(_.toString).orElse(cachedUrl(cache, file))).headOption.toRight("no jar in its resolution")
    catch case NonFatal(e) => Left(e.getMessage)

  /** The repository and the path there of sbt's own copy of a Scala library (from its boot
    * directory, with no URL of its own): those of the jar the build's resolvers give for its module,
    * a repository the build declares; else a refusal naming the jar and the resolvers tried. The
    * export never lists a repository the build does not declare. */
  private[sbt] def bootRepository(key: String, resolved: Either[String, Option[String]], repositories: Seq[Repository]): Either[String, (Repository, String)] =
    val found = resolved.toOption.flatten.flatMap(u => repositoryOf(repositories, u).map(r => (r, u.stripPrefix(r.url))))
    found.toRight {
      val tried = if repositories.isEmpty then "none" else repositories.map(r => s"${r.id} (${r.url})").mkString(", ")
      val why = resolved match
        case Left(reason) => s": $reason"
        case Right(Some(u)) => s": it resolves from $u"
        case Right(None) => ""
      s"sbt's own $key, from its boot directory, is the jar of none of the build's Maven repositories (tried $tried)$why"
    }

  /** The repository a URL lies under: the one of the longest root. */
  private def repositoryOf(repositories: Seq[Repository], url: String): Option[Repository] =
    repositories.filter(r => url.startsWith(r.url)).maxByOption(_.url.length)

  private def slug(name: String): String =
    name.toLowerCase.replaceAll("[^a-z0-9]+", "-").stripPrefix("-").stripSuffix("-") match
      case "" => "repository"
      case s => s

  /** The URL of a file coursier's cache holds (`<cache>/https/<host>/<path>`, a character
    * coursier's `CachePath.escape` takes as `%` and two digits of base 16, such as a port's `:`,
    * read back). */
  private[sbt] def cachedUrl(cache: File, file: File): Option[String] =
    val path = file.toPath.toAbsolutePath.normalize
    val root = cache.toPath.toAbsolutePath.normalize
    Option.when(path.startsWith(root))(root.relativize(path).toString.replace('\\', '/')).collect {
      case relative if relative.startsWith("https/") || relative.startsWith("http/") =>
        val (scheme, rest) = relative.span(_ != '/')
        val unescaped = "%([0-9A-F]{2})".r.replaceAllIn(rest, m => scala.util.matching.Regex.quoteReplacement(Integer.parseInt(m.group(1), 16).toChar.toString))
        s"$scheme:/$unescaped"
    }

  /** A command generator's block: the command with the generated directory appended, relative to
    * its working directory; its outputs the generated directory and the command's own, relative to
    * the build's root. */
  private[sbt] def commandJson(root: Path, command: TeqCommand, output: String, project: String, refusals: mutable.Buffer[String], buildTool: Boolean): Json.Value =
    val cwd = root.resolve(command.cwd).normalize
    if buildTool && !cwd.startsWith(root) then refusals += s"$project: the TeqCommand ${command.run.mkString(" ")} runs in ${command.cwd}, outside the build's root"
    val out = cwd.relativize(root.resolve(output)).toString.replace('\\', '/')
    val written = command.outputs.map { o =>
      val path = root.resolve(o).normalize
      if buildTool && !path.startsWith(root) then refusals += s"$project: the TeqCommand ${command.run.mkString(" ")} writes $o, outside the build's root"
      if path.startsWith(root) then relativeTo(root, path.toFile) else absolute(path.toFile)
    }
    Json.Obj(Map(
      "kind" -> Json.Str("command"),
      "run" -> strings(command.run :+ out),
      "cwd" -> Json.Str(if cwd.startsWith(root) then relativeTo(root, cwd.toFile) else absolute(cwd.toFile)),
      "inputs" -> strings(command.inputs),
      "outputs" -> strings(output +: written),
    ))

  /** `dev` of a Scala.js project: the package manager and the lockfile of the nearest
    * `package.json` at or above its base, and the command, `teqDevCommand` or that package's
    * `dev` script; none without either. */
  private def devOf(root: Path, base: File, command: Seq[String]): Option[Json.Value] =
    val start = base.toPath.toAbsolutePath.normalize
    val packageDir = Iterator.iterate(start)(_.getParent).takeWhile(d => d != null && d.startsWith(root)).find(d => Files.isRegularFile(d.resolve("package.json")))
    packageDir.flatMap { dir =>
      val manifest = try Json.parse(IO.read(dir.resolve("package.json").toFile, UTF_8)) catch case NonFatal(_) => Json.Null
      val declared = manifest("packageManager").str.takeWhile(_ != '@')
      val lockfiles = Seq("yarn" -> "yarn.lock", "pnpm" -> "pnpm-lock.yaml", "bun" -> "bun.lock", "npm" -> "package-lock.json")
      val manager = Some(declared).filter(m => lockfiles.exists(_._1 == m))
        .orElse(lockfiles.collectFirst { case (m, lock) if Files.isRegularFile(dir.resolve(lock)) => m })
        .getOrElse("npm")
      val script = Option.when(dir == start && !manifest("scripts")("dev").isNull)(Seq(manager, "run", "dev"))
      Some(command).filter(_.nonEmpty).orElse(script).map { run =>
        Json.Obj(Map(
          "packageManager" -> Json.Str(manager),
          "command" -> strings(run),
          "lockfile" -> Json.Str(relativeTo(root, dir.resolve(lockfiles.find(_._1 == manager).get._2).toFile)),
        ))
      }
    }

  /** Runs the configuration's `teqGenerators` from the build's root into `out` and gives the
    * sources they wrote there. `teq`, the build's binary, runs a command whose first word is
    * `teq`: given for such a command alone, since resolving it may download it; a bare name is
    * looked up on the PATH as `teqBinary` says. */
  def generate(commands: Seq[TeqCommand], root: File, out: File, teq: Option[File], log: Logger): Seq[File] =
    for command <- commands do
      val cwd = (root / command.cwd).getCanonicalFile
      val target = cwd.toPath.relativize(out.getCanonicalFile.toPath).toString
      val stderr = mutable.ArrayBuffer.empty[String]
      val first =
        if !command.runsTeq then program(command.run.head)
        else teq match
          case Some(binary) if binary.getParentFile == null => program(binary.getName)
          case Some(binary) => binary.getAbsolutePath
          case None => throw new MessageOnlyException(s"teq: the generator ${command.run.mkString(" ")} runs teq, and no binary was given for it")
      val code =
        try Process((first +: command.run.tail) :+ target, cwd).!(ProcessLogger(line => log.info(line), line => stderr += line))
        catch case e: java.io.IOException => throw new MessageOnlyException(s"teq: the generator ${command.run.mkString(" ")} could not be started: ${e.getMessage}")
      stderr.foreach(line => log.error(line))
      if code != 0 then throw new MessageOnlyException(s"teq: the generator ${command.run.mkString(" ")} exited with code $code")
    if !out.isDirectory then Nil
    else (out ** ("*.scala" || "*.java")).get().filter(_.isFile).sorted

  /** A generator's program as the system starts it: on Windows a bare name is the first file on the
    * PATH with one of PATHEXT's extensions (`npm` is `npm.cmd`), which Java's process creation
    * would not look for, as teq's own runs find it (`task::generators::resolved`); a path, a name
    * with an extension and every name elsewhere as given. */
  def program(name: String, variable: String => Option[String] = n => Option(System.getenv(n)), osName: String = System.getProperty("os.name")): String =
    if !osName.toLowerCase.startsWith("windows") || name.exists(c => c == '/' || c == '\\') || name.contains('.') then name
    else
      val extensions = variable("PATHEXT").getOrElse(".COM;.EXE;.BAT;.CMD").split(';').filter(_.nonEmpty).map(_.toLowerCase)
      val dirs = variable("PATH").toSeq.flatMap(_.split(';')).filter(_.nonEmpty)
      dirs.iterator.flatMap(dir => extensions.iterator.map(e => new File(dir, name + e))).find(_.isFile).fold(name)(_.getPath)

  /** What has a watch run the configuration's generators again: their input globs, and the files of
    * the build that their arguments name, the script among them. */
  def generatorInputs(commands: Seq[TeqCommand], root: File): Seq[Glob] =
    val base = root.getCanonicalFile.toPath
    commands.flatMap { command =>
      val named = command.run.map(arg => (root / command.cwd / arg).getCanonicalFile).filter(f => f.isFile && f.toPath.startsWith(base))
      command.inputs.map(Glob(base, _)) ++ named.map(f => Glob(f.toPath))
    }.distinct

  /** The `teq` block: the compiler the build names (`teqVersion`) for every classifier its release's manifest lists,
    * or, for a SNAPSHOT published locally, the first of the build's repositories that serves it holds (`pinned`), no
    * binary downloaded. A release before 0.1.7 is refused (`Release.floor`), and so is a release its base does not
    * hold, unless teqExportSnapshots allows the table to stay empty: the table of a release not published yet. */
  private val binaries: Def.Initialize[Task[Binaries]] = Def.task {
    val log = streams.value.log
    val snapshots = teqExportSnapshots.value
    val (org, name, version) = (TeqPlugin.SnapshotGroup, teqArtifact.value, teqVersion.value)
    val base = teqReleases.value
    val plugin = BuildInfo.version
    val repositories = repositoriesOf(externalResolvers.value)
    val served = Served(allCredentials.value, log)
    val refusals = mutable.ArrayBuffer.empty[String]
    for (what, v) <- Seq("teqVersion" -> version, "the sbt-teq plugin's version" -> plugin) if !snapshots && (isSnapshot(v) || isDynamic(v)) do
      refusals += s"$what is $v, which a later resolution can make another file: name a release (teqExportSnapshots := true allows it)"
    if isSnapshot(version) then
      val pins = pinned(org, name, version, Classifiers, repositories, served, log)
      refusals ++= pins.refusals
      if pins.untold.nonEmpty then
        log.warn(s"teq: no binary of $org:$name:$version could be read from the build's repositories: ${pins.untold.mkString("; ")}; ${FileName} names none")
      else if pins.found.isEmpty then
        val why = s"no binary of $org:$name:$version is served by the build's repositories for any of ${Classifiers.mkString(", ")}"
        if snapshots then log.warn(s"teq: $why; ${FileName} names none") else refusals += why
      Binaries(version, pins.found, refusals.toSeq)
    else if Release.floor(version).nonEmpty then Binaries(version, Nil, (refusals ++ Release.floor(version)).toSeq)
    else
      // A release's binaries, from its manifest, checked against its SHA256SUMS: no binary downloaded.
      Release.manifest(base, version, served) match
        case Right(m) =>
          if m.entries.isEmpty then refusals += s"the release v$version's ${Release.manifestName(version)} lists no binary"
          Binaries(version, m.entries.map(e => Binary(e.classifier, Release.assetUrl(base, version, e.classifier), e.sha1, e.size)), refusals.toSeq)
        case Left(unread) if unread.absent =>
          val why = s"no release v$version of teq is at ${Release.directory(base, version)}: ${unread.why}"
          if snapshots then log.warn(s"teq: $why; ${FileName} names no binary") else refusals += why
          Binaries(version, Nil, refusals.toSeq)
        case Left(unread) =>
          log.warn(s"teq: the release v$version of teq could not be read: ${unread.why}; ${FileName} names no binary")
          Binaries(version, Nil, refusals.toSeq)
  }

  /** What the repositories give of the compiler's binaries: the records and the refusals; and when
    * no repository serves the version while some could not tell (out of reach, or answering other
    * than 200 or 404), why each could not: the export then names no binary, with a warning, so that
    * an export with no repository in reach still writes the lock. */
  private[sbt] final case class Pinned(found: Seq[Binary], refusals: Seq[String], untold: Seq[String])

  /** The path of the compiler's pom under a repository, or of a classifier's binary beside it, in
    * the Maven layout a local publish of a SNAPSHOT gives them: `build/teq/teq/<version>/teq-<version>[-<classifier>.exe|.pom]`. */
  private[sbt] def binaryPath(org: String, name: String, version: String, classifier: Option[String]): String =
    s"${org.replace('.', '/')}/$name/$version/$name-$version${classifier.fold(".pom")(c => s"-$c.exe")}"

  /** The compiler's binaries as the repositories serve them, none downloaded. The repositories are
    * asked in the build's order whether they serve the version, by a HEAD of its pom: coursier
    * resolves the module by its pom, so a repository serving the binaries without one is never
    * taken, and the first that serves it gives every classifier, a classifier it lacks not looked
    * for in a later one, as in coursier. A repository that cannot tell is passed over with a warning
    * when a later one serves the version, as coursier passes it over. Per classifier, the sha1 is
    * the `.sha1` beside the binary and the size a HEAD's `Content-Length`, the URL the resolver's
    * layout gives (redirects followed for the requests alone). A classifier is not published when
    * both its `.sha1` and its binary answer 404; a binary served without its `.sha1`, and any other
    * answer, is a refusal naming the classifier. */
  private[sbt] def pinned(org: String, name: String, version: String, classifiers: Seq[String], repositories: Seq[Repository], served: Served, log: Logger): Pinned =
    val module = s"$org:$name:$version"
    val pom = binaryPath(org, name, version, None)
    val unanswered = mutable.ArrayBuffer.empty[(Repository, String)]
    val serving = repositories.iterator.map { r =>
      served.head(r.url + pom) match
        case Right(answer) if answer.status == 200 => Some(r)
        case Right(answer) if answer.status == 404 => None
        case Right(answer) =>
          unanswered += r -> s"HEAD ${r.url}$pom answered ${answer.status}"
          None
        case Left(why) =>
          unanswered += r -> why
          None
    }.collectFirst { case Some(r) => r }
    def described(r: Repository) = s"the repository ${r.id} (${r.url})"
    serving match
      case None =>
        Pinned(Nil, Nil, unanswered.toSeq.map((r, why) => s"whether ${described(r)} serves $module cannot be told ($why)"))
      case Some(repository) =>
        for (r, why) <- unanswered do log.warn(s"teq: whether ${described(r)} serves $module cannot be told ($why); ${repository.id} serves it")
        val records: Seq[Either[String, Binary]] = classifiers.flatMap { classifier =>
          val path = binaryPath(org, name, version, Some(classifier))
          val url = repository.url + path
          def refused(why: String) = Some(Left(s"teq's binary for $classifier cannot be pinned from ${described(repository)}: $why"))
          served.get(url + ".sha1") match
            case Left(why) => refused(why)
            case Right(answer) if answer.status == 404 =>
              served.head(url) match
                case Left(why) => refused(why)
                case Right(head) if head.status == 404 => None
                case Right(head) if head.status == 200 => refused(s"$url is served without its checksum (GET $url.sha1 answered 404)")
                case Right(head) => refused(s"HEAD $url answered ${head.status}")
            case Right(answer) if answer.status != 200 => refused(s"GET $url.sha1 answered ${answer.status}")
            case Right(answer) =>
              Sha1.in(answer.body) match
                case None => refused(s"$url.sha1 holds no SHA-1: \"${answer.body.trim.linesIterator.nextOption().getOrElse("").take(80)}\"")
                case Some(sha1) =>
                  served.head(url) match
                    case Left(why) => refused(why)
                    case Right(head) if head.status != 200 => refused(s"HEAD $url answered ${head.status}")
                    case Right(head) =>
                      head.length.filter(_ >= 0) match
                        case None => refused(s"HEAD $url gave no Content-Length")
                        case Some(size) => Some(Right(Binary(classifier, repository.url.stripSuffix("/") + "/" + path.stripPrefix("/"), sha1, size, Some(repository))))
        }
        val (refusals, found) = records.partitionMap(identity)
        Pinned(found, refusals, Nil)

  /** The cache of teq's binaries that the launchers, the vite plugin and the language server read:
    * `$TEQ_CACHE_DIR`, else `$XDG_CACHE_HOME/teq`, else `%LOCALAPPDATA%\teq` on Windows,
    * `~/Library/Caches/teq` on macOS and `~/.cache/teq` elsewhere (`src/jarcache.rs`). */
  def cacheRoot(variable: String => Option[String] = sys.env.get, osName: String = System.getProperty("os.name")): Option[File] =
    def env(name: String) = variable(name).filter(_.nonEmpty).map(new File(_))
    val os = osName.toLowerCase
    env("TEQ_CACHE_DIR")
      .orElse(env("XDG_CACHE_HOME").map(_ / "teq"))
      .orElse(
        if os.startsWith("windows") then env("LOCALAPPDATA").map(_ / "teq")
        else env("HOME").map(home => if os.startsWith("mac") then home / "Library" / "Caches" / "teq" else home / ".cache" / "teq"))

  /** The binary at `<cache root>/bin/<sha1>/<name>`, copied there beside the file and renamed into
    * place when it is not there with the same size. */
  def share(binary: File, sha1: String, name: String, root: Option[File], log: Logger): Unit =
    root.foreach { cache =>
      val target = cache / "bin" / sha1 / name
      if !(target.isFile && target.length == binary.length) then
        try
          IO.createDirectory(target.getParentFile)
          val part = Files.createTempFile(target.getParentFile.toPath, s".$name.", ".part")
          try
            Files.copy(binary.toPath, part, StandardCopyOption.REPLACE_EXISTING)
            part.toFile.setExecutable(true)
            Files.move(part, target.toPath, StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
          finally Files.deleteIfExists(part)
        catch case NonFatal(e) => log.warn(s"teq: the binary was not copied to $target: ${e.getMessage}")
    }
