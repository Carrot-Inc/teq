package sbt.internal.teq

import java.io.File
import java.nio.file.{FileSystems, Files, Path}
import java.util.Optional

import scala.collection.mutable
import scala.jdk.CollectionConverters.*
import scala.sys.process.*

import sbt.*
import sbt.Keys.*
import sbt.internal.inc.{Analysis, CompileOutput, Incremental, JarUtils, LoggedReporter, Lookup, NoopExternalLookup, OracleRelations, Stamps}
import sbt.util.{InterfaceUtil, Logger}
import xsbti.{AnalysisCallback, VirtualFile, VirtualFileRef}
import xsbti.api.{AnalyzedClass, ClassLike}
import xsbti.compile.{CompileAnalysis, CompileOrder, FileHash, IncOptions, MiniOptions, MiniSetup, ScalaCompiler}

import dev.teq.sbt.Json

/** The oracle of `tests/analysis.sh`: every case of `ANALYSIS_CASES` (directories, comma
  * separated; a directory with `a`, `b`, `c` or `main` below it is a module case, each module a
  * project depending on the ones before it) compiled by scalac 3.8.4 and built by teq (`TEQ`)
  * with `--analysis-version`, each through zinc's own incremental compiler (`Incremental.apply`)
  * with zinc's API storage on: scalac through sbt's compiler bridge, teq's answer through the
  * plugin's adapter. Each module is compiled into products of its own against the products of
  * the modules before it, and on both sides zinc finds an upstream module's classes in that
  * module's analysis, as sbt's lookup finds a project dependency's. Each side is written to
  * `target/analysis/<project>/` as the graph of every class with zinc's hashes of it, and the
  * dependencies: the callbacks the compile made (`Recording`) and the relations zinc stored,
  * for the comparison. */
