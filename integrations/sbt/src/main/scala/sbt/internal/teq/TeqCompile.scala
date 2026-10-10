package sbt.internal.teq

import java.io.File
import java.util.Optional

import scala.collection.mutable
import scala.collection.compat.*

import sbt.*
import sbt.internal.inc.CompileFailed
import sbt.util.Logger
import xsbti.{AnalysisCallback, FileConverter, Position, Problem, Reporter, Severity, VirtualFile}
import xsbti.compile.{ClassFileManager, ClasspathOptions, CompileProgress, DependencyChanges, ExternalHooks, IncOptions, Output, ScalaCompiler, ScalaInstance}

import dev.teq.sbt.{ArgsFile, Json}

/** teq as zinc's Scala compiler for a configuration under `teqCompiler` (docs/TARGETS.md, "The
  * module model"): zinc keeps its invalidation, its analysis and its class-file manager, and
  * calls `Compiler` with the sources it invalidated; each call is one `teq` process over them
  * against the configuration's own class directory, which the products of the sources it does
  * not list stay in, and the upstream configurations' directories and jars, a JVM build of
  * class files and TASTy or a Scala.js project's check, which writes TASTy alone. The answer
  * feeds zinc's callback (`TeqAnalysis.feed`). Nothing is kept between compiles but what is on
  * disk. The contract with the compiler is docs/TARGETS.md's "The contract between the plugin
  * and the compiler": the compiler publishes a batch's products and rewrites the products'
  * manifest (`teq-products.json`) only once the batch passed; the plugin owns the run's
  * rollback (`Journal`) and the manifest's removals, which zinc never calls a compiler for
  * (`reconcile`). This object lives under `sbt.` for the internals it reads. */
object TeqCompile {
  val manifestName = "teq-products.json"

  /** The format of the products manifest this plugin reads, a protocol between the plugin and the compiler
    * (docs/TOOLING.md, "Releases"): 1, a manifest's that names none. A manifest of another is refused before the
    * class directory is touched, naming the teq that wrote it, rather than read as owning no product. */
  val manifestFormat = 1

  /** The manifest `m` of `file`, refused unless of `manifestFormat`. */
  private[teq] def readable(m: Json.Value, file: File): Json.Value =
    m("format") match {
      case Json.Null => m
      case Json.Num(n) if n.toIntOption.contains(manifestFormat) => m
      case other =>
        throw new MessageOnlyException(s"teq: $file is a products manifest of format ${other.str}, which teq ${Option(m("teq").str).filter(_.nonEmpty).getOrElse("(unknown)")} wrote and this sbt-teq does not read (it reads format $manifestFormat): use the sbt-teq that release names, or a teq that writes format $manifestFormat")
    }

  /** One configuration's compile: the binary, the directory it runs in (the build's root, the
    * answer's paths relative to it), the class directory, whether it is a Scala.js project's
    * check (and its platform, which picks the TASTy jars it reads), the flags of the
    * configuration (its workers, its cacheable state and macro state, the scalac options it
    * maps), the configuration's current sources, its class path for the manifest's own
    * operation, the identity that the setup and the cache carry, and where the argument file
    * each process starts with goes (`ArgsFile`). */
  final case class Command(
      teq: String,
      root: File,
      classes: File,
      check: Boolean,
      platform: String,
      flags: Seq[String],
      sources: Seq[File],
      classpath: Seq[File],
      identity: String,
      argsPlace: ArgsFile.Place,
  ) {
    def build: Seq[String] =
      Seq("compiler") ++ (if (check) Seq("check") else Seq("build", "--target", "jvm", "--std=scala-library")) ++ Seq("--products", classes.getAbsolutePath)

    /** What a class path entry gives the build: a directory, and on the JVM every jar, in a check
      * the Scala 3 jars of the platform, whose TASTy it reads. */
    def reads(entry: File): Boolean =
      entry.isDirectory || entry.getName.endsWith(".jar") && (!check || isTastyJar(platform, entry))
  }

