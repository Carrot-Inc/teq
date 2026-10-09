package sbt.internal.teq

import java.io.File
import java.nio.file.{FileSystems, Files, Path}
import java.util.Optional

import scala.collection.mutable
import scala.jdk.CollectionConverters.*
import scala.jdk.OptionConverters.*

import sbt.*
import sbt.internal.inc.{Analysis, CompileOutput, Incremental, InvalidationZinc, JarUtils, LoggedReporter, OracleRelations, Stamps}
import sbt.util.Logger
import xsbti.{AnalysisCallback, FileConverter, VirtualFile, VirtualFileRef}
import xsbti.compile.{CompileOrder, FileHash, MiniOptions, MiniSetup, ScalaCompiler}

import dev.teq.sbt.Json

/** The invalidation oracle (`tests/invalidation.sh`, docs/TARGETS.md, "The invalidation oracle"):
  * what zinc's own incremental compiler recompiles after an edit, on scalac's analysis and on
  * teq's. Per scenario of `tests/analysis/invalidation/scenarios.txt` and variant of its options,
  * the case's modules are copied to a working tree, each built from an empty analysis into a
  * class directory of its own, upstream first, on both sides (scalac through sbt's compiler
  * bridge, teq through `TeqBatch`); the edit is applied; each module is compiled again by
  * `Incremental.apply` over its edited sources with the analysis it kept, zinc's compile
  * callback compiling exactly the batch it asks for, with zinc's profiler (`InvalidationZinc`)
  * recording the initial changes and every cycle, and sbt's lookup over the module's class path
  * (its own class directory, the upstream modules' with their new analyses, the jars). Then a
  * fresh build of the edited tree into class directories of their own, the reference: every
  * file of a module's class directory against the fresh build's, each with the source that owns
  * it and whether the run compiled that source again, and the diagnostics of both. Each side is
  * written to `target/invalidation/results/<scenario>.<variant>.<side>.json` for
  * `tests/analysis/invalidation.py`. Nothing here invalidates: zinc does, the driver feeds and
  * reads. */
