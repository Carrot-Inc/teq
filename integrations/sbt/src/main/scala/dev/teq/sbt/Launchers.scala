package dev.teq.sbt

import java.io.File
import java.nio.charset.StandardCharsets.UTF_8

import java.nio.file.Files

/** The launchers `teqExportAll` writes beside `teq.lock` (docs/TARGETS.md, "The launchers"): `teq`
  * (POSIX sh) and `teq.cmd`, teq's `tools/launcher/`, which the plugin carries in every version it
  * has shipped. A launcher is written when absent and replaced by the current one only while it is
  * one of those, unedited (line ends aside); any other (edited, or a later plugin's) is kept and
  * reported. The versions are the plugin's own ordering of the copies it carries, in no file. */
private[teq] object Launchers {
  val Names = Seq("teq", "teq.cmd")

  /** The templates by version, oldest first, each launcher's text with `\n` line ends. */
  lazy val shipped: Seq[(Int, Map[String, String])] = {
    def resource(path: String) = {
      val in = getClass.getResourceAsStream(s"/dev/teq/sbt/launcher/$path")
      if (in == null) sys.error(s"sbt-teq carries no launcher $path")
      try new String(in.readAllBytes, UTF_8) finally in.close()
    }
    resource("versions").linesIterator.filter(_.nonEmpty).map(_.toInt).toSeq.map(v => v -> Names.map(n => n -> resource(s"$v/$n")).toMap)
  }

  private def normalized(text: String) = text.replace("\r\n", "\n")

  /** A launcher's text as written: `teq.cmd` with cmd's line ends. */
  private def lineEnds(name: String, text: String) = if (name.endsWith(".cmd")) text.replace("\n", "\r\n") else text

  /** What `write` did to a launcher: written or replaced, or kept with the update it was not given. */
  final case class Report(kept: Boolean, message: String)

  /** Writes or keeps each launcher at `root`, and says what it did: a report per launcher written,
    * replaced or kept, none for one already current. */
  def write(root: File, templates: Seq[(Int, Map[String, String])] = shipped): Seq[Report] = {
    val current = templates.last._2
    Names.flatMap { name =>
      val file = new File(root, name)
      def place(): Unit = {
        Files.write(file.toPath, lineEnds(name, current(name)).getBytes(UTF_8))
        if (name == "teq") file.setExecutable(true, false)
      }
      if (!file.isFile) {
        place()
        Some(Report(false, s"wrote $file"))
      }
      else {
        val text = normalized(new String(Files.readAllBytes(file.toPath), UTF_8))
        if (text == current(name)) {
          if (name == "teq" && !file.canExecute) file.setExecutable(true, false)
          None
        }
        else templates.find(_._2(name) == text) match {
          case Some(_) =>
            place()
            Some(Report(false, s"replaced $file, an earlier launcher, by this plugin's"))
          case None =>
            val why = "it is edited, or a later sbt-teq's"
            val (ours, kept) = (current(name).linesIterator.toVector, text.linesIterator.toVector)
            val at = ours.indices.find(i => kept.lift(i) != Some(ours(i))).getOrElse(ours.size)
            val first = ours.lift(at).fold("(its end)")(line => s"`$line`")
            Some(Report(true, s"kept $file, since $why: this plugin's differs from it first at line ${at + 1}, $first"))
        }
      }
    }
  }
}
