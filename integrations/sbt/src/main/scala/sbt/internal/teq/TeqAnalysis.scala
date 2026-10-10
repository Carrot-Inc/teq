package sbt.internal.teq

import java.io.File
import java.net.URI
import java.nio.file.{FileSystems, Path, Paths}
import java.util.EnumSet
import scala.collection.compat.*

import sbt.internal.inc.{Lookup, NoopExternalLookup}
import xsbti.{AnalysisCallback, FileConverter, UseScope, VirtualFileRef}
import xsbti.api.DependencyContext
import xsbti.compile.{CompileAnalysis, FileHash}

import dev.teq.sbt.Json

/** An answer of teq's with `analysisVersion` 2 or 3 handed to zinc's `AnalysisCallback` as
  * scalac's bridge hands it the analysis of a compile: per file its classes' APIs (`ApiGraph`),
  * under version 3 the dependencies its classes' code has (`deps`), its entry points and its
  * products, so that zinc computes the hashes and builds its `Analysis` itself: the compile under
  * `teqCompiler` (`TeqCompile.Compiler`) and the invalidation oracle's batches. */
object TeqAnalysis {
  /** The versions this adapter reads, and the one it asks for: 2 is the graph alone, 3 the graph
    * and the dependencies. */
  val versions = Set(2, 3)
  val version = 3

  /** What a class of a file depends on, as zinc's callback takes it: the names it uses with
    * their scopes, the classes of the same compile it depends on, and the classes of the class
    * path, each with the binary entry that holds it (a jar, a class file) and its binary name. */
  final case class Dependencies(
      names: Seq[(String, EnumSet[UseScope])],
      classes: Seq[(String, DependencyContext)],
      binaries: Seq[(Path, String, DependencyContext)],
  )

  /** A file of a passing answer: its graph, its classes' dependencies by class (none under
    * version 2), and its local classes' class files and its entry points as the discovery
    * analysis names them. */
  final case class FileAnalysis(graph: ApiGraph.File, deps: Seq[(String, Dependencies)], local: Seq[String], mains: Seq[String])

  /** What a class's non-local product is: a JVM build's class file, or the stamp the plugin
    * writes for a Scala.js check's class (`TeqCompile.Stamps`). */
  sealed abstract class Products {
    def file(classDir: File, binary: String): File =
      new File(classDir, binary.replace('.', '/') + (if (this == Products.ClassFiles) ".class" else ".teq"))
  }
  object Products {
    case object ClassFiles extends Products
    case object Stamps extends Products
  }

  /** Calls `callback` with what `answer` says of each file it has a graph for; `classDir` is
    * where the answer's products are named from. In the order of scalac's phases: every file's
    * classes (`api`), then every file's dependencies, then every file's products and entry
    * points: a class's product (`products`), each local class's class file, and every other file
    * the answer's journal (`generated`) lists under the file, a `.tasty` among them, as a local
    * product, so that zinc's class-file manager moves it aside with the source. Nothing reaches
    * the callback unless the whole answer passes `checked`. */
  def feed(answer: Json.Value, root: File, classDir: File, callback: AnalysisCallback, converter: FileConverter, products: Products = Products.ClassFiles): Unit = {
    val files = checked(answer).map(f => (f, converter.toVirtualFile(absolute(root, f.graph.path).toPath)))
    val journal = generated(answer, classDir)
    for ((f, source) <- files) {
      callback.startSource(source)
      f.graph.classes.foreach(callback.api(source, _))
    }
    for ((f, source) <- files; (from, d) <- f.deps) {
      for ((name, scopes) <- d.names) callback.usedName(from, name, scopes)
      for ((on, context) <- d.classes) callback.classDependency(on, from, context)
      for ((entry, binary, context) <- d.binaries) callback.binaryDependency(entry, binary, from, source, context)
    }
    for ((f, source) <- files) {
      val nonLocal = f.graph.products.map{ case (name, binary) => (name, binary, products.file(classDir, binary))}
      for ((name, binary, file) <- nonLocal) callback.generatedNonLocalClass(source, file.toPath, binary, name)
      val local = f.local.map(path => new File(classDir, path))
      local.foreach(file => callback.generatedLocalClass(source, file.toPath))
      val registered = (nonLocal.map(_._3) ++ local).map(_.getAbsoluteFile).toSet
      for (file <- journal.getOrElse(absolute(root, f.graph.path).getCanonicalFile, Nil) if !registered(file.getAbsoluteFile))
        callback.generatedLocalClass(source, file.toPath)
      f.mains.foreach(callback.mainClass(source, _))
    }
  }