object Invalidation:
  final case class Variant(name: String, transitiveStep: Int, recompileAllFraction: Double, useOptimizedSealed: Boolean)
  /** An edit of the case; one `before` a project applies to both sides' class directories just
    * before that project's run, after the runs of the projects upstream of it. */
  final case class Edit(action: String, args: Seq[String], before: Option[String])
  final case class Scenario(name: String, fixture: File, modules: Seq[String], pad: Int, variants: Seq[Variant], edits: Seq[Edit], jars: Seq[String],
      teqChecks: Set[String], refusals: Set[(String, String)])

  /** One compile of a batch: its sources, whether it passed, its diagnostics, what it reported
    * as products. */
  final case class Batch(sources: Seq[String], passed: Boolean, problems: Seq[TeqBatch.Problem], products: Seq[(String, String, String)], local: Seq[(String, String)])

  /** A module's run: whether it passed, its batches, what the profiler recorded, the analysis
    * and the setup it leaves. */
  final case class Run(passed: Boolean, failure: String, batches: Seq[Batch], recorder: InvalidationZinc.Recorder, analysis: Analysis, setup: MiniSetup)

  private val defaultVariant = Variant("default", 3, 0.5, false)

  def scenarios(file: File, tests: File): Seq[Scenario] =
    val blocks = mutable.Buffer.empty[mutable.Buffer[Seq[String]]]
    for raw <- IO.readLines(file) do
      val line = raw.split("#", 2)(0).trim
      if line.nonEmpty then
        val words = line.split("\\s+").toSeq
        if words.head == "scenario" then blocks += mutable.Buffer(words)
        else if blocks.isEmpty then throw new MessageOnlyException(s"$file: `$line` before any scenario")
        else blocks.last += words
    blocks.toSeq.map { lines =>
      val name = lines.head(1)
      def all(key: String) = lines.tail.toSeq.filter(_.head == key).map(_.tail)
      val fixture = all("fixture").headOption.map(w => tests / w.head).getOrElse(tests / "analysis" / "invalidation" / name)
      val modules = all("modules").headOption.getOrElse(Seq("a", "b", "c", "main").filter(m => (fixture / m).isDirectory)) match
        case Seq() => Seq("")
        case ms => ms
      val variants = all("variant").map { w =>
        w.tail.foldLeft(defaultVariant.copy(name = w.head)) { (v, opt) =>
          opt.split("=", 2) match
            case Array("transitiveStep", n) => v.copy(transitiveStep = n.toInt)
            case Array("recompileAllFraction", n) => v.copy(recompileAllFraction = n.toDouble)
            case Array("useOptimizedSealed", b) => v.copy(useOptimizedSealed = b.toBoolean)
            case _ => throw new MessageOnlyException(s"$file: scenario $name: unknown option $opt")
        }
      }
      val edits = all("edit").map {
        case Seq("before", m, action, args*) => Edit(action, args, Some(m))
        case w => Edit(w.head, w.tail, None)
      }
      for e <- edits if e.before.nonEmpty && !Set("delete-product", "replace-product")(e.action) do
        throw new MessageOnlyException(s"$file: scenario $name: an edit before a project changes its products alone")
      Scenario(name, fixture, modules, all("pad").headOption.fold(0)(_.head.toInt), if variants.isEmpty then Seq(defaultVariant) else variants,
        edits, all("jar").map(_.head), all("teq-check").map(_.head).toSet, all("refuse").map(w => (w(0), w(1))).toSet)
    }

  def run(log: Logger, converter: FileConverter, base: File, tests: File, jars: Seq[Path], scalac: ScalaCompiler, options: Seq[String], teq: String,
      only: Option[String]): Unit =
    Canonical.check()
    val all = scenarios(tests / "analysis" / "invalidation" / "scenarios.txt", tests)
    val wanted = only.map(_.split(',').map(_.trim).filter(_.nonEmpty).toSet).filter(_.nonEmpty)
    val chosen = all.filter(s => wanted.forall(_(s.name)))
    for w <- wanted; n <- w if !all.exists(_.name == n) do throw new MessageOnlyException(s"no scenario $n in scenarios.txt")
    val results = base / "target" / "invalidation" / "results"
    IO.delete(results)
    IO.createDirectory(results)
    val all0 = System.nanoTime()
    for s <- chosen; v <- s.variants do
      val started = System.nanoTime()
      val work = base / "target" / "invalidation" / s.name / v.name
      val sides = Seq("scalac", "teq")
      val dumps =
        try new ScenarioRun(log, converter, work, s, v, jars, scalac, options, teq).dumps()
        catch
          case e: Exception =>
            log.error(s"invalidation ${s.name}/${v.name}: $e")
            sides.map(side => side -> s"""{"scenario":${Json.string(s.name)},"variant":${Json.string(v.name)},"side":${Json.string(side)},"error":${Json.string(e.toString)}}""")
      for (side, text) <- dumps do IO.write(results / s"${s.name}.${v.name}.$side.json", text)
      log.info(f"invalidation ${s.name}/${v.name}: ${(System.nanoTime() - started) / 1e9}%.1f s")
    log.info(f"invalidation: ${chosen.map(_.variants.size).sum} runs of ${chosen.size} scenarios in ${(System.nanoTime() - all0) / 1e9}%.1f s")

  /** One scenario's variant on both sides: the working tree, the builds, the edit, the runs and
    * the reference. */
  private final class ScenarioRun(log: Logger, converter: FileConverter, work: File, s: Scenario, v: Variant, jars0: Seq[Path], scalac: ScalaCompiler,
      options: Seq[String], teq: String):
    private val src = work / "src"
    private val jarDir = work / "jars"
    private val sides = Seq("scalac", "teq")

    def dumps(): Seq[(String, String)] =
      IO.delete(work)
      IO.createDirectory(src)
      for m <- s.modules do
        IO.copyDirectory(if m.isEmpty then s.fixture else s.fixture / m, src / m)
        for i <- 1 to s.pad do
          val pkg = if m.isEmpty then "pad" else s"pad$m"
          IO.write(src / m / s"Pad$i.scala", s"package $pkg\n\nobject Pad$i:\n  def value: Int = $i\n")
      // A case without modules is its own directory, with the edit's files beside it.
      if s.modules == Seq("") then IO.delete(Seq(src / "edit") ++ s.jars.map(src / _))
      for lib <- s.jars do buildJar(s.fixture / lib, jarDir / s"$lib.jar")
      val before = sides.map(side => side -> buildAll(side, "out", None)).toMap
      for side <- sides; (m, r) <- before(side) if !r.passed && !s.refusals((side, m)) do
        throw new MessageOnlyException(s"the $side build before the edit fails in ${label(m)}: ${r.failure}")
      for (side, m) <- s.refusals if before(side).find(_._1 == m).forall(_._2.passed) do
        throw new MessageOnlyException(s"the $side build before the edit of ${label(m)} passes, which the scenario says it refuses")
      // A side that refuses a project before the edit stops there, its builds the dump.
      val going = sides.filter(side => before(side).forall(_._2.passed))
      for e <- s.edits if e.before.isEmpty do edit(e, going)
      val runs = going.map(side => side -> buildAll(side, "out", Some(before(side).map((m, r) => m -> (r.analysis, r.setup)).toMap))).toMap
      val clean = going.map(side => side -> buildAll(side, "clean", None)).toMap
      sides.map(side => side -> (if going.contains(side) then dump(side, runs(side), clean(side)) else refused(side, before(side))))

    private def label(m: String) = if m.isEmpty then "the project" else s"module $m"

    private def jars: Seq[Path] = jars0 ++ s.jars.map(l => (jarDir / s"$l.jar").toPath)

    private def classDir(side: String, kind: String, m: String): File = work / side / kind / (if m.isEmpty then "top" else m)

    /** A source by its path under the working tree's sources. */
    private def source(ref: VirtualFileRef): String =
      val p = converter.toPath(ref).toAbsolutePath.normalize
      val b = src.toPath.toAbsolutePath.normalize
      if p.startsWith(b) then b.relativize(p).toString else p.toString

    /** A product or a binary entry: a module's class file as `<module>:<path>`, a jar by its
      * name, the JDK's runtime image's class file as `jrt:<path>`. */
    private def entry(p: Path): String =
      if p.getFileSystem ne FileSystems.getDefault then s"jrt:$p"
      else
        val a = p.toAbsolutePath.normalize
        val dirs = for side <- sides; kind <- Seq("out", "clean"); m <- s.modules yield (classDir(side, kind, m).toPath.toAbsolutePath.normalize, if m.isEmpty then "top" else m)
        dirs.find((d, _) => a.startsWith(d)) match
          case Some((d, m)) => s"$m:${d.relativize(a)}"
          case None if a.toString.endsWith(".jar") => s"jar:${a.getFileName}"
          case None if a.toString.startsWith("/modules/") => s"jrt:$a"
          case None => s"file:$a"

    private def product(ref: VirtualFileRef): String =
      try entry(converter.toPath(ref))
      catch case _: Exception => "ref:" + ref.id

    private def scalaSources(dir: File): Seq[Path] =
      if !dir.isDirectory then Nil
      else
        val walk = Files.walk(dir.toPath)
        try walk.iterator.asScala.filter(p => p.toString.endsWith(".scala") && Files.isRegularFile(p)).toSeq.sorted
        finally walk.close()

    /** Every module of the case on `side`, upstream first, into the class directories `kind`,
      * each from its previous analysis or an empty one; a module whose upstream failed is not
      * run. */
    private def buildAll(side: String, kind: String, previous: Option[Map[String, (Analysis, MiniSetup)]]): Seq[(String, Run)] =
      val done = mutable.Buffer.empty[(String, Run)]
      for m <- s.modules do
        if done.forall(_._2.passed) then
          if previous.nonEmpty && kind == "out" then
            for e <- s.edits if e.before.contains(m) do edit(e, Seq(side))
          val upstream = done.toSeq.map((u, r) => (classDir(side, kind, u), r.analysis))
          done += m -> module(side, kind, m, upstream, previous.map(_(m)))
      done.toSeq

    private def module(side: String, kind: String, m: String, upstream: Seq[(File, Analysis)], previous: Option[(Analysis, MiniSetup)]): Run =
      val dir = classDir(side, kind, m)
      IO.createDirectory(dir)
      val backup = work / side / "bak" / (if m.isEmpty then "top" else m)
      val output = CompileOutput(dir.toPath)
      val classpath: Seq[VirtualFile] = ((dir +: upstream.map(_._1)).map(_.toPath) ++ jars).map(converter.toVirtualFile)
      val analysisOf = upstream.map((d, a) => converter.toVirtualFile(d.toPath).id -> a).toMap
      val hash = InvalidationZinc.classpathHash(classpath, converter)
      val lookup = new InvalidationZinc.SbtLookup(classpath, e => analysisOf.get(e.id), previous.fold(Vector.empty[FileHash])(_._2.options.classpathHash.toVector), hash)
      val recorder = new InvalidationZinc.Recorder(source, product)
      val opts = InvalidationZinc.options(v.transitiveStep, v.recompileAllFraction, v.useOptimizedSealed, recorder, backup, log)
      val compilerVersion = if side == "scalac" then "3.8.4" else "teq"
      val setup = MiniSetup.of(output, MiniOptions.of(hash.toArray, options.toArray, Array()), compilerVersion, CompileOrder.Mixed, true, Array())
      val ordered = scalaSources(src / m).map(converter.toVirtualFile)
      val sources = ordered.toSet
      val batches = mutable.Buffer.empty[Batch]
      val manifest = dir / TeqBatch.manifestName
      val manifestBefore = if manifest.exists then Some(IO.read(manifest)) else None
      try
        val (_, analysis) = Incremental.apply(
          sources, converter, lookup, previous.fold(Analysis.empty)(_._1), opts, setup, Stamps.timeWrapBinaryStamps(converter), output,
          JarUtils.createOutputJarContent(output), None, None, None, log,
        ) { (batch, changes, callback, manager) =>
          val names = batch.toSeq.map(source).sorted
          // As sbt's compiler (`MixedAnalyzingCompiler.compile`): the class directory made, and the
          // batch's sources that the configuration still has compiled, none when it has none.
          IO.createDirectory(dir)
          val compiled = ordered.filter(batch.contains)
          val r = Recording.of(callback, entry, source)
          def record(passed: Boolean, problems: Seq[TeqBatch.Problem]) =
            batches += Batch(names, passed, problems, r.products.toSeq.sorted, r.localProducts.toSeq.sorted)
          try
            if compiled.isEmpty then ()
            else if side == "scalac" then
              val reporter = new LoggedReporter(100, log)
              val failed =
                try
                  scalac.compile(compiled.toArray, classpath.toArray, converter, changes, options.toArray, output, r, reporter, Optional.empty(), log)
                  None
                catch case e: Exception => Some(e)
              val problems = reporter.problems.toSeq.filter(_.severity == xsbti.Severity.Error).map { p =>
                val file = p.position.sourcePath.toScala.map(f => source(VirtualFileRef.of(f))).getOrElse("?")
                TeqBatch.Problem(file, p.position.line.toScala.fold(0)(_.intValue), p.message)
              }
              if failed.nonEmpty || problems.nonEmpty then
                throw new TeqBatch.Failed(problems, s"scalac failed on ${names.mkString(" ")}: ${problems.map(_.message).take(3).mkString(" | ")}${failed.fold("")(e => s" ($e)")}")
            else
              val upstreamDirs = upstream.map(_._1)
              TeqBatch.compile(teq, s.teqChecks(m), compiled.map(f => converter.toPath(f).toFile), ordered.map(f => converter.toPath(f).toFile), src, dir,
                upstreamDirs ++ jars.map(_.toFile), r, manager, converter, log)
            record(true, Nil)
          catch
            case e: Exception =>
              record(false, e match { case f: TeqBatch.Failed => f.problems; case _ => Nil })
              throw e
        }
        if side == "teq" then TeqBatch.reconcile(teq, s.teqChecks(m), ordered.map(f => converter.toPath(f).toFile), src, dir, upstream.map(_._1) ++ jars.map(_.toFile))
        Run(true, "", batches.toSeq, recorder, analysis, setup)
      catch
        case e: Exception =>
          manifestBefore match
            case Some(text) => IO.write(manifest, text)
            case None => IO.delete(manifest)
          Run(false, e.toString, batches.toSeq, recorder, previous.fold(Analysis.empty)(_._1), previous.fold(setup)(_._2))

    /** A library jar of the case, compiled by scalac from its sources. */
    private def buildJar(dir: File, jar: File): Unit =
      val classes = work / "jar-classes" / jar.getName.stripSuffix(".jar")
      IO.delete(classes)
      IO.createDirectory(classes)
      val output = CompileOutput(classes.toPath)
      val sources = scalaSources(dir).map(converter.toVirtualFile).toSet
      val reporter = new LoggedReporter(100, log)
      val setup = MiniSetup.of(output, MiniOptions.of(Array(), options.toArray, Array()), "3.8.4", CompileOrder.Mixed, true, Array())
      Incremental.apply(
        sources, converter, TeqAnalysis.noLookup, Analysis.empty, xsbti.compile.IncOptions.of(), setup, Stamps.timeWrapBinaryStamps(converter), output,
        JarUtils.createOutputJarContent(output), None, None, None, log,
      ) { (batch, changes, callback, _) =>
        scalac.compile(batch.toArray, jars0.map(converter.toVirtualFile).toArray, converter, changes, options.toArray, output, callback, reporter, Optional.empty(), log)
      }
      if reporter.hasErrors then throw new MessageOnlyException(s"the jar $jar does not compile")
      IO.createDirectory(jar.getParentFile)
      val files = (classes ** "*").get().filter(_.isFile).map(f => f -> IO.relativize(classes, f).get).sortBy(_._2)
      IO.jar(files, jar, new java.util.jar.Manifest, Some(0L))

    /** The edit, on the working tree's sources and on the class directories of `sides`. */
    private def edit(e: Edit, sides: Seq[String]): Unit = e.action match
      case "replace" | "add" =>
        val path = e.args.head
        val from = if e.args.length > 1 then s.fixture / e.args(1) else s.fixture / "edit" / path
        if !from.isFile then throw new MessageOnlyException(s"scenario ${s.name}: no ${from}")
        if e.action == "replace" && !(src / path).isFile then throw new MessageOnlyException(s"scenario ${s.name}: replace $path, which the case has not")
        IO.copyFile(from, src / path)
      case "remove" =>
        val f = src / e.args.head
        if !f.isFile then throw new MessageOnlyException(s"scenario ${s.name}: remove ${e.args.head}, which the case has not")
        IO.delete(f)
      case "touch" =>
        val f = src / e.args.head
        IO.write(f, IO.read(f))
      case "delete-product" =>
        for side <- sides do
          val f = classDir(side, "out", e.args(0)) / e.args(1)
          if !f.isFile then throw new MessageOnlyException(s"scenario ${s.name}: $side has no product ${e.args(1)} in ${e.args(0)}")
          IO.delete(f)
      case "replace-product" =>
        for side <- sides do
          val dir = classDir(side, "out", e.args(0))
          if !(dir / e.args(1)).isFile || !(dir / e.args(2)).isFile then throw new MessageOnlyException(s"scenario ${s.name}: $side lacks ${e.args(1)} or ${e.args(2)}")
          IO.copyFile(dir / e.args(2), dir / e.args(1), preserveLastModified = false)
      case "replace-jar" =>
        val lib = e.args.head
        buildJar(s.fixture / "edit" / lib, jarDir / s"$lib.jar")
      case other => throw new MessageOnlyException(s"scenario ${s.name}: unknown edit $other")

    /** Every file of a class directory, by its path: its bytes' hash and the hash of what they
      * say, a class file as `javap -v` prints it without its constant pool's numbering, its
      * header and its TASTy attribute's identifier, a `.tasty` file after its header's
      * identifier, the manifest by its entries apart. */
    private def files(dir: File): Map[String, (String, String)] =
      if !dir.isDirectory then Map.empty
      else
        (dir ** "*").get().filter(_.isFile).map { f =>
          val rel = IO.relativize(dir, f).get
          val bytes = IO.readBytes(f)
          val canonical =
            if f.getName == TeqBatch.manifestName then
              val m = Json.parse(IO.read(f))
              // An entry's digest restates its files' bytes, which are compared on their own.
              def withoutDigest(e: Json.Value) = e match
                case Json.Obj(fields) => Json.Obj(fields - "digest")
                case other => other
              def entries(field: String) = m(field).items.map(withoutDigest).map(TeqBatch.render).sorted
              (entries("products") ++ entries("std").map("std " + _) ++ entries("inits").map("init " + _)).mkString("\n").getBytes("UTF-8")
            else if f.getName.endsWith(".class") then Canonical.classFile(f).getBytes("UTF-8")
            else if f.getName.endsWith(".tasty") then Canonical.tasty(bytes)
            else bytes
          rel -> (Canonical.sha(bytes), Canonical.sha(canonical))
        }.toMap

    private def str(x: String): String = Json.string(x)
    private def strs(xs: Iterable[String]): String = xs.map(str).mkString("[", ",", "]")
    private def row(fields: String*): String = fields.mkString("[", ",", "]")
    private def rows(items: Iterable[String]): String = items.mkString("[", ",", "]")

    private def change(c: InvalidationZinc.Change): String =
      row(str(c.cls), str(c.kind), rows(c.names.map((n, sc) => row(str(n), strs(sc)))))

    private def ownership(o: InvalidationZinc.Ownership): String =
      s"""{"classes":${rows(o.classes.map((s, c) => row(str(s), str(c))))},"products":${rows(o.products.map((s, b, p) => row(str(s), str(b), str(p))))},"local":${rows(o.local.map((s, p) => row(str(s), str(p))))}}"""

    private def relations(a: Analysis): String =
      val r = OracleRelations.of(a, source, product)
      s"""{"classes":${rows(r.classDeps.map((f, k, t, c) => row(str(f), str(k), str(t), str(c))))},"libraries":${rows(r.libraries.map((s, l) => row(str(s), str(l))))},"names":${rows(r.names.map((c, n, sc) => row(str(c), str(n), strs(sc))))}}"""

    /** A side that refused a project before the edit: its builds up to that project. */
    private def refused(side: String, builds: Seq[(String, Run)]): String =
      val modules = s.modules.map { m =>
        val name = if m.isEmpty then "top" else m
        builds.find(_._1 == m) match
          case Some((_, r)) if !r.passed => s"""{"module":${str(name)},"status":"refused","failure":${str(r.failure)}}"""
          case Some(_) => s"""{"module":${str(name)},"status":"built"}"""
          case None => s"""{"module":${str(name)},"status":"not-run"}"""
      }
      val opts = s"""{"transitiveStep":${v.transitiveStep},"recompileAllFraction":${v.recompileAllFraction},"useOptimizedSealed":${v.useOptimizedSealed}}"""
      s"""{"scenario":${str(s.name)},"variant":${str(v.name)},"side":${str(side)},"options":$opts,"refused":true,"modules":${rows(modules)}}"""

    /** A side's dump: per module the run's initial changes, cycles, batches and what the analysis
      * it leaves says each source owns, and the reference: the fresh build's status, diagnostics
      * and ownership, and every file of the class directory against the fresh build's. */
    private def dump(side: String, runs: Seq[(String, Run)], clean: Seq[(String, Run)]): String =
      val cleanBy = clean.toMap
      val modules = s.modules.map { m =>
        val name = if m.isEmpty then "top" else m
        runs.find(_._1 == m) match
          case None => s"""{"module":${str(name)},"status":"not-run"}"""
          case Some((_, r)) =>
            val initial = r.recorder.initial.fold("null") { i =>
              s"""{"added":${strs(i.added)},"removed":${strs(i.removed)},"changed":${strs(i.changed)},"removedProducts":${strs(i.removedProducts)},"libraryDeps":${strs(i.libraryDeps)},"external":${rows(i.external.map(change))}}"""
            }
            val cycles = r.recorder.cycles.map { c =>
              val events = rows(c.events.map(e => row(str(e.kind), strs(e.inputs), strs(e.outputs))))
              s"""{"invalidated":${strs(c.invalidated)},"packageObjects":${strs(c.packageObjects)},"initialSources":${strs(c.initialSources)},"sources":${strs(c.sources)},"recompiled":${strs(c.recompiled)},"changes":${rows(c.changes.map(change))},"next":${strs(c.next)},"continues":${c.continues},"events":$events}"""
            }
            val batches = r.batches.map { b =>
              s"""{"sources":${strs(b.sources)},"passed":${b.passed},"errors":${rows(b.problems.map(p => row(str(p.file), p.line.toString, str(p.message))))},"products":${rows(b.products.map((s, p, n) => row(str(s), str(p), str(n))))},"local":${rows(b.local.map((s, p) => row(str(s), str(p))))}}"""
            }
            val reference = cleanBy.get(m) match
              case None => """{"status":"not-run"}"""
              case Some(c) =>
                val errors = c.batches.flatMap(_.problems).map(p => row(str(p.file), p.line.toString, str(p.message)))
                val compiled = r.batches.flatMap(_.sources).toSet
                val owners = InvalidationZinc.ownership(r.analysis, source, product)
                val cleanOwners = InvalidationZinc.ownership(c.analysis, source, product)
                // A product's owner, a `.tasty` file's the owner of its class file.
                val ownerOf = (owners.products.map((s, _, p) => p -> s) ++ owners.local.map((s, p) => p -> s) ++
                  cleanOwners.products.map((s, _, p) => p -> s) ++ cleanOwners.local.map((s, p) => p -> s)).toMap
                def owner(rel: String): String =
                  val key = s"$name:$rel"
                  ownerOf.get(key)
                    .orElse(if rel.endsWith(".tasty") then Seq(".class", "$.class").map(x => s"$name:${rel.stripSuffix(".tasty")}$x").flatMap(ownerOf.get).headOption else None)
                    .getOrElse("")
                val (mine, theirs) = (files(classDir(side, "out", m)), files(classDir(side, "clean", m)))
                val compared = (mine.keySet ++ theirs.keySet).toSeq.sorted.map { rel =>
                  val state = (mine.get(rel), theirs.get(rel)) match
                    case (Some(a), Some(b)) => if a == b then "same" else if a._2 == b._2 then "equivalent" else "differs"
                    case (Some(_), None) => "only-run"
                    case _ => "only-fresh"
                  val o = owner(rel)
                  row(str(rel), str(state), str(o), (o.nonEmpty && compiled(o)).toString)
                }
                s"""{"status":${str(if c.passed then "passed" else "failed")},"errors":${rows(errors)},"ownership":${ownership(cleanOwners)},"relations":${relations(c.analysis)},"files":${rows(compared)}}"""
            s"""{"module":${str(name)},"status":${str(if r.passed then "passed" else "failed")},"failure":${str(r.failure)},"initial":$initial,"cycles":${rows(cycles)},"batches":${rows(batches)},"ownership":${ownership(InvalidationZinc.ownership(r.analysis, source, product))},"relations":${relations(r.analysis)},"reference":$reference}"""
      }
      val opts = s"""{"transitiveStep":${v.transitiveStep},"recompileAllFraction":${v.recompileAllFraction},"useOptimizedSealed":${v.useOptimizedSealed}}"""
      s"""{"scenario":${str(s.name)},"variant":${str(v.name)},"side":${str(side)},"options":$opts,"modules":${rows(modules)}}"""