  private val stdJars = Seq("scala-library-", "scala3-library_", "scalajs-library_", "scalajs-scalalib_")

  /** A configuration under the toggle has no Java sources: teq reads Java class files only, and
    * zinc would hand a change of Java sources alone to javac without calling teq. */
  def refuseJava(sources: Seq[File]): Unit = {
    val java = sources.filter(_.getName.endsWith(".java"))
    if (java.nonEmpty)
      throw new MessageOnlyException(
        s"teq: Java sources are not compiled under teqCompiler (teq reads Java class files only): set teqCompiler := false for this project " +
          s"or move them to a project of their own: ${java.map(_.getPath).mkString(", ")}")
  }

  /** A Scala 3 library of the platform: `_sjs1_3-*.jar` on Scala.js, `_3-*.jar` on the JVM. */
  def isTastyJar(platform: String, jar: File): Boolean = {
    val name = jar.getName
    val ofPlatform =
      if (platform == "jvm") name.contains("_3-") && !name.contains("_sjs") && !name.contains("_native")
      else name.contains(s"_${platform}_3-")
    name.endsWith(".jar") && ofPlatform && !stdJars.exists(name.startsWith)
  }

  /** The files of this checkout the class directory's manifest names as sources. */
  private def manifestSources(classes: File, root: File): Seq[File] = {
    val f = classes / manifestName
    if (!f.isFile) Nil
    else {
      val m = readable(try Json.parse(IO.read(f)) catch { case _: Json.Malformed => Json.Null }, f)
      val base = Option(m("root").str).filter(_.nonEmpty).map(new File(_)).getOrElse(root)
      // A source is the file the filesystem names, a link resolved before a `..` after it; under
      // the root of a manifest written in another checkout (sbt's cache), also resolved, it is
      // this checkout's, as teq reads it (`classpath::entry_source`).
      val home = base.getCanonicalFile
      val moved = home != root.getCanonicalFile
      m("products").items.map(_("source").str).filter(_.nonEmpty).distinct.map { s =>
        val file = new File(s)
        val real = (if (file.isAbsolute) file else new File(base, s)).getCanonicalFile
        if (moved) IO.relativize(home, real).fold(real)(new File(root, _).getCanonicalFile) else real
      }.distinct
    }
  }

  /** The sources the manifest names that the configuration no longer has, as files of this
    * checkout, which teq reads the manifest's sources as: a file deleted, or left out of sbt's source
    * set while still on disk. */
  def removed(cmd: Command): Seq[String] = removed(cmd.classes, cmd.root, cmd.sources)

  def removed(classes: File, root: File, sources: Seq[File]): Seq[String] = {
    val current = sources.map(_.getCanonicalFile).toSet
    manifestSources(classes, root).filterNot(current).map(_.getPath)
  }

  /** The configuration's compiler: one `teq` process per batch zinc asks for, its answer fed to
    * zinc's callback. The original compiler is kept for what zinc asks of a compiler besides
    * compiling (its instance, which names the Scala version in the setup, and its class path
    * options). */
  final class Compiler(cmd: Command, scalac: ScalaCompiler, journal: Journal) extends ScalaCompiler {
    def scalaInstance(): ScalaInstance = scalac.scalaInstance()
    def classpathOptions(): ClasspathOptions = scalac.classpathOptions()