  /** The answer's journal (`generated`): every file the build wrote, by the source that owns it,
    * the runtime classes (no source's) left out; none in an answer without one. */
  def generated(answer: Json.Value, classDir: File): Map[File, Seq[File]] =
    answer("generated") match {
      case Json.Arr(rows) =>
        rows.collect {
          case row if row("source") != Json.Null =>
            new File(row("source").str).getCanonicalFile -> row("files").strings.map(path => new File(classDir, path))
        }.groupMapReduce(_._1)(_._2)(_ ++ _)
      case _ => Map.empty
    }

  /** Every file of the journal's source rows. */
  def generatedFiles(answer: Json.Value, classDir: File): Seq[File] =
    generated(answer, classDir).values.flatten.toSeq

  /** The files of `answer`, refused unless the answer is of a version this adapter reads, of a
    * build that passed, with its discovery analysis and a graph of the shape `ApiGraph.read`
    * reads, every product naming a class of its file's graph, and under version 3 every file
    * with its dependencies, each keyed by a class of the file's graph: a failed build, or one
    * whose classes teq could not state (`apiErrors`), is not an empty compile, and a file
    * without its dependencies is not a file without any. */
  def checked(answer: Json.Value): Seq[FileAnalysis] = {
    val v = answer("analysisVersion") match {
      case Json.Num(n) if n.toIntOption.exists(versions) => n.toInt
      case Json.Null => throw new ApiGraph.Malformed("the answer carries no analysis graph (analysisVersion)")
      case other => throw new ApiGraph.Malformed(s"analysisVersion ${other.str} is not a version this plugin reads (${versions.toSeq.sorted.mkString(", ")})")
    }
    answer("apiErrors") match {
      case Json.Null => ()
      case errors => throw new ApiGraph.Malformed(s"teq could not state the classes' API: ${errors.strings.mkString("; ")}")
    }
    if (answer("ok") != Json.Bool(true)) throw new ApiGraph.Malformed("the answer is a failed build's, which has no analysis")
    val discovery = ApiGraph.array(answer("analysis"), "the answer's discovery analysis (analysis)").map { f =>
      val path = ApiGraph.string(f("file"), "a discovery entry's file")
      val local = ApiGraph.array(f("local"), s"$path's local classes").map(ApiGraph.string(_, s"a local class of $path"))
      val mains = ApiGraph.array(f("classes"), s"$path's classes").filter(c => ApiGraph.boolean(c("main"), s"a class of $path's main")).map(c => ApiGraph.string(c("name"), s"a class of $path's name"))
      path -> (local, mains.distinct)
    }.toMap
    val (files, fileEntries, binaryEntries) = answer("api") match {
      case Json.Null => throw new ApiGraph.Malformed("the answer carries no analysis graph (api)")
      case api =>
        val binaries = (v, api("entries")) match {
          case (2, Json.Null) => IndexedSeq.empty
          case (2, _) => throw new ApiGraph.Malformed("an answer of version 2 has no dependencies' entries (entries)")
          case (_, Json.Null) => throw new ApiGraph.Malformed("the answer carries no dependencies' entries (entries)")
          case (_, e) => ApiGraph.array(e, "the dependencies' entries (entries)").map(x => binaryEntry(ApiGraph.string(x, "an entry of the dependencies"))).toIndexedSeq
        }
        (ApiGraph.read(api), ApiGraph.array(api("files"), "the graph's files"), binaries)
    }
    for ((graph, entry) <- files.zip(fileEntries)) yield {
      val named = graph.classes.map(_.name).toSet
      for ((name, binary) <- graph.products)
        // zinc hashes every class a product names, so a product needs its class's API.
        if (!named.contains(name)) throw new ApiGraph.Malformed(s"${graph.path}: the product $binary names $name, which the graph has no API for")
      val deps = (v, entry("deps")) match {
        case (2, Json.Null) => Nil
        case (2, _) => throw new ApiGraph.Malformed(s"${graph.path}: an answer of version 2 has no dependencies (deps)")
        case (_, Json.Null) => throw new ApiGraph.Malformed(s"${graph.path} carries no dependencies (deps)")
        case (_, Json.Obj(byClass)) =>
          byClass.toSeq.sortBy(_._1).map { case (from, d) =>
            if (!named.contains(from)) throw new ApiGraph.Malformed(s"${graph.path}: the dependencies of $from, which the graph has no API for")
            from -> dependencies(d, s"${graph.path}'s dependencies of $from", binaryEntries)
          }
        case _ => throw new ApiGraph.Malformed(s"${graph.path}'s dependencies (deps) are not an object")
      }
      val (local, mains) = discovery.getOrElse(graph.path, (Nil, Nil))
      FileAnalysis(graph, deps, local, mains)
    }
  }

