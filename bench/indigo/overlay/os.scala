// The part of os-lib (com.lihaoyi::os-lib) Indigo's plugin generators call, for bench/indigo/overlay/generate.scala:
// absolute and relative paths, a walk of a directory (in the order of its names, so that what is generated from
// it is the same on every machine), reads and writes, over java.nio.file. os-lib itself does not run under
// `teq interp` yet (bench/tyrian/overlay/os.scala).
package os

import java.nio.file.{Files, Paths, StandardOpenOption}

final class Path(val wrapped: java.nio.file.Path):
  def /(segment: String): Path = Path(wrapped.resolve(segment).normalize)
  def /(rel: RelPath): Path =
    var at = wrapped
    for _ <- 0 until rel.ups do at = at.getParent
    Path(rel.segments.foldLeft(at)((p, s) => p.resolve(s)).normalize)
  def last: String = wrapped.getFileName.toString
  def ext: String = RelPath.extOf(last)
  def baseName: String = RelPath.baseOf(last)
  def segments: IndexedSeq[String] = (0 until wrapped.getNameCount).map(i => wrapped.getName(i).toString)
  def relativeTo(base: Path): RelPath = RelPath(base.wrapped.relativize(wrapped).toString)
  def startsWith(base: Path): Boolean = wrapped.startsWith(base.wrapped)
  def toIO: java.io.File = wrapped.toFile
  def toNIO: java.nio.file.Path = wrapped
  override def toString: String = wrapped.toString
  override def equals(that: Any): Boolean = that match
    case p: Path => p.wrapped == wrapped
    case _ => false
  override def hashCode: Int = wrapped.hashCode

object Path:
  def apply(path: java.nio.file.Path): Path = new Path(path.toAbsolutePath.normalize)
  def apply(path: String): Path = new Path(Paths.get(path).toAbsolutePath.normalize)
  def apply(file: java.io.File): Path = apply(file.toPath)
  def apply(path: Path): Path = path

final class RelPath(val segments: IndexedSeq[String], val ups: Int):
  def /(segment: String): RelPath = segment match
    case ".." if segments.isEmpty => RelPath(segments, ups + 1)
    case ".." => RelPath(segments.init, ups)
    case "." | "" => this
    case s => RelPath(segments :+ s, ups)
  def /(rel: RelPath): RelPath =
    val kept = segments.dropRight(rel.ups)
    RelPath(kept ++ rel.segments, ups + math.max(0, rel.ups - segments.length))
  def last: String = segments.last
  def ext: String = RelPath.extOf(last)
  def baseName: String = RelPath.baseOf(last)
  def resolveFrom(base: Path): Path = base / this
  def startsWith(other: RelPath): Boolean = ups == other.ups && segments.startsWith(other.segments)
  def endsWith(other: RelPath): Boolean = other.ups == 0 && segments.endsWith(other.segments)
  override def toString: String = (Seq.fill(ups)("..") ++ segments).mkString("/")
  override def equals(that: Any): Boolean = that match
    case r: RelPath => r.segments == segments && r.ups == ups
    case _ => false
  override def hashCode: Int = segments.hashCode * 31 + ups

object RelPath:
  def apply(segments: IndexedSeq[String], ups: Int): RelPath = new RelPath(segments, ups)
  def apply(path: String): RelPath = path.split("/").filter(_.nonEmpty).foldLeft(new RelPath(Vector.empty, 0))(_ / _)
  def apply(file: java.io.File): RelPath = apply(file.getPath)
  val up: RelPath = new RelPath(Vector.empty, 1)
  val rel: RelPath = new RelPath(Vector.empty, 0)
  def extOf(name: String): String = name.lastIndexOf('.') match
    case -1 => ""
    case i => name.substring(i + 1)
  def baseOf(name: String): String = name.lastIndexOf('.') match
    case -1 => name
    case i => name.substring(0, i)

val up: RelPath = RelPath.up

def pwd: Path = Path(Paths.get("").toAbsolutePath)

def exists(path: Path): Boolean = Files.exists(path.wrapped)
def isDir(path: Path): Boolean = Files.isDirectory(path.wrapped)
def isFile(path: Path): Boolean = Files.isRegularFile(path.wrapped)

// Every path under the directory, a directory before what it holds, each directory's entries by name.
def walk(dir: Path): IndexedSeq[Path] =
  val out = scala.collection.mutable.ArrayBuffer.empty[Path]
  def visit(at: java.nio.file.Path): Unit =
    val entries = Files.list(at).toArray.toVector.map(_.asInstanceOf[java.nio.file.Path]).sortBy(_.getFileName.toString)
    for e <- entries do
      out += Path(e)
      if Files.isDirectory(e) then visit(e)
  visit(dir.wrapped)
  out.toVector

def list(dir: Path): IndexedSeq[Path] =
  Files.list(dir.wrapped).toArray.toVector.map(p => Path(p.asInstanceOf[java.nio.file.Path])).sortBy(_.last)

object makeDir:
  def all(path: Path): Unit = Files.createDirectories(path.wrapped)
  def apply(path: Path): Unit = Files.createDirectory(path.wrapped)

object remove:
  def all(path: Path): Unit =
    if Files.isDirectory(path.wrapped) then
      for p <- walk(path).reverse do Files.delete(p.wrapped)
    Files.deleteIfExists(path.wrapped)
  def apply(path: Path): Unit = Files.deleteIfExists(path.wrapped)

object write:
  def apply(path: Path, data: String): Unit = Files.writeString(path.wrapped, data, StandardOpenOption.CREATE_NEW)
  def over(path: Path, data: String): Unit = Files.writeString(path.wrapped, data)
  def over(path: Path, data: Array[Byte]): Unit = Files.write(path.wrapped, data)

object read:
  def apply(path: Path): String = Files.readString(path.wrapped)
  def lines(path: Path): IndexedSeq[String] = Files.readString(path.wrapped).linesIterator.toVector
  def bytes(path: Path): Array[Byte] = Files.readAllBytes(path.wrapped)

// A process the plugin's shader embedding would start to validate (glslangValidator), which no module of the
// pin asks for: none is started.
final case class CommandResult(exitCode: Int)
final class proc(command: String*):
  def call(cwd: Path = null): CommandResult = throw new UnsupportedOperationException("os.proc: no process under teq interp: " + command.mkString(" "))