object Oracle extends AutoPlugin:
  override def trigger = allRequirements

  object autoImport:
    val analysisScalac = taskKey[File]("scalac's analysis of the project through zinc, as a graph with zinc's hashes and the dependencies")
    val analysisTeq = taskKey[File]("teq's analysis of the project through the adapter and zinc, as a graph with zinc's hashes and the dependencies")
    val analysisCost = taskKey[Unit]("the JVM's side of an answer's analysis (ANALYSIS_ANSWER, its class files in ANALYSIS_CLASSES), timed")
    val tastyProbe = taskKey[Unit]("what zinc makes of binary dependencies on a product directory of TASTy files alone (TastyProbe)")
    val invalidationOracle = taskKey[Unit]("what zinc's incremental compiler recompiles after each scenario's edit on both sides (Invalidation; INVALIDATION_SCENARIOS names some)")

  import autoImport.*

  private val modules = Seq("a", "b", "c", "main")

  private def cases: Seq[File] =
    sys.env.getOrElse("ANALYSIS_CASES", "").split(',').toSeq.map(_.trim).filter(_.nonEmpty).map(new File(_))

  private def id(name: String): String = name.map(c => if c.isLetterOrDigit then c else '_')

  /** Each side's analysis of the projects compiled so far in this run, which a downstream
    * project's lookup finds its upstream classes in. */
  private val analyses = Map("scalac" -> mutable.Map.empty[String, Analysis], "teq" -> mutable.Map.empty[String, Analysis])

  override def projectSettings: Seq[Setting[?]] = Seq(
    invalidationOracle := Def.uncached {
      val base = (ThisBuild / baseDirectory).value
      val converter = fileConverter.value
      val jars = (Compile / externalDependencyClasspath).value.map(_.data).map(converter.toPath)
      val teq = sys.env.getOrElse("TEQ", throw new MessageOnlyException("TEQ names the teq binary"))
      Invalidation.run(streams.value.log, converter, base, (base / ".." / ".." / ".." / "tests").getCanonicalFile, jars, compilers.value.scalac,
        (Compile / scalacOptions).value, teq, sys.env.get("INVALIDATION_SCENARIOS"))
    },
    tastyProbe := Def.uncached {
      val base = (ThisBuild / baseDirectory).value
      val root = base / ".." / ".." / ".."
      val converter = fileConverter.value
      val jars = (Compile / externalDependencyClasspath).value.map(_.data).map(converter.toPath).map(_.toFile)
      TastyProbe.run(streams.value.log, fileConverter.value, target.value / "tasty-probe", root / "tests" / "modules" / "basic" / "a", root / "tests" / "modules" / "basic" / "b", sys.env.getOrElse("TEQ", "teq"), jars)
    },
    analysisCost := Def.uncached {
      val log = streams.value.log
      val converter = fileConverter.value
      val answerFile = new File(sys.env.getOrElse("ANALYSIS_ANSWER", throw new MessageOnlyException("ANALYSIS_ANSWER names an answer")))
      val classes = new File(sys.env.getOrElse("ANALYSIS_CLASSES", throw new MessageOnlyException("ANALYSIS_CLASSES names its class files")))
      val text = IO.read(answerFile)
      // The first runs warm the JVM up. Apart: the JSON's parse, the objects' construction
      // (every lazy field forced), zinc's hashes of them (`HashAPI` and `NameHashing`, what its
      // callback's `api` computes), and the whole callback with the analysis it builds.
      for run <- 1 to 5 do
        val parseStart = System.nanoTime()
        val answer = Json.parse(text)
        val parseMs = (System.nanoTime() - parseStart) / 1e6
        val buildStart = System.nanoTime()
        val graphs = ApiGraph.read(answer("api"))
        val forcing = new xsbt.api.Visit
        graphs.foreach(_.classes.foreach(forcing.visitAPI))
        val buildMs = (System.nanoTime() - buildStart) / 1e6
        val hashStart = System.nanoTime()
        graphs.foreach(_.classes.foreach { c =>
          xsbt.api.HashAPI(c)
          new xsbt.api.NameHashing(false).nameHashes(c)
        })
        val hashMs = (System.nanoTime() - hashStart) / 1e6
        System.gc()
        val before = java.lang.Runtime.getRuntime.totalMemory - java.lang.Runtime.getRuntime.freeMemory
        // As sbt runs zinc: the stored classes minimized, no API diff kept for debugging.
        val (analysis, feedMs, zincMs) = zincAnalysis(answer, classes, converter, log, TeqAnalysis.noLookup, keepApis = false)(identity)
        System.gc()
        val after = java.lang.Runtime.getRuntime.totalMemory - java.lang.Runtime.getRuntime.freeMemory
        val classesIn = graphs.map(_.classes.size).sum
        log.info(f"analysisCost run $run: ${text.length} bytes, $classesIn classes; JSON parsed in $parseMs%.1f ms, objects built in $buildMs%.1f ms, hashed in $hashMs%.1f ms; the callback $feedMs%.1f ms and zinc's analysis $zincMs%.1f ms; ${(after - before) / 1048576.0}%.1f MB retained with the analysis")
    },
  )

  override def extraProjects: Seq[Project] =
    cases.flatMap { dir =>
      val present = modules.filter(m => new File(dir, m).isDirectory)
      val parts = if present.isEmpty then Seq(("", dir)) else present.map(m => (m, new File(dir, m)))
      val projects = parts.map { (module, src) =>
        val name = id(if module.isEmpty then dir.getName else s"${dir.getName}_$module")
        (name, src)
      }
      projects.zipWithIndex.map { case ((name, src), i) =>
        val upstream = projects.take(i)
        Project(name, file("target") / "cases" / name)
          .settings(caseSettings(src, upstream.map(_._1)))
      }
    }

  private def caseSettings(src: File, upstreamNames: Seq[String]): Seq[Setting[?]] = Seq(
    scalaVersion := "3.8.4",
    Compile / unmanagedSourceDirectories := Seq(src),
    Compile / unmanagedSources / includeFilter := "*.scala",
    analysisScalac := Def.uncached {
      val dir = target.value / "analysis"
      IO.delete(Seq(dir / "scalac.json", dir / "scalac-error.txt"))
      val converter = fileConverter.value
      val jars = (Compile / externalDependencyClasspath).value.map(_.data).map(converter.toPath)
      val scalac = compilers.value.scalac
      val options = (Compile / scalacOptions).value
      try analysisScalacNow(streams.value.log, converter, (ThisBuild / baseDirectory).value, name.value, dir, src, upstreamNames, jars, scalac, options)
      catch
        case e: Exception =>
          IO.write(dir / "scalac-error.txt", e.toString)
          throw e
    },
    analysisTeq := Def.uncached {
      val dir = target.value / "analysis"
      IO.delete(Seq(dir / "teq.json", dir / "teq-error.txt"))
      val converter = fileConverter.value
      val jars = (Compile / externalDependencyClasspath).value.map(_.data).map(converter.toPath).map(_.toString)
      try analysisTeqNow(streams.value.log, converter, (ThisBuild / baseDirectory).value, name.value, dir, src, upstreamNames, jars)
      catch
        case e: Exception =>
          // The next project's run goes on: the comparison reports the failure.
          IO.write(dir / "teq-error.txt", e.toString)
          dir / "teq-error.txt"
    },
  )

  /** scalac's compile of the project through sbt's compiler bridge, driven by zinc as sbt's
    * `compile` drives it, into products of its own against the upstream projects'. */
  private def analysisScalacNow(log: Logger, converter: xsbti.FileConverter, root: File, name: String, dir: File, src: File, upstreamNames: Seq[String],
      jars: Seq[Path], scalac: ScalaCompiler, options: Seq[String]): File =
    val products = productsDir(root, "scalac", name)
    IO.delete(products)
    IO.createDirectory(products)
    val classpath = (upstreamNames.map(productsDir(root, "scalac", _).toPath) ++ jars).map(converter.toVirtualFile).toArray
    val sources = scalaSources(src).map(converter.toVirtualFile).toSet
    val output = CompileOutput(products.toPath)
    val reporter = new LoggedReporter(100, log)
    val setup = MiniSetup.of(output, MiniOptions.of(Array(), options.toArray, Array()), "3.8.4", CompileOrder.Mixed, true, Array())
    var recording: Option[Recording] = None
    val (_, analysis) = Incremental.apply(
      sources, converter, upstream("scalac", upstreamNames), Analysis.empty, IncOptions.of().withStoreApis(true).withApiDebug(true), setup,
      Stamps.timeWrapBinaryStamps(converter), output, JarUtils.createOutputJarContent(output), None, None, None, log,
    ) { (batch, changes, callback, _) =>
      val r = Recording.of(callback, entry(root, converter), relative(src, converter))
      recording = Some(r)
      scalac.compile(batch.toArray, classpath, converter, changes, options.toArray, output, r, reporter, Optional.empty(), log)
    }
    if reporter.hasErrors then throw new MessageOnlyException(s"scalac failed on $name: ${reporter.problems.map(_.message).mkString("; ")}")
    analyses("scalac")(name) = analysis
    write(dir / "scalac.json", dump(analysis, src, converter, root, recording))

  private def analysisTeqNow(log: Logger, converter: xsbti.FileConverter, root: File, name: String, dir: File, src: File, upstreamNames: Seq[String], jars: Seq[String]): File =
    {
      val teq = sys.env.getOrElse("TEQ", throw new MessageOnlyException("TEQ names the teq binary"))
      // Each module is built into its products against the upstream modules' products, as a
      // module's compile is in the module model (docs/TARGETS.md).
      val products = productsDir(root, "teq", name)
      IO.delete(products)
      val upstreamProducts = upstreamNames.map(productsDir(root, "teq", _).getPath)
      val command = Seq(teq, "build", "--target", "jvm", "--std=scala-library", "--products", products.getPath,
        "--classpath", (upstreamProducts ++ jars).mkString(":"), "--sourceroot", root.getPath, "--analysis-version", TeqAnalysis.version.toString, src.getPath)
      val stdout = new StringBuilder
      val started = System.nanoTime()
      val code = Process(command).!(ProcessLogger(line => stdout ++= line ++= "\n", line => log.info(s"teq: $line")))
      val teqMs = (System.nanoTime() - started) / 1e6
      if code != 0 then throw new MessageOnlyException(s"teq failed ($code): ${command.mkString(" ")}")
      val answerText = stdout.result().trim
      val parseStart = System.nanoTime()
      val answer = Json.parse(answerText)
      val parseMs = (System.nanoTime() - parseStart) / 1e6
      var recording: Option[Recording] = None
      val (analysis, adaptMs, zincMs) = zincAnalysis(answer, products, converter, log, upstream("teq", upstreamNames)) { callback =>
        val r = Recording.of(callback, entry(root, converter), relative(src, converter))
        recording = Some(r)
        r
      }
      analyses("teq")(name) = analysis
      IO.write(dir / "teq-answer.json", answerText)
      IO.write(dir / "teq-costs.json", f"""{"teqMs":$teqMs%.1f,"answerBytes":${answerText.length},"parseMs":$parseMs%.1f,"adaptMs":$adaptMs%.1f,"zincMs":$zincMs%.1f}""")
      write(dir / "teq.json", dump(analysis, src, converter, root, recording))
    }

  private def productsDir(root: File, side: String, project: String): File = root / "target" / s"$side-products" / project

  private def scalaSources(src: File): Seq[Path] =
    val walk = Files.walk(src.toPath)
    try walk.iterator.asScala.filter(p => p.toString.endsWith(".scala") && Files.isRegularFile(p)).toSeq.sorted
    finally walk.close()

  /** A binary entry named apart from where the build put it: an upstream project's products
    * as `products:<project>/<path>` (scalac's and teq's under their own directories), a jar
    * by its name, a class file of the JDK's runtime image as `jrt:<path>`. */
  private def entry(root: File, converter: xsbti.FileConverter)(p: Path): String =
    if p.getFileSystem ne FileSystems.getDefault then s"jrt:$p"
    else
      val s = p.toAbsolutePath.normalize.toString
      val products = Seq("scalac", "teq").map(side => (root / "target" / s"$side-products").getAbsolutePath + "/")
      products.find(s.startsWith) match
        case Some(prefix) => "products:" + s.stripPrefix(prefix)
        case None if s.endsWith(".jar") => "jar:" + artifact(p.getFileName.toString)
        case None if s.startsWith("/modules/") => s"jrt:$s"
        case None => "file:" + s

  /** A jar's artifact without its version: sbt's compiler bridge puts the Scala instance's
    * `scala-library.jar` on scalac's class path where the build's resolution names
    * `scala-library-3.8.4.jar`, the same artifact. */
  private def artifact(jar: String): String =
    jar.stripSuffix(".jar").replaceFirst("-[0-9][^/]*$", "")

  private def entryOf(root: File, converter: xsbti.FileConverter)(ref: VirtualFileRef): String =
    try entry(root, converter)(converter.toPath(ref))
    catch case _: Exception => "ref:" + ref.id

  /** A source by its path under the project's directory. */
  private def relative(src: File, converter: xsbti.FileConverter)(ref: VirtualFileRef): String =
    val p = converter.toPath(ref).toAbsolutePath.normalize
    val base = src.toPath.toAbsolutePath.normalize
    if p.startsWith(base) then base.relativize(p).toString else p.toString

  /** A lookup that finds a class of the upstream projects in their analyses on `side`, as
    * sbt's finds a class of a project dependency: a dependency on it is external, not a
    * library's. */
  private def upstream(side: String, names: Seq[String]): Lookup =
    val found = names.flatMap(analyses(side).get)
    new Lookup with NoopExternalLookup:
      def changedClasspathHash: Option[Vector[FileHash]] = None
      def analyses: Vector[CompileAnalysis] = found.toVector
      def lookupOnClasspath(binaryClassName: String): Option[VirtualFileRef] = None
      def lookupAnalysis(binaryClassName: String): Option[CompileAnalysis] =
        found.find(_.relations.productClassName.reverse(binaryClassName).nonEmpty)
      override def lookupAnalyzedClass(binaryClassName: String, file: Option[VirtualFileRef]): Option[AnalyzedClass] =
        for
          analysis <- found.find(_.relations.productClassName.reverse(binaryClassName).nonEmpty)
          className <- analysis.relations.productClassName.reverse(binaryClassName).headOption
          analyzed <- analysis.apis.internal.get(className)
        yield analyzed

  private def write(f: File, text: String): File =
    IO.write(f, text)
    f

  /** teq's answer through the adapter and zinc's own incremental compiler, which calls the
    * adapter as its Scala compiler once and computes the hashes; what the callback took and
    * what the rest of zinc's analysis took, in milliseconds. `keepApis` keeps the classes whole
    * in the analysis (`apiDebug`), for the comparison; `wrap` is the callback the adapter feeds,
    * given zinc's. */
  private def zincAnalysis(answer: Json.Value, classDir: File, converter: xsbti.FileConverter, log: Logger, lookup: Lookup, keepApis: Boolean = true)(
      wrap: AnalysisCallback => AnalysisCallback): (Analysis, Double, Double) =
    val root = new File(".").getAbsoluteFile
    val sources: Set[VirtualFile] = answer("api")("files").items.map(f => converter.toVirtualFile(TeqAnalysis.absolute(root, f("file").str).toPath)).toSet
    val output = CompileOutput(classDir.toPath)
    val setup = MiniSetup.of(output, MiniOptions.of(Array(), Array(), Array()), "teq", CompileOrder.Mixed, true, Array())
    val stamper = Stamps.timeWrapBinaryStamps(converter)
    var adaptMs = 0.0
    val started = System.nanoTime()
    val (_, analysis) = Incremental.apply(
      sources, converter, lookup, Analysis.empty, IncOptions.of().withStoreApis(true).withApiDebug(keepApis), setup, stamper, output,
      JarUtils.createOutputJarContent(output), None, None, None, log,
    ) { (_, _, callback, _) =>
      val t = System.nanoTime()
      TeqAnalysis.feed(answer, root, classDir, wrap(callback), converter)
      adaptMs = (System.nanoTime() - t) / 1e6
    }
    val totalMs = (System.nanoTime() - started) / 1e6
    (analysis, adaptMs, totalMs - adaptMs)

  /** The analysis's classes by source, as graphs, and zinc's hashes of each; the dependency
    * callbacks the compile made, as `recording` has them, and the relations zinc stored. */
  private def dump(analysis: Analysis, src: File, converter: xsbti.FileConverter, root: File, recording: Option[Recording]): String =
    val apis = analysis.apis.internal
    val bySource = apis.keys.toSeq.groupBy(c => analysis.relations.classes.reverse(c).headOption.map(_.id).getOrElse("?"))
    val binaries = analysis.relations.productClassName
    val files = bySource.toSeq.sortBy(_._1).map { (source, names) =>
      val classes = names.sorted.flatMap { n =>
        val c = apis(n).api
        Seq(c.classApi, c.objectApi).filter(present)
      }
      val products = names.sorted.flatMap(n => binaries.forward(n).toSeq.sorted.map(b => (n, b)))
      ApiGraph.File(source, classes, products, Nil)
    }
    val hashes = apis.toSeq.sortBy(_._1).map { (n, ac) => hashesOf(ac) }
    val deps = recording match
      case Some(r) => ",\n\"deps\":" + depsOf(r, OracleRelations.of(analysis, relative(src, converter), entryOf(root, converter)))
      case None => ""
    s"""{"api":${ApiGraph.write(files)},\n"hashes":[${hashes.mkString(",\n")}]$deps}"""

  private def row(fields: String*): String = fields.mkString("[", ",", "]")
  private def rows(items: Iterable[String]): String = items.mkString("[\n", ",\n", "]")
  private def str(s: String): String = Json.string(s)
  private def strs(s: Seq[String]): String = s.map(str).mkString("[", ",", "]")

  private def depsOf(r: Recording, stored: OracleRelations.Stored): String =
    val used = r.usedNames.toSeq.sortBy(u => (u._1, u._2)).map((c, n, s) => row(str(c), str(n), strs(s)))
    val classes = r.classDependencies.toSeq.sorted.map((on, from, ctx) => row(str(on), str(from), str(ctx)))
    val binaries = r.binaryDependencies.toSeq.sorted.map((path, binary, from, source, ctx) => row(str(path), str(binary), str(from), str(source), str(ctx)))
    val relations = Seq(
      s""""classes":${rows(stored.classDeps.map((from, kind, to, ctx) => row(str(from), str(kind), str(to), str(ctx))))}""",
      s""""libraries":${rows(stored.libraries.map((s, l) => row(str(s), str(l))))}""",
      s""""libraryClasses":${rows(stored.libraryClasses.map((l, c) => row(str(l), str(c))))}""",
      s""""names":${rows(stored.names.map((c, n, s) => row(str(c), str(n), strs(s))))}""",
    )
    s"""{"usedName":${rows(used)},\n"classDependency":${rows(classes)},\n"binaryDependency":${rows(binaries)},\n"relations":{${relations.mkString(",\n")}}}"""

  /** A placeholder zinc puts in the place of a class or an object the source does not define. */
  private def present(c: ClassLike): Boolean = c.structure.parents.nonEmpty

  private def hashesOf(ac: AnalyzedClass): String =
    val names = ac.nameHashes.toSeq.map(h => (h.name, h.scope.name, h.hash)).sorted
    val nameHashes = names.map((n, s, h) => s"[${Json.string(n)},${Json.string(s)},$h]").mkString("[", ",", "]")
    s"""{"name":${Json.string(ac.name)},"apiHash":${ac.apiHash},"extraHash":${ac.extraHash},"nameHashes":$nameHashes}"""
