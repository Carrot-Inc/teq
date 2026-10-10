package sbt.internal.teq

import java.io.File

import scala.sys.process.*

import sbt.*
import sbt.util.Logger
import xsbti.{AnalysisCallback, FileConverter}
import xsbti.compile.ClassFileManager

import dev.teq.sbt.Json

/** The test-only incremental adapter of the invalidation oracle (`Invalidation`), teq as zinc's
  * Scala compiler for the batch zinc requests, as the plugin's compile runs it (`TeqCompile`,
  * docs/TARGETS.md "The contract between the plugin and the compiler"): one `teq compiler build` of the
  * batch's sources into the module's own class directory, first on the class path, then the
  * upstream modules' and the jars (sbt's order), with every source the directory's manifest
  * names and the module no longer has removed; the compiler publishes the batch's products and
  * the manifest once the batch passed. The answer's journal is enrolled with zinc's class-file
  * manager, which the oracle's compile function is handed (the plugin's journal stands for it,
  * since sbt hands its compiler none), and the answer reaches zinc's callback through the
  * plugin's adapter (`TeqAnalysis.feed`); a check's classes get the plugin's stamps. A run that
  * compiled no batch of a removed source drops it from the manifest with the manifest's own
  * operation (`reconcile`); the manifest itself is outside zinc's transaction, so the oracle
  * keeps it and restores it after a failed run (`Invalidation`). */
object TeqBatch:
  final case class Problem(file: String, line: Int, message: String)
  final class Failed(val problems: Seq[Problem], message: String) extends Exception(message)

  val manifestName = TeqCompile.manifestName

  private def command(teq: String, check: Boolean, classDir: File, classpath: Seq[File], root: File, removed: Seq[String]): Seq[String] =
    val build = if check then Seq("check") else Seq("build", "--target", "jvm")
    Seq(teq, "compiler") ++ build ++ Seq("--std=scala-library", "--products", classDir.getAbsolutePath,
      "--classpath", (classDir +: classpath).map(_.getAbsolutePath).mkString(File.pathSeparator), "--sourceroot", root.getAbsolutePath,
      "--analysis-version", TeqAnalysis.version.toString) ++ removed.flatMap(Seq("--removed", _))

  private def run(command: Seq[String], root: File): (Int, Json.Value, String) =
    val stdout = new StringBuilder
    val stderr = new StringBuilder
    val code = Process(command, root).!(ProcessLogger(line => stdout ++= line ++= "\n", line => stderr ++= line ++= "\n"))
    val answer = scala.util.Try(Json.parse(stdout.result().trim.linesIterator.toSeq.lastOption.getOrElse(""))).getOrElse(Json.Null)
    (code, answer, stderr.result())

  /** One batch of the module whose current sources are `sources`; with `check`, a check build
    * of the products, its pickles alone. */
  def compile(teq: String, check: Boolean, batch: Seq[File], sources: Seq[File], root: File, classDir: File, classpath: Seq[File], callback: AnalysisCallback,
      manager: ClassFileManager, converter: FileConverter, log: Logger): Unit =
    val removed = TeqCompile.removed(classDir, root, sources)
    val (code, answer, stderr) = run(command(teq, check, classDir, classpath, root, removed) ++ batch.map(_.getAbsolutePath), root)
    if code != 0 || answer("ok") != Json.Bool(true) then
      val problems = errors(stderr, root)
      throw new Failed(problems, s"teq failed ($code) on ${batch.map(_.getName).mkString(" ")}: ${stderr.linesIterator.take(6).mkString(" | ")}")
    val stamps = if check then TeqCompile.Stamps.write(answer, root, classDir) else Nil
    manager.generated((TeqAnalysis.generatedFiles(answer, classDir) ++ stamps).map(f => converter.toVirtualFile(f.toPath)).toArray)
    TeqAnalysis.feed(answer, root, classDir, callback, converter, if check then TeqAnalysis.Products.Stamps else TeqAnalysis.Products.ClassFiles)

  /** After a run, the manifest without the sources the module no longer has, which a run that
    * compiled no batch has not dropped. */
  def reconcile(teq: String, check: Boolean, sources: Seq[File], root: File, classDir: File, classpath: Seq[File]): Unit =
    val removed = TeqCompile.removed(classDir, root, sources)
    if removed.nonEmpty then
      val (code, answer, stderr) = run(command(teq, check, classDir, classpath, root, removed), root)
      if code != 0 || answer("ok") != Json.Bool(true) then
        throw new MessageOnlyException(s"teq's manifest operation failed ($code) on ${removed.mkString(" ")}: ${stderr.linesIterator.take(6).mkString(" | ")}")

  /** teq's diagnostics, `<file>:<line>:<column>: error: <message>`, with the file under `root`. */
  def errors(stderr: String, root: File): Seq[Problem] =
    val error = """^(.+?):(\d+):(\d+): error: (.*)$""".r
    val base = root.getAbsoluteFile.toPath.normalize
    stderr.linesIterator.toSeq.collect { case error(file, line, _, message) =>
      val p = new File(file).getAbsoluteFile.toPath.normalize
      Problem(if p.startsWith(base) then base.relativize(p).toString else file, line.toInt, message)
    }

  def render(v: Json.Value): String = v match
    case Json.Obj(fields) => fields.toSeq.sortBy(_._1).map((k, x) => Json.string(k) + ":" + render(x)).mkString("{", ",", "}")
    case Json.Arr(items) => items.map(render).mkString("[", ",", "]")
    case Json.Str(s) => Json.string(s)
    case Json.Num(n) => n
    case Json.Bool(b) => b.toString
    case Json.Null => "null"
