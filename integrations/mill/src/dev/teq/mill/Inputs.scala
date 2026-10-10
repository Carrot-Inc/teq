package dev.teq.mill

import java.nio.file.Path

import org.snakeyaml.engine.v2.api.{Load, LoadSettings}

import dev.teq.sbt.{Exported, Json}

/** A Mill build's inputs as the lock records them (docs/TOOLING.md, "The export of a Mill build"): its
  * definition files as Mill 1.x's `FileImportGraph.walkBuildFiles` finds the build's scripts, the
  * meta-build's files, the launcher's files and the overlay, and the inputs the overlay declares. teq's
  * reader finds the same files (`task::export::mill_definition_files`), so that an edit, a file added and
  * a file removed each make the lock stale. */
private[mill] object Inputs:
  /** The root build files of Mill 1.0 (`build.mill`, `build.mill.scala`, `build.sc`) and of Mill 1.1
    * (`build.mill.yaml`, `build.mill`). */
  val RootFiles = Seq("build.mill.yaml", "build.mill", "build.mill.scala", "build.sc")

  /** The files at a build's root that choose its launcher's Mill and JVM and their options. */
  val LauncherFiles = Seq(".mill-version", ".config/mill-version", ".mill-jvm-version", ".config/mill-jvm-version", ".mill-jvm-opts", ".mill-opts")

  /** The root build file Mill takes: Mill 1.1's first (`build.mill.yaml`, then `build.mill`), else Mill 1.0's. */
  def rootFile(root: os.Path): Option[os.Path] = RootFiles.map(root / _).find(os.isFile)

  /** The extensions of a build's scripts: `.mill` and `.mill.yaml` for a `build.mill` or `build.mill.yaml` build
    * (Mill 1.0 reads the first, Mill 1.1 both), `.mill.scala` or `.sc` for a build whose root file is
    * `build.mill.scala` or `build.sc` (Mill 1.0 alone). */
  def extensions(root: os.Path): Seq[String] =
    if os.isFile(root / "build.mill") || os.isFile(root / "build.mill.yaml") then Seq("mill", "mill.yaml")
    else if os.isFile(root / "build.mill.scala") then Seq("mill.scala")
    else if os.isFile(root / "build.sc") then Seq("sc")
    else Nil

  /** The definition files, by path relative to the root with `/`, as Mill 1.0's `FileImportGraph.walkBuildFiles`
    * and Mill 1.1's `BuildFileDiscovery.walkBuildFiles` find a build's scripts, the files of either: every
    * `package.<ext>` of a directory reached from the root through directories that hold one (`out/` and
    * `mill-build/` left out), and every file whose name ends in `.<ext>` beside the root's build file or a
    * `package.<ext>`, for each extension of the build's (`extensions`); every file under `mill-build/` outside its
    * hidden directories and its `out/`; the launcher's files and the overlay. None without a root build file. A
    * build that allows nested build files (Mill 1.1's `mill-allow-nested-build-mill`) is refused by the export. */
  def definitionFiles(root: os.Path): Seq[(String, Path)] =
    val exts = extensions(root)
    if exts.isEmpty then Nil
    else
      val packages = exts.map(e => s"package.$e")
      val dirs = Iterator.iterate(Seq(root))(_.flatMap { dir =>
        os.list(dir).filter(p => os.isDir(p) && packages.exists(n => os.isFile(p / n)) && !(dir == root && (p.last == "out" || p.last == "mill-build")))
      }).takeWhile(_.nonEmpty).flatten.toSeq
      val scripts = dirs.flatMap(dir => os.list(dir).filter(p => os.isFile(p) && exts.exists(e => p.last.endsWith(s".$e"))))
      def meta(dir: os.Path): Seq[os.Path] =
        if !os.isDir(dir) then Nil
        else os.list(dir).flatMap { p =>
          if os.isDir(p) then (if p.last == "out" || p.last.startsWith(".") then Nil else meta(p))
          else if os.isFile(p) then Seq(p)
          else Nil
        }
      val others = (LauncherFiles :+ Overlay.FileName).map(f => root / os.RelPath(f)).filter(os.isFile)
      (scripts ++ meta(root / "mill-build") ++ others).map(p => p.relativeTo(root).toString -> p.toNIO)
        .distinctBy(_._1).sortBy(_._1)(using Json.utf8)

  /** The key of Mill 1.1's header that allows nested build files. */
  val NestedBuilds = "mill-allow-nested-build-mill"

  /** A root file's header as Mill reads it (`mill.constants.Util.readBuildHeader`): a YAML root file whole, else
    * its leading `//|` lines, after a `#!` line, each less its `//| ` (a bare `//|` an empty line). */
  def header(file: os.Path): String =
    if file.last.endsWith(".yaml") then os.read(file)
    else
      val lines = os.read.lines(file)
      lines.drop(if lines.headOption.exists(_.startsWith("#!")) then 1 else 0).takeWhile(_.startsWith("//|"))
        .map(l => if l == "//|" then "" else l.drop(4)).mkString("\n")

  /** Whether the root build file allows nested build files (Mill 1.1's `mill-allow-nested-build-mill` in its
    * header, or a `build.mill.yaml`'s field), which makes a `build.mill` below the root one of its scripts. The
    * header is read as YAML as Mill reads it (`Util.readBooleanFromBuildHeader`, with snakeyaml-engine): a
    * comment, quotes or the flow style change nothing; any value but false counts, since Mill takes it as true
    * or fails, and so does a header naming the key that does not parse. */
  def allowsNestedBuilds(root: os.Path): Boolean =
    rootFile(root).exists { file =>
      val text = header(file)
      text.contains(NestedBuilds) && (
        try
          new Load(LoadSettings.builder().build()).loadFromString(text) match
            case m: java.util.Map[?, ?] if m.containsKey(NestedBuilds) =>
              m.get(NestedBuilds) match
                case b: java.lang.Boolean => b.booleanValue
                case v: String => !v.trim.equalsIgnoreCase("false")
                case _ => true
            case _ => false
        catch case scala.util.control.NonFatal(_) => true)
    }

  /** The inputs block: the definition files and the declared inputs that exist, each with its digest. */
  def of(root: os.Path, declared: Seq[String]): Exported.Inputs =
    val sorted = declared.distinct.sorted(using Json.utf8)
    val files = (definitionFiles(root) ++ sorted.map(d => d -> (root / os.RelPath(d)).toNIO).filter((_, p) => java.nio.file.Files.isRegularFile(p)))
      .distinctBy(_._1).sortBy(_._1)(using Json.utf8)
    Exported.digests(files, sorted)

  /** How a repository keeps a Mill build's definition files LF in every checkout, so that a lock exported
    * from one checkout is current in another: the remedy of the warning on a CRLF file (teq's reader names
    * the same, `task::export::Exporter::lf_remedy`). */
  val LfRemedy = ".gitattributes keeps the build's files LF in every checkout with *.mill text eol=lf, mill-build/** text eol=lf, .mill-version text eol=lf and teq.mill.json text eol=lf: add them, delete the files and check them out again, then export again"

  /** The warning of an export whose definition files hold CRLF; none when every file is LF. */
  def crlfWarning(root: os.Path): Option[String] =
    val crlf = definitionFiles(root).collect { case (path, p) if holdsCrlf(p) => path }
    Option.when(crlf.nonEmpty) {
      val (have, their) = if crlf.size == 1 then ("has", "its") else ("have", "their")
      s"teq: ${crlf.mkString(", ")} $have CRLF line ends, and teq.lock records $their bytes, so that a checkout with LF finds it stale: $LfRemedy"
    }

  private def holdsCrlf(p: Path): Boolean =
    try
      val bytes = java.nio.file.Files.readAllBytes(p)
      (1 until bytes.length).exists(i => bytes(i) == '\n' && bytes(i - 1) == '\r')
    catch case scala.util.control.NonFatal(_) => false