/** The comparison of a product with the fresh build's apart from what differs between two
  * compiles of one tree: the order of a class file's constant pool and the identifier a TASTy
  * file's header and its class file's `TASTY` attribute carry. */
object Canonical:
  def sha(bytes: Array[Byte]): String =
    java.util.HexFormat.of.formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes))

  private lazy val javap = java.util.spi.ToolProvider.findFirst("javap").orElseThrow(() => new MessageOnlyException("the JDK's javap is not there"))

  def classFile(f: File): String =
    val out = new java.io.StringWriter
    val err = new java.io.StringWriter
    javap.run(new java.io.PrintWriter(out), new java.io.PrintWriter(err), "-p", "-c", "-v", f.getAbsolutePath)
    val lines = out.toString.linesIterator.toSeq
    val header = Set("Classfile", "Last modified", "SHA-256 checksum", "MD5 checksum")
    val body = mutable.Buffer.empty[String]
    var inPool = false
    var inTasty = false
    for line <- lines do
      val t = line.trim
      if t == "Constant pool:" then inPool = true
      else if inPool && t == "{" then
        inPool = false
        body += t
      else if inPool then ()
      else if header.exists(t.startsWith) then ()
      else if t.startsWith("TASTY:") then
        inTasty = true
        body += t
      else if inTasty && t.matches("([0-9A-F]{2} ?)+") then ()
      else
        inTasty = false
        body += line
    body.map(poolless).mkString("\n")

  /** A line of `javap -v` without the numbers of the constant pool's entries it refers to (its
    * `#n` tokens); the text `javap` prints of a constant stays as it is: a quoted string, the
    * comment it resolves entries in, a `ConstantValue:` attribute's value, and a bootstrap
    * method's argument after its entry (`#n text`, a concatenation's recipe among them). */
  def poolless(line: String): String =
    val argument = """(\s*)#\d+((?: .*)?)""".r
    if line.trim.startsWith("ConstantValue:") then line
    else
      line match
        case argument(indent, rest) => s"$indent#$rest"
        case _ => references(line)

  private def references(line: String): String =
    val out = new StringBuilder
    var i = 0
    var quoted = false
    while i < line.length do
      val c = line.charAt(i)
      if quoted then
        out += c
        if c == '\\' && i + 1 < line.length then
          out += line.charAt(i + 1)
          i += 1
        else if c == '"' then quoted = false
        i += 1
      else if c == '"' then
        quoted = true
        out += c
        i += 1
      else if line.startsWith("//", i) then
        out ++= line.substring(i)
        i = line.length
      else if c == '#' && i + 1 < line.length && line.charAt(i + 1).isDigit then
        out += '#'
        i += 1
        while i < line.length && line.charAt(i).isDigit do i += 1
      else
        out += c
        i += 1
    out.toString

  /** The comparison's own check, before any scenario: the pool's numbering is left out, a
    * literal's text is not, wherever `javap` prints it. */
  def check(): Unit =
    val same = Seq(
      ("  1: ldc #19 // String a", "  1: ldc #23 // String a"),
      ("      0: #15(#16=s#17)", "      0: #18(#19=s#20)"),
      ("      #43 (Ljava/lang/Object;)Ljava/lang/Object;", "      #47 (Ljava/lang/Object;)Ljava/lang/Object;"),
    )
    val other = Seq(
      ("  1: ldc #19 // String #1", "  1: ldc #19 // String #2"),
      ("      description=\"#1\"", "      description=\"#2\""),
      ("      value=\"a//#1\"", "      value=\"a//#2\""),
      ("    ConstantValue: String #1", "    ConstantValue: String #2"),
      ("      #22 \u0001#1", "      #22 \u0001#2"),
    )
    val broken = same.filter((a, b) => poolless(a) != poolless(b)) ++ other.filter((a, b) => poolless(a) == poolless(b))
    if broken.nonEmpty then
      throw new MessageOnlyException(s"the class files' comparison is broken: it keeps the pool's numbering or loses a literal's text (${broken.map(_._1.trim).mkString("; ")})")

  /** The bytes after a TASTy header's identifier: the magic number, the three versions, the
    * tooling's version and the sixteen bytes of the identifier. */
  def tasty(bytes: Array[Byte]): Array[Byte] =
    var i = 4
    def nat(): Long =
      var x = 0L
      var b = 0
      while
        b = bytes(i) & 0xff
        i += 1
        x = (x << 7) | (b & 0x7f)
        (b & 0x80) == 0
      do ()
      x
    try
      nat(); nat(); nat()
      val tooling = nat().toInt
      i += tooling + 16
      bytes.drop(i)
    catch case _: IndexOutOfBoundsException => bytes
