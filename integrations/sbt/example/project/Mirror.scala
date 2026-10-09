import sbt.*

import java.nio.file.{Files, StandardCopyOption}

/** The mirror of a linker's output directory that a dev server and a bundler read, as the
  * reference application keeps one: the Scala.js linker rewrites its files in place, so what
  * reads them reads a copy that only ever holds complete files. The directory is flat; a file
  * is copied when its length differs or the mirror's copy is older, through a file beside the
  * mirror and an atomic rename, and what the directory no longer has leaves the mirror. */
object Mirror:
  def of(directory: File): File = file(directory.getPath + "-staged")

  def sync(directory: File, log: Logger): Unit =
    val mirror = of(directory)
    IO.createDirectory(mirror)
    val files = IO.listFiles(directory).toList.filter(_.isFile)
    val names = files.map(_.getName).toSet
    IO.delete(IO.listFiles(mirror).filterNot(f => names(f.getName)))
    val changed = files.filter { f =>
      val copy = mirror / f.getName
      copy.length != f.length || copy.lastModified < f.lastModified
    }
    for f <- changed do
      val staging = directory.getParentFile / s".staging-${f.getName}"
      IO.copyFile(f, staging, preserveLastModified = true)
      Files.move(staging.toPath, (mirror / f.getName).toPath, StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
    if changed.nonEmpty then log.info(s"Mirrored ${changed.size} files into ${mirror.getName}")