    def compile(sources: Array[VirtualFile], classpath: Array[VirtualFile], converter: FileConverter, changes: DependencyChanges, options: Array[String],
        output: Output, callback: AnalysisCallback, reporter: Reporter, progressOpt: Optional[CompileProgress], log: xsbti.Logger): Unit = {
      val files = sources.toSeq.map(f => converter.toPath(f).toFile)
      refuseJava(files)
      val entries = classpath.toSeq.map(f => converter.toPath(f).toFile).filter(cmd.reads)
      val classes = cmd.classes.getAbsoluteFile
      val path = (classes +: entries.filterNot(_.getAbsoluteFile == classes)).map(_.getAbsolutePath).distinct
      val command = Seq(cmd.teq) ++ cmd.build ++ Seq("--classpath", path.mkString(File.pathSeparator), "--sourceroot", cmd.root.getAbsolutePath,
        "--analysis-version", TeqAnalysis.version.toString) ++ removed(cmd).flatMap(Seq("--removed", _)) ++ cmd.flags ++ files.map(_.getAbsolutePath)
      journal.begin()
      val started = System.nanoTime()
      val (code, answer, stderr) = run(command, cmd)
      val ran = (System.nanoTime() - started) / 1e6
      val problems = answer("diagnostics").items.map(problem(cmd.root, _))
      if (code != 0 || answer("ok") != Json.Bool(true)) {
        // A failure the answer's diagnostics name is reported by them, teq's printing of them
        // kept to the debug log; one they do not name, by what teq printed.
        val errors =
          if (problems.exists(_.severity == Severity.Error)) {
            stderr.foreach(line => log.debug(() => line))
            problems
          }
          else problems :+ failure(command, code, stderr)
        errors.foreach(reporter.log)
        reporter.printSummary()
        throw new CompileFailed(command.toArray, "teq: compilation failed", errors.toArray, None, null)
      }
      if (cmd.check) Stamps.write(answer, cmd.root, classes)
      val fed = System.nanoTime()
      TeqAnalysis.feed(answer, cmd.root, classes, callback, converter, if (cmd.check) TeqAnalysis.Products.Stamps else TeqAnalysis.Products.ClassFiles)
      problems.foreach { p =>
        reporter.log(p)
        callback.problem(p.category, p.position, p.message, p.severity, true)
      }
      val ms = answer("ms")
      def phase(name: String) = ms(name) match {
        case Json.Num(n) => f"${n.toDouble}%.0f"
        case _ => "-"
      }
      val total = ms("total") match {
        case Json.Num(n) => n.toDouble
        case _ => ran
      }
      // A compiler that answers before it publishes leaves the publication out of its own times.
      val outside = ms("write") match {
        case Json.Num(_) => f"process start ${ran - total}%.0f, writing ${phase("write")}"
        case _ => f"process start and publication ${ran - total}%.0f"
      }
      log.info(() =>
        f"teq: ${files.size} source${if (files.size == 1) "" else "s"} of ${classes.getName} in $ran%.0f ms ($outside, class path ${phase("classpath")}, " +
          f"type ${phase("type")}, emission ${if (cmd.check) phase("tasty") else phase("emit")}, graph ${phase("api")}, callback ${(System.nanoTime() - fed) / 1e6}%.0f)")
    }
  } // end Compiler

  /** Runs `command` to its end, from the build's root with its arguments in the command's file:
    * the exit code, the answer (the last line of stdout, parsed) and the lines of stderr. A
    * compile cancelled meanwhile (sbt interrupts its thread) stops the process before it gives
    * way, so that nothing it would publish lands after zinc's rollback. */
  private def run(command: Seq[String], cmd: Command): (Int, Json.Value, Seq[String]) = {
    val process =
      try new java.lang.ProcessBuilder(ArgsFile.command(command, cmd.argsPlace)*).directory(cmd.root).start()
      catch { case e: java.io.IOException => throw new MessageOnlyException(s"teq could not be started as ${command.head} (teqBinary or TEQ overrides it): ${e.getMessage}") }
    process.getOutputStream.close()
    def lines(in: java.io.InputStream): java.util.concurrent.CompletableFuture[Seq[String]] =
      java.util.concurrent.CompletableFuture.supplyAsync(() => {
        val reader = new java.io.BufferedReader(new java.io.InputStreamReader(in, "UTF-8"))
        try Iterator.continually(reader.readLine()).takeWhile(_ != null).toVector
        finally reader.close()
      })
    val (stdout, stderr) = (lines(process.getInputStream), lines(process.getErrorStream))
    val code =
      try process.waitFor()
      catch {
        case e: InterruptedException =>
          process.destroy()
          if (!process.waitFor(5, java.util.concurrent.TimeUnit.SECONDS)) process.destroyForcibly().waitFor()
          throw e
      }
    val out = stdout.get()
    val answer = out.lastOption.map(line => try Json.parse(line) catch { case _: Json.Malformed => Json.Null }).getOrElse(Json.Null)
    (code, answer, stderr.get())
  }

