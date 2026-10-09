package dev.teq.sbt

import java.io.File
import java.nio.charset.StandardCharsets.UTF_8

import sbt.io.IO

/** The launchers `teqExportAll` writes beside `teq.lock` (docs/TARGETS.md, "The launchers"): `teq`
  * (POSIX sh) and `teq.cmd`, teq's `tools/launcher/`, which the plugin carries in every version it
  * has shipped. A launcher is written when absent and replaced by the current one only while it is
  * one of those, unedited (line ends aside); any other (edited, without the marker, or a later
  * plugin's) is kept and reported. */
private[sbt] object Launchers:
  val Names = Seq("teq", "teq.cmd")
  private val Marker = """(?m)^(?:#|rem) teq launcher (\d+):""".r

  /** The templates by version, oldest first, each launcher's text with `\n` line ends. */
  lazy val shipped: Seq[(Int, Map[String, String])] =
    def resource(path: String) =
      val in = getClass.getResourceAsStream(s"/dev/teq/sbt/launcher/$path")
      if in == null then sys.error(s"sbt-teq carries no launcher $path")
      try new String(in.readAllBytes, UTF_8) finally in.close()
    resource("versions").linesIterator.filter(_.nonEmpty).map(_.toInt).toSeq.map(v => v -> Names.map(n => n -> resource(s"$v/$n")).toMap)

  private def normalized(text: String) = text.replace("\r\n", "\n")

  /** A launcher's text as written: `teq.cmd` with cmd's line ends. */
  private def lineEnds(name: String, text: String) = if name.endsWith(".cmd") then text.replace("\n", "\r\n") else text

  /** What `write` did to a launcher: written or replaced, or kept with the update it was not given. */
  final case class Report(kept: Boolean, message: String)

  /** Writes or keeps each launcher at `root`, and says what it did: a report per launcher written,
    * replaced or kept, none for one already current. */
  def write(root: File, templates: Seq[(Int, Map[String, String])] = shipped): Seq[Report] =
    val (version, current) = templates.last
    Names.flatMap { name =>
      val file = new File(root, name)
      def place(): Unit =
        IO.write(file, lineEnds(name, current(name)), UTF_8)
        if name == "teq" then file.setExecutable(true, false)
      if !file.isFile then
        place()
        Some(Report(false, s"wrote $file, launcher $version"))
      else
        val text = normalized(IO.read(file, UTF_8))
        if text == current(name) then
          if name == "teq" && !file.canExecute then file.setExecutable(true, false)
          None
        else templates.find(_._2(name) == text) match
          case Some((old, _)) =>
            place()
            Some(Report(false, s"replaced $file, launcher $old, by launcher $version"))
          case None =>
            val theirs = Marker.findFirstMatchIn(text).map(_.group(1).toInt)
            val why = theirs match
              case None => "it has no `teq launcher <version>:` line"
              case Some(v) if v > version => s"it is launcher $v, a later sbt-teq's"
              case Some(v) => s"it is launcher $v, edited"
            val (ours, kept) = (current(name).linesIterator.toVector, text.linesIterator.toVector)
            val at = ours.indices.find(i => kept.lift(i) != Some(ours(i))).getOrElse(ours.size)
            val first = ours.lift(at).fold("(its end)")(line => s"`$line`")
            Some(Report(true, s"kept $file, since $why: launcher $version differs from it first at line ${at + 1}, $first"))
    }
