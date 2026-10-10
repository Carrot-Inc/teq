package dev.teq.sbt

import java.io.File
import java.nio.charset.StandardCharsets
import java.nio.file.{Files, StandardCopyOption}
import scala.collection.compat.*

import sbt.MessageOnlyException

/** teq started with its arguments in a file, `teq @<file>` (docs/TARGETS.md, "Argument
  * files"): Windows refuses a command line over 32,767 characters (error 206), which a class path
  * of a few hundred jars runs past, so every command of the plugin that carries one goes through a
  * file, on every platform, under the project's `target/teq/`, kept for the life of the process
  * it starts. A file is named by its project's configuration and kind and by a digest of its
  * whole contents: two commands with other arguments never share one, and none is replaced with
  * other contents, so that no process reads another's arguments and no lock is needed. */
object ArgsFile {
  /** Where the files of a project's commands of one configuration and kind go: `dir`, each named
    * `<stem>-<digest of its contents>.args`. */
  final case class Place(dir: File, stem: String)

  /** The files of a project's commands of a configuration and kind, under the project's
    * `target/teq/` beside the binary's copy: `batch` for the batches of `teqCompiler`, `build` for
    * `teqBuild`; a link's are `into`'s. */
  def of(base: File, configuration: String, kind: String): Place =
    Place(new File(new File(base, "target"), "teq"), s"$configuration-$kind")

  /** The files of a link of a kind (`link`, its resident; `full`, the full link) into
    * `directory`: their stem names the directory and a digest of its canonical path, the name
    * cut at a code point under a budget of UTF-8 bytes (`NameBytes`), so that a file's name and
    * the temporary one beside it stay under a file system's 255 bytes. */
  def into(base: File, configuration: String, kind: String, directory: File): Place = {
    val fixed = s"$configuration-$kind--${"0" * 8}-${"0" * Digits}.args".getBytes(StandardCharsets.UTF_8).length
    val name = truncated(directory.getName, (NameBytes - fixed).min(64).max(0))
    of(base, configuration, s"$kind-$name-${hex(sha256(directory.getCanonicalPath.getBytes(StandardCharsets.UTF_8))).take(8)}")
  }

  /** The most bytes a file's name takes: 255 less the temporary name's suffix
    * (`Files.createTempFile`'s `.<at most 20 digits>.tmp`). */
  val NameBytes = 255 - 25

  /** How many hex digits of the contents' SHA-256 name a file. */
  private val Digits = 16

  /** The longest prefix of `s` whose UTF-8 encoding fits in `budget` bytes, cut between two code
    * points. */
  def truncated(s: String, budget: Int): String = {
    val out = new java.lang.StringBuilder
    var used = 0
    val points = s.codePoints.iterator
    var full = false
    while (!full && points.hasNext) {
      val point = points.next()
      val bytes = new String(Character.toChars(point)).getBytes(StandardCharsets.UTF_8).length
      if (used + bytes > budget) full = true
      else {
        out.appendCodePoint(point)
        used += bytes
      }
    }
    out.toString
  }

  /** The file the arguments go to at `place`. */
  def file(place: Place, args: Seq[String]): File =
    new File(place.dir, s"${place.stem}-${hex(sha256(serialized(args))).take(Digits)}.args")

  /** The command that starts `command.head` with the rest of `command` in its file at `place`,
    * written first. */
  def command(command: Seq[String], place: Place): Seq[String] =
    Seq(command.head, "@" + write(place, command.tail).getAbsolutePath)

  /** Writes the arguments to their file at `place`, one per line, in UTF-8 whatever the JVM's
    * default charset, and returns it: written beside it and moved into place, so that no reader
    * meets a part of it; when it holds them already, left as it is but for its time, which a sweep
    * reads. An argument with a line end in it is refused, since the file would read it as two.
    * Then the other files of the place's stem older than `Kept` go (`sweep`). */
  def write(place: Place, args: Seq[String]): File = {
    val bytes = serialized(args)
    val target = file(place, args)
    if (target.isFile && java.util.Arrays.equals(Files.readAllBytes(target.toPath), bytes)) target.setLastModified(System.currentTimeMillis)
    else {
      val dir = target.getAbsoluteFile.getParentFile
      Files.createDirectories(dir.toPath)
      val written = Files.createTempFile(dir.toPath, target.getName + ".", ".tmp")
      try {
        Files.write(written, bytes)
        Files.move(written, target.toPath, StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
      }
      finally Files.deleteIfExists(written)
    }
    sweep(place, target)
    target
  }

  /** How long a file stays once another of its stem is written: a child reads its file at its
    * start (a batch again at its retry by one worker, within its run), and nothing holds one longer
    * than an hour. */
  val Kept: Long = 60L * 60 * 1000

  /** Removes the files of the place's stem, and the temporary ones of a write that did not end,
    * whose time is older than `Kept`, all but `written`; each is named by other contents, so no
    * process started within the hour reads it. */
  def sweep(place: Place, written: File): Unit = {
    val ofStem = (java.util.regex.Pattern.quote(place.stem) + s"-[0-9a-f]{$Digits}\\.args(\\.[0-9]+\\.tmp)?").r
    val before = System.currentTimeMillis - Kept
    for {
      f <- Option(place.dir.listFiles).toSeq.flatten
      if f.getName != written.getName && ofStem.pattern.matcher(f.getName).matches && f.lastModified < before
    }
      try Files.deleteIfExists(f.toPath)
      catch { case _: java.io.IOException => () }
  }

  /** The argument files under a project's `target/teq/`, of every stem and of earlier versions'
    * names, which `clean` removes; what else the plugin keeps there (the lock `teq.lock` where the
    * export writes it there, the binary's copy in `bin/`, the link directories with their `.lock`
    * and `.backend` files, `out/`, `test-entry/`) stays. */
  def files(base: File): Seq[File] =
    Option(new File(new File(base, "target"), "teq").listFiles((_: File, name: String) => name.matches(".*\\.args(\\.[0-9]+\\.tmp)?"))).toSeq.flatten

  /** The file's contents: each argument and a line end, in UTF-8; an argument holding a line end
    * is refused. */
  private def serialized(args: Seq[String]): Array[Byte] = {
    args.find(a => a.contains('\n') || a.contains('\r')).foreach { a =>
      throw new MessageOnlyException(s"teq: cannot pass ${a.replace("\n", "\\n").replace("\r", "\\r")} in an argument file: it holds a line end")
    }
    args.map(_ + "\n").mkString.getBytes(StandardCharsets.UTF_8)
  }

  private def sha256(bytes: Array[Byte]): Array[Byte] = java.security.MessageDigest.getInstance("SHA-256").digest(bytes)

  private def hex(bytes: Array[Byte]): String = bytes.map(b => f"${b & 0xff}%02x").mkString
}