  /** The failure of a build whose answer names no error: what teq printed, as one problem. */
  private def failure(command: Seq[String], code: Int, stderr: Seq[String]): Problem = {
    val text = if (stderr.isEmpty) s"teq exited with code $code" else stderr.take(20).mkString("\n")
    plainProblem(text, Severity.Error)
  }

  /** After zinc's run, the manifest's own operation for the sources the configuration no longer
    * has, which zinc calls no compiler for when they are all that changed: their entries leave
    * the manifest, their products having left with zinc's class-file manager. */
  def reconcile(cmd: Command, log: Logger): Unit = {
    val gone = removed(cmd)
    if (gone.nonEmpty) {
      val path = (cmd.classes +: cmd.classpath.filter(cmd.reads)).map(_.getAbsolutePath).distinct
      val command = Seq(cmd.teq) ++ cmd.build ++ Seq("--classpath", path.mkString(File.pathSeparator), "--sourceroot", cmd.root.getAbsolutePath,
        "--analysis-version", TeqAnalysis.version.toString) ++ gone.flatMap(Seq("--removed", _)) ++ cmd.flags
      val (code, answer, stderr) = run(command, cmd)
      if (code != 0 || answer("ok") != Json.Bool(true)) {
        stderr.foreach(line => log.error(line))
        throw new MessageOnlyException(s"teq: the products' manifest of ${cmd.classes} could not drop ${gone.mkString(", ")} (exit code $code)")
      }
      log.debug(s"teq: ${cmd.classes}'s manifest dropped ${gone.mkString(", ")}")
    }
  }

  /** The rollback of a run, zinc's external class-file manager, which zinc's own completes after
    * it. Before the run's first batch, once zinc has moved the invalidated sources' products
    * aside, it keeps the products' manifest, the class directory's listing, and a copy of every
    * file there that zinc does not own (no manifest entry's class files or pickle: the runtime's
    * and the std's classes, which a later batch of the run may remove or replace) in a directory
    * of the run's own beside the class directory. When the run fails (a batch, the callback, a
    * later cycle, a cancellation, with or without the batch's answer read), every file the run
    * added to the directory goes, a check's stamps among them, what a publication cut short
    * moved into the compiler's staging directory comes back and that directory goes, the copies
    * and the manifest are put back; zinc's manager then restores what it moved aside. */
  final class Journal(classes: File) extends ClassFileManager {
    private var kept: Option[(Option[String], Set[File])] = None
    private val saved = new File(classes.getPath + ".teq-run")
    private val stage = new File(classes.getPath + ".teq-stage")

    def begin(): Unit = synchronized {
      if (kept.isEmpty) {
        val manifest = classes / manifestName
        val text = if (manifest.isFile) Some(IO.read(manifest)) else None
        for (found <- text; m <- scala.util.Try(Json.parse(found)).toOption) readable(m, manifest)
        val before = listing
        val owned = text.fold(Set.empty[String])(entryFiles)
        IO.delete(saved)
        // A link sbt's cache restored whose blob the store has lost reads as no file: nothing to keep.
        for (f <- before; rel <- IO.relativize(classes, f) if rel != manifestName && !owned(rel) && !rel.endsWith(".teq") && f.exists)
          IO.copyFile(f, saved / rel)
        kept = Some((text, before))
      }
    }

    private def listing: Set[File] =
      if (classes.isDirectory) (classes ** -DirectoryFilter).get().toSet else Set.empty

