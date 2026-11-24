//> using scala 3.8.4
//> using dep org.scala-lang::scala3-compiler:3.8.4

// tests/support/scalac_cells.scala: the scalac 3.8.4 leg of a table whose cells are libraries and users
// (tests/support/pkgobj_given_import.py), many cells in one JVM. A cell is <cells>/<cell>/lib/*.scala and
// use/*.scala; it is built WHOLE (one compilation of both) and SPLIT (lib, then use with lib's classes on the class
// path), each build into a directory of its own, and its entry point (`fd.run`, or the one <cell>/main.txt names) is run
// in this JVM by a class loader of its own, its output captured. Appends to <cells>/scalac.tsv, per cell and build:
//   cell <TAB> whole|split <TAB> run <TAB> ok:<output, lines joined by |> | err:<file>:<line>:<col>: <message's first line>
// the first error scalac reports (columns from 1, as teq's), with scalac's console filters (UniqueMessagePositions,
// HideNonSensicalMessages), or the exception the run throws.
//
// With `--over <tag>`, scalac compiles each cell's user over teq's library instead, the products of the split build
// tests/support/pkgobj-given-import.sh's run <tag> left (<cells>/teq-<tag>/<cell>/split/lib, class files and TASTy),
// and runs it beside them: `cell <TAB> split <TAB> over <TAB> <verdict>` lines in <cells>/scalac-over-<tag>.tsv.
//
//   scala-cli run tests/support/scalac_cells.scala --server=false -q -- <cells dir> <out dir> <scala-library jar> [<first> <last>] [--over <tag>]

import dotty.tools.dotc.Driver
import dotty.tools.dotc.core.Contexts.*
import dotty.tools.dotc.reporting.{Diagnostic, Reporter, UniqueMessagePositions, HideNonSensicalMessages}
import dotty.tools.dotc.interfaces.Diagnostic.ERROR
import java.io.{ByteArrayOutputStream, PrintStream}
import java.net.URLClassLoader
import java.nio.file.{Files, Path, Paths}
import scala.jdk.CollectionConverters.*

class FirstError extends Reporter with UniqueMessagePositions with HideNonSensicalMessages:
  var first: Option[String] = None
  def doReport(d: Diagnostic)(using Context): Unit =
    if d.level == ERROR && first.isEmpty then
      val where = if d.pos.exists then s"${d.pos.source.file.name}:${d.pos.line + 1}:${d.pos.column + 1}" else "-:-:-"
      val msg = try d.msg.message.linesIterator.nextOption().getOrElse("") catch case e: Throwable => s"<message failed: $e>"
      first = Some(s"$where: ${msg.replace('\t', ' ')}".take(240))

def sources(dir: Path): Seq[String] =
  if !Files.isDirectory(dir) then Nil
  else Files.list(dir).iterator().asScala.map(_.toString).filter(_.endsWith(".scala")).toSeq.sorted

def compile(files: Seq[String], out: Path, classpath: String): Option[String] =
  Files.createDirectories(out)
  val reporter = new FirstError
  val args = Array("-d", out.toString, "-classpath", classpath, "-color:never") ++ files
  try
    new Driver().process(args, reporter)
    if reporter.hasErrors then Some(reporter.first.getOrElse("-:-:-: an error without a message")) else None
  catch case e: Throwable => Some(s"-:-:-: scalac crashed: $e".take(240))

def run(dirs: Seq[Path], main: String): String =
  val loader = new URLClassLoader(dirs.map(_.toUri.toURL).toArray, classOf[FirstError].getClassLoader)
  val bytes = new ByteArrayOutputStream
  val printed = new PrintStream(bytes, true, "UTF-8")
  try
    Console.withOut(printed) {
      loader.loadClass(main).getMethod("main", classOf[Array[String]]).invoke(null, Array.empty[String])
    }
    printed.flush()
    "ok:" + bytes.toString("UTF-8").linesIterator.mkString("|")
  catch
    case e: java.lang.reflect.InvocationTargetException => s"err:-:-:-: ${e.getCause}".take(240).replace('\t', ' ')
    case e: Throwable => s"err:-:-:-: $e".take(240).replace('\t', ' ')
  finally loader.close()

@main def scalacCells(cellsDir: String, outDir: String, library: String, rest: String*): Unit =
  val cells = Paths.get(cellsDir)
  val names = Files.readAllLines(cells.resolve("cells.txt")).asScala.map(_.trim).filter(_.nonEmpty).toSeq
  val over = rest.sliding(2).collectFirst { case Seq("--over", tag) => tag }
  val range = rest.filterNot(a => a == "--over" || over.contains(a))
  val (first, last) = range match
    case Seq(a, b) => (a.toInt, b.toInt)
    case _ => (1, names.size)
  val tableName = over.fold("scalac.tsv")(tag => s"scalac-over-$tag.tsv")
  val table = new java.io.PrintWriter(Files.newBufferedWriter(cells.resolve(tableName), java.nio.file.StandardOpenOption.CREATE, java.nio.file.StandardOpenOption.APPEND))
  for name <- names.slice(first - 1, last) do
    val lib = sources(cells.resolve(name).resolve("lib"))
    val use = sources(cells.resolve(name).resolve("use"))
    val out = Paths.get(outDir, name)
    val mainFile = cells.resolve(name).resolve("main.txt")
    val main = if Files.exists(mainFile) then Files.readString(mainFile).trim else "fd.run"
    over match
      case Some(tag) =>
        val teqLib = cells.resolve(s"teq-$tag").resolve(name).resolve("split").resolve("lib")
        val verdict =
          if !Files.isDirectory(teqLib) then "err:-:-:-: no products of teq's library"
          else compile(use, out.resolve(s"over-$tag"), s"$library:$teqLib") match
            case Some(e) => s"err:$e"
            case None => run(Seq(out.resolve(s"over-$tag"), teqLib), main)
        table.println(s"$name\tsplit\tover\t$verdict")
        table.flush()
      case None =>
        val whole = compile(lib ++ use, out.resolve("whole"), library) match
          case Some(e) => s"err:$e"
          case None => run(Seq(out.resolve("whole")), main)
        val split = compile(lib, out.resolve("lib"), library) match
          case Some(e) => s"err:$e"
          case None => compile(use, out.resolve("use"), s"$library:${out.resolve("lib")}") match
            case Some(e) => s"err:$e"
            case None => run(Seq(out.resolve("use"), out.resolve("lib")), main)
        table.println(s"$name\twhole\trun\t$whole")
        table.println(s"$name\tsplit\trun\t$split")
        table.flush()
  table.close()
  println(s"scalac_cells: cells $first-$last of ${names.size}")
