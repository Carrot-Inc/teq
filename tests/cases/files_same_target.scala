//> using platform jvm
// os: unix
// `Files.copy` and `Files.move` onto the file itself do nothing, as the JDK decides it (`UnixCopyFile`:
// the target's device and inode, not followed, against the source's): by the same path, by another
// spelling of it, by a hard link; with or without REPLACE_EXISTING. A symbolic link to the source
// is another file: copying onto it with REPLACE_EXISTING replaces the link.
import java.nio.file.{Files, LinkOption, Path, StandardCopyOption}

object Main:
  def show(name: String)(f: => Any): Unit =
    val r =
      try f.toString
      catch case e: Exception => e.getClass.getName
    println(s"$name: $r")

  def main(args: Array[String]): Unit =
    val dir = Files.createTempDirectory("same-target")
    val p = dir.resolve("data")
    Files.writeString(p, "before")
    show("copy onto itself, replacing")(Files.copy(p, p, StandardCopyOption.REPLACE_EXISTING).getFileName.toString + " " + Files.readString(p))
    show("copy onto itself")(Files.copy(p, p).getFileName.toString + " " + Files.readString(p))
    val spelled = dir.resolve(".").resolve("data")
    show("copy onto another spelling")(Files.copy(p, spelled, StandardCopyOption.REPLACE_EXISTING) == spelled && Files.readString(p) == "before")
    val link = dir.resolve("hard")
    Files.createLink(link, p)
    show("copy onto a hard link")(Files.copy(p, link, StandardCopyOption.REPLACE_EXISTING) == link && Files.isSameFile(p, link))
    show("move onto a hard link")(Files.move(p, link) == link && Files.exists(p) && Files.exists(link) && Files.readString(link) == "before")
    show("move onto itself")(Files.move(p, p, StandardCopyOption.REPLACE_EXISTING) == p && Files.readString(p) == "before")
    show("move onto another spelling")(Files.move(p, spelled) == spelled && Files.readString(p) == "before")
    val other = dir.resolve("other")
    Files.writeString(other, "other")
    show("copy onto another file")(Files.copy(p, other))
    val sym = dir.resolve("sym")
    Files.createSymbolicLink(sym, p)
    show("copy onto a link to it")(Files.copy(p, sym, StandardCopyOption.REPLACE_EXISTING) == sym && !Files.isSymbolicLink(sym) && Files.readString(sym) == "before")
    List("sym", "other", "hard", "data").foreach(n => Files.delete(dir.resolve(n)))
    Files.delete(dir)
    show("cleaned")(Files.exists(dir))