    @deprecated("zinc calls the variant of VirtualFiles", "")
    override def delete(classes: Array[File]): Unit = ()
    @deprecated("zinc calls the variant of VirtualFiles", "")
    override def generated(classes: Array[File]): Unit = ()
    override def delete(classes: Array[VirtualFile]): Unit = ()
    override def generated(classes: Array[VirtualFile]): Unit = ()

    def complete(success: Boolean): Unit = synchronized {
      if (!success)
        kept.foreach { case (manifest, before) =>
          (listing -- before).foreach(IO.delete)
          val aside = stage / "old"
          for (f <- (if (aside.isDirectory) (aside ** -DirectoryFilter).get() else Nil); rel <- IO.relativize(aside, f)) {
            val target = classes / rel
            if (before(target) && !target.exists) {
              IO.createDirectory(target.getParentFile)
              IO.move(f, target)
            }
          }
          IO.delete(stage)
          for (f <- (if (saved.isDirectory) (saved ** -DirectoryFilter).get() else Nil); rel <- IO.relativize(saved, f)) {
            val target = classes / rel
            IO.delete(target)
            IO.createDirectory(target.getParentFile)
            IO.copyFile(f, target)
          }
          manifest match {
            case Some(text) => IO.write(classes / manifestName, text)
            case None => IO.delete(classes / manifestName)
          }
        }
      IO.delete(saved)
      kept = None
    }
  }

  /** The files the manifest's entries own, their class files and pickles, relative to the class
    * directory; none of a manifest that cannot be read. */
  private def entryFiles(manifest: String): Set[String] =
    scala.util.Try(Json.parse(manifest)).toOption.fold(Set.empty[String]) { m =>
      m("products") match {
        case Json.Arr(entries) =>
          entries.flatMap { e =>
            val classes = e("classes") match {
              case Json.Arr(cs) => cs.collect { case Json.Str(c) => c }
              case _ => Nil
            }
            classes ++ (e("tasty") match {
              case Json.Str(t) => Seq(t)
              case _ => Nil
            })
          }.toSet
        case _ => Set.empty
      }
    }

  /** `options` with `journal` as zinc's external class-file manager, beside any the build has. */
  def withJournal(options: IncOptions, journal: Journal): IncOptions = {
    val hooks: ExternalHooks = Option(options.externalHooks()).getOrElse(IncOptions.defaultExternal())
    val manager: ClassFileManager = hooks.getExternalClassFileManager.map[ClassFileManager](other => xsbti.compile.WrappedClassFileManager.of(journal, Optional.of(other))).orElse(journal)
    options.withExternalHooks(hooks.withExternalClassFileManager(manager))
  }

  private def absolute(root: File, path: String): File = {
    val f = new File(path)
    if (f.isAbsolute) f else new File(root, path)
  }

  /** A diagnostic of the answer, as a session's answer gives it, as zinc's problem. */
  private def problem(root: File, d: Json.Value): Problem = {
    val path = Option(d("file").str).filter(_.nonEmpty).map(p => absolute(root, p))
    def int(key: String): Optional[Integer] = d(key) match {
      case Json.Num(n) => Optional.of(Integer.valueOf(n.toInt))
      case _ => Optional.empty()
    }
    val col = d("col") match {
      case Json.Num(n) => Some(n.toInt - 1)
      case _ => None
    }
    val pos = new Position {
      def line(): Optional[Integer] = int("line")
      def lineContent(): String = d("source").str
      def offset(): Optional[Integer] = Optional.empty()
      def pointer(): Optional[Integer] = col.map(Integer.valueOf).fold(Optional.empty[Integer]())(Optional.of)
      def pointerSpace(): Optional[String] = col.map(" " * _).fold(Optional.empty[String]())(Optional.of)
      def sourcePath(): Optional[String] = path.map(_.getPath).fold(Optional.empty[String]())(Optional.of)
      def sourceFile(): Optional[File] = path.fold(Optional.empty[File]())(Optional.of)
      override def startLine(): Optional[Integer] = int("line")
      override def startColumn(): Optional[Integer] = pointer()
      override def endLine(): Optional[Integer] = int("endLine")
      override def endColumn(): Optional[Integer] = d("endCol") match {
        case Json.Num(n) => Optional.of(Integer.valueOf(n.toInt - 1))
        case _ => Optional.empty()
      }
    }
    val sev = d("severity").str match {
      case "error" => Severity.Error
      case "warning" => Severity.Warn
      case _ => Severity.Info
    }
    val text = d("message").str
    new Problem {
      def category(): String = "teq"
      def severity(): Severity = sev
      def message(): String = text
      def position(): Position = pos
    }
  }

