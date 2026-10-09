//> using platform jvm
// os: windows
// Windows' file system against the JDK on Windows (the coordinator's acceptance run, `teq.cmd interp`; its
// expectation is the Windows JDK's): the platform's properties as the JDK names them; drive, rooted and
// backslash paths taken apart and put together; a directory with a space and non-ASCII characters and a
// path past 260 characters written, read and listed; a file moved, its time set and read back, and
// deleted, each with its streams closed first; the POSIX permissions refused as the JDK refuses them on
// NTFS; a symbolic link, which needs a privilege or the developer mode. What depends on the machine (its
// user, its temporary directory) is printed as a comparison.
import java.nio.file.{Files, FileSystemException, Path, Paths}
import java.nio.file.attribute.{FileTime, PosixFilePermissions}
import scala.jdk.CollectionConverters.*

object Main:
  def show(name: String)(f: => Any): Unit =
    val r =
      try f.toString
      catch case e: Exception => e.getClass.getName
    println(s"$name: $r")

  def main(args: Array[String]): Unit =
    show("os.name is Windows")(System.getProperty("os.name").startsWith("Windows"))
    show("os.arch")(Set("amd64", "aarch64").contains(System.getProperty("os.arch")))
    show("file.separator")(System.getProperty("file.separator"))
    show("path.separator")(System.getProperty("path.separator"))
    show("separators")(java.io.File.separator + " " + java.io.File.pathSeparator)
    show("line.separator is CRLF")(System.lineSeparator() == "\r\n")
    show("user.home is USERPROFILE")(System.getProperty("user.home") == System.getenv("USERPROFILE"))
    show("java.io.tmpdir ends with a backslash")(System.getProperty("java.io.tmpdir").endsWith("\\"))

    val p = Paths.get("C:\\a dir\\sub\\file.txt")
    show("root")(p.getRoot)
    show("file name")(p.getFileName)
    show("names")(p.getNameCount)
    show("absolute")(p.isAbsolute)
    show("parent")(p.getParent)
    show("slashes")(Paths.get("C:/x/y"))
    show("relative")(Paths.get("a/b\\c"))
    show("normalized")(Paths.get("C:\\a\\b\\..\\c\\.\\d").normalize)
    show("resolved")(Paths.get("C:\\x").resolve("y z"))
    show("resolved absolute")(Paths.get("C:\\x").resolve("D:\\w"))
    show("relativized")(Paths.get("C:\\x\\y").relativize(Paths.get("C:\\x\\z\\w")))
    show("rooted absolute")(Paths.get("\\rooted\\f").isAbsolute)
    show("rooted root")(Paths.get("\\rooted\\f").getRoot)
    show("drive relative root")(Paths.get("C:rel").getRoot)
    show("same path, other case")(Paths.get("C:\\A\\B") == Paths.get("c:\\a\\b"))
    show("file of a path")(Paths.get("C:\\a\\b").toFile.getPath)

    val base = Files.createTempDirectory("teq win \u00fc")
    val dir = Files.createDirectories(base.resolve("sub dir \u00f1\u4e2d"))
    val f = dir.resolve("na\u00efve file.txt")
    Files.writeString(f, "h\u00e9llo\n")
    show("written and read")(Files.readString(f) == "h\u00e9llo\n")
    show("listed")(Files.list(dir).iterator().asScala.map(_.getFileName.toString).toList == List("na\u00efve file.txt"))
    show("the directory's real path is itself")(dir.toRealPath().getFileName == dir.getFileName)

    var deep = base
    while deep.toString.length < 300 do deep = deep.resolve("a segment of a long path")
    Files.createDirectories(deep)
    val far = deep.resolve("far.txt")
    Files.writeString(far, "far")
    show("a long path written and read")(far.toString.length > 260 && Files.readString(far) == "far")
    show("a long path listed")(Files.list(deep).count())

    val out = Files.newOutputStream(dir.resolve("moved.txt"))
    out.write("bytes".getBytes)
    out.close()
    val moved = Files.move(dir.resolve("moved.txt"), dir.resolve("moved again.txt"))
    show("moved")(Files.exists(moved) && !Files.exists(dir.resolve("moved.txt")))
    val in = Files.newInputStream(moved)
    in.readAllBytes()
    in.close()
    Files.setLastModifiedTime(moved, FileTime.fromMillis(1700000000123L))
    show("time")(Files.getLastModifiedTime(moved).toMillis)
    Files.delete(moved)
    show("deleted")(Files.exists(moved))

    show("permissions read")(Files.getPosixFilePermissions(f))
    show("permissions set")(Files.setPosixFilePermissions(f, PosixFilePermissions.fromString("rw-r--r--")))

    val link = dir.resolve("link")
    val linked =
      try
        Files.createSymbolicLink(link, f)
        Files.isSymbolicLink(link) && Files.readString(link) == "h\u00e9llo\n"
      catch case _: FileSystemException => true
    show("a link made, or refused for the privilege")(linked)

    Files.walk(base).iterator().asScala.toList.reverse.foreach(Files.delete)
    show("cleaned")(Files.exists(base))
