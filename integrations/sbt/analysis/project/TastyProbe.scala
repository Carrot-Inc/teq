package sbt.internal.teq

import java.io.File

import scala.sys.process.*

import sbt.*
import sbt.internal.inc.{Analysis, CompileOutput, Incremental, InvalidationZinc, JarUtils, Stamps}
import sbt.util.Logger
import xsbti.{VirtualFile, VirtualFileRef}
import xsbti.compile.{CompileOrder, MiniOptions, MiniSetup}

import dev.teq.sbt.Json

/** What zinc makes of a build against a product directory that holds TASTy files and no class
  * files (a Scala.js module's `teq compiler check --products`, against which teq refuses
  * `--analysis-version 3`): an upstream module and a downstream one are built for the JVM with
  * `--analysis-version 3`, the upstream's class files taken away after each of its builds, which
  * leaves the answers naming class files a TASTy-only directory lacks. Each module is compiled by
  * zinc's `Incremental.apply` with sbt's options and sbt's lookup (the oracle's, `InvalidationZinc`)
  * over a class path of the module's class directory, the upstream's and the jars; the compile
  * callback builds the module with teq into its class directory and feeds the answer through the
  * adapter. The downstream is compiled, then compiled again from that state with the upstream as
  * it was and after a class the downstream extends gains a member, each with the class path's
  * hash unchanged and changed (a directory's hash is empty, so only a jar's change changes it),
  * with the answer's entries as teq names them (the `.class` beside each `.tasty`) and with the
  * `.tasty` files in their place, beside the upstream's class files kept as a control; and with
  * the upstream's analysis on the class path (an sbt project upstream) and without it (a
  * directory zinc knows no analysis of). Each run's sources compiled, initial changes, libraries
  * and external dependencies are logged. */
