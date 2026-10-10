package dev.teq.sbt

import java.io.{File, FileInputStream, IOException}
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import scala.collection.compat.*

import sbt.io.IO

/** The SHA-1 digests the plugin names files by: a resolved file's (the export's jars, the copy of
  * the binary under `target/teq/bin`), and the stamp beside that copy, `<copy>.sha1`,
  * which holds the digest it was copied with, so that the copy is made again whenever the resolved
  * file is another one (a binary published locally again under its SNAPSHOT version) and its bytes
  * are not read at every start. */
private[sbt] object Sha1 {
  private val digests = new ConcurrentHashMap[(String, Long, Long), String]()

  /** The digest of a resolved file, computed from its bytes once per file, size and date in a
    * session; not read from coursier's `.sha1` beside it, which can be another publication's
    * under another checksum policy. None when the file cannot be read. */
  def of(file: File): Option[String] =
    Option(digests.computeIfAbsent((file.getPath, file.length, file.lastModified), _ =>
      try compute(file) catch { case _: IOException => null }))

  def compute(file: File): String = {
    val digest = MessageDigest.getInstance("SHA-1")
    val in = new FileInputStream(file)
    try {
      val buffer = new Array[Byte](1 << 16)
      var read = in.read(buffer)
      while (read >= 0) {
        digest.update(buffer, 0, read)
        read = in.read(buffer)
      }
    }
    finally in.close()
    digest.digest().map(byte => f"$byte%02x").mkString
  }

  /** The 40 hex characters of a checksum file: bare, or followed by the file's name. */
  def in(text: String): Option[String] =
    text.trim.split("\\s+").headOption.filter(_.matches("[0-9a-fA-F]{40}")).map(_.toLowerCase)

  def stamp(copy: File): File = new File(copy.getPath + ".sha1")

  /** The copy's digest, from the stamp beside it; computed and written when the stamp is missing
    * or unreadable. None without a copy. */
  def stamped(copy: File): Option[String] =
    if (!copy.isFile) None
    else {
      val kept = try Option(stamp(copy)).filter(_.isFile).flatMap(found => in(IO.read(found))) catch { case _: IOException => None }
      kept.orElse(Some(writeStamp(copy)))
    }

  def writeStamp(copy: File): String = {
    val checksum = compute(copy)
    IO.write(stamp(copy), checksum)
    checksum
  }
}