  /** A class's dependencies: `{"names":[name...],"patmat":[name...],"classes":[[class,context]...],
    * "binaries":[[entry,binaryName,context]...]}`, a name used with the scope `Default` or,
    * under `patmat`, with `Default` and `PatMatTarget`, a binary's entry an index into the
    * answer's `entries`; each tuple once, a name once in each list. */
  private def dependencies(d: Json.Value, what: String, entries: IndexedSeq[java.nio.file.Path]): Dependencies = {
    d match {
      case Json.Obj(fields) if fields.keySet == Set("names", "patmat", "classes", "binaries") => ()
      case _ => throw new ApiGraph.Malformed(s"$what are not an object of names, patmat, classes and binaries")
    }
    def tuples[T](field: String, arity: Int)(read: Seq[Json.Value] => T): Seq[T] = {
      val items = ApiGraph.array(d(field), s"$what: $field").map { t =>
        val fields = ApiGraph.array(t, s"$what: an entry of $field")
        if (fields.length != arity) throw new ApiGraph.Malformed(s"$what: an entry of $field has ${fields.length} fields, not $arity")
        read(fields)
      }
      if (items.distinct.length != items.length) throw new ApiGraph.Malformed(s"$what: $field names an entry twice")
      items
    }
    def used(field: String, scopes: UseScope*): Seq[(String, EnumSet[UseScope])] =
      ApiGraph.array(d(field), s"$what: $field").map { n =>
        val set = EnumSet.noneOf(classOf[UseScope])
        scopes.foreach(set.add)
        (ApiGraph.string(n, s"$what: a name of $field"), set)
      }
    val (plain, patmat) = (used("names", UseScope.Default), used("patmat", UseScope.Default, UseScope.PatMatTarget))
    // A name is a term's or a type's to scalac, each used with its scopes: a sealed class's type
    // name under `patmat` and its companion's term name under `names` are two uses.
    for (list <- Seq(plain, patmat))
      if (list.map(_._1).distinct.length != list.length) throw new ApiGraph.Malformed(s"$what: names a name twice in one list")
    val names = plain ++ patmat
    Dependencies(
      names,
      tuples("classes", 2) { case Seq(on, context) =>
        (ApiGraph.string(on, s"$what: a class depended on"), constant(context, s"$what: a context")(DependencyContext.valueOf))
      },
      tuples("binaries", 3) { case Seq(entry, binary, context) =>
        val i = entry match {
          case Json.Num(n) => n.toIntOption.filter(i => i >= 0 && i < entries.length).getOrElse(throw new ApiGraph.Malformed(s"$what: $n is no entry of the answer's"))
          case _ => throw new ApiGraph.Malformed(s"$what: a binary's entry is not an index")
        }
        (entries(i), ApiGraph.string(binary, s"$what: a binary class name"), constant(context, s"$what: a context")(DependencyContext.valueOf))
      },
    )
  }

  private def constant[T](v: Json.Value, what: => String)(valueOf: String => T): T = {
    val name = ApiGraph.string(v, what)
    try valueOf(name)
    catch { case _: IllegalArgumentException => throw new ApiGraph.Malformed(s"$what: $name is not a constant of its enum") }
  }

  /** The file a binary dependency names: a jar or a class file, or with `jrt:` a class file of
    * the JDK's runtime image, where scalac's class path finds the JDK's classes. */
  def binaryEntry(entry: String): Path =
    if (entry.startsWith("jrt:")) FileSystems.getFileSystem(URI.create("jrt:/")).getPath(entry.stripPrefix("jrt:"))
    else Paths.get(entry)

  def absolute(root: File, path: String): File = {
    val f = new File(path)
    if (f.isAbsolute) f else new File(root, path)
  }

  /** A lookup that knows no other analysis: every class of the class path is outside the
    * compile. */
  val noLookup: Lookup = new Lookup with NoopExternalLookup {
    def changedClasspathHash: Option[Vector[FileHash]] = None
    def analyses: Vector[CompileAnalysis] = Vector.empty
    def lookupOnClasspath(binaryClassName: String): Option[VirtualFileRef] = None
    def lookupAnalysis(binaryClassName: String): Option[CompileAnalysis] = None
  }
}