object TastyProbe:
  def run(log: Logger, converter: xsbti.FileConverter, work: File, up: File, down: File, teq: String, jars: Seq[File]): Unit =
    IO.delete(work)
    val upDir = work / "up"
    val upFull = work / "up-full"
    val downDir = work / "down"
    val downFirst = work / "down-first"
    val upSrc = work / "up-src"
    IO.copyDirectory(up, upSrc)
    def classpath(dir: File, upstream: Seq[File]): Seq[VirtualFile] = ((dir +: upstream) ++ jars).map(f => converter.toVirtualFile(f.toPath))

    var keep = false
    def restoreClasses(): Unit = for f <- (upFull ** "*.class").get() do IO.copyFile(f, upDir / IO.relativize(upFull, f).get)

    def upstream(previous: Analysis): Analysis =
      val (analysis, _, _) = compile(log, converter, upSrc, upDir, classpath(upDir, Nil), None, previous, changed = false, work / "bak-up") { () =>
        val answer = build(teq, upSrc, upDir, jars)
        IO.delete(upFull)
        IO.copyDirectory(upDir, upFull)
        if !keep then tastyOnly(upDir)
        Json.parse(answer)
      }
      analysis

    def downstream(naming: String, analysed: Boolean, previous: Analysis, upAnalysis: Analysis, changed: Boolean): (Analysis, Seq[String], InvalidationZinc.Recorder) =
      val upstreamAnalysis = if analysed then Some(upDir -> upAnalysis) else None
      compile(log, converter, down, downDir, classpath(downDir, Seq(upDir)), upstreamAnalysis, previous, changed, work / "bak-down") { () =>
        restoreClasses()
        val answer = try build(teq, down, downDir, upDir +: jars) finally if !keep then tastyOnly(upDir)
        Json.parse(if naming == "tasty" then withTasty(answer) else answer)
      }

    var current = upstream(Analysis.empty)
    for naming <- Seq("class", "tasty", "class files kept"); analysed <- Seq(true, false) do
      keep = naming == "class files kept"
      if keep then restoreClasses() else tastyOnly(upDir)
      val label = s"$naming, ${if analysed then "the upstream's analysis on the class path" else "no analysis of the upstream"}"
      IO.delete(downDir)
      val (first, compiled0, recorder0) = downstream(naming, analysed, Analysis.empty, current, changed = false)
      report(log, s"$label, first build", first, compiled0, recorder0)
      IO.delete(downFirst)
      IO.copyDirectory(downDir, downFirst, preserveLastModified = true)
      def again(what: String, upAnalysis: Analysis, changed: Boolean): Unit =
        IO.delete(downDir)
        IO.copyDirectory(downFirst, downDir, preserveLastModified = true)
        val (analysis, compiled, recorder) = downstream(naming, analysed, first, upAnalysis, changed)
        report(log, s"$label, $what, class path hash ${if changed then "changed" else "unchanged"}", analysis, compiled, recorder)
      for changed <- Seq(false, true) do again("upstream unchanged", current, changed)
      // The upstream changed: a class the downstream extends gains a member.
      val base = upSrc / "A.scala"
      IO.write(base, IO.read(base).replace("  def show: String = \"n=\" + n", "  def show: String = \"n=\" + n\n  def extra: Int = 1"))
      val edited = upstream(current)
      for changed <- Seq(false, true) do again("upstream changed", edited, changed)
      IO.copyDirectory(up, upSrc, overwrite = true)
      current = upstream(edited)

  /** `Incremental.apply` of the module whose sources are under `src` over `previous`, with sbt's
    * options and lookup, the upstream's analysis that of its class directory, and the class
    * path's hash changed or not; `answer` builds the batch and gives teq's answer. */
  private def compile(log: Logger, converter: xsbti.FileConverter, src: File, classDir: File, classpath: Seq[VirtualFile], upstream: Option[(File, Analysis)],
      previous: Analysis, changed: Boolean, backup: File)(answer: () => Json.Value): (Analysis, Seq[String], InvalidationZinc.Recorder) =
    val root = new File(".").getAbsoluteFile
    val sources = (src ** "*.scala").get().map(f => converter.toVirtualFile(f.toPath))
    val analysisOf = upstream.map((d, a) => converter.toVirtualFile(d.toPath).id -> a).toMap
    val hash = InvalidationZinc.classpathHash(classpath, converter)
    val lookup = new InvalidationZinc.SbtLookup(classpath, e => analysisOf.get(e.id), if changed then Vector.empty else hash, hash)
    val name = (r: VirtualFileRef) => new File(r.id).getName
    val recorder = new InvalidationZinc.Recorder(name, name)
    val output = CompileOutput(classDir.toPath)
    val setup = MiniSetup.of(output, MiniOptions.of(hash.toArray, Array(), Array()), "teq", CompileOrder.Mixed, true, Array())
    IO.createDirectory(classDir)
    var compiled = Seq.empty[String]
    val (_, analysis) = Incremental.apply(
      sources.toSet, converter, lookup, previous, InvalidationZinc.options(3, 0.5, false, recorder, backup, log), setup, Stamps.timeWrapBinaryStamps(converter),
      output, JarUtils.createOutputJarContent(output), None, None, None, log,
    ) { (batch, _, callback, _) =>
      compiled = batch.toSeq.map(name).sorted
      val a = answer()
      val fed = a("api")("files").items.map(f => converter.toVirtualFile(TeqAnalysis.absolute(root, f("file").str).toPath).id)
      val ids = sources.map(_.id).toSet
      for f <- fed if !ids.contains(f) do throw new MessageOnlyException(s"tastyProbe: teq's answer names $f, which is not among zinc's sources ${ids.mkString(", ")}")
      TeqAnalysis.feed(a, root, classDir, callback, converter)
    }
    (analysis, compiled, recorder)

  private def build(teq: String, src: File, products: File, classpath: Seq[File]): String =
    val cp = Seq("--classpath", classpath.map(_.getAbsolutePath).mkString(File.pathSeparator))
    val command = Seq(teq, "compiler", "build", "--target", "jvm", "--std=scala-library", "--products", products.getAbsolutePath) ++ cp ++ Seq("--analysis-version", "3", src.getAbsolutePath)
    try Process(command).!!
    catch case e: RuntimeException => throw new MessageOnlyException(s"${command.mkString(" ")}: ${e.getMessage}")

  /** The product directory as a check leaves it: its TASTy files alone. */
  private def tastyOnly(dir: File): Unit =
    IO.delete((dir ** "*.class").get())

  /** The answer with each entry that names a `.class` no file holds named by its `.tasty`. */
  private def withTasty(text: String): String =
    val entries = Json.parse(text)("api")("entries").items.map(_.str)
    entries.foldLeft(text) { (t, e) =>
      if e.endsWith(".class") && !new File(e).exists && new File(e.stripSuffix(".class") + ".tasty").exists then
        t.replace(Json.string(e), Json.string(e.stripSuffix(".class") + ".tasty"))
      else t
    }

  private def report(log: Logger, what: String, analysis: Analysis, compiled: Seq[String], recorder: InvalidationZinc.Recorder): Unit =
    val libraries = analysis.relations.allLibraryDeps.toSeq.map(_.id).sorted
    val ofProducts = libraries.filter(_.contains("/up/")).map(l => s"${new File(l).getName}: ${analysis.stamps.library(VirtualFileRef.of(l))}")
    val external = analysis.relations.allExternalDeps.toSeq.sorted
    val initial = recorder.initial.fold("none") { i =>
      val changes = i.external.map(c => s"${c.kind}(${(c.cls +: c.names.map(_._1)).mkString(" ")})")
      s"external ${changes.mkString(", ")}; libraries ${i.libraryDeps.mkString(", ")}; products ${i.removedProducts.mkString(", ")}"
    }
    log.info(s"tastyProbe $what: zinc compiles ${compiled.size} of the downstream's sources (${compiled.mkString(", ")}); initial changes: $initial; " +
      s"libraries in the product directory: ${ofProducts.mkString("; ")}; external class dependencies: ${external.mkString(", ")}")