  private def plainProblem(text: String, sev: Severity): Problem =
    new Problem {
      def category(): String = "teq"
      def severity(): Severity = sev
      def message(): String = text
      def position(): Position = nowhere
    }

  private val nowhere: Position = new Position {
    def line(): Optional[Integer] = Optional.empty()
    def lineContent(): String = ""
    def offset(): Optional[Integer] = Optional.empty()
    def pointer(): Optional[Integer] = Optional.empty()
    def pointerSpace(): Optional[String] = Optional.empty()
    def sourcePath(): Optional[String] = Optional.empty()
    def sourceFile(): Optional[File] = Optional.empty()
  }

  /** The products of a check, which writes TASTy alone: a small file under each binary name the
    * answer gives a class (`p/Foo.teq`, `p/Foo$.teq`), zinc's non-local product for the class and
    * what sbt's test digest stamps, whose content is a hash of its source's text and its binary
    * name, so that it moves whenever the source does, as a class file does. The Scala.js linker
    * never reads them: with the toggle off the class directory is emptied first (`Marker`). */
  object Stamps {
    def write(answer: Json.Value, root: File, classes: File): Seq[File] =
      for {
        f <- answer("api")("files").items
        source = absolute(root, f("file").str)
        text = if (source.isFile) IO.read(source) else ""
        product <- f("products").items
        binary = product.items.lift(1).fold("")(_.str)
        if binary.nonEmpty
      }
      yield {
        val file = classes / (binary.replace('.', '/') + ".teq")
        val content = hash(text + "\n" + binary) + "\n"
        if (!file.isFile || IO.read(file) != content) IO.write(file, content)
        file
      }

    private def hash(text: String): String = {
      val digest = java.security.MessageDigest.getInstance("SHA-1").digest(text.getBytes("UTF-8"))
      digest.map(b => f"${b & 0xff}%02x").mkString
    }
  }

  /** The marker beside a class directory naming the compiler that wrote it: `<dir>.backend`,
    * beside rather than inside the directory so that `packageBin` never sees it. zinc deletes
    * the products its analysis registered when the compiler changes (the setup's `extra`), not
    * the products' manifest or the runtime classes, which a teq downstream would read as teq's
    * products over what scalac wrote; so a directory another compiler wrote is emptied first. */
  object Marker {
    val Teq = "teq"

    private def file(classes: File): File = new File(classes.getParentFile, classes.getName + ".backend")

    /** Before a compile: `teq` is whether teq compiles the directory. A marker of an earlier
      * plugin names teq with its binary and flags, still teq's kind. */
    def check(classes: File, analysisFile: File, teq: Boolean, log: Logger): Unit = {
      val f = file(classes)
      val previous = if (f.isFile) Some(IO.read(f)) else None
      val wasTeq = previous.exists(p => p == Teq || p.startsWith("teq "))
      if (teq && !wasTeq && (previous.isDefined || classes.exists)) {
        log.info(s"teq: $classes was written by ${previous.getOrElse("scalac")}; teq compiles it afresh")
        IO.delete(classes)
        IO.delete(analysisFile)
      }
      else if (!teq && wasTeq) {
        log.info(s"teq: $classes was written by teq; scalac compiles it afresh")
        IO.delete(classes)
        IO.delete(analysisFile)
        IO.delete(f)
      }
      if (teq && previous != Some(Teq)) IO.write(f, Teq)
    }
  }
}
